//! The 3D kit: Z-up primitives, an orbit camera, and deterministic 3D verbs.

use macroquad::prelude::{vec2, vec3, Quat, Vec2, Vec3};
use std::collections::BTreeSet;

use crate::easing::Easing;
use crate::lang::ast::ExprKind;
use crate::lang::diag::Error;
use crate::lang::lower::{resolve_color, resolve_easing, Args, Registry};
use crate::movie::CAMERA3_ID;
use crate::primitives::{Entity, Shape};
use crate::primitives3d::{
    Entity3D, Link3, Material3, Morph3, Morph3Kind, Projection3D, ProjectionPlane3, Shading3,
    Shape3D, SurfaceFn, Texture3,
};
use crate::scene::{Pin3, Pin3Target, Scene};
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TimelineEvent, TrackSpec, Value};

fn track(id: &str, prop: Prop, target: TargetValue, dur: f32, easing: Easing) -> TrackSpec {
    TrackSpec {
        id: id.into(),
        prop,
        target,
        start: 0.0,
        dur,
        easing,
    }
}

fn timing(a: &Args, at: usize, default: f32) -> Result<(f32, Easing), Error> {
    let dur = a.opt_num(at)?.unwrap_or(default);
    let easing = if a.len() > at + 1 {
        resolve_easing(&a.ident(at + 1)?, a.span_of(at + 1))?
    } else {
        Easing::InOutCubic
    };
    Ok((dur, easing))
}

fn add(s: &mut Scene, id: String, shape: Shape3D, pos: Vec3) {
    s.add_3d(Entity3D::new(id, shape, pos, style::CYAN));
}

/// `camera3((eye), (target), [fov], [perspective|orthographic])`.
fn c_camera(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let eye = a.triple(0)?;
    let target = a.triple(1)?;
    let fov = a.opt_num(2)?.unwrap_or(45.0);
    let projection = if a.len() > 3 {
        match a.ident(3)?.as_str() {
            "perspective" | "persp" => Projection3D::Perspective,
            "orthographic" | "ortho" => Projection3D::Orthographic,
            other => {
                return Err(Error::new(
                    format!("unknown 3D projection `{other}` (try: perspective, orthographic)"),
                    a.span_of(3),
                ))
            }
        }
    } else {
        Projection3D::Perspective
    };
    let d = eye - target;
    let radius = d.length().max(0.01);
    let flat = d.x.hypot(d.y);
    // Azimuth is undefined at an exact pole. Pick the convention whose
    // analytical orbit frame keeps +Y screen-up for a directly overhead (or
    // underside) camera. This makes a later pole-crossing orbit start from a
    // deterministic orientation instead of depending on atan2(0, 0).
    let azimuth = if flat <= 1e-6 {
        if d.z >= 0.0 {
            -90.0
        } else {
            90.0
        }
    } else {
        d.y.atan2(d.x).to_degrees()
    };
    let rotation = vec3(azimuth, d.z.atan2(flat).to_degrees(), 0.0);
    if let Some(cam) = s.get_3d_mut(CAMERA3_ID) {
        cam.pos = target;
        cam.rotation = rotation;
        cam.scale = radius;
        cam.shape = Shape3D::Camera { fov, projection };
    } else {
        let mut cam = Entity3D::new(
            CAMERA3_ID,
            Shape3D::Camera { fov, projection },
            target,
            style::VOID,
        );
        cam.rotation = rotation;
        cam.scale = radius;
        cam.opacity = 0.0;
        s.add_3d(cam);
    }
    Ok(())
}

fn c_point(s: &mut Scene, a: &Args) -> Result<(), Error> {
    add(
        s,
        a.ident(0)?,
        Shape3D::Point {
            radius: a.opt_num(2)?.unwrap_or(0.08),
        },
        a.triple(1)?,
    );
    Ok(())
}

fn c_line(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let from = a.triple(1)?;
    add(
        s,
        a.ident(0)?,
        Shape3D::Line {
            to: a.triple(2)? - from,
        },
        from,
    );
    Ok(())
}

fn c_arrow(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let from = a.triple(1)?;
    add(
        s,
        a.ident(0)?,
        Shape3D::Arrow {
            to: a.triple(2)? - from,
        },
        from,
    );
    Ok(())
}

fn c_cube(s: &mut Scene, a: &Args) -> Result<(), Error> {
    add(
        s,
        a.ident(0)?,
        Shape3D::Cube { size: a.triple(2)? },
        a.triple(1)?,
    );
    Ok(())
}

fn c_sphere(s: &mut Scene, a: &Args) -> Result<(), Error> {
    add(
        s,
        a.ident(0)?,
        Shape3D::Sphere { radius: a.num(2)? },
        a.triple(1)?,
    );
    Ok(())
}

fn c_grid(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let half = a.num(2)? as i32;
    if half < 1 {
        return Err(Error::new(
            "`grid3` half-count must be at least 1",
            a.span_of(2),
        ));
    }
    add(
        s,
        a.ident(0)?,
        Shape3D::Grid {
            half,
            spacing: a.opt_num(3)?.unwrap_or(1.0),
        },
        a.triple(1)?,
    );
    Ok(())
}

/// `axes3(id, (origin), length, [step])` — three colored arrows, plus tick
/// marks and numeric labels every `step` units (default 1; `step <= 0` = plain
/// arrows). The numbers are 2D `text` entities auto-`pin3`ed to each tick, so
/// they stay readable as the camera orbits.
fn c_axes(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let origin = a.triple(1)?;
    let len = a.num(2)?;
    let step = a.opt_num(3)?.unwrap_or(1.0);
    let tick = (len * 0.04).clamp(0.05, 0.25); // tick half-length
                                               // Fan each axis's tick numbers off the axis line in a distinct screen
                                               // direction (px, y-down) so short axes sharing an origin don't stack their
                                               // labels on top of each other: x below, y above, z to the right.
    for (suffix, dir, tickdir, color, lbl_off) in [
        (
            "x",
            vec3(1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            style::CYAN,
            vec2(0.0, 20.0),
        ),
        (
            "y",
            vec3(0.0, 1.0, 0.0),
            vec3(1.0, 0.0, 0.0),
            style::MAGENTA,
            vec2(0.0, -20.0),
        ),
        (
            "z",
            vec3(0.0, 0.0, 1.0),
            vec3(1.0, 0.0, 0.0),
            style::LIME,
            vec2(22.0, 0.0),
        ),
    ] {
        let mut arrow = Entity3D::new(
            format!("{id}.{suffix}"),
            Shape3D::Arrow { to: dir * len },
            origin,
            color,
        );
        arrow.tags.push(id.clone());
        s.add_3d(arrow);
        if step <= 0.0 {
            continue;
        }
        let (mut v, mut n) = (step, 1);
        while v <= len + 1e-3 {
            let at = origin + dir * v;
            // tick mark: a short 3D segment centred on `at`
            let mut mark = Entity3D::new(
                format!("{id}.tick.{suffix}.{n}"),
                Shape3D::Line {
                    to: tickdir * (tick * 2.0),
                },
                at - tickdir * tick,
                color,
            );
            mark.tags.push(id.clone());
            s.add_3d(mark);
            // numeric label: a 2D text pinned to the tick's 3D position
            let label = format!("{id}.num.{suffix}.{n}");
            let mut num = Entity::new(
                label.clone(),
                Shape::Text {
                    content: trim_num(v),
                    size: 20.0,
                },
                Vec2::ZERO,
                color,
            );
            num.tags.push(id.clone());
            s.add(num);
            s.pins.push(Pin3 {
                label,
                target: Pin3Target::Point(at),
                offset: lbl_off,
                declutter: true,
                world_height: None,
            });
            v += step;
            n += 1;
        }
    }
    Ok(())
}

/// Format a tick value without a trailing `.0` (2.0 → "2", 2.5 → "2.5").
fn trim_num(v: f32) -> String {
    if (v - v.round()).abs() < 1e-3 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// `pin3(label, (x,y,z) | entity3)` — glue an existing 2D `text`/`label` to a 3D
/// point, or to a 3D entity's current position. Reprojected every frame at
/// render time so the label tracks the point as the camera orbits.
fn c_pin(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let label = a.ident(0)?;
    let target = match a.triple(1) {
        Ok(p) => Pin3Target::Point(p),
        Err(_) => Pin3Target::Entity(a.ident(1)?),
    };
    s.pins.push(Pin3 {
        label,
        target,
        offset: Vec2::ZERO,
        declutter: false,
        world_height: None,
    });
    Ok(())
}

/// `curve3(id, "x(t)", "y(t)", "z(t)", [(t0,t1)])` — a parametric 3D curve
/// sampled from three formulas of the parameter `t` (default range `0..2π`),
/// drawn as a glowing polyline. Reuses the `plot` expression engine.
fn c_curve3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    use crate::kits::math::expr;
    let id = a.ident(0)?;
    let compile = |i: usize| -> Result<expr::Node, Error> {
        let src = a.text(i)?;
        expr::compile(&src).map_err(|m| Error::new(format!("in curve3 formula: {m}"), a.span_of(i)))
    };
    let (fx, fy, fz) = (compile(1)?, compile(2)?, compile(3)?);
    let (t0, t1) = match a.pair(4) {
        Ok(p) => (p.x, p.y),
        Err(_) => (0.0, std::f32::consts::TAU),
    };
    const N: usize = 240;
    let mut points = Vec::with_capacity(N + 1);
    for i in 0..=N {
        let t = t0 + (t1 - t0) * i as f32 / N as f32;
        let p = vec3(fx.eval(t, 0.0), fy.eval(t, 0.0), fz.eval(t, 0.0));
        if p.is_finite() {
            points.push(p);
        }
    }
    s.add_3d(Entity3D::new(
        id,
        Shape3D::Path { points },
        Vec3::ZERO,
        style::CYAN,
    ));
    Ok(())
}

/// `revolve3(id, (cx,cy,cz), "r(t)", (t0,t1), [sides])` — a solid of revolution:
/// the radius profile `r(t)` over height `t ∈ [t0,t1]` is swept around the
/// vertical axis into a wireframe mesh (default 32 angular segments). Vases,
/// spheres (`sqrt(1-t*t)`), cones (`t`), hyperboloids, … centred on its position.
fn c_revolve3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    use crate::kits::math::expr;
    const AXIAL: usize = 48;
    let id = a.ident(0)?;
    let center = a.triple(1)?;
    let src = a.text(2)?;
    let f = expr::compile(&src)
        .map_err(|m| Error::new(format!("in revolve3 profile: {m}"), a.span_of(2)))?;
    let (t0, t1) = {
        let p = a.pair(3)?;
        (p.x, p.y)
    };
    let sides = match a.opt_num(4)? {
        Some(n) => (n.round() as i64).clamp(3, 256) as usize,
        None => 32,
    };
    let mid = (t0 + t1) / 2.0; // centre the solid vertically on its position
    let nu = sides + 1; // angular (last col == first, closing the ring)
    let nv = AXIAL + 1; // axial
    let mut pts = Vec::with_capacity(nu * nv);
    for j in 0..=AXIAL {
        let t = t0 + (t1 - t0) * j as f32 / AXIAL as f32;
        let r = f.eval(t, 0.0);
        let r = if r.is_finite() { r } else { 0.0 };
        let z = t - mid;
        for aa in 0..=sides {
            let th = std::f32::consts::TAU * aa as f32 / sides as f32;
            pts.push(vec3(r * th.cos(), r * th.sin(), z));
        }
    }
    s.add_3d(Entity3D::new(
        id,
        Shape3D::Surface {
            pts,
            nu: nu as u32,
            nv: nv as u32,
        },
        center,
        style::CYAN,
    ));
    Ok(())
}

/// A regular n-gon ring of `sides` vertices at height `z`, radius `radius`,
/// centred on the local origin (XY plane).
fn ngon_ring(sides: usize, radius: f32, z: f32) -> Vec<Vec3> {
    (0..sides)
        .map(|k| {
            let a = std::f32::consts::TAU * k as f32 / sides as f32;
            vec3(radius * a.cos(), radius * a.sin(), z)
        })
        .collect()
}

/// Read + validate the `sides` argument (integer ≥ 3, capped for sanity).
fn read_sides(a: &Args, i: usize) -> Result<usize, Error> {
    let n = a.num(i)?.round() as i64;
    if n < 3 {
        return Err(Error::new("needs at least 3 sides", a.span_of(i)));
    }
    Ok((n as usize).min(256))
}

/// `prism3(id, (cx,cy,cz), sides, radius, height)` — a regular n-gon prism
/// (sides ≥ 3; use many sides for a cylinder), centred on its position.
fn c_prism3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = a.triple(1)?;
    let sides = read_sides(a, 2)?;
    let radius = a.num(3)?;
    let h = a.num(4)? / 2.0;
    let mut verts = ngon_ring(sides, radius, -h);
    verts.extend(ngon_ring(sides, radius, h));
    let n = sides as u32;
    let mut edges = Vec::with_capacity(3 * sides);
    let mut faces = Vec::with_capacity(4 * sides);
    for k in 0..n {
        let k1 = (k + 1) % n;
        edges.push((k, k1)); // base ring
        edges.push((n + k, n + k1)); // top ring
        edges.push((k, n + k)); // vertical
                                // side quad (base_k, base_k1, top_k1, top_k) as two triangles
        faces.push([k, k1, n + k1]);
        faces.push([k, n + k1, n + k]);
    }
    for k in 1..n - 1 {
        faces.push([0, k, k + 1]); // bottom cap (fan)
        faces.push([n, n + k, n + k + 1]); // top cap (fan)
    }
    s.add_3d(Entity3D::new(
        id,
        Shape3D::Mesh {
            verts,
            edges,
            faces,
        },
        center,
        style::CYAN,
    ));
    Ok(())
}

/// `pyramid3(id, (cx,cy,cz), sides, radius, height)` — a regular n-gon pyramid
/// (many sides ≈ a cone), centred on its position.
fn c_pyramid3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = a.triple(1)?;
    let sides = read_sides(a, 2)?;
    let radius = a.num(3)?;
    let h = a.num(4)? / 2.0;
    let mut verts = ngon_ring(sides, radius, -h);
    verts.push(vec3(0.0, 0.0, h)); // apex
    let n = sides as u32;
    let apex = n;
    let mut edges = Vec::with_capacity(2 * sides);
    let mut faces = Vec::with_capacity(2 * sides);
    for k in 0..n {
        let k1 = (k + 1) % n;
        edges.push((k, k1)); // base ring
        edges.push((k, apex)); // slant
        faces.push([k, k1, apex]); // side triangle
    }
    for k in 1..n - 1 {
        faces.push([0, k, k + 1]); // bottom cap (fan)
    }
    s.add_3d(Entity3D::new(
        id,
        Shape3D::Mesh {
            verts,
            edges,
            faces,
        },
        center,
        style::LIME,
    ));
    Ok(())
}

/// `surface3(id, "z(x,y)", (x0,x1), (y0,y1), [res])` — a height-field surface
/// `z = f(x,y)` sampled over the x/y rectangle into a `(res+1)²` wireframe mesh.
fn c_surface3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    use crate::kits::math::expr;
    let id = a.ident(0)?;
    let src = a.text(1)?;
    let f = expr::compile(&src)
        .map_err(|m| Error::new(format!("in surface3 formula: {m}"), a.span_of(1)))?;
    let (x0, x1) = {
        let p = a.pair(2)?;
        (p.x, p.y)
    };
    let (y0, y1) = {
        let p = a.pair(3)?;
        (p.x, p.y)
    };
    let res = (a.opt_num(4)?.unwrap_or(20.0) as usize).clamp(2, 120);
    let n = res + 1; // points per side
    let mut pts = Vec::with_capacity(n * n);
    for j in 0..n {
        let y = y0 + (y1 - y0) * j as f32 / res as f32;
        for i in 0..n {
            let x = x0 + (x1 - x0) * i as f32 / res as f32;
            let z = f.eval(x, y);
            pts.push(vec3(x, y, if z.is_finite() { z } else { 0.0 }));
        }
    }
    let mut e = Entity3D::new(
        id,
        Shape3D::Surface {
            pts,
            nu: n as u32,
            nv: n as u32,
        },
        Vec3::ZERO,
        style::CYAN,
    );
    // remember z(x,y) + domain so gradient3/tangentplane3/volume3 can query it
    e.surf = Some(SurfaceFn { f, x0, x1, y0, y1 });
    s.add_3d(e);
    Ok(())
}

/// `heightmap3(id, grid, "z(x,y,h)", [size])` — bridge a 2-D grid-kit grid into a
/// 3-D terrain mesh. Reads the grid's per-cell state (`h` = 1 for a filled/`wall`
/// or alive cell, else 0 — using the latest CA/WFC frame if the grid has run one,
/// else its base cells) and evaluates the height formula per cell over `x`/`y`
/// (the cell's position across a `size`-wide field, default 6) plus `h`. So
/// `"h*1.5"` raises the walls, `"sin(x*2)*cos(y*2)*0.4 + h"` ripples them. The
/// bridge lives entirely on the 3-D side — the grid kit needs no 3-D awareness.
fn c_heightmap3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    use crate::kits::grid::CellKind;
    use crate::kits::math::expr;
    let id = a.ident(0)?;
    let grid_id = a.ident(1)?;
    let src = a.text(2)?;
    let f = expr::compile(&src)
        .map_err(|m| Error::new(format!("in heightmap3 formula: {m}"), a.span_of(2)))?;
    let size = a.opt_num(3)?.unwrap_or(6.0).max(0.1);
    let g = s.grids.get(&grid_id).cloned().ok_or_else(|| {
        Error::new(
            format!("`{grid_id}` is not a grid — call `grid({grid_id}, ...)` first"),
            a.span_of(1),
        )
    })?;
    let (cols, rows) = (g.cols, g.rows);
    // The cell state to lift: the settled CA/WFC frame if one exists, else the base cells.
    let state = g.frames.last().unwrap_or(&g.kinds);
    let span = |n: usize, k: usize| {
        if n <= 1 {
            0.0
        } else {
            (k as f32 / (n - 1) as f32 - 0.5) * size
        }
    };
    let mut pts = Vec::with_capacity(cols * rows);
    for r in 0..rows {
        for c in 0..cols {
            let hv = if state[r * cols + c] == CellKind::Wall {
                1.0
            } else {
                0.0
            };
            let (x, y) = (span(cols, c), span(rows, r));
            let z = f.eval3(x, y, hv);
            pts.push(vec3(x, y, if z.is_finite() { z } else { 0.0 }));
        }
    }
    let e = Entity3D::new(
        id,
        Shape3D::Surface {
            pts,
            nu: cols as u32,
            nv: rows as u32,
        },
        Vec3::ZERO,
        style::CYAN,
    );
    s.add_3d(e);
    Ok(())
}

/// Fetch a `surface3`'s remembered height field by id, or a clear error.
fn fetch_surf(s: &Scene, a: &Args, idx: usize) -> Result<SurfaceFn, Error> {
    let src = a.ident(idx)?;
    s.get_3d(&src).and_then(|e| e.surf.clone()).ok_or_else(|| {
        Error::new(
            format!(
                "expected a `surface3` as argument {}, but `{src}` isn't one (draw it with `surface3` first)",
                idx + 1
            ),
            a.span_of(idx),
        )
    })
}

/// `gradient3(id, surface, x, y, [color])` — an arrow on a plotted `surface3` at
/// `(x,y)` pointing in the direction of steepest ascent (the gradient ∇f), its
/// length growing with the slope. The uphill tangent `(∂f/∂x, ∂f/∂y, |∇f|²)`.
fn c_gradient3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let surf = fetch_surf(s, a, 1)?;
    let (x, y) = (a.num(2)?, a.num(3)?);
    let color = if a.len() > 4 {
        resolve_color(&a.ident(4)?, a.span_of(4))?
    } else {
        style::GOLD
    };
    let (fx, fy) = (surf.dx(x, y), surf.dy(x, y));
    let g = (fx * fx + fy * fy).sqrt();
    let mut dir = vec3(fx, fy, fx * fx + fy * fy);
    let len = dir.length();
    dir = if len > 1e-6 {
        dir / len * g.clamp(0.3, 2.5)
    } else {
        Vec3::ZERO
    };
    let mut e = Entity3D::new(
        id.clone(),
        Shape3D::Arrow { to: dir },
        surf.point(x, y),
        color,
    );
    e.tags.push(id);
    s.add_3d(e);
    Ok(())
}

/// `tangentplane3(id, surface, x, y, [color])` — the plane tangent to a plotted
/// `surface3` at `(x,y)`: `z = f(a) + fx·(u−x) + fy·(v−y)`, a small translucent
/// patch. The 2-D analog of the tangent line.
fn c_tangentplane3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let surf = fetch_surf(s, a, 1)?;
    let (x, y) = (a.num(2)?, a.num(3)?);
    let color = if a.len() > 4 {
        resolve_color(&a.ident(4)?, a.span_of(4))?
    } else {
        style::MAGENTA
    };
    let (fx, fy, z0) = (surf.dx(x, y), surf.dy(x, y), surf.z(x, y));
    let w = (surf.x1 - surf.x0).abs().min((surf.y1 - surf.y0).abs()) * 0.075;
    let res = 6usize;
    let m = res + 1;
    let mut pts = Vec::with_capacity(m * m);
    for j in 0..m {
        let v = y - w + 2.0 * w * j as f32 / res as f32;
        for i in 0..m {
            let u = x - w + 2.0 * w * i as f32 / res as f32;
            pts.push(vec3(u, v, z0 + fx * (u - x) + fy * (v - y)));
        }
    }
    let mut e = Entity3D::new(
        id.clone(),
        Shape3D::Surface {
            pts,
            nu: m as u32,
            nv: m as u32,
        },
        Vec3::ZERO,
        color,
    );
    e.opacity = 0.42;
    e.tags.push(id);
    s.add_3d(e);
    Ok(())
}

/// `volume3(id, surface, [res], [color])` — the volume under a plotted `surface3`
/// as a 3-D Riemann sum: a `res×res` grid of translucent columns from `z=0` up
/// to the surface (double integral, made solid). Columns `{id}0…` tagged `id`.
fn c_volume3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let surf = fetch_surf(s, a, 1)?;
    let res = (a.opt_num(2)?.unwrap_or(7.0) as usize).clamp(2, 24);
    let color = if a.len() > 3 {
        resolve_color(&a.ident(3)?, a.span_of(3))?
    } else {
        style::CYAN
    };
    let dx = (surf.x1 - surf.x0) / res as f32;
    let dy = (surf.y1 - surf.y0) / res as f32;
    let mut k = 0;
    for j in 0..res {
        for i in 0..res {
            let cx = surf.x0 + (i as f32 + 0.5) * dx;
            let cy = surf.y0 + (j as f32 + 0.5) * dy;
            let h = surf.z(cx, cy);
            if !h.is_finite() {
                continue;
            }
            let mut e = Entity3D::new(
                format!("{id}{k}"),
                Shape3D::Cube {
                    size: vec3(dx * 0.9, dy * 0.9, h.abs().max(1e-3)),
                },
                vec3(cx, cy, h * 0.5),
                color,
            );
            e.opacity = 0.5;
            e.tags.push(id.clone());
            s.add_3d(e);
            k += 1;
        }
    }
    Ok(())
}

/// `param3(id, "x(u,v)", "y(u,v)", "z(u,v)", (u0,u1), (v0,v1), [res])` — a general
/// parametric surface: three formulas of the parameters `u`,`v` sampled over the
/// rectangle into a `(res+1)²` filled, flat-shaded mesh (default `res` 24).
/// Unlike `surface3` (a height field `z=f(x,y)`) this can wrap and close — tori,
/// shells, Möbius strips, parametric spheres.
fn c_param3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    use crate::kits::math::expr;
    let id = a.ident(0)?;
    let compile = |i: usize| -> Result<expr::Node, Error> {
        let src = a.text(i)?;
        expr::compile(&src).map_err(|m| Error::new(format!("in param3 formula: {m}"), a.span_of(i)))
    };
    let (fx, fy, fz) = (compile(1)?, compile(2)?, compile(3)?);
    let (u0, u1) = {
        let p = a.pair(4)?;
        (p.x, p.y)
    };
    let (v0, v1) = {
        let p = a.pair(5)?;
        (p.x, p.y)
    };
    let res = (a.opt_num(6)?.unwrap_or(24.0) as usize).clamp(2, 120);
    let n = res + 1; // points per side
    let mut pts = Vec::with_capacity(n * n);
    for j in 0..n {
        let v = v0 + (v1 - v0) * j as f32 / res as f32;
        for i in 0..n {
            let u = u0 + (u1 - u0) * i as f32 / res as f32;
            let p = vec3(fx.eval(u, v), fy.eval(u, v), fz.eval(u, v));
            pts.push(if p.is_finite() { p } else { Vec3::ZERO });
        }
    }
    s.add_3d(Entity3D::new(
        id,
        Shape3D::Surface {
            pts,
            nu: n as u32,
            nv: n as u32,
        },
        Vec3::ZERO,
        style::CYAN,
    ));
    Ok(())
}

/// `follow3(id, target, [(dx,dy,dz)])` — glue a 3D entity to another's position
/// (plus an optional offset), recomputed every frame as the target moves.
fn c_follow3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let target = a.ident(1)?;
    if s.get_3d(&target).is_none() {
        return Err(Error::new(
            format!("no 3D entity named `{target}`"),
            a.span_of(1),
        ));
    }
    let offset = a.triple(2).unwrap_or(Vec3::ZERO);
    let e = s
        .get_3d_mut(&id)
        .ok_or_else(|| Error::new(format!("no 3D entity named `{id}`"), a.span_of(0)))?;
    e.follow = Some((target, offset));
    Ok(())
}

/// `midpoint3(id, a, b)` — a point at the midpoint of two 3D entities,
/// recomputed each frame as they move.
fn c_midpoint3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let p = a.ident(1)?;
    let q = a.ident(2)?;
    let seed = match (s.get_3d(&p), s.get_3d(&q)) {
        (Some(ea), Some(eb)) => (ea.pos + eb.pos) * 0.5,
        _ => Vec3::ZERO,
    };
    let mut e = Entity3D::new(id, Shape3D::Point { radius: 0.12 }, seed, style::FG);
    e.deps = vec![p, q];
    e.derive = Some(|e, pts| {
        if pts.len() == 2 {
            e.pos = (pts[0] + pts[1]) * 0.5;
        }
    });
    s.add_3d(e);
    Ok(())
}

/// `link3(id,a,b,[trim])` — a live edge recomputed from its endpoints.
fn c_link3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    a.max(4)?;
    let id = a.ident(0)?;
    let from = a.ident(1)?;
    let to = a.ident(2)?;
    let trim = a.opt_num(3)?.unwrap_or(0.0).max(0.0);
    let pa = s
        .get_3d(&from)
        .ok_or_else(|| Error::new(format!("no 3D entity named `{from}`"), a.span_of(1)))?
        .pos;
    let pb = s
        .get_3d(&to)
        .ok_or_else(|| Error::new(format!("no 3D entity named `{to}`"), a.span_of(2)))?
        .pos;
    let mut entity = Entity3D::new(id, Shape3D::Line { to: pb - pa }, pa, style::FG);
    entity.link = Some(Link3 { from, to, trim });
    s.add_3d(entity);
    Ok(())
}

/// `project3(id,source,"xy|xz|yz")` — live orthogonal projection point.
fn c_project3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    a.max(3)?;
    let id = a.ident(0)?;
    let source = a.ident(1)?;
    let source_pos = s
        .get_3d(&source)
        .ok_or_else(|| Error::new(format!("no 3D entity named `{source}`"), a.span_of(1)))?
        .pos;
    let plane = match a.text(2)?.as_str() {
        "xy" => ProjectionPlane3::Xy,
        "xz" => ProjectionPlane3::Xz,
        "yz" => ProjectionPlane3::Yz,
        _ => {
            return Err(Error::new(
                "project3 plane is \"xy\", \"xz\", or \"yz\"",
                a.span_of(2),
            ))
        }
    };
    let pos = match plane {
        ProjectionPlane3::Xy => vec3(source_pos.x, source_pos.y, 0.0),
        ProjectionPlane3::Xz => vec3(source_pos.x, 0.0, source_pos.z),
        ProjectionPlane3::Yz => vec3(0.0, source_pos.y, source_pos.z),
    };
    let mut entity = Entity3D::new(id, Shape3D::Point { radius: 0.1 }, pos, style::CYAN);
    entity.projection = Some((source, plane));
    s.add_3d(entity);
    Ok(())
}

fn contour_edge(a: Vec3, av: f32, b: Vec3, bv: f32) -> Option<Vec3> {
    if (av < 0.0) == (bv < 0.0) || (av - bv).abs() < 1e-8 {
        return None;
    }
    Some(a.lerp(b, av / (av - bv)))
}

/// `contour3(id,surface,level)` — level curve generated from a height field.
fn c_contour3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    a.max(3)?;
    let id = a.ident(0)?;
    let surface_id = a.ident(1)?;
    let level = a.num(2)?;
    let surface = s
        .authored_world_entity_3d(&surface_id)
        .ok_or_else(|| Error::new(format!("no 3D surface named `{surface_id}`"), a.span_of(1)))?;
    let sf = surface.surf.clone().ok_or_else(|| {
        Error::new(
            format!("`{surface_id}` is not a surface3 height field"),
            a.span_of(1),
        )
    })?;
    const N: usize = 48;
    let mut verts = Vec::new();
    let mut edges = Vec::new();
    for j in 0..N {
        let y0 = sf.y0 + (sf.y1 - sf.y0) * j as f32 / N as f32;
        let y1 = sf.y0 + (sf.y1 - sf.y0) * (j + 1) as f32 / N as f32;
        for i in 0..N {
            let x0 = sf.x0 + (sf.x1 - sf.x0) * i as f32 / N as f32;
            let x1 = sf.x0 + (sf.x1 - sf.x0) * (i + 1) as f32 / N as f32;
            let p = [
                sf.point(x0, y0),
                sf.point(x1, y0),
                sf.point(x1, y1),
                sf.point(x0, y1),
            ];
            let v = [
                p[0].z - level,
                p[1].z - level,
                p[2].z - level,
                p[3].z - level,
            ];
            let mut hits = Vec::new();
            for (ea, eb) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
                if let Some(hit) = contour_edge(p[ea], v[ea], p[eb], v[eb]) {
                    hits.push(surface.world_point(hit));
                }
            }
            for pair in hits.chunks_exact(2) {
                let index = verts.len() as u32;
                verts.extend_from_slice(pair);
                edges.push((index, index + 1));
            }
        }
    }
    s.add_3d(Entity3D::new(
        id,
        Shape3D::Mesh {
            verts,
            edges,
            faces: Vec::new(),
        },
        Vec3::ZERO,
        style::GOLD,
    ));
    Ok(())
}

/// `label3(label,target,[world_height])` — pin3 plus optional depth scaling.
fn c_label3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    a.max(3)?;
    let label = a.ident(0)?;
    if s.get(&label).is_none() {
        return Err(Error::new(
            format!("no 2D label named `{label}`"),
            a.span_of(0),
        ));
    }
    let target = match a.triple(1) {
        Ok(point) => Pin3Target::Point(point),
        Err(_) => Pin3Target::Entity(a.ident(1)?),
    };
    let world_height = a.opt_num(2)?.map(|height| height.max(0.001));
    s.pins.push(Pin3 {
        label,
        target,
        offset: Vec2::ZERO,
        declutter: false,
        world_height,
    });
    Ok(())
}

fn obj_index(token: &str, len: usize) -> Option<usize> {
    let raw = token.split('/').next()?.parse::<isize>().ok()?;
    let index = if raw < 0 { len as isize + raw } else { raw - 1 };
    (index >= 0 && (index as usize) < len).then_some(index as usize)
}

/// `model3(id,"asset:models/name.obj"|"file.obj",center,[scale])` —
/// deterministic geometry-only OBJ from the bundled catalog or a supplied path.
fn c_model3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    a.max(4)?;
    let id = a.ident(0)?;
    let path = a.text(1)?;
    let source_path =
        crate::assets::resolve(&path).map_err(|message| Error::new(message, a.span_of(1)))?;
    if source_path
        .extension()
        .and_then(|extension| extension.to_str())
        != Some("obj")
    {
        return Err(Error::new(
            "model3 currently accepts only .obj geometry",
            a.span_of(1),
        ));
    }
    let metadata = std::fs::metadata(&source_path).map_err(|error| {
        Error::new(
            format!("cannot read model3 `{path}`: {error}"),
            a.span_of(1),
        )
    })?;
    if metadata.len() > 16 * 1024 * 1024 {
        return Err(Error::new(
            "model3 OBJ is larger than the 16 MB safety limit",
            a.span_of(1),
        ));
    }
    let source = std::fs::read_to_string(&source_path).map_err(|error| {
        Error::new(
            format!("cannot read model3 `{path}`: {error}"),
            a.span_of(1),
        )
    })?;
    let mut verts = Vec::new();
    let mut faces = Vec::new();
    let mut edge_set = BTreeSet::new();
    for (line_number, line) in source.lines().enumerate() {
        let mut fields = line.split_whitespace();
        match fields.next() {
            Some("v") => {
                let values: Vec<_> = fields.take(3).collect();
                if values.len() != 3 {
                    return Err(Error::new(
                        format!("invalid OBJ vertex on line {}", line_number + 1),
                        a.span_of(1),
                    ));
                }
                let parse =
                    |value: &str| value.parse::<f32>().ok().filter(|value| value.is_finite());
                let Some((x, y, z)) = parse(values[0])
                    .zip(parse(values[1]))
                    .zip(parse(values[2]))
                    .map(|((x, y), z)| (x, y, z))
                else {
                    return Err(Error::new(
                        format!("invalid OBJ number on line {}", line_number + 1),
                        a.span_of(1),
                    ));
                };
                verts.push(vec3(x, y, z));
            }
            Some("f") => {
                let polygon: Vec<_> = fields
                    .filter_map(|field| obj_index(field, verts.len()))
                    .collect();
                if polygon.len() < 3 {
                    return Err(Error::new(
                        format!("invalid OBJ face on line {}", line_number + 1),
                        a.span_of(1),
                    ));
                }
                for i in 1..polygon.len() - 1 {
                    faces.push([polygon[0] as u32, polygon[i] as u32, polygon[i + 1] as u32]);
                }
                for i in 0..polygon.len() {
                    let pair = (polygon[i] as u32, polygon[(i + 1) % polygon.len()] as u32);
                    edge_set.insert(if pair.0 < pair.1 {
                        pair
                    } else {
                        (pair.1, pair.0)
                    });
                }
            }
            Some("l") => {
                let line: Vec<_> = fields
                    .filter_map(|field| obj_index(field, verts.len()))
                    .collect();
                for pair in line.windows(2) {
                    let pair = (pair[0] as u32, pair[1] as u32);
                    edge_set.insert(if pair.0 < pair.1 {
                        pair
                    } else {
                        (pair.1, pair.0)
                    });
                }
            }
            _ => {}
        }
        if verts.len() > 500_000 || faces.len() > 1_000_000 {
            return Err(Error::new(
                "model3 OBJ exceeds the geometry safety limit",
                a.span_of(1),
            ));
        }
    }
    if verts.is_empty() {
        return Err(Error::new("model3 OBJ has no vertices", a.span_of(1)));
    }
    let mut entity = Entity3D::new(
        id,
        Shape3D::Mesh {
            verts,
            edges: edge_set.into_iter().collect(),
            faces,
        },
        a.triple(2)?,
        style::FG,
    );
    entity.scale = a.opt_num(3)?.unwrap_or(1.0);
    s.add_3d(entity);
    Ok(())
}

/// `tube3(id,path,"radius(t)",[sides])` — variable-radius tube along a 3D path.
fn c_tube3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    use crate::kits::math::expr;
    a.max(4)?;
    let id = a.ident(0)?;
    let path_id = a.ident(1)?;
    let profile = expr::compile(&a.text(2)?).map_err(|message| {
        Error::new(format!("in tube3 radius profile: {message}"), a.span_of(2))
    })?;
    let sides = (a.opt_num(3)?.unwrap_or(12.0).round() as usize).clamp(3, 64);
    let path = s
        .authored_world_entity_3d(&path_id)
        .ok_or_else(|| Error::new(format!("no 3D path named `{path_id}`"), a.span_of(1)))?;
    let points = path_points3(&path).ok_or_else(|| {
        Error::new(
            format!("`{path_id}` is not a line3, arrow3, or curve3"),
            a.span_of(1),
        )
    })?;
    if points.len() < 2 {
        return Err(Error::new(
            "tube3 path needs at least two points",
            a.span_of(1),
        ));
    }
    let tangents: Vec<Vec3> = (0..points.len())
        .map(|index| {
            let tangent = if index == 0 {
                points[1] - points[0]
            } else if index + 1 == points.len() {
                points[index] - points[index - 1]
            } else {
                points[index + 1] - points[index - 1]
            };
            tangent.normalize_or_zero()
        })
        .collect();
    let helper = if tangents[0].z.abs() < 0.9 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    let mut normal = tangents[0].cross(helper).normalize_or_zero();
    let mut verts = Vec::with_capacity(points.len() * sides);
    for (index, point) in points.iter().enumerate() {
        if index > 0 {
            let axis = tangents[index - 1].cross(tangents[index]);
            let axis_len = axis.length();
            if axis_len > 1e-6 {
                let angle = tangents[index - 1]
                    .dot(tangents[index])
                    .clamp(-1.0, 1.0)
                    .acos();
                normal =
                    (Quat::from_axis_angle(axis / axis_len, angle) * normal).normalize_or_zero();
            }
            normal = (normal - tangents[index] * normal.dot(tangents[index])).normalize_or_zero();
        }
        let binormal = tangents[index].cross(normal).normalize_or_zero();
        let t = index as f32 / (points.len() - 1) as f32;
        let radius = profile.eval(t, 0.0).abs().clamp(0.0001, 1000.0);
        for side in 0..sides {
            let angle = std::f32::consts::TAU * side as f32 / sides as f32;
            verts.push(*point + (normal * angle.cos() + binormal * angle.sin()) * radius);
        }
    }
    let mut faces = Vec::new();
    let mut edges = Vec::new();
    for ring in 0..points.len() - 1 {
        for side in 0..sides {
            let next = (side + 1) % sides;
            let a0 = (ring * sides + side) as u32;
            let a1 = (ring * sides + next) as u32;
            let b0 = ((ring + 1) * sides + side) as u32;
            let b1 = ((ring + 1) * sides + next) as u32;
            faces.extend([[a0, b0, b1], [a0, b1, a1]]);
            edges.extend([(a0, a1), (a0, b0)]);
        }
    }
    s.add_3d(Entity3D::new(
        id,
        Shape3D::Mesh {
            verts,
            edges,
            faces,
        },
        Vec3::ZERO,
        style::CYAN,
    ));
    Ok(())
}

/// `thick(id, radius)` — render a 3D path (`curve3`) as a solid tube of the
/// given world-space radius instead of a 1px line. `0` restores the thin line.
fn c_thick(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let w = a.num(1)?;
    let e = s
        .get_3d_mut(&id)
        .ok_or_else(|| Error::new(format!("no 3D entity named `{id}`"), a.span_of(0)))?;
    e.thickness = w.max(0.0);
    Ok(())
}

/// `finish3(id,"...")` — one bounded surface for the optional 3D render look.
fn c_finish3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    a.max(2)?;
    let id = a.ident(0)?;
    let spec = a.text(1)?;
    let e = s
        .get_3d_mut(&id)
        .ok_or_else(|| Error::new(format!("no 3D entity named `{id}`"), a.span_of(0)))?;
    for token in spec.split_whitespace() {
        let (key, value) = token.split_once('=').ok_or_else(|| {
            Error::new(
                format!("finish3 option `{token}` needs key=value"),
                a.span_of(1),
            )
        })?;
        let unit = |value: &str| -> Result<f32, Error> {
            value
                .parse::<f32>()
                .map(|v| v.clamp(0.0, 1.0))
                .map_err(|_| {
                    Error::new(
                        format!("finish3 `{key}` needs a number from 0 to 1"),
                        a.span_of(1),
                    )
                })
        };
        match key {
            "shading" => e.finish.shading = match value {
                "flat" => Shading3::Flat,
                "smooth" => Shading3::Smooth,
                _ => return Err(Error::new("finish3 shading is flat or smooth", a.span_of(1))),
            },
            "material" => e.finish.material = match value {
                "matte" => Material3::Matte,
                "metal" => Material3::Metal,
                "glass" => Material3::Glass,
                _ => return Err(Error::new("finish3 material is matte, metal, or glass", a.span_of(1))),
            },
            "texture" => e.finish.texture = match value {
                "solid" => Texture3::Solid,
                "checker" => Texture3::Checker,
                "stripes" => Texture3::Stripes,
                _ => return Err(Error::new("finish3 texture is solid, checker, or stripes", a.span_of(1))),
            },
            "scale" => {
                e.finish.texture_scale = value.parse::<f32>().map_err(|_| {
                    Error::new("finish3 scale needs a number", a.span_of(1))
                })?.clamp(0.25, 32.0)
            }
            "mesh" => e.finish.mesh = unit(value)?,
            "depth" => e.finish.depth = unit(value)?,
            "shadow" => e.finish.shadow = unit(value)?,
            _ => return Err(Error::new(
                format!("unknown finish3 option `{key}` — use shading, material, texture, scale, mesh, depth, or shadow"),
                a.span_of(1),
            )),
        }
    }
    Ok(())
}

// ------------------------------ morph3 ------------------------------

/// Resample a 3D polyline to exactly `n` points, evenly by arc length
/// (endpoints included).
fn resample3(pts: &[Vec3], n: usize) -> Vec<Vec3> {
    if pts.len() < 2 {
        return vec![pts.first().copied().unwrap_or(Vec3::ZERO); n];
    }
    let mut cum = vec![0.0f32];
    for w in pts.windows(2) {
        cum.push(cum.last().unwrap() + (w[1] - w[0]).length());
    }
    let total = *cum.last().unwrap();
    if total < 1e-6 {
        return vec![pts[0]; n];
    }
    (0..n)
        .map(|k| {
            let d = total * k as f32 / (n - 1).max(1) as f32;
            let mut i = 0;
            while i + 1 < cum.len() && cum[i + 1] < d {
                i += 1;
            }
            let seg = cum[i + 1] - cum[i];
            let t = if seg > 1e-6 { (d - cum[i]) / seg } else { 0.0 };
            pts[i] + (pts[i + 1] - pts[i]) * t
        })
        .collect()
}

/// Bilinearly resample a row-major `nu0`×`nv0` grid to `ru`×`rv` points.
fn resample_surface(pts: &[Vec3], nu0: u32, nv0: u32, ru: u32, rv: u32) -> Vec<Vec3> {
    let (nu0, nv0) = (nu0 as usize, nv0 as usize);
    if nu0 < 2 || nv0 < 2 || pts.len() != nu0 * nv0 {
        return vec![Vec3::ZERO; (ru * rv) as usize];
    }
    let mut out = Vec::with_capacity((ru * rv) as usize);
    for iv in 0..rv {
        let gv = iv as f32 / (rv - 1).max(1) as f32 * (nv0 - 1) as f32;
        let v0 = (gv.floor() as usize).min(nv0 - 1);
        let v1 = (v0 + 1).min(nv0 - 1);
        let tv = gv - v0 as f32;
        for iu in 0..ru {
            let gu = iu as f32 / (ru - 1).max(1) as f32 * (nu0 - 1) as f32;
            let u0 = (gu.floor() as usize).min(nu0 - 1);
            let u1 = (u0 + 1).min(nu0 - 1);
            let tu = gu - u0 as f32;
            let a = pts[v0 * nu0 + u0].lerp(pts[v0 * nu0 + u1], tu);
            let b = pts[v1 * nu0 + u0].lerp(pts[v1 * nu0 + u1], tu);
            out.push(a.lerp(b, tv));
        }
    }
    out
}

/// Möller–Trumbore ray/triangle intersection; returns the positive hit distance.
fn ray_tri(o: Vec3, d: Vec3, t: &[Vec3; 3]) -> Option<f32> {
    let (e1, e2) = (t[1] - t[0], t[2] - t[0]);
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-8 {
        return None;
    }
    let inv = 1.0 / det;
    let tv = o - t[0];
    let u = tv.dot(p) * inv;
    if u < -1e-4 || u > 1.0 + 1e-4 {
        return None;
    }
    let q = tv.cross(e1);
    let v = d.dot(q) * inv;
    if v < -1e-4 || u + v > 1.0 + 1e-4 {
        return None;
    }
    let tt = e2.dot(q) * inv;
    (tt > 1e-5).then_some(tt)
}

/// Reparameterise a star-shaped solid (its surface `tris`) onto a spherical
/// `(θ,φ)` grid: from the mesh's bbox centre, cast a ray in each direction and
/// keep the farthest surface hit. Any two solids sampled this way share a grid
/// topology, so a cube and a sphere morph smoothly into one another.
fn spherical_grid(tris: &[[Vec3; 3]], cols: usize, rows: usize) -> Vec<Vec3> {
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for t in tris {
        for p in t {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
    }
    let center = (lo + hi) * 0.5;
    let mut pts = Vec::with_capacity((cols + 1) * (rows + 1));
    // Per-column last good radius, so a stray missed ray inherits its
    // neighbour's radius (continuity) instead of spiking to some fixed value.
    let mut prev_r = vec![(hi - lo).length() * 0.25; cols + 1];
    // Nudge the polar angle off the exact poles: a ray straight along ±Z passes
    // through a mesh's pole *vertex*, where ray/triangle intersection is
    // degenerate and misses — which would collapse or spike the pole.
    let eps = 0.02;
    for iv in 0..=rows {
        let phi = eps + (std::f32::consts::PI - 2.0 * eps) * iv as f32 / rows as f32;
        let (sp, cp) = phi.sin_cos();
        for iu in 0..=cols {
            let (st, ct) = (std::f32::consts::TAU * iu as f32 / cols as f32).sin_cos();
            let dir = vec3(sp * ct, sp * st, cp);
            let mut best = 0.0f32;
            for t in tris {
                if let Some(tt) = ray_tri(center, dir, t) {
                    if tt > best {
                        best = tt;
                    }
                }
            }
            if best <= 1e-4 {
                best = prev_r[iu]; // miss: hold the column's last radius
            } else {
                prev_r[iu] = best;
            }
            pts.push(center + dir * best);
        }
    }
    pts
}

fn is_solid3(sh: &Shape3D) -> bool {
    matches!(
        sh,
        Shape3D::Cube { .. } | Shape3D::Sphere { .. } | Shape3D::Mesh { .. }
    )
}

/// Sample two 3D shapes to a common representation for morphing. Both must be
/// the same family: curves, surfaces, or solids.
pub(crate) fn build_morph3(
    sa: Shape3D,
    sb: Shape3D,
) -> Result<(Vec<Vec3>, Vec<Vec3>, Morph3Kind), String> {
    const NCURVE: usize = 160;
    const RU: u32 = 44; // common surface grid
    const RV: u32 = 44;
    const GU: usize = 48; // spherical azimuth / polar divisions
    const GV: usize = 32;
    match (sa, sb) {
        (Shape3D::Path { points: pa }, Shape3D::Path { points: pb }) => Ok((
            resample3(&pa, NCURVE),
            resample3(&pb, NCURVE),
            Morph3Kind::Path,
        )),
        (
            Shape3D::Surface {
                pts: pa,
                nu: ua,
                nv: va,
            },
            Shape3D::Surface {
                pts: pb,
                nu: ub,
                nv: vb,
            },
        ) => Ok((
            resample_surface(&pa, ua, va, RU, RV),
            resample_surface(&pb, ub, vb, RU, RV),
            Morph3Kind::Surface { nu: RU, nv: RV },
        )),
        (sa, sb) if is_solid3(&sa) && is_solid3(&sb) => {
            let ta =
                crate::render3d::shape_tris(&sa).ok_or("this solid has no surface to morph")?;
            let tb =
                crate::render3d::shape_tris(&sb).ok_or("this solid has no surface to morph")?;
            Ok((
                spherical_grid(&ta, GU, GV),
                spherical_grid(&tb, GU, GV),
                Morph3Kind::Surface {
                    nu: (GU + 1) as u32,
                    nv: (GV + 1) as u32,
                },
            ))
        }
        _ => Err(
            "morph3 needs two shapes of the same family: both curves (curve3), \
                  both surfaces (surface3/revolve3), or both solids \
                  (cube3/sphere3/prism3/pyramid3/extrude3)"
                .into(),
        ),
    }
}

/// `morph3(a, b, [spin])` — set 3D entity `a` up to morph into `b`'s shape.
/// Both are sampled now to a shared representation (curves → a polyline;
/// surfaces and solids → a grid, with solids reparameterised spherically so
/// e.g. a cube can become a sphere). Animate with `to(a, morph, 1, dur)`;
/// `spin` adds a winding rotation about the vertical axis over the blend.
fn c_morph3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let ida = a.ident(0)?;
    let idb = a.ident(1)?;
    let spin = a.opt_num(2)?.unwrap_or(0.0);
    let sa = s
        .get_3d(&ida)
        .ok_or_else(|| Error::new(format!("no 3D entity named `{ida}`"), a.span_of(0)))?
        .shape
        .clone();
    let sb = s
        .get_3d(&idb)
        .ok_or_else(|| Error::new(format!("no 3D entity named `{idb}`"), a.span_of(1)))?
        .shape
        .clone();
    let (from, to, kind) = build_morph3(sa, sb).map_err(|m| Error::new(m, a.name_span))?;
    let start = match kind {
        Morph3Kind::Path => Shape3D::Path {
            points: from.clone(),
        },
        Morph3Kind::Surface { nu, nv } => Shape3D::Surface {
            pts: from.clone(),
            nu,
            nv,
        },
    };
    let e = s.get_3d_mut(&ida).unwrap();
    e.shape = start;
    e.morph3 = Some(Morph3 {
        from,
        to,
        kind,
        spin,
    });
    Ok(())
}

/// `extrude3(id, source, height, [(cx,cy,cz)])` — sweep a 2D fillable shape
/// (rect/circle/sector/annulus/polygon or a boolean `Region`) straight up into
/// a solid prism of `height`, centred on `center`. Because the source can be a
/// `union`/`difference`/`intersect`/`xor` region, this doubles as CSG solids
/// (e.g. a plate `difference` a hole → an extruded plate-with-a-hole). The 2D
/// source is auto-hidden — it only served as the cross-section recipe.
fn c_extrude3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let src = a.ident(1)?;
    let height = a.num(2)?;
    let center = a.triple(3).unwrap_or(Vec3::ZERO);
    if !(height > 0.0) {
        return Err(Error::new("extrude height must be > 0", a.span_of(2)));
    }
    let (tris2d, rings) = {
        let e = s
            .get(&src)
            .ok_or_else(|| Error::new(format!("no 2D entity named `{src}`"), a.span_of(1)))?;
        crate::geom::cross_section(e).map_err(|m| Error::new(m, a.span_of(1)))?
    };

    // Recentre on the footprint's bounding box so `center`/`pos` places the
    // solid predictably regardless of where the 2D source sat.
    let mut lo = Vec2::splat(f32::INFINITY);
    let mut hi = Vec2::splat(f32::NEG_INFINITY);
    for ring in &rings {
        for &p in ring {
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    let mid = (lo + hi) * 0.5;
    let (zb, zt) = (-height * 0.5, height * 0.5);
    // 2D screen-space is Y-down; negate Y so the extruded footprint keeps its
    // handedness in the Z-up world (an L stays an L, not its mirror).
    let at = |p: Vec2, z: f32| vec3(p.x - mid.x, mid.y - p.y, z);

    let mut verts: Vec<Vec3> = Vec::new();
    let mut faces: Vec<[u32; 3]> = Vec::new();
    {
        let mut push_tri = |p: Vec3, q: Vec3, r: Vec3| {
            let i = verts.len() as u32;
            verts.extend_from_slice(&[p, q, r]);
            faces.push([i, i + 1, i + 2]);
        };
        // Bottom + top caps from the 2D triangulation.
        for t in &tris2d {
            push_tri(at(t[0], zb), at(t[1], zb), at(t[2], zb));
            push_tri(at(t[0], zt), at(t[1], zt), at(t[2], zt));
        }
        // Side walls: a quad per ring edge (exterior loops + holes).
        for ring in &rings {
            let n = ring.len();
            if n < 2 {
                continue;
            }
            for i in 0..n {
                let (a0, b0) = (ring[i], ring[(i + 1) % n]);
                let (pa, pb) = (at(a0, zb), at(b0, zb));
                let (pc, pd) = (at(a0, zt), at(b0, zt));
                push_tri(pa, pc, pb);
                push_tri(pb, pc, pd);
            }
        }
    }

    s.add_3d(Entity3D::new(
        id,
        Shape3D::Mesh {
            verts,
            edges: Vec::new(),
            faces,
        },
        center,
        style::FG,
    ));
    // Consume the 2D source (opacity 0) — see the doc comment.
    if let Some(src_e) = s.get_mut(&src) {
        src_e.opacity = 0.0;
    }
    Ok(())
}

fn v2_timing(a: &Args, at: usize, default: f32) -> Result<(f32, Easing), Error> {
    let (dur, easing) = timing(a, at, default)?;
    if dur <= 0.0 || !dur.is_finite() {
        return Err(Error::new(
            format!("{} duration must be positive", a.name),
            a.span_of(at),
        ));
    }
    Ok((dur, easing))
}

fn authored_attachment3(s: &Scene, id: &str) -> Option<(String, Vec3)> {
    match s.authored_attachments_3d.get(id) {
        Some(value) => value
            .as_ref()
            .map(|value| (value.target.clone(), value.offset)),
        None => s.authored_entity_3d(id).and_then(|entity| entity.follow),
    }
}

fn authored_attached_position3(s: &Scene, id: &str) -> Option<Vec3> {
    s.authored_world_entity_3d(id).map(|entity| entity.pos)
}

fn targets3(s: &Scene, id_or_tag: &str) -> Vec<String> {
    if s.get_3d(id_or_tag).is_some() && id_or_tag != CAMERA3_ID {
        return vec![id_or_tag.to_string()];
    }
    s.entities_3d
        .iter()
        .filter(|entity| entity.id != CAMERA3_ID && entity.tags.iter().any(|tag| tag == id_or_tag))
        .map(|entity| entity.id.clone())
        .collect()
}

fn path_points3(path: &Entity3D) -> Option<Vec<Vec3>> {
    Some(match &path.shape {
        Shape3D::Line { to } | Shape3D::Arrow { to } => {
            vec![path.world_point(Vec3::ZERO), path.world_point(*to)]
        }
        Shape3D::Path { points } => points
            .iter()
            .copied()
            .map(|point| path.world_point(point))
            .collect(),
        _ => return None,
    })
}

/// `travel3(entity,path,[duration],[ease])` — move one persistent 3-D entity
/// along a line, arrow, or sampled curve and hold its exact endpoint.
fn v_travel3(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    a.max(4)?;
    let id = a.ident(0)?;
    let path_id = a.ident(1)?;
    if id == path_id {
        return Err(Error::new(
            "travel3 needs different entity and path ids",
            a.span_of(1),
        ));
    }
    if s.authored_entity_3d(&id).is_none() {
        return Err(Error::new(
            format!("no 3D entity named `{id}`"),
            a.span_of(0),
        ));
    }
    let path = s
        .authored_entity_3d(&path_id)
        .ok_or_else(|| Error::new(format!("no 3D path named `{path_id}`"), a.span_of(1)))?;
    let points = path_points3(&path).ok_or_else(|| {
        Error::new(
            format!("`{path_id}` is not a line3, arrow3, or curve3"),
            a.span_of(1),
        )
    })?;
    if points.len() < 2 {
        return Err(Error::new(
            format!("path `{path_id}` has fewer than two points"),
            a.span_of(1),
        ));
    }
    let (dur, easing) = v2_timing(a, 2, 1.0)?;
    Ok(Clip {
        tracks: Vec::new(),
        events: vec![TimelineEvent::travel3(id, path_id, dur, easing)],
        dur,
    })
}

/// `attach3(child,target,[(dx,dy,dz)],[mode])`; `mode=rigid` inherits rotation.
fn v_attach3(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    a.max(4)?;
    let child = a.ident(0)?;
    let target = a.ident(1)?;
    let offset = if a.len() > 2 {
        a.triple(2)?
    } else {
        Vec3::ZERO
    };
    let rigid = if a.len() > 3 {
        match a.ident(3)?.as_str() {
            "rigid" => true,
            "position" => false,
            other => {
                return Err(Error::new(
                    format!("unknown attach3 mode `{other}`; use `position` or `rigid`"),
                    a.span_of(3),
                ))
            }
        }
    } else {
        false
    };
    if s.authored_entity_3d(&child).is_none() || child == CAMERA3_ID {
        return Err(Error::new(
            format!("no attachable 3D entity named `{child}`"),
            a.span_of(0),
        ));
    }
    if target == "none" {
        let world = s.authored_world_entity_3d(&child).ok_or_else(|| {
            Error::new(
                format!("cannot release `{child}` because its attachment chain is cyclic"),
                a.span_of(0),
            )
        })?;
        return Ok(Clip {
            tracks: vec![
                track(
                    &child,
                    Prop::Pos,
                    TargetValue::Abs(Value::V3(world.pos)),
                    0.0,
                    Easing::Linear,
                ),
                track(
                    &child,
                    Prop::Orient3,
                    TargetValue::Abs(Value::Q(world.orientation)),
                    0.0,
                    Easing::Linear,
                ),
            ],
            events: vec![TimelineEvent::attachment3(
                child,
                None,
                Vec3::ZERO,
                false,
                Quat::IDENTITY,
                0.0,
            )],
            dur: 0.0,
        });
    }
    if child == target {
        return Err(Error::new(
            "a 3D entity cannot attach to itself",
            a.span_of(1),
        ));
    }
    if s.authored_entity_3d(&target).is_none() || target == CAMERA3_ID {
        return Err(Error::new(
            format!("no attachable 3D target named `{target}`"),
            a.span_of(1),
        ));
    }
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
        let Some((next, _)) = authored_attachment3(s, &cursor) else {
            break;
        };
        cursor = next;
    }
    let relative_orientation = if rigid {
        let child_world = s
            .authored_world_entity_3d(&child)
            .ok_or_else(|| Error::new(format!("cannot resolve `{child}`"), a.span_of(0)))?;
        let target_world = s
            .authored_world_entity_3d(&target)
            .ok_or_else(|| Error::new(format!("cannot resolve `{target}`"), a.span_of(1)))?;
        target_world.rotation_quat().inverse() * child_world.rotation_quat()
    } else {
        Quat::IDENTITY
    };
    Ok(Clip {
        tracks: Vec::new(),
        events: vec![TimelineEvent::attachment3(
            child,
            Some(target),
            offset,
            rigid,
            relative_orientation,
            0.0,
        )],
        dur: 0.0,
    })
}

/// `become3(source,blueprint,[duration],[ease])` — retain identity and settle
/// on the exact target transform, geometry, and style.
fn v_become3(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    a.max(4)?;
    let id = a.ident(0)?;
    let target_id = a.ident(1)?;
    if id == target_id {
        return Err(Error::new(
            "become3 needs different source and target ids",
            a.span_of(1),
        ));
    }
    let from = s
        .authored_entity_3d(&id)
        .ok_or_else(|| Error::new(format!("no 3D entity named `{id}`"), a.span_of(0)))?;
    let mut target = s.authored_entity_3d(&target_id).ok_or_else(|| {
        Error::new(
            format!("no 3D target blueprint named `{target_id}`"),
            a.span_of(1),
        )
    })?;
    if matches!(from.shape, Shape3D::Camera { .. })
        || matches!(target.shape, Shape3D::Camera { .. })
    {
        return Err(Error::new(
            "become3 transforms drawable 3D entities, not camera3",
            a.span_of(1),
        ));
    }
    let (dur, easing) = v2_timing(a, 2, 1.0)?;
    if target.opacity <= 1e-6 && from.opacity > 1e-6 {
        target.opacity = from.opacity;
    }
    let morph = build_morph3(from.shape.clone(), target.shape.clone())
        .ok()
        .map(|(from, to, kind)| Morph3 {
            from,
            to,
            kind,
            spin: 0.0,
        });
    let crossfade = morph.is_none();
    let mut tracks = vec![
        track(
            &id,
            Prop::Pos,
            TargetValue::Abs(Value::V3(target.pos)),
            dur,
            easing,
        ),
        track(
            &id,
            Prop::Color,
            TargetValue::Abs(Value::C(target.color)),
            dur,
            easing,
        ),
        track(
            &id,
            Prop::Scale,
            TargetValue::Abs(Value::F(target.scale)),
            dur,
            easing,
        ),
        track(
            &id,
            Prop::Rot3,
            TargetValue::Abs(Value::V3(target.rotation)),
            dur,
            easing,
        ),
        track(
            &id,
            Prop::Orient3,
            TargetValue::Abs(Value::Q(target.orientation)),
            dur,
            easing,
        ),
        track(
            &id,
            Prop::Trace,
            TargetValue::Abs(Value::F(target.trace)),
            dur,
            easing,
        ),
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
        tracks.push(track(
            &id,
            Prop::Opacity,
            TargetValue::Abs(Value::F(target.opacity)),
            dur,
            easing,
        ));
    }
    Ok(Clip {
        tracks,
        events: vec![TimelineEvent::visual_transition3(
            id, from, target, morph, dur, easing, crossfade,
        )],
        dur,
    })
}

fn point_arg3(s: &Scene, a: &Args, index: usize, what: &str) -> Result<Vec3, Error> {
    match a.exprs.get(index).map(|expr| &expr.kind) {
        Some(ExprKind::Triple(x, y, z)) => Ok(Vec3::new(*x, *y, *z)),
        Some(ExprKind::Ident(id)) => authored_attached_position3(s, id)
            .ok_or_else(|| Error::new(format!("no 3D {what} named `{id}`"), a.span_of(index))),
        _ => Err(Error::new(
            format!("{what} should be a `(x, y, z)` point or 3D entity name"),
            a.span_of(index),
        )),
    }
}

fn axis_arg3(a: &Args, index: usize) -> Result<Vec3, Error> {
    let axis = match a.exprs.get(index).map(|expr| &expr.kind) {
        Some(ExprKind::Triple(x, y, z)) => Vec3::new(*x, *y, *z),
        Some(ExprKind::Ident(name)) => match name.as_str() {
            "x" => Vec3::X,
            "y" => Vec3::Y,
            "z" => Vec3::Z,
            _ => {
                return Err(Error::new(
                    "turn3 axis should be x, y, z, or a `(x, y, z)` vector",
                    a.span_of(index),
                ))
            }
        },
        _ => {
            return Err(Error::new(
                "turn3 axis should be x, y, z, or a `(x, y, z)` vector",
                a.span_of(index),
            ))
        }
    };
    if !axis.is_finite() || axis.length_squared() <= 1e-10 {
        return Err(Error::new(
            "turn3 axis must be a finite non-zero vector",
            a.span_of(index),
        ));
    }
    Ok(axis.normalize())
}

/// `turn3(id_or_tag,pivot,axis,degrees,[duration],[ease])` — rigidly rotate a
/// spatial arrangement around one world-space axis.
fn v_turn3(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    a.max(6)?;
    let id_or_tag = a.ident(0)?;
    let pivot = point_arg3(s, a, 1, "pivot")?;
    let axis = axis_arg3(a, 2)?;
    let degrees = a.num(3)?;
    if !degrees.is_finite() {
        return Err(Error::new("turn3 degrees must be finite", a.span_of(3)));
    }
    let (dur, easing) = v2_timing(a, 4, 0.9)?;
    let targets = targets3(s, &id_or_tag);
    if targets.is_empty() {
        return Err(Error::new(
            format!("no 3D entity or tag named `{id_or_tag}`"),
            a.span_of(0),
        ));
    }
    let segments = ((degrees.abs() / 5.0).ceil() as usize).clamp(1, 96);
    let segment_dur = dur / segments as f32;
    let rotation = Quat::from_axis_angle(axis, degrees.to_radians());
    let mut tracks = Vec::new();
    for id in targets {
        let entity = s
            .authored_entity_3d(&id)
            .expect("turn3 target was validated");
        let mut previous_angle = 0.0;
        for segment in 1..=segments {
            let angle = degrees * easing.apply(segment as f32 / segments as f32);
            let delta = angle - previous_angle;
            tracks.push(TrackSpec {
                id: id.clone(),
                prop: Prop::Pos,
                target: TargetValue::RotateAround3 {
                    pivot,
                    axis,
                    degrees: delta,
                },
                start: segment_dur * (segment - 1) as f32,
                dur: segment_dur,
                easing: Easing::Linear,
            });
            previous_angle = angle;
        }
        tracks.push(track(
            &id,
            Prop::Orient3,
            TargetValue::Abs(Value::Q(rotation * entity.orientation)),
            dur,
            easing,
        ));
    }
    Ok(Clip {
        tracks,
        events: Vec::new(),
        dur,
    })
}

fn camera_direction(azimuth: f32, elevation: f32) -> Vec3 {
    let (sa, ca) = azimuth.to_radians().sin_cos();
    let (se, ce) = elevation.to_radians().sin_cos();
    Vec3::new(ce * ca, ce * sa, se)
}

fn direction_angles(direction: Vec3) -> Vec2 {
    let direction = direction.normalize_or_zero();
    let flat = direction.x.hypot(direction.y);
    let azimuth = if flat <= 1e-6 {
        if direction.z >= 0.0 {
            -90.0
        } else {
            90.0
        }
    } else {
        direction.y.atan2(direction.x).to_degrees()
    };
    Vec2::new(azimuth, direction.z.atan2(flat).to_degrees())
}

fn bounds_corners(lo: Vec3, hi: Vec3) -> [Vec3; 8] {
    [
        Vec3::new(lo.x, lo.y, lo.z),
        Vec3::new(hi.x, lo.y, lo.z),
        Vec3::new(lo.x, hi.y, lo.z),
        Vec3::new(hi.x, hi.y, lo.z),
        Vec3::new(lo.x, lo.y, hi.z),
        Vec3::new(hi.x, lo.y, hi.z),
        Vec3::new(lo.x, hi.y, hi.z),
        Vec3::new(hi.x, hi.y, hi.z),
    ]
}

/// `view3(target_or_tag,"front|side|top|isometric|fit",[dur],[ease],[margin])`
/// — aim and frame the authored spatial bounds in one camera move.
fn v_view3(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    a.max(5)?;
    let target = a.ident(0)?;
    let view = a.text(1)?;
    let cam = s
        .authored_entity_3d(CAMERA3_ID)
        .ok_or_else(|| Error::new("view3 needs camera3(...) first", a.name_span))?;
    let Shape3D::Camera { fov, projection } = cam.shape else {
        return Err(Error::new("view3 needs camera3(...) first", a.name_span));
    };
    let (lo, hi) = s.authored_bounds_3d(&target).ok_or_else(|| {
        Error::new(
            format!("no bounded 3D entity or tag named `{target}`"),
            a.span_of(0),
        )
    })?;
    let (dur, easing) = v2_timing(a, 2, 1.0)?;
    let margin = a.opt_num(4)?.unwrap_or(1.18);
    if !margin.is_finite() || margin < 1.0 {
        return Err(Error::new(
            "view3 margin must be a finite number at least 1",
            a.span_of(4),
        ));
    }
    let direction = match view.as_str() {
        "fit" => camera_direction(cam.rotation.x, cam.rotation.y),
        "front" => Vec3::new(0.0, -1.0, 0.0),
        "side" | "right" => Vec3::X,
        "left" => -Vec3::X,
        "top" => Vec3::Z,
        "isometric" | "iso" => Vec3::new(1.0, -1.0, 0.82).normalize(),
        other => {
            return Err(Error::new(
                format!("unknown view3 shot `{other}` (try: front, side, top, isometric, fit)"),
                a.span_of(1),
            ))
        }
    };
    let angles = direction_angles(direction);
    let az = angles.x.to_radians();
    let el = angles.y.to_radians();
    let right = Vec3::new(-az.sin(), az.cos(), 0.0);
    let up = Vec3::new(-az.cos() * el.sin(), -az.sin() * el.sin(), el.cos());
    let center = (lo + hi) * 0.5;
    let mut half_w: f32 = 0.0;
    let mut half_h: f32 = 0.0;
    let mut half_depth: f32 = 0.0;
    for corner in bounds_corners(lo, hi) {
        let delta = corner - center;
        half_w = half_w.max(delta.dot(right).abs());
        half_h = half_h.max(delta.dot(up).abs());
        half_depth = half_depth.max(delta.dot(direction).abs());
    }
    let canvas = s.canvas();
    let aspect = (canvas.x / canvas.y).max(0.01);
    let region = s.creator_media_rect().unwrap_or(crate::scene::CreatorRect {
        center: canvas * 0.5,
        size: canvas,
    });
    let fraction = Vec2::new(
        (region.size.x / canvas.x).clamp(0.05, 1.0),
        (region.size.y / canvas.y).clamp(0.05, 1.0),
    );
    let ndc_center = Vec2::new(
        region.center.x / canvas.x * 2.0 - 1.0,
        region.center.y / canvas.y * 2.0 - 1.0,
    );
    let mut radius = cam.scale.max(0.01);
    let mut fitted_fov = fov;
    let look_target;
    match projection {
        Projection3D::Perspective => {
            let vhalf = (fov.to_radians() * 0.5).clamp(0.01, 1.55);
            let hhalf = (vhalf.tan() * aspect).atan();
            let available_v = vhalf.tan() * fraction.y;
            let available_h = hhalf.tan() * fraction.x;
            radius =
                ((half_h / available_v).max(half_w / available_h) + half_depth).max(0.05) * margin;
            look_target = center - right * (ndc_center.x * radius * hhalf.tan())
                + up * (ndc_center.y * radius * vhalf.tan());
        }
        Projection3D::Orthographic => {
            fitted_fov = (2.0 * (half_h / fraction.y).max(half_w / (aspect * fraction.x)))
                .max(0.05)
                * margin;
            radius = radius.max(half_depth + 1.0);
            look_target = center - right * (ndc_center.x * fitted_fov * aspect * 0.5)
                + up * (ndc_center.y * fitted_fov * 0.5);
        }
    }
    let mut tracks = vec![
        track(
            CAMERA3_ID,
            Prop::Pos,
            TargetValue::Abs(Value::V3(look_target)),
            dur,
            easing,
        ),
        track(
            CAMERA3_ID,
            Prop::Orbit3,
            TargetValue::Abs(Value::V3(Vec3::new(angles.x, angles.y, cam.rotation.z))),
            dur,
            easing,
        ),
        track(
            CAMERA3_ID,
            Prop::Scale,
            TargetValue::Abs(Value::F(radius)),
            dur,
            easing,
        ),
    ];
    if matches!(projection, Projection3D::Orthographic) {
        tracks.push(track(
            CAMERA3_ID,
            Prop::Fov3,
            TargetValue::Abs(Value::F(fitted_fov)),
            dur,
            easing,
        ));
    }
    Ok(Clip {
        tracks,
        events: Vec::new(),
        dur,
    })
}

fn require_3d<'a>(s: &'a Scene, a: &Args) -> Result<(&'a Entity3D, String), Error> {
    let id = a.ident(0)?;
    let e = s
        .get_3d(&id)
        .ok_or_else(|| Error::new(format!("no 3D entity named `{id}`"), a.span_of(0)))?;
    Ok((e, id))
}

fn v_move(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let (_, id) = require_3d(s, a)?;
    let (dur, easing) = timing(a, 2, 0.7)?;
    Ok(Clip {
        dur,
        tracks: vec![track(
            &id,
            Prop::Pos,
            TargetValue::Abs(Value::V3(a.triple(1)?)),
            dur,
            easing,
        )],
        events: vec![],
    })
}

fn v_shift(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let (_, id) = require_3d(s, a)?;
    let (dur, easing) = timing(a, 2, 0.7)?;
    Ok(Clip {
        dur,
        tracks: vec![track(
            &id,
            Prop::Pos,
            TargetValue::Rel(Value::V3(a.triple(1)?)),
            dur,
            easing,
        )],
        events: vec![],
    })
}

fn v_rotate(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let (_, id) = require_3d(s, a)?;
    let (dur, easing) = timing(a, 2, 0.8)?;
    Ok(Clip {
        dur,
        tracks: vec![track(
            &id,
            Prop::Rot3,
            TargetValue::Abs(Value::V3(a.triple(1)?)),
            dur,
            easing,
        )],
        events: vec![],
    })
}

fn v_grow(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let (_, id) = require_3d(s, a)?;
    let e = s
        .authored_entity_3d(&id)
        .expect("grow3 target was validated");
    if !matches!(e.shape, Shape3D::Line { .. } | Shape3D::Arrow { .. }) {
        return Err(Error::new("`grow3` needs a line3 or arrow3", a.span_of(0)));
    }
    let local = a.triple(1)? - e.pos;
    let (dur, easing) = timing(a, 2, 0.7)?;
    Ok(Clip {
        dur,
        tracks: vec![track(
            &id,
            Prop::To,
            TargetValue::Abs(Value::V3(local)),
            dur,
            easing,
        )],
        events: vec![],
    })
}

/// `orbit3(azimuth, elevation, radius, [dur], [ease])`.
fn v_orbit(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let cam = s
        .authored_entity_3d(CAMERA3_ID)
        .ok_or_else(|| Error::new("`orbit3` needs `camera3(...)`", a.name_span))?;
    let rot = vec3(a.num(0)?, a.num(1)?, cam.rotation.z);
    let radius = a.num(2)?.max(0.01);
    let (dur, easing) = timing(a, 3, 1.2)?;
    Ok(Clip {
        dur,
        tracks: vec![
            track(
                CAMERA3_ID,
                Prop::Orbit3,
                TargetValue::Abs(Value::V3(rot)),
                dur,
                easing,
            ),
            track(
                CAMERA3_ID,
                Prop::Scale,
                TargetValue::Abs(Value::F(radius)),
                dur,
                easing,
            ),
        ],
        events: vec![],
    })
}

/// `roll3(degrees, [duration], [ease])` — rotate the camera's up direction
/// around its viewing axis. Separate from `orbit3`, so both compose in `par`.
fn v_roll(s: &Scene, a: &Args) -> Result<Clip, Error> {
    if s.get_3d(CAMERA3_ID).is_none() {
        return Err(Error::new("`roll3` needs `camera3(...)`", a.name_span));
    }
    let degrees = a.num(0)?;
    let (dur, easing) = timing(a, 1, 1.2)?;
    Ok(Clip {
        dur,
        tracks: vec![track(
            CAMERA3_ID,
            Prop::Roll3,
            TargetValue::Abs(Value::F(degrees)),
            dur,
            easing,
        )],
        events: vec![],
    })
}

fn v_look(s: &Scene, a: &Args) -> Result<Clip, Error> {
    if s.get_3d(CAMERA3_ID).is_none() {
        return Err(Error::new("`look3` needs `camera3(...)`", a.name_span));
    }
    let (dur, easing) = timing(a, 1, 0.8)?;
    Ok(Clip {
        dur,
        tracks: vec![track(
            CAMERA3_ID,
            Prop::Pos,
            TargetValue::Abs(Value::V3(a.triple(0)?)),
            dur,
            easing,
        )],
        events: vec![],
    })
}

/// The 8 corners (local space), 12 edges, and 12 triangle faces of the
/// parallelepiped spanned by column vectors `c1, c2, c3` — the image of the unit
/// cube. Corner `i` has bits x = i&1, y = i&2, z = i&4.
fn parallelepiped(c1: Vec3, c2: Vec3, c3: Vec3) -> (Vec<Vec3>, Vec<(u32, u32)>, Vec<[u32; 3]>) {
    let mut verts = Vec::with_capacity(8);
    for i in 0..8u32 {
        let (x, y, z) = ((i & 1) as f32, ((i >> 1) & 1) as f32, ((i >> 2) & 1) as f32);
        verts.push(c1 * x + c2 * y + c3 * z);
    }
    let edges = vec![
        (0, 1),
        (0, 2),
        (0, 4),
        (1, 3),
        (1, 5),
        (2, 3),
        (2, 6),
        (3, 7),
        (4, 5),
        (4, 6),
        (5, 7),
        (6, 7),
    ];
    let faces = vec![
        [0, 1, 3],
        [0, 3, 2], // z = 0
        [4, 5, 7],
        [4, 7, 6], // z = 1
        [0, 1, 5],
        [0, 5, 4], // y = 0
        [2, 3, 7],
        [2, 7, 6], // y = 1
        [0, 2, 6],
        [0, 6, 4], // x = 0
        [1, 3, 7],
        [1, 7, 5], // x = 1
    ];
    (verts, edges, faces)
}

/// Determinant of the 3×3 `[[a,b,c],[d,e,f],[g,h,i]]`.
#[allow(clippy::too_many_arguments)]
fn det3(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32, g: f32, h: f32, i: f32) -> f32 {
    a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g)
}

/// `linmap3(id, (cx,cy,cz), a,b,c,d,e,f,g,h,i, [color])` — a 3×3 matrix applied to
/// space (the 3-D echo of `linmap`/`determinant`): the unit cube (faint wireframe)
/// becomes a parallelepiped — its image — with basis arrows i (cyan), j (magenta),
/// k (lime) landing on the matrix's columns. The parallelepiped's signed volume
/// IS the determinant (labelled); it flips colour when det < 0 and collapses to a
/// plane/line when det = 0.
fn c_linmap3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let o = a.triple(1)?;
    let m = [
        a.num(2)?,
        a.num(3)?,
        a.num(4)?,
        a.num(5)?,
        a.num(6)?,
        a.num(7)?,
        a.num(8)?,
        a.num(9)?,
        a.num(10)?,
    ];
    // columns = images of the basis vectors  (î → (a,d,g), etc.)
    let c1 = vec3(m[0], m[3], m[6]);
    let c2 = vec3(m[1], m[4], m[7]);
    let c3 = vec3(m[2], m[5], m[8]);
    let det = det3(m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8]);
    let fill = if a.len() > 11 {
        resolve_color(&a.ident(11)?, a.span_of(11))?
    } else if det < 0.0 {
        style::MAGENTA
    } else {
        style::LIME
    };
    // faint reference unit cube (identity), wireframe only
    let (uv, ue, _) = parallelepiped(
        vec3(1.0, 0.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        vec3(0.0, 0.0, 1.0),
    );
    let mut refc = Entity3D::new(
        format!("{id}.ref"),
        Shape3D::Mesh {
            verts: uv,
            edges: ue,
            faces: vec![],
        },
        o,
        style::DIM,
    );
    refc.opacity = 0.4;
    refc.tags.push(id.clone());
    s.add_3d(refc);
    // the image parallelepiped (filled + wireframe)
    let (pv, pe, pf) = parallelepiped(c1, c2, c3);
    let mut img = Entity3D::new(
        id.clone(),
        Shape3D::Mesh {
            verts: pv,
            edges: pe,
            faces: pf,
        },
        o,
        fill,
    );
    img.opacity = 0.32;
    img.tags.push(id.clone());
    s.add_3d(img);
    // basis arrows on the columns, with pinned labels
    for (nm, col, col_c) in [
        ("i", c1, style::CYAN),
        ("j", c2, style::MAGENTA),
        ("k", c3, style::LIME),
    ] {
        let mut arr = Entity3D::new(format!("{id}.{nm}"), Shape3D::Arrow { to: col }, o, col_c);
        arr.tags.push(id.clone());
        s.add_3d(arr);
        let lbl = format!("{id}.l{nm}");
        let mut t = Entity::new(
            lbl.clone(),
            Shape::Text {
                content: nm.to_string(),
                size: 22.0,
            },
            Vec2::ZERO,
            col_c,
        );
        t.tags.push(id.clone());
        s.add(t);
        s.pins.push(Pin3 {
            label: lbl,
            target: Pin3Target::Point(o + col),
            offset: vec2(10.0, -10.0),
            declutter: true,
            world_height: None,
        });
    }
    // det = signed volume, pinned to the parallelepiped's centroid
    let centroid = o + (c1 + c2 + c3) * 0.5;
    let vlbl = format!("{id}.val");
    let mut vt = Entity::new(
        vlbl.clone(),
        Shape::Text {
            content: format!("det = {det:.2}"),
            size: 24.0,
        },
        Vec2::ZERO,
        style::GOLD,
    );
    vt.tags.push(id.clone());
    s.add(vt);
    s.pins.push(Pin3 {
        label: vlbl,
        target: Pin3Target::Point(centroid),
        offset: vec2(0.0, 0.0),
        declutter: false,
        world_height: None,
    });
    Ok(())
}

/// Real roots of `x³ + a2·x² + a1·x + a0`. Returns 1 root (one real, two complex)
/// or 3 roots (all real, counting multiplicity) — so the count tells the caller
/// how many eigenvalues are complex.
fn real_cubic_roots(a2: f32, a1: f32, a0: f32) -> Vec<f32> {
    let shift = -a2 / 3.0;
    // depressed cubic t³ + p·t + q = 0  (x = t + shift)
    let p = a1 - a2 * a2 / 3.0;
    let q = 2.0 * a2 * a2 * a2 / 27.0 - a2 * a1 / 3.0 + a0;
    let disc = q * q / 4.0 + p * p * p / 27.0;
    if disc > 1e-9 {
        let s = disc.sqrt();
        let u = (-q / 2.0 + s).cbrt();
        let v = (-q / 2.0 - s).cbrt();
        vec![u + v + shift]
    } else if p.abs() < 1e-9 {
        vec![shift, shift, shift] // triple root
    } else {
        let r = (-p / 3.0).sqrt();
        let phi = ((3.0 * q) / (2.0 * p) * (-3.0 / p).sqrt())
            .clamp(-1.0, 1.0)
            .acos();
        (0..3)
            .map(|k| 2.0 * r * (phi / 3.0 - std::f32::consts::TAU * k as f32 / 3.0).cos() + shift)
            .collect()
    }
}

/// A unit eigenvector of the 3×3 `m` for eigenvalue `l`: a null vector of
/// `A − λI`, found as the largest cross product of its rows (they span the row
/// space, so their cross is orthogonal to both — i.e. in the null space).
fn eigvec3(m: &[f32; 9], l: f32) -> Vec3 {
    let b0 = vec3(m[0] - l, m[1], m[2]);
    let b1 = vec3(m[3], m[4] - l, m[5]);
    let b2 = vec3(m[6], m[7], m[8] - l);
    let cands = [b0.cross(b1), b1.cross(b2), b2.cross(b0)];
    let best = cands
        .iter()
        .copied()
        .max_by(|a, b| a.length().partial_cmp(&b.length()).unwrap())
        .unwrap();
    if best.length() < 1e-5 {
        vec3(1.0, 0.0, 0.0) // degenerate (repeated/scalar eigenspace) — pick an axis
    } else {
        best.normalize()
    }
}

/// Real eigenpairs `(λ, unit eigenvector)` of the 3×3 `m`, plus how many of the
/// three eigenvalues are complex (0 or 2). Repeated real eigenvalues are merged.
fn eig3(m: &[f32; 9]) -> (Vec<(f32, Vec3)>, usize) {
    let tr = m[0] + m[4] + m[8];
    let minors =
        (m[4] * m[8] - m[5] * m[7]) + (m[0] * m[8] - m[2] * m[6]) + (m[0] * m[4] - m[1] * m[3]);
    let dt = det3(m[0], m[1], m[2], m[3], m[4], m[5], m[6], m[7], m[8]);
    let roots = real_cubic_roots(-tr, minors, -dt);
    let n_complex = 3 - roots.len();
    let mut lambdas: Vec<f32> = Vec::new();
    for r in roots {
        if !lambdas.iter().any(|x: &f32| (x - r).abs() < 1e-3) {
            lambdas.push(r);
        }
    }
    let pairs = lambdas.into_iter().map(|l| (l, eigvec3(m, l))).collect();
    (pairs, n_complex)
}

/// `eigen3(id, (cx,cy,cz), a,b,c,d,e,f,g,h,i, [color])` — the real **eigenvectors**
/// of a 3×3 matrix as invariant lines through the origin (a vector on them only
/// stretches, by λ). The 3-D echo of `eigen`. A real 3×3 always has ≥1 real
/// eigenvector; a rotation leaves 2 complex eigenvalues, noted.
fn c_eigen3(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let o = a.triple(1)?;
    let m = [
        a.num(2)?,
        a.num(3)?,
        a.num(4)?,
        a.num(5)?,
        a.num(6)?,
        a.num(7)?,
        a.num(8)?,
        a.num(9)?,
        a.num(10)?,
    ];
    let explicit = if a.len() > 11 {
        Some(resolve_color(&a.ident(11)?, a.span_of(11))?)
    } else {
        None
    };
    let (pairs, n_complex) = eig3(&m);
    let palette = [style::CYAN, style::MAGENTA, style::LIME];
    let ext = 2.5;
    for (k, (l, v)) in pairs.iter().enumerate() {
        let col = explicit.unwrap_or(palette[k % 3]);
        let mut line = Entity3D::new(
            format!("{id}.axis{k}"),
            Shape3D::Line {
                to: *v * (2.0 * ext),
            },
            o - *v * ext,
            col,
        );
        line.tags.push(id.clone());
        s.add_3d(line);
        let lbl = format!("{id}.l{k}");
        let mut t = Entity::new(
            lbl.clone(),
            Shape::Text {
                content: format!("lambda = {l:.2}"),
                size: 22.0,
            },
            Vec2::ZERO,
            col,
        );
        t.tags.push(id.clone());
        s.add(t);
        s.pins.push(Pin3 {
            label: lbl,
            target: Pin3Target::Point(o + *v * ext),
            offset: vec2(10.0, -10.0),
            declutter: true,
            world_height: None,
        });
    }
    if n_complex > 0 {
        let note = format!("{id}.note");
        let mut t = Entity::new(
            note.clone(),
            Shape::Text {
                content: format!("+ {n_complex} complex eigenvalues (a rotation)"),
                size: 20.0,
            },
            Vec2::ZERO,
            style::DIM,
        );
        t.tags.push(id.clone());
        s.add(t);
        s.pins.push(Pin3 {
            label: note,
            target: Pin3Target::Point(o),
            offset: vec2(0.0, 44.0),
            declutter: false,
            world_height: None,
        });
    }
    Ok(())
}

pub fn register(r: &mut Registry) {
    r.ctor("linmap3", c_linmap3);
    r.ctor("eigen3", c_eigen3);
    r.ctor("camera3", c_camera);
    r.ctor("point3", c_point);
    r.ctor("line3", c_line);
    r.ctor("arrow3", c_arrow);
    r.ctor("cube3", c_cube);
    r.ctor("sphere3", c_sphere);
    r.ctor("grid3", c_grid);
    r.ctor("axes3", c_axes);
    r.verb("move3", v_move);
    r.verb("shift3", v_shift);
    r.verb("rotate3", v_rotate);
    r.verb("grow3", v_grow);
    r.verb("orbit3", v_orbit);
    r.verb("roll3", v_roll);
    r.verb("look3", v_look);
    r.mut_verb("view3", v_view3);
    r.mut_verb("travel3", v_travel3);
    r.mut_verb("attach3", v_attach3);
    r.mut_verb("become3", v_become3);
    r.mut_verb("turn3", v_turn3);
    r.ctor("pin3", c_pin);
    r.ctor("follow3", c_follow3);
    r.ctor("midpoint3", c_midpoint3);
    r.ctor("link3", c_link3);
    r.ctor("project3", c_project3);
    r.ctor("contour3", c_contour3);
    r.ctor("label3", c_label3);
    r.ctor("model3", c_model3);
    r.ctor("tube3", c_tube3);
    r.ctor("curve3", c_curve3);
    r.ctor("surface3", c_surface3);
    r.ctor("heightmap3", c_heightmap3);
    r.ctor("param3", c_param3);
    r.ctor("gradient3", c_gradient3);
    r.ctor("tangentplane3", c_tangentplane3);
    r.ctor("volume3", c_volume3);
    r.ctor("prism3", c_prism3);
    r.ctor("pyramid3", c_pyramid3);
    r.ctor("revolve3", c_revolve3);
    r.ctor("extrude3", c_extrude3);
    r.ctor("morph3", c_morph3);
    r.ctor("thick", c_thick);
    r.ctor("finish3", c_finish3);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn det3_is_the_signed_volume() {
        // the identity leaves the unit cube (volume 1)
        assert!((det3(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0) - 1.0).abs() < 1e-5);
        // a diagonal scales volume by the product of the diagonal
        assert!((det3(2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0) - 24.0).abs() < 1e-4);
        // a shear preserves volume (det = 1)
        assert!((det3(1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0) - 1.0).abs() < 1e-5);
        // swapping two columns flips orientation (det < 0)
        assert!(det3(0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0) < 0.0);
        // the parallelepiped has 8 corners, 12 edges, 12 triangle faces
        let (v, e, f) = parallelepiped(
            vec3(1.0, 0.0, 0.0),
            vec3(0.0, 1.0, 0.0),
            vec3(0.0, 0.0, 1.0),
        );
        assert_eq!((v.len(), e.len(), f.len()), (8, 12, 12));
    }

    #[test]
    fn eig3_finds_real_eigenpairs() {
        // diagonal diag(2,3,4): eigenvalues 2,3,4 along the axes, A·v = λ·v
        let d = [2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0];
        let (pairs, nc) = eig3(&d);
        assert_eq!(nc, 0);
        assert_eq!(pairs.len(), 3);
        for (l, v) in &pairs {
            let av = vec3(
                d[0] * v.x + d[1] * v.y + d[2] * v.z,
                d[3] * v.x + d[4] * v.y + d[5] * v.z,
                d[6] * v.x + d[7] * v.y + d[8] * v.z,
            );
            assert!((av - *v * *l).length() < 1e-3, "A·v != λ·v for λ={l}");
        }
        assert!([2.0, 3.0, 4.0]
            .iter()
            .all(|t| pairs.iter().any(|(l, _)| (l - t).abs() < 1e-3)));
        // a 90° rotation about z: one real eigenvalue (λ=1, axis = z), two complex
        let rot = [0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let (rp, rc) = eig3(&rot);
        assert_eq!(rc, 2);
        assert_eq!(rp.len(), 1);
        assert!((rp[0].0 - 1.0).abs() < 1e-3 && rp[0].1.z.abs() > 0.99); // eigenaxis is z
    }

    #[test]
    fn surface_partials_match_calculus() {
        // f(x,y) = x^2 + y^2 : fx = 2x, fy = 2y  (a paraboloid bowl)
        let f = crate::kits::math::expr::compile("x*x + y*y").unwrap();
        let sf = SurfaceFn {
            f,
            x0: -3.0,
            x1: 3.0,
            y0: -3.0,
            y1: 3.0,
        };
        assert!(
            (sf.dx(1.0, 0.5) - 2.0).abs() < 1e-2,
            "fx = {}",
            sf.dx(1.0, 0.5)
        );
        assert!(
            (sf.dy(0.5, 1.5) - 3.0).abs() < 1e-2,
            "fy = {}",
            sf.dy(0.5, 1.5)
        );
        // gradient is zero at the minimum (0,0)
        assert!(sf.dx(0.0, 0.0).abs() < 1e-2 && sf.dy(0.0, 0.0).abs() < 1e-2);
    }

    #[test]
    fn complete_3d_scene_lowers_and_animates() {
        let src = r#"
            canvas(1280, 720);
            camera3((8, -10, 6), (0, 0, 0), 45);
            grid3(floor, (0, 0, 0), 4, 1);
            axes3(world, (0, 0, 0), 3);
            cube3(box, (0, 0, 1), (2, 2, 2));
            color(box, magenta);
            par {
                move3(box, (2, 1, 2), 1, linear);
                rotate3(box, (0, 0, 180), 1, linear);
                orbit3(60, 25, 10, 1, linear);
                roll3(-90, 1, linear);
            }
        "#;
        let movie = crate::parse(src).unwrap();
        // cam + grid + cube = 3, plus axes3(len 3, step 1) = 3 arrows + 9 ticks
        assert_eq!(movie.base().entities_3d.len(), 15);
        // axes3 numeric labels: 3 per axis (1,2,3) = 9 2D texts, each pin3'd
        let nums = movie
            .base()
            .entities
            .iter()
            .filter(|e| e.id.contains(".num."))
            .count();
        assert_eq!(nums, 9);
        assert_eq!(movie.base().pins.len(), 9);
        let (base, timeline) = movie.finalize();
        let start_az = base.get_3d(CAMERA3_ID).unwrap().rotation.x;
        let frame = timeline.apply(&base, 0.5);
        assert_eq!(frame.get_3d("box").unwrap().pos, vec3(1.0, 0.5, 1.5));
        assert!(
            (frame.get_3d(CAMERA3_ID).unwrap().rotation.x - (start_az + 60.0) / 2.0).abs() < 0.01
        );
        assert!((frame.get_3d(CAMERA3_ID).unwrap().rotation.z + 45.0).abs() < 0.01);
    }

    #[test]
    fn view3_fits_authored_group_bounds_inside_the_canvas() {
        let src = r#"
            canvas(1080, 1920);
            camera3((8, -10, 6), (0, 0, 0), 45);
            cube3(a, (-3, 0, 1), (2, 2, 2)); tag(a, subject);
            sphere3(b, (3, 1, 2), 1.5); tag(b, subject);
            move3(b, (4, 2, 2), 0.4, linear);
            view3(subject, "isometric", 1, linear, 1.2);
        "#;
        let movie = crate::parse(src).unwrap();
        let authored = movie.scene.authored_bounds_3d("subject").unwrap();
        assert!((authored.0 - vec3(-4.0, -1.0, 0.0)).length() < 1e-3);
        assert!((authored.1 - vec3(5.5, 3.5, 3.5)).length() < 1e-3);
        let (base, timeline) = movie.finalize();
        let frame = timeline.apply(&base, timeline.dur);
        let cam = frame.get_3d(CAMERA3_ID).unwrap();
        assert!((cam.pos - (authored.0 + authored.1) * 0.5).length() < 1e-3);
        for corner in bounds_corners(authored.0, authored.1) {
            let pixel = crate::render3d::project(&frame, 1080.0 / 1920.0, corner, 1080.0, 1920.0)
                .expect("fitted bound should remain in front of the camera");
            assert!(
                (0.0..=1080.0).contains(&pixel.x) && (0.0..=1920.0).contains(&pixel.y),
                "corner {corner:?} projected outside: {pixel:?}"
            );
        }
    }

    #[test]
    fn view3_uses_the_creator_media_safe_rectangle() {
        let src = r#"
            canvas(1080,1920);
            creator(me,"@a footer=social safe=reels");
            camera3((8,-10,6),(0,0,0),45);
            cube3(subject,(0,0,1),(4,2,2));
            view3(subject,"front",1,linear,1.05);
        "#;
        let movie = crate::parse(src).unwrap();
        let region = movie.scene.creator_media_rect().unwrap();
        let bounds = movie.scene.authored_bounds_3d("subject").unwrap();
        let (base, timeline) = movie.finalize();
        let frame = timeline.apply(&base, timeline.dur);
        let (safe_lo, safe_hi) = region.edges();
        for corner in bounds_corners(bounds.0, bounds.1) {
            let pixel = crate::render3d::project(&frame, 1080.0 / 1920.0, corner, 1080.0, 1920.0)
                .expect("safe-framed bound should remain in front of the camera");
            assert!(
                pixel.x >= safe_lo.x - 1.0
                    && pixel.x <= safe_hi.x + 1.0
                    && pixel.y >= safe_lo.y - 1.0
                    && pixel.y <= safe_hi.y + 1.0,
                "corner {corner:?} projected outside creator media {safe_lo:?}..{safe_hi:?}: {pixel:?}"
            );
        }
        let projected_center = crate::render3d::project(
            &frame,
            1080.0 / 1920.0,
            (bounds.0 + bounds.1) * 0.5,
            1080.0,
            1920.0,
        )
        .unwrap();
        assert!(
            projected_center.distance(region.center) < 2.0,
            "projected {projected_center:?}, expected {:?}",
            region.center
        );
    }

    #[test]
    fn travel3_attach3_and_release_are_exact_and_scrubbable() {
        let src = r#"
            camera3((8,-10,6),(0,0,0),45);
            point3(ship,(0,0,0),0.2);
            point3(badge,(0,0,1),0.1);
            line3(route,(0,0,0),(4,2,1));
            attach3(badge,ship,(0,0,1));
            travel3(ship,route,1,linear);
            attach3(badge,none);
            move3(ship,(8,0,0),1,linear);
        "#;
        let (base, timeline) = crate::parse(src).unwrap().finalize();
        let arrival = timeline.apply(&base, 1.0);
        assert!((arrival.get_3d("ship").unwrap().pos - vec3(4.0, 2.0, 1.0)).length() < 1e-4);
        assert!((arrival.get_3d("badge").unwrap().pos - vec3(4.0, 2.0, 2.0)).length() < 1e-4);
        let settled = timeline.apply(&base, 2.0);
        assert!((settled.get_3d("ship").unwrap().pos - vec3(8.0, 0.0, 0.0)).length() < 1e-4);
        assert!((settled.get_3d("badge").unwrap().pos - vec3(4.0, 2.0, 2.0)).length() < 1e-4);
        let backwards = timeline.apply(&base, 0.5);
        let forwards = timeline.apply(&base, 0.5);
        assert_eq!(
            backwards.get_3d("ship").unwrap().pos,
            forwards.get_3d("ship").unwrap().pos
        );
        assert_eq!(
            backwards.get_3d("badge").unwrap().pos,
            forwards.get_3d("badge").unwrap().pos
        );
    }

    #[test]
    fn travel3_samples_a_path_while_the_path_transforms() {
        let src = r#"
            camera3((8,-10,6),(0,0,0),45);
            point3(ship,(0,0,0),0.2);
            line3(route,(0,0,0),(4,0,0));
            par {
                travel3(ship,route,1,linear);
                rotate3(route,(0,0,90),1,linear);
            }
        "#;
        let (base, timeline) = crate::parse(src).unwrap().finalize();
        let halfway = timeline.apply(&base, 0.5).get_3d("ship").unwrap().pos;
        assert!(
            (halfway - vec3(2.0_f32.sqrt(), 2.0_f32.sqrt(), 0.0)).length() < 1e-3,
            "{halfway:?}"
        );
        let end = timeline.apply(&base, 1.0).get_3d("ship").unwrap().pos;
        assert!((end - vec3(0.0, 4.0, 0.0)).length() < 1e-3, "{end:?}");
    }

    #[test]
    fn rigid_attach3_inherits_orientation_and_release_freezes_world_pose() {
        let src = r#"
            camera3((8,-10,6),(0,0,0),45);
            cube3(parent,(0,0,0),(1,1,1));
            cube3(child,(1,0,0),(0.4,0.4,0.4));
            attach3(child,parent,(1,0,0),rigid);
            turn3(parent,(0,0,0),z,90,1,linear);
            attach3(child,none);
            turn3(parent,(0,0,0),z,90,1,linear);
        "#;
        let (base, timeline) = crate::parse(src).unwrap().finalize();
        let attached = timeline.apply(&base, 1.0);
        let child = attached.get_3d("child").unwrap();
        assert!(
            (child.pos - vec3(0.0, 1.0, 0.0)).length() < 1e-3,
            "{:?}",
            child.pos
        );
        let attached_orientation = child.rotation_quat();
        let released = timeline.apply(&base, 2.0);
        let child = released.get_3d("child").unwrap();
        assert!(
            (child.pos - vec3(0.0, 1.0, 0.0)).length() < 1e-3,
            "{:?}",
            child.pos
        );
        assert!(child.rotation_quat().dot(attached_orientation).abs() > 0.999);
    }

    #[test]
    fn finish3_is_one_bounded_opt_in_render_surface() {
        let movie = crate::parse(
            "camera3((6,-6,4),(0,0,0),45); sphere3(globe,(0,0,0),1); \
             finish3(globe,\"shading=smooth material=metal texture=checker scale=3 mesh=0.2 depth=0.4 shadow=0.5\");",
        ).unwrap();
        let globe = movie.base().get_3d("globe").unwrap();
        assert_eq!(globe.finish.shading, Shading3::Smooth);
        assert_eq!(globe.finish.material, Material3::Metal);
        assert_eq!(globe.finish.texture, Texture3::Checker);
        assert!((globe.finish.shadow - 0.5).abs() < 1e-5);
    }

    #[test]
    fn spatial_explanations_recompute_and_contours_are_geometry() {
        let src = r#"
            camera3((7,-8,5),(0,0,0),45);
            point3(source,(1,2,3),0.1);
            point3(anchor,(-1,0,0),0.1);
            project3(shadow,source,"xy");
            link3(drop,source,shadow,0.05);
            surface3(bowl,"x^2+y^2",(-2,2),(-2,2),16);
            contour3(ring,bowl,1);
            text(note,(0,0),"P"); label3(note,source,0.4);
            move3(source,(3,4,5),1,linear);
        "#;
        let movie = crate::parse(src).unwrap();
        let ring = movie.base().get_3d("ring").unwrap();
        assert!(matches!(&ring.shape, Shape3D::Mesh { edges, .. } if !edges.is_empty()));
        assert_eq!(movie.base().pins.last().unwrap().world_height, Some(0.4));
        let (base, timeline) = movie.finalize();
        let frame = timeline.apply(&base, 1.0);
        assert!((frame.get_3d("shadow").unwrap().pos - vec3(3.0, 4.0, 0.0)).length() < 1e-4);
        let drop = frame.get_3d("drop").unwrap();
        assert!(matches!(drop.shape, Shape3D::Line { .. }));
    }

    #[test]
    fn tube3_builds_a_variable_radius_mesh() {
        let movie = crate::parse(
            "camera3((7,-8,5),(0,0,0),45); \
             curve3(spine,\"4*t-2\",\"0\",\"0\",(0,1)); \
             tube3(horn,spine,\"0.08+0.35*t\",10);",
        )
        .unwrap();
        let horn = movie.base().get_3d("horn").unwrap();
        assert!(
            matches!(&horn.shape, Shape3D::Mesh { verts, faces, .. } if verts.len() > 20 && !faces.is_empty())
        );
    }

    #[test]
    fn model3_loads_geometry_only_obj() {
        let path = std::env::temp_dir().join(format!("manic-model3-{}.obj", std::process::id()));
        std::fs::write(&path, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n").unwrap();
        let src = format!(
            "camera3((5,-5,4),(0,0,0),45); model3(mark,\"{}\",(0,0,0),2);",
            path.display()
        );
        let movie = crate::parse(&src).unwrap();
        let mark = movie.base().get_3d("mark").unwrap();
        assert!(
            matches!(&mark.shape, Shape3D::Mesh { verts, faces, .. } if verts.len() == 3 && faces.len() == 1)
        );
        assert_eq!(mark.scale, 2.0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn model3_resolves_a_bundled_asset_without_a_cwd_relative_path() {
        let movie = crate::parse(
            "camera3((5,-5,4),(0,0,0),45); \
             model3(mark,\"asset:models/manic-pyramid.obj\",(0,0,0),1);",
        )
        .unwrap();
        let mark = movie.base().get_3d("mark").unwrap();
        assert!(
            matches!(&mark.shape, Shape3D::Mesh { verts, faces, .. } if verts.len() == 5 && faces.len() == 6)
        );
    }

    #[test]
    fn model3_rejects_bundled_asset_traversal() {
        let err = match crate::parse(
            "camera3((5,-5,4),(0,0,0),45); \
             model3(mark,\"asset:../Cargo.toml\",(0,0,0),1);",
        ) {
            Err(error) => error,
            Ok(_) => panic!("expected bundled asset traversal to fail"),
        };
        assert!(err.msg.contains("without `..`"));
    }

    #[test]
    fn view3_bounds_resolve_an_active_attachment_chain() {
        let src = r#"
            canvas(1080,1920);
            camera3((8,-10,6),(0,0,0),45);
            cube3(ship,(0,0,0),(2,2,2)); tag(ship,rig);
            sphere3(sensor,(0,0,0),0.5); tag(sensor,rig);
            attach3(sensor,ship,(0,0,2));
            move3(ship,(4,3,1),1,linear);
            view3(rig,"fit",1,linear,1.2);
        "#;
        let movie = crate::parse(src).unwrap();
        let sensor = movie.scene.authored_world_entity_3d("sensor").unwrap();
        assert!((sensor.pos - vec3(4.0, 3.0, 3.0)).length() < 1e-4);
        let (lo, hi) = movie.scene.authored_bounds_3d("rig").unwrap();
        assert!(lo.x <= 3.0 && lo.y <= 2.0 && lo.z <= 0.0);
        assert!(hi.x >= 4.5 && hi.y >= 3.5 && hi.z >= 3.5);
    }

    #[test]
    fn attach3_rejects_relationship_cycles() {
        let err = match crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\n\
             point3(a,(0,0,0)); point3(b,(1,0,0));\n\
             attach3(a,b); attach3(b,a);",
        ) {
            Err(error) => error,
            Ok(_) => panic!("expected attach3 cycle to fail"),
        };
        assert!(err.msg.contains("cycle"), "{}", err.msg);
    }

    #[test]
    fn turn3_preserves_a_rigid_group_and_rotates_orientation() {
        let src = r#"
            camera3((6,-6,4),(0,0,0),45);
            cube3(a,(1,0,0),(1,1,1)); tag(a,rig);
            cube3(b,(0,1,0),(1,1,1)); tag(b,rig);
            turn3(rig,(0,0,0),z,90,1,linear);
        "#;
        let (base, timeline) = crate::parse(src).unwrap().finalize();
        let before = timeline.apply(&base, 0.0);
        let after = timeline.apply(&base, 1.0);
        let pa = after.get_3d("a").unwrap().pos;
        let pb = after.get_3d("b").unwrap().pos;
        assert!((pa - vec3(0.0, 1.0, 0.0)).length() < 1e-3, "{pa:?}");
        assert!((pb - vec3(-1.0, 0.0, 0.0)).length() < 1e-3, "{pb:?}");
        let d0 = before
            .get_3d("a")
            .unwrap()
            .pos
            .distance(before.get_3d("b").unwrap().pos);
        assert!((pa.distance(pb) - d0).abs() < 1e-4);
        let facing = after.get_3d("a").unwrap().orientation * Vec3::X;
        assert!((facing - Vec3::Y).length() < 1e-3, "{facing:?}");
        assert_eq!(
            timeline.apply(&base, 0.37).get_3d("a").unwrap().pos,
            timeline.apply(&base, 0.37).get_3d("a").unwrap().pos
        );
    }

    #[test]
    fn become3_morphs_compatible_solids_and_settles_on_exact_blueprint() {
        let src = r#"
            camera3((6,-6,4),(0,0,0),45);
            cube3(seed,(0,0,0),(2,2,2)); color(seed,cyan);
            sphere3(goal,(3,2,1),1.4); color(goal,magenta); hidden(goal);
            become3(seed,goal,1,linear);
        "#;
        let (base, timeline) = crate::parse(src).unwrap().finalize();
        let mid = timeline.apply(&base, 0.5);
        assert!(matches!(
            mid.get_3d("seed").unwrap().shape,
            Shape3D::Surface { .. }
        ));
        let end = timeline.apply(&base, 1.0);
        let seed = end.get_3d("seed").unwrap();
        assert_eq!(seed.shape, Shape3D::Sphere { radius: 1.4 });
        assert!((seed.pos - vec3(3.0, 2.0, 1.0)).length() < 1e-4);
        assert!((seed.color.r - style::MAGENTA.r).abs() < 1e-4);
        assert!(
            seed.opacity > 0.99,
            "hidden blueprint should not hide the source"
        );
    }

    #[test]
    fn become3_crossfade_fallback_still_installs_the_exact_blueprint() {
        let src = r#"
            camera3((6,-6,4),(0,0,0),45);
            line3(seed,(0,0,0),(2,0,0)); color(seed,cyan);
            cube3(goal,(3,2,1),(2,1,3)); color(goal,gold); hidden(goal);
            become3(seed,goal,1,linear);
        "#;
        let (base, timeline) = crate::parse(src).unwrap().finalize();
        let halfway = timeline.apply(&base, 0.5);
        assert!(halfway.get_3d("seed").unwrap().opacity <= 1e-4);
        let seed = timeline.apply(&base, 1.0).get_3d("seed").unwrap().clone();
        assert_eq!(
            seed.shape,
            Shape3D::Cube {
                size: vec3(2.0, 1.0, 3.0)
            }
        );
        assert!((seed.pos - vec3(3.0, 2.0, 1.0)).length() < 1e-4);
        assert!((seed.color.r - style::GOLD.r).abs() < 1e-4);
        assert!(seed.opacity > 0.99);
    }

    #[test]
    fn creator_3d_verbs_reject_ambiguous_or_degenerate_inputs() {
        let cases = [
            (
                "camera3((5,-5,4),(0,0,0),45); cube3(a,(0,0,0),(1,1,1)); turn3(a,(0,0,0),(0,0,0),45);",
                "non-zero",
            ),
            (
                "camera3((5,-5,4),(0,0,0),45); point3(a,(0,0,0)); sphere3(path,(0,0,0),1); travel3(a,path);",
                "not a line3, arrow3, or curve3",
            ),
            (
                "camera3((5,-5,4),(0,0,0),45); cube3(a,(0,0,0),(1,1,1)); view3(a,\"isometric\",1,smooth,0.8);",
                "at least 1",
            ),
        ];
        for (source, expected) in cases {
            let error = match crate::parse(source) {
                Err(error) => error,
                Ok(_) => panic!("invalid 3D V2 input should fail"),
            };
            assert!(
                error.msg.contains(expected),
                "expected {expected:?} in {:?}",
                error.msg
            );
        }
    }

    #[test]
    fn midpoint3_and_follow3_recompute_as_sources_move() {
        let src = r#"
            canvas(1280, 720);
            camera3((8, -10, 6), (0, 0, 0), 45);
            point3(a, (0, 0, 0), 0.2);
            point3(b, (4, 0, 0), 0.2);
            midpoint3(m, a, b);
            point3(dotp, (0, 0, 0), 0.1);
            follow3(dotp, m, (0, 0, 1));
            move3(a, (0, 4, 0), 1, linear);
        "#;
        let (base, timeline) = crate::parse(src).unwrap().finalize();
        // t=0: m = midpoint((0,0,0),(4,0,0)) = (2,0,0); dotp = m + (0,0,1)
        let f0 = timeline.apply(&base, 0.0);
        assert_eq!(f0.get_3d("m").unwrap().pos, vec3(2.0, 0.0, 0.0));
        assert_eq!(f0.get_3d("dotp").unwrap().pos, vec3(2.0, 0.0, 1.0));
        // t=1: a → (0,4,0), so m = (2,2,0) and dotp tracks to (2,2,1)
        let f1 = timeline.apply(&base, 1.0);
        assert_eq!(f1.get_3d("m").unwrap().pos, vec3(2.0, 2.0, 0.0));
        assert_eq!(f1.get_3d("dotp").unwrap().pos, vec3(2.0, 2.0, 1.0));
    }

    #[test]
    fn curve3_samples_a_parametric_path() {
        let src = r#"
            canvas(1280, 720);
            camera3((8, -10, 6), (0, 0, 0), 45);
            curve3(helix, "cos(t)", "sin(t)", "t*0.2");
        "#;
        let (base, _tl) = crate::parse(src).unwrap().finalize();
        match &base.get_3d("helix").unwrap().shape {
            Shape3D::Path { points } => {
                assert!(points.len() > 100);
                // t=0 → (cos 0, sin 0, 0) = (1, 0, 0)
                assert!((points[0] - vec3(1.0, 0.0, 0.0)).length() < 1e-3);
            }
            other => panic!("expected Path, got {other:?}"),
        }
    }

    #[test]
    fn param3_meshes_a_parametric_surface() {
        // a torus: (R + r cos v) around u, r sin v up
        let (base, _) = crate::parse(
            "camera3((6,-6,4),(0,0,0),45);\n\
             param3(t,\"(3+cos(v))*cos(u)\",\"(3+cos(v))*sin(u)\",\"sin(v)\",(0,6.283),(0,6.283),16);",
        )
        .unwrap()
        .finalize();
        match &base.get_3d("t").unwrap().shape {
            Shape3D::Surface { pts, nu, nv } => {
                assert_eq!(*nu, 17);
                assert_eq!(*nv, 17);
                assert_eq!(pts.len(), 17 * 17);
                // torus lies within radius R+r = 4 of the axis
                for p in pts {
                    assert!(p.length() < 4.5, "point off the torus: {p:?}");
                }
            }
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn surface3_meshes_a_height_field() {
        let src = r#"
            canvas(1280, 720);
            camera3((8, -10, 6), (0, 0, 0), 45);
            surface3(sheet, "x + y", (0, 2), (0, 2), 2);
        "#;
        let (base, _tl) = crate::parse(src).unwrap().finalize();
        match &base.get_3d("sheet").unwrap().shape {
            Shape3D::Surface { pts, nu, nv } => {
                assert_eq!((*nu, *nv), (3, 3)); // res 2 → 3×3 grid
                assert_eq!(pts.len(), 9);
                // (0,0) → z = 0; opposite corner (2,2) → z = x+y = 4 (2-var expr)
                assert!((pts[0] - vec3(0.0, 0.0, 0.0)).length() < 1e-3);
                assert!((pts[8] - vec3(2.0, 2.0, 4.0)).length() < 1e-3);
            }
            other => panic!("expected Surface, got {other:?}"),
        }
    }

    #[test]
    fn heightmap3_lifts_a_grid_by_cell_value() {
        // a 3×2 grid with two walls; heightmap3 raises walls (h=1) by 2, others 0.
        let src = r##"
            canvas(1280, 720);
            camera3((8, -10, 6), (0, 0, 0), 45);
            grid(g, "# . . ; . . #", (640, 360), 3, 2, 40);
            heightmap3(land, g, "h*2", 4);
        "##;
        let (base, _tl) = crate::parse(src).unwrap().finalize();
        match &base.get_3d("land").unwrap().shape {
            Shape3D::Surface { pts, nu, nv } => {
                assert_eq!((*nu, *nv), (3, 2)); // cols × rows
                assert_eq!(pts.len(), 6);
                assert!((pts[0].z - 2.0).abs() < 1e-3, "wall cell (0,0) lifts to 2");
                assert!(pts[1].z.abs() < 1e-3, "open cell (0,1) stays flat");
                assert!((pts[5].z - 2.0).abs() < 1e-3, "wall cell (1,2) lifts to 2");
            }
            other => panic!("expected Surface, got {other:?}"),
        }
    }

    #[test]
    fn expr_third_variable_h_is_independent() {
        use crate::kits::math::expr;
        let f = expr::compile("x + 10*y + 100*h").unwrap();
        assert!((f.eval3(1.0, 2.0, 3.0) - 321.0).abs() < 1e-4);
        // two-variable eval ignores the third (h defaults to 0)
        assert!((f.eval(1.0, 2.0) - 21.0).abs() < 1e-4);
    }

    #[test]
    fn prism3_and_pyramid3_build_meshes() {
        let (base, _) = crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\nprism3(pr,(0,0,0),6,1,2);\npyramid3(py,(3,0,0),4,1,2);",
        )
        .unwrap()
        .finalize();
        match &base.get_3d("pr").unwrap().shape {
            Shape3D::Mesh {
                verts,
                edges,
                faces,
            } => {
                assert_eq!(verts.len(), 12); // 2 × 6-gon rings
                assert_eq!(edges.len(), 18); // base + top + verticals
                assert_eq!(faces.len(), 20); // 2×6 sides + 2×(6-2) caps
            }
            o => panic!("prism: {o:?}"),
        }
        match &base.get_3d("py").unwrap().shape {
            Shape3D::Mesh {
                verts,
                edges,
                faces,
            } => {
                assert_eq!(verts.len(), 5); // 4 base + apex
                assert_eq!(edges.len(), 8); // 4 base + 4 slants
                assert_eq!(faces.len(), 6); // 4 sides + (4-2) base-cap
            }
            o => panic!("pyramid: {o:?}"),
        }
    }

    #[test]
    fn revolve3_meshes_a_surface_of_revolution() {
        // constant radius 1 over t∈[0,2], 8 sides → a tube; verify the grid + a point
        let (base, _) = crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\nrevolve3(vase, (0,0,0), \"1\", (0,2), 8);",
        )
        .unwrap()
        .finalize();
        match &base.get_3d("vase").unwrap().shape {
            Shape3D::Surface { pts, nu, nv } => {
                assert_eq!((*nu, *nv), (9, 49)); // sides+1, AXIAL+1
                assert_eq!(pts.len(), 9 * 49);
                // axial 0 (t=0 → z = 0 - mid(1) = -1), angle 0 → (r=1, 0, -1)
                assert!((pts[0] - vec3(1.0, 0.0, -1.0)).length() < 1e-3);
            }
            o => panic!("expected Surface, got {o:?}"),
        }
    }

    #[test]
    fn prism3_edge_cases() {
        let good = "camera3((5,-5,4),(0,0,0),45);\n";
        // fewer than 3 sides is rejected
        assert!(crate::parse(&format!("{good}prism3(p,(0,0,0),2,1,2);")).is_err());
        // non-integer sides round (5.6 → 6)
        let (b, _) = crate::parse(&format!("{good}prism3(p,(0,0,0),5.6,1,2);"))
            .unwrap()
            .finalize();
        match &b.get_3d("p").unwrap().shape {
            Shape3D::Mesh { verts, .. } => assert_eq!(verts.len(), 12),
            o => panic!("{o:?}"),
        }
        // absurd side counts are capped (no runaway / panic)
        let (b, _) = crate::parse(&format!("{good}prism3(p,(0,0,0),9999,1,2);"))
            .unwrap()
            .finalize();
        match &b.get_3d("p").unwrap().shape {
            Shape3D::Mesh { verts, .. } => assert_eq!(verts.len(), 512), // capped 256 × 2
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn extrude3_caps_a_rect_and_hides_its_source() {
        let (base, _) = crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\nrect(pl,(0,0),2,2);\nextrude3(sol,pl,1.0,(0,0,0));",
        )
        .unwrap()
        .finalize();
        // the 2D source is consumed (it was only the cross-section recipe)
        assert_eq!(base.get("pl").unwrap().opacity, 0.0);
        match &base.get_3d("sol").unwrap().shape {
            Shape3D::Mesh {
                verts,
                faces,
                edges,
            } => {
                // rect ring → 2 cap tris ×2 ends + 4 wall edges ×2 = 12 tris
                assert_eq!(faces.len(), 12);
                assert_eq!(verts.len(), 36);
                assert!(edges.is_empty()); // faces present → no wireframe fallback
            }
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn extrude3_of_a_difference_punches_a_hole() {
        // A boolean Region as the source ⇒ a CSG solid (plate minus a hole).
        let (base, _) = crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\nrect(pl,(0,0),3,3);\ncircle(hole,(0,0),0.8);\n\
             difference(cut,pl,hole);\nextrude3(sol,cut,1.0);",
        )
        .unwrap()
        .finalize();
        assert_eq!(base.get("cut").unwrap().opacity, 0.0); // region consumed
        match &base.get_3d("sol").unwrap().shape {
            // an outer square + an inner circular ring → far more tris than a box
            Shape3D::Mesh { faces, .. } => assert!(faces.len() > 40, "faces={}", faces.len()),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn thick_applies_to_lines_and_arrows() {
        let (base, _) = crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\narrow3(v,(0,0,0),(1,1,1));\nthick(v,0.05);",
        )
        .unwrap()
        .finalize();
        assert_eq!(base.get_3d("v").unwrap().thickness, 0.05);
    }

    #[test]
    fn twod_only_modifier_on_3d_gives_a_clear_error() {
        // `hue` (and other 2D-only modifiers) must not report "no entity named"
        // on a 3D entity — they should say they're 2D-only.
        let err = match crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\ncube3(c,(0,0,0),(2,2,2));\nhue(c,200);",
        ) {
            Err(e) => e,
            Ok(_) => panic!("expected hue-on-3D to error"),
        };
        assert!(err.msg.contains("2D-only"), "got: {}", err.msg);
        assert!(!err.msg.contains("no entity"), "got: {}", err.msg);
    }

    #[test]
    fn stroke_on_a_3d_entity_redirects_to_thick() {
        // `stroke` is 2D-only; on a 3D arrow it must not report "no entity" —
        // it should point the author at `thick` instead.
        let err = match crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\narrow3(v,(0,0,0),(1,1,1));\nstroke(v,3);",
        ) {
            Err(e) => e,
            Ok(_) => panic!("expected stroke-on-3D to error"),
        };
        assert!(err.msg.contains("thick"), "got: {}", err.msg);
        assert!(!err.msg.contains("no entity"), "got: {}", err.msg);
    }

    #[test]
    fn morph3_curves_set_up_a_path_morph() {
        let (base, _) = crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\n\
             curve3(shp,\"cos(t)\",\"sin(t)\",\"t*0.2\",(0,6));\n\
             curve3(tgt,\"cos(t)\",\"sin(t)\",\"0\",(0,6));\n\
             morph3(shp,tgt);",
        )
        .unwrap()
        .finalize();
        let e = base.get_3d("shp").unwrap();
        assert!(e.morph3.is_some());
        assert!(matches!(e.shape, Shape3D::Path { .. }));
    }

    #[test]
    fn morph3_solids_reparameterise_to_a_common_grid() {
        // A cube and a sphere have different topology; morph3 samples both to a
        // shared spherical grid so their point sets line up.
        let (base, _) = crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\n\
             cube3(shp,(0,0,0),(2,2,2));\n\
             sphere3(tgt,(0,0,0),1);\n\
             morph3(shp,tgt);",
        )
        .unwrap()
        .finalize();
        let e = base.get_3d("shp").unwrap();
        assert!(matches!(e.shape, Shape3D::Surface { .. }));
        let m = e.morph3.as_ref().expect("morph set");
        assert_eq!(m.from.len(), m.to.len());
        assert!(m.from.len() > 100);
    }

    #[test]
    fn morph3_rejects_mismatched_families() {
        // a curve can't morph into a solid
        assert!(crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\n\
             cube3(shp,(0,0,0),(2,2,2));\n\
             curve3(tgt,\"t\",\"t\",\"t\",(0,1));\n\
             morph3(shp,tgt);"
        )
        .is_err());
    }

    #[test]
    fn extrude3_rejects_zero_height_and_arealess_sources() {
        let good = "camera3((5,-5,4),(0,0,0),45);\nrect(pl,(0,0),2,2);\n";
        assert!(crate::parse(&format!("{good}extrude3(s,pl,0);")).is_err());
        // a line has no fillable area to extrude
        assert!(crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\nline(seg,(0,0),(1,1));\nextrude3(s,seg,1);"
        )
        .is_err());
    }
}
