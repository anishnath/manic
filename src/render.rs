//! The macroquad draw pass: scene → pixels, plus the neon terminal chrome.
//!
//! New primitive = match arm in [`draw_entity`]. All world coordinates flow
//! through [`View::xform`]: supersampling scale + 2D camera today, a 3D
//! projection later. The neon identity comes from a soft glow (halo) pass
//! drawn behind fully-traced strokes and text.

use macroquad::prelude::*;

use crate::primitives::{Align, Entity, FontKind, Shape};
use crate::scene::Scene;
use crate::style::{self, Fonts};

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

fn font_of(fonts: &Fonts, kind: FontKind) -> Option<&Font> {
    match kind {
        FontKind::Display => fonts.display.as_ref(),
        FontKind::Mono => fonts.mono.as_ref(),
        FontKind::MonoBold => fonts.mono_bold.as_ref(),
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
fn draw_stroke_path(pts: &[Vec2], frac: f32, width: f32, color: Color, arrow: bool) {
    if !arrow {
        draw_path(pts, frac, width, color);
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
    draw_path(pts, body_frac, width, color);
    draw_head(tip, dir, width, color);
}

// ---- neon glow ------------------------------------------------------------

/// A soft, low-alpha version of `c` for the halo pass. `opacity` is the
/// entity's own alpha; `g` its glow multiplier.
fn halo(c: Color, opacity: f32, g: f32) -> Color {
    Color::new(c.r, c.g, c.b, (opacity * 0.18 * g).clamp(0.0, 1.0))
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

fn wrap_lines(
    text: &str,
    font: Option<&Font>,
    font_size: u16,
    font_scale: f32,
    max_w: f32,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let cand = if cur.is_empty() {
            word.to_string()
        } else {
            format!("{cur} {word}")
        };
        if !cur.is_empty() && measure_text(&cand, font, font_size, font_scale).width > max_w {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        } else {
            cur = cand;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
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
    font: Option<&Font>,
    rot_deg: f32,
    wrap: Option<f32>,
    align: Align,
    trace: f32,
    cursor: bool,
) {
    let font_size = raster.max(1.0).round() as u16;
    let font_scale = size.max(0.01) / font_size as f32;
    let lines = match wrap {
        Some(w) => wrap_lines(text, font, font_size, font_scale, w),
        None => vec![text.to_string()],
    };
    let total_chars: usize = lines.iter().map(|l| l.chars().count()).sum();
    let mut char_budget = if trace >= 1.0 {
        usize::MAX
    } else {
        (total_chars as f32 * trace.max(0.0)) as usize
    };

    let line_h = size * 1.4;
    let y0 = pos.y - line_h * (lines.len() as f32 - 1.0) / 2.0;
    for (i, line) in lines.iter().enumerate() {
        if char_budget == 0 {
            break;
        }
        let n_chars = line.chars().count();
        let (mut shown, typing_here): (String, bool) = if char_budget >= n_chars {
            char_budget -= n_chars;
            (line.clone(), false)
        } else {
            let s: String = line.chars().take(char_budget).collect();
            char_budget = 0;
            (s, true)
        };
        // a typewriter cursor rides the line being typed (or the last line once done)
        if cursor && (typing_here || i == lines.len() - 1) {
            shown.push('_');
        }
        let x = match align {
            Align::Center => {
                // anchor on the full line so typing doesn't shift the block
                let full = measure_text(line, font, font_size, font_scale);
                pos.x - full.width / 2.0
            }
            Align::Left => pos.x,
        };
        let dims = measure_text(&shown, font, font_size, font_scale);
        draw_text_ex(
            &shown,
            x,
            y0 + line_h * i as f32 + dims.offset_y / 2.0,
            TextParams {
                font,
                font_size,
                font_scale,
                font_scale_aspect: 1.0,
                rotation: rot_deg.to_radians(),
                color,
            },
        );
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
    font: Option<&Font>,
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
            font,
            rot_deg,
            wrap,
            align,
            1.0,
            false,
        );
    }
}

// ---- entities -----------------------------------------------------------------

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
                draw_circle(p.x, p.y, r, fill);
            }
            if e.stroke.outline {
                if trace >= 1.0 {
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
                if e.stroke.fill {
                    draw_rectangle(x, y, w, h, fill);
                }
                if e.stroke.outline {
                    if trace >= 1.0 {
                        if glow_on {
                            draw_rectangle_lines(x, y, w, h, width * 5.0, halo(outline, e.opacity, glow));
                        }
                        draw_rectangle_lines(x, y, w, h, width * 2.0, outline);
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
                let corner =
                    |dx: f32, dy: f32| rot_pt(Vec2::new(p.x + dx, p.y + dy), p, rad);
                let cs = [
                    corner(-hw, -hh),
                    corner(hw, -hh),
                    corner(hw, hh),
                    corner(-hw, hh),
                ];
                if e.stroke.fill {
                    draw_triangle(cs[0], cs[1], cs[2], fill);
                    draw_triangle(cs[0], cs[2], cs[3], fill);
                }
                if e.stroke.outline {
                    let closed = [cs[0], cs[1], cs[2], cs[3], cs[0]];
                    if glow_on {
                        draw_path(&closed, trace, width * 3.0, halo(outline, e.opacity, glow));
                    }
                    draw_path(&closed, trace, width, outline);
                }
            }
        }
        Shape::Line { to } => {
            let q = rot_pt(view.xform(*to), p, rad);
            if glow_on {
                draw_path(&[p, q], trace, width * e.scale * 3.0, halo(stroke_c, e.opacity, glow));
            }
            draw_path(&[p, q], trace, width * e.scale, stroke_c);
        }
        Shape::Arrow { to } => {
            let pts = [p, rot_pt(view.xform(*to), p, rad)];
            if glow_on {
                draw_stroke_path(&pts, trace, width * e.scale * 3.0, halo(stroke_c, e.opacity, glow), true);
            }
            draw_stroke_path(&pts, trace, width * e.scale, stroke_c, true);
        }
        Shape::Curve { ctrl, to, arrow } => {
            let ctrl_p = rot_pt(view.xform(*ctrl), p, rad);
            let to_p = rot_pt(view.xform(*to), p, rad);
            let pts = bezier_pts(p, ctrl_p, to_p, 32);
            if glow_on {
                draw_stroke_path(&pts, trace, width * e.scale * 3.0, halo(stroke_c, e.opacity, glow), *arrow);
            }
            draw_stroke_path(&pts, trace, width * e.scale, stroke_c, *arrow);
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
                for i in 1..phys.len() - 1 {
                    draw_triangle(phys[0], phys[i], phys[i + 1], fill);
                }
            }
            if e.stroke.outline {
                let mut closed = phys.clone();
                closed.push(phys[0]);
                if glow_on {
                    draw_path(&closed, trace, width * 3.0, halo(outline, e.opacity, glow));
                }
                draw_path(&closed, trace, width, outline);
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
            if glow_on {
                draw_path(&phys, trace, width * e.scale * 3.0, halo(stroke_c, e.opacity, glow));
            }
            draw_path(&phys, trace, width * e.scale, stroke_c);
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
                if solid {
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
                            draw_path(&inner_pts, trace, width * 3.0, halo(outline, e.opacity, glow));
                        }
                        draw_path(&inner_pts, trace, width, outline);
                    }
                }
            } else {
                // plain arc: just the outer curve, no radii
                if glow_on {
                    draw_path(&outer, trace, width * e.scale * 3.0, halo(stroke_c, e.opacity, glow));
                }
                draw_path(&outer, trace, width * e.scale, stroke_c);
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
                for t in tris {
                    draw_triangle(place(t[0]), place(t[1]), place(t[2]), fill);
                }
            }
            if e.stroke.outline {
                for ring in rings {
                    if ring.len() < 2 {
                        continue;
                    }
                    let mut phys: Vec<Vec2> = ring.iter().map(|&q| place(q)).collect();
                    phys.push(phys[0]);
                    if glow_on {
                        draw_path(&phys, trace, width * 3.0, halo(outline, e.opacity, glow));
                    }
                    draw_path(&phys, trace, width, outline);
                }
            }
        }
        Shape::Text { content, size } => {
            let phys_size = size * e.scale * k;
            let raster = size * view.ss;
            let font = font_of(fonts, e.font);
            let wrap = e.wrap.map(|w| w * k);
            if glow_on {
                draw_text_glow(
                    content, p, phys_size, raster, stroke_c, e.opacity, glow, font, e.rot,
                    wrap, e.align,
                );
            }
            // rasterize at the zoom-independent size so camera zooms and
            // pulses scale glyphs smoothly instead of re-rasterizing
            draw_text_block(
                content, p, phys_size, raster, stroke_c, font, e.rot, wrap, e.align, trace,
                e.type_cursor,
            );
        }
    }
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

/// Draw the background and (per the template's chrome level) the page chrome:
/// a glowing border with corner brackets, three "window dots", the title, a
/// masthead, and a two-tone rule. `Chrome::None` (the default `plain` template)
/// draws only the background — a blank screen. It lives in world coordinates,
/// so camera moves treat the chrome as part of the page rather than sticky UI.
pub fn draw_page_chrome(
    tpl: &style::Template,
    title: &str,
    w: f32,
    h: f32,
    fonts: &Fonts,
    view: &View,
) {
    clear_background(tpl.palette.bg);
    if tpl.chrome == style::Chrome::None {
        return; // plain: blank screen, content only
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
        draw_rectangle_lines(p.x, p.y, bw * k, bh * k, 1.5 * k, style::with_opacity(pal.cyan, 0.5));
    }
    // corner brackets, brighter neon
    let br = 26.0;
    for (cx, cy, sx, sy) in [
        (bx, by, 1.0, 1.0),
        (bx + bw, by, -1.0, 1.0),
        (bx, by + bh, 1.0, -1.0),
        (bx + bw, by + bh, -1.0, -1.0),
    ] {
        line(Vec2::new(cx, cy), Vec2::new(cx + br * sx, cy), 2.5, pal.cyan);
        line(Vec2::new(cx, cy), Vec2::new(cx, cy + br * sy), 2.5, pal.cyan);
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
    for off in [Vec2::new(2.0, 0.0), Vec2::new(-2.0, 0.0), Vec2::new(0.0, 2.0), Vec2::new(0.0, -2.0)] {
        draw_text_block(
            &title_upper,
            tpos + off,
            34.0 * k,
            34.0 * view.ss,
            halo(pal.cyan, 1.0, 1.6),
            fonts.display.as_ref(),
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
        fonts.display.as_ref(),
        0.0,
        None,
        Align::Center,
        1.0,
        false,
    );
    } // end Full-only

    // masthead: shell prompt (left) + status (right), dim mono — Full & Minimal
    if let Some(mono) = fonts.mono.as_ref() {
        let fs = (14.0 * view.ss).round() as u16;
        let fscale = 14.0 * k / fs as f32;
        if !tpl.masthead_left.is_empty() {
            let left = view.xform(Vec2::new(150.0, 54.0));
            draw_text_ex(
                &tpl.masthead_left,
                left.x,
                left.y,
                TextParams {
                    font: Some(mono),
                    font_size: fs,
                    font_scale: fscale,
                    font_scale_aspect: 1.0,
                    rotation: 0.0,
                    color: pal.dim,
                },
            );
        }
        if !tpl.masthead_right.is_empty() {
            let rdims = measure_text(&tpl.masthead_right, Some(mono), fs, fscale);
            let right = view.xform(Vec2::new(w - 44.0, 54.0));
            draw_text_ex(
                &tpl.masthead_right,
                right.x - rdims.width,
                right.y,
                TextParams {
                    font: Some(mono),
                    font_size: fs,
                    font_scale: fscale,
                    font_scale_aspect: 1.0,
                    rotation: 0.0,
                    color: pal.dim,
                },
            );
        }
    }

    // two-tone synthwave rule under the header (Full only)
    if full {
        line(Vec2::new(40.0, 84.0), Vec2::new(w - 40.0, 84.0), 2.0, pal.cyan);
        line(Vec2::new(40.0, 89.0), Vec2::new(w - 40.0, 89.0), 1.0, pal.magenta);
    }
}
