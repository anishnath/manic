//! Depth-tested 3D render pass, composited beneath Manic's 2D entities.

use macroquad::prelude::*;

use crate::movie::CAMERA3_ID;
use crate::primitives3d::{Entity3D, Projection3D, Shape3D};
use crate::scene::Scene;
use crate::style;

/// Macroquad compensates for render-target Y orientation in `Camera2D`, but
/// not in `Camera3D`. Manic always renders offscreen, so apply the same
/// clip-space correction here before the texture is composited or captured.
pub struct TargetCamera3D {
    inner: Camera3D,
}

fn target_matrix(matrix: Mat4) -> Mat4 {
    Mat4::from_scale(vec3(1.0, -1.0, 1.0)) * matrix
}

impl Camera for TargetCamera3D {
    fn matrix(&self) -> Mat4 {
        target_matrix(self.inner.matrix())
    }

    fn depth_enabled(&self) -> bool {
        true
    }

    fn render_pass(&self) -> Option<RenderPass> {
        self.inner
            .render_target
            .as_ref()
            .map(|target| target.render_pass.clone())
    }

    fn viewport(&self) -> Option<(i32, i32, i32, i32)> {
        self.inner.viewport
    }
}

/// Eye position of the orbit-camera rig (azimuth/elevation/radius packed into
/// its `rotation`/`scale`). Shared by the render camera and `project`.
fn eye_of(rig: &Entity3D) -> Vec3 {
    let az = rig.rotation.x.to_radians();
    let el = rig.rotation.y.to_radians();
    let radius = rig.scale.max(0.01);
    let flat = el.cos() * radius;
    rig.pos + vec3(flat * az.cos(), flat * az.sin(), radius * el.sin())
}

/// A continuous orbit-camera frame with explicit roll around the view axis.
///
/// Building this from `forward × world_up` requires a discrete fallback at a
/// pole. That fallback produced a visible snap as an orbit crossed its cutoff.
/// The spherical azimuth/elevation tangents below are the same Z-up frame away
/// from a pole, but they also have a deterministic finite limit at the pole.
fn up_of(rig: &Entity3D, eye: Vec3) -> Vec3 {
    let forward = (rig.pos - eye).normalize_or_zero();
    let az = rig.rotation.x.to_radians();
    let el = rig.rotation.y.to_radians();
    let right = vec3(-az.sin(), az.cos(), 0.0);
    let base_up = vec3(-az.cos() * el.sin(), -az.sin() * el.sin(), el.cos());
    let (sn, cs) = rig.rotation.z.to_radians().sin_cos();
    let up = (base_up * cs + right * sn).normalize_or_zero();
    debug_assert!(up.dot(forward).abs() < 1e-3);
    up
}

/// The world→clip matrix the 3D pass uses (render-target Y-flip included), or
/// `None` when there's no camera. Screen X/Y derived from this are independent
/// of the near/far planes, so those are fixed constants here.
pub fn view_proj(scene: &Scene, aspect: f32) -> Option<Mat4> {
    let rig = scene.get_3d(CAMERA3_ID)?;
    let Shape3D::Camera { fov, projection } = rig.shape else {
        return None;
    };
    let eye = eye_of(rig);
    let view = Mat4::look_at_rh(eye, rig.pos, up_of(rig, eye));
    let proj = match projection {
        Projection3D::Perspective => {
            Mat4::perspective_rh_gl(fov.to_radians(), aspect, 0.01, 1000.0)
        }
        Projection3D::Orthographic => {
            let hh = fov.max(0.01) / 2.0;
            let hw = hh * aspect;
            Mat4::orthographic_rh_gl(-hw, hw, -hh, hh, 0.01, 1000.0)
        }
    };
    Some(target_matrix(proj * view))
}

/// Project a world point to render-target pixels (top-left origin, y-down) so a
/// 2D overlay lands exactly where the 3D pass drew it. `None` if behind the
/// camera. Used by `pin3`.
pub fn project(scene: &Scene, aspect: f32, world: Vec3, pw: f32, ph: f32) -> Option<Vec2> {
    let clip = view_proj(scene, aspect)? * world.extend(1.0);
    if clip.w <= 1e-6 {
        return None; // behind / on the camera plane
    }
    let ndc = clip.truncate() / clip.w; // (-1..1)
    // `target_matrix` already applied the 3D pass's Y-flip, so +ndc.y maps to
    // increasing pixel-Y here. Sign visually confirmed correct (pin3 labels +
    // axis numbers land on their points and track the orbit) — do not flip.
    Some(vec2(
        (ndc.x * 0.5 + 0.5) * pw,
        (ndc.y * 0.5 + 0.5) * ph,
    ))
}

/// Build Macroquad's camera from the deterministic camera entity in `scene`.
pub fn camera(scene: &Scene, target: RenderTarget, aspect: f32) -> Option<TargetCamera3D> {
    let rig = scene.get_3d(CAMERA3_ID)?;
    let Shape3D::Camera { fov, projection } = rig.shape else {
        return None;
    };
    let eye = eye_of(rig);
    let up = up_of(rig, eye);
    let projection = match projection {
        Projection3D::Perspective => Projection::Perspective,
        Projection3D::Orthographic => Projection::Orthographics,
    };
    Some(TargetCamera3D {
        inner: Camera3D {
            position: eye,
            target: rig.pos,
            up,
            // Macroquad interprets this as radians for perspective, but as the
            // visible world height for orthographic projection.
            fovy: match projection {
                Projection::Perspective => fov.to_radians(),
                Projection::Orthographics => fov,
            },
            aspect: Some(aspect),
            projection,
            render_target: Some(target),
            ..Default::default()
        },
    })
}

fn entity_matrix(e: &Entity3D) -> Mat4 {
    let r = vec3(
        e.rotation.x.to_radians(),
        e.rotation.y.to_radians(),
        e.rotation.z.to_radians(),
    );
    Mat4::from_scale_rotation_translation(
        Vec3::splat(e.scale),
        Quat::from_euler(EulerRot::ZYX, r.z, r.y, r.x),
        e.pos,
    )
}

fn draw_arrow_head(tip: Vec3, dir: Vec3, color: Color) {
    let len = dir.length();
    if len <= 1e-5 {
        return;
    }
    let d = dir / len;
    let helper = if d.z.abs() < 0.9 { Vec3::Z } else { Vec3::Y };
    let side = d.cross(helper).normalize_or_zero() * (len * 0.09).clamp(0.08, 0.3);
    let up = d.cross(side).normalize_or_zero() * side.length();
    let base = tip - d * (len * 0.22).clamp(0.18, 0.7);
    for p in [base + side, base - side, base + up, base - up] {
        draw_line_3d(tip, p, color);
    }
}

/// Multiply a colour's alpha by the draw-on `trace` (0..1).
fn faded(base: Color, trace: f32) -> Color {
    Color::new(base.r, base.g, base.b, base.a * trace.clamp(0.0, 1.0))
}

/// Triangles of a row-major `nu`×`nv` surface grid (`surface3`/`revolve3`).
pub(crate) fn surface_grid_tris(pts: &[Vec3], nu: u32, nv: u32) -> Vec<[Vec3; 3]> {
    let (nu, nv) = (nu as usize, nv as usize);
    if nu < 2 || nv < 2 || pts.len() != nu * nv {
        return Vec::new();
    }
    let mut tris = Vec::with_capacity((nu - 1) * (nv - 1) * 2);
    for v in 0..nv - 1 {
        for u in 0..nu - 1 {
            let a = pts[v * nu + u];
            let b = pts[v * nu + u + 1];
            let d = pts[(v + 1) * nu + u];
            let c = pts[(v + 1) * nu + u + 1];
            tris.push([a, b, c]);
            tris.push([a, c, d]);
        }
    }
    tris
}

/// Surface triangles of a solid shape in its local frame, for morph sampling
/// (`morph3`). `None` for shapes that aren't a closed/filled solid.
pub(crate) fn shape_tris(shape: &Shape3D) -> Option<Vec<[Vec3; 3]>> {
    match shape {
        Shape3D::Cube { size } => Some(box_tris(*size)),
        Shape3D::Sphere { radius } => Some(sphere_tris(*radius)),
        Shape3D::Surface { pts, nu, nv } => Some(surface_grid_tris(pts, *nu, *nv)),
        Shape3D::Mesh { verts, faces, .. } if !faces.is_empty() => Some(
            faces
                .iter()
                .filter_map(|f| {
                    Some([
                        *verts.get(f[0] as usize)?,
                        *verts.get(f[1] as usize)?,
                        *verts.get(f[2] as usize)?,
                    ])
                })
                .collect(),
        ),
        _ => None,
    }
}

/// Triangles of an axis-aligned box of `size`, centred on the origin (6 faces,
/// 2 tris each). Per-face flat normals give the cube crisp shaded facets.
pub(crate) fn box_tris(size: Vec3) -> Vec<[Vec3; 3]> {
    let h = size * 0.5;
    let v = |x: f32, y: f32, z: f32| vec3(x * h.x, y * h.y, z * h.z);
    let quad = |a, b, c, d| [[a, b, c], [a, c, d]];
    [
        quad(v(1., -1., -1.), v(1., 1., -1.), v(1., 1., 1.), v(1., -1., 1.)), // +X
        quad(v(-1., -1., -1.), v(-1., -1., 1.), v(-1., 1., 1.), v(-1., 1., -1.)), // -X
        quad(v(-1., 1., -1.), v(-1., 1., 1.), v(1., 1., 1.), v(1., 1., -1.)), // +Y
        quad(v(-1., -1., -1.), v(1., -1., -1.), v(1., -1., 1.), v(-1., -1., 1.)), // -Y
        quad(v(-1., -1., 1.), v(1., -1., 1.), v(1., 1., 1.), v(-1., 1., 1.)), // +Z
        quad(v(-1., -1., -1.), v(-1., 1., -1.), v(1., 1., -1.), v(1., -1., -1.)), // -Z
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Triangles of a Z-up UV sphere of `radius` centred on the origin.
pub(crate) fn sphere_tris(radius: f32) -> Vec<[Vec3; 3]> {
    const RINGS: usize = 16;
    const SECTORS: usize = 24;
    let mut pts = Vec::with_capacity((RINGS + 1) * (SECTORS + 1));
    for i in 0..=RINGS {
        let phi = std::f32::consts::PI * i as f32 / RINGS as f32; // 0..π from +Z
        let (sp, cp) = phi.sin_cos();
        for j in 0..=SECTORS {
            let theta = std::f32::consts::TAU * j as f32 / SECTORS as f32;
            let (st, ct) = theta.sin_cos();
            pts.push(radius * vec3(sp * ct, sp * st, cp));
        }
    }
    let stride = SECTORS + 1;
    let mut tris = Vec::with_capacity(RINGS * SECTORS * 2);
    for i in 0..RINGS {
        for j in 0..SECTORS {
            let a = i * stride + j;
            let (b, c, d) = (a + 1, a + stride, a + stride + 1);
            tris.push([pts[a], pts[c], pts[b]]);
            tris.push([pts[b], pts[c], pts[d]]);
        }
    }
    tris
}

/// Triangles of a tube of `radius` swept along `path`, with `sides` faces
/// around the cross-section. Uses a rotation-minimising (parallel-transport)
/// frame so the ring doesn't twist along the curve. Ends are left open.
fn tube_tris(path: &[Vec3], radius: f32, sides: usize) -> Vec<[Vec3; 3]> {
    let n = path.len();
    if n < 2 || radius <= 0.0 || sides < 3 {
        return Vec::new();
    }
    // Per-point tangents (central difference in the interior).
    let mut tan = Vec::with_capacity(n);
    for i in 0..n {
        let t = if i == 0 {
            path[1] - path[0]
        } else if i == n - 1 {
            path[n - 1] - path[n - 2]
        } else {
            path[i + 1] - path[i - 1]
        };
        tan.push(t.normalize_or_zero());
    }
    // Seed a normal perpendicular to the first tangent, then parallel-transport.
    let mut normal = {
        let helper = if tan[0].z.abs() < 0.9 { Vec3::Z } else { Vec3::Y };
        tan[0].cross(helper).normalize_or_zero()
    };
    let mut rings: Vec<Vec<Vec3>> = Vec::with_capacity(n);
    for i in 0..n {
        if i > 0 {
            let (t0, t1) = (tan[i - 1], tan[i]);
            let axis = t0.cross(t1);
            let al = axis.length();
            if al > 1e-6 {
                let angle = t0.dot(t1).clamp(-1.0, 1.0).acos();
                normal = (Quat::from_axis_angle(axis / al, angle) * normal).normalize_or_zero();
            }
            // Re-orthogonalise against the current tangent to fight drift.
            normal = (normal - tan[i] * normal.dot(tan[i])).normalize_or_zero();
        }
        let binormal = tan[i].cross(normal).normalize_or_zero();
        let ring = (0..sides)
            .map(|k| {
                let a = std::f32::consts::TAU * k as f32 / sides as f32;
                path[i] + (normal * a.cos() + binormal * a.sin()) * radius
            })
            .collect();
        rings.push(ring);
    }
    let mut tris = Vec::with_capacity((n - 1) * sides * 2);
    for i in 0..n - 1 {
        for k in 0..sides {
            let kn = (k + 1) % sides;
            let (a, b) = (rings[i][k], rings[i][kn]);
            let (c, d) = (rings[i + 1][k], rings[i + 1][kn]);
            tris.push([a, c, b]);
            tris.push([b, c, d]);
        }
    }
    tris
}

/// Triangles of a cone from `base` (ring of `radius`) to the `tip` apex, with a
/// base cap — the solid head of a `thick` `arrow3`.
fn cone_tris(base: Vec3, tip: Vec3, radius: f32, sides: usize) -> Vec<[Vec3; 3]> {
    let axis = tip - base;
    let len = axis.length();
    if len < 1e-6 || radius <= 0.0 || sides < 3 {
        return Vec::new();
    }
    let dir = axis / len;
    let helper = if dir.z.abs() < 0.9 { Vec3::Z } else { Vec3::Y };
    let n = dir.cross(helper).normalize_or_zero();
    let b = dir.cross(n).normalize_or_zero();
    let ring: Vec<Vec3> = (0..sides)
        .map(|k| {
            let a = std::f32::consts::TAU * k as f32 / sides as f32;
            base + (n * a.cos() + b * a.sin()) * radius
        })
        .collect();
    let mut tris = Vec::with_capacity(sides * 2);
    for k in 0..sides {
        let kn = (k + 1) % sides;
        tris.push([ring[k], tip, ring[kn]]); // side
        tris.push([base, ring[kn], ring[k]]); // base cap
    }
    tris
}

/// Fill triangles with flat lambert shading baked into per-face vertex colours
/// (Macroquad has no GPU lighting). A fixed key light; `abs(n·l)` so both faces
/// are lit (no black back-faces), plus ambient. Chunked to stay under the u16
/// index limit for large meshes. When `base` is translucent, triangles are
/// drawn back-to-front (painter's order, using the model-local eye) so blending
/// is correct; opaque meshes lean on the depth buffer and skip the sort.
fn fill_tris(tris: &[[Vec3; 3]], base: Color, local_eye: Option<Vec3>) {
    if tris.is_empty() {
        return;
    }
    let light = vec3(0.4, -0.55, 0.72).normalize();

    // Draw order: far→near for translucent fills, natural order otherwise.
    let mut order: Vec<usize> = (0..tris.len()).collect();
    if base.a < 0.999 {
        if let Some(eye) = local_eye {
            let d2 = |t: &[Vec3; 3]| ((t[0] + t[1] + t[2]) / 3.0 - eye).length_squared();
            order.sort_by(|&a, &b| d2(&tris[b]).total_cmp(&d2(&tris[a])));
        }
    }

    // macroquad batches each `draw_mesh` into a shared buffer capped at ~10k
    // vertices / 5k indices per drawcall; over that it silently clamps (drops
    // triangles → holes). 1000 tris = 3000 verts / 3000 indices leaves margin.
    for chunk in order.chunks(1000) {
        let mut vertices = Vec::with_capacity(chunk.len() * 3);
        let mut indices: Vec<u16> = Vec::with_capacity(chunk.len() * 3);
        for &ti in chunk {
            let t = &tris[ti];
            let n = (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero();
            let lam = 0.35 + 0.65 * n.dot(light).abs();
            let c = Color::new(base.r * lam, base.g * lam, base.b * lam, base.a);
            let i0 = vertices.len() as u16;
            vertices.push(Vertex::new2(t[0], Vec2::ZERO, c));
            vertices.push(Vertex::new2(t[1], Vec2::ZERO, c));
            vertices.push(Vertex::new2(t[2], Vec2::ZERO, c));
            indices.push(i0);
            indices.push(i0 + 1);
            indices.push(i0 + 2);
        }
        draw_mesh(&Mesh {
            vertices,
            indices,
            texture: None,
        });
    }
}

fn draw_entity(e: &Entity3D, tpl: &style::Template, eye: Option<Vec3>) {
    if e.opacity <= 0.001 || matches!(e.shape, Shape3D::Camera { .. }) {
        return;
    }
    let base = tpl.palette.remap(e.color);
    let color = Color::new(base.r, base.g, base.b, base.a * e.opacity);
    let trace = e.trace.clamp(0.0, 1.0);
    let matrix = entity_matrix(e);
    // Eye in this entity's local frame (fill triangles live in local space, so
    // sort them there). Uniform scale preserves the depth ordering.
    let local_eye = eye.map(|w| matrix.inverse().transform_point3(w));

    // Macroquad exposes model transforms through its internal batching context.
    // Keep the unsafe access isolated to this renderer module.
    unsafe {
        macroquad::window::get_internal_gl()
            .quad_gl
            .push_model_matrix(matrix);
    }
    match &e.shape {
        Shape3D::Point { radius } => draw_sphere(Vec3::ZERO, *radius, None, color),
        Shape3D::Line { to } => {
            let end = *to * trace;
            if e.thickness > 0.0 {
                fill_tris(&tube_tris(&[Vec3::ZERO, end], e.thickness, 8), color, local_eye);
            } else {
                draw_line_3d(Vec3::ZERO, end, color);
            }
        }
        Shape3D::Arrow { to } => {
            let tip = *to * trace;
            if e.thickness > 0.0 {
                // A shaded tube shaft capped by a solid cone head. Head is
                // sized off `thickness` so it stays proportional to the shaft.
                let len = tip.length();
                if len > 1e-4 && trace > 1e-3 {
                    let dir = tip / len;
                    let head_r = (e.thickness * 2.4).max(0.03);
                    let head_len = (len * 0.3).clamp(head_r * 1.3, head_r * 2.4).min(len);
                    let base = tip - dir * head_len;
                    if head_len < len {
                        fill_tris(&tube_tris(&[Vec3::ZERO, base], e.thickness, 8), color, local_eye);
                    }
                    fill_tris(&cone_tris(base, tip, head_r, 12), color, local_eye);
                }
            } else {
                draw_line_3d(Vec3::ZERO, tip, color);
                if trace > 0.001 {
                    draw_arrow_head(tip, *to, color);
                }
            }
        }
        Shape3D::Cube { size } => {
            let size = *size * trace.max(0.001);
            fill_tris(&box_tris(size), color, local_eye);
            // A crisp edge overlay keeps the neon-diagram look over the fill.
            draw_cube_wires(Vec3::ZERO, size, color);
        }
        Shape3D::Sphere { radius } => {
            fill_tris(&sphere_tris(radius * trace.max(0.001)), color, local_eye)
        }
        Shape3D::Grid { half, spacing } => {
            let extent = *half as f32 * *spacing;
            for i in -*half..=*half {
                let p = i as f32 * *spacing;
                let c = if i == 0 {
                    color
                } else {
                    Color::new(color.r, color.g, color.b, color.a * 0.32)
                };
                draw_line_3d(vec3(p, -extent, 0.0), vec3(p, extent, 0.0), c);
                draw_line_3d(vec3(-extent, p, 0.0), vec3(extent, p, 0.0), c);
            }
        }
        Shape3D::Path { points } => {
            if points.len() >= 2 {
                let segs = points.len() - 1;
                let drawn = (trace * segs as f32).clamp(0.0, segs as f32);
                let full = drawn.floor() as usize;
                // The portion of the polyline revealed so far (draw-on `trace`),
                // ending on the interpolated point inside the current segment.
                let mut pts: Vec<Vec3> = points[..=full.min(segs)].to_vec();
                if full < segs {
                    let f = drawn - full as f32;
                    if f > 1e-3 {
                        let (a, b) = (points[full], points[full + 1]);
                        pts.push(a + (b - a) * f);
                    }
                }
                if pts.len() >= 2 {
                    if e.thickness > 0.0 {
                        fill_tris(&tube_tris(&pts, e.thickness, 8), color, local_eye);
                    } else {
                        for w in pts.windows(2) {
                            draw_line_3d(w[0], w[1], color);
                        }
                    }
                }
            }
        }
        Shape3D::Surface { pts, nu, nv } => {
            fill_tris(&surface_grid_tris(pts, *nu, *nv), faded(color, trace), local_eye);
        }
        Shape3D::Mesh { verts, edges, faces } => {
            let c = faded(color, trace);
            if !faces.is_empty() {
                let tris: Vec<[Vec3; 3]> = faces
                    .iter()
                    .filter_map(|f| {
                        Some([
                            *verts.get(f[0] as usize)?,
                            *verts.get(f[1] as usize)?,
                            *verts.get(f[2] as usize)?,
                        ])
                    })
                    .collect();
                fill_tris(&tris, c, local_eye);
            } else {
                for &(a, b) in edges {
                    if let (Some(pa), Some(pb)) = (verts.get(a as usize), verts.get(b as usize)) {
                        draw_line_3d(*pa, *pb, c);
                    }
                }
            }
        }
        Shape3D::Camera { .. } => {}
    }
    unsafe {
        macroquad::window::get_internal_gl()
            .quad_gl
            .pop_model_matrix();
    }
}

pub fn draw_scene(scene: &Scene, tpl: &style::Template) {
    let eye = scene.get_3d(CAMERA3_ID).map(eye_of);

    // Opaque geometry first (the depth buffer sorts it), then translucent
    // entities back-to-front so their blending composites correctly. `trace`
    // (draw-on) is an animation fade, not an ordering concern, so ordering keys
    // off the resolved fill alpha only.
    let alpha = |e: &Entity3D| tpl.palette.remap(e.color).a * e.opacity;
    let (opaque, mut translucent): (Vec<&Entity3D>, Vec<&Entity3D>) =
        scene.entities_3d.iter().partition(|e| alpha(e) >= 0.999);
    if let Some(eye) = eye {
        translucent
            .sort_by(|a, b| (b.pos - eye).length_squared().total_cmp(&(a.pos - eye).length_squared()));
    }
    for entity in opaque.into_iter().chain(translucent) {
        draw_entity(entity, tpl, eye);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_target_projects_to_screen_centre() {
        // The point the camera looks at must land dead-centre — independent of
        // the Y-flip, so this pins down the matrix + NDC→pixel math.
        let mut scene = Scene::new();
        let mut rig = Entity3D::new(
            CAMERA3_ID.to_string(),
            Shape3D::Camera {
                fov: 45.0,
                projection: Projection3D::Perspective,
            },
            Vec3::ZERO,
            crate::style::CYAN,
        );
        rig.rotation = vec3(45.0, 30.0, 0.0); // azimuth, elevation
        rig.scale = 12.0; // radius
        scene.add_3d(rig);
        let px = project(&scene, 16.0 / 9.0, Vec3::ZERO, 1920.0, 1080.0).unwrap();
        assert!((px.x - 960.0).abs() < 0.5, "x={}", px.x);
        assert!((px.y - 540.0).abs() < 0.5, "y={}", px.y);
    }

    #[test]
    fn overhead_camera_has_a_finite_up_and_rolls_around_view_axis() {
        let mut rig = Entity3D::new(
            CAMERA3_ID.to_string(),
            Shape3D::Camera {
                fov: 8.0,
                projection: Projection3D::Orthographic,
            },
            Vec3::ZERO,
            crate::style::CYAN,
        );
        rig.rotation = vec3(0.0, 90.0, 0.0);
        rig.scale = 10.0;
        let eye = eye_of(&rig);
        let forward = (rig.pos - eye).normalize();
        let up0 = up_of(&rig, eye);
        assert!(up0.is_finite());
        assert!(up0.dot(forward).abs() < 1e-5);

        rig.rotation.z = -90.0;
        let up1 = up_of(&rig, eye);
        assert!(up1.is_finite());
        assert!(up1.dot(forward).abs() < 1e-5);
        assert!(up0.dot(up1).abs() < 1e-4);
    }

    #[test]
    fn orbit_up_frame_is_continuous_through_the_old_pole_cutoff() {
        let mut rig = Entity3D::new(
            CAMERA3_ID.to_string(),
            Shape3D::Camera {
                fov: 8.0,
                projection: Projection3D::Orthographic,
            },
            Vec3::ZERO,
            crate::style::CYAN,
        );
        rig.rotation = vec3(-90.0, 90.0, 0.0);
        rig.scale = 10.0;

        let mut previous = up_of(&rig, eye_of(&rig));
        for elevation in (0..=180).map(|i| 90.0 - i as f32) {
            rig.rotation.y = elevation;
            // Match the plane-flip choreography: roll changes continuously as
            // elevation passes from +90 to -90 degrees.
            rig.rotation.z = (90.0 - elevation) * 0.5;
            let current = up_of(&rig, eye_of(&rig));
            assert!(current.is_finite());
            assert!(
                previous.dot(current) > 0.998,
                "camera up snapped around elevation {elevation}"
            );
            previous = current;
        }
    }

    #[test]
    fn target_camera_flips_only_clip_y() {
        let base = Mat4::perspective_rh_gl(45.0_f32.to_radians(), 16.0 / 9.0, 0.01, 100.0)
            * Mat4::look_at_rh(vec3(6.0, -8.0, 5.0), Vec3::ZERO, Vec3::Z);
        let point = vec4(1.0, 2.0, 3.0, 1.0);
        let before = base * point;
        let after = target_matrix(base) * point;
        assert_eq!(after.x, before.x);
        assert_eq!(after.y, -before.y);
        assert_eq!(after.z, before.z);
        assert_eq!(after.w, before.w);
    }
}
