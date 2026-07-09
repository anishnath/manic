//! The house visual style: **neon terminal / synthwave**.
//!
//! A near-black void, glowing monospace type, and three saturated neon spot
//! colors (cyan, magenta, lime). Every video reads like a frame captured off a
//! phosphor CRT running the same terminal. Change the palette here and every
//! movie follows.

use macroquad::prelude::Color;
use macroquad::text::{load_ttf_font_from_bytes, Font};

/// Background: deep indigo-black — the void behind the phosphor.
pub const VOID: Color = Color::new(0.051, 0.043, 0.102, 1.0);
/// Primary foreground: soft lavender-white glow. The default "ink".
pub const FG: Color = Color::new(0.878, 0.882, 0.953, 1.0);
/// Spot color: hot magenta. Use for highlights, set bits, the thing on fire.
pub const MAGENTA: Color = Color::new(1.0, 0.176, 0.584, 1.0);
/// Primary structural neon: electric cyan. Node outlines, the main current.
pub const CYAN: Color = Color::new(0.0, 0.898, 1.0, 1.0);
/// Secondary spot color: acid lime. Use for "the other branch" / success.
pub const LIME: Color = Color::new(0.486, 1.0, 0.42, 1.0);
/// De-emphasised muted slate-violet for annotations, indices, rules.
pub const DIM: Color = Color::new(0.42, 0.40, 0.56, 1.0);
/// Slightly lifted panel fill (nodes, cells, section cards) over the void.
pub const PANEL: Color = Color::new(0.098, 0.086, 0.169, 1.0);

/// Prompt printed at the top-left of every frame (fake shell).
pub const MASTHEAD_LEFT: &str = "manic ~ %";
/// Status printed at the top-right of every frame.
pub const MASTHEAD_RIGHT: &str = "60FPS · DETERMINISTIC";

/// Loaded font set. `None` fields fall back to macroquad's built-in font.
///
/// The neon-terminal look is all monospace: `display` (headlines) and
/// `mono_bold` (emphasised labels) share IBM Plex Mono Bold; `mono` is the
/// regular weight for data and captions. Bold bytes are embedded once and the
/// [`Font`] cloned into both bold slots.
pub struct Fonts {
    pub display: Option<Font>,
    pub mono: Option<Font>,
    pub mono_bold: Option<Font>,
}

impl Fonts {
    /// Load the embedded house fonts (IBM Plex Mono, OFL-licensed and compiled
    /// into the binary, so movies render identically on any machine).
    pub fn load() -> Fonts {
        let bold = load_ttf_font_from_bytes(include_bytes!(
            "../assets/fonts/IBMPlexMono-Bold.ttf"
        ))
        .ok();
        Fonts {
            display: bold.clone(),
            mono: load_ttf_font_from_bytes(include_bytes!(
                "../assets/fonts/IBMPlexMono-Regular.ttf"
            ))
            .ok(),
            mono_bold: bold,
        }
    }
}

/// `c` with its alpha multiplied by `opacity`.
pub fn with_opacity(c: Color, opacity: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * opacity)
}
