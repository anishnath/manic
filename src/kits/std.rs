//! The **std kit**: the base vocabulary every manic program has. Generic
//! shapes and animation verbs — no domain knowledge. Domain kits (math, algo)
//! layer on top of this.
//!
//! Constructors declare/modify the cast at t=0; verbs produce timeline clips.

use macroquad::prelude::Vec2;

use crate::animate::act;
use crate::easing::Easing;
use crate::geom;
use crate::lang::diag::Error;
use crate::lang::lower::{apply_dur_ease, resolve_color, resolve_easing, Args, Registry};
use crate::primitives::{Entity, FontKind, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TrackSpec, Value};

fn neon_stroke() -> StrokeStyle {
    StrokeStyle {
        fill: true,
        outline: true,
        width: 2.5,
        outline_color: Some(style::CYAN),
    }
}

fn ent_mut<'a>(scene: &'a mut Scene, a: &Args) -> Result<&'a mut Entity, Error> {
    let id = a.ident(0)?;
    let span = a.span_of(0);
    if scene.get(&id).is_some() {
        return Ok(scene.get_mut(&id).unwrap());
    }
    // A clear message when a 2D-only modifier is aimed at a 3D entity (a common
    // slip — `hue`/`stroke`/`glow`/`size`/… don't apply to 3D shapes).
    if scene.get_3d(&id).is_some() {
        return Err(Error::new(
            format!(
                "`{}` is a 2D-only modifier — it can't address the 3D entity `{id}`. \
                 3D entities take: `color`, `opacity`, `hidden`, `untraced`, `tag`, \
                 `thick`; verbs `move3`/`shift3`/`rotate3`/`grow3`/`orbit3`, \
                 `show`/`fade`/`draw`/`flash`/`pulse`/`scale`, `to`, and `morph3`",
                a.name
            ),
            span,
        ));
    }
    Err(Error::new(format!("no entity named `{id}`"), span))
}

// ---- constructors ---------------------------------------------------------

fn c_text(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let pos = a.pair(1)?;
    let txt = a.text(2)?;
    s.add(Entity::new(
        id,
        Shape::Text {
            content: txt,
            size: 28.0,
        },
        pos,
        style::FG,
    ));
    Ok(())
}

/// How many points a morph outline is sampled to (both shapes match this).
const MORPH_N: usize = 96;

/// Resample a point list to exactly `n` points, evenly by arc length.
fn resample(pts: &[Vec2], n: usize, closed: bool) -> Vec<Vec2> {
    let mut poly = pts.to_vec();
    if closed && poly.len() > 1 && poly[0] != poly[poly.len() - 1] {
        poly.push(poly[0]);
    }
    if poly.len() < 2 {
        return vec![poly.first().copied().unwrap_or(Vec2::ZERO); n];
    }
    let mut cum = vec![0.0f32];
    for w in poly.windows(2) {
        cum.push(cum.last().unwrap() + (w[1] - w[0]).length());
    }
    let total = *cum.last().unwrap();
    if total < 1e-6 {
        return vec![poly[0]; n];
    }
    (0..n)
        .map(|k| {
            let d = total * k as f32 / n as f32;
            let mut i = 0;
            while i + 1 < cum.len() && cum[i + 1] < d {
                i += 1;
            }
            let seg = cum[i + 1] - cum[i];
            let t = if seg > 1e-6 { (d - cum[i]) / seg } else { 0.0 };
            poly[i] + (poly[i + 1] - poly[i]) * t
        })
        .collect()
}

/// Sample an entity's outline to `n` points (for a morph).
fn sample_outline(e: &Entity, n: usize) -> Vec<Vec2> {
    use std::f32::consts::TAU;
    match &e.shape {
        Shape::Circle { r } => (0..n)
            .map(|k| {
                let a = TAU * k as f32 / n as f32;
                e.pos + Vec2::new(r * a.cos(), r * a.sin())
            })
            .collect(),
        Shape::Rect { w, h } => {
            let (hw, hh) = (w / 2.0, h / 2.0);
            let corners = [
                e.pos + Vec2::new(hw, -hh),
                e.pos + Vec2::new(hw, hh),
                e.pos + Vec2::new(-hw, hh),
                e.pos + Vec2::new(-hw, -hh),
            ];
            resample(&corners, n, true)
        }
        Shape::Polyline { pts } => resample(pts, n, false),
        Shape::Polygon { pts, .. } => resample(pts, n, true),
        Shape::Line { to } | Shape::Arrow { to } | Shape::Curve { to, .. } => {
            resample(&[e.pos, *to], n, false)
        }
        _ => vec![e.pos; n], // text / arc / region: degenerate, holds a point
    }
}

/// `caption(id, "some words", (cx,cy), [size], [color])` — lay out the words in
/// a centred row as `{id}.w0`, `{id}.w1`, … (tagged `{id}.words`). Animate them
/// in sequence with `karaoke` / `wordpop`, or address the group by tag.
/// Widths use the monospace advance (~0.6 em), so no render-time measuring.
fn c_caption(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let text = a.text(1)?;
    let center = a.pair(2)?;
    let size = a.opt_num(3)?.unwrap_or(40.0);
    let color = if a.len() > 4 {
        resolve_color(&a.ident(4)?, a.span_of(4))?
    } else {
        style::FG
    };
    let advance = size * 0.6; // IBM Plex Mono ~0.6 em per glyph
    let words: Vec<&str> = text.split_whitespace().collect();
    let total_chars: usize =
        words.iter().map(|w| w.chars().count()).sum::<usize>() + words.len().saturating_sub(1); // + single spaces
    let x_left = center.x - total_chars as f32 * advance / 2.0;
    let mut char_pos = 0usize;
    for (k, w) in words.iter().enumerate() {
        let len = w.chars().count();
        let x = x_left + (char_pos as f32 + len as f32 / 2.0) * advance;
        let mut e = Entity::new(
            format!("{id}.w{k}"),
            Shape::Text {
                content: w.to_string(),
                size,
            },
            Vec2::new(x, center.y),
            color,
        );
        e.font = FontKind::MonoBold;
        e.tags = vec![format!("{id}.words")];
        s.add(e);
        char_pos += len + 1;
    }
    Ok(())
}

/// The word ids of a caption, in order (`{id}.w0`, `{id}.w1`, …).
fn caption_words(s: &Scene, id: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut k = 0;
    while s.contains(&format!("{id}.w{k}")) {
        out.push(format!("{id}.w{k}"));
        k += 1;
    }
    out
}

/// `karaoke(id, [delay], [color])` — highlight a caption's words in sequence
/// (lyrics-style), one every `delay` seconds.
fn v_karaoke(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let delay = a.opt_num(1)?.unwrap_or(0.25);
    let color = if a.len() > 2 {
        resolve_color(&a.ident(2)?, a.span_of(2))?
    } else {
        style::LIME
    };
    let words = caption_words(s, &id);
    if words.is_empty() {
        return Err(Error::new(
            format!("no caption words for `{id}` — call `caption(...)` first"),
            a.span_of(0),
        ));
    }
    let tracks = words
        .iter()
        .enumerate()
        .map(|(k, wid)| TrackSpec {
            id: wid.clone(),
            prop: Prop::Color,
            target: TargetValue::Abs(Value::C(color)),
            start: k as f32 * delay,
            dur: 0.25,
            easing: Easing::OutQuad,
        })
        .collect();
    Ok(Clip {
        dur: (words.len().saturating_sub(1)) as f32 * delay + 0.25,
        tracks,
        events: Vec::new(),
    })
}

/// `wordpop(id, [delay])` — reveal a caption's words one at a time with a pop
/// (TikTok-caption style). Hide them first (`hidden(id.words)`) for the reveal.
fn v_wordpop(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let delay = a.opt_num(1)?.unwrap_or(0.12);
    let words = caption_words(s, &id);
    if words.is_empty() {
        return Err(Error::new(
            format!("no caption words for `{id}` — call `caption(...)` first"),
            a.span_of(0),
        ));
    }
    let mut tracks = Vec::new();
    for (k, wid) in words.iter().enumerate() {
        let t0 = k as f32 * delay;
        tracks.push(TrackSpec {
            id: wid.clone(),
            prop: Prop::Opacity,
            target: TargetValue::Abs(Value::F(1.0)),
            start: t0,
            dur: 0.16,
            easing: Easing::OutQuad,
        });
        tracks.push(TrackSpec {
            id: wid.clone(),
            prop: Prop::Scale,
            target: TargetValue::Abs(Value::F(1.35)),
            start: t0,
            dur: 0.12,
            easing: Easing::OutQuad,
        });
        tracks.push(TrackSpec {
            id: wid.clone(),
            prop: Prop::Scale,
            target: TargetValue::Abs(Value::F(1.0)),
            start: t0 + 0.12,
            dur: 0.22,
            easing: Easing::OutBack,
        });
    }
    Ok(Clip {
        dur: (words.len().saturating_sub(1)) as f32 * delay + 0.34,
        tracks,
        events: Vec::new(),
    })
}

/// `copy(new_id, src)` — duplicate an existing entity under a new id (standalone:
/// the copy inherits the source's shape/style/position but not its group tags).
/// Enables Manim's `TransformFromCopy`: `copy(c, a)` then morph/move `c`.
fn c_copy(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let newid = a.ident(0)?;
    let srcid = a.ident(1)?;
    let mut e = s
        .get(&srcid)
        .ok_or_else(|| Error::new(format!("no entity named `{srcid}`"), a.span_of(1)))?
        .clone();
    e.id = newid;
    e.tags.clear();
    s.add(e);
    Ok(())
}

/// `morph(a, b, [spin_deg])` — set `a` up to morph into `b`'s outline. Samples
/// both outlines now; animate with `to(a, morph, 1, dur)`. `a` becomes a stroked
/// polyline. Optional `spin_deg` winds the blend (clockwise if positive) —
/// Manim's Clockwise/CounterclockwiseTransform.
fn c_morph(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let ida = a.ident(0)?;
    let idb = a.ident(1)?;
    let spin = a.opt_num(2)?.unwrap_or(0.0);
    let mut from = {
        let ea = s
            .get(&ida)
            .ok_or_else(|| Error::new(format!("no entity named `{ida}`"), a.span_of(0)))?;
        sample_outline(ea, MORPH_N)
    };
    let mut to = {
        let eb = s
            .get(&idb)
            .ok_or_else(|| Error::new(format!("no entity named `{idb}`"), a.span_of(1)))?;
        sample_outline(eb, MORPH_N)
    };
    // close the loop so the outline has no gap
    from.push(from[0]);
    to.push(to[0]);
    let ea = s.get_mut(&ida).unwrap();
    ea.shape = Shape::Polyline { pts: from.clone() };
    ea.pos = Vec2::ZERO; // polyline points are absolute (like geo shapes)
    ea.stroke.fill = false;
    ea.stroke.outline = true;
    ea.morph = Some((from, to, spin));
    Ok(())
}

/// `counter(id, (x,y), value, [decimals], ["prefix"], ["suffix"])` — a text
/// showing a number, animatable with `to(id, value, target)` so it counts
/// live. Defaults: 0 decimals, empty prefix/suffix.
fn c_counter(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let pos = a.pair(1)?;
    let value = a.num(2)?;
    let decimals = a.opt_num(3)?.unwrap_or(0.0).max(0.0) as u8;
    let prefix = if a.len() > 4 {
        a.text(4)?
    } else {
        String::new()
    };
    let suffix = if a.len() > 5 {
        a.text(5)?
    } else {
        String::new()
    };
    let counter = crate::primitives::Counter {
        value,
        decimals,
        prefix,
        suffix,
    };
    let mut e = Entity::new(
        id,
        Shape::Text {
            content: counter.render(),
            size: 28.0,
        },
        pos,
        style::FG,
    );
    e.counter = Some(counter);
    s.add(e);
    Ok(())
}

fn c_dot(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let pos = a.pair(1)?;
    let r = a.opt_num(2)?.unwrap_or(6.0);
    let mut e = Entity::new(id, Shape::Circle { r }, pos, style::CYAN);
    e.stroke = StrokeStyle {
        fill: true,
        outline: false,
        ..Default::default()
    };
    s.add(e);
    Ok(())
}

fn c_circle(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let pos = a.pair(1)?;
    let r = a.num(2)?;
    let mut e = Entity::new(id, Shape::Circle { r }, pos, style::PANEL);
    e.stroke = neon_stroke();
    s.add(e);
    Ok(())
}

fn c_rect(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let pos = a.pair(1)?;
    let (w, h) = (a.num(2)?, a.num(3)?);
    let mut e = Entity::new(id, Shape::Rect { w, h }, pos, style::PANEL);
    e.stroke = neon_stroke();
    s.add(e);
    Ok(())
}

fn c_line(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let from = a.pair(1)?;
    let to = a.pair(2)?;
    s.add(Entity::new(id, Shape::Line { to }, from, style::FG));
    Ok(())
}

/// `polygon(id, (x1,y1), (x2,y2), (x3,y3), …, [color])` — a filled polygon through
/// the given points (screen coordinates; ≥ 3). A trailing colour word is optional.
/// Filled with a matching outline; drop the opacity (`opacity(id, 0.3)`) for a
/// translucent region, or `outline(id)` for edges only.
fn c_polygon(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    // Collect points from index 1; a non-pair arg (the optional colour) ends the list.
    let mut pts = Vec::new();
    let mut i = 1;
    while i < a.len() {
        match a.pair(i) {
            Ok(p) => {
                pts.push(p);
                i += 1;
            }
            Err(_) => break,
        }
    }
    if pts.len() < 3 {
        return Err(Error::new(
            "polygon needs at least 3 points".to_string(),
            a.span_of(0),
        ));
    }
    let color = if i < a.len() {
        resolve_color(&a.ident(i)?, a.span_of(i))?
    } else {
        style::CYAN
    };
    let mut e = Entity::new(id.clone(), Shape::Polygon { pts }, Vec2::ZERO, color);
    e.stroke.fill = true;
    e.stroke.outline = true;
    e.stroke.outline_color = Some(color);
    e.tags.push(id);
    s.add(e);
    Ok(())
}

fn c_arrow(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let from = a.pair(1)?;
    let to = a.pair(2)?;
    s.add(Entity::new(id, Shape::Arrow { to }, from, style::FG));
    Ok(())
}

/// Sample a quadratic Bézier `a`→(control `c`)→`b` into `n + 1` points.
fn quad(a: Vec2, c: Vec2, b: Vec2, n: usize) -> Vec<Vec2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            a * (u * u) + c * (2.0 * u * t) + b * (t * t)
        })
        .collect()
}

/// A curly-brace polyline spanning `p1`→`p2`, bulging `depth` px to one side
/// (negative flips the side). Returns `(points, tip)` — `tip` is the central
/// cusp, a natural anchor for a label. Two smooth quadratic-Bézier halves meet
/// at the cusp (the classic SVG-brace construction).
fn brace_path(p1: Vec2, p2: Vec2, depth: f32) -> (Vec<Vec2>, Vec2) {
    let d = p1 - p2;
    let len = d.length().max(1e-3);
    let u = d / len; // unit vector p2 -> p1
    let perp = Vec2::new(u.y, -u.x); // outward normal
    let w = depth;
    let q = 0.6;
    let along = |frac: f32| p1 - u * (frac * len);

    let c1 = p1 + perp * (q * w);
    let e1 = along(0.25) + perp * ((1.0 - q) * w);
    let tip = along(0.5) + perp * w;
    let c3 = p2 + perp * (q * w);
    let e2 = along(0.75) + perp * ((1.0 - q) * w);

    let mut pts = quad(p1, c1, e1, 12);
    pts.extend(quad(e1, e1 * 2.0 - c1, tip, 12)); // smooth ("T") continuation
    let mut right = quad(p2, c3, e2, 12);
    right.extend(quad(e2, e2 * 2.0 - c3, tip, 12));
    right.reverse(); // tip -> ... -> p2
    pts.extend(right);
    (pts, tip)
}

fn brace_style() -> StrokeStyle {
    StrokeStyle {
        fill: false,
        outline: true,
        width: 3.0,
        outline_color: Some(style::FG),
    }
}

/// `brace(id, (x1,y1), (x2,y2), [depth])` — a curly brace spanning the two
/// points, bulging `depth` px to one side (default 22; negative flips it).
fn c_brace(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let p1 = a.pair(1)?;
    let p2 = a.pair(2)?;
    let depth = a.opt_num(3)?.unwrap_or(22.0);
    let (pts, _) = brace_path(p1, p2, depth);
    let mut e = Entity::new(id.clone(), Shape::Polyline { pts }, Vec2::ZERO, style::FG);
    e.stroke = brace_style();
    e.tags.push(id);
    s.add(e);
    Ok(())
}

/// `bracelabel(id, (x1,y1), (x2,y2), "text", [depth])` (alias `bracetext`) — a
/// brace with a text label centred just beyond its cusp. Child `{id}.label`.
fn c_bracelabel(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let p1 = a.pair(1)?;
    let p2 = a.pair(2)?;
    let text = a.text(3)?;
    let depth = a.opt_num(4)?.unwrap_or(22.0);
    let (pts, tip) = brace_path(p1, p2, depth);
    let mut e = Entity::new(id.clone(), Shape::Polyline { pts }, Vec2::ZERO, style::FG);
    e.stroke = brace_style();
    e.tags.push(id.clone());
    s.add(e);

    // label sits just beyond the cusp, along the same outward normal
    let u = (p1 - p2) / (p1 - p2).length().max(1e-3);
    let perp = Vec2::new(u.y, -u.x);
    let sign = if depth >= 0.0 { 1.0 } else { -1.0 };
    let lp = tip + perp * (sign * 24.0);
    let mut t = Entity::new(
        format!("{id}.label"),
        Shape::Text {
            content: text,
            size: 24.0,
        },
        lp,
        style::FG,
    );
    t.tags.push(id);
    s.add(t);
    Ok(())
}

// ---- modifiers ------------------------------------------------------------

fn m_hidden(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    if let Some(e) = s.get_mut(&id) {
        e.opacity = 0.0;
    } else if let Some(e) = s.get_3d_mut(&id) {
        e.opacity = 0.0;
    } else {
        return Err(Error::new(format!("no entity named `{id}`"), a.span_of(0)));
    }
    Ok(())
}

fn m_color(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let c = resolve_color(&a.ident(1)?, a.span_of(1))?;
    let id = a.ident(0)?;
    if let Some(e) = s.get_mut(&id) {
        e.color = c;
    } else if let Some(e) = s.get_3d_mut(&id) {
        e.color = c;
    } else {
        return Err(Error::new(format!("no entity named `{id}`"), a.span_of(0)));
    }
    Ok(())
}

/// `hue(id, deg, [sat], [light])` — set the colour from an HSL hue in degrees
/// (sat default 1.0, light default 0.6 for a bright neon). Perfect with a loop:
/// `hue(bar{i}, 360*i/n)` gives each entity its own colour. The angle is also
/// stored so it can be animated (`to(id, hue, deg)`) for colour cycling.
fn m_hue(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let deg = a.num(1)?;
    let sat = a.opt_num(2)?.unwrap_or(1.0).clamp(0.0, 1.0);
    let light = a.opt_num(3)?.unwrap_or(0.6).clamp(0.0, 1.0);
    let c = style::hsl(deg, sat, light);
    let e = ent_mut(s, a)?;
    e.color = c;
    e.hue = Some(deg);
    if e.stroke.outline_color.is_some() {
        e.stroke.outline_color = Some(c);
    }
    Ok(())
}

fn m_outlined(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let e = ent_mut(s, a)?;
    e.stroke.fill = false;
    e.stroke.outline = true;
    Ok(())
}

fn m_filled(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let e = ent_mut(s, a)?;
    e.stroke.fill = true;
    e.stroke.outline = false;
    Ok(())
}

fn m_outline(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let c = resolve_color(&a.ident(1)?, a.span_of(1))?;
    let e = ent_mut(s, a)?;
    e.stroke.outline = true;
    e.stroke.outline_color = Some(c);
    Ok(())
}

fn m_size(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let n = a.num(1)?;
    if let Shape::Text { size, .. } = &mut ent_mut(s, a)?.shape {
        *size = n;
    }
    Ok(())
}

fn m_stroke(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let n = a.num(1)?;
    let id = a.ident(0)?;
    if let Some(e) = s.get_mut(&id) {
        e.stroke.width = n;
    } else if s.get_3d(&id).is_some() {
        return Err(Error::new(
            format!("`stroke` is 2D-only; for a 3D line/arrow/curve use `thick({id}, radius)`"),
            a.span_of(0),
        ));
    } else {
        return Err(Error::new(format!("no entity named `{id}`"), a.span_of(0)));
    }
    Ok(())
}

fn m_glow(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let n = a.num(1)?;
    ent_mut(s, a)?.glow = n;
    Ok(())
}

fn m_z(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let n = a.num(1)?;
    ent_mut(s, a)?.z = n as i32;
    Ok(())
}

fn m_tag(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let tag = a.ident(1)?;
    if let Some(e) = s.get_mut(&id) {
        e.tags.push(tag);
    } else if let Some(e) = s.get_3d_mut(&id) {
        e.tags.push(tag);
    } else {
        return Err(Error::new(format!("no entity named `{id}`"), a.span_of(0)));
    }
    Ok(())
}

fn m_bold(s: &mut Scene, a: &Args) -> Result<(), Error> {
    ent_mut(s, a)?.font = FontKind::MonoBold;
    Ok(())
}

fn m_display(s: &mut Scene, a: &Args) -> Result<(), Error> {
    ent_mut(s, a)?.font = FontKind::Display;
    Ok(())
}

fn m_untraced(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    if let Some(e) = s.get_mut(&id) {
        e.trace = 0.0;
    } else if let Some(e) = s.get_3d_mut(&id) {
        e.trace = 0.0;
    } else {
        return Err(Error::new(format!("no entity named `{id}`"), a.span_of(0)));
    }
    Ok(())
}

/// `cursor(id)` — give a text entity a typewriter cursor (`_`) at the end of its
/// revealed text; pairs with `type`/`trace` for a terminal-prompt look.
fn m_cursor(s: &mut Scene, a: &Args) -> Result<(), Error> {
    ent_mut(s, a)?.type_cursor = true;
    Ok(())
}

fn m_rot(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let deg = a.num(1)?;
    ent_mut(s, a)?.rot = deg;
    Ok(())
}

fn m_opacity(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let n = a.num(1)?;
    let id = a.ident(0)?;
    if let Some(e) = s.get_mut(&id) {
        e.opacity = n;
    } else if let Some(e) = s.get_3d_mut(&id) {
        e.opacity = n;
    } else {
        return Err(Error::new(format!("no entity named `{id}`"), a.span_of(0)));
    }
    Ok(())
}

fn m_label(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let text = a.text(1)?;
    // optional `(dx, dy)` offset so the label sits beside its anchor
    let offset = if a.len() > 2 { a.pair(2)? } else { Vec2::ZERO };
    let (pz, psticky) = {
        let e = s
            .get(&id)
            .ok_or_else(|| Error::new(format!("no entity named `{id}`"), a.span_of(0)))?;
        (e.z, e.sticky)
    };
    let lbl_id = format!("{id}.label");
    if s.contains(&lbl_id) {
        return Err(Error::new(
            format!("`{id}` already has a label"),
            a.name_span,
        ));
    }
    let mut lbl = Entity::new(
        lbl_id,
        Shape::Text {
            content: text,
            size: 24.0,
        },
        Vec2::ZERO,
        style::FG,
    );
    lbl.font = FontKind::MonoBold;
    lbl.z = pz + 1;
    lbl.sticky = psticky;
    lbl.follow = Some((id, offset));
    s.add(lbl);
    Ok(())
}

// ---- verbs ----------------------------------------------------------------

fn v_show(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    Ok(apply_dur_ease(act().fade_in(&id), a, 1)?.into())
}

fn v_fade(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    Ok(apply_dur_ease(act().fade_out(&id), a, 1)?.into())
}

fn v_move(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let to = a.point(1, s)?;
    Ok(apply_dur_ease(act().move_to(&id, to), a, 2)?.into())
}

fn v_shift(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let by = a.pair(1)?;
    Ok(apply_dur_ease(act().move_by(&id, by), a, 2)?.into())
}

fn v_grow(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let to = a.point(1, s)?;
    Ok(apply_dur_ease(act().grow_to(&id, to), a, 2)?.into())
}

fn v_draw(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    Ok(apply_dur_ease(act().trace_in(&id), a, 1)?.into())
}

fn v_erase(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    Ok(apply_dur_ease(act().trace_out(&id), a, 1)?.into())
}

fn v_type(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let mut b = act().type_in(&id);
    if let Some(d) = a.opt_num(1)? {
        b = b.dur(d);
    }
    Ok(b.into())
}

fn v_say(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let text = a.text(1)?;
    let mut b = act().set_text(&id, &text);
    if let Some(d) = a.opt_num(2)? {
        b = b.dur(d);
    }
    Ok(b.into())
}

fn v_recolor(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let c = resolve_color(&a.ident(1)?, a.span_of(1))?;
    let mut b = act().color_to(&id, c);
    if let Some(d) = a.opt_num(2)? {
        b = b.dur(d);
    }
    Ok(b.into())
}

fn v_flash(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let c = if a.len() > 1 {
        resolve_color(&a.ident(1)?, a.span_of(1))?
    } else {
        style::MAGENTA
    };
    Ok(act().highlight(&id, c).into())
}

fn v_pulse(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let mut b = act().pulse(&id);
    if let Some(d) = a.opt_num(1)? {
        b = b.dur(d);
    }
    Ok(b.into())
}

fn v_shake(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let mut b = act().shake(&id);
    if let Some(d) = a.opt_num(1)? {
        b = b.dur(d);
    }
    Ok(b.into())
}

fn v_scale(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let f = a.num(1)?;
    Ok(apply_dur_ease(act().scale_to(&id, f), a, 2)?.into())
}

/// The general escape hatch: `to(id, property, value, [dur], [ease])` animates
/// any single property to a value. Named verbs (`move`, `recolor`, …) are
/// ergonomic shortcuts over the same tracks; this is here for whatever we
/// didn't pre-name, so authors can animate however they like.
fn v_to(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let prop_name = a.ident(1)?;
    let here = a.span_of(0);
    // (property track, target value) — resolved against the 2D scene, or the
    // 3D scene for the shared properties (`move3`/`rotate3`/`grow3` cover 3D
    // position, rotation, and size).
    let (prop, target) = if let Some(cur) = s.get(&id) {
        match prop_name.as_str() {
            // for a graph view (tangent/normal/slope/area), `x` is the moving
            // parameter in the curve's own units — slide it, everything follows
            "x" if cur.graph_view.is_some() => {
                (Prop::PlotX, TargetValue::Abs(Value::F(a.num(2)?)))
            }
            "x" => (
                Prop::Pos,
                TargetValue::Abs(Value::V(Vec2::new(a.num(2)?, cur.pos.y))),
            ),
            "y" => (
                Prop::Pos,
                TargetValue::Abs(Value::V(Vec2::new(cur.pos.x, a.num(2)?))),
            ),
            "opacity" | "alpha" => (Prop::Opacity, TargetValue::Abs(Value::F(a.num(2)?))),
            "scale" => (Prop::Scale, TargetValue::Abs(Value::F(a.num(2)?))),
            "trace" => (Prop::Trace, TargetValue::Abs(Value::F(a.num(2)?))),
            "color" => (
                Prop::Color,
                TargetValue::Abs(Value::C(resolve_color(&a.ident(2)?, a.span_of(2))?)),
            ),
            "angle" | "rot" | "rotation" => (Prop::Rot, TargetValue::Abs(Value::F(a.num(2)?))),
            "hue" => (Prop::Hue, TargetValue::Abs(Value::F(a.num(2)?))),
            "value" | "count" => (Prop::Value, TargetValue::Abs(Value::F(a.num(2)?))),
            "morph" => (Prop::Morph, TargetValue::Abs(Value::F(a.num(2)?))),
            other => {
                return Err(Error::new(
                    format!(
                        "can't animate property `{other}` (try: x, y, opacity, scale, trace, color, hue, angle)"
                    ),
                    a.span_of(1),
                ))
            }
        }
    } else if s.get_3d(&id).is_some() {
        match prop_name.as_str() {
            "morph" => (Prop::Morph, TargetValue::Abs(Value::F(a.num(2)?))),
            "opacity" | "alpha" => (Prop::Opacity, TargetValue::Abs(Value::F(a.num(2)?))),
            "scale" => (Prop::Scale, TargetValue::Abs(Value::F(a.num(2)?))),
            "trace" => (Prop::Trace, TargetValue::Abs(Value::F(a.num(2)?))),
            "color" => (
                Prop::Color,
                TargetValue::Abs(Value::C(resolve_color(&a.ident(2)?, a.span_of(2))?)),
            ),
            other => {
                return Err(Error::new(
                    format!(
                        "for a 3D entity, `to` animates morph, opacity, scale, trace, or color (use move3/shift3/rotate3/grow3 for position, rotation, and size); got `{other}`"
                    ),
                    a.span_of(1),
                ))
            }
        }
    } else {
        return Err(Error::new(format!("no entity named `{id}`"), here));
    };

    let dur = a.opt_num(3)?.unwrap_or(0.5);
    let easing = if a.len() > 4 {
        resolve_easing(&a.ident(4)?, a.span_of(4))?
    } else {
        Easing::InOutCubic
    };
    Ok(Clip {
        dur,
        tracks: vec![TrackSpec {
            id,
            prop,
            target,
            start: 0.0,
            dur,
            easing,
        }],
        events: Vec::new(),
    })
}

/// `transform(id, (ox,oy), a, b, c, d, [dur], [ease])` — apply the 2×2 matrix
/// `[[a,b],[c,d]]` to the entity about origin `(ox,oy)`: its anchor moves to
/// `origin + M·(pos − origin)`, and a line/arrow/curve endpoint moves the same
/// way. Broadcast over a tag to transform a whole group (a grid, vectors, a dot
/// cloud) at once — Manim's `ApplyMatrix` / a linear map of the plane.
fn v_transform(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let o = a.pair(1)?;
    let (m00, m01, m10, m11) = (a.num(2)?, a.num(3)?, a.num(4)?, a.num(5)?);
    let dur = a.opt_num(6)?.unwrap_or(0.9);
    let easing = if a.len() > 7 {
        resolve_easing(&a.ident(7)?, a.span_of(7))?
    } else {
        Easing::InOutCubic
    };
    let e = s
        .get(&id)
        .ok_or_else(|| Error::new(format!("no entity named `{id}`"), a.span_of(0)))?;
    let apply = |v: Vec2| {
        let w = v - o;
        o + Vec2::new(m00 * w.x + m01 * w.y, m10 * w.x + m11 * w.y)
    };
    let track = |prop, v: Vec2| TrackSpec {
        id: id.clone(),
        prop,
        target: TargetValue::Abs(Value::V(apply(v))),
        start: 0.0,
        dur,
        easing,
    };
    let mut tracks = vec![track(Prop::Pos, e.pos)];
    if let Shape::Line { to } | Shape::Arrow { to } | Shape::Curve { to, .. } = &e.shape {
        tracks.push(track(Prop::To, *to));
    }
    Ok(Clip {
        dur,
        tracks,
        events: Vec::new(),
    })
}

/// `swap(a, b, [dur], [ease])` — animate two entities into each other's position.
///
/// **Array form:** if the first argument is an `array` (has slot occupancy),
/// the call is `swap(arr, i, j, [dur])`: the values currently in slots `i` and
/// `j` **slide** past each other into the swapped slots (one hops over the top),
/// and the array's live occupancy is updated. Because occupancy carries forward,
/// a *chain* of swaps composes correctly — real in-place sorting, no `say`.
fn v_swap(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let id0 = a.ident(0)?;

    // ---- array slot-swap (stateful, chains across a sort) ----
    if s.occ.contains_key(&id0) {
        let i = a.num(1)? as usize;
        let j = a.num(2)? as usize;
        let dur = a.opt_num(3)?.unwrap_or(0.62);
        let (ei, ej, n) = {
            let occ = &s.occ[&id0];
            let n = occ.len();
            if i >= n || j >= n {
                return Err(Error::new(
                    format!("slot out of range for `{id0}` (have 0..{n})"),
                    a.span_of(if i >= n { 1 } else { 2 }),
                ));
            }
            (occ[i].clone(), occ[j].clone(), n)
        };
        let _ = n;
        if i == j {
            return Ok(Clip::wait(0.0));
        }
        let slot_pos = |k: usize| -> Result<Vec2, Error> {
            s.get(&format!("{id0}.box{k}"))
                .map(|e| e.pos)
                .ok_or_else(|| Error::new(format!("`{id0}` has no slot box {k}"), a.span_of(0)))
        };
        let pi = slot_pos(i)?;
        let pj = slot_pos(j)?;
        let lift = 54.0;
        let tr = |id: &str, to: Vec2, start: f32, d: f32, e: Easing| TrackSpec {
            id: id.into(),
            prop: Prop::Pos,
            target: TargetValue::Abs(Value::V(to)),
            start,
            dur: d,
            easing: e,
        };
        let h = dur * 0.5;
        let tracks = vec![
            // ei rises and travels across the top, then drops into slot j
            tr(&ei, Vec2::new(pj.x, pi.y - lift), 0.0, h, Easing::OutQuad),
            tr(&ei, pj, h, dur - h, Easing::InQuad),
            // ej slides along the baseline into slot i (passes under ei)
            tr(&ej, pi, 0.0, dur, Easing::InOutCubic),
        ];
        s.occ.get_mut(&id0).unwrap().swap(i, j);
        return Ok(Clip {
            dur,
            tracks,
            events: Vec::new(),
        });
    }

    // ---- generic two-entity position swap ----
    let idb = a.ident(1)?;
    let pa = s
        .get(&id0)
        .ok_or_else(|| Error::new(format!("no entity named `{id0}`"), a.span_of(0)))?
        .pos;
    let pb = s
        .get(&idb)
        .ok_or_else(|| Error::new(format!("no entity named `{idb}`"), a.span_of(1)))?
        .pos;
    let dur = a.opt_num(2)?.unwrap_or(0.6);
    let easing = if a.len() > 3 {
        resolve_easing(&a.ident(3)?, a.span_of(3))?
    } else {
        Easing::InOutCubic
    };
    let track = |id: String, to: Vec2| TrackSpec {
        id,
        prop: Prop::Pos,
        target: TargetValue::Abs(Value::V(to)),
        start: 0.0,
        dur,
        easing,
    };
    Ok(Clip {
        dur,
        tracks: vec![track(id0, pb), track(idb, pa)],
        events: Vec::new(),
    })
}

/// Build a single-track rotation clip (degrees), absolute or relative.
fn rot_clip(id: String, target: TargetValue, a: &Args, from: usize) -> Result<Clip, Error> {
    let dur = a.opt_num(from)?.unwrap_or(0.5);
    let easing = if a.len() > from + 1 {
        resolve_easing(&a.ident(from + 1)?, a.span_of(from + 1))?
    } else {
        Easing::InOutCubic
    };
    Ok(Clip {
        dur,
        tracks: vec![TrackSpec {
            id,
            prop: Prop::Rot,
            target,
            start: 0.0,
            dur,
            easing,
        }],
        events: Vec::new(),
    })
}

fn v_rotate(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let deg = a.num(1)?;
    rot_clip(id, TargetValue::Abs(Value::F(deg)), a, 2)
}

fn v_spin(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let by = a.num(1)?;
    rot_clip(id, TargetValue::Rel(Value::F(by)), a, 2)
}

fn v_cam(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let to = a.pair(0)?;
    Ok(apply_dur_ease(act().cam_to(to), a, 1)?.into())
}

fn v_zoom(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let z = a.num(0)?;
    Ok(apply_dur_ease(act().cam_zoom(z), a, 1)?.into())
}

// ---- boolean shape ops ----------------------------------------------------

/// `op(id, a, b, [color])` — combine two fillable shapes into a new region.
/// Operands `a`/`b` must already be declared (booleans read their geometry at
/// build time). The result is a filled `Region` entity `id`.
fn boolean(op: &str, s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let ida = a.ident(1)?;
    let idb = a.ident(2)?;
    let color = if a.len() > 3 {
        resolve_color(&a.ident(3)?, a.span_of(3))?
    } else {
        style::LIME
    };
    let (mpa, mpb) = {
        let ea = s
            .get(&ida)
            .ok_or_else(|| Error::new(format!("no entity named `{ida}`"), a.span_of(1)))?;
        let eb = s
            .get(&idb)
            .ok_or_else(|| Error::new(format!("no entity named `{idb}`"), a.span_of(2)))?;
        let mpa = geom::entity_to_multipolygon(ea).map_err(|m| Error::new(m, a.span_of(1)))?;
        let mpb = geom::entity_to_multipolygon(eb).map_err(|m| Error::new(m, a.span_of(2)))?;
        (mpa, mpb)
    };
    let (tris, rings) =
        geom::boolean_region(op, &mpa, &mpb).map_err(|m| Error::new(m, a.name_span))?;
    let mut e = Entity::new(id, Shape::Region { tris, rings }, Vec2::ZERO, color);
    e.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 2.5,
        outline_color: Some(style::FG),
    };
    s.add(e);
    Ok(())
}

fn c_union(s: &mut Scene, a: &Args) -> Result<(), Error> {
    boolean("union", s, a)
}
fn c_intersect(s: &mut Scene, a: &Args) -> Result<(), Error> {
    boolean("intersection", s, a)
}
fn c_difference(s: &mut Scene, a: &Args) -> Result<(), Error> {
    boolean("difference", s, a)
}
fn c_exclusion(s: &mut Scene, a: &Args) -> Result<(), Error> {
    boolean("xor", s, a)
}

/// Register the std kit into `r`.
pub fn register(r: &mut Registry) {
    // constructors
    r.ctor("text", c_text);
    r.ctor("counter", c_counter);
    r.ctor("caption", c_caption);
    r.ctor("morph", c_morph);
    r.ctor("copy", c_copy);
    r.ctor("dot", c_dot);
    r.ctor("circle", c_circle);
    r.ctor("rect", c_rect);
    r.ctor("line", c_line);
    r.ctor("polygon", c_polygon);
    r.ctor("arrow", c_arrow);
    r.ctor("brace", c_brace);
    r.ctor("bracelabel", c_bracelabel);
    r.ctor("bracetext", c_bracelabel);
    // modifiers (also constructors: they touch the base scene)
    r.ctor("hidden", m_hidden);
    r.ctor("untraced", m_untraced);
    r.ctor("cursor", m_cursor);
    r.ctor("rot", m_rot);
    r.ctor("opacity", m_opacity);
    r.ctor("color", m_color);
    r.ctor("hue", m_hue);
    r.ctor("outlined", m_outlined);
    r.ctor("filled", m_filled);
    r.ctor("outline", m_outline);
    r.ctor("size", m_size);
    r.ctor("stroke", m_stroke);
    r.ctor("glow", m_glow);
    r.ctor("z", m_z);
    r.ctor("tag", m_tag);
    r.ctor("bold", m_bold);
    r.ctor("display", m_display);
    r.ctor("label", m_label);
    // boolean shape ops → a new filled region
    r.ctor("union", c_union);
    r.ctor("intersect", c_intersect);
    r.ctor("intersection", c_intersect);
    r.ctor("difference", c_difference);
    r.ctor("subtract", c_difference);
    r.ctor("exclusion", c_exclusion);
    r.ctor("xor", c_exclusion);
    // verbs
    r.verb("show", v_show);
    r.verb("fade", v_fade);
    r.verb("move", v_move);
    r.verb("shift", v_shift);
    r.verb("grow", v_grow);
    r.verb("draw", v_draw);
    r.verb("erase", v_erase);
    r.verb("type", v_type);
    r.verb("say", v_say);
    r.verb("recolor", v_recolor);
    r.verb("flash", v_flash);
    r.verb("pulse", v_pulse);
    r.verb("shake", v_shake);
    r.verb("scale", v_scale);
    r.verb("rotate", v_rotate); // to an absolute angle (degrees)
    r.verb("spin", v_spin); // by a relative angle (degrees)
    r.verb("to", v_to); // general escape hatch: animate any property
    r.verb("set", v_to); // alias
    r.verb("cam", v_cam);
    r.verb("zoom", v_zoom);
    r.verb("transform", v_transform); // apply a 2x2 matrix (ApplyMatrix)
    r.mut_verb("swap", v_swap); // two entities, or stateful array slot-swap
    r.verb("karaoke", v_karaoke); // highlight caption words in sequence
    r.verb("wordpop", v_wordpop); // pop caption words in one at a time
}
