//! LaTeX math typesetting via [RaTeX](https://github.com/erweixin/RaTeX) — a
//! pure-Rust, KaTeX-grade renderer. We take a LaTeX string through
//! parse → layout → display-list, recolour every item WHITE, and rasterise it to
//! a transparent PNG. manic then draws that PNG **tinted by the entity's colour**
//! (`Shape::Image { tint: true }`), so an equation follows the template palette
//! and can be `color`/`recolor`-ed like any text.
//!
//! Fonts are baked in by RaTeX's `embed-fonts` feature — the binary is
//! self-contained (no system fonts, no font directory).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use ratex_types::color::Color as RatexColor;
use ratex_types::display_item::{DisplayItem, DisplayList};

use crate::style::Palette;

const EQUATION_PAD: f32 = 6.0;
const PART_PAD: u32 = 2;

/// Logical crop of one RaTeX display item inside the complete equation image.
/// Stored at build time so the player can rerasterise the same item sharply at
/// any recording scale without repeating alpha-bound discovery.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EquationPartCrop {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Coarse mathematical layout role for one rendered item. RaTeX's display
/// list is intentionally flat, so `rewrite` records enough structure here to
/// prevent visually identical glyphs from changing jobs (for example the
/// exponent in `x^2` becoming the denominator in `b/(2a)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquationPartRole {
    Main,
    Above,
    Below,
    Numerator,
    Denominator,
    FractionRule,
    Structural,
}

/// One independently animatable visual item from a laid-out equation.
#[derive(Debug, Clone)]
pub struct EquationPart {
    pub index: usize,
    pub key: String,
    /// Unicode identity for glyph items. Structural rules and paths have no
    /// symbol. Keeping this alongside the visual key lets `rewrite` recognise
    /// stable relation anchors without reverse-engineering the cache key.
    pub symbol: Option<u32>,
    pub role: EquationPartRole,
    /// RaTeX math-style scale for glyphs (`1.0` on the main line, then smaller
    /// for scripts and nested scripts). Matching keeps distinct script depths
    /// apart even when their coarse region is the same, as in `d^2/(dx^2)`.
    pub layout_scale: Option<f32>,
    pub prev_key: Option<String>,
    pub next_key: Option<String>,
    pub path: String,
    pub crop: EquationPartCrop,
    /// Offset of the cropped item's centre from the whole equation's centre.
    pub offset: macroquad::prelude::Vec2,
}

/// Exact RaTeX layout plus the cropped visual items used by `rewrite`.
#[derive(Debug, Clone)]
pub struct EquationLayout {
    pub w: f32,
    pub h: f32,
    pub parts: Vec<EquationPart>,
}

fn display_list(latex: &str) -> Result<DisplayList, String> {
    let nodes = ratex_parser::parse(latex).map_err(|e| format!("{e:?}"))?;
    let lbox = ratex_layout::layout(&nodes, &ratex_layout::LayoutOptions::default());
    Ok(ratex_layout::to_display_list(&lbox))
}

fn item_color(item: &DisplayItem) -> RatexColor {
    match item {
        DisplayItem::GlyphPath { color, .. }
        | DisplayItem::Line { color, .. }
        | DisplayItem::Rect { color, .. }
        | DisplayItem::Path { color, .. } => *color,
    }
}

fn color_key(c: RatexColor) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}",
        (c.r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        (c.a.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

fn quant(v: f64) -> i64 {
    (v * 1000.0).round() as i64
}

/// A location-independent visual identity. Glyph scale is intentionally not
/// part of the key: an `x` may move between normal text and a superscript while
/// visibly remaining the same symbol. Structural rules and paths retain their
/// dimensions so unrelated fraction bars/radicals do not stretch unnaturally.
fn item_key(item: &DisplayItem) -> String {
    let color = color_key(item_color(item));
    match item {
        DisplayItem::GlyphPath {
            font, char_code, ..
        } => format!("g:{font}:{char_code}:{color}"),
        DisplayItem::Line {
            width,
            thickness,
            dashed,
            ..
        } => format!(
            "l:{}:{}:{}:{color}",
            quant(*width),
            quant(*thickness),
            u8::from(*dashed)
        ),
        DisplayItem::Rect { width, height, .. } => {
            format!("r:{}:{}:{color}", quant(*width), quant(*height))
        }
        DisplayItem::Path { commands, fill, .. } => {
            // Debug text is deterministic for this value-only enum and excludes
            // the item's absolute x/y placement.
            format!("p:{}:{fill}:{commands:?}:{color}", commands.len())
        }
    }
}

fn render_list(dl: &DisplayList, size: f32, dpr: f32, padding: f32) -> Result<Vec<u8>, String> {
    ratex_render::render_to_png(
        dl,
        &ratex_render::RenderOptions {
            font_size: size,
            padding,
            background_color: RatexColor::new(0.0, 0.0, 0.0, 0.0),
            device_pixel_ratio: dpr,
            ..Default::default()
        },
    )
}

fn discover_crop(dl: &DisplayList, index: usize, size: f32) -> Result<EquationPartCrop, String> {
    let item = dl
        .items
        .get(index)
        .ok_or_else(|| format!("equation display item {index} is missing"))?
        .clone();
    let isolated = DisplayList {
        items: vec![item],
        width: dl.width,
        height: dl.height,
        depth: dl.depth,
    };
    let png = render_list(&isolated, size, 1.0, EQUATION_PAD)?;
    let rgba = image::load_from_memory(&png)
        .map_err(|e| format!("decode equation item: {e}"))?
        .to_rgba8();
    let (iw, ih) = rgba.dimensions();
    let mut min_x = iw;
    let mut min_y = ih;
    let mut max_x = 0_u32;
    let mut max_y = 0_u32;
    let mut found = false;
    for (x, y, px) in rgba.enumerate_pixels() {
        if px[3] == 0 {
            continue;
        }
        found = true;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    if !found {
        return Err(format!(
            "equation display item {index} has no visible pixels"
        ));
    }
    let x0 = min_x.saturating_sub(PART_PAD);
    let y0 = min_y.saturating_sub(PART_PAD);
    let x1 = (max_x + 1 + PART_PAD).min(iw);
    let y1 = (max_y + 1 + PART_PAD).min(ih);
    Ok(EquationPartCrop {
        x: x0 as f32,
        y: y0 as f32,
        w: (x1 - x0).max(1) as f32,
        h: (y1 - y0).max(1) as f32,
    })
}

/// Deterministic cache path for one cropped RaTeX display item.
pub fn eq_part_path(latex: &str, size: f32, index: usize) -> String {
    let mut hasher = DefaultHasher::new();
    "part-v1".hash(&mut hasher);
    latex.hash(&mut hasher);
    size.to_bits().hash(&mut hasher);
    index.hash(&mut hasher);
    let key = hasher.finish();
    std::env::temp_dir()
        .join("manic-eq")
        .join(format!("part-{key:016x}.png"))
        .to_string_lossy()
        .into_owned()
}

/// Lay out an equation into exact, cropped RaTeX display items. This work is
/// only requested by the opt-in `rewrite` verb; ordinary equations retain the
/// existing single-image fast path.
pub fn layout_parts(latex: &str, size: f32) -> Result<EquationLayout, String> {
    let dl = display_list(latex)?;
    let w = (dl.width as f32 * size + 2.0 * EQUATION_PAD).max(1.0);
    let h = ((dl.height + dl.depth) as f32 * size + 2.0 * EQUATION_PAD).max(1.0);
    let keys: Vec<String> = dl.items.iter().map(item_key).collect();
    let crops: Vec<EquationPartCrop> = (0..dl.items.len())
        .map(|index| discover_crop(&dl, index, size))
        .collect::<Result<_, _>>()?;
    let centres: Vec<macroquad::prelude::Vec2> = crops
        .iter()
        .map(|crop| macroquad::prelude::Vec2::new(crop.x + crop.w / 2.0, crop.y + crop.h / 2.0))
        .collect();

    // A horizontal rule is a fraction bar only when it separates visible
    // glyphs above and below inside its span. This excludes radical overbars,
    // overlines, and matrix rules from numerator/denominator classification.
    let fraction_rules: Vec<usize> = dl
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            if !matches!(item, DisplayItem::Line { dashed: false, .. }) {
                return None;
            }
            let crop = crops[index];
            let left = crop.x - 2.0;
            let right = crop.x + crop.w + 2.0;
            let y = centres[index].y;
            let near = size * 1.35;
            let has_above = dl.items.iter().enumerate().any(|(i, item)| {
                matches!(item, DisplayItem::GlyphPath { .. })
                    && centres[i].x >= left
                    && centres[i].x <= right
                    && centres[i].y < y - 1.0
                    && y - centres[i].y <= near
            });
            let has_below = dl.items.iter().enumerate().any(|(i, item)| {
                matches!(item, DisplayItem::GlyphPath { .. })
                    && centres[i].x >= left
                    && centres[i].x <= right
                    && centres[i].y > y + 1.0
                    && centres[i].y - y <= near
            });
            (has_above && has_below).then_some(index)
        })
        .collect();

    let baseline_y = dl.height as f32 * size + EQUATION_PAD;
    let mut parts = Vec::with_capacity(dl.items.len());
    for index in 0..dl.items.len() {
        let crop = crops[index];
        let center = centres[index];
        let role = match &dl.items[index] {
            DisplayItem::GlyphPath { .. } => {
                let containing_rule = fraction_rules
                    .iter()
                    .copied()
                    .filter(|rule_index| {
                        let rule = crops[*rule_index];
                        center.x >= rule.x - 2.0
                            && center.x <= rule.x + rule.w + 2.0
                            && (center.y - centres[*rule_index].y).abs() <= size * 1.35
                    })
                    // Prefer the narrowest containing rule for nested fractions.
                    .min_by(|a, b| crops[*a].w.total_cmp(&crops[*b].w));
                if let Some(rule_index) = containing_rule {
                    if center.y < centres[rule_index].y {
                        EquationPartRole::Numerator
                    } else {
                        EquationPartRole::Denominator
                    }
                } else if center.y < baseline_y - size * 0.12 {
                    EquationPartRole::Above
                } else if center.y > baseline_y + size * 0.12 {
                    EquationPartRole::Below
                } else {
                    EquationPartRole::Main
                }
            }
            DisplayItem::Line { .. } if fraction_rules.contains(&index) => {
                EquationPartRole::FractionRule
            }
            _ => EquationPartRole::Structural,
        };
        parts.push(EquationPart {
            index,
            key: keys[index].clone(),
            symbol: match &dl.items[index] {
                DisplayItem::GlyphPath { char_code, .. } => Some(*char_code),
                _ => None,
            },
            role,
            layout_scale: match &dl.items[index] {
                DisplayItem::GlyphPath { scale, .. } => Some(*scale as f32),
                _ => None,
            },
            prev_key: index.checked_sub(1).map(|i| keys[i].clone()),
            next_key: keys.get(index + 1).cloned(),
            path: eq_part_path(latex, size, index),
            crop,
            offset: center - macroquad::prelude::Vec2::new(w / 2.0, h / 2.0),
        });
    }
    Ok(EquationLayout { w, h, parts })
}

fn shift_item(item: &mut DisplayItem, dx: f64, dy: f64) {
    match item {
        DisplayItem::GlyphPath { x, y, .. }
        | DisplayItem::Line { x, y, .. }
        | DisplayItem::Rect { x, y, .. }
        | DisplayItem::Path { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
    }
}

/// Rasterise one cropped display item at the requested output density.
pub fn render_part_to_path(
    latex: &str,
    size: f32,
    dpr: f32,
    index: usize,
    crop: EquationPartCrop,
    preserve_color: bool,
    path: &str,
) -> Result<(), String> {
    let dl = display_list(latex)?;
    let mut item = dl
        .items
        .get(index)
        .ok_or_else(|| format!("equation display item {index} is missing"))?
        .clone();
    // Original full-equation pixels are item*size + EQUATION_PAD. Translate
    // that crop into a zero-padded local image.
    shift_item(
        &mut item,
        ((EQUATION_PAD - crop.x) / size) as f64,
        ((EQUATION_PAD - crop.y) / size) as f64,
    );
    if !preserve_color {
        match &mut item {
            DisplayItem::GlyphPath { color, .. }
            | DisplayItem::Line { color, .. }
            | DisplayItem::Rect { color, .. }
            | DisplayItem::Path { color, .. } => *color = RatexColor::WHITE,
        }
    }
    let cropped = DisplayList {
        items: vec![item],
        width: (crop.w / size) as f64,
        height: (crop.h / size) as f64,
        depth: 0.0,
    };
    let png = render_list(&cropped, size, dpr, 0.0)?;
    write_png_atomic(path, &png)
}

fn write_png_atomic(path: &str, png: &[u8]) -> Result<(), String> {
    let p = PathBuf::from(path);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&tmp, png).map_err(|e| e.to_string())?;
    let _ = std::fs::rename(&tmp, &p);
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

/// Render `latex` to a white-on-transparent PNG at `em_px` em-size and `dpr`
/// pixel density. Returns the PNG bytes, its pixel width/height, and the baseline
/// offset (pixels from the top of the image down to the math baseline — for
/// inline alignment with surrounding text).
pub fn render_png(latex: &str, em_px: f32, dpr: f32) -> Result<(Vec<u8>, u32, u32, f32), String> {
    let nodes = ratex_parser::parse(latex).map_err(|e| format!("{e:?}"))?;
    let lbox = ratex_layout::layout(&nodes, &ratex_layout::LayoutOptions::default());
    let mut dl = ratex_layout::to_display_list(&lbox);

    // Recolour to white so manic can tint the texture to the template colour.
    for item in &mut dl.items {
        match item {
            DisplayItem::GlyphPath { color, .. }
            | DisplayItem::Line { color, .. }
            | DisplayItem::Rect { color, .. }
            | DisplayItem::Path { color, .. } => *color = RatexColor::WHITE,
        }
    }

    let pad = EQUATION_PAD;
    let opts = ratex_render::RenderOptions {
        font_size: em_px,
        padding: pad,
        background_color: RatexColor::new(0.0, 0.0, 0.0, 0.0), // transparent
        device_pixel_ratio: dpr,
        ..Default::default()
    };
    let png = ratex_render::render_to_png(&dl, &opts)?;

    // Pixel dimensions (mirrors ratex-render's own computation).
    let em = em_px * dpr;
    let pad_px = pad * dpr;
    let w = ((dl.width as f32) * em + 2.0 * pad_px).ceil().max(1.0) as u32;
    let h = ((dl.height + dl.depth) as f32 * em + 2.0 * pad_px)
        .ceil()
        .max(1.0) as u32;
    let baseline = (dl.height as f32) * em + pad_px; // top → baseline, in pixels
    Ok((png, w, h, baseline))
}

/// Whether a formula requests explicit term colours through standard LaTeX.
pub fn has_explicit_color(latex: &str) -> bool {
    latex.contains("\\textcolor{") || latex.contains("\\color{")
}

fn hex(c: macroquad::prelude::Color) -> String {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", q(c.r), q(c.g), q(c.b))
}

/// Resolve Manic's semantic names inside `\textcolor{...}` / `\color{...}`
/// through the active template and wrap uncoloured terms in that template's
/// foreground. The returned string is ordinary LaTeX understood by RaTeX.
pub fn with_palette_colors(latex: &str, palette: &Palette) -> String {
    let roles = [
        ("fg", palette.fg),
        ("white", palette.fg),
        ("cyan", palette.cyan),
        ("magenta", palette.magenta),
        ("pink", palette.magenta),
        ("accent", palette.magenta),
        ("lime", palette.lime),
        ("green", palette.lime),
        ("gold", palette.gold),
        ("amber", palette.gold),
        ("yellow", palette.gold),
        ("red", palette.red),
        ("crimson", palette.red),
        ("orange", palette.orange),
        ("blue", palette.blue),
        ("azure", palette.blue),
        ("dim", palette.dim),
        ("gray", palette.dim),
        ("grey", palette.dim),
        ("panel", palette.panel),
        ("void", palette.bg),
        ("bg", palette.bg),
    ];
    let mut out = latex.to_string();
    for (name, color) in roles {
        let mapped = hex(color);
        out = out.replace(
            &format!("\\textcolor{{{name}}}"),
            &format!("\\textcolor{{{mapped}}}"),
        );
        out = out.replace(
            &format!("\\color{{{name}}}"),
            &format!("\\color{{{mapped}}}"),
        );
    }
    format!("\\textcolor{{{}}}{{{out}}}", hex(palette.fg))
}

/// Render a formula while preserving the colours already carried by its
/// display list. Used for semantic `\textcolor` equations; ordinary equations
/// still take the cheaper white-texture + runtime tint path above.
pub fn render_png_preserve(
    latex: &str,
    em_px: f32,
    dpr: f32,
) -> Result<(Vec<u8>, u32, u32, f32), String> {
    let nodes = ratex_parser::parse(latex).map_err(|e| format!("{e:?}"))?;
    let lbox = ratex_layout::layout(&nodes, &ratex_layout::LayoutOptions::default());
    let dl = ratex_layout::to_display_list(&lbox);
    let pad = EQUATION_PAD;
    let opts = ratex_render::RenderOptions {
        font_size: em_px,
        padding: pad,
        background_color: RatexColor::new(0.0, 0.0, 0.0, 0.0),
        device_pixel_ratio: dpr,
        ..Default::default()
    };
    let png = ratex_render::render_to_png(&dl, &opts)?;
    let em = em_px * dpr;
    let pad_px = pad * dpr;
    let w = ((dl.width as f32) * em + 2.0 * pad_px).ceil().max(1.0) as u32;
    let h = ((dl.height + dl.depth) as f32 * em + 2.0 * pad_px)
        .ceil()
        .max(1.0) as u32;
    let baseline = (dl.height as f32) * em + pad_px;
    Ok((png, w, h, baseline))
}

/// LOGICAL (dpr-independent) box of a typeset equation, in px: total width/height
/// and the baseline offset from the top. Cheap — layout only, no rasterisation.
pub fn layout_dims(latex: &str, size: f32) -> Result<(f32, f32, f32), String> {
    let nodes = ratex_parser::parse(latex).map_err(|e| format!("{e:?}"))?;
    let lbox = ratex_layout::layout(&nodes, &ratex_layout::LayoutOptions::default());
    let dl = ratex_layout::to_display_list(&lbox);
    let pad = EQUATION_PAD;
    let w = ((dl.width as f32) * size + 2.0 * pad).max(1.0);
    let h = ((dl.height + dl.depth) as f32 * size + 2.0 * pad).max(1.0);
    let baseline = (dl.height as f32) * size + pad;
    Ok((w, h, baseline))
}

/// The deterministic cache path for an equation, keyed by (latex, size) only —
/// the pixel density is chosen at render time, so one logical equation maps to
/// one file that the player (re)renders at the current render scale.
pub fn eq_path(latex: &str, size: f32) -> String {
    let mut hasher = DefaultHasher::new();
    latex.hash(&mut hasher);
    size.to_bits().hash(&mut hasher);
    let key = hasher.finish();
    std::env::temp_dir()
        .join("manic-eq")
        .join(format!("{key:016x}.png"))
        .to_string_lossy()
        .into_owned()
}

/// Rasterise `latex` at `dpr` and write it to `path` (atomic). Called by the
/// player with `dpr = render scale`, so the PNG is pixel-1:1 with the display.
pub fn render_to_path(latex: &str, size: f32, dpr: f32, path: &str) -> Result<(), String> {
    let (png, _, _, _) = render_png(latex, size, dpr)?;
    write_png_atomic(path, &png)
}

/// Coloured-equation counterpart of [`render_to_path`].
pub fn render_to_path_preserve(latex: &str, size: f32, dpr: f32, path: &str) -> Result<(), String> {
    let (png, _, _, _) = render_png_preserve(latex, size, dpr)?;
    write_png_atomic(path, &png)
}

#[cfg(test)]
mod semantic_color_tests {
    use super::*;

    #[test]
    fn semantic_latex_colours_follow_the_template_palette() {
        let palette = crate::style::Palette::paper();
        let styled = with_palette_colors(
            r"\textcolor{magenta}{\mathrm{slope}}=\frac{\textcolor{cyan}{\mathrm{rise}}}{\textcolor{gold}{\mathrm{run}}}",
            &palette,
        );
        assert!(
            styled.contains("#c7176b"),
            "paper magenta should be baked: {styled}"
        );
        assert!(
            styled.contains("#00709e"),
            "paper cyan should be baked: {styled}"
        );
        assert!(styled.starts_with("\\textcolor{#1f1f29}"));
        let (png, w, h, _) = render_png_preserve(&styled, 36.0, 1.5).unwrap();
        assert!(png.len() > 100 && w > 10 && h > 10);
    }
}
