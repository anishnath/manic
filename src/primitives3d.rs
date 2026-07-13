//! Drawable 3D primitives and their per-entity transform state.

use macroquad::prelude::{Color, Vec3};

/// Projection used by the scene's optional 3D camera rig.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Projection3D {
    Perspective,
    Orthographic,
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
pub struct Entity3D {
    pub id: String,
    pub shape: Shape3D,
    pub pos: Vec3,
    /// Euler degrees, applied in Z-Y-X order by the renderer.
    pub rotation: Vec3,
    pub scale: f32,
    pub color: Color,
    pub opacity: f32,
    pub trace: f32,
    /// Stroke radius in world units for tube-rendered paths (`thick`); `0` =
    /// thin 1px line. Only `Shape3D::Path` (curve3) consults it today.
    pub thickness: f32,
    pub tags: Vec<String>,
    /// Track another 3D entity's position + offset each frame (`follow3`).
    pub follow: Option<(String, Vec3)>,
    /// Input entity ids for [`Entity3D::derive`].
    pub deps: Vec<String>,
    /// Recompute this entity from `deps` each frame (`midpoint3`, …).
    pub derive: Option<DeriveFn3>,
    /// A set-up shape morph (`morph3`); the `Prop::Morph` track blends it.
    pub morph3: Option<Morph3>,
}

impl Entity3D {
    pub fn new(id: impl Into<String>, shape: Shape3D, pos: Vec3, color: Color) -> Self {
        Self {
            id: id.into(),
            shape,
            pos,
            rotation: Vec3::ZERO,
            scale: 1.0,
            color,
            opacity: 1.0,
            trace: 1.0,
            thickness: 0.0,
            tags: Vec::new(),
            follow: None,
            deps: Vec::new(),
            derive: None,
            morph3: None,
        }
    }
}
