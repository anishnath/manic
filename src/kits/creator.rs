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
use crate::lang::diag::{Error, Span};
use crate::lang::lower::{resolve_color, Args, Registry};
use crate::primitives::{Align, Counter, Entity, FontKind, Shape};
use crate::scene::{
    CreatorFooter, CreatorMotion, CreatorPace, CreatorProfile, CreatorRect, CreatorSafe,
    CreatorTimerSpec, PlaybackTrack, QuizData, QuizDensity, QuizLabels, QuizLayout, QuizOpt,
    QuizReveal, QuizSkin, QuizTimer, QuizTiming, Scene, SimData, TimerDirection, TimerFinish,
    TimerFont, TimerNumber, TimerPosition, TimingData, TimingPhase,
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
    rect_edges(
        canvas.x * l,
        canvas.y * t,
        canvas.x * (1.0 - r),
        canvas.y * (1.0 - b),
    )
}

fn creator_regions(canvas: Vec2, safe: CreatorSafe, layout: QuizLayout) -> CreatorRegions {
    let safe_r = safe_rect(canvas, safe);
    let scale = (canvas.x.min(canvas.y) / 1080.0).clamp(0.55, 1.45);
    let tall = canvas.y / canvas.x >= 1.34;
    let (header, media, choices, timer, footer) = if tall {
        let media_end = if layout == QuizLayout::MediaFirst {
            0.48
        } else {
            0.43
        };
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
    CreatorRegions {
        header,
        media,
        choices,
        timer,
        footer,
        scale,
    }
}

fn decode_spec_text(s: &str) -> String {
    s.replace('_', " ")
}

/// `creator(id, "spec")` — a reusable social profile (set once, drawn by
/// `socials(id)`). `spec` is space-separated: a display handle (the first bare
/// token, e.g. `@myname`), `platform=user` pairs (`yt=`, `x=`, `ig=`, `tt=`,
/// `fb=`, `li=`, `gh=`, `web=`, `email=`), and `accent=colour`. Creates no
/// drawables itself; `socials` resolves aliases into normalized native marks.
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
                "url" | "site" | "website" | "web" => {
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

fn canonical_platform(platform: &str) -> &'static str {
    match platform.trim().to_ascii_lowercase().as_str() {
        "yt" | "youtube" => "youtube",
        "x" | "twitter" => "x",
        "ig" | "instagram" | "insta" => "instagram",
        "tt" | "tiktok" => "tiktok",
        "fb" | "facebook" => "facebook",
        "li" | "linkedin" => "linkedin",
        "gh" | "github" => "github",
        "web" | "www" | "site" | "url" | "website" | "link" => "web",
        "mail" | "email" => "email",
        _ => "link",
    }
}

fn social_tags(id: &str, platform: &str, role: &str) -> Vec<String> {
    vec![
        id.to_string(),
        format!("{id}.footer"),
        format!("{id}.socials"),
        format!("{id}.social"),
        format!("{id}.social.{platform}"),
        format!("{id}.social.{role}"),
        format!("{id}.social.{platform}.{role}"),
    ]
}

fn icon_part(
    id: &str,
    k: usize,
    sfx: &str,
    platform: &str,
    shape: Shape,
    pos: Vec2,
    col: Color,
    fill: bool,
    outline: bool,
    w: f32,
) -> Entity {
    let mut e = Entity::new(format!("{id}.icon{k}{sfx}"), shape, pos, col);
    e.stroke.fill = fill;
    e.stroke.outline = outline;
    e.stroke.width = w;
    e.tags = social_tags(id, platform, "icon");
    e
}

/// Draw one platform icon from native scalable primitives in a common 44×44
/// optical box. Unknown platform keys intentionally fall back to a link mark.
fn draw_icon(s: &mut Scene, id: &str, k: usize, plat: &str, c: Vec2, accent: Color, ui: f32) {
    let platform = canonical_platform(plat);
    let (fg, void) = (style::FG, style::VOID);
    let line = (3.0 * ui).max(1.5);
    let p = |dx: f32, dy: f32| Vec2::new(c.x + dx * ui, c.y + dy * ui);
    match platform {
        "youtube" => {
            let mut body = icon_part(
                id,
                k,
                "b",
                platform,
                Shape::Rect {
                    w: 44.0 * ui,
                    h: 31.0 * ui,
                },
                c,
                accent,
                true,
                false,
                0.0,
            );
            body.corner_radius = 8.0 * ui;
            s.add(body);
            s.add(icon_part(
                id,
                k,
                "p",
                platform,
                Shape::Polygon {
                    pts: vec![p(-5.0, -8.0), p(-5.0, 8.0), p(10.0, 0.0)],
                },
                Vec2::ZERO,
                void,
                true,
                false,
                0.0,
            ));
        }
        "x" => {
            s.add(icon_part(
                id,
                k,
                "1",
                platform,
                Shape::Line { to: p(15.0, 19.0) },
                p(-16.0, -19.0),
                fg,
                false,
                false,
                4.0 * ui,
            ));
            s.add(icon_part(
                id,
                k,
                "2",
                platform,
                Shape::Line { to: p(-15.0, 19.0) },
                p(14.0, -19.0),
                fg,
                false,
                false,
                line,
            ));
        }
        "instagram" => {
            let mut body = icon_part(
                id,
                k,
                "b",
                platform,
                Shape::Rect {
                    w: 40.0 * ui,
                    h: 40.0 * ui,
                },
                c,
                fg,
                false,
                true,
                line,
            );
            body.corner_radius = 10.0 * ui;
            s.add(body);
            s.add(icon_part(
                id,
                k,
                "c",
                platform,
                Shape::Circle { r: 10.0 * ui },
                c,
                fg,
                false,
                true,
                line,
            ));
            s.add(icon_part(
                id,
                k,
                "d",
                platform,
                Shape::Circle { r: 3.0 * ui },
                p(12.0, -12.0),
                accent,
                true,
                false,
                0.0,
            ));
        }
        "tiktok" => {
            s.add(icon_part(
                id,
                k,
                "s",
                platform,
                Shape::Line { to: p(6.0, 11.0) },
                p(6.0, -19.0),
                fg,
                false,
                false,
                5.0 * ui,
            ));
            s.add(icon_part(
                id,
                k,
                "f",
                platform,
                Shape::Polyline {
                    pts: vec![p(6.0, -18.0), p(11.0, -12.0), p(19.0, -10.0)],
                },
                Vec2::ZERO,
                fg,
                false,
                false,
                5.0 * ui,
            ));
            s.add(icon_part(
                id,
                k,
                "h",
                platform,
                Shape::Circle { r: 8.0 * ui },
                p(-2.0, 12.0),
                accent,
                false,
                true,
                5.0 * ui,
            ));
        }
        "facebook" => {
            s.add(icon_part(
                id,
                k,
                "b",
                platform,
                Shape::Circle { r: 21.0 * ui },
                c,
                accent,
                true,
                false,
                0.0,
            ));
            let mut mark = icon_part(
                id,
                k,
                "f",
                platform,
                Shape::Text {
                    content: "f".into(),
                    size: 36.0 * ui,
                },
                p(1.0, 3.0),
                void,
                true,
                false,
                0.0,
            );
            mark.font = FontKind::MonoBold;
            s.add(mark);
        }
        "linkedin" => {
            let mut body = icon_part(
                id,
                k,
                "b",
                platform,
                Shape::Rect {
                    w: 40.0 * ui,
                    h: 40.0 * ui,
                },
                c,
                accent,
                true,
                false,
                0.0,
            );
            body.corner_radius = 6.0 * ui;
            s.add(body);
            let mut mark = icon_part(
                id,
                k,
                "i",
                platform,
                Shape::Text {
                    content: "in".into(),
                    size: 24.0 * ui,
                },
                p(1.0, 2.0),
                void,
                true,
                false,
                0.0,
            );
            mark.font = FontKind::MonoBold;
            s.add(mark);
        }
        "github" => {
            s.add(icon_part(
                id,
                k,
                "b",
                platform,
                Shape::Circle { r: 20.0 * ui },
                c,
                fg,
                true,
                false,
                0.0,
            ));
            s.add(icon_part(
                id,
                k,
                "e1",
                platform,
                Shape::Polygon {
                    pts: vec![p(-16.0, -11.0), p(-8.0, -20.0), p(-5.0, -8.0)],
                },
                Vec2::ZERO,
                fg,
                true,
                false,
                0.0,
            ));
            s.add(icon_part(
                id,
                k,
                "e2",
                platform,
                Shape::Polygon {
                    pts: vec![p(16.0, -11.0), p(8.0, -20.0), p(5.0, -8.0)],
                },
                Vec2::ZERO,
                fg,
                true,
                false,
                0.0,
            ));
            s.add(icon_part(
                id,
                k,
                "eye1",
                platform,
                Shape::Circle { r: 2.0 * ui },
                p(-7.0, 0.0),
                void,
                true,
                false,
                0.0,
            ));
            s.add(icon_part(
                id,
                k,
                "eye2",
                platform,
                Shape::Circle { r: 2.0 * ui },
                p(7.0, 0.0),
                void,
                true,
                false,
                0.0,
            ));
        }
        "email" => {
            let mut body = icon_part(
                id,
                k,
                "b",
                platform,
                Shape::Rect {
                    w: 42.0 * ui,
                    h: 31.0 * ui,
                },
                c,
                fg,
                false,
                true,
                line,
            );
            body.corner_radius = 5.0 * ui;
            s.add(body);
            s.add(icon_part(
                id,
                k,
                "l",
                platform,
                Shape::Polyline {
                    pts: vec![p(-19.0, -11.0), p(0.0, 4.0), p(19.0, -11.0)],
                },
                Vec2::ZERO,
                fg,
                false,
                false,
                line,
            ));
        }
        "web" => {
            s.add(icon_part(
                id,
                k,
                "b",
                platform,
                Shape::Circle { r: 21.0 * ui },
                c,
                fg,
                false,
                true,
                line,
            ));
            s.add(icon_part(
                id,
                k,
                "v",
                platform,
                Shape::Line { to: p(0.0, 21.0) },
                p(0.0, -21.0),
                fg,
                false,
                false,
                2.0 * ui,
            ));
            s.add(icon_part(
                id,
                k,
                "h",
                platform,
                Shape::Line { to: p(21.0, 0.0) },
                p(-21.0, 0.0),
                fg,
                false,
                false,
                2.0 * ui,
            ));
            s.add(icon_part(
                id,
                k,
                "a1",
                platform,
                Shape::Arc {
                    r: 13.0 * ui,
                    inner: 0.0,
                    start: 68.0,
                    sweep: 224.0,
                },
                c,
                fg,
                false,
                true,
                1.8 * ui,
            ));
        }
        _ => {
            s.add(icon_part(
                id,
                k,
                "a",
                platform,
                Shape::Arc {
                    r: 10.0 * ui,
                    inner: 0.0,
                    start: -45.0,
                    sweep: 230.0,
                },
                p(-7.0, 7.0),
                fg,
                false,
                true,
                line,
            ));
            s.add(icon_part(
                id,
                k,
                "b",
                platform,
                Shape::Arc {
                    r: 10.0 * ui,
                    inner: 0.0,
                    start: 135.0,
                    sweep: 230.0,
                },
                p(7.0, -7.0),
                accent,
                false,
                true,
                line,
            ));
            s.add(icon_part(
                id,
                k,
                "j",
                platform,
                Shape::Line { to: p(8.0, -8.0) },
                p(-8.0, 8.0),
                fg,
                false,
                false,
                line,
            ));
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
        Error::new(
            format!("no creator `{id}` — call `creator({id}, \"...\")` first"),
            a.span_of(0),
        )
    })?;
    if prof.footer == CreatorFooter::None {
        return Ok(());
    }
    let canvas = s.canvas();
    let regions = creator_regions(canvas, prof.safe, QuizLayout::Auto);
    let at = if a.len() >= 2 {
        a.pair(1)?
    } else {
        regions.footer.center
    };
    let ui = regions.scale;
    let accent = prof.accent.unwrap_or(style::MAGENTA);
    let tags = || vec![id.clone(), format!("{id}.footer")];

    // A quiet rule creates separation without turning the footer into chrome.
    let rule_y = at.y - regions.footer.size.y * 0.42;
    let half_rule = regions.footer.size.x * 0.43;
    let mut rule = Entity::new(
        format!("{id}.rule"),
        Shape::Line {
            to: Vec2::new(at.x + half_rule, rule_y),
        },
        Vec2::new(at.x - half_rule, rule_y),
        style::DIM,
    );
    rule.stroke.width = (1.5 * ui).max(1.0);
    rule.opacity = 0.55;
    rule.tags = vec![
        id.clone(),
        format!("{id}.footer"),
        format!("{id}.socials"),
        format!("{id}.social.rule"),
    ];
    s.add(rule);

    match prof.footer {
        CreatorFooter::Social => {
            // Up to three configured identities are shown as compact
            // icon+value lockups. Wider registries fall back to an icon row so
            // the footer remains useful on square and landscape canvases.
            let text_size = (21.0 * ui).clamp(15.0, 27.0);
            let icon_w = 44.0 * ui;
            let icon_text_gap = 11.0 * ui;
            let item_gap = 28.0 * ui;
            let item_width = |value: &str| {
                icon_w + icon_text_gap + value.chars().count() as f32 * text_size * 0.56
            };
            let labelled_total = prof
                .platforms
                .iter()
                .map(|(_, value)| item_width(value))
                .sum::<f32>()
                + item_gap * prof.platforms.len().saturating_sub(1) as f32;
            let available = regions.footer.size.x * 0.88;
            let show_values = !prof.platforms.is_empty()
                && prof.platforms.len() <= 3
                && labelled_total <= available;

            if show_values {
                let mut x = at.x - labelled_total / 2.0;
                let x_platform = prof
                    .platforms
                    .iter()
                    .position(|(platform, _)| canonical_platform(platform) == "x");
                for (k, (platform_key, value)) in prof.platforms.iter().enumerate() {
                    let platform = canonical_platform(platform_key);
                    let width = item_width(value);
                    let icon_x = x + icon_w / 2.0;
                    draw_icon(s, &id, k, platform_key, Vec2::new(icon_x, at.y), accent, ui);
                    let label_id = if Some(k) == x_platform || (x_platform.is_none() && k == 0) {
                        format!("{id}.handle")
                    } else {
                        format!("{id}.platform{k}")
                    };
                    let mut label = Entity::new(
                        label_id,
                        Shape::Text {
                            content: value.clone(),
                            size: text_size,
                        },
                        Vec2::new(icon_x + icon_w / 2.0 + icon_text_gap, at.y),
                        style::DIM,
                    );
                    label.align = Align::Left;
                    label.font = FontKind::MonoBold;
                    label.tags = social_tags(&id, platform, "label");
                    s.add(label);
                    x += width + item_gap;
                }
            } else {
                let gap = (58.0 * ui).clamp(38.0, 70.0);
                let n = prof.platforms.len() as f32;
                let handle_w = prof.handle.chars().count() as f32 * text_size * 0.56;
                let icons_w = if n > 0.0 { n * gap } else { 0.0 };
                let total = icons_w
                    + if n > 0.0 && handle_w > 0.0 {
                        22.0 * ui
                    } else {
                        0.0
                    }
                    + handle_w;
                let mut x = at.x - total / 2.0 + gap * 0.40;
                for (k, (platform, _)) in prof.platforms.iter().enumerate() {
                    draw_icon(s, &id, k, platform, Vec2::new(x, at.y), accent, ui);
                    x += gap;
                }
                if !prof.handle.is_empty() {
                    let hx = at.x - total / 2.0
                        + icons_w
                        + if n > 0.0 { 22.0 * ui } else { 0.0 }
                        + handle_w / 2.0;
                    let mut handle = Entity::new(
                        format!("{id}.handle"),
                        Shape::Text {
                            content: prof.handle.clone(),
                            size: text_size,
                        },
                        Vec2::new(hx, at.y),
                        style::DIM,
                    );
                    handle.font = FontKind::MonoBold;
                    handle.tags = vec![
                        id.clone(),
                        format!("{id}.footer"),
                        format!("{id}.socials"),
                        format!("{id}.social.handle"),
                    ];
                    s.add(handle);
                }
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
                    Shape::Image {
                        path: prof.logo.clone(),
                        w: logo_size,
                        h: logo_size,
                        tint: false,
                    },
                    Vec2::new(x0, at.y),
                    style::FG,
                );
                logo.tags = tags();
                s.add(logo);
            } else {
                let mut mark = Entity::new(
                    format!("{id}.logo"),
                    Shape::Circle { r: logo_size * 0.5 },
                    Vec2::new(x0, at.y),
                    accent,
                );
                mark.stroke.fill = true;
                mark.stroke.outline = false;
                mark.glow = 0.2;
                mark.tags = tags();
                s.add(mark);
                let initial = prof
                    .display_name
                    .chars()
                    .next()
                    .unwrap_or('•')
                    .to_ascii_uppercase()
                    .to_string();
                let mut letter = Entity::new(
                    format!("{id}.initial"),
                    Shape::Text {
                        content: initial,
                        size: name_size,
                    },
                    Vec2::new(x0, at.y),
                    style::VOID,
                );
                letter.font = FontKind::MonoBold;
                letter.tags = tags();
                s.add(letter);
            }
            let tx = x0 + logo_size * 0.78;
            let mut name = Entity::new(
                format!("{id}.name"),
                Shape::Text {
                    content: prof.display_name.clone(),
                    size: name_size,
                },
                Vec2::new(tx, at.y - if signature { 10.0 * ui } else { 0.0 }),
                style::FG,
            );
            name.align = Align::Left;
            name.font = FontKind::Display;
            name.tags = tags();
            s.add(name);
            let meta = if signature && !prof.tagline.is_empty() {
                &prof.tagline
            } else {
                &prof.handle
            };
            if !meta.is_empty() {
                let mut sub = Entity::new(
                    format!("{id}.handle"),
                    Shape::Text {
                        content: meta.clone(),
                        size: (19.0 * ui).clamp(15.0, 25.0),
                    },
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
    q_panel: bool, // filled panel behind the question
    q_panel_glow: f32,
    q_panel_edge: Option<Color>, // panel outline colour (None = no outline)
    q_kicker: Option<&'static str>,
    q_kicker_pill: bool, // kicker as a filled pill vs plain letters
    q_rule: bool,        // thin accent rule under the question
    q_size: f32,
    // answer cards
    card_fill: bool,
    card_edge: Option<Color>,
    card_edge_w: f32,
    card_glow: f32,
    card_radius: f32,
    badge: bool, // filled letter-badge chip
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
            q_panel: true,
            q_panel_glow: 0.10,
            q_panel_edge: Some(DIM),
            q_kicker: Some("QUICK QUIZ"),
            q_kicker_pill: false,
            q_rule: true,
            q_size: 46.0,
            card_fill: true,
            card_edge: Some(DIM),
            card_edge_w: 1.5,
            card_glow: 0.08,
            card_radius: 26.0,
            badge: true,
            badge_color: accent,
            correct_color: LIME,
            correct_glow: 0.8,
        },
        QuizSkin::Badge => SkinSpec {
            q_panel: true,
            q_panel_glow: 0.0,
            q_panel_edge: None,
            q_kicker: Some("QUESTION"),
            q_kicker_pill: true,
            q_rule: false,
            q_size: 46.0,
            card_fill: true,
            card_edge: Some(DIM),
            card_edge_w: 2.0,
            card_glow: 0.0,
            card_radius: 12.0,
            badge: true,
            badge_color: accent,
            correct_color: LIME,
            correct_glow: 2.4,
        },
        QuizSkin::Minimal => SkinSpec {
            q_panel: false,
            q_panel_glow: 0.0,
            q_panel_edge: None,
            q_kicker: Some("QUESTION"),
            q_kicker_pill: false,
            q_rule: true,
            q_size: 48.0,
            card_fill: false,
            card_edge: Some(DIM),
            card_edge_w: 2.0,
            card_glow: 0.0,
            card_radius: 18.0,
            badge: false,
            badge_color: accent,
            correct_color: LIME,
            correct_glow: 1.8,
        },
        QuizSkin::Glass => SkinSpec {
            q_panel: true,
            q_panel_glow: 1.5,
            q_panel_edge: Some(CYAN),
            q_kicker: None,
            q_kicker_pill: false,
            q_rule: false,
            q_size: 46.0,
            card_fill: true,
            card_edge: Some(CYAN),
            card_edge_w: 2.5,
            card_glow: 1.3,
            card_radius: 30.0,
            badge: true,
            badge_color: accent,
            correct_color: LIME,
            correct_glow: 3.0,
        },
        QuizSkin::Plain => SkinSpec {
            q_panel: false,
            q_panel_glow: 0.0,
            q_panel_edge: None,
            q_kicker: None,
            q_kicker_pill: false,
            q_rule: false,
            q_size: 44.0,
            card_fill: true,
            card_edge: None,
            card_edge_w: 0.0,
            card_glow: 0.0,
            card_radius: 8.0,
            badge: false,
            badge_color: accent,
            correct_color: LIME,
            correct_glow: 2.0,
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

fn parse_timer_look(name: &str) -> Option<QuizTimer> {
    match name.trim().to_ascii_lowercase().as_str() {
        "ring" | "circle" => Some(QuizTimer::Ring),
        "bar" | "progress" | "line" => Some(QuizTimer::Bar),
        "number" | "digit" | "digits" => Some(QuizTimer::Number),
        "segments" | "segment" | "blocks" => Some(QuizTimer::Segments),
        "ticks" | "tick" | "radial" => Some(QuizTimer::Ticks),
        "pulse" | "beat" => Some(QuizTimer::Pulse),
        "none" | "off" | "hidden" => Some(QuizTimer::None),
        _ => None,
    }
}

fn parse_pace(name: &str) -> Option<CreatorPace> {
    match name.trim().to_ascii_lowercase().as_str() {
        "quick" | "fast" => Some(CreatorPace::Quick),
        "balanced" | "default" | "normal" => Some(CreatorPace::Balanced),
        "calm" | "slow" => Some(CreatorPace::Calm),
        "dramatic" | "drama" => Some(CreatorPace::Dramatic),
        _ => None,
    }
}

fn spec_num(value: &str, key: &str, span: Span) -> Result<f32, Error> {
    value
        .parse::<f32>()
        .map_err(|_| Error::new(format!("`{key}` expects a number, got {value:?}"), span))
}

fn parse_timer_spec_text(
    mut out: CreatorTimerSpec,
    spec: &str,
    span: Span,
) -> Result<CreatorTimerSpec, Error> {
    for tok in spec.split_whitespace() {
        let (key, value) = tok.split_once('=').unwrap_or(("", tok));
        if key.is_empty() {
            out.look = parse_timer_look(value).ok_or_else(|| {
                Error::new(
                    "unknown timer look — try ring, bar, number, segments, ticks, pulse, or none",
                    span,
                )
            })?;
            continue;
        }
        match key.to_ascii_lowercase().as_str() {
            "look" | "timer" | "style" => {
                out.look = parse_timer_look(value).ok_or_else(|| {
                    Error::new("unknown timer look — try ring, bar, number, segments, ticks, pulse, or none", span)
                })?;
            }
            "position" | "pos" => {
                out.position = match value.to_ascii_lowercase().as_str() {
                    "auto" => TimerPosition::Auto,
                    "header" | "top" => TimerPosition::Header,
                    "media" | "figure" => TimerPosition::Media,
                    "below" | "bottom" | "choices" => TimerPosition::Below,
                    _ => {
                        return Err(Error::new(
                            "unknown timer position — try auto, header, media, or below",
                            span,
                        ))
                    }
                };
            }
            "number" | "digit" => {
                out.number = match value.to_ascii_lowercase().as_str() {
                    "inside" | "in" | "on" => TimerNumber::Inside,
                    "outside" | "out" => TimerNumber::Outside,
                    "none" | "off" | "hidden" => TimerNumber::None,
                    _ => {
                        return Err(Error::new(
                            "unknown number placement — try inside, outside, or none",
                            span,
                        ))
                    }
                };
            }
            "direction" | "dir" => {
                out.direction = match value.to_ascii_lowercase().as_str() {
                    "drain" | "down" | "countdown" => TimerDirection::Drain,
                    "fill" | "up" | "countup" => TimerDirection::Fill,
                    _ => {
                        return Err(Error::new(
                            "unknown timer direction — try drain or fill",
                            span,
                        ))
                    }
                };
            }
            "finish" | "end" => {
                out.finish = match value.to_ascii_lowercase().as_str() {
                    "fade" | "hide" => TimerFinish::Fade,
                    "hold" | "stay" => TimerFinish::Hold,
                    "flash" => TimerFinish::Flash,
                    "pulse" | "pop" => TimerFinish::Pulse,
                    _ => {
                        return Err(Error::new(
                            "unknown timer finish — try fade, hold, flash, or pulse",
                            span,
                        ))
                    }
                };
            }
            "font" | "digits" => {
                out.font = match value.to_ascii_lowercase().as_str() {
                    "mono" | "monospace" => TimerFont::Mono,
                    "display" | "bold" => TimerFont::Display,
                    _ => return Err(Error::new("unknown timer font — try mono or display", span)),
                };
            }
            "size" => {
                out.size = match value.to_ascii_lowercase().as_str() {
                    "small" | "sm" => 0.78,
                    "medium" | "md" | "normal" => 1.0,
                    "large" | "lg" => 1.28,
                    _ => spec_num(value, key, span)?.clamp(0.5, 2.0),
                };
            }
            "thickness" | "stroke" => out.thickness = spec_num(value, key, span)?.clamp(0.4, 3.0),
            "color" | "colour" | "accent" => out.color = Some(resolve_color(value, span)?),
            "track" | "trackcolor" | "trackcolour" => out.track = Some(resolve_color(value, span)?),
            "label" => out.label = decode_spec_text(value),
            _ => {
                return Err(Error::new(
                    format!("unknown timer option `{key}` — use look, position, number, direction, size, thickness, color, track, label, font, or finish"),
                    span,
                ));
            }
        }
    }
    if out.look == QuizTimer::Number && out.number == TimerNumber::None {
        return Err(Error::new(
            "timer look `number` cannot also use number=none",
            span,
        ));
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, Default)]
struct QuizConfig {
    reveal: QuizReveal,
    skin: QuizSkin,
    layout: QuizLayout,
    density: QuizDensity,
    labels: QuizLabels,
    timer: QuizTimer,
    pace: CreatorPace,
    seconds: Option<f32>,
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
                "labels" | "label" | "indices" => cfg.labels = match value.to_ascii_lowercase().as_str() {
                    "letters" | "letter" | "alpha" | "abcd" => QuizLabels::Letters,
                    "numbers" | "number" | "numeric" | "1234" => QuizLabels::Numbers,
                    "none" | "off" | "hidden" => QuizLabels::None,
                    _ => return Err(Error::new("unknown option labels — try letters, numbers, or none", a.span_of(2))),
                },
                "timer" => cfg.timer = parse_timer_look(value).ok_or_else(|| {
                    Error::new("unknown timer — try ring, bar, number, segments, ticks, pulse, or none", a.span_of(2))
                })?,
                "pace" | "timing" => cfg.pace = parse_pace(value).ok_or_else(|| {
                    Error::new("unknown pace — try quick, balanced, calm, or dramatic", a.span_of(2))
                })?,
                "seconds" | "think" => {
                    let seconds = spec_num(value, key, a.span_of(2))?;
                    if seconds <= 0.0 {
                        return Err(Error::new("timer seconds must be greater than zero", a.span_of(2)));
                    }
                    cfg.seconds = Some(seconds);
                }
                "motion" => cfg.motion = match value.to_ascii_lowercase().as_str() {
                    "calm" | "soft" => CreatorMotion::Calm,
                    "studio" | "default" => CreatorMotion::Studio,
                    "punch" | "energetic" => CreatorMotion::Punch,
                    "cut" | "none" => CreatorMotion::Cut,
                    _ => return Err(Error::new("unknown motion — try calm, studio, punch, or cut", a.span_of(2))),
                },
                "safe" | "platform" => cfg.safe = parse_safe(value).ok_or_else(|| Error::new("unknown safe area — try shorts, reels, tiktok, or clean", a.span_of(2)))?,
                "accent" => cfg.accent = Some(resolve_color(value, a.span_of(2))?),
                _ => return Err(Error::new(format!("unknown quiz option `{key}` — use skin, reveal, layout, density, labels, timer, pace, seconds, motion, safe, or accent"), a.span_of(2))),
            }
        }
    }
    Ok(cfg)
}

const TIMER_SEGMENT_COUNT: usize = 10;
const TIMER_TICK_COUNT: usize = 12;

fn timer_tags(owner: &str, role: &str) -> Vec<String> {
    vec![
        owner.to_string(),
        format!("{owner}.parts"),
        format!("{owner}.timer"),
        format!("{owner}.timer.{role}"),
    ]
}

fn timer_anchor(
    timer: CreatorRect,
    header: CreatorRect,
    media: CreatorRect,
    position: TimerPosition,
) -> Vec2 {
    match position {
        TimerPosition::Auto | TimerPosition::Below => timer.center,
        TimerPosition::Header => {
            header.center + Vec2::new(header.size.x * 0.38, -header.size.y * 0.31)
        }
        TimerPosition::Media => media.center + Vec2::new(media.size.x * 0.39, -media.size.y * 0.34),
    }
}

fn upsert_timer_entity(s: &mut Scene, e: Entity) {
    if let Some(existing) = s.get_mut(&e.id) {
        *existing = e;
    } else {
        s.add(e);
    }
}

fn timer_part(
    owner: &str,
    id: String,
    role: &str,
    shape: Shape,
    pos: Vec2,
    color: Color,
    opacity: f32,
) -> Entity {
    let mut e = Entity::new(id, shape, pos, color);
    e.tags = timer_tags(owner, role);
    e.opacity = opacity;
    e
}

fn activate_timer_part(e: &mut Entity, owner: &str, role: &str) {
    e.tags.push(format!("{owner}.timer.active.{role}"));
}

/// Build or restyle a timer widget from native manic primitives. Quiz timers
/// start hidden and are revealed by `run`; standalone countdowns start visible.
fn configure_timer_widget(
    s: &mut Scene,
    owner: &str,
    timer: CreatorRect,
    header: CreatorRect,
    media: CreatorRect,
    ui: f32,
    seconds: f32,
    spec: &CreatorTimerSpec,
    fallback_accent: Color,
    initially_hidden: bool,
) {
    let group = format!("{owner}.timer");
    let active_prefix = format!("{owner}.timer.active.");
    for e in &mut s.entities {
        if e.tags.iter().any(|t| t == &group) {
            e.opacity = 0.0;
            e.tags.retain(|t| !t.starts_with(&active_prefix));
        }
    }

    let accent = spec.color.unwrap_or(fallback_accent);
    let track_color = spec.track.unwrap_or(style::DIM);
    let anchor = timer_anchor(timer, header, media, spec.position);
    let scale = ui * spec.size;
    let radius = (54.0 * scale).clamp(22.0, timer.size.y * 0.40 * spec.size.max(0.7));
    let thick = (7.0 * ui * spec.thickness).clamp(2.0, 18.0);
    let bar_half = (timer.size.x * 0.42 * spec.size)
        .min(330.0 * scale)
        .max(80.0 * scale);
    let segment_half = (timer.size.x * 0.72 * spec.size)
        .min(500.0 * scale)
        .max(170.0 * scale)
        * 0.5;
    let visible = |v: f32| {
        if initially_hidden || spec.look == QuizTimer::None {
            0.0
        } else {
            v
        }
    };
    let progress_visible = visible(if spec.direction == TimerDirection::Drain {
        1.0
    } else {
        0.0
    });

    let (main_shape, main_pos) = match spec.look {
        QuizTimer::Bar => (
            Shape::Line {
                to: anchor + Vec2::new(bar_half, 0.0),
            },
            anchor - Vec2::new(bar_half, 0.0),
        ),
        _ => (
            Shape::Arc {
                r: radius,
                inner: 0.0,
                start: -90.0,
                sweep: 360.0,
            },
            anchor,
        ),
    };
    let main_roles_visible = matches!(spec.look, QuizTimer::Ring | QuizTimer::Bar);

    let mut track = timer_part(
        owner,
        format!("{owner}.timer.track.main"),
        "track",
        if spec.look == QuizTimer::Pulse {
            Shape::Circle { r: radius * 0.92 }
        } else {
            main_shape.clone()
        },
        main_pos,
        if spec.look == QuizTimer::Pulse {
            style::PANEL
        } else {
            track_color
        },
        visible(if main_roles_visible || spec.look == QuizTimer::Pulse {
            0.30
        } else {
            0.0
        }),
    );
    track.stroke.width = thick * 0.78;
    track.stroke.fill = spec.look == QuizTimer::Pulse;
    track.stroke.outline = true;
    if spec.look == QuizTimer::Pulse {
        track.stroke.outline_color = Some(accent);
        track.corner_radius = radius;
    }
    if main_roles_visible || spec.look == QuizTimer::Pulse {
        activate_timer_part(&mut track, owner, "track");
    }
    upsert_timer_entity(s, track);

    let mut progress = timer_part(
        owner,
        format!("{owner}.timer.progress.main"),
        "progress",
        main_shape,
        main_pos,
        accent,
        if main_roles_visible {
            progress_visible
        } else {
            0.0
        },
    );
    progress.stroke.width = thick;
    progress.stroke.fill = false;
    progress.stroke.outline = true;
    progress.glow = 0.24;
    progress.trace = if spec.direction == TimerDirection::Fill {
        0.0
    } else {
        1.0
    };
    if main_roles_visible {
        activate_timer_part(&mut progress, owner, "progress");
    }
    upsert_timer_entity(s, progress);

    let number_wanted = spec.number != TimerNumber::None && spec.look != QuizTimer::None;
    let value_pos = match (spec.look, spec.number) {
        (QuizTimer::Bar, TimerNumber::Outside) => anchor + Vec2::new(bar_half + 42.0 * scale, 0.0),
        (QuizTimer::Segments, TimerNumber::Outside) => {
            anchor + Vec2::new(segment_half + 42.0 * scale, 0.0)
        }
        (QuizTimer::Bar | QuizTimer::Segments, _) => anchor - Vec2::new(0.0, 34.0 * scale),
        (_, TimerNumber::Outside) => anchor + Vec2::new(radius + 42.0 * scale, 0.0),
        _ => anchor,
    };
    let initial_value = if spec.direction == TimerDirection::Drain {
        seconds
    } else {
        0.0
    };
    let counter = Counter {
        value: initial_value,
        decimals: 0,
        prefix: "".into(),
        suffix: "".into(),
    };
    let value_color = if matches!(spec.look, QuizTimer::Number | QuizTimer::Pulse) {
        accent
    } else {
        style::FG
    };
    let mut value = timer_part(
        owner,
        format!("{owner}.timer.value.main"),
        "value",
        Shape::Text {
            content: counter.render(),
            size: (50.0 * scale).clamp(24.0, 82.0),
        },
        value_pos,
        value_color,
        visible(if number_wanted { 1.0 } else { 0.0 }),
    );
    value.font = if spec.font == TimerFont::Display {
        FontKind::Display
    } else {
        FontKind::MonoBold
    };
    value.counter = Some(counter);
    if number_wanted {
        activate_timer_part(&mut value, owner, "value");
    }
    upsert_timer_entity(s, value);

    let mut label = timer_part(
        owner,
        format!("{owner}.timer.label.main"),
        "label",
        Shape::Text {
            content: spec.label.clone(),
            size: (17.0 * scale).clamp(13.0, 28.0),
        },
        anchor
            + Vec2::new(
                0.0,
                if matches!(spec.look, QuizTimer::Bar | QuizTimer::Segments) {
                    thick * 0.8 + 24.0 * scale
                } else {
                    radius + 24.0 * scale
                },
            ),
        track_color,
        visible(if spec.label.is_empty() { 0.0 } else { 1.0 }),
    );
    label.font = FontKind::MonoBold;
    if !spec.label.is_empty() && spec.look != QuizTimer::None {
        activate_timer_part(&mut label, owner, "label");
    }
    upsert_timer_entity(s, label);

    let mut effect = timer_part(
        owner,
        format!("{owner}.timer.effect.main"),
        "effects",
        Shape::Circle { r: radius * 1.16 },
        anchor,
        accent,
        0.0,
    );
    effect.stroke.fill = false;
    effect.stroke.outline = true;
    effect.stroke.width = thick * 0.72;
    effect.glow = 0.75;
    if spec.look != QuizTimer::None {
        activate_timer_part(&mut effect, owner, "effects");
    }
    upsert_timer_entity(s, effect);

    if spec.look == QuizTimer::Segments {
        let total = segment_half * 2.0;
        let gap = 7.0 * ui;
        let seg_w = (total - gap * (TIMER_SEGMENT_COUNT - 1) as f32) / TIMER_SEGMENT_COUNT as f32;
        let x0 = anchor.x - total * 0.5 + seg_w * 0.5;
        for i in 0..TIMER_SEGMENT_COUNT {
            let p = Vec2::new(x0 + i as f32 * (seg_w + gap), anchor.y);
            let mut tr = timer_part(
                owner,
                format!("{owner}.timer.track.seg{i}"),
                "track",
                Shape::Rect {
                    w: seg_w,
                    h: thick * 1.35,
                },
                p,
                track_color,
                visible(0.28),
            );
            tr.stroke.fill = true;
            tr.stroke.outline = false;
            tr.corner_radius = thick * 0.68;
            activate_timer_part(&mut tr, owner, "track");
            upsert_timer_entity(s, tr);
            let mut pr = timer_part(
                owner,
                format!("{owner}.timer.progress.seg{i}"),
                "progress",
                Shape::Rect {
                    w: seg_w,
                    h: thick * 1.35,
                },
                p,
                accent,
                progress_visible,
            );
            pr.stroke.fill = true;
            pr.stroke.outline = false;
            pr.corner_radius = thick * 0.68;
            pr.glow = 0.18;
            activate_timer_part(&mut pr, owner, "progress");
            upsert_timer_entity(s, pr);
        }
    }

    if spec.look == QuizTimer::Ticks {
        let inner = radius * 0.78;
        let outer = radius * 1.04;
        for i in 0..TIMER_TICK_COUNT {
            let a = -std::f32::consts::FRAC_PI_2
                + std::f32::consts::TAU * i as f32 / TIMER_TICK_COUNT as f32;
            let dir = Vec2::new(a.cos(), a.sin());
            let from = anchor + dir * inner;
            let to = anchor + dir * outer;
            let mut tr = timer_part(
                owner,
                format!("{owner}.timer.track.tick{i}"),
                "track",
                Shape::Line { to },
                from,
                track_color,
                visible(0.28),
            );
            tr.stroke.width = thick * 0.70;
            activate_timer_part(&mut tr, owner, "track");
            upsert_timer_entity(s, tr);
            let mut pr = timer_part(
                owner,
                format!("{owner}.timer.progress.tick{i}"),
                "progress",
                Shape::Line { to },
                from,
                accent,
                progress_visible,
            );
            pr.stroke.width = thick;
            pr.glow = 0.18;
            activate_timer_part(&mut pr, owner, "progress");
            upsert_timer_entity(s, pr);
        }
    }
}

fn set_timer_counter(s: &mut Scene, owner: &str, seconds: f32, direction: TimerDirection) {
    if let Some(e) = s.get_mut(&format!("{owner}.timer.value.main")) {
        let value = if direction == TimerDirection::Drain {
            seconds
        } else {
            0.0
        };
        if let Some(c) = &mut e.counter {
            c.value = value;
            if let Shape::Text { content, .. } = &mut e.shape {
                *content = c.render();
            }
        }
    }
}

/// `quiz(id, "question", ["style"])` — start a responsive quiz format. `style`
/// is an order-free mix of a card skin (`studio` default), question reveal,
/// layout/density, `labels=letters|numbers|none`, timer, motion and safe-area
/// controls. Add answers with `option(...)`, then `run(id, [dur])` plays the
/// complete ask → choices → countdown → reveal beat.
pub fn c_quiz(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let q = a.text(1)?;
    let cfg = parse_quiz_spec(a)?;
    let sp = skin_spec(cfg.skin, cfg.accent);
    let regions = creator_regions(s.canvas(), cfg.safe, cfg.layout);
    let question_tags = |role: &str| {
        let mut tags = vec![
            id.clone(),
            format!("{id}.parts"),
            format!("{id}.question"),
            format!("{id}.question.{role}"),
        ];
        // Preserve the v1/v2 question group used by some authored modifiers.
        tags.push(format!("{id}.q"));
        tags
    };
    let accent = cfg.accent.unwrap_or(sp.badge_color);
    let header = regions.header;
    let htop = header.center.y - header.size.y * 0.5;

    // ---- question header (panel · kicker · accent rule) ----
    if sp.q_panel {
        let mut p = Entity::new(
            format!("{id}.qpanel"),
            Shape::Rect {
                w: header.size.x * 0.98,
                h: header.size.y * 0.94,
            },
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
        p.tags = question_tags("panel");
        s.add(p);
    }
    if let Some(kick) = sp.q_kicker {
        if sp.q_kicker_pill {
            let pc = Vec2::new(
                header.center.x - header.size.x * 0.31,
                htop + header.size.y * 0.25,
            );
            let mut pill = Entity::new(
                format!("{id}.qkbg"),
                Shape::Rect {
                    w: 210.0 * regions.scale,
                    h: 48.0 * regions.scale,
                },
                pc,
                accent,
            );
            pill.stroke.fill = true;
            pill.stroke.outline = false;
            pill.corner_radius = 24.0 * regions.scale;
            pill.tags = question_tags("kicker");
            s.add(pill);
            let mut kt = Entity::new(
                format!("{id}.qk"),
                Shape::Text {
                    content: kick.into(),
                    size: (22.0 * regions.scale).clamp(16.0, 28.0),
                },
                pc,
                style::VOID,
            );
            kt.font = FontKind::MonoBold;
            kt.tags = question_tags("kicker");
            s.add(kt);
        } else {
            let mut kt = Entity::new(
                format!("{id}.qk"),
                Shape::Text {
                    content: kick.into(),
                    size: (23.0 * regions.scale).clamp(16.0, 30.0),
                },
                Vec2::new(header.center.x, htop + header.size.y * 0.23),
                accent,
            );
            kt.font = FontKind::MonoBold;
            kt.tags = question_tags("kicker");
            s.add(kt);
        }
    }
    if sp.q_rule {
        let mut rule = Entity::new(
            format!("{id}.qrule"),
            Shape::Rect {
                w: header.size.x * 0.52,
                h: (4.0 * regions.scale).max(2.0),
            },
            Vec2::new(header.center.x, htop + header.size.y * 0.34),
            accent,
        );
        rule.stroke.fill = true;
        rule.stroke.outline = false;
        rule.corner_radius = 2.0 * regions.scale;
        rule.tags = question_tags("rule");
        s.add(rule);
    }
    // question text — heavy display font, wrapped, title-safe. Initial state
    // depends on the reveal: `type` starts undrawn; the others start drawn but
    // hidden (opacity/offset/scale) so the beat can bring them in.
    let decorated = sp.q_kicker.is_some() || sp.q_rule;
    let text_top = htop + header.size.y * if decorated { 0.42 } else { 0.10 };
    let text_bottom = htop + header.size.y * 0.89;
    let text_h = (text_bottom - text_top).max(30.0 * regions.scale);
    let q_wrap = header.size.x * 0.84;
    let mut q_size = (sp.q_size * regions.scale).clamp(20.0, 62.0);
    // Fit by the estimated wrapped line count. Repeating once accounts for the
    // fact that reducing type size may itself reduce the number of lines.
    for _ in 0..2 {
        let rough_w = q.chars().count().max(1) as f32 * q_size * 0.59;
        let lines = (rough_w / q_wrap.max(1.0)).ceil().clamp(1.0, 4.0);
        q_size = q_size.min(text_h / (lines * 1.20));
    }
    q_size = q_size.max((18.0 * regions.scale).max(16.0));
    let rest = Vec2::new(header.center.x, (text_top + text_bottom) * 0.5);
    let mut qe = Entity::new(
        format!("{id}.q"),
        Shape::Text {
            content: q,
            size: q_size,
        },
        rest,
        style::FG,
    );
    qe.font = FontKind::Display;
    qe.wrap = Some(q_wrap);
    qe.tags = question_tags("text");
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
    let mut timing = QuizTiming::preset(cfg.pace);
    if let Some(seconds) = cfg.seconds {
        timing.think = seconds;
        timing.custom = true;
    }
    let timer_spec = CreatorTimerSpec {
        look: cfg.timer,
        ..CreatorTimerSpec::default()
    };
    s.quizzes.insert(
        id.clone(),
        QuizData {
            reveal: cfg.reveal,
            skin: cfg.skin,
            layout: cfg.layout,
            density: cfg.density,
            labels: cfg.labels,
            timer_style: timer_spec.clone(),
            timing,
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
        },
    );
    configure_timer_widget(
        s,
        &id,
        regions.timer,
        regions.header,
        regions.media,
        regions.scale,
        timing.think,
        &timer_spec,
        accent,
        true,
    );
    Ok(())
}

fn parse_generic_phases(spec: &str, span: Span) -> Result<Vec<TimingPhase>, Error> {
    let mut phases = Vec::new();
    let mut duration: Option<f32> = None;
    for tok in spec.split_whitespace() {
        let (raw_name, raw_value) = tok.split_once('=').ok_or_else(|| {
            Error::new(
                "generic timing phases use name=seconds, for example `intro=1 demo=6 finish=1`",
                span,
            )
        })?;
        let name = raw_name.to_ascii_lowercase();
        let valid_name = name
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'));
        if !valid_name {
            return Err(Error::new(
                format!(
                    "invalid timing phase name `{raw_name}` — use letters, digits, `_`, or `-`"
                ),
                span,
            ));
        }
        let seconds = spec_num(raw_value, raw_name, span)?;
        if !seconds.is_finite() || seconds <= 0.0 {
            return Err(Error::new(
                format!("timing phase `{raw_name}` must be greater than zero seconds"),
                span,
            ));
        }
        if matches!(name.as_str(), "duration" | "total") {
            if duration.replace(seconds).is_some() {
                return Err(Error::new(
                    "generic timing declares `duration` more than once",
                    span,
                ));
            }
            continue;
        }
        if phases.iter().any(|phase: &TimingPhase| phase.name == name) {
            return Err(Error::new(
                format!("timing phase `{raw_name}` is declared more than once"),
                span,
            ));
        }
        phases.push(TimingPhase {
            name,
            duration: seconds,
        });
    }
    if let Some(seconds) = duration {
        if !phases.is_empty() {
            return Err(Error::new(
                "use either `duration=seconds` or named phases, not both",
                span,
            ));
        }
        phases.push(TimingPhase {
            name: "main".into(),
            duration: seconds,
        });
    }
    if phases.is_empty() {
        return Err(Error::new(
            "generic timing needs at least one phase, for example `main=6`",
            span,
        ));
    }
    if phases.len() > 32 {
        return Err(Error::new(
            "generic timing supports at most 32 named phases",
            span,
        ));
    }
    Ok(phases)
}

/// `timing(id, [(x,y)], "spec")` is deliberately overloaded: a quiz id keeps
/// the established ask/options/think/reveal behavior, while a fresh id creates
/// a format-neutral named-phase controller for `timed`/`during` blocks.
pub fn c_timing(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    if let Some(current) = s.quizzes.get(&id) {
        a.max(2)?;
        let spec = a.text(1)?;
        let span = a.span_of(1);
        let mut out = current.timing;

        // Pace is a baseline, independent of token order; explicit numeric
        // values are applied in a second pass.
        for tok in spec.split_whitespace() {
            let (key, value) = tok.split_once('=').unwrap_or(("", tok));
            if key.is_empty()
                || key.eq_ignore_ascii_case("pace")
                || key.eq_ignore_ascii_case("preset")
            {
                let pace = parse_pace(value).ok_or_else(|| {
                    Error::new(
                        "unknown pace — try quick, balanced, calm, or dramatic",
                        span,
                    )
                })?;
                out = QuizTiming::preset(pace);
            }
        }
        for tok in spec.split_whitespace() {
            let (key, value) = tok.split_once('=').unwrap_or(("", tok));
            if key.is_empty()
                || key.eq_ignore_ascii_case("pace")
                || key.eq_ignore_ascii_case("preset")
            {
                continue;
            }
            let value = spec_num(value, key, span)?;
            match key.to_ascii_lowercase().as_str() {
                "ask" | "question" => out.ask = value,
                "options" | "answers" => out.options = value,
                "think" | "seconds" | "countdown" => out.think = value,
                "reveal" | "answer" => out.reveal = value,
                "hold" | "endhold" => out.hold = value,
                "stagger" => out.stagger = value,
                _ => {
                    return Err(Error::new(
                        format!("unknown timing option `{key}` — use pace, ask, options, think, reveal, hold, or stagger"),
                        span,
                    ));
                }
            }
            out.custom = true;
        }
        if out.ask < 0.0
            || out.options <= 0.0
            || out.think <= 0.0
            || out.reveal < 0.0
            || out.hold < 0.0
            || out.stagger < 0.0
        {
            return Err(Error::new(
                "timing phases must be non-negative; options and think must be greater than zero",
                span,
            ));
        }
        let direction = current.timer_style.direction;
        s.quizzes.get_mut(&id).unwrap().timing = out;
        set_timer_counter(s, &id, out.think, direction);
        return Ok(());
    }

    if s.timings.contains_key(&id) || s.sims.contains_key(&id) || s.contains(&id) {
        return Err(Error::new(
            format!(
                "timing controller id `{id}` is already in use — choose a fresh id such as `clock`"
            ),
            a.span_of(0),
        ));
    }
    let canvas = s.canvas();
    let ui = (canvas.x.min(canvas.y) / 1080.0).clamp(0.55, 1.45);
    let (at, spec_index) = match a.exprs.get(1).map(|expr| &expr.kind) {
        Some(ExprKind::Pair(_, _)) => (a.pair(1)?, 2usize),
        _ => (Vec2::new(canvas.x - 90.0 * ui, 90.0 * ui), 1usize),
    };
    a.max(spec_index + 1)?;
    let phases = parse_generic_phases(&a.text(spec_index)?, a.span_of(spec_index))?;
    let total: f32 = phases.iter().map(|phase| phase.duration).sum();
    let spec = CreatorTimerSpec::default();
    let regions = creator_regions(canvas, CreatorSafe::Clean, QuizLayout::Auto);
    let timer_rect = CreatorRect {
        center: at,
        size: Vec2::new((canvas.x * 0.36).clamp(260.0 * ui, 560.0 * ui), 150.0 * ui),
    };
    configure_timer_widget(
        s,
        &id,
        timer_rect,
        regions.header,
        regions.media,
        ui,
        total,
        &spec,
        style::CYAN,
        false,
    );
    configure_timer_playback(s, &id, total, &spec);
    s.timings.insert(
        id,
        TimingData {
            phases,
            timer_style: spec,
            timer_rect,
            ui_scale: ui,
        },
    );
    Ok(())
}

/// Restyle either a quiz countdown or a generic timing controller. Generic
/// controllers additionally accept `timerstyle(id, (x,y), "spec")`.
pub fn c_timerstyle(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    if let Some(qd) = s.quizzes.get(&id).cloned() {
        a.max(2)?;
        let spec_text = a.text(1)?;
        let spec = parse_timer_spec_text(qd.timer_style.clone(), &spec_text, a.span_of(1))?;
        let fallback = qd
            .accent
            .unwrap_or_else(|| skin_spec(qd.skin, qd.accent).badge_color);
        s.quizzes.get_mut(&id).unwrap().timer_style = spec.clone();
        configure_timer_widget(
            s,
            &id,
            qd.timer,
            qd.header,
            qd.media,
            qd.ui_scale,
            qd.timing.think,
            &spec,
            fallback,
            true,
        );
        return Ok(());
    }
    if let Some(timing) = s.timings.get(&id).cloned() {
        let (at, spec_index) = match a.exprs.get(1).map(|expr| &expr.kind) {
            Some(ExprKind::Pair(_, _)) => (a.pair(1)?, 2usize),
            _ => (timing.timer_rect.center, 1usize),
        };
        a.max(spec_index + 1)?;
        let spec = parse_timer_spec_text(
            timing.timer_style.clone(),
            &a.text(spec_index)?,
            a.span_of(spec_index),
        )?;
        let mut timer_rect = timing.timer_rect;
        timer_rect.center = at;
        let regions = creator_regions(s.canvas(), CreatorSafe::Clean, QuizLayout::Auto);
        configure_timer_widget(
            s,
            &id,
            timer_rect,
            regions.header,
            regions.media,
            timing.ui_scale,
            timing.total(),
            &spec,
            style::CYAN,
            false,
        );
        configure_timer_playback(s, &id, timing.total(), &spec);
        let current = s.timings.get_mut(&id).unwrap();
        current.timer_rect = timer_rect;
        current.timer_style = spec;
        return Ok(());
    }
    Err(Error::new(
        format!("no quiz or generic timing controller `{id}` — call `quiz(...)` or `timing({id}, \"main=6\")` first"),
        a.span_of(0),
    ))
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
        let last_row_is_single = n % 2 == 1 && row == rows - 1;
        let x = if last_row_is_single {
            qd.choices.center.x
        } else if col == 0 {
            qd.choices.center.x - (w + gx) / 2.0
        } else {
            qd.choices.center.x + (w + gx) / 2.0
        };
        let total = rows as f32 * h + (rows as f32 - 1.0) * gy;
        let y0 = qd.choices.center.y - total / 2.0 + h / 2.0;
        Vec2::new(x, y0 + row as f32 * (h + gy))
    }
}

fn option_key(i: usize) -> String {
    ((b'a' + i as u8) as char).to_string()
}

fn option_tags(id: &str, i: usize, role: &str, correct: bool) -> Vec<String> {
    let key = option_key(i);
    let mut tags = vec![
        id.to_string(),
        format!("{id}.parts"),
        format!("{id}.options"),
        format!("{id}.option"),
        format!("{id}.option.{key}"),
        format!("{id}.option.{role}"),
        format!("{id}.option.{key}.{role}"),
    ];
    if correct {
        tags.push(format!("{id}.option.correct"));
    }
    tags
}

/// `option(id, "text", [correct])` — add an answer to quiz `id`. `run` lays out
/// 1–6 answers as a centred stack or balanced grid. A trailing `correct` marks
/// the right one (lime highlight + a check on reveal). Stable semantic tags
/// expose A–F and each card/badge/label/text/check role for customization.
pub fn c_option(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let txt = a.text(1)?;
    let correct = a.len() >= 3 && a.ident(2).map(|w| w == "correct").unwrap_or(false);
    let qd = s.quizzes.get(&id).ok_or_else(|| {
        Error::new(
            format!("no quiz `{id}` — call `quiz({id}, \"...\")` first"),
            a.span_of(0),
        )
    })?;
    let i = qd.options.len();
    if correct && qd.options.iter().any(|o| o.correct) {
        return Err(Error::new(
            "quiz already has a correct option; mark exactly one answer correct",
            a.span_of(2),
        ));
    }
    if qd.layout == QuizLayout::Stack && i >= 4 {
        return Err(Error::new("stack layout supports up to four options; use layout=auto or layout=grid for five or six", a.span_of(1)));
    }
    if i >= 6 {
        return Err(Error::new(
            "quiz supports 2–6 options; this would be option 7",
            a.span_of(1),
        ));
    }
    let sp = skin_spec(qd.skin, qd.accent);
    let card_size = qd.card_size;
    let ui = qd.ui_scale;
    let neutral = qd.choices.center; // `run` moves everything to its final slot
    let display_label = match qd.labels {
        QuizLabels::Letters => Some(((b'A' + i as u8) as char).to_string()),
        QuizLabels::Numbers => Some((i + 1).to_string()),
        QuizLabels::None => None,
    };
    let badge_enabled = sp.badge && display_label.is_some();
    let mut parts: Vec<(String, Vec2)> = Vec::new();

    // card
    let card_id = format!("{id}.c{i}");
    let mut card = Entity::new(
        card_id.clone(),
        Shape::Rect {
            w: card_size.x,
            h: card_size.y,
        },
        neutral,
        style::PANEL,
    );
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
    card.tags = option_tags(&id, i, "card", correct);
    s.add(card);
    parts.push((card_id.clone(), Vec2::ZERO));

    // correct-answer highlight (a soft fill tint + a glowing outline), created
    // now but hidden — the beat reveals it. Placed BEHIND the text so the answer
    // stays readable over the tint.
    if correct {
        let mut fillh = Entity::new(
            format!("{id}.hlfill"),
            Shape::Rect {
                w: card_size.x,
                h: card_size.y,
            },
            neutral,
            sp.correct_color,
        );
        fillh.stroke.fill = true;
        fillh.stroke.outline = false;
        fillh.corner_radius = sp.card_radius * ui;
        fillh.opacity = 0.0;
        fillh.tags = option_tags(&id, i, "correct_fill", true);
        s.add(fillh);
        let mut hl = Entity::new(
            format!("{id}.hl"),
            Shape::Rect {
                w: card_size.x + 10.0 * ui,
                h: card_size.y + 10.0 * ui,
            },
            neutral,
            sp.correct_color,
        );
        hl.stroke.fill = false;
        hl.stroke.outline = true;
        hl.stroke.width = (4.0 * ui).max(2.0);
        hl.glow = sp.correct_glow;
        hl.corner_radius = (sp.card_radius + 5.0) * ui;
        hl.opacity = 0.0;
        hl.tags = option_tags(&id, i, "correct_outline", true);
        s.add(hl);
    }

    // letter badge (filled chip) + the letter itself
    let badge_r = (card_size.y * 0.27).min(33.0 * ui).max(17.0 * ui);
    let badge_off = Vec2::new(-card_size.x / 2.0 + badge_r + 20.0 * ui, 0.0);
    if badge_enabled {
        let bid = format!("{id}.b{i}");
        let mut b = Entity::new(
            bid.clone(),
            Shape::Circle { r: badge_r },
            neutral + badge_off,
            sp.badge_color,
        );
        b.stroke.fill = true;
        b.stroke.outline = false;
        b.glow = sp.card_glow;
        b.opacity = 0.0;
        b.tags = option_tags(&id, i, "badge", correct);
        s.add(b);
        parts.push((bid, badge_off));
        // the correct badge turns green on reveal: a hidden green disc over the
        // accent badge (the letter, added next, stays on top and readable)
        if correct {
            let mut bw = Entity::new(
                format!("{id}.bwin"),
                Shape::Circle { r: badge_r },
                neutral + badge_off,
                sp.correct_color,
            );
            bw.stroke.fill = true;
            bw.stroke.outline = false;
            bw.glow = sp.correct_glow;
            bw.opacity = 0.0;
            bw.tags = option_tags(&id, i, "correct_badge", true);
            s.add(bw);
        }
    }
    if let Some(display_label) = display_label.as_ref() {
        let lid = format!("{id}.l{i}");
        let letter_color = if badge_enabled {
            style::VOID
        } else {
            sp.badge_color
        };
        let letter_size = (30.0 * ui).clamp(18.0, 38.0);
        let mut le = Entity::new(
            lid.clone(),
            Shape::Text {
                content: display_label.clone(),
                size: letter_size,
            },
            neutral + badge_off,
            letter_color,
        );
        le.font = FontKind::MonoBold;
        le.opacity = 0.0;
        le.tags = option_tags(&id, i, "label", correct);
        s.add(le);
        parts.push((lid, badge_off));
    }

    // answer text — left-aligned, starting just after the badge
    let left_column = if display_label.is_some() {
        badge_r * 2.0 + 40.0 * ui
    } else {
        28.0 * ui
    };
    let text_off = Vec2::new(-card_size.x / 2.0 + left_column, 0.0);
    let text_id = format!("{id}.t{i}");
    let length_factor = if txt.chars().count() > 58 {
        0.76
    } else if txt.chars().count() > 34 {
        0.86
    } else {
        1.0
    };
    // Every card reserves the same right-side success zone, so correct-state
    // checks never collide with long text or shift the answer alignment.
    let wrap_w = (card_size.x - left_column - 94.0 * ui).max(88.0 * ui);
    let base_text_size = (30.0 * ui * length_factor).clamp(16.0, 38.0);
    let rough_w = txt.chars().count().max(1) as f32 * base_text_size * 0.61;
    let estimated_lines = (rough_w / wrap_w).ceil().clamp(1.0, 4.0);
    let height_fit = card_size.y * 0.70 / (estimated_lines * 1.22);
    let text_size = base_text_size.min(height_fit).max((15.0 * ui).max(13.0));
    let mut te = Entity::new(
        text_id.clone(),
        Shape::Text {
            content: txt,
            size: text_size,
        },
        neutral + text_off,
        style::FG,
    );
    te.align = Align::Left;
    te.font = FontKind::MonoBold;
    te.wrap = Some(wrap_w);
    te.opacity = 0.0;
    te.tags = option_tags(&id, i, "text", correct);
    s.add(te);
    parts.push((text_id.clone(), text_off));

    // a DRAWN check-mark as ONE polyline (moves cleanly via Pos, draws-on via
    // trace — no glyph dependency), on the card's right edge
    if correct {
        let ck_off = Vec2::new(card_size.x / 2.0 - 34.0 * ui, 0.0);
        let mut ck = Entity::new(
            format!("{id}.check"),
            Shape::Polyline {
                pts: vec![
                    Vec2::new(-11.0, 0.0) * ui,
                    Vec2::new(-3.0, 11.0) * ui,
                    Vec2::new(15.0, -13.0) * ui,
                ],
            },
            neutral + ck_off,
            sp.correct_color,
        );
        ck.stroke.width = (6.0 * ui).max(3.0);
        ck.glow = sp.correct_glow;
        ck.trace = 0.0;
        ck.tags = option_tags(&id, i, "check", true);
        s.add(ck);
    }

    let qd = s.quizzes.get_mut(&id).unwrap();
    qd.options.push(QuizOpt {
        card: card_id,
        text: text_id,
        correct,
        badge: badge_enabled,
        parts,
    });
    Ok(())
}

fn active_timer_ids(s: &Scene, owner: &str, role: &str) -> Vec<String> {
    let tag = format!("{owner}.timer.active.{role}");
    s.entities
        .iter()
        .filter(|e| e.tags.iter().any(|t| t == &tag))
        .map(|e| e.id.clone())
        .collect()
}

/// Build the quiz beat as a [`Clip`] (called by the shared `run` verb): reveal
/// the question, stage the answers, run the independently styled timer, then
/// reveal and hold the result. Presets may be proportionally scaled by
/// `run(q,dur)`; explicit numeric phases use `run(q)` to avoid ambiguity.
pub fn build_quiz_clip(s: &Scene, id: &str, dur: Option<f32>, span: Span) -> Result<Clip, Error> {
    let qd = s.quizzes.get(id).unwrap();
    if qd.timing.custom && dur.is_some() {
        return Err(Error::new(
            "this quiz has explicit timing phases; use `run(quiz)` without a duration, or remove the numeric `timing(...)` overrides",
            span,
        ));
    }
    let mut timing = qd.timing;
    if let Some(total) = dur {
        let total = total.max(4.0);
        let scale = total / timing.total().max(0.01);
        timing.ask *= scale;
        timing.options *= scale;
        timing.think *= scale;
        timing.reveal *= scale;
        timing.hold *= scale;
        timing.stagger *= scale;
    }
    let clip_dur = timing.total().max(0.1);
    let n = qd.options.len();
    let stagger_total = timing.stagger * n.saturating_sub(1) as f32;
    if timing.options <= stagger_total + 0.02 {
        return Err(Error::new(
            format!(
                "options phase ({:.2}s) is too short for {n} answers at {:.2}s stagger",
                timing.options, timing.stagger
            ),
            span,
        ));
    }

    let f = |v: f32| TargetValue::Abs(Value::F(v));
    let pos = |p: Vec2| TargetValue::Abs(Value::V(p));
    let mut t: Vec<TrackSpec> = Vec::new();
    let mut push = |eid: String, prop: Prop, target: TargetValue, start: f32, d: f32, e: Easing| {
        t.push(TrackSpec {
            id: eid,
            prop,
            target,
            start,
            dur: d.max(0.001),
            easing: e,
        });
    };
    let (rise, option_ease) = match qd.motion {
        CreatorMotion::Calm => (36.0, Easing::OutCubic),
        CreatorMotion::Studio => (48.0, Easing::OutCubic),
        CreatorMotion::Punch => (66.0, Easing::OutBack),
        CreatorMotion::Cut => (0.0, Easing::Linear),
    };
    let ask_end = timing.ask;
    let options_end = ask_end + timing.options;
    let reveal_start = options_end + timing.think;

    // 1) question reveal.
    let qid = format!("{id}.q");
    match qd.reveal {
        QuizReveal::Type => push(qid, Prop::Trace, f(1.0), 0.0, timing.ask, Easing::Linear),
        QuizReveal::Fade => push(
            qid,
            Prop::Opacity,
            f(1.0),
            0.0,
            timing.ask,
            Easing::OutCubic,
        ),
        QuizReveal::Rise => {
            push(
                qid.clone(),
                Prop::Opacity,
                f(1.0),
                0.0,
                timing.ask,
                Easing::OutCubic,
            );
            push(
                qid,
                Prop::Pos,
                pos(qd.question_pos),
                0.0,
                timing.ask,
                Easing::OutCubic,
            );
        }
        QuizReveal::Pop => {
            push(
                qid.clone(),
                Prop::Opacity,
                f(1.0),
                0.0,
                timing.ask * 0.55,
                Easing::OutCubic,
            );
            push(
                qid.clone(),
                Prop::Scale,
                f(1.06),
                0.0,
                timing.ask * 0.72,
                Easing::OutCubic,
            );
            push(
                qid,
                Prop::Scale,
                f(1.0),
                timing.ask * 0.72,
                timing.ask * 0.28,
                Easing::OutBack,
            );
        }
        QuizReveal::Cut => {}
    }

    // 2) answer cards occupy their own explicit phase.
    let option_dur = (timing.options - stagger_total).max(0.06);
    for (i, opt) in qd.options.iter().enumerate() {
        let sl = slot(qd, n, i);
        let st = ask_end + i as f32 * timing.stagger;
        for (pid, off) in &opt.parts {
            let rest = sl + *off;
            let below = rest + Vec2::new(0.0, rise * qd.ui_scale);
            push(
                pid.clone(),
                Prop::Pos,
                pos(below),
                0.0,
                0.01,
                Easing::Linear,
            );
            push(
                pid.clone(),
                Prop::Pos,
                pos(rest),
                st,
                option_dur,
                option_ease,
            );
            push(
                pid.clone(),
                Prop::Opacity,
                f(1.0),
                st,
                option_dur,
                option_ease,
            );
        }
        if opt.correct {
            for part in [format!("{id}.hlfill"), format!("{id}.hl")] {
                push(part, Prop::Pos, pos(sl), 0.0, 0.01, Easing::Linear);
            }
            push(
                format!("{id}.check"),
                Prop::Pos,
                pos(sl + Vec2::new(qd.card_size.x / 2.0 - 34.0 * qd.ui_scale, 0.0)),
                0.0,
                0.01,
                Easing::Linear,
            );
            if opt.badge {
                let badge_r = (qd.card_size.y * 0.27)
                    .min(33.0 * qd.ui_scale)
                    .max(17.0 * qd.ui_scale);
                let rest =
                    sl + Vec2::new(-qd.card_size.x / 2.0 + badge_r + 20.0 * qd.ui_scale, 0.0);
                push(
                    format!("{id}.bwin"),
                    Prop::Pos,
                    pos(rest),
                    0.0,
                    0.01,
                    Easing::Linear,
                );
            }
        }
    }

    // 3) timer behaviour is independent of its visual look.
    let track_ids = active_timer_ids(s, id, "track");
    let progress_ids = active_timer_ids(s, id, "progress");
    let value_ids = active_timer_ids(s, id, "value");
    let label_ids = active_timer_ids(s, id, "label");
    let effect_ids = active_timer_ids(s, id, "effects");
    let enter = timing.think.min(0.18).max(0.06);
    for eid in &track_ids {
        push(
            eid.clone(),
            Prop::Opacity,
            f(0.30),
            options_end,
            enter,
            Easing::OutCubic,
        );
    }
    for eid in &label_ids {
        push(
            eid.clone(),
            Prop::Opacity,
            f(1.0),
            options_end,
            enter,
            Easing::OutCubic,
        );
    }
    for eid in &value_ids {
        let start_value = if qd.timer_style.direction == TimerDirection::Drain {
            timing.think
        } else {
            0.0
        };
        let end_value = if qd.timer_style.direction == TimerDirection::Drain {
            0.0
        } else {
            timing.think
        };
        push(
            eid.clone(),
            Prop::Value,
            f(start_value),
            options_end,
            0.01,
            Easing::Linear,
        );
        push(
            eid.clone(),
            Prop::Opacity,
            f(1.0),
            options_end,
            enter,
            Easing::OutCubic,
        );
        push(
            eid.clone(),
            Prop::Value,
            f(end_value),
            options_end + 0.01,
            timing.think - 0.01,
            Easing::Linear,
        );
    }
    match qd.timer_style.look {
        QuizTimer::Ring | QuizTimer::Bar => {
            for eid in &progress_ids {
                let (start_trace, end_trace) = if qd.timer_style.direction == TimerDirection::Drain
                {
                    (1.0, 0.0)
                } else {
                    (0.0, 1.0)
                };
                push(
                    eid.clone(),
                    Prop::Trace,
                    f(start_trace),
                    options_end,
                    0.01,
                    Easing::Linear,
                );
                push(
                    eid.clone(),
                    Prop::Opacity,
                    f(1.0),
                    options_end,
                    enter,
                    Easing::OutCubic,
                );
                push(
                    eid.clone(),
                    Prop::Trace,
                    f(end_trace),
                    options_end + 0.01,
                    timing.think - 0.01,
                    Easing::Linear,
                );
            }
        }
        QuizTimer::Segments | QuizTimer::Ticks => {
            let count = progress_ids.len().max(1);
            for (i, eid) in progress_ids.iter().enumerate() {
                let from = if qd.timer_style.direction == TimerDirection::Drain {
                    1.0
                } else {
                    0.0
                };
                let to = if qd.timer_style.direction == TimerDirection::Drain {
                    0.0
                } else {
                    1.0
                };
                push(
                    eid.clone(),
                    Prop::Opacity,
                    f(from),
                    options_end,
                    enter,
                    Easing::OutCubic,
                );
                let step = options_end + timing.think * (i + 1) as f32 / count as f32;
                push(
                    eid.clone(),
                    Prop::Opacity,
                    f(to),
                    step - 0.06,
                    0.06,
                    Easing::OutCubic,
                );
            }
        }
        QuizTimer::Pulse => {
            let beats = timing.think.ceil().max(1.0) as usize;
            let targets = if value_ids.is_empty() {
                &effect_ids
            } else {
                &value_ids
            };
            for beat in 0..beats {
                let at = options_end + timing.think * beat as f32 / beats as f32;
                for eid in targets {
                    push(eid.clone(), Prop::Scale, f(1.12), at, 0.08, Easing::OutBack);
                    push(
                        eid.clone(),
                        Prop::Scale,
                        f(1.0),
                        at + 0.08,
                        0.12,
                        Easing::OutCubic,
                    );
                }
            }
        }
        QuizTimer::Number | QuizTimer::None => {}
    }

    // Timer finish cue. `hold` intentionally leaves the active timer visible.
    let mut visible_timer_ids = Vec::new();
    visible_timer_ids.extend(track_ids.iter().cloned());
    visible_timer_ids.extend(progress_ids.iter().cloned());
    visible_timer_ids.extend(value_ids.iter().cloned());
    visible_timer_ids.extend(label_ids.iter().cloned());
    match qd.timer_style.finish {
        TimerFinish::Hold => {}
        TimerFinish::Fade => {
            for eid in &visible_timer_ids {
                push(
                    eid.clone(),
                    Prop::Opacity,
                    f(0.0),
                    reveal_start,
                    timing.reveal.max(0.12) * 0.45,
                    Easing::OutCubic,
                );
            }
        }
        TimerFinish::Flash | TimerFinish::Pulse => {
            for eid in &effect_ids {
                push(
                    eid.clone(),
                    Prop::Opacity,
                    f(1.0),
                    reveal_start,
                    0.08,
                    Easing::OutCubic,
                );
                push(
                    eid.clone(),
                    Prop::Scale,
                    f(1.16),
                    reveal_start,
                    0.12,
                    Easing::OutBack,
                );
                push(
                    eid.clone(),
                    Prop::Opacity,
                    f(0.0),
                    reveal_start + 0.14,
                    0.18,
                    Easing::OutCubic,
                );
            }
            if qd.timer_style.finish == TimerFinish::Pulse {
                for eid in &value_ids {
                    push(
                        eid.clone(),
                        Prop::Scale,
                        f(1.18),
                        reveal_start,
                        0.12,
                        Easing::OutBack,
                    );
                    push(
                        eid.clone(),
                        Prop::Scale,
                        f(1.0),
                        reveal_start + 0.12,
                        0.14,
                        Easing::OutCubic,
                    );
                }
            }
            for eid in &visible_timer_ids {
                push(
                    eid.clone(),
                    Prop::Opacity,
                    f(0.0),
                    reveal_start + 0.18,
                    0.18,
                    Easing::OutCubic,
                );
            }
        }
    }

    // 4) answer reveal and optional explanation.
    let reveal_anim = timing.reveal.max(0.12);
    for opt in &qd.options {
        if opt.correct {
            push(
                format!("{id}.hlfill"),
                Prop::Opacity,
                f(0.16),
                reveal_start,
                reveal_anim * 0.45,
                Easing::OutCubic,
            );
            push(
                format!("{id}.hl"),
                Prop::Opacity,
                f(1.0),
                reveal_start,
                reveal_anim * 0.45,
                Easing::OutCubic,
            );
            push(
                format!("{id}.hl"),
                Prop::Scale,
                f(1.06),
                reveal_start,
                reveal_anim * 0.45,
                Easing::OutCubic,
            );
            push(
                format!("{id}.hl"),
                Prop::Scale,
                f(1.0),
                reveal_start + reveal_anim * 0.45,
                reveal_anim * 0.40,
                Easing::OutBack,
            );
            push(
                format!("{id}.check"),
                Prop::Trace,
                f(1.0),
                reveal_start + reveal_anim * 0.20,
                reveal_anim * 0.55,
                Easing::OutCubic,
            );
            if opt.badge {
                push(
                    format!("{id}.bwin"),
                    Prop::Opacity,
                    f(1.0),
                    reveal_start,
                    reveal_anim * 0.45,
                    Easing::OutCubic,
                );
            }
        } else {
            for (pid, _) in &opt.parts {
                push(
                    pid.clone(),
                    Prop::Opacity,
                    f(0.26),
                    reveal_start,
                    reveal_anim * 0.55,
                    Easing::OutCubic,
                );
            }
        }
    }
    if !qd.explanation.is_empty() {
        push(
            qd.explanation.clone(),
            Prop::Opacity,
            f(1.0),
            reveal_start + reveal_anim * 0.25,
            reveal_anim * 0.60,
            Easing::OutCubic,
        );
    }
    if !qd.source.is_empty() {
        push(
            qd.source.clone(),
            Prop::Opacity,
            f(1.0),
            reveal_start + reveal_anim * 0.48,
            reveal_anim * 0.48,
            Easing::OutCubic,
        );
    }
    Ok(Clip {
        tracks: t,
        events: vec![],
        dur: clip_dur,
    })
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
            let profile = parse_safe(name).ok_or_else(|| {
                Error::new(
                    "unknown safe-area profile — try shorts, reels, tiktok, or clean",
                    a.span_of(1),
                )
            })?;
            safe_rect(canvas, profile)
        }
        _ => {
            return Err(Error::new(
                "safezone expects a numeric inset or a safe-area name",
                a.span_of(1),
            ))
        }
    };
    let mut r = Entity::new(
        id.clone(),
        Shape::Rect {
            w: area.size.x,
            h: area.size.y,
        },
        area.center,
        style::DIM,
    );
    r.stroke.fill = false;
    r.stroke.outline = true;
    r.stroke.width = 2.0;
    r.opacity = 0.35;
    r.tags = vec![id.clone()];
    s.add(r);
    Ok(())
}

// ---- countdown : standalone Timing v2 widget ----

/// Build the scalar playback shared by standalone countdowns and generic
/// timing controllers. The visual entities have already been configured.
fn configure_timer_playback(s: &mut Scene, id: &str, secs: f32, spec: &CreatorTimerSpec) {
    let n = 61usize;
    let samples = |f: &dyn Fn(f32) -> f32| -> Vec<Vec2> {
        (0..n)
            .map(|k| Vec2::new(f(k as f32 / (n - 1) as f32), 0.0))
            .collect()
    };
    let finish_factor = |u: f32| -> f32 {
        match spec.finish {
            TimerFinish::Hold => 1.0,
            _ if u <= 0.94 => 1.0,
            _ => (1.0 - (u - 0.94) / 0.06).clamp(0.0, 1.0),
        }
    };
    let mut playback = Vec::new();
    let progress_ids = active_timer_ids(s, id, "progress");
    match spec.look {
        QuizTimer::Ring | QuizTimer::Bar => {
            let points = samples(&|u| {
                if spec.direction == TimerDirection::Drain {
                    1.0 - u
                } else {
                    u
                }
            });
            for eid in &progress_ids {
                playback.push(PlaybackTrack {
                    id: eid.clone(),
                    prop: Prop::Trace,
                    points: points.clone(),
                });
                if spec.finish != TimerFinish::Hold {
                    playback.push(PlaybackTrack {
                        id: eid.clone(),
                        prop: Prop::Opacity,
                        points: samples(&finish_factor),
                    });
                }
            }
        }
        QuizTimer::Segments | QuizTimer::Ticks => {
            let count = progress_ids.len().max(1);
            for (i, eid) in progress_ids.iter().enumerate() {
                let threshold = (i + 1) as f32 / count as f32;
                let points = samples(&|u| {
                    let progress = if spec.direction == TimerDirection::Drain {
                        if u < threshold {
                            1.0
                        } else {
                            0.0
                        }
                    } else if u >= threshold {
                        1.0
                    } else {
                        0.0
                    };
                    progress * finish_factor(u)
                });
                playback.push(PlaybackTrack {
                    id: eid.clone(),
                    prop: Prop::Opacity,
                    points,
                });
            }
        }
        QuizTimer::Pulse | QuizTimer::Number | QuizTimer::None => {}
    }
    for eid in active_timer_ids(s, id, "value") {
        let values = samples(&|u| {
            if spec.direction == TimerDirection::Drain {
                secs * (1.0 - u)
            } else {
                secs * u
            }
        });
        playback.push(PlaybackTrack {
            id: eid.clone(),
            prop: Prop::Value,
            points: values,
        });
        if spec.finish != TimerFinish::Hold {
            playback.push(PlaybackTrack {
                id: eid.clone(),
                prop: Prop::Opacity,
                points: samples(&finish_factor),
            });
        }
        if spec.look == QuizTimer::Pulse {
            playback.push(PlaybackTrack {
                id: eid,
                prop: Prop::Scale,
                points: samples(&|u| {
                    1.0 + 0.12 * (std::f32::consts::TAU * secs * u).sin().max(0.0)
                }),
            });
        }
    }
    if spec.finish != TimerFinish::Hold {
        for role in ["track", "label"] {
            for eid in active_timer_ids(s, id, role) {
                let base = if role == "track" { 0.30 } else { 1.0 };
                playback.push(PlaybackTrack {
                    id: eid,
                    prop: Prop::Opacity,
                    points: samples(&|u| base * finish_factor(u)),
                });
            }
        }
    }
    if matches!(spec.finish, TimerFinish::Flash | TimerFinish::Pulse) {
        for eid in active_timer_ids(s, id, "effects") {
            playback.push(PlaybackTrack {
                id: eid.clone(),
                prop: Prop::Opacity,
                points: samples(&|u| {
                    if u < 0.88 {
                        0.0
                    } else {
                        ((u - 0.88) / 0.12 * std::f32::consts::PI).sin().max(0.0)
                    }
                }),
            });
            playback.push(PlaybackTrack {
                id: eid,
                prop: Prop::Scale,
                points: samples(&|u| 1.0 + 0.18 * u),
            });
        }
    }
    s.sims.insert(
        id.to_string(),
        SimData {
            playback,
            dt: secs / n as f32,
            ..Default::default()
        },
    );
}

/// Playback for a format-neutral timing controller. Its named phases are the
/// source of truth, so a second duration on `run(id, dur)` is rejected rather
/// than letting the visual clock drift from `timed`/`during` choreography.
pub fn build_generic_timing_clip(
    s: &Scene,
    id: &str,
    dur: Option<f32>,
    span: Span,
) -> Result<Clip, Error> {
    let timing = s
        .timings
        .get(id)
        .ok_or_else(|| Error::new(format!("no generic timing controller `{id}`"), span))?;
    if dur.is_some() {
        return Err(Error::new(
            "this timing controller already defines exact named phases; use `run(id)` or `timed(id) { ... }` without a second duration",
            span,
        ));
    }
    let total = timing.total().max(0.001);
    let sim = s.sims.get(id).ok_or_else(|| {
        Error::new(
            format!("timing controller `{id}` has no timer playback"),
            span,
        )
    })?;
    let frames = sim
        .playback
        .iter()
        .map(|track| track.points.len())
        .max()
        .unwrap_or(0);
    if frames < 2 {
        return Ok(Clip::wait(total));
    }
    let frame = total / (frames - 1) as f32;
    let mut tracks = Vec::new();
    for playback in &sim.playback {
        for k in 1..playback.points.len() {
            let target = match playback.prop {
                Prop::Value | Prop::Opacity | Prop::Scale | Prop::Rot | Prop::Trace | Prop::Hue => {
                    TargetValue::Abs(Value::F(playback.points[k].x))
                }
                _ => TargetValue::Abs(Value::V(playback.points[k])),
            };
            tracks.push(TrackSpec {
                id: playback.id.clone(),
                prop: playback.prop,
                target,
                start: (k - 1) as f32 * frame,
                dur: frame,
                easing: Easing::Linear,
            });
        }
    }
    Ok(Clip {
        tracks,
        events: vec![],
        dur: total,
    })
}

/// `countdown(id, [at], [secs], ["style"])` — the standalone form of the same
/// native timer system used by quizzes. Play it with `run(id, secs)`.
pub fn c_countdown(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let canvas = s.canvas();
    let ui = (canvas.x.min(canvas.y) / 1080.0).clamp(0.55, 1.45);
    let at = if a.len() >= 2 {
        a.pair(1)?
    } else {
        canvas * 0.5
    };
    let secs = a.opt_num(2)?.unwrap_or(5.0).max(1.0);
    let mut spec = CreatorTimerSpec::default();
    if let Some(text) = a.opt_text(3)? {
        spec = parse_timer_spec_text(spec, &text, a.span_of(3))?;
    }
    let regions = creator_regions(canvas, CreatorSafe::Clean, QuizLayout::Auto);
    let timer_rect = CreatorRect {
        center: at,
        size: Vec2::new(canvas.x * 0.68, 180.0 * ui),
    };
    configure_timer_widget(
        s,
        &id,
        timer_rect,
        regions.header,
        regions.media,
        ui,
        secs,
        &spec,
        style::CYAN,
        false,
    );

    configure_timer_playback(s, &id, secs, &spec);
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
                for d in [
                    Vec2::new(-hw, -hh),
                    Vec2::new(hw, -hh),
                    Vec2::new(hw, hh),
                    Vec2::new(-hw, hh),
                ] {
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
                for p in pts {
                    point(e.pos + *p * scale);
                }
            }
            Shape::Region { tris, rings } => {
                for tri in tris {
                    for p in tri {
                        point(e.pos + *p * scale);
                    }
                }
                for ring in rings {
                    for p in ring {
                        point(e.pos + *p * scale);
                    }
                }
            }
            Shape::Text { content, size } => {
                let em = *size * scale;
                let rough = content.chars().count().max(1) as f32 * em * 0.61;
                let width = e
                    .wrap
                    .map(|w| rough.min(w * scale))
                    .unwrap_or(rough)
                    .max(em * 0.5);
                let lines = e
                    .wrap
                    .map(|w| (rough / (w * scale).max(1.0)).ceil())
                    .unwrap_or(1.0)
                    .max(1.0);
                let height = em * 1.25 * lines;
                let left = if e.align == Align::Left {
                    0.0
                } else {
                    -width * 0.5
                };
                for d in [
                    Vec2::new(left, -height * 0.5),
                    Vec2::new(left + width, -height * 0.5),
                    Vec2::new(left + width, height * 0.5),
                    Vec2::new(left, height * 0.5),
                ] {
                    point(e.pos + d);
                }
            }
            Shape::RichText { runs, size } => {
                let em = *size * scale;
                let width = runs
                    .iter()
                    .map(|run| match run {
                        crate::primitives::TextRun::Text(t) => t.chars().count() as f32 * em * 0.61,
                        crate::primitives::TextRun::Math { w, .. } => *w * scale,
                    })
                    .sum::<f32>()
                    .max(em);
                let height = runs
                    .iter()
                    .map(|run| match run {
                        crate::primitives::TextRun::Text(_) => em * 1.25,
                        crate::primitives::TextRun::Math { h, .. } => *h * scale,
                    })
                    .fold(em, f32::max);
                let left = if e.align == Align::Left {
                    0.0
                } else {
                    -width * 0.5
                };
                for d in [
                    Vec2::new(left, -height * 0.5),
                    Vec2::new(left + width, -height * 0.5),
                    Vec2::new(left + width, height * 0.5),
                    Vec2::new(left, height * 0.5),
                ] {
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
    let zc = if a.len() >= 2 {
        a.pair(1)?
    } else {
        default_zone.center
    };
    let zs = if a.len() >= 3 {
        a.pair(2)?
    } else {
        default_zone.size
    };
    // gather the group: entities tagged `target` (or the entity itself)
    let ids: Vec<usize> = (0..s.entities.len())
        .filter(|&i| s.entities[i].id == target || s.entities[i].tags.iter().any(|t| t == &target))
        .collect();
    if ids.is_empty() {
        return Err(Error::new(
            format!("figure: no entity or group `{target}`"),
            a.span_of(0),
        ));
    }
    // A derived construction recomputes from its dependencies every frame. If a
    // dependency is left outside the fitted group, the first rendered frame
    // would snap the construction back apart; fail early with a useful message.
    for &i in &ids {
        let e = &s.entities[i];
        if e.derive.is_some()
            && e.deps
                .iter()
                .any(|dep| !ids.iter().any(|&j| s.entities[j].id == *dep))
        {
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
            Shape::Arc { r, inner, .. } => {
                *r *= sc;
                *inner *= sc;
            }
            Shape::Rect { w, h } => {
                *w *= sc;
                *h *= sc;
            }
            Shape::Line { to } | Shape::Arrow { to } | Shape::Coil { to, .. } => {
                *to = zc + (*to - bc) * sc
            }
            Shape::Curve { ctrl, to, .. } => {
                *ctrl = zc + (*ctrl - bc) * sc;
                *to = zc + (*to - bc) * sc;
            }
            Shape::Polyline { pts } | Shape::Polygon { pts } => {
                for p in pts.iter_mut() {
                    *p *= sc;
                }
            }
            Shape::Region { tris, rings } => {
                for tri in tris.iter_mut() {
                    for p in tri.iter_mut() {
                        *p *= sc;
                    }
                }
                for ring in rings.iter_mut() {
                    for p in ring.iter_mut() {
                        *p *= sc;
                    }
                }
            }
            Shape::Text { size, .. } | Shape::RichText { size, .. } => *size *= sc,
            Shape::Image { w, h, .. } => {
                *w *= sc;
                *h *= sc;
            }
        }
        if let Some(w) = &mut e.wrap {
            *w *= sc;
        }
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
        Error::new(
            format!("no quiz `{id}` — call `quiz({id}, \"...\")` first"),
            a.span_of(0),
        )
    })?;
    let at = qd.timer.center;
    let ui = qd.ui_scale;
    let wrap = qd.timer.size.x * 0.92;
    let explanation_id = format!("{id}.explain");
    let mut text = Entity::new(
        explanation_id.clone(),
        Shape::Text {
            content,
            size: (27.0 * ui).clamp(18.0, 34.0),
        },
        at - Vec2::new(0.0, source.as_ref().map(|_| 10.0 * ui).unwrap_or(0.0)),
        style::FG,
    );
    text.font = FontKind::MonoBold;
    text.wrap = Some(wrap);
    text.opacity = 0.0;
    text.tags = vec![
        id.clone(),
        format!("{id}.parts"),
        format!("{id}.explanation"),
    ];
    s.add(text);

    let mut source_id = String::new();
    if let Some(source) = source {
        source_id = format!("{id}.source");
        let mut se = Entity::new(
            source_id.clone(),
            Shape::Text {
                content: source,
                size: (18.0 * ui).clamp(14.0, 24.0),
            },
            at + Vec2::new(0.0, 30.0 * ui),
            style::DIM,
        );
        se.wrap = Some(wrap);
        se.opacity = 0.0;
        se.tags = vec![
            id.clone(),
            format!("{id}.parts"),
            format!("{id}.explanation"),
        ];
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
        Error::new(
            format!("no creator `{id}` — call `creator({id}, \"...\")` first"),
            a.span_of(0),
        )
    })?;
    let mut cta = prof.cta.clone();
    let mut safe = prof.safe;
    let mut title = prof.display_name.clone();
    if let Some(spec) = a.opt_text(1)? {
        for tok in spec.split_whitespace() {
            let (k, v) = tok
                .split_once('=')
                .ok_or_else(|| Error::new("endcard options use key=value", a.span_of(1)))?;
            match k.to_ascii_lowercase().as_str() {
                "cta" => cta = decode_spec_text(v),
                "title" | "name" => title = decode_spec_text(v),
                "safe" | "platform" => {
                    safe = parse_safe(v).ok_or_else(|| {
                        Error::new(
                            "unknown safe area — try shorts, reels, tiktok, or clean",
                            a.span_of(1),
                        )
                    })?
                }
                _ => {
                    return Err(Error::new(
                        format!("unknown endcard option `{k}` — use cta, title, or safe"),
                        a.span_of(1),
                    ))
                }
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
    let panel_size = Vec2::new(
        region.size.x * 0.88,
        region.size.y
            * if canvas.y / canvas.x > 1.3 {
                0.62
            } else {
                0.78
            },
    );
    let center = region.center;

    let mut panel = Entity::new(
        format!("{id}.end.panel"),
        Shape::Rect {
            w: panel_size.x,
            h: panel_size.y,
        },
        center,
        style::PANEL,
    );
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
                Shape::Image {
                    path: prof.logo.clone(),
                    w: logo_size,
                    h: logo_size,
                    tint: false,
                },
                Vec2::new(center.x, top),
                style::FG,
            ),
            &tag,
        );
    } else {
        let mut mark = Entity::new(
            format!("{id}.end.logo"),
            Shape::Circle { r: logo_size * 0.5 },
            Vec2::new(center.x, top),
            accent,
        );
        mark.stroke.fill = true;
        mark.glow = 0.25;
        add_end_part(s, mark, &tag);
        let initial = title
            .chars()
            .next()
            .unwrap_or('•')
            .to_ascii_uppercase()
            .to_string();
        let mut letter = Entity::new(
            format!("{id}.end.initial"),
            Shape::Text {
                content: initial,
                size: logo_size * 0.42,
            },
            Vec2::new(center.x, top),
            style::VOID,
        );
        letter.font = FontKind::Display;
        add_end_part(s, letter, &tag);
    }

    let mut name = Entity::new(
        format!("{id}.end.name"),
        Shape::Text {
            content: title,
            size: (46.0 * ui).clamp(27.0, 58.0),
        },
        Vec2::new(center.x, center.y - panel_size.y * 0.02),
        style::FG,
    );
    name.font = FontKind::Display;
    name.wrap = Some(panel_size.x * 0.78);
    add_end_part(s, name, &tag);

    let meta = if !prof.tagline.is_empty() {
        prof.tagline.clone()
    } else if !prof.handle.is_empty() {
        prof.handle.clone()
    } else {
        prof.website.clone()
    };
    if !meta.is_empty() {
        let mut sub = Entity::new(
            format!("{id}.end.meta"),
            Shape::Text {
                content: meta,
                size: (24.0 * ui).clamp(17.0, 31.0),
            },
            Vec2::new(center.x, center.y + panel_size.y * 0.10),
            style::DIM,
        );
        sub.wrap = Some(panel_size.x * 0.72);
        add_end_part(s, sub, &tag);
    }

    let cta_at = Vec2::new(center.x, center.y + panel_size.y * 0.28);
    let cta_w = (cta.chars().count() as f32 * 18.0 * ui + 110.0 * ui)
        .clamp(panel_size.x * 0.34, panel_size.x * 0.76);
    let mut button = Entity::new(
        format!("{id}.end.cta_bg"),
        Shape::Rect {
            w: cta_w,
            h: 70.0 * ui,
        },
        cta_at,
        accent,
    );
    button.stroke.fill = true;
    button.stroke.outline = true;
    button.stroke.outline_color = Some(secondary);
    button.stroke.width = (2.0 * ui).max(1.0);
    button.corner_radius = 35.0 * ui;
    button.glow = 0.35;
    add_end_part(s, button, &tag);
    let mut button_text = Entity::new(
        format!("{id}.end.cta"),
        Shape::Text {
            content: cta,
            size: (24.0 * ui).clamp(17.0, 30.0),
        },
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
    r.ctor("timing", c_timing);
    r.ctor("timerstyle", c_timerstyle);
    r.ctor("safezone", c_safezone);
    r.ctor("countdown", c_countdown);
    r.ctor("figure", c_figure);
    r.ctor("explain", c_explain);
    r.ctor("endcard", c_endcard);
}

#[cfg(test)]
mod tests {
    use crate::primitives::Shape;

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
        assert!(
            m.validate().is_ok(),
            "creator+socials should validate: {:?}",
            m.validate().err()
        );
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
        for sub in [
            "q.q",
            "q.timer.progress.main",
            "q.timer.value.main",
            "q.c0",
            "q.t0",
            "q.hl",
            "q.c3",
        ] {
            assert!(base.contains(sub), "missing quiz entity `{sub}`");
        }
        // `option` must NOT broadcast over the quiz's parts (consumes the id)
        assert!(
            m.validate().is_ok(),
            "quiz+option+run should validate: {:?}",
            m.validate().err()
        );
    }

    #[test]
    fn quiz_v24_labels_and_semantic_option_tags() {
        let numbered = crate::parse(
            "quiz(q, \"Choose the invariant\", \"labels=numbers\");\n\
             option(q, \"Area\", correct); option(q, \"Perimeter\"); run(q, 8);",
        )
        .unwrap();
        let base = numbered.base();
        assert!(
            matches!(&base.get("q.l0").unwrap().shape, Shape::Text { content, .. } if content == "1")
        );
        assert!(
            matches!(&base.get("q.l1").unwrap().shape, Shape::Text { content, .. } if content == "2")
        );
        for tag in [
            "q.options",
            "q.option.a",
            "q.option.a.card",
            "q.option.correct",
        ] {
            assert!(
                base.get("q.c0")
                    .unwrap()
                    .tags
                    .iter()
                    .any(|actual| actual == tag),
                "card missing semantic tag {tag}"
            );
        }
        assert!(base
            .get("q.q")
            .unwrap()
            .tags
            .iter()
            .any(|tag| tag == "q.question.text"));
        assert!(base
            .get("q.qrule")
            .unwrap()
            .tags
            .iter()
            .any(|tag| tag == "q.question.rule"));

        let plain = crate::parse(
            "quiz(q, \"Choose the invariant\", \"labels=none\");\n\
             option(q, \"Area\", correct); option(q, \"Perimeter\"); run(q, 8);",
        )
        .unwrap();
        assert!(!plain.base().contains("q.l0") && !plain.base().contains("q.b0"));
        assert!(
            !plain.base().contains("q.bwin"),
            "no-label mode must not animate a missing badge"
        );
        assert!(plain.validate().is_ok());
    }

    #[test]
    fn quiz_v24_centres_a_single_card_in_the_last_grid_row() {
        let m = crate::parse(
            "canvas(1080,1920); quiz(q, \"Pick one\", \"layout=grid\");\n\
             option(q, \"A\"); option(q, \"B\"); option(q, \"C\", correct);\n\
             option(q, \"D\"); option(q, \"E\"); run(q, 8);",
        )
        .unwrap();
        let centre_x = m.base().quizzes.get("q").unwrap().choices.center.x;
        let (base, timeline) = m.finalize();
        let frame = timeline.apply(&base, 3.2);
        assert!((frame.get("q.c4").unwrap().pos.x - centre_x).abs() < 0.01);
    }

    #[test]
    fn creator_v24_native_social_registry_and_values() {
        let labelled = crate::parse(
            "canvas(1080,1920); creator(me, \"@anish2good yt=zarigatongy x=@anish2good web=8gwifi.org/manic accent=cyan\"); socials(me);",
        )
        .unwrap();
        let base = labelled.base();
        assert!(
            matches!(&base.get("me.platform0").unwrap().shape, Shape::Text { content, .. } if content == "zarigatongy")
        );
        assert!(
            matches!(&base.get("me.handle").unwrap().shape, Shape::Text { content, .. } if content == "@anish2good")
        );
        assert!(
            matches!(&base.get("me.platform2").unwrap().shape, Shape::Text { content, .. } if content == "8gwifi.org/manic")
        );
        for (id, tag) in [
            ("me.icon0b", "me.social.youtube"),
            ("me.icon11", "me.social.x"),
            ("me.icon2b", "me.social.web"),
        ] {
            assert!(
                base.get(id)
                    .unwrap()
                    .tags
                    .iter()
                    .any(|actual| actual == tag),
                "{id} missing {tag}"
            );
        }

        let registry = crate::parse(
            "creator(me, \"@all fb=a li=b gh=c tt=d ig=e mail=f mystery=g\"); socials(me);",
        )
        .unwrap();
        for platform in [
            "facebook",
            "linkedin",
            "github",
            "tiktok",
            "instagram",
            "email",
            "link",
        ] {
            let tag = format!("me.social.{platform}");
            assert!(
                registry
                    .base()
                    .entities
                    .iter()
                    .any(|entity| entity.tags.iter().any(|actual| actual == &tag)),
                "native registry missing {platform}"
            );
        }
        assert!(registry.validate().is_ok());
    }

    /// The question reveal style is controllable; default is typewriter, and an
    /// unknown style is a clear error listing the valid ones.
    #[test]
    fn quiz_reveal_style_controllable() {
        // default = typewriter → question starts undrawn (trace 0)
        let d = crate::parse(
            "canvas(\"9:16\"); quiz(q, \"hi?\"); option(q, \"a\", correct); run(q, 8);",
        )
        .unwrap();
        assert_eq!(
            d.base().get("q.q").unwrap().trace,
            0.0,
            "default should be typewriter (trace 0)"
        );

        // `fade` → fully drawn (trace 1) but transparent (opacity 0)
        let f = crate::parse(
            "canvas(\"9:16\"); quiz(q, \"hi?\", \"fade\"); option(q, \"a\", correct); run(q, 8);",
        )
        .unwrap();
        let qe = f.base().get("q.q").unwrap();
        assert_eq!(qe.trace, 1.0, "fade should leave text drawn");
        assert_eq!(qe.opacity, 0.0, "fade should start transparent");

        // aliases resolve; each style validates end-to-end
        for style in ["typewriter", "rise", "pop", "cut", "instant", "slide"] {
            let src = format!("canvas(\"9:16\"); quiz(q, \"hi?\", \"{style}\"); option(q, \"a\", correct); run(q, 8);");
            let m = crate::parse(&src).unwrap();
            assert!(
                m.validate().is_ok(),
                "style {style:?} should validate: {:?}",
                m.validate().err()
            );
        }

        // an unknown style is rejected
        assert!(
            crate::parse("canvas(\"9:16\"); quiz(q, \"hi?\", \"sparkle\");").is_err(),
            "unknown style should error"
        );
    }

    /// The card skin is controllable, defaults to Badge, and the spec mixes a
    /// skin word + a reveal word order-free.
    #[test]
    fn quiz_skin_controllable() {
        // default skin = Badge → a framed panel + letter-badge entities appear
        let d = crate::parse(
            "canvas(\"9:16\"); quiz(q, \"hi?\"); option(q, \"a\", correct); run(q, 8);",
        )
        .unwrap();
        let base = d.base();
        assert!(
            base.contains("q.qpanel"),
            "badge (default) should draw a question panel"
        );
        assert!(
            base.contains("q.b0"),
            "badge (default) should draw a letter-badge"
        );
        assert!(
            base.contains("q.bwin"),
            "correct badge should have a green-win overlay"
        );

        // `minimal` → no panel, no badges (outline rows)
        let m = crate::parse("canvas(\"9:16\"); quiz(q, \"hi?\", \"minimal\"); option(q, \"a\", correct); run(q, 8);").unwrap();
        assert!(!m.base().contains("q.qpanel"), "minimal has no panel");
        assert!(!m.base().contains("q.b0"), "minimal has no letter-badge");

        // order-free spec: skin + reveal together, either order
        for spec in ["glass fade", "fade glass", "plain cut"] {
            let src = format!("canvas(\"9:16\"); quiz(q, \"hi?\", \"{spec}\"); option(q, \"a\", correct); run(q, 8);");
            let mv = crate::parse(&src).unwrap();
            assert!(
                mv.validate().is_ok(),
                "spec {spec:?} should validate: {:?}",
                mv.validate().err()
            );
        }

        // an unknown token in the spec is rejected
        assert!(
            crate::parse("canvas(\"9:16\"); quiz(q, \"hi?\", \"badge wobble\");").is_err(),
            "unknown spec token should error"
        );
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
        assert!(
            base.contains("sz")
                && base.contains("cd.timer.progress.main")
                && base.contains("cd.timer.value.main"),
            "countdown/safezone parts missing"
        );
        // figure fitted the group into the zone (moved the circles toward centre)
        let a = base.get("a").unwrap().pos;
        assert!(
            a.y < 900.0,
            "figure should move the group up into the zone (was y=900)"
        );
        assert!(
            m.validate().is_ok(),
            "countdown+figure should validate: {:?}",
            m.validate().err()
        );
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
            assert!(
                m.validate().is_ok(),
                "{w}x{h} should validate: {:?}",
                m.validate().err()
            );
            let qd = m.base().quizzes.get("q").unwrap();
            for region in [qd.header, qd.media, qd.choices, qd.timer, qd.footer] {
                let lo = region.center - region.size * 0.5;
                let hi = region.center + region.size * 0.5;
                assert!(
                    lo.x >= -0.1
                        && lo.y >= -0.1
                        && hi.x <= w as f32 + 0.1
                        && hi.y <= h as f32 + 0.1,
                    "region escaped {w}x{h}: {region:?}"
                );
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
                assert!(
                    lo.x >= clo.x - 1.0
                        && hi.x <= chi.x + 1.0
                        && lo.y >= clo.y - 1.0
                        && hi.y <= chi.y + 1.0,
                    "card {i} escaped choices at {w}x{h}: {lo:?}..{hi:?} vs {clo:?}..{chi:?}"
                );
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
        assert_eq!(q.timer_style.look, super::QuizTimer::Number);
        assert_eq!(q.motion, super::CreatorMotion::Calm);
        assert_eq!(q.safe, super::CreatorSafe::Reels);
        assert!(m.base().contains("q.explain") && m.base().contains("q.source"));
        assert!(m.validate().is_ok());
    }

    #[test]
    fn timing_v2_uses_absolute_phases_and_rejects_a_second_total() {
        let m = crate::parse(
            "quiz(q, \"pick\"); option(q, \"yes\", correct); option(q, \"no\");\n\
             timing(q, \"calm ask=1 options=1 think=6 reveal=0.5 hold=2 stagger=0.05\"); run(q);",
        )
        .unwrap();
        let timing = m.base().quizzes.get("q").unwrap().timing;
        assert_eq!(timing.pace, super::CreatorPace::Calm);
        assert!(timing.custom);
        assert!((timing.total() - 10.5).abs() < 1e-4);
        let clip =
            super::build_quiz_clip(m.base(), "q", None, crate::lang::diag::Span::new(1, 1, 1))
                .unwrap();
        assert!(
            (clip.dur - 10.5).abs() < 1e-3,
            "explicit phases should define the clip duration"
        );

        assert!(crate::parse(
            "quiz(q, \"pick\"); option(q, \"yes\", correct); timing(q, \"think=6\"); run(q, 12);"
        )
        .is_err(), "explicit phases plus run duration should be rejected as ambiguous");

        let scaled = crate::parse(
            "quiz(q, \"pick\"); option(q, \"yes\", correct); timing(q, \"quick\"); run(q, 14);",
        )
        .unwrap();
        let clip = super::build_quiz_clip(
            scaled.base(),
            "q",
            Some(14.0),
            crate::lang::diag::Span::new(1, 1, 1),
        )
        .unwrap();
        assert!(
            (clip.dur - 14.0).abs() < 1e-3,
            "a pace preset should remain scalable by run duration"
        );
    }

    #[test]
    fn timing_v2_timer_looks_build_native_active_parts() {
        for (look, expected_progress) in [
            ("ring", 1usize),
            ("bar", 1),
            ("number", 0),
            ("segments", super::TIMER_SEGMENT_COUNT),
            ("ticks", super::TIMER_TICK_COUNT),
            ("pulse", 0),
            ("none", 0),
        ] {
            let src = format!(
                "quiz(q, \"pick\"); option(q, \"yes\", correct); option(q, \"no\"); timerstyle(q, \"look={look}\"); run(q, 8);"
            );
            let m = crate::parse(&src).unwrap();
            let tag = "q.timer.active.progress";
            let progress = m
                .base()
                .entities
                .iter()
                .filter(|e| e.tags.iter().any(|t| t == tag))
                .count();
            assert_eq!(
                progress, expected_progress,
                "wrong active progress-part count for {look}"
            );
            assert!(
                m.validate().is_ok(),
                "timer look {look} should validate: {:?}",
                m.validate().err()
            );
        }
    }

    #[test]
    fn timing_v2_style_tokens_and_standalone_countdown_share_the_system() {
        let m = crate::parse(
            "canvas(1080,1920); quiz(q, \"pick\"); option(q, \"yes\", correct);\n\
             timerstyle(q, \"segments position=media number=outside direction=fill size=large thickness=1.4 color=magenta track=dim label=THINK font=display finish=pulse\");\n\
             timing(q, \"balanced seconds=7\"); run(q);\n\
             countdown(cd, (540,1400), 5, \"ticks direction=fill label=GO finish=flash\"); run(cd,5);",
        )
        .unwrap();
        let q = m.base().quizzes.get("q").unwrap();
        assert_eq!(q.timer_style.look, super::QuizTimer::Segments);
        assert_eq!(q.timer_style.position, super::TimerPosition::Media);
        assert_eq!(q.timer_style.number, super::TimerNumber::Outside);
        assert_eq!(q.timer_style.direction, super::TimerDirection::Fill);
        assert_eq!(q.timer_style.finish, super::TimerFinish::Pulse);
        assert_eq!(q.timer_style.font, super::TimerFont::Display);
        assert_eq!(q.timer_style.label, "THINK");
        assert!((q.timing.think - 7.0).abs() < 1e-4);
        assert!(m.base().contains("cd.timer.progress.tick0"));
        assert!(m.validate().is_ok());
    }

    #[test]
    fn timing_v2_generic_controller_schedules_non_quiz_phases_exactly() {
        let m = crate::parse(
            "canvas(1280,720); text(a,(300,200),\"INTRO\"); hidden(a); text(b,(900,520),\"DONE\"); hidden(b);\n\
             timing(clock,(1110,70),\"intro=1 motion=3 finish=1\");\n\
             timerstyle(clock,\"look=ticks direction=fill label=SCENE finish=hold\");\n\
             timed(clock) {\n\
               during(\"finish\") { show(b,0.4); }\n\
               during(\"intro\") { show(a,0.5); }\n\
               during(\"motion\") { wait(2.5); }\n\
             }",
        )
        .unwrap();
        let timing = m.base().timings.get("clock").unwrap();
        assert!((timing.total() - 5.0).abs() < 1e-4);
        assert_eq!(timing.phase("motion"), Some((1.0, 3.0)));
        assert_eq!(timing.timer_style.look, super::QuizTimer::Ticks);
        assert!(m.base().contains("clock.timer.progress.tick0"));
        assert!(m.validate().is_ok());
        let (_, timeline) = m.finalize();
        assert!(
            (timeline.dur - 6.0).abs() < 1e-3,
            "five-second controller plus the standard one-second export tail"
        );
    }

    #[test]
    fn timing_v2_generic_controller_rejects_phase_drift() {
        assert!(
            crate::parse(
                "timing(clock,\"intro=1\"); timed(clock) { during(\"intro\") { wait(1.2); } }"
            )
            .is_err(),
            "a phase block must not overrun its declared duration"
        );
        assert!(
            crate::parse(
                "timing(clock,\"intro=1\"); timed(clock) { during(\"missing\") { wait(0.2); } }"
            )
            .is_err(),
            "unknown phase names must be rejected"
        );
        assert!(crate::parse(
            "timing(clock,\"intro=1\"); timed(clock) { during(\"intro\") { wait(0.2); } during(\"intro\") { wait(0.2); } }"
        )
        .is_err(), "a phase must have at most one during block");
    }

    #[test]
    fn timing_v2_generic_duration_alias_and_run_are_exact() {
        let m = crate::parse(
            "timing(clock,\"duration=4\"); timerstyle(clock,\"look=none\"); run(clock);",
        )
        .unwrap();
        assert_eq!(
            m.base().timings.get("clock").unwrap().phase("main"),
            Some((0.0, 4.0))
        );
        let (_, timeline) = m.finalize();
        assert!((timeline.dur - 5.0).abs() < 1e-3);
        assert!(
            crate::parse("timing(clock,\"main=4\"); run(clock,8);").is_err(),
            "run duration must not compete with exact generic phases"
        );
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
        assert!(
            m.validate().is_ok(),
            "v2 profile/endcard should validate: {:?}",
            m.validate().err()
        );
    }

    #[test]
    fn quiz_v2_rejects_ambiguous_answers_and_overfull_stack() {
        assert!(
            crate::parse(
                "quiz(q, \"pick\"); option(q, \"a\", correct); option(q, \"b\", correct);"
            )
            .is_err(),
            "two correct options should be a friendly authoring error"
        );
        assert!(crate::parse(
            "quiz(q, \"pick\", \"layout=stack\"); option(q, \"a\"); option(q, \"b\"); option(q, \"c\"); option(q, \"d\"); option(q, \"e\");"
        ).is_err(), "stack should reject a fifth option before cards overlap");
    }

    #[test]
    fn figure_rejects_incomplete_live_group() {
        let err = crate::parse(
            "point(A, (100,100)); point(B, (300,100)); midpoint(m, A, B); tag(m, fig); figure(fig);"
        ).err().expect("missing live dependencies should error");
        assert!(
            err.msg.contains("source points"),
            "unexpected diagnostic: {}",
            err.msg
        );
    }
}
