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
    scene
        .get_mut(&id)
        .ok_or_else(|| Error::new(format!("no entity named `{id}`"), span))
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
    ent_mut(s, a)?.opacity = 0.0;
    Ok(())
}

fn m_color(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let c = resolve_color(&a.ident(1)?, a.span_of(1))?;
    ent_mut(s, a)?.color = c;
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
    ent_mut(s, a)?.stroke.width = n;
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
    let tag = a.ident(1)?;
    ent_mut(s, a)?.tags.push(tag);
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
    ent_mut(s, a)?.trace = 0.0;
    Ok(())
}

fn m_rot(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let deg = a.num(1)?;
    ent_mut(s, a)?.rot = deg;
    Ok(())
}

fn m_opacity(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let n = a.num(1)?;
    ent_mut(s, a)?.opacity = n;
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
    let cur = s
        .get(&id)
        .ok_or_else(|| Error::new(format!("no entity named `{id}`"), here))?;

    // (property track, target value)
    let (prop, target) = match prop_name.as_str() {
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
        other => {
            return Err(Error::new(
                format!(
                    "can't animate property `{other}` (try: x, y, opacity, scale, trace, color)"
                ),
                a.span_of(1),
            ))
        }
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
    r.ctor("dot", c_dot);
    r.ctor("circle", c_circle);
    r.ctor("rect", c_rect);
    r.ctor("line", c_line);
    r.ctor("arrow", c_arrow);
    r.ctor("brace", c_brace);
    r.ctor("bracelabel", c_bracelabel);
    r.ctor("bracetext", c_bracelabel);
    // modifiers (also constructors: they touch the base scene)
    r.ctor("hidden", m_hidden);
    r.ctor("untraced", m_untraced);
    r.ctor("rot", m_rot);
    r.ctor("opacity", m_opacity);
    r.ctor("color", m_color);
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
}
