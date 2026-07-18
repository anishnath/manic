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

use ratex_types::color::Color;
use ratex_types::display_item::DisplayItem;

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
            | DisplayItem::Path { color, .. } => *color = Color::WHITE,
        }
    }

    let pad = 6.0_f32;
    let opts = ratex_render::RenderOptions {
        font_size: em_px,
        padding: pad,
        background_color: Color::new(0.0, 0.0, 0.0, 0.0), // transparent
        device_pixel_ratio: dpr,
        ..Default::default()
    };
    let png = ratex_render::render_to_png(&dl, &opts)?;

    // Pixel dimensions (mirrors ratex-render's own computation).
    let em = em_px * dpr;
    let pad_px = pad * dpr;
    let w = ((dl.width as f32) * em + 2.0 * pad_px).ceil().max(1.0) as u32;
    let h = ((dl.height + dl.depth) as f32 * em + 2.0 * pad_px).ceil().max(1.0) as u32;
    let baseline = (dl.height as f32) * em + pad_px; // top → baseline, in pixels
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
