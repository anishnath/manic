//! Drawable primitives. One entity type; its look is data (`Shape`).
//! New primitive = new `Shape` variant + match arm in `render::draw_entity`.

use macroquad::prelude::{Color, Vec2};

/// What an [`Entity`] looks like. Positions inside a shape (e.g. `to`,
/// polygon points) are in absolute scene coordinates; `Entity::pos` is added
/// as an offset for polygons and is the anchor/centre for everything else.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// Circle centred on `pos`.
    Circle { r: f32 },
    /// Rectangle centred on `pos`.
    Rect { w: f32, h: f32 },
    /// Line from `pos` to `to` (absolute).
    Line { to: Vec2 },
    /// Arrow from `pos` to `to` (absolute), with a triangular head at `to`.
    Arrow { to: Vec2 },
    /// Quadratic bézier from `pos` to `to` bending through `ctrl`;
    /// `arrow` adds a head at `to`.
    Curve { ctrl: Vec2, to: Vec2, arrow: bool },
    /// Filled/outlined polygon. Points are absolute; `pos` is added as an
    /// offset so the whole polygon can be moved by animating `pos`.
    Polygon { pts: Vec<Vec2> },
    /// Open polyline through absolute points (offset by `pos`), stroked only.
    /// The backbone of function plots and sampled curves; supports draw-on
    /// via `trace`.
    Polyline { pts: Vec<Vec2> },
    /// Circular arc / sector / annulus centred on `pos`. `start`/`sweep` are
    /// in degrees; `inner` is the inner radius (0 = solid disc/sector). With
    /// fill on it's a sector (or annular sector / annulus when `inner > 0`);
    /// with fill off it's a plain arc line. One primitive covers Manim's Arc,
    /// Sector, Annulus, and AnnularSector.
    Arc {
        r: f32,
        inner: f32,
        start: f32,
        sweep: f32,
    },
    /// A baked boolean-op result: `tris` fill it (from triangulation), `rings`
    /// are its outline loops (exterior + holes). Points are absolute (offset
    /// by `pos`). Produced by `crate::geom` for union/intersection/etc.
    Region {
        tris: Vec<[Vec2; 3]>,
        rings: Vec<Vec<Vec2>>,
    },
    /// Text anchored on `pos`.
    Text { content: String, size: f32 },
}

/// Horizontal anchoring for [`Shape::Text`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// Centred on `pos` (labels, captions).
    #[default]
    Center,
    /// Starts at `pos` (code blocks, typewriter reveals).
    Left,
}

/// Which font family to render a [`Shape::Text`] with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontKind {
    /// Heavy display mono — headlines, section banners.
    Display,
    /// Monospace regular (IBM Plex Mono) — labels, captions, data.
    #[default]
    Mono,
    /// Bold monospace — emphasised labels.
    MonoBold,
}

/// Fill/outline styling. The neon look leans on glowing outlines over dark
/// fills, so both fill and outline can be on at once, with an independent
/// outline color.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeStyle {
    /// Fill the shape with `Entity::color`.
    pub fill: bool,
    /// Draw the outline.
    pub outline: bool,
    /// Outline thickness in pixels.
    pub width: f32,
    /// Outline color override. `None` = use `Entity::color`.
    pub outline_color: Option<Color>,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        StrokeStyle {
            fill: true,
            outline: false,
            width: 2.5,
            outline_color: None,
        }
    }
}

/// Makes a line/arrow's endpoints track two other entities: `pos` follows
/// `from`, the shape's `to` follows `to`, each trimmed inward by `trim` px
/// (so it meets node borders). Resolved every frame in
/// [`crate::timeline::Timeline::apply`], so linked edges reflow when their
/// nodes move — an updater expressed as a pure function of `t`.
#[derive(Debug, Clone)]
pub struct Link {
    pub from: String,
    pub to: String,
    pub trim: f32,
}

/// A recompute hook for a *derived* entity: given the current positions of its
/// [`Entity::deps`], mutate the entity (its `pos`, and shape params like a
/// circle's radius or an arc's angles). Run every frame in
/// [`crate::timeline::Timeline::apply`], so derived constructions track their
/// inputs — the general form of the `follow`/`link` updaters, kept
/// domain-agnostic (the core just calls the hook; kits supply the geometry).
pub type DeriveFn = fn(&mut Entity, &[Vec2]);

/// One drawable object in a [`crate::scene::Scene`].
#[derive(Debug, Clone)]
pub struct Entity {
    /// Unique id within the scene. Animations address entities by this.
    pub id: String,
    pub shape: Shape,
    /// Anchor position: centre for circles/rects/text, tail for
    /// lines/arrows/curves, offset for polygons.
    pub pos: Vec2,
    /// Primary color (fill, or stroke when there is no fill).
    pub color: Color,
    /// 0.0 = invisible, 1.0 = opaque. Multiplied into all colors at draw time.
    pub opacity: f32,
    /// Uniform scale about `pos`.
    pub scale: f32,
    /// Draw-on progress, 0.0–1.0. Stroked shapes: fraction of path/outline
    /// traced (fills fade in alongside). Text: fraction of characters shown
    /// (typewriter). Declare `.untraced()`, animate with `trace_in`/`type_in`.
    pub trace: f32,
    /// Draw order: higher `z` draws on top.
    pub z: i32,
    pub stroke: StrokeStyle,
    /// Font for `Shape::Text`.
    pub font: FontKind,
    /// Horizontal anchoring for `Shape::Text`.
    pub align: Align,
    /// Rotation in degrees, applied to text only (used for e.g. stamps).
    pub rot: f32,
    /// Max text width in logical pixels; longer text word-wraps into
    /// centred lines. `None` = single line. Text only.
    pub wrap: Option<f32>,
    /// Group labels for addressing many entities at once
    /// (`Movie::tagged` + `all(...)`).
    pub tags: Vec<String>,
    /// Draw in screen coordinates, ignoring camera pan/zoom. Use for HUD
    /// overlays; normal page/world elements should leave this false.
    pub sticky: bool,
    /// Multiplier on the neon halo drawn behind this entity. 0 disables the
    /// glow (crisp UI chrome); 1 is the house default.
    pub glow: f32,
    /// Pin `pos` to `pos_of(other) + offset` each frame; opacity is
    /// multiplied by the followed entity's opacity. Used by labels.
    pub follow: Option<(String, Vec2)>,
    /// Track two entities as a reflowing edge (see [`Link`]).
    pub link: Option<Link>,
    /// Input entity ids for [`Entity::derive`].
    pub deps: Vec<String>,
    /// Recompute this entity from its `deps` each frame (see [`DeriveFn`]).
    pub derive: Option<DeriveFn>,
    /// If set, an HSL hue angle (degrees) that drives `color`; animatable via
    /// [`crate::timeline::Prop::Hue`] for colour cycling.
    pub hue: Option<f32>,
    /// If set, a live numeric readout: the `Shape::Text` content is
    /// `prefix + value + suffix`. Animate `value` via
    /// [`crate::timeline::Prop::Value`] and the text updates each frame.
    pub counter: Option<Counter>,
}

/// A live numeric readout attached to a text entity.
#[derive(Debug, Clone)]
pub struct Counter {
    pub value: f32,
    pub decimals: u8,
    pub prefix: String,
    pub suffix: String,
}

impl Counter {
    /// Format `value` with the given decimals, wrapped in prefix/suffix.
    pub fn render(&self) -> String {
        format!(
            "{}{:.*}{}",
            self.prefix, self.decimals as usize, self.value, self.suffix
        )
    }
}

impl Entity {
    /// New entity with defaults: opaque, scale 1, fully traced, z 0, glowing.
    pub fn new(id: impl Into<String>, shape: Shape, pos: Vec2, color: Color) -> Self {
        Entity {
            id: id.into(),
            shape,
            pos,
            color,
            opacity: 1.0,
            scale: 1.0,
            trace: 1.0,
            z: 0,
            stroke: StrokeStyle::default(),
            font: FontKind::default(),
            align: Align::default(),
            rot: 0.0,
            wrap: None,
            tags: Vec::new(),
            sticky: false,
            glow: 1.0,
            follow: None,
            link: None,
            deps: Vec::new(),
            derive: None,
            hue: None,
            counter: None,
        }
    }
}
