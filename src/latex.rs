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
use ratex_types::display_item::DisplayItem;

use crate::style::Palette;

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

    let pad = 6.0_f32;
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
    let pad = 6.0_f32;
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
    let pad = 6.0_f32;
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
    let p = PathBuf::from(path);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&tmp, &png).map_err(|e| e.to_string())?;
    let _ = std::fs::rename(&tmp, &p);
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

/// Coloured-equation counterpart of [`render_to_path`].
pub fn render_to_path_preserve(latex: &str, size: f32, dpr: f32, path: &str) -> Result<(), String> {
    let (png, _, _, _) = render_png_preserve(latex, size, dpr)?;
    let p = PathBuf::from(path);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&tmp, &png).map_err(|e| e.to_string())?;
    let _ = std::fs::rename(&tmp, &p);
    let _ = std::fs::remove_file(&tmp);
    Ok(())
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
