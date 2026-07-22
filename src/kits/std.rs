//! The **std kit**: the base vocabulary every manic program has. Generic
//! shapes and animation verbs — no domain knowledge. Domain kits (math, algo)
//! layer on top of this.
//!
//! Constructors declare/modify the cast at t=0; verbs produce timeline clips.

use macroquad::prelude::Vec2;

use crate::animate::act;
use crate::easing::Easing;
use crate::geom;
use crate::lang::ast::ExprKind;
use crate::lang::diag::Error;
use crate::lang::lower::{apply_dur_ease, resolve_color, resolve_easing, Args, Registry};
use crate::primitives::{
    BoundProperty, Counter, Entity, FontKind, Link, Parameter, ParameterBinding, ParameterMap,
    Shape, StrokeStyle,
};
use crate::scene::{EquationState, ParticleGroup, PendingEquationPart, Scene};
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TimelineEvent, TrackSpec, Value};

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

#[derive(Clone, Copy)]
struct TinyRng(u64);

impl TinyRng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn unit(&mut self) -> f32 {
        // xorshift64*: tiny, deterministic and sufficient for visual sampling.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let v = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        ((v >> 40) as f32) / ((1u32 << 24) as f32)
    }
}

fn stable_seed(id: &str) -> u64 {
    id.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x1000_0000_01b3)
    })
}

fn rot_local(v: Vec2, deg: f32) -> Vec2 {
    let (sn, cs) = deg.to_radians().sin_cos();
    Vec2::new(v.x * cs - v.y * sn, v.x * sn + v.y * cs)
}

fn authored_point(a: &Args, index: usize, scene: &Scene) -> Result<Vec2, Error> {
    let expr = a.exprs.get(index).ok_or_else(|| {
        Error::new(
            format!("`{}` needs at least {} argument(s)", a.name, index + 1),
            a.name_span,
        )
    })?;
    match &expr.kind {
        ExprKind::Pair(x, y) => Ok(Vec2::new(*x, *y)),
        ExprKind::Ident(id) => scene
            .authored_entity(id)
            .map(|entity| entity.pos)
            .ok_or_else(|| Error::new(format!("no entity named `{id}` to point at"), expr.span)),
        _ => Err(Error::new(
            format!(
                "argument {} of `{}` should be a `(x, y)` point or an entity name",
                index + 1,
                a.name
            ),
            expr.span,
        )),
    }
}

/// Deterministically sample inset points inside a circle or rectangle. Both
/// shapes are convex, so tweening between any two samples also stays contained.
/// Supporting arbitrary concave regions would require path planning rather
/// than a small creator-facing primitive, so it is intentionally out of v1.
fn particle_points(
    container: &Entity,
    count: usize,
    radius: f32,
    seed: u64,
) -> Result<Vec<Vec2>, String> {
    let mut rng = TinyRng::new(seed);
    match &container.shape {
        Shape::Circle { r } => {
            let usable = r * container.scale.abs() - radius;
            if usable < 0.0 {
                return Err(format!(
                    "particle radius {radius} is larger than the circle's interior"
                ));
            }
            Ok((0..count)
                .map(|_| {
                    let a = std::f32::consts::TAU * rng.unit();
                    let d = usable * rng.unit().sqrt();
                    container.pos + Vec2::new(a.cos() * d, a.sin() * d)
                })
                .collect())
        }
        Shape::Rect { w, h } => {
            let hw = w * container.scale.abs() * 0.5 - radius;
            let hh = h * container.scale.abs() * 0.5 - radius;
            if hw < 0.0 || hh < 0.0 {
                return Err(format!(
                    "particle radius {radius} is larger than the rectangle's interior"
                ));
            }
            Ok((0..count)
                .map(|_| {
                    let local =
                        Vec2::new((rng.unit() * 2.0 - 1.0) * hw, (rng.unit() * 2.0 - 1.0) * hh);
                    container.pos + rot_local(local, container.rot)
                })
                .collect())
        }
        _ => Err("particles currently support circle or rect containers".into()),
    }
}

/// Lay particles out as an ordered grid inside a rectangle. The final partial
/// row is centred, which keeps small groups visually balanced. A grid inside a
/// circle is intentionally not guessed: "ordered" has no single obvious
/// circular layout, while a rectangle has an unambiguous row/column reading.
fn particle_grid_points(
    container: &Entity,
    count: usize,
    radius: f32,
) -> Result<Vec<Vec2>, String> {
    let Shape::Rect { w, h } = &container.shape else {
        return Err("grid particle layout currently needs a rectangle container".into());
    };
    let usable_w = w * container.scale.abs() - 2.0 * radius;
    let usable_h = h * container.scale.abs() - 2.0 * radius;
    if usable_w < 0.0 || usable_h < 0.0 {
        return Err(format!(
            "particle radius {radius} is larger than the rectangle's interior"
        ));
    }
    if count == 1 {
        return Ok(vec![container.pos]);
    }

    let aspect = (usable_w / usable_h.max(1.0)).max(0.01);
    let cols = ((count as f32 * aspect).sqrt().ceil() as usize).clamp(1, count);
    let rows = count.div_ceil(cols);
    let step_x = if cols > 1 {
        usable_w / (cols - 1) as f32
    } else {
        0.0
    };
    let step_y = if rows > 1 {
        usable_h / (rows - 1) as f32
    } else {
        0.0
    };
    if (cols > 1 && step_x + 1e-4 < radius * 2.0) || (rows > 1 && step_y + 1e-4 < radius * 2.0) {
        return Err(format!(
            "{count} particles of radius {radius} do not fit as a grid in this rectangle"
        ));
    }

    let mut points = Vec::with_capacity(count);
    for row in 0..rows {
        let row_start = row * cols;
        let row_count = (count - row_start).min(cols);
        let y = if rows > 1 {
            -usable_h * 0.5 + row as f32 * step_y
        } else {
            0.0
        };
        let x0 = -(row_count.saturating_sub(1) as f32) * step_x * 0.5;
        for col in 0..row_count {
            let local = Vec2::new(x0 + col as f32 * step_x, y);
            points.push(container.pos + rot_local(local, container.rot));
        }
    }
    Ok(points)
}

/// Place particles evenly around a circular container. This is an ordered
/// layout just like `grid`, but for cyclic/radial stories: clocks, orbits,
/// rings, radial menus, and state transitions all use the same primitive.
fn particle_ring_points(
    container: &Entity,
    count: usize,
    radius: f32,
) -> Result<Vec<Vec2>, String> {
    let Shape::Circle { r } = &container.shape else {
        return Err("ring particle layout currently needs a circle container".into());
    };
    let orbit = r * container.scale.abs() - radius;
    if orbit < 0.0 {
        return Err(format!(
            "particle radius {radius} is larger than the circle's interior"
        ));
    }
    if count == 1 {
        return Ok(vec![container.pos]);
    }
    Ok((0..count)
        .map(|i| {
            let angle =
                -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * i as f32 / count as f32;
            container.pos + Vec2::new(angle.cos(), angle.sin()) * orbit
        })
        .collect())
}

fn particle_layout_points(
    container: &Entity,
    count: usize,
    radius: f32,
    seed: u64,
    layout: &str,
) -> Result<Vec<Vec2>, String> {
    match layout {
        "random" => particle_points(container, count, radius, seed),
        "grid" => particle_grid_points(container, count, radius),
        "ring" => particle_ring_points(container, count, radius),
        other => Err(format!(
            "unknown particle layout `{other}` (try: random, grid, ring)"
        )),
    }
}

/// `particles(id, container, count, [radius], [seed], ["layout"])` —
/// deterministic small dots inside a circle or rectangle. `layout` is `random`
/// (default), `grid` (rectangles), or `ring` (circles). Meaning comes from the author's id
/// (`bubbles`, `dust`, `stars`, `molecules`, …), not domain-specific engine code.
fn c_particles(s: &mut Scene, a: &Args) -> Result<(), Error> {
    a.max(6)?;
    let id = a.ident(0)?;
    if s.particle_groups.contains_key(&id) || s.contains(&id) {
        return Err(Error::new(format!("`{id}` already exists"), a.span_of(0)));
    }
    let container_id = a.ident(1)?;
    let count_num = a.num(2)?;
    if !(1.0..=500.0).contains(&count_num) {
        return Err(Error::new(
            "particle count must be between 1 and 500",
            a.span_of(2),
        ));
    }
    let count = count_num.round() as usize;
    let radius = a.opt_num(3)?.unwrap_or(5.0);
    if !(0.5..=64.0).contains(&radius) {
        return Err(Error::new(
            "particle radius must be between 0.5 and 64",
            a.span_of(3),
        ));
    }
    let seed = a
        .opt_num(4)?
        .map(|v| v.max(1.0).round() as u64)
        .unwrap_or_else(|| stable_seed(&id));
    let layout = a.opt_text(5)?.unwrap_or_else(|| "random".into());
    let container = s.get(&container_id).cloned().ok_or_else(|| {
        Error::new(
            format!("no 2-D container named `{container_id}`"),
            a.span_of(1),
        )
    })?;
    let points = particle_layout_points(&container, count, radius, seed, &layout).map_err(|m| {
        Error::new(
            format!("`{container_id}` cannot contain particles: {m}"),
            a.span_of(if a.len() > 5 { 5 } else { 1 }),
        )
    })?;
    let mut children = Vec::with_capacity(count);
    for (i, p) in points.into_iter().enumerate() {
        let child = format!("{id}.p{i}");
        let mut e = Entity::new(child.clone(), Shape::Circle { r: radius }, p, style::CYAN);
        e.stroke.fill = true;
        e.stroke.outline = false;
        e.glow = 0.7;
        e.z = container.z + 1;
        e.tags.push(id.clone());
        e.tags.push(format!("{id}.particles"));
        s.add(e);
        children.push(child);
    }
    s.particle_groups.insert(
        id,
        ParticleGroup {
            container: container_id,
            children,
            radius,
            seed,
        },
    );
    Ok(())
}

fn trim_to_boundary(e: &Entity, dir_world: Vec2) -> f32 {
    match &e.shape {
        Shape::Circle { r } => r * e.scale,
        Shape::Rect { w, h } => {
            let d = rot_local(dir_world, -e.rot);
            let hw = w * e.scale * 0.5;
            let hh = h * e.scale * 0.5;
            let tx = if d.x.abs() > 1e-5 {
                hw / d.x.abs()
            } else {
                f32::INFINITY
            };
            let ty = if d.y.abs() > 1e-5 {
                hh / d.y.abs()
            } else {
                f32::INFINITY
            };
            tx.min(ty)
        }
        _ => 0.0,
    }
}

/// `link(id, a, b, [bend])` — a public, tracked std edge. It meets circle/rect
/// boundaries automatically and remains attached when either endpoint moves.
fn c_link(s: &mut Scene, a: &Args) -> Result<(), Error> {
    a.max(4)?;
    let id = a.ident(0)?;
    let from_id = a.ident(1)?;
    let to_id = a.ident(2)?;
    let bend = a.opt_num(3)?.unwrap_or(0.0);
    let from_e = s
        .get(&from_id)
        .cloned()
        .ok_or_else(|| Error::new(format!("no 2-D entity named `{from_id}`"), a.span_of(1)))?;
    let to_e = s
        .get(&to_id)
        .cloned()
        .ok_or_else(|| Error::new(format!("no 2-D entity named `{to_id}`"), a.span_of(2)))?;
    let dir = (to_e.pos - from_e.pos).normalize_or_zero();
    let trim_from = trim_to_boundary(&from_e, dir);
    let trim_to = trim_to_boundary(&to_e, -dir);
    let from = from_e.pos + dir * trim_from;
    let to = to_e.pos - dir * trim_to;
    let shape = if bend.abs() <= 1e-4 {
        Shape::Line { to }
    } else {
        let delta = to - from;
        let perp = Vec2::new(-delta.y, delta.x).normalize_or_zero();
        Shape::Curve {
            ctrl: (from + to) * 0.5 + perp * bend,
            to,
            arrow: false,
        }
    };
    let mut e = Entity::new(id, shape, from, style::FG);
    e.link = Some(Link {
        from: from_id,
        to: to_id,
        trim_from,
        trim_to,
        auto_trim: true,
        bend,
    });
    s.add(e);
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

fn shape_outline_is_closed(shape: &Shape) -> bool {
    matches!(
        shape,
        Shape::Circle { .. } | Shape::Rect { .. } | Shape::Polygon { .. } | Shape::Region { .. }
    )
}

/// `caption(id, "some words", (cx,cy), [size], [color])` — lay out the words in
/// a centred row as `{id}.w0`, `{id}.w1`, … (tagged both the bare `{id}` and
/// `{id}.words`). Animate them in sequence with `karaoke` / `wordpop`, or address
/// the whole group by the bare `id` (`show(id)`/`draw(id)`/`hidden(id)` broadcast).
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
                              // Any `$…$` math keeps the caption as ONE unit so the inline-math pass can
                              // typeset it (whole-span → equation image; mixed → RichText). A formula can't
                              // be karaoke'd word-by-word anyway, so this only forgoes word-split on
                              // math-bearing captions.
    let words: Vec<&str> = if text.contains('$') {
        vec![text.trim()]
    } else {
        text.split_whitespace().collect()
    };
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
        // Tag both the bare `{id}` (so `show`/`draw`/`hidden`/… broadcast over the
        // whole caption, like every other grouped builtin) and `{id}.words`.
        e.tags = vec![id.clone(), format!("{id}.words")];
        s.add(e);
        char_pos += len + 1;
    }
    Ok(())
}

/// `support(id, (cx,cy), [len], ["dir"])` — a **hatched fixed support**: the
/// diagonal-tick pattern that marks a wall / ceiling / floor in mechanics
/// diagrams. `len` is the baseline length in px (default 220); `"dir"` is the
/// OPEN side (where things hang / rest): `"down"` (ceiling, default), `"up"`
/// (floor), `"left"` or `"right"` (walls). Lays out the baseline `{id}.line` +
/// hatch ticks `{id}.tick{i}`, tagged bare `{id}` + `{id}.parts` so `color(id,…)`
/// and `show(id)`/`draw(id)` broadcast over the whole support.
fn c_support(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = a.pair(1)?;
    let len = a.opt_num(2)?.unwrap_or(220.0).max(12.0);
    let dir = if a.len() > 3 {
        a.text(3)?
    } else {
        "down".to_string()
    };
    // material normal — points INTO the solid, away from the open side
    let nrm = match dir.as_str() {
        "up" => Vec2::new(0.0, 1.0),     // floor: solid below the line
        "left" => Vec2::new(1.0, 0.0),   // wall, open left → solid on the right
        "right" => Vec2::new(-1.0, 0.0), // wall, open right → solid on the left
        _ => Vec2::new(0.0, -1.0),       // "down" / ceiling: solid above the line
    };
    let u = Vec2::new(-nrm.y, nrm.x); // unit along the baseline
    let (tick, spacing) = (13.0f32, 15.0f32);
    let p0 = center - u * (len / 2.0);
    let tdir = (nrm + u).normalize_or_zero() * tick; // 45° hatch into the solid

    let parts = format!("{id}.parts");
    let tags = vec![id.clone(), parts.clone()];
    let mut base = Entity::new(
        format!("{id}.line"),
        Shape::Line {
            to: center + u * (len / 2.0),
        },
        p0,
        style::FG,
    );
    base.stroke.width = 3.0;
    base.tags = tags.clone();
    s.add(base);
    let n = (len / spacing) as usize;
    for i in 0..=n {
        let bp = p0 + u * (i as f32 * spacing);
        let mut t = Entity::new(
            format!("{id}.tick{i}"),
            Shape::Line { to: bp + tdir },
            bp,
            style::FG,
        );
        t.stroke.width = 1.5;
        t.tags = tags.clone();
        s.add(t);
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
    let (mut from, from_closed) = {
        let ea = s
            .get(&ida)
            .ok_or_else(|| Error::new(format!("no entity named `{ida}`"), a.span_of(0)))?;
        (
            sample_outline(ea, MORPH_N),
            shape_outline_is_closed(&ea.shape),
        )
    };
    let (mut to, to_closed) = {
        let eb = s
            .get(&idb)
            .ok_or_else(|| Error::new(format!("no entity named `{idb}`"), a.span_of(1)))?;
        (
            sample_outline(eb, MORPH_N),
            shape_outline_is_closed(&eb.shape),
        )
    };
    // Preserve topology for path-to-path transforms. Closing an open plot or
    // line creates a visible diagonal chord halfway through the morph. Closed
    // outlines still repeat their first point so circles/polygons have no gap.
    if from_closed && to_closed {
        from.push(from[0]);
        to.push(to[0]);
    }
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

/// `parameter(id,(x,y),initial,min,max,["label"],[decimals])` — a visible,
/// bounded creator control. Animate it with the ordinary
/// `to(id,value,target,dur)` verb and connect visuals with `bind`.
fn c_parameter(s: &mut Scene, a: &Args) -> Result<(), Error> {
    a.max(7)?;
    let id = a.ident(0)?;
    let pos = a.pair(1)?;
    let initial = a.num(2)?;
    let min = a.num(3)?;
    let max = a.num(4)?;
    if !min.is_finite() || !max.is_finite() || min >= max {
        return Err(Error::new(
            "`parameter` needs a finite range with min < max",
            a.span_of(3),
        ));
    }
    if !initial.is_finite() || initial < min || initial > max {
        return Err(Error::new(
            format!("parameter initial value must be inside {min}..{max}"),
            a.span_of(2),
        ));
    }
    let label = a.opt_text(5)?.unwrap_or_else(|| id.clone());
    let decimals = a.opt_num(6)?.unwrap_or(2.0).clamp(0.0, 6.0) as u8;
    let counter = Counter {
        value: initial,
        decimals,
        prefix: format!("{label} = "),
        suffix: String::new(),
    };
    let mut readout = Entity::new(
        id.clone(),
        Shape::Text {
            content: counter.render(),
            size: 27.0,
        },
        pos,
        style::FG,
    );
    readout.counter = Some(counter);
    readout.parameter = Some(Parameter { min, max });
    readout.font = FontKind::MonoBold;
    readout.tags.push(id.clone());
    readout.tags.push(format!("{id}.widget"));
    readout.z = 3;
    s.add(readout);

    let left = Vec2::new(pos.x - 96.0, pos.y + 34.0);
    let right = Vec2::new(pos.x + 96.0, pos.y + 34.0);
    let u = (initial - min) / (max - min);
    let live = left.lerp(right, u);

    let mut track = Entity::new(
        format!("{id}.track"),
        Shape::Line { to: right },
        left,
        style::DIM,
    );
    track.stroke.width = 3.0;
    track.opacity = 0.62;
    track.glow = 0.0;
    track.tags.push(id.clone());
    track.tags.push(format!("{id}.widget"));
    s.add(track);

    let mut fill = Entity::new(
        format!("{id}.fill"),
        Shape::Line { to: live },
        left,
        style::CYAN,
    );
    fill.stroke.width = 4.5;
    fill.tags.push(id.clone());
    fill.tags.push(format!("{id}.widget"));
    fill.z = 1;
    s.add(fill);

    let mut dot = Entity::new(
        format!("{id}.dot"),
        Shape::Circle { r: 7.0 },
        live,
        style::CYAN,
    );
    dot.stroke.fill = true;
    dot.stroke.outline = false;
    dot.tags.push(id.clone());
    dot.tags.push(format!("{id}.widget"));
    dot.z = 2;
    s.add(dot);
    Ok(())
}

/// `bind(parameter,target,property,"formula")` or
/// `bind(parameter,target,property,from,to)`. Formula maps use `p` for the
/// parameter; a plot `formula` additionally uses `x` for its coordinate.
fn c_bind(s: &mut Scene, a: &Args) -> Result<(), Error> {
    a.max(5)?;
    let source = a.ident(0)?;
    let target = a.ident(1)?;
    let property_name = a.ident(2)?;
    let source_entity = s.get(&source).ok_or_else(|| {
        Error::new(
            format!("`bind` needs a parameter, but `{source}` does not exist"),
            a.span_of(0),
        )
    })?;
    let Some(_parameter) = source_entity.parameter else {
        return Err(Error::new(
            format!("`{source}` is not a `parameter`"),
            a.span_of(0),
        ));
    };
    let initial = source_entity
        .counter
        .as_ref()
        .map(|c| c.value)
        .unwrap_or(0.0);
    let target_entity = s.get(&target).ok_or_else(|| {
        Error::new(
            format!("`bind` currently needs a 2-D entity; no entity named `{target}`"),
            a.span_of(1),
        )
    })?;
    if source == target {
        return Err(Error::new(
            "a parameter cannot bind to itself",
            a.span_of(1),
        ));
    }

    let property = match property_name.as_str() {
        "x" => BoundProperty::X,
        "y" => BoundProperty::Y,
        "opacity" | "alpha" => BoundProperty::Opacity,
        "scale" => BoundProperty::Scale,
        "angle" | "rot" | "rotation" => BoundProperty::Rot,
        "hue" => BoundProperty::Hue,
        "value" | "count" => BoundProperty::Value,
        "trace" => BoundProperty::Trace,
        "formula" | "plot" => BoundProperty::Formula,
        other => {
            return Err(Error::new(
                format!("can't bind property `{other}` (try: x, y, opacity, scale, angle, hue, value, trace, formula)"),
                a.span_of(2),
            ))
        }
    };
    if property == BoundProperty::Formula && target_entity.graph.is_none() {
        return Err(Error::new(
            format!("`{target}` is not a plot; `formula` bindings target a `plot` entity"),
            a.span_of(1),
        ));
    }
    if property == BoundProperty::Value {
        if target_entity.counter.is_none() {
            return Err(Error::new(
                format!("`{target}` has no live numeric `value`; use a `counter`"),
                a.span_of(1),
            ));
        }
        if target_entity.parameter.is_some() {
            return Err(Error::new(
                "bind visuals to a parameter; do not chain one parameter into another",
                a.span_of(1),
            ));
        }
    }

    let map = if a.len() == 4 {
        let formula = a.text(3)?;
        let node = crate::kits::math::expr::compile(&formula)
            .map_err(|message| Error::new(format!("in bind formula: {message}"), a.span_of(3)))?;
        if property != BoundProperty::Formula {
            let value = node.eval(0.0, initial);
            if !value.is_finite() {
                return Err(Error::new(
                    "bind formula is not finite at the parameter's initial value",
                    a.span_of(3),
                ));
            }
        }
        ParameterMap::Formula(node)
    } else if a.len() == 5 {
        if property == BoundProperty::Formula {
            return Err(Error::new(
                "a plot formula binding needs a string such as `\"p*x^2\"`",
                a.span_of(3),
            ));
        }
        let from = a.num(3)?;
        let to = a.num(4)?;
        if !from.is_finite() || !to.is_finite() {
            return Err(Error::new(
                "bind range endpoints must be finite",
                a.span_of(3),
            ));
        }
        ParameterMap::Range { from, to }
    } else {
        return Err(Error::new(
            "`bind` needs either one formula string or two numeric range endpoints",
            a.name_span,
        ));
    };

    if s.parameter_bindings
        .iter()
        .any(|binding| binding.target == target && binding.property == property)
    {
        return Err(Error::new(
            format!("`{target}.{property_name}` already has a parameter binding"),
            a.span_of(2),
        ));
    }
    s.parameter_bindings.push(ParameterBinding {
        source,
        target,
        property,
        map,
    });
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

/// `image(id, (x,y), "asset:name.png"|"path", [w], [h])` — a raster image
/// (PNG/JPG) centred on `(x,y)`, drawn `w`×`h` px (default 300 square; `h`
/// defaults to `w`). Loaded once at render start; animate it like any entity
/// (`show`/`move`/`fade`/…). A missing ordinary file draws a crossed placeholder.
fn c_image(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let pos = a.pair(1)?;
    let path = a.text(2)?;
    let path = crate::assets::resolve(&path)
        .map_err(|message| Error::new(message, a.span_of(2)))?
        .to_string_lossy()
        .into_owned();
    let w = a.opt_num(3)?.unwrap_or(300.0).max(1.0);
    let h = a.opt_num(4)?.unwrap_or(w).max(1.0);
    s.add(Entity::new(
        id,
        Shape::Image {
            path,
            w,
            h,
            tint: false,
        },
        pos,
        style::FG,
    ));
    Ok(())
}

/// `equation(id, (x,y), "latex", [size])` — typeset a LaTeX math string (real
/// fractions/roots/exponents/Greek via RaTeX) centred at `(x,y)`. `size` is the
/// em height in px (default 48). Rendered white-on-transparent and drawn tinted
/// by the entity colour, so it takes the template palette and `color`/`recolor`
/// work. Standard `\textcolor{name}{...}` can colour individual terms with a
/// Manic semantic colour; those roles are remapped through the active template.
/// Animate with `show`/`fade`/`move`/`scale` (it's an image, so no `draw`).
fn c_equation(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let pos = a.pair(1)?;
    let latex = a.text(2)?;
    let size = a.opt_num(3)?.unwrap_or(48.0).clamp(6.0, 400.0);
    // Layout now (cheap); the player rasterises at the render scale (pixel-sharp).
    let (w, h, _baseline) = crate::latex::layout_dims(&latex, size)
        .map_err(|e| Error::new(format!("equation: {e}"), a.span_of(2)))?;
    let path = crate::latex::eq_path(&latex, size);
    let tint = !crate::latex::has_explicit_color(&latex);
    s.pending_eqs.push((path.clone(), latex.clone(), size));
    s.equation_states.insert(
        id.clone(),
        EquationState {
            latex,
            size,
            visual_scale: 1.0,
            rewrite_n: 0,
        },
    );
    s.add(Entity::new(
        id,
        Shape::Image { path, w, h, tint },
        pos,
        style::FG,
    ));
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

/// `wrap(id, width)` — wrap a text/caption/`$…$` label to `width` px (breaks at
/// word boundaries; inline math stays atomic). Without it, text is a single line.
fn m_wrap(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let w = a.num(1)?.max(1.0);
    ent_mut(s, a)?.wrap = Some(w);
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

/// `dashed(id, [dash], [gap])` — render a path-like entity with a repeating
/// dash/gap pattern in logical pixels (defaults 16/10). This is deliberately a
/// base Manic modifier: plots use it, but so do links, trajectories, arrows,
/// curves, splines, coils, and plain arcs.
fn m_dashed(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let dash = a.opt_num(1)?.unwrap_or(16.0);
    let gap = a.opt_num(2)?.unwrap_or(10.0);
    if dash <= 0.0 || gap <= 0.0 {
        return Err(Error::new(
            "dashed lengths must be positive",
            a.span_of(if dash <= 0.0 { 1 } else { 2 }),
        ));
    }
    let e = s
        .get_mut(&id)
        .ok_or_else(|| Error::new(format!("no entity named `{id}`"), a.span_of(0)))?;
    if !matches!(
        e.shape,
        Shape::Line { .. }
            | Shape::Arrow { .. }
            | Shape::Curve { .. }
            | Shape::Coil { .. }
            | Shape::Polyline { .. }
            | Shape::Arc { .. }
    ) {
        return Err(Error::new(
            format!("`{id}` is not a path-like entity"),
            a.span_of(0),
        ));
    }
    e.dash = Some((dash, gap));
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
    let mut matched = false;
    for entity in &mut s.entities {
        if entity.id == id || entity.tags.iter().any(|tag| tag == &id) {
            entity.trace = 0.0;
            matched = true;
        }
    }
    if matched {
        return Ok(());
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

/// `sticky(id)` — pin an entity to screen coordinates so it stays fixed while the
/// camera pans or zooms (a HUD overlay). Use for captions / counters / readouts
/// that must stay readable through a `cam`/`zoom` move. Broadcasts over a tag.
fn m_sticky(s: &mut Scene, a: &Args) -> Result<(), Error> {
    ent_mut(s, a)?.sticky = true;
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
    let to = authored_point(a, 1, s)?;
    Ok(apply_dur_ease(act().move_to(&id, to), a, 2)?.into())
}

fn v_shift(_s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let by = a.pair(1)?;
    Ok(apply_dur_ease(act().move_by(&id, by), a, 2)?.into())
}

fn v_grow(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let to = authored_point(a, 1, s)?;
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
    let (prop, target) = if let Some(cur) = s.authored_entity(&id) {
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
        .authored_entity(&id)
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
    // Transform the current authored endpoint, not the constructor position,
    // so ordinary movement and stateful layouts compose without snapping.
    let mut tracks = vec![track(Prop::Pos, e.pos)];
    if let Shape::Line { to }
    | Shape::Arrow { to }
    | Shape::Curve { to, .. }
    | Shape::Coil { to, .. } = &e.shape
    {
        tracks.push(track(Prop::To, *to));
    }
    if let Shape::Curve { ctrl, .. } = &e.shape {
        tracks.push(track(Prop::Ctrl, *ctrl));
    }
    Ok(Clip {
        dur,
        tracks,
        events: Vec::new(),
    })
}

fn rewrite_track(
    id: impl Into<String>,
    prop: Prop,
    target: TargetValue,
    start: f32,
    dur: f32,
    easing: Easing,
) -> TrackSpec {
    TrackSpec {
        id: id.into(),
        prop,
        target,
        start,
        dur,
        easing,
    }
}

#[derive(Debug, Clone)]
struct EquationMatchPlan {
    pairs: Vec<(usize, usize)>,
    source_coverage: f32,
    target_coverage: f32,
    mean_travel: f32,
    inversion_ratio: f32,
}

#[derive(Debug, Clone, Copy)]
struct SequenceScore {
    matches: u16,
    cost: f32,
    step: u8,
}

fn better_sequence_score(a: SequenceScore, b: SequenceScore) -> bool {
    a.matches > b.matches || (a.matches == b.matches && a.cost < b.cost - 1e-6)
}

fn equation_part_area(part: &crate::latex::EquationPart) -> f32 {
    (part.crop.w * part.crop.h).max(1.0)
}

fn equation_match_cost(
    source: &crate::latex::EquationPart,
    target: &crate::latex::EquationPart,
    diagonal: f32,
) -> f32 {
    let distance = source.offset.distance(target.offset) / diagonal.max(1.0);
    let row_shift = (source.offset.y - target.offset.y).abs() / diagonal.max(1.0);
    let scale_shift = if source.crop.h > 0.0 && target.crop.h > 0.0 {
        (source.crop.h / target.crop.h).ln().abs()
    } else {
        0.0
    };
    let mut cost = distance + row_shift * 0.85 + scale_shift * 0.18;
    if source.prev_key != target.prev_key {
        cost += 0.08;
    }
    if source.next_key != target.next_key {
        cost += 0.08;
    }
    cost
}

fn equation_parts_can_match(
    source: &crate::latex::EquationPart,
    target: &crate::latex::EquationPart,
) -> bool {
    if source.key != target.key || source.role != target.role {
        return false;
    }
    match (source.layout_scale, target.layout_scale) {
        (Some(a), Some(b)) if a > 0.0 && b > 0.0 => {
            // RaTeX uses discrete math styles for scripts. A little tolerance
            // admits equivalent layouts while keeping adjacent nesting levels
            // (normally about 0.7x apart) semantically distinct.
            (a / b).ln().abs() <= 1.18_f32.ln()
        }
        _ => true,
    }
}

/// Match the longest order-preserving visual sequence, using movement and
/// neighbouring context only as tie-breakers. This is deliberately different
/// from greedy nearest-glyph matching: repeated zeros, brackets, and variables
/// retain reading order instead of crossing each other unpredictably.
fn ordered_equation_matches(
    from: &[crate::latex::EquationPart],
    to: &[crate::latex::EquationPart],
) -> Vec<(usize, usize)> {
    let n = from.len();
    let m = to.len();
    let diagonal = from
        .iter()
        .chain(to.iter())
        .map(|p| p.offset.length())
        .fold(1.0_f32, f32::max)
        * 2.0;
    let mut dp = vec![
        SequenceScore {
            matches: 0,
            cost: 0.0,
            step: 0,
        };
        (n + 1) * (m + 1)
    ];
    let at = |i: usize, j: usize| i * (m + 1) + j;

    for i in 1..=n {
        for j in 1..=m {
            let mut best = dp[at(i - 1, j)];
            best.step = 1; // skip source
            let mut skip_target = dp[at(i, j - 1)];
            skip_target.step = 2;
            if better_sequence_score(skip_target, best) {
                best = skip_target;
            }
            if equation_parts_can_match(&from[i - 1], &to[j - 1]) {
                let prev = dp[at(i - 1, j - 1)];
                let matched = SequenceScore {
                    matches: prev.matches.saturating_add(1),
                    cost: prev.cost + equation_match_cost(&from[i - 1], &to[j - 1], diagonal),
                    step: 3,
                };
                if better_sequence_score(matched, best) {
                    best = matched;
                }
            }
            dp[at(i, j)] = best;
        }
    }

    let mut pairs = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        match dp[at(i, j)].step {
            3 if i > 0 && j > 0 => {
                pairs.push((i - 1, j - 1));
                i -= 1;
                j -= 1;
            }
            1 if i > 0 => i -= 1,
            2 if j > 0 => j -= 1,
            _ if i > 0 => i -= 1,
            _ if j > 0 => j -= 1,
            _ => break,
        }
    }
    pairs.reverse();
    pairs
}

/// Build a continuity-biased match plan. The ordered pass protects unchanged
/// subexpressions and repeated entries. A second pass permits a genuinely
/// unique symbol to cross an equals sign, which keeps algebraic moves alive
/// without allowing every repeated `0` or `x` to teleport.
fn match_equation_parts(
    from: &crate::latex::EquationLayout,
    to: &crate::latex::EquationLayout,
) -> EquationMatchPlan {
    use std::collections::HashMap;

    let mut pairs = ordered_equation_matches(&from.parts, &to.parts);
    let mut source_used = vec![false; from.parts.len()];
    let mut target_used = vec![false; to.parts.len()];
    for &(si, ti) in &pairs {
        source_used[si] = true;
        target_used[ti] = true;
    }

    let mut remaining_from: HashMap<(&str, crate::latex::EquationPartRole), Vec<usize>> =
        HashMap::new();
    let mut remaining_to: HashMap<(&str, crate::latex::EquationPartRole), Vec<usize>> =
        HashMap::new();
    for (i, part) in from.parts.iter().enumerate() {
        if !source_used[i] {
            remaining_from
                .entry((&part.key, part.role))
                .or_default()
                .push(i);
        }
    }
    for (i, part) in to.parts.iter().enumerate() {
        if !target_used[i] {
            remaining_to
                .entry((&part.key, part.role))
                .or_default()
                .push(i);
        }
    }
    for (key, source_indices) in remaining_from {
        let Some(target_indices) = remaining_to.get(&key) else {
            continue;
        };
        if source_indices.len() == 1 && target_indices.len() == 1 {
            let si = source_indices[0];
            let ti = target_indices[0];
            if !equation_parts_can_match(&from.parts[si], &to.parts[ti]) {
                continue;
            }
            source_used[si] = true;
            target_used[ti] = true;
            pairs.push((si, ti));
        }
    }
    pairs.sort_unstable();

    let source_total: f32 = from.parts.iter().map(equation_part_area).sum();
    let target_total: f32 = to.parts.iter().map(equation_part_area).sum();
    let source_matched: f32 = pairs
        .iter()
        .map(|(si, _)| equation_part_area(&from.parts[*si]))
        .sum();
    let target_matched: f32 = pairs
        .iter()
        .map(|(_, ti)| equation_part_area(&to.parts[*ti]))
        .sum();
    let diagonal = (from.w.max(to.w).powi(2) + from.h.max(to.h).powi(2))
        .sqrt()
        .max(1.0);
    let travel_weight: f32 = pairs
        .iter()
        .map(|(si, ti)| {
            from.parts[*si].offset.distance(to.parts[*ti].offset)
                * equation_part_area(&from.parts[*si]).min(equation_part_area(&to.parts[*ti]))
        })
        .sum();
    let matched_weight: f32 = pairs
        .iter()
        .map(|(si, ti)| {
            equation_part_area(&from.parts[*si]).min(equation_part_area(&to.parts[*ti]))
        })
        .sum();
    let inversions = pairs
        .iter()
        .enumerate()
        .map(|(i, (_, ti))| {
            pairs[i + 1..]
                .iter()
                .filter(|(_, later_ti)| later_ti < ti)
                .count()
        })
        .sum::<usize>();
    let possible_inversions = pairs.len().saturating_mul(pairs.len().saturating_sub(1)) / 2;

    EquationMatchPlan {
        pairs,
        source_coverage: source_matched / source_total.max(1.0),
        target_coverage: target_matched / target_total.max(1.0),
        mean_travel: travel_weight / matched_weight.max(1.0) / diagonal,
        inversion_ratio: inversions as f32 / possible_inversions.max(1) as f32,
    }
}

fn latex_has_matrix(latex: &str) -> bool {
    [
        "\\begin{matrix}",
        "\\begin{pmatrix}",
        "\\begin{bmatrix}",
        "\\begin{Bmatrix}",
        "\\begin{vmatrix}",
        "\\begin{Vmatrix}",
    ]
    .iter()
    .any(|needle| latex.contains(needle))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LatexStructure {
    fractions: usize,
    radicals: usize,
    scalable_left: usize,
    scalable_right: usize,
    arrays: usize,
}

fn latex_structure(latex: &str) -> LatexStructure {
    LatexStructure {
        fractions: latex.matches("\\frac").count(),
        radicals: latex.matches("\\sqrt").count(),
        scalable_left: latex.matches("\\left").count(),
        scalable_right: latex.matches("\\right").count(),
        arrays: [
            "\\begin{matrix}",
            "\\begin{pmatrix}",
            "\\begin{bmatrix}",
            "\\begin{Bmatrix}",
            "\\begin{vmatrix}",
            "\\begin{Vmatrix}",
        ]
        .iter()
        .map(|needle| latex.matches(needle).count())
        .sum(),
    }
}

fn latex_structure_is_close(source: &str, target: &str) -> bool {
    let a = latex_structure(source);
    let b = latex_structure(target);
    a.radicals == b.radicals
        && a.arrays == b.arrays
        && a.fractions.abs_diff(b.fractions) <= 1
        && a.scalable_left.abs_diff(b.scalable_left) <= 2
        && a.scalable_right.abs_diff(b.scalable_right) <= 2
}

fn top_level_equation_sides(latex: &str) -> Option<(&str, &str)> {
    let mut depth = 0_i32;
    let mut relation = None;
    for (index, ch) in latex.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = (depth - 1).max(0),
            '=' if depth == 0 => {
                if relation.is_some() {
                    return None;
                }
                relation = Some(index);
            }
            _ => {}
        }
    }
    let index = relation?;
    Some((&latex[..index], &latex[index + 1..]))
}

fn compact_latex(latex: &str) -> String {
    latex.chars().filter(|ch| !ch.is_whitespace()).collect()
}

/// When either side of an equality changes mathematical topology, retain the
/// relation and every structurally compatible side, but dissolve the changed
/// side locally. This avoids pretending that every glyph in a factorisation or
/// newly introduced fraction has a meaningful one-to-one destination.
fn prefer_local_side_dissolve(
    plan: &mut EquationMatchPlan,
    from: &crate::latex::EquationLayout,
    to: &crate::latex::EquationLayout,
    source_latex: &str,
    target_latex: &str,
) -> bool {
    let Some((source_left, source_right)) = top_level_equation_sides(source_latex) else {
        return false;
    };
    let Some((target_left, target_right)) = top_level_equation_sides(target_latex) else {
        return false;
    };
    let left_changed = compact_latex(source_left) != compact_latex(target_left)
        && latex_structure(source_left) != latex_structure(target_left);
    let right_changed = compact_latex(source_right) != compact_latex(target_right)
        && latex_structure(source_right) != latex_structure(target_right);
    if !left_changed && !right_changed {
        return false;
    }

    let source_equals: Vec<_> = from
        .parts
        .iter()
        .filter(|part| part.symbol == Some('=' as u32))
        .collect();
    let target_equals: Vec<_> = to
        .parts
        .iter()
        .filter(|part| part.symbol == Some('=' as u32))
        .collect();
    if source_equals.len() != 1 || target_equals.len() != 1 {
        return false;
    }
    let source_x = source_equals[0].offset.x;
    let target_x = target_equals[0].offset.x;
    plan.pairs.retain(|(si, ti)| {
        let source_changed = (left_changed && from.parts[*si].offset.x < source_x - 1.0)
            || (right_changed && from.parts[*si].offset.x > source_x + 1.0);
        let target_changed = (left_changed && to.parts[*ti].offset.x < target_x - 1.0)
            || (right_changed && to.parts[*ti].offset.x > target_x + 1.0);
        !source_changed && !target_changed
    });
    true
}

fn use_structured_equation_rewrite(
    plan: &EquationMatchPlan,
    source_latex: &str,
    target_latex: &str,
) -> bool {
    let coverage = plan.source_coverage.min(plan.target_coverage);
    let matrix_topology_changed = latex_has_matrix(source_latex) != latex_has_matrix(target_latex);
    !matrix_topology_changed
        && plan.pairs.len() >= 2
        && coverage >= 0.38
        && plan.mean_travel <= 0.62
        && plan.inversion_ratio <= 0.22
        && latex_structure_is_close(source_latex, target_latex)
}

fn queue_equation_part(
    s: &mut Scene,
    source: &Entity,
    latex: &str,
    size: f32,
    part: &crate::latex::EquationPart,
    id: String,
    anchor: Vec2,
    visual_scale: f32,
) {
    if !s.pending_eq_parts.iter().any(|p| p.path == part.path) {
        s.pending_eq_parts.push(PendingEquationPart {
            path: part.path.clone(),
            latex: latex.to_string(),
            size,
            index: part.index,
            crop: part.crop,
        });
    }
    let tint = !crate::latex::has_explicit_color(latex);
    let mut entity = Entity::new(
        id,
        Shape::Image {
            path: part.path.clone(),
            w: part.crop.w,
            h: part.crop.h,
            tint,
        },
        anchor + part.offset * visual_scale,
        source.color,
    );
    entity.opacity = 0.0;
    entity.scale = visual_scale;
    entity.z = source.z + 1;
    entity.sticky = source.sticky;
    entity.rot = source.rot;
    entity.glow = source.glow;
    s.add(entity);
}

fn queue_equation_target(
    s: &mut Scene,
    source: &Entity,
    id: String,
    target_shape: Shape,
    target_scale: f32,
) {
    let mut target = source.clone();
    target.id = id;
    target.shape = target_shape;
    target.opacity = 0.0;
    target.scale = target_scale * 0.985;
    target.z = source.z + 1;
    // Internal handoff layers must never join later tag broadcasts from the
    // public equation (for example `recolor("proof", ...)`).
    target.tags.clear();
    s.add(target);
}

fn fallback_equation_rewrite(
    s: &mut Scene,
    source: &Entity,
    id: String,
    target_shape: Shape,
    target_scale: f32,
    dur: f32,
    easing: Easing,
    serial: usize,
) -> Clip {
    let target_id = format!("__rewrite.{id}.{serial}.target");
    queue_equation_target(
        s,
        source,
        target_id.clone(),
        target_shape.clone(),
        target_scale,
    );
    Clip {
        dur,
        tracks: vec![
            // Use a short, dim overlap rather than superimposing two complete
            // equations at equal strength. The target is already visible when
            // the source leaves, so continuity survives without a ghosted
            // midpoint.
            rewrite_track(
                id.clone(),
                Prop::Opacity,
                TargetValue::Abs(Value::F(0.0)),
                0.0,
                dur * 0.45,
                Easing::InQuad,
            ),
            rewrite_track(
                target_id.clone(),
                Prop::Opacity,
                TargetValue::Abs(Value::F(1.0)),
                dur * 0.30,
                dur * 0.70,
                Easing::OutQuad,
            ),
            rewrite_track(
                id.clone(),
                Prop::Scale,
                TargetValue::Abs(Value::F(target_scale)),
                0.0,
                dur,
                easing,
            ),
            rewrite_track(
                target_id.clone(),
                Prop::Scale,
                TargetValue::Abs(Value::F(target_scale)),
                0.0,
                dur,
                easing,
            ),
            rewrite_track(
                id.clone(),
                Prop::Opacity,
                TargetValue::Abs(Value::F(1.0)),
                dur,
                0.0,
                Easing::Linear,
            ),
            rewrite_track(
                target_id,
                Prop::Opacity,
                TargetValue::Abs(Value::F(0.0)),
                dur,
                0.0,
                Easing::Linear,
            ),
        ],
        events: vec![TimelineEvent::shape(id, target_shape, dur)],
    }
}

/// ``rewrite(id, `next latex`, [dur], [ease])`` — opt-in structured LaTeX
/// transformation. The authored formulas remain the source of mathematical
/// truth; Manic only animates the visual difference between two exact RaTeX
/// layouts. Existing equation behavior is untouched when this verb is unused.
fn v_rewrite(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    a.max(4)?;
    let id = a.ident(0)?;
    let target_latex = a.text(1)?;
    let dur = a.opt_num(2)?.unwrap_or(0.9).max(0.05);
    let easing = if a.len() > 3 {
        resolve_easing(&a.ident(3)?, a.span_of(3))?
    } else {
        Easing::InOutCubic
    };
    let state = s.equation_states.get(&id).cloned().ok_or_else(|| {
        Error::new(
            format!("`rewrite` needs an equation id; `{id}` is not an equation"),
            a.span_of(0),
        )
    })?;
    let source = s
        .authored_entity(&id)
        .ok_or_else(|| Error::new(format!("no equation named `{id}`"), a.span_of(0)))?;
    if !matches!(source.shape, Shape::Image { .. }) {
        return Err(Error::new(
            format!("`{id}` is no longer an equation image"),
            a.span_of(0),
        ));
    }

    // Validate and queue the exact settled target first. A bad target is an
    // authoring error; visual matching failure later is recoverable by fallback.
    let (target_w, target_h, _) = crate::latex::layout_dims(&target_latex, state.size)
        .map_err(|e| Error::new(format!("rewrite: {e}"), a.span_of(1)))?;
    let target_path = crate::latex::eq_path(&target_latex, state.size);
    if !s.pending_eqs.iter().any(|(p, _, _)| p == &target_path) {
        s.pending_eqs
            .push((target_path.clone(), target_latex.clone(), state.size));
    }
    let target_shape = Shape::Image {
        path: target_path,
        w: target_w,
        h: target_h,
        tint: !crate::latex::has_explicit_color(&target_latex),
    };

    // Keep the formula inside a conservative, format-independent equation
    // region. Scale only ever stays fixed or shrinks across a chain, avoiding
    // distracting size "breathing" between short and long steps.
    let canvas = s.canvas();
    let max_w = canvas.x * 0.90;
    let max_h = canvas.y * 0.46;
    let (source_w, source_h, _) = crate::latex::layout_dims(&state.latex, state.size)
        .map_err(|e| Error::new(format!("rewrite source: {e}"), a.span_of(0)))?;
    let target_scale = state
        .visual_scale
        .min(max_w / source_w.max(target_w).max(1.0))
        .min(max_h / source_h.max(target_h).max(1.0))
        .clamp(0.16, 1.0);

    let source_layout = crate::latex::layout_parts(&state.latex, state.size);
    let target_layout = crate::latex::layout_parts(&target_latex, state.size);
    let serial = state.rewrite_n + 1;

    let clip = match (source_layout, target_layout) {
        (Ok(from), Ok(to)) if from.parts.len() + to.parts.len() <= 256 => {
            let mut plan = match_equation_parts(&from, &to);
            let local_side_dissolve =
                prefer_local_side_dissolve(&mut plan, &from, &to, &state.latex, &target_latex);
            if !local_side_dissolve
                && !use_structured_equation_rewrite(&plan, &state.latex, &target_latex)
            {
                fallback_equation_rewrite(
                    s,
                    &source,
                    id.clone(),
                    target_shape.clone(),
                    target_scale,
                    dur,
                    easing,
                    serial,
                )
            } else {
                let mut target_of = vec![None; from.parts.len()];
                let mut target_used = vec![false; to.parts.len()];
                for (si, ti) in plan.pairs {
                    target_of[si] = Some(ti);
                    target_used[ti] = true;
                }
                let has_unmatched_source = target_of.iter().any(Option::is_none);
                let has_unmatched_target = target_used.iter().any(|used| !used);
                let stage_replacements =
                    local_side_dissolve || (has_unmatched_source && has_unmatched_target);

                let mut tracks = vec![
                    // Exact handoff: replace the whole formula with its parts at
                    // t=0, then restore the same public entity at the final frame.
                    rewrite_track(
                        id.clone(),
                        Prop::Opacity,
                        TargetValue::Abs(Value::F(0.0)),
                        0.0,
                        0.0,
                        Easing::Linear,
                    ),
                    rewrite_track(
                        id.clone(),
                        Prop::Opacity,
                        TargetValue::Abs(Value::F(1.0)),
                        dur,
                        0.0,
                        Easing::Linear,
                    ),
                    rewrite_track(
                        id.clone(),
                        Prop::Scale,
                        TargetValue::Abs(Value::F(target_scale)),
                        0.0,
                        dur,
                        easing,
                    ),
                ];

                for (si, part) in from.parts.iter().enumerate() {
                    let part_id = format!("__rewrite.{id}.{serial}.from.{si}");
                    queue_equation_part(
                        s,
                        &source,
                        &state.latex,
                        state.size,
                        part,
                        part_id.clone(),
                        source.pos,
                        state.visual_scale,
                    );
                    tracks.push(rewrite_track(
                        part_id.clone(),
                        Prop::Opacity,
                        TargetValue::Abs(Value::F(1.0)),
                        0.0,
                        0.0,
                        Easing::Linear,
                    ));
                    if let Some(ti) = target_of[si] {
                        let target = &to.parts[ti];
                        let target_pos = source.pos + target.offset * target_scale;
                        let intrinsic_scale = if part.crop.h > 0.0 {
                            target.crop.h / part.crop.h
                        } else {
                            1.0
                        };
                        tracks.push(rewrite_track(
                            part_id.clone(),
                            Prop::Pos,
                            TargetValue::Abs(Value::V(target_pos)),
                            0.0,
                            dur,
                            easing,
                        ));
                        tracks.push(rewrite_track(
                            part_id.clone(),
                            Prop::Scale,
                            TargetValue::Abs(Value::F(target_scale * intrinsic_scale)),
                            0.0,
                            dur,
                            easing,
                        ));
                        tracks.push(rewrite_track(
                            part_id,
                            Prop::Opacity,
                            TargetValue::Abs(Value::F(0.0)),
                            dur,
                            0.0,
                            Easing::Linear,
                        ));
                    } else {
                        let fade_out_duration = if stage_replacements {
                            dur * 0.45
                        } else {
                            dur * 0.72
                        };
                        tracks.push(rewrite_track(
                            part_id,
                            Prop::Opacity,
                            TargetValue::Abs(Value::F(0.0)),
                            0.0,
                            fade_out_duration,
                            if stage_replacements {
                                Easing::InQuad
                            } else {
                                Easing::Linear
                            },
                        ));
                    }
                }

                for (ti, part) in to.parts.iter().enumerate() {
                    if target_used[ti] {
                        continue;
                    }
                    let part_id = format!("__rewrite.{id}.{serial}.to.{ti}");
                    queue_equation_part(
                        s,
                        &source,
                        &target_latex,
                        state.size,
                        part,
                        part_id.clone(),
                        source.pos,
                        target_scale,
                    );
                    tracks.push(rewrite_track(
                        part_id.clone(),
                        Prop::Opacity,
                        TargetValue::Abs(Value::F(1.0)),
                        if stage_replacements { dur * 0.30 } else { 0.0 },
                        if stage_replacements {
                            dur * 0.70
                        } else {
                            dur * 0.72
                        },
                        if stage_replacements {
                            Easing::OutQuad
                        } else {
                            Easing::Linear
                        },
                    ));
                    tracks.push(rewrite_track(
                        part_id,
                        Prop::Opacity,
                        TargetValue::Abs(Value::F(0.0)),
                        dur,
                        0.0,
                        Easing::Linear,
                    ));
                }

                Clip {
                    dur,
                    tracks,
                    events: vec![TimelineEvent::shape(id.clone(), target_shape.clone(), dur)],
                }
            }
        }
        _ => fallback_equation_rewrite(
            s,
            &source,
            id.clone(),
            target_shape.clone(),
            target_scale,
            dur,
            easing,
            serial,
        ),
    };

    s.equation_states.insert(
        id,
        EquationState {
            latex: target_latex,
            size: state.size,
            visual_scale: target_scale,
            rewrite_n: serial,
        },
    );
    Ok(clip)
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
        .motion_pos
        .get(&id0)
        .copied()
        .or_else(|| s.get(&id0).map(|e| e.pos))
        .ok_or_else(|| Error::new(format!("no entity named `{id0}`"), a.span_of(0)))?;
    let pb = s
        .motion_pos
        .get(&idb)
        .copied()
        .or_else(|| s.get(&idb).map(|e| e.pos))
        .ok_or_else(|| Error::new(format!("no entity named `{idb}`"), a.span_of(1)))?;
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
    s.motion_pos.insert(id0.clone(), pb);
    s.motion_pos.insert(idb.clone(), pa);
    Ok(Clip {
        dur,
        tracks: vec![track(id0, pb), track(idb, pa)],
        events: Vec::new(),
    })
}

/// Point on the signed circular arc from `from` to `to`. `sweep` is radians;
/// zero degenerates to a straight line. Positive/negative sweeps bend to
/// opposite sides, which naturally sends a two-object cycle around both sides.
fn arc_point(from: Vec2, to: Vec2, sweep: f32, u: f32) -> Vec2 {
    let chord = to - from;
    let len = chord.length();
    if len < 1e-5 || sweep.abs() < 1e-4 {
        return from.lerp(to, u);
    }
    let half_tan = (sweep * 0.5).tan();
    if half_tan.abs() < 1e-5 {
        return from.lerp(to, u);
    }
    let perp = Vec2::new(-chord.y, chord.x) / len;
    let centre = (from + to) * 0.5 + perp * (len / (2.0 * half_tan));
    let start = from - centre;
    let angle = sweep * u;
    let (sn, cs) = angle.sin_cos();
    centre + Vec2::new(start.x * cs - start.y * sn, start.x * sn + start.y * cs)
}

/// `cycle(a, b, c, ..., [dur], [arc_deg], [ease])` — move every entity into
/// the next one's position, with the last returning to the first. The default
/// 90-degree path arc is the compact Manic equivalent of Manim CyclicReplace.
fn v_cycle(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let first_num = a
        .exprs
        .iter()
        .position(|e| matches!(&e.kind, ExprKind::Num(_)))
        .unwrap_or(a.len());
    if first_num < 2 {
        return Err(Error::new(
            "cycle needs at least two entity names",
            a.name_span,
        ));
    }

    let mut ids = Vec::with_capacity(first_num);
    for i in 0..first_num {
        ids.push(a.ident(i)?);
    }
    let tail = &a.exprs[first_num..];
    if tail.len() > 3 {
        return Err(Error::new(
            "cycle tail is [duration], [arc degrees], [ease]",
            tail[3].span,
        ));
    }
    let dur = match tail.first() {
        Some(e) => match &e.kind {
            ExprKind::Num(n) if *n > 0.0 => *n,
            ExprKind::Num(_) => return Err(Error::new("cycle duration must be positive", e.span)),
            _ => return Err(Error::new("cycle duration should be a number", e.span)),
        },
        None => 0.8,
    };
    let arc_deg = match tail.get(1) {
        Some(e) => match &e.kind {
            ExprKind::Num(n) => *n,
            _ => return Err(Error::new("cycle arc should be degrees", e.span)),
        },
        None => 90.0,
    };
    let easing = match tail.get(2) {
        Some(e) => match &e.kind {
            ExprKind::Ident(name) => resolve_easing(name, e.span)?,
            _ => return Err(Error::new("cycle easing should be a name", e.span)),
        },
        None => Easing::InOutCubic,
    };

    let mut from = Vec::with_capacity(ids.len());
    for (i, id) in ids.iter().enumerate() {
        let p = s
            .motion_pos
            .get(id)
            .copied()
            .or_else(|| s.get(id).map(|e| e.pos))
            .ok_or_else(|| Error::new(format!("no entity named `{id}`"), a.span_of(i)))?;
        from.push(p);
    }
    let targets: Vec<Vec2> = (0..ids.len()).map(|i| from[(i + 1) % from.len()]).collect();
    let sweep = arc_deg.to_radians();
    let segments = if sweep.abs() < 1e-4 { 1 } else { 12 };
    let mut tracks = Vec::with_capacity(ids.len() * segments);
    for (id, (&p0, &p1)) in ids.iter().zip(from.iter().zip(targets.iter())) {
        for k in 1..=segments {
            let raw_u = k as f32 / segments as f32;
            let u = easing.apply(raw_u);
            tracks.push(TrackSpec {
                id: id.clone(),
                prop: Prop::Pos,
                target: TargetValue::Abs(Value::V(arc_point(p0, p1, sweep, u))),
                start: dur * (k - 1) as f32 / segments as f32,
                dur: dur / segments as f32,
                easing: Easing::Linear,
            });
        }
    }
    for (id, target) in ids.iter().zip(targets) {
        s.motion_pos.insert(id.clone(), target);
    }
    Ok(Clip {
        dur,
        tracks,
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

/// Sample a drawable path in scene coordinates. Keeping this build-time and
/// turning it into ordinary position tracks preserves Manic's stateless
/// preview/scrubbing contract.
fn travel_path_points(path: &Entity) -> Option<Vec<Vec2>> {
    let rotate_about = |point: Vec2| path.pos + rot_local(point - path.pos, path.rot);
    Some(match &path.shape {
        Shape::Line { to } | Shape::Arrow { to } => {
            vec![path.pos, rotate_about(*to)]
        }
        Shape::Curve { ctrl, to, .. } => (0..=64)
            .map(|i| {
                let u = i as f32 / 64.0;
                let v = 1.0 - u;
                path.pos * (v * v)
                    + rotate_about(*ctrl) * (2.0 * v * u)
                    + rotate_about(*to) * (u * u)
            })
            .collect(),
        Shape::Polyline { pts } => {
            let mut out: Vec<Vec2> = pts.iter().map(|point| *point + path.pos).collect();
            if path.rot.abs() > 1e-3 && !out.is_empty() {
                let centre = out.iter().copied().sum::<Vec2>() / out.len() as f32;
                for point in &mut out {
                    *point = centre + rot_local(*point - centre, path.rot);
                }
            }
            out
        }
        Shape::Arc {
            r, start, sweep, ..
        } => {
            let segments = ((sweep.abs() / 4.0).ceil() as usize).max(16);
            let orbit = r * path.scale.abs();
            (0..=segments)
                .map(|i| {
                    let angle =
                        (start + path.rot + sweep * i as f32 / segments as f32).to_radians();
                    path.pos + Vec2::new(angle.cos(), angle.sin()) * orbit
                })
                .collect()
        }
        _ => return None,
    })
}

fn point_along_polyline(points: &[Vec2], u: f32) -> Vec2 {
    if points.len() < 2 {
        return points.first().copied().unwrap_or(Vec2::ZERO);
    }
    let mut lengths = Vec::with_capacity(points.len());
    lengths.push(0.0);
    for pair in points.windows(2) {
        lengths.push(lengths.last().copied().unwrap() + pair[0].distance(pair[1]));
    }
    let total = *lengths.last().unwrap();
    if total <= 1e-6 {
        return points[0];
    }
    let distance = total * u.clamp(0.0, 1.0);
    let mut segment = 0;
    while segment + 1 < lengths.len() && lengths[segment + 1] < distance {
        segment += 1;
    }
    if segment + 1 >= points.len() {
        return *points.last().unwrap();
    }
    let span = lengths[segment + 1] - lengths[segment];
    let local = if span > 1e-6 {
        (distance - lengths[segment]) / span
    } else {
        0.0
    };
    points[segment].lerp(points[segment + 1], local)
}

/// `travel(entity, path, [duration], [ease])` — move one persistent entity
/// once along an existing line/arrow/curve/plot/spline/arc and leave it at the
/// endpoint. This is intentionally distinct from `flow`: `flow` is a transient
/// highlight, while `travel` moves the author's actual marker or object.
fn v_travel(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    a.max(4)?;
    let id = a.ident(0)?;
    let path_id = a.ident(1)?;
    if id == path_id {
        return Err(Error::new(
            "travel needs different entity and path ids",
            a.span_of(1),
        ));
    }
    if s.get(&id).is_none() {
        return Err(Error::new(
            format!("no 2-D entity named `{id}`"),
            a.span_of(0),
        ));
    }
    let path = s
        .authored_entity(&path_id)
        .ok_or_else(|| Error::new(format!("no path named `{path_id}`"), a.span_of(1)))?;
    let points = travel_path_points(&path).ok_or_else(|| {
        Error::new(
            format!("`{path_id}` is not a line, arrow, curve, plot, spline, or arc"),
            a.span_of(1),
        )
    })?;
    if points.len() < 2 {
        return Err(Error::new(
            format!("path `{path_id}` has fewer than two points"),
            a.span_of(1),
        ));
    }
    let dur = a.opt_num(2)?.unwrap_or(1.0);
    if dur <= 0.0 {
        return Err(Error::new("travel duration must be positive", a.span_of(2)));
    }
    let easing = if a.len() > 3 {
        resolve_easing(&a.ident(3)?, a.span_of(3))?
    } else {
        Easing::InOutCubic
    };
    let segments = points.len().saturating_sub(1).clamp(16, 96);
    let mut tracks = Vec::with_capacity(segments);
    for i in 1..=segments {
        let u = easing.apply(i as f32 / segments as f32);
        tracks.push(TrackSpec {
            id: id.clone(),
            prop: Prop::Pos,
            target: TargetValue::Abs(Value::V(point_along_polyline(&points, u))),
            start: dur * (i - 1) as f32 / segments as f32,
            dur: dur / segments as f32,
            easing: Easing::Linear,
        });
    }
    let endpoint = *points.last().unwrap();
    s.motion_pos.insert(id, endpoint);
    Ok(Clip {
        tracks,
        events: Vec::new(),
        dur,
    })
}

/// `arrange(particles, container, ["random|grid|ring"], [duration], [ease])` —
/// move one persistent particle set into a deterministic layout in a circle or
/// rectangle. Random layouts use stable curved routes rather than one shared
/// straight tween, so a gas or crowd feels organic without sacrificing exact
/// replay/scrubbing. Because the children retain identity, `grid → random →
/// grid` reads as an exact reversible rearrangement rather than a crossfade.
fn v_arrange(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    a.max(5)?;
    let id = a.ident(0)?;
    let container_id = a.ident(1)?;
    let layout = a.opt_text(2)?.unwrap_or_else(|| "random".into());
    let dur = a.opt_num(3)?.unwrap_or(1.2);
    if dur <= 0.0 {
        return Err(Error::new(
            "arrange duration must be positive",
            a.span_of(3),
        ));
    }
    let easing = if a.len() > 4 {
        resolve_easing(&a.ident(4)?, a.span_of(4))?
    } else {
        Easing::InOutCubic
    };
    let group = s.particle_groups.get(&id).cloned().ok_or_else(|| {
        Error::new(
            format!("no particle group `{id}` — call particles({id}, ...) first"),
            a.span_of(0),
        )
    })?;
    let container = s.authored_entity(&container_id).ok_or_else(|| {
        Error::new(
            format!("no 2-D container named `{container_id}`"),
            a.span_of(1),
        )
    })?;
    let targets = particle_layout_points(
        &container,
        group.children.len(),
        group.radius,
        group.seed,
        &layout,
    )
    .map_err(|m| {
        Error::new(
            format!("`{container_id}` cannot arrange particles: {m}"),
            a.span_of(2),
        )
    })?;

    // An unordered state should not look like every dot was assigned a ruler.
    // Give each child one deterministic Bézier route and compile it to ordinary
    // position tracks. This keeps evaluation pure and seekable while avoiding
    // the visibly synchronized straight-line pattern of a single tween.
    let route_controls = if layout == "random" {
        Some(
            particle_points(
                &container,
                group.children.len(),
                group.radius,
                group.seed
                    ^ stable_seed(&format!("{id}:{container_id}:arrange-route"))
                    ^ 0xd1b5_4a32_d192_ed03,
            )
            .map_err(|m| {
                Error::new(
                    format!("`{container_id}` cannot route particles: {m}"),
                    a.span_of(2),
                )
            })?,
        )
    } else {
        None
    };
    let route_segments = 12usize;
    let capacity = if route_controls.is_some() {
        group.children.len() * route_segments
    } else {
        group.children.len()
    };
    let mut tracks = Vec::with_capacity(capacity);
    for (index, (child, target)) in group.children.iter().zip(targets).enumerate() {
        if let Some(controls) = &route_controls {
            let from = s
                .motion_pos
                .get(child)
                .copied()
                .or_else(|| s.authored_entity(child).map(|entity| entity.pos))
                .unwrap_or(target);
            let control = controls[index];
            for segment in 1..=route_segments {
                let u = easing.apply(segment as f32 / route_segments as f32);
                let v = 1.0 - u;
                let point = from * (v * v) + control * (2.0 * v * u) + target * (u * u);
                tracks.push(TrackSpec {
                    id: child.clone(),
                    prop: Prop::Pos,
                    target: TargetValue::Abs(Value::V(point)),
                    start: dur * (segment - 1) as f32 / route_segments as f32,
                    dur: dur / route_segments as f32,
                    easing: Easing::Linear,
                });
            }
        } else {
            tracks.push(TrackSpec {
                id: child.clone(),
                prop: Prop::Pos,
                target: TargetValue::Abs(Value::V(target)),
                start: 0.0,
                dur,
                easing,
            });
        }
        s.motion_pos.insert(child.clone(), target);
    }
    s.particle_groups.get_mut(&id).unwrap().container = container_id;
    Ok(Clip {
        tracks,
        events: Vec::new(),
        dur,
    })
}

/// `wander(particles, [duration])` — contained, deterministic ambient motion.
/// It expands to ordinary position tracks at build time, preserving the core
/// invariant that evaluating frame `t` is pure and freely scrubbable.
fn v_wander(s: &Scene, a: &Args) -> Result<Clip, Error> {
    a.max(2)?;
    let id = a.ident(0)?;
    let duration = a.opt_num(1)?.unwrap_or(4.0);
    if duration <= 0.0 {
        return Err(Error::new("wander duration must be positive", a.span_of(1)));
    }
    let group = s.particle_groups.get(&id).ok_or_else(|| {
        Error::new(
            format!("no particle group `{id}` — call particles({id}, ...) first"),
            a.span_of(0),
        )
    })?;
    let container = s.authored_entity(&group.container).ok_or_else(|| {
        Error::new(
            format!("particle container `{}` no longer exists", group.container),
            a.span_of(0),
        )
    })?;
    let segments = ((duration / 0.85).ceil() as usize).clamp(1, 32);
    let step = duration / segments as f32;
    let targets = particle_points(
        &container,
        group.children.len() * segments,
        group.radius,
        group.seed ^ 0x9e37_79b9_7f4a_7c15,
    )
    .map_err(|m| {
        Error::new(
            format!("cannot wander inside `{}`: {m}", group.container),
            a.span_of(0),
        )
    })?;
    let mut tracks = Vec::with_capacity(group.children.len() * segments);
    for (i, child) in group.children.iter().enumerate() {
        for k in 0..segments {
            tracks.push(TrackSpec {
                id: child.clone(),
                prop: Prop::Pos,
                target: TargetValue::Abs(Value::V(targets[k * group.children.len() + i])),
                start: k as f32 * step,
                dur: step,
                easing: Easing::InOutQuad,
            });
        }
    }
    Ok(Clip {
        tracks,
        events: Vec::new(),
        dur: duration,
    })
}

fn flow_path_length(entity: &Entity) -> f32 {
    match &entity.shape {
        Shape::Line { to } | Shape::Arrow { to } => (*to - entity.pos).length(),
        Shape::Curve { ctrl, to, .. } => {
            let mut length = 0.0;
            let mut previous = entity.pos;
            for index in 1..=32 {
                let t = index as f32 / 32.0;
                let u = 1.0 - t;
                let point = entity.pos * (u * u) + *ctrl * (2.0 * u * t) + *to * (t * t);
                length += (point - previous).length();
                previous = point;
            }
            length
        }
        Shape::Polyline { pts } => pts
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).length())
            .sum(),
        Shape::Arc { r, sweep, .. } => r * sweep.to_radians().abs(),
        _ => 0.0,
    }
}

/// `flow(path, [duration], [direction], [mode])` — send a luminous pulse or a
/// finite, cleanly draining stream. Direction is `forward`/`reverse`/`both`; mode is
/// `once`/`continuous`. Repeated calls compose from a monotonic signed phase.
fn v_flow(s: &Scene, a: &Args) -> Result<Clip, Error> {
    a.max(4)?;
    let id = a.ident(0)?;
    let paths: Vec<(&Entity, f32)> = s
        .entities
        .iter()
        .filter(|entity| entity.id == id || entity.tags.iter().any(|tag| tag == &id))
        .filter(|entity| {
            matches!(
                entity.shape,
                Shape::Line { .. }
                    | Shape::Arrow { .. }
                    | Shape::Curve { .. }
                    | Shape::Polyline { .. }
                    | Shape::Arc { .. }
            )
        })
        .map(|entity| (entity, flow_path_length(entity)))
        .collect();
    if paths.is_empty() {
        let message = if s.get(&id).is_some() {
            format!("`{id}` is not a line, curve, spline, arc, or link")
        } else {
            format!("no line, curve, spline, arc, or link named or tagged `{id}`")
        };
        return Err(Error::new(message, a.span_of(0)));
    }
    let dur = a.opt_num(1)?.unwrap_or(1.0);
    if dur <= 0.0 {
        return Err(Error::new("flow duration must be positive", a.span_of(1)));
    }
    let direction = if a.len() > 2 {
        a.ident(2)?
    } else {
        "forward".to_string()
    };
    let channels: &[Prop] = match direction.as_str() {
        "forward" => &[Prop::Flow],
        "reverse" => &[Prop::FlowBack],
        "both" => &[Prop::Flow, Prop::FlowBack],
        _ => {
            return Err(Error::new(
                "flow direction must be `forward`, `reverse`, or `both`",
                a.span_of(2),
            ))
        }
    };
    let mode = if a.len() > 3 {
        a.ident(3)?
    } else {
        "once".to_string()
    };
    if mode != "once" && mode != "continuous" {
        return Err(Error::new(
            "flow mode must be `once` or `continuous`",
            a.span_of(3),
        ));
    }
    Ok(Clip {
        tracks: paths
            .into_iter()
            .flat_map(|(path, length)| {
                let cycles = if mode == "continuous" {
                    (dur * 420.0 / length.max(1.0)).round().max(2.0)
                } else {
                    1.0
                };
                channels.iter().map(move |channel| TrackSpec {
                    id: path.id.clone(),
                    prop: *channel,
                    target: TargetValue::Rel(Value::F(cycles)),
                    start: 0.0,
                    dur,
                    easing: Easing::Linear,
                })
            })
            .collect(),
        events: Vec::new(),
        dur,
    })
}

fn authored_attachment(s: &Scene, id: &str) -> Option<(String, Vec2)> {
    match s.authored_attachments.get(id) {
        Some(value) => value.clone(),
        None => s.authored_entity(id).and_then(|entity| entity.follow),
    }
}

fn authored_attached_position(s: &Scene, id: &str, visiting: &mut Vec<String>) -> Option<Vec2> {
    if visiting.iter().any(|item| item == id) {
        return None;
    }
    visiting.push(id.to_string());
    let entity = s.authored_entity(id)?;
    let position = match authored_attachment(s, id) {
        Some((target, offset)) => {
            authored_attached_position(s, &target, visiting).map(|position| position + offset)?
        }
        None => entity.pos,
    };
    visiting.pop();
    Some(position)
}

/// `attach(child, target, [(dx,dy)])` — create a persistent, pure per-frame
/// relationship. `attach(child, none)` releases it at the current authored
/// position; no second vocabulary word is needed.
fn v_attach(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    a.max(3)?;
    let child = a.ident(0)?;
    let target = a.ident(1)?;
    let offset = if a.len() > 2 { a.pair(2)? } else { Vec2::ZERO };
    if s.get(&child).is_none() {
        return Err(Error::new(
            format!("no 2-D entity named `{child}` to attach"),
            a.span_of(0),
        ));
    }

    if target == "none" {
        let position = authored_attached_position(s, &child, &mut Vec::new()).ok_or_else(|| {
            Error::new(
                format!("cannot release `{child}` because its attachment chain is cyclic"),
                a.span_of(0),
            )
        })?;
        return Ok(Clip {
            tracks: vec![TrackSpec {
                id: child.clone(),
                prop: Prop::Pos,
                target: TargetValue::Abs(Value::V(position)),
                start: 0.0,
                dur: 0.0,
                easing: Easing::Linear,
            }],
            events: vec![TimelineEvent::attachment(child, None, Vec2::ZERO, 0.0)],
            dur: 0.0,
        });
    }

    if child == target {
        return Err(Error::new(
            "an entity cannot attach to itself",
            a.span_of(1),
        ));
    }
    if s.get(&target).is_none() {
        return Err(Error::new(
            format!("no 2-D entity named `{target}` to attach to"),
            a.span_of(1),
        ));
    }

    // Follow the proposed target chain before accepting the new edge. This is
    // a build-time diagnostic; runtime evaluation never has to recover from a
    // relationship cycle.
    let mut cursor = target.clone();
    let mut visited = vec![child.clone()];
    loop {
        if visited.iter().any(|id| id == &cursor) {
            return Err(Error::new(
                format!("attaching `{child}` to `{target}` would create a cycle"),
                a.span_of(1),
            ));
        }
        visited.push(cursor.clone());
        let Some((next, _)) = authored_attachment(s, &cursor) else {
            break;
        };
        cursor = next;
    }

    Ok(Clip {
        tracks: Vec::new(),
        events: vec![TimelineEvent::attachment(child, Some(target), offset, 0.0)],
        dur: 0.0,
    })
}

/// `become(source, target, [duration], [ease])` — retain the source identity
/// while moving it to the target's visual blueprint. Compatible shapes blend
/// directly; every other pair gets a deterministic local crossfade and the
/// same exact settled target.
fn v_become(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    a.max(4)?;
    let id = a.ident(0)?;
    let target_id = a.ident(1)?;
    if id == target_id {
        return Err(Error::new(
            "`become` needs different source and target ids",
            a.span_of(1),
        ));
    }
    let from = s.authored_entity(&id).ok_or_else(|| {
        Error::new(
            format!("no 2-D entity named `{id}` to transform"),
            a.span_of(0),
        )
    })?;
    let mut target = s.authored_entity(&target_id).ok_or_else(|| {
        Error::new(
            format!("no 2-D target blueprint named `{target_id}`"),
            a.span_of(1),
        )
    })?;
    let dur = a.opt_num(2)?.unwrap_or(0.8);
    if dur <= 0.0 || !dur.is_finite() {
        return Err(Error::new("become duration must be positive", a.span_of(2)));
    }
    let easing = if a.len() > 3 {
        resolve_easing(&a.ident(3)?, a.span_of(3))?
    } else {
        Easing::InOutCubic
    };

    // `hidden(target)` is the natural blueprint pattern. Hidden is a property
    // of the target entity's authored visibility, not a request to make the
    // transformed source disappear at the end.
    if target.opacity <= 1e-6 && from.opacity > 1e-6 {
        target.opacity = from.opacity;
    }
    let crossfade = !crate::timeline::shape_transition_compatible(&from.shape, &target.shape);
    let mut tracks = vec![
        TrackSpec {
            id: id.clone(),
            prop: Prop::Pos,
            target: TargetValue::Abs(Value::V(target.pos)),
            start: 0.0,
            dur,
            easing,
        },
        TrackSpec {
            id: id.clone(),
            prop: Prop::Color,
            target: TargetValue::Abs(Value::C(target.color)),
            start: 0.0,
            dur,
            easing,
        },
        TrackSpec {
            id: id.clone(),
            prop: Prop::Scale,
            target: TargetValue::Abs(Value::F(target.scale)),
            start: 0.0,
            dur,
            easing,
        },
        TrackSpec {
            id: id.clone(),
            prop: Prop::Rot,
            target: TargetValue::Abs(Value::F(target.rot)),
            start: 0.0,
            dur,
            easing,
        },
        TrackSpec {
            id: id.clone(),
            prop: Prop::Trace,
            target: TargetValue::Abs(Value::F(target.trace)),
            start: 0.0,
            dur,
            easing,
        },
    ];
    if crossfade {
        tracks.push(TrackSpec {
            id: id.clone(),
            prop: Prop::Opacity,
            target: TargetValue::Abs(Value::F(0.0)),
            start: 0.0,
            dur: dur * 0.5,
            easing: Easing::InQuad,
        });
        tracks.push(TrackSpec {
            id: id.clone(),
            prop: Prop::Opacity,
            target: TargetValue::Abs(Value::F(target.opacity)),
            start: dur * 0.5,
            dur: dur * 0.5,
            easing: Easing::OutQuad,
        });
    } else {
        tracks.push(TrackSpec {
            id: id.clone(),
            prop: Prop::Opacity,
            target: TargetValue::Abs(Value::F(target.opacity)),
            start: 0.0,
            dur,
            easing,
        });
    }

    Ok(Clip {
        tracks,
        events: vec![TimelineEvent::visual_transition(
            id, from, target, dur, easing, crossfade,
        )],
        dur,
    })
}

fn turn_targets(s: &Scene, id_or_tag: &str) -> Vec<String> {
    if s.get(id_or_tag).is_some() {
        return vec![id_or_tag.to_string()];
    }
    s.entities
        .iter()
        .filter(|entity| entity.tags.iter().any(|tag| tag == id_or_tag))
        .map(|entity| entity.id.clone())
        .collect()
}

/// `turn(id_or_tag, pivot, degrees, [duration], [ease])` — rotate one object
/// or a tagged arrangement as a rigid system around a shared pivot.
fn v_turn(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    a.max(5)?;
    let id_or_tag = a.ident(0)?;
    let pivot = match &a.exprs.get(1).map(|expr| &expr.kind) {
        Some(ExprKind::Pair(x, y)) => Vec2::new(*x, *y),
        Some(ExprKind::Ident(id)) => authored_attached_position(s, id, &mut Vec::new())
            .ok_or_else(|| Error::new(format!("no 2-D pivot named `{id}`"), a.span_of(1)))?,
        _ => {
            return Err(Error::new(
                "turn pivot should be a `(x, y)` point or an entity name",
                a.span_of(1),
            ))
        }
    };
    let degrees = a.num(2)?;
    if !degrees.is_finite() {
        return Err(Error::new("turn degrees must be finite", a.span_of(2)));
    }
    let dur = a.opt_num(3)?.unwrap_or(0.7);
    if dur <= 0.0 || !dur.is_finite() {
        return Err(Error::new("turn duration must be positive", a.span_of(3)));
    }
    let easing = if a.len() > 4 {
        resolve_easing(&a.ident(4)?, a.span_of(4))?
    } else {
        Easing::InOutCubic
    };
    let targets = turn_targets(s, &id_or_tag);
    if targets.is_empty() {
        return Err(Error::new(
            format!("no 2-D entity or tag named `{id_or_tag}` to turn"),
            a.span_of(0),
        ));
    }

    let segments = ((degrees.abs() / 7.5).ceil() as usize).clamp(1, 64);
    let segment_dur = dur / segments as f32;
    let mut tracks = Vec::new();
    for id in targets {
        let entity = s.authored_entity(&id).expect("turn target was validated");
        let mut previous_angle = 0.0;
        for segment in 1..=segments {
            let angle = degrees * easing.apply(segment as f32 / segments as f32);
            let delta = angle - previous_angle;
            tracks.push(TrackSpec {
                id: id.clone(),
                prop: Prop::Pos,
                target: TargetValue::RotateAround {
                    pivot,
                    degrees: delta,
                },
                start: segment_dur * (segment - 1) as f32,
                dur: segment_dur,
                easing: Easing::Linear,
            });
            if matches!(
                entity.shape,
                Shape::Line { .. } | Shape::Arrow { .. } | Shape::Curve { .. } | Shape::Coil { .. }
            ) {
                tracks.push(TrackSpec {
                    id: id.clone(),
                    prop: Prop::To,
                    target: TargetValue::RotateAround {
                        pivot,
                        degrees: delta,
                    },
                    start: segment_dur * (segment - 1) as f32,
                    dur: segment_dur,
                    easing: Easing::Linear,
                });
            }
            if matches!(entity.shape, Shape::Curve { .. }) {
                tracks.push(TrackSpec {
                    id: id.clone(),
                    prop: Prop::Ctrl,
                    target: TargetValue::RotateAround {
                        pivot,
                        degrees: delta,
                    },
                    start: segment_dur * (segment - 1) as f32,
                    dur: segment_dur,
                    easing: Easing::Linear,
                });
            }
            previous_angle = angle;
        }
        if !matches!(
            entity.shape,
            Shape::Line { .. } | Shape::Arrow { .. } | Shape::Curve { .. } | Shape::Coil { .. }
        ) {
            tracks.push(TrackSpec {
                id,
                prop: Prop::Rot,
                target: TargetValue::Rel(Value::F(degrees)),
                start: 0.0,
                dur,
                easing,
            });
        }
    }
    Ok(Clip {
        tracks,
        events: Vec::new(),
        dur,
    })
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
    r.ctor("parameter", c_parameter);
    r.ctor("bind", c_bind);
    r.ctor("caption", c_caption);
    r.ctor("support", c_support);
    r.ctor("morph", c_morph);
    r.ctor("copy", c_copy);
    r.ctor("dot", c_dot);
    r.ctor("particles", c_particles);
    r.ctor("circle", c_circle);
    r.ctor("rect", c_rect);
    r.ctor("image", c_image);
    r.ctor("equation", c_equation);
    r.ctor("line", c_line);
    r.ctor("link", c_link);
    r.ctor("polygon", c_polygon);
    r.ctor("arrow", c_arrow);
    r.ctor("brace", c_brace);
    r.ctor("bracelabel", c_bracelabel);
    r.ctor("bracetext", c_bracelabel);
    // modifiers (also constructors: they touch the base scene)
    r.ctor("hidden", m_hidden);
    r.ctor("untraced", m_untraced);
    r.ctor("cursor", m_cursor);
    r.ctor("sticky", m_sticky);
    r.ctor("rot", m_rot);
    r.ctor("opacity", m_opacity);
    r.ctor("color", m_color);
    r.ctor("hue", m_hue);
    r.ctor("outlined", m_outlined);
    r.ctor("filled", m_filled);
    r.ctor("outline", m_outline);
    r.ctor("size", m_size);
    r.ctor("wrap", m_wrap);
    r.ctor("stroke", m_stroke);
    r.ctor("dashed", m_dashed);
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
    r.mut_verb("rewrite", v_rewrite); // matching LaTeX transformation, opt-in
    r.mut_verb("swap", v_swap); // two entities, or stateful array slot-swap
    r.mut_verb("cycle", v_cycle); // variadic CyclicReplace with an optional path arc
    r.mut_verb("travel", v_travel); // move a persistent entity once along a path
    r.mut_verb("arrange", v_arrange); // persistent particles: grid/random/ring + new container
    r.mut_verb("attach", v_attach); // persistent per-frame relationship; target `none` releases
    r.mut_verb("become", v_become); // retain source id while adopting a visual blueprint
    r.mut_verb("turn", v_turn); // rotate an entity/tag rigidly about a shared pivot
    r.verb("karaoke", v_karaoke); // highlight caption words in sequence
    r.verb("wordpop", v_wordpop); // pop caption words in one at a time
    r.verb("wander", v_wander); // contained deterministic ambient particle motion
    r.verb("flow", v_flow); // luminous pulse travelling over a path
}

#[cfg(test)]
mod tests {
    use macroquad::prelude::Vec2;

    use crate::primitives::Shape;

    use super::{
        equation_part_area, equation_parts_can_match, match_equation_parts,
        prefer_local_side_dissolve,
    };

    #[test]
    fn parameter_drives_properties_readouts_and_native_widget_purely() {
        let movie = crate::parse(
            "canvas(1280,720);\n\
             parameter(p,(640,90),0,-1,1,\"p\",2);\n\
             dot(body,(200,300),10);\n\
             counter(square,(640,180),0,2,\"p² = \",\"\");\n\
             bind(p,body,x,200,1000);\n\
             bind(p,body,scale,\"1+p*p\");\n\
             bind(p,square,value,\"p*p\");\n\
             to(p,value,1,2,linear);",
        )
        .unwrap();
        let (base, timeline) = movie.finalize();
        let half = timeline.apply(&base, 1.0);
        let parameter = half.get("p").unwrap().counter.as_ref().unwrap().value;
        assert!((parameter - 0.5).abs() < 1e-4);
        assert!((half.get("body").unwrap().pos.x - 800.0).abs() < 1e-3);
        assert!((half.get("body").unwrap().scale - 1.25).abs() < 1e-3);
        assert!((half.get("square").unwrap().counter.as_ref().unwrap().value - 0.25).abs() < 1e-3);
        let widget = half.get("p.dot").unwrap().pos;
        assert!((widget.x - (640.0 + 48.0)).abs() < 1e-3);

        let _later = timeline.apply(&base, 1.8);
        let after_seek = timeline.apply(&base, 0.35);
        let fresh = timeline.apply(&base, 0.35);
        assert_eq!(
            after_seek.get("body").unwrap().pos,
            fresh.get("body").unwrap().pos
        );
        assert_eq!(
            after_seek.get("p.dot").unwrap().pos,
            fresh.get("p.dot").unwrap().pos
        );
    }

    #[test]
    fn parameter_formula_rebuilds_plot_and_all_analysis_views() {
        let movie = crate::parse(
            "canvas(1280,720);\n\
             parameter(a,(180,80),1,-2,2,\"a\",1);\n\
             plot(f,(640,420),80,35,\"x*x\",(-3,3));\n\
             tangent(t,f,1,160); slope(m,f,1); area(r,f,0,1,80);\n\
             bind(a,f,formula,\"p*x*x\");\n\
             to(a,value,2,1,linear);",
        )
        .unwrap();
        let (base, timeline) = movie.finalize();
        let end = timeline.apply(&base, 1.0);
        let graph = end.get("f").unwrap().graph.as_ref().unwrap();
        assert!((graph.y(2.0) - 8.0).abs() < 1e-3);
        let tangent = end.get("t").unwrap().graph_view.as_ref().unwrap();
        assert!((tangent.graph().slope(1.0) - 4.0).abs() < 0.02);
        let slope = end.get("m").unwrap().counter.as_ref().unwrap().value;
        assert!((slope - 4.0).abs() < 0.02);
        let area = end.get("r").unwrap().graph_view.as_ref().unwrap().value();
        assert!((area - 2.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn parameter_formula_supports_a_damping_family() {
        let movie = crate::parse(
            "parameter(damping,(180,80),0.1,0,1,\"damping\",2);\n\
             plot(wave,(640,360),90,80,\"cos(4*x)\",(-3,3));\n\
             bind(damping,wave,formula,\"exp(-p*abs(x))*cos(4*x)\");\n\
             to(damping,value,0.7,1,linear);",
        )
        .unwrap();
        let (base, timeline) = movie.finalize();
        let end = timeline.apply(&base, 1.0);
        let graph = end.get("wave").unwrap().graph.as_ref().unwrap();
        let expected = (-0.7_f32).exp() * 4.0_f32.cos();
        assert!((graph.y(1.0) - expected).abs() < 1e-4);
    }

    #[test]
    fn parameter_refresh_leaves_unbound_graph_views_alone() {
        let movie = crate::parse(
            "parameter(a,(100,60),1,-2,2);\n\
             plot(bound,(400,360),50,30,\"x*x\",(-3,3));\n\
             plot(ordinary,(900,360),50,30,\"x*x\",(-3,3));\n\
             tangent(live,bound,1,120);\n\
             tangent(untouched,ordinary,1,120);\n\
             bind(a,bound,formula,\"p*x*x\");\n\
             to(untouched,y,250,1,linear);",
        )
        .unwrap();
        let (base, timeline) = movie.finalize();
        let end = timeline.apply(&base, 1.0);
        assert!((end.get("untouched").unwrap().pos.y - 250.0).abs() < 1e-4);
        assert!(
            (end.get("live")
                .unwrap()
                .graph_view
                .as_ref()
                .unwrap()
                .graph()
                .y(2.0)
                - 4.0)
                .abs()
                < 1e-3
        );
    }

    #[test]
    fn parameter_range_clamps_and_bind_errors_are_clear() {
        let movie = crate::parse(
            "parameter(p,(200,80),0,-1,1); dot(d,(0,200)); bind(p,d,x,100,300); to(p,value,9,1,linear);",
        )
        .unwrap();
        let (base, timeline) = movie.finalize();
        let end = timeline.apply(&base, 1.0);
        assert_eq!(end.get("p").unwrap().counter.as_ref().unwrap().value, 1.0);
        assert!((end.get("d").unwrap().pos.x - 300.0).abs() < 1e-4);

        for (source, expected) in [
            ("parameter(p,(0,0),0,1,1);", "min < max"),
            (
                "counter(p,(0,0),0); dot(d,(0,0)); bind(p,d,x,0,1);",
                "not a `parameter`",
            ),
            (
                "parameter(p,(0,0),0,-1,1); dot(d,(0,0)); bind(p,d,x,\"wat(p)\");",
                "unknown function",
            ),
            (
                "parameter(p,(0,0),0,-1,1); dot(d,(0,0)); bind(p,d,formula,\"p*x\");",
                "not a plot",
            ),
        ] {
            let error = match crate::parse(source) {
                Err(error) => error,
                Ok(_) => panic!("expected `{source}` to fail"),
            };
            assert!(error.msg.contains(expected), "got: {}", error.msg);
        }
    }

    #[test]
    fn particles_are_seeded_repeatable_and_contained() {
        let src = "canvas(\"16:9\");\n\
                   circle(tank, (400, 300), 100);\n\
                   particles(bubbles, tank, 24, 5, 7);\n";
        let a = crate::parse(src).unwrap();
        let b = crate::parse(src).unwrap();
        let group = a
            .base()
            .particle_groups
            .get("bubbles")
            .expect("particle group");
        assert_eq!(group.children.len(), 24);
        assert_eq!(group.seed, 7);
        for child in &group.children {
            let pa = a.base().get(child).unwrap().pos;
            let pb = b.base().get(child).unwrap().pos;
            assert_eq!(pa, pb, "the same seed must reproduce `{child}` exactly");
            assert!(
                pa.distance(Vec2::new(400.0, 300.0)) <= 95.001,
                "`{child}` must stay inset by its radius"
            );
        }
        assert!(a.validate().is_ok());
    }

    #[test]
    fn particles_arrange_between_ordered_random_and_larger_containers() {
        let movie = crate::parse(
            "canvas(800,500);\n\
             rect(box, (250,180), 300, 160);\n\
             particles(bits, box, 12, 5, 17, \"grid\");\n\
             arrange(bits, box, \"random\", 1, linear);\n\
             arrange(bits, box, \"grid\", 1, linear);\n\
             rect(left, (150,380), 120, 100); rect(full, (400,380), 600, 100);\n\
             particles(gas, left, 10, 4, 23);\n\
             arrange(gas, full, \"random\", 1, smooth);\n\
             circle(orbit, (650,180), 70);\n\
             arrange(bits, orbit, \"ring\", 1, smooth);",
        )
        .unwrap();
        let (base, timeline) = movie.finalize();
        let ordered: Vec<Vec2> = base.particle_groups["bits"]
            .children
            .iter()
            .map(|id| base.get(id).unwrap().pos)
            .collect();

        let random = timeline.apply(&base, 1.0);
        assert!(
            base.particle_groups["bits"]
                .children
                .iter()
                .zip(&ordered)
                .any(|(id, start)| random.get(id).unwrap().pos != *start),
            "grid → random must visibly rearrange persistent children"
        );
        let curved = timeline.apply(&base, 0.5);
        assert!(
            base.particle_groups["bits"]
                .children
                .iter()
                .zip(&ordered)
                .any(|(id, start)| {
                    let end = random.get(id).unwrap().pos;
                    let direct_midpoint = (*start + end) * 0.5;
                    curved.get(id).unwrap().pos.distance(direct_midpoint) > 1.0
                }),
            "random arrangement should use independent curved routes, not one straight tween"
        );

        let restored = timeline.apply(&base, 2.0);
        for (id, start) in base.particle_groups["bits"].children.iter().zip(&ordered) {
            assert_eq!(
                restored.get(id).unwrap().pos,
                *start,
                "random → grid must reconstruct the exact ordered state"
            );
        }

        let expanded = timeline.apply(&base, 3.0);
        for id in &base.particle_groups["gas"].children {
            let p = expanded.get(id).unwrap().pos;
            assert!((100.0..=700.0).contains(&p.x));
            assert!((330.0..=430.0).contains(&p.y));
        }

        let ring = timeline.apply(&base, 4.0);
        for id in &base.particle_groups["bits"].children {
            let distance = ring.get(id).unwrap().pos.distance(Vec2::new(650.0, 180.0));
            assert!((distance - 65.0).abs() < 1e-3);
        }
    }

    #[test]
    fn transform_after_arrange_uses_the_arranged_endpoint_without_snapping() {
        let movie = crate::parse(
            "canvas(800,500);\n\
             rect(box, (180,250), 180, 180);\n\
             circle(orbit, (560,250), 90);\n\
             particles(dots, box, 8, 5, 17, \"grid\");\n\
             arrange(dots, orbit, \"ring\", 1, linear);\n\
             transform(dots, (560,250), 0, -1, 1, 0, 1, linear);",
        )
        .unwrap();
        let (base, timeline) = movie.finalize();
        let arranged = timeline.apply(&base, 1.0);
        let transformed = timeline.apply(&base, 2.0);
        for id in &base.particle_groups["dots"].children {
            let p = arranged.get(id).unwrap().pos;
            let expected = Vec2::new(560.0 - (p.y - 250.0), 250.0 + (p.x - 560.0));
            assert!(
                transformed.get(id).unwrap().pos.distance(expected) < 1e-3,
                "`{id}` should rotate from its ring position"
            );
        }
    }

    #[test]
    fn particle_layout_errors_are_clear() {
        for (source, expected) in [
            (
                "circle(c,(100,100),80); particles(p,c,8,4,7,\"grid\");",
                "grid particle layout currently needs a rectangle",
            ),
            (
                "rect(r,(100,100),100,100); particles(p,r,8); arrange(p,r,\"rows\");",
                "unknown particle layout `rows`",
            ),
            (
                "rect(r,(100,100),100,100); particles(p,r,8,4,7,\"ring\");",
                "ring particle layout currently needs a circle",
            ),
        ] {
            let error = match crate::parse(source) {
                Err(error) => error,
                Ok(_) => panic!("invalid particle layout must fail: {source}"),
            };
            assert!(error.msg.contains(expected), "got: {}", error.msg);
        }
    }

    #[test]
    fn travel_moves_once_along_a_path_and_holds_at_the_endpoint() {
        let movie = crate::parse(
            "canvas(800,500);\n\
             plot(path, (100,400), 80, 90, \"1-exp(-x)\", (0,4));\n\
             dot(marker, (100,400), 5);\n\
             travel(marker, path, 2, smooth);",
        )
        .unwrap();
        let (base, timeline) = movie.finalize();
        let Shape::Polyline { pts } = &base.get("path").unwrap().shape else {
            panic!("plot must be a polyline");
        };
        let expected = *pts.last().unwrap() + base.get("path").unwrap().pos;

        let midpoint = timeline.apply(&base, 1.0).get("marker").unwrap().pos;
        assert_ne!(midpoint, base.get("marker").unwrap().pos);
        assert_ne!(midpoint, expected);
        assert_eq!(
            timeline.apply(&base, 2.0).get("marker").unwrap().pos,
            expected
        );
        assert_eq!(
            timeline.apply(&base, 8.0).get("marker").unwrap().pos,
            expected
        );
        let early = timeline.apply(&base, 0.7).get("marker").unwrap().pos;
        let _later = timeline.apply(&base, 1.7);
        let early_again = timeline.apply(&base, 0.7).get("marker").unwrap().pos;
        assert_eq!(
            early, early_again,
            "travel must remain deterministic under out-of-order scrubbing"
        );

        let bad = match crate::parse("circle(c,(100,100),20); dot(d,(0,0)); travel(d,c,1);") {
            Err(error) => error,
            Ok(_) => panic!("a filled shape is not a travel path"),
        };
        assert!(bad
            .msg
            .contains("is not a line, arrow, curve, plot, spline, or arc"));
    }

    #[test]
    fn morph_preserves_open_and_closed_path_topology() {
        let open =
            crate::parse("line(a,(0,0),(100,40)); line(b,(20,80),(180,80)); morph(a,b);").unwrap();
        let Shape::Polyline { pts } = &open.base().get("a").unwrap().shape else {
            panic!("morph source must become a polyline");
        };
        assert_ne!(
            pts.first(),
            pts.last(),
            "an open line must not gain a closing chord"
        );

        let closed =
            crate::parse("circle(a,(100,100),40); rect(b,(100,100),90,60); morph(a,b);").unwrap();
        let Shape::Polyline { pts } = &closed.base().get("a").unwrap().shape else {
            panic!("morph source must become a polyline");
        };
        assert_eq!(
            pts.first(),
            pts.last(),
            "closed outlines must remain closed"
        );
    }

    #[test]
    fn wander_is_contained_and_evaluation_order_independent() {
        let m = crate::parse(
            "canvas(\"16:9\");\n\
             circle(tank, (400, 300), 100);\n\
             particles(bubbles, tank, 20, 5, 11);\n\
             wander(bubbles, 4);\n",
        )
        .unwrap();
        let (base, timeline) = m.finalize();
        let group = base.particle_groups.get("bubbles").unwrap();
        let mut changed = false;
        for t in [0.4, 1.5, 3.7] {
            let frame = timeline.apply(&base, t);
            for child in &group.children {
                let p = frame.get(child).unwrap().pos;
                changed |= p != base.get(child).unwrap().pos;
                assert!(
                    p.distance(Vec2::new(400.0, 300.0)) <= 95.001,
                    "`{child}` escaped the circle at t={t}"
                );
            }
        }
        assert!(changed, "wander must actually move the particles");

        let _later = timeline.apply(&base, 3.2);
        let after_seek = timeline.apply(&base, 0.65);
        let fresh = timeline.apply(&base, 0.65);
        for child in &group.children {
            assert_eq!(
                after_seek.get(child).unwrap().pos,
                fresh.get(child).unwrap().pos
            );
        }
    }

    #[test]
    fn cycle_follows_an_arc_and_repeated_cycles_compose() {
        let m = crate::parse(
            "canvas(\"16:9\");\n\
             dot(a, (100, 100)); dot(b, (300, 100));\n\
             cycle(a, b, 1, 90, smooth);\n\
             cycle(a, b, 1, 90, smooth);\n",
        )
        .unwrap();
        let (base, timeline) = m.finalize();
        let halfway = timeline.apply(&base, 0.5);
        assert!(
            (halfway.get("a").unwrap().pos.y - 100.0).abs() > 10.0,
            "a non-zero cycle arc must leave the straight chord"
        );
        let swapped = timeline.apply(&base, 1.0);
        assert!((swapped.get("a").unwrap().pos - Vec2::new(300.0, 100.0)).length() < 0.01);
        assert!((swapped.get("b").unwrap().pos - Vec2::new(100.0, 100.0)).length() < 0.01);
        let returned = timeline.apply(&base, 2.0);
        assert!((returned.get("a").unwrap().pos - Vec2::new(100.0, 100.0)).length() < 0.01);
        assert!((returned.get("b").unwrap().pos - Vec2::new(300.0, 100.0)).length() < 0.01);
    }

    #[test]
    fn cycle_rotates_three_positions() {
        let m = crate::parse(
            "canvas(\"16:9\");\n\
             dot(a, (100,100)); dot(b, (200,100)); dot(c, (300,100));\n\
             cycle(a, b, c, 1, 60, linear);\n",
        )
        .unwrap();
        let (base, timeline) = m.finalize();
        let end = timeline.apply(&base, 1.0);
        assert!((end.get("a").unwrap().pos - Vec2::new(200.0, 100.0)).length() < 0.01);
        assert!((end.get("b").unwrap().pos - Vec2::new(300.0, 100.0)).length() < 0.01);
        assert!((end.get("c").unwrap().pos - Vec2::new(100.0, 100.0)).length() < 0.01);
    }

    #[test]
    fn bent_link_tracks_moving_entities_and_flow_phase() {
        use crate::primitives::Shape;

        let m = crate::parse(
            "canvas(\"16:9\");\n\
             circle(A, (200, 300), 50);\n\
             circle(B, (500, 300), 70);\n\
             link(ab, A, B, 40);\n\
             par { move(A, (250, 200), 1); move(B, (550, 400), 1); flow(ab, 1); }\n",
        )
        .unwrap();
        let (base, timeline) = m.finalize();
        let frame = timeline.apply(&base, 0.5);
        let a = frame.get("A").unwrap().pos;
        let b = frame.get("B").unwrap().pos;
        let edge = frame.get("ab").unwrap();
        let Shape::Curve { ctrl, to, .. } = edge.shape else {
            panic!("a non-zero bend must create a curve");
        };
        assert!((edge.pos.distance(a) - 50.0).abs() < 0.01);
        assert!((to.distance(b) - 70.0).abs() < 0.01);
        assert!((ctrl - (edge.pos + to) * 0.5).length() > 39.9);
        assert!((edge.flow - 0.5).abs() < 0.001);
        assert!((timeline.apply(&base, 1.0).get("ab").unwrap().flow - 1.0).abs() < 0.001);

        // Rectangle trim is directional, so it must be recomputed as the other
        // endpoint moves from the short side to the long side.
        let rect_movie = crate::parse(
            "canvas(\"16:9\");\n\
             rect(box, (200, 200), 120, 60);\n\
             circle(B, (500, 200), 20);\n\
             link(edge, box, B);\n\
             move(B, (200, 500), 1);\n",
        )
        .unwrap();
        let (rect_base, rect_timeline) = rect_movie.finalize();
        let end = rect_timeline.apply(&rect_base, 1.0);
        assert_eq!(end.get("edge").unwrap().pos, Vec2::new(200.0, 230.0));
    }

    #[test]
    fn flow_supports_reverse_and_finite_continuous_streams() {
        let forward = crate::parse(
            "canvas(800,500); line(path,(100,250),(700,250)); flow(path,4,forward,continuous);",
        )
        .unwrap();
        let (base, timeline) = forward.finalize();
        let middle = timeline.apply(&base, 2.0).get("path").unwrap().flow;
        let end = timeline.apply(&base, 4.0).get("path").unwrap().flow;
        assert!(
            middle > 1.0,
            "continuous flow must complete repeated cycles"
        );
        assert_eq!(
            end.fract(),
            0.0,
            "a finite stream must drain on a cycle boundary"
        );

        let reverse = crate::parse(
            "canvas(800,500); line(path,(100,250),(700,250)); flow(path,2,reverse,once);",
        )
        .unwrap();
        let (base, timeline) = reverse.finalize();
        assert!((timeline.apply(&base, 1.0).get("path").unwrap().flow_back - 0.5).abs() < 0.001);
        assert!((timeline.apply(&base, 2.0).get("path").unwrap().flow_back - 1.0).abs() < 0.001);

        let duplex = crate::parse(
            "canvas(800,500); line(path,(100,250),(700,250)); flow(path,2,both,continuous);",
        )
        .unwrap();
        let (base, timeline) = duplex.finalize();
        let frame = timeline.apply(&base, 0.75);
        assert!(frame.get("path").unwrap().flow > 0.0);
        assert!(frame.get("path").unwrap().flow_back > 0.0);
    }

    #[test]
    fn generic_flow_composition_does_not_require_connected_or_domain_objects() {
        let movie = crate::parse(
            r#"
            canvas(800,500);
            line(a,(100,250),(700,100)); tag(a,lanes);
            line(b,(100,250),(700,250)); tag(b,lanes);
            line(c,(100,250),(700,400)); tag(c,lanes);
            rect(selected,(100,250),20,20);
            circle(copy1,(100,250),8); rect(copy2,(100,250),16,16);
            text(copy3,(100,250),"X");
            spline(ribbon,(80,420),(240,330),(430,440),(720,320));
            hidden(ribbon.knots);
            step("select") { par { travel(selected,b,1,linear); flow(b,1); } }
            step("together") { par {
              travel(copy1,a,1,linear); travel(copy2,b,1,linear); travel(copy3,c,1,linear);
              flow(lanes,1,forward,once);
            } }
            step("free-design") { flow(ribbon,2,both,continuous); }
            "#,
        )
        .expect("generic objects and unconnected paths should compose without domain metadata");
        let (base, timeline) = movie.finalize();

        let selected = timeline.apply(&base, 0.5);
        assert_eq!(selected.get("a").unwrap().flow, 0.0);
        assert!(selected.get("b").unwrap().flow > 0.0);
        assert_eq!(selected.get("c").unwrap().flow, 0.0);

        let broadcast = timeline.apply(&base, 1.5);
        for lane in ["a", "b", "c"] {
            assert!(broadcast.get(lane).unwrap().flow > 0.0);
        }
        assert_eq!(
            timeline.apply(&base, 2.0).get("copy1").unwrap().pos,
            Vec2::new(700.0, 100.0)
        );
        assert_eq!(
            timeline.apply(&base, 2.0).get("copy2").unwrap().pos,
            Vec2::new(700.0, 250.0)
        );
        assert_eq!(
            timeline.apply(&base, 2.0).get("copy3").unwrap().pos,
            Vec2::new(700.0, 400.0)
        );

        let free_frame = timeline.apply(&base, 3.0);
        let free = free_frame.get("ribbon").unwrap();
        assert!(free.flow > 0.0 && free.flow_back > 0.0);
    }

    #[test]
    fn motion_vocabulary_rejects_invalid_targets() {
        let bad_container = crate::parse(
            "canvas(\"16:9\"); line(path, (10,10), (100,100)); particles(bits, path, 8);",
        )
        .err()
        .expect("line cannot contain particles")
        .to_string();
        assert!(bad_container.contains("circle or rect"), "{bad_container}");

        let too_large = crate::parse(
            "canvas(\"16:9\"); circle(tiny, (20,20), 2); particles(bits, tiny, 3, 4);",
        )
        .err()
        .expect("oversized particles must fail")
        .to_string();
        assert!(too_large.contains("larger than the circle"), "{too_large}");

        let bad_wander = crate::parse("canvas(\"16:9\"); wander(bits, 2);")
            .err()
            .expect("unknown particle group must fail")
            .to_string();
        assert!(bad_wander.contains("no particle group"), "{bad_wander}");

        let bad_flow = crate::parse("canvas(\"16:9\"); circle(c, (100,100), 20); flow(c, 1);")
            .err()
            .expect("circle is not a path")
            .to_string();
        assert!(bad_flow.contains("is not a line"), "{bad_flow}");
    }

    /// `image(id, at, "path", [w], [h])` builds a `Shape::Image` entity carrying
    /// the path + size; validates with default and explicit sizes.
    #[test]
    fn image_builds_shape() {
        use crate::primitives::Shape;
        let m =
            crate::parse("canvas(\"16:9\");\nimage(logo, (640, 360), \"foo.png\", 400, 200);\n")
                .unwrap();
        let e = m.base().get("logo").expect("image entity");
        match &e.shape {
            Shape::Image { path, w, h, .. } => {
                assert_eq!(path, "foo.png");
                assert_eq!((*w, *h), (400.0, 200.0));
            }
            other => panic!("expected Shape::Image, got {other:?}"),
        }
        let bundled =
            crate::parse("canvas(320,180); image(logo,(160,90),\"asset:manic-logo.png\",64,64);")
                .unwrap();
        assert!(matches!(
            &bundled.base().get("logo").unwrap().shape,
            Shape::Image { path, .. } if path.ends_with("assets/manic-logo.png")
        ));
        // an equation renders (via RaTeX) to a tinted Shape::Image with real px dims
        let e = crate::parse(
            "canvas(\"16:9\");\nequation(q, (640, 360), `\\frac{1}{2}+\\sqrt{x}`, 48);\n",
        )
        .unwrap();
        match &e.base().get("q").expect("equation entity").shape {
            Shape::Image { tint, w, h, path } => {
                assert!(*tint, "equation image must be tinted by entity colour");
                assert!(*w > 0.0 && *h > 0.0, "equation should have real pixel dims");
                assert!(path.ends_with(".png"), "equation caches a PNG: {path}");
            }
            other => panic!("expected equation Shape::Image, got {other:?}"),
        }
        let colored = crate::parse(
            "canvas(\"16:9\"); equation(q,(cx,cy),`\\textcolor{cyan}{x}=\\textcolor{gold}{1}`,48);",
        )
        .unwrap();
        match &colored.base().get("q").unwrap().shape {
            Shape::Image { tint, .. } => {
                assert!(!*tint, "term-coloured equations preserve their own pixels")
            }
            other => panic!("expected coloured equation Shape::Image, got {other:?}"),
        }
        // defaults: w=300 square, and it validates in a scene
        let m2 =
            crate::parse("canvas(\"16:9\");\nimage(l, (100, 100), \"x.png\");\nshow(l, 0.5);\n")
                .unwrap();
        assert!(
            m2.validate().is_ok(),
            "image + show should validate: {:?}",
            m2.validate().err()
        );
    }

    #[test]
    fn rewrite_keeps_one_equation_id_and_exact_settled_latex() {
        use crate::primitives::Shape;

        let movie = crate::parse(
            "canvas(1280,720);\n\
             equation(work,(cx,cy),`x+x=2`,64);\n\
             rewrite(work,`2x=2`,0.8);\n\
             rewrite(work,`x=1`,0.8);\n",
        )
        .unwrap();
        assert!(
            !movie.base().pending_eq_parts.is_empty(),
            "opt-in rewrite should decompose RaTeX display items"
        );
        assert_eq!(
            movie
                .base()
                .entities
                .iter()
                .filter(|e| e.id == "work")
                .count(),
            1,
            "the public equation id stays stable"
        );

        let (base, timeline) = movie.finalize();
        let moving = timeline.apply(&base, 0.4);
        assert_eq!(moving.get("work").unwrap().opacity, 0.0);
        assert!(
            moving
                .entities
                .iter()
                .filter(|e| e.id.starts_with("__rewrite.work.1.from."))
                .any(|e| e.opacity > 0.0),
            "matched source parts should be visible during the transition"
        );

        let final_frame = timeline.apply(&base, 1.6);
        let work = final_frame.get("work").unwrap();
        assert!((work.opacity - 1.0).abs() < 1e-6);
        match &work.shape {
            Shape::Image { path, .. } => {
                assert_eq!(path, &crate::latex::eq_path("x=1", 64.0));
            }
            other => panic!("settled rewrite must be the exact equation image: {other:?}"),
        }

        // Stateless guarantee: querying an earlier time after the final frame
        // reconstructs the same intermediate state.
        let moving_again = timeline.apply(&base, 0.4);
        assert_eq!(moving_again.get("work").unwrap().opacity, 0.0);
    }

    #[test]
    fn rewrite_new_rhs_starts_appearing_immediately() {
        let movie = crate::parse(
            "canvas(1280,720); equation(work,(cx,cy),`x=0`,64); rewrite(work,`x=0+y`,1.0,smooth);",
        )
        .unwrap();
        assert!(
            movie
                .base()
                .entities
                .iter()
                .any(|e| e.id.starts_with("__rewrite.work.1.to.")),
            "this compatible rewrite should introduce unmatched RHS parts"
        );
        let (base, timeline) = movie.finalize();
        let early = timeline.apply(&base, 0.10);
        assert!(
            early.entities.iter().any(|e| {
                e.id.starts_with("__rewrite.work.1.to.")
                    && e.pos.x > base.canvas().x * 0.5
                    && e.opacity > 0.0
            }),
            "new RHS content must begin entering before 36% of the rewrite"
        );
    }

    #[test]
    fn rewrite_replacement_leaves_before_the_new_glyph_settles() {
        let from = r"x=2";
        let to = r"x=3";
        let from_layout = crate::latex::layout_parts(from, 52.0).unwrap();
        let to_layout = crate::latex::layout_parts(to, 52.0).unwrap();
        let old_index = from_layout
            .parts
            .iter()
            .position(|part| part.symbol == Some('2' as u32))
            .expect("old replacement glyph");
        let new_index = to_layout
            .parts
            .iter()
            .position(|part| part.symbol == Some('3' as u32))
            .expect("new replacement glyph");
        let movie = crate::parse(
            "canvas(1280,720); equation(work,(cx,cy),`x=2`,52); rewrite(work,`x=3`,1.0,smooth);",
        )
        .unwrap();
        let (base, timeline) = movie.finalize();
        let midpoint = timeline.apply(&base, 0.50);
        let old = midpoint
            .get(&format!("__rewrite.work.1.from.{old_index}"))
            .expect("old glyph layer");
        let new = midpoint
            .get(&format!("__rewrite.work.1.to.{new_index}"))
            .expect("new glyph layer");
        assert!(old.opacity <= 1e-4, "the old 2 must be gone at midpoint");
        assert!(
            new.opacity >= 0.45,
            "the new 3 should already be readable at midpoint"
        );
    }

    #[test]
    fn rewrite_fallback_stays_visible_without_equal_strength_ghosting() {
        let target = r"A^2=\begin{bmatrix}0&0&1\\0&-1&2\\1&-1&1\end{bmatrix}";
        let src = format!(
            "canvas(1280,720); equation(work,(cx,cy),`A^{{-1}}\\stackrel{{?}}{{=}}A^2`,52); rewrite(work,`{target}`,1.0,smooth);"
        );
        let movie = crate::parse(&src).unwrap();
        assert!(
            movie.base().contains("__rewrite.work.1.target"),
            "matrix topology changes should select the continuity-safe fallback"
        );
        assert!(
            movie
                .base()
                .get("__rewrite.work.1.target")
                .unwrap()
                .tags
                .is_empty(),
            "temporary rewrite layers must not leak into public tag broadcasts"
        );
        let (base, timeline) = movie.finalize();
        for frame in 0..=60 {
            let t = frame as f32 / 60.0;
            let scene = timeline.apply(&base, t);
            let source_alpha = scene.get("work").unwrap().opacity;
            let target_alpha = scene.get("__rewrite.work.1.target").unwrap().opacity;
            assert!(
                source_alpha + target_alpha >= 0.37,
                "rewrite became unreadably dim at t={t:.3}: source={source_alpha:.3}, target={target_alpha:.3}"
            );
        }
        let midpoint = timeline.apply(&base, 0.50);
        assert!(midpoint.get("work").unwrap().opacity <= 1e-4);
        assert!(midpoint.get("__rewrite.work.1.target").unwrap().opacity >= 0.45);
    }

    #[test]
    fn rewrite_preserves_quadratic_rhs_as_a_stable_subexpression() {
        let from = r"x\left(x+\frac{b}{2a}\right)+\frac{b}{2a}\left(x+\frac{b}{2a}\right)=\frac{b^2-4ac}{4a^2}";
        let to = r"\left(x+\frac{b}{2a}\right)^2=\frac{b^2-4ac}{4a^2}";
        let from_layout = crate::latex::layout_parts(from, 48.0).unwrap();
        let to_layout = crate::latex::layout_parts(to, 48.0).unwrap();
        let mut plan = match_equation_parts(&from_layout, &to_layout);
        assert!(
            prefer_local_side_dissolve(&mut plan, &from_layout, &to_layout, from, to),
            "factoring should dissolve only the changing LHS instead of inventing glyph paths"
        );

        let target_equals_x = to_layout
            .parts
            .iter()
            .find(|p| p.symbol == Some('=' as u32))
            .expect("target equals sign")
            .offset
            .x;
        let rhs_indices: Vec<_> = to_layout
            .parts
            .iter()
            .enumerate()
            .filter(|(_, p)| p.offset.x > target_equals_x + 2.0)
            .map(|(i, _)| i)
            .collect();
        let matched_rhs_area: f32 = plan
            .pairs
            .iter()
            .filter(|(_, ti)| rhs_indices.contains(ti))
            .map(|(_, ti)| equation_part_area(&to_layout.parts[*ti]))
            .sum();
        let rhs_area: f32 = rhs_indices
            .iter()
            .map(|ti| equation_part_area(&to_layout.parts[*ti]))
            .sum();
        assert!(
            matched_rhs_area / rhs_area.max(1.0) >= 0.90,
            "an unchanged quadratic RHS should remain visible as stable matched parts"
        );

        let source_equals_x = from_layout
            .parts
            .iter()
            .find(|p| p.symbol == Some('=' as u32))
            .expect("source equals sign")
            .offset
            .x;
        assert!(
            plan.pairs.iter().all(|(si, ti)| {
                from_layout.parts[*si].offset.x >= source_equals_x - 1.0
                    && to_layout.parts[*ti].offset.x >= target_equals_x - 1.0
            }),
            "the changing LHS must not retain arbitrary glyph-to-glyph matches"
        );
    }

    #[test]
    fn rewrite_dissolves_the_changed_side_of_the_reported_quadratic_step() {
        let from = r"x^2+\frac{b}{a}x=-\frac{c}{a}";
        let to = r"x^2+\frac{b}{2a}x+\frac{b}{2a}x=-\frac{c}{a}";
        let from_layout = crate::latex::layout_parts(from, 48.0).unwrap();
        let to_layout = crate::latex::layout_parts(to, 48.0).unwrap();
        let mut plan = match_equation_parts(&from_layout, &to_layout);

        assert!(
            prefer_local_side_dissolve(&mut plan, &from_layout, &to_layout, from, to),
            "adding two fractions changes LHS topology and should select a local dissolve"
        );
        let source_equals_x = from_layout
            .parts
            .iter()
            .find(|p| p.symbol == Some('=' as u32))
            .expect("source equals sign")
            .offset
            .x;
        let target_equals_x = to_layout
            .parts
            .iter()
            .find(|p| p.symbol == Some('=' as u32))
            .expect("target equals sign")
            .offset
            .x;
        assert!(
            plan.pairs.iter().all(|(si, ti)| {
                from_layout.parts[*si].offset.x >= source_equals_x - 1.0
                    && to_layout.parts[*ti].offset.x >= target_equals_x - 1.0
            }),
            "x^2 and every other changing-LHS glyph must dissolve instead of jumping"
        );
    }

    #[test]
    fn rewrite_never_turns_a_quadratic_exponent_into_a_denominator() {
        use crate::latex::EquationPartRole;

        let from = r"x^2+\frac{b}{a}x=-\frac{c}{a}";
        let to = r"x^2+\frac{b}{2a}x+\frac{b}{2a}x=-\frac{c}{a}";
        let from_layout = crate::latex::layout_parts(from, 48.0).unwrap();
        let to_layout = crate::latex::layout_parts(to, 48.0).unwrap();
        let plan = match_equation_parts(&from_layout, &to_layout);
        let source_square = from_layout
            .parts
            .iter()
            .position(|part| {
                part.symbol == Some('2' as u32) && part.role == EquationPartRole::Above
            })
            .expect("source x-squared exponent");
        let target_index = plan
            .pairs
            .iter()
            .find_map(|(si, ti)| (*si == source_square).then_some(*ti))
            .expect("source exponent should retain identity");
        assert_eq!(
            to_layout.parts[target_index].role,
            EquationPartRole::Above,
            "the exponent from x^2 must remain the exponent; denominator twos enter separately"
        );
        assert!(
            to_layout.parts.iter().any(|part| {
                part.symbol == Some('2' as u32) && part.role == EquationPartRole::Denominator
            }),
            "fixture must contain the denominator twos reported by the reviewer"
        );
    }

    #[test]
    fn rewrite_keeps_nested_scripts_derivative_orders_and_limits_distinct() {
        let derivative_from = crate::latex::layout_parts(r"\frac{d^2y}{dx^2}", 48.0).unwrap();
        let derivative_to = crate::latex::layout_parts(r"\frac{2dy}{dx}", 48.0).unwrap();
        let derivative_plan = match_equation_parts(&derivative_from, &derivative_to);
        assert!(
            !derivative_plan.pairs.iter().any(|(si, ti)| {
                derivative_from.parts[*si].symbol == Some('2' as u32)
                    && derivative_to.parts[*ti].symbol == Some('2' as u32)
            }),
            "the order in d² must not become an ordinary numerator coefficient"
        );

        let tower_from = crate::latex::layout_parts(r"y=e^{e^{e^x}}", 48.0).unwrap();
        let tower_to = crate::latex::layout_parts(r"y=e^{e^{e^{x+1}}}", 48.0).unwrap();
        let tower_plan = match_equation_parts(&tower_from, &tower_to);
        let nested_e_pairs: Vec<_> = tower_plan
            .pairs
            .iter()
            .filter(|(si, ti)| {
                tower_from.parts[*si].symbol == Some('e' as u32)
                    && tower_to.parts[*ti].symbol == Some('e' as u32)
            })
            .collect();
        assert!(
            nested_e_pairs.len() >= 3,
            "the existing exponential tower should remain recognisable"
        );
        assert!(nested_e_pairs.iter().all(|(si, ti)| {
            equation_parts_can_match(&tower_from.parts[*si], &tower_to.parts[*ti])
        }));

        let integral_from = crate::latex::layout_parts(r"\int_0^1 f(x)\,dx", 48.0).unwrap();
        let integral_to = crate::latex::layout_parts(r"\int_1^0 f(x)\,dx", 48.0).unwrap();
        let integral_plan = match_equation_parts(&integral_from, &integral_to);
        assert!(
            !integral_plan.pairs.iter().any(|(si, ti)| {
                let source = &integral_from.parts[*si];
                let target = &integral_to.parts[*ti];
                matches!(source.symbol, Some(code) if code == '0' as u32 || code == '1' as u32)
                    && source.symbol == target.symbol
            }),
            "upper and lower integral limits must leave and re-enter rather than swap jobs"
        );
    }

    #[test]
    fn rewrite_matrix_repeated_entries_keep_reading_order() {
        let from = r"A^2=\begin{bmatrix}0&0&1\\0&-1&2\\1&-1&1\end{bmatrix}";
        let to = r"A^3=\begin{bmatrix}1&0&0\\0&1&0\\0&0&1\end{bmatrix}=I";
        let from_layout = crate::latex::layout_parts(from, 48.0).unwrap();
        let to_layout = crate::latex::layout_parts(to, 48.0).unwrap();
        let plan = match_equation_parts(&from_layout, &to_layout);

        let zero_pairs: Vec<_> = plan
            .pairs
            .iter()
            .copied()
            .filter(|(si, ti)| {
                from_layout.parts[*si].symbol == Some('0' as u32)
                    && to_layout.parts[*ti].symbol == Some('0' as u32)
            })
            .collect();
        assert!(
            zero_pairs.len() >= 2,
            "expected repeated matrix zeros to match"
        );
        assert!(
            zero_pairs.windows(2).all(|w| w[0].1 < w[1].1),
            "repeated matrix entries must not cross or reverse their reading order: {zero_pairs:?}"
        );
    }

    #[test]
    fn rewrite_is_opt_in_and_rejects_non_equations_or_bad_latex() {
        let ordinary = crate::parse("canvas(1280,720); equation(q,(cx,cy),`x^2`,48);")
            .expect("ordinary equation remains unchanged");
        assert!(ordinary.base().pending_eq_parts.is_empty());

        let non_equation =
            crate::parse("canvas(1280,720); text(label,(cx,cy),\"x\"); rewrite(label,`x+1`);")
                .err()
                .expect("rewrite should require an equation")
                .to_string();
        assert!(non_equation.contains("needs an equation"), "{non_equation}");

        let bad_latex =
            crate::parse("canvas(1280,720); equation(q,(cx,cy),`x`,48); rewrite(q,`\\frac{`);")
                .err()
                .expect("malformed target LaTeX should fail")
                .to_string();
        assert!(bad_latex.contains("rewrite"), "{bad_latex}");
    }

    #[test]
    fn rewrite_chain_fits_portrait_without_scale_breathing() {
        let long = r"\frac{a_1}{b_1}+\frac{a_2}{b_2}+\frac{a_3}{b_3}+\frac{a_4}{b_4}+\frac{a_5}{b_5}+\frac{a_6}{b_6}=0";
        let src = format!(
            "canvas(\"9:16\"); equation(q,(cx,cy),`x=0`,72); rewrite(q,`{long}`,0.8); rewrite(q,`x=1`,0.8);"
        );
        let movie = crate::parse(&src).unwrap();
        let state = &movie.base().equation_states["q"];
        assert!(
            state.visual_scale < 1.0,
            "portrait overflow should auto-fit"
        );
        let (long_w, _, _) = crate::latex::layout_dims(long, 72.0).unwrap();
        assert!(
            long_w * state.visual_scale <= movie.base().canvas().x * 0.90 + 0.1,
            "the widest state must fit the equation-safe width"
        );
        let (_, timeline) = movie.finalize();
        let final_scale = timeline.apply(movie.base(), 1.6).get("q").unwrap().scale;
        assert!((final_scale - state.visual_scale).abs() < 1e-6);
    }

    #[test]
    fn rewrite_latex_corpus_covers_creator_math_and_physics_notation() {
        use crate::primitives::Shape;

        // These are notation families, not special-cased domains. Every row
        // travels through the same public equation/rewrite pipeline and must
        // settle on the exact RaTeX-rendered target image.
        let cases = [
            (
                "algebraic rearrangement",
                r"2(x+3)=14",
                r"2x=8\quad\Rightarrow\quad x=4",
            ),
            (
                "integrals and derivatives",
                r"F(x)=\int_0^x t^2\,dt",
                r"F'(x)=\frac{d}{dx}\int_0^x t^2\,dt=x^2",
            ),
            (
                "fractions roots powers and limits",
                r"x^2+\sqrt{x}+\frac{1}{x}",
                r"\lim_{x\to0}\frac{\sqrt{1+x}-1}{x}=\frac{1}{2}",
            ),
            (
                "nested exponential towers",
                r"y=e^{e^{e^x}}",
                r"y=e^{e^{e^{x+1}}}",
            ),
            (
                "logarithms with compound bases",
                r"y=\log_{e^t}x",
                r"y=\frac{\ln x}{t}",
            ),
            (
                "complex contour integrals",
                r"I=\oint_{\Gamma}\frac{f(z)}{z-z_0}\,dz",
                r"I=2\pi i f(z_0)",
            ),
            (
                "differential limits",
                r"f'(x)=\lim_{h\to0}\frac{f(x+h)-f(x)}{h}",
                r"f'(x)=2x",
            ),
            (
                "ordinary differential equations",
                r"\frac{d^2y}{dt^2}+\omega^2y=0",
                r"y(t)=A\cos(\omega t)+B\sin(\omega t)",
            ),
            (
                "partial differential equations",
                r"\frac{\partial u}{\partial t}=\alpha\frac{\partial^2u}{\partial x^2}",
                r"u(x,t)=e^{-\alpha k^2t}\sin(kx)",
            ),
            (
                "trigonometric identities",
                r"\sin^2\theta+\cos^2\theta",
                r"\sin^2\theta+\cos^2\theta=1",
            ),
            (
                "set notation and logic",
                r"x\in A\cap B",
                r"(x\in A)\land(x\in B)",
            ),
            (
                "summations and products",
                r"\sum_{k=1}^{n}k",
                r"\prod_{k=1}^{n}k=n!",
            ),
            (
                "physics formulas and units",
                r"F=ma",
                r"[F]=\mathrm{kg}\cdot\mathrm{m}\cdot\mathrm{s}^{-2}",
            ),
            (
                "probability expressions",
                r"P(A\mid B)",
                r"P(A\mid B)=\frac{P(B\mid A)P(A)}{P(B)}",
            ),
            (
                "matrices and vectors",
                r"\vec v=\begin{bmatrix}1\\2\end{bmatrix}",
                r"A\vec v=\begin{bmatrix}a&b\\c&d\end{bmatrix}\begin{bmatrix}1\\2\end{bmatrix}",
            ),
            (
                "text mixed with mathematical notation",
                r"\text{Area of a circle}=\pi r^2",
                r"\text{when }r=2,\quad A=4\pi",
            ),
            (
                "user-created notation",
                r"\mathcal{R}_{\star}(x)\equiv x^2+1",
                r"\mathcal{R}_{\star}(2)=5",
            ),
        ];

        for (name, from, to) in cases {
            let src = format!(
                "canvas(1280,720); equation(work,(cx,cy),`{from}`,42); rewrite(work,`{to}`,0.2,smooth);"
            );
            let movie = crate::parse(&src)
                .unwrap_or_else(|e| panic!("{name} should parse and lower through rewrite: {e}"));
            assert!(
                !movie.base().pending_eq_parts.is_empty()
                    || movie
                        .base()
                        .entities
                        .iter()
                        .any(|e| e.id == "__rewrite.work.1.target"),
                "{name} should use either structured parts or the safe hybrid fallback"
            );
            let (base, timeline) = movie.finalize();
            let settled = timeline.apply(&base, 0.2);
            let work = settled.get("work").expect("persistent equation id");
            match &work.shape {
                Shape::Image { path, .. } => assert_eq!(
                    path,
                    &crate::latex::eq_path(to, 42.0),
                    "{name} must settle on the exact target equation"
                ),
                other => panic!("{name} settled as {other:?}, not an equation image"),
            }
            assert!(
                (work.opacity - 1.0).abs() < 1e-6,
                "{name} must settle visible"
            );
        }

        let mixed =
            crate::parse("canvas(1280,720); text(note,(cx,cy),`Energy $E=mc^2$ uses mass $m$.`);")
                .expect("ordinary prose with multiple inline formulas should typeset");
        assert!(
            matches!(
                mixed.base().get("note").unwrap().shape,
                Shape::RichText { .. }
            ),
            "mixed prose/math should remain a rich-text entity"
        );
    }

    #[test]
    fn dashed_is_a_generic_path_modifier() {
        let movie = crate::parse(
            "canvas(1280,720);\
             line(guide,(100,100),(500,100)); dashed(guide);\
             plot(curve,(640,360),80,80,\"sin(x)\",(-3,3)); dashed(curve,20,7);",
        )
        .expect("line and plot should share the same dashed modifier");
        assert_eq!(movie.base().get("guide").unwrap().dash, Some((16.0, 10.0)));
        assert_eq!(movie.base().get("curve").unwrap().dash, Some((20.0, 7.0)));

        let bad = crate::parse("canvas(1280,720); circle(c,(cx,cy),80); dashed(c);")
            .err()
            .expect("filled shapes are not path-like")
            .to_string();
        assert!(bad.contains("path-like"), "{bad}");
    }

    /// Holistic inline LaTeX: a whole-`$…$` string in ANY text (plain `text`, a
    /// geo point label, a `caption`) is typeset to a tinted equation image by the
    /// build post-pass — with zero per-kit code — while plain text is untouched.
    #[test]
    fn inline_dollar_math_typeset_everywhere() {
        use crate::primitives::Shape;
        let m = crate::parse(
            "canvas(\"16:9\");\n\
             text(plain, (cx, 80), \"just x^2 text\");\n\
             text(cap, (cx, 200), `$E = mc^2$`);\n\
             point(A, (cx, 300), `$\\alpha$`);\n\
             caption(c2, `$\\int_0^1 x\\,dx$`, (cx, 400));\n",
        )
        .unwrap();
        let base = m.base();
        // plain text (no `$`) is byte-identically untouched → still Text
        assert!(
            matches!(base.get("plain").unwrap().shape, Shape::Text { .. }),
            "plain text must not change"
        );
        // every whole-`$…$` label became a tinted equation image
        for id in ["cap", "A.label", "c2.w0"] {
            match &base.get(id).unwrap_or_else(|| panic!("missing {id}")).shape {
                Shape::Image { tint, .. } => {
                    assert!(*tint, "{id} should be a tinted equation image")
                }
                o => panic!("{id}: expected typeset image, got {o:?}"),
            }
        }
    }

    /// Mixed text + inline `$…$` on one line → `RichText` with text·math·text runs
    /// (Phase 2b). Plain strings (no `$`) stay `Shape::Text` — no regression.
    #[test]
    fn mixed_inline_math_becomes_richtext() {
        use crate::primitives::{Shape, TextRun};
        let m = crate::parse(
            "canvas(\"16:9\");\ntext(t, (cx, 100), `The area is $\\pi r^2$ units`);\n",
        )
        .unwrap();
        match &m.base().get("t").unwrap().shape {
            Shape::RichText { runs, .. } => {
                assert!(
                    matches!(runs.first(), Some(TextRun::Text(_))),
                    "starts with text"
                );
                assert!(
                    runs.iter().any(|r| matches!(r, TextRun::Math { .. })),
                    "has a math run"
                );
                assert!(
                    matches!(runs.last(), Some(TextRun::Text(_))),
                    "ends with text"
                );
            }
            o => panic!("expected RichText, got {o:?}"),
        }
        let p = crate::parse("canvas(\"16:9\");\ntext(t, (cx, 100), \"plain only\");\n").unwrap();
        assert!(
            matches!(p.base().get("t").unwrap().shape, Shape::Text { .. }),
            "no-$ stays Text"
        );
    }

    /// `sticky(id)` sets the screen-pin flag on an entity, and broadcasts over a tag.
    #[test]
    fn sticky_pins_entity_and_broadcasts() {
        let m = crate::parse(
            "canvas(\"16:9\");\n\
             text(hud, (cx, 40), \"score\"); display(hud); sticky(hud);\n\
             text(a, (100, 100), \"x\"); tag(a, grp);\n\
             text(b, (200, 100), \"y\"); tag(b, grp);\n\
             sticky(grp);\n",
        )
        .unwrap();
        assert!(
            m.base().get("hud").unwrap().sticky,
            "sticky(hud) should pin the entity"
        );
        assert!(
            m.base().get("a").unwrap().sticky,
            "sticky(grp) should broadcast to tagged `a`"
        );
        assert!(
            m.base().get("b").unwrap().sticky,
            "sticky(grp) should broadcast to tagged `b`"
        );
        assert!(m.validate().is_ok());
    }

    #[test]
    fn attach_follows_then_releases_without_a_snap() {
        let movie = crate::parse(
            "canvas(800,600);\n\
             dot(anchor,(100,200),8);\n\
             text(note,(100,170),\"tracking\");\n\
             attach(note,anchor,(0,-30));\n\
             move(anchor,(300,200),1,linear);\n\
             attach(note,none);\n\
             move(anchor,(500,200),1,linear);",
        )
        .expect("attach/release should lower without flags");
        let (base, timeline) = movie.finalize();

        let moving = timeline.apply(&base, 0.5);
        assert!((moving.get("anchor").unwrap().pos.x - 200.0).abs() < 0.01);
        assert!((moving.get("note").unwrap().pos.x - 200.0).abs() < 0.01);
        assert!((moving.get("note").unwrap().pos.y - 170.0).abs() < 0.01);

        let released = timeline.apply(&base, 1.5);
        assert!((released.get("anchor").unwrap().pos.x - 400.0).abs() < 0.01);
        assert!((released.get("note").unwrap().pos.x - 300.0).abs() < 0.01);
        assert!((released.get("note").unwrap().pos.y - 170.0).abs() < 0.01);
        assert!(released.get("note").unwrap().follow.is_none());

        // Scrubbing backwards after sampling the end must reproduce the same
        // attached midpoint exactly.
        let backwards = timeline.apply(&base, 0.5);
        assert_eq!(
            backwards.get("note").unwrap().pos,
            moving.get("note").unwrap().pos
        );
    }

    #[test]
    fn attach_rejects_cycles_at_build_time() {
        let error = crate::parse(
            "canvas(800,600); dot(a,(100,100)); dot(b,(200,100)); attach(a,b); attach(b,a);",
        )
        .err()
        .expect("attachment cycles must be rejected")
        .to_string();
        assert!(error.contains("cycle"), "{error}");
    }

    #[test]
    fn become_interpolates_compatible_shapes_and_settles_exactly() {
        let movie = crate::parse(
            "canvas(800,600);\n\
             circle(source,(100,250),20); color(source,cyan); outlined(source); stroke(source,2);\n\
             circle(goal,(300,250),80); color(goal,magenta); outlined(goal); stroke(goal,9); hidden(goal);\n\
             become(source,goal,1,linear);",
        )
        .expect("compatible become should lower");
        let (base, timeline) = movie.finalize();
        let halfway = timeline.apply(&base, 0.5);
        let source = halfway.get("source").unwrap();
        assert!((source.pos.x - 200.0).abs() < 0.01);
        assert!(matches!(source.shape, Shape::Circle { r } if (r - 50.0).abs() < 0.01));
        assert!((source.stroke.width - 5.5).abs() < 0.01);

        let settled = timeline.apply(&base, 1.0);
        let source = settled.get("source").unwrap();
        let goal = settled.get("goal").unwrap();
        assert_eq!(source.shape, Shape::Circle { r: 80.0 });
        assert_eq!(source.pos, Vec2::new(300.0, 250.0));
        assert_eq!(source.color, crate::style::MAGENTA);
        assert!((source.stroke.width - 9.0).abs() < 1e-6);
        assert!((source.opacity - 1.0).abs() < 1e-6);
        assert!(
            (goal.opacity - 0.0).abs() < 1e-6,
            "blueprint visibility must not change"
        );
    }

    #[test]
    fn become_uses_a_safe_local_crossfade_for_incompatible_shapes() {
        let movie = crate::parse(
            "canvas(800,600); circle(source,(100,250),20); rect(goal,(300,250),120,70); hidden(goal); become(source,goal,1,linear);",
        )
        .expect("incompatible shapes should use the documented fallback");
        let (base, timeline) = movie.finalize();
        let midpoint = timeline.apply(&base, 0.5);
        assert!(midpoint.get("source").unwrap().opacity.abs() < 1e-6);
        let settled = timeline.apply(&base, 1.0);
        assert_eq!(
            settled.get("source").unwrap().shape,
            Shape::Rect { w: 120.0, h: 70.0 }
        );
        assert!((settled.get("source").unwrap().opacity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn turn_rotates_a_moved_group_and_path_rigidly_about_one_pivot() {
        let movie = crate::parse(
            "canvas(800,600);\n\
             dot(pivot,(0,0),3); hidden(pivot);\n\
             dot(a,(100,0),5); tag(a,rotor);\n\
             dot(b,(0,100),5); tag(b,rotor);\n\
             line(ray,(100,0),(200,0)); tag(ray,rotor);\n\
             move(a,(200,0),1,linear);\n\
             turn(rotor,pivot,90,1,linear);",
        )
        .expect("turn should accept an entity pivot and tag target");
        let (base, timeline) = movie.finalize();
        let settled = timeline.apply(&base, 2.0);
        assert!(
            settled
                .get("a")
                .unwrap()
                .pos
                .distance(Vec2::new(0.0, 200.0))
                < 0.02
        );
        assert!(
            settled
                .get("b")
                .unwrap()
                .pos
                .distance(Vec2::new(-100.0, 0.0))
                < 0.02
        );
        let ray = settled.get("ray").unwrap();
        assert!(ray.pos.distance(Vec2::new(0.0, 100.0)) < 0.02);
        assert!(
            matches!(ray.shape, Shape::Line { to } if to.distance(Vec2::new(0.0, 200.0)) < 0.02)
        );

        let middle = timeline.apply(&base, 1.5);
        assert!((middle.get("a").unwrap().pos.length() - 200.0).abs() < 0.05);
        let repeat = timeline.apply(&base, 2.0);
        assert_eq!(repeat.get("a").unwrap().pos, settled.get("a").unwrap().pos);
    }

    /// `support(...)` lays out a hatched support (baseline + ticks), the bare id
    /// broadcasts over the whole thing, and the `dir` string is accepted.
    #[test]
    fn support_builds_hatched_baseline() {
        let m = crate::parse(
            "canvas(\"16:9\");\n\
             support(ceil, (cx, 100), 300);\n\
             support(floor, (cx, 600), 200, \"up\");\n\
             color(ceil, cyan);\n",
        )
        .unwrap();
        for sub in ["ceil.line", "ceil.tick0", "floor.line", "floor.tick0"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        // the bare id tags every part, so color(ceil, …) broadcasts to the ticks
        assert_eq!(
            m.base().get("ceil.tick0").unwrap().color,
            crate::style::CYAN
        );
        assert!(m.validate().is_ok());
    }
}
