//! The 3D kit: Z-up primitives, an orbit camera, and deterministic 3D verbs.

use macroquad::prelude::{vec2, vec3, Vec2, Vec3};

use crate::easing::Easing;
use crate::lang::diag::Error;
use crate::lang::lower::{resolve_color, resolve_easing, Args, Registry};
use crate::movie::CAMERA3_ID;
use crate::primitives::{Entity, Shape};
use crate::primitives3d::{Entity3D, Morph3, Morph3Kind, Projection3D, Shape3D, SurfaceFn};
use crate::scene::{Pin3, Pin3Target, Scene};
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TrackSpec, Value};

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
    let rotation = vec3(
        d.y.atan2(d.x).to_degrees(),
        d.z.atan2(d.x.hypot(d.y)).to_degrees(),
        0.0,
    );
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
        ("x", vec3(1.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0), style::CYAN, vec2(0.0, 20.0)),
        ("y", vec3(0.0, 1.0, 0.0), vec3(1.0, 0.0, 0.0), style::MAGENTA, vec2(0.0, -20.0)),
        ("z", vec3(0.0, 0.0, 1.0), vec3(1.0, 0.0, 0.0), style::LIME, vec2(22.0, 0.0)),
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
                Shape3D::Line { to: tickdir * (tick * 2.0) },
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
        expr::compile(&src)
            .map_err(|m| Error::new(format!("in curve3 formula: {m}"), a.span_of(i)))
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
    let (t0, t1) = { let p = a.pair(3)?; (p.x, p.y) };
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
        Shape3D::Mesh { verts, edges, faces },
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
        Shape3D::Mesh { verts, edges, faces },
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
    let (x0, x1) = { let p = a.pair(2)?; (p.x, p.y) };
    let (y0, y1) = { let p = a.pair(3)?; (p.x, p.y) };
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
        Shape3D::Surface { pts, nu: n as u32, nv: n as u32 },
        Vec3::ZERO,
        style::CYAN,
    );
    // remember z(x,y) + domain so gradient3/tangentplane3/volume3 can query it
    e.surf = Some(SurfaceFn { f, x0, x1, y0, y1 });
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
    let mut e = Entity3D::new(id.clone(), Shape3D::Arrow { to: dir }, surf.point(x, y), color);
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
        Shape3D::Surface { pts, nu: m as u32, nv: m as u32 },
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
        expr::compile(&src)
            .map_err(|m| Error::new(format!("in param3 formula: {m}"), a.span_of(i)))
    };
    let (fx, fy, fz) = (compile(1)?, compile(2)?, compile(3)?);
    let (u0, u1) = { let p = a.pair(4)?; (p.x, p.y) };
    let (v0, v1) = { let p = a.pair(5)?; (p.x, p.y) };
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
        Shape3D::Surface { pts, nu: n as u32, nv: n as u32 },
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
fn build_morph3(sa: Shape3D, sb: Shape3D) -> Result<(Vec<Vec3>, Vec<Vec3>, Morph3Kind), String> {
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
            Shape3D::Surface { pts: pa, nu: ua, nv: va },
            Shape3D::Surface { pts: pb, nu: ub, nv: vb },
        ) => Ok((
            resample_surface(&pa, ua, va, RU, RV),
            resample_surface(&pb, ub, vb, RU, RV),
            Morph3Kind::Surface { nu: RU, nv: RV },
        )),
        (sa, sb) if is_solid3(&sa) && is_solid3(&sb) => {
            let ta = crate::render3d::shape_tris(&sa)
                .ok_or("this solid has no surface to morph")?;
            let tb = crate::render3d::shape_tris(&sb)
                .ok_or("this solid has no surface to morph")?;
            Ok((
                spherical_grid(&ta, GU, GV),
                spherical_grid(&tb, GU, GV),
                Morph3Kind::Surface { nu: (GU + 1) as u32, nv: (GV + 1) as u32 },
            ))
        }
        _ => Err("morph3 needs two shapes of the same family: both curves (curve3), \
                  both surfaces (surface3/revolve3), or both solids \
                  (cube3/sphere3/prism3/pyramid3/extrude3)"
            .into()),
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
        Morph3Kind::Path => Shape3D::Path { points: from.clone() },
        Morph3Kind::Surface { nu, nv } => Shape3D::Surface { pts: from.clone(), nu, nv },
    };
    let e = s.get_3d_mut(&ida).unwrap();
    e.shape = start;
    e.morph3 = Some(Morph3 { from, to, kind, spin });
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
    let (e, id) = require_3d(s, a)?;
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
        .get_3d(CAMERA3_ID)
        .ok_or_else(|| Error::new("`orbit3` needs `camera3(...)`", a.name_span))?;
    let rot = vec3(a.num(0)?, a.num(1)?, cam.rotation.z);
    let radius = a.num(2)?.max(0.01);
    let (dur, easing) = timing(a, 3, 1.2)?;
    Ok(Clip {
        dur,
        tracks: vec![
            track(
                CAMERA3_ID,
                Prop::Rot3,
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

pub fn register(r: &mut Registry) {
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
    r.verb("look3", v_look);
    r.ctor("pin3", c_pin);
    r.ctor("follow3", c_follow3);
    r.ctor("midpoint3", c_midpoint3);
    r.ctor("curve3", c_curve3);
    r.ctor("surface3", c_surface3);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_partials_match_calculus() {
        // f(x,y) = x^2 + y^2 : fx = 2x, fy = 2y  (a paraboloid bowl)
        let f = crate::kits::math::expr::compile("x*x + y*y").unwrap();
        let sf = SurfaceFn { f, x0: -3.0, x1: 3.0, y0: -3.0, y1: 3.0 };
        assert!((sf.dx(1.0, 0.5) - 2.0).abs() < 1e-2, "fx = {}", sf.dx(1.0, 0.5));
        assert!((sf.dy(0.5, 1.5) - 3.0).abs() < 1e-2, "fy = {}", sf.dy(0.5, 1.5));
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
    fn prism3_and_pyramid3_build_meshes() {
        let (base, _) = crate::parse(
            "camera3((5,-5,4),(0,0,0),45);\nprism3(pr,(0,0,0),6,1,2);\npyramid3(py,(3,0,0),4,1,2);",
        )
        .unwrap()
        .finalize();
        match &base.get_3d("pr").unwrap().shape {
            Shape3D::Mesh { verts, edges, faces } => {
                assert_eq!(verts.len(), 12); // 2 × 6-gon rings
                assert_eq!(edges.len(), 18); // base + top + verticals
                assert_eq!(faces.len(), 20); // 2×6 sides + 2×(6-2) caps
            }
            o => panic!("prism: {o:?}"),
        }
        match &base.get_3d("py").unwrap().shape {
            Shape3D::Mesh { verts, edges, faces } => {
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
            Shape3D::Mesh { verts, faces, edges } => {
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
