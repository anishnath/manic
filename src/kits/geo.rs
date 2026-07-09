//! The **geo kit**: olympiad / Euclidean geometry helpers, in the spirit of
//! Asymptote's `olympiad.asy` + `cse5.asy`. High-level constructors compute a
//! point / circle / mark from *referenced points*, so the author writes the
//! geometry, not coordinates.
//!
//! Like asy, constructions are evaluated at build time (the referenced points
//! must be declared first). Segments track their endpoints (reflow via `link`);
//! derived points and circles are static. A third domain proving the core is
//! domain-agnostic: this file + one `default_registry` line, zero core changes.

use macroquad::prelude::Vec2;

use crate::lang::diag::Error;
use crate::lang::lower::{Args, Registry};
use crate::primitives::{Entity, FontKind, Link, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;

/// Read a referenced point's position, or error nicely.
fn pt(s: &Scene, a: &Args, i: usize) -> Result<Vec2, Error> {
    let id = a.ident(i)?;
    s.get(&id)
        .map(|e| e.pos)
        .ok_or_else(|| Error::new(format!("no point named `{id}`"), a.span_of(i)))
}

fn dot(pos: Vec2, r: f32, color: macroquad::prelude::Color) -> Entity {
    let mut e = Entity::new(String::new(), Shape::Circle { r }, pos, color);
    e.stroke = StrokeStyle {
        fill: true,
        outline: false,
        ..Default::default()
    };
    e.z = 6;
    e
}

/// Add a dot entity `id` at `pos`.
fn add_dot(s: &mut Scene, id: &str, pos: Vec2, r: f32, color: macroquad::prelude::Color) {
    let mut e = dot(pos, r, color);
    e.id = id.to_string();
    s.add(e);
}

fn outlined_circle(id: &str, center: Vec2, r: f32, color: macroquad::prelude::Color) -> Entity {
    let mut e = Entity::new(id.to_string(), Shape::Circle { r }, center, color);
    e.stroke = StrokeStyle {
        fill: false,
        outline: true,
        width: 2.5,
        outline_color: Some(color),
    };
    e
}

// ---- geometry math --------------------------------------------------------

fn circumcenter(a: Vec2, b: Vec2, c: Vec2) -> Option<Vec2> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-6 {
        return None;
    }
    let (a2, b2, c2) = (a.length_squared(), b.length_squared(), c.length_squared());
    let ux = (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d;
    let uy = (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d;
    Some(Vec2::new(ux, uy))
}

fn incenter(a: Vec2, b: Vec2, c: Vec2) -> Vec2 {
    let la = (c - b).length(); // opposite A
    let lb = (a - c).length(); // opposite B
    let lc = (b - a).length(); // opposite C
    (a * la + b * lb + c * lc) / (la + lb + lc)
}

fn tri_area(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    ((b - a).x * (c - a).y - (b - a).y * (c - a).x).abs() * 0.5
}

fn inradius(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    let s = ((c - b).length() + (a - c).length() + (b - a).length()) * 0.5;
    if s < 1e-6 {
        0.0
    } else {
        tri_area(a, b, c) / s
    }
}

fn foot(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let d = ab.length_squared();
    if d < 1e-6 {
        return a;
    }
    a + ab * ((p - a).dot(ab) / d)
}

fn line_meet(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Option<Vec2> {
    let r = b - a;
    let s = d - c;
    let rxs = r.x * s.y - r.y * s.x;
    if rxs.abs() < 1e-6 {
        return None;
    }
    let t = ((c - a).x * s.y - (c - a).y * s.x) / rxs;
    Some(a + r * t)
}

// ---- point constructors ---------------------------------------------------

/// `point(id, (x,y), ["label"])` — a labelled dot.
fn c_point(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let pos = a.pair(1)?;
    add_dot(s, &id, pos, 5.0, style::CYAN);
    if a.len() > 2 {
        let text = a.text(2)?;
        let mut lbl = Entity::new(
            format!("{id}.label"),
            Shape::Text {
                content: text,
                size: 22.0,
            },
            Vec2::ZERO,
            style::FG,
        );
        lbl.font = FontKind::MonoBold;
        lbl.z = 7;
        lbl.follow = Some((id, Vec2::new(0.0, -22.0)));
        s.add(lbl);
    }
    Ok(())
}

// ---- derive hooks: recompute an entity from its deps' positions ----------
// These run every frame in Timeline::apply, so a construction tracks its
// inputs when they move.

fn d_midpoint(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 2 {
        e.pos = (p[0] + p[1]) * 0.5;
    }
}
fn d_centroid(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 3 {
        e.pos = (p[0] + p[1] + p[2]) / 3.0;
    }
}
fn d_circumcenter(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 3 {
        if let Some(o) = circumcenter(p[0], p[1], p[2]) {
            e.pos = o;
        }
    }
}
fn d_incenter(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 3 {
        e.pos = incenter(p[0], p[1], p[2]);
    }
}
fn d_orthocenter(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 3 {
        if let Some(o) = circumcenter(p[0], p[1], p[2]) {
            e.pos = p[0] + p[1] + p[2] - o * 2.0;
        }
    }
}
fn d_foot(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 3 {
        e.pos = foot(p[0], p[1], p[2]);
    }
}
fn d_meet(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 4 {
        if let Some(m) = line_meet(p[0], p[1], p[2], p[3]) {
            e.pos = m;
        }
    }
}
fn d_circumcircle(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 3 {
        if let Some(o) = circumcenter(p[0], p[1], p[2]) {
            e.pos = o;
            if let Shape::Circle { r } = &mut e.shape {
                *r = (p[0] - o).length();
            }
        }
    }
}
fn d_incircle(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 3 {
        e.pos = incenter(p[0], p[1], p[2]);
        if let Shape::Circle { r } = &mut e.shape {
            *r = inradius(p[0], p[1], p[2]);
        }
    }
}
fn d_anglemark(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 3 {
        let (a, b, c) = (p[0], p[1], p[2]);
        e.pos = b;
        let start = (a - b).y.atan2((a - b).x).to_degrees();
        let end = (c - b).y.atan2((c - b).x).to_degrees();
        let mut sweep = end - start;
        while sweep <= -180.0 {
            sweep += 360.0;
        }
        while sweep > 180.0 {
            sweep -= 360.0;
        }
        if let Shape::Arc {
            start: st, sweep: sw, ..
        } = &mut e.shape
        {
            *st = start;
            *sw = sweep;
        }
    }
}
fn d_rightangle(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 3 {
        let (a, b, c) = (p[0], p[1], p[2]);
        let u = (a - b).normalize_or_zero();
        let v = (c - b).normalize_or_zero();
        let sz = 16.0;
        if let Shape::Polyline { pts } = &mut e.shape {
            *pts = vec![b + u * sz, b + u * sz + v * sz, b + v * sz];
        }
    }
}

/// A derived-point constructor: reads `$n` referenced points, makes a dot, and
/// wires up `deps` + a `derive` hook so it tracks them.
macro_rules! derived_point {
    ($name:ident, $n:expr, $dfn:path) => {
        fn $name(s: &mut Scene, a: &Args) -> Result<(), Error> {
            let id = a.ident(0)?;
            let mut deps = Vec::with_capacity($n);
            let mut ps = Vec::with_capacity($n);
            for i in 0..$n {
                deps.push(a.ident(i + 1)?);
                ps.push(pt(s, a, i + 1)?);
            }
            let mut e = dot(Vec2::ZERO, 5.0, style::LIME);
            e.id = id;
            e.deps = deps;
            e.derive = Some($dfn);
            $dfn(&mut e, &ps); // initial value
            s.add(e);
            Ok(())
        }
    };
}

derived_point!(c_midpoint, 2, d_midpoint);
derived_point!(c_centroid, 3, d_centroid);
derived_point!(c_circumcenter, 3, d_circumcenter);
derived_point!(c_incenter, 3, d_incenter);
derived_point!(c_orthocenter, 3, d_orthocenter);
derived_point!(c_foot, 3, d_foot);
derived_point!(c_meet, 4, d_meet);

// ---- segments, circles, marks ---------------------------------------------

/// `segment(id, a, b)` — a line joining two points; reflows if they move.
fn c_segment(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let ida = a.ident(1)?;
    let idb = a.ident(2)?;
    let (pa, pb) = (pt(s, a, 1)?, pt(s, a, 2)?);
    let mut e = Entity::new(id, Shape::Line { to: pb }, pa, style::FG);
    e.stroke.width = 2.0;
    e.z = 1;
    e.link = Some(Link {
        from: ida,
        to: idb,
        trim: 0.0,
    });
    s.add(e);
    Ok(())
}

/// Collect `n` referenced point ids + their current positions.
fn deps_and_pts(s: &Scene, a: &Args, n: usize) -> Result<(Vec<String>, Vec<Vec2>), Error> {
    let mut deps = Vec::with_capacity(n);
    let mut ps = Vec::with_capacity(n);
    for i in 0..n {
        deps.push(a.ident(i + 1)?);
        ps.push(pt(s, a, i + 1)?);
    }
    Ok((deps, ps))
}

/// `circumcircle(id, a, b, c)` — circle through three points (magenta), tracks them.
fn c_circumcircle(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let (deps, ps) = deps_and_pts(s, a, 3)?;
    let mut e = outlined_circle(&id, Vec2::ZERO, 1.0, style::MAGENTA);
    e.deps = deps;
    e.derive = Some(d_circumcircle);
    d_circumcircle(&mut e, &ps);
    s.add(e);
    Ok(())
}

/// `incircle(id, a, b, c)` — inscribed circle of a triangle (lime), tracks it.
fn c_incircle(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let (deps, ps) = deps_and_pts(s, a, 3)?;
    let mut e = outlined_circle(&id, Vec2::ZERO, 1.0, style::LIME);
    e.deps = deps;
    e.derive = Some(d_incircle);
    d_incircle(&mut e, &ps);
    s.add(e);
    Ok(())
}

/// `anglemark(id, a, b, c)` — an arc marking the angle at vertex `b`, tracks it.
fn c_anglemark(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let (deps, ps) = deps_and_pts(s, a, 3)?;
    let mut e = Entity::new(
        id,
        Shape::Arc {
            r: 26.0,
            inner: 0.0,
            start: 0.0,
            sweep: 0.0,
        },
        Vec2::ZERO,
        style::LIME,
    );
    e.stroke = StrokeStyle {
        fill: false,
        outline: true,
        width: 2.0,
        outline_color: Some(style::LIME),
    };
    e.z = 2;
    e.deps = deps;
    e.derive = Some(d_anglemark);
    d_anglemark(&mut e, &ps);
    s.add(e);
    Ok(())
}

/// `rightangle(id, a, b, c)` — a small square marking a right angle at `b`, tracks it.
fn c_rightangle(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let (deps, ps) = deps_and_pts(s, a, 3)?;
    let mut e = Entity::new(id, Shape::Polyline { pts: Vec::new() }, Vec2::ZERO, style::LIME);
    e.stroke.width = 2.0;
    e.z = 2;
    e.deps = deps;
    e.derive = Some(d_rightangle);
    d_rightangle(&mut e, &ps);
    s.add(e);
    Ok(())
}

/// Register the geo kit into `r`.
pub fn register(r: &mut Registry) {
    r.ctor("point", c_point);
    r.ctor("segment", c_segment);
    r.ctor("midpoint", c_midpoint);
    r.ctor("centroid", c_centroid);
    r.ctor("circumcenter", c_circumcenter);
    r.ctor("incenter", c_incenter);
    r.ctor("orthocenter", c_orthocenter);
    r.ctor("foot", c_foot);
    r.ctor("meet", c_meet);
    r.ctor("circumcircle", c_circumcircle);
    r.ctor("incircle", c_incircle);
    r.ctor("anglemark", c_anglemark);
    r.ctor("rightangle", c_rightangle);
}
