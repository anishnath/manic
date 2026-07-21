//! The **Manic ML kit**: small, numerically truthful machine-learning stories.
//!
//! ML1 starts with three words:
//!
//! - `network` declares a deterministic feed-forward model and its responsive
//!   layered view;
//! - `activation` draws a named scalar activation function;
//! - `forward` computes and reveals one real forward pass.
//!
//! ML2 adds one supervised learning step without replacing that model:
//!
//! - `loss` compares the current output with an authored target;
//! - `backward` computes exact reverse-mode gradients;
//! - `update` applies gradient descent and recomputes the prediction;
//! - `checkpoint` + `restore` make one exact rollback visible without claiming
//!   to perform general machine unlearning.
//!
//! The kit does not attempt to be a training framework. Model arithmetic is
//! computed once while lowering and emitted as ordinary stateless timeline
//! tracks, so seeking and recording remain pure functions of time.

use macroquad::prelude::{Color, Vec2};

use crate::easing::Easing;
use crate::lang::diag::Error;
use crate::lang::lower::{resolve_easing, Args, Registry};
use crate::primitives::{Entity, FontKind, Link, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TextEvent, TrackSpec, Value};

const MAX_UNITS: usize = 128;
const MAX_TOTAL_UNITS: usize = 512;
const MAX_VISIBLE_UNITS: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    Linear,
    Relu,
    Sigmoid,
    Tanh,
    Softmax,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossKind {
    Mse,
    CrossEntropy,
}

impl LossKind {
    fn parse(word: &str) -> Option<Self> {
        match word.to_ascii_lowercase().as_str() {
            "mse" | "meansquarederror" | "mean_squared_error" => Some(Self::Mse),
            "crossentropy" | "cross_entropy" | "ce" => Some(Self::CrossEntropy),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Mse => "MSE",
            Self::CrossEntropy => "Cross-entropy",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MlLossData {
    pub kind: LossKind,
    pub target: Vec<f32>,
    pub value: f32,
}

#[derive(Debug, Clone)]
pub struct MlGradientData {
    /// Layer → gradient shown at that layer. Layer zero is dL/dx; later layers
    /// are dL/dz for the affine value before the layer activation.
    pub node: Vec<Vec<f32>>,
    /// Transition → output unit → input unit.
    pub weights: Vec<Vec<Vec<f32>>>,
    /// Transition → output unit.
    pub biases: Vec<Vec<f32>>,
    pub norm: f32,
}

#[derive(Debug, Clone)]
pub struct MlUpdateData {
    pub learning_rate: f32,
    pub old_loss: f32,
    pub new_loss: f32,
}

impl Activation {
    pub(crate) fn parse(word: &str) -> Option<Self> {
        match word.to_ascii_lowercase().as_str() {
            "linear" | "identity" => Some(Self::Linear),
            "relu" => Some(Self::Relu),
            "sigmoid" | "logistic" => Some(Self::Sigmoid),
            "tanh" => Some(Self::Tanh),
            "softmax" => Some(Self::Softmax),
            _ => None,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::Relu => "ReLU",
            Self::Sigmoid => "Sigmoid",
            Self::Tanh => "Tanh",
            Self::Softmax => "Softmax",
        }
    }

    pub(crate) fn scalar(self, x: f32) -> f32 {
        match self {
            Self::Linear => x,
            Self::Relu => x.max(0.0),
            Self::Sigmoid => {
                // Stable on both sides instead of blindly evaluating exp(-x).
                if x >= 0.0 {
                    1.0 / (1.0 + (-x).exp())
                } else {
                    let e = x.exp();
                    e / (1.0 + e)
                }
            }
            Self::Tanh => x.tanh(),
            // Softmax is a vector operation and is handled by `apply`.
            Self::Softmax => x,
        }
    }

    fn apply(self, values: &mut [f32]) {
        if self == Self::Softmax {
            stable_softmax(values);
        } else {
            for value in values {
                *value = self.scalar(*value);
            }
        }
    }
}

/// Build-time state retained by a `network`. It is public only so `Scene` can
/// own it; creators interact through the DSL.
#[derive(Debug, Clone)]
pub struct MlNetworkData {
    pub layers: Vec<usize>,
    pub activations: Vec<Activation>,
    /// Transition → output unit → input unit.
    pub weights: Vec<Vec<Vec<f32>>>,
    /// Transition → output unit.
    pub biases: Vec<Vec<f32>>,
    /// Actual unit indices represented visually in each layer. Large layers
    /// keep their full arithmetic while the view uses a deterministic subset.
    pub visible: Vec<Vec<usize>>,
    pub positions: Vec<Vec<Vec2>>,
    pub radius: f32,
    pub bar_direction: f32,
    pub last_values: Option<Vec<Vec<f32>>>,
    pub last_loss: Option<MlLossData>,
    pub last_gradients: Option<MlGradientData>,
    pub last_update: Option<MlUpdateData>,
}

/// One exact, build-time network snapshot retained by `checkpoint`. It is
/// intentionally tied to the originating network: the compact vocabulary
/// teaches rollback of a known learning step, not model-file serialization.
#[derive(Debug, Clone)]
pub struct MlNetworkCheckpointData {
    pub network: String,
    pub state: MlNetworkData,
}

fn split_words(src: &str) -> impl Iterator<Item = &str> {
    src.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|part| !part.is_empty())
}

fn parse_layers(src: &str) -> Result<Vec<usize>, String> {
    let mut layers = Vec::new();
    for word in split_words(src) {
        let size = word
            .parse::<usize>()
            .map_err(|_| format!("layer size `{word}` is not a positive integer"))?;
        if size == 0 {
            return Err("layer sizes must be greater than zero".into());
        }
        if size > MAX_UNITS {
            return Err(format!(
                "a layer may contain at most {MAX_UNITS} units in ML1 (got {size})"
            ));
        }
        layers.push(size);
    }
    if layers.len() < 2 {
        return Err("network needs at least an input and output layer".into());
    }
    let total: usize = layers.iter().sum();
    if total > MAX_TOTAL_UNITS {
        return Err(format!(
            "network contains {total} units; ML1 supports at most {MAX_TOTAL_UNITS}"
        ));
    }
    Ok(layers)
}

fn parse_activations(src: &str, transitions: usize) -> Result<Vec<Activation>, String> {
    let mut activations = Vec::new();
    for word in split_words(src) {
        activations.push(Activation::parse(word).ok_or_else(|| {
            format!("unknown activation `{word}` (try: linear, relu, sigmoid, tanh, softmax)")
        })?);
    }
    if activations.len() == 1 && transitions > 1 {
        activations.resize(transitions, activations[0]);
    }
    if activations.len() != transitions {
        return Err(format!(
            "network has {transitions} transitions but {} activation name(s); provide one to reuse or one per transition",
            activations.len()
        ));
    }
    // Softmax only has a coherent meaning on a complete vector. Allow it on
    // any transition, but never combine it element-by-element.
    Ok(activations)
}

fn parse_values(src: &str) -> Result<Vec<f32>, String> {
    let mut values = Vec::new();
    for word in split_words(src) {
        let value = word
            .parse::<f32>()
            .map_err(|_| format!("input value `{word}` is not a number"))?;
        if !value.is_finite() {
            return Err(format!("input value `{word}` is not finite"));
        }
        values.push(value);
    }
    if values.is_empty() {
        return Err("forward input cannot be empty".into());
    }
    Ok(values)
}

fn parse_target(src: &str) -> Result<Vec<f32>, String> {
    parse_values(src).map_err(|message| message.replace("input", "target"))
}

fn stable_softmax(values: &mut [f32]) {
    if values.is_empty() {
        return;
    }
    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0;
    for value in values.iter_mut() {
        *value = (*value - max).exp();
        sum += *value;
    }
    if sum.is_finite() && sum > 0.0 {
        for value in values {
            *value /= sum;
        }
    }
}

fn forward_values(model: &MlNetworkData, input: &[f32]) -> Result<Vec<Vec<f32>>, String> {
    if input.len() != model.layers[0] {
        return Err(format!(
            "network expects {} input values, got {}",
            model.layers[0],
            input.len()
        ));
    }
    let mut all = vec![input.to_vec()];
    for transition in 0..model.weights.len() {
        let source = all.last().expect("input was seeded");
        let mut output = Vec::with_capacity(model.layers[transition + 1]);
        for (row, bias) in model.weights[transition]
            .iter()
            .zip(&model.biases[transition])
        {
            let value = row
                .iter()
                .zip(source)
                .fold(*bias, |sum, (weight, input)| sum + weight * input);
            if !value.is_finite() {
                return Err(format!(
                    "layer {} produced a non-finite affine value",
                    transition + 1
                ));
            }
            output.push(value);
        }
        model.activations[transition].apply(&mut output);
        if output.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "{} at layer {} produced a non-finite value",
                model.activations[transition].name(),
                transition + 1
            ));
        }
        all.push(output);
    }
    Ok(all)
}

fn activation_backward(activation: Activation, output: &[f32], upstream: &[f32]) -> Vec<f32> {
    debug_assert_eq!(output.len(), upstream.len());
    match activation {
        Activation::Linear => upstream.to_vec(),
        Activation::Relu => output
            .iter()
            .zip(upstream)
            .map(|(&value, &grad)| if value > 0.0 { grad } else { 0.0 })
            .collect(),
        Activation::Sigmoid => output
            .iter()
            .zip(upstream)
            .map(|(&value, &grad)| grad * value * (1.0 - value))
            .collect(),
        Activation::Tanh => output
            .iter()
            .zip(upstream)
            .map(|(&value, &grad)| grad * (1.0 - value * value))
            .collect(),
        Activation::Softmax => {
            // J_softmax^T g = y ⊙ (g - <g,y>). The Jacobian is symmetric,
            // but writing the vector-Jacobian product avoids an O(n²) matrix.
            let dot = output
                .iter()
                .zip(upstream)
                .map(|(&value, &grad)| value * grad)
                .sum::<f32>();
            output
                .iter()
                .zip(upstream)
                .map(|(&value, &grad)| value * (grad - dot))
                .collect()
        }
    }
}

fn validate_target(kind: LossKind, output: &[f32], target: &[f32]) -> Result<(), String> {
    if target.len() != output.len() {
        return Err(format!(
            "network has {} outputs but loss target has {} values",
            output.len(),
            target.len()
        ));
    }
    if target.iter().any(|value| !value.is_finite()) {
        return Err("loss target values must be finite".into());
    }
    if kind == LossKind::CrossEntropy {
        if target.iter().any(|&value| value < 0.0) {
            return Err("cross-entropy targets cannot be negative".into());
        }
        let sum = target.iter().sum::<f32>();
        if (sum - 1.0).abs() > 1e-4 {
            return Err(format!(
                "cross-entropy targets must sum to 1 (got {sum:.6})"
            ));
        }
    }
    Ok(())
}

fn loss_value_and_output_gradient(
    kind: LossKind,
    output: &[f32],
    target: &[f32],
) -> Result<(f32, Vec<f32>), String> {
    validate_target(kind, output, target)?;
    match kind {
        LossKind::Mse => {
            let scale = 1.0 / output.len().max(1) as f32;
            let gradient = output
                .iter()
                .zip(target)
                .map(|(&value, &wanted)| (value - wanted) * scale)
                .collect::<Vec<_>>();
            let value = output
                .iter()
                .zip(target)
                .map(|(&value, &wanted)| {
                    let error = value - wanted;
                    0.5 * error * error * scale
                })
                .sum();
            Ok((value, gradient))
        }
        LossKind::CrossEntropy => {
            let epsilon = 1e-12;
            let value = output
                .iter()
                .zip(target)
                .map(|(&probability, &wanted)| -wanted * probability.clamp(epsilon, 1.0).ln())
                .sum();
            let gradient = output
                .iter()
                .zip(target)
                .map(|(&probability, &wanted)| -wanted / probability.max(epsilon))
                .collect();
            Ok((value, gradient))
        }
    }
}

fn backward_values(
    model: &MlNetworkData,
    values: &[Vec<f32>],
    loss: &MlLossData,
) -> Result<MlGradientData, String> {
    if values.len() != model.layers.len() {
        return Err("stored forward values do not match the network depth".into());
    }
    let output = values.last().ok_or("network has no output values")?;
    let (_, output_gradient) = loss_value_and_output_gradient(loss.kind, output, &loss.target)?;
    let last = model.layers.len() - 1;
    let mut node = model
        .layers
        .iter()
        .map(|&size| vec![0.0; size])
        .collect::<Vec<_>>();
    node[last] = if loss.kind == LossKind::CrossEntropy
        && model.activations.last() == Some(&Activation::Softmax)
    {
        output
            .iter()
            .zip(&loss.target)
            .map(|(&probability, &wanted)| probability - wanted)
            .collect()
    } else {
        activation_backward(model.activations[last - 1], output, &output_gradient)
    };

    let mut weight_gradients = model
        .weights
        .iter()
        .map(|matrix| {
            matrix
                .iter()
                .map(|row| vec![0.0; row.len()])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut bias_gradients = model
        .biases
        .iter()
        .map(|bias| vec![0.0; bias.len()])
        .collect::<Vec<_>>();

    for transition in (0..model.weights.len()).rev() {
        for output_unit in 0..model.layers[transition + 1] {
            bias_gradients[transition][output_unit] = node[transition + 1][output_unit];
            for input_unit in 0..model.layers[transition] {
                weight_gradients[transition][output_unit][input_unit] =
                    node[transition + 1][output_unit] * values[transition][input_unit];
            }
        }

        let mut upstream = vec![0.0; model.layers[transition]];
        for (input_unit, value) in upstream.iter_mut().enumerate() {
            *value = (0..model.layers[transition + 1])
                .map(|output_unit| {
                    model.weights[transition][output_unit][input_unit]
                        * node[transition + 1][output_unit]
                })
                .sum();
        }
        node[transition] = if transition == 0 {
            upstream
        } else {
            activation_backward(
                model.activations[transition - 1],
                &values[transition],
                &upstream,
            )
        };
    }

    let norm = weight_gradients
        .iter()
        .flatten()
        .flatten()
        .chain(bias_gradients.iter().flatten())
        .map(|gradient| gradient * gradient)
        .sum::<f32>()
        .sqrt();
    if !norm.is_finite() {
        return Err("backward pass produced a non-finite gradient".into());
    }
    Ok(MlGradientData {
        node,
        weights: weight_gradients,
        biases: bias_gradients,
        norm,
    })
}

fn lcg(seed: &mut u64) -> f32 {
    *seed = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*seed >> 40) as u32) as f32 / (1u32 << 24) as f32
}

fn generated_parameters(layers: &[usize], seed: u64) -> (Vec<Vec<Vec<f32>>>, Vec<Vec<f32>>) {
    let mut state = seed.max(1);
    let mut weights = Vec::with_capacity(layers.len() - 1);
    let mut biases = Vec::with_capacity(layers.len() - 1);
    for pair in layers.windows(2) {
        let (inputs, outputs) = (pair[0], pair[1]);
        // Xavier uniform keeps the small explanatory models active without
        // immediately saturating sigmoid/tanh.
        let limit = (6.0 / (inputs + outputs) as f32).sqrt();
        let mut matrix = Vec::with_capacity(outputs);
        for _ in 0..outputs {
            let mut row = Vec::with_capacity(inputs);
            for _ in 0..inputs {
                row.push((lcg(&mut state) * 2.0 - 1.0) * limit);
            }
            matrix.push(row);
        }
        let bias = (0..outputs)
            .map(|_| (lcg(&mut state) * 2.0 - 1.0) * 0.08)
            .collect();
        weights.push(matrix);
        biases.push(bias);
    }
    (weights, biases)
}

fn display_slots(count: usize) -> Vec<Option<usize>> {
    if count <= MAX_VISIBLE_UNITS {
        return (0..count).map(Some).collect();
    }
    let side = (MAX_VISIBLE_UNITS - 1) / 2;
    (0..side)
        .map(Some)
        .chain(std::iter::once(None))
        .chain(((count - side)..count).map(Some))
        .collect()
}

fn mix(a: Color, b: Color, amount: f32) -> Color {
    let u = amount.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * u,
        a.g + (b.g - a.g) * u,
        a.b + (b.b - a.b) * u,
        a.a + (b.a - a.a) * u,
    )
}

fn fmt_value(value: f32) -> String {
    if value.abs() >= 100.0 || (value != 0.0 && value.abs() < 0.01) {
        format!("{value:.1e}")
    } else {
        format!("{value:.2}")
    }
}

fn node_id(network: &str, layer: usize, unit: usize) -> String {
    format!("{network}.l{layer}.n{unit}")
}

fn edge_id(network: &str, transition: usize, input: usize, output: usize) -> String {
    format!("{network}.e{transition}.{input}.{output}")
}

fn add_tag(entity: &mut Entity, network: &str, group: &str) {
    entity.tags.push(network.to_string());
    entity.tags.push(format!("{network}.{group}"));
}

/// `network(id, (cx,cy), "3 5 2", "relu softmax", [width], [height], [seed])`
fn c_network(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(7)?;
    let id = args.ident(0)?;
    if scene.ml_networks.contains_key(&id) {
        return Err(Error::new(
            format!("ML network `{id}` already exists"),
            args.span_of(0),
        ));
    }
    let center = args.pair(1)?;
    let layers =
        parse_layers(&args.text(2)?).map_err(|message| Error::new(message, args.span_of(2)))?;
    let activations = parse_activations(&args.text(3)?, layers.len() - 1)
        .map_err(|message| Error::new(message, args.span_of(3)))?;
    let width = args
        .opt_num(4)?
        .unwrap_or((scene.canvas_size.x * 0.68).clamp(460.0, 980.0));
    let height = args
        .opt_num(5)?
        .unwrap_or((scene.canvas_size.y * 0.48).clamp(300.0, 620.0));
    let seed_num = args.opt_num(6)?.unwrap_or(7.0);
    if width < 240.0 || height < 180.0 {
        return Err(Error::new(
            "network width must be at least 240 and height at least 180",
            args.span_of(if width < 240.0 { 4 } else { 5 }),
        ));
    }
    if !seed_num.is_finite() || seed_num < 0.0 {
        return Err(Error::new(
            "network seed must be a finite non-negative number",
            args.span_of(6),
        ));
    }
    let seed = seed_num.round() as u64;
    let (weights, biases) = generated_parameters(&layers, seed);

    let max_slots = layers
        .iter()
        .map(|&count| display_slots(count).len())
        .max()
        .unwrap_or(1);
    let radius = (height / (max_slots as f32 * 2.65)).clamp(13.0, 28.0);
    let x0 = center.x - width * 0.5;
    let dx = width / (layers.len() - 1) as f32;
    let mut visible = Vec::with_capacity(layers.len());
    let mut positions = Vec::with_capacity(layers.len());

    for (layer, &count) in layers.iter().enumerate() {
        let slots = display_slots(count);
        let gap = if slots.len() <= 1 {
            0.0
        } else {
            height / (slots.len() - 1) as f32
        };
        let x = x0 + layer as f32 * dx;
        let top = center.y - height * 0.5;
        let mut layer_positions = Vec::new();
        let mut layer_visible = Vec::new();
        for (slot, unit) in slots.into_iter().enumerate() {
            let pos = Vec2::new(
                x,
                if gap == 0.0 {
                    center.y
                } else {
                    top + slot as f32 * gap
                },
            );
            match unit {
                Some(unit) => {
                    let nid = node_id(&id, layer, unit);
                    let mut node =
                        Entity::new(nid.clone(), Shape::Circle { r: radius }, pos, style::PANEL);
                    node.stroke = StrokeStyle {
                        fill: true,
                        outline: true,
                        width: 2.0,
                        outline_color: Some(if layer == 0 {
                            style::CYAN
                        } else if layer + 1 == layers.len() {
                            style::GOLD
                        } else {
                            style::LIME
                        }),
                    };
                    node.glow = 0.25;
                    node.z = 5;
                    add_tag(&mut node, &id, "nodes");
                    node.tags.push(format!("{id}.layer{layer}"));
                    if layer == 0 {
                        node.tags.push(format!("{id}.input"));
                    } else if layer + 1 == layers.len() {
                        node.tags.push(format!("{id}.output"));
                    } else {
                        node.tags.push(format!("{id}.hidden"));
                    }
                    scene.add(node);

                    let initial = if layer == 0 {
                        format!("x{}", unit + 1)
                    } else if layer + 1 == layers.len() {
                        format!("y{}", unit + 1)
                    } else {
                        format!("h{}.{}", layer, unit + 1)
                    };
                    let mut value = Entity::new(
                        format!("{nid}.value"),
                        Shape::Text {
                            content: initial,
                            size: (radius * 0.68).clamp(13.0, 19.0),
                        },
                        Vec2::ZERO,
                        style::FG,
                    );
                    value.follow = Some((nid.clone(), Vec2::ZERO));
                    value.font = FontKind::MonoBold;
                    value.z = 7;
                    value.glow = 0.15;
                    add_tag(&mut value, &id, "values");
                    value.tags.push(format!("{id}.layer{layer}"));
                    scene.add(value);

                    layer_visible.push(unit);
                    layer_positions.push(pos);
                }
                None => {
                    let mut dots = Entity::new(
                        format!("{id}.l{layer}.more"),
                        Shape::Text {
                            content: "· · ·".into(),
                            size: 18.0,
                        },
                        pos,
                        style::DIM,
                    );
                    dots.rot = 90.0;
                    dots.z = 6;
                    add_tag(&mut dots, &id, "nodes");
                    dots.tags.push(format!("{id}.layer{layer}"));
                    scene.add(dots);
                }
            }
        }
        visible.push(layer_visible);
        positions.push(layer_positions);

        let role = if layer == 0 {
            "INPUT".to_string()
        } else if layer + 1 == layers.len() {
            "OUTPUT".to_string()
        } else {
            format!("HIDDEN {layer}")
        };
        let mut label = Entity::new(
            format!("{id}.layer{layer}.label"),
            Shape::Text {
                content: format!("{role}  ·  {count}"),
                size: 18.0,
            },
            Vec2::new(x, center.y - height * 0.5 - radius - 34.0),
            style::DIM,
        );
        label.font = FontKind::MonoBold;
        label.z = 8;
        add_tag(&mut label, &id, "labels");
        label.tags.push(format!("{id}.layer{layer}"));
        scene.add(label);
    }

    // Weighted visible connections. Full matrices remain in model state; only
    // a bounded deterministic subset is drawn for large layers.
    for transition in 0..layers.len() - 1 {
        let max_weight = weights[transition]
            .iter()
            .flatten()
            .map(|weight| weight.abs())
            .fold(0.0f32, f32::max)
            .max(1e-6);
        for (source_slot, &source) in visible[transition].iter().enumerate() {
            for (target_slot, &target) in visible[transition + 1].iter().enumerate() {
                let from = positions[transition][source_slot];
                let to = positions[transition + 1][target_slot];
                let weight = weights[transition][target][source];
                let magnitude = (weight.abs() / max_weight).clamp(0.0, 1.0);
                let mut edge = Entity::new(
                    edge_id(&id, transition, source, target),
                    Shape::Line { to },
                    from,
                    if weight >= 0.0 {
                        style::CYAN
                    } else {
                        style::MAGENTA
                    },
                );
                edge.opacity = 0.08 + magnitude * 0.16;
                edge.stroke.width = 0.8 + magnitude * 1.5;
                edge.z = 1;
                edge.glow = 0.15;
                edge.link = Some(Link {
                    from: node_id(&id, transition, source),
                    to: node_id(&id, transition + 1, target),
                    trim_from: radius,
                    trim_to: radius,
                    auto_trim: true,
                    bend: 0.0,
                });
                add_tag(&mut edge, &id, "edges");
                edge.tags.push(format!("{id}.transition{transition}"));
                scene.add(edge);
            }
        }
    }

    // A persistent status strip gives each beat a readable semantic meaning.
    let mut status = Entity::new(
        format!("{id}.status"),
        Shape::Text {
            content: "Ready · provide an input".into(),
            size: 20.0,
        },
        Vec2::new(center.x, center.y + height * 0.5 + radius + 42.0),
        style::DIM,
    );
    status.font = FontKind::MonoBold;
    status.z = 8;
    add_tag(&mut status, &id, "labels");
    scene.add(status);

    // Output bars begin at zero length. `forward` moves their endpoints to the
    // computed probabilities/scores and updates the readouts.
    let output_layer = layers.len() - 1;
    let output_x = x0 + output_layer as f32 * dx;
    let right_room = scene.canvas_size.x - (output_x + radius);
    let left_room = output_x - radius;
    let bar_direction = if right_room >= 185.0 || right_room >= left_room {
        1.0
    } else {
        -1.0
    };
    for (slot, &unit) in visible[output_layer].iter().enumerate() {
        let node_pos = positions[output_layer][slot];
        let start = node_pos + Vec2::new(bar_direction * (radius + 12.0), 0.0);
        let mut bar = Entity::new(
            format!("{id}.out{unit}.bar"),
            Shape::Line { to: start },
            start,
            style::GOLD,
        );
        bar.stroke.width = 7.0;
        bar.glow = 0.35;
        bar.opacity = 0.9;
        bar.z = 3;
        add_tag(&mut bar, &id, "probabilities");
        bar.tags.push(format!("{id}.output"));
        scene.add(bar);

        let mut readout = Entity::new(
            format!("{id}.out{unit}.readout"),
            Shape::Text {
                content: "—".into(),
                size: 17.0,
            },
            start + Vec2::new(bar_direction * 70.0, -17.0),
            style::GOLD,
        );
        readout.font = FontKind::MonoBold;
        readout.z = 8;
        add_tag(&mut readout, &id, "probabilities");
        readout.tags.push(format!("{id}.output"));
        scene.add(readout);

        let mut target = Entity::new(
            format!("{id}.out{unit}.target"),
            Shape::Text {
                content: "target —".into(),
                size: 15.0,
            },
            start + Vec2::new(bar_direction * 70.0, 17.0),
            style::MAGENTA,
        );
        target.font = FontKind::MonoBold;
        target.opacity = 0.0;
        target.z = 8;
        add_tag(&mut target, &id, "loss");
        target.tags.push(format!("{id}.output"));
        scene.add(target);
    }

    scene.ml_networks.insert(
        id,
        MlNetworkData {
            layers,
            activations,
            weights,
            biases,
            visible,
            positions,
            radius,
            bar_direction,
            last_values: None,
            last_loss: None,
            last_gradients: None,
            last_update: None,
        },
    );
    Ok(())
}

/// `checkpoint(id, network)`
///
/// Capture the parameters plus the latest prediction/target/loss. Active
/// gradient badges are deliberately not part of the saved state: after a
/// restore, a creator must call `backward` again before another `update`.
fn v_checkpoint(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(2)?;
    let checkpoint = args.ident(0)?;
    let network = args.ident(1)?;
    if scene.ml_network_checkpoints.contains_key(&checkpoint) {
        return Err(Error::new(
            format!("ML checkpoint `{checkpoint}` already exists"),
            args.span_of(0),
        ));
    }
    let mut state = scene.ml_networks.get(&network).cloned().ok_or_else(|| {
        Error::new(
            format!("`{network}` is not an ML network to checkpoint"),
            args.span_of(1),
        )
    })?;
    if state.last_values.is_none() || state.last_loss.is_none() {
        return Err(Error::new(
            format!(
                "checkpoint needs forward({network}, ...) then loss({network}, ...) so its prediction and comparison are exact"
            ),
            args.span_of(1),
        ));
    }
    state.last_gradients = None;
    state.last_update = None;
    scene
        .ml_network_checkpoints
        .insert(checkpoint, MlNetworkCheckpointData { network, state });
    Ok(Clip {
        tracks: Vec::new(),
        events: Vec::new(),
        dur: 0.0,
    })
}

fn curve_range(activation: Activation) -> (f32, f32, f32, f32) {
    match activation {
        Activation::Linear => (-4.0, 4.0, -4.0, 4.0),
        Activation::Relu => (-4.0, 4.0, -0.5, 4.0),
        Activation::Sigmoid => (-6.0, 6.0, -0.1, 1.1),
        Activation::Tanh => (-4.0, 4.0, -1.2, 1.2),
        Activation::Softmax => (-1.0, 1.0, 0.0, 1.0),
    }
}

/// `activation(id, (cx,cy), relu, [width], [height])`
fn c_activation(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(5)?;
    let id = args.ident(0)?;
    let center = args.pair(1)?;
    let word = args.ident(2)?;
    let activation = Activation::parse(&word).ok_or_else(|| {
        Error::new(
            format!("unknown activation `{word}` (try: linear, relu, sigmoid, tanh)"),
            args.span_of(2),
        )
    })?;
    if activation == Activation::Softmax {
        return Err(Error::new(
            "softmax is a vector activation; show it through a network output",
            args.span_of(2),
        ));
    }
    let width = args.opt_num(3)?.unwrap_or(360.0);
    let height = args.opt_num(4)?.unwrap_or(220.0);
    if width < 120.0 || height < 100.0 {
        return Err(Error::new(
            "activation plot width must be at least 120 and height at least 100",
            args.span_of(if width < 120.0 { 3 } else { 4 }),
        ));
    }
    let (xmin, xmax, ymin, ymax) = curve_range(activation);
    let map = |x: f32, y: f32| {
        Vec2::new(
            center.x + (x - (xmin + xmax) * 0.5) / (xmax - xmin) * width,
            center.y - (y - (ymin + ymax) * 0.5) / (ymax - ymin) * height,
        )
    };
    let axis_color = style::DIM;
    let mut x_axis = Entity::new(
        format!("{id}.xaxis"),
        Shape::Line { to: map(xmax, 0.0) },
        map(xmin, 0.0),
        axis_color,
    );
    x_axis.stroke.width = 1.5;
    x_axis.glow = 0.0;
    x_axis.tags.extend([id.clone(), format!("{id}.axes")]);
    scene.add(x_axis);

    let mut y_axis = Entity::new(
        format!("{id}.yaxis"),
        Shape::Line { to: map(0.0, ymax) },
        map(0.0, ymin),
        axis_color,
    );
    y_axis.stroke.width = 1.5;
    y_axis.glow = 0.0;
    y_axis.tags.extend([id.clone(), format!("{id}.axes")]);
    scene.add(y_axis);

    let points = (0..=120)
        .map(|sample| {
            let x = xmin + (xmax - xmin) * sample as f32 / 120.0;
            map(x, activation.scalar(x))
        })
        .collect();
    let mut curve = Entity::new(
        format!("{id}.curve"),
        Shape::Polyline { pts: points },
        Vec2::ZERO,
        style::LIME,
    );
    curve.stroke.width = 4.0;
    curve.glow = 0.4;
    curve.z = 3;
    curve.tags.extend([id.clone(), format!("{id}.curve")]);
    scene.add(curve);

    let mut label = Entity::new(
        format!("{id}.label"),
        Shape::Text {
            content: activation.name().into(),
            size: 22.0,
        },
        Vec2::new(center.x, center.y - height * 0.5 - 30.0),
        style::FG,
    );
    label.font = FontKind::MonoBold;
    label.z = 5;
    label.tags.extend([id, format!("{word}.label")]);
    scene.add(label);
    Ok(())
}

fn track(
    id: String,
    prop: Prop,
    target: TargetValue,
    start: f32,
    dur: f32,
    easing: Easing,
) -> TrackSpec {
    TrackSpec {
        id,
        prop,
        target,
        start,
        dur,
        easing,
    }
}

fn layer_color(layer: usize, total: usize) -> Color {
    if layer == 0 {
        style::CYAN
    } else if layer + 1 == total {
        style::GOLD
    } else {
        style::LIME
    }
}

fn gradient_color(gradient: f32) -> Color {
    if gradient >= 0.0 {
        style::MAGENTA
    } else {
        style::CYAN
    }
}

fn weight_color(weight: f32) -> Color {
    if weight >= 0.0 {
        style::CYAN
    } else {
        style::MAGENTA
    }
}

fn gradient_badge_text(gradient: f32) -> String {
    format!("∇{gradient:+.2}")
}

fn ensure_gradient_badge(
    scene: &mut Scene,
    root: &str,
    layer: usize,
    unit: usize,
    pos: Vec2,
    radius: f32,
    gradient: f32,
) -> (String, String) {
    let badge_id = format!("{root}.l{layer}.n{unit}.gradient.badge");
    let value_id = format!("{root}.l{layer}.n{unit}.gradient.value");
    if scene.get(&badge_id).is_none() {
        let color = gradient_color(gradient);
        let center = pos + Vec2::new(0.0, radius + 13.0);
        let mut badge = Entity::new(
            badge_id.clone(),
            Shape::Rect {
                w: (radius * 2.15).clamp(38.0, 58.0),
                h: (radius * 0.70).clamp(14.0, 19.0),
            },
            center,
            mix(style::PANEL, color, 0.24),
        );
        badge.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 1.0,
            outline_color: Some(mix(style::DIM, color, 0.65)),
        };
        badge.opacity = 0.0;
        badge.z = 8;
        add_tag(&mut badge, root, "gradients");
        badge.tags.push(format!("{root}.layer{layer}"));
        scene.add(badge);

        let mut value = Entity::new(
            value_id.clone(),
            Shape::Text {
                content: gradient_badge_text(gradient),
                size: (radius * 0.42).clamp(9.0, 12.0),
            },
            center,
            color,
        );
        value.font = FontKind::MonoBold;
        value.opacity = 0.0;
        value.z = 9;
        add_tag(&mut value, root, "gradients");
        value.tags.push(format!("{root}.layer{layer}"));
        scene.add(value);
    }
    (badge_id, value_id)
}

fn positive_duration(args: &Args, index: usize, fallback: f32, action: &str) -> Result<f32, Error> {
    let duration = args.opt_num(index)?.unwrap_or(fallback);
    if !duration.is_finite() || duration <= 0.0 {
        return Err(Error::new(
            format!("{action} duration must be a positive finite number"),
            args.span_of(index),
        ));
    }
    Ok(duration)
}

fn optional_easing(args: &Args, index: usize) -> Result<Easing, Error> {
    if args.len() > index {
        let word = args.ident(index)?;
        resolve_easing(&word, args.span_of(index))
    } else {
        Ok(Easing::InOutCubic)
    }
}

/// `forward(network, "v1 v2 ...", [duration], [ease])`
fn v_forward(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(4)?;
    let id = args.ident(0)?;
    let input =
        parse_values(&args.text(1)?).map_err(|message| Error::new(message, args.span_of(1)))?;
    let duration = args.opt_num(2)?.unwrap_or(3.2);
    if !duration.is_finite() || duration <= 0.0 {
        return Err(Error::new(
            "forward duration must be a positive finite number",
            args.span_of(2),
        ));
    }
    let easing = if args.len() > 3 {
        let word = args.ident(3)?;
        resolve_easing(&word, args.span_of(3))?
    } else {
        Easing::InOutCubic
    };
    let mut model = scene
        .ml_networks
        .get(&id)
        .cloned()
        .ok_or_else(|| Error::new(format!("`{id}` is not an ML network"), args.span_of(0)))?;
    let values =
        forward_values(&model, &input).map_err(|message| Error::new(message, args.span_of(1)))?;

    let mut tracks = Vec::new();
    let mut events = Vec::new();
    let input_beat = duration * 0.16;
    let transition_beat = (duration - input_beat) / (model.layers.len() - 1) as f32;
    let output_layer = model.layers.len() - 1;

    // A new input begins a fresh supervised beat. Any target shown by a prior
    // `loss` belongs to the old prediction and fades before values propagate.
    for &unit in &model.visible[output_layer] {
        tracks.push(track(
            format!("{id}.out{unit}.target"),
            Prop::Opacity,
            TargetValue::Abs(Value::F(0.0)),
            0.0,
            input_beat * 0.45,
            Easing::InOutCubic,
        ));
    }

    // Input values arrive together and remain readable for the rest of the pass.
    let input_max = input
        .iter()
        .map(|value| value.abs())
        .fold(0.0f32, f32::max)
        .max(1e-6);
    for &unit in &model.visible[0] {
        let nid = node_id(&id, 0, unit);
        events.push(TextEvent::text(
            format!("{nid}.value"),
            fmt_value(input[unit]),
            input_beat * 0.15,
        ));
        tracks.push(track(
            nid.clone(),
            Prop::Color,
            TargetValue::Abs(Value::C(mix(
                style::PANEL,
                style::CYAN,
                0.25 + 0.55 * (input[unit].abs() / input_max),
            ))),
            0.0,
            input_beat * 0.75,
            easing,
        ));
        tracks.push(track(
            nid.clone(),
            Prop::Scale,
            TargetValue::Abs(Value::F(1.12)),
            0.0,
            input_beat * 0.4,
            Easing::OutQuad,
        ));
        tracks.push(track(
            nid,
            Prop::Scale,
            TargetValue::Abs(Value::F(1.0)),
            input_beat * 0.4,
            input_beat * 0.35,
            Easing::InOutCubic,
        ));
    }
    events.push(TextEvent::text(
        format!("{id}.status"),
        format!("Input · {} features", model.layers[0]),
        0.0,
    ));

    for transition in 0..model.layers.len() - 1 {
        let start = input_beat + transition as f32 * transition_beat;
        let arrive = start + transition_beat * 0.68;
        let source_values = &values[transition];
        let target_values = &values[transition + 1];
        let target_max = target_values
            .iter()
            .map(|value| value.abs())
            .fold(0.0f32, f32::max)
            .max(1e-6);

        events.push(TextEvent::text(
            format!("{id}.status"),
            format!(
                "Layer {} · {}",
                transition + 1,
                model.activations[transition].name()
            ),
            start,
        ));

        let mut max_contribution = 0.0f32;
        for &source in &model.visible[transition] {
            for &target in &model.visible[transition + 1] {
                max_contribution = max_contribution
                    .max((model.weights[transition][target][source] * source_values[source]).abs());
            }
        }
        max_contribution = max_contribution.max(1e-6);

        for &source in &model.visible[transition] {
            for &target in &model.visible[transition + 1] {
                let eid = edge_id(&id, transition, source, target);
                let contribution =
                    (model.weights[transition][target][source] * source_values[source]).abs();
                let active_opacity = 0.28 + 0.72 * (contribution / max_contribution).sqrt();
                let resting = scene.get(&eid).map(|edge| edge.opacity).unwrap_or(0.12);
                let settled = resting.max(0.10 + 0.24 * (contribution / max_contribution).sqrt());
                tracks.push(track(
                    eid.clone(),
                    Prop::Opacity,
                    TargetValue::Abs(Value::F(active_opacity)),
                    start,
                    transition_beat * 0.18,
                    Easing::OutQuad,
                ));
                tracks.push(track(
                    eid.clone(),
                    Prop::Flow,
                    TargetValue::Rel(Value::F(1.0)),
                    start + transition_beat * 0.06,
                    transition_beat * 0.62,
                    easing,
                ));
                tracks.push(track(
                    eid,
                    Prop::Opacity,
                    TargetValue::Abs(Value::F(settled)),
                    arrive,
                    transition_beat * 0.25,
                    Easing::InOutCubic,
                ));
            }
        }

        for &unit in &model.visible[transition + 1] {
            let nid = node_id(&id, transition + 1, unit);
            let magnitude = (target_values[unit].abs() / target_max).clamp(0.0, 1.0);
            events.push(TextEvent::text(
                format!("{nid}.value"),
                fmt_value(target_values[unit]),
                arrive,
            ));
            tracks.push(track(
                nid.clone(),
                Prop::Color,
                TargetValue::Abs(Value::C(mix(
                    style::PANEL,
                    layer_color(transition + 1, model.layers.len()),
                    0.22 + 0.62 * magnitude,
                ))),
                arrive - transition_beat * 0.12,
                transition_beat * 0.28,
                easing,
            ));
            tracks.push(track(
                nid.clone(),
                Prop::Scale,
                TargetValue::Abs(Value::F(1.13)),
                arrive - transition_beat * 0.08,
                transition_beat * 0.16,
                Easing::OutQuad,
            ));
            tracks.push(track(
                nid,
                Prop::Scale,
                TargetValue::Abs(Value::F(1.0)),
                arrive + transition_beat * 0.08,
                transition_beat * 0.18,
                Easing::InOutCubic,
            ));
        }
    }

    let output = values.last().expect("network has an output layer");
    let is_probability = model.activations.last() == Some(&Activation::Softmax);
    let output_scale = if is_probability {
        1.0
    } else {
        output
            .iter()
            .map(|value| value.abs())
            .fold(0.0f32, f32::max)
            .max(1.0)
    };
    let bar_len = (scene.canvas_size.x * 0.11).clamp(80.0, 150.0);
    let bar_start = duration * 0.78;
    for (slot, &unit) in model.visible[output_layer].iter().enumerate() {
        let start = model.positions[output_layer][slot]
            + Vec2::new(model.bar_direction * (model.radius + 12.0), 0.0);
        let amount = if is_probability {
            output[unit].clamp(0.0, 1.0)
        } else {
            (output[unit].abs() / output_scale).clamp(0.0, 1.0)
        };
        tracks.push(track(
            format!("{id}.out{unit}.bar"),
            Prop::To,
            TargetValue::Abs(Value::V(
                start + Vec2::new(model.bar_direction * bar_len * amount, 0.0),
            )),
            bar_start,
            duration - bar_start,
            easing,
        ));
        events.push(TextEvent::text(
            format!("{id}.out{unit}.readout"),
            if is_probability {
                format!("{:.1}%", output[unit] * 100.0)
            } else {
                fmt_value(output[unit])
            },
            bar_start,
        ));
    }
    let prediction = output
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(index, value)| (index, *value))
        .unwrap_or((0, 0.0));
    events.push(TextEvent::text(
        format!("{id}.status"),
        if is_probability {
            format!(
                "Prediction · class {} · {:.1}%",
                prediction.0 + 1,
                prediction.1 * 100.0
            )
        } else {
            format!("Output · class {} has the largest score", prediction.0 + 1)
        },
        duration * 0.96,
    ));

    model.last_values = Some(values);
    model.last_loss = None;
    model.last_gradients = None;
    model.last_update = None;
    scene.ml_networks.insert(id, model);
    Ok(Clip {
        tracks,
        events,
        dur: duration,
    })
}

/// `loss(network, "target values", [crossentropy|mse], [duration], [ease])`
fn v_loss(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(5)?;
    let id = args.ident(0)?;
    let target =
        parse_target(&args.text(1)?).map_err(|message| Error::new(message, args.span_of(1)))?;
    let mut model = scene
        .ml_networks
        .get(&id)
        .cloned()
        .ok_or_else(|| Error::new(format!("`{id}` is not an ML network"), args.span_of(0)))?;
    let values = model.last_values.clone().ok_or_else(|| {
        Error::new(
            format!("loss needs a completed forward pass for `{id}`"),
            args.span_of(0),
        )
    })?;
    let kind = if args.len() > 2 {
        let word = args.ident(2)?;
        LossKind::parse(&word).ok_or_else(|| {
            Error::new(
                format!("unknown loss `{word}` (try: crossentropy or mse)"),
                args.span_of(2),
            )
        })?
    } else if model.activations.last() == Some(&Activation::Softmax) {
        LossKind::CrossEntropy
    } else {
        LossKind::Mse
    };
    if kind == LossKind::CrossEntropy && model.activations.last() != Some(&Activation::Softmax) {
        return Err(Error::new(
            "crossentropy currently requires a softmax output layer; use mse for other outputs",
            args.span_of(2),
        ));
    }
    let duration = positive_duration(args, 3, 1.6, "loss")?;
    let easing = optional_easing(args, 4)?;
    let output = values.last().expect("forward values contain an output");
    let (value, _) = loss_value_and_output_gradient(kind, output, &target)
        .map_err(|message| Error::new(message, args.span_of(1)))?;

    let output_layer = model.layers.len() - 1;
    let mut tracks = Vec::new();
    let mut events = vec![TextEvent::text(
        format!("{id}.status"),
        format!("{} · compare prediction with target", kind.name()),
        0.0,
    )];
    let max_error = output
        .iter()
        .zip(&target)
        .map(|(&prediction, &wanted)| (prediction - wanted).abs())
        .fold(0.0f32, f32::max)
        .max(1e-6);
    for &unit in &model.visible[output_layer] {
        let target_id = format!("{id}.out{unit}.target");
        events.push(TextEvent::text(
            target_id.clone(),
            format!("target {}", fmt_value(target[unit])),
            duration * 0.12,
        ));
        tracks.push(track(
            target_id,
            Prop::Opacity,
            TargetValue::Abs(Value::F(1.0)),
            duration * 0.08,
            duration * 0.30,
            easing,
        ));
        let node = node_id(&id, output_layer, unit);
        let error = (output[unit] - target[unit]).abs() / max_error;
        tracks.push(track(
            node.clone(),
            Prop::Color,
            TargetValue::Abs(Value::C(mix(
                style::PANEL,
                style::MAGENTA,
                0.3 + 0.6 * error,
            ))),
            duration * 0.12,
            duration * 0.32,
            easing,
        ));
        tracks.push(track(
            node.clone(),
            Prop::Scale,
            TargetValue::Abs(Value::F(1.13)),
            duration * 0.15,
            duration * 0.20,
            Easing::OutQuad,
        ));
        tracks.push(track(
            node,
            Prop::Scale,
            TargetValue::Abs(Value::F(1.0)),
            duration * 0.35,
            duration * 0.20,
            easing,
        ));
    }
    events.push(TextEvent::text(
        format!("{id}.status"),
        format!("{} = {:.5}", kind.name(), value),
        duration * 0.68,
    ));

    model.last_loss = Some(MlLossData {
        kind,
        target,
        value,
    });
    model.last_gradients = None;
    model.last_update = None;
    scene.ml_networks.insert(id, model);
    Ok(Clip {
        tracks,
        events,
        dur: duration,
    })
}

/// `backward(network, [duration], [ease])`
fn v_backward(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(3)?;
    let id = args.ident(0)?;
    let duration = positive_duration(args, 1, 3.2, "backward")?;
    let easing = optional_easing(args, 2)?;
    let mut model = scene
        .ml_networks
        .get(&id)
        .cloned()
        .ok_or_else(|| Error::new(format!("`{id}` is not an ML network"), args.span_of(0)))?;
    let values = model.last_values.clone().ok_or_else(|| {
        Error::new(
            format!("backward needs a completed forward pass for `{id}`"),
            args.span_of(0),
        )
    })?;
    let loss = model.last_loss.clone().ok_or_else(|| {
        Error::new(
            format!("backward needs loss({id}, ...) before gradients can flow"),
            args.span_of(0),
        )
    })?;
    let gradients = backward_values(&model, &values, &loss)
        .map_err(|message| Error::new(message, args.span_of(0)))?;
    let transitions = model.weights.len();
    let beat = duration / transitions as f32;
    let mut tracks = Vec::new();
    let mut events = vec![TextEvent::text(
        format!("{id}.status"),
        "Backward · output error starts the gradient".into(),
        0.0,
    )];

    for reverse_step in 0..transitions {
        let transition = transitions - 1 - reverse_step;
        let start = reverse_step as f32 * beat;
        let arrive = start + beat * 0.68;
        events.push(TextEvent::text(
            format!("{id}.status"),
            format!("Backward · layer {} · ∂L/∂W", transition + 1),
            start,
        ));
        let mut max_gradient = 0.0f32;
        for &source in &model.visible[transition] {
            for &target in &model.visible[transition + 1] {
                max_gradient =
                    max_gradient.max(gradients.weights[transition][target][source].abs());
            }
        }
        max_gradient = max_gradient.max(1e-9);
        for &source in &model.visible[transition] {
            for &target in &model.visible[transition + 1] {
                let gradient = gradients.weights[transition][target][source];
                let emphasis = (gradient.abs() / max_gradient).sqrt();
                let edge = edge_id(&id, transition, source, target);
                tracks.push(track(
                    edge.clone(),
                    Prop::Color,
                    TargetValue::Abs(Value::C(gradient_color(gradient))),
                    start,
                    beat * 0.18,
                    Easing::OutQuad,
                ));
                tracks.push(track(
                    edge.clone(),
                    Prop::Opacity,
                    TargetValue::Abs(Value::F(0.24 + 0.76 * emphasis)),
                    start,
                    beat * 0.18,
                    Easing::OutQuad,
                ));
                tracks.push(track(
                    edge.clone(),
                    Prop::Flow,
                    TargetValue::Rel(Value::F(-1.0)),
                    start + beat * 0.05,
                    beat * 0.62,
                    easing,
                ));
                tracks.push(track(
                    edge.clone(),
                    Prop::Color,
                    TargetValue::Abs(Value::C(weight_color(
                        model.weights[transition][target][source],
                    ))),
                    arrive,
                    beat * 0.24,
                    easing,
                ));
                let resting = scene
                    .get(&edge)
                    .map(|entity| entity.opacity)
                    .unwrap_or(0.12);
                tracks.push(track(
                    edge,
                    Prop::Opacity,
                    TargetValue::Abs(Value::F(resting)),
                    arrive,
                    beat * 0.24,
                    easing,
                ));
            }
        }

        if reverse_step == 0 {
            for (slot, &unit) in model.visible[transition + 1].iter().enumerate() {
                let gradient = gradients.node[transition + 1][unit];
                let (badge, value) = ensure_gradient_badge(
                    scene,
                    &id,
                    transition + 1,
                    unit,
                    model.positions[transition + 1][slot],
                    model.radius,
                    gradient,
                );
                events.push(TextEvent::text(
                    value.clone(),
                    gradient_badge_text(gradient),
                    start + beat * 0.08,
                ));
                for target in [badge, value] {
                    tracks.push(track(
                        target,
                        Prop::Opacity,
                        TargetValue::Abs(Value::F(1.0)),
                        start + beat * 0.06,
                        beat * 0.20,
                        Easing::OutQuad,
                    ));
                }
            }
        }
        for (slot, &unit) in model.visible[transition].iter().enumerate() {
            let node = node_id(&id, transition, unit);
            let gradient = gradients.node[transition][unit];
            let (badge, value) = ensure_gradient_badge(
                scene,
                &id,
                transition,
                unit,
                model.positions[transition][slot],
                model.radius,
                gradient,
            );
            events.push(TextEvent::text(
                value.clone(),
                gradient_badge_text(gradient),
                arrive,
            ));
            for target in [badge, value] {
                tracks.push(track(
                    target,
                    Prop::Opacity,
                    TargetValue::Abs(Value::F(1.0)),
                    arrive - beat * 0.08,
                    beat * 0.22,
                    Easing::OutQuad,
                ));
            }
            tracks.push(track(
                node.clone(),
                Prop::Color,
                TargetValue::Abs(Value::C(mix(style::PANEL, gradient_color(gradient), 0.66))),
                arrive - beat * 0.10,
                beat * 0.25,
                easing,
            ));
            tracks.push(track(
                node,
                Prop::Scale,
                TargetValue::Abs(Value::F(1.10)),
                arrive - beat * 0.08,
                beat * 0.20,
                Easing::OutQuad,
            ));
        }
    }
    events.push(TextEvent::text(
        format!("{id}.status"),
        format!("Gradients ready · ||∇θ|| = {:.5}", gradients.norm),
        duration * 0.94,
    ));
    model.last_gradients = Some(gradients);
    model.last_update = None;
    scene.ml_networks.insert(id, model);
    Ok(Clip {
        tracks,
        events,
        dur: duration,
    })
}

/// `update(network, [learning_rate], [duration], [ease])`
fn v_update(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(4)?;
    let id = args.ident(0)?;
    let learning_rate = args.opt_num(1)?.unwrap_or(0.15);
    if !learning_rate.is_finite() || learning_rate <= 0.0 {
        return Err(Error::new(
            "update learning rate must be a positive finite number",
            args.span_of(1),
        ));
    }
    let duration = positive_duration(args, 2, 2.4, "update")?;
    let easing = optional_easing(args, 3)?;
    let mut model = scene
        .ml_networks
        .get(&id)
        .cloned()
        .ok_or_else(|| Error::new(format!("`{id}` is not an ML network"), args.span_of(0)))?;
    let gradients = model.last_gradients.clone().ok_or_else(|| {
        Error::new(
            format!("update needs backward({id}, ...) before parameters can change"),
            args.span_of(0),
        )
    })?;
    let old_values = model.last_values.clone().ok_or_else(|| {
        Error::new(
            format!("update needs a completed forward pass for `{id}`"),
            args.span_of(0),
        )
    })?;
    let old_loss = model.last_loss.clone().ok_or_else(|| {
        Error::new(
            format!("update needs loss({id}, ...) before parameters can change"),
            args.span_of(0),
        )
    })?;
    let mut selected = (0usize, 0usize, 0usize);
    let mut selected_magnitude = -1.0f32;
    for transition in 0..gradients.weights.len() {
        for output in 0..gradients.weights[transition].len() {
            for input in 0..gradients.weights[transition][output].len() {
                let magnitude = gradients.weights[transition][output][input].abs();
                if magnitude > selected_magnitude {
                    selected = (transition, output, input);
                    selected_magnitude = magnitude;
                }
            }
        }
    }
    let selected_old = model.weights[selected.0][selected.1][selected.2];
    for transition in 0..model.weights.len() {
        for output in 0..model.weights[transition].len() {
            model.biases[transition][output] -=
                learning_rate * gradients.biases[transition][output];
            for input in 0..model.weights[transition][output].len() {
                model.weights[transition][output][input] -=
                    learning_rate * gradients.weights[transition][output][input];
            }
        }
    }
    let selected_new = model.weights[selected.0][selected.1][selected.2];
    let new_values = forward_values(&model, &old_values[0])
        .map_err(|message| Error::new(message, args.span_of(0)))?;
    let new_output = new_values.last().expect("updated network has output");
    let (new_loss_value, _) =
        loss_value_and_output_gradient(old_loss.kind, new_output, &old_loss.target)
            .map_err(|message| Error::new(message, args.span_of(0)))?;

    let mut tracks = Vec::new();
    let mut events = vec![TextEvent::text(
        format!("{id}.status"),
        format!("Update · θ ← θ − {:.3}∇θ", learning_rate),
        0.0,
    )];
    events.push(TextEvent::text(
        format!("{id}.status"),
        format!(
            "w{}.{}→{} · {:.5} → {:.5}",
            selected.0 + 1,
            selected.2 + 1,
            selected.1 + 1,
            selected_old,
            selected_new
        ),
        duration * 0.26,
    ));
    for entity in &scene.entities {
        if entity
            .tags
            .iter()
            .any(|tag| tag == &format!("{id}.gradients"))
        {
            tracks.push(track(
                entity.id.clone(),
                Prop::Opacity,
                TargetValue::Abs(Value::F(0.0)),
                0.0,
                duration * 0.16,
                Easing::InOutCubic,
            ));
        }
    }
    let max_gradient = gradients
        .weights
        .iter()
        .flatten()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0f32, f32::max)
        .max(1e-9);
    for transition in 0..model.weights.len() {
        for &source in &model.visible[transition] {
            for &target in &model.visible[transition + 1] {
                let gradient = gradients.weights[transition][target][source];
                let edge = edge_id(&id, transition, source, target);
                tracks.push(track(
                    edge.clone(),
                    Prop::Color,
                    TargetValue::Abs(Value::C(gradient_color(gradient))),
                    duration * 0.05,
                    duration * 0.22,
                    easing,
                ));
                tracks.push(track(
                    edge.clone(),
                    Prop::Opacity,
                    TargetValue::Abs(Value::F(
                        0.25 + 0.70 * (gradient.abs() / max_gradient).sqrt(),
                    )),
                    duration * 0.05,
                    duration * 0.22,
                    easing,
                ));
                tracks.push(track(
                    edge.clone(),
                    Prop::Color,
                    TargetValue::Abs(Value::C(weight_color(
                        model.weights[transition][target][source],
                    ))),
                    duration * 0.32,
                    duration * 0.25,
                    easing,
                ));
                let resting = scene
                    .get(&edge)
                    .map(|entity| entity.opacity)
                    .unwrap_or(0.12);
                tracks.push(track(
                    edge,
                    Prop::Opacity,
                    TargetValue::Abs(Value::F(resting)),
                    duration * 0.32,
                    duration * 0.25,
                    easing,
                ));
            }
        }
    }
    events.push(TextEvent::text(
        format!("{id}.status"),
        "Recompute · the updated model sees the same input".into(),
        duration * 0.48,
    ));
    for layer in 0..model.layers.len() {
        let max_value = new_values[layer]
            .iter()
            .map(|value| value.abs())
            .fold(0.0f32, f32::max)
            .max(1e-6);
        for &unit in &model.visible[layer] {
            let node = node_id(&id, layer, unit);
            events.push(TextEvent::text(
                format!("{node}.value"),
                fmt_value(new_values[layer][unit]),
                duration * 0.58,
            ));
            tracks.push(track(
                node,
                Prop::Color,
                TargetValue::Abs(Value::C(mix(
                    style::PANEL,
                    layer_color(layer, model.layers.len()),
                    0.22 + 0.62 * (new_values[layer][unit].abs() / max_value),
                ))),
                duration * 0.52,
                duration * 0.30,
                easing,
            ));
        }
    }

    let output_layer = model.layers.len() - 1;
    let is_probability = model.activations.last() == Some(&Activation::Softmax);
    let output_scale = if is_probability {
        1.0
    } else {
        new_output
            .iter()
            .map(|value| value.abs())
            .fold(0.0f32, f32::max)
            .max(1.0)
    };
    let bar_len = (scene.canvas_size.x * 0.11).clamp(80.0, 150.0);
    for (slot, &unit) in model.visible[output_layer].iter().enumerate() {
        let start = model.positions[output_layer][slot]
            + Vec2::new(model.bar_direction * (model.radius + 12.0), 0.0);
        let amount = if is_probability {
            new_output[unit].clamp(0.0, 1.0)
        } else {
            (new_output[unit].abs() / output_scale).clamp(0.0, 1.0)
        };
        tracks.push(track(
            format!("{id}.out{unit}.bar"),
            Prop::To,
            TargetValue::Abs(Value::V(
                start + Vec2::new(model.bar_direction * bar_len * amount, 0.0),
            )),
            duration * 0.58,
            duration * 0.30,
            easing,
        ));
        events.push(TextEvent::text(
            format!("{id}.out{unit}.readout"),
            if is_probability {
                format!("{:.1}%", new_output[unit] * 100.0)
            } else {
                fmt_value(new_output[unit])
            },
            duration * 0.58,
        ));
    }
    events.push(TextEvent::text(
        format!("{id}.status"),
        format!(
            "Loss · {:.5} → {:.5} · exact updated result",
            old_loss.value, new_loss_value
        ),
        duration * 0.91,
    ));

    model.last_values = Some(new_values);
    model.last_loss = Some(MlLossData {
        kind: old_loss.kind,
        target: old_loss.target,
        value: new_loss_value,
    });
    model.last_gradients = None;
    model.last_update = Some(MlUpdateData {
        learning_rate,
        old_loss: old_loss.value,
        new_loss: new_loss_value,
    });
    scene.ml_networks.insert(id, model);
    Ok(Clip {
        tracks,
        events,
        dur: duration,
    })
}

/// `restore(network, checkpoint, [duration], [ease])`
///
/// Restore every weight and bias together with the saved prediction and loss.
/// This is an exact rollback to an authored state, not dataset-level machine
/// unlearning. The animation is still a set of ordinary stateless tracks.
fn v_restore(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(4)?;
    let id = args.ident(0)?;
    let checkpoint_id = args.ident(1)?;
    let duration = positive_duration(args, 2, 2.3, "restore")?;
    let easing = optional_easing(args, 3)?;
    let current = scene
        .ml_networks
        .get(&id)
        .cloned()
        .ok_or_else(|| Error::new(format!("`{id}` is not an ML network"), args.span_of(0)))?;
    let checkpoint = scene
        .ml_network_checkpoints
        .get(&checkpoint_id)
        .cloned()
        .ok_or_else(|| {
            Error::new(
                format!("no ML checkpoint named `{checkpoint_id}`"),
                args.span_of(1),
            )
        })?;
    if checkpoint.network != id {
        return Err(Error::new(
            format!(
                "checkpoint `{checkpoint_id}` belongs to `{}`, not `{id}`",
                checkpoint.network
            ),
            args.span_of(1),
        ));
    }
    let mut restored = checkpoint.state;
    let values = restored
        .last_values
        .clone()
        .expect("checkpoint validates a completed forward pass");
    let saved_loss = restored
        .last_loss
        .clone()
        .expect("checkpoint validates a completed loss");
    let old_loss = current.last_loss.as_ref().map(|loss| loss.value);

    let mut selected = (0usize, 0usize, 0usize);
    let mut selected_delta = -1.0f32;
    for transition in 0..restored.weights.len() {
        for output in 0..restored.weights[transition].len() {
            for input in 0..restored.weights[transition][output].len() {
                let delta = (current.weights[transition][output][input]
                    - restored.weights[transition][output][input])
                    .abs();
                if delta > selected_delta {
                    selected = (transition, output, input);
                    selected_delta = delta;
                }
            }
        }
    }
    let selected_current = current.weights[selected.0][selected.1][selected.2];
    let selected_restored = restored.weights[selected.0][selected.1][selected.2];

    let mut tracks = Vec::new();
    let mut events = vec![TextEvent::text(
        format!("{id}.status"),
        format!("Rollback · restoring checkpoint `{checkpoint_id}`"),
        0.0,
    )];
    events.push(TextEvent::text(
        format!("{id}.status"),
        format!(
            "w{}.{}→{} · {:.5} → {:.5}",
            selected.0 + 1,
            selected.2 + 1,
            selected.1 + 1,
            selected_current,
            selected_restored
        ),
        duration * 0.22,
    ));

    // A backward-moving pulse makes the reversal legible, while the settled
    // colour/opacity is derived only from the checkpointed weights.
    for transition in 0..restored.weights.len() {
        let max_weight = restored.weights[transition]
            .iter()
            .flatten()
            .map(|weight| weight.abs())
            .fold(0.0f32, f32::max)
            .max(1e-6);
        for &source in &restored.visible[transition] {
            for &target in &restored.visible[transition + 1] {
                let edge = edge_id(&id, transition, source, target);
                let weight = restored.weights[transition][target][source];
                let opacity = 0.08 + 0.16 * (weight.abs() / max_weight).clamp(0.0, 1.0);
                tracks.push(track(
                    edge.clone(),
                    Prop::Color,
                    TargetValue::Abs(Value::C(style::GOLD)),
                    duration * 0.04,
                    duration * 0.18,
                    Easing::OutQuad,
                ));
                tracks.push(track(
                    edge.clone(),
                    Prop::Flow,
                    TargetValue::Rel(Value::F(-1.0)),
                    duration * 0.05,
                    duration * 0.56,
                    easing,
                ));
                tracks.push(track(
                    edge.clone(),
                    Prop::Color,
                    TargetValue::Abs(Value::C(weight_color(weight))),
                    duration * 0.38,
                    duration * 0.28,
                    easing,
                ));
                tracks.push(track(
                    edge,
                    Prop::Opacity,
                    TargetValue::Abs(Value::F(opacity)),
                    duration * 0.38,
                    duration * 0.28,
                    easing,
                ));
            }
        }
    }

    events.push(TextEvent::text(
        format!("{id}.status"),
        "Recompute · restore the saved prediction from exact parameters".into(),
        duration * 0.48,
    ));
    for layer in 0..restored.layers.len() {
        let max_value = values[layer]
            .iter()
            .map(|value| value.abs())
            .fold(0.0f32, f32::max)
            .max(1e-6);
        for &unit in &restored.visible[layer] {
            let node = node_id(&id, layer, unit);
            events.push(TextEvent::text(
                format!("{node}.value"),
                fmt_value(values[layer][unit]),
                duration * 0.58,
            ));
            tracks.push(track(
                node,
                Prop::Color,
                TargetValue::Abs(Value::C(mix(
                    style::PANEL,
                    layer_color(layer, restored.layers.len()),
                    0.22 + 0.62 * (values[layer][unit].abs() / max_value),
                ))),
                duration * 0.52,
                duration * 0.30,
                easing,
            ));
        }
    }

    let output_layer = restored.layers.len() - 1;
    let output = values.last().expect("restored network has output values");
    let is_probability = restored.activations.last() == Some(&Activation::Softmax);
    let output_scale = if is_probability {
        1.0
    } else {
        output
            .iter()
            .map(|value| value.abs())
            .fold(0.0f32, f32::max)
            .max(1.0)
    };
    let bar_len = (scene.canvas_size.x * 0.11).clamp(80.0, 150.0);
    for (slot, &unit) in restored.visible[output_layer].iter().enumerate() {
        let start = restored.positions[output_layer][slot]
            + Vec2::new(restored.bar_direction * (restored.radius + 12.0), 0.0);
        let amount = if is_probability {
            output[unit].clamp(0.0, 1.0)
        } else {
            (output[unit].abs() / output_scale).clamp(0.0, 1.0)
        };
        tracks.push(track(
            format!("{id}.out{unit}.bar"),
            Prop::To,
            TargetValue::Abs(Value::V(
                start + Vec2::new(restored.bar_direction * bar_len * amount, 0.0),
            )),
            duration * 0.56,
            duration * 0.30,
            easing,
        ));
        events.push(TextEvent::text(
            format!("{id}.out{unit}.readout"),
            if is_probability {
                format!("{:.1}%", output[unit] * 100.0)
            } else {
                fmt_value(output[unit])
            },
            duration * 0.56,
        ));
        events.push(TextEvent::text(
            format!("{id}.out{unit}.target"),
            format!("target {}", fmt_value(saved_loss.target[unit])),
            duration * 0.56,
        ));
        tracks.push(track(
            format!("{id}.out{unit}.target"),
            Prop::Opacity,
            TargetValue::Abs(Value::F(1.0)),
            duration * 0.54,
            duration * 0.20,
            Easing::InOutCubic,
        ));
    }
    events.push(TextEvent::text(
        format!("{id}.status"),
        match old_loss {
            Some(previous) => format!(
                "Rollback complete · loss {:.5} → {:.5} · exact saved state",
                previous, saved_loss.value
            ),
            None => format!(
                "Rollback complete · loss {:.5} restored · exact saved state",
                saved_loss.value
            ),
        },
        duration * 0.91,
    ));

    restored.last_gradients = None;
    restored.last_update = None;
    scene.ml_networks.insert(id, restored);
    Ok(Clip {
        tracks,
        events,
        dur: duration,
    })
}

pub fn register(registry: &mut Registry) {
    registry.ctor("network", c_network);
    registry.ctor("activation", c_activation);
    registry.mut_verb("forward", v_forward);
    registry.mut_verb("loss", v_loss);
    registry.mut_verb("backward", v_backward);
    registry.mut_verb("checkpoint", v_checkpoint);
    registry.mut_verb("update", v_update);
    registry.mut_verb("restore", v_restore);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movie::Movie;

    fn movie(src: &str) -> Movie {
        crate::parse(src).unwrap_or_else(|error| panic!("parse failed: {error:?}"))
    }

    fn frame(movie: &Movie, t: f32) -> Scene {
        let (base, timeline) = movie.finalize();
        timeline.apply(&base, t)
    }

    fn text(scene: &Scene, id: &str) -> String {
        match &scene.get(id).expect("text entity exists").shape {
            Shape::Text { content, .. } => content.clone(),
            _ => panic!("{id} is not text"),
        }
    }

    #[test]
    fn stable_softmax_is_finite_and_normalized_for_large_logits() {
        let mut values = vec![10_000.0, 10_001.0, 9_999.0];
        stable_softmax(&mut values);
        assert!(values.iter().all(|value| value.is_finite()));
        assert!((values.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert_eq!(
            values
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(index, _)| index),
            Some(1)
        );
    }

    #[test]
    fn generated_network_is_deterministic_and_dimensionally_correct() {
        let layers = vec![3, 4, 2];
        let (wa, ba) = generated_parameters(&layers, 42);
        let (wb, bb) = generated_parameters(&layers, 42);
        assert_eq!(wa, wb);
        assert_eq!(ba, bb);
        assert_eq!(wa.len(), 2);
        assert_eq!(wa[0].len(), 4);
        assert_eq!(wa[0][0].len(), 3);
        assert_eq!(wa[1].len(), 2);
        assert_eq!(wa[1][0].len(), 4);
    }

    #[test]
    fn forward_computes_real_softmax_values() {
        let m = movie(
            "network(net,(640,340),\"3 4 2\",\"relu softmax\",640,360,11); forward(net,\"0.2 0.8 0.4\",2,smooth);",
        );
        let values = m.scene.ml_networks["net"]
            .last_values
            .as_ref()
            .expect("forward stores values");
        assert_eq!(
            values.iter().map(Vec::len).collect::<Vec<_>>(),
            vec![3, 4, 2]
        );
        assert!((values[2].iter().sum::<f32>() - 1.0).abs() < 1e-5);

        let settled = frame(&m, 100.0);
        assert!(text(&settled, "net.status").starts_with("Prediction"));
        assert!(text(&settled, "net.out0.readout").ends_with('%'));
        assert!(settled.get("net.out0.bar").is_some());
        assert!(m.scene.entities.iter().any(|base| {
            base.tags.iter().any(|tag| tag == "net.edges")
                && settled
                    .get(&base.id)
                    .is_some_and(|edge| edge.opacity > base.opacity + 0.01)
        }));
    }

    #[test]
    fn forward_is_stateless_when_seeking_out_of_order() {
        let m = movie(
            "network(net,(640,340),\"2 3 2\",\"tanh softmax\",620,340,5); forward(net,\"1 -1\",2.4,smooth);",
        );
        let late_first = frame(&m, 2.2);
        let early = frame(&m, 0.2);
        let late_again = frame(&m, 2.2);
        assert_eq!(
            text(&late_first, "net.status"),
            text(&late_again, "net.status")
        );
        assert_eq!(
            late_first.get("net.out0.bar").unwrap().shape,
            late_again.get("net.out0.bar").unwrap().shape
        );
        assert_ne!(text(&early, "net.status"), text(&late_again, "net.status"));
    }

    #[test]
    fn large_layers_use_level_of_detail_without_changing_arithmetic() {
        let m = movie(
            "network(net,(640,340),\"24 18 3\",\"relu softmax\",700,380,9); forward(net,\"0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1\",2);",
        );
        assert_eq!(m.scene.ml_networks["net"].layers, vec![24, 18, 3]);
        assert_eq!(m.scene.ml_networks["net"].visible[0].len(), 8);
        assert!(m.scene.get("net.l0.more").is_some());
        assert!(
            m.scene.entities.len() < 250,
            "LOD should bound drawable count"
        );
    }

    #[test]
    fn invalid_shapes_and_inputs_fail_early() {
        assert!(crate::parse("network(n,(0,0),\"3\",\"relu\");").is_err());
        assert!(crate::parse("network(n,(0,0),\"3 2\",\"mystery\");").is_err());
        assert!(
            crate::parse("network(n,(400,300),\"3 2\",\"softmax\"); forward(n,\"1 2\");").is_err()
        );
    }

    #[test]
    fn activation_derivatives_cover_scalar_and_vector_cases() {
        let upstream = vec![0.7, -0.4, 0.2];
        assert_eq!(
            activation_backward(Activation::Linear, &[1.0, 2.0, 3.0], &upstream),
            upstream
        );
        assert_eq!(
            activation_backward(Activation::Relu, &[0.0, 2.0, 3.0], &[1.0, 1.0, 1.0]),
            vec![0.0, 1.0, 1.0]
        );
        let sigmoid = activation_backward(Activation::Sigmoid, &[0.25], &[2.0]);
        assert!((sigmoid[0] - 0.375).abs() < 1e-6);
        let tanh = activation_backward(Activation::Tanh, &[0.5], &[2.0]);
        assert!((tanh[0] - 1.5).abs() < 1e-6);

        let softmax_output = vec![0.2, 0.3, 0.5];
        let softmax_gradient =
            activation_backward(Activation::Softmax, &softmax_output, &[1.0, -0.5, 0.2]);
        assert!(softmax_gradient.iter().sum::<f32>().abs() < 1e-6);
    }

    #[test]
    fn cross_entropy_and_mse_match_textbook_values() {
        let (cross_entropy, _) = loss_value_and_output_gradient(
            LossKind::CrossEntropy,
            &[0.1, 0.7, 0.2],
            &[0.0, 1.0, 0.0],
        )
        .unwrap();
        assert!((cross_entropy + 0.7f32.ln()).abs() < 1e-6);

        let (mse, gradient) =
            loss_value_and_output_gradient(LossKind::Mse, &[1.0, 3.0], &[0.0, 1.0]).unwrap();
        assert!((mse - 1.25).abs() < 1e-6);
        assert_eq!(gradient, vec![0.5, 1.0]);
    }

    #[test]
    fn reverse_mode_gradients_match_finite_differences() {
        let m = movie("network(net,(640,340),\"2 3 2\",\"tanh softmax\",620,340,17);");
        let model = m.scene.ml_networks["net"].clone();
        let input = vec![0.35, -0.8];
        let target = vec![0.0, 1.0];
        let values = forward_values(&model, &input).unwrap();
        let (value, _) =
            loss_value_and_output_gradient(LossKind::CrossEntropy, values.last().unwrap(), &target)
                .unwrap();
        let loss = MlLossData {
            kind: LossKind::CrossEntropy,
            target: target.clone(),
            value,
        };
        let gradients = backward_values(&model, &values, &loss).unwrap();
        let epsilon = 1e-3;

        for transition in 0..model.weights.len() {
            for output in 0..model.weights[transition].len() {
                for input_unit in 0..model.weights[transition][output].len() {
                    let objective = |delta: f32| {
                        let mut perturbed = model.clone();
                        perturbed.weights[transition][output][input_unit] += delta;
                        let output = forward_values(&perturbed, &input).unwrap();
                        loss_value_and_output_gradient(
                            LossKind::CrossEntropy,
                            output.last().unwrap(),
                            &target,
                        )
                        .unwrap()
                        .0
                    };
                    let numerical = (objective(epsilon) - objective(-epsilon)) / (2.0 * epsilon);
                    let analytic = gradients.weights[transition][output][input_unit];
                    assert!(
                        (numerical - analytic).abs() < 2e-3,
                        "weight gradient mismatch at {transition}/{output}/{input_unit}: numerical={numerical}, analytic={analytic}"
                    );
                }
                let objective = |delta: f32| {
                    let mut perturbed = model.clone();
                    perturbed.biases[transition][output] += delta;
                    let output = forward_values(&perturbed, &input).unwrap();
                    loss_value_and_output_gradient(
                        LossKind::CrossEntropy,
                        output.last().unwrap(),
                        &target,
                    )
                    .unwrap()
                    .0
                };
                let numerical = (objective(epsilon) - objective(-epsilon)) / (2.0 * epsilon);
                let analytic = gradients.biases[transition][output];
                assert!((numerical - analytic).abs() < 2e-3);
            }
        }
    }

    #[test]
    fn loss_backward_and_update_make_one_truthful_learning_step() {
        let m = movie(
            "network(net,(640,340),\"3 4 3\",\"tanh softmax\",640,340,21); forward(net,\"0.15 0.92 0.38\",1); loss(net,\"0 0 1\",crossentropy,0.5); backward(net,1); update(net,0.1,0.8);",
        );
        let model = &m.scene.ml_networks["net"];
        let update = model.last_update.as_ref().expect("update is recorded");
        assert_eq!(update.learning_rate, 0.1);
        assert!(
            update.new_loss < update.old_loss,
            "small gradient step should reduce this fixture: {} -> {}",
            update.old_loss,
            update.new_loss
        );
        assert!(model.last_gradients.is_none());
        assert!((model.last_loss.as_ref().unwrap().value - update.new_loss).abs() < 1e-7);

        let settled = frame(&m, 100.0);
        assert!(text(&settled, "net.status").starts_with("Loss ·"));
        assert!(text(&settled, "net.out2.target").contains('1'));
    }

    #[test]
    fn checkpoint_and_restore_recover_the_exact_supervised_state() {
        let m = movie(
            "network(net,(640,340),\"3 4 3\",\"tanh softmax\",640,340,21); forward(net,\"0.15 0.92 0.38\",1); loss(net,\"1 0 0\",crossentropy,0.5); backward(net,1); checkpoint(before_update,net); update(net,0.12,0.8); restore(net,before_update,0.8,smooth);",
        );
        let checkpoint = &m.scene.ml_network_checkpoints["before_update"].state;
        let restored = &m.scene.ml_networks["net"];
        assert_eq!(restored.weights, checkpoint.weights);
        assert_eq!(restored.biases, checkpoint.biases);
        assert_eq!(restored.last_values, checkpoint.last_values);
        assert_eq!(
            restored.last_loss.as_ref().map(|loss| loss.value),
            checkpoint.last_loss.as_ref().map(|loss| loss.value)
        );
        assert!(restored.last_gradients.is_none());
        assert!(restored.last_update.is_none());

        let settled = frame(&m, 100.0);
        assert!(text(&settled, "net.status").starts_with("Rollback complete"));
        assert!(text(&settled, "net.status").contains("exact saved state"));
        assert!(text(&settled, "net.out0.target").contains("1.00"));
    }

    #[test]
    fn restore_timeline_is_stateless_when_seeking_out_of_order() {
        let m = movie(
            "network(net,(640,340),\"2 3 2\",\"tanh softmax\",620,340,5); forward(net,\"1 -1\",1); loss(net,\"1 0\",crossentropy,0.5); backward(net,1); checkpoint(saved,net); update(net,0.08,0.8); restore(net,saved,1);",
        );
        let late_first = frame(&m, 4.2);
        let early = frame(&m, 3.5);
        let late_again = frame(&m, 4.2);
        assert_eq!(
            text(&late_first, "net.status"),
            text(&late_again, "net.status")
        );
        assert_eq!(
            late_first.get("net.out0.bar").unwrap().shape,
            late_again.get("net.out0.bar").unwrap().shape
        );
        assert_eq!(
            late_first.get("net.e0.0.0").unwrap().color,
            late_again.get("net.e0.0.0").unwrap().color
        );
        assert_ne!(text(&early, "net.status"), text(&late_again, "net.status"));
    }

    #[test]
    fn ml2_timeline_remains_stateless_when_seeking_out_of_order() {
        let m = movie(
            "network(net,(640,340),\"2 3 2\",\"tanh softmax\",620,340,5); forward(net,\"1 -1\",1); loss(net,\"1 0\",crossentropy,0.5); backward(net,1.2); update(net,0.08,0.8);",
        );
        let late_first = frame(&m, 3.4);
        let early = frame(&m, 1.2);
        let late_again = frame(&m, 3.4);
        assert_eq!(
            text(&late_first, "net.status"),
            text(&late_again, "net.status")
        );
        assert_eq!(
            late_first.get("net.out0.bar").unwrap().shape,
            late_again.get("net.out0.bar").unwrap().shape
        );
        assert_ne!(text(&early, "net.status"), text(&late_again, "net.status"));
    }

    #[test]
    fn backward_uses_external_gradient_badges_and_update_clears_them() {
        let backward_only = movie(
            "network(net,(640,340),\"2 3 2\",\"tanh softmax\",620,340,5); forward(net,\"1 -1\",1); loss(net,\"1 0\",crossentropy,0.5); backward(net,2,smooth);",
        );
        let backward_frame = frame(&backward_only, 3.4);
        let badge = backward_frame
            .get("net.l0.n0.gradient.badge")
            .expect("gradient badge exists");
        assert!(badge.opacity > 0.9);
        assert!(text(&backward_frame, "net.l0.n0.gradient.value").starts_with('∇'));
        assert!(!text(&backward_frame, "net.l0.n0.value").starts_with('∇'));

        let with_update = movie(
            "network(net,(640,340),\"2 3 2\",\"tanh softmax\",620,340,5); forward(net,\"1 -1\",1); loss(net,\"1 0\",crossentropy,0.5); backward(net,2,smooth); update(net,0.08,1,smooth);",
        );
        let settled = frame(&with_update, 5.0);
        assert!(
            settled
                .get("net.l0.n0.gradient.badge")
                .expect("gradient badge persists for direct seeking")
                .opacity
                < 0.01
        );
    }

    #[test]
    fn ml2_rejects_bad_order_targets_and_hyperparameters() {
        assert!(
            crate::parse("network(n,(400,300),\"2 2\",\"softmax\"); loss(n,\"1 0\");").is_err()
        );
        assert!(crate::parse(
            "network(n,(400,300),\"2 2\",\"softmax\"); forward(n,\"1 0\"); backward(n);"
        )
        .is_err());
        assert!(crate::parse(
            "network(n,(400,300),\"2 2\",\"softmax\"); forward(n,\"1 0\"); loss(n,\"0.3 0.3\",crossentropy);"
        )
        .is_err());
        assert!(crate::parse(
            "network(n,(400,300),\"2 2\",\"softmax\"); forward(n,\"1 0\"); loss(n,\"1 0\"); backward(n); update(n,-0.1);"
        )
        .is_err());
        assert!(crate::parse("checkpoint(saved,missing);").is_err());
        assert!(
            crate::parse("network(n,(400,300),\"2 2\",\"softmax\"); checkpoint(saved,n);").is_err()
        );
        assert!(crate::parse(
            "network(n,(400,300),\"2 2\",\"softmax\"); forward(n,\"1 0\"); loss(n,\"1 0\"); checkpoint(saved,n); checkpoint(saved,n);"
        )
        .is_err());
        assert!(crate::parse(
            "network(n,(400,300),\"2 2\",\"softmax\"); forward(n,\"1 0\"); loss(n,\"1 0\"); checkpoint(saved,n); restore(n,missing);"
        )
        .is_err());
        assert!(crate::parse(
            "network(a,(300,300),\"2 2\",\"softmax\"); network(b,(700,300),\"2 2\",\"softmax\"); forward(a,\"1 0\"); loss(a,\"1 0\"); checkpoint(saved,a); restore(b,saved);"
        )
        .is_err());
        assert!(crate::parse(
            "network(n,(400,300),\"2 2\",\"softmax\"); forward(n,\"1 0\"); loss(n,\"1 0\"); backward(n); checkpoint(saved,n); update(n,0.05); restore(n,saved); update(n,0.05);"
        )
        .is_err());
        assert!(crate::parse(
            "network(n,(400,300),\"2 2\",\"softmax\"); forward(n,\"1 0\"); loss(n,\"1 0\"); backward(n); checkpoint(saved,n); update(n,0.05); restore(n,saved); backward(n); update(n,0.05);"
        )
        .is_ok());
    }

    #[test]
    fn a_new_forward_pass_clears_old_supervision_state_and_labels() {
        let m = movie(
            "network(n,(400,300),\"2 2\",\"softmax\",400,240,3); forward(n,\"1 0\",0.4); loss(n,\"1 0\",crossentropy,0.3); backward(n,0.4); update(n,0.05,0.3); forward(n,\"0 1\",0.4);",
        );
        let model = &m.scene.ml_networks["n"];
        assert!(model.last_loss.is_none());
        assert!(model.last_gradients.is_none());
        assert!(model.last_update.is_none());
        assert_eq!(frame(&m, 100.0).get("n.out0.target").unwrap().opacity, 0.0);
    }

    #[test]
    fn activation_plot_is_generic_and_tagged() {
        let m = movie("activation(a,(500,300),relu,320,200); hidden(a.axes);");
        assert!(m.scene.get("a.curve").is_some());
        assert_eq!(m.base().get("a.xaxis").unwrap().opacity, 0.0);
        assert_eq!(m.base().get("a.yaxis").unwrap().opacity, 0.0);
    }
}
