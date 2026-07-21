//! ML3 tensor operators: truthful numeric grids with one shared scan animation.
//!
//! The renderer receives ordinary rectangles and text. Numeric tensors,
//! convolution/pooling results, and scan steps are computed while lowering so
//! direct seeking remains deterministic and no frame performs ML arithmetic.

use macroquad::prelude::{Color, Vec2};

use super::ml::Activation;
use crate::easing::Easing;
use crate::lang::diag::Error;
use crate::lang::lower::{resolve_color, resolve_easing, Args, Registry};
use crate::primitives::{Entity, FontKind, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TextEvent, TrackSpec, Value};

const MAX_TENSOR_AXIS: usize = 16;
const MAX_TENSOR_VALUES: usize = 2_048;

#[derive(Debug, Clone)]
pub struct MlTensorData {
    pub channels: usize,
    pub rows: usize,
    pub cols: usize,
    /// Channel → row → column.
    pub values: Vec<Vec<Vec<f32>>>,
    pub center: Vec2,
    pub cell: f32,
}

#[derive(Debug, Clone)]
pub struct MlKernelData {
    pub tensor: MlTensorData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolKind {
    Max,
    Average,
}

impl PoolKind {
    fn parse(word: &str) -> Option<Self> {
        match word.to_ascii_lowercase().as_str() {
            "max" | "maximum" => Some(Self::Max),
            "avg" | "average" | "mean" => Some(Self::Average),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Max => "MAX",
            Self::Average => "AVERAGE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MlScanStep {
    pub source_pos: Vec2,
    pub output_pos: Vec2,
    pub pick_pos: Option<Vec2>,
    pub output_channel: usize,
    pub output_row: usize,
    pub output_col: usize,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct MlScanData {
    pub source: String,
    pub output: String,
    pub operator: String,
    pub steps: Vec<MlScanStep>,
    pub source_frame: String,
    pub operator_frame: Option<String>,
    pub output_frame: String,
    pub pick: String,
    pub status: String,
}

fn split_numbers(row: &str) -> impl Iterator<Item = &str> {
    row.split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter(|part| !part.is_empty())
}

fn parse_grid(src: &str, noun: &str) -> Result<MlTensorData, String> {
    let raw_channels = src.split('|').collect::<Vec<_>>();
    if raw_channels.iter().any(|channel| channel.trim().is_empty()) {
        return Err(format!("{noun} contains an empty channel"));
    }
    let mut channels = Vec::with_capacity(raw_channels.len());
    let mut expected = None;
    for (channel_index, channel) in raw_channels.iter().enumerate() {
        let raw_rows = channel
            .split(';')
            .filter(|row| !row.trim().is_empty())
            .collect::<Vec<_>>();
        if raw_rows.is_empty() {
            return Err(format!("{noun} channel {} has no rows", channel_index + 1));
        }
        let mut rows: Vec<Vec<f32>> = Vec::with_capacity(raw_rows.len());
        for (row_index, row) in raw_rows.iter().enumerate() {
            let mut values = Vec::new();
            for word in split_numbers(row) {
                let value = word
                    .parse::<f32>()
                    .map_err(|_| format!("{noun} value `{word}` is not a finite number"))?;
                if !value.is_finite() {
                    return Err(format!("{noun} value `{word}` is not finite"));
                }
                values.push(value);
            }
            if values.is_empty() {
                return Err(format!(
                    "{noun} channel {} row {} has no values",
                    channel_index + 1,
                    row_index + 1
                ));
            }
            if let Some((wanted_rows, wanted_cols)) = expected {
                if values.len() != wanted_cols {
                    return Err(format!(
                        "all {noun} rows/channels must have shape {wanted_rows}×{wanted_cols}; channel {} row {} has {} columns",
                        channel_index + 1,
                        row_index + 1,
                        values.len()
                    ));
                }
            } else if row_index > 0 && values.len() != rows[0].len() {
                return Err(format!(
                    "{noun} rows must have equal length; row 1 has {} columns but row {} has {}",
                    rows[0].len(),
                    row_index + 1,
                    values.len()
                ));
            }
            rows.push(values);
        }
        if let Some((wanted_rows, wanted_cols)) = expected {
            if rows.len() != wanted_rows {
                return Err(format!(
                    "all {noun} channels must have shape {wanted_rows}×{wanted_cols}; channel {} has {} rows",
                    channel_index + 1,
                    rows.len()
                ));
            }
        } else {
            expected = Some((rows.len(), rows[0].len()));
        }
        channels.push(rows);
    }
    let (rows, cols) = expected.expect("a non-empty first channel was validated");
    validate_tensor_shape(channels.len(), rows, cols, noun)?;
    Ok(MlTensorData {
        channels: channels.len(),
        rows,
        cols,
        values: channels,
        center: Vec2::ZERO,
        cell: 0.0,
    })
}

fn validate_tensor_shape(
    channels: usize,
    rows: usize,
    cols: usize,
    noun: &str,
) -> Result<(), String> {
    if channels > MAX_TENSOR_AXIS || rows > MAX_TENSOR_AXIS || cols > MAX_TENSOR_AXIS {
        return Err(format!(
            "{noun} axes may contain at most {MAX_TENSOR_AXIS} cells (got {channels}×{rows}×{cols})"
        ));
    }
    let total = channels
        .checked_mul(rows)
        .and_then(|value| value.checked_mul(cols))
        .ok_or_else(|| format!("{noun} dimensions are too large"))?;
    if total > MAX_TENSOR_VALUES {
        return Err(format!(
            "{noun} contains {total} values; ML3 supports at most {MAX_TENSOR_VALUES}"
        ));
    }
    Ok(())
}

fn positive_integer(
    args: &Args,
    index: usize,
    fallback: usize,
    noun: &str,
) -> Result<usize, Error> {
    let raw = args.opt_num(index)?.unwrap_or(fallback as f32);
    if !raw.is_finite() || raw < 1.0 || raw.fract().abs() > 1e-6 {
        return Err(Error::new(
            format!("{noun} must be a positive integer"),
            args.span_of(index),
        ));
    }
    Ok(raw as usize)
}

fn nonnegative_integer(
    args: &Args,
    index: usize,
    fallback: usize,
    noun: &str,
) -> Result<usize, Error> {
    let raw = args.opt_num(index)?.unwrap_or(fallback as f32);
    if !raw.is_finite() || raw < 0.0 || raw.fract().abs() > 1e-6 {
        return Err(Error::new(
            format!("{noun} must be a non-negative integer"),
            args.span_of(index),
        ));
    }
    Ok(raw as usize)
}

fn cell_size(args: &Args, index: usize, fallback: f32) -> Result<f32, Error> {
    let cell = args.opt_num(index)?.unwrap_or(fallback);
    if !cell.is_finite() || !(18.0..=120.0).contains(&cell) {
        return Err(Error::new(
            "tensor cell size must be between 18 and 120",
            args.span_of(index),
        ));
    }
    Ok(cell)
}

fn fmt(value: f32) -> String {
    if value.abs() >= 100.0 || (value != 0.0 && value.abs() < 0.01) {
        format!("{value:.1e}")
    } else if (value - value.round()).abs() < 1e-5 {
        format!("{value:.0}")
    } else {
        format!("{value:.2}")
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

fn channel_offset(data: &MlTensorData, channel: f32) -> Vec2 {
    let depth = (channel - (data.channels.saturating_sub(1) as f32 * 0.5)) * data.cell * 0.34;
    Vec2::new(depth, -depth)
}

fn grid_pos(data: &MlTensorData, channel: f32, row: f32, col: f32) -> Vec2 {
    data.center
        + channel_offset(data, channel)
        + Vec2::new(
            (col - (data.cols.saturating_sub(1) as f32 * 0.5)) * data.cell,
            (row - (data.rows.saturating_sub(1) as f32 * 0.5)) * data.cell,
        )
}

fn cell_id(id: &str, channel: usize, row: usize, col: usize) -> String {
    format!("{id}.c{channel}.r{row}c{col}")
}

fn tag(entity: &mut Entity, id: &str, group: &str) {
    entity.tags.push(id.to_string());
    entity.tags.push(format!("{id}.{group}"));
}

fn add_grid(scene: &mut Scene, id: &str, data: &MlTensorData, role: &str, base: Color) {
    let max = data
        .values
        .iter()
        .flatten()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0f32, f32::max)
        .max(1e-6);
    for channel in 0..data.channels {
        for row in 0..data.rows {
            for col in 0..data.cols {
                let value = data.values[channel][row][col];
                let cid = cell_id(id, channel, row, col);
                let accent = if value < 0.0 { style::MAGENTA } else { base };
                let mut cell = Entity::new(
                    cid.clone(),
                    Shape::Rect {
                        w: data.cell * 0.90,
                        h: data.cell * 0.90,
                    },
                    grid_pos(data, channel as f32, row as f32, col as f32),
                    mix(style::PANEL, accent, 0.12 + 0.64 * value.abs() / max),
                );
                cell.stroke = StrokeStyle {
                    fill: true,
                    outline: true,
                    width: 1.4,
                    outline_color: Some(mix(style::DIM, accent, 0.55)),
                };
                cell.z = 2 + channel as i32 * 2;
                tag(&mut cell, id, "cells");
                cell.tags.push(format!("{id}.channel{channel}"));
                cell.tags.push(format!("{id}.row{row}"));
                cell.tags.push(format!("{id}.col{col}"));
                scene.add(cell);

                let mut label = Entity::new(
                    format!("{cid}.value"),
                    Shape::Text {
                        content: fmt(value),
                        size: (data.cell * 0.34).clamp(11.0, 20.0),
                    },
                    grid_pos(data, channel as f32, row as f32, col as f32),
                    style::FG,
                );
                label.font = FontKind::MonoBold;
                label.z = 3 + channel as i32 * 2;
                tag(&mut label, id, "values");
                label.tags.push(format!("{id}.channel{channel}"));
                scene.add(label);
            }
        }
    }

    let spread = data.channels.saturating_sub(1) as f32 * data.cell * 0.34;
    let mut label = Entity::new(
        format!("{id}.label"),
        Shape::Text {
            content: format!("{role} · {}×{}×{}", data.channels, data.rows, data.cols),
            size: (data.cell * 0.38).clamp(14.0, 20.0),
        },
        Vec2::new(
            data.center.x,
            data.center.y - data.rows as f32 * data.cell * 0.5 - spread - 28.0,
        ),
        style::DIM,
    );
    label.font = FontKind::MonoBold;
    label.z = 20;
    tag(&mut label, id, "labels");
    scene.add(label);
}

fn ensure_new(scene: &Scene, id: &str, args: &Args) -> Result<(), Error> {
    if scene.ml_tensors.contains_key(id)
        || scene.ml_kernels.contains_key(id)
        || scene.ml_scans.contains_key(id)
    {
        return Err(Error::new(
            format!("ML tensor/kernel `{id}` already exists"),
            args.span_of(0),
        ));
    }
    Ok(())
}

/// `tensor(id, (cx,cy), "rows | channels", [cell], [color])`
fn c_tensor(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(5)?;
    let id = args.ident(0)?;
    ensure_new(scene, &id, args)?;
    let mut data = parse_grid(&args.text(2)?, "tensor")
        .map_err(|message| Error::new(message, args.span_of(2)))?;
    data.center = args.pair(1)?;
    data.cell = cell_size(args, 3, 48.0)?;
    let color = if args.len() > 4 {
        resolve_color(&args.ident(4)?, args.span_of(4))?
    } else {
        style::CYAN
    };
    add_grid(scene, &id, &data, "TENSOR", color);
    scene.ml_tensors.insert(id, data);
    Ok(())
}

/// `kernel(id, (cx,cy), "rows | input channels", [cell], [color])`
fn c_kernel(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(5)?;
    let id = args.ident(0)?;
    ensure_new(scene, &id, args)?;
    let mut tensor = parse_grid(&args.text(2)?, "kernel")
        .map_err(|message| Error::new(message, args.span_of(2)))?;
    tensor.center = args.pair(1)?;
    tensor.cell = cell_size(args, 3, 48.0)?;
    let color = if args.len() > 4 {
        resolve_color(&args.ident(4)?, args.span_of(4))?
    } else {
        style::MAGENTA
    };
    add_grid(scene, &id, &tensor, "KERNEL", color);
    scene.ml_kernels.insert(id, MlKernelData { tensor });
    Ok(())
}

fn output_shape(
    input: usize,
    window: usize,
    stride: usize,
    padding: usize,
    noun: &str,
) -> Result<usize, String> {
    let padded = input
        .checked_add(padding.saturating_mul(2))
        .ok_or_else(|| format!("{noun} padding is too large"))?;
    if window > padded {
        return Err(format!(
            "{noun} window/kernel {window} exceeds padded input size {padded}"
        ));
    }
    Ok((padded - window) / stride + 1)
}

fn add_scan_entities(
    scene: &mut Scene,
    id: &str,
    source: &MlTensorData,
    output: &MlTensorData,
    window_rows: usize,
    window_cols: usize,
    operator: Option<&MlTensorData>,
    operator_label: &str,
    with_pick: bool,
    all_source_channels: bool,
) -> (String, Option<String>, String, String, String) {
    let source_frame = format!("{id}.scan.source");
    let output_frame = format!("{id}.scan.output");
    let pick = format!("{id}.scan.pick");
    let status = format!("{id}.scan.status");
    let source_spread = if all_source_channels {
        source.channels.saturating_sub(1) as f32 * source.cell * 0.34
    } else {
        0.0
    };
    let mut source_box = Entity::new(
        source_frame.clone(),
        Shape::Rect {
            w: window_cols as f32 * source.cell + source_spread,
            h: window_rows as f32 * source.cell + source_spread,
        },
        source.center,
        style::CYAN,
    );
    source_box.stroke.fill = false;
    source_box.stroke.outline = true;
    source_box.stroke.width = 4.0;
    source_box.glow = 0.55;
    source_box.opacity = 0.0;
    source_box.z = 30;
    tag(&mut source_box, id, "scan");
    scene.add(source_box);

    let mut output_box = Entity::new(
        output_frame.clone(),
        Shape::Rect {
            w: output.cell,
            h: output.cell,
        },
        output.center,
        style::GOLD,
    );
    output_box.stroke.fill = false;
    output_box.stroke.outline = true;
    output_box.stroke.width = 4.0;
    output_box.glow = 0.55;
    output_box.opacity = 0.0;
    output_box.z = 30;
    tag(&mut output_box, id, "scan");
    scene.add(output_box);

    let operator_frame = operator.map(|operator| {
        let operator_frame = format!("{id}.scan.operator");
        let operator_spread = operator.channels.saturating_sub(1) as f32 * operator.cell * 0.34;
        let mut frame = Entity::new(
            operator_frame.clone(),
            Shape::Rect {
                w: operator.cols as f32 * operator.cell + operator_spread,
                h: operator.rows as f32 * operator.cell + operator_spread,
            },
            operator.center,
            style::MAGENTA,
        );
        frame.stroke.fill = false;
        frame.stroke.outline = true;
        frame.stroke.width = 4.0;
        frame.glow = 0.55;
        frame.opacity = 0.0;
        frame.z = 30;
        tag(&mut frame, id, "scan");
        scene.add(frame);
        operator_frame
    });

    let mut pick_entity = Entity::new(
        pick.clone(),
        Shape::Circle {
            r: (source.cell * 0.16).clamp(5.0, 11.0),
        },
        source.center,
        style::GOLD,
    );
    pick_entity.glow = 0.7;
    pick_entity.opacity = 0.0;
    pick_entity.z = 34;
    tag(&mut pick_entity, id, "scan");
    if with_pick {
        pick_entity.tags.push(format!("{id}.selection"));
    }
    scene.add(pick_entity);

    let stack = output.channels.saturating_sub(1) as f32 * output.cell * 0.34;
    let mut status_entity = Entity::new(
        status.clone(),
        Shape::Text {
            content: format!("{operator_label} · ready to scan"),
            size: (output.cell * 0.38).clamp(14.0, 19.0),
        },
        Vec2::new(
            output.center.x,
            output.center.y + output.rows as f32 * output.cell * 0.5 + stack + 32.0,
        ),
        style::DIM,
    );
    status_entity.font = FontKind::MonoBold;
    status_entity.opacity = 0.0;
    status_entity.z = 35;
    tag(&mut status_entity, id, "scan");
    scene.add(status_entity);
    (source_frame, operator_frame, output_frame, pick, status)
}

/// `convolve(id, input, kernel, (cx,cy), [stride], [padding], [bias], [activation], [cell])`
fn c_convolve(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(9)?;
    let id = args.ident(0)?;
    ensure_new(scene, &id, args)?;
    let source_id = args.ident(1)?;
    let kernel_id = args.ident(2)?;
    let source = scene.ml_tensors.get(&source_id).cloned().ok_or_else(|| {
        Error::new(
            format!("`{source_id}` is not an ML tensor"),
            args.span_of(1),
        )
    })?;
    let kernel = scene.ml_kernels.get(&kernel_id).cloned().ok_or_else(|| {
        Error::new(
            format!("`{kernel_id}` is not an ML kernel"),
            args.span_of(2),
        )
    })?;
    if source.channels != kernel.tensor.channels {
        return Err(Error::new(
            format!(
                "convolution input has {} channel(s) but kernel has {}",
                source.channels, kernel.tensor.channels
            ),
            args.span_of(2),
        ));
    }
    let center = args.pair(3)?;
    let stride = positive_integer(args, 4, 1, "convolution stride")?;
    let padding = nonnegative_integer(args, 5, 0, "convolution padding")?;
    let bias = args.opt_num(6)?.unwrap_or(0.0);
    if !bias.is_finite() {
        return Err(Error::new(
            "convolution bias must be finite",
            args.span_of(6),
        ));
    }
    let activation = if args.len() > 7 {
        let word = args.ident(7)?;
        Activation::parse(&word).ok_or_else(|| {
            Error::new(
                format!("unknown convolution activation `{word}`"),
                args.span_of(7),
            )
        })?
    } else {
        Activation::Linear
    };
    if activation == Activation::Softmax {
        return Err(Error::new(
            "convolution activation is cellwise; softmax is vector-valued",
            args.span_of(7),
        ));
    }
    let cell = cell_size(args, 8, source.cell)?;
    let rows = output_shape(
        source.rows,
        kernel.tensor.rows,
        stride,
        padding,
        "convolution",
    )
    .map_err(|message| Error::new(message, args.span_of(2)))?;
    let cols = output_shape(
        source.cols,
        kernel.tensor.cols,
        stride,
        padding,
        "convolution",
    )
    .map_err(|message| Error::new(message, args.span_of(2)))?;
    validate_tensor_shape(1, rows, cols, "convolution output")
        .map_err(|message| Error::new(message, args.span_of(3)))?;
    let mut values = vec![vec![vec![0.0; cols]; rows]];
    let mut steps = Vec::with_capacity(rows * cols);
    for row in 0..rows {
        for col in 0..cols {
            let mut sum = bias;
            for channel in 0..source.channels {
                for kr in 0..kernel.tensor.rows {
                    for kc in 0..kernel.tensor.cols {
                        let sr = row as isize * stride as isize + kr as isize - padding as isize;
                        let sc = col as isize * stride as isize + kc as isize - padding as isize;
                        if sr >= 0
                            && sc >= 0
                            && (sr as usize) < source.rows
                            && (sc as usize) < source.cols
                        {
                            sum += source.values[channel][sr as usize][sc as usize]
                                * kernel.tensor.values[channel][kr][kc];
                        }
                    }
                }
            }
            let result = activation.scalar(sum);
            if !result.is_finite() {
                return Err(Error::new(
                    "convolution produced a non-finite output",
                    args.span_of(0),
                ));
            }
            values[0][row][col] = result;
        }
    }
    let output = MlTensorData {
        channels: 1,
        rows,
        cols,
        values,
        center,
        cell,
    };
    for row in 0..rows {
        for col in 0..cols {
            let source_row = row as f32 * stride as f32 - padding as f32
                + (kernel.tensor.rows.saturating_sub(1) as f32 * 0.5);
            let source_col = col as f32 * stride as f32 - padding as f32
                + (kernel.tensor.cols.saturating_sub(1) as f32 * 0.5);
            steps.push(MlScanStep {
                source_pos: grid_pos(
                    &source,
                    source.channels.saturating_sub(1) as f32 * 0.5,
                    source_row,
                    source_col,
                ),
                output_pos: grid_pos(&output, 0.0, row as f32, col as f32),
                pick_pos: None,
                output_channel: 0,
                output_row: row,
                output_col: col,
                summary: format!(
                    "Σ(input × kernel) + {} → {} = {}",
                    fmt(bias),
                    activation.name(),
                    fmt(output.values[0][row][col])
                ),
            });
        }
    }
    add_grid(scene, &id, &output, "FEATURE MAP", style::GOLD);
    let (source_frame, operator_frame, output_frame, pick, status) = add_scan_entities(
        scene,
        &id,
        &source,
        &output,
        kernel.tensor.rows,
        kernel.tensor.cols,
        Some(&kernel.tensor),
        "CONVOLUTION",
        false,
        true,
    );
    scene.ml_tensors.insert(id.clone(), output);
    scene.ml_scans.insert(
        id.clone(),
        MlScanData {
            source: source_id,
            output: id,
            operator: format!("CONV · {kernel_id}"),
            steps,
            source_frame,
            operator_frame,
            output_frame,
            pick,
            status,
        },
    );
    Ok(())
}

/// `pool(id, input, (cx,cy), max|average, [window], [stride], [padding], [cell])`
fn c_pool(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(8)?;
    let id = args.ident(0)?;
    ensure_new(scene, &id, args)?;
    let source_id = args.ident(1)?;
    let source = scene.ml_tensors.get(&source_id).cloned().ok_or_else(|| {
        Error::new(
            format!("`{source_id}` is not an ML tensor"),
            args.span_of(1),
        )
    })?;
    let center = args.pair(2)?;
    let kind_word = args.ident(3)?;
    let kind = PoolKind::parse(&kind_word).ok_or_else(|| {
        Error::new(
            format!("unknown pool kind `{kind_word}` (try: max or average)"),
            args.span_of(3),
        )
    })?;
    let window = positive_integer(args, 4, 2, "pool window")?;
    let stride = positive_integer(args, 5, window, "pool stride")?;
    let padding = nonnegative_integer(args, 6, 0, "pool padding")?;
    let cell = cell_size(args, 7, source.cell)?;
    let rows = output_shape(source.rows, window, stride, padding, "pool")
        .map_err(|message| Error::new(message, args.span_of(4)))?;
    let cols = output_shape(source.cols, window, stride, padding, "pool")
        .map_err(|message| Error::new(message, args.span_of(4)))?;
    validate_tensor_shape(source.channels, rows, cols, "pool output")
        .map_err(|message| Error::new(message, args.span_of(2)))?;
    let mut output = MlTensorData {
        channels: source.channels,
        rows,
        cols,
        values: vec![vec![vec![0.0; cols]; rows]; source.channels],
        center,
        cell,
    };
    let mut steps = Vec::with_capacity(source.channels * rows * cols);
    for channel in 0..source.channels {
        for row in 0..rows {
            for col in 0..cols {
                let mut best = f32::NEG_INFINITY;
                let mut best_pos = None;
                let mut sum = 0.0;
                let mut count = 0usize;
                for wr in 0..window {
                    for wc in 0..window {
                        let sr = row as isize * stride as isize + wr as isize - padding as isize;
                        let sc = col as isize * stride as isize + wc as isize - padding as isize;
                        if sr >= 0
                            && sc >= 0
                            && (sr as usize) < source.rows
                            && (sc as usize) < source.cols
                        {
                            let value = source.values[channel][sr as usize][sc as usize];
                            // Strict `>` makes max-pool ties select the first
                            // valid cell in channel/row/column order.
                            if value > best {
                                best = value;
                                best_pos =
                                    Some(grid_pos(&source, channel as f32, sr as f32, sc as f32));
                            }
                            sum += value;
                            count += 1;
                        }
                    }
                }
                if count == 0 {
                    return Err(Error::new(
                        "pool window contains only padding",
                        args.span_of(4),
                    ));
                }
                let result = match kind {
                    PoolKind::Max => best,
                    PoolKind::Average => sum / count as f32,
                };
                output.values[channel][row][col] = result;
                let source_row = row as f32 * stride as f32 - padding as f32
                    + (window.saturating_sub(1) as f32 * 0.5);
                let source_col = col as f32 * stride as f32 - padding as f32
                    + (window.saturating_sub(1) as f32 * 0.5);
                steps.push(MlScanStep {
                    source_pos: grid_pos(&source, channel as f32, source_row, source_col),
                    output_pos: grid_pos(&output, channel as f32, row as f32, col as f32),
                    pick_pos: if kind == PoolKind::Max {
                        best_pos
                    } else {
                        None
                    },
                    output_channel: channel,
                    output_row: row,
                    output_col: col,
                    summary: match kind {
                        PoolKind::Max => format!("max({count} values) = {}", fmt(result)),
                        PoolKind::Average => {
                            format!("sum / {count} = {}", fmt(result))
                        }
                    },
                });
            }
        }
    }
    add_grid(scene, &id, &output, "POOLED", style::LIME);
    let (source_frame, operator_frame, output_frame, pick, status) = add_scan_entities(
        scene,
        &id,
        &source,
        &output,
        window,
        window,
        None,
        &format!("{} POOL", kind.label()),
        kind == PoolKind::Max,
        false,
    );
    scene.ml_tensors.insert(id.clone(), output);
    scene.ml_scans.insert(
        id.clone(),
        MlScanData {
            source: source_id,
            output: id,
            operator: format!("{} POOL", kind.label()),
            steps,
            source_frame,
            operator_frame,
            output_frame,
            pick,
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

/// `scan(convolution_or_pool_output, [duration], [ease])`
fn v_scan(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(3)?;
    let id = args.ident(0)?;
    let scan = scene.ml_scans.get(&id).cloned().ok_or_else(|| {
        Error::new(
            format!("`{id}` is not a convolution or pooling result"),
            args.span_of(0),
        )
    })?;
    let fallback = (scan.steps.len() as f32 * 0.42).clamp(2.4, 9.0);
    let duration = args.opt_num(1)?.unwrap_or(fallback);
    if !duration.is_finite() || duration <= 0.0 {
        return Err(Error::new(
            "scan duration must be a positive finite number",
            args.span_of(1),
        ));
    }
    let easing = if args.len() > 2 {
        let word = args.ident(2)?;
        resolve_easing(&word, args.span_of(2))?
    } else {
        Easing::InOutCubic
    };
    let output = scene
        .ml_tensors
        .get(&scan.output)
        .expect("scan output tensor is retained");
    let beat = duration / scan.steps.len().max(1) as f32;
    let settle = (beat * 0.22).min(0.12);
    let mut tracks = Vec::new();
    let mut events = Vec::new();

    // Chained operators reuse the preceding output as their source. Once the
    // next scan starts, the upstream arithmetic strip has served its purpose;
    // quiet it so the new operator owns one clear explanation line.
    if let Some(upstream) = scene.ml_scans.get(&scan.source) {
        tracks.push(track(
            upstream.status.clone(),
            Prop::Opacity,
            TargetValue::Abs(Value::F(0.0)),
            0.0,
            beat.min(0.20),
            Easing::InOutCubic,
        ));
    }

    // Clear the destination values at the start of this scan, then reveal one
    // exact cell at a time. A repeated scan remains deterministic because
    // these are ordinary absolute opacity tracks.
    for channel in 0..output.channels {
        for row in 0..output.rows {
            for col in 0..output.cols {
                let cell = cell_id(&scan.output, channel, row, col);
                tracks.push(track(
                    cell.clone(),
                    Prop::Opacity,
                    TargetValue::Abs(Value::F(0.16)),
                    0.0,
                    settle,
                    Easing::InOutCubic,
                ));
                tracks.push(track(
                    format!("{cell}.value"),
                    Prop::Opacity,
                    TargetValue::Abs(Value::F(0.10)),
                    0.0,
                    settle,
                    Easing::InOutCubic,
                ));
            }
        }
    }
    for frame in [&scan.source_frame, &scan.output_frame] {
        tracks.push(track(
            frame.clone(),
            Prop::Opacity,
            TargetValue::Abs(Value::F(0.95)),
            0.0,
            beat.min(0.20),
            Easing::OutQuad,
        ));
    }
    if let Some(operator) = &scan.operator_frame {
        tracks.push(track(
            operator.clone(),
            Prop::Opacity,
            TargetValue::Abs(Value::F(0.80)),
            0.0,
            beat.min(0.20),
            Easing::OutQuad,
        ));
    }
    tracks.push(track(
        scan.status.clone(),
        Prop::Opacity,
        TargetValue::Abs(Value::F(1.0)),
        0.0,
        beat.min(0.20),
        Easing::OutQuad,
    ));

    for (index, step) in scan.steps.iter().enumerate() {
        let start = index as f32 * beat;
        tracks.push(track(
            scan.source_frame.clone(),
            Prop::Pos,
            TargetValue::Abs(Value::V(step.source_pos)),
            start,
            beat * 0.34,
            easing,
        ));
        tracks.push(track(
            scan.output_frame.clone(),
            Prop::Pos,
            TargetValue::Abs(Value::V(step.output_pos)),
            start,
            beat * 0.34,
            easing,
        ));
        if let Some(pick) = step.pick_pos {
            tracks.push(track(
                scan.pick.clone(),
                Prop::Pos,
                TargetValue::Abs(Value::V(pick)),
                start,
                beat * 0.34,
                easing,
            ));
            tracks.push(track(
                scan.pick.clone(),
                Prop::Opacity,
                TargetValue::Abs(Value::F(1.0)),
                start + beat * 0.24,
                beat * 0.18,
                Easing::OutQuad,
            ));
        } else {
            tracks.push(track(
                scan.pick.clone(),
                Prop::Opacity,
                TargetValue::Abs(Value::F(0.0)),
                start,
                beat * 0.12,
                Easing::InOutCubic,
            ));
        }
        events.push(TextEvent::text(
            scan.status.clone(),
            format!(
                "{} · c{} r{} c{} · {}",
                scan.operator,
                step.output_channel + 1,
                step.output_row + 1,
                step.output_col + 1,
                step.summary
            ),
            start + beat * 0.24,
        ));
        let cell = cell_id(
            &scan.output,
            step.output_channel,
            step.output_row,
            step.output_col,
        );
        tracks.push(track(
            cell.clone(),
            Prop::Opacity,
            TargetValue::Abs(Value::F(0.94)),
            start + beat * 0.48,
            beat * 0.30,
            easing,
        ));
        tracks.push(track(
            format!("{cell}.value"),
            Prop::Opacity,
            TargetValue::Abs(Value::F(1.0)),
            start + beat * 0.48,
            beat * 0.30,
            easing,
        ));
        tracks.push(track(
            cell.clone(),
            Prop::Scale,
            TargetValue::Abs(Value::F(1.10)),
            start + beat * 0.46,
            beat * 0.20,
            Easing::OutQuad,
        ));
        tracks.push(track(
            cell,
            Prop::Scale,
            TargetValue::Abs(Value::F(1.0)),
            start + beat * 0.68,
            beat * 0.22,
            easing,
        ));
    }
    let fade_start = duration * 0.94;
    for frame in [&scan.source_frame, &scan.output_frame, &scan.pick] {
        tracks.push(track(
            frame.clone(),
            Prop::Opacity,
            TargetValue::Abs(Value::F(0.0)),
            fade_start,
            duration - fade_start,
            Easing::InOutCubic,
        ));
    }
    if let Some(operator) = &scan.operator_frame {
        tracks.push(track(
            operator.clone(),
            Prop::Opacity,
            TargetValue::Abs(Value::F(0.0)),
            fade_start,
            duration - fade_start,
            Easing::InOutCubic,
        ));
    }
    events.push(TextEvent::text(
        scan.status,
        format!(
            "{} · scan complete · {} output cells",
            scan.operator,
            scan.steps.len()
        ),
        duration * 0.96,
    ));
    Ok(Clip {
        tracks,
        events,
        dur: duration,
    })
}

pub fn register(registry: &mut Registry) {
    registry.ctor("tensor", c_tensor);
    registry.ctor("kernel", c_kernel);
    registry.ctor("convolve", c_convolve);
    registry.ctor("pool", c_pool);
    registry.mut_verb("scan", v_scan);
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

    #[test]
    fn tensor_parser_supports_channels_and_rejects_ragged_grids() {
        let data = parse_grid("1 2; 3 4 | 5 6; 7 8", "tensor").unwrap();
        assert_eq!((data.channels, data.rows, data.cols), (2, 2, 2));
        assert_eq!(data.values[1][1][0], 7.0);
        assert!(parse_grid("1 2; 3", "tensor").is_err());
        assert!(parse_grid("1 2; 3 4 | 5 6", "tensor").is_err());
    }

    #[test]
    fn convolution_matches_a_textbook_edge_kernel() {
        let m = movie(
            "tensor(x,(250,300),\"1 2 3; 4 5 6; 7 8 9\",40); kernel(k,(500,300),\"1 0; 0 -1\",40); convolve(y,x,k,(760,300),1,0,0,linear,40);",
        );
        let output = &m.scene.ml_tensors["y"];
        assert_eq!((output.channels, output.rows, output.cols), (1, 2, 2));
        assert_eq!(output.values[0], vec![vec![-4.0, -4.0], vec![-4.0, -4.0]]);
    }

    #[test]
    fn multichannel_convolution_sums_every_input_channel() {
        let m = movie(
            "tensor(x,(250,300),\"1 2; 3 4 | 10 20; 30 40\",40); kernel(k,(500,300),\"1 | 0.5\",40); convolve(y,x,k,(760,300));",
        );
        assert_eq!(
            m.scene.ml_tensors["y"].values[0],
            vec![vec![6.0, 12.0], vec![18.0, 24.0]]
        );
        let Shape::Rect { w, .. } = &m.scene.get("y.scan.source").unwrap().shape else {
            panic!("scan source should be a rectangle");
        };
        assert!(*w > m.scene.ml_tensors["x"].cell);
    }

    #[test]
    fn convolution_stride_padding_bias_and_relu_are_truthful() {
        let m = movie(
            "tensor(x,(250,300),\"1 2; 3 4\",40); kernel(k,(500,300),\"1 1; 1 1\",40); convolve(y,x,k,(760,300),2,1,-2,relu,40);",
        );
        assert_eq!(
            m.scene.ml_tensors["y"].values[0],
            vec![vec![0.0, 0.0], vec![1.0, 2.0]]
        );
    }

    #[test]
    fn max_and_average_pooling_are_exact_and_ties_are_stable() {
        let m = movie(
            "tensor(x,(250,300),\"4 4 1 2; 4 3 5 5; 0 1 5 2; 2 2 1 1\",40); pool(mx,x,(650,220),max,2,2,0,40); pool(av,x,(650,500),average,2,2,0,40);",
        );
        assert_eq!(
            m.scene.ml_tensors["mx"].values[0],
            vec![vec![4.0, 5.0], vec![2.0, 5.0]]
        );
        assert_eq!(
            m.scene.ml_tensors["av"].values[0],
            vec![vec![3.75, 3.25], vec![1.25, 2.25]]
        );
        let first = &m.scene.ml_scans["mx"].steps[0];
        assert_eq!(
            first.pick_pos,
            Some(grid_pos(&m.scene.ml_tensors["x"], 0.0, 0.0, 0.0))
        );
    }

    #[test]
    fn pooling_padding_excludes_virtual_cells_from_the_value() {
        let m = movie(
            "tensor(x,(250,300),\"4\",40); pool(mx,x,(600,220),max,2,1,1,40); pool(av,x,(600,500),average,2,1,1,40);",
        );
        let expected = vec![vec![4.0, 4.0], vec![4.0, 4.0]];
        assert_eq!(m.scene.ml_tensors["mx"].values[0], expected);
        assert_eq!(m.scene.ml_tensors["av"].values[0], expected);
    }

    #[test]
    fn a_downstream_scan_quiets_the_upstream_arithmetic_strip() {
        let m = movie(
            "tensor(x,(180,300),\"1 2; 3 4\",40); kernel(k,(430,300),\"1\",40); convolve(y,x,k,(680,300)); pool(p,y,(900,300),max,2); scan(y,1); scan(p,1);",
        );
        let final_frame = frame(&m, 2.0);
        assert_eq!(final_frame.get("y.scan.status").unwrap().opacity, 0.0);
        assert_eq!(final_frame.get("p.scan.status").unwrap().opacity, 1.0);
    }

    #[test]
    fn scan_is_stateless_when_frames_are_requested_out_of_order() {
        let m = movie(
            "tensor(x,(250,300),\"1 2 3; 4 5 6; 7 8 9\",40); kernel(k,(500,300),\"1 0; 0 -1\",40); convolve(y,x,k,(760,300)); scan(y,2,smooth);",
        );
        let late_first = frame(&m, 1.4);
        let early = frame(&m, 0.2);
        let late_again = frame(&m, 1.4);
        assert_eq!(
            late_first.get("y.scan.source").unwrap().pos,
            late_again.get("y.scan.source").unwrap().pos
        );
        assert_eq!(
            late_first.get("y.c0.r1c0").unwrap().opacity,
            late_again.get("y.c0.r1c0").unwrap().opacity
        );
        assert_ne!(
            early.get("y.scan.source").unwrap().pos,
            late_again.get("y.scan.source").unwrap().pos
        );
    }

    #[test]
    fn invalid_shapes_channels_and_scan_targets_fail_early() {
        assert!(crate::parse("tensor(x,(0,0),\"1 2; 3\");").is_err());
        assert!(crate::parse(
            "tensor(x,(0,0),\"1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1\");"
        )
        .is_err());
        assert!(crate::parse(
            "tensor(x,(0,0),\"1 2; 3 4 | 5 6; 7 8\"); kernel(k,(0,0),\"1\"); convolve(y,x,k,(0,0));"
        )
        .is_err());
        assert!(crate::parse(
            "tensor(x,(0,0),\"1\"); kernel(k,(0,0),\"1\"); convolve(y,x,k,(0,0),1,10);"
        )
        .is_err());
        assert!(crate::parse("tensor(x,(0,0),\"1 2; 3 4\"); pool(y,x,(0,0),max,3);").is_err());
        assert!(crate::parse("tensor(x,(0,0),\"1\"); pool(y,x,(0,0),average,1,1,10);").is_err());
        assert!(crate::parse("tensor(x,(0,0),\"1\"); scan(x);").is_err());
    }
}
