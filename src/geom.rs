//! 2D polygon boolean operations (union / intersection / difference / xor).
//!
//! Domain-agnostic core helper: turn a fillable [`Entity`] into polygons,
//! run a robust boolean op via [`geo`], then bake the result (which may have
//! holes and multiple disjoint pieces) into triangles for filling and outline
//! rings for stroking. The baked geometry is stored in [`Shape::Region`], so a
//! boolean result is a static shape you then animate as a whole — same model
//! as Manim's boolean VMobjects.

use geo::{BooleanOps, Coord, LineString, MultiPolygon, Polygon};
use macroquad::prelude::Vec2;

use crate::primitives::{Entity, Shape};

fn rad(deg: f32) -> f64 {
    (deg as f64).to_radians()
}

/// Rotate `(x, y)` about `(cx, cy)` by `deg` degrees.
fn rot(x: f64, y: f64, cx: f64, cy: f64, deg: f32) -> Coord<f64> {
    if deg == 0.0 {
        return Coord { x, y };
    }
    let (s, c) = rad(deg).sin_cos();
    let (dx, dy) = (x - cx, y - cy);
    Coord {
        x: cx + dx * c - dy * s,
        y: cy + dx * s + dy * c,
    }
}

fn ring(coords: Vec<Coord<f64>>) -> LineString<f64> {
    LineString::new(coords)
}

fn mp(poly: Polygon<f64>) -> MultiPolygon<f64> {
    MultiPolygon::new(vec![poly])
}

fn arc_coords(cx: f64, cy: f64, r: f64, start: f32, sweep: f32) -> Vec<Coord<f64>> {
    let n = ((sweep.abs() / 6.0).ceil() as usize).max(2);
    let a0 = rad(start);
    let da = rad(sweep) / n as f64;
    (0..=n)
        .map(|i| {
            let a = a0 + da * i as f64;
            Coord {
                x: cx + a.cos() * r,
                y: cy + a.sin() * r,
            }
        })
        .collect()
}

/// Convert a fillable entity to a [`MultiPolygon`] in world coordinates,
/// honouring its position and (for rect/polygon) rotation. Errors — with a
/// human message — for shapes that have no fillable area.
pub fn entity_to_multipolygon(e: &Entity) -> Result<MultiPolygon<f64>, String> {
    let (px, py) = (e.pos.x as f64, e.pos.y as f64);
    match &e.shape {
        Shape::Circle { r } => Ok(mp(Polygon::new(
            ring(arc_coords(px, py, *r as f64, 0.0, 360.0)),
            vec![],
        ))),
        Shape::Rect { w, h } => {
            let (hw, hh) = (*w as f64 / 2.0, *h as f64 / 2.0);
            let cs = vec![
                rot(px - hw, py - hh, px, py, e.rot),
                rot(px + hw, py - hh, px, py, e.rot),
                rot(px + hw, py + hh, px, py, e.rot),
                rot(px - hw, py + hh, px, py, e.rot),
            ];
            Ok(mp(Polygon::new(ring(cs), vec![])))
        }
        Shape::Polygon { pts } => {
            if pts.len() < 3 {
                return Err("polygon needs at least 3 points to boolean".into());
            }
            // centroid (for rotation) in world space
            let (mut sx, mut sy) = (0.0f64, 0.0f64);
            for p in pts {
                sx += (p.x + e.pos.x) as f64;
                sy += (p.y + e.pos.y) as f64;
            }
            let (cx, cy) = (sx / pts.len() as f64, sy / pts.len() as f64);
            let cs = pts
                .iter()
                .map(|p| rot((p.x + e.pos.x) as f64, (p.y + e.pos.y) as f64, cx, cy, e.rot))
                .collect();
            Ok(mp(Polygon::new(ring(cs), vec![])))
        }
        Shape::Arc {
            r,
            inner,
            start,
            sweep,
        } => {
            if !e.stroke.fill {
                return Err(
                    "can't boolean an arc line — use a filled `sector`/`annulus` instead".into(),
                );
            }
            let start = start + e.rot;
            let full = sweep.abs() >= 359.999;
            if *inner <= 0.5 {
                // solid sector / disc
                let mut cs = if full {
                    Vec::new()
                } else {
                    vec![Coord { x: px, y: py }]
                };
                cs.extend(arc_coords(px, py, *r as f64, start, *sweep));
                Ok(mp(Polygon::new(ring(cs), vec![])))
            } else if full {
                // annulus: outer ring with an inner hole
                let outer = arc_coords(px, py, *r as f64, 0.0, 360.0);
                let inner_ring = arc_coords(px, py, *inner as f64, 0.0, 360.0);
                Ok(mp(Polygon::new(ring(outer), vec![ring(inner_ring)])))
            } else {
                // annular sector: a closed band (outer arc + inner arc reversed)
                let mut cs = arc_coords(px, py, *r as f64, start, *sweep);
                let mut inner_arc = arc_coords(px, py, *inner as f64, start, *sweep);
                inner_arc.reverse();
                cs.extend(inner_arc);
                Ok(mp(Polygon::new(ring(cs), vec![])))
            }
        }
        Shape::Region { .. } => {
            Err("boolean operands must be basic shapes (circle, rect, polygon, sector) — nesting booleans isn't supported yet".into())
        }
        Shape::Line { .. } | Shape::Arrow { .. } | Shape::Curve { .. } | Shape::Coil { .. } => {
            Err("can't boolean a line/arrow — it has no area".into())
        }
        Shape::Polyline { .. } => Err("can't boolean a polyline — it has no area".into()),
        Shape::Text { .. } => Err("can't boolean text".into()),
        Shape::Image { .. } => Err("can't boolean an image".into()),
        Shape::RichText { .. } => Err("can't boolean rich text".into()),
    }
}

fn ring_pts(ls: &LineString<f64>) -> Vec<[f64; 2]> {
    let mut v: Vec<[f64; 2]> = ls.0.iter().map(|c| [c.x, c.y]).collect();
    // geo closes rings (first == last); drop the duplicate for triangulation
    if v.len() > 1 && v[0] == v[v.len() - 1] {
        v.pop();
    }
    v
}

fn to_vec2(p: [f64; 2]) -> Vec2 {
    Vec2::new(p[0] as f32, p[1] as f32)
}

fn triangulate(poly: &Polygon<f64>) -> Vec<[Vec2; 3]> {
    let mut flat: Vec<f64> = Vec::new();
    let mut holes: Vec<usize> = Vec::new();
    let ext = ring_pts(poly.exterior());
    for p in &ext {
        flat.push(p[0]);
        flat.push(p[1]);
    }
    let mut vcount = ext.len();
    for hole in poly.interiors() {
        let hp = ring_pts(hole);
        if hp.is_empty() {
            continue;
        }
        holes.push(vcount);
        for p in &hp {
            flat.push(p[0]);
            flat.push(p[1]);
        }
        vcount += hp.len();
    }
    let verts: Vec<Vec2> = flat
        .chunks(2)
        .map(|c| Vec2::new(c[0] as f32, c[1] as f32))
        .collect();
    match earcutr::earcut(&flat, &holes, 2) {
        Ok(idx) => idx
            .chunks(3)
            .filter(|t| t.len() == 3)
            .map(|t| [verts[t[0]], verts[t[1]], verts[t[2]]])
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn rings_of(poly: &Polygon<f64>) -> Vec<Vec<Vec2>> {
    let mut out = vec![ring_pts(poly.exterior()).into_iter().map(to_vec2).collect()];
    for h in poly.interiors() {
        out.push(ring_pts(h).into_iter().map(to_vec2).collect());
    }
    out
}

/// Run a boolean op (`"union"`/`"intersection"`/`"difference"`/`"xor"`) on two
/// multipolygons and bake the result into (fill triangles, outline rings).
pub fn boolean_region(
    op: &str,
    a: &MultiPolygon<f64>,
    b: &MultiPolygon<f64>,
) -> Result<(Vec<[Vec2; 3]>, Vec<Vec<Vec2>>), String> {
    let res: MultiPolygon<f64> = match op {
        "union" => a.union(b),
        "intersection" => a.intersection(b),
        "difference" => a.difference(b),
        "xor" => a.xor(b),
        other => return Err(format!("unknown boolean op `{other}`")),
    };
    if res.0.is_empty() {
        return Err("the result is empty — the shapes don't overlap the way this op needs".into());
    }
    let mut tris = Vec::new();
    let mut rings = Vec::new();
    for poly in &res.0 {
        tris.extend(triangulate(poly));
        rings.extend(rings_of(poly));
    }
    Ok((tris, rings))
}

/// The cross-section of a 2D fillable entity as (fill triangles, outline rings)
/// in world coordinates — the input `extrude3` sweeps into a solid. A boolean
/// [`Shape::Region`] reuses its already-baked data (so extruding a
/// union/difference/… yields a CSG solid); any other fillable shape
/// (rect/circle/sector/annulus/polygon) is triangulated on the fly.
pub fn cross_section(e: &Entity) -> Result<(Vec<[Vec2; 3]>, Vec<Vec<Vec2>>), String> {
    if let Shape::Region { tris, rings } = &e.shape {
        return Ok((tris.clone(), rings.clone()));
    }
    let mp = entity_to_multipolygon(e)?;
    let mut tris = Vec::new();
    let mut rings = Vec::new();
    for poly in &mp.0 {
        tris.extend(triangulate(poly));
        rings.extend(rings_of(poly));
    }
    if tris.is_empty() {
        return Err("this shape has no fillable area to extrude".into());
    }
    Ok((tris, rings))
}
