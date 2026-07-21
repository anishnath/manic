//! ML5: truthful token boundaries, educational embeddings, and position.
//!
//! `tokenize` creates a small deterministic token sequence. `embedding` turns
//! that sequence into explicit or seeded educational vectors, adds either no
//! position or the standard sinusoidal encoding, and exposes every stage as
//! ordinary tagged entities. No renderer-side ML runtime is introduced.

use macroquad::prelude::{Color, Vec2};

use crate::lang::diag::Error;
use crate::lang::lower::{Args, Registry};
use crate::primitives::{Entity, FontKind, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;

const MAX_TOKENS: usize = 12;
const MAX_DIMENSION: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlTokenMode {
    Authored,
    Word,
    Character,
}

impl MlTokenMode {
    fn name(self) -> &'static str {
        match self {
            Self::Authored => "AUTHORED",
            Self::Word => "WORD",
            Self::Character => "CHARACTER",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MlTokenData {
    pub tokens: Vec<String>,
    pub mode: MlTokenMode,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MlEmbeddingSource {
    Explicit,
    Seeded { seed: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlPositionMode {
    None,
    Sinusoidal,
}

impl MlPositionMode {
    fn name(self) -> &'static str {
        match self {
            Self::None => "NO POSITION",
            Self::Sinusoidal => "SINUSOIDAL POSITION",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MlEmbeddingData {
    pub token_sequence: String,
    pub tokens: Vec<String>,
    pub token_vectors: Vec<Vec<f32>>,
    pub position_vectors: Vec<Vec<f32>>,
    pub combined_vectors: Vec<Vec<f32>>,
    pub dimension: usize,
    pub source: MlEmbeddingSource,
    pub position: MlPositionMode,
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn tag(entity: &mut Entity, root: &str, role: &str) {
    entity.tags.push(root.to_string());
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

fn display_token(token: &str) -> String {
    let visible = token.replace(' ', "␠").replace('\t', "⇥");
    let mut chars = visible.chars();
    let short: String = chars.by_ref().take(12).collect();
    if chars.next().is_some() {
        format!("{short}…")
    } else {
        short
    }
}

fn authored_tokens(source: &str) -> Result<Vec<String>, String> {
    if !source.contains('|') {
        return Err("authored tokenization needs `|` between token boundaries".into());
    }
    let mut tokens = Vec::new();
    for (index, part) in source.split('|').enumerate() {
        let token = part.trim();
        if token.is_empty() {
            return Err(format!(
                "authored token {} is empty; remove the extra `|` or provide a token",
                index + 1
            ));
        }
        tokens.push(token.to_string());
    }
    Ok(tokens)
}

fn word_tokens(source: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    for ch in source.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            word.push(ch);
            continue;
        }
        if !word.is_empty() {
            tokens.push(std::mem::take(&mut word));
        }
        if !ch.is_whitespace() {
            tokens.push(ch.to_string());
        }
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    tokens
}

fn character_tokens(source: &str) -> Vec<String> {
    source.chars().map(|ch| ch.to_string()).collect()
}

fn tokenize_source(source: &str, mode: MlTokenMode) -> Result<Vec<String>, String> {
    if source.is_empty() {
        return Err("tokenize text cannot be empty".into());
    }
    let tokens = match mode {
        MlTokenMode::Authored => authored_tokens(source)?,
        MlTokenMode::Word => word_tokens(source),
        MlTokenMode::Character => character_tokens(source),
    };
    if tokens.is_empty() {
        return Err("tokenize text contains no visible tokens".into());
    }
    if tokens.len() > MAX_TOKENS {
        return Err(format!(
            "tokenize produced {} tokens; ML5 stories support at most {MAX_TOKENS}",
            tokens.len()
        ));
    }
    Ok(tokens)
}

fn parse_token_mode(args: &Args, index: usize) -> Result<MlTokenMode, Error> {
    if args.len() <= index {
        return Ok(MlTokenMode::Word);
    }
    let name = args.ident(index)?;
    match name.as_str() {
        "authored" => Ok(MlTokenMode::Authored),
        "word" => Ok(MlTokenMode::Word),
        "character" | "char" => Ok(MlTokenMode::Character),
        _ => Err(Error::new(
            format!("unknown tokenizer `{name}` (try: authored, word, character)"),
            args.span_of(index),
        )),
    }
}

fn parse_position_mode(args: &Args, index: usize) -> Result<MlPositionMode, Error> {
    if args.len() <= index {
        return Ok(MlPositionMode::Sinusoidal);
    }
    let name = args.ident(index)?;
    match name.as_str() {
        "none" => Ok(MlPositionMode::None),
        "sinusoidal" | "sine" => Ok(MlPositionMode::Sinusoidal),
        _ => Err(Error::new(
            format!("unknown positional encoding `{name}` (try: sinusoidal, none)"),
            args.span_of(index),
        )),
    }
}

fn c_tokenize(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(5)?;
    let id = args.ident(0)?;
    if scene.ml_tokens.contains_key(&id) {
        return Err(Error::new(
            format!("token sequence `{id}` already exists"),
            args.span_of(0),
        ));
    }
    let center = args.pair(1)?;
    let source = args.text(2)?;
    let mode = parse_token_mode(args, 3)?;
    let tokens =
        tokenize_source(&source, mode).map_err(|message| Error::new(message, args.span_of(2)))?;
    let width = args.opt_num(4)?.unwrap_or(760.0);
    if !width.is_finite() || width < 360.0 {
        return Err(Error::new(
            "tokenize width must be a finite number of at least 360",
            args.span_of(4),
        ));
    }

    add_text(
        scene,
        &id,
        "labels",
        format!("{id}.heading"),
        format!("TOKENIZE · {} · {} TOKENS", mode.name(), tokens.len()),
        Vec2::new(center.x, center.y - 74.0),
        17.0,
        style::DIM,
    );
    add_text(
        scene,
        &id,
        "source",
        format!("{id}.source"),
        source.replace('|', "│"),
        Vec2::new(center.x, center.y - 42.0),
        16.0,
        style::FG,
    );

    let slot = width / tokens.len() as f32;
    let card_w = (slot * 0.82).clamp(30.0, 150.0);
    let start_x = center.x - width * 0.5 + slot * 0.5;
    for (index, token) in tokens.iter().enumerate() {
        let x = start_x + index as f32 * slot;
        let mut card = Entity::new(
            format!("{id}.token{index}.box"),
            Shape::Rect { w: card_w, h: 38.0 },
            Vec2::new(x, center.y + 10.0),
            mix(style::PANEL, style::CYAN, 0.12),
        );
        card.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 1.4,
            outline_color: Some(style::DIM),
        };
        tag(&mut card, &id, "tokens");
        card.tags.push(format!("{id}.token{index}"));
        scene.add(card);
        let mut label = Entity::new(
            format!("{id}.token{index}.text"),
            Shape::Text {
                content: display_token(token),
                size: (card_w * 0.14).clamp(10.0, 16.0),
            },
            Vec2::new(x, center.y + 10.0),
            style::FG,
        );
        label.font = FontKind::MonoBold;
        label.z = 5;
        tag(&mut label, &id, "tokens");
        label.tags.push(format!("{id}.token{index}"));
        scene.add(label);
        add_text(
            scene,
            &id,
            "indices",
            format!("{id}.token{index}.index"),
            index.to_string(),
            Vec2::new(x, center.y + 42.0),
            12.0,
            style::DIM,
        );
    }

    scene.ml_tokens.insert(
        id,
        MlTokenData {
            tokens,
            mode,
            source,
        },
    );
    Ok(())
}

fn parse_explicit_matrix(source: &str) -> Result<Vec<Vec<f32>>, String> {
    let mut rows: Vec<Vec<f32>> = Vec::new();
    for (row_index, row) in source.split(';').enumerate() {
        if row.trim().is_empty() {
            return Err(format!("embedding row {} is empty", row_index + 1));
        }
        let mut values = Vec::new();
        for word in row
            .split(|ch: char| ch == ',' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
        {
            let value = word
                .parse::<f32>()
                .map_err(|_| format!("embedding value `{word}` is not a finite number"))?;
            if !value.is_finite() {
                return Err(format!("embedding value `{word}` is not finite"));
            }
            values.push(value);
        }
        if values.is_empty() {
            return Err(format!("embedding row {} has no values", row_index + 1));
        }
        if let Some(first) = rows.first() {
            if values.len() != first.len() {
                return Err(format!(
                    "all embedding rows must contain {} values; row {} contains {}",
                    first.len(),
                    row_index + 1,
                    values.len()
                ));
            }
        }
        rows.push(values);
    }
    if rows.is_empty() {
        return Err("embedding values contain no rows".into());
    }
    Ok(rows)
}

fn lcg(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 40) as u32) as f32 / (1u32 << 24) as f32
}

fn token_hash(token: &str) -> u64 {
    token
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}

fn seeded_embeddings(tokens: &[String], dimension: usize, seed: u64) -> Vec<Vec<f32>> {
    let limit = (3.0 / dimension as f32).sqrt();
    tokens
        .iter()
        .map(|token| {
            let mut state = (seed ^ token_hash(token)).max(1);
            (0..dimension)
                .map(|_| (lcg(&mut state) * 2.0 - 1.0) * limit)
                .collect()
        })
        .collect()
}

fn parse_seeded_spec(source: &str, tokens: &[String]) -> Result<(Vec<Vec<f32>>, u64), String> {
    let parts: Vec<_> = source.split_whitespace().collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Err("seeded embedding syntax is `\"seeded DIM [SEED]\"`".into());
    }
    let dimension = parts[1]
        .parse::<usize>()
        .map_err(|_| "seeded embedding dimension must be a positive integer".to_string())?;
    if !(1..=MAX_DIMENSION).contains(&dimension) {
        return Err(format!(
            "seeded embedding dimension must be between 1 and {MAX_DIMENSION}"
        ));
    }
    let seed = if parts.len() == 3 {
        parts[2]
            .parse::<u64>()
            .map_err(|_| "seeded embedding seed must be a non-negative integer".to_string())?
    } else {
        17
    };
    Ok((seeded_embeddings(tokens, dimension, seed), seed))
}

fn sinusoidal_positions(rows: usize, dimension: usize) -> Vec<Vec<f32>> {
    (0..rows)
        .map(|position| {
            (0..dimension)
                .map(|index| {
                    let pair = 2 * (index / 2);
                    let angle = position as f32 / 10_000_f32.powf(pair as f32 / dimension as f32);
                    if index % 2 == 0 {
                        angle.sin()
                    } else {
                        angle.cos()
                    }
                })
                .collect()
        })
        .collect()
}

fn add_vector_cell(
    scene: &mut Scene,
    root: &str,
    role: &str,
    row: usize,
    col: usize,
    value: f32,
    pos: Vec2,
    cell_w: f32,
    cell_h: f32,
    show_value: bool,
) {
    let accent = if value >= 0.0 {
        match role {
            "positions" => style::GOLD,
            "combined" => style::LIME,
            _ => style::CYAN,
        }
    } else {
        style::MAGENTA
    };
    let strength = (value.abs() / 2.0).clamp(0.08, 0.78);
    let mut cell = Entity::new(
        format!("{root}.{role}.r{row}c{col}.cell"),
        Shape::Rect {
            w: cell_w * 0.88,
            h: cell_h * 0.72,
        },
        pos,
        mix(style::PANEL, accent, strength),
    );
    cell.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 1.0,
        outline_color: Some(mix(style::DIM, accent, strength)),
    };
    cell.z = 3;
    tag(&mut cell, root, role);
    cell.tags.push(format!("{root}.row{row}"));
    cell.tags.push(format!("{root}.dim{col}"));
    scene.add(cell);

    if show_value {
        let mut text = Entity::new(
            format!("{root}.{role}.r{row}c{col}.value"),
            Shape::Text {
                content: format!("{value:.2}"),
                size: (cell_w * 0.27).clamp(8.0, 12.0),
            },
            pos,
            style::FG,
        );
        text.font = FontKind::MonoBold;
        text.z = 4;
        tag(&mut text, root, role);
        text.tags.push(format!("{root}.values"));
        text.tags.push(format!("{root}.row{row}"));
        text.tags.push(format!("{root}.dim{col}"));
        scene.add(text);
    }
}

fn c_embedding(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(7)?;
    let id = args.ident(0)?;
    if scene.ml_embeddings.contains_key(&id) {
        return Err(Error::new(
            format!("embedding figure `{id}` already exists"),
            args.span_of(0),
        ));
    }
    let token_sequence = args.ident(1)?;
    let token_data =
        scene
            .ml_tokens
            .get(&token_sequence)
            .cloned()
            .ok_or_else(|| {
                Error::new(
            format!("`{token_sequence}` is not a token sequence; create it with `tokenize` first"),
            args.span_of(1),
        )
            })?;
    let center = args.pair(2)?;
    let vector_spec = args.text(3)?;
    let (token_vectors, source) = if vector_spec
        .split_whitespace()
        .next()
        .is_some_and(|part| part.eq_ignore_ascii_case("seeded"))
    {
        let (values, seed) = parse_seeded_spec(&vector_spec, &token_data.tokens)
            .map_err(|message| Error::new(message, args.span_of(3)))?;
        (values, MlEmbeddingSource::Seeded { seed })
    } else {
        let values = parse_explicit_matrix(&vector_spec)
            .map_err(|message| Error::new(message, args.span_of(3)))?;
        (values, MlEmbeddingSource::Explicit)
    };
    if token_vectors.len() != token_data.tokens.len() {
        return Err(Error::new(
            format!(
                "embedding has {} token(s) but {} vector row(s)",
                token_data.tokens.len(),
                token_vectors.len()
            ),
            args.span_of(3),
        ));
    }
    let dimension = token_vectors[0].len();
    if !(1..=MAX_DIMENSION).contains(&dimension) {
        return Err(Error::new(
            format!("embedding dimension must be between 1 and {MAX_DIMENSION}"),
            args.span_of(3),
        ));
    }
    let position = parse_position_mode(args, 4)?;
    let width = args.opt_num(5)?.unwrap_or(1040.0);
    let height = args.opt_num(6)?.unwrap_or(460.0);
    if !width.is_finite() || width < 660.0 {
        return Err(Error::new(
            "embedding width must be a finite number of at least 660",
            args.span_of(5),
        ));
    }
    if !height.is_finite() || height < 280.0 {
        return Err(Error::new(
            "embedding height must be a finite number of at least 280",
            args.span_of(6),
        ));
    }

    let position_vectors = match position {
        MlPositionMode::None => vec![vec![0.0; dimension]; token_data.tokens.len()],
        MlPositionMode::Sinusoidal => sinusoidal_positions(token_data.tokens.len(), dimension),
    };
    let combined_vectors: Vec<Vec<f32>> = token_vectors
        .iter()
        .zip(&position_vectors)
        .map(|(token, positional)| {
            token
                .iter()
                .zip(positional)
                .map(|(left, right)| left + right)
                .collect()
        })
        .collect();

    let source_label = match source {
        MlEmbeddingSource::Explicit => "EXPLICIT".to_string(),
        MlEmbeddingSource::Seeded { seed } => format!("SEEDED EDUCATIONAL · seed {seed}"),
    };
    add_text(
        scene,
        &id,
        "labels",
        format!("{id}.heading"),
        format!(
            "EMBEDDING · d={dimension} · {source_label} · {}",
            position.name()
        ),
        Vec2::new(center.x, center.y - height * 0.48),
        17.0,
        style::DIM,
    );

    let token_x = center.x - width * 0.45;
    let embedding_x = center.x - width * 0.22;
    let plus_x = center.x - width * 0.07;
    let position_x = center.x + width * 0.08;
    let equals_x = center.x + width * 0.23;
    let combined_x = center.x + width * 0.39;
    let count = token_data.tokens.len();
    let row_h = height * 0.68 / count as f32;
    let y0 = center.y - row_h * (count.saturating_sub(1) as f32) * 0.5;
    let matrix_w = width * 0.20;
    let cell_w = (matrix_w / dimension as f32).clamp(16.0, 40.0);
    let cell_h = row_h.clamp(24.0, 48.0);
    let token_w = (width * 0.13).clamp(86.0, 150.0);

    for (text, x, role) in [
        ("TOKEN", token_x, "tokens"),
        ("TOKEN VECTOR", embedding_x, "vectors"),
        ("POSITION", position_x, "positions"),
        ("MODEL INPUT", combined_x, "combined"),
    ] {
        add_text(
            scene,
            &id,
            role,
            format!("{id}.{role}.header"),
            text.into(),
            Vec2::new(x, center.y - height * 0.39),
            14.0,
            style::DIM,
        );
    }

    for row in 0..count {
        let y = y0 + row as f32 * row_h;
        let mut token_box = Entity::new(
            format!("{id}.token{row}.box"),
            Shape::Rect {
                w: token_w,
                h: cell_h * 0.72,
            },
            Vec2::new(token_x, y),
            mix(style::PANEL, style::CYAN, 0.12),
        );
        token_box.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 1.2,
            outline_color: Some(style::DIM),
        };
        tag(&mut token_box, &id, "tokens");
        token_box.tags.push(format!("{id}.token{row}"));
        token_box.tags.push(format!("{id}.row{row}"));
        scene.add(token_box);
        let mut token_label = Entity::new(
            format!("{id}.token{row}.text"),
            Shape::Text {
                content: format!("{row}  {}", display_token(&token_data.tokens[row])),
                size: (token_w * 0.105).clamp(10.0, 15.0),
            },
            Vec2::new(token_x, y),
            style::FG,
        );
        token_label.font = FontKind::MonoBold;
        token_label.z = 5;
        tag(&mut token_label, &id, "tokens");
        token_label.tags.push(format!("{id}.token{row}"));
        token_label.tags.push(format!("{id}.row{row}"));
        scene.add(token_label);

        add_text(
            scene,
            &id,
            "operators",
            format!("{id}.plus{row}"),
            "+".into(),
            Vec2::new(plus_x, y),
            18.0,
            style::DIM,
        );
        add_text(
            scene,
            &id,
            "operators",
            format!("{id}.equals{row}"),
            "=".into(),
            Vec2::new(equals_x, y),
            18.0,
            style::DIM,
        );

        for col in 0..dimension {
            let dx = (col as f32 - (dimension.saturating_sub(1) as f32) * 0.5) * cell_w;
            add_vector_cell(
                scene,
                &id,
                "vectors",
                row,
                col,
                token_vectors[row][col],
                Vec2::new(embedding_x + dx, y),
                cell_w,
                cell_h,
                true,
            );
            add_vector_cell(
                scene,
                &id,
                "positions",
                row,
                col,
                position_vectors[row][col],
                Vec2::new(position_x + dx, y),
                cell_w,
                cell_h,
                true,
            );
            add_vector_cell(
                scene,
                &id,
                "combined",
                row,
                col,
                combined_vectors[row][col],
                Vec2::new(combined_x + dx, y),
                cell_w,
                cell_h,
                true,
            );
        }
    }

    add_text(
        scene,
        &id,
        "labels",
        format!("{id}.status"),
        format!(
            "token vector + {} = model input",
            if position == MlPositionMode::Sinusoidal {
                "position"
            } else {
                "zero position"
            }
        ),
        Vec2::new(center.x, center.y + height * 0.47),
        16.0,
        style::DIM,
    );

    scene.ml_embeddings.insert(
        id,
        MlEmbeddingData {
            token_sequence,
            tokens: token_data.tokens,
            token_vectors,
            position_vectors,
            combined_vectors,
            dimension,
            source,
            position,
        },
    );
    Ok(())
}

pub fn register(registry: &mut Registry) {
    registry.ctor("tokenize", c_tokenize);
    registry.ctor("embedding", c_embedding);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn movie(src: &str) -> crate::movie::Movie {
        crate::parse(src).unwrap_or_else(|error| panic!("parse failed: {error:?}"))
    }

    fn parse_error(src: &str) -> crate::lang::diag::Error {
        match crate::parse(src) {
            Ok(_) => panic!("expected parse to fail"),
            Err(error) => error,
        }
    }

    #[test]
    fn authored_word_and_character_tokenization_are_deterministic() {
        assert_eq!(
            tokenize_source("Art | ificial | intelligence", MlTokenMode::Authored).unwrap(),
            vec!["Art", "ificial", "intelligence"]
        );
        assert_eq!(
            tokenize_source("Café learns, quickly!", MlTokenMode::Word).unwrap(),
            vec!["Café", "learns", ",", "quickly", "!"]
        );
        assert_eq!(
            tokenize_source("A B", MlTokenMode::Character).unwrap(),
            vec!["A", " ", "B"]
        );
    }

    #[test]
    fn sinusoidal_encoding_matches_the_standard_fixture() {
        let values = sinusoidal_positions(2, 4);
        assert_eq!(values[0], vec![0.0, 1.0, 0.0, 1.0]);
        assert!((values[1][0] - 1.0_f32.sin()).abs() < 1e-6);
        assert!((values[1][1] - 1.0_f32.cos()).abs() < 1e-6);
        assert!((values[1][2] - 0.01_f32.sin()).abs() < 1e-6);
        assert!((values[1][3] - 0.01_f32.cos()).abs() < 1e-6);
    }

    #[test]
    fn seeded_embeddings_repeat_and_explicit_vectors_add_exactly() {
        let tokens = vec!["same".into(), "other".into(), "same".into()];
        assert_eq!(
            seeded_embeddings(&tokens, 4, 23),
            seeded_embeddings(&tokens, 4, 23)
        );
        assert_ne!(
            seeded_embeddings(&tokens, 4, 23),
            seeded_embeddings(&tokens, 4, 24)
        );
        let values = seeded_embeddings(&tokens, 4, 23);
        assert_eq!(
            values[0], values[2],
            "the same token must reuse its embedding"
        );

        let movie = movie(
            "tokenize(words,(400,120),\"one | two\",authored); embedding(context,words,(500,350),\"1 2; 3 4\",sinusoidal,760,320);",
        );
        let data = movie.scene.ml_embeddings.get("context").unwrap();
        assert_eq!(data.dimension, 2);
        assert_eq!(data.combined_vectors[0], vec![1.0, 3.0]);
        assert!((data.combined_vectors[1][0] - (3.0 + 1.0_f32.sin())).abs() < 1e-6);
        assert!((data.combined_vectors[1][1] - (4.0 + 1.0_f32.cos())).abs() < 1e-6);
    }

    #[test]
    fn ml5_figures_expose_stable_story_tags() {
        let movie = movie(
            "tokenize(words,(400,120),\"one two\",word); embedding(context,words,(500,350),\"seeded 4 9\",sinusoidal,760,320);",
        );
        for tag in [
            "words.tokens",
            "words.indices",
            "context.tokens",
            "context.vectors",
            "context.positions",
            "context.combined",
            "context.row0",
            "context.dim0",
        ] {
            assert!(
                movie
                    .scene
                    .entities
                    .iter()
                    .any(|entity| entity.tags.iter().any(|candidate| candidate == tag)),
                "missing tag {tag}"
            );
        }
        let data = movie.scene.ml_embeddings.get("context").unwrap();
        assert_eq!(data.source, MlEmbeddingSource::Seeded { seed: 9 });
        for id in [
            "context.vectors.r0c0.value",
            "context.positions.r0c0.value",
            "context.combined.r0c0.value",
        ] {
            assert!(movie.scene.get(id).is_some(), "missing visible value {id}");
        }
    }

    #[test]
    fn ml5_errors_name_the_actual_contract() {
        let error = parse_error("tokenize(t,(0,0),\"one||two\",authored);");
        assert!(error.msg.contains("token 2 is empty"));

        let error = parse_error("tokenize(t,(0,0),\"one two\",bpe);");
        assert!(error.msg.contains("authored, word, character"));

        let error = parse_error(
            "tokenize(words,(0,0),\"one | two\",authored); embedding(context,words,(0,0),\"1 2\");",
        );
        assert!(error.msg.contains("2 token(s) but 1 vector row(s)"));

        let error = parse_error(
            "tokenize(words,(0,0),\"one | two\",authored); embedding(context,words,(0,0),\"1 2; 3\");",
        );
        assert!(error.msg.contains("row 2 contains 1"));

        let error = parse_error(
            "tokenize(words,(0,0),\"one | two\",authored); embedding(context,words,(0,0),\"seeded 4\",learned);",
        );
        assert!(error.msg.contains("sinusoidal, none"));
    }

    #[test]
    fn ml5_story_text_remains_visible_after_group_pulses() {
        let movie = movie(include_str!(
            "../../examples/manic-ml-token-embedding.manic"
        ));
        let (base, timeline) = movie.finalize();
        let frame = timeline.apply(&base, 10.2);
        for id in [
            "headline",
            "context.heading",
            "context.token0.text",
            "context.vectors.r0c0.value",
            "context.positions.r0c0.cell",
            "context.combined.r0c0.cell",
        ] {
            let entity = frame
                .get(id)
                .unwrap_or_else(|| panic!("missing entity {id}"));
            assert!(entity.opacity > 0.9, "{id} opacity is {}", entity.opacity);
            assert!(entity.scale > 0.9, "{id} scale is {}", entity.scale);
        }
    }
}
