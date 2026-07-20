//! Drawable primitives. One entity type; its look is data (`Shape`).
//! New primitive = new `Shape` variant + match arm in `render::draw_entity`.

use macroquad::prelude::{Color, Vec2};

/// What an [`Entity`] looks like. Positions inside a shape (e.g. `to`,
/// One run of a [`Shape::RichText`] line: either a plain-text span (drawn with
/// the entity's font/colour) or a pre-rendered math image (a cached RaTeX PNG
/// with its logical width/height and the baseline offset from its top).
#[derive(Debug, Clone, PartialEq)]
pub enum TextRun {
    Text(String),
    Math {
        path: String,
        w: f32,
        h: f32,
        baseline: f32,
    },
}

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
    /// A spring/coil zigzag from `pos` to `to` (absolute) with `turns` coils —
    /// stretches and compresses as `to` animates (like a `Line`). Stroked only.
    Coil { to: Vec2, turns: u32 },
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
    /// A raster image (PNG/JPG) centred on `pos`, drawn at `w`×`h` px (scaled by
    /// `Entity::scale`). `path` is loaded once into a texture cache at render
    /// start; a missing/unloaded image draws a placeholder box. `tint`: when true
    /// the texture is multiplied by `Entity::color` (for a white-on-transparent
    /// glyph image like a rendered equation, so it takes the template colour);
    /// when false it draws at full colour (photos/logos).
    Image {
        path: String,
        w: f32,
        h: f32,
        tint: bool,
    },
    /// **Mixed text + inline math** on one line: a sequence of plain-text and
    /// pre-rendered math runs, laid out left-to-right and baseline-aligned at
    /// render time. Built by the inline-`$…$` pass from a `Shape::Text` whose
    /// content has embedded math. `size` is the em height.
    RichText { runs: Vec<TextRun>, size: f32 },
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
    pub trim_from: f32,
    pub trim_to: f32,
    /// Recompute circle/rectangle boundary intersections every frame. Generic
    /// std `link`s enable this; kit edges with explicit trim distances do not.
    pub auto_trim: bool,
    /// Quadratic-bezier bow in logical pixels. Zero keeps a straight edge;
    /// positive bends left of the from→to direction.
    pub bend: f32,
}

/// A recompute hook for a *derived* entity: given the current positions of its
/// [`Entity::deps`], mutate the entity (its `pos`, and shape params like a
/// circle's radius or an arc's angles). Run every frame in
/// [`crate::timeline::Timeline::apply`], so derived constructions track their
/// inputs — the general form of the `follow`/`link` updaters, kept
/// domain-agnostic (the core just calls the hook; kits supply the geometry).
pub type DeriveFn = fn(&mut Entity, &[Vec2]);

/// A visible creator parameter. The entity itself is a live [`Counter`]; the
/// range keeps authored journeys honest and drives its small track widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parameter {
    pub min: f32,
    pub max: f32,
}

/// A target property controlled by a creator parameter through [`bind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundProperty {
    X,
    Y,
    Opacity,
    Scale,
    Rot,
    Hue,
    Value,
    Trace,
    Formula,
}

/// How a parameter value maps onto one target property.
#[derive(Debug, Clone)]
pub enum ParameterMap {
    /// Map the parameter's declared min/max onto these output endpoints.
    Range { from: f32, to: f32 },
    /// Evaluate a formula where `p` is the live parameter. Plot-formula
    /// bindings additionally expose the plotted coordinate as `x`.
    Formula(crate::kits::math::expr::Node),
}

/// One pure per-frame connection created by `bind(parameter,target,...)`.
#[derive(Debug, Clone)]
pub struct ParameterBinding {
    pub source: String,
    pub target: String,
    pub property: BoundProperty,
    pub map: ParameterMap,
}

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
    /// Monotonic path-flow phase. Its fractional part renders as a short
    /// luminous pulse travelling over path-like shapes; integer values are
    /// resting states with no pulse. The `flow` verb advances it by one.
    pub flow: f32,
    /// Draw order: higher `z` draws on top.
    pub z: i32,
    pub stroke: StrokeStyle,
    /// Optional `(dash, gap)` lengths in logical pixels for path-like shapes.
    /// `None` keeps the normal solid stroke. Kept on the entity (rather than a
    /// calculus/plot primitive) so the same visual language works for plots,
    /// lines, links, arrows, curves, splines, coils, and arcs.
    pub dash: Option<(f32, f32)>,
    /// Font for `Shape::Text`.
    pub font: FontKind,
    /// Horizontal anchoring for `Shape::Text`.
    pub align: Align,
    /// Rotation in degrees, applied to text only (used for e.g. stamps).
    pub rot: f32,
    /// Corner radius for rectangle entities. Zero keeps the classic square
    /// rectangle; creator/UI cards use a positive value for polished panels.
    /// Stored on the entity rather than `Shape::Rect` so existing constructors
    /// and shape pattern matches remain backwards compatible.
    pub corner_radius: f32,
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
    /// If set, this counter is a bounded creator parameter. Animate its normal
    /// `value` property; bindings and the native widget resolve from it each
    /// frame without accumulating state.
    pub parameter: Option<Parameter>,
    /// If set, `(from, to, spin_deg)` for a shape morph: two outline point-sets
    /// (same length) and a winding angle. Animate [`crate::timeline::Prop::Morph`]
    /// `0→1` to blend the `Polyline` between the outlines, rotating by `spin_deg`
    /// (clockwise if positive) as it goes.
    pub morph: Option<(Vec<Vec2>, Vec<Vec2>, f32)>,
    /// Draw a typewriter cursor at the end of the revealed text (`Shape::Text`).
    pub type_cursor: bool,
    /// If set, this entity is a plotted function graph (from `plot`): its
    /// function + screen mapping, so `tangent`/`slope` can query it by id.
    pub graph: Option<GraphFn>,
    /// If set, this entity is a view over a plotted graph (`tangent`, `normal`,
    /// `slope`, or `area`). Recomputed from its moving parameter `x`, animatable
    /// via `to(id, x, …)`.
    pub graph_view: Option<GraphView>,
    /// Source plot id for a graph view. This lets a parameter-bound plot push
    /// its changing formula into tangents, areas, slopes, and markers too.
    pub graph_source: Option<String>,
}

/// Where a [`GraphFn`]'s values come from.
#[derive(Debug, Clone)]
pub enum GraphSrc {
    /// A compiled formula in `x` (from `plot`).
    Expr(crate::kits::math::expr::Node),
    /// A live two-variable formula: plot coordinate `x` plus creator parameter
    /// `p`. `bind(param, plot, formula, "...")` updates only `p` each frame.
    ParameterExpr {
        node: crate::kits::math::expr::Node,
        p: f32,
    },
    /// A numerically-sampled curve (from `deriv`/`accum`): ascending `xs` with
    /// matching `ys`, evaluated by linear interpolation. This makes derived
    /// curves first-class — you can `tangent`/`slope`/`area` them too.
    Samples { xs: Vec<f32>, ys: Vec<f32> },
}

/// A plotted function remembered on its entity: its value source plus the
/// screen mapping `plot` used (`(cx + x*sx, cy - f(x)*sy)` over `x ∈ [x0,x1]`).
/// `plot` fills this in so later constructions — `tangent`/`slope`/`normal`/
/// `area`, and the derived `deriv`/`accum` curves — can *query the function by
/// the plot's id* instead of the author retyping the formula.
#[derive(Debug, Clone)]
pub struct GraphFn {
    pub src: GraphSrc,
    /// Screen anchor: math `(0,0)` maps here.
    pub center: Vec2,
    pub sx: f32,
    pub sy: f32,
    pub x0: f32,
    pub x1: f32,
}

impl GraphFn {
    /// `f(x)` in math units.
    pub fn y(&self, x: f32) -> f32 {
        match &self.src {
            GraphSrc::Expr(n) => n.eval(x, 0.0),
            GraphSrc::ParameterExpr { node, p } => node.eval(x, *p),
            GraphSrc::Samples { xs, ys } => interp(xs, ys, x),
        }
    }
    /// The point on the curve at `x`, in screen coords.
    pub fn point(&self, x: f32) -> Vec2 {
        Vec2::new(
            self.center.x + x * self.sx,
            self.center.y - self.y(x) * self.sy,
        )
    }
    /// Slope `dy/dx` at `x` (math units) by symmetric central difference — a
    /// tight numerical estimate; non-finite at corners/breaks so the caller can
    /// decline to draw a fake tangent.
    pub fn slope(&self, x: f32) -> f32 {
        let h = ((self.x1 - self.x0).abs() * 1e-3).max(1e-4);
        // one-sided at the domain edges so a *sampled* curve (whose `y` clamps
        // outside the range) isn't halved there; central in the interior
        if x - h < self.x0 {
            (self.y(x + h) - self.y(x)) / h
        } else if x + h > self.x1 {
            (self.y(x) - self.y(x - h)) / h
        } else {
            (self.y(x + h) - self.y(x - h)) / (2.0 * h)
        }
    }
    /// Second derivative `f''(x)` by central difference — drives concavity and
    /// inflection detection. Uses a wider step than `slope` (second differences
    /// are noisier).
    pub fn second(&self, x: f32) -> f32 {
        let h = ((self.x1 - self.x0).abs() * 5e-3).max(1e-3);
        (self.y(x + h) - 2.0 * self.y(x) + self.y(x - h)) / (h * h)
    }
    /// The `k`-th derivative at `a` via the central finite-difference stencil
    /// `f⁽ᵏ⁾(a) ≈ h⁻ᵏ Σ (−1)ⁱ C(k,i) f(a + (k/2 − i)h)`. Drives Taylor
    /// coefficients. Accurate for low `k` on smooth functions; higher orders get
    /// progressively noisier (a fundamental limit of numeric differentiation).
    pub fn nth_deriv(&self, a: f32, k: u32) -> f32 {
        if k == 0 {
            return self.y(a);
        }
        let h = 0.3f32;
        let k = k as i32;
        let mut sum = 0.0f32;
        for i in 0..=k {
            let mut c = if i % 2 == 0 { 1.0f64 } else { -1.0 };
            for j in 0..i {
                c = c * (k - j) as f64 / (j + 1) as f64; // (-1)^i * C(k, i)
            }
            sum += c as f32 * self.y(a + (k as f32 / 2.0 - i as f32) * h);
        }
        sum / h.powi(k)
    }
    /// Refine a root bracketed in `[a, b]` (where `y` changes sign) by bisection.
    fn bisect(&self, mut a: f32, mut b: f32) -> f32 {
        let mut fa = self.y(a);
        for _ in 0..60 {
            let m = 0.5 * (a + b);
            let fm = self.y(m);
            if !fm.is_finite() {
                break;
            }
            if fa * fm <= 0.0 {
                b = m;
            } else {
                a = m;
                fa = fm;
            }
        }
        0.5 * (a + b)
    }
    /// Every zero-crossing of the function across its domain `[x0, x1]`, found by
    /// scanning for sign changes and refining each by bisection (math units).
    pub fn roots(&self) -> Vec<f32> {
        const STEPS: usize = 500;
        let span = self.x1 - self.x0;
        let mut out: Vec<f32> = Vec::new();
        let mut px = self.x0;
        let mut py = self.y(px);
        for i in 1..=STEPS {
            let x = self.x0 + span * i as f32 / STEPS as f32;
            let y = self.y(x);
            if py.is_finite() && y.is_finite() {
                // a strict sign change brackets a root; a zero landing exactly on
                // this sample is caught separately (the strict test steps over
                // it, since the product is 0, not < 0)
                let r = if py * y < 0.0 {
                    Some(self.bisect(px, x))
                } else if y == 0.0 && py != 0.0 {
                    Some(x)
                } else {
                    None
                };
                if let Some(r) = r {
                    if out
                        .last()
                        .map_or(true, |&l| (r - l).abs() > span.abs() * 1e-3)
                    {
                        out.push(r);
                    }
                }
            }
            px = x;
            py = y;
        }
        out
    }
    /// Newton's method from `x0`: the zig-zag of screen points that visualises
    /// the iteration — curve point → down the tangent to the x-axis (the next
    /// guess) → back up to the curve → … Stops on convergence, a flat slope, a
    /// non-finite step, or leaving the domain.
    pub fn newton_path(&self, x0: f32, steps: u32) -> Vec<Vec2> {
        let mut pts = Vec::new();
        let p0 = self.point(x0);
        if !(p0.x.is_finite() && p0.y.is_finite()) {
            return pts;
        }
        pts.push(p0);
        let span = (self.x1 - self.x0).abs();
        let mut x = x0;
        for _ in 0..steps.clamp(1, 40) {
            let (y, m) = (self.y(x), self.slope(x));
            if !y.is_finite() || !m.is_finite() || m.abs() < 1e-6 {
                break;
            }
            let nx = x - y / m; // where the tangent meets the x-axis
            if !nx.is_finite() {
                break;
            }
            // down/along to the axis (math y = 0 → screen center.y), then up to
            // the curve at the new guess
            pts.push(Vec2::new(self.center.x + nx * self.sx, self.center.y));
            let pc = self.point(nx);
            if pc.x.is_finite() && pc.y.is_finite() {
                pts.push(pc);
            }
            if (nx - x).abs() < 1e-5 {
                break;
            }
            x = nx;
            if x < self.x0 - span * 0.25 || x > self.x1 + span * 0.25 {
                break;
            }
        }
        pts
    }
}

/// Linear interpolation of `(xs, ys)` (xs ascending) at `x`, clamped to the ends.
fn interp(xs: &[f32], ys: &[f32], x: f32) -> f32 {
    let n = xs.len();
    if n == 0 {
        return f32::NAN;
    }
    if x <= xs[0] {
        return ys[0];
    }
    if x >= xs[n - 1] {
        return ys[n - 1];
    }
    let (mut lo, mut hi) = (0usize, n - 1);
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if xs[mid] <= x {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let t = (x - xs[lo]) / (xs[hi] - xs[lo]);
    ys[lo] + (ys[hi] - ys[lo]) * t
}

/// Definite integral of a graph's function over `[a, b]` by composite Simpson's
/// rule. Exposed so `accum` can build the accumulation function `∫ₐˣ f`.
pub(crate) fn integrate(g: &GraphFn, a: f32, b: f32, n: u32) -> f32 {
    let n = {
        let m = n.max(2);
        if m % 2 == 1 {
            m + 1
        } else {
            m
        }
    } as usize;
    // a signed step gives the signed integral directly (negative when b < a)
    let h = (b - a) / n as f32;
    if h == 0.0 {
        return 0.0;
    }
    let mut s = g.y(a) + g.y(b);
    for i in 1..n {
        let y = g.y(a + h * i as f32);
        s += y * if i % 2 == 1 { 4.0 } else { 2.0 };
    }
    s * h / 3.0
}

/// A **view over a plotted function** — the shared state behind `tangent`,
/// `normal`, `slope`, and `area`. Each carries the source [`GraphFn`] and a
/// moving parameter `x` (in the graph's own units), animatable as one property
/// (`to(id, x, …)` → [`crate::timeline::Prop::PlotX`]): for the line/readout
/// forms `x` is the touch point; for `Area` it's the sweeping right bound. This
/// is the "ask the function you already drew a question" substrate.
#[derive(Debug, Clone)]
pub enum GraphView {
    /// Tangent line at `x` (segment + contact dot).
    Tangent { graph: GraphFn, x: f32, half: f32 },
    /// Normal (perpendicular) line at `x` (segment + contact dot).
    Normal { graph: GraphFn, x: f32, half: f32 },
    /// A live slope readout riding the point at `x` (`off` = screen offset).
    Slope { graph: GraphFn, x: f32, off: Vec2 },
    /// Filled region under the curve from `a` to the moving bound `x`, sampled
    /// with `n` intervals.
    Area {
        graph: GraphFn,
        a: f32,
        x: f32,
        n: u32,
    },
    /// A live readout of the definite integral from `a` to the moving bound `x`,
    /// pinned at screen position `at` (climbs as `x` sweeps).
    Integral {
        graph: GraphFn,
        a: f32,
        x: f32,
        n: u32,
        at: Vec2,
    },
    /// A plain dot riding the curve at `x` (a `Shape::Circle` whose `pos`
    /// follows the curve). Used by `limit`'s approaching point; slide it with
    /// `to(id, x, …)`.
    Mark { graph: GraphFn, x: f32 },
}

impl GraphView {
    /// The moving parameter.
    pub fn x(&self) -> f32 {
        match self {
            GraphView::Tangent { x, .. }
            | GraphView::Normal { x, .. }
            | GraphView::Slope { x, .. }
            | GraphView::Area { x, .. }
            | GraphView::Integral { x, .. }
            | GraphView::Mark { x, .. } => *x,
        }
    }
    /// Set the moving parameter (the timeline calls this to animate).
    pub fn set_x(&mut self, v: f32) {
        match self {
            GraphView::Tangent { x, .. }
            | GraphView::Normal { x, .. }
            | GraphView::Slope { x, .. }
            | GraphView::Area { x, .. }
            | GraphView::Integral { x, .. }
            | GraphView::Mark { x, .. } => *x = v,
        }
    }
    /// The source graph.
    pub fn graph(&self) -> &GraphFn {
        match self {
            GraphView::Tangent { graph, .. }
            | GraphView::Normal { graph, .. }
            | GraphView::Slope { graph, .. }
            | GraphView::Area { graph, .. }
            | GraphView::Integral { graph, .. }
            | GraphView::Mark { graph, .. } => graph,
        }
    }
    /// Replace the source graph while keeping this view's own moving parameter
    /// and layout. Parameter-bound plots use this to keep analysis views live.
    pub fn set_graph(&mut self, next: GraphFn) {
        match self {
            GraphView::Tangent { graph, .. }
            | GraphView::Normal { graph, .. }
            | GraphView::Slope { graph, .. }
            | GraphView::Area { graph, .. }
            | GraphView::Integral { graph, .. }
            | GraphView::Mark { graph, .. } => *graph = next,
        }
    }
    /// Line-segment endpoints `(tail, head)` for `Tangent`/`Normal`, centred on
    /// the contact point (`None` for the readout/area forms). Both endpoints
    /// collapse to the contact point when the slope is undefined
    /// (corner/asymptote) — honest: no fake line is drawn, only the dot.
    pub fn segment(&self) -> Option<(Vec2, Vec2)> {
        let (graph, x, half, perp) = match self {
            GraphView::Tangent { graph, x, half } => (graph, *x, *half, false),
            GraphView::Normal { graph, x, half } => (graph, *x, *half, true),
            _ => return None,
        };
        let t = graph.point(x);
        let m = graph.slope(x);
        if !t.x.is_finite() || !t.y.is_finite() || !m.is_finite() {
            return Some((t, t));
        }
        // a math step (1, m) maps to screen direction (sx, -m*sy); the normal is
        // that turned 90° in screen space
        let mut dir = Vec2::new(graph.sx, -m * graph.sy);
        if perp {
            dir = Vec2::new(-dir.y, dir.x);
        }
        let len = dir.length();
        if len < 1e-6 {
            return Some((t, t));
        }
        let d = dir / len * half;
        Some((t - d, t + d))
    }
    /// Contact/anchor point on the curve, in screen coords.
    pub fn touch(&self) -> Vec2 {
        self.graph().point(self.x())
    }
    /// The numeric readout: slope for `Slope`, running integral for
    /// `Area`/`Integral`.
    pub fn value(&self) -> f32 {
        match self {
            GraphView::Slope { graph, x, .. } => graph.slope(*x),
            GraphView::Area { graph, a, x, n } | GraphView::Integral { graph, a, x, n, .. } => {
                integrate(graph, *a, *x, *n)
            }
            _ => 0.0,
        }
    }
    /// Screen position for a readout: the point + offset for `Slope`, the pinned
    /// `at` for `Integral`.
    pub fn readout_pos(&self) -> Vec2 {
        match self {
            GraphView::Slope { graph, x, off } => graph.point(*x) + *off,
            GraphView::Integral { at, .. } => *at,
            _ => self.touch(),
        }
    }
    /// Baked `(tris, rings)` for an `Area` fill: vertical strips between the
    /// curve and the baseline (math `y = 0`), correct for any wiggly curve.
    pub fn region(&self) -> (Vec<[Vec2; 3]>, Vec<Vec<Vec2>>) {
        let GraphView::Area { graph, a, x, n } = self else {
            return (Vec::new(), Vec::new());
        };
        let (lo, hi) = (a.min(*x), a.max(*x));
        let steps = (*n).max(2);
        let base_y = graph.center.y; // screen y of math y = 0
        let mut top = Vec::with_capacity(steps as usize + 1);
        for i in 0..=steps {
            let xx = lo + (hi - lo) * i as f32 / steps as f32;
            let p = graph.point(xx);
            if p.x.is_finite() && p.y.is_finite() {
                top.push(p);
            }
        }
        if top.len() < 2 {
            return (Vec::new(), Vec::new());
        }
        let mut tris = Vec::with_capacity((top.len() - 1) * 2);
        for i in 0..top.len() - 1 {
            let (p0, p1) = (top[i], top[i + 1]);
            let (b0, b1) = (Vec2::new(p0.x, base_y), Vec2::new(p1.x, base_y));
            tris.push([p0, p1, b1]);
            tris.push([p0, b1, b0]);
        }
        let mut ring = top.clone();
        ring.push(Vec2::new(top[top.len() - 1].x, base_y));
        ring.push(Vec2::new(top[0].x, base_y));
        (tris, vec![ring])
    }
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
            flow: 0.0,
            z: 0,
            stroke: StrokeStyle::default(),
            dash: None,
            font: FontKind::default(),
            align: Align::default(),
            rot: 0.0,
            corner_radius: 0.0,
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
            parameter: None,
            morph: None,
            type_cursor: false,
            graph: None,
            graph_view: None,
            graph_source: None,
        }
    }
}
