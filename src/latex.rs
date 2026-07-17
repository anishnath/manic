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

/// Write `png` to a stable per-content cache file and return its path. Keyed by
/// (latex, size) so identical equations reuse one file; drawn by `Shape::Image`.
pub fn cache_png(latex: &str, size: f32, png: &[u8]) -> Result<String, String> {
    let mut hasher = DefaultHasher::new();
    latex.hash(&mut hasher);
    size.to_bits().hash(&mut hasher);
    let key = hasher.finish();

    let dir: PathBuf = std::env::temp_dir().join("manic-eq");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{key:016x}.png"));
    if !path.exists() {
        // Atomic write: a unique temp file + rename, so concurrent renders of the
        // same equation can't observe (or produce) a half-written PNG.
        let tmp = dir.join(format!("{key:016x}.{}.tmp", std::process::id()));
        std::fs::write(&tmp, png).map_err(|e| e.to_string())?;
        // rename is atomic on the same filesystem; ignore the race where another
        // process already produced the final file.
        let _ = std::fs::rename(&tmp, &path);
        let _ = std::fs::remove_file(&tmp);
    }
    path.into_os_string().into_string().map_err(|_| "non-utf8 cache path".to_string())
}
