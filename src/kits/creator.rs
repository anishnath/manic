//! creator — responsive social-video **format** templates for content creators.
//!
//! A layer orthogonal to the domain kits (math/physics/optics add *subjects*;
//! this adds *shapes of video*). Creator Kit v2 provides viewport-aware layout,
//! platform safe areas, token-driven quiz skins/motion, reusable brand profiles,
//! responsive footers, answer explanations, and end cards while preserving the
//! v1 `creator`/`socials`/`quiz`/`option`/`run` surface.
//!
//! Social icons are DRAWN from primitives (no bundled trademark logos, no
//! downloads) so they render on any template; a creator who wants exact brand
//! logos uses the `image(...)` builtin with their own files.

use crate::easing::Easing;
use crate::lang::ast::ExprKind;
use crate::lang::diag::Error;
use crate::lang::lower::{resolve_color, Args, Registry};
use crate::primitives::{Align, Counter, Entity, FontKind, Shape};
use crate::scene::{
    CreatorFooter, CreatorMotion, CreatorProfile, CreatorRect, CreatorSafe, PlaybackTrack,
    QuizData, QuizDensity, QuizLayout, QuizOpt, QuizReveal, QuizSkin, QuizTimer, Scene,
    SimData,
};
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TrackSpec, Value};
use macroquad::prelude::{Color, Vec2};

#[derive(Debug, Clone, Copy)]
struct CreatorRegions {
    header: CreatorRect,
    media: CreatorRect,
    choices: CreatorRect,
    timer: CreatorRect,
    footer: CreatorRect,
    scale: f32,
}

fn rect_edges(left: f32, top: f32, right: f32, bottom: f32) -> CreatorRect {
    CreatorRect {
        center: Vec2::new((left + right) * 0.5, (top + bottom) * 0.5),
        size: Vec2::new((right - left).max(1.0), (bottom - top).max(1.0)),
    }
}

fn subrect(r: CreatorRect, x0: f32, y0: f32, x1: f32, y1: f32) -> CreatorRect {
    let lo = r.center - r.size * 0.5;
    rect_edges(
        lo.x + r.size.x * x0,
        lo.y + r.size.y * y0,
        lo.x + r.size.x * x1,
        lo.y + r.size.y * y1,
    )
}

fn parse_safe(name: &str) -> Option<CreatorSafe> {
    match name.trim().to_ascii_lowercase().as_str() {
        "shorts" | "short" | "youtube" => Some(CreatorSafe::Shorts),
        "reels" | "reel" | "instagram" => Some(CreatorSafe::Reels),
        "tiktok" | "tt" => Some(CreatorSafe::Tiktok),
        "clean" | "none" | "canvas" => Some(CreatorSafe::Clean),
        _ => None,
    }
}

/// Platform-safe rectangle in logical canvas coordinates. Insets are ratios so
/// the guide scales consistently across portrait, feed, square and landscape.
fn safe_rect(canvas: Vec2, safe: CreatorSafe) -> CreatorRect {
    let (l, r, t, b) = match safe {
        CreatorSafe::Shorts => (0.060, 0.090, 0.055, 0.110),
        CreatorSafe::Reels => (0.065, 0.105, 0.075, 0.135),
        CreatorSafe::Tiktok => (0.065, 0.145, 0.075, 0.155),
        CreatorSafe::Clean => (0.045, 0.045, 0.045, 0.045),
    };
    rect_edges(canvas.x * l, canvas.y * t, canvas.x * (1.0 - r), canvas.y * (1.0 - b))
}

fn creator_regions(canvas: Vec2, safe: CreatorSafe, layout: QuizLayout) -> CreatorRegions {
    let safe_r = safe_rect(canvas, safe);
    let scale = (canvas.x.min(canvas.y) / 1080.0).clamp(0.55, 1.45);
    let tall = canvas.y / canvas.x >= 1.34;
    let (header, media, choices, timer, footer) = if tall {
        let media_end = if layout == QuizLayout::MediaFirst { 0.48 } else { 0.43 };
        (
            subrect(safe_r, 0.0, 0.00, 1.0, 0.17),
            subrect(safe_r, 0.04, 0.19, 0.96, media_end),
            subrect(safe_r, 0.0, media_end + 0.02, 1.0, 0.76),
            subrect(safe_r, 0.12, 0.78, 0.88, 0.88),
            subrect(safe_r, 0.0, 0.91, 1.0, 1.0),
        )
    } else {
        // Square/landscape uses a split editorial composition: question+media
        // on the left, choices+timer on the right.
        (
            subrect(safe_r, 0.00, 0.00, 0.46, 0.34),
            subrect(safe_r, 0.00, 0.38, 0.46, 0.88),
            subrect(safe_r, 0.52, 0.02, 1.00, 0.74),
            subrect(safe_r, 0.56, 0.77, 0.96, 0.88),
            subrect(safe_r, 0.00, 0.92, 1.00, 1.00),
        )
    };
    CreatorRegions { header, media, choices, timer, footer, scale }
}

fn decode_spec_text(s: &str) -> String {
    s.replace('_', " ")
}

/// `creator(id, "spec")` — a reusable social profile (set once, drawn by
/// `socials(id)`). `spec` is space-separated: a display handle (the first bare
/// token, e.g. `@myname`), `platform=user` pairs (`yt=`, `x=`, `ig=`, `tt=`,
/// `gh=`, `web=`), and `accent=colour`. Creates no drawables itself.
pub fn c_creator(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let spec = a.text(1)?;
    let mut prof = CreatorProfile::default();
    for tok in spec.split_whitespace() {
        if let Some((k, v)) = tok.split_once('=') {
            match k.to_ascii_lowercase().as_str() {
                "accent" => prof.accent = resolve_color(v, a.span_of(1)).ok(),
                "secondary" => prof.secondary = resolve_color(v, a.span_of(1)).ok(),
                "name" | "display" => prof.display_name = decode_spec_text(v),
                "tagline" | "tag" => prof.tagline = decode_spec_text(v),
                "logo" | "avatar" => prof.logo = v.to_string(),
                "cta" => prof.cta = decode_spec_text(v),
                "url" | "site" => prof.website = decode_spec_text(v),
                "web" => {
                    prof.website = decode_spec_text(v);
                    prof.platforms.push((k.to_string(), v.to_string()));
                }
                "footer" => {
                    prof.footer = match v.to_ascii_lowercase().as_str() {
                        "social" | "icons" => CreatorFooter::Social,
                        "compact" | "small" => CreatorFooter::Compact,
                        "signature" | "brand" => CreatorFooter::Signature,
                        "none" | "off" | "hidden" => CreatorFooter::None,
                        _ => {
                            return Err(Error::new(
                                "unknown footer style — try social, compact, signature, or none",
                                a.span_of(1),
                            ));
                        }
                    };
                }
                "safe" | "platform" => {
                    prof.safe = parse_safe(v).ok_or_else(|| {
                        Error::new(
                            "unknown safe-area profile — try shorts, reels, tiktok, or clean",
                            a.span_of(1),
                        )
                    })?;
                }
                p => prof.platforms.push((p.to_string(), v.to_string())),
            }
        } else if prof.handle.is_empty() {
            prof.handle = tok.to_string();
        }
    }
    if prof.display_name.is_empty() {
        prof.display_name = prof.handle.trim_start_matches('@').to_string();
    }
    s.creators.insert(id, prof);
    Ok(())
}

fn icon_part(id: &str, k: usize, sfx: &str, shape: Shape, pos: Vec2, col: Color, fill: bool, outline: bool, w: f32) -> Entity {
    let mut e = Entity::new(format!("{id}.icon{k}{sfx}"), shape, pos, col);
    e.stroke.fill = fill;
    e.stroke.outline = outline;
    e.stroke.width = w;
    e.tags = vec![id.to_string(), format!("{id}.footer")];
    e
}

/// Draw one platform icon (a few primitives) centred at `c`. Unknown platforms
/// fall back to a globe/link mark.
fn draw_icon(s: &mut Scene, id: &str, k: usize, plat: &str, c: Vec2, accent: Color) {
    let (fg, void) = (style::FG, style::VOID);
    match plat.to_ascii_lowercase().as_str() {
        "yt" | "youtube" => {
            s.add(icon_part(id, k, "b", Shape::Rect { w: 52.0, h: 36.0 }, c, accent, true, false, 0.0));
            s.add(icon_part(id, k, "p",
                Shape::Polygon { pts: vec![Vec2::new(c.x - 8.0, c.y - 10.0), Vec2::new(c.x - 8.0, c.y + 10.0), Vec2::new(c.x + 12.0, c.y)] },
                Vec2::ZERO, void, true, false, 0.0));
        }
        "x" | "twitter" => {
            s.add(icon_part(id, k, "b", Shape::Rect { w: 44.0, h: 44.0 }, c, fg, false, true, 3.0));
            s.add(icon_part(id, k, "1", Shape::Line { to: Vec2::new(c.x + 11.0, c.y + 11.0) }, Vec2::new(c.x - 11.0, c.y - 11.0), fg, false, false, 3.0));
            s.add(icon_part(id, k, "2", Shape::Line { to: Vec2::new(c.x - 11.0, c.y + 11.0) }, Vec2::new(c.x + 11.0, c.y - 11.0), fg, false, false, 3.0));
        }
        "ig" | "instagram" | "insta" => {
            s.add(icon_part(id, k, "b", Shape::Rect { w: 44.0, h: 44.0 }, c, fg, false, true, 3.0));
            s.add(icon_part(id, k, "c", Shape::Circle { r: 12.0 }, c, fg, false, true, 3.0));
            s.add(icon_part(id, k, "d", Shape::Circle { r: 3.0 }, Vec2::new(c.x + 12.0, c.y - 12.0), fg, true, false, 0.0));
        }
        "tt" | "tiktok" => {
            s.add(icon_part(id, k, "s", Shape::Line { to: Vec2::new(c.x + 8.0, c.y + 12.0) }, Vec2::new(c.x + 8.0, c.y - 18.0), fg, false, false, 4.0));
            s.add(icon_part(id, k, "f", Shape::Line { to: Vec2::new(c.x + 20.0, c.y - 22.0) }, Vec2::new(c.x + 8.0, c.y - 18.0), fg, false, false, 4.0));
            s.add(icon_part(id, k, "h", Shape::Circle { r: 8.0 }, Vec2::new(c.x, c.y + 12.0), accent, true, false, 0.0));
        }
        "gh" | "github" => {
            s.add(icon_part(id, k, "b", Shape::Circle { r: 22.0 }, c, fg, false, true, 3.0));
            s.add(icon_part(id, k, "d", Shape::Circle { r: 5.0 }, c, fg, true, false, 0.0));
        }
        _ => {
            // web / link / site — a little globe
            s.add(icon_part(id, k, "b", Shape::Circle { r: 22.0 }, c, fg, false, true, 3.0));
            s.add(icon_part(id, k, "v", Shape::Line { to: Vec2::new(c.x, c.y + 22.0) }, Vec2::new(c.x, c.y - 22.0), fg, false, false, 2.0));
            s.add(icon_part(id, k, "h", Shape::Line { to: Vec2::new(c.x + 22.0, c.y) }, Vec2::new(c.x - 22.0, c.y), fg, false, false, 2.0));
        }
    }
}

/// `socials(id, [at])` — draw creator `id`'s footer: a rule, a row of platform
/// icons (only the configured ones), and the handle. `at` centres the row
/// (default the 9:16 bottom safe zone `(540, 1815)`). Everything is tagged bare
/// `{id}` + `{id}.footer`, so `show(id)` / `hidden(id.footer)` animate it.
pub fn c_socials(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let prof = s.creators.get(&id).cloned().ok_or_else(|| {
        Error::new(format!("no creator `{id}` — call `creator({id}, \"...\")` first"), a.span_of(0))
    })?;
    if prof.footer == CreatorFooter::None {
        return Ok(());
    }
    let canvas = s.canvas();
    let regions = creator_regions(canvas, prof.safe, QuizLayout::Auto);
    let at = if a.len() >= 2 { a.pair(1)? } else { regions.footer.center };
    let ui = regions.scale;
    let accent = prof.accent.unwrap_or(style::MAGENTA);
    let tags = || vec![id.clone(), format!("{id}.footer")];

    // A quiet rule creates separation without turning the footer into chrome.
    let rule_y = at.y - regions.footer.size.y * 0.42;
    let half_rule = regions.footer.size.x * 0.43;
    let mut rule = Entity::new(
        format!("{id}.rule"),
        Shape::Line { to: Vec2::new(at.x + half_rule, rule_y) },
        Vec2::new(at.x - half_rule, rule_y),
        style::DIM,
    );
    rule.stroke.width = (1.5 * ui).max(1.0);
    rule.opacity = 0.55;
    rule.tags = tags();
    s.add(rule);

    match prof.footer {
        CreatorFooter::Social => {
            // Backwards-compatible [icons · handle] treatment, now responsive.
            let gap = (58.0 * ui).clamp(42.0, 70.0);
            let n = prof.platforms.len() as f32;
            let text_size = (28.0 * ui).clamp(19.0, 34.0);
            let handle_w = prof.handle.chars().count() as f32 * text_size * 0.56;
            let icons_w = if n > 0.0 { n * gap } else { 0.0 };
            let total = icons_w + if n > 0.0 && handle_w > 0.0 { 24.0 * ui } else { 0.0 } + handle_w;
            let mut x = at.x - total / 2.0 + gap * 0.40;
            for (k, (plat, _user)) in prof.platforms.iter().enumerate() {
                draw_icon(s, &id, k, plat, Vec2::new(x, at.y), accent);
                x += gap;
            }
            if !prof.handle.is_empty() {
                let hx = at.x - total / 2.0 + icons_w + if n > 0.0 { 24.0 * ui } else { 0.0 } + handle_w / 2.0;
                let mut h = Entity::new(
                    format!("{id}.handle"),
                    Shape::Text { content: prof.handle.clone(), size: text_size },
                    Vec2::new(hx, at.y),
                    style::DIM,
                );
                h.font = FontKind::MonoBold;
                h.tags = tags();
                s.add(h);
            }
        }
        CreatorFooter::Compact | CreatorFooter::Signature => {
            let signature = prof.footer == CreatorFooter::Signature;
            let logo_size = if signature { 62.0 } else { 46.0 } * ui;
            let name_size = (if signature { 30.0 } else { 25.0 } * ui).clamp(18.0, 38.0);
            let x0 = at.x - regions.footer.size.x * 0.34;
            if !prof.logo.is_empty() {
                let mut logo = Entity::new(
                    format!("{id}.logo"),
                    Shape::Image { path: prof.logo.clone(), w: logo_size, h: logo_size, tint: false },
                    Vec2::new(x0, at.y),
                    style::FG,
                );
                logo.tags = tags();
                s.add(logo);
            } else {
                let mut mark = Entity::new(format!("{id}.logo"), Shape::Circle { r: logo_size * 0.5 }, Vec2::new(x0, at.y), accent);
                mark.stroke.fill = true;
                mark.stroke.outline = false;
                mark.glow = 0.2;
                mark.tags = tags();
                s.add(mark);
                let initial = prof.display_name.chars().next().unwrap_or('•').to_ascii_uppercase().to_string();
                let mut letter = Entity::new(format!("{id}.initial"), Shape::Text { content: initial, size: name_size }, Vec2::new(x0, at.y), style::VOID);
                letter.font = FontKind::MonoBold;
                letter.tags = tags();
                s.add(letter);
            }
            let tx = x0 + logo_size * 0.78;
            let mut name = Entity::new(
                format!("{id}.name"),
                Shape::Text { content: prof.display_name.clone(), size: name_size },
                Vec2::new(tx, at.y - if signature { 10.0 * ui } else { 0.0 }),
                style::FG,
            );
            name.align = Align::Left;
            name.font = FontKind::Display;
            name.tags = tags();
            s.add(name);
            let meta = if signature && !prof.tagline.is_empty() { &prof.tagline } else { &prof.handle };
            if !meta.is_empty() {
                let mut sub = Entity::new(
                    format!("{id}.handle"),
                    Shape::Text { content: meta.clone(), size: (19.0 * ui).clamp(15.0, 25.0) },
                    Vec2::new(tx, at.y + if signature { 24.0 * ui } else { 25.0 * ui }),
                    style::DIM,
                );
                sub.align = Align::Left;
                sub.tags = tags();
                s.add(sub);
            }
        }
        CreatorFooter::None => {}
    }
    Ok(())
}

// ---- quiz Short: question + option cards + auto ask→countdown→reveal beat ----

/// `quiz(id, "question")` — start a quiz Short: lays out the (typewriter)
/// question at the top and a countdown widget. Add answers with `option(...)`,
/// then `run(id, [dur])` plays the whole beat. 9:16 layout.
/// The visual parameters of a quiz [`QuizSkin`]. One struct drives the question
/// header, the answer cards, and the correct-answer reveal — so adding a skin is
/// a single table entry and every skin still works under any global `template()`.
#[derive(Clone, Copy)]
struct SkinSpec {
    // question header
    q_panel: bool,               // filled panel behind the question
    q_panel_glow: f32,
    q_panel_edge: Option<Color>, // panel outline colour (None = no outline)
    q_kicker: Option<&'static str>,
    q_kicker_pill: bool,         // kicker as a filled pill vs plain letters
    q_rule: bool,                // thin accent rule under the question
    q_size: f32,
    // answer cards
    card_fill: bool,
    card_edge: Option<Color>,
    card_edge_w: f32,
    card_glow: f32,
    card_radius: f32,
    badge: bool,                 // filled letter-badge chip
    badge_color: Color,
    // correct-answer reveal
    correct_color: Color,
    correct_glow: f32,
}

fn skin_spec(skin: QuizSkin, accent: Option<Color>) -> SkinSpec {
    use style::{CYAN, DIM, LIME};
    let accent = accent.unwrap_or(CYAN);
    match skin {
        QuizSkin::Studio => SkinSpec {
            q_panel: true, q_panel_glow: 0.10, q_panel_edge: Some(DIM),
            q_kicker: Some("QUICK QUIZ"), q_kicker_pill: false, q_rule: true, q_size: 46.0,
            card_fill: true, card_edge: Some(DIM), card_edge_w: 1.5, card_glow: 0.08,
            card_radius: 26.0, badge: true, badge_color: accent,
            correct_color: LIME, correct_glow: 0.8,
        },
        QuizSkin::Badge => SkinSpec {
            q_panel: true, q_panel_glow: 0.0, q_panel_edge: None,
            q_kicker: Some("QUESTION"), q_kicker_pill: true, q_rule: false, q_size: 46.0,
            card_fill: true, card_edge: Some(DIM), card_edge_w: 2.0, card_glow: 0.0,
            card_radius: 12.0, badge: true, badge_color: accent,
            correct_color: LIME, correct_glow: 2.4,
        },
        QuizSkin::Minimal => SkinSpec {
            q_panel: false, q_panel_glow: 0.0, q_panel_edge: None,
            q_kicker: Some("QUESTION"), q_kicker_pill: false, q_rule: true, q_size: 48.0,
            card_fill: false, card_edge: Some(DIM), card_edge_w: 2.0, card_glow: 0.0,
            card_radius: 18.0, badge: false, badge_color: accent,
            correct_color: LIME, correct_glow: 1.8,
        },
        QuizSkin::Glass => SkinSpec {
            q_panel: true, q_panel_glow: 1.5, q_panel_edge: Some(CYAN),
            q_kicker: None, q_kicker_pill: false, q_rule: false, q_size: 46.0,
            card_fill: true, card_edge: Some(CYAN), card_edge_w: 2.5, card_glow: 1.3,
            card_radius: 30.0, badge: true, badge_color: accent,
            correct_color: LIME, correct_glow: 3.0,
        },
        QuizSkin::Plain => SkinSpec {
            q_panel: false, q_panel_glow: 0.0, q_panel_edge: None,
            q_kicker: None, q_kicker_pill: false, q_rule: false, q_size: 44.0,
            card_fill: true, card_edge: None, card_edge_w: 0.0, card_glow: 0.0,
            card_radius: 8.0, badge: false, badge_color: accent,
            correct_color: LIME, correct_glow: 2.0,
        },
    }
}

/// Map a skin name (with manic-flavoured aliases) to a [`QuizSkin`].
fn parse_skin(name: &str) -> Option<QuizSkin> {
    match name.trim().to_lowercase().as_str() {
        "studio" | "professional" | "editorial" => Some(QuizSkin::Studio),
        "badge" | "card" | "cards" => Some(QuizSkin::Badge),
        "minimal" | "clean" => Some(QuizSkin::Minimal),
        "glass" | "neon" | "reels" => Some(QuizSkin::Glass),
        "plain" | "flat" | "basic" => Some(QuizSkin::Plain),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct QuizConfig {
    reveal: QuizReveal,
    skin: QuizSkin,
    layout: QuizLayout,
    density: QuizDensity,
    timer: QuizTimer,
    motion: CreatorMotion,
    safe: CreatorSafe,
    accent: Option<Color>,
}

/// Parse the optional quiz spec (3rd arg): a space-separated, order-free mix of
/// a reveal word (`type`/`fade`/…) and a skin word (`badge`/`glass`/…).
fn parse_quiz_spec(a: &Args) -> Result<QuizConfig, Error> {
    let mut cfg = QuizConfig::default();
    if let Some(spec) = a.opt_text(2)? {
        for tok in spec.split_whitespace() {
            let (key, value) = tok.split_once('=').unwrap_or(("", tok));
            if key.is_empty() {
                if let Some(r) = parse_reveal(value) {
                    cfg.reveal = r;
                } else if let Some(k) = parse_skin(value) {
                    cfg.skin = k;
                } else {
                    return Err(Error::new(
                        format!("unknown quiz style {tok:?} — skins: studio/badge/minimal/glass/plain · reveals: type/fade/rise/pop/cut"),
                        a.span_of(2),
                    ));
                }
                continue;
            }
            match key.to_ascii_lowercase().as_str() {
                "skin" | "style" => cfg.skin = parse_skin(value).ok_or_else(|| Error::new("unknown quiz skin — try studio, badge, minimal, glass, or plain", a.span_of(2)))?,
                "reveal" => cfg.reveal = parse_reveal(value).ok_or_else(|| Error::new("unknown reveal — try type, fade, rise, pop, or cut", a.span_of(2)))?,
                "layout" => cfg.layout = match value.to_ascii_lowercase().as_str() {
                    "auto" => QuizLayout::Auto,
                    "stack" | "column" => QuizLayout::Stack,
                    "grid" => QuizLayout::Grid,
                    "media-first" | "media" | "visual" => QuizLayout::MediaFirst,
                    _ => return Err(Error::new("unknown layout — try auto, stack, grid, or media-first", a.span_of(2))),
                },
                "density" => cfg.density = match value.to_ascii_lowercase().as_str() {
                    "compact" | "tight" => QuizDensity::Compact,
                    "comfortable" | "normal" => QuizDensity::Comfortable,
                    "spacious" | "airy" => QuizDensity::Spacious,
                    _ => return Err(Error::new("unknown density — try compact, comfortable, or spacious", a.span_of(2))),
                },
                "timer" => cfg.timer = match value.to_ascii_lowercase().as_str() {
                    "ring" => QuizTimer::Ring,
                    "bar" | "progress" => QuizTimer::Bar,
                    "number" | "digit" => QuizTimer::Number,
                    "none" | "off" | "hidden" => QuizTimer::None,
                    _ => return Err(Error::new("unknown timer — try ring, bar, number, or none", a.span_of(2))),
                },
                "motion" => cfg.motion = match value.to_ascii_lowercase().as_str() {
                    "calm" | "soft" => CreatorMotion::Calm,
                    "studio" | "default" => CreatorMotion::Studio,
                    "punch" | "energetic" => CreatorMotion::Punch,
                    "cut" | "none" => CreatorMotion::Cut,
                    _ => return Err(Error::new("unknown motion — try calm, studio, punch, or cut", a.span_of(2))),
                },
                "safe" | "platform" => cfg.safe = parse_safe(value).ok_or_else(|| Error::new("unknown safe area — try shorts, reels, tiktok, or clean", a.span_of(2)))?,
                "accent" => cfg.accent = Some(resolve_color(value, a.span_of(2))?),
                _ => return Err(Error::new(format!("unknown quiz option `{key}` — use skin, reveal, layout, density, timer, motion, safe, or accent"), a.span_of(2))),
            }
        }
    }
    Ok(cfg)
}

/// `quiz(id, "question", ["style"])` — start a quiz Short. `style` is an order-free
/// mix of a card SKIN (`badge` default, `minimal`, `glass`, `plain`) and a question
/// REVEAL (`type` default, `fade`, `rise`, `pop`, `cut`). Add answers with
/// `option(...)`, then `run(id, [dur])` plays the whole beat. 9:16 layout.
pub fn c_quiz(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let q = a.text(1)?;
    let cfg = parse_quiz_spec(a)?;
    let sp = skin_spec(cfg.skin, cfg.accent);
    let regions = creator_regions(s.canvas(), cfg.safe, cfg.layout);
    let tag = |extra: String| vec![id.clone(), format!("{id}.parts"), extra];
    let accent = cfg.accent.unwrap_or(sp.badge_color);
    let header = regions.header;
    let htop = header.center.y - header.size.y * 0.5;

    // ---- question header (panel · kicker · accent rule) ----
    if sp.q_panel {
        let mut p = Entity::new(
            format!("{id}.qpanel"),
            Shape::Rect { w: header.size.x * 0.98, h: header.size.y * 0.94 },
            header.center,
            style::PANEL,
        );
        p.stroke.fill = true;
        p.corner_radius = sp.card_radius * regions.scale;
        match sp.q_panel_edge {
            Some(edge) => {
                p.stroke.outline = true;
                p.stroke.outline_color = Some(edge);
                p.stroke.width = (sp.card_edge_w * regions.scale).max(1.0);
            }
            None => p.stroke.outline = false,
        }
        p.glow = sp.q_panel_glow;
        p.tags = tag(format!("{id}.q"));
        s.add(p);
    }
    if let Some(kick) = sp.q_kicker {
        if sp.q_kicker_pill {
            let pc = Vec2::new(header.center.x - header.size.x * 0.31, htop + header.size.y * 0.25);
            let mut pill = Entity::new(format!("{id}.qkbg"), Shape::Rect { w: 210.0 * regions.scale, h: 48.0 * regions.scale }, pc, accent);
            pill.stroke.fill = true;
            pill.stroke.outline = false;
            pill.corner_radius = 24.0 * regions.scale;
            pill.tags = tag(format!("{id}.q"));
            s.add(pill);
            let mut kt = Entity::new(format!("{id}.qk"), Shape::Text { content: kick.into(), size: (22.0 * regions.scale).clamp(16.0, 28.0) }, pc, style::VOID);
            kt.font = FontKind::MonoBold;
            kt.tags = tag(format!("{id}.q"));
            s.add(kt);
        } else {
            let mut kt = Entity::new(
                format!("{id}.qk"),
                Shape::Text { content: kick.into(), size: (23.0 * regions.scale).clamp(16.0, 30.0) },
                Vec2::new(header.center.x, htop + header.size.y * 0.23),
                accent,
            );
            kt.font = FontKind::MonoBold;
            kt.tags = tag(format!("{id}.q"));
            s.add(kt);
        }
    }
    if sp.q_rule {
        let mut rule = Entity::new(
            format!("{id}.qrule"),
            Shape::Rect { w: header.size.x * 0.52, h: (4.0 * regions.scale).max(2.0) },
            Vec2::new(header.center.x, htop + header.size.y * 0.38),
            accent,
        );
        rule.stroke.fill = true;
        rule.stroke.outline = false;
        rule.corner_radius = 2.0 * regions.scale;
        rule.tags = tag(format!("{id}.q"));
        s.add(rule);
    }
    // question text — heavy display font, wrapped, title-safe. Initial state
    // depends on the reveal: `type` starts undrawn; the others start drawn but
    // hidden (opacity/offset/scale) so the beat can bring them in.
    let rest = Vec2::new(header.center.x, header.center.y + if sp.q_kicker.is_some() || sp.q_rule { header.size.y * 0.13 } else { 0.0 });
    let q_size = (sp.q_size * regions.scale).clamp(25.0, 62.0);
    let mut qe = Entity::new(format!("{id}.q"), Shape::Text { content: q, size: q_size }, rest, style::FG);
    qe.font = FontKind::Display;
    qe.wrap = Some(header.size.x * 0.86);
    qe.tags = vec![id.clone(), format!("{id}.parts")];
    match cfg.reveal {
        QuizReveal::Type => qe.trace = 0.0,
        QuizReveal::Fade => qe.opacity = 0.0,
        QuizReveal::Rise => {
            qe.opacity = 0.0;
            qe.pos = Vec2::new(rest.x, rest.y + 44.0 * regions.scale);
        }
        QuizReveal::Pop => {
            qe.opacity = 0.0;
            qe.scale = 0.7;
        }
        QuizReveal::Cut => {}
    }
    s.add(qe);

    // ---- countdown widget: a faint static TRACK + the draining ring + digit ----
    let tc = regions.timer.center;
    let ring_r = (58.0 * regions.scale).min(regions.timer.size.y * 0.38).max(24.0);
    let arc = || Shape::Arc { r: ring_r, inner: 0.0, start: -90.0, sweep: 360.0 };
    let bar_half = regions.timer.size.x * 0.44;
    let timer_shape = |style: QuizTimer| match style {
        QuizTimer::Bar => Shape::Line { to: Vec2::new(tc.x + bar_half, tc.y) },
        _ => arc(),
    };
    let timer_pos = if cfg.timer == QuizTimer::Bar { Vec2::new(tc.x - bar_half, tc.y) } else { tc };
    let mut track = Entity::new(format!("{id}.track"), timer_shape(cfg.timer), timer_pos, style::DIM);
    track.stroke.fill = false;
    track.stroke.outline = true;
    track.stroke.width = (6.0 * regions.scale).max(3.0);
    track.opacity = if matches!(cfg.timer, QuizTimer::Ring | QuizTimer::Bar) { 0.30 } else { 0.0 };
    track.tags = tag(format!("{id}.ring"));
    s.add(track);
    let mut ring = Entity::new(format!("{id}.ring"), timer_shape(cfg.timer), timer_pos, accent);
    ring.stroke.fill = false;
    ring.stroke.outline = true;
    ring.stroke.width = (8.0 * regions.scale).max(4.0);
    ring.glow = sp.card_glow.max(0.2);
    ring.opacity = 0.0;
    ring.tags = vec![id.clone(), format!("{id}.parts")];
    s.add(ring);
    let counter = Counter { value: 5.0, decimals: 0, prefix: "".into(), suffix: "".into() };
    let mut timer = Entity::new(
        format!("{id}.timer"),
        Shape::Text { content: counter.render(), size: (54.0 * regions.scale).clamp(26.0, 68.0) },
        if cfg.timer == QuizTimer::Bar { tc + Vec2::new(0.0, -30.0 * regions.scale) } else { tc },
        style::FG,
    );
    timer.font = FontKind::MonoBold;
    timer.counter = Some(counter);
    timer.opacity = 0.0;
    timer.tags = vec![id.clone(), format!("{id}.parts")];
    s.add(timer);

    let gap = 30.0 * regions.scale;
    let card_w = if cfg.layout == QuizLayout::Stack {
        regions.choices.size.x * 0.94
    } else {
        (regions.choices.size.x - gap) * 0.5
    };
    let base_h = match cfg.density {
        QuizDensity::Compact => 94.0,
        QuizDensity::Comfortable => 120.0,
        QuizDensity::Spacious => 144.0,
    } * regions.scale;
    let max_h = ((regions.choices.size.y - gap * 2.0) / 3.0).max(54.0 * regions.scale);
    let card_size = Vec2::new(card_w.max(180.0 * regions.scale), base_h.min(max_h));
    s.quizzes.insert(id, QuizData {
        reveal: cfg.reveal,
        skin: cfg.skin,
        layout: cfg.layout,
        density: cfg.density,
        timer_style: cfg.timer,
        motion: cfg.motion,
        safe: cfg.safe,
        accent: cfg.accent,
        header: regions.header,
        media: regions.media,
        choices: regions.choices,
        timer: regions.timer,
        footer: regions.footer,
        card_size,
        question_pos: rest,
        ui_scale: regions.scale,
        ..QuizData::default()
    });
    Ok(())
}

/// Map a text-style name to a `QuizReveal`. Accepts manic-flavoured aliases so
/// creators can name the effect the way the rest of the language does.
fn parse_reveal(name: &str) -> Option<QuizReveal> {
    match name.trim().to_lowercase().as_str() {
        "type" | "typewriter" | "" => Some(QuizReveal::Type),
        "fade" | "fadein" => Some(QuizReveal::Fade),
        "rise" | "slide" | "slideup" => Some(QuizReveal::Rise),
        "pop" | "grow" | "wordpop" => Some(QuizReveal::Pop),
        "cut" | "instant" | "show" | "none" => Some(QuizReveal::Cut),
        _ => None,
    }
}

/// The resting slot for option `i` of `n` — a single centred column for ≤3
/// answers, a 2-column grid for 4+. (Card size is fixed; `run` slides each card
/// into its slot, so the layout adapts to the final option count.)
fn slot(qd: &QuizData, n: usize, i: usize) -> Vec2 {
    let (w, h) = (qd.card_size.x, qd.card_size.y);
    let stack = qd.layout == QuizLayout::Stack || (qd.layout != QuizLayout::Grid && n <= 3);
    if stack {
        let gap = 24.0 * qd.ui_scale;
        let total = n as f32 * h + (n as f32 - 1.0) * gap;
        let y0 = qd.choices.center.y - total / 2.0 + h / 2.0;
        Vec2::new(qd.choices.center.x, y0 + i as f32 * (h + gap))
    } else {
        let (gx, gy) = (30.0 * qd.ui_scale, 24.0 * qd.ui_scale);
        let rows = (n + 1) / 2;
        let (col, row) = (i % 2, i / 2);
        let x = if col == 0 { qd.choices.center.x - (w + gx) / 2.0 } else { qd.choices.center.x + (w + gx) / 2.0 };
        let total = rows as f32 * h + (rows as f32 - 1.0) * gy;
        let y0 = qd.choices.center.y - total / 2.0 + h / 2.0;
        Vec2::new(x, y0 + row as f32 * (h + gy))
    }
}

/// `option(id, "text", [correct])` — add an answer to quiz `id`. `run` lays the
/// answers out automatically (a centred column for ≤3, a 2×2 grid for 4). A
/// trailing `correct` marks the right one (lime highlight + a check on reveal).
/// Entities are created at a neutral spot; `run` slides them into their slot.
pub fn c_option(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let txt = a.text(1)?;
    let correct = a.len() >= 3 && a.ident(2).map(|w| w == "correct").unwrap_or(false);
    let qd = s.quizzes.get(&id).ok_or_else(|| {
        Error::new(format!("no quiz `{id}` — call `quiz({id}, \"...\")` first"), a.span_of(0))
    })?;
    let i = qd.options.len();
    if correct && qd.options.iter().any(|o| o.correct) {
        return Err(Error::new("quiz already has a correct option; mark exactly one answer correct", a.span_of(2)));
    }
    if qd.layout == QuizLayout::Stack && i >= 4 {
        return Err(Error::new("stack layout supports up to four options; use layout=auto or layout=grid for five or six", a.span_of(1)));
    }
    if i >= 6 {
        return Err(Error::new("quiz supports 2–6 options; this would be option 7", a.span_of(1)));
    }
    let sp = skin_spec(qd.skin, qd.accent);
    let card_size = qd.card_size;
    let ui = qd.ui_scale;
    let neutral = qd.choices.center; // `run` moves everything to its final slot
    let tags = || vec![id.clone(), format!("{id}.parts")];
    let letter = (b'A' + i as u8) as char;
    let mut parts: Vec<(String, Vec2)> = Vec::new();

    // card
    let card_id = format!("{id}.c{i}");
    let mut card = Entity::new(card_id.clone(), Shape::Rect { w: card_size.x, h: card_size.y }, neutral, style::PANEL);
    card.stroke.fill = sp.card_fill;
    match sp.card_edge {
        Some(edge) => {
            card.stroke.outline = true;
            card.stroke.outline_color = Some(edge);
            card.stroke.width = (sp.card_edge_w * ui).max(1.0);
        }
        None => card.stroke.outline = false,
    }
    card.glow = sp.card_glow;
    card.corner_radius = sp.card_radius * ui;
    card.opacity = 0.0;
    card.tags = tags();
    s.add(card);
    parts.push((card_id.clone(), Vec2::ZERO));

    // correct-answer highlight (a soft fill tint + a glowing outline), created
    // now but hidden — the beat reveals it. Placed BEHIND the text so the answer
    // stays readable over the tint.
    if correct {
        let mut fillh = Entity::new(format!("{id}.hlfill"), Shape::Rect { w: card_size.x, h: card_size.y }, neutral, sp.correct_color);
        fillh.stroke.fill = true;
        fillh.stroke.outline = false;
        fillh.corner_radius = sp.card_radius * ui;
        fillh.opacity = 0.0;
        fillh.tags = tags();
        s.add(fillh);
        let mut hl = Entity::new(format!("{id}.hl"), Shape::Rect { w: card_size.x + 10.0 * ui, h: card_size.y + 10.0 * ui }, neutral, sp.correct_color);
        hl.stroke.fill = false;
        hl.stroke.outline = true;
        hl.stroke.width = (4.0 * ui).max(2.0);
        hl.glow = sp.correct_glow;
        hl.corner_radius = (sp.card_radius + 5.0) * ui;
        hl.opacity = 0.0;
        hl.tags = tags();
        s.add(hl);
    }

    // letter badge (filled chip) + the letter itself
    let badge_r = (card_size.y * 0.27).min(33.0 * ui).max(17.0 * ui);
    let badge_off = Vec2::new(-card_size.x / 2.0 + badge_r + 20.0 * ui, 0.0);
    if sp.badge {
        let bid = format!("{id}.b{i}");
        let mut b = Entity::new(bid.clone(), Shape::Circle { r: badge_r }, neutral + badge_off, sp.badge_color);
        b.stroke.fill = true;
        b.stroke.outline = false;
        b.glow = sp.card_glow;
        b.opacity = 0.0;
        b.tags = tags();
        s.add(b);
        parts.push((bid, badge_off));
        // the correct badge turns green on reveal: a hidden green disc over the
        // accent badge (the letter, added next, stays on top and readable)
        if correct {
            let mut bw = Entity::new(format!("{id}.bwin"), Shape::Circle { r: badge_r }, neutral + badge_off, sp.correct_color);
            bw.stroke.fill = true;
            bw.stroke.outline = false;
            bw.glow = sp.correct_glow;
            bw.opacity = 0.0;
            bw.tags = tags();
            s.add(bw);
        }
    }
    let lid = format!("{id}.l{i}");
    let letter_color = if sp.badge { style::VOID } else { sp.badge_color };
    let letter_size = (30.0 * ui).clamp(18.0, 38.0);
    let mut le = Entity::new(lid.clone(), Shape::Text { content: letter.to_string(), size: letter_size }, neutral + badge_off, letter_color);
    le.font = FontKind::MonoBold;
    le.opacity = 0.0;
    le.tags = tags();
    s.add(le);
    parts.push((lid, badge_off));

    // answer text — left-aligned, starting just after the badge
    let text_off = Vec2::new(-card_size.x / 2.0 + badge_r * 2.0 + 40.0 * ui, 0.0);
    let text_id = format!("{id}.t{i}");
    let length_factor = if txt.chars().count() > 58 { 0.76 } else if txt.chars().count() > 34 { 0.86 } else { 1.0 };
    let wrap_w = (card_size.x - badge_r * 2.0 - 86.0 * ui).max(88.0 * ui);
    let base_text_size = (30.0 * ui * length_factor).clamp(16.0, 38.0);
    let rough_w = txt.chars().count().max(1) as f32 * base_text_size * 0.61;
    let estimated_lines = (rough_w / wrap_w).ceil().clamp(1.0, 4.0);
    let height_fit = card_size.y * 0.70 / (estimated_lines * 1.22);
    let text_size = base_text_size.min(height_fit).max((15.0 * ui).max(13.0));
    let mut te = Entity::new(text_id.clone(), Shape::Text { content: txt, size: text_size }, neutral + text_off, style::FG);
    te.align = Align::Left;
    te.font = FontKind::MonoBold;
    te.wrap = Some(wrap_w);
    te.opacity = 0.0;
    te.tags = tags();
    s.add(te);
    parts.push((text_id.clone(), text_off));

    // a DRAWN check-mark as ONE polyline (moves cleanly via Pos, draws-on via
    // trace — no glyph dependency), on the card's right edge
    if correct {
        let ck_off = Vec2::new(card_size.x / 2.0 - 34.0 * ui, 0.0);
        let mut ck = Entity::new(
            format!("{id}.check"),
            Shape::Polyline { pts: vec![Vec2::new(-11.0, 0.0) * ui, Vec2::new(-3.0, 11.0) * ui, Vec2::new(15.0, -13.0) * ui] },
            neutral + ck_off,
            sp.correct_color,
        );
        ck.stroke.width = (6.0 * ui).max(3.0);
        ck.glow = sp.correct_glow;
        ck.trace = 0.0;
        ck.tags = tags();
        s.add(ck);
    }

    let qd = s.quizzes.get_mut(&id).unwrap();
    qd.options.push(QuizOpt { card: card_id, text: text_id, correct, parts });
    Ok(())
}

/// Build the quiz beat as a [`Clip`] (called by the shared `run` verb): type the
/// question → stagger the cards → countdown → reveal (highlight correct, fade the
/// rest). Timings are fractions of `dur` (default 12 s).
pub fn build_quiz_clip(s: &Scene, id: &str, dur: Option<f32>) -> Result<Clip, Error> {
    let qd = s.quizzes.get(id).unwrap();
    let sp = skin_spec(qd.skin, qd.accent);
    let dur = dur.unwrap_or(12.0).max(4.0);
    let f = |v: f32| TargetValue::Abs(Value::F(v));
    let pos = |p: Vec2| TargetValue::Abs(Value::V(p));
    let mut t: Vec<TrackSpec> = Vec::new();
    let mut push = |id: String, prop: Prop, target: TargetValue, start: f32, d: f32, e: Easing| {
        t.push(TrackSpec { id, prop, target, start, dur: d, easing: e });
    };
    let n = qd.options.len();
    let (q_dur, option_start, stagger, option_dur, cd, rv, rise, option_ease) = match qd.motion {
        CreatorMotion::Calm => (0.15, 0.22, 0.065, 0.10, 0.52, 0.91, 36.0, Easing::OutCubic),
        CreatorMotion::Studio => (0.12, 0.18, 0.055, 0.075, 0.46, 0.90, 48.0, Easing::OutCubic),
        CreatorMotion::Punch => (0.08, 0.14, 0.040, 0.060, 0.42, 0.83, 66.0, Easing::OutBack),
        CreatorMotion::Cut => (0.01, 0.12, 0.001, 0.010, 0.34, 0.72, 0.0, Easing::Linear),
    };
    // 1) question reveal — style chosen at `quiz(...)` time (default typewriter)
    let qid = format!("{id}.q");
    match qd.reveal {
        QuizReveal::Type => {
            push(qid, Prop::Trace, f(1.0), 0.0, q_dur * dur, Easing::Linear);
        }
        QuizReveal::Fade => {
            push(qid, Prop::Opacity, f(1.0), 0.0, q_dur * dur, Easing::OutCubic);
        }
        QuizReveal::Rise => {
            push(qid.clone(), Prop::Opacity, f(1.0), 0.0, q_dur * dur, Easing::OutCubic);
            push(qid, Prop::Pos, pos(qd.question_pos), 0.0, q_dur * dur, Easing::OutCubic);
        }
        QuizReveal::Pop => {
            push(qid.clone(), Prop::Opacity, f(1.0), 0.0, q_dur * 0.55 * dur, Easing::OutCubic);
            push(qid.clone(), Prop::Scale, f(1.06), 0.0, q_dur * 0.75 * dur, Easing::OutCubic);
            push(qid, Prop::Scale, f(1.0), q_dur * 0.75 * dur, q_dur * 0.5 * dur, Easing::OutBack);
        }
        QuizReveal::Cut => {} // already visible on frame 0
    }
    // 2) options: each card's PARTS (card, badge, letter, text) slide up from
    //    just below their slot + fade in, staggered. Every part keeps its local
    //    offset from the card centre so the card moves as a rigid unit.
    for (i, opt) in qd.options.iter().enumerate() {
        let sl = slot(qd, n, i);
        let st = (option_start + i as f32 * stagger) * dur;
        let d = option_dur * dur;
        for (pid, off) in &opt.parts {
            let rest = sl + *off;
            let below = rest + Vec2::new(0.0, rise * qd.ui_scale);
            push(pid.clone(), Prop::Pos, pos(below), 0.0, 0.02 * dur, Easing::Linear);
            push(pid.clone(), Prop::Pos, pos(rest), st, d, option_ease);
            push(pid.clone(), Prop::Opacity, f(1.0), st, d, option_ease);
        }
        // park the correct-answer highlight + check at this slot (invisible)
        if opt.correct {
            for part in [format!("{id}.hlfill"), format!("{id}.hl")] {
                push(part, Prop::Pos, pos(sl), 0.0, 0.02 * dur, Easing::Linear);
            }
            push(format!("{id}.check"), Prop::Pos, pos(sl + Vec2::new(qd.card_size.x / 2.0 - 34.0 * qd.ui_scale, 0.0)), 0.0, 0.02 * dur, Easing::Linear);
            if sp.badge {
                let badge_r = (qd.card_size.y * 0.27).min(33.0 * qd.ui_scale).max(17.0 * qd.ui_scale);
                let bwin_rest = sl + Vec2::new(-qd.card_size.x / 2.0 + badge_r + 20.0 * qd.ui_scale, 0.0);
                push(format!("{id}.bwin"), Prop::Pos, pos(bwin_rest), 0.0, 0.02 * dur, Easing::Linear);
            }
        }
    }
    // 3) countdown widget in, ring DRAINS (trace 1→0) + digit 5 → 0
    let cd_start = cd * dur;
    let count_start = (cd + 0.02) * dur;
    let count_dur = (rv - cd - 0.04).max(0.08) * dur;
    if matches!(qd.timer_style, QuizTimer::Ring | QuizTimer::Bar) {
        push(format!("{id}.ring"), Prop::Opacity, f(1.0), cd_start, 0.03 * dur, Easing::OutCubic);
        push(format!("{id}.ring"), Prop::Trace, f(0.0), count_start, count_dur, Easing::Linear);
    }
    if qd.timer_style != QuizTimer::None {
        push(format!("{id}.timer"), Prop::Opacity, f(1.0), cd_start, 0.03 * dur, Easing::OutCubic);
        push(format!("{id}.timer"), Prop::Value, f(0.0), count_start, count_dur, Easing::Linear);
        push(format!("{id}.timer"), Prop::Opacity, f(0.0), rv * dur, 0.04 * dur, Easing::OutCubic);
    }
    if matches!(qd.timer_style, QuizTimer::Ring | QuizTimer::Bar) {
        push(format!("{id}.track"), Prop::Opacity, f(0.0), rv * dur, 0.04 * dur, Easing::OutCubic);
    }
    // 4) reveal: correct card lights up (soft fill tint + glowing outline pop +
    //    drawn check); the rest dim back
    let rv = rv * dur;
    for opt in &qd.options {
        if opt.correct {
            push(format!("{id}.hlfill"), Prop::Opacity, f(0.16), rv, 0.05 * dur, Easing::OutCubic);
            push(format!("{id}.hl"), Prop::Opacity, f(1.0), rv, 0.05 * dur, Easing::OutCubic);
            push(format!("{id}.hl"), Prop::Scale, f(1.06), rv, 0.05 * dur, Easing::OutCubic);
            push(format!("{id}.hl"), Prop::Scale, f(1.0), rv + 0.06 * dur, 0.06 * dur, Easing::OutBack);
            push(format!("{id}.check"), Prop::Trace, f(1.0), rv + 0.04 * dur, 0.07 * dur, Easing::OutCubic);
            if sp.badge {
                push(format!("{id}.bwin"), Prop::Opacity, f(1.0), rv, 0.05 * dur, Easing::OutCubic);
            }
        } else {
            for (pid, _) in &opt.parts {
                push(pid.clone(), Prop::Opacity, f(0.26), rv, 0.06 * dur, Easing::OutCubic);
            }
        }
    }
    if !qd.explanation.is_empty() {
        push(qd.explanation.clone(), Prop::Opacity, f(1.0), rv + 0.03 * dur, 0.07 * dur, Easing::OutCubic);
    }
    if !qd.source.is_empty() {
        push(qd.source.clone(), Prop::Opacity, f(1.0), rv + 0.07 * dur, 0.06 * dur, Easing::OutCubic);
    }
    Ok(Clip { tracks: t, events: vec![], dur })
}

// ---- safezone : platform-UI guide for vertical video ----

/// `safezone(id, [inset])` — draw a faint guide rectangle marking the content-safe
/// area of a 9:16 Short (clear of the top clock and the bottom caption/action bar).
/// A composing aid: `hidden(id)` it (or delete the line) for the final render.
pub fn c_safezone(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let canvas = s.canvas();
    let area = match a.exprs.get(1).map(|e| &e.kind) {
        None => safe_rect(canvas, CreatorSafe::Shorts),
        Some(ExprKind::Num(n)) => {
            let inset = n.max(0.0).min(canvas.x.min(canvas.y) * 0.45);
            rect_edges(inset, inset, canvas.x - inset, canvas.y - inset)
        }
        Some(ExprKind::Str(name)) | Some(ExprKind::Ident(name)) => {
            let profile = parse_safe(name).ok_or_else(|| Error::new("unknown safe-area profile — try shorts, reels, tiktok, or clean", a.span_of(1)))?;
            safe_rect(canvas, profile)
        }
        _ => return Err(Error::new("safezone expects a numeric inset or a safe-area name", a.span_of(1))),
    };
    let mut r = Entity::new(id.clone(), Shape::Rect { w: area.size.x, h: area.size.y }, area.center, style::DIM);
    r.stroke.fill = false;
    r.stroke.outline = true;
    r.stroke.width = 2.0;
    r.opacity = 0.35;
    r.tags = vec![id.clone()];
    s.add(r);
    Ok(())
}

// ---- countdown : a standalone draining-ring + digit timer ----

/// `countdown(id, [at], [secs])` — a countdown widget (a draining ring + a digit)
/// centred at `at` (default screen centre), counting from `secs` (default 5).
/// Play it with `run(id, secs)`.
pub fn c_countdown(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let canvas = s.canvas();
    let ui = (canvas.x.min(canvas.y) / 1080.0).clamp(0.55, 1.45);
    let at = if a.len() >= 2 { a.pair(1)? } else { canvas * 0.5 };
    let secs = a.opt_num(2)?.unwrap_or(5.0).max(1.0);
    let mut ring = Entity::new(
        format!("{id}.ring"),
        Shape::Arc { r: 62.0 * ui, inner: 0.0, start: -90.0, sweep: 360.0 },
        at,
        style::CYAN,
    );
    ring.stroke.fill = false;
    ring.stroke.outline = true;
    ring.stroke.width = (6.0 * ui).max(3.0);
    ring.tags = vec![id.clone(), format!("{id}.parts")];
    s.add(ring);
    let counter = Counter { value: secs, decimals: 0, prefix: "".into(), suffix: "".into() };
    let mut timer = Entity::new(format!("{id}.timer"), Shape::Text { content: counter.render(), size: (62.0 * ui).clamp(28.0, 76.0) }, at, style::FG);
    timer.counter = Some(counter);
    timer.tags = vec![id.clone(), format!("{id}.parts")];
    s.add(timer);
    // playback: ring drains (trace 1→0), digit counts secs→0 — replayed by `run`
    let n = 61usize;
    let ring_pts: Vec<Vec2> = (0..n).map(|k| Vec2::new(1.0 - k as f32 / (n - 1) as f32, 0.0)).collect();
    let val_pts: Vec<Vec2> = (0..n).map(|k| Vec2::new(secs * (1.0 - k as f32 / (n - 1) as f32), 0.0)).collect();
    s.sims.insert(
        id.clone(),
        SimData {
            playback: vec![
                PlaybackTrack { id: format!("{id}.ring"), prop: Prop::Trace, points: ring_pts },
                PlaybackTrack { id: format!("{id}.timer"), prop: Prop::Value, points: val_pts },
            ],
            dt: secs / n as f32,
            ..Default::default()
        },
    );
    Ok(())
}

// ---- figure : auto-fit any entity/group into the figure zone ----

fn rotated_about(p: Vec2, center: Vec2, degrees: f32) -> Vec2 {
    if degrees.abs() < 1e-3 {
        return p;
    }
    let a = degrees.to_radians();
    let (sn, cs) = a.sin_cos();
    let d = p - center;
    center + Vec2::new(d.x * cs - d.y * sn, d.x * sn + d.y * cs)
}

/// Union bounding box of a group, including scale, rotation, stroke padding,
/// text/image/equation extents and every path-like primitive used by kits.
fn group_bbox(s: &Scene, ids: &[usize]) -> Option<(Vec2, Vec2)> {
    let (mut lo, mut hi) = (Vec2::new(f32::MAX, f32::MAX), Vec2::new(f32::MIN, f32::MIN));
    let mut any = false;
    for &i in ids {
        let e = &s.entities[i];
        let (mut elo, mut ehi) = (Vec2::new(f32::MAX, f32::MAX), Vec2::new(f32::MIN, f32::MIN));
        let mut acc = |p: Vec2| {
            elo = elo.min(p);
            ehi = ehi.max(p);
        };
        let scale = e.scale.abs().max(0.001);
        let mut point = |p: Vec2| acc(rotated_about(p, e.pos, e.rot));
        match &e.shape {
            Shape::Circle { r } | Shape::Arc { r, .. } => {
                let r = *r * scale;
                point(e.pos + Vec2::new(r, r));
                point(e.pos - Vec2::new(r, r));
            }
            Shape::Rect { w, h } | Shape::Image { w, h, .. } => {
                let (hw, hh) = (*w * scale * 0.5, *h * scale * 0.5);
                for d in [Vec2::new(-hw, -hh), Vec2::new(hw, -hh), Vec2::new(hw, hh), Vec2::new(-hw, hh)] {
                    point(e.pos + d);
                }
            }
            Shape::Line { to } | Shape::Arrow { to } | Shape::Coil { to, .. } => {
                point(e.pos);
                point(*to);
            }
            Shape::Curve { ctrl, to, .. } => {
                point(e.pos);
                point(*ctrl);
                point(*to);
            }
            Shape::Polyline { pts } | Shape::Polygon { pts } => {
                for p in pts { point(e.pos + *p * scale); }
            }
            Shape::Region { tris, rings } => {
                for tri in tris { for p in tri { point(e.pos + *p * scale); } }
                for ring in rings { for p in ring { point(e.pos + *p * scale); } }
            }
            Shape::Text { content, size } => {
                let em = *size * scale;
                let rough = content.chars().count().max(1) as f32 * em * 0.61;
                let width = e.wrap.map(|w| rough.min(w * scale)).unwrap_or(rough).max(em * 0.5);
                let lines = e.wrap.map(|w| (rough / (w * scale).max(1.0)).ceil()).unwrap_or(1.0).max(1.0);
                let height = em * 1.25 * lines;
                let left = if e.align == Align::Left { 0.0 } else { -width * 0.5 };
                for d in [Vec2::new(left, -height * 0.5), Vec2::new(left + width, -height * 0.5), Vec2::new(left + width, height * 0.5), Vec2::new(left, height * 0.5)] {
                    point(e.pos + d);
                }
            }
            Shape::RichText { runs, size } => {
                let em = *size * scale;
                let width = runs.iter().map(|run| match run {
                    crate::primitives::TextRun::Text(t) => t.chars().count() as f32 * em * 0.61,
                    crate::primitives::TextRun::Math { w, .. } => *w * scale,
                }).sum::<f32>().max(em);
                let height = runs.iter().map(|run| match run {
                    crate::primitives::TextRun::Text(_) => em * 1.25,
                    crate::primitives::TextRun::Math { h, .. } => *h * scale,
                }).fold(em, f32::max);
                let left = if e.align == Align::Left { 0.0 } else { -width * 0.5 };
                for d in [Vec2::new(left, -height * 0.5), Vec2::new(left + width, -height * 0.5), Vec2::new(left + width, height * 0.5), Vec2::new(left, height * 0.5)] {
                    point(e.pos + d);
                }
            }
        }
        let pad = (e.stroke.width * scale * 0.5).max(1.0);
        lo = lo.min(elo - Vec2::splat(pad));
        hi = hi.max(ehi + Vec2::splat(pad));
        any = true;
    }
    any.then_some((lo, hi))
}

/// `figure(target, [center], [size])` — scale + translate the group `target` to
/// FIT the figure zone (default centred above the quiz options), so any entity /
/// kit sim can drop in as a Short's illustration without hand-placing it.
pub fn c_figure(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let target = a.ident(0)?;
    let default_zone = creator_regions(s.canvas(), CreatorSafe::Shorts, QuizLayout::Auto).media;
    let zc = if a.len() >= 2 { a.pair(1)? } else { default_zone.center };
    let zs = if a.len() >= 3 { a.pair(2)? } else { default_zone.size };
    // gather the group: entities tagged `target` (or the entity itself)
    let ids: Vec<usize> = (0..s.entities.len())
        .filter(|&i| s.entities[i].id == target || s.entities[i].tags.iter().any(|t| t == &target))
        .collect();
    if ids.is_empty() {
        return Err(Error::new(format!("figure: no entity or group `{target}`"), a.span_of(0)));
    }
    // A derived construction recomputes from its dependencies every frame. If a
    // dependency is left outside the fitted group, the first rendered frame
    // would snap the construction back apart; fail early with a useful message.
    for &i in &ids {
        let e = &s.entities[i];
        if e.derive.is_some() && e.deps.iter().any(|dep| !ids.iter().any(|&j| s.entities[j].id == *dep)) {
            return Err(Error::new(
                format!("figure: `{}` is live but not all of its source points are tagged `{target}`; tag the hidden sources too or place the construction directly", e.id),
                a.span_of(0),
            ));
        }
    }
    let Some((lo, hi)) = group_bbox(s, &ids) else {
        return Ok(());
    };
    let (bw, bh) = ((hi.x - lo.x).max(1.0), (hi.y - lo.y).max(1.0));
    let bc = (lo + hi) * 0.5;
    let sc = (zs.x / bw).min(zs.y / bh).clamp(0.1, 4.0) * 0.92;
    // transform each entity: position, and its shape's own geometry
    for &i in &ids {
        let e = &mut s.entities[i];
        e.pos = zc + (e.pos - bc) * sc;
        e.corner_radius *= sc;
        e.stroke.width *= sc.clamp(0.5, 2.0);
        match &mut e.shape {
            Shape::Circle { r } => *r *= sc,
            Shape::Arc { r, inner, .. } => { *r *= sc; *inner *= sc; }
            Shape::Rect { w, h } => { *w *= sc; *h *= sc; }
            Shape::Line { to } | Shape::Arrow { to } | Shape::Coil { to, .. } => *to = zc + (*to - bc) * sc,
            Shape::Curve { ctrl, to, .. } => {
                *ctrl = zc + (*ctrl - bc) * sc;
                *to = zc + (*to - bc) * sc;
            }
            Shape::Polyline { pts } | Shape::Polygon { pts } => { for p in pts.iter_mut() { *p *= sc; } }
            Shape::Region { tris, rings } => {
                for tri in tris.iter_mut() { for p in tri.iter_mut() { *p *= sc; } }
                for ring in rings.iter_mut() { for p in ring.iter_mut() { *p *= sc; } }
            }
            Shape::Text { size, .. } | Shape::RichText { size, .. } => *size *= sc,
            Shape::Image { w, h, .. } => { *w *= sc; *h *= sc; }
        }
        if let Some(w) = &mut e.wrap { *w *= sc; }
    }
    Ok(())
}

// ---- explanation : optional, author-supplied reveal copy -----------------

/// `explain(quiz, "text", ["source"])` — add concise answer context that is
/// revealed with the correct option. Nothing is generated implicitly: the
/// worked explanation remains author-controlled.
pub fn c_explain(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let content = a.text(1)?;
    let source = a.opt_text(2)?;
    let qd = s.quizzes.get(&id).ok_or_else(|| {
        Error::new(format!("no quiz `{id}` — call `quiz({id}, \"...\")` first"), a.span_of(0))
    })?;
    let at = qd.timer.center;
    let ui = qd.ui_scale;
    let wrap = qd.timer.size.x * 0.92;
    let explanation_id = format!("{id}.explain");
    let mut text = Entity::new(
        explanation_id.clone(),
        Shape::Text { content, size: (27.0 * ui).clamp(18.0, 34.0) },
        at - Vec2::new(0.0, source.as_ref().map(|_| 10.0 * ui).unwrap_or(0.0)),
        style::FG,
    );
    text.font = FontKind::MonoBold;
    text.wrap = Some(wrap);
    text.opacity = 0.0;
    text.tags = vec![id.clone(), format!("{id}.parts"), format!("{id}.explanation")];
    s.add(text);

    let mut source_id = String::new();
    if let Some(source) = source {
        source_id = format!("{id}.source");
        let mut se = Entity::new(
            source_id.clone(),
            Shape::Text { content: source, size: (18.0 * ui).clamp(14.0, 24.0) },
            at + Vec2::new(0.0, 30.0 * ui),
            style::DIM,
        );
        se.wrap = Some(wrap);
        se.opacity = 0.0;
        se.tags = vec![id.clone(), format!("{id}.parts"), format!("{id}.explanation")];
        s.add(se);
    }
    let qd = s.quizzes.get_mut(&id).unwrap();
    qd.explanation = explanation_id;
    qd.source = source_id;
    Ok(())
}

// ---- end card : reusable creator lockup ----------------------------------

fn add_end_part(s: &mut Scene, mut e: Entity, tag: &str) {
    e.opacity = 0.0;
    e.tags = vec![tag.to_string()];
    s.add(e);
}

/// `endcard(profile, ["cta=... safe=..."])` — build a hidden professional
/// creator lockup. Reveal it at the desired final beat with
/// `show(profile.endcard)`.
pub fn c_endcard(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let prof = s.creators.get(&id).cloned().ok_or_else(|| {
        Error::new(format!("no creator `{id}` — call `creator({id}, \"...\")` first"), a.span_of(0))
    })?;
    let mut cta = prof.cta.clone();
    let mut safe = prof.safe;
    let mut title = prof.display_name.clone();
    if let Some(spec) = a.opt_text(1)? {
        for tok in spec.split_whitespace() {
            let (k, v) = tok.split_once('=').ok_or_else(|| Error::new("endcard options use key=value", a.span_of(1)))?;
            match k.to_ascii_lowercase().as_str() {
                "cta" => cta = decode_spec_text(v),
                "title" | "name" => title = decode_spec_text(v),
                "safe" | "platform" => safe = parse_safe(v).ok_or_else(|| Error::new("unknown safe area — try shorts, reels, tiktok, or clean", a.span_of(1)))?,
                _ => return Err(Error::new(format!("unknown endcard option `{k}` — use cta, title, or safe"), a.span_of(1))),
            }
        }
    }
    if cta.is_empty() {
        cta = "FOLLOW FOR MORE".into();
    }
    let canvas = s.canvas();
    let region = safe_rect(canvas, safe);
    let ui = (canvas.x.min(canvas.y) / 1080.0).clamp(0.55, 1.45);
    let accent = prof.accent.unwrap_or(style::CYAN);
    let secondary = prof.secondary.unwrap_or(style::MAGENTA);
    let tag = format!("{id}.endcard");
    let panel_size = Vec2::new(region.size.x * 0.88, region.size.y * if canvas.y / canvas.x > 1.3 { 0.62 } else { 0.78 });
    let center = region.center;

    let mut panel = Entity::new(format!("{id}.end.panel"), Shape::Rect { w: panel_size.x, h: panel_size.y }, center, style::PANEL);
    panel.stroke.fill = true;
    panel.stroke.outline = true;
    panel.stroke.outline_color = Some(style::DIM);
    panel.stroke.width = (1.5 * ui).max(1.0);
    panel.corner_radius = 34.0 * ui;
    panel.glow = 0.08;
    add_end_part(s, panel, &tag);

    let top = center.y - panel_size.y * 0.25;
    let logo_size = (128.0 * ui).min(panel_size.y * 0.24);
    if !prof.logo.is_empty() {
        add_end_part(
            s,
            Entity::new(
                format!("{id}.end.logo"),
                Shape::Image { path: prof.logo.clone(), w: logo_size, h: logo_size, tint: false },
                Vec2::new(center.x, top),
                style::FG,
            ),
            &tag,
        );
    } else {
        let mut mark = Entity::new(format!("{id}.end.logo"), Shape::Circle { r: logo_size * 0.5 }, Vec2::new(center.x, top), accent);
        mark.stroke.fill = true;
        mark.glow = 0.25;
        add_end_part(s, mark, &tag);
        let initial = title.chars().next().unwrap_or('•').to_ascii_uppercase().to_string();
        let mut letter = Entity::new(format!("{id}.end.initial"), Shape::Text { content: initial, size: logo_size * 0.42 }, Vec2::new(center.x, top), style::VOID);
        letter.font = FontKind::Display;
        add_end_part(s, letter, &tag);
    }

    let mut name = Entity::new(
        format!("{id}.end.name"),
        Shape::Text { content: title, size: (46.0 * ui).clamp(27.0, 58.0) },
        Vec2::new(center.x, center.y - panel_size.y * 0.02),
        style::FG,
    );
    name.font = FontKind::Display;
    name.wrap = Some(panel_size.x * 0.78);
    add_end_part(s, name, &tag);

    let meta = if !prof.tagline.is_empty() { prof.tagline.clone() } else if !prof.handle.is_empty() { prof.handle.clone() } else { prof.website.clone() };
    if !meta.is_empty() {
        let mut sub = Entity::new(
            format!("{id}.end.meta"),
            Shape::Text { content: meta, size: (24.0 * ui).clamp(17.0, 31.0) },
            Vec2::new(center.x, center.y + panel_size.y * 0.10),
            style::DIM,
        );
        sub.wrap = Some(panel_size.x * 0.72);
        add_end_part(s, sub, &tag);
    }

    let cta_at = Vec2::new(center.x, center.y + panel_size.y * 0.28);
    let cta_w = (cta.chars().count() as f32 * 18.0 * ui + 110.0 * ui).clamp(panel_size.x * 0.34, panel_size.x * 0.76);
    let mut button = Entity::new(format!("{id}.end.cta_bg"), Shape::Rect { w: cta_w, h: 70.0 * ui }, cta_at, accent);
    button.stroke.fill = true;
    button.stroke.outline = true;
    button.stroke.outline_color = Some(secondary);
    button.stroke.width = (2.0 * ui).max(1.0);
    button.corner_radius = 35.0 * ui;
    button.glow = 0.35;
    add_end_part(s, button, &tag);
    let mut button_text = Entity::new(
        format!("{id}.end.cta"),
        Shape::Text { content: cta, size: (24.0 * ui).clamp(17.0, 30.0) },
        cta_at,
        style::VOID,
    );
    button_text.font = FontKind::MonoBold;
    add_end_part(s, button_text, &tag);
    Ok(())
}

pub fn register(r: &mut Registry) {
    r.ctor("creator", c_creator);
    r.ctor("socials", c_socials);
    r.ctor("quiz", c_quiz);
    r.ctor("option", c_option);
    r.ctor("safezone", c_safezone);
    r.ctor("countdown", c_countdown);
    r.ctor("figure", c_figure);
    r.ctor("explain", c_explain);
    r.ctor("endcard", c_endcard);
}

#[cfg(test)]
mod tests {
    /// `creator(...)` stores a profile (no drawables); `socials(...)` reads it and
    /// draws a footer of icons + handle, tagged for animation.
    #[test]
    fn creator_profile_and_socials_footer() {
        let m = crate::parse(
            "canvas(\"9:16\");\n\
             creator(me, \"@manic yt=@chan x=manic ig=manic accent=gold\");\n\
             socials(me);\n",
        )
        .unwrap();
        let base = m.base();
        // icons for the 3 platforms + the handle + the rule
        assert!(base.contains("me.rule"), "footer rule missing");
        assert!(base.contains("me.icon0b"), "first platform icon missing");
        assert!(base.contains("me.handle"), "handle missing");
        assert!(m.validate().is_ok(), "creator+socials should validate: {:?}", m.validate().err());
    }

    /// `quiz` + `option` lay out the question, cards, countdown + correct
    /// highlight, and `run(q)` (dispatched via the shared verb) emits the beat.
    #[test]
    fn quiz_builds_and_runs() {
        let m = crate::parse(
            "canvas(\"9:16\");\n\
             quiz(q, \"which line?\");\n\
             option(q, \"Euler line\", correct);\n\
             option(q, \"perpendicular bisector\");\n\
             option(q, \"angle bisector\");\n\
             option(q, \"median\");\n\
             run(q, 12);\n",
        )
        .unwrap();
        let base = m.base();
        for sub in ["q.q", "q.ring", "q.timer", "q.c0", "q.t0", "q.hl", "q.c3"] {
            assert!(base.contains(sub), "missing quiz entity `{sub}`");
        }
        // `option` must NOT broadcast over the quiz's parts (consumes the id)
        assert!(m.validate().is_ok(), "quiz+option+run should validate: {:?}", m.validate().err());
    }

    /// The question reveal style is controllable; default is typewriter, and an
    /// unknown style is a clear error listing the valid ones.
    #[test]
    fn quiz_reveal_style_controllable() {
        // default = typewriter → question starts undrawn (trace 0)
        let d = crate::parse("canvas(\"9:16\"); quiz(q, \"hi?\"); option(q, \"a\", correct); run(q, 8);").unwrap();
        assert_eq!(d.base().get("q.q").unwrap().trace, 0.0, "default should be typewriter (trace 0)");

        // `fade` → fully drawn (trace 1) but transparent (opacity 0)
        let f = crate::parse("canvas(\"9:16\"); quiz(q, \"hi?\", \"fade\"); option(q, \"a\", correct); run(q, 8);").unwrap();
        let qe = f.base().get("q.q").unwrap();
        assert_eq!(qe.trace, 1.0, "fade should leave text drawn");
        assert_eq!(qe.opacity, 0.0, "fade should start transparent");

        // aliases resolve; each style validates end-to-end
        for style in ["typewriter", "rise", "pop", "cut", "instant", "slide"] {
            let src = format!("canvas(\"9:16\"); quiz(q, \"hi?\", \"{style}\"); option(q, \"a\", correct); run(q, 8);");
            let m = crate::parse(&src).unwrap();
            assert!(m.validate().is_ok(), "style {style:?} should validate: {:?}", m.validate().err());
        }

        // an unknown style is rejected
        assert!(crate::parse("canvas(\"9:16\"); quiz(q, \"hi?\", \"sparkle\");").is_err(), "unknown style should error");
    }

    /// The card skin is controllable, defaults to Badge, and the spec mixes a
    /// skin word + a reveal word order-free.
    #[test]
    fn quiz_skin_controllable() {
        // default skin = Badge → a framed panel + letter-badge entities appear
        let d = crate::parse("canvas(\"9:16\"); quiz(q, \"hi?\"); option(q, \"a\", correct); run(q, 8);").unwrap();
        let base = d.base();
        assert!(base.contains("q.qpanel"), "badge (default) should draw a question panel");
        assert!(base.contains("q.b0"), "badge (default) should draw a letter-badge");
        assert!(base.contains("q.bwin"), "correct badge should have a green-win overlay");

        // `minimal` → no panel, no badges (outline rows)
        let m = crate::parse("canvas(\"9:16\"); quiz(q, \"hi?\", \"minimal\"); option(q, \"a\", correct); run(q, 8);").unwrap();
        assert!(!m.base().contains("q.qpanel"), "minimal has no panel");
        assert!(!m.base().contains("q.b0"), "minimal has no letter-badge");

        // order-free spec: skin + reveal together, either order
        for spec in ["glass fade", "fade glass", "plain cut"] {
            let src = format!("canvas(\"9:16\"); quiz(q, \"hi?\", \"{spec}\"); option(q, \"a\", correct); run(q, 8);");
            let mv = crate::parse(&src).unwrap();
            assert!(mv.validate().is_ok(), "spec {spec:?} should validate: {:?}", mv.validate().err());
        }

        // an unknown token in the spec is rejected
        assert!(crate::parse("canvas(\"9:16\"); quiz(q, \"hi?\", \"badge wobble\");").is_err(), "unknown spec token should error");
    }

    /// `countdown` (run-driven) · `safezone` · `figure` (auto-fit a group) all
    /// build and validate; `figure` consumes the group id (no broadcast).
    #[test]
    fn countdown_safezone_figure_build() {
        let m = crate::parse(
            "canvas(\"9:16\");\n\
             safezone(sz);\n\
             countdown(cd, (540, 700), 5);\n\
             run(cd, 5);\n\
             circle(a, (200, 900), 40); tag(a, fig);\n\
             circle(b, (800, 1100), 40); tag(b, fig);\n\
             figure(fig);\n",
        )
        .unwrap();
        let base = m.base();
        assert!(base.contains("sz") && base.contains("cd.ring") && base.contains("cd.timer"), "countdown/safezone parts missing");
        // figure fitted the group into the zone (moved the circles toward centre)
        let a = base.get("a").unwrap().pos;
        assert!(a.y < 900.0, "figure should move the group up into the zone (was y=900)");
        assert!(m.validate().is_ok(), "countdown+figure should validate: {:?}", m.validate().err());
    }

    /// One v2 source adapts to portrait, feed, square and landscape canvases;
    /// six option cards finish inside the computed choices region without
    /// overlapping one another.
    #[test]
    fn quiz_v2_is_responsive_across_aspects() {
        for (w, h) in [(1080, 1920), (1080, 1350), (1080, 1080), (1280, 720)] {
            let src = format!(
                "canvas({w}, {h});\n\
                 quiz(q, \"Which result is correct?\", \"studio layout=auto density=compact timer=bar motion=studio safe=clean\");\n\
                 option(q, \"First answer\"); option(q, \"Second answer\");\n\
                 option(q, \"Third answer\", correct); option(q, \"Fourth answer\");\n\
                 option(q, \"Fifth answer\"); option(q, \"Sixth answer\"); run(q, 8);"
            );
            let m = crate::parse(&src).unwrap();
            assert!(m.validate().is_ok(), "{w}x{h} should validate: {:?}", m.validate().err());
            let qd = m.base().quizzes.get("q").unwrap();
            for region in [qd.header, qd.media, qd.choices, qd.timer, qd.footer] {
                let lo = region.center - region.size * 0.5;
                let hi = region.center + region.size * 0.5;
                assert!(lo.x >= -0.1 && lo.y >= -0.1 && hi.x <= w as f32 + 0.1 && hi.y <= h as f32 + 0.1,
                    "region escaped {w}x{h}: {region:?}");
            }
            let (base, timeline) = m.finalize();
            let frame = timeline.apply(&base, 3.2);
            let mut boxes = Vec::new();
            for i in 0..6 {
                let p = frame.get(&format!("q.c{i}")).unwrap().pos;
                let half = qd.card_size * 0.5;
                let lo = p - half;
                let hi = p + half;
                boxes.push((lo, hi));
                let clo = qd.choices.center - qd.choices.size * 0.5;
                let chi = qd.choices.center + qd.choices.size * 0.5;
                assert!(lo.x >= clo.x - 1.0 && hi.x <= chi.x + 1.0 && lo.y >= clo.y - 1.0 && hi.y <= chi.y + 1.0,
                    "card {i} escaped choices at {w}x{h}: {lo:?}..{hi:?} vs {clo:?}..{chi:?}");
            }
            for i in 0..boxes.len() {
                for j in i + 1..boxes.len() {
                    let (a0, a1) = boxes[i];
                    let (b0, b1) = boxes[j];
                    let overlap = a0.x < b1.x && a1.x > b0.x && a0.y < b1.y && a1.y > b0.y;
                    assert!(!overlap, "cards {i}/{j} overlap at {w}x{h}");
                }
            }
        }
    }

    #[test]
    fn quiz_v2_spec_controls_are_parsed() {
        let m = crate::parse(
            "canvas(1080,1920); quiz(q, \"hi?\", \"skin=studio reveal=rise layout=media-first density=spacious timer=number motion=calm safe=reels accent=gold\");\n\
             option(q, \"yes\", correct); option(q, \"no\"); explain(q, \"Because it follows.\", \"Source: author\"); run(q, 8);"
        ).unwrap();
        let q = m.base().quizzes.get("q").unwrap();
        assert_eq!(q.skin, super::QuizSkin::Studio);
        assert_eq!(q.reveal, super::QuizReveal::Rise);
        assert_eq!(q.layout, super::QuizLayout::MediaFirst);
        assert_eq!(q.density, super::QuizDensity::Spacious);
        assert_eq!(q.timer_style, super::QuizTimer::Number);
        assert_eq!(q.motion, super::CreatorMotion::Calm);
        assert_eq!(q.safe, super::CreatorSafe::Reels);
        assert!(m.base().contains("q.explain") && m.base().contains("q.source"));
        assert!(m.validate().is_ok());
    }

    #[test]
    fn creator_v2_profile_footer_and_endcard() {
        let m = crate::parse(
            "canvas(1080,1920); template(\"shorts\");\n\
             creator(me, \"@optics name=Optics_Lab tagline=Physics_made_visible logo=assets/manic-logo.png accent=cyan secondary=magenta footer=signature cta=Watch_the_next_one safe=reels\");\n\
             socials(me); endcard(me, \"cta=Follow_for_more\"); show(me.endcard, 0.5);"
        ).unwrap();
        let p = m.base().creators.get("me").unwrap();
        assert_eq!(p.display_name, "Optics Lab");
        assert_eq!(p.tagline, "Physics made visible");
        assert_eq!(p.footer, super::CreatorFooter::Signature);
        assert_eq!(p.safe, super::CreatorSafe::Reels);
        assert!(m.base().contains("me.logo"));
        assert!(m.base().contains("me.end.panel"));
        assert_eq!(m.base().get("me.end.panel").unwrap().opacity, 0.0);
        assert!(m.validate().is_ok(), "v2 profile/endcard should validate: {:?}", m.validate().err());
    }

    #[test]
    fn quiz_v2_rejects_ambiguous_answers_and_overfull_stack() {
        assert!(crate::parse(
            "quiz(q, \"pick\"); option(q, \"a\", correct); option(q, \"b\", correct);"
        ).is_err(), "two correct options should be a friendly authoring error");
        assert!(crate::parse(
            "quiz(q, \"pick\", \"layout=stack\"); option(q, \"a\"); option(q, \"b\"); option(q, \"c\"); option(q, \"d\"); option(q, \"e\");"
        ).is_err(), "stack should reject a fifth option before cards overlap");
    }

    #[test]
    fn figure_rejects_incomplete_live_group() {
        let err = crate::parse(
            "point(A, (100,100)); point(B, (300,100)); midpoint(m, A, B); tag(m, fig); figure(fig);"
        ).err().expect("missing live dependencies should error");
        assert!(err.msg.contains("source points"), "unexpected diagnostic: {}", err.msg);
    }
}
