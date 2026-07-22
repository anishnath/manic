//! The visual-template system. New movies default to a restrained monochrome
//! editorial palette; the original neon, terminal, paper, blueprint, and
//! creator palettes remain explicit choices.

use macroquad::prelude::Color;
use macroquad::text::{load_ttf_font_from_bytes, Font};
use std::sync::OnceLock;
use ttf_parser::Face;

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
/// Warm red — danger, the net/acceleration vector, "the result". (Distinct from
/// magenta; the classic physics-diagram red.)
pub const RED: Color = Color::new(0.95, 0.27, 0.32, 1.0);
/// Orange — a warm accent between red and gold.
pub const ORANGE: Color = Color::new(1.0, 0.55, 0.16, 1.0);
/// True blue — distinct from the structural cyan (e.g. gravity vectors, water).
pub const BLUE: Color = Color::new(0.36, 0.52, 1.0, 1.0);
/// De-emphasised muted slate-violet for annotations, indices, rules.
pub const DIM: Color = Color::new(0.42, 0.40, 0.56, 1.0);
/// Slightly lifted panel fill (nodes, cells, section cards) over the void.
pub const PANEL: Color = Color::new(0.098, 0.086, 0.169, 1.0);

/// The semantic colour roles a template can retint. The engine bakes the neon
/// values everywhere; the renderer remaps them to the active template's palette
/// at draw time (so `--template` retints content, and bespoke colours — `hue`,
/// explicit RGB — pass through untouched).
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub fg: Color,
    pub cyan: Color,
    pub magenta: Color,
    pub lime: Color,
    pub gold: Color,
    pub red: Color,
    pub orange: Color,
    pub blue: Color,
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
            gold: GOLD,
            red: RED,
            orange: ORANGE,
            blue: BLUE,
            dim: DIM,
            panel: PANEL,
        }
    }

    /// Restrained black-and-white editorial palette. Semantic colours retain
    /// different luminance levels, so meaning survives without hue.
    pub fn mono() -> Palette {
        Palette {
            bg: Color::new(0.022, 0.022, 0.022, 1.0),
            fg: Color::new(0.95, 0.95, 0.95, 1.0),
            cyan: Color::new(0.82, 0.82, 0.82, 1.0),
            magenta: Color::new(0.68, 0.68, 0.68, 1.0),
            lime: Color::new(0.98, 0.98, 0.98, 1.0),
            gold: Color::new(0.88, 0.88, 0.88, 1.0),
            red: Color::new(0.58, 0.58, 0.58, 1.0),
            orange: Color::new(0.76, 0.76, 0.76, 1.0),
            blue: Color::new(0.72, 0.72, 0.72, 1.0),
            dim: Color::new(0.43, 0.43, 0.43, 1.0),
            panel: Color::new(0.075, 0.075, 0.075, 1.0),
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
            gold: GOLD,
            red: RED,
            orange: ORANGE,
            blue: BLUE,
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
            gold: GOLD,
            red: RED,
            orange: ORANGE,
            blue: BLUE,
            dim: Color::new(0.45, 0.58, 0.80, 1.0),
            panel: Color::new(0.10, 0.18, 0.34, 1.0),
        }
    }

    /// Restrained dark editorial palette for creator formats. It preserves the
    /// semantic roles while reducing saturation and glow fatigue on long-form
    /// phone viewing.
    pub fn studio() -> Palette {
        Palette {
            bg: Color::new(0.035, 0.047, 0.075, 1.0),
            fg: Color::new(0.94, 0.95, 0.98, 1.0),
            cyan: Color::new(0.30, 0.72, 1.0, 1.0),
            magenta: Color::new(0.63, 0.48, 1.0, 1.0),
            lime: Color::new(0.35, 0.86, 0.58, 1.0),
            gold: GOLD,
            red: RED,
            orange: ORANGE,
            blue: BLUE,
            dim: Color::new(0.46, 0.52, 0.64, 1.0),
            panel: Color::new(0.075, 0.098, 0.145, 1.0),
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
            (neon.gold, self.gold),
            (neon.red, self.red),
            (neon.orange, self.orange),
            (neon.blue, self.blue),
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
/// `mono`; `plain` retains the original neon palette without chrome.
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
    /// The default: restrained monochrome on near-black, no page chrome.
    pub fn mono() -> Template {
        Template {
            name: "mono".into(),
            chrome: Chrome::None,
            palette: Palette::mono(),
            glow: 0.35,
            crt: false,
            masthead_left: String::new(),
            masthead_right: String::new(),
        }
    }

    /// The original neon palette on a blank screen — background + content only.
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

    /// Professional creator look for Shorts/Reels: restrained editorial colour,
    /// crisp panels and a small halo for emphasis, with no window chrome.
    pub fn shorts() -> Template {
        Template {
            name: "shorts".into(),
            chrome: Chrome::None,
            palette: Palette::studio(),
            glow: 0.65,
            crt: false,
            masthead_left: String::new(),
            masthead_right: String::new(),
        }
    }

    /// Resolve a template by name (`None` if unknown).
    pub fn by_name(name: &str) -> Option<Template> {
        match name.trim().to_ascii_lowercase().as_str() {
            "mono" | "monochrome" | "blackwhite" | "black-white" | "bw" => Some(Self::mono()),
            "plain" | "blank" | "clean" => Some(Self::plain()),
            "terminal" | "neon" | "shell" => Some(Self::terminal()),
            "paper" | "print" | "light" => Some(Self::paper()),
            "blueprint" | "blue" => Some(Self::blueprint()),
            "shorts" | "short" | "punch" => Some(Self::shorts()),
            _ => None,
        }
    }
}

impl Default for Template {
    fn default() -> Self {
        Template::mono()
    }
}

#[cfg(test)]
mod template_tests {
    use super::*;

    #[test]
    fn mono_is_the_default_template() {
        assert_eq!(Template::default().name, "mono");
        assert_eq!(Template::by_name("monochrome").unwrap().name, "mono");
        assert_eq!(Template::by_name("bw").unwrap().name, "mono");
    }

    #[test]
    fn mono_remaps_every_named_semantic_colour_to_greyscale() {
        let mono = Palette::mono();
        for source in [
            VOID, FG, CYAN, MAGENTA, LIME, GOLD, RED, ORANGE, BLUE, DIM, PANEL,
        ] {
            let mapped = mono.remap(source);
            assert!((mapped.r - mapped.g).abs() < 0.0001);
            assert!((mapped.g - mapped.b).abs() < 0.0001);
        }
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

pub(crate) const PLEX_REGULAR_BYTES: &[u8] =
    include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf");
pub(crate) const PLEX_BOLD_BYTES: &[u8] = include_bytes!("../assets/fonts/IBMPlexMono-Bold.ttf");
pub(crate) const NOTO_SANS_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
pub(crate) const NOTO_MATH_BYTES: &[u8] =
    include_bytes!("../assets/fonts/NotoSansMath-Regular.ttf");
pub(crate) const NOTO_SYMBOLS_BYTES: &[u8] =
    include_bytes!("../assets/fonts/NotoSansSymbols-Regular.ttf");
pub(crate) const NOTO_SYMBOLS2_BYTES: &[u8] =
    include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");
pub(crate) const NOTO_ARABIC_BYTES: &[u8] =
    include_bytes!("../assets/fonts/NotoSansArabic-Regular.ttf");
pub(crate) const NOTO_DEVANAGARI_BYTES: &[u8] =
    include_bytes!("../assets/fonts/NotoSansDevanagari-Regular.ttf");

/// Stable font slot selected for one Unicode grapheme cluster.
///
/// This is deliberately internal to rendering: authors still choose the
/// semantic `FontKind`; fallback never leaks into the Manic vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FontSlot {
    Primary,
    Math,
    General,
    Symbols,
    Symbols2,
    Arabic,
    Devanagari,
}

struct Coverage {
    regular: Face<'static>,
    bold: Face<'static>,
    general: Face<'static>,
    math: Face<'static>,
    symbols: Face<'static>,
    symbols2: Face<'static>,
    arabic: Face<'static>,
    devanagari: Face<'static>,
}

impl Coverage {
    fn load() -> Coverage {
        let parse = |bytes: &'static [u8], name: &str| {
            Face::parse(bytes, 0)
                .unwrap_or_else(|err| panic!("embedded font `{name}` is invalid: {err}"))
        };
        Coverage {
            regular: parse(PLEX_REGULAR_BYTES, "IBM Plex Mono Regular"),
            bold: parse(PLEX_BOLD_BYTES, "IBM Plex Mono Bold"),
            general: parse(NOTO_SANS_BYTES, "Noto Sans"),
            math: parse(NOTO_MATH_BYTES, "Noto Sans Math"),
            symbols: parse(NOTO_SYMBOLS_BYTES, "Noto Sans Symbols"),
            symbols2: parse(NOTO_SYMBOLS2_BYTES, "Noto Sans Symbols 2"),
            arabic: parse(NOTO_ARABIC_BYTES, "Noto Sans Arabic"),
            devanagari: parse(NOTO_DEVANAGARI_BYTES, "Noto Sans Devanagari"),
        }
    }

    fn primary(&self, kind: crate::primitives::FontKind) -> &Face<'static> {
        match kind {
            crate::primitives::FontKind::Mono => &self.regular,
            crate::primitives::FontKind::Display | crate::primitives::FontKind::MonoBold => {
                &self.bold
            }
        }
    }

    fn slot_for_grapheme(
        &self,
        kind: crate::primitives::FontKind,
        grapheme: &str,
    ) -> Option<FontSlot> {
        let supports = |face: &Face<'static>| {
            let mut has_visible = false;
            let supported = grapheme.chars().all(|ch| {
                if ch.is_whitespace() {
                    true
                } else if is_shaping_control(ch) {
                    true
                } else {
                    has_visible = true;
                    !ch.is_control() && face.glyph_index(ch).is_some()
                }
            });
            supported && (has_visible || grapheme.chars().all(char::is_whitespace))
        };
        if supports(self.primary(kind)) {
            Some(FontSlot::Primary)
        } else if supports(&self.math) {
            Some(FontSlot::Math)
        } else if supports(&self.general) {
            Some(FontSlot::General)
        } else if supports(&self.symbols) {
            Some(FontSlot::Symbols)
        } else if supports(&self.symbols2) {
            Some(FontSlot::Symbols2)
        } else if supports(&self.arabic) {
            Some(FontSlot::Arabic)
        } else if supports(&self.devanagari) {
            Some(FontSlot::Devanagari)
        } else {
            None
        }
    }
}

/// Format characters consumed by a shaper rather than drawn as independent
/// glyphs. They are accepted only as part of a grapheme whose visible code
/// points are covered by one bundled face.
fn is_shaping_control(ch: char) -> bool {
    matches!(ch, '\u{200C}' | '\u{200D}' | '\u{FE0E}' | '\u{FE0F}')
        || ('\u{FE00}'..='\u{FE0F}').contains(&ch)
        || ('\u{E0100}'..='\u{E01EF}').contains(&ch)
}

/// Loaded deterministic font set. Every face is required: a corrupt or
/// renderer-incompatible bundled font is a package defect, never permission to
/// fall back to a host/default font.
///
/// The neon-terminal look is all monospace: `display` (headlines) and
/// `mono_bold` (emphasised labels) share IBM Plex Mono Bold; `mono` is the
/// regular weight for data and captions. Bold bytes are embedded once and the
/// [`Font`] cloned into both bold slots.
pub struct Fonts {
    pub display: Font,
    pub mono: Font,
    pub mono_bold: Font,
}

impl Fonts {
    /// Load the embedded house fonts (IBM Plex Mono, OFL-licensed and compiled
    /// into the binary, so movies render identically on any machine), plus the
    /// Noto fallback chain used for math, arrows, technical symbols and marks.
    pub fn load() -> Fonts {
        let load = |bytes: &'static [u8], name: &str| {
            load_ttf_font_from_bytes(bytes)
                .unwrap_or_else(|err| panic!("bundled font `{name}` failed to initialize: {err}"))
        };
        let bold = load(PLEX_BOLD_BYTES, "IBM Plex Mono Bold");
        Fonts {
            display: bold.clone(),
            mono: load(PLEX_REGULAR_BYTES, "IBM Plex Mono Regular"),
            mono_bold: bold,
        }
    }
}

fn bundled_coverage() -> &'static Coverage {
    static COVERAGE: OnceLock<Coverage> = OnceLock::new();
    COVERAGE.get_or_init(Coverage::load)
}

pub(crate) fn bundled_grapheme_supports(kind: crate::primitives::FontKind, grapheme: &str) -> bool {
    bundled_coverage()
        .slot_for_grapheme(kind, grapheme)
        .is_some()
}

#[cfg(test)]
mod font_tests {
    use super::*;
    use crate::primitives::FontKind;
    use sha2::{Digest, Sha256};

    #[test]
    fn bundled_fallbacks_cover_the_p0_symbol_corpus() {
        let coverage = Coverage::load();
        let corpus = "→ ← ↔ ⇒ ✓ ✗ ● ○ ◆ ◇ ∞ ≤ ≥";
        for ch in corpus.chars().filter(|ch| !ch.is_whitespace()) {
            assert!(
                coverage
                    .slot_for_grapheme(FontKind::Mono, &ch.to_string())
                    .is_some(),
                "missing bundled glyph U+{:04X} `{ch}`",
                ch as u32
            );
        }
    }

    #[test]
    fn ascii_stays_on_the_requested_primary_face() {
        let coverage = Coverage::load();
        for kind in [FontKind::Display, FontKind::Mono, FontKind::MonoBold] {
            for ch in "Manic 123".chars() {
                assert_eq!(
                    coverage.slot_for_grapheme(kind, &ch.to_string()),
                    Some(FontSlot::Primary)
                );
            }
        }
    }

    #[test]
    fn decomposed_accent_resolves_as_one_supported_cluster() {
        let coverage = Coverage::load();
        assert!(coverage
            .slot_for_grapheme(FontKind::Mono, "e\u{301}")
            .is_some());
    }

    #[test]
    fn bundled_script_faces_cover_arabic_devanagari_and_shaping_controls() {
        let coverage = Coverage::load();
        assert_eq!(
            coverage.slot_for_grapheme(FontKind::Mono, "ع"),
            Some(FontSlot::Arabic)
        );
        assert_eq!(
            coverage.slot_for_grapheme(FontKind::Mono, "ज्ञ"),
            Some(FontSlot::Devanagari)
        );
        assert!(coverage
            .slot_for_grapheme(FontKind::Mono, "क\u{200D}्")
            .is_some());
        assert_eq!(
            coverage.slot_for_grapheme(FontKind::Mono, "\u{200D}"),
            None,
            "a shaping control without a visible bundled base must fail"
        );
    }

    #[test]
    fn invisible_non_whitespace_control_is_not_silently_accepted() {
        let coverage = Coverage::load();
        assert_eq!(coverage.slot_for_grapheme(FontKind::Mono, "\0"), None);
    }

    #[test]
    fn bundled_font_manifest_matches_the_packaged_bytes_and_licence() {
        let manifest = include_str!("../assets/fonts/README.md");
        let licence = include_str!("../LICENSE-FONTS");
        let fonts = [
            (
                "IBMPlexMono-Regular.ttf",
                PLEX_REGULAR_BYTES,
                "6a3412f058c7d8dfd9170c41e85ade48e5156ecb89356110ca57a0a27734af46",
            ),
            (
                "IBMPlexMono-Bold.ttf",
                PLEX_BOLD_BYTES,
                "ac27abd6450a64dd94467580a02fe6235156d5b92f2926ebbc8e7489df64e0be",
            ),
            (
                "NotoSansMath-Regular.ttf",
                NOTO_MATH_BYTES,
                "ff5e5e7638e05bf7bc159d8801a28a40eddf76c155bec4fee53150babd795e1a",
            ),
            (
                "NotoSans-Regular.ttf",
                NOTO_SANS_BYTES,
                "b85c38ecea8a7cfb39c24e395a4007474fa5a4fc864f6ee33309eb4948d232d5",
            ),
            (
                "NotoSansSymbols-Regular.ttf",
                NOTO_SYMBOLS_BYTES,
                "8f02f31959bbdf6061547a188248e13f84dc5fdd940326ec494675f453f072bb",
            ),
            (
                "NotoSansSymbols2-Regular.ttf",
                NOTO_SYMBOLS2_BYTES,
                "630846d528dbe4c4981370a4d0a9475a1fd1491a129bb411f8e157cdb5de13c6",
            ),
            (
                "NotoSansArabic-Regular.ttf",
                NOTO_ARABIC_BYTES,
                "ceea25b464a656dc3b26849bab9356740401af62aedf1bfa8b7f0d9b75925b1b",
            ),
            (
                "NotoSansDevanagari-Regular.ttf",
                NOTO_DEVANAGARI_BYTES,
                "385e78e6359a9d88a0f243d53b1209d7548361ba2194e2b9ec779bcaa7e8949d",
            ),
        ];

        assert!(licence.contains("SIL OPEN FONT LICENSE Version 1.1"));
        for (filename, bytes, expected_sha256) in fonts {
            let actual_sha256 = Sha256::digest(bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            assert_eq!(actual_sha256, expected_sha256, "hash drift for {filename}");
            assert!(
                manifest.contains(filename) && manifest.contains(expected_sha256),
                "font manifest is missing `{filename}` or its immutable hash"
            );
        }
    }
}

/// `c` with its alpha multiplied by `opacity`.
pub fn with_opacity(c: Color, opacity: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * opacity)
}
