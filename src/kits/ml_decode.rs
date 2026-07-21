//! ML7: a language-model projection, temperature-scaled probabilities, and
//! deterministic decoding strategies on top of an ML6 transformer block.
//!
//! `logits` deliberately remains separate from the transformer's MLP: it
//! projects one final hidden row into an authored educational vocabulary, then
//! applies `softmax(logits / temperature)`. `sample` filters and renormalizes
//! that complete distribution before making one seeded choice. The renderer
//! receives only ordinary entities, text events, and absolute tracks.

use std::collections::HashSet;

use macroquad::prelude::{Color, Vec2};

use crate::easing::Easing;
use crate::lang::diag::Error;
use crate::lang::lower::{resolve_easing, Args, Registry};
use crate::primitives::{Entity, FontKind, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TextEvent, TrackSpec, Value};

const MAX_CANDIDATES: usize = 12;

#[derive(Debug, Clone)]
pub struct MlLogitsData {
    pub transformer: String,
    /// Zero-based row in the transformer's final hidden representation.
    pub token: usize,
    pub token_label: String,
    pub labels: Vec<String>,
    pub hidden: Vec<f32>,
    /// One output-projection row per authored candidate.
    pub projection: Vec<Vec<f32>>,
    pub bias: Vec<f32>,
    pub logits: Vec<f32>,
    pub temperature: f32,
    pub probabilities: Vec<f32>,
    pub seed: u64,
    pub bar_start_x: f32,
    pub bar_width: f32,
    pub row_y: Vec<f32>,
    pub bar_ids: Vec<String>,
    pub marker_ids: Vec<String>,
    pub probability_ids: Vec<String>,
    pub row_ids: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlSamplingStrategy {
    Greedy,
    Categorical,
    TopK,
    TopP,
}

impl MlSamplingStrategy {
    fn label(self) -> &'static str {
        match self {
            Self::Greedy => "GREEDY",
            Self::Categorical => "CATEGORICAL",
            Self::TopK => "TOP-K",
            Self::TopP => "TOP-P",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MlSampleData {
    pub logits: String,
    pub strategy: MlSamplingStrategy,
    pub parameter: Option<f32>,
    pub seed: u64,
    /// The exact filtered and renormalized distribution used for the draw.
    pub probabilities: Vec<f32>,
    pub selected: usize,
}

#[derive(Debug, Clone, Copy)]
struct SampleConfig {
    strategy: MlSamplingStrategy,
    parameter: Option<f32>,
    seed: Option<u64>,
}

fn lcg(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 40) as u32) as f32 / (1u32 << 24) as f32
}

fn generated_matrix(rows: usize, cols: usize, state: &mut u64) -> Vec<Vec<f32>> {
    let scale = (2.0 / cols as f32).sqrt();
    (0..rows)
        .map(|_| {
            (0..cols)
                .map(|_| (lcg(state) * 2.0 - 1.0) * scale)
                .collect()
        })
        .collect()
}

fn generated_bias(size: usize, state: &mut u64) -> Vec<f32> {
    (0..size).map(|_| (lcg(state) * 2.0 - 1.0) * 0.08).collect()
}

fn project(input: &[f32], weights: &[Vec<f32>], bias: &[f32]) -> Vec<f32> {
    weights
        .iter()
        .zip(bias)
        .map(|(row, bias)| {
            row.iter()
                .zip(input)
                .map(|(weight, value)| weight * value)
                .sum::<f32>()
                + bias
        })
        .collect()
}

fn stable_softmax(logits: &[f32], temperature: f32) -> Vec<f32> {
    let scaled: Vec<f32> = logits.iter().map(|value| value / temperature).collect();
    let max = scaled.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exponents: Vec<f32> = scaled.iter().map(|value| (value - max).exp()).collect();
    let sum = exponents.iter().sum::<f32>();
    exponents.into_iter().map(|value| value / sum).collect()
}

fn parse_labels(source: &str) -> Result<Vec<String>, String> {
    let labels: Vec<String> = source
        .split('|')
        .map(str::trim)
        .map(str::to_string)
        .collect();
    if labels.len() < 2 {
        return Err("logits needs at least two `|`-separated candidate labels".into());
    }
    if labels.len() > MAX_CANDIDATES {
        return Err(format!(
            "logits supports at most {MAX_CANDIDATES} candidate labels in one view"
        ));
    }
    if labels.iter().any(|label| label.is_empty()) {
        return Err("logits candidate labels cannot be empty".into());
    }
    let unique: HashSet<&str> = labels.iter().map(String::as_str).collect();
    if unique.len() != labels.len() {
        return Err("logits candidate labels must be unique".into());
    }
    Ok(labels)
}

fn parse_index(value: f32, noun: &str) -> Result<usize, String> {
    if !value.is_finite() || value < 1.0 || value.fract() != 0.0 {
        return Err(format!("{noun} must be a positive 1-based integer"));
    }
    Ok(value as usize - 1)
}

fn parse_seed(value: f32, noun: &str) -> Result<u64, String> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
        return Err(format!("{noun} must be a non-negative integer"));
    }
    Ok(value as u64)
}

fn parse_sample_config(source: &str) -> Result<SampleConfig, String> {
    let fields: Vec<&str> = source.split_whitespace().collect();
    let Some(strategy_word) = fields.first() else {
        return Err("sample strategy must be greedy, categorical, top-k K, or top-p P".into());
    };
    let strategy = match strategy_word.to_ascii_lowercase().as_str() {
        "greedy" => MlSamplingStrategy::Greedy,
        "categorical" | "random" => MlSamplingStrategy::Categorical,
        "top-k" | "topk" => MlSamplingStrategy::TopK,
        "top-p" | "topp" | "nucleus" => MlSamplingStrategy::TopP,
        _ => return Err("sample strategy must be greedy, categorical, top-k K, or top-p P".into()),
    };
    let mut parameter = None;
    let mut seed = None;
    for field in fields.iter().skip(1) {
        if let Some(raw) = field.strip_prefix("seed=") {
            if seed.is_some() {
                return Err("sample seed is repeated".into());
            }
            seed = Some(
                raw.parse::<u64>()
                    .map_err(|_| "sample seed must be a non-negative integer".to_string())?,
            );
        } else if parameter.is_none() {
            parameter = Some(
                field
                    .parse::<f32>()
                    .map_err(|_| format!("sample parameter `{field}` is not a number"))?,
            );
        } else {
            return Err(format!("unknown sample option `{field}`"));
        }
    }
    match strategy {
        MlSamplingStrategy::Greedy | MlSamplingStrategy::Categorical => {
            if parameter.is_some() {
                return Err(format!(
                    "{} sampling does not take a cutoff parameter",
                    strategy.label().to_ascii_lowercase()
                ));
            }
        }
        MlSamplingStrategy::TopK => {
            let value = parameter
                .ok_or_else(|| "top-k sampling needs K, for example `top-k 3`".to_string())?;
            if !value.is_finite() || value < 1.0 || value.fract() != 0.0 {
                return Err("top-k K must be a positive integer".into());
            }
        }
        MlSamplingStrategy::TopP => {
            let value = parameter
                .ok_or_else(|| "top-p sampling needs P, for example `top-p 0.9`".to_string())?;
            if !value.is_finite() || !(0.0..=1.0).contains(&value) || value == 0.0 {
                return Err("top-p P must be greater than 0 and at most 1".into());
            }
        }
    }
    Ok(SampleConfig {
        strategy,
        parameter,
        seed,
    })
}

fn sorted_indices(probabilities: &[f32]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..probabilities.len()).collect();
    indices.sort_by(|left, right| {
        probabilities[*right]
            .total_cmp(&probabilities[*left])
            .then_with(|| left.cmp(right))
    });
    indices
}

fn sample_distribution(
    probabilities: &[f32],
    config: SampleConfig,
    fallback_seed: u64,
) -> Result<MlSampleData, String> {
    let mut filtered = vec![0.0; probabilities.len()];
    match config.strategy {
        MlSamplingStrategy::Greedy => {
            let selected = sorted_indices(probabilities)[0];
            filtered[selected] = 1.0;
        }
        MlSamplingStrategy::Categorical => filtered.copy_from_slice(probabilities),
        MlSamplingStrategy::TopK => {
            let k = config.parameter.unwrap() as usize;
            if k > probabilities.len() {
                return Err(format!(
                    "top-k K={k} exceeds this view's {} candidates",
                    probabilities.len()
                ));
            }
            for index in sorted_indices(probabilities).into_iter().take(k) {
                filtered[index] = probabilities[index];
            }
        }
        MlSamplingStrategy::TopP => {
            let cutoff = config.parameter.unwrap();
            let mut cumulative = 0.0;
            for index in sorted_indices(probabilities) {
                filtered[index] = probabilities[index];
                cumulative += probabilities[index];
                if cumulative + 1e-7 >= cutoff {
                    break;
                }
            }
        }
    }
    let sum = filtered.iter().sum::<f32>();
    for probability in &mut filtered {
        *probability /= sum;
    }
    let seed = config.seed.unwrap_or(fallback_seed);
    let selected = if config.strategy == MlSamplingStrategy::Greedy {
        sorted_indices(&filtered)[0]
    } else {
        let mut state = seed.max(1);
        let draw = lcg(&mut state);
        let mut cumulative = 0.0;
        let mut selected = filtered.len() - 1;
        for (index, probability) in filtered.iter().enumerate() {
            cumulative += probability;
            if draw < cumulative {
                selected = index;
                break;
            }
        }
        selected
    };
    Ok(MlSampleData {
        logits: String::new(),
        strategy: config.strategy,
        parameter: config.parameter,
        seed,
        probabilities: filtered,
        selected,
    })
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

/// `logits(id, transformer, token, (cx,cy), "a | b | c", [temperature], [width], [height], [seed])`
fn c_logits(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(9)?;
    let id = args.ident(0)?;
    if scene.ml_logits.contains_key(&id) {
        return Err(Error::new(
            format!("logits view `{id}` already exists"),
            args.span_of(0),
        ));
    }
    let transformer_id = args.ident(1)?;
    let transformer = scene
        .ml_transformers
        .get(&transformer_id)
        .cloned()
        .ok_or_else(|| {
            Error::new(
                format!("`{transformer_id}` is not a transformer block; create it with `transformer` first"),
                args.span_of(1),
            )
        })?;
    let token = parse_index(args.num(2)?, "logits token")
        .map_err(|message| Error::new(message, args.span_of(2)))?;
    if token >= transformer.output.len() {
        return Err(Error::new(
            format!(
                "logits token {} exceeds transformer `{transformer_id}`'s {} tokens",
                token + 1,
                transformer.output.len()
            ),
            args.span_of(2),
        ));
    }
    let center = args.pair(3)?;
    let labels =
        parse_labels(&args.text(4)?).map_err(|message| Error::new(message, args.span_of(4)))?;
    let temperature = args.opt_num(5)?.unwrap_or(1.0);
    if !temperature.is_finite() || temperature <= 0.0 {
        return Err(Error::new(
            "logits temperature must be positive and finite",
            args.span_of(5),
        ));
    }
    let width = args.opt_num(6)?.unwrap_or(760.0);
    let height = args.opt_num(7)?.unwrap_or(440.0);
    if !width.is_finite() || width < 560.0 {
        return Err(Error::new(
            "logits width must be a finite number of at least 560",
            args.span_of(6),
        ));
    }
    if !height.is_finite() || height < 320.0 {
        return Err(Error::new(
            "logits height must be a finite number of at least 320",
            args.span_of(7),
        ));
    }
    let seed = args
        .opt_num(8)?
        .map(|value| parse_seed(value, "logits seed"))
        .transpose()
        .map_err(|message| Error::new(message, args.span_of(8)))?
        .unwrap_or(73);

    let hidden = transformer.output[token].clone();
    let mut state = seed.max(1);
    let projection = generated_matrix(labels.len(), hidden.len(), &mut state);
    let bias = generated_bias(labels.len(), &mut state);
    let logits = project(&hidden, &projection, &bias);
    let probabilities = stable_softmax(&logits, temperature);
    let token_label = transformer.tokens[token].clone();
    let status = format!("{id}.status");

    let mut panel = Entity::new(
        format!("{id}.panel"),
        Shape::Rect {
            w: width,
            h: height,
        },
        center,
        mix(style::VOID, style::PANEL, 0.74),
    );
    panel.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 1.5,
        outline_color: Some(mix(style::DIM, style::CYAN, 0.34)),
    };
    panel.z = 1;
    tag(&mut panel, &id, "structure");
    scene.add(panel);

    add_text(
        scene,
        &id,
        "labels",
        format!("{id}.heading"),
        "EDUCATIONAL LM PROJECTION".into(),
        Vec2::new(center.x, center.y - height * 0.43),
        16.0,
        style::DIM,
    );
    add_text(
        scene,
        &id,
        "projection",
        format!("{id}.source"),
        format!(
            "BLOCK OUTPUT [{}]  →  W_lm h + b  →  {} LOGITS",
            token_label,
            labels.len()
        ),
        Vec2::new(center.x, center.y - height * 0.34),
        14.0,
        style::FG,
    );
    add_text(
        scene,
        &id,
        "temperature",
        format!("{id}.temperature"),
        format!("softmax(logits / T)     T = {temperature:.2}"),
        Vec2::new(center.x, center.y - height * 0.265),
        15.0,
        style::GOLD,
    );
    add_text(
        scene,
        &id,
        "labels",
        format!("{id}.candidate_header"),
        "CANDIDATE".into(),
        Vec2::new(center.x - width * 0.37, center.y - height * 0.225),
        11.0,
        style::DIM,
    );
    add_text(
        scene,
        &id,
        "logits",
        format!("{id}.logit_header"),
        "LOGIT".into(),
        Vec2::new(center.x - width * 0.19, center.y - height * 0.225),
        11.0,
        style::DIM,
    );
    add_text(
        scene,
        &id,
        "probabilities",
        format!("{id}.prob_header"),
        "PROBABILITY".into(),
        Vec2::new(center.x + width * 0.23, center.y - height * 0.225),
        11.0,
        style::DIM,
    );

    let available_h = height * 0.42;
    let row_h = available_h / labels.len() as f32;
    let y0 = center.y - row_h * (labels.len().saturating_sub(1) as f32) * 0.5;
    let bar_start_x = center.x - width * 0.04;
    let bar_width = width * 0.42;
    let mut row_y = Vec::with_capacity(labels.len());
    let mut bar_ids = Vec::with_capacity(labels.len());
    let mut marker_ids = Vec::with_capacity(labels.len());
    let mut probability_ids = Vec::with_capacity(labels.len());
    let mut row_ids = Vec::with_capacity(labels.len());

    for (index, label) in labels.iter().enumerate() {
        let y = y0 + index as f32 * row_h;
        row_y.push(y);
        let row_id = format!("{id}.candidate{index}.row");
        let mut row = Entity::new(
            row_id.clone(),
            Shape::Rect {
                w: width * 0.90,
                h: (row_h * 0.72).max(18.0),
            },
            Vec2::new(center.x, y),
            mix(style::PANEL, style::CYAN, 0.035),
        );
        row.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 0.6,
            outline_color: Some(mix(style::DIM, style::PANEL, 0.48)),
        };
        row.z = 2;
        tag(&mut row, &id, "candidates");
        alias(&mut row, &id, &format!("candidate{index}"));
        alias(&mut row, &id, "sampling");
        scene.add(row);
        row_ids.push(row_id);

        add_text(
            scene,
            &id,
            "candidates",
            format!("{id}.candidate{index}.label"),
            label.clone(),
            Vec2::new(center.x - width * 0.37, y),
            (row_h * 0.31).clamp(10.0, 15.0),
            style::FG,
        );
        if let Some(entity) = scene.entities.last_mut() {
            alias(entity, &id, &format!("candidate{index}"));
        }
        add_text(
            scene,
            &id,
            "logits",
            format!("{id}.candidate{index}.logit"),
            format!("{:+.3}", logits[index]),
            Vec2::new(center.x - width * 0.19, y),
            (row_h * 0.29).clamp(9.0, 14.0),
            style::MAGENTA,
        );
        if let Some(entity) = scene.entities.last_mut() {
            alias(entity, &id, &format!("candidate{index}"));
        }

        let track_id = format!("{id}.candidate{index}.track");
        let mut probability_track = Entity::new(
            track_id,
            Shape::Line {
                to: Vec2::new(bar_start_x + bar_width, y),
            },
            Vec2::new(bar_start_x, y),
            mix(style::PANEL, style::DIM, 0.48),
        );
        probability_track.stroke = StrokeStyle {
            fill: false,
            outline: true,
            width: 2.0,
            outline_color: None,
        };
        probability_track.z = 4;
        tag(&mut probability_track, &id, "probabilities");
        alias(&mut probability_track, &id, &format!("candidate{index}"));
        scene.add(probability_track);

        let bar_id = format!("{id}.candidate{index}.bar");
        let endpoint = Vec2::new(bar_start_x + bar_width * probabilities[index], y);
        let mut bar = Entity::new(
            bar_id.clone(),
            Shape::Line { to: endpoint },
            Vec2::new(bar_start_x, y),
            style::CYAN,
        );
        bar.stroke = StrokeStyle {
            fill: false,
            outline: true,
            width: 5.0,
            outline_color: None,
        };
        bar.z = 7;
        tag(&mut bar, &id, "probabilities");
        alias(&mut bar, &id, "bars");
        alias(&mut bar, &id, &format!("candidate{index}"));
        scene.add(bar);
        bar_ids.push(bar_id);

        let marker_id = format!("{id}.candidate{index}.marker");
        let mut marker = Entity::new(
            marker_id.clone(),
            Shape::Circle { r: 5.0 },
            endpoint,
            style::CYAN,
        );
        marker.z = 9;
        tag(&mut marker, &id, "probabilities");
        alias(&mut marker, &id, "markers");
        alias(&mut marker, &id, &format!("candidate{index}"));
        scene.add(marker);
        marker_ids.push(marker_id);

        let probability_id = format!("{id}.candidate{index}.probability");
        add_text(
            scene,
            &id,
            "probabilities",
            probability_id.clone(),
            format!("{:>6.2}%", probabilities[index] * 100.0),
            Vec2::new(center.x + width * 0.42, y),
            (row_h * 0.29).clamp(9.0, 14.0),
            style::FG,
        );
        if let Some(entity) = scene.entities.last_mut() {
            alias(entity, &id, &format!("candidate{index}"));
        }
        probability_ids.push(probability_id);
    }

    add_text(
        scene,
        &id,
        "labels",
        status.clone(),
        format!(
            "Full softmax · {} candidates · probabilities sum to 1",
            labels.len()
        ),
        Vec2::new(center.x, center.y + height * 0.43),
        13.0,
        style::DIM,
    );

    scene.ml_logits.insert(
        id,
        MlLogitsData {
            transformer: transformer_id,
            token,
            token_label,
            labels,
            hidden,
            projection,
            bias,
            logits,
            temperature,
            probabilities,
            seed,
            bar_start_x,
            bar_width,
            row_y,
            bar_ids,
            marker_ids,
            probability_ids,
            row_ids,
            status,
        },
    );
    Ok(())
}

/// `sample(logits, "greedy|categorical|top-k K|top-p P [seed=N]", [duration], [ease])`
fn v_sample(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(4)?;
    let id = args.ident(0)?;
    let data = scene.ml_logits.get(&id).cloned().ok_or_else(|| {
        Error::new(
            format!("`{id}` is not a logits probability view"),
            args.span_of(0),
        )
    })?;
    let config_source = args.text(1)?;
    let config = parse_sample_config(&config_source)
        .map_err(|message| Error::new(message, args.span_of(1)))?;
    let duration = args.opt_num(2)?.unwrap_or(3.6);
    if !duration.is_finite() || duration <= 0.0 {
        return Err(Error::new(
            "sample duration must be positive and finite",
            args.span_of(2),
        ));
    }
    let easing = if args.len() > 3 {
        let word = args.ident(3)?;
        resolve_easing(&word, args.span_of(3))?
    } else {
        Easing::InOutCubic
    };
    let mut result = sample_distribution(&data.probabilities, config, data.seed.wrapping_add(101))
        .map_err(|message| Error::new(message, args.span_of(1)))?;
    result.logits = id.clone();

    let mut tracks = Vec::new();
    let mut events = Vec::new();
    add_tracks(
        scene,
        &mut tracks,
        id.clone(),
        Prop::Opacity,
        TargetValue::Abs(Value::F(0.14)),
        0.0,
        duration * 0.04,
        Easing::InOutCubic,
    );
    for (role, start) in [
        ("structure", 0.00),
        ("labels", 0.02),
        ("projection", 0.08),
        ("temperature", 0.18),
        ("logits", 0.28),
        ("probabilities", 0.40),
        ("candidates", 0.40),
    ] {
        reveal_role(
            scene,
            &mut tracks,
            &id,
            role,
            duration * start,
            duration * 0.10,
            easing,
        );
    }
    events.push(TextEvent::text(
        data.status.clone(),
        format!(
            "Projection kept separate · divide every logit by T={:.2}",
            data.temperature
        ),
        duration * 0.14,
    ));
    events.push(TextEvent::text(
        data.status.clone(),
        format!(
            "{} filtering · renormalize the exact allowed support",
            result.strategy.label()
        ),
        duration * 0.56,
    ));

    let filter_at = duration * 0.57;
    let filter_dur = duration * 0.18;
    for index in 0..data.labels.len() {
        let probability = result.probabilities[index];
        let endpoint = Vec2::new(
            data.bar_start_x + data.bar_width * probability,
            data.row_y[index],
        );
        tracks.push(track(
            data.bar_ids[index].clone(),
            Prop::To,
            TargetValue::Abs(Value::V(endpoint)),
            filter_at,
            filter_dur,
            easing,
        ));
        tracks.push(track(
            data.marker_ids[index].clone(),
            Prop::Pos,
            TargetValue::Abs(Value::V(endpoint)),
            filter_at,
            filter_dur,
            easing,
        ));
        let opacity = if probability == 0.0 { 0.13 } else { 1.0 };
        add_tracks(
            scene,
            &mut tracks,
            format!("{id}.candidate{index}"),
            Prop::Opacity,
            TargetValue::Abs(Value::F(opacity)),
            filter_at,
            filter_dur,
            easing,
        );
        events.push(TextEvent::text(
            data.probability_ids[index].clone(),
            format!("{:>6.2}%", probability * 100.0),
            filter_at + filter_dur,
        ));
    }
    tracks.push(track(
        data.row_ids[result.selected].clone(),
        Prop::Scale,
        TargetValue::Abs(Value::F(1.065)),
        duration * 0.78,
        duration * 0.10,
        Easing::OutBack,
    ));
    tracks.push(track(
        data.row_ids[result.selected].clone(),
        Prop::Color,
        TargetValue::Abs(Value::C(mix(style::PANEL, style::LIME, 0.28))),
        duration * 0.78,
        duration * 0.10,
        easing,
    ));
    events.push(TextEvent::text(
        data.status.clone(),
        format!(
            "Selected [{}] · {} · seed={} · sampled probability {:.2}%",
            data.labels[result.selected],
            result.strategy.label(),
            result.seed,
            result.probabilities[result.selected] * 100.0
        ),
        duration * 0.80,
    ));

    scene.ml_samples.insert(id, result);
    Ok(Clip {
        tracks,
        events,
        dur: duration,
    })
}

pub fn register(registry: &mut Registry) {
    registry.ctor("logits", c_logits);
    registry.mut_verb("sample", v_sample);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(temperature: f32, seed: u64) -> String {
        format!(
            "tokenize(words,(400,100),\"a b c\",word,500);\
             embedding(context,words,(600,300),\"0.1 0.2 0.3 0.4; 0.5 -0.2 0.7 0.1; -0.4 0.8 0.2 0.6\",none,700,300);\
             transformer(block,context,(700,500),\"heads=2 mask=causal mlp=8 activation=gelu norm=pre dropout=0 mode=inference seed=41\",1000,500);\
             logits(next,block,3,(700,500),\"red | blue | green | gold | black\",{temperature},700,400,{seed});"
        )
    }

    fn entropy(values: &[f32]) -> f32 {
        -values
            .iter()
            .filter(|value| **value > 0.0)
            .map(|value| value * value.ln())
            .sum::<f32>()
    }

    #[test]
    fn projection_is_separate_exact_and_temperature_changes_full_softmax() {
        let cool = crate::parse(&source(0.45, 73)).unwrap();
        let warm = crate::parse(&source(1.8, 73)).unwrap();
        let a = &cool.scene.ml_logits["next"];
        let b = &warm.scene.ml_logits["next"];
        assert_eq!(a.hidden, cool.scene.ml_transformers["block"].output[2]);
        assert_eq!(a.projection, b.projection);
        assert_eq!(a.bias, b.bias);
        assert_eq!(a.logits, b.logits);
        for index in 0..a.logits.len() {
            let exact = a.projection[index]
                .iter()
                .zip(&a.hidden)
                .map(|(weight, value)| weight * value)
                .sum::<f32>()
                + a.bias[index];
            assert!((a.logits[index] - exact).abs() < 1e-6);
            assert!((a.probabilities[index] - b.probabilities[index]).abs() > 1e-7);
        }
        assert!((a.probabilities.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!((b.probabilities.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!(entropy(&a.probabilities) < entropy(&b.probabilities));
    }

    #[test]
    fn every_sampling_strategy_filters_and_renormalizes_truthfully() {
        let probabilities = vec![0.40, 0.30, 0.15, 0.10, 0.05];
        let config = |source| parse_sample_config(source).unwrap();
        let greedy = sample_distribution(&probabilities, config("greedy"), 8).unwrap();
        assert_eq!(greedy.probabilities, vec![1.0, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(greedy.selected, 0);

        let categorical =
            sample_distribution(&probabilities, config("categorical seed=8"), 99).unwrap();
        assert_eq!(categorical.probabilities, probabilities);

        let topk = sample_distribution(&probabilities, config("top-k 2 seed=8"), 99).unwrap();
        assert_eq!(topk.probabilities.iter().filter(|p| **p > 0.0).count(), 2);
        assert_eq!(topk.probabilities[2], 0.0);
        assert!((topk.probabilities.iter().sum::<f32>() - 1.0).abs() < 1e-6);

        let topp = sample_distribution(&probabilities, config("top-p 0.65 seed=8"), 99).unwrap();
        assert_eq!(topp.probabilities.iter().filter(|p| **p > 0.0).count(), 2);
        assert_eq!(topp.probabilities[2], 0.0);
        assert!((topp.probabilities.iter().sum::<f32>() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn seeded_categorical_sampling_is_reproducible() {
        let probabilities = vec![0.2, 0.3, 0.5];
        let config = parse_sample_config("categorical seed=1234").unwrap();
        let a = sample_distribution(&probabilities, config, 9).unwrap();
        let b = sample_distribution(&probabilities, config, 70).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn top_p_keeps_the_smallest_ranked_prefix_reaching_the_cutoff() {
        let probabilities = vec![0.45, 0.25, 0.15, 0.10, 0.05];
        let result = sample_distribution(
            &probabilities,
            parse_sample_config("top-p 0.71 seed=3").unwrap(),
            9,
        )
        .unwrap();
        assert_eq!(result.probabilities.iter().filter(|p| **p > 0.0).count(), 3);
        assert_eq!(result.probabilities[3], 0.0);
    }

    #[test]
    fn diagnostics_name_invalid_temperature_labels_token_and_cutoff() {
        let base = source(1.0, 73);
        let error = |source: &str| match crate::parse(source) {
            Ok(_) => panic!("expected ML7 source to fail"),
            Err(error) => error.msg,
        };
        assert!(error(&base.replace(",1,700,400,73)", ",0,700,400,73)")).contains("temperature"));
        assert!(
            error(&base.replace("red | blue | green | gold | black", "red"))
                .contains("at least two")
        );
        assert!(error(&base.replace("block,3,", "block,7,")).contains("exceeds"));
        let sample = format!("{} sample(next,\"top-k 9\");", base);
        assert!(error(&sample).contains("exceeds"));
    }

    #[test]
    fn sample_emits_exact_filtered_endpoints_and_is_directly_seekable() {
        let movie = crate::parse(&format!(
            "{} hidden(next); sample(next,\"top-k 2 seed=17\",2,smooth);",
            source(0.8, 73)
        ))
        .unwrap();
        let result = movie.scene.ml_samples["next"].clone();
        let data = movie.scene.ml_logits["next"].clone();
        let (base, timeline) = movie.finalize();
        let end = timeline.apply(&base, timeline.dur);
        let _later = timeline.apply(&base, timeline.dur + 5.0);
        let end_again = timeline.apply(&base, timeline.dur);
        for index in 0..result.probabilities.len() {
            let expected = Vec2::new(
                data.bar_start_x + data.bar_width * result.probabilities[index],
                data.row_y[index],
            );
            let marker = end.get(&data.marker_ids[index]).unwrap();
            assert!((marker.pos - expected).length() < 1e-4);
            assert_eq!(
                marker.pos,
                end_again.get(&data.marker_ids[index]).unwrap().pos
            );
        }
    }

    #[test]
    fn stable_tags_expose_projection_temperature_distribution_and_candidates() {
        let movie = crate::parse(&source(1.0, 73)).unwrap();
        for tag in [
            "next.projection",
            "next.temperature",
            "next.logits",
            "next.probabilities",
            "next.candidate0",
            "next.bars",
        ] {
            assert!(movie
                .scene
                .entities
                .iter()
                .any(|entity| entity.tags.iter().any(|candidate| candidate == tag)));
        }
    }

    #[test]
    fn ml7_story_keeps_explanation_and_decision_visible_at_the_takeaway() {
        let source = include_str!("../../examples/manic-ml-logits-sampling.manic");
        let movie = crate::parse(source).unwrap();
        let (base, timeline) = movie.finalize();
        let frame = timeline.apply(&base, 14.4);
        for id in ["headline", "caption", "warm.status"] {
            let entity = frame.get(id).unwrap_or_else(|| panic!("missing `{id}`"));
            assert!(entity.opacity > 0.45, "`{id}` faded at the takeaway");
            match &entity.shape {
                Shape::Text { content, .. } => assert!(!content.trim().is_empty()),
                _ => panic!("`{id}` should remain native text"),
            }
        }
        assert!(frame
            .get("warm.status")
            .and_then(|entity| match &entity.shape {
                Shape::Text { content, .. } => Some(content.contains("Selected")),
                _ => None,
            })
            .unwrap_or(false));
    }
}
