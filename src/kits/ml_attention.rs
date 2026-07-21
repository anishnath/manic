//! ML4: small, truthful self-attention stories.
//!
//! `attention` computes one deterministic scaled dot-product self-attention
//! head from explicit token embeddings. `attend` focuses one query without
//! rebuilding the figure. `topk` projects that token's residual vector into an
//! authored vocabulary and shows real softmax probabilities. Everything is
//! lowered to ordinary entities and absolute tracks, so seeking stays pure.

use std::cmp::Ordering;

use macroquad::prelude::{Color, Vec2};

use crate::easing::Easing;
use crate::lang::diag::Error;
use crate::lang::lower::{resolve_easing, Args, Registry};
use crate::primitives::{Entity, FontKind, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TextEvent, TrackSpec, Value};

const MAX_TOKENS: usize = 8;
const MAX_EMBEDDING: usize = 8;
const MAX_VOCABULARY: usize = 16;

#[derive(Debug, Clone)]
pub struct MlAttentionData {
    pub tokens: Vec<String>,
    pub embeddings: Vec<Vec<f32>>,
    pub queries: Vec<Vec<f32>>,
    pub keys: Vec<Vec<f32>>,
    pub values: Vec<Vec<f32>>,
    pub weights: Vec<Vec<f32>>,
    pub outputs: Vec<Vec<f32>>,
    pub seed: u64,
    pub status: String,
}

fn split_tokens(src: &str) -> Vec<String> {
    let parts: Vec<&str> = if src.contains('|') {
        src.split('|').collect()
    } else {
        src.split(|ch: char| ch == ',' || ch.is_whitespace())
            .collect()
    };
    parts
        .into_iter()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_matrix(src: &str, noun: &str) -> Result<Vec<Vec<f32>>, String> {
    let mut matrix: Vec<Vec<f32>> = Vec::new();
    for (row_index, row) in src.split(';').enumerate() {
        if row.trim().is_empty() {
            continue;
        }
        let mut values = Vec::new();
        for word in row
            .split(|ch: char| ch == ',' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
        {
            let value = word
                .parse::<f32>()
                .map_err(|_| format!("{noun} value `{word}` is not a finite number"))?;
            if !value.is_finite() {
                return Err(format!("{noun} value `{word}` is not finite"));
            }
            values.push(value);
        }
        if values.is_empty() {
            return Err(format!("{noun} row {} has no values", row_index + 1));
        }
        if let Some(first) = matrix.first() {
            if values.len() != first.len() {
                return Err(format!(
                    "all {noun} rows must contain {} values; row {} contains {}",
                    first.len(),
                    row_index + 1,
                    values.len()
                ));
            }
        }
        matrix.push(values);
    }
    if matrix.is_empty() {
        return Err(format!("{noun} has no rows"));
    }
    Ok(matrix)
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

fn project(input: &[f32], weights: &[Vec<f32>]) -> Vec<f32> {
    weights
        .iter()
        .map(|row| {
            row.iter()
                .zip(input)
                .map(|(weight, value)| weight * value)
                .sum()
        })
        .collect()
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

fn compute_attention(
    embeddings: &[Vec<f32>],
    seed: u64,
) -> (
    Vec<Vec<f32>>,
    Vec<Vec<f32>>,
    Vec<Vec<f32>>,
    Vec<Vec<f32>>,
    Vec<Vec<f32>>,
) {
    let dimension = embeddings[0].len();
    let mut state = seed.max(1);
    let wq = generated_matrix(dimension, dimension, &mut state);
    let wk = generated_matrix(dimension, dimension, &mut state);
    let wv = generated_matrix(dimension, dimension, &mut state);
    let queries: Vec<_> = embeddings.iter().map(|row| project(row, &wq)).collect();
    let keys: Vec<_> = embeddings.iter().map(|row| project(row, &wk)).collect();
    let values: Vec<_> = embeddings.iter().map(|row| project(row, &wv)).collect();
    let scale = (dimension as f32).sqrt();
    let mut weights = Vec::with_capacity(embeddings.len());
    for query in &queries {
        let mut row: Vec<f32> = keys
            .iter()
            .map(|key| query.iter().zip(key).map(|(q, k)| q * k).sum::<f32>() / scale)
            .collect();
        stable_softmax(&mut row);
        weights.push(row);
    }
    let outputs = weights
        .iter()
        .map(|row| {
            (0..dimension)
                .map(|axis| {
                    row.iter()
                        .zip(&values)
                        .map(|(weight, value)| weight * value[axis])
                        .sum()
                })
                .collect()
        })
        .collect();
    (queries, keys, values, weights, outputs)
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

fn fmt(value: f32) -> String {
    if (value - value.round()).abs() < 1e-5 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
    }
}

fn vector_summary(values: &[f32]) -> String {
    let shown = values
        .iter()
        .take(4)
        .map(|value| fmt(*value))
        .collect::<Vec<_>>()
        .join(", ");
    if values.len() > 4 {
        format!("[{shown}, …]")
    } else {
        format!("[{shown}]")
    }
}

fn compact_vector_summary(values: &[f32]) -> String {
    let shown = values
        .iter()
        .take(2)
        .map(|value| format!("{value:+.2}"))
        .collect::<Vec<_>>()
        .join(" ");
    if values.len() > 2 {
        format!("[{shown} …]")
    } else {
        format!("[{shown}]")
    }
}

fn display_token(token: &str) -> String {
    let mut chars = token.chars();
    let shown: String = chars.by_ref().take(12).collect();
    if chars.next().is_some() {
        format!("{shown}…")
    } else {
        shown
    }
}

fn tag(entity: &mut Entity, id: &str, role: &str) {
    entity.tags.push(id.to_string());
    entity.tags.push(format!("{id}.{role}"));
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

fn seed_arg(args: &Args, index: usize, fallback: u64, noun: &str) -> Result<u64, Error> {
    let raw = args.opt_num(index)?.unwrap_or(fallback as f32);
    if !raw.is_finite() || raw < 0.0 || raw.fract().abs() > 1e-6 {
        return Err(Error::new(
            format!("{noun} seed must be a non-negative integer"),
            args.span_of(index),
        ));
    }
    Ok(raw as u64)
}

fn positive_index(args: &Args, index: usize, noun: &str) -> Result<usize, Error> {
    let raw = args.num(index)?;
    if !raw.is_finite() || raw < 1.0 || raw.fract().abs() > 1e-6 {
        return Err(Error::new(
            format!("{noun} must be a positive 1-based integer"),
            args.span_of(index),
        ));
    }
    Ok(raw as usize - 1)
}

/// `attention(id, (cx,cy), "tokens", "embedding rows", [width], [height], [seed])`
fn c_attention(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(7)?;
    let id = args.ident(0)?;
    if scene.ml_attention.contains_key(&id) {
        return Err(Error::new(
            format!("attention figure `{id}` already exists"),
            args.span_of(0),
        ));
    }
    let center = args.pair(1)?;
    let tokens = split_tokens(&args.text(2)?);
    if !(2..=MAX_TOKENS).contains(&tokens.len()) {
        return Err(Error::new(
            format!("attention needs 2–{MAX_TOKENS} tokens"),
            args.span_of(2),
        ));
    }
    let embeddings = parse_matrix(&args.text(3)?, "embedding")
        .map_err(|message| Error::new(message, args.span_of(3)))?;
    if embeddings.len() != tokens.len() {
        return Err(Error::new(
            format!(
                "attention has {} token(s) but {} embedding row(s)",
                tokens.len(),
                embeddings.len()
            ),
            args.span_of(3),
        ));
    }
    let dimension = embeddings[0].len();
    if dimension > MAX_EMBEDDING {
        return Err(Error::new(
            format!("attention embeddings support at most {MAX_EMBEDDING} values per token"),
            args.span_of(3),
        ));
    }
    let width = args.opt_num(4)?.unwrap_or(900.0);
    let height = args.opt_num(5)?.unwrap_or(430.0);
    if !width.is_finite() || !height.is_finite() || width < 560.0 || height < 280.0 {
        return Err(Error::new(
            "attention width must be at least 560 and height at least 280",
            args.span_of(if width < 560.0 { 4 } else { 5 }),
        ));
    }
    let seed = seed_arg(args, 6, 17, "attention")?;
    let (queries, keys, values, weights, outputs) = compute_attention(&embeddings, seed);

    let count = tokens.len();
    let y0 = center.y - height * 0.32;
    let dy = height * 0.64 / (count.saturating_sub(1).max(1) as f32);
    let input_x = center.x - width * 0.43;
    let q_x = center.x - width * 0.28;
    let k_x = center.x - width * 0.21;
    let v_x = center.x - width * 0.14;
    let heatmap_center = Vec2::new(center.x + width * 0.07, center.y);
    let output_x = center.x + width * 0.43;
    let cell = (height * 0.58 / count as f32)
        .min(width * 0.27 / count as f32)
        .clamp(24.0, 52.0);
    let token_w = (width * 0.13).clamp(84.0, 142.0);
    let token_h = (dy * 0.48).clamp(24.0, 40.0);

    add_text(
        scene,
        &id,
        "labels",
        format!("{id}.heading"),
        format!("SELF-ATTENTION · 1 HEAD · d={dimension} · seed {seed}"),
        Vec2::new(center.x, center.y - height * 0.49),
        18.0,
        style::DIM,
    );
    for (label, x, color) in [
        ("Q", q_x, style::CYAN),
        ("K", k_x, style::MAGENTA),
        ("V", v_x, style::LIME),
    ] {
        add_text(
            scene,
            &id,
            &label.to_ascii_lowercase(),
            format!("{id}.{label}.header"),
            label.into(),
            Vec2::new(x, center.y - height * 0.40),
            17.0,
            color,
        );
    }
    add_text(
        scene,
        &id,
        "matrix",
        format!("{id}.matrix.header"),
        "SOFTMAX(QK^T / sqrt d)".into(),
        Vec2::new(heatmap_center.x, center.y - height * 0.40),
        16.0,
        style::DIM,
    );
    add_text(
        scene,
        &id,
        "outputs",
        format!("{id}.output.header"),
        "WEIGHTED V".into(),
        Vec2::new(output_x, center.y - height * 0.40),
        16.0,
        style::DIM,
    );

    for index in 0..count {
        let y = y0 + index as f32 * dy;
        let mut token = Entity::new(
            format!("{id}.token{index}.box"),
            Shape::Rect {
                w: token_w,
                h: token_h,
            },
            Vec2::new(input_x, y),
            mix(style::PANEL, style::CYAN, 0.15),
        );
        token.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 1.5,
            outline_color: Some(style::DIM),
        };
        tag(&mut token, &id, "tokens");
        token.tags.push(format!("{id}.token{index}"));
        scene.add(token);
        let mut token_text = Entity::new(
            format!("{id}.token{index}.text"),
            Shape::Text {
                content: display_token(&tokens[index]),
                size: 15.0,
            },
            Vec2::new(input_x, y),
            style::FG,
        );
        token_text.font = FontKind::MonoBold;
        token_text.z = 5;
        tag(&mut token_text, &id, "tokens");
        token_text.tags.push(format!("{id}.token{index}"));
        scene.add(token_text);

        for (role, x, color) in [
            ("q", q_x, style::CYAN),
            ("k", k_x, style::MAGENTA),
            ("v", v_x, style::LIME),
        ] {
            let mut node = Entity::new(
                format!("{id}.{role}{index}"),
                Shape::Circle { r: 8.0 },
                Vec2::new(x, y),
                color,
            );
            node.glow = 0.35;
            node.opacity = 0.58;
            node.z = 5;
            tag(&mut node, &id, role);
            node.tags.push(format!("{id}.{role}{index}"));
            scene.add(node);
        }

        let mut output = Entity::new(
            format!("{id}.out{index}.box"),
            Shape::Rect {
                w: token_w * 0.90,
                h: token_h,
            },
            Vec2::new(output_x, y),
            mix(style::PANEL, style::GOLD, 0.18),
        );
        output.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 1.5,
            outline_color: Some(style::DIM),
        };
        output.opacity = 0.45;
        tag(&mut output, &id, "outputs");
        output.tags.push(format!("{id}.out{index}"));
        scene.add(output);
        let mut output_text = Entity::new(
            format!("{id}.out{index}.text"),
            Shape::Text {
                content: compact_vector_summary(&outputs[index]),
                size: (token_w * 0.072).clamp(8.0, 10.5),
            },
            Vec2::new(output_x, y),
            style::GOLD,
        );
        output_text.font = FontKind::MonoBold;
        output_text.opacity = 0.55;
        output_text.z = 5;
        tag(&mut output_text, &id, "outputs");
        output_text.tags.push(format!("{id}.out{index}"));
        scene.add(output_text);

        let start = Vec2::new(input_x + token_w * 0.5, y);
        let end = Vec2::new(output_x - token_w * 0.45, y);
        let mut residual = Entity::new(
            format!("{id}.residual{index}"),
            Shape::Curve {
                ctrl: Vec2::new(center.x, center.y - height * 0.47 - index as f32 * 2.0),
                to: end,
                arrow: true,
            },
            start,
            style::DIM,
        );
        residual.stroke.width = 1.4;
        residual.opacity = 0.07;
        residual.z = 0;
        tag(&mut residual, &id, "residual");
        residual.tags.push(format!("{id}.residual{index}"));
        scene.add(residual);
    }

    for row in 0..count {
        for col in 0..count {
            let pos = heatmap_center
                + Vec2::new(
                    (col as f32 - (count - 1) as f32 * 0.5) * cell,
                    (row as f32 - (count - 1) as f32 * 0.5) * cell,
                );
            let weight = weights[row][col];
            let mut cell_entity = Entity::new(
                format!("{id}.weight{row}.{col}.cell"),
                Shape::Rect {
                    w: cell * 0.88,
                    h: cell * 0.88,
                },
                pos,
                mix(style::PANEL, style::MAGENTA, 0.12 + weight * 0.80),
            );
            cell_entity.stroke = StrokeStyle {
                fill: true,
                outline: true,
                width: 1.0,
                outline_color: Some(mix(style::DIM, style::MAGENTA, weight)),
            };
            cell_entity.opacity = 0.55;
            cell_entity.z = 3;
            tag(&mut cell_entity, &id, "matrix");
            cell_entity.tags.push(format!("{id}.row{row}"));
            cell_entity.tags.push(format!("{id}.weight{row}.{col}"));
            scene.add(cell_entity);
            let mut value = Entity::new(
                format!("{id}.weight{row}.{col}.value"),
                Shape::Text {
                    content: format!("{:.0}", weight * 100.0),
                    size: (cell * 0.27).clamp(9.0, 14.0),
                },
                pos,
                style::FG,
            );
            value.font = FontKind::MonoBold;
            value.opacity = 0.62;
            value.z = 4;
            tag(&mut value, &id, "matrix");
            value.tags.push(format!("{id}.row{row}"));
            value.tags.push(format!("{id}.weight{row}.{col}"));
            scene.add(value);

            let y_key = y0 + col as f32 * dy;
            let mut key_link = Entity::new(
                format!("{id}.keylink{row}.{col}"),
                Shape::Line { to: pos },
                Vec2::new(q_x, y0 + row as f32 * dy),
                style::MAGENTA,
            );
            key_link.stroke.width = 1.2;
            key_link.opacity = 0.025;
            key_link.z = 1;
            tag(&mut key_link, &id, "connections");
            key_link.tags.push(format!("{id}.fan{row}.{col}"));
            scene.add(key_link);

            let mut value_link = Entity::new(
                format!("{id}.valuelink{row}.{col}"),
                Shape::Line {
                    to: Vec2::new(output_x - token_w * 0.45, y0 + row as f32 * dy),
                },
                Vec2::new(v_x, y_key),
                style::CYAN,
            );
            value_link.stroke.width = 1.2;
            value_link.opacity = 0.025;
            value_link.z = 1;
            tag(&mut value_link, &id, "connections");
            value_link.tags.push(format!("{id}.fan{row}.{col}"));
            scene.add(value_link);
        }
    }

    let status = format!("{id}.status");
    let mut status_entity = Entity::new(
        status.clone(),
        Shape::Text {
            content: "Choose a token · Q asks · K matches · V contributes".into(),
            size: 17.0,
        },
        Vec2::new(center.x, center.y + height * 0.48),
        style::DIM,
    );
    status_entity.font = FontKind::MonoBold;
    status_entity.z = 20;
    tag(&mut status_entity, &id, "labels");
    scene.add(status_entity);

    scene.ml_attention.insert(
        id,
        MlAttentionData {
            tokens,
            embeddings,
            queries,
            keys,
            values,
            weights,
            outputs,
            seed,
            status,
        },
    );
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
            .map(|entity| track(entity.id.clone(), prop, target, start, dur, easing)),
    );
}

/// `attend(attention, token_1_based, [duration], [ease])`
fn v_attend(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(4)?;
    let id = args.ident(0)?;
    let data = scene.ml_attention.get(&id).cloned().ok_or_else(|| {
        Error::new(
            format!("`{id}` is not an attention figure"),
            args.span_of(0),
        )
    })?;
    let focus = positive_index(args, 1, "attention token index")?;
    if focus >= data.tokens.len() {
        return Err(Error::new(
            format!(
                "attention token {} is out of range; `{id}` has {} token(s)",
                focus + 1,
                data.tokens.len()
            ),
            args.span_of(1),
        ));
    }
    let duration = args.opt_num(2)?.unwrap_or(4.2);
    if !duration.is_finite() || duration <= 0.0 {
        return Err(Error::new(
            "attend duration must be positive and finite",
            args.span_of(2),
        ));
    }
    let easing = if args.len() > 3 {
        let word = args.ident(3)?;
        resolve_easing(&word, args.span_of(3))?
    } else {
        Easing::InOutCubic
    };
    let count = data.tokens.len();
    let beat = duration * 0.66 / count as f32;
    let start_fan = duration * 0.16;
    let mut tracks = Vec::new();
    let mut events = Vec::new();

    for role in [
        "tokens",
        "q",
        "k",
        "v",
        "matrix",
        "outputs",
        "connections",
        "residual",
    ] {
        add_tracks(
            scene,
            &mut tracks,
            format!("{id}.{role}"),
            Prop::Opacity,
            TargetValue::Abs(Value::F(if role == "connections" { 0.02 } else { 0.20 })),
            0.0,
            duration * 0.10,
            Easing::InOutCubic,
        );
    }
    for selected in [format!("{id}.token{focus}"), format!("{id}.q{focus}")] {
        add_tracks(
            scene,
            &mut tracks,
            selected.clone(),
            Prop::Opacity,
            TargetValue::Abs(Value::F(1.0)),
            duration * 0.04,
            duration * 0.10,
            Easing::OutQuad,
        );
        add_tracks(
            scene,
            &mut tracks,
            selected.clone(),
            Prop::Scale,
            TargetValue::Abs(Value::F(1.12)),
            duration * 0.04,
            duration * 0.10,
            Easing::OutQuad,
        );
        add_tracks(
            scene,
            &mut tracks,
            selected,
            Prop::Scale,
            TargetValue::Abs(Value::F(1.0)),
            duration * 0.15,
            duration * 0.10,
            easing,
        );
    }
    events.push(TextEvent::text(
        data.status.clone(),
        format!(
            "{} asks with Q = {}",
            data.tokens[focus],
            vector_summary(&data.queries[focus])
        ),
        duration * 0.05,
    ));

    let max_weight = data.weights[focus]
        .iter()
        .copied()
        .fold(0.0f32, f32::max)
        .max(1e-6);
    for target in 0..count {
        let start = start_fan + target as f32 * beat;
        let weight = data.weights[focus][target];
        let emphasis = 0.30 + 0.70 * weight / max_weight;
        for selected in [
            format!("{id}.k{target}"),
            format!("{id}.v{target}"),
            format!("{id}.weight{focus}.{target}"),
        ] {
            add_tracks(
                scene,
                &mut tracks,
                selected,
                Prop::Opacity,
                TargetValue::Abs(Value::F(0.45 + emphasis * 0.55)),
                start,
                beat * 0.45,
                easing,
            );
        }
        add_tracks(
            scene,
            &mut tracks,
            format!("{id}.fan{focus}.{target}"),
            Prop::Opacity,
            TargetValue::Abs(Value::F(0.10 + emphasis * 0.72)),
            start,
            beat * 0.45,
            easing,
        );
        add_tracks(
            scene,
            &mut tracks,
            format!("{id}.fan{focus}.{target}"),
            Prop::Flow,
            TargetValue::Rel(Value::F(1.0)),
            start + beat * 0.08,
            beat * 0.62,
            easing,
        );
        events.push(TextEvent::text(
            data.status.clone(),
            format!(
                "{} → {} · softmax weight {:.1}% · V {}",
                data.tokens[focus],
                data.tokens[target],
                weight * 100.0,
                vector_summary(&data.values[target])
            ),
            start + beat * 0.25,
        ));
    }
    let finish = duration * 0.84;
    for selected in [format!("{id}.out{focus}"), format!("{id}.residual{focus}")] {
        add_tracks(
            scene,
            &mut tracks,
            selected,
            Prop::Opacity,
            TargetValue::Abs(Value::F(1.0)),
            finish,
            duration * 0.10,
            Easing::OutQuad,
        );
    }
    add_tracks(
        scene,
        &mut tracks,
        format!("{id}.residual{focus}"),
        Prop::Flow,
        TargetValue::Rel(Value::F(1.0)),
        finish,
        duration * 0.10,
        easing,
    );
    add_tracks(
        scene,
        &mut tracks,
        format!("{id}.out{focus}"),
        Prop::Scale,
        TargetValue::Abs(Value::F(1.12)),
        finish,
        duration * 0.07,
        Easing::OutQuad,
    );
    add_tracks(
        scene,
        &mut tracks,
        format!("{id}.out{focus}"),
        Prop::Scale,
        TargetValue::Abs(Value::F(1.0)),
        finish + duration * 0.07,
        duration * 0.07,
        easing,
    );
    events.push(TextEvent::text(
        data.status,
        format!(
            "weighted V mix for {} = {} · residual lane remains visible",
            data.tokens[focus],
            vector_summary(&data.outputs[focus])
        ),
        finish,
    ));
    Ok(Clip {
        tracks,
        events,
        dur: duration,
    })
}

/// `topk(id, attention, token_1_based, (cx,cy), "labels", [k], [width], [height], [seed])`
fn c_topk(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(9)?;
    let id = args.ident(0)?;
    let attention_id = args.ident(1)?;
    let data = scene
        .ml_attention
        .get(&attention_id)
        .cloned()
        .ok_or_else(|| {
            Error::new(
                format!("`{attention_id}` is not an attention figure"),
                args.span_of(1),
            )
        })?;
    let focus = positive_index(args, 2, "top-k token index")?;
    if focus >= data.tokens.len() {
        return Err(Error::new(
            format!("top-k token {} is out of range", focus + 1),
            args.span_of(2),
        ));
    }
    let center = args.pair(3)?;
    let labels = split_tokens(&args.text(4)?);
    if labels.len() < 2 || labels.len() > MAX_VOCABULARY {
        return Err(Error::new(
            format!("topk needs 2–{MAX_VOCABULARY} candidate labels"),
            args.span_of(4),
        ));
    }
    let k_raw = args.opt_num(5)?.unwrap_or(3.0);
    if !k_raw.is_finite() || k_raw < 1.0 || k_raw.fract().abs() > 1e-6 {
        return Err(Error::new(
            "top-k count must be a positive integer",
            args.span_of(5),
        ));
    }
    let k = k_raw as usize;
    if k > labels.len() || k > 8 {
        return Err(Error::new(
            format!("top-k count must be at most {}", labels.len().min(8)),
            args.span_of(5),
        ));
    }
    let width = args.opt_num(6)?.unwrap_or(420.0);
    let height = args.opt_num(7)?.unwrap_or(230.0);
    if !width.is_finite() || !height.is_finite() || width < 260.0 || height < 150.0 {
        return Err(Error::new(
            "topk width must be at least 260 and height at least 150",
            args.span_of(if width < 260.0 { 6 } else { 7 }),
        ));
    }
    let seed = seed_arg(args, 8, data.seed.wrapping_add(101), "top-k")?;
    let residual: Vec<f32> = data.embeddings[focus]
        .iter()
        .zip(&data.outputs[focus])
        .map(|(input, output)| input + output)
        .collect();
    let mut state = seed.max(1);
    let projection = generated_matrix(labels.len(), residual.len(), &mut state);
    let mut logits: Vec<f32> = projection
        .iter()
        .map(|row| row.iter().zip(&residual).map(|(w, x)| w * x).sum())
        .collect();
    for logit in &mut logits {
        *logit += (lcg(&mut state) * 2.0 - 1.0) * 0.06;
    }
    stable_softmax(&mut logits);
    let mut order: Vec<usize> = (0..labels.len()).collect();
    order.sort_by(|left, right| {
        logits[*right]
            .partial_cmp(&logits[*left])
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.cmp(right))
    });

    add_text(
        scene,
        &id,
        "labels",
        format!("{id}.title"),
        format!("TOP {k} · RESIDUAL → OUTPUT SOFTMAX"),
        Vec2::new(center.x, center.y - height * 0.57),
        17.0,
        style::DIM,
    );
    let row_h = height / k as f32;
    let label_x = center.x - width * 0.30;
    let bar_x = center.x - width * 0.12;
    let max_bar = width * 0.55;
    for (rank, &candidate) in order.iter().take(k).enumerate() {
        let y = center.y - height * 0.5 + row_h * (rank as f32 + 0.5);
        let probability = logits[candidate];
        let mut background = Entity::new(
            format!("{id}.rank{rank}.track"),
            Shape::Rect {
                w: max_bar,
                h: row_h * 0.28,
            },
            Vec2::new(bar_x + max_bar * 0.5, y),
            style::PANEL,
        );
        background.opacity = 0.45;
        tag(&mut background, &id, "probabilities");
        background.tags.push(format!("{id}.rank{rank}"));
        scene.add(background);
        let bar_width = (max_bar * probability).max(3.0);
        let mut bar = Entity::new(
            format!("{id}.rank{rank}.bar"),
            Shape::Rect {
                w: bar_width,
                h: row_h * 0.28,
            },
            Vec2::new(bar_x + bar_width * 0.5, y),
            if rank == 0 { style::GOLD } else { style::CYAN },
        );
        bar.glow = if rank == 0 { 0.35 } else { 0.12 };
        tag(&mut bar, &id, "bars");
        bar.tags.push(format!("{id}.rank{rank}"));
        scene.add(bar);
        let mut label = Entity::new(
            format!("{id}.rank{rank}.label"),
            Shape::Text {
                content: display_token(&labels[candidate]),
                size: 16.0,
            },
            Vec2::new(label_x, y),
            style::FG,
        );
        label.font = FontKind::MonoBold;
        tag(&mut label, &id, "labels");
        label.tags.push(format!("{id}.rank{rank}"));
        scene.add(label);
        let mut readout = Entity::new(
            format!("{id}.rank{rank}.value"),
            Shape::Text {
                content: format!("{:.1}%", probability * 100.0),
                size: 15.0,
            },
            Vec2::new(center.x + width * 0.43, y),
            if rank == 0 { style::GOLD } else { style::DIM },
        );
        readout.font = FontKind::MonoBold;
        tag(&mut readout, &id, "probabilities");
        readout.tags.push(format!("{id}.rank{rank}"));
        scene.add(readout);
    }
    add_text(
        scene,
        &id,
        "labels",
        format!("{id}.status"),
        format!("{} residual → seeded projection {seed}", data.tokens[focus]),
        Vec2::new(center.x, center.y + height * 0.58),
        14.0,
        style::DIM,
    );
    Ok(())
}

pub fn register(registry: &mut Registry) {
    registry.ctor("attention", c_attention);
    registry.mut_verb("attend", v_attend);
    registry.ctor("topk", c_topk);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movie::Movie;
    use crate::scene::Scene;

    fn movie(src: &str) -> Movie {
        crate::parse(src).unwrap_or_else(|error| panic!("parse failed: {error:?}"))
    }

    fn frame(movie: &Movie, t: f32) -> Scene {
        let (base, timeline) = movie.finalize();
        timeline.apply(&base, t)
    }

    #[test]
    fn attention_rows_are_finite_normalized_and_outputs_are_exact_mixes() {
        let movie =
            movie("attention(a,(640,360),\"one two three\",\"1 0 0; 0 1 0; 0 0 1\",900,430,19);");
        let data = &movie.scene.ml_attention["a"];
        for row in &data.weights {
            assert!((row.iter().sum::<f32>() - 1.0).abs() < 1e-5);
            assert!(row.iter().all(|value| value.is_finite() && *value >= 0.0));
        }
        for token in 0..data.tokens.len() {
            for axis in 0..data.outputs[token].len() {
                let expected: f32 = data.weights[token]
                    .iter()
                    .zip(&data.values)
                    .map(|(weight, value)| weight * value[axis])
                    .sum();
                assert!((data.outputs[token][axis] - expected).abs() < 1e-5);
            }
            let label = match &movie
                .scene
                .get(&format!("a.out{token}.text"))
                .expect("weighted-value summary exists")
                .shape
            {
                Shape::Text { content, .. } => content,
                _ => panic!("weighted-value summary should be text"),
            };
            assert!(label.starts_with('[') && label.contains('…'));
            assert_ne!(label, "mix");
        }
    }

    #[test]
    fn seeded_attention_is_reproducible() {
        let source = "attention(a,(640,360),\"one two\",\"1 2; 3 4\",700,360,7);";
        let first = movie(source);
        let second = movie(source);
        assert_eq!(
            first.scene.ml_attention["a"].weights,
            second.scene.ml_attention["a"].weights
        );
        assert_eq!(
            first.scene.ml_attention["a"].outputs,
            second.scene.ml_attention["a"].outputs
        );
    }

    #[test]
    fn attend_is_stateless_when_seeking_out_of_order() {
        let movie = movie(
            "attention(a,(640,360),\"one two three\",\"1 0; 0 1; 1 1\",900,430,9); attend(a,2,3,smooth);",
        );
        let late_first = frame(&movie, 2.4);
        let early = frame(&movie, 0.4);
        let late_again = frame(&movie, 2.4);
        assert_eq!(
            late_first.get("a.out1.box").unwrap().opacity,
            late_again.get("a.out1.box").unwrap().opacity
        );
        assert_eq!(
            late_first.get("a.status").unwrap().shape,
            late_again.get("a.status").unwrap().shape
        );
        assert_ne!(
            early.get("a.weight1.0.cell").unwrap().opacity,
            late_again.get("a.weight1.0.cell").unwrap().opacity
        );
    }

    #[test]
    fn topk_probabilities_are_sorted_and_keep_full_softmax_percentages() {
        let movie = movie(
            "attention(a,(500,300),\"one two\",\"1 0; 0 1\",700,330,4); topk(p,a,1,(1000,300),\"red blue green yellow\",3,360,210,8);",
        );
        let text = |id: &str| match &movie.scene.get(id).unwrap().shape {
            Shape::Text { content, .. } => content.clone(),
            _ => panic!("{id} should be text"),
        };
        let values: Vec<f32> = (0..3)
            .map(|rank| {
                text(&format!("p.rank{rank}.value"))
                    .trim_end_matches('%')
                    .parse::<f32>()
                    .unwrap()
            })
            .collect();
        assert!(values.windows(2).all(|pair| pair[0] >= pair[1]));
        assert!(values.iter().all(|value| *value > 0.0 && *value < 100.0));
    }

    #[test]
    fn malformed_attention_and_focus_fail_early() {
        assert!(crate::parse("attention(a,(0,0),\"one\",\"1 2\");").is_err());
        assert!(crate::parse("attention(a,(0,0),\"one two\",\"1 2; 3\",700,330);").is_err());
        assert!(
            crate::parse("attention(a,(0,0),\"one two\",\"1 2; 3 4\",700,330); attend(a,3);")
                .is_err()
        );
        assert!(crate::parse(
            "attention(a,(0,0),\"one two\",\"1 2; 3 4\",700,330); topk(p,a,1,(0,0),\"yes no\",3);"
        )
        .is_err());
    }
}
