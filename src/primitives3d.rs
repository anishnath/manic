//! Drawable 3D primitives and their per-entity transform state.

use macroquad::prelude::{Color, EulerRot, Quat, Vec3};

/// Projection used by the scene's optional 3D camera rig.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Projection3D {
    Perspective,
    Orthographic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shading3 {
    Flat,
    Smooth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material3 {
    Matte,
    Metal,
    Glass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Texture3 {
    Solid,
    Checker,
    Stripes,
}

/// Compact, renderer-owned finish choices configured by one creator verb.
#[derive(Debug, Clone, Copy)]
pub struct Finish3 {
    pub shading: Shading3,
    pub material: Material3,
    pub texture: Texture3,
    pub texture_scale: f32,
    pub mesh: f32,
    pub depth: f32,
    pub shadow: f32,
}

impl Default for Finish3 {
    fn default() -> Self {
        Self {
            shading: Shading3::Flat,
            material: Material3::Matte,
            texture: Texture3::Solid,
            texture_scale: 4.0,
            mesh: 0.0,
            depth: 0.0,
            shadow: 0.0,
        }
    }
}

/// Geometry carried by a 3D entity. Segment endpoints are local to `Entity3D::pos`.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape3D {
    Point {
        radius: f32,
    },
    Line {
        to: Vec3,
    },
    Arrow {
        to: Vec3,
    },
    Cube {
        size: Vec3,
    },
    Sphere {
        radius: f32,
    },
    /// Square grid in the XY plane, centred on the entity position.
    Grid {
        half: i32,
        spacing: f32,
    },
    /// A sampled 3D polyline (`curve3`), drawn on by `trace`.
    Path {
        points: Vec<Vec3>,
    },
    /// A sampled surface mesh (`surface3`), `nu`×`nv` grid of points (row-major,
    /// index = `v*nu + u`), drawn as a wireframe.
    Surface {
        pts: Vec<Vec3>,
        nu: u32,
        nv: u32,
    },
    /// An indexed mesh (`prism3`/`pyramid3`/…): local-space vertices, plus
    /// triangle `faces` (filled + flat-shaded) and `edges` (wireframe fallback
    /// when there are no faces).
    Mesh {
        verts: Vec<Vec3>,
        edges: Vec<(u32, u32)>,
        faces: Vec<[u32; 3]>,
    },
    /// Non-drawable orbit camera. `pos` is the target, `rotation.x/y` are
    /// azimuth/elevation in degrees, and `scale` is orbit radius.
    Camera {
        fov: f32,
        projection: Projection3D,
    },
}

/// What geometry a [`Morph3`] blends into — the shape the interpolated points
/// are rebuilt as each frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Morph3Kind {
    /// A sampled polyline (`curve3`): points rebuild a `Shape3D::Path`.
    Path,
    /// A `nu`×`nv` grid (surfaces + spherically-sampled solids): points rebuild
    /// a `Shape3D::Surface`.
    Surface { nu: u32, nv: u32 },
}

/// A set-up 3D shape morph (`morph3`): two equal-length point sets sampled to a
/// common representation, blended by the `Prop::Morph` fraction `0→1`. `spin`
/// adds a winding rotation about the vertical axis over the blend.
#[derive(Debug, Clone)]
pub struct Morph3 {
    pub from: Vec<Vec3>,
    pub to: Vec<Vec3>,
    pub kind: Morph3Kind,
    pub spin: f32,
}

/// One id-addressable 3D object in a scene.
/// Recompute a 3D entity from its `deps`' positions each frame (`midpoint3`, …).
pub type DeriveFn3 = fn(&mut Entity3D, &[Vec3]);

#[derive(Debug, Clone)]
pub struct Link3 {
    pub from: String,
    pub to: String,
    pub trim: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionPlane3 {
    Xy,
    Xz,
    Yz,
}

/// A height-field surface remembered on its entity: the compiled `z(x,y)` plus
/// its domain. `surface3` fills this in so `gradient3`/`tangentplane3`/`volume3`
/// can *query the surface by id* — the 3D analog of [`crate::primitives::GraphFn`].
#[derive(Debug, Clone)]
pub struct SurfaceFn {
    pub f: crate::kits::math::expr::Node,
    pub x0: f32,
    pub x1: f32,
    pub y0: f32,
    pub y1: f32,
}

impl SurfaceFn {
    /// `z = f(x, y)`.
    pub fn z(&self, x: f32, y: f32) -> f32 {
        self.f.eval(x, y)
    }
    /// The surface point `(x, y, f(x,y))` in world coords.
    pub fn point(&self, x: f32, y: f32) -> Vec3 {
        Vec3::new(x, y, self.z(x, y))
    }
    /// Partial `∂f/∂x` by central difference.
    pub fn dx(&self, x: f32, y: f32) -> f32 {
        let h = ((self.x1 - self.x0).abs() * 1e-3).max(1e-4);
        (self.z(x + h, y) - self.z(x - h, y)) / (2.0 * h)
    }
    /// Partial `∂f/∂y` by central difference.
    pub fn dy(&self, x: f32, y: f32) -> f32 {
        let h = ((self.y1 - self.y0).abs() * 1e-3).max(1e-4);
        (self.z(x, y + h) - self.z(x, y - h)) / (2.0 * h)
    }
}

#[derive(Debug, Clone)]
pub struct Entity3D {
    pub id: String,
    pub shape: Shape3D,
    pub pos: Vec3,
    /// Euler degrees, applied in Z-Y-X order by the renderer.
    pub rotation: Vec3,
    /// Additional world-space orientation used by V2 relational turns.
    /// Existing `rotation` remains the authored Euler-compatible surface;
    /// keeping the quaternion separate avoids changing old files while giving
    /// axis turns a stable, composable interpolation.
    pub orientation: Quat,
    pub scale: f32,
    pub color: Color,
    pub opacity: f32,
    pub trace: f32,
    /// Stroke radius in world units for tube-rendered paths (`thick`); `0` =
    /// thin 1px line. Only `Shape3D::Path` (curve3) consults it today.
    pub thickness: f32,
    pub finish: Finish3,
    pub tags: Vec<String>,
    /// Track another 3D entity's position + offset each frame (`follow3`).
    pub follow: Option<(String, Vec3)>,
    /// When true, the follow offset is expressed in the target's local frame
    /// and this entity inherits its orientation (`attach3(...,"rigid")`).
    pub follow_local: bool,
    /// Child orientation relative to the followed target for a rigid attach.
    pub follow_orientation: Quat,
    /// Input entity ids for [`Entity3D::derive`].
    pub deps: Vec<String>,
    /// Recompute this entity from `deps` each frame (`midpoint3`, …).
    pub derive: Option<DeriveFn3>,
    /// A live line whose endpoints are recomputed from two 3D entities.
    pub link: Option<Link3>,
    /// Live orthogonal projection of another entity's position.
    pub projection: Option<(String, ProjectionPlane3)>,
    /// A set-up shape morph (`morph3`); the `Prop::Morph` track blends it.
    pub morph3: Option<Morph3>,
    /// If set, this is a `surface3` height field: its `z(x,y)` + domain, so
    /// `gradient3`/`tangentplane3`/`volume3` can query it by id.
    pub surf: Option<SurfaceFn>,
}

impl Entity3D {
    pub fn new(id: impl Into<String>, shape: Shape3D, pos: Vec3, color: Color) -> Self {
        Self {
            id: id.into(),
            shape,
            pos,
            rotation: Vec3::ZERO,
            orientation: Quat::IDENTITY,
            scale: 1.0,
            color,
            opacity: 1.0,
            trace: 1.0,
            thickness: 0.0,
            finish: Finish3::default(),
            tags: Vec::new(),
            follow: None,
            follow_local: false,
            follow_orientation: Quat::IDENTITY,
            deps: Vec::new(),
            derive: None,
            link: None,
            projection: None,
            morph3: None,
            surf: None,
        }
    }

    /// Complete local→world orientation. V2's world-space orientation is
    /// pre-multiplied so a group turn rotates an already-oriented object as one
    /// rigid system rather than rewriting its creator-authored Euler angles.
    pub fn rotation_quat(&self) -> Quat {
        let r = Vec3::new(
            self.rotation.x.to_radians(),
            self.rotation.y.to_radians(),
            self.rotation.z.to_radians(),
        );
        self.orientation * Quat::from_euler(EulerRot::ZYX, r.z, r.y, r.x)
    }

    pub fn world_point(&self, local: Vec3) -> Vec3 {
        self.pos + self.rotation_quat() * (local * self.scale)
    }

    /// Axis-aligned world bounds after the entity's complete authored
    /// transform. Camera rigs deliberately have no drawable bounds.
    pub fn world_bounds(&self) -> Option<(Vec3, Vec3)> {
        let radius_bounds = |radius: f32| {
            let r = Vec3::splat(radius * self.scale.abs());
            (self.pos - r, self.pos + r)
        };
        if let Shape3D::Point { radius } | Shape3D::Sphere { radius } = self.shape {
            return Some(radius_bounds(radius));
        }

        let mut local = Vec::new();
        match &self.shape {
            Shape3D::Line { to } | Shape3D::Arrow { to } => {
                local.extend([Vec3::ZERO, *to]);
            }
            Shape3D::Cube { size } => {
                let h = *size * 0.5;
                for x in [-h.x, h.x] {
                    for y in [-h.y, h.y] {
                        for z in [-h.z, h.z] {
                            local.push(Vec3::new(x, y, z));
                        }
                    }
                }
            }
            Shape3D::Grid { half, spacing } => {
                let h = *half as f32 * *spacing;
                local.extend([
                    Vec3::new(-h, -h, 0.0),
                    Vec3::new(h, -h, 0.0),
                    Vec3::new(h, h, 0.0),
                    Vec3::new(-h, h, 0.0),
                ]);
            }
            Shape3D::Path { points } => local.extend(points.iter().copied()),
            Shape3D::Surface { pts, .. } => local.extend(pts.iter().copied()),
            Shape3D::Mesh { verts, .. } => local.extend(verts.iter().copied()),
            Shape3D::Point { .. } | Shape3D::Sphere { .. } => unreachable!(),
            Shape3D::Camera { .. } => return None,
        }
        let mut points = local.into_iter().map(|point| self.world_point(point));
        let first = points.next()?;
        let mut lo = first;
        let mut hi = first;
        for point in points {
            lo = Vec3::new(lo.x.min(point.x), lo.y.min(point.y), lo.z.min(point.z));
            hi = Vec3::new(hi.x.max(point.x), hi.y.max(point.y), hi.z.max(point.z));
        }
        Some((lo, hi))
    }
}
