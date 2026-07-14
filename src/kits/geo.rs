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

/// Reflection of `p` across the line through `a`, `b`.
fn reflect_pt(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    foot(p, a, b) * 2.0 - p
}

/// Unit direction of the internal angle bisector at vertex `b`.
fn bisect_dir(a: Vec2, b: Vec2, c: Vec2) -> Vec2 {
    ((a - b).normalize_or_zero() + (c - b).normalize_or_zero()).normalize_or_zero()
}

/// The (up to two) intersections of the line through `a`,`b` with the circle
/// centred at `o` of radius `r`. `None` if the line misses the circle.
fn lc_intersect(a: Vec2, b: Vec2, o: Vec2, r: f32) -> Option<(Vec2, Vec2)> {
    let f = foot(o, a, b);
    let dist2 = (o - f).length_squared();
    if dist2 > r * r {
        return None;
    }
    let half = (r * r - dist2).max(0.0).sqrt();
    let dir = (b - a).normalize_or_zero();
    Some((f + dir * half, f - dir * half))
}

/// The (up to two) intersections of two circles.
fn cc_intersect(c0: Vec2, r0: f32, c1: Vec2, r1: f32) -> Option<(Vec2, Vec2)> {
    let d = (c1 - c0).length();
    if d < 1e-6 || d > r0 + r1 || d < (r0 - r1).abs() {
        return None;
    }
    let a = (r0 * r0 - r1 * r1 + d * d) / (2.0 * d);
    let h = (r0 * r0 - a * a).max(0.0).sqrt();
    let u = (c1 - c0) / d;
    let mid = c0 + u * a;
    let perp = Vec2::new(-u.y, u.x);
    Some((mid + perp * h, mid - perp * h))
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
            start: st,
            sweep: sw,
            ..
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

fn d_reflect(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 3 {
        e.pos = reflect_pt(p[0], p[1], p[2]);
    }
}
fn d_bisector(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 3 {
        let len = ((p[0] - p[1]).length() + (p[2] - p[1]).length()) * 0.5;
        e.pos = p[1] + bisect_dir(p[0], p[1], p[2]) * len;
    }
}
/// Circle centred at `p[0]` passing through `p[1]`.
fn d_circle2(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 2 {
        e.pos = p[0];
        if let Shape::Circle { r } = &mut e.shape {
            *r = (p[0] - p[1]).length();
        }
    }
}

// line(p0,p1) ∩ circle(centre p2, through p3) — two touch points, or the
// closest point on the line when they miss (so the dot doesn't fly off).
fn meetlc(e: &mut Entity, p: &[Vec2], which: usize) {
    if p.len() >= 4 {
        let r = (p[2] - p[3]).length();
        e.pos = match lc_intersect(p[0], p[1], p[2], r) {
            Some((a, b)) => {
                if which == 0 {
                    a
                } else {
                    b
                }
            }
            None => foot(p[2], p[0], p[1]),
        };
    }
}
fn d_linecircle0(e: &mut Entity, p: &[Vec2]) {
    meetlc(e, p, 0);
}
fn d_linecircle1(e: &mut Entity, p: &[Vec2]) {
    meetlc(e, p, 1);
}

// circle(centre p0, through p1) ∩ circle(centre p2, through p3).
fn meetcc(e: &mut Entity, p: &[Vec2], which: usize) {
    if p.len() >= 4 {
        let (r0, r1) = ((p[0] - p[1]).length(), (p[2] - p[3]).length());
        e.pos = match cc_intersect(p[0], r0, p[2], r1) {
            Some((a, b)) => {
                if which == 0 {
                    a
                } else {
                    b
                }
            }
            None => (p[0] + p[2]) * 0.5,
        };
    }
}
fn d_circlecircle0(e: &mut Entity, p: &[Vec2]) {
    meetcc(e, p, 0);
}
fn d_circlecircle1(e: &mut Entity, p: &[Vec2]) {
    meetcc(e, p, 1);
}

// tangent touch-points from external point p0 to circle(centre p1, through p2).
// The touch points are where the original circle meets the circle on diameter
// p0-p1 (Thales), so this is just a circle∩circle.
fn tangent(e: &mut Entity, p: &[Vec2], which: usize) {
    if p.len() >= 3 {
        let r = (p[1] - p[2]).length();
        let mid = (p[0] + p[1]) * 0.5;
        let r2 = (p[0] - p[1]).length() * 0.5;
        e.pos = match cc_intersect(p[1], r, mid, r2) {
            Some((a, b)) => {
                if which == 0 {
                    a
                } else {
                    b
                }
            }
            None => p[1] + (p[0] - p[1]).normalize_or_zero() * r,
        };
    }
}
fn d_tangent0(e: &mut Entity, p: &[Vec2]) {
    tangent(e, p, 0);
}
fn d_tangent1(e: &mut Entity, p: &[Vec2]) {
    tangent(e, p, 1);
}

// --- point-by-parameter hooks. These read a scalar stashed in `e.rot` at
// construction (a dot's rotation is unused), so the construction can carry an
// angle / fraction while staying dynamic in its point inputs. ---

/// `p[0]` rotated about centre `p[1]` by `e.rot` degrees.
fn d_rotpoint(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 2 {
        let (s, c) = e.rot.to_radians().sin_cos();
        let d = p[0] - p[1];
        e.pos = p[1] + Vec2::new(d.x * c - d.y * s, d.x * s + d.y * c);
    }
}
/// The point a fraction `e.rot` of the way from `p[0]` to `p[1]`.
fn d_between(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 2 {
        e.pos = p[0] + (p[1] - p[0]) * e.rot;
    }
}
/// A point on the circle (centre `p[0]`, through `p[1]`) at absolute angle
/// `e.rot` degrees.
fn d_anglepoint(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 2 {
        let r = (p[0] - p[1]).length();
        let a = e.rot.to_radians();
        e.pos = p[0] + Vec2::new(a.cos(), a.sin()) * r;
    }
}
/// A line through `p[0]`,`p[1]` extended far past both (looks infinite).
fn d_fullline(e: &mut Entity, p: &[Vec2]) {
    if p.len() >= 2 {
        let dir = (p[1] - p[0]).normalize_or_zero();
        let big = 4000.0;
        e.pos = p[0] - dir * big;
        if let Shape::Line { to } = &mut e.shape {
            *to = p[1] + dir * big;
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
derived_point!(c_reflect, 3, d_reflect);
derived_point!(c_bisector, 3, d_bisector);

/// A two-point construction (intersections, tangents): reads `$n` referenced
/// points and makes dots `{id}0` and `{id}1`, each tracking the inputs.
macro_rules! derived_pair {
    ($name:ident, $n:expr, $dfn0:path, $dfn1:path) => {
        fn $name(s: &mut Scene, a: &Args) -> Result<(), Error> {
            let id = a.ident(0)?;
            let mut deps = Vec::with_capacity($n);
            let mut ps = Vec::with_capacity($n);
            for i in 0..$n {
                deps.push(a.ident(i + 1)?);
                ps.push(pt(s, a, i + 1)?);
            }
            for (suffix, dfn) in [("0", $dfn0 as crate::primitives::DeriveFn), ("1", $dfn1)] {
                let mut e = dot(Vec2::ZERO, 5.0, style::CYAN);
                e.id = format!("{id}{suffix}");
                e.deps = deps.clone();
                e.derive = Some(dfn);
                dfn(&mut e, &ps);
                s.add(e);
            }
            Ok(())
        }
    };
}

derived_pair!(c_linecircle, 4, d_linecircle0, d_linecircle1);
derived_pair!(c_circlecircle, 4, d_circlecircle0, d_circlecircle1);
derived_pair!(c_tangent_geo, 3, d_tangent0, d_tangent1);

/// `tangent` is overloaded so it "just works" on whatever you point it at:
/// - `tangent(id, curve, x, [len])` — the **calculus** tangent to a plotted
///   function at `x` (numeric 3rd arg), handled by the math kit.
/// - `tangent(id, p, c, thru)` — the classic **geometry** construction: the two
///   touch-points of the tangents from external point `p` to a circle (all-name
///   args).
///
/// The numeric-vs-name 3rd argument tells the two apart, so each branch reports
/// its own errors cleanly.
fn c_tangent(s: &mut Scene, a: &Args) -> Result<(), Error> {
    if a.num(2).is_ok() {
        crate::kits::math::c_graph_tangent(s, a)
    } else {
        c_tangent_geo(s, a)
    }
}

/// A derived point with `$n` point inputs plus a trailing **scalar** (angle or
/// fraction), stashed in `e.rot` for the hook to read.
macro_rules! derived_scalar_point {
    ($name:ident, $n:expr, $dfn:path) => {
        fn $name(s: &mut Scene, a: &Args) -> Result<(), Error> {
            let id = a.ident(0)?;
            let mut deps = Vec::with_capacity($n);
            let mut ps = Vec::with_capacity($n);
            for i in 0..$n {
                deps.push(a.ident(i + 1)?);
                ps.push(pt(s, a, i + 1)?);
            }
            let scalar = a.num($n + 1)?;
            let mut e = dot(Vec2::ZERO, 5.0, style::LIME);
            e.id = id;
            e.rot = scalar; // stashed param (unused for a dot's rendering)
            e.deps = deps;
            e.derive = Some($dfn);
            $dfn(&mut e, &ps);
            s.add(e);
            Ok(())
        }
    };
}

derived_scalar_point!(c_rotpoint, 2, d_rotpoint); // rotpoint(id, p, center, deg)
derived_scalar_point!(c_between, 2, d_between); //   between(id, a, b, t)
derived_scalar_point!(c_anglepoint, 2, d_anglepoint); // anglepoint(id, center, on, deg)

/// `fullline(id, a, b)` — a line through two points, extended past both so it
/// reads as infinite; tracks the points.
fn c_fullline(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let (deps, ps) = deps_and_pts(s, a, 2)?;
    let mut e = Entity::new(id, Shape::Line { to: Vec2::ZERO }, Vec2::ZERO, style::FG);
    e.stroke.width = 2.0;
    e.z = 1;
    e.deps = deps;
    e.derive = Some(d_fullline);
    d_fullline(&mut e, &ps);
    s.add(e);
    Ok(())
}

/// `ellipse(id, (cx,cy), rx, ry, [rot_deg])` — a (rotatable) ellipse outline.
fn c_ellipse(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let rx = a.num(2)?;
    let ry = a.num(3)?;
    let (sr, cr) = a.opt_num(4)?.unwrap_or(0.0).to_radians().sin_cos();
    let n = 96;
    let pts = (0..=n)
        .map(|i| {
            let t = std::f32::consts::TAU * i as f32 / n as f32;
            let (x, y) = (rx * t.cos(), ry * t.sin());
            Vec2::new(c.x + x * cr - y * sr, c.y + x * sr + y * cr)
        })
        .collect();
    let mut e = Entity::new(id, Shape::Polyline { pts }, Vec2::ZERO, style::MAGENTA);
    e.stroke = StrokeStyle {
        fill: false,
        outline: true,
        width: 2.5,
        outline_color: Some(style::MAGENTA),
    };
    e.z = 1;
    s.add(e);
    Ok(())
}

/// Add a stroked open curve entity `id` from a point list.
fn add_curve(
    s: &mut Scene,
    id: &str,
    pts: Vec<Vec2>,
    color: macroquad::prelude::Color,
    tag: Option<&str>,
) {
    let mut e = Entity::new(id.to_string(), Shape::Polyline { pts }, Vec2::ZERO, color);
    e.stroke = StrokeStyle {
        fill: false,
        outline: true,
        width: 2.5,
        outline_color: Some(color),
    };
    e.z = 1;
    if let Some(t) = tag {
        e.tags.push(t.to_string());
    }
    s.add(e);
}

/// `parabola(id, (vx,vy), halfwidth, height)` — a parabola with vertex `(vx,vy)`,
/// arms reaching `height` px up at `±halfwidth` px (negative height opens down).
fn c_parabola(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let v = a.pair(1)?;
    let hw = a.num(2)?;
    let h = a.num(3)?;
    let k = if hw.abs() < 1e-3 { 0.0 } else { h / (hw * hw) };
    let n = 80;
    let pts = (0..=n)
        .map(|i| {
            let t = -hw + 2.0 * hw * i as f32 / n as f32;
            Vec2::new(v.x + t, v.y - k * t * t)
        })
        .collect();
    add_curve(s, &id, pts, style::MAGENTA, None);
    Ok(())
}

/// `hyperbola(id, (cx,cy), a, b, [srange])` — a hyperbola centred at `(cx,cy)`
/// with semi-axes `a` (horizontal) / `b` (vertical), drawn as its two branches
/// `{id}.r` and `{id}.l` (both tagged `id`, so `color(id, …)` hits both).
fn c_hyperbola(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let ax = a.num(2)?;
    let by = a.num(3)?;
    let sr = a.opt_num(4)?.unwrap_or(1.7);
    let n = 64;
    let branch = |sign: f32| -> Vec<Vec2> {
        (0..=n)
            .map(|i| {
                let sp = -sr + 2.0 * sr * i as f32 / n as f32;
                Vec2::new(c.x + sign * ax * sp.cosh(), c.y + by * sp.sinh())
            })
            .collect()
    };
    add_curve(s, &format!("{id}.r"), branch(1.0), style::CYAN, Some(&id));
    add_curve(s, &format!("{id}.l"), branch(-1.0), style::CYAN, Some(&id));
    Ok(())
}

/// `circle2(id, center, through)` — a circle centred at `center` passing through
/// `through`; tracks both (radius = their distance).
fn c_circle2(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let (deps, ps) = deps_and_pts(s, a, 2)?;
    let mut e = outlined_circle(&id, Vec2::ZERO, 1.0, style::CYAN);
    e.deps = deps;
    e.derive = Some(d_circle2);
    d_circle2(&mut e, &ps);
    s.add(e);
    Ok(())
}

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
    let mut e = Entity::new(
        id,
        Shape::Polyline { pts: Vec::new() },
        Vec2::ZERO,
        style::LIME,
    );
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
    r.ctor("reflect", c_reflect);
    r.ctor("bisector", c_bisector);
    r.ctor("linecircle", c_linecircle);
    r.ctor("circlecircle", c_circlecircle);
    r.ctor("tangent", c_tangent);
    r.ctor("circle2", c_circle2);
    r.ctor("ellipse", c_ellipse);
    r.ctor("parabola", c_parabola);
    r.ctor("hyperbola", c_hyperbola);
    r.ctor("fullline", c_fullline);
    r.ctor("rotpoint", c_rotpoint);
    r.ctor("between", c_between);
    r.ctor("anglepoint", c_anglepoint);
    r.ctor("circumcircle", c_circumcircle);
    r.ctor("incircle", c_incircle);
    r.ctor("anglemark", c_anglemark);
    r.ctor("rightangle", c_rightangle);
}
