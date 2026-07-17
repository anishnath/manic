//! creator — social-video **format** templates for content creators.
//!
//! A layer orthogonal to the domain kits (math/physics/optics add *subjects*;
//! this adds *shapes of video* — Shorts / Reels). A creator sets a reusable
//! profile once and fills slot-based templates. This file ships the profile +
//! footer (`creator` / `socials`); `quiz`/`countdown`/`choices`/`reveal` build
//! on top next.
//!
//! Social icons are DRAWN from primitives (no bundled trademark logos, no
//! downloads) so they render on any template; a creator who wants exact brand
//! logos uses the `image(...)` builtin with their own files.

use crate::easing::Easing;
use crate::lang::diag::Error;
use crate::lang::lower::{resolve_color, Args, Registry};
use crate::primitives::{Align, Counter, Entity, FontKind, Shape};
use crate::scene::{CreatorProfile, PlaybackTrack, QuizData, QuizOpt, QuizReveal, QuizSkin, Scene, SimData};
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TrackSpec, Value};
use macroquad::prelude::{Color, Vec2};

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
                p => prof.platforms.push((p.to_string(), v.to_string())),
            }
        } else if prof.handle.is_empty() {
            prof.handle = tok.to_string();
        }
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
    let at = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(540.0, 1815.0) };
    let accent = prof.accent.unwrap_or(style::MAGENTA);

    // rule above the row
    let mut rule = Entity::new(
        format!("{id}.rule"),
        Shape::Line { to: Vec2::new(at.x + 390.0, at.y - 72.0) },
        Vec2::new(at.x - 390.0, at.y - 72.0),
        style::DIM,
    );
    rule.stroke.width = 2.0;
    rule.tags = vec![id.clone(), format!("{id}.footer")];
    s.add(rule);

    // centre [icons · gap · handle]
    let gap = 58.0;
    let n = prof.platforms.len() as f32;
    let handle_w = prof.handle.chars().count() as f32 * 16.0;
    let icons_w = if n > 0.0 { n * gap } else { 0.0 };
    let total = icons_w + if n > 0.0 && handle_w > 0.0 { 30.0 } else { 0.0 } + handle_w;
    let mut x = at.x - total / 2.0 + 23.0;
    for (k, (plat, _user)) in prof.platforms.iter().enumerate() {
        draw_icon(s, &id, k, plat, Vec2::new(x, at.y), accent);
        x += gap;
    }
    if !prof.handle.is_empty() {
        let hx = at.x - total / 2.0 + icons_w + if n > 0.0 { 30.0 } else { 0.0 } + handle_w / 2.0;
        let mut h = Entity::new(
            format!("{id}.handle"),
            Shape::Text { content: prof.handle.clone(), size: 30.0 },
            Vec2::new(hx, at.y),
            style::DIM,
        );
        h.tags = vec![id.clone(), format!("{id}.footer")];
        s.add(h);
    }
    Ok(())
}

// ---- quiz Short: question + option cards + auto ask→countdown→reveal beat ----

/// `quiz(id, "question")` — start a quiz Short: lays out the (typewriter)
/// question at the top and a countdown widget. Add answers with `option(...)`,
/// then `run(id, [dur])` plays the whole beat. 9:16 layout.
// Shared quiz layout constants (9:16 canvas).
const CARD_W: f32 = 428.0;
const CARD_H: f32 = 122.0;
const CARDS_CY: f32 = 1000.0; // vertical centre of the answer block
const TIMER_CY: f32 = 1450.0; // countdown widget, below the cards
const RING_R: f32 = 64.0;

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
    badge: bool,                 // filled letter-badge chip
    badge_color: Color,
    // correct-answer reveal
    correct_color: Color,
    correct_glow: f32,
}

fn skin_spec(skin: QuizSkin) -> SkinSpec {
    use style::{CYAN, DIM, LIME, MAGENTA};
    match skin {
        QuizSkin::Badge => SkinSpec {
            q_panel: true, q_panel_glow: 0.0, q_panel_edge: None,
            q_kicker: Some("QUESTION"), q_kicker_pill: true, q_rule: false, q_size: 46.0,
            card_fill: true, card_edge: Some(DIM), card_edge_w: 2.0, card_glow: 0.0,
            badge: true, badge_color: CYAN,
            correct_color: LIME, correct_glow: 2.4,
        },
        QuizSkin::Minimal => SkinSpec {
            q_panel: false, q_panel_glow: 0.0, q_panel_edge: None,
            q_kicker: Some("QUESTION"), q_kicker_pill: false, q_rule: true, q_size: 48.0,
            card_fill: false, card_edge: Some(DIM), card_edge_w: 2.0, card_glow: 0.0,
            badge: false, badge_color: CYAN,
            correct_color: LIME, correct_glow: 1.8,
        },
        QuizSkin::Glass => SkinSpec {
            q_panel: true, q_panel_glow: 1.5, q_panel_edge: Some(CYAN),
            q_kicker: None, q_kicker_pill: false, q_rule: false, q_size: 46.0,
            card_fill: true, card_edge: Some(CYAN), card_edge_w: 2.5, card_glow: 1.3,
            badge: true, badge_color: MAGENTA,
            correct_color: LIME, correct_glow: 3.0,
        },
        QuizSkin::Plain => SkinSpec {
            q_panel: false, q_panel_glow: 0.0, q_panel_edge: None,
            q_kicker: None, q_kicker_pill: false, q_rule: false, q_size: 44.0,
            card_fill: true, card_edge: None, card_edge_w: 0.0, card_glow: 0.0,
            badge: false, badge_color: CYAN,
            correct_color: LIME, correct_glow: 2.0,
        },
    }
}

/// Map a skin name (with manic-flavoured aliases) to a [`QuizSkin`].
fn parse_skin(name: &str) -> Option<QuizSkin> {
    match name.trim().to_lowercase().as_str() {
        "badge" | "card" | "cards" => Some(QuizSkin::Badge),
        "minimal" | "editorial" | "clean" => Some(QuizSkin::Minimal),
        "glass" | "neon" | "reels" => Some(QuizSkin::Glass),
        "plain" | "flat" | "basic" => Some(QuizSkin::Plain),
        _ => None,
    }
}

/// Parse the optional quiz spec (3rd arg): a space-separated, order-free mix of
/// a reveal word (`type`/`fade`/…) and a skin word (`badge`/`glass`/…).
fn parse_quiz_spec(a: &Args) -> Result<(QuizReveal, QuizSkin), Error> {
    let mut reveal = QuizReveal::Type;
    let mut skin = QuizSkin::default();
    if let Some(spec) = a.opt_text(2)? {
        for tok in spec.split_whitespace() {
            if let Some(r) = parse_reveal(tok) {
                reveal = r;
            } else if let Some(k) = parse_skin(tok) {
                skin = k;
            } else {
                return Err(Error::new(
                    format!(
                        "unknown quiz style {tok:?} — reveals: type/fade/rise/pop/cut · \
                         skins: badge/minimal/glass/plain"
                    ),
                    a.span_of(2),
                ));
            }
        }
    }
    Ok((reveal, skin))
}

/// `quiz(id, "question", ["style"])` — start a quiz Short. `style` is an order-free
/// mix of a card SKIN (`badge` default, `minimal`, `glass`, `plain`) and a question
/// REVEAL (`type` default, `fade`, `rise`, `pop`, `cut`). Add answers with
/// `option(...)`, then `run(id, [dur])` plays the whole beat. 9:16 layout.
pub fn c_quiz(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let q = a.text(1)?;
    let (reveal, skin) = parse_quiz_spec(a)?;
    let sp = skin_spec(skin);
    let tag = |extra: String| vec![id.clone(), format!("{id}.parts"), extra];
    let qcx = 540.0;

    // ---- question header (panel · kicker · accent rule) ----
    if sp.q_panel {
        let mut p = Entity::new(format!("{id}.qpanel"), Shape::Rect { w: 940.0, h: 250.0 }, Vec2::new(qcx, 300.0), style::PANEL);
        p.stroke.fill = true;
        match sp.q_panel_edge {
            Some(edge) => {
                p.stroke.outline = true;
                p.stroke.outline_color = Some(edge);
                p.stroke.width = 3.0;
            }
            None => p.stroke.outline = false,
        }
        p.glow = sp.q_panel_glow;
        p.tags = tag(format!("{id}.q"));
        s.add(p);
    }
    if let Some(kick) = sp.q_kicker {
        if sp.q_kicker_pill {
            let pc = Vec2::new(qcx - 318.0, 205.0);
            let mut pill = Entity::new(format!("{id}.qkbg"), Shape::Rect { w: 224.0, h: 54.0 }, pc, style::MAGENTA);
            pill.stroke.fill = true;
            pill.stroke.outline = false;
            pill.tags = tag(format!("{id}.q"));
            s.add(pill);
            let mut kt = Entity::new(format!("{id}.qk"), Shape::Text { content: kick.into(), size: 24.0 }, pc, style::VOID);
            kt.font = FontKind::MonoBold;
            kt.tags = tag(format!("{id}.q"));
            s.add(kt);
        } else {
            let mut kt = Entity::new(format!("{id}.qk"), Shape::Text { content: kick.into(), size: 26.0 }, Vec2::new(qcx, 196.0), style::CYAN);
            kt.font = FontKind::MonoBold;
            kt.tags = tag(format!("{id}.q"));
            s.add(kt);
        }
    }
    if sp.q_rule {
        let mut rule = Entity::new(format!("{id}.qrule"), Shape::Rect { w: 520.0, h: 5.0 }, Vec2::new(qcx, 244.0), style::CYAN);
        rule.stroke.fill = true;
        rule.stroke.outline = false;
        rule.tags = tag(format!("{id}.q"));
        s.add(rule);
    }
    // question text — heavy display font, wrapped, title-safe. Initial state
    // depends on the reveal: `type` starts undrawn; the others start drawn but
    // hidden (opacity/offset/scale) so the beat can bring them in.
    let rest = Vec2::new(qcx, if sp.q_rule { 330.0 } else { 300.0 });
    let mut qe = Entity::new(format!("{id}.q"), Shape::Text { content: q, size: sp.q_size }, rest, style::FG);
    qe.font = FontKind::Display;
    qe.wrap = Some(840.0);
    qe.tags = vec![id.clone(), format!("{id}.parts")];
    match reveal {
        QuizReveal::Type => qe.trace = 0.0,
        QuizReveal::Fade => qe.opacity = 0.0,
        QuizReveal::Rise => {
            qe.opacity = 0.0;
            qe.pos = Vec2::new(rest.x, rest.y + 50.0);
        }
        QuizReveal::Pop => {
            qe.opacity = 0.0;
            qe.scale = 0.7;
        }
        QuizReveal::Cut => {}
    }
    s.add(qe);

    // ---- countdown widget: a faint static TRACK + the draining ring + digit ----
    let tc = Vec2::new(qcx, TIMER_CY);
    let arc = || Shape::Arc { r: RING_R, inner: 0.0, start: -90.0, sweep: 360.0 };
    let mut track = Entity::new(format!("{id}.track"), arc(), tc, style::DIM);
    track.stroke.fill = false;
    track.stroke.outline = true;
    track.stroke.width = 6.0;
    track.opacity = 0.30;
    track.tags = tag(format!("{id}.ring"));
    s.add(track);
    let mut ring = Entity::new(format!("{id}.ring"), arc(), tc, sp.badge_color);
    ring.stroke.fill = false;
    ring.stroke.outline = true;
    ring.stroke.width = 8.0;
    ring.glow = 1.2;
    ring.opacity = 0.0;
    ring.tags = vec![id.clone(), format!("{id}.parts")];
    s.add(ring);
    let counter = Counter { value: 5.0, decimals: 0, prefix: "".into(), suffix: "".into() };
    let mut timer = Entity::new(format!("{id}.timer"), Shape::Text { content: counter.render(), size: 58.0 }, tc, style::FG);
    timer.font = FontKind::MonoBold;
    timer.counter = Some(counter);
    timer.opacity = 0.0;
    timer.tags = vec![id.clone(), format!("{id}.parts")];
    s.add(timer);

    s.quizzes.insert(id, QuizData { reveal, skin, ..QuizData::default() });
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
fn slot(n: usize, i: usize) -> Vec2 {
    let (w, h) = (CARD_W, CARD_H);
    if n <= 3 {
        let gap = 26.0;
        let total = n as f32 * h + (n as f32 - 1.0) * gap;
        let y0 = CARDS_CY - total / 2.0 + h / 2.0;
        Vec2::new(540.0, y0 + i as f32 * (h + gap))
    } else {
        let (gx, gy) = (40.0, 26.0);
        let rows = (n + 1) / 2;
        let (col, row) = (i % 2, i / 2);
        let x = if col == 0 { 540.0 - (w + gx) / 2.0 } else { 540.0 + (w + gx) / 2.0 };
        let total = rows as f32 * h + (rows as f32 - 1.0) * gy;
        let y0 = CARDS_CY - total / 2.0 + h / 2.0;
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
    let sp = skin_spec(qd.skin);
    let neutral = Vec2::new(540.0, CARDS_CY); // `run` moves everything to its slot
    let tags = || vec![id.clone(), format!("{id}.parts")];
    let letter = (b'A' + i as u8) as char;
    let mut parts: Vec<(String, Vec2)> = Vec::new();

    // card
    let card_id = format!("{id}.c{i}");
    let mut card = Entity::new(card_id.clone(), Shape::Rect { w: CARD_W, h: CARD_H }, neutral, style::PANEL);
    card.stroke.fill = sp.card_fill;
    match sp.card_edge {
        Some(edge) => {
            card.stroke.outline = true;
            card.stroke.outline_color = Some(edge);
            card.stroke.width = sp.card_edge_w;
        }
        None => card.stroke.outline = false,
    }
    card.glow = sp.card_glow;
    card.opacity = 0.0;
    card.tags = tags();
    s.add(card);
    parts.push((card_id.clone(), Vec2::ZERO));

    // correct-answer highlight (a soft fill tint + a glowing outline), created
    // now but hidden — the beat reveals it. Placed BEHIND the text so the answer
    // stays readable over the tint.
    if correct {
        let mut fillh = Entity::new(format!("{id}.hlfill"), Shape::Rect { w: CARD_W, h: CARD_H }, neutral, sp.correct_color);
        fillh.stroke.fill = true;
        fillh.stroke.outline = false;
        fillh.opacity = 0.0;
        fillh.tags = tags();
        s.add(fillh);
        let mut hl = Entity::new(format!("{id}.hl"), Shape::Rect { w: CARD_W + 10.0, h: CARD_H + 10.0 }, neutral, sp.correct_color);
        hl.stroke.fill = false;
        hl.stroke.outline = true;
        hl.stroke.width = 5.0;
        hl.glow = sp.correct_glow;
        hl.opacity = 0.0;
        hl.tags = tags();
        s.add(hl);
    }

    // letter badge (filled chip) + the letter itself
    let badge_off = Vec2::new(-CARD_W / 2.0 + 54.0, 0.0);
    if sp.badge {
        let bid = format!("{id}.b{i}");
        let mut b = Entity::new(bid.clone(), Shape::Circle { r: 33.0 }, neutral + badge_off, sp.badge_color);
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
            let mut bw = Entity::new(format!("{id}.bwin"), Shape::Circle { r: 33.0 }, neutral + badge_off, sp.correct_color);
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
    let mut le = Entity::new(lid.clone(), Shape::Text { content: letter.to_string(), size: 34.0 }, neutral + badge_off, letter_color);
    le.font = FontKind::MonoBold;
    le.opacity = 0.0;
    le.tags = tags();
    s.add(le);
    parts.push((lid, badge_off));

    // answer text — left-aligned, starting just after the badge
    let text_off = Vec2::new(-CARD_W / 2.0 + 108.0, 0.0);
    let text_id = format!("{id}.t{i}");
    let mut te = Entity::new(text_id.clone(), Shape::Text { content: txt, size: 34.0 }, neutral + text_off, style::FG);
    te.align = Align::Left;
    te.font = FontKind::MonoBold;
    te.wrap = Some(CARD_W - 132.0);
    te.opacity = 0.0;
    te.tags = tags();
    s.add(te);
    parts.push((text_id.clone(), text_off));

    // a DRAWN check-mark as ONE polyline (moves cleanly via Pos, draws-on via
    // trace — no glyph dependency), on the card's right edge
    if correct {
        let ck_off = Vec2::new(CARD_W / 2.0 - 40.0, 0.0);
        let mut ck = Entity::new(
            format!("{id}.check"),
            Shape::Polyline { pts: vec![Vec2::new(-13.0, 0.0), Vec2::new(-4.0, 13.0), Vec2::new(17.0, -15.0)] },
            neutral + ck_off,
            sp.correct_color,
        );
        ck.stroke.width = 7.0;
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
    let sp = skin_spec(qd.skin);
    let dur = dur.unwrap_or(12.0).max(4.0);
    let f = |v: f32| TargetValue::Abs(Value::F(v));
    let pos = |p: Vec2| TargetValue::Abs(Value::V(p));
    let mut t: Vec<TrackSpec> = Vec::new();
    let mut push = |id: String, prop: Prop, target: TargetValue, start: f32, d: f32, e: Easing| {
        t.push(TrackSpec { id, prop, target, start, dur: d, easing: e });
    };
    let n = qd.options.len();
    // 1) question reveal — style chosen at `quiz(...)` time (default typewriter)
    let qid = format!("{id}.q");
    let qrest = Vec2::new(540.0, 300.0);
    match qd.reveal {
        QuizReveal::Type => {
            push(qid, Prop::Trace, f(1.0), 0.0, 0.12 * dur, Easing::Linear);
        }
        QuizReveal::Fade => {
            push(qid, Prop::Opacity, f(1.0), 0.0, 0.10 * dur, Easing::OutCubic);
        }
        QuizReveal::Rise => {
            push(qid.clone(), Prop::Opacity, f(1.0), 0.0, 0.10 * dur, Easing::OutCubic);
            push(qid, Prop::Pos, pos(qrest), 0.0, 0.12 * dur, Easing::OutCubic);
        }
        QuizReveal::Pop => {
            push(qid.clone(), Prop::Opacity, f(1.0), 0.0, 0.06 * dur, Easing::OutCubic);
            push(qid.clone(), Prop::Scale, f(1.06), 0.0, 0.09 * dur, Easing::OutCubic);
            push(qid, Prop::Scale, f(1.0), 0.09 * dur, 0.06 * dur, Easing::OutBack);
        }
        QuizReveal::Cut => {} // already visible on frame 0
    }
    // 2) options: each card's PARTS (card, badge, letter, text) slide up from
    //    just below their slot + fade in, staggered. Every part keeps its local
    //    offset from the card centre so the card moves as a rigid unit.
    for (i, opt) in qd.options.iter().enumerate() {
        let sl = slot(n, i);
        let st = (0.18 + i as f32 * 0.06) * dur;
        let d = 0.07 * dur;
        for (pid, off) in &opt.parts {
            let rest = sl + *off;
            let below = rest + Vec2::new(0.0, 55.0);
            push(pid.clone(), Prop::Pos, pos(below), 0.0, 0.02 * dur, Easing::Linear);
            push(pid.clone(), Prop::Pos, pos(rest), st, d, Easing::OutCubic);
            push(pid.clone(), Prop::Opacity, f(1.0), st, d, Easing::OutCubic);
        }
        // park the correct-answer highlight + check at this slot (invisible)
        if opt.correct {
            for part in [format!("{id}.hlfill"), format!("{id}.hl")] {
                push(part, Prop::Pos, pos(sl), 0.0, 0.02 * dur, Easing::Linear);
            }
            push(format!("{id}.check"), Prop::Pos, pos(sl + Vec2::new(CARD_W / 2.0 - 40.0, 0.0)), 0.0, 0.02 * dur, Easing::Linear);
            if sp.badge {
                let bwin_rest = sl + Vec2::new(-CARD_W / 2.0 + 54.0, 0.0);
                push(format!("{id}.bwin"), Prop::Pos, pos(bwin_rest), 0.0, 0.02 * dur, Easing::Linear);
            }
        }
    }
    // 3) countdown widget in, ring DRAINS (trace 1→0) + digit 5 → 0
    let cd = 0.46 * dur;
    push(format!("{id}.ring"), Prop::Opacity, f(1.0), cd, 0.03 * dur, Easing::OutCubic);
    push(format!("{id}.ring"), Prop::Trace, f(0.0), 0.48 * dur, 0.40 * dur, Easing::Linear);
    push(format!("{id}.timer"), Prop::Opacity, f(1.0), cd, 0.03 * dur, Easing::OutCubic);
    push(format!("{id}.timer"), Prop::Value, f(0.0), 0.48 * dur, 0.40 * dur, Easing::Linear);
    // 4) reveal: correct card lights up (soft fill tint + glowing outline pop +
    //    drawn check); the rest dim back
    let rv = 0.90 * dur;
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
    Ok(Clip { tracks: t, events: vec![], dur })
}

// ---- safezone : platform-UI guide for vertical video ----

/// `safezone(id, [inset])` — draw a faint guide rectangle marking the content-safe
/// area of a 9:16 Short (clear of the top clock and the bottom caption/action bar).
/// A composing aid: `hidden(id)` it (or delete the line) for the final render.
pub fn c_safezone(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let inset = a.opt_num(1)?.unwrap_or(54.0).max(0.0);
    let (top, bot) = (110.0, 210.0);
    let (w, h) = (1080.0 - 2.0 * inset, 1920.0 - top - bot);
    let mut r = Entity::new(id.clone(), Shape::Rect { w, h }, Vec2::new(540.0, top + h / 2.0), style::DIM);
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
    let at = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(540.0, 960.0) };
    let secs = a.opt_num(2)?.unwrap_or(5.0).max(1.0);
    let mut ring = Entity::new(
        format!("{id}.ring"),
        Shape::Arc { r: 62.0, inner: 0.0, start: -90.0, sweep: 360.0 },
        at,
        style::CYAN,
    );
    ring.stroke.fill = false;
    ring.stroke.outline = true;
    ring.stroke.width = 6.0;
    ring.tags = vec![id.clone(), format!("{id}.parts")];
    s.add(ring);
    let counter = Counter { value: secs, decimals: 0, prefix: "".into(), suffix: "".into() };
    let mut timer = Entity::new(format!("{id}.timer"), Shape::Text { content: counter.render(), size: 62.0 }, at, style::FG);
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

/// Union bounding box of a group's entities (approximate: uses each entity's
/// position plus its shape's own extent).
fn group_bbox(s: &Scene, ids: &[usize]) -> Option<(Vec2, Vec2)> {
    let (mut lo, mut hi) = (Vec2::new(f32::MAX, f32::MAX), Vec2::new(f32::MIN, f32::MIN));
    let mut any = false;
    for &i in ids {
        let e = &s.entities[i];
        let mut acc = |p: Vec2| {
            lo = lo.min(p);
            hi = hi.max(p);
        };
        acc(e.pos);
        match &e.shape {
            Shape::Circle { r } => { acc(e.pos + Vec2::new(*r, *r)); acc(e.pos - Vec2::new(*r, *r)); }
            Shape::Arc { r, .. } => { acc(e.pos + Vec2::new(*r, *r)); acc(e.pos - Vec2::new(*r, *r)); }
            Shape::Rect { w, h } => { acc(e.pos + Vec2::new(w / 2.0, h / 2.0)); acc(e.pos - Vec2::new(w / 2.0, h / 2.0)); }
            Shape::Line { to } => acc(*to),
            Shape::Arrow { to } => acc(*to),
            Shape::Polyline { pts } | Shape::Polygon { pts } => { for p in pts { acc(e.pos + *p); } }
            _ => {}
        }
        any = true;
    }
    any.then_some((lo, hi))
}

/// `figure(target, [center], [size])` — scale + translate the group `target` to
/// FIT the figure zone (default centred above the quiz options), so any entity /
/// kit sim can drop in as a Short's illustration without hand-placing it.
pub fn c_figure(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let target = a.ident(0)?;
    let zc = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(540.0, 640.0) };
    let zs = if a.len() >= 3 { a.pair(2)? } else { Vec2::new(840.0, 470.0) };
    // gather the group: entities tagged `target` (or the entity itself)
    let ids: Vec<usize> = (0..s.entities.len())
        .filter(|&i| s.entities[i].id == target || s.entities[i].tags.iter().any(|t| t == &target))
        .collect();
    if ids.is_empty() {
        return Err(Error::new(format!("figure: no entity or group `{target}`"), a.span_of(0)));
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
        match &mut e.shape {
            Shape::Circle { r } => *r *= sc,
            Shape::Arc { r, inner, .. } => { *r *= sc; *inner *= sc; }
            Shape::Rect { w, h } => { *w *= sc; *h *= sc; }
            Shape::Line { to } | Shape::Arrow { to } => *to = zc + (*to - bc) * sc,
            Shape::Polyline { pts } | Shape::Polygon { pts } => { for p in pts.iter_mut() { *p *= sc; } }
            Shape::Text { size, .. } => *size *= sc.clamp(0.6, 1.6),
            _ => {}
        }
    }
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
}
