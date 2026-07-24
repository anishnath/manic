//! The macroquad draw pass: scene → pixels, plus the neon terminal chrome.
//!
//! New primitive = match arm in [`draw_entity`]. All world coordinates flow
//! through [`View::xform`]: supersampling scale + the 2D camera. The separate
//! `render3d` pass is composited underneath. The neon identity comes from a
//! soft glow (halo) pass drawn behind fully-traced strokes and text.

use macroquad::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate::primitives::{Align, Entity, FontKind, Shape, TextRun};
use crate::scene::Scene;
use crate::style::{self, Fonts};
use crate::text::{LayoutKey, RasterImage, RasterKey, ShapedLayout, TextEngine};

thread_local! {
    /// Loaded image textures, keyed by their `path`. macroquad runs
    /// single-threaded (the render loop), so a thread-local cache is safe.
    static TEXTURES: RefCell<HashMap<String, Texture2D>> = RefCell::new(HashMap::new());
    /// Ordinary text has one bundled-only shaper/raster cache for the entire
    /// playback thread. Measurement and drawing request the same LayoutKey.
    static TEXT_ENGINE: RefCell<TextEngine> = RefCell::new(TextEngine::new());
    static TEXT_GLYPH_TEXTURES: RefCell<HashMap<RasterKey, Texture2D>> = RefCell::new(HashMap::new());
}

/// Load every referenced image path into the texture cache once, before the
/// frame loop. Call from the async render loop. A path that fails to load is
/// simply skipped (its entity draws a placeholder box).
pub async fn preload_textures<I: IntoIterator<Item = String>>(paths: I) {
    for path in paths {
        let cached = TEXTURES.with(|t| t.borrow().contains_key(&path));
        if cached {
            continue;
        }
        match load_texture(&path).await {
            Ok(tex) => {
                tex.set_filter(FilterMode::Linear);
                TEXTURES.with(|t| t.borrow_mut().insert(path.clone(), tex));
            }
            Err(_) => eprintln!("image: could not load `{path}` (drawing a placeholder)"),
        }
    }
}

fn get_texture(path: &str) -> Option<Texture2D> {
    TEXTURES.with(|t| t.borrow().get(path).cloned())
}

/// World (logical) → output (physical) transform: supersampling factor `ss`
/// plus the animatable 2D camera (`cam` centre, `zoom` factor).
#[derive(Debug, Clone, Copy)]
pub struct View {
    pub ss: f32,
    pub cam: Vec2,
    pub zoom: f32,
    /// Logical canvas centre; the camera zooms about this after recentering.
    pub center: Vec2,
}

impl View {
    /// Identity camera at supersampling factor `ss` for a `w`×`h` canvas.
    pub fn neutral(w: f32, h: f32, ss: f32) -> View {
        let center = Vec2::new(w / 2.0, h / 2.0);
        View {
            ss,
            cam: center,
            zoom: 1.0,
            center,
        }
    }

    /// Read the camera pose from the scene's `"__cam"` entity, if present.
    pub fn from_scene(scene: &Scene, w: f32, h: f32, ss: f32) -> View {
        let mut v = View::neutral(w, h, ss);
        if let Some(cam) = scene.get(crate::movie::CAMERA_ID) {
            v.cam = cam.pos;
            v.zoom = cam.scale;
        }
        v
    }

    #[inline]
    pub fn xform(&self, p: Vec2) -> Vec2 {
        ((p - self.cam) * self.zoom + self.center) * self.ss
    }

    /// Size multiplier (camera zoom × supersampling).
    #[inline]
    pub fn k(&self) -> f32 {
        self.zoom * self.ss
    }
}

// ---- paths & tracing ------------------------------------------------------

/// Draw the first `frac` (by arc length) of a polyline.
fn draw_path(pts: &[Vec2], frac: f32, width: f32, color: Color) {
    if pts.len() < 2 || frac <= 0.0 {
        return;
    }
    let total: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    let mut budget = total * frac.min(1.0);
    for w in pts.windows(2) {
        let seg = (w[1] - w[0]).length();
        if seg <= 0.0 {
            continue;
        }
        if budget >= seg {
            draw_line(w[0].x, w[0].y, w[1].x, w[1].y, width, color);
            budget -= seg;
        } else {
            let end = w[0] + (w[1] - w[0]) * (budget / seg);
            draw_line(w[0].x, w[0].y, end.x, end.y, width, color);
            return;
        }
    }
}

/// Draw the traced portion of a polyline with a repeating dash/gap pattern.
/// Pattern state flows continuously across segment boundaries, so a sampled
/// plot reads as one dashed curve instead of restarting at every sample.
fn draw_dashed_path(pts: &[Vec2], frac: f32, width: f32, color: Color, dash: f32, gap: f32) {
    if pts.len() < 2 || frac <= 0.0 || dash <= 0.0 || gap <= 0.0 {
        return;
    }
    let total: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    let mut budget = total * frac.min(1.0);
    let mut drawing = true;
    let mut pattern_left = dash;

    for w in pts.windows(2) {
        if budget <= 0.0 {
            break;
        }
        let delta = w[1] - w[0];
        let seg_len = delta.length();
        if seg_len <= 0.0 {
            continue;
        }
        let dir = delta / seg_len;
        let mut at = w[0];
        let mut seg_left = seg_len.min(budget);

        while seg_left > 1e-4 {
            let step = seg_left.min(pattern_left);
            let next = at + dir * step;
            if drawing {
                draw_line(at.x, at.y, next.x, next.y, width, color);
            }
            at = next;
            seg_left -= step;
            pattern_left -= step;
            if pattern_left <= 1e-4 {
                drawing = !drawing;
                pattern_left = if drawing { dash } else { gap };
            }
        }
        budget -= seg_len.min(budget);
    }
}

#[inline]
fn draw_styled_path(pts: &[Vec2], frac: f32, width: f32, color: Color, dash: Option<(f32, f32)>) {
    if let Some((on, off)) = dash {
        draw_dashed_path(pts, frac, width, color, on, off);
    } else {
        draw_path(pts, frac, width, color);
    }
}

/// Point and unit tangent at `frac` of a polyline's arc length.
fn path_point(pts: &[Vec2], frac: f32) -> (Vec2, Vec2) {
    let total: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    let mut budget = total * frac.clamp(0.0, 1.0);
    for w in pts.windows(2) {
        let seg = (w[1] - w[0]).length();
        if seg <= 0.0 {
            continue;
        }
        if budget <= seg {
            let dir = (w[1] - w[0]) / seg;
            return (w[0] + dir * budget, dir);
        }
        budget -= seg;
    }
    let n = pts.len();
    let dir = (pts[n - 1] - pts[n - 2]).normalize_or_zero();
    (pts[n - 1], dir)
}

fn bezier_pts(from: Vec2, ctrl: Vec2, to: Vec2, n: usize) -> Vec<Vec2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let a = from.lerp(ctrl, t);
            let b = ctrl.lerp(to, t);
            a.lerp(b, t)
        })
        .collect()
}

/// A spring coil as a zigzag polyline from `p` to `q`: a short straight lead-in
/// at each end and `turns` alternating perpendicular peaks between. The amplitude
/// scales (clamped) with length, so it reads as a coil that stretches/compresses.
fn coil_points(p: Vec2, q: Vec2, turns: u32) -> Vec<Vec2> {
    let d = q - p;
    let len = d.length();
    if len < 1.0 || turns == 0 {
        return vec![p, q];
    }
    let dir = d / len;
    let perp = Vec2::new(-dir.y, dir.x);
    let amp = (len * 0.05).clamp(6.0, 16.0);
    let lead = 0.12;
    let a = p + dir * (len * lead);
    let b = q - dir * (len * lead);
    let seg = (turns * 2).max(2);
    let mut pts = Vec::with_capacity(seg as usize + 4);
    pts.push(p);
    pts.push(a);
    for i in 1..seg {
        let f = i as f32 / seg as f32;
        let along = a.lerp(b, f);
        let off = if i % 2 == 1 { amp } else { -amp };
        pts.push(along + perp * off);
    }
    pts.push(b);
    pts.push(q);
    pts
}

fn circle_pts(c: Vec2, r: f32, n: usize) -> Vec<Vec2> {
    (0..=n)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / n as f32 - std::f32::consts::FRAC_PI_2;
            c + Vec2::new(a.cos(), a.sin()) * r
        })
        .collect()
}

/// Arrowhead sized from stroke width, at `tip`, pointing along `dir`.
fn draw_head(tip: Vec2, dir: Vec2, width: f32, color: Color) {
    if dir == Vec2::ZERO {
        return;
    }
    let head_len = 10.0 + width * 2.5;
    let head_w = head_len * 0.5;
    let base = tip - dir * head_len;
    let perp = Vec2::new(-dir.y, dir.x);
    draw_triangle(tip, base + perp * head_w, base - perp * head_w, color);
}

/// Path with an optional arrowhead riding its traced tip. The stroke stops
/// short of the tip so the head doesn't overlap it.
fn draw_stroke_path(
    pts: &[Vec2],
    frac: f32,
    width: f32,
    color: Color,
    arrow: bool,
    dash: Option<(f32, f32)>,
) {
    if !arrow {
        draw_styled_path(pts, frac, width, color, dash);
        return;
    }
    let total: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    let drawn = total * frac;
    if drawn < 1.0 {
        return;
    }
    let (tip, dir) = path_point(pts, frac);
    let head_len = (10.0 + width * 2.5).min(drawn);
    let body_frac = frac * (1.0 - head_len / drawn.max(1e-3)).max(0.0);
    draw_styled_path(pts, body_frac, width, color, dash);
    draw_head(tip, dir, width, color);
}

// ---- neon glow ------------------------------------------------------------

/// A soft, low-alpha version of `c` for the halo pass. `opacity` is the
/// entity's own alpha; `g` its glow multiplier.
fn halo(c: Color, opacity: f32, g: f32) -> Color {
    Color::new(c.r, c.g, c.b, (opacity * 0.18 * g).clamp(0.0, 1.0))
}

// ---- gradient paint --------------------------------------------------------

/// An [`crate::primitives::Gradient`] resolved into physical space: stops
/// template-remapped and opacity-baked, bounds taken from the geometry being
/// drawn. Sampling is a pure function of position (+ a precomputed parameter
/// for `Along`/`Speed`/`Curvature`), so gradients stay deterministic and
/// scrub-safe.
struct GradPaint {
    stops: Vec<Color>,
    kind: crate::primitives::GradientKind,
    min: Vec2,
    max: Vec2,
    /// True farthest distance from the bounds centre to the geometry, so a
    /// radial gradient's last stop lands exactly on the drawn rim (a circle's
    /// bounding-box corner would overshoot it by √2).
    rmax: f32,
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// Piecewise-linear sample of evenly spaced color stops at `t ∈ [0, 1]`.
fn sample_stops(stops: &[Color], t: f32) -> Color {
    match stops.len() {
        0 => Color::new(1.0, 1.0, 1.0, 1.0),
        1 => stops[0],
        n => {
            let x = t.clamp(0.0, 1.0) * (n - 1) as f32;
            let i = (x.floor() as usize).min(n - 2);
            lerp_color(stops[i], stops[i + 1], x - i as f32)
        }
    }
}

impl GradPaint {
    /// Resolve an authored gradient against the active template and the
    /// physical bounds of the geometry it paints. `alpha` is the paint's
    /// overall opacity (fill: `opacity * trace`; stroke: `opacity`).
    fn resolve(
        g: &crate::primitives::Gradient,
        tpl: &style::Template,
        alpha: f32,
        pts: &[Vec2],
    ) -> Self {
        let mut min = Vec2::new(f32::INFINITY, f32::INFINITY);
        let mut max = Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        for p in pts {
            min = min.min(*p);
            max = max.max(*p);
        }
        let c = (min + max) * 0.5;
        let rmax = pts
            .iter()
            .map(|p| (*p - c).length())
            .fold(0.0_f32, f32::max)
            .max(1e-4);
        GradPaint {
            stops: g
                .stops
                .iter()
                .map(|&c| style::with_opacity(tpl.palette.remap(c), alpha))
                .collect(),
            kind: g.kind,
            min,
            max,
            rmax,
        }
    }

    /// Same gradient, halo-toned for the glow pass.
    fn halo(&self, opacity: f32, g: f32) -> Self {
        GradPaint {
            stops: self.stops.iter().map(|&c| halo(c, opacity, g)).collect(),
            kind: self.kind,
            min: self.min,
            max: self.max,
            rmax: self.rmax,
        }
    }

    /// Gradient parameter (0..1) from physical position — only meaningful for
    /// the position-driven kinds (`Linear`/`Radial`).
    fn param_of(&self, pos: Vec2) -> f32 {
        use crate::primitives::GradientKind::*;
        match self.kind {
            Linear(angle) => {
                let a = angle.to_radians();
                let axis = Vec2::new(a.cos(), a.sin());
                // project the bounds onto the axis to normalise
                let corners = [
                    self.min,
                    Vec2::new(self.max.x, self.min.y),
                    self.max,
                    Vec2::new(self.min.x, self.max.y),
                ];
                let dots: Vec<f32> = corners.iter().map(|c| c.dot(axis)).collect();
                let lo = dots.iter().cloned().fold(f32::INFINITY, f32::min);
                let hi = dots.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                if hi - lo < 1e-4 {
                    0.0
                } else {
                    (pos.dot(axis) - lo) / (hi - lo)
                }
            }
            Radial => {
                let c = (self.min + self.max) * 0.5;
                (pos - c).length() / self.rmax
            }
            _ => 0.0,
        }
    }

    /// Color at physical position `pos`; `t_path` is the precomputed path
    /// parameter for the path-driven kinds (`Along` = arc-length fraction,
    /// `Speed`/`Curvature` = the normalised quantity). Absolute over the full
    /// path, so a half-traced curve shows the true start of its gradient.
    fn at(&self, pos: Vec2, t_path: f32) -> Color {
        use crate::primitives::GradientKind::*;
        let t = match self.kind {
            Along | Speed | Curvature => t_path,
            Linear(_) | Radial => self.param_of(pos),
        };
        sample_stops(&self.stops, t)
    }
}

/// Per-point normalised parameter (0..1) for the quantity-driven stroke
/// kinds. The quantity is computed from the path itself, then normalised over
/// its own min…max — the visual truth of the gradient.
///
/// - `Speed`: segment length per step. Truthful only when points are
///   uniformly time-sampled (a physics trajectory); the DSL layer enforces
///   that before the entity ever reaches the renderer.
/// - `Curvature`: Menger curvature through each interior point — pure
///   geometry, defined on any path.
fn quantity_params(pts: &[Vec2], kind: crate::primitives::GradientKind) -> Option<Vec<f32>> {
    use crate::primitives::GradientKind::*;
    let n = pts.len();
    if n < 2 {
        return None;
    }
    let mut q: Vec<f32> = match kind {
        Speed => {
            let seg: Vec<f32> = pts.windows(2).map(|w| (w[1] - w[0]).length()).collect();
            (0..n)
                .map(|i| {
                    if i == 0 {
                        seg[0]
                    } else if i == n - 1 {
                        seg[n - 2]
                    } else {
                        (seg[i - 1] + seg[i]) * 0.5
                    }
                })
                .collect()
        }
        Curvature => {
            let menger = |a: Vec2, b: Vec2, c: Vec2| -> f32 {
                let (ab, bc, ca) = ((b - a).length(), (c - b).length(), (a - c).length());
                let denom = ab * bc * ca;
                if denom < 1e-6 {
                    return 0.0;
                }
                let cross = (b - a).perp_dot(c - b);
                (2.0 * cross.abs()) / denom
            };
            (0..n)
                .map(|i| {
                    let j = i.clamp(1, n.saturating_sub(2).max(1));
                    if n < 3 {
                        0.0
                    } else {
                        menger(pts[j - 1], pts[j], pts[j + 1])
                    }
                })
                .collect()
        }
        _ => return None,
    };
    let lo = q.iter().cloned().fold(f32::INFINITY, f32::min);
    let hi = q.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    if hi - lo > 1e-6 {
        for v in &mut q {
            *v = (*v - lo) / (hi - lo);
        }
    } else {
        // constant quantity → uniformly the first stop (no variation to show)
        for v in &mut q {
            *v = 0.0;
        }
    }
    Some(q)
}

/// Path parameter at arc-length fraction `frac`, interpolating a per-point
/// parameter array (used to color a gradient arrowhead at its traced tip).
fn param_at_frac(pts: &[Vec2], params: &[f32], frac: f32) -> f32 {
    let total: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    let mut budget = total * frac.clamp(0.0, 1.0);
    for (i, w) in pts.windows(2).enumerate() {
        let seg = (w[1] - w[0]).length();
        if seg <= 0.0 {
            continue;
        }
        if budget <= seg {
            return params[i] + (params[i + 1] - params[i]) * (budget / seg);
        }
        budget -= seg;
    }
    *params.last().unwrap_or(&1.0)
}

/// Gradient counterpart of [`draw_styled_path`]: walks the polyline with the
/// same arc-length budget and dash pattern, but colors each short step from
/// the gradient. Steps are capped so straight segments (a plain `line`) still
/// blend smoothly. For `Speed`/`Curvature` the color follows the per-point
/// quantity computed from the path itself.
fn draw_grad_path(pts: &[Vec2], frac: f32, width: f32, gp: &GradPaint, dash: Option<(f32, f32)>) {
    if pts.len() < 2 || frac <= 0.0 {
        return;
    }
    let total: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    if total <= 0.0 {
        return;
    }
    if let Some((on, off)) = dash {
        if on <= 0.0 || off <= 0.0 {
            return;
        }
    }
    let params = quantity_params(pts, gp.kind);
    let mut budget = total * frac.min(1.0);
    let mut dist = 0.0_f32;
    let mut drawing = true;
    let mut pattern_left = dash.map(|(on, _)| on).unwrap_or(f32::INFINITY);
    const STEP: f32 = 6.0;

    for (i, w) in pts.windows(2).enumerate() {
        if budget <= 1e-4 {
            break;
        }
        let delta = w[1] - w[0];
        let seg_len = delta.length();
        if seg_len <= 0.0 {
            continue;
        }
        let dir = delta / seg_len;
        let mut at = w[0];
        let mut walked = 0.0_f32;
        let mut seg_left = seg_len.min(budget);
        budget -= seg_left;

        while seg_left > 1e-4 {
            let step = seg_left.min(pattern_left).min(STEP);
            let next = at + dir * step;
            if drawing {
                let t = match &params {
                    Some(q) => {
                        let f = (walked + step * 0.5) / seg_len;
                        q[i] + (q[i + 1] - q[i]) * f
                    }
                    None => (dist + step * 0.5) / total,
                };
                let c = gp.at((at + next) * 0.5, t);
                draw_line(at.x, at.y, next.x, next.y, width, c);
            }
            at = next;
            dist += step;
            walked += step;
            seg_left -= step;
            if let Some((on, off)) = dash {
                pattern_left -= step;
                if pattern_left <= 1e-4 {
                    drawing = !drawing;
                    pattern_left = if drawing { on } else { off };
                }
            }
        }
    }
}

/// Gradient counterpart of [`draw_stroke_path`]: the arrowhead takes the
/// gradient's color at the traced tip.
fn draw_grad_stroke_path(
    pts: &[Vec2],
    frac: f32,
    width: f32,
    gp: &GradPaint,
    arrow: bool,
    dash: Option<(f32, f32)>,
) {
    if !arrow {
        draw_grad_path(pts, frac, width, gp, dash);
        return;
    }
    let total: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum();
    let drawn = total * frac;
    if drawn < 1.0 {
        return;
    }
    let (tip, dir) = path_point(pts, frac);
    let head_len = (10.0 + width * 2.5).min(drawn);
    let body_frac = frac * (1.0 - head_len / drawn.max(1e-3)).max(0.0);
    draw_grad_path(pts, body_frac, width, gp, dash);
    let tip_t = match quantity_params(pts, gp.kind) {
        Some(q) => param_at_frac(pts, &q, frac),
        None => frac,
    };
    draw_head(tip, dir, width, gp.at(tip, tip_t));
}

/// Fill triangles with per-vertex gradient colors via a mesh. Two-stop linear
/// gradients are affine in position, so vertex interpolation reproduces them
/// exactly; with more stops (or radial fills over long edges) triangles are
/// subdivided until each spans a small slice of the gradient, so middle stops
/// can't vanish inside one big triangle. Chunked to stay under macroquad's
/// per-drawcall vertex budget.
fn draw_tris_grad(tris: &[[Vec2; 3]], gp: &GradPaint) {
    let subdivided;
    let tris: &[[Vec2; 3]] = if gp.stops.len() > 2 {
        subdivided = subdivide_tris(tris, gp);
        &subdivided
    } else {
        tris
    };
    for chunk in tris.chunks(1000) {
        let mut vertices = Vec::with_capacity(chunk.len() * 3);
        let mut indices: Vec<u16> = Vec::with_capacity(chunk.len() * 3);
        for t in chunk {
            let i0 = vertices.len() as u16;
            for p in t {
                vertices.push(Vertex::new(p.x, p.y, 0.0, 0.0, 0.0, gp.at(*p, 0.0)));
            }
            indices.extend_from_slice(&[i0, i0 + 1, i0 + 2]);
        }
        draw_mesh(&Mesh {
            vertices,
            indices,
            texture: None,
        });
    }
}

/// Split triangles (longest edge at the midpoint) until each spans at most a
/// quarter of one stop interval of the gradient parameter, with a depth cap
/// so degenerate geometry can't explode the mesh.
fn subdivide_tris(tris: &[[Vec2; 3]], gp: &GradPaint) -> Vec<[Vec2; 3]> {
    let max_span = 0.25 / (gp.stops.len().max(2) - 1) as f32;
    let mut out = Vec::with_capacity(tris.len() * 4);
    let mut stack: Vec<([Vec2; 3], u8)> = tris.iter().map(|t| (*t, 0u8)).collect();
    while let Some((t, depth)) = stack.pop() {
        let ps = [gp.param_of(t[0]), gp.param_of(t[1]), gp.param_of(t[2])];
        let span = ps.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
            - ps.iter().cloned().fold(f32::INFINITY, f32::min);
        if span <= max_span || depth >= 6 {
            out.push(t);
            continue;
        }
        // split the longest edge
        let lens = [
            (t[1] - t[0]).length(),
            (t[2] - t[1]).length(),
            (t[0] - t[2]).length(),
        ];
        let e = if lens[0] >= lens[1] && lens[0] >= lens[2] {
            0
        } else if lens[1] >= lens[2] {
            1
        } else {
            2
        };
        let (a, b, c) = (t[e], t[(e + 1) % 3], t[(e + 2) % 3]);
        let m = (a + b) * 0.5;
        stack.push(([a, m, c], depth + 1));
        stack.push(([m, b, c], depth + 1));
    }
    out
}

/// Triangle fan from `center` over a ring of perimeter points.
fn fan_tris(center: Vec2, ring: &[Vec2]) -> Vec<[Vec2; 3]> {
    let n = ring.len();
    (0..n)
        .map(|i| [center, ring[i], ring[(i + 1) % n]])
        .collect()
}

/// Insert intermediate points so no ring edge exceeds `max_step` — keeps
/// radial fan fills smooth on shapes with long straight edges (rects,
/// polygons).
fn subdivide_ring(ring: &[Vec2], max_step: f32) -> Vec<Vec2> {
    let mut out = Vec::with_capacity(ring.len() * 2);
    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        out.push(a);
        let len = (b - a).length();
        let extra = (len / max_step).floor() as usize;
        for k in 1..=extra {
            out.push(a.lerp(b, k as f32 / (extra + 1) as f32));
        }
    }
    out
}

/// Perimeter ring of a rounded rectangle (open — first point not repeated).
fn rounded_rect_pts(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<Vec2> {
    let mut pts = Vec::with_capacity(36);
    for (cx, cy, a0) in [
        (x + w - r, y + r, -90.0_f32),
        (x + w - r, y + h - r, 0.0),
        (x + r, y + h - r, 90.0),
        (x + r, y + r, 180.0),
    ] {
        for i in 0..=8 {
            let a = (a0 + i as f32 * 90.0 / 8.0).to_radians();
            pts.push(Vec2::new(cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    pts
}

/// Filled rounded rectangle as a non-overlapping triangle fan. Avoiding layered
/// bars/circles matters for translucent UI fills: overlapping alpha would show
/// as darker discs at every corner.
fn draw_rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    let r = r.clamp(0.0, w.min(h) / 2.0);
    if r <= 0.5 {
        draw_rectangle(x, y, w, h, color);
        return;
    }
    let pts = rounded_rect_pts(x, y, w, h, r);
    let c = Vec2::new(x + w * 0.5, y + h * 0.5);
    for i in 0..pts.len() {
        draw_triangle(c, pts[i], pts[(i + 1) % pts.len()], color);
    }
}

/// Rounded outline sampled as one closed path, so glow and trace semantics stay
/// consistent with every other stroked manic shape.
fn draw_rounded_rect_lines(x: f32, y: f32, w: f32, h: f32, r: f32, width: f32, color: Color) {
    let r = r.clamp(0.0, w.min(h) / 2.0);
    if r <= 0.5 {
        draw_rectangle_lines(x, y, w, h, width, color);
        return;
    }
    let mut pts = Vec::with_capacity(37);
    let mut corner = |cx: f32, cy: f32, a0: f32| {
        for i in 0..=8 {
            let a = (a0 + i as f32 * 90.0 / 8.0).to_radians();
            pts.push(Vec2::new(cx + r * a.cos(), cy + r * a.sin()));
        }
    };
    corner(x + w - r, y + r, -90.0);
    corner(x + w - r, y + h - r, 0.0);
    corner(x + r, y + h - r, 90.0);
    corner(x + r, y + r, 180.0);
    pts.push(pts[0]);
    draw_path(&pts, 1.0, width, color);
}

/// Rotate `p` about `center` by `rad` radians.
fn rot_pt(p: Vec2, center: Vec2, rad: f32) -> Vec2 {
    if rad == 0.0 {
        return p;
    }
    let (s, c) = rad.sin_cos();
    let d = p - center;
    center + Vec2::new(d.x * c - d.y * s, d.x * s + d.y * c)
}

/// Centroid of a point set (for rotating polygons/polylines in place).
fn centroid(pts: &[Vec2]) -> Vec2 {
    if pts.is_empty() {
        return Vec2::ZERO;
    }
    pts.iter().copied().sum::<Vec2>() / pts.len() as f32
}

// ---- text -------------------------------------------------------------------

fn shaped_layout(
    text: &str,
    kind: FontKind,
    raster: f32,
    max_width: Option<f32>,
    align: Align,
) -> Arc<ShapedLayout> {
    TEXT_ENGINE.with(|engine| {
        engine
            .borrow_mut()
            .layout(LayoutKey::new(text, kind, raster, max_width, align))
    })
}

fn shaped_raster(
    layout: &Arc<ShapedLayout>,
    visible_graphemes: usize,
) -> (RasterKey, Arc<RasterImage>) {
    TEXT_ENGINE.with(|engine| engine.borrow_mut().raster(layout, visible_graphemes))
}

fn glyph_texture(key: RasterKey, image: &RasterImage) -> Texture2D {
    TEXT_GLYPH_TEXTURES.with(|textures| {
        let mut textures = textures.borrow_mut();
        if let Some(texture) = textures.get(&key) {
            return texture.clone();
        }
        // Keep the GPU cache bounded alongside TextEngine's CPU raster cache.
        if textures.len() >= 1024 {
            textures.clear();
        }
        let texture = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
        texture.set_filter(FilterMode::Linear);
        textures.insert(key, texture.clone());
        texture
    })
}

#[allow(clippy::too_many_arguments)]
fn draw_shaped_raster(
    layout: &Arc<ShapedLayout>,
    visible_graphemes: usize,
    x: f32,
    y: f32,
    scale: f32,
    rotation: f32,
    pivot: Vec2,
    color: Color,
) {
    let (key, image) = shaped_raster(layout, visible_graphemes);
    let texture = glyph_texture(key, &image);
    draw_texture_ex(
        &texture,
        x - image.pad * scale,
        y - image.pad * scale,
        color,
        DrawTextureParams {
            dest_size: Some(vec2(
                image.width as f32 * scale,
                image.height as f32 * scale,
            )),
            rotation,
            pivot: Some(pivot),
            ..Default::default()
        },
    );
}

fn measure_resolved_text(
    text: &str,
    _fonts: &Fonts,
    kind: FontKind,
    font_size: u16,
    font_scale: f32,
) -> TextDimensions {
    let layout = shaped_layout(text, kind, font_size as f32, None, Align::Left);
    TextDimensions {
        width: layout.width * font_scale,
        height: (layout.ascent + layout.descent) * font_scale,
        offset_y: layout.ascent * font_scale,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_resolved_text(
    text: &str,
    x: f32,
    y: f32,
    _fonts: &Fonts,
    kind: FontKind,
    font_size: u16,
    font_scale: f32,
    rotation: f32,
    color: Color,
) {
    let layout = shaped_layout(text, kind, font_size as f32, None, Align::Left);
    let top = y - layout.ascent * font_scale;
    draw_shaped_raster(
        &layout,
        layout.graphemes,
        x,
        top,
        font_scale,
        rotation,
        vec2(x, y),
        color,
    );
}

/// Draw text at `pos` (physical pixels, physical `size`). Handles wrapping,
/// alignment, rotation and typewriter `trace`.
///
/// `raster` is the size the glyphs are rasterized at; the remaining factor up
/// to `size` is applied as a smooth vertex scale. Keeping `raster` constant
/// while the camera zooms (pass logical size × supersampling) avoids the
/// per-frame re-rasterization that makes text jitter during zooms.
#[allow(clippy::too_many_arguments)]
pub fn draw_text_block(
    text: &str,
    pos: Vec2,
    size: f32,
    raster: f32,
    color: Color,
    _fonts: &Fonts,
    kind: FontKind,
    rot_deg: f32,
    wrap: Option<f32>,
    align: Align,
    trace: f32,
    cursor: bool,
) {
    let raster = raster.max(1.0);
    let font_scale = size.max(0.01) / raster;
    let raster_wrap = wrap.map(|width| width / font_scale);
    let layout = shaped_layout(text, kind, raster, raster_wrap, align);
    let visible = if trace >= 1.0 {
        layout.graphemes
    } else {
        (layout.graphemes as f32 * trace.clamp(0.0, 1.0)).floor() as usize
    };
    let x = match align {
        Align::Center => pos.x - layout.width * font_scale / 2.0,
        Align::Left => pos.x,
    };
    let y = pos.y - layout.height * font_scale / 2.0;
    draw_shaped_raster(
        &layout,
        visible,
        x,
        y,
        font_scale,
        rot_deg.to_radians(),
        pos,
        color,
    );

    // The cursor is geometry, not another text layout: revealing it cannot
    // reshape the sentence or move its final line box.
    if cursor {
        let progress = if layout.graphemes == 0 {
            0.0
        } else {
            visible as f32 / layout.graphemes as f32
        };
        let caret_x = if layout.rtl {
            x + layout.width * font_scale * (1.0 - progress)
        } else {
            x + layout.width * font_scale * progress
        };
        let caret_h = size * 0.75;
        let a = rot_deg.to_radians();
        let from = rot_pt(vec2(caret_x, pos.y - caret_h / 2.0), pos, a);
        let to = rot_pt(vec2(caret_x, pos.y + caret_h / 2.0), pos, a);
        draw_line(from.x, from.y, to.x, to.y, (size * 0.055).max(1.0), color);
    }
}

/// The four-offset faint copies that give text its neon bloom. Cheap enough
/// at typical entity counts; gated on `glow > 0` by the caller.
#[allow(clippy::too_many_arguments)]
fn draw_text_glow(
    text: &str,
    pos: Vec2,
    size: f32,
    raster: f32,
    color: Color,
    opacity: f32,
    g: f32,
    fonts: &Fonts,
    kind: FontKind,
    rot_deg: f32,
    wrap: Option<f32>,
    align: Align,
) {
    let c = halo(color, opacity, g * 1.6);
    let d = (raster * 0.06).max(1.0);
    for off in [
        Vec2::new(d, 0.0),
        Vec2::new(-d, 0.0),
        Vec2::new(0.0, d),
        Vec2::new(0.0, -d),
    ] {
        draw_text_block(
            text,
            pos + off,
            size,
            raster,
            c,
            fonts,
            kind,
            rot_deg,
            wrap,
            align,
            1.0,
            false,
        );
    }
}

// ---- entities -----------------------------------------------------------------

fn flow_path_points(e: &Entity, view: &View) -> Option<Vec<Vec2>> {
    let p = view.xform(e.pos);
    let rad = e.rot.to_radians();
    Some(match &e.shape {
        Shape::Line { to } | Shape::Arrow { to } => {
            vec![p, rot_pt(view.xform(*to), p, rad)]
        }
        Shape::Curve { ctrl, to, .. } => bezier_pts(
            p,
            rot_pt(view.xform(*ctrl), p, rad),
            rot_pt(view.xform(*to), p, rad),
            48,
        ),
        Shape::Polyline { pts } => {
            let mut out: Vec<Vec2> = pts.iter().map(|q| view.xform(*q + e.pos)).collect();
            if e.rot.abs() > 1e-3 {
                let c = centroid(&out);
                for q in &mut out {
                    *q = rot_pt(*q, c, rad);
                }
            }
            out
        }
        Shape::Arc {
            r, start, sweep, ..
        } => {
            let n = ((sweep.abs() / 4.0).ceil() as usize).max(8);
            let a0 = (start + e.rot).to_radians();
            let da = sweep.to_radians() / n as f32;
            let rr = r * e.scale * view.k();
            (0..=n)
                .map(|i| {
                    let a = a0 + da * i as f32;
                    p + Vec2::new(a.cos(), a.sin()) * rr
                })
                .collect()
        }
        _ => return None,
    })
}

fn draw_flow_overlay(e: &Entity, view: &View, tpl: &style::Template) {
    draw_flow_channel(e, view, tpl, e.flow, false);
    draw_flow_channel(e, view, tpl, e.flow_back, true);
}

fn draw_flow_channel(e: &Entity, view: &View, tpl: &style::Template, value: f32, reverse: bool) {
    let cycle = value.rem_euclid(1.0);
    if value <= 1e-4 || cycle <= 1e-4 || cycle >= 0.9999 {
        return;
    }
    let phase = if reverse { 1.0 - cycle } else { cycle };
    let Some(pts) = flow_path_points(e, view) else {
        return;
    };
    if pts.len() < 2 {
        return;
    }
    let tail_phase = if reverse {
        (phase + 0.075).min(1.0)
    } else {
        (phase - 0.075).max(0.0)
    };
    let (tail, _) = path_point(&pts, tail_phase);
    let (head, _) = path_point(&pts, phase);
    let c = style::with_opacity(tpl.palette.fg, e.opacity);
    let width = (e.stroke.width * view.k()).max(2.0);
    draw_line(
        tail.x,
        tail.y,
        head.x,
        head.y,
        width * 4.5,
        halo(c, e.opacity, 1.8),
    );
    draw_line(tail.x, tail.y, head.x, head.y, width * 1.8, c);
    draw_circle(head.x, head.y, width * 3.2, halo(c, e.opacity, 2.2));
    draw_circle(head.x, head.y, width * 1.25, c);
}

/// Draw one entity through `view`.
pub fn draw_entity(e: &Entity, fonts: &Fonts, view: &View, tpl: &style::Template) {
    if e.opacity <= 0.001 || e.id == crate::movie::CAMERA_ID {
        return;
    }
    let trace = e.trace.clamp(0.0, 1.0);
    // retint the neon-baked colour to the active template's palette; bespoke
    // colours (hues, explicit RGB) pass through unchanged
    let base = tpl.palette.remap(e.color);
    let outline_base = tpl.palette.remap(e.stroke.outline_color.unwrap_or(e.color));
    // the template scales every entity's glow (0 = crisp, for print looks)
    let glow = e.glow * tpl.glow;
    // fills fade in as their outline is traced
    let fill = style::with_opacity(base, e.opacity * trace);
    let stroke_c = style::with_opacity(base, e.opacity);
    let outline = style::with_opacity(outline_base, e.opacity);
    let k = view.k();
    let dash = e.dash.map(|(on, off)| (on * k, off * k));
    let p = view.xform(e.pos);
    let width = e.stroke.width * k;
    let rad = e.rot.to_radians();
    let rotated = e.rot.abs() > 1e-3;
    // glow only once a shape/text is fully drawn — a halo over a half-traced
    // path reads as a rendering bug
    let glow_on = glow > 0.01 && trace >= 0.999;

    match &e.shape {
        Shape::Circle { r } => {
            let r = r * e.scale * k;
            if e.stroke.fill {
                if let Some(g) = &e.gradient {
                    let ring = circle_pts(p, r, 64);
                    let gp = GradPaint::resolve(g, tpl, e.opacity * trace, &ring);
                    draw_tris_grad(&fan_tris(p, &ring[..ring.len() - 1]), &gp);
                } else {
                    draw_circle(p.x, p.y, r, fill);
                }
            }
            if e.stroke.outline {
                if let (Some(g), false) = (&e.gradient, e.stroke.fill) {
                    let ring = circle_pts(p, r, 64);
                    let gp = GradPaint::resolve(g, tpl, e.opacity, &ring);
                    if glow_on {
                        draw_grad_path(&ring, trace, width * 3.0, &gp.halo(e.opacity, glow), None);
                    }
                    draw_grad_path(&ring, trace, width, &gp, None);
                } else if trace >= 1.0 {
                    if glow_on {
                        draw_circle_lines(p.x, p.y, r, width * 3.0, halo(outline, e.opacity, glow));
                    }
                    draw_circle_lines(p.x, p.y, r, width, outline);
                } else {
                    draw_path(&circle_pts(p, r, 64), trace, width, outline);
                }
            }
        }
        Shape::Rect { w, h } => {
            let (w, h) = (w * e.scale * k, h * e.scale * k);
            if !rotated {
                let (x, y) = (p.x - w / 2.0, p.y - h / 2.0);
                let rr = (e.corner_radius * e.scale * k).clamp(0.0, w.min(h) / 2.0);
                if e.stroke.fill {
                    if let Some(g) = &e.gradient {
                        let ring = if rr > 0.5 {
                            rounded_rect_pts(x, y, w, h, rr)
                        } else {
                            vec![
                                Vec2::new(x, y),
                                Vec2::new(x + w, y),
                                Vec2::new(x + w, y + h),
                                Vec2::new(x, y + h),
                            ]
                        };
                        let ring = subdivide_ring(&ring, (w.min(h) / 4.0).max(8.0));
                        let gp = GradPaint::resolve(g, tpl, e.opacity * trace, &ring);
                        draw_tris_grad(&fan_tris(Vec2::new(x + w * 0.5, y + h * 0.5), &ring), &gp);
                    } else if rr > 0.5 {
                        draw_rounded_rect(x, y, w, h, rr, fill);
                    } else {
                        draw_rectangle(x, y, w, h, fill);
                    }
                }
                if let (Some(g), false, true) = (&e.gradient, e.stroke.fill, e.stroke.outline) {
                    let mut ring = if rr > 0.5 {
                        rounded_rect_pts(x, y, w, h, rr)
                    } else {
                        vec![
                            Vec2::new(x, y),
                            Vec2::new(x + w, y),
                            Vec2::new(x + w, y + h),
                            Vec2::new(x, y + h),
                        ]
                    };
                    ring.push(ring[0]);
                    let gp = GradPaint::resolve(g, tpl, e.opacity, &ring);
                    if glow_on {
                        draw_grad_path(&ring, trace, width * 3.0, &gp.halo(e.opacity, glow), None);
                    }
                    draw_grad_path(&ring, trace, width * 2.0, &gp, None);
                } else if e.stroke.outline {
                    if trace >= 1.0 {
                        if glow_on {
                            if rr > 0.5 {
                                draw_rounded_rect_lines(
                                    x,
                                    y,
                                    w,
                                    h,
                                    rr,
                                    width * 5.0,
                                    halo(outline, e.opacity, glow),
                                );
                            } else {
                                draw_rectangle_lines(
                                    x,
                                    y,
                                    w,
                                    h,
                                    width * 5.0,
                                    halo(outline, e.opacity, glow),
                                );
                            }
                        }
                        if rr > 0.5 {
                            draw_rounded_rect_lines(x, y, w, h, rr, width * 2.0, outline);
                        } else {
                            draw_rectangle_lines(x, y, w, h, width * 2.0, outline);
                        }
                    } else {
                        let c = [
                            Vec2::new(x, y),
                            Vec2::new(x + w, y),
                            Vec2::new(x + w, y + h),
                            Vec2::new(x, y + h),
                            Vec2::new(x, y),
                        ];
                        draw_path(&c, trace, width, outline);
                    }
                }
            } else {
                // rotated: draw as a quad spun about the centre `p`
                let (hw, hh) = (w / 2.0, h / 2.0);
                let corner = |dx: f32, dy: f32| rot_pt(Vec2::new(p.x + dx, p.y + dy), p, rad);
                let cs = [
                    corner(-hw, -hh),
                    corner(hw, -hh),
                    corner(hw, hh),
                    corner(-hw, hh),
                ];
                if e.stroke.fill {
                    if let Some(g) = &e.gradient {
                        let ring = subdivide_ring(&cs, (w.min(h) / 4.0).max(8.0));
                        let gp = GradPaint::resolve(g, tpl, e.opacity * trace, &ring);
                        draw_tris_grad(&fan_tris(p, &ring), &gp);
                    } else {
                        draw_triangle(cs[0], cs[1], cs[2], fill);
                        draw_triangle(cs[0], cs[2], cs[3], fill);
                    }
                }
                if e.stroke.outline {
                    let closed = [cs[0], cs[1], cs[2], cs[3], cs[0]];
                    if let (Some(g), false) = (&e.gradient, e.stroke.fill) {
                        let gp = GradPaint::resolve(g, tpl, e.opacity, &closed);
                        if glow_on {
                            draw_grad_path(
                                &closed,
                                trace,
                                width * 3.0,
                                &gp.halo(e.opacity, glow),
                                None,
                            );
                        }
                        draw_grad_path(&closed, trace, width, &gp, None);
                    } else {
                        if glow_on {
                            draw_path(&closed, trace, width * 3.0, halo(outline, e.opacity, glow));
                        }
                        draw_path(&closed, trace, width, outline);
                    }
                }
            }
        }
        Shape::Line { to } => {
            let q = rot_pt(view.xform(*to), p, rad);
            if let Some(g) = &e.gradient {
                let pts = [p, q];
                let gp = GradPaint::resolve(g, tpl, e.opacity, &pts);
                if glow_on {
                    draw_grad_path(
                        &pts,
                        trace,
                        width * e.scale * 3.0,
                        &gp.halo(e.opacity, glow),
                        dash,
                    );
                }
                draw_grad_path(&pts, trace, width * e.scale, &gp, dash);
            } else {
                if glow_on {
                    draw_styled_path(
                        &[p, q],
                        trace,
                        width * e.scale * 3.0,
                        halo(stroke_c, e.opacity, glow),
                        dash,
                    );
                }
                draw_styled_path(&[p, q], trace, width * e.scale, stroke_c, dash);
            }
            // a tangent/normal carries a contact dot at the touch point (the
            // segment's midpoint, since the segment is centred on it)
            if matches!(
                e.graph_view,
                Some(crate::primitives::GraphView::Tangent { .. })
                    | Some(crate::primitives::GraphView::Normal { .. })
            ) {
                let mid = (p + q) * 0.5;
                let r = 5.0 * k;
                if glow_on {
                    draw_circle(mid.x, mid.y, r * 1.9, halo(stroke_c, e.opacity, glow));
                }
                draw_circle(mid.x, mid.y, r, stroke_c);
            }
        }
        Shape::Coil { to, turns } => {
            let q = rot_pt(view.xform(*to), p, rad);
            let pts = coil_points(p, q, *turns);
            if let Some(g) = &e.gradient {
                let gp = GradPaint::resolve(g, tpl, e.opacity, &pts);
                if glow_on {
                    draw_grad_path(
                        &pts,
                        trace,
                        width * e.scale * 3.0,
                        &gp.halo(e.opacity, glow),
                        dash,
                    );
                }
                draw_grad_path(&pts, trace, width * e.scale, &gp, dash);
            } else {
                if glow_on {
                    draw_styled_path(
                        &pts,
                        trace,
                        width * e.scale * 3.0,
                        halo(stroke_c, e.opacity, glow),
                        dash,
                    );
                }
                draw_styled_path(&pts, trace, width * e.scale, stroke_c, dash);
            }
        }
        Shape::Arrow { to } => {
            let pts = [p, rot_pt(view.xform(*to), p, rad)];
            if let Some(g) = &e.gradient {
                let gp = GradPaint::resolve(g, tpl, e.opacity, &pts);
                if glow_on {
                    draw_grad_stroke_path(
                        &pts,
                        trace,
                        width * e.scale * 3.0,
                        &gp.halo(e.opacity, glow),
                        true,
                        dash,
                    );
                }
                draw_grad_stroke_path(&pts, trace, width * e.scale, &gp, true, dash);
            } else {
                if glow_on {
                    draw_stroke_path(
                        &pts,
                        trace,
                        width * e.scale * 3.0,
                        halo(stroke_c, e.opacity, glow),
                        true,
                        dash,
                    );
                }
                draw_stroke_path(&pts, trace, width * e.scale, stroke_c, true, dash);
            }
        }
        Shape::Curve { ctrl, to, arrow } => {
            let ctrl_p = rot_pt(view.xform(*ctrl), p, rad);
            let to_p = rot_pt(view.xform(*to), p, rad);
            let pts = bezier_pts(p, ctrl_p, to_p, 32);
            if let Some(g) = &e.gradient {
                let gp = GradPaint::resolve(g, tpl, e.opacity, &pts);
                if glow_on {
                    draw_grad_stroke_path(
                        &pts,
                        trace,
                        width * e.scale * 3.0,
                        &gp.halo(e.opacity, glow),
                        *arrow,
                        dash,
                    );
                }
                draw_grad_stroke_path(&pts, trace, width * e.scale, &gp, *arrow, dash);
            } else {
                if glow_on {
                    draw_stroke_path(
                        &pts,
                        trace,
                        width * e.scale * 3.0,
                        halo(stroke_c, e.opacity, glow),
                        *arrow,
                        dash,
                    );
                }
                draw_stroke_path(&pts, trace, width * e.scale, stroke_c, *arrow, dash);
            }
        }
        Shape::Polygon { pts } => {
            if pts.len() < 3 {
                return;
            }
            let mut phys: Vec<Vec2> = pts.iter().map(|&q| view.xform(q + e.pos)).collect();
            if rotated {
                let c = centroid(&phys);
                for q in &mut phys {
                    *q = rot_pt(*q, c, rad);
                }
            }
            if e.stroke.fill {
                if let Some(g) = &e.gradient {
                    let gp = GradPaint::resolve(g, tpl, e.opacity * trace, &phys);
                    let tris: Vec<[Vec2; 3]> = (1..phys.len() - 1)
                        .map(|i| [phys[0], phys[i], phys[i + 1]])
                        .collect();
                    draw_tris_grad(&tris, &gp);
                } else {
                    for i in 1..phys.len() - 1 {
                        draw_triangle(phys[0], phys[i], phys[i + 1], fill);
                    }
                }
            }
            if e.stroke.outline {
                let mut closed = phys.clone();
                closed.push(phys[0]);
                if let (Some(g), false) = (&e.gradient, e.stroke.fill) {
                    let gp = GradPaint::resolve(g, tpl, e.opacity, &closed);
                    if glow_on {
                        draw_grad_path(&closed, trace, width * 3.0, &gp.halo(e.opacity, glow), None);
                    }
                    draw_grad_path(&closed, trace, width, &gp, None);
                } else {
                    if glow_on {
                        draw_path(&closed, trace, width * 3.0, halo(outline, e.opacity, glow));
                    }
                    draw_path(&closed, trace, width, outline);
                }
            }
        }
        Shape::Polyline { pts } => {
            if pts.len() < 2 {
                return;
            }
            let mut phys: Vec<Vec2> = pts.iter().map(|&q| view.xform(q + e.pos)).collect();
            if rotated {
                let c = centroid(&phys);
                for q in &mut phys {
                    *q = rot_pt(*q, c, rad);
                }
            }
            if let Some(g) = &e.gradient {
                let gp = GradPaint::resolve(g, tpl, e.opacity, &phys);
                if glow_on {
                    draw_grad_path(
                        &phys,
                        trace,
                        width * e.scale * 3.0,
                        &gp.halo(e.opacity, glow),
                        dash,
                    );
                }
                draw_grad_path(&phys, trace, width * e.scale, &gp, dash);
            } else {
                if glow_on {
                    draw_styled_path(
                        &phys,
                        trace,
                        width * e.scale * 3.0,
                        halo(stroke_c, e.opacity, glow),
                        dash,
                    );
                }
                draw_styled_path(&phys, trace, width * e.scale, stroke_c, dash);
            }
        }
        Shape::Arc {
            r,
            inner,
            start,
            sweep,
        } => {
            let ro = r * e.scale * k;
            let ri = inner * e.scale * k;
            let sweep = sweep.clamp(-360.0, 360.0);
            let n = ((sweep.abs() / 6.0).ceil() as usize).max(2);
            let a0 = (start + e.rot).to_radians();
            let da = sweep.to_radians() / n as f32;
            let at = |rad_len: f32, i: usize| {
                let a = a0 + da * i as f32;
                Vec2::new(p.x + a.cos() * rad_len, p.y + a.sin() * rad_len)
            };
            let outer: Vec<Vec2> = (0..=n).map(|i| at(ro, i)).collect();
            let inner_pts: Vec<Vec2> = (0..=n).map(|i| at(ri, i)).collect();
            let full = sweep.abs() >= 359.999;
            let solid = ri <= 0.5;

            if e.stroke.fill {
                if let Some(g) = &e.gradient {
                    let mut tris: Vec<[Vec2; 3]> = Vec::with_capacity(n * 2);
                    if solid {
                        for i in 0..n {
                            tris.push([p, outer[i], outer[i + 1]]);
                        }
                    } else {
                        for i in 0..n {
                            tris.push([inner_pts[i], outer[i], outer[i + 1]]);
                            tris.push([inner_pts[i], outer[i + 1], inner_pts[i + 1]]);
                        }
                    }
                    let gp = GradPaint::resolve(g, tpl, e.opacity * trace, &outer);
                    draw_tris_grad(&tris, &gp);
                } else if solid {
                    for i in 0..n {
                        draw_triangle(p, outer[i], outer[i + 1], fill);
                    }
                } else {
                    for i in 0..n {
                        draw_triangle(inner_pts[i], outer[i], outer[i + 1], fill);
                        draw_triangle(inner_pts[i], outer[i + 1], inner_pts[i + 1], fill);
                    }
                }
                // sector boundary
                if e.stroke.outline {
                    let mut b: Vec<Vec2> = Vec::new();
                    if full {
                        b = outer.clone();
                    } else if solid {
                        b.push(p);
                        b.extend_from_slice(&outer);
                        b.push(p);
                    } else {
                        b.extend_from_slice(&outer);
                        b.extend(inner_pts.iter().rev());
                        b.push(outer[0]);
                    }
                    if glow_on {
                        draw_path(&b, trace, width * 3.0, halo(outline, e.opacity, glow));
                    }
                    draw_path(&b, trace, width, outline);
                    if full && !solid {
                        // inner ring for a full annulus
                        if glow_on {
                            draw_path(
                                &inner_pts,
                                trace,
                                width * 3.0,
                                halo(outline, e.opacity, glow),
                            );
                        }
                        draw_path(&inner_pts, trace, width, outline);
                    }
                }
            } else if let Some(g) = &e.gradient {
                // plain arc with a gradient stroke
                let gp = GradPaint::resolve(g, tpl, e.opacity, &outer);
                if glow_on {
                    draw_grad_path(
                        &outer,
                        trace,
                        width * e.scale * 3.0,
                        &gp.halo(e.opacity, glow),
                        dash,
                    );
                }
                draw_grad_path(&outer, trace, width * e.scale, &gp, dash);
            } else {
                // plain arc: just the outer curve, no radii
                if glow_on {
                    draw_styled_path(
                        &outer,
                        trace,
                        width * e.scale * 3.0,
                        halo(stroke_c, e.opacity, glow),
                        dash,
                    );
                }
                draw_styled_path(&outer, trace, width * e.scale, stroke_c, dash);
            }
        }
        Shape::Region { tris, rings } => {
            // world → (optional scale/rotate about the region's centroid) →
            // physical. Centroid taken from the first outline ring.
            let cw = rings.first().map(|r| centroid(r)).unwrap_or(Vec2::ZERO) + e.pos;
            let place = |q: Vec2| -> Vec2 {
                let mut w = q + e.pos;
                if e.scale != 1.0 || rotated {
                    w = cw + (w - cw) * e.scale;
                    if rotated {
                        w = rot_pt(w, cw, rad);
                    }
                }
                view.xform(w)
            };
            if e.stroke.fill {
                if let Some(g) = &e.gradient {
                    let placed: Vec<[Vec2; 3]> = tris
                        .iter()
                        .map(|t| [place(t[0]), place(t[1]), place(t[2])])
                        .collect();
                    let flat: Vec<Vec2> = placed.iter().flatten().copied().collect();
                    let gp = GradPaint::resolve(g, tpl, e.opacity * trace, &flat);
                    draw_tris_grad(&placed, &gp);
                } else {
                    for t in tris {
                        draw_triangle(place(t[0]), place(t[1]), place(t[2]), fill);
                    }
                }
            }
            if e.stroke.outline {
                for ring in rings {
                    if ring.len() < 2 {
                        continue;
                    }
                    let mut phys: Vec<Vec2> = ring.iter().map(|&q| place(q)).collect();
                    phys.push(phys[0]);
                    if let (Some(g), false) = (&e.gradient, e.stroke.fill) {
                        let gp = GradPaint::resolve(g, tpl, e.opacity, &phys);
                        if glow_on {
                            draw_grad_path(&phys, trace, width * 3.0, &gp.halo(e.opacity, glow), None);
                        }
                        draw_grad_path(&phys, trace, width, &gp, None);
                    } else {
                        if glow_on {
                            draw_path(&phys, trace, width * 3.0, halo(outline, e.opacity, glow));
                        }
                        draw_path(&phys, trace, width, outline);
                    }
                }
            }
        }
        Shape::Text { content, size } => {
            let phys_size = size * e.scale * k;
            let raster = size * view.ss;
            let wrap = e.wrap.map(|w| w * k);
            if glow_on {
                draw_text_glow(
                    content, p, phys_size, raster, stroke_c, e.opacity, glow, fonts, e.font, e.rot,
                    wrap, e.align,
                );
            }
            // rasterize at the zoom-independent size so camera zooms and
            // pulses scale glyphs smoothly instead of re-rasterizing
            draw_text_block(
                content,
                p,
                phys_size,
                raster,
                stroke_c,
                fonts,
                e.font,
                e.rot,
                wrap,
                e.align,
                trace,
                e.type_cursor,
            );
        }
        Shape::Image { path, w, h, tint } => {
            let (dw, dh) = (w * e.scale * k, h * e.scale * k);
            // honour the entity's alignment: Left anchors the LEFT edge at `pos`
            // (a `$…$` label that sat right of a badge stays put), Center centres.
            let x = match e.align {
                Align::Left => p.x,
                Align::Center => p.x - dw / 2.0,
            };
            let y = p.y - dh / 2.0;
            if let Some(tex) = get_texture(path) {
                // Equation and other tintable images use the same semantic
                // template remap as text and vector shapes. Without this,
                // FG-baked formula labels stay pale on light templates.
                let tint_color = if *tint { base } else { WHITE };
                draw_texture_ex(
                    &tex,
                    x,
                    y,
                    style::with_opacity(tint_color, e.opacity),
                    DrawTextureParams {
                        dest_size: Some(vec2(dw, dh)),
                        rotation: rad,
                        ..Default::default()
                    },
                );
            } else {
                // missing/unloaded → a crossed placeholder box (reads as a slot)
                let lw = k.max(1.0);
                draw_rectangle_lines(x, y, dw, dh, 2.0 * lw, outline);
                draw_line(x, y, x + dw, y + dh, lw, outline);
                draw_line(x + dw, y, x, y + dh, lw, outline);
            }
        }
        Shape::RichText { runs, size } => {
            // Mixed text + inline math, baseline-aligned, with word-WRAP: text
            // breaks at spaces, math runs are atomic. Text draws with the entity
            // font/colour; math runs are pre-rendered PNGs tinted by the colour.
            let scale = e.scale * k;
            let phys = size * scale;
            let raster = (size * view.ss).max(1.0);
            let font_size = raster.round() as u16;
            let font_scale = phys.max(0.01) / font_size as f32;
            let measure =
                |t: &str| measure_resolved_text(t, fonts, e.font, font_size, font_scale).width;
            let space_w = measure(" ");
            let wrap_w = e.wrap.map(|w| w * k).unwrap_or(f32::INFINITY);

            // 1) tokenise the runs into words / spaces / math atoms / hard breaks
            enum Tok {
                Word(String),
                Space,
                Break,       // `\n` — a hard line break
                Math(usize), // index into `runs`
            }
            let mut toks: Vec<Tok> = Vec::new();
            for (ri, r) in runs.iter().enumerate() {
                match r {
                    TextRun::Text(s) => {
                        // `\n` (literal backslash-n) forces a line break
                        for (si, seg) in s.split("\\n").enumerate() {
                            if si > 0 {
                                toks.push(Tok::Break);
                            }
                            let mut word = String::new();
                            for ch in seg.chars() {
                                if ch.is_whitespace() {
                                    if !word.is_empty() {
                                        toks.push(Tok::Word(std::mem::take(&mut word)));
                                    }
                                    if !matches!(toks.last(), Some(Tok::Space)) {
                                        toks.push(Tok::Space);
                                    }
                                } else {
                                    word.push(ch);
                                }
                            }
                            if !word.is_empty() {
                                toks.push(Tok::Word(word));
                            }
                        }
                    }
                    TextRun::Math { .. } => toks.push(Tok::Math(ri)),
                }
            }
            let width_of = |t: &Tok| match t {
                Tok::Word(w) => measure(w),
                Tok::Space => space_w,
                Tok::Break => 0.0,
                Tok::Math(i) => match &runs[*i] {
                    TextRun::Math { w, .. } => w * scale,
                    _ => 0.0,
                },
            };

            // 2) greedy line-break (break before a word/math that overflows; a
            //    trailing space at the break is dropped)
            let mut lines: Vec<Vec<usize>> = vec![vec![]];
            let mut cur_w = 0.0;
            for (ti, t) in toks.iter().enumerate() {
                let tw = width_of(t);
                let line = lines.last_mut().unwrap();
                if matches!(t, Tok::Break) {
                    // drop a trailing space, then start a fresh line
                    if matches!(line.last().map(|&j| &toks[j]), Some(Tok::Space)) {
                        line.pop();
                    }
                    lines.push(Vec::new());
                    cur_w = 0.0;
                } else if matches!(t, Tok::Space) {
                    if !line.is_empty() {
                        line.push(ti);
                        cur_w += tw;
                    }
                } else {
                    if !line.is_empty() && cur_w + tw > wrap_w {
                        if matches!(
                            lines.last().unwrap().last().map(|&j| &toks[j]),
                            Some(Tok::Space)
                        ) {
                            lines.last_mut().unwrap().pop();
                        }
                        lines.push(vec![ti]);
                        cur_w = tw;
                    } else {
                        lines.last_mut().unwrap().push(ti);
                        cur_w += tw;
                    }
                }
            }

            // 3) PER-LINE metrics: each line is only as tall as its own content
            //    (text ascent/descent, or a tall fraction/integral where present),
            //    so text-only lines stay tight and math lines get just enough room.
            let ascent = measure_resolved_text("Xg", fonts, e.font, font_size, font_scale).offset_y;
            let descent = phys * 0.22;
            let gap = phys * 0.16;
            let metrics: Vec<(f32, f32)> = lines
                .iter()
                .map(|line| {
                    let (mut above, mut below) = (ascent, descent);
                    for &j in line {
                        match &toks[j] {
                            Tok::Word(word) => {
                                let dims = measure_resolved_text(
                                    word, fonts, e.font, font_size, font_scale,
                                );
                                above = above.max(dims.offset_y);
                                below = below.max((dims.height - dims.offset_y).max(0.0));
                            }
                            Tok::Math(i) => {
                                if let TextRun::Math { h, baseline, .. } = &runs[*i] {
                                    above = above.max(baseline * scale);
                                    below = below.max((h - baseline) * scale);
                                }
                            }
                            Tok::Space | Tok::Break => {}
                        }
                    }
                    (above, below)
                })
                .collect();
            let total_h: f32 = metrics.iter().map(|(a, b)| a + b + gap).sum();
            let mut y = p.y - total_h / 2.0;
            for (li, line) in lines.iter().enumerate() {
                let (above, below) = metrics[li];
                let line_w: f32 = line.iter().map(|&j| width_of(&toks[j])).sum();
                let mut x = match e.align {
                    Align::Center => p.x - line_w / 2.0,
                    Align::Left => p.x,
                };
                let baseline_y = y + above;
                y += above + below + gap;
                for &j in line {
                    match &toks[j] {
                        Tok::Break => {} // never enters a line; here for exhaustiveness
                        Tok::Space => x += space_w,
                        Tok::Word(w) => {
                            draw_resolved_text(
                                w, x, baseline_y, fonts, e.font, font_size, font_scale, 0.0,
                                stroke_c,
                            );
                            x += measure(w);
                        }
                        Tok::Math(i) => {
                            if let TextRun::Math {
                                path,
                                w,
                                h,
                                baseline,
                            } = &runs[*i]
                            {
                                let (dw, dh) = (w * scale, h * scale);
                                if let Some(tex) = get_texture(path) {
                                    draw_texture_ex(
                                        &tex,
                                        x,
                                        baseline_y - baseline * scale,
                                        stroke_c,
                                        DrawTextureParams {
                                            dest_size: Some(vec2(dw, dh)),
                                            ..Default::default()
                                        },
                                    );
                                }
                                x += dw;
                            }
                        }
                    }
                }
            }
        }
    }
    draw_flow_overlay(e, view, tpl);
}

/// Draw a whole scene in z-order (stable within equal z).
pub fn draw_scene(scene: &Scene, fonts: &Fonts, view: &View, tpl: &style::Template) {
    let mut order: Vec<usize> = (0..scene.entities.len()).collect();
    order.sort_by_key(|&i| scene.entities[i].z);
    let sticky_view = View {
        cam: view.center,
        zoom: 1.0,
        ..*view
    };
    for i in order {
        let entity = &scene.entities[i];
        draw_entity(
            entity,
            fonts,
            if entity.sticky { &sticky_view } else { view },
            tpl,
        );
    }
}

// ---- terminal chrome ------------------------------------------------------

/// Draw (per the template's chrome level) the page chrome: a glowing border
/// with corner brackets, three "window dots", the title, a
/// masthead, and a two-tone rule. `Chrome::None` (used by default `mono` and by
/// `plain`) draws only the background — a blank screen. It lives in world coordinates,
/// so camera moves treat the chrome as part of the page rather than sticky UI.
pub fn draw_page_chrome(
    tpl: &style::Template,
    title: &str,
    w: f32,
    h: f32,
    fonts: &Fonts,
    view: &View,
) {
    if tpl.chrome == style::Chrome::None {
        return; // no chrome: blank screen, content only
    }
    let full = tpl.chrome == style::Chrome::Full;
    let pal = tpl.palette;
    let k = view.k();
    let line = |a: Vec2, b: Vec2, width: f32, color: Color| {
        let a = view.xform(a);
        let b = view.xform(b);
        draw_line(a.x, a.y, b.x, b.y, width * k, color);
    };

    // --- Full-only: border, corner brackets, window dots, title ---
    if full {
        // outer border: faint glowing cyan frame
        let (bx, by, bw, bh) = (18.0, 18.0, w - 36.0, h - 36.0);
        {
            let p = view.xform(Vec2::new(bx, by));
            draw_rectangle_lines(p.x, p.y, bw * k, bh * k, 4.0 * k, halo(pal.cyan, 1.0, 1.0));
            draw_rectangle_lines(
                p.x,
                p.y,
                bw * k,
                bh * k,
                1.5 * k,
                style::with_opacity(pal.cyan, 0.5),
            );
        }
        // corner brackets, brighter neon
        let br = 26.0;
        for (cx, cy, sx, sy) in [
            (bx, by, 1.0, 1.0),
            (bx + bw, by, -1.0, 1.0),
            (bx, by + bh, 1.0, -1.0),
            (bx + bw, by + bh, -1.0, -1.0),
        ] {
            line(
                Vec2::new(cx, cy),
                Vec2::new(cx + br * sx, cy),
                2.5,
                pal.cyan,
            );
            line(
                Vec2::new(cx, cy),
                Vec2::new(cx, cy + br * sy),
                2.5,
                pal.cyan,
            );
        }

        // three window dots, top-left inside the frame
        for (i, c) in [pal.magenta, pal.lime, pal.cyan].iter().enumerate() {
            let d = view.xform(Vec2::new(44.0 + i as f32 * 22.0, 50.0));
            draw_circle(d.x, d.y, 6.0 * k, halo(*c, 1.0, 1.4));
            draw_circle(d.x, d.y, 4.0 * k, *c);
        }

        // title, centred, glowing display mono, uppercase
        let title_upper = title.to_uppercase();
        let tpos = view.xform(Vec2::new(w / 2.0, 58.0));
        for off in [
            Vec2::new(2.0, 0.0),
            Vec2::new(-2.0, 0.0),
            Vec2::new(0.0, 2.0),
            Vec2::new(0.0, -2.0),
        ] {
            draw_text_block(
                &title_upper,
                tpos + off,
                34.0 * k,
                34.0 * view.ss,
                halo(pal.cyan, 1.0, 1.6),
                fonts,
                FontKind::Display,
                0.0,
                None,
                Align::Center,
                1.0,
                false,
            );
        }
        draw_text_block(
            &title_upper,
            tpos,
            34.0 * k,
            34.0 * view.ss,
            pal.cyan,
            fonts,
            FontKind::Display,
            0.0,
            None,
            Align::Center,
            1.0,
            false,
        );
    } // end Full-only

    // masthead: shell prompt (left) + status (right), dim mono — Full & Minimal
    let fs = (14.0 * view.ss).round() as u16;
    let fscale = 14.0 * k / fs as f32;
    if !tpl.masthead_left.is_empty() {
        let left = view.xform(Vec2::new(150.0, 54.0));
        draw_resolved_text(
            &tpl.masthead_left,
            left.x,
            left.y,
            fonts,
            FontKind::Mono,
            fs,
            fscale,
            0.0,
            pal.dim,
        );
    }
    if !tpl.masthead_right.is_empty() {
        let rdims = measure_resolved_text(&tpl.masthead_right, fonts, FontKind::Mono, fs, fscale);
        let right = view.xform(Vec2::new(w - 44.0, 54.0));
        draw_resolved_text(
            &tpl.masthead_right,
            right.x - rdims.width,
            right.y,
            fonts,
            FontKind::Mono,
            fs,
            fscale,
            0.0,
            pal.dim,
        );
    }

    // two-tone synthwave rule under the header (Full only)
    if full {
        line(
            Vec2::new(40.0, 84.0),
            Vec2::new(w - 40.0, 84.0),
            2.0,
            pal.cyan,
        );
        line(
            Vec2::new(40.0, 89.0),
            Vec2::new(w - 40.0, 89.0),
            1.0,
            pal.magenta,
        );
    }
}

/// Clear the active render target to the template background.
pub fn clear_page_background(tpl: &style::Template) {
    clear_background(tpl.palette.bg);
}

#[cfg(test)]
mod tests {
    use super::{lerp_color, quantity_params, sample_stops, subdivide_tris, GradPaint};
    use crate::primitives::GradientKind;
    use macroquad::prelude::{Color, Vec2};

    const A: Color = Color::new(0.0, 0.0, 0.0, 1.0);
    const B: Color = Color::new(1.0, 1.0, 1.0, 1.0);

    fn gp(kind: GradientKind) -> GradPaint {
        // bounds 200×400 centred at (200,400); rmax = the far corner, as if
        // the geometry were the bounds rectangle itself
        GradPaint {
            stops: vec![A, B],
            kind,
            min: Vec2::new(100.0, 200.0),
            max: Vec2::new(300.0, 600.0),
            rmax: (Vec2::new(300.0, 600.0) - Vec2::new(200.0, 400.0)).length(),
        }
    }

    #[test]
    fn along_maps_arc_length_exactly() {
        let g = gp(GradientKind::Along);
        assert_eq!(g.at(Vec2::ZERO, 0.0), A);
        assert_eq!(g.at(Vec2::ZERO, 1.0), B);
        assert!((g.at(Vec2::ZERO, 0.5).r - 0.5).abs() < 1e-6);
    }

    #[test]
    fn linear_projects_onto_the_axis_over_the_bounds() {
        // 0° = left→right over x ∈ [100, 300]
        let g = gp(GradientKind::Linear(0.0));
        assert!((g.at(Vec2::new(100.0, 400.0), 0.0).r - 0.0).abs() < 1e-5);
        assert!((g.at(Vec2::new(300.0, 400.0), 0.0).r - 1.0).abs() < 1e-5);
        assert!((g.at(Vec2::new(200.0, 400.0), 0.0).r - 0.5).abs() < 1e-5);
        // 90° = top→bottom over y ∈ [200, 600] (screen y grows downward)
        let g = gp(GradientKind::Linear(90.0));
        assert!((g.at(Vec2::new(200.0, 200.0), 0.0).r - 0.0).abs() < 1e-5);
        assert!((g.at(Vec2::new(200.0, 600.0), 0.0).r - 1.0).abs() < 1e-5);
        // 270° runs the same axis in reverse: bottom→top
        let g = gp(GradientKind::Linear(270.0));
        assert!((g.at(Vec2::new(200.0, 600.0), 0.0).r - 0.0).abs() < 1e-4);
        assert!((g.at(Vec2::new(200.0, 200.0), 0.0).r - 1.0).abs() < 1e-4);
    }

    #[test]
    fn radial_runs_centre_to_farthest_corner() {
        let g = gp(GradientKind::Radial);
        let c = Vec2::new(200.0, 400.0);
        assert!((g.at(c, 0.0).r - 0.0).abs() < 1e-5);
        // a corner of the bounds is the farthest point → exactly `to`
        assert!((g.at(Vec2::new(300.0, 600.0), 0.0).r - 1.0).abs() < 1e-5);
    }

    #[test]
    fn lerp_clamps_out_of_range() {
        assert_eq!(lerp_color(A, B, -1.0), A);
        assert_eq!(lerp_color(A, B, 2.0), B);
    }

    #[test]
    fn multi_stop_sampling_hits_middle_stops_exactly() {
        let mid = Color::new(0.5, 0.0, 0.5, 1.0);
        let stops = [A, mid, B];
        assert_eq!(sample_stops(&stops, 0.0), A);
        assert_eq!(sample_stops(&stops, 0.5), mid);
        assert_eq!(sample_stops(&stops, 1.0), B);
        // quarter point = halfway into the first interval
        assert!((sample_stops(&stops, 0.25).r - 0.25).abs() < 1e-6);
    }

    #[test]
    fn speed_params_follow_segment_lengths() {
        // time-uniform points: long segments = fast, short = slow
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0), // fast
            Vec2::new(12.0, 0.0), // slow
            Vec2::new(22.0, 0.0), // fast
        ];
        let q = quantity_params(&pts, GradientKind::Speed).unwrap();
        assert_eq!(q.len(), 4);
        assert!((q[0] - 1.0).abs() < 1e-5, "first segment is the fastest");
        assert!(q[1] < q[0] && q[2] < q[3], "interior slows then speeds up");
        assert!((q[3] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn curvature_params_peak_at_the_bend() {
        // an L-shaped path: the corner point has all the curvature
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(20.0, 0.0),
            Vec2::new(20.0, 10.0),
            Vec2::new(20.0, 20.0),
        ];
        let q = quantity_params(&pts, GradientKind::Curvature).unwrap();
        let peak = q
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0;
        assert_eq!(peak, 2, "corner point carries the max curvature");
        assert!((q[1] - 0.0).abs() < 1e-5, "straight run is flat");
        // constant quantity (straight line) → all zeros, no fake variation
        let line = [Vec2::new(0.0, 0.0), Vec2::new(5.0, 0.0), Vec2::new(10.0, 0.0)];
        let q = quantity_params(&line, GradientKind::Curvature).unwrap();
        assert!(q.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn multi_stop_fills_subdivide_until_stops_resolve() {
        // one huge triangle spanning the whole gradient axis would swallow the
        // middle stop of a 3-stop fill without subdivision
        let g = GradPaint {
            stops: vec![A, Color::new(0.5, 0.0, 0.5, 1.0), B],
            kind: GradientKind::Linear(0.0),
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(800.0, 400.0),
            rmax: 1.0,
        };
        let tri = [[
            Vec2::new(0.0, 0.0),
            Vec2::new(800.0, 0.0),
            Vec2::new(0.0, 400.0),
        ]];
        let out = subdivide_tris(&tri, &g);
        assert!(out.len() > 8, "the spanning triangle must split");
        let max_span = 0.25 / 2.0;
        for t in &out {
            let ps = [g.param_of(t[0]), g.param_of(t[1]), g.param_of(t[2])];
            let span = ps.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
                - ps.iter().cloned().fold(f32::INFINITY, f32::min);
            // depth cap allows slight overshoot; spans must be near the target
            assert!(span <= max_span + 1e-3, "tri span {span} too wide");
        }
    }
}
