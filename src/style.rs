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
pub const GOLD: Color = Color::new(1.0, 0.82, 0.4, 1.0);
/// De-emphasised muted slate-violet for annotations, indices, rules.
pub const DIM: Color = Color::new(0.42, 0.40, 0.56, 1.0);
/// Slightly lifted panel fill (nodes, cells, section cards) over the void.
pub const PANEL: Color = Color::new(0.098, 0.086, 0.169, 1.0);

/// The seven semantic colour roles a template can retint. The engine bakes the
/// neon values everywhere; the renderer remaps them to the active template's
/// palette at draw time (so `--template` retints content, and bespoke colours —
/// `hue`, explicit RGB — pass through untouched).
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub fg: Color,
    pub cyan: Color,
    pub magenta: Color,
    pub lime: Color,
    pub dim: Color,
    pub panel: Color,
}

impl Palette {
    /// The house neon-terminal palette — the values the engine bakes with.
    pub fn neon() -> Palette {
        Palette {
            bg: VOID,
            fg: FG,
            cyan: CYAN,
            magenta: MAGENTA,
            lime: LIME,
            dim: DIM,
            panel: PANEL,
        }
    }

    /// A light ink-on-cream palette for a print / handout look.
    pub fn paper() -> Palette {
        Palette {
            bg: Color::new(0.96, 0.95, 0.90, 1.0),
            fg: Color::new(0.12, 0.12, 0.16, 1.0),
            cyan: Color::new(0.0, 0.44, 0.62, 1.0),
            magenta: Color::new(0.78, 0.09, 0.42, 1.0),
            lime: Color::new(0.18, 0.53, 0.22, 1.0),
            dim: Color::new(0.55, 0.54, 0.58, 1.0),
            panel: Color::new(0.89, 0.88, 0.83, 1.0),
        }
    }

    /// A blueprint palette — white/cyan lines on deep navy.
    pub fn blueprint() -> Palette {
        Palette {
            bg: Color::new(0.05, 0.11, 0.24, 1.0),
            fg: Color::new(0.90, 0.94, 1.0, 1.0),
            cyan: Color::new(0.55, 0.80, 1.0, 1.0),
            magenta: Color::new(1.0, 0.62, 0.40, 1.0),
            lime: Color::new(0.62, 0.92, 0.70, 1.0),
            dim: Color::new(0.45, 0.58, 0.80, 1.0),
            panel: Color::new(0.10, 0.18, 0.34, 1.0),
        }
    }

    /// Remap a neon-palette colour to this palette's corresponding role,
    /// preserving alpha. Colours that aren't a palette role pass through.
    pub fn remap(&self, c: Color) -> Color {
        let neon = Palette::neon();
        let close = |a: Color, b: Color| {
            (a.r - b.r).abs() < 0.004 && (a.g - b.g).abs() < 0.004 && (a.b - b.b).abs() < 0.004
        };
        for (from, to) in [
            (neon.bg, self.bg),
            (neon.fg, self.fg),
            (neon.cyan, self.cyan),
            (neon.magenta, self.magenta),
            (neon.lime, self.lime),
            (neon.dim, self.dim),
            (neon.panel, self.panel),
        ] {
            if close(c, from) {
                return Color::new(to.r, to.g, to.b, c.a);
            }
        }
        c
    }
}

/// How much page chrome a template draws under the content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chrome {
    /// Nothing but the background — a clean, blank screen (the default).
    None,
    /// The masthead line only (no window frame).
    Minimal,
    /// The full neon terminal frame: border, dots, title, masthead, rule.
    Full,
}

/// A visual template: what look the whole movie renders in. The default is
/// `plain` (a blank screen); `terminal` is the opt-in neon-terminal chrome.
#[derive(Debug, Clone)]
pub struct Template {
    pub name: String,
    pub chrome: Chrome,
    pub palette: Palette,
    /// Neon-halo multiplier applied to every entity's glow (1 = house default,
    /// 0 = crisp, no halo — right for print/blueprint looks).
    pub glow: f32,
    /// Whether the CRT post-process is on by default (`--crt` can force it on).
    pub crt: bool,
    pub masthead_left: String,
    pub masthead_right: String,
}

impl Template {
    /// The default: a plain blank screen — background + content only.
    pub fn plain() -> Template {
        Template {
            name: "plain".into(),
            chrome: Chrome::None,
            palette: Palette::neon(),
            glow: 1.0,
            crt: false,
            masthead_left: String::new(),
            masthead_right: String::new(),
        }
    }

    /// The neon terminal-window look (border, dots, centred title, rule). The
    /// masthead is empty by default — no engine branding; set it with the
    /// `masthead(...)` statement if you want your own header text.
    pub fn terminal() -> Template {
        Template {
            name: "terminal".into(),
            chrome: Chrome::Full,
            palette: Palette::neon(),
            glow: 1.0,
            crt: false,
            masthead_left: String::new(),
            masthead_right: String::new(),
        }
    }

    /// A light print/handout look: ink on cream, no glow, no chrome.
    pub fn paper() -> Template {
        Template {
            name: "paper".into(),
            chrome: Chrome::None,
            palette: Palette::paper(),
            glow: 0.0,
            crt: false,
            masthead_left: String::new(),
            masthead_right: String::new(),
        }
    }

    /// Blueprint: white/cyan on navy, crisp (no glow), plain.
    pub fn blueprint() -> Template {
        Template {
            name: "blueprint".into(),
            chrome: Chrome::None,
            palette: Palette::blueprint(),
            glow: 0.0,
            crt: false,
            masthead_left: String::new(),
            masthead_right: String::new(),
        }
    }

    /// Resolve a template by name (`None` if unknown).
    pub fn by_name(name: &str) -> Option<Template> {
        match name.trim().to_ascii_lowercase().as_str() {
            "plain" | "blank" | "clean" => Some(Self::plain()),
            "terminal" | "neon" | "shell" => Some(Self::terminal()),
            "paper" | "print" | "light" => Some(Self::paper()),
            "blueprint" | "blue" => Some(Self::blueprint()),
            _ => None,
        }
    }
}

impl Default for Template {
    fn default() -> Self {
        Template::plain()
    }
}

/// HSL → RGB (`h` in degrees, `s`/`l` in 0..1), full opacity. The shared basis
/// for the `hue` modifier and the animatable `Prop::Hue` track.
pub fn hsl(h: f32, s: f32, l: f32) -> Color {
    let h = h.rem_euclid(360.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    Color::new(r + m, g + m, b + m, 1.0)
}

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
        let bold =
            load_ttf_font_from_bytes(include_bytes!("../assets/fonts/IBMPlexMono-Bold.ttf")).ok();
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
