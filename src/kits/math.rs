//! The **math kit**: coordinate frames, function plots, vectors, number
//! lines. The first domain built on the manic core.
//!
//! Everything here is a *composition* of core primitives registered as a
//! constructor — `axes` is two arrows, `plot` is a sampled polyline, a
//! `vector` is an arrow. Adding this kit touches no core code, which is the
//! whole point of the registry design. LaTeX typesetting is a later addition;
//! labels are mono text for now.

use macroquad::prelude::Vec2;

use crate::lang::diag::Error;
use crate::lang::lower::{resolve_color, Args, Registry};
use crate::primitives::{Entity, FontKind, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;

/// Evaluate a named function at `x`. Returns `None` for names we don't know
/// and non-finite results (asymptotes), so the caller can break the polyline.
fn eval(name: &str, x: f32) -> Option<f32> {
    let y = match name {
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "parabola" | "sq" | "square" => x * x,
        "cubic" | "cube" => x * x * x,
        "line" | "id" | "identity" => x,
        "abs" => x.abs(),
        "exp" => x.exp(),
        "sqrt" if x >= 0.0 => x.sqrt(),
        "log" | "ln" if x > 0.0 => x.ln(),
        "recip" | "inv" if x.abs() > 1e-3 => 1.0 / x,
        "gauss" | "bell" => (-x * x).exp(),
        _ => return None,
    };
    y.is_finite().then_some(y)
}

fn known_fn(name: &str) -> bool {
    matches!(
        name,
        "sin" | "cos" | "tan" | "parabola" | "sq" | "square" | "cubic" | "cube" | "line" | "id"
            | "identity" | "abs" | "exp" | "sqrt" | "log" | "ln" | "recip" | "inv" | "gauss"
            | "bell"
    )
}

fn stroked(color: macroquad::prelude::Color, width: f32) -> StrokeStyle {
    StrokeStyle {
        fill: false,
        outline: true,
        width,
        outline_color: Some(color),
    }
}

fn fmt_num(v: f32) -> String {
    if (v.fract()).abs() < 1e-4 {
        format!("{}", v as i64)
    } else {
        format!("{v:.1}")
    }
}

// ---- builtins -------------------------------------------------------------

/// `axes(id, (cx,cy), halfw, halfh)` — a cyan-dim coordinate cross with
/// arrowheads on the +x and +y ends. Children `{id}.x`, `{id}.y`, tag `id`.
fn c_axes(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let hw = a.num(2)?;
    let hh = a.num(3)?;

    let mut x = Entity::new(
        format!("{id}.x"),
        Shape::Arrow {
            to: Vec2::new(c.x + hw, c.y),
        },
        Vec2::new(c.x - hw, c.y),
        style::DIM,
    );
    x.stroke.width = 2.0;
    x.tags.push(id.clone());
    s.add(x);

    // y grows up the screen (smaller y), so the head is at the top
    let mut y = Entity::new(
        format!("{id}.y"),
        Shape::Arrow {
            to: Vec2::new(c.x, c.y - hh),
        },
        Vec2::new(c.x, c.y + hh),
        style::DIM,
    );
    y.stroke.width = 2.0;
    y.tags.push(id);
    s.add(y);
    Ok(())
}

/// `plot(id, (cx,cy), sx, sy, fn, [domain])` — sample a named function over
/// `x ∈ [-domain, domain]` (default 6) and draw it as a glowing polyline in
/// screen space: `(cx + x*sx, cy - f(x)*sy)`.
fn c_plot(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let sx = a.num(2)?;
    let sy = a.num(3)?;
    let f = a.ident(4)?;
    let domain = a.opt_num(5)?.unwrap_or(6.0);
    if !known_fn(&f) {
        return Err(Error::new(
            format!(
                "unknown function `{f}` (try: sin, cos, tan, parabola, cubic, line, abs, exp, sqrt, log, recip, gauss)"
            ),
            a.span_of(4),
        ));
    }

    const N: usize = 240;
    let mut pts = Vec::with_capacity(N + 1);
    for i in 0..=N {
        let x = -domain + 2.0 * domain * i as f32 / N as f32;
        if let Some(y) = eval(&f, x) {
            pts.push(Vec2::new(c.x + x * sx, c.y - y * sy));
        }
    }
    let mut e = Entity::new(id.clone(), Shape::Polyline { pts }, Vec2::ZERO, style::CYAN);
    e.stroke = stroked(style::CYAN, 3.0);
    e.tags.push(id);
    s.add(e);
    Ok(())
}

/// `vector(id, (cx,cy), (dx,dy), [color])` — a glowing arrow from the origin
/// to origin + (dx, -dy) (dy is up). Defaults to magenta.
fn c_vector(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let o = a.pair(1)?;
    let d = a.pair(2)?;
    let color = if a.len() > 3 {
        resolve_color(&a.ident(3)?, a.span_of(3))?
    } else {
        style::MAGENTA
    };
    let mut e = Entity::new(
        id,
        Shape::Arrow {
            to: Vec2::new(o.x + d.x, o.y - d.y),
        },
        o,
        color,
    );
    e.stroke.width = 3.0;
    s.add(e);
    Ok(())
}

/// `numberline(id, (cx,cy), halfw, from, to, step)` — a dim axis with ticks
/// and labels. Children `{id}.axis`, `{id}.tN`, `{id}.lN`, tag `id`.
fn c_numberline(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let hw = a.num(2)?;
    let from = a.num(3)?;
    let to = a.num(4)?;
    let step = a.num(5)?;
    if step <= 0.0 || to <= from {
        return Err(Error::new(
            "numberline needs step > 0 and to > from",
            a.name_span,
        ));
    }

    let mut axis = Entity::new(
        format!("{id}.axis"),
        Shape::Arrow {
            to: Vec2::new(c.x + hw, c.y),
        },
        Vec2::new(c.x - hw, c.y),
        style::DIM,
    );
    axis.stroke.width = 2.0;
    axis.tags.push(id.clone());
    s.add(axis);

    let span = to - from;
    let mut v = from;
    let mut i = 0;
    // guard against float drift producing a runaway loop
    while v <= to + 1e-4 && i < 1000 {
        let x = c.x - hw + (v - from) / span * (2.0 * hw);
        let mut tick = Entity::new(
            format!("{id}.t{i}"),
            Shape::Line {
                to: Vec2::new(x, c.y + 8.0),
            },
            Vec2::new(x, c.y - 8.0),
            style::DIM,
        );
        tick.stroke.width = 2.0;
        tick.tags.push(id.clone());
        s.add(tick);

        let mut lbl = Entity::new(
            format!("{id}.l{i}"),
            Shape::Text {
                content: fmt_num(v),
                size: 18.0,
            },
            Vec2::new(x, c.y + 30.0),
            style::FG,
        );
        lbl.font = FontKind::Mono;
        lbl.tags.push(id.clone());
        s.add(lbl);

        v += step;
        i += 1;
    }
    Ok(())
}

/// `arc(id, (cx,cy), r, start, sweep)` — a plain circular arc line (degrees).
fn c_arc(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let r = a.num(2)?;
    let start = a.num(3)?;
    let sweep = a.num(4)?;
    let mut e = Entity::new(id, Shape::Arc { r, inner: 0.0, start, sweep }, c, style::CYAN);
    e.stroke = StrokeStyle {
        fill: false,
        outline: true,
        width: 3.0,
        outline_color: Some(style::CYAN),
    };
    s.add(e);
    Ok(())
}

fn neon_sector(id: String, c: Vec2, r: f32, inner: f32, start: f32, sweep: f32) -> Entity {
    let mut e = Entity::new(id, Shape::Arc { r, inner, start, sweep }, c, style::PANEL);
    e.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 2.5,
        outline_color: Some(style::CYAN),
    };
    e
}

/// `sector(id, (cx,cy), r, start, sweep)` — a filled pie slice (degrees).
fn c_sector(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let r = a.num(2)?;
    let start = a.num(3)?;
    let sweep = a.num(4)?;
    s.add(neon_sector(id, c, r, 0.0, start, sweep));
    Ok(())
}

/// `annulus(id, (cx,cy), outer, inner)` — a full ring.
fn c_annulus(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let outer = a.num(2)?;
    let inner = a.num(3)?;
    if inner >= outer {
        return Err(Error::new("annulus needs outer > inner", a.name_span));
    }
    s.add(neon_sector(id, c, outer, inner, 0.0, 360.0));
    Ok(())
}

/// `pie(id, (cx,cy), r, n)` — a circle cut into `n` equal filled sectors,
/// each addressable as `{id}0 … {id}{n-1}` (tag `id`). The one-liner behind
/// "cut a circle equally".
fn c_pie(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let r = a.num(2)?;
    let n = a.num(3)? as i64;
    if n < 1 || n > 360 {
        return Err(Error::new("pie needs 1..=360 slices", a.span_of(3)));
    }
    let step = 360.0 / n as f32;
    for k in 0..n {
        let mut e = neon_sector(format!("{id}{k}"), c, r, 0.0, k as f32 * step, step);
        e.tags.push(id.clone());
        s.add(e);
    }
    Ok(())
}

/// Register the math kit into `r`.
pub fn register(r: &mut Registry) {
    r.ctor("axes", c_axes);
    r.ctor("plot", c_plot);
    r.ctor("vector", c_vector);
    r.ctor("numberline", c_numberline);
    r.ctor("arc", c_arc);
    r.ctor("sector", c_sector);
    r.ctor("annulus", c_annulus);
    r.ctor("pie", c_pie);
}
