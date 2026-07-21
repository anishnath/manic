//! ML6: complete, small transformer blocks built on ML5 embeddings.
//!
//! `transformer` computes a deterministic educational encoder block: optional
//! pre/post layer normalization, true multi-head scaled dot-product attention,
//! optional causal masking, concatenation plus output projection, both residual
//! additions, an activation-bearing MLP, and deterministic inverted dropout in
//! explicitly authored training mode. `encode` only choreographs those already
//! computed values with ordinary absolute tracks, so direct seeking stays pure.

use std::collections::HashSet;

use macroquad::prelude::{Color, Vec2};

use crate::easing::Easing;
use crate::lang::diag::Error;
use crate::lang::lower::{resolve_easing, Args, Registry};
use crate::primitives::{Entity, FontKind, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TextEvent, TrackSpec, Value};

const MAX_HEADS: usize = 4;
const MAX_MLP_WIDTH: usize = 32;
const LAYER_NORM_EPSILON: f32 = 1e-5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlTransformerMask {
    None,
    Causal,
}

impl MlTransformerMask {
    fn label(self) -> &'static str {
        match self {
            Self::None => "FULL",
            Self::Causal => "CAUSAL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlTransformerNorm {
    Pre,
    Post,
}

impl MlTransformerNorm {
    fn label(self) -> &'static str {
        match self {
            Self::Pre => "PRE-NORM",
            Self::Post => "POST-NORM",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlTransformerActivation {
    Gelu,
    Relu,
    Silu,
    Tanh,
}

impl MlTransformerActivation {
    fn label(self) -> &'static str {
        match self {
            Self::Gelu => "GELU",
            Self::Relu => "RELU",
            Self::Silu => "SILU",
            Self::Tanh => "TANH",
        }
    }

    fn apply(self, value: f32) -> f32 {
        match self {
            // The standard tanh approximation used by many production stacks.
            Self::Gelu => {
                let inner =
                    (2.0 / std::f32::consts::PI).sqrt() * (value + 0.044_715 * value.powi(3));
                0.5 * value * (1.0 + inner.tanh())
            }
            Self::Relu => value.max(0.0),
            Self::Silu => value / (1.0 + (-value).exp()),
            Self::Tanh => value.tanh(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlTransformerMode {
    Inference,
    Training,
}

impl MlTransformerMode {
    fn label(self) -> &'static str {
        match self {
            Self::Inference => "INFERENCE",
            Self::Training => "TRAINING",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MlTransformerConfig {
    pub heads: usize,
    pub mask: MlTransformerMask,
    pub mlp_width: usize,
    pub activation: MlTransformerActivation,
    pub norm: MlTransformerNorm,
    pub dropout: f32,
    pub mode: MlTransformerMode,
    pub seed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MlTransformerHeadData {
    pub queries: Vec<Vec<f32>>,
    pub keys: Vec<Vec<f32>>,
    pub values: Vec<Vec<f32>>,
    /// Masked entries are negative infinity; unmasked entries are scaled dots.
    pub scores: Vec<Vec<f32>>,
    pub weights: Vec<Vec<f32>>,
    pub outputs: Vec<Vec<f32>>,
}

#[derive(Debug, Clone)]
pub struct MlTransformerData {
    pub embedding: String,
    pub tokens: Vec<String>,
    pub input: Vec<Vec<f32>>,
    pub config: MlTransformerConfig,
    pub heads: Vec<MlTransformerHeadData>,
    pub concatenated: Vec<Vec<f32>>,
    pub attention_output: Vec<Vec<f32>>,
    pub attention_after_dropout: Vec<Vec<f32>>,
    pub attention_dropout_mask: Vec<Vec<bool>>,
    pub residual1: Vec<Vec<f32>>,
    pub norm1: Vec<Vec<f32>>,
    pub mlp_hidden: Vec<Vec<f32>>,
    pub mlp_activated: Vec<Vec<f32>>,
    pub mlp_output: Vec<Vec<f32>>,
    pub mlp_after_dropout: Vec<Vec<f32>>,
    pub mlp_dropout_mask: Vec<Vec<bool>>,
    pub residual2: Vec<Vec<f32>>,
    pub norm2: Vec<Vec<f32>>,
    pub output: Vec<Vec<f32>>,
    pub status: String,
}

fn lcg(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 40) as u32) as f32 / (1u32 << 24) as f32
}

fn generated_matrix(rows: usize, cols: usize, state: &mut u64) -> Vec<Vec<f32>> {
    let limit = (6.0 / (rows + cols) as f32).sqrt();
    (0..rows)
        .map(|_| {
            (0..cols)
                .map(|_| (lcg(state) * 2.0 - 1.0) * limit)
                .collect()
        })
        .collect()
}

fn generated_bias(size: usize, state: &mut u64) -> Vec<f32> {
    (0..size).map(|_| (lcg(state) * 2.0 - 1.0) * 0.04).collect()
}

fn project(input: &[f32], weights: &[Vec<f32>], bias: Option<&[f32]>) -> Vec<f32> {
    weights
        .iter()
        .enumerate()
        .map(|(row, output)| {
            output
                .iter()
                .zip(input)
                .map(|(weight, value)| weight * value)
                .sum::<f32>()
                + bias.map_or(0.0, |values| values[row])
        })
        .collect()
}

fn project_rows(rows: &[Vec<f32>], weights: &[Vec<f32>], bias: Option<&[f32]>) -> Vec<Vec<f32>> {
    rows.iter().map(|row| project(row, weights, bias)).collect()
}

fn stable_softmax(values: &mut [f32]) {
    let max = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0;
    for value in values.iter_mut() {
        *value = (*value - max).exp();
        sum += *value;
    }
    for value in values {
        *value /= sum;
    }
}

fn layer_norm_row(row: &[f32]) -> Vec<f32> {
    let mean = row.iter().sum::<f32>() / row.len() as f32;
    let variance = row.iter().map(|value| (value - mean).powi(2)).sum::<f32>() / row.len() as f32;
    let denominator = (variance + LAYER_NORM_EPSILON).sqrt();
    row.iter()
        .map(|value| (value - mean) / denominator)
        .collect()
}

fn layer_norm(rows: &[Vec<f32>]) -> Vec<Vec<f32>> {
    rows.iter().map(|row| layer_norm_row(row)).collect()
}

fn add_rows(left: &[Vec<f32>], right: &[Vec<f32>]) -> Vec<Vec<f32>> {
    left.iter()
        .zip(right)
        .map(|(a, b)| a.iter().zip(b).map(|(x, y)| x + y).collect())
        .collect()
}

fn apply_dropout(
    rows: &[Vec<f32>],
    probability: f32,
    mode: MlTransformerMode,
    state: &mut u64,
) -> (Vec<Vec<f32>>, Vec<Vec<bool>>) {
    if mode == MlTransformerMode::Inference || probability == 0.0 {
        return (
            rows.to_vec(),
            rows.iter().map(|row| vec![true; row.len()]).collect(),
        );
    }
    let keep_scale = 1.0 / (1.0 - probability);
    let mut mask = Vec::with_capacity(rows.len());
    let output = rows
        .iter()
        .map(|row| {
            let mut row_mask = Vec::with_capacity(row.len());
            let values = row
                .iter()
                .map(|value| {
                    let keep = lcg(state) >= probability;
                    row_mask.push(keep);
                    if keep {
                        value * keep_scale
                    } else {
                        0.0
                    }
                })
                .collect();
            mask.push(row_mask);
            values
        })
        .collect();
    (output, mask)
}

fn parse_positive_usize(value: &str, noun: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("transformer {noun} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("transformer {noun} must be a positive integer"));
    }
    Ok(parsed)
}

fn parse_config(source: &str, dimension: usize) -> Result<MlTransformerConfig, String> {
    let mut heads = 2usize;
    let mut mask = MlTransformerMask::Causal;
    let mut mlp_width = dimension * 2;
    let mut activation = MlTransformerActivation::Gelu;
    let mut norm = MlTransformerNorm::Pre;
    let mut dropout = 0.0f32;
    let mut mode = MlTransformerMode::Inference;
    let mut seed = 41u64;
    let mut seen = HashSet::new();

    for field in source.split_whitespace() {
        let Some((raw_key, raw_value)) = field.split_once('=') else {
            return Err(format!(
                "transformer option `{field}` needs `key=value` syntax"
            ));
        };
        let key = raw_key.to_ascii_lowercase();
        let value = raw_value.to_ascii_lowercase();
        if !seen.insert(key.clone()) {
            return Err(format!("transformer option `{key}` is repeated"));
        }
        match key.as_str() {
            "heads" => heads = parse_positive_usize(&value, "heads")?,
            "mask" => {
                mask = match value.as_str() {
                    "none" | "full" => MlTransformerMask::None,
                    "causal" => MlTransformerMask::Causal,
                    _ => return Err("transformer mask must be `none` or `causal`".into()),
                }
            }
            "mlp" => mlp_width = parse_positive_usize(&value, "MLP width")?,
            "activation" => {
                activation = match value.as_str() {
                    "gelu" => MlTransformerActivation::Gelu,
                    "relu" => MlTransformerActivation::Relu,
                    "silu" | "swish" => MlTransformerActivation::Silu,
                    "tanh" => MlTransformerActivation::Tanh,
                    _ => {
                        return Err(
                            "transformer activation must be gelu, relu, silu, or tanh".into(),
                        )
                    }
                }
            }
            "norm" => {
                norm = match value.as_str() {
                    "pre" => MlTransformerNorm::Pre,
                    "post" => MlTransformerNorm::Post,
                    _ => return Err("transformer norm must be `pre` or `post`".into()),
                }
            }
            "dropout" => {
                dropout = value
                    .parse::<f32>()
                    .map_err(|_| "transformer dropout must be a finite number".to_string())?;
                if !dropout.is_finite() || !(0.0..1.0).contains(&dropout) {
                    return Err("transformer dropout must be at least 0 and less than 1".into());
                }
            }
            "mode" => {
                mode = match value.as_str() {
                    "inference" | "eval" => MlTransformerMode::Inference,
                    "training" | "train" => MlTransformerMode::Training,
                    _ => return Err("transformer mode must be `inference` or `training`".into()),
                }
            }
            "seed" => {
                seed = value
                    .parse::<u64>()
                    .map_err(|_| "transformer seed must be a non-negative integer".to_string())?
            }
            _ => {
                return Err(format!(
                    "unknown transformer option `{key}` (try: heads, mask, mlp, activation, norm, dropout, mode, seed)"
                ))
            }
        }
    }

    if heads > MAX_HEADS {
        return Err(format!(
            "transformer supports at most {MAX_HEADS} attention heads"
        ));
    }
    if dimension % heads != 0 {
        return Err(format!(
            "transformer model dimension {dimension} must divide exactly across {heads} heads"
        ));
    }
    if mlp_width > MAX_MLP_WIDTH {
        return Err(format!(
            "transformer MLP width must be at most {MAX_MLP_WIDTH}"
        ));
    }

    Ok(MlTransformerConfig {
        heads,
        mask,
        mlp_width,
        activation,
        norm,
        dropout,
        mode,
        seed,
    })
}

fn compute_heads(
    input: &[Vec<f32>],
    config: &MlTransformerConfig,
    state: &mut u64,
) -> Vec<MlTransformerHeadData> {
    let dimension = input[0].len();
    let head_dimension = dimension / config.heads;
    (0..config.heads)
        .map(|_| {
            let wq = generated_matrix(head_dimension, dimension, state);
            let wk = generated_matrix(head_dimension, dimension, state);
            let wv = generated_matrix(head_dimension, dimension, state);
            let queries = project_rows(input, &wq, None);
            let keys = project_rows(input, &wk, None);
            let values = project_rows(input, &wv, None);
            let scale = (head_dimension as f32).sqrt();
            let mut scores = Vec::with_capacity(input.len());
            let mut weights = Vec::with_capacity(input.len());
            for (row, query) in queries.iter().enumerate() {
                let score_row: Vec<f32> = keys
                    .iter()
                    .enumerate()
                    .map(|(col, key)| {
                        if config.mask == MlTransformerMask::Causal && col > row {
                            f32::NEG_INFINITY
                        } else {
                            query.iter().zip(key).map(|(q, k)| q * k).sum::<f32>() / scale
                        }
                    })
                    .collect();
                let mut weight_row = score_row.clone();
                stable_softmax(&mut weight_row);
                scores.push(score_row);
                weights.push(weight_row);
            }
            let outputs = weights
                .iter()
                .map(|row| {
                    (0..head_dimension)
                        .map(|axis| {
                            row.iter()
                                .zip(&values)
                                .map(|(weight, value)| weight * value[axis])
                                .sum()
                        })
                        .collect()
                })
                .collect();
            MlTransformerHeadData {
                queries,
                keys,
                values,
                scores,
                weights,
                outputs,
            }
        })
        .collect()
}

fn concatenate_heads(heads: &[MlTransformerHeadData], rows: usize) -> Vec<Vec<f32>> {
    (0..rows)
        .map(|row| {
            heads
                .iter()
                .flat_map(|head| head.outputs[row].iter().copied())
                .collect()
        })
        .collect()
}

fn compute_transformer(input: &[Vec<f32>], config: &MlTransformerConfig) -> MlTransformerData {
    let dimension = input[0].len();
    let mut state = config.seed.max(1);
    let attention_input = if config.norm == MlTransformerNorm::Pre {
        layer_norm(input)
    } else {
        input.to_vec()
    };
    let heads = compute_heads(&attention_input, config, &mut state);
    let concatenated = concatenate_heads(&heads, input.len());
    let output_projection = generated_matrix(dimension, dimension, &mut state);
    let attention_output = project_rows(&concatenated, &output_projection, None);
    let (attention_after_dropout, attention_dropout_mask) =
        apply_dropout(&attention_output, config.dropout, config.mode, &mut state);
    let residual1 = add_rows(input, &attention_after_dropout);
    let norm1 = if config.norm == MlTransformerNorm::Pre {
        attention_input
    } else {
        layer_norm(&residual1)
    };
    let mlp_input = if config.norm == MlTransformerNorm::Pre {
        layer_norm(&residual1)
    } else {
        norm1.clone()
    };
    let w1 = generated_matrix(config.mlp_width, dimension, &mut state);
    let b1 = generated_bias(config.mlp_width, &mut state);
    let mlp_hidden = project_rows(&mlp_input, &w1, Some(&b1));
    let mlp_activated: Vec<Vec<f32>> = mlp_hidden
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| config.activation.apply(*value))
                .collect()
        })
        .collect();
    let w2 = generated_matrix(dimension, config.mlp_width, &mut state);
    let b2 = generated_bias(dimension, &mut state);
    let mlp_output = project_rows(&mlp_activated, &w2, Some(&b2));
    let (mlp_after_dropout, mlp_dropout_mask) =
        apply_dropout(&mlp_output, config.dropout, config.mode, &mut state);
    let residual2_base = if config.norm == MlTransformerNorm::Pre {
        &residual1
    } else {
        &norm1
    };
    let residual2 = add_rows(residual2_base, &mlp_after_dropout);
    let norm2 = if config.norm == MlTransformerNorm::Pre {
        mlp_input
    } else {
        layer_norm(&residual2)
    };
    let output = if config.norm == MlTransformerNorm::Pre {
        residual2.clone()
    } else {
        norm2.clone()
    };

    MlTransformerData {
        embedding: String::new(),
        tokens: Vec::new(),
        input: input.to_vec(),
        config: config.clone(),
        heads,
        concatenated,
        attention_output,
        attention_after_dropout,
        attention_dropout_mask,
        residual1,
        norm1,
        mlp_hidden,
        mlp_activated,
        mlp_output,
        mlp_after_dropout,
        mlp_dropout_mask,
        residual2,
        norm2,
        output,
        status: String::new(),
    }
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

fn tag(entity: &mut Entity, root: &str, role: &str) {
    entity.tags.push(root.to_string());
    entity.tags.push(format!("{root}.{role}"));
}

fn alias(entity: &mut Entity, root: &str, role: &str) {
    entity.tags.push(format!("{root}.{role}"));
}

fn add_text(
    scene: &mut Scene,
    root: &str,
    role: &str,
    id: String,
    content: String,
    pos: Vec2,
    size: f32,
    color: Color,
) {
    let mut entity = Entity::new(id, Shape::Text { content, size }, pos, color);
    entity.font = FontKind::MonoBold;
    entity.z = 20;
    tag(&mut entity, root, role);
    scene.add(entity);
}

fn add_stage_card(
    scene: &mut Scene,
    root: &str,
    id: &str,
    role: &str,
    aliases: &[&str],
    center: Vec2,
    width: f32,
    height: f32,
    label: &str,
    accent: Color,
) {
    let mut card = Entity::new(
        format!("{root}.{id}.card"),
        Shape::Rect {
            w: width,
            h: height,
        },
        center,
        mix(style::PANEL, accent, 0.10),
    );
    card.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 1.4,
        outline_color: Some(mix(style::DIM, accent, 0.42)),
    };
    card.z = 2;
    tag(&mut card, root, role);
    for alias_role in aliases {
        alias(&mut card, root, alias_role);
    }
    scene.add(card);
    let mut text = Entity::new(
        format!("{root}.{id}.label"),
        Shape::Text {
            content: label.into(),
            size: (width * 0.105).clamp(9.0, 15.0),
        },
        Vec2::new(center.x, center.y - height * 0.40),
        style::FG,
    );
    text.font = FontKind::MonoBold;
    text.z = 4;
    tag(&mut text, root, role);
    for alias_role in aliases {
        alias(&mut text, root, alias_role);
    }
    scene.add(text);
}

fn add_stage_vector(
    scene: &mut Scene,
    root: &str,
    id: &str,
    role: &str,
    aliases: &[&str],
    center: Vec2,
    width: f32,
    height: f32,
    values: &[f32],
    caption: &str,
    positive: Color,
) {
    let shown = values.len().min(6);
    let max = values
        .iter()
        .take(shown)
        .map(|value| value.abs())
        .fold(0.0f32, f32::max)
        .max(1e-6);
    let cell_w = (width * 0.72 / shown.max(1) as f32).clamp(5.0, 14.0);
    let gap = cell_w * 1.18;
    let start_x = center.x - gap * (shown.saturating_sub(1) as f32) * 0.5;
    for (axis, value) in values.iter().take(shown).enumerate() {
        let magnitude = (value.abs() / max).sqrt();
        let bar_h = height * (0.12 + 0.20 * magnitude);
        let color = if *value >= 0.0 {
            positive
        } else {
            style::MAGENTA
        };
        let mut bar = Entity::new(
            format!("{root}.{id}.axis{axis}"),
            Shape::Rect {
                w: cell_w,
                h: bar_h,
            },
            Vec2::new(start_x + axis as f32 * gap, center.y + height * 0.05),
            mix(style::PANEL, color, 0.28 + magnitude * 0.62),
        );
        bar.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 0.8,
            outline_color: Some(mix(style::DIM, color, 0.45 + magnitude * 0.45)),
        };
        bar.z = 4;
        tag(&mut bar, root, role);
        for alias_role in aliases {
            alias(&mut bar, root, alias_role);
        }
        scene.add(bar);
    }
    add_text(
        scene,
        root,
        role,
        format!("{root}.{id}.summary"),
        caption.into(),
        Vec2::new(center.x, center.y + height * 0.35),
        (width * 0.075).clamp(7.5, 10.0),
        style::DIM,
    );
    if let Some(summary) = scene.get_mut(&format!("{root}.{id}.summary")) {
        for alias_role in aliases {
            alias(summary, root, alias_role);
        }
    }
}

fn add_residual_bypass(
    scene: &mut Scene,
    root: &str,
    id: &str,
    role: &str,
    from: Vec2,
    ctrl: Vec2,
    to: Vec2,
) {
    let mut bypass = Entity::new(
        format!("{root}.{id}"),
        Shape::Curve {
            ctrl,
            to,
            arrow: true,
        },
        from,
        style::GOLD,
    );
    bypass.stroke.width = 1.8;
    bypass.opacity = 0.34;
    bypass.glow = 0.18;
    bypass.z = 1;
    tag(&mut bypass, root, role);
    scene.add(bypass);
}

fn add_connector(scene: &mut Scene, root: &str, index: usize, from: Vec2, to: Vec2) {
    let mut connector = Entity::new(
        format!("{root}.flow{index}"),
        Shape::Curve {
            ctrl: (from + to) * 0.5,
            to,
            arrow: true,
        },
        from,
        style::DIM,
    );
    connector.stroke.width = 1.4;
    connector.opacity = 0.48;
    connector.z = 1;
    tag(&mut connector, root, "structure");
    scene.add(connector);
}

fn display_token(token: &str) -> String {
    let mut chars = token.chars();
    let shown: String = chars.by_ref().take(10).collect();
    if chars.next().is_some() {
        format!("{shown}…")
    } else {
        shown
    }
}

fn vector_summary(values: &[f32]) -> String {
    let shown = values
        .iter()
        .take(3)
        .map(|value| format!("{value:.2}"))
        .collect::<Vec<_>>()
        .join(",");
    if values.len() > 3 {
        format!("[{shown},…]")
    } else {
        format!("[{shown}]")
    }
}

fn visible_indices(count: usize) -> Vec<usize> {
    if count <= 6 {
        return (0..count).collect();
    }
    vec![0, 1, 2, count - 3, count - 2, count - 1]
}

fn add_head_views(
    scene: &mut Scene,
    id: &str,
    data: &MlTransformerData,
    center: Vec2,
    width: f32,
    height: f32,
) {
    let per_head = height / data.heads.len() as f32;
    let visible = visible_indices(data.tokens.len());
    for (head_index, head) in data.heads.iter().enumerate() {
        let y = center.y - height * 0.5 + per_head * (head_index as f32 + 0.5);
        let mut card = Entity::new(
            format!("{id}.head{head_index}.card"),
            Shape::Rect {
                w: width,
                h: per_head * 0.86,
            },
            Vec2::new(center.x, y),
            mix(style::PANEL, style::MAGENTA, 0.09),
        );
        card.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 1.1,
            outline_color: Some(mix(style::DIM, style::MAGENTA, 0.35)),
        };
        card.z = 2;
        tag(&mut card, id, "heads");
        alias(&mut card, id, &format!("head{head_index}"));
        scene.add(card);

        add_text(
            scene,
            id,
            "heads",
            format!("{id}.head{head_index}.title"),
            format!(
                "H{} · d={} · {}",
                head_index + 1,
                data.input[0].len() / data.heads.len(),
                data.config.mask.label()
            ),
            Vec2::new(center.x - width * 0.27, y - per_head * 0.29),
            (per_head * 0.115).clamp(8.0, 12.0),
            style::DIM,
        );
        if let Some(title) = scene.get_mut(&format!("{id}.head{head_index}.title")) {
            alias(title, id, &format!("head{head_index}"));
        }
        for (badge, role, dx, color) in [
            ("Q", "q", -0.29, style::CYAN),
            ("K", "k", -0.29, style::MAGENTA),
            ("V", "v", -0.29, style::LIME),
        ] {
            let offset = match badge {
                "Q" => -0.12,
                "K" => 0.0,
                _ => 0.12,
            };
            add_text(
                scene,
                id,
                role,
                format!("{id}.head{head_index}.{role}"),
                badge.into(),
                Vec2::new(center.x + width * dx, y + per_head * offset),
                (per_head * 0.12).clamp(8.0, 12.0),
                color,
            );
        }

        let side = (per_head * 0.62).min(width * 0.56);
        let cell = side / visible.len() as f32;
        let grid_center = Vec2::new(center.x + width * 0.15, y + per_head * 0.03);
        for (visual_row, &row) in visible.iter().enumerate() {
            for (visual_col, &col) in visible.iter().enumerate() {
                let masked = data.config.mask == MlTransformerMask::Causal && col > row;
                let weight = head.weights[row][col];
                let pos = grid_center
                    + Vec2::new(
                        (visual_col as f32 - (visible.len() - 1) as f32 * 0.5) * cell,
                        (visual_row as f32 - (visible.len() - 1) as f32 * 0.5) * cell,
                    );
                let mut heat = Entity::new(
                    format!("{id}.head{head_index}.weight{row}.{col}"),
                    Shape::Rect {
                        w: cell * 0.82,
                        h: cell * 0.82,
                    },
                    pos,
                    if masked {
                        mix(style::PANEL, style::DIM, 0.08)
                    } else {
                        mix(style::PANEL, style::MAGENTA, 0.12 + weight * 0.76)
                    },
                );
                heat.stroke = StrokeStyle {
                    fill: true,
                    outline: true,
                    width: 0.8,
                    outline_color: Some(if masked {
                        style::DIM
                    } else {
                        mix(style::DIM, style::MAGENTA, weight)
                    }),
                };
                heat.opacity = if masked { 0.24 } else { 0.72 };
                heat.z = 3;
                tag(&mut heat, id, "matrix");
                alias(&mut heat, id, "heads");
                alias(&mut heat, id, &format!("head{head_index}"));
                alias(&mut heat, id, &format!("row{row}"));
                if masked {
                    alias(&mut heat, id, "mask");
                }
                scene.add(heat);
            }
        }
    }
}

/// `transformer(id, embedding, (cx,cy), "config", [width], [height])`
fn c_transformer(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(6)?;
    let id = args.ident(0)?;
    if scene.ml_transformers.contains_key(&id) {
        return Err(Error::new(
            format!("transformer figure `{id}` already exists"),
            args.span_of(0),
        ));
    }
    let embedding_id = args.ident(1)?;
    let embedding = scene
        .ml_embeddings
        .get(&embedding_id)
        .cloned()
        .ok_or_else(|| {
            Error::new(
                format!("`{embedding_id}` is not an embedding; create it with `embedding` first"),
                args.span_of(1),
            )
        })?;
    let center = args.pair(2)?;
    let config_source = args.text(3)?;
    let config = parse_config(&config_source, embedding.dimension)
        .map_err(|message| Error::new(message, args.span_of(3)))?;
    let width = args.opt_num(4)?.unwrap_or(1120.0);
    let height = args.opt_num(5)?.unwrap_or(520.0);
    if !width.is_finite() || width < 760.0 {
        return Err(Error::new(
            "transformer width must be a finite number of at least 760",
            args.span_of(4),
        ));
    }
    if !height.is_finite() || height < 400.0 {
        return Err(Error::new(
            "transformer height must be a finite number of at least 400",
            args.span_of(5),
        ));
    }

    let mut data = compute_transformer(&embedding.combined_vectors, &config);
    data.embedding = embedding_id;
    data.tokens = embedding.tokens;
    data.status = format!("{id}.status");

    let dropout_label = if config.mode == MlTransformerMode::Training {
        format!("DROP {:.0}%", config.dropout * 100.0)
    } else {
        "DROPOUT OFF".into()
    };
    add_text(
        scene,
        &id,
        "labels",
        format!("{id}.heading"),
        format!(
            "TRANSFORMER BLOCK · {} HEADS · {} · {} · {} · {}",
            config.heads,
            config.mask.label(),
            config.norm.label(),
            config.mode.label(),
            dropout_label
        ),
        Vec2::new(center.x, center.y - height * 0.49),
        17.0,
        style::DIM,
    );

    let input_x = center.x - width * 0.445;
    let heads_x = center.x - width * 0.285;
    let project_x = center.x - width * 0.105;
    let stage1_x = center.x + width * 0.035;
    let mlp_x = center.x + width * 0.19;
    let stage2_x = center.x + width * 0.335;
    let output_x = center.x + width * 0.455;
    let stage_y = center.y;
    let stage_h = height * 0.38;
    let row_h = height * 0.56 / data.tokens.len() as f32;
    let y0 = center.y - row_h * (data.tokens.len().saturating_sub(1) as f32) * 0.5;
    let token_w = (width * 0.115).clamp(86.0, 138.0);
    let token_h = (row_h * 0.68).clamp(24.0, 42.0);

    add_text(
        scene,
        &id,
        "input",
        format!("{id}.input.header"),
        "MODEL INPUT".into(),
        Vec2::new(input_x, center.y - height * 0.36),
        12.0,
        style::DIM,
    );
    add_text(
        scene,
        &id,
        "output",
        format!("{id}.output.header"),
        "BLOCK OUTPUT".into(),
        Vec2::new(output_x, center.y - height * 0.36),
        12.0,
        style::DIM,
    );
    for row in 0..data.tokens.len() {
        let y = y0 + row as f32 * row_h;
        for (side, x, role, vector) in [
            ("in", input_x, "input", &data.input[row]),
            ("out", output_x, "output", &data.output[row]),
        ] {
            let mut card = Entity::new(
                format!("{id}.{side}{row}.box"),
                Shape::Rect {
                    w: token_w,
                    h: token_h,
                },
                Vec2::new(x, y),
                mix(
                    style::PANEL,
                    if role == "input" {
                        style::CYAN
                    } else {
                        style::LIME
                    },
                    0.11,
                ),
            );
            card.stroke = StrokeStyle {
                fill: true,
                outline: true,
                width: 1.1,
                outline_color: Some(style::DIM),
            };
            card.z = 3;
            tag(&mut card, &id, role);
            alias(&mut card, &id, &format!("token{row}"));
            alias(&mut card, &id, &format!("row{row}"));
            scene.add(card);
            add_text(
                scene,
                &id,
                role,
                format!("{id}.{side}{row}.label"),
                format!(
                    "{} {}",
                    display_token(&data.tokens[row]),
                    vector_summary(vector)
                ),
                Vec2::new(x, y),
                (token_w * 0.075).clamp(8.0, 11.0),
                style::FG,
            );
        }
    }

    let head_aliases: Vec<&str> = if config.norm == MlTransformerNorm::Pre {
        vec!["norm1"]
    } else {
        vec![]
    };
    let head_label = if config.norm == MlTransformerNorm::Pre {
        "NORM 1 → MHA"
    } else {
        "MULTI-HEAD ATTENTION"
    };
    add_stage_card(
        scene,
        &id,
        "heads.stage",
        "heads",
        &head_aliases,
        Vec2::new(heads_x, stage_y),
        width * 0.17,
        stage_h,
        head_label,
        style::MAGENTA,
    );
    add_head_views(
        scene,
        &id,
        &data,
        Vec2::new(heads_x, stage_y + height * 0.025),
        width * 0.145,
        stage_h * 0.78,
    );

    add_stage_card(
        scene,
        &id,
        "project",
        "projection",
        &["concat", "dropout", "dropout.attention"],
        Vec2::new(project_x, stage_y),
        width * 0.105,
        stage_h * 0.62,
        "CONCAT + Wo",
        style::CYAN,
    );
    let focus_row = data.tokens.len() / 2;
    add_stage_vector(
        scene,
        &id,
        "project",
        "projection",
        &["concat", "dropout", "dropout.attention"],
        Vec2::new(project_x, stage_y),
        width * 0.105,
        stage_h * 0.62,
        &data.attention_output[focus_row],
        &format!("H1+H2 → d{}", data.input[0].len()),
        style::CYAN,
    );
    let stage1_label = if config.norm == MlTransformerNorm::Pre {
        "ADD 1"
    } else {
        "ADD 1 → NORM"
    };
    let stage1_aliases: Vec<&str> = if config.norm == MlTransformerNorm::Post {
        vec!["norm1"]
    } else {
        vec![]
    };
    add_stage_card(
        scene,
        &id,
        "residual1",
        "residual1",
        &stage1_aliases,
        Vec2::new(stage1_x, stage_y),
        width * 0.105,
        stage_h * 0.54,
        stage1_label,
        style::GOLD,
    );
    add_stage_vector(
        scene,
        &id,
        "residual1",
        "residual1",
        &stage1_aliases,
        Vec2::new(stage1_x, stage_y),
        width * 0.105,
        stage_h * 0.54,
        &data.residual1[focus_row],
        "x + attention",
        style::GOLD,
    );
    let mlp_label = if config.norm == MlTransformerNorm::Pre {
        format!("NORM 2 → {} MLP", config.activation.label())
    } else {
        format!("{} MLP", config.activation.label())
    };
    let mlp_aliases: Vec<&str> = if config.norm == MlTransformerNorm::Pre {
        vec!["norm2", "activation", "dropout", "dropout.mlp"]
    } else {
        vec!["activation", "dropout", "dropout.mlp"]
    };
    add_stage_card(
        scene,
        &id,
        "mlp",
        "mlp",
        &mlp_aliases,
        Vec2::new(mlp_x, stage_y),
        width * 0.125,
        stage_h * 0.66,
        &mlp_label,
        style::LIME,
    );
    add_stage_vector(
        scene,
        &id,
        "mlp",
        "mlp",
        &mlp_aliases,
        Vec2::new(mlp_x, stage_y),
        width * 0.125,
        stage_h * 0.66,
        &data.mlp_activated[focus_row],
        &format!(
            "d{} → {} → d{}",
            data.input[0].len(),
            config.mlp_width,
            data.input[0].len()
        ),
        style::LIME,
    );
    let stage2_label = if config.norm == MlTransformerNorm::Pre {
        "ADD 2"
    } else {
        "ADD 2 → NORM"
    };
    let stage2_aliases: Vec<&str> = if config.norm == MlTransformerNorm::Post {
        vec!["norm2"]
    } else {
        vec![]
    };
    add_stage_card(
        scene,
        &id,
        "residual2",
        "residual2",
        &stage2_aliases,
        Vec2::new(stage2_x, stage_y),
        width * 0.105,
        stage_h * 0.54,
        stage2_label,
        style::GOLD,
    );
    add_stage_vector(
        scene,
        &id,
        "residual2",
        "residual2",
        &stage2_aliases,
        Vec2::new(stage2_x, stage_y),
        width * 0.105,
        stage_h * 0.54,
        &data.residual2[focus_row],
        "skip + MLP",
        style::GOLD,
    );

    add_residual_bypass(
        scene,
        &id,
        "skip1",
        "residual1",
        Vec2::new(input_x + token_w * 0.42, stage_y - token_h * 0.34),
        Vec2::new(project_x, stage_y - stage_h * 0.72),
        Vec2::new(stage1_x, stage_y - stage_h * 0.28),
    );
    add_residual_bypass(
        scene,
        &id,
        "skip2",
        "residual2",
        Vec2::new(stage1_x, stage_y + stage_h * 0.28),
        Vec2::new(mlp_x, stage_y + stage_h * 0.72),
        Vec2::new(stage2_x, stage_y + stage_h * 0.28),
    );

    let points = [
        Vec2::new(input_x + token_w * 0.55, stage_y),
        Vec2::new(heads_x - width * 0.09, stage_y),
        Vec2::new(heads_x + width * 0.09, stage_y),
        Vec2::new(project_x - width * 0.057, stage_y),
        Vec2::new(project_x + width * 0.057, stage_y),
        Vec2::new(stage1_x - width * 0.057, stage_y),
        Vec2::new(stage1_x + width * 0.057, stage_y),
        Vec2::new(mlp_x - width * 0.067, stage_y),
        Vec2::new(mlp_x + width * 0.067, stage_y),
        Vec2::new(stage2_x - width * 0.057, stage_y),
        Vec2::new(stage2_x + width * 0.057, stage_y),
        Vec2::new(output_x - token_w * 0.55, stage_y),
    ];
    for (index, pair) in points.chunks_exact(2).enumerate() {
        add_connector(scene, &id, index, pair[0], pair[1]);
    }

    add_text(
        scene,
        &id,
        "labels",
        data.status.clone(),
        "Ready · model input stays visible through both residual paths".into(),
        Vec2::new(center.x, center.y + height * 0.42),
        15.0,
        style::DIM,
    );

    scene.ml_transformers.insert(id, data);
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

fn add_tracks(
    scene: &Scene,
    tracks: &mut Vec<TrackSpec>,
    id_or_tag: String,
    prop: Prop,
    target: TargetValue,
    start: f32,
    dur: f32,
    easing: Easing,
) {
    if scene.get(&id_or_tag).is_some() {
        tracks.push(track(id_or_tag, prop, target, start, dur, easing));
        return;
    }
    tracks.extend(
        scene
            .entities
            .iter()
            .filter(|entity| entity.tags.iter().any(|tag| tag == &id_or_tag))
            .map(|entity| track(entity.id.clone(), prop, target.clone(), start, dur, easing)),
    );
}

fn reveal_role(
    scene: &Scene,
    tracks: &mut Vec<TrackSpec>,
    id: &str,
    role: &str,
    start: f32,
    duration: f32,
    easing: Easing,
) {
    add_tracks(
        scene,
        tracks,
        format!("{id}.{role}"),
        Prop::Opacity,
        TargetValue::Abs(Value::F(1.0)),
        start,
        duration,
        easing,
    );
}

/// `encode(transformer, [duration], [ease])`
fn v_encode(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(3)?;
    let id = args.ident(0)?;
    let data = scene.ml_transformers.get(&id).cloned().ok_or_else(|| {
        Error::new(
            format!("`{id}` is not a transformer block"),
            args.span_of(0),
        )
    })?;
    let duration = args.opt_num(1)?.unwrap_or(5.4);
    if !duration.is_finite() || duration <= 0.0 {
        return Err(Error::new(
            "encode duration must be positive and finite",
            args.span_of(1),
        ));
    }
    let easing = if args.len() > 2 {
        let word = args.ident(2)?;
        resolve_easing(&word, args.span_of(2))?
    } else {
        Easing::InOutCubic
    };
    let mut tracks = Vec::new();
    let mut events = Vec::new();
    add_tracks(
        scene,
        &mut tracks,
        id.clone(),
        Prop::Opacity,
        TargetValue::Abs(Value::F(0.12)),
        0.0,
        duration * 0.04,
        Easing::InOutCubic,
    );
    reveal_role(
        scene,
        &mut tracks,
        &id,
        "labels",
        0.0,
        duration * 0.05,
        easing,
    );
    reveal_role(
        scene,
        &mut tracks,
        &id,
        "structure",
        0.0,
        duration * 0.05,
        easing,
    );
    reveal_role(
        scene,
        &mut tracks,
        &id,
        "input",
        duration * 0.03,
        duration * 0.08,
        easing,
    );
    events.push(TextEvent::text(
        data.status.clone(),
        format!(
            "{} token vectors enter with d_model = {}",
            data.tokens.len(),
            data.input[0].len()
        ),
        duration * 0.04,
    ));

    let head_start = duration * 0.14;
    let head_span = duration * 0.25;
    for head in 0..data.config.heads {
        let start = head_start + head_span * head as f32 / data.config.heads as f32;
        reveal_role(
            scene,
            &mut tracks,
            &id,
            &format!("head{head}"),
            start,
            head_span / data.config.heads as f32 * 0.80,
            easing,
        );
        events.push(TextEvent::text(
            data.status.clone(),
            format!(
                "head {} computes scaled QKᵀ, applies {} mask, then mixes V",
                head + 1,
                data.config.mask.label().to_ascii_lowercase()
            ),
            start,
        ));
    }
    for role in ["q", "k", "v", "matrix", "mask"] {
        reveal_role(
            scene,
            &mut tracks,
            &id,
            role,
            head_start,
            head_span * 0.85,
            easing,
        );
    }
    add_tracks(
        scene,
        &mut tracks,
        format!("{id}.flow0"),
        Prop::Flow,
        TargetValue::Rel(Value::F(1.0)),
        head_start,
        duration * 0.10,
        easing,
    );

    let project_start = duration * 0.43;
    for role in ["concat", "projection"] {
        reveal_role(
            scene,
            &mut tracks,
            &id,
            role,
            project_start,
            duration * 0.09,
            easing,
        );
    }
    events.push(TextEvent::text(
        data.status.clone(),
        format!(
            "{} heads concatenate back to d_model = {}, then Wᵒ projects once",
            data.config.heads,
            data.input[0].len()
        ),
        project_start,
    ));
    add_tracks(
        scene,
        &mut tracks,
        format!("{id}.flow1"),
        Prop::Flow,
        TargetValue::Rel(Value::F(1.0)),
        project_start,
        duration * 0.09,
        easing,
    );

    let residual1_start = duration * 0.55;
    for role in ["residual1", "norm1"] {
        reveal_role(
            scene,
            &mut tracks,
            &id,
            role,
            residual1_start,
            duration * 0.09,
            easing,
        );
    }
    add_tracks(
        scene,
        &mut tracks,
        format!("{id}.residual1"),
        Prop::Flow,
        TargetValue::Rel(Value::F(1.0)),
        residual1_start,
        duration * 0.10,
        easing,
    );
    add_tracks(
        scene,
        &mut tracks,
        format!("{id}.flow2"),
        Prop::Flow,
        TargetValue::Rel(Value::F(1.0)),
        residual1_start,
        duration * 0.09,
        easing,
    );
    events.push(TextEvent::text(
        data.status.clone(),
        format!(
            "first residual preserves the token lane · {} ordering",
            data.config.norm.label().to_ascii_lowercase()
        ),
        residual1_start,
    ));

    let mlp_start = duration * 0.68;
    for role in ["mlp", "activation", "dropout"] {
        reveal_role(
            scene,
            &mut tracks,
            &id,
            role,
            mlp_start,
            duration * 0.10,
            easing,
        );
    }
    events.push(TextEvent::text(
        data.status.clone(),
        format!(
            "MLP expands each token to {} values, applies {}, then projects back",
            data.config.mlp_width,
            data.config.activation.label()
        ),
        mlp_start,
    ));
    add_tracks(
        scene,
        &mut tracks,
        format!("{id}.flow3"),
        Prop::Flow,
        TargetValue::Rel(Value::F(1.0)),
        mlp_start,
        duration * 0.09,
        easing,
    );

    let finish = duration * 0.82;
    for role in ["residual2", "norm2", "output"] {
        reveal_role(
            scene,
            &mut tracks,
            &id,
            role,
            finish,
            duration * 0.10,
            easing,
        );
    }
    add_tracks(
        scene,
        &mut tracks,
        format!("{id}.residual2"),
        Prop::Flow,
        TargetValue::Rel(Value::F(1.0)),
        finish,
        duration * 0.10,
        easing,
    );
    for connector in [4, 5] {
        add_tracks(
            scene,
            &mut tracks,
            format!("{id}.flow{connector}"),
            Prop::Flow,
            TargetValue::Rel(Value::F(1.0)),
            finish,
            duration * 0.10,
            easing,
        );
    }
    events.push(TextEvent::text(
        data.status,
        format!(
            "block output keeps {} token identities · first row {}",
            data.tokens.len(),
            vector_summary(&data.output[0])
        ),
        finish,
    ));

    Ok(Clip {
        tracks,
        events,
        dur: duration,
    })
}

pub fn register(registry: &mut Registry) {
    registry.ctor("transformer", c_transformer);
    registry.mut_verb("encode", v_encode);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movie_with(config: &str) -> crate::movie::Movie {
        crate::parse(&format!(
            "tokenize(words,(400,100),\"a b c\",word,500);\
             embedding(context,words,(600,300),\"0.1 0.2 0.3 0.4; 0.5 -0.2 0.7 0.1; -0.4 0.8 0.2 0.6\",none,700,300);\
             transformer(b,context,(700,500),\"{config}\",1000,500);"
        ))
        .unwrap_or_else(|error| panic!("parse failed: {error:?}"))
    }

    fn frame(movie: &crate::movie::Movie, time: f32) -> Scene {
        let (base, timeline) = movie.finalize();
        timeline.apply(&base, time)
    }

    #[test]
    fn multi_head_causal_attention_is_normalized_and_masked() {
        let movie = movie_with(
            "heads=2 mask=causal mlp=6 activation=gelu norm=pre dropout=0 mode=inference seed=7",
        );
        let data = &movie.scene.ml_transformers["b"];
        assert_eq!(data.heads.len(), 2);
        assert!(data.heads.iter().all(|head| head.outputs[0].len() == 2));
        for head in &data.heads {
            for (row, weights) in head.weights.iter().enumerate() {
                assert!((weights.iter().sum::<f32>() - 1.0).abs() < 1e-5);
                for col in row + 1..weights.len() {
                    assert_eq!(weights[col], 0.0);
                    assert!(head.scores[row][col].is_infinite());
                }
            }
        }
        assert!(data
            .concatenated
            .iter()
            .all(|row| row.len() == data.input[0].len()));
    }

    #[test]
    fn pre_norm_residuals_and_mlp_values_are_exact() {
        let movie = movie_with(
            "heads=2 mask=none mlp=7 activation=silu norm=pre dropout=0 mode=inference seed=9",
        );
        let data = &movie.scene.ml_transformers["b"];
        for row in 0..data.input.len() {
            for axis in 0..data.input[0].len() {
                assert!(
                    (data.residual1[row][axis]
                        - data.input[row][axis]
                        - data.attention_output[row][axis])
                        .abs()
                        < 1e-5
                );
                assert!(
                    (data.output[row][axis]
                        - data.residual1[row][axis]
                        - data.mlp_output[row][axis])
                        .abs()
                        < 1e-5
                );
            }
            let mean = data.norm2[row].iter().sum::<f32>() / data.norm2[row].len() as f32;
            assert!(mean.abs() < 1e-5);
        }
    }

    #[test]
    fn post_norm_finishes_with_normalized_rows() {
        let movie = movie_with(
            "heads=1 mask=none mlp=5 activation=tanh norm=post dropout=0 mode=inference seed=12",
        );
        let data = &movie.scene.ml_transformers["b"];
        assert_eq!(data.output, data.norm2);
        for row in &data.output {
            let mean = row.iter().sum::<f32>() / row.len() as f32;
            assert!(mean.abs() < 1e-5);
        }
    }

    #[test]
    fn training_dropout_is_inverted_seeded_and_inference_is_disabled() {
        let config =
            "heads=2 mask=causal mlp=8 activation=relu norm=pre dropout=0.5 mode=training seed=21";
        let first = movie_with(config);
        let second = movie_with(config);
        let a = &first.scene.ml_transformers["b"];
        let b = &second.scene.ml_transformers["b"];
        assert_eq!(a.attention_dropout_mask, b.attention_dropout_mask);
        assert_eq!(a.mlp_dropout_mask, b.mlp_dropout_mask);
        assert_eq!(a.output, b.output);
        assert!(a.attention_dropout_mask.iter().flatten().any(|keep| !keep));
        for row in 0..a.attention_output.len() {
            for axis in 0..a.attention_output[0].len() {
                let expected = if a.attention_dropout_mask[row][axis] {
                    a.attention_output[row][axis] * 2.0
                } else {
                    0.0
                };
                assert!((a.attention_after_dropout[row][axis] - expected).abs() < 1e-5);
            }
        }

        let inference = movie_with(
            "heads=2 mask=causal mlp=8 activation=relu norm=pre dropout=0.5 mode=inference seed=21",
        );
        let inference = &inference.scene.ml_transformers["b"];
        assert_eq!(
            inference.attention_output,
            inference.attention_after_dropout
        );
        assert!(inference
            .attention_dropout_mask
            .iter()
            .flatten()
            .all(|keep| *keep));
    }

    #[test]
    fn every_ml6_activation_is_finite() {
        for activation in ["gelu", "relu", "silu", "tanh"] {
            let movie = movie_with(&format!(
                "heads=2 mask=none mlp=6 activation={activation} norm=pre dropout=0 mode=inference seed=4"
            ));
            assert!(movie.scene.ml_transformers["b"]
                .output
                .iter()
                .flatten()
                .all(|value| value.is_finite()));
        }
    }

    #[test]
    fn transformer_errors_name_the_failed_contract() {
        let parse_error = |config: &str| {
            let source = format!(
                "tokenize(words,(400,100),\"a b\",word,500);\
                 embedding(context,words,(600,300),\"seeded 6 2\",none,700,300);\
                 transformer(b,context,(700,500),\"{config}\",1000,500);"
            );
            match crate::parse(&source) {
                Ok(_) => panic!("expected transformer config to fail"),
                Err(error) => error.msg,
            }
        };
        assert!(parse_error("heads=4").contains("divide exactly"));
        assert!(parse_error("dropout=1").contains("less than 1"));
        assert!(parse_error("mask=future").contains("mask"));
        assert!(parse_error("activation=softmax").contains("activation"));
        assert!(parse_error("heads=2 heads=2").contains("repeated"));
        let missing = match crate::parse("encode(missing,2);") {
            Ok(_) => panic!("expected encode target to fail"),
            Err(error) => error.msg,
        };
        assert!(missing.contains("not a transformer"));
    }

    #[test]
    fn stable_tags_and_encode_are_seekable() {
        let movie = crate::parse(
            "tokenize(words,(400,100),\"a b c\",word,500);\
             embedding(context,words,(600,300),\"seeded 4 2\",sinusoidal,700,300);\
             transformer(b,context,(700,500),\"heads=2 mask=causal mlp=8 activation=gelu norm=pre dropout=0 mode=inference seed=4\",1000,500);\
             encode(b,5,smooth);",
        )
        .unwrap();
        for tag in [
            "b.input",
            "b.head0",
            "b.matrix",
            "b.mask",
            "b.concat",
            "b.projection",
            "b.residual1",
            "b.norm1",
            "b.mlp",
            "b.activation",
            "b.dropout.attention",
            "b.dropout.mlp",
            "b.residual2",
            "b.norm2",
            "b.output",
        ] {
            assert!(movie
                .scene
                .entities
                .iter()
                .any(|entity| entity.tags.iter().any(|candidate| candidate == tag)));
        }
        for id in [
            "b.project.summary",
            "b.residual1.summary",
            "b.mlp.summary",
            "b.residual2.summary",
            "b.skip1",
            "b.skip2",
        ] {
            assert!(
                movie.scene.get(id).is_some(),
                "missing visual evidence {id}"
            );
        }
        let late_first = frame(&movie, 4.7);
        let early = frame(&movie, 0.5);
        let late_again = frame(&movie, 4.7);
        assert_eq!(
            late_first.get("b.out0.box").unwrap().opacity,
            late_again.get("b.out0.box").unwrap().opacity
        );
        assert_eq!(
            late_first.get("b.status").unwrap().shape,
            late_again.get("b.status").unwrap().shape
        );
        assert_ne!(
            early.get("b.out0.box").unwrap().opacity,
            late_again.get("b.out0.box").unwrap().opacity
        );
    }

    #[test]
    fn ml6_story_keeps_token_lanes_and_text_visible_at_the_takeaway() {
        let movie = crate::parse(include_str!(
            "../../examples/manic-ml-transformer-block.manic"
        ))
        .unwrap();
        let (base, timeline) = movie.finalize();
        let frame = timeline.apply(&base, 12.2);
        for id in [
            "headline",
            "block.heading",
            "block.in0.label",
            "block.head0.title",
            "block.out0.label",
            "block.status",
            "caption",
        ] {
            let entity = frame
                .get(id)
                .unwrap_or_else(|| panic!("missing entity {id}"));
            assert!(entity.opacity > 0.9, "{id} opacity is {}", entity.opacity);
            assert!(entity.scale > 0.9, "{id} scale is {}", entity.scale);
        }
    }
}
