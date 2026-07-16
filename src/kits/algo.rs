//! The **algo kit**: data-structure & algorithm vocabulary — the second
//! domain, proving the core is domain-agnostic (this file + one line in
//! `default_registry`, zero core changes).
//!
//! v1 centrepiece: `graph` (Manim's Graph / DiGraph). Nodes are labelled
//! circles, edges are lines (`a-b`) or arrows (`a>b`) trimmed to node borders,
//! laid out by a named layout. Everything is tagged so a whole graph animates
//! with one verb via tag-broadcast (`draw(g.edges)`, `flash(g.nodes, cyan)`).

use std::collections::{HashMap, HashSet, VecDeque};

use macroquad::prelude::{Color, Vec2};

use crate::animate::act;
use crate::easing::Easing;
use crate::lang::diag::Error;
use crate::lang::lower::{resolve_color, Args, Registry};
use crate::layout;
use crate::primitives::{Align, Entity, FontKind, Link, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TextEvent, TrackSpec, Value};

/// `graph(id, "v1 v2 …", "a-b a>c …", layout, (cx,cy), scale, [radius])`
///
/// Vertices are whitespace-separated names → nodes `{id}.{name}` (+ labels).
/// Edges are whitespace/comma-separated: `a-b` (undirected line) or `a>b`
/// (directed arrow) → `{id}.{a}-{b}`. Layout ∈ circular | row | grid.
/// All entities carry tags `id`, `{id}.nodes`, `{id}.edges`.
fn c_graph(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let verts_str = a.text(1)?;
    let edges_str = a.text(2)?;
    let layout_name = a.ident(3)?;
    let center = a.pair(4)?;
    let scale = a.num(5)?;
    let radius = a.opt_num(6)?.unwrap_or(30.0);

    let names: Vec<String> = verts_str
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    if names.is_empty() {
        return Err(Error::new("graph needs at least one vertex", a.span_of(1)));
    }
    let n = names.len();

    let pos: Vec<Vec2> = match layout_name.as_str() {
        "circular" | "circle" | "ring" => layout::ring(n, center, scale),
        "row" | "line" => layout::row(n, center.y, center.x - scale, center.x + scale),
        "grid" => {
            let cols = (n as f32).sqrt().ceil().max(1.0) as usize;
            let rows = n.div_ceil(cols);
            layout::grid(
                cols,
                rows,
                Vec2::new(center.x - scale, center.y - scale),
                Vec2::new(center.x + scale, center.y + scale),
            )
        }
        other => {
            return Err(Error::new(
                format!("unknown layout `{other}` (try: circular, row, grid)"),
                a.span_of(3),
            ))
        }
    };

    let idx = |name: &str| names.iter().position(|x| x == name);
    let nodes_tag = format!("{id}.nodes");
    let edges_tag = format!("{id}.edges");

    // nodes + labels
    for (i, name) in names.iter().enumerate() {
        let nid = format!("{id}.{name}");
        let mut node = Entity::new(
            nid.clone(),
            Shape::Circle { r: radius },
            pos[i],
            style::PANEL,
        );
        node.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 2.5,
            outline_color: Some(style::CYAN),
        };
        node.z = 5;
        node.tags.push(id.clone());
        node.tags.push(nodes_tag.clone());
        s.add(node);

        let mut lbl = Entity::new(
            format!("{nid}.label"),
            Shape::Text {
                content: name.clone(),
                size: 22.0,
            },
            Vec2::ZERO,
            style::FG,
        );
        lbl.font = FontKind::MonoBold;
        lbl.z = 6;
        lbl.follow = Some((nid, Vec2::ZERO));
        lbl.tags.push(id.clone());
        s.add(lbl);
    }

    // edges, trimmed to node borders so arrowheads are visible. A trailing
    // `:w` gives the edge a weight (drawn as a midpoint label; used by dijkstra).
    let mut weights: Vec<String> = Vec::new();
    for tok in edges_str
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
    {
        let (epart, wt) = match tok.split_once(':') {
            Some((e, w)) => (
                e,
                Some(w.trim().parse::<f32>().map_err(|_| {
                    Error::new(format!("edge `{tok}` has a bad weight `{w}`"), a.span_of(2))
                })?),
            ),
            None => (tok, None),
        };
        let directed = epart.contains('>');
        let sep = if directed { '>' } else { '-' };
        let parts: Vec<&str> = epart.split(sep).collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(Error::new(
                format!("bad edge `{tok}` — use `a-b` (line) or `a>b` (arrow), opt. `:w`"),
                a.span_of(2),
            ));
        }
        let (u, v) = (parts[0], parts[1]);
        let iu = idx(u).ok_or_else(|| {
            Error::new(
                format!("edge `{tok}` uses unknown vertex `{u}`"),
                a.span_of(2),
            )
        })?;
        let iv = idx(v).ok_or_else(|| {
            Error::new(
                format!("edge `{tok}` uses unknown vertex `{v}`"),
                a.span_of(2),
            )
        })?;
        let dir = (pos[iv] - pos[iu]).normalize_or_zero();
        let from = pos[iu] + dir * radius;
        let to = pos[iv] - dir * radius;
        let shape = if directed {
            Shape::Arrow { to }
        } else {
            Shape::Line { to }
        };
        let mut edge = Entity::new(format!("{id}.{u}{sep}{v}"), shape, from, style::DIM);
        edge.stroke.width = 2.0;
        edge.z = 1;
        // track the two nodes so the edge reflows when they move
        edge.link = Some(Link {
            from: format!("{id}.{u}"),
            to: format!("{id}.{v}"),
            trim: radius,
        });
        edge.tags.push(id.clone());
        edge.tags.push(edges_tag.clone());
        s.add(edge);

        if let Some(w) = wt {
            let mid = (pos[iu] + pos[iv]) * 0.5;
            let perp = Vec2::new(-dir.y, dir.x) * 16.0;
            let mut lbl = Entity::new(
                format!("{id}.{u}{sep}{v}.w"),
                Shape::Text {
                    content: fmt_num(w),
                    size: 20.0,
                },
                mid + perp,
                style::CYAN,
            );
            lbl.font = FontKind::MonoBold;
            lbl.z = 7;
            lbl.tags.push(id.clone());
            s.add(lbl);
            weights.push(format!("{id}.{u}|{id}.{v}|{w}"));
        }
    }
    if !weights.is_empty() {
        s.occ.insert(format!("{id}#w"), weights);
    }
    Ok(())
}

/// `array(id, "5 2 8 1", (cx,cy), [cellw], [cellh])` — a row of value cells:
/// fixed box "slots" `{id}.box{k}` (tag `{id}.boxes`) plus value texts
/// `{id}.c{k}` (tag `{id}.cells`) centred in them. `c{k}` always names slot `k`,
/// so `flash(a.c2, cyan)` highlights a comparison and `say(a.c1, "3")` rewrites a
/// cell's value — drive a sort by `say`-ing each slot to its new value at every
/// step (see examples/bubble_sort.manic).
fn c_array(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let vals_str = a.text(1)?;
    let vals: Vec<String> = vals_str.split_whitespace().map(|w| w.to_string()).collect();
    let center = a.pair(2)?;
    let cw = a.opt_num(3)?.unwrap_or(74.0);
    let ch = a.opt_num(4)?.unwrap_or(74.0);
    if vals.is_empty() {
        return Err(Error::new("array has no values", a.span_of(1)));
    }
    let x0 = center.x - vals.len() as f32 * cw / 2.0;
    let boxes = format!("{id}.boxes");
    let cells = format!("{id}.cells");
    for (k, v) in vals.iter().enumerate() {
        let cx = x0 + (k as f32 + 0.5) * cw;
        let pos = Vec2::new(cx, center.y);
        // the fixed slot box
        let mut b = Entity::new(
            format!("{id}.box{k}"),
            Shape::Rect {
                w: cw * 0.9,
                h: ch * 0.9,
            },
            pos,
            style::PANEL,
        );
        b.stroke = StrokeStyle {
            fill: false,
            outline: true,
            width: 2.5,
            outline_color: Some(style::DIM),
        };
        b.z = 1;
        b.tags = vec![boxes.clone()];
        s.add(b);
        // the value (moves during a swap)
        let mut val = Entity::new(
            format!("{id}.c{k}"),
            Shape::Text {
                content: v.clone(),
                size: ch * 0.42,
            },
            pos,
            style::FG,
        );
        val.font = FontKind::MonoBold;
        val.z = 3;
        val.tags = vec![cells.clone()];
        s.add(val);
    }
    // seed slot occupancy so `swap`/`compare` track the live order
    s.occ.insert(
        id.clone(),
        (0..vals.len()).map(|k| format!("{id}.c{k}")).collect(),
    );
    Ok(())
}

/// `compare(a, i, j, [color])` — the comparison step of a sort: flash the values
/// *currently* in slots `i` and `j`. Reads the array's live occupancy, so after
/// swaps it highlights whatever now sits there. Colour defaults to cyan.
fn v_compare(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let (ei, ej) = {
        let occ = s
            .occ
            .get(&id)
            .ok_or_else(|| Error::new(format!("`{id}` is not an array"), a.span_of(0)))?;
        let n = occ.len();
        let i = a.num(1)? as usize;
        let j = a.num(2)? as usize;
        if i >= n || j >= n {
            return Err(Error::new(
                format!("slot out of range for `{id}` (have 0..{n})"),
                a.span_of(if i >= n { 1 } else { 2 }),
            ));
        }
        (occ[i].clone(), occ[j].clone())
    };
    let color = if a.len() > 3 {
        resolve_color(&a.ident(3)?, a.span_of(3))?
    } else {
        style::CYAN
    };
    Ok(Clip::par(vec![
        act().highlight(&ei, color).into(),
        act().highlight(&ej, color).into(),
    ]))
}

/// `pointer(id, arr, slot, [label])` — an index marker: a filled triangle caret
/// that sits **below** slot `slot` of array `arr` and points up at it, with an
/// optional text label (`"i"`, `"lo"`, …). Move it with `pointat(id, arr, slot)`.
/// The caret is `{id}` and rides at the slot's x; `{id}.label` follows it.
/// Declare it *after* the array it indexes.
fn c_pointer(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let arr = a.ident(1)?;
    let slot = a.num(2)? as usize;
    let (bpos, bh) = {
        let b = s
            .get(&format!("{arr}.box{slot}"))
            .ok_or_else(|| Error::new(format!("`{arr}` has no slot box {slot}"), a.span_of(1)))?;
        let h = if let Shape::Rect { h, .. } = &b.shape {
            *h
        } else {
            60.0
        };
        (b.pos, h)
    };
    // apex sits just below the box bottom; caret points up
    let apex_y = bpos.y + bh / 2.0 + 16.0;
    let pos = Vec2::new(bpos.x, apex_y + 10.0);
    let caret = vec![
        Vec2::new(-11.0, 10.0),
        Vec2::new(11.0, 10.0),
        Vec2::new(0.0, -10.0),
    ];
    let mut mark = Entity::new(
        id.clone(),
        Shape::Polygon { pts: caret },
        pos,
        style::MAGENTA,
    );
    mark.stroke = StrokeStyle {
        fill: true,
        outline: false,
        width: 2.0,
        outline_color: None,
    };
    mark.z = 8;
    s.add(mark);
    if a.len() > 3 {
        let mut lbl = Entity::new(
            format!("{id}.label"),
            Shape::Text {
                content: a.text(3)?,
                size: 24.0,
            },
            Vec2::ZERO,
            style::MAGENTA,
        );
        lbl.font = FontKind::MonoBold;
        lbl.z = 8;
        lbl.follow = Some((id.clone(), Vec2::new(0.0, 34.0)));
        s.add(lbl);
    }
    Ok(())
}

/// `caret(id, (x,y), "label", [dir])` — a small filled triangle marker with a
/// text label, for annotating *where* an action happens (a stack top, a queue
/// front/back). `dir` ∈ `up|down|left|right` is the way it points (default up).
/// It is a rigid marker, so `move(id, (x,y))` / `shift` slides it (and its
/// `{id}.label`) to track a moving endpoint.
fn c_caret(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let at = a.pair(1)?;
    let label = a.text(2)?;
    let dir = if a.len() > 3 {
        a.ident(3)?
    } else {
        "up".to_string()
    };
    let p = |x: f32, y: f32| Vec2::new(x, y);
    let (pts, loff) = match dir.as_str() {
        "up" => (
            vec![p(-11.0, 10.0), p(11.0, 10.0), p(0.0, -10.0)],
            p(0.0, 32.0),
        ),
        "down" => (
            vec![p(-11.0, -10.0), p(11.0, -10.0), p(0.0, 10.0)],
            p(0.0, -32.0),
        ),
        "left" => (
            vec![p(10.0, -11.0), p(10.0, 11.0), p(-10.0, 0.0)],
            p(48.0, 0.0),
        ),
        "right" => (
            vec![p(-10.0, -11.0), p(-10.0, 11.0), p(10.0, 0.0)],
            p(-48.0, 0.0),
        ),
        other => {
            return Err(Error::new(
                format!("unknown caret dir `{other}` (up|down|left|right)"),
                a.span_of(3),
            ))
        }
    };
    let mut mark = Entity::new(id.clone(), Shape::Polygon { pts }, at, style::MAGENTA);
    mark.stroke = StrokeStyle {
        fill: true,
        outline: false,
        width: 2.0,
        outline_color: None,
    };
    mark.z = 9;
    s.add(mark);
    let mut lbl = Entity::new(
        format!("{id}.label"),
        Shape::Text {
            content: label,
            size: 22.0,
        },
        Vec2::ZERO,
        style::MAGENTA,
    );
    lbl.font = FontKind::MonoBold;
    lbl.z = 9;
    lbl.follow = Some((id.clone(), loff));
    s.add(lbl);
    Ok(())
}

/// `pointat(id, arr, slot, [dur])` — slide index marker `id` to sit under slot
/// `slot` of array `arr` (its label follows). Points at the slot *position*, so
/// it stays put as values swap through that slot. (Named `pointat`, not `point`,
/// since geo's `point` constructor owns that word.)
fn v_point(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let arr = a.ident(1)?;
    let slot = a.num(2)? as usize;
    let cur_y = s
        .get(&id)
        .ok_or_else(|| Error::new(format!("no pointer named `{id}`"), a.span_of(0)))?
        .pos
        .y;
    let bx = s
        .get(&format!("{arr}.box{slot}"))
        .ok_or_else(|| Error::new(format!("`{arr}` has no slot box {slot}"), a.span_of(1)))?
        .pos
        .x;
    let mut b = act()
        .move_to(&id, Vec2::new(bx, cur_y))
        .ease(Easing::InOutCubic);
    if let Some(d) = a.opt_num(3)? {
        b = b.dur(d);
    }
    Ok(b.into())
}

// ---- stack & queue (dynamic, stateful) ------------------------------------
//
// A container is just a hidden `{id}.anchor` (holding its origin + cell size)
// plus a `Scene::occ` entry listing its cells front→back / bottom→top. Cells are
// **added to the base scene** by the push/enqueue mutating verbs — spawned
// invisibly off-position, then dropped/slid in by the clip. Because a pushed
// entity lives in the base scene, `resolve()` chains its later pop/dequeue
// motion off the entrance automatically. Popped cells stay in the scene
// (invisible), which also gives a free monotonic id counter.

/// `stack(id, (x,y), [cw], [ch])` / `queue(id, (x,y), [cw], [ch])` — an empty
/// container anchored at `(x,y)`: for a stack that's the bottom cell's centre
/// (it grows up); for a queue it's the front cell's centre (it grows right).
fn c_container(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let at = a.pair(1)?;
    let cw = a.opt_num(2)?.unwrap_or(84.0);
    let ch = a.opt_num(3)?.unwrap_or(64.0);
    let mut anchor = Entity::new(
        format!("{id}.anchor"),
        Shape::Rect { w: cw, h: ch },
        at,
        style::PANEL,
    );
    anchor.opacity = 0.0;
    anchor.stroke = StrokeStyle {
        fill: false,
        outline: false,
        width: 0.0,
        outline_color: None,
    };
    s.add(anchor);
    s.occ.insert(id, Vec::new());
    Ok(())
}

fn tk(id: String, prop: Prop, target: TargetValue, dur: f32, easing: Easing) -> TrackSpec {
    TrackSpec {
        id,
        prop,
        target,
        start: 0.0,
        dur,
        easing,
    }
}

/// The origin + cell size of a container (`x,y` = anchor centre).
fn anchor_geom(s: &Scene, id: &str, a: &Args) -> Result<(Vec2, f32, f32), Error> {
    let e = s
        .get(&format!("{id}.anchor"))
        .ok_or_else(|| Error::new(format!("`{id}` is not a stack or queue"), a.span_of(0)))?;
    if let Shape::Rect { w, h } = &e.shape {
        Ok((e.pos, *w, *h))
    } else {
        Ok((e.pos, 84.0, 64.0))
    }
}

/// Monotonic count of cells ever created for `id` (popped ones linger in the
/// scene), so a fresh push/enqueue always gets a unique id.
fn cell_count(s: &Scene, id: &str) -> usize {
    let prefix = format!("{id}.cell");
    s.entities
        .iter()
        .filter(|e| e.id.starts_with(&prefix) && e.id.ends_with(".box"))
        .count()
}

/// Add a container cell (filled box + centred value text that rides it),
/// spawned invisibly at `spawn` so the entrance clip can bring it in.
fn add_cell(s: &mut Scene, cell: &str, spawn: Vec2, cw: f32, ch: f32, val: &str, tag: &str) {
    let mut b = Entity::new(
        format!("{cell}.box"),
        Shape::Rect {
            w: cw * 0.9,
            h: ch * 0.9,
        },
        spawn,
        style::PANEL,
    );
    b.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 2.5,
        outline_color: Some(style::CYAN),
    };
    b.opacity = 0.0;
    b.z = 2;
    b.tags = vec![tag.to_string()];
    s.add(b);

    let mut t = Entity::new(
        format!("{cell}.v"),
        Shape::Text {
            content: val.to_string(),
            size: ch * 0.42,
        },
        spawn,
        style::FG,
    );
    t.font = FontKind::MonoBold;
    t.opacity = 0.0;
    t.z = 4;
    t.follow = Some((format!("{cell}.box"), Vec2::ZERO));
    t.tags = vec![tag.to_string()];
    s.add(t);
}

/// Tracks that bring a spawned cell into `target` (drop/slide + fade).
fn enter_tracks(cell: &str, target: Vec2, dur: f32) -> Vec<TrackSpec> {
    vec![
        tk(
            format!("{cell}.box"),
            Prop::Pos,
            TargetValue::Abs(Value::V(target)),
            dur,
            Easing::OutBack,
        ),
        tk(
            format!("{cell}.box"),
            Prop::Opacity,
            TargetValue::Abs(Value::F(1.0)),
            dur,
            Easing::OutQuad,
        ),
        tk(
            format!("{cell}.v"),
            Prop::Opacity,
            TargetValue::Abs(Value::F(1.0)),
            dur,
            Easing::OutQuad,
        ),
    ]
}

/// Tracks that send a cell out to `out` and fade it (pop/dequeue). Its `from`
/// chains off the entrance target automatically in `resolve()`.
fn exit_tracks(cell: &str, out: Vec2, dur: f32) -> Vec<TrackSpec> {
    vec![
        tk(
            format!("{cell}.box"),
            Prop::Pos,
            TargetValue::Abs(Value::V(out)),
            dur,
            Easing::InQuad,
        ),
        tk(
            format!("{cell}.box"),
            Prop::Opacity,
            TargetValue::Abs(Value::F(0.0)),
            dur,
            Easing::InQuad,
        ),
        tk(
            format!("{cell}.v"),
            Prop::Opacity,
            TargetValue::Abs(Value::F(0.0)),
            dur,
            Easing::InQuad,
        ),
    ]
}

/// `push(id, "val", [dur])` — drop a new cell onto the top of stack `id`.
fn v_push(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let val = a.text(1)?;
    let dur = a.opt_num(2)?.unwrap_or(0.5);
    let (anchor, cw, ch) = anchor_geom(s, &id, a)?;
    let slot = s.occ.get(&id).map(|v| v.len()).unwrap_or(0);
    let cell = format!("{id}.cell{}", cell_count(s, &id));
    let target = Vec2::new(anchor.x, anchor.y - slot as f32 * ch);
    let spawn = Vec2::new(anchor.x, target.y - 110.0);
    add_cell(s, &cell, spawn, cw, ch, &val, &format!("{id}.cells"));
    s.occ.entry(id).or_default().push(cell.clone());
    Ok(Clip {
        dur,
        tracks: enter_tracks(&cell, target, dur),
        events: Vec::new(),
    })
}

/// `pop(id, [dur])` — lift the top cell off stack `id`.
fn v_pop(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let dur = a.opt_num(1)?.unwrap_or(0.45);
    let (anchor, _cw, ch) = anchor_geom(s, &id, a)?;
    let cell = s
        .occ
        .get_mut(&id)
        .ok_or_else(|| Error::new(format!("`{id}` is not a stack or queue"), a.span_of(0)))?
        .pop()
        .ok_or_else(|| Error::new(format!("`{id}` is empty — nothing to pop"), a.span_of(0)))?;
    let slot = s.occ.get(&id).map(|v| v.len()).unwrap_or(0);
    let out = Vec2::new(anchor.x, anchor.y - slot as f32 * ch - 120.0);
    Ok(Clip {
        dur,
        tracks: exit_tracks(&cell, out, dur),
        events: Vec::new(),
    })
}

/// `enqueue(id, "val", [dur])` — slide a new cell onto the back (right) of queue `id`.
fn v_enqueue(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let val = a.text(1)?;
    let dur = a.opt_num(2)?.unwrap_or(0.5);
    let (anchor, cw, ch) = anchor_geom(s, &id, a)?;
    let slot = s.occ.get(&id).map(|v| v.len()).unwrap_or(0);
    let cell = format!("{id}.cell{}", cell_count(s, &id));
    let target = Vec2::new(anchor.x + slot as f32 * cw, anchor.y);
    let spawn = Vec2::new(target.x + 110.0, anchor.y);
    add_cell(s, &cell, spawn, cw, ch, &val, &format!("{id}.cells"));
    s.occ.entry(id).or_default().push(cell.clone());
    Ok(Clip {
        dur,
        tracks: enter_tracks(&cell, target, dur),
        events: Vec::new(),
    })
}

/// `dequeue(id, [dur])` — the front (left) cell exits and the rest advance.
fn v_dequeue(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let dur = a.opt_num(1)?.unwrap_or(0.5);
    let (anchor, cw, _ch) = anchor_geom(s, &id, a)?;
    let front = {
        let v = s
            .occ
            .get_mut(&id)
            .ok_or_else(|| Error::new(format!("`{id}` is not a stack or queue"), a.span_of(0)))?;
        if v.is_empty() {
            return Err(Error::new(
                format!("`{id}` is empty — nothing to dequeue"),
                a.span_of(0),
            ));
        }
        v.remove(0)
    };
    let out = Vec2::new(anchor.x - 130.0, anchor.y);
    let mut tracks = exit_tracks(&front, out, dur);
    // remaining cells advance one slot toward the front
    let rest = s.occ.get(&id).cloned().unwrap_or_default();
    for (i, cell) in rest.iter().enumerate() {
        let target = Vec2::new(anchor.x + i as f32 * cw, anchor.y);
        tracks.push(tk(
            format!("{cell}.box"),
            Prop::Pos,
            TargetValue::Abs(Value::V(target)),
            dur,
            Easing::InOutCubic,
        ));
    }
    Ok(Clip {
        dur,
        tracks,
        events: Vec::new(),
    })
}

// ---- doubly-linked list (classic anatomy: singly / doubly / circular) -------
//
// A node is a framed box split into compartments — singly `[ data | •next ]`,
// doubly `[ •prev | data | next• ]` — where a pointer field carries a dot that
// its arrow originates from. A `head` pointer marks the entry node; the tail's
// `next` ends at a `NULL` terminator (singly/doubly) or curves back to the head
// (circular). All the sub-parts ride the node box, so a node moves as a unit.
//
// `Scene::occ` holds node order under `id`, the kind under `{id}#kind`, and the
// live arrow ids under `{id}#arrows`. Arrows are rebuilt from (order, kind) on
// every op — correct across all three kinds and every edge case — while the new
// node fades in and the old pointers fade out.

const PW: f32 = 26.0; // pointer-field width

struct Geom {
    nw: f32,
    data_off: f32,
    next_off: f32,
    prev_off: Option<f32>,
    dividers: Vec<f32>,
}

fn geom(kind: &str, cw: f32) -> Geom {
    if kind == "doubly" {
        Geom {
            nw: 2.0 * PW + cw,
            data_off: 0.0,
            next_off: (PW + cw) / 2.0,
            prev_off: Some(-(PW + cw) / 2.0),
            dividers: vec![-cw / 2.0, cw / 2.0],
        }
    } else {
        Geom {
            nw: cw + PW,
            data_off: -PW / 2.0,
            next_off: cw / 2.0,
            prev_off: None,
            dividers: vec![(cw - PW) / 2.0],
        }
    }
}

fn count_ids(s: &Scene, prefix: &str, suffix: &str) -> usize {
    s.entities
        .iter()
        .filter(|e| e.id.starts_with(prefix) && e.id.ends_with(suffix))
        .count()
}

/// Build a classic node: framed box + data text + compartment dividers + pointer
/// dots, all following the box so the whole node moves/fades as one.
fn place_node(
    s: &mut Scene,
    nid: &str,
    c: Vec2,
    kind: &str,
    cw: f32,
    ch: f32,
    val: &str,
    list: &str,
    op: f32,
) {
    let g = geom(kind, cw);
    let tag = format!("{list}.nodes");
    let mut b = Entity::new(
        nid.to_string(),
        Shape::Rect { w: g.nw, h: ch },
        c,
        style::PANEL,
    );
    b.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 2.5,
        outline_color: Some(style::CYAN),
    };
    b.opacity = op;
    b.z = 3;
    b.tags = vec![list.to_string(), tag.clone()];
    s.add(b);

    let mut t = Entity::new(
        format!("{nid}.v"),
        Shape::Text {
            content: val.to_string(),
            size: ch * 0.42,
        },
        c,
        style::FG,
    );
    t.font = FontKind::MonoBold;
    t.z = 5;
    t.follow = Some((nid.to_string(), Vec2::new(g.data_off, 0.0)));
    t.tags = vec![list.to_string(), tag.clone()];
    s.add(t);

    for (j, off) in g.dividers.iter().enumerate() {
        let mut d = Entity::new(
            format!("{nid}.dv{j}"),
            Shape::Rect {
                w: 2.5,
                h: ch * 0.9,
            },
            Vec2::new(c.x + off, c.y),
            style::DIM,
        );
        d.stroke = StrokeStyle {
            fill: true,
            outline: false,
            width: 0.0,
            outline_color: None,
        };
        d.z = 4;
        d.follow = Some((nid.to_string(), Vec2::new(*off, 0.0)));
        d.tags = vec![list.to_string(), tag.clone()];
        s.add(d);
    }

    let dot = |suffix: &str, off: f32, s: &mut Scene| {
        let mut e = Entity::new(
            format!("{nid}.{suffix}"),
            Shape::Circle { r: 4.0 },
            Vec2::new(c.x + off, c.y),
            style::CYAN,
        );
        e.z = 6;
        e.follow = Some((nid.to_string(), Vec2::new(off, 0.0)));
        e.tags = vec![list.to_string(), tag.clone()];
        s.add(e);
    };
    dot("pn", g.next_off, s);
    if let Some(po) = g.prev_off {
        dot("pp", po, s);
    }
}

/// Rebuild every structural arrow from the current (order, kind): inter-node
/// next/prev, the tail terminator (NULL or circular wrap) and the head arrow.
/// On a dynamic op it fades the previous arrows, draws the new ones, and slides
/// the persistent `head`/`NULL` labels to the new ends. Returns the tracks.
fn wire(
    s: &mut Scene,
    id: &str,
    kind: &str,
    cw: f32,
    ch: f32,
    dur: f32,
    initial: bool,
) -> Vec<TrackSpec> {
    let mut tracks = Vec::new();
    let nodes = s.occ.get(id).cloned().unwrap_or_default();
    let g = geom(kind, cw);
    let l = if kind == "doubly" { 9.0 } else { 0.0 };
    let v = |x: f32, y: f32| Vec2::new(x, y);

    if !initial {
        if let Some(old) = s.occ.get(&format!("{id}#arrows")).cloned() {
            for aid in old {
                tracks.push(tk(
                    aid,
                    Prop::Opacity,
                    TargetValue::Abs(Value::F(0.0)),
                    dur * 0.5,
                    Easing::InQuad,
                ));
            }
        }
    }

    let centers: Vec<Vec2> = nodes
        .iter()
        .map(|nid| s.get(nid).map(|e| e.pos).unwrap_or(Vec2::ZERO))
        .collect();
    let mut arrows: Vec<String> = Vec::new();
    let trace0 = if initial { 1.0 } else { 0.0 };

    let add = |s: &mut Scene,
               tail: Vec2,
               head: Vec2,
               color: Color,
               kind_tag: &str,
               tracks: &mut Vec<TrackSpec>,
               arrows: &mut Vec<String>| {
        let aid = format!("{id}.ar{}", count_ids(s, &format!("{id}.ar"), ""));
        let mut e = Entity::new(aid.clone(), Shape::Arrow { to: head }, tail, color);
        e.stroke.width = 2.5;
        e.z = 1;
        e.trace = trace0;
        e.tags = vec![id.to_string(), format!("{id}.{kind_tag}")];
        s.add(e);
        if !initial {
            tracks.push(tk(
                aid.clone(),
                Prop::Trace,
                TargetValue::Abs(Value::F(1.0)),
                dur,
                Easing::OutQuad,
            ));
        }
        arrows.push(aid);
    };

    // inter-node pointers
    for i in 0..centers.len().saturating_sub(1) {
        let (ca, cb) = (centers[i], centers[i + 1]);
        add(
            s,
            ca + v(g.next_off, -l),
            cb + v(-g.nw / 2.0, -l),
            style::CYAN,
            "next",
            &mut tracks,
            &mut arrows,
        );
        if let Some(po) = g.prev_off {
            add(
                s,
                cb + v(po, l),
                ca + v(g.nw / 2.0, l),
                style::DIM,
                "prev",
                &mut tracks,
                &mut arrows,
            );
        }
    }

    // terminator: NULL (singly/doubly) or wrap-to-head (circular)
    if let (Some(&first), Some(&last)) = (centers.first(), centers.last()) {
        if kind == "circular" {
            let (tail, head) = (last + v(g.next_off, 0.0), first + v(-g.nw / 2.0, 0.0));
            let ctrl = Vec2::new((tail.x + head.x) / 2.0, tail.y + 2.3 * ch + 20.0);
            let aid = format!("{id}.ar{}", count_ids(s, &format!("{id}.ar"), ""));
            let mut e = Entity::new(
                aid.clone(),
                Shape::Curve {
                    ctrl,
                    to: head,
                    arrow: true,
                },
                tail,
                style::MAGENTA,
            );
            e.stroke.width = 2.5;
            e.z = 1;
            e.trace = trace0;
            e.tags = vec![id.to_string(), format!("{id}.next")];
            s.add(e);
            if !initial {
                tracks.push(tk(
                    aid.clone(),
                    Prop::Trace,
                    TargetValue::Abs(Value::F(1.0)),
                    dur,
                    Easing::OutQuad,
                ));
            }
            arrows.push(aid);
        } else {
            if let Some(np) = s.get(&format!("{id}.null")).map(|e| e.pos) {
                add(
                    s,
                    last + v(g.next_off, -l),
                    np + v(-18.0, -l),
                    style::CYAN,
                    "next",
                    &mut tracks,
                    &mut arrows,
                );
            }
            if kind == "doubly" {
                if let Some(nlp) = s.get(&format!("{id}.nullL")).map(|e| e.pos) {
                    add(
                        s,
                        first + v(g.prev_off.unwrap_or(0.0), l),
                        nlp + v(18.0, l),
                        style::DIM,
                        "prev",
                        &mut tracks,
                        &mut arrows,
                    );
                }
            }
        }
    }

    // head arrow into node 0
    if let Some(&c0) = centers.first() {
        add(
            s,
            v(c0.x, c0.y - ch / 2.0 - 30.0),
            v(c0.x, c0.y - ch / 2.0 - 5.0),
            style::MAGENTA,
            "head",
            &mut tracks,
            &mut arrows,
        );
    }

    // slide persistent labels to the new ends
    if !initial {
        if let (Some(&first), Some(&last)) = (centers.first(), centers.last()) {
            let mv = |s: &Scene, mid: &str, to: Vec2, tracks: &mut Vec<TrackSpec>| {
                if s.contains(mid) {
                    tracks.push(tk(
                        mid.to_string(),
                        Prop::Pos,
                        TargetValue::Abs(Value::V(to)),
                        dur,
                        Easing::InOutCubic,
                    ));
                }
            };
            mv(
                s,
                &format!("{id}.head"),
                v(first.x, first.y - ch / 2.0 - 48.0),
                &mut tracks,
            );
            mv(
                s,
                &format!("{id}.null"),
                v(last.x + g.nw / 2.0 + 52.0, last.y),
                &mut tracks,
            );
            mv(
                s,
                &format!("{id}.nullL"),
                v(first.x - g.nw / 2.0 - 52.0, first.y),
                &mut tracks,
            );
        }
    }

    s.occ.insert(format!("{id}#arrows"), arrows);
    tracks
}

/// `list(id, "3 8 5", (cx,cy), [kind], [cw], [ch])` — a linked list drawn with
/// the classic node anatomy. `kind` ∈ `singly` (next + NULL), `doubly` (next &
/// prev, NULL both ends), `circular` (tail wraps to head); default `doubly`.
/// Grow with `insert(id, after, "v")`, shrink with `remove(id, i)`.
fn c_list(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let vals: Vec<String> = a
        .text(1)?
        .split_whitespace()
        .map(|w| w.to_string())
        .collect();
    let center = a.pair(2)?;
    let kind = if a.len() > 3 {
        a.ident(3)?
    } else {
        "doubly".to_string()
    };
    if !matches!(kind.as_str(), "singly" | "doubly" | "circular") {
        return Err(Error::new(
            format!("unknown list kind `{kind}` (singly|doubly|circular)"),
            a.span_of(3),
        ));
    }
    let cw = a.opt_num(4)?.unwrap_or(74.0);
    let ch = a.opt_num(5)?.unwrap_or(56.0);
    if vals.is_empty() {
        return Err(Error::new("list has no values", a.span_of(1)));
    }
    let g = geom(&kind, cw);
    let spacing = g.nw + 50.0;
    let x0 = center.x - (vals.len() as f32 - 1.0) * spacing / 2.0;
    let mut nodes = Vec::new();
    for (k, val) in vals.iter().enumerate() {
        let c = Vec2::new(x0 + k as f32 * spacing, center.y);
        let nid = format!("{id}.node{k}");
        place_node(s, &nid, c, &kind, cw, ch, val, &id, 1.0);
        nodes.push(nid);
    }
    let first = Vec2::new(x0, center.y);
    let last = Vec2::new(x0 + (vals.len() as f32 - 1.0) * spacing, center.y);

    // persistent labels
    let mut head = Entity::new(
        format!("{id}.head"),
        Shape::Text {
            content: "head".into(),
            size: 20.0,
        },
        Vec2::new(first.x, first.y - ch / 2.0 - 48.0),
        style::MAGENTA,
    );
    head.font = FontKind::MonoBold;
    head.z = 6;
    head.tags = vec![id.clone()];
    s.add(head);
    if kind != "circular" {
        let mut nul = Entity::new(
            format!("{id}.null"),
            Shape::Text {
                content: "NULL".into(),
                size: 20.0,
            },
            Vec2::new(last.x + g.nw / 2.0 + 52.0, last.y),
            style::DIM,
        );
        nul.font = FontKind::MonoBold;
        nul.z = 6;
        nul.tags = vec![id.clone()];
        s.add(nul);
    }
    if kind == "doubly" {
        let mut nul = Entity::new(
            format!("{id}.nullL"),
            Shape::Text {
                content: "NULL".into(),
                size: 20.0,
            },
            Vec2::new(first.x - g.nw / 2.0 - 52.0, first.y),
            style::DIM,
        );
        nul.font = FontKind::MonoBold;
        nul.z = 6;
        nul.tags = vec![id.clone()];
        s.add(nul);
    }

    s.occ.insert(id.clone(), nodes);
    s.occ.insert(format!("{id}#kind"), vec![kind.clone()]);
    let _ = wire(s, &id, &kind, cw, ch, 0.0, true);
    Ok(())
}

/// Read a list's kind + cell size back from the scene.
fn list_meta(s: &Scene, id: &str, a: &Args) -> Result<(String, f32, f32), Error> {
    let kind = s
        .occ
        .get(&format!("{id}#kind"))
        .and_then(|v| v.first())
        .cloned()
        .ok_or_else(|| Error::new(format!("`{id}` is not a list"), a.span_of(0)))?;
    // recover cw/ch from a node box
    let nid = format!("{id}.node0");
    let (cw, ch) = match s.get(&nid).map(|e| (&e.shape, e.pos)) {
        Some((Shape::Rect { w, h }, _)) => {
            let cw = if kind == "doubly" {
                w - 2.0 * PW
            } else {
                w - PW
            };
            (cw, *h)
        }
        _ => (74.0, 56.0),
    };
    Ok((kind, cw, ch))
}

/// `insert(id, after, "v", [dur])` — splice a new node after index `after`. The
/// row stays put; the node fades in below the gap and the pointers re-thread.
fn v_insert(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let after = a.num(1)? as usize;
    let val = a.text(2)?;
    let dur = a.opt_num(3)?.unwrap_or(0.6);
    let (kind, cw, ch) = list_meta(s, &id, a)?;
    let nodes = s.occ.get(&id).cloned().unwrap_or_default();
    if after >= nodes.len() {
        return Err(Error::new(
            format!("index {after} out of range for `{id}` (0..{})", nodes.len()),
            a.span_of(1),
        ));
    }
    let g = geom(&kind, cw);
    let spacing = g.nw + 50.0;
    let apos = s.get(&nodes[after]).unwrap().pos;
    let newpos = match nodes.get(after + 1) {
        Some(b) => {
            let bp = s.get(b).unwrap().pos;
            Vec2::new((apos.x + bp.x) / 2.0, apos.y + ch + 52.0) // below the gap
        }
        None => Vec2::new(apos.x + spacing, apos.y), // append into the row
    };
    let nid = format!("{id}.node{}", count_ids(s, &format!("{id}.node"), ".v"));
    place_node(s, &nid, newpos, &kind, cw, ch, &val, &id, 0.0);
    let mut tracks = vec![
        tk(
            nid.clone(),
            Prop::Opacity,
            TargetValue::Abs(Value::F(1.0)),
            dur,
            Easing::OutQuad,
        ),
        tk(
            nid.clone(),
            Prop::Scale,
            TargetValue::Abs(Value::F(1.0)),
            dur,
            Easing::OutBack,
        ),
    ];
    // pop-in: start slightly small
    if let Some(e) = s.get_mut(&nid) {
        e.scale = 0.6;
    }

    let mut order = nodes;
    order.insert(after + 1, nid);
    s.occ.insert(id.clone(), order);
    tracks.extend(wire(s, &id, &kind, cw, ch, dur, false));
    Ok(Clip {
        dur,
        tracks,
        events: Vec::new(),
    })
}

/// `remove(id, i, [dur])` — unlink node `i`: it fades away and the pointers
/// re-thread (a bypass appears between its neighbours, ends re-terminate).
fn v_remove(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let i = a.num(1)? as usize;
    let dur = a.opt_num(2)?.unwrap_or(0.55);
    let (kind, cw, ch) = list_meta(s, &id, a)?;
    let nodes = s.occ.get(&id).cloned().unwrap_or_default();
    if i >= nodes.len() {
        return Err(Error::new(
            format!("index {i} out of range for `{id}` (0..{})", nodes.len()),
            a.span_of(1),
        ));
    }
    let xid = nodes[i].clone();
    let xpos = s.get(&xid).unwrap().pos;
    let mut tracks = vec![
        tk(
            xid.clone(),
            Prop::Pos,
            TargetValue::Abs(Value::V(Vec2::new(xpos.x, xpos.y + 120.0))),
            dur,
            Easing::InQuad,
        ),
        tk(
            xid.clone(),
            Prop::Opacity,
            TargetValue::Abs(Value::F(0.0)),
            dur,
            Easing::InQuad,
        ),
    ];
    let mut order = nodes;
    order.remove(i);
    s.occ.insert(id.clone(), order);
    tracks.extend(wire(s, &id, &kind, cw, ch, dur, false));
    Ok(Clip {
        dur,
        tracks,
        events: Vec::new(),
    })
}

// ---- hash map (array of buckets + separate chaining) -----------------------
//
// A vertical column of numbered buckets; each bucket grows a horizontal chain of
// `key:val` entries (separate chaining). `put(h, k, v)` hashes the key to a
// bucket and chains the entry on; `get(h, k)` hashes, then scans that bucket's
// chain highlighting each entry until it hits the key (or falls off the end =
// miss). `Scene::occ` holds `#n` (bucket count), and per bucket `#b{i}` (entry
// ids) + `#bk{i}` (their keys) so `get` knows what's where. Composes the array
// (buckets) and list (chains) ideas from above.

/// Sum-of-bytes hash — deterministic and easy to reason about in tests.
fn hash_str(s: &str) -> usize {
    s.bytes().map(|b| b as usize).sum()
}

/// Format a number: `inf`, an integer if whole, else one decimal.
fn fmt_num(x: f32) -> String {
    if x.is_infinite() {
        "inf".to_string()
    } else if (x - x.round()).abs() < 1e-6 {
        format!("{}", x.round() as i64)
    } else {
        format!("{x:.1}")
    }
}

fn hm_n(s: &Scene, id: &str, a: &Args) -> Result<usize, Error> {
    s.occ
        .get(&format!("{id}#n"))
        .and_then(|v| v.first())
        .and_then(|x| x.parse::<usize>().ok())
        .ok_or_else(|| Error::new(format!("`{id}` is not a hashmap"), a.span_of(0)))
}

/// `hashmap(id, n, (cx,cy), [entryw], [ch])` — `n` buckets in a column at `cx`;
/// chains grow to the right. Bucket boxes are `{id}.bucket{i}` (tag
/// `{id}.buckets`); entries are `{id}.e{k}` (tag `{id}.entries`).
fn c_hashmap(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let n = a.num(1)? as usize;
    if n == 0 {
        return Err(Error::new(
            "hashmap needs at least one bucket",
            a.span_of(1),
        ));
    }
    let center = a.pair(2)?;
    let ew = a.opt_num(3)?.unwrap_or(120.0);
    let ch = a.opt_num(4)?.unwrap_or(46.0);
    let bw = 48.0;
    let rowh = ch + 18.0;
    let y0 = center.y - (n as f32 - 1.0) * rowh / 2.0;
    for i in 0..n {
        let by = y0 + i as f32 * rowh;
        let mut b = Entity::new(
            format!("{id}.bucket{i}"),
            Shape::Rect { w: bw, h: ch },
            Vec2::new(center.x, by),
            style::PANEL,
        );
        b.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 2.5,
            outline_color: Some(style::DIM),
        };
        b.z = 2;
        b.tags = vec![format!("{id}.buckets")];
        s.add(b);
        let mut t = Entity::new(
            format!("{id}.bucket{i}.v"),
            Shape::Text {
                content: i.to_string(),
                size: ch * 0.44,
            },
            Vec2::new(center.x, by),
            style::DIM,
        );
        t.font = FontKind::MonoBold;
        t.z = 4;
        t.follow = Some((format!("{id}.bucket{i}"), Vec2::ZERO));
        s.add(t);
        s.occ.insert(format!("{id}#b{i}"), Vec::new());
        s.occ.insert(format!("{id}#bk{i}"), Vec::new());
    }
    let mut anchor = Entity::new(
        format!("{id}.anchor"),
        Shape::Rect { w: ew, h: ch },
        center,
        style::PANEL,
    );
    anchor.opacity = 0.0;
    anchor.stroke = StrokeStyle {
        fill: false,
        outline: false,
        width: 0.0,
        outline_color: None,
    };
    s.add(anchor);
    s.occ.insert(format!("{id}#n"), vec![n.to_string()]);
    Ok(())
}

/// `put(h, "key", "val", [dur])` — hash the key to a bucket and chain the entry.
fn v_put(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let key = a.text(1)?;
    let val = a.text(2)?;
    let dur = a.opt_num(3)?.unwrap_or(0.5);
    let n = hm_n(s, &id, a)?;
    let b = hash_str(&key) % n;
    let bpos = s.get(&format!("{id}.bucket{b}")).unwrap().pos;
    let (ew, ch) = match &s.get(&format!("{id}.anchor")).unwrap().shape {
        Shape::Rect { w, h } => (*w, *h),
        _ => (120.0, 46.0),
    };
    let bw = 48.0;
    let spacing = ew + 44.0;
    let chain: Vec<String> = s
        .occ
        .get(&format!("{id}#b{b}"))
        .cloned()
        .unwrap_or_default();
    let x = bpos.x + bw / 2.0 + 44.0 + chain.len() as f32 * spacing + ew / 2.0;
    let y = bpos.y;
    let eid = format!("{id}.e{}", count_ids(s, &format!("{id}.e"), ".v"));

    let mut e = Entity::new(
        eid.clone(),
        Shape::Rect { w: ew, h: ch },
        Vec2::new(x, y),
        style::PANEL,
    );
    e.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 2.5,
        outline_color: Some(style::CYAN),
    };
    e.opacity = 0.0;
    e.scale = 0.6;
    e.z = 3;
    e.tags = vec![format!("{id}.entries")];
    s.add(e);
    let mut t = Entity::new(
        format!("{eid}.v"),
        Shape::Text {
            content: format!("{key}:{val}"),
            size: ch * 0.4,
        },
        Vec2::new(x, y),
        style::FG,
    );
    t.font = FontKind::MonoBold;
    t.z = 5;
    t.follow = Some((eid.clone(), Vec2::ZERO));
    t.tags = vec![format!("{id}.entries")];
    s.add(t);

    // chain arrow from the previous link (bucket or last entry) to this entry
    let (px, phalf) = if let Some(last) = chain.last() {
        (s.get(last).unwrap().pos.x, ew / 2.0)
    } else {
        (bpos.x, bw / 2.0)
    };
    let aid = format!("{id}.ar{}", count_ids(s, &format!("{id}.ar"), ""));
    let mut arr = Entity::new(
        aid.clone(),
        Shape::Arrow {
            to: Vec2::new(x - ew / 2.0, y),
        },
        Vec2::new(px + phalf, y),
        style::CYAN,
    );
    arr.stroke.width = 2.5;
    arr.z = 1;
    arr.trace = 0.0;
    arr.tags = vec![format!("{id}.chains")];
    s.add(arr);

    s.occ
        .entry(format!("{id}#b{b}"))
        .or_default()
        .push(eid.clone());
    s.occ.entry(format!("{id}#bk{b}")).or_default().push(key);

    Ok(Clip {
        dur,
        tracks: vec![
            tk(
                eid.clone(),
                Prop::Opacity,
                TargetValue::Abs(Value::F(1.0)),
                dur,
                Easing::OutQuad,
            ),
            tk(
                eid.clone(),
                Prop::Scale,
                TargetValue::Abs(Value::F(1.0)),
                dur,
                Easing::OutBack,
            ),
            tk(
                aid,
                Prop::Trace,
                TargetValue::Abs(Value::F(1.0)),
                dur,
                Easing::OutQuad,
            ),
        ],
        events: Vec::new(),
    })
}

/// `get(h, "key", [dur])` — hash, then scan the bucket's chain: each entry
/// flashes in turn until the key matches (lime) or the chain ends (bucket
/// flashes magenta = miss).
fn v_get(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let key = a.text(1)?;
    let step = a.opt_num(2)?.unwrap_or(0.45);
    let n = hm_n(s, &id, a)?;
    let b = hash_str(&key) % n;
    let bucket = format!("{id}.bucket{b}");
    let keys: Vec<String> = s
        .occ
        .get(&format!("{id}#bk{b}"))
        .cloned()
        .unwrap_or_default();
    let entries: Vec<String> = s
        .occ
        .get(&format!("{id}#b{b}"))
        .cloned()
        .unwrap_or_default();
    let hit = keys.iter().position(|k| *k == key);

    let mut tracks = vec![TrackSpec {
        id: bucket.clone(),
        prop: Prop::Color,
        target: TargetValue::Abs(Value::C(style::CYAN)),
        start: 0.0,
        dur: step,
        easing: Easing::OutQuad,
    }];
    let scan_to = hit.unwrap_or(entries.len().saturating_sub(1));
    let mut t = step;
    for (i, e) in entries.iter().enumerate() {
        if i > scan_to {
            break;
        }
        let is_hit = hit == Some(i);
        tracks.push(TrackSpec {
            id: e.clone(),
            prop: Prop::Color,
            target: TargetValue::Abs(Value::C(if is_hit { style::LIME } else { style::MAGENTA })),
            start: t,
            dur: step,
            easing: Easing::OutQuad,
        });
        t += step;
    }
    if hit.is_none() {
        // miss: bucket flashes magenta
        tracks.push(TrackSpec {
            id: bucket,
            prop: Prop::Color,
            target: TargetValue::Abs(Value::C(style::MAGENTA)),
            start: t,
            dur: step,
            easing: Easing::OutQuad,
        });
        t += step;
    }
    Ok(Clip {
        dur: t + 0.2,
        tracks,
        events: Vec::new(),
    })
}

// ---- graph traversal: BFS / DFS --------------------------------------------
//
// BFS and DFS are the *same* algorithm differing only in the frontier: a queue
// (BFS → level order) or a stack (DFS → go deep). `bfs(g, start)` / `dfs(g,
// start)` read the graph's adjacency straight from its edge entities, run the
// traversal at build time, and emit one sequenced clip: each node cycles through
// colour states — discovered (cyan) → current (magenta, with a pop) → done
// (lime) — tree edges light up as they're taken, and two live readouts show the
// frontier (`queue:`/`stack:`) and the visited order. Directed edges (`a>b`) are
// followed one way; undirected (`a-b`) both ways.

/// Colour states for traversal.
const C_FRONTIER: Color = style::CYAN;
const C_CURRENT: Color = style::MAGENTA;
const C_DONE: Color = style::LIME;

fn traverse(s: &mut Scene, a: &Args, use_stack: bool) -> Result<Clip, Error> {
    let g = a.ident(0)?;
    let start_name = a.ident(1)?;
    let start = format!("{g}.{start_name}");
    if !s.contains(&start) {
        return Err(Error::new(
            format!("graph `{g}` has no node `{start_name}`"),
            a.span_of(1),
        ));
    }
    let name = |id: &str| id.strip_prefix(&format!("{g}.")).unwrap_or(id).to_string();

    // adjacency + edge-id lookup, in edge-declaration order
    let ntag = format!("{g}.nodes");
    let etag = format!("{g}.edges");
    let node_ids: Vec<String> = s
        .entities
        .iter()
        .filter(|e| e.tags.iter().any(|t| *t == ntag))
        .map(|e| e.id.clone())
        .collect();
    if node_ids.is_empty() {
        return Err(Error::new(format!("`{g}` is not a graph"), a.span_of(0)));
    }
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let mut edge_of: HashMap<(String, String), String> = HashMap::new();
    for e in &s.entities {
        if !e.tags.iter().any(|t| *t == etag) {
            continue;
        }
        let Some(link) = &e.link else { continue };
        let (u, v) = (link.from.clone(), link.to.clone());
        adj.entry(u.clone()).or_default().push(v.clone());
        edge_of.insert((u.clone(), v.clone()), e.id.clone());
        if !e.id.contains('>') {
            adj.entry(v.clone()).or_default().push(u.clone());
            edge_of.insert((v.clone(), u.clone()), e.id.clone());
        }
    }

    // place two left-aligned readouts under the graph
    let (mut minx, mut maxy) = (f32::MAX, f32::MIN);
    for nid in &node_ids {
        if let Some(e) = s.get(nid) {
            minx = minx.min(e.pos.x);
            maxy = maxy.max(e.pos.y);
        }
    }
    let word = if use_stack { "stack" } else { "queue" };
    let fid = format!("{g}.frontier");
    let vid = format!("{g}.visited");
    for (id, y, color, init) in [
        (&fid, maxy + 72.0, C_FRONTIER, format!("{word}:")),
        (&vid, maxy + 110.0, C_DONE, "visited:".to_string()),
    ] {
        if !s.contains(id) {
            let mut t = Entity::new(
                id.clone(),
                Shape::Text {
                    content: init,
                    size: 24.0,
                },
                Vec2::new(minx - 34.0, y),
                color,
            );
            t.font = FontKind::MonoBold;
            t.align = Align::Left;
            t.z = 8;
            s.add(t);
        }
    }

    let mut tracks: Vec<TrackSpec> = Vec::new();
    let mut events: Vec<TextEvent> = Vec::new();
    let recolor = |tr: &mut Vec<TrackSpec>, id: &str, c: Color, at: f32| {
        tr.push(TrackSpec {
            id: id.into(),
            prop: Prop::Color,
            target: TargetValue::Abs(Value::C(c)),
            start: at,
            dur: 0.3,
            easing: Easing::OutQuad,
        });
    };
    let show = |ev: &mut Vec<TextEvent>, id: &str, prefix: &str, items: &[String], at: f32| {
        let content = if items.is_empty() {
            format!("{prefix}:")
        } else {
            format!("{prefix}: {}", items.join(" "))
        };
        ev.push(TextEvent {
            id: id.into(),
            content,
            at,
        });
    };

    // iterative traversal; discovered-at-enqueue so each node joins the frontier once
    let mut discovered: HashSet<String> = HashSet::new();
    let mut frontier: VecDeque<String> = VecDeque::new();
    let mut visited: Vec<String> = Vec::new();
    let names = |f: &VecDeque<String>| f.iter().map(|x| name(x)).collect::<Vec<_>>();

    discovered.insert(start.clone());
    frontier.push_back(start.clone());
    recolor(&mut tracks, &start, C_FRONTIER, 0.0);
    show(&mut events, &fid, word, &names(&frontier), 0.15);

    let step = 0.9;
    let mut t = 0.5;
    while let Some(u) = if use_stack {
        frontier.pop_back()
    } else {
        frontier.pop_front()
    } {
        recolor(&mut tracks, &u, C_CURRENT, t);
        tracks.push(TrackSpec {
            id: u.clone(),
            prop: Prop::Scale,
            target: TargetValue::Abs(Value::F(1.18)),
            start: t,
            dur: 0.2,
            easing: Easing::OutQuad,
        });
        tracks.push(TrackSpec {
            id: u.clone(),
            prop: Prop::Scale,
            target: TargetValue::Abs(Value::F(1.0)),
            start: t + 0.2,
            dur: 0.2,
            easing: Easing::InQuad,
        });
        show(&mut events, &fid, word, &names(&frontier), t + 0.05);

        let mut sub = t + 0.3;
        if let Some(nes) = adj.get(&u).cloned() {
            for v in nes {
                if discovered.contains(&v) {
                    continue;
                }
                discovered.insert(v.clone());
                if let Some(eid) = edge_of.get(&(u.clone(), v.clone())) {
                    recolor(&mut tracks, eid, C_DONE, sub);
                }
                recolor(&mut tracks, &v, C_FRONTIER, sub + 0.1);
                frontier.push_back(v.clone());
                show(&mut events, &fid, word, &names(&frontier), sub + 0.15);
                sub += 0.24;
            }
        }
        let done = sub.max(t + 0.5);
        recolor(&mut tracks, &u, C_DONE, done);
        visited.push(name(&u));
        show(&mut events, &vid, "visited", &visited, done + 0.05);
        t = done + 0.4;
    }
    let _ = step;
    Ok(Clip {
        dur: t + 0.3,
        tracks,
        events,
    })
}

fn v_bfs(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    traverse(s, a, false)
}
fn v_dfs(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    traverse(s, a, true)
}

/// `dijkstra(g, start)` — shortest paths from `start` on a weighted graph
/// (`a-b:w` edges; unweighted edges count as 1). Each node shows a live distance
/// (`inf` → the best found), the settled node highlights (magenta) then locks in
/// (lime), relaxed edges light up, and the final shortest-path-tree edges stay
/// lime. Classic priority-by-min-distance, ties broken by declaration order.
fn v_dijkstra(s: &mut Scene, a: &Args) -> Result<Clip, Error> {
    let g = a.ident(0)?;
    let start_name = a.ident(1)?;
    let start = format!("{g}.{start_name}");
    if !s.contains(&start) {
        return Err(Error::new(
            format!("graph `{g}` has no node `{start_name}`"),
            a.span_of(1),
        ));
    }
    let ntag = format!("{g}.nodes");
    let etag = format!("{g}.edges");
    let node_ids: Vec<String> = s
        .entities
        .iter()
        .filter(|e| e.tags.iter().any(|t| *t == ntag))
        .map(|e| e.id.clone())
        .collect();
    if node_ids.is_empty() {
        return Err(Error::new(format!("`{g}` is not a graph"), a.span_of(0)));
    }
    // adjacency (with edge ids) + symmetric weight lookup
    let mut adj: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for e in &s.entities {
        if !e.tags.iter().any(|t| *t == etag) {
            continue;
        }
        let Some(link) = &e.link else { continue };
        let (u, v) = (link.from.clone(), link.to.clone());
        adj.entry(u.clone())
            .or_default()
            .push((v.clone(), e.id.clone()));
        if !e.id.contains('>') {
            adj.entry(v).or_default().push((u, e.id.clone()));
        }
    }
    let mut wmap: HashMap<(String, String), f32> = HashMap::new();
    if let Some(ws) = s.occ.get(&format!("{g}#w")) {
        for line in ws {
            let p: Vec<&str> = line.split('|').collect();
            if let [u, v, w] = p[..] {
                if let Ok(val) = w.parse::<f32>() {
                    wmap.insert((u.into(), v.into()), val);
                    wmap.insert((v.into(), u.into()), val);
                }
            }
        }
    }
    let weight = |u: &str, v: &str| *wmap.get(&(u.into(), v.into())).unwrap_or(&1.0);

    // distance labels under each node
    for n in &node_ids {
        let p = s.get(n).unwrap().pos;
        let did = format!("{n}.d");
        if !s.contains(&did) {
            let init = if *n == start {
                "0".to_string()
            } else {
                "inf".to_string()
            };
            let mut t = Entity::new(
                did,
                Shape::Text {
                    content: init,
                    size: 20.0,
                },
                Vec2::new(p.x, p.y + 44.0),
                style::LIME,
            );
            t.font = FontKind::MonoBold;
            t.z = 8;
            s.add(t);
        }
    }

    let mut dist: HashMap<String, f32> = node_ids
        .iter()
        .map(|n| (n.clone(), f32::INFINITY))
        .collect();
    dist.insert(start.clone(), 0.0);
    let mut settled: HashSet<String> = HashSet::new();
    let mut parent_edge: HashMap<String, String> = HashMap::new();

    let mut tracks: Vec<TrackSpec> = Vec::new();
    let mut events: Vec<TextEvent> = Vec::new();
    let recolor = |tr: &mut Vec<TrackSpec>, id: &str, c: Color, at: f32| {
        tr.push(TrackSpec {
            id: id.into(),
            prop: Prop::Color,
            target: TargetValue::Abs(Value::C(c)),
            start: at,
            dur: 0.3,
            easing: Easing::OutQuad,
        });
    };
    recolor(&mut tracks, &start, C_FRONTIER, 0.0);

    let mut t = 0.5;
    loop {
        let mut best: Option<String> = None;
        let mut bd = f32::INFINITY;
        for n in &node_ids {
            if !settled.contains(n) && dist[n] < bd {
                bd = dist[n];
                best = Some(n.clone());
            }
        }
        let Some(u) = best else { break };
        if !bd.is_finite() {
            break;
        }
        settled.insert(u.clone());
        recolor(&mut tracks, &u, C_CURRENT, t);
        let mut sub = t + 0.3;
        if let Some(nes) = adj.get(&u).cloned() {
            for (v, eid) in nes {
                if settled.contains(&v) {
                    continue;
                }
                let nd = dist[&u] + weight(&u, &v);
                if nd < dist[&v] {
                    dist.insert(v.clone(), nd);
                    parent_edge.insert(v.clone(), eid.clone());
                    recolor(&mut tracks, &eid, C_FRONTIER, sub);
                    recolor(&mut tracks, &v, C_FRONTIER, sub + 0.1);
                    events.push(TextEvent {
                        id: format!("{v}.d"),
                        content: fmt_num(nd),
                        at: sub + 0.15,
                    });
                    sub += 0.26;
                }
            }
        }
        let done = sub.max(t + 0.5);
        recolor(&mut tracks, &u, C_DONE, done);
        t = done + 0.4;
    }
    // shortest-path tree edges settle to done colour
    for eid in parent_edge.values() {
        recolor(&mut tracks, eid, C_DONE, t);
    }
    Ok(Clip {
        dur: t + 0.3,
        tracks,
        events,
    })
}

/// Register the algo kit into `r`.
pub fn register(r: &mut Registry) {
    r.ctor("graph", c_graph);
    r.ctor("array", c_array);
    r.ctor("pointer", c_pointer);
    r.ctor("caret", c_caret);
    r.ctor("stack", c_container);
    r.ctor("queue", c_container);
    r.ctor("list", c_list);
    r.verb("compare", v_compare);
    r.verb("pointat", v_point);
    r.mut_verb("push", v_push);
    r.mut_verb("pop", v_pop);
    r.mut_verb("enqueue", v_enqueue);
    r.mut_verb("dequeue", v_dequeue);
    r.mut_verb("insert", v_insert);
    r.mut_verb("remove", v_remove);
    r.mut_verb("bfs", v_bfs);
    r.mut_verb("dfs", v_dfs);
    r.ctor("hashmap", c_hashmap);
    r.mut_verb("put", v_put);
    r.verb("get", v_get);
    r.mut_verb("dijkstra", v_dijkstra);
}

#[cfg(test)]
mod tests {
    use super::Shape;
    use crate::movie::Movie;
    use crate::scene::Scene;

    fn movie(src: &str) -> Movie {
        crate::parse(src).unwrap_or_else(|e| panic!("parse failed: {e:?}"))
    }
    fn at(m: &Movie, t: f32) -> Scene {
        let (base, tl) = m.finalize();
        tl.apply(&base, t)
    }
    fn text_of(sc: &Scene, id: &str) -> String {
        match &sc.get(id).unwrap().shape {
            Shape::Text { content, .. } => content.clone(),
            _ => panic!("{id} is not text"),
        }
    }
    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.5
    }

    // ---- list: the bare id broadcasts over every part (like graph/caption) ----
    #[test]
    fn list_bare_id_broadcasts_and_ops_still_work() {
        let m = movie("list(dll, \"3 8 5\", (640,300), doubly, 70, 50); color(dll, magenta); insert(dll, 1, \"7\");");
        // color(dll, …) reached the node box, its value text, the head/NULL labels,
        // and a structural arrow — i.e. the whole list is addressable by `dll`
        for part in ["dll.node0", "dll.node0.v", "dll.head", "dll.null", "dll.ar0"] {
            assert_eq!(
                m.base().get(part).unwrap().color,
                crate::style::MAGENTA,
                "color(dll,…) should broadcast to `{part}`"
            );
        }
        // insert is a mut_verb — it consumes the id (bypasses broadcast) and still
        // threads a 4th node into the occupancy
        assert_eq!(m.scene.occ["dll"].len(), 4, "insert should add a node");
    }

    // ---- array: stateful swap chains (bubble sort) ----
    #[test]
    fn array_swap_chain_sorts() {
        let m = movie("array(a, \"3 1 2\", (500,300), 100,100); swap(a,0,1); swap(a,1,2);");
        // occupancy: slot0=c1(1), slot1=c2(2), slot2=c0(3)
        assert_eq!(m.scene.occ["a"], vec!["a.c1", "a.c2", "a.c0"]);
        let sc = at(&m, 100.0);
        let bx = |k: usize| sc.get(&format!("a.box{k}")).unwrap().pos;
        assert!(close(sc.get("a.c1").unwrap().pos.x, bx(0).x), "1 -> slot0");
        assert!(close(sc.get("a.c2").unwrap().pos.x, bx(1).x), "2 -> slot1");
        assert!(close(sc.get("a.c0").unwrap().pos.x, bx(2).x), "3 -> slot2");
        // values ride with their entities
        assert_eq!(text_of(&sc, "a.c1"), "1");
        assert_eq!(text_of(&sc, "a.c0"), "3");
    }

    #[test]
    fn array_compare_leaves_order() {
        let m = movie("array(a, \"3 1 2\", (500,300)); compare(a,0,1); compare(a,1,2,lime);");
        assert_eq!(m.scene.occ["a"], vec!["a.c0", "a.c1", "a.c2"]);
    }

    #[test]
    fn array_swap_out_of_range_errs() {
        assert!(crate::parse("array(a, \"1 2\", (0,0)); swap(a,0,5);").is_err());
    }

    // ---- stack ----
    #[test]
    fn stack_push_pop() {
        let m = movie("stack(s,(300,500)); push(s,\"5\"); push(s,\"3\"); pop(s);");
        assert_eq!(m.scene.occ["s"], vec!["s.cell0"]); // 3 popped, 5 remains
        let sc = at(&m, 100.0);
        assert!(sc.get("s.cell0.box").unwrap().opacity > 0.9, "5 visible");
        assert!(sc.get("s.cell1.box").unwrap().opacity < 0.1, "3 gone");
    }

    #[test]
    fn stack_pop_empty_errs() {
        assert!(crate::parse("stack(s,(0,0)); pop(s);").is_err());
    }

    // ---- queue (incl. dequeue advance) ----
    #[test]
    fn queue_enqueue_dequeue_advances() {
        let m = movie(
            "queue(q,(700,300)); enqueue(q,\"A\"); enqueue(q,\"B\"); enqueue(q,\"C\"); dequeue(q);",
        );
        assert_eq!(m.scene.occ["q"], vec!["q.cell1", "q.cell2"]); // A left
        let sc = at(&m, 100.0);
        let ax = sc.get("q.anchor").unwrap().pos.x;
        assert!(
            close(sc.get("q.cell1.box").unwrap().pos.x, ax),
            "B advanced to front"
        );
        assert!(sc.get("q.cell0.box").unwrap().opacity < 0.1, "A gone");
    }

    #[test]
    fn queue_dequeue_empty_errs() {
        assert!(crate::parse("queue(q,(0,0)); dequeue(q);").is_err());
    }

    // ---- doubly-linked list ----
    #[test]
    fn list_insert_middle() {
        let m = movie("list(l,\"3 8 5\",(600,300)); insert(l,1,\"7\");");
        assert_eq!(
            m.scene.occ["l"],
            vec!["l.node0", "l.node1", "l.node3", "l.node2"]
        );
        assert_eq!(
            m.scene.occ["l#arrows"].len(),
            9,
            "doubly, 4 nodes -> 2n+1 arrows"
        );
        assert!(m.scene.get("l.node3").is_some());
    }

    #[test]
    fn list_insert_append() {
        let m = movie("list(l,\"3 8 5\",(600,300)); insert(l,2,\"9\");");
        assert_eq!(
            m.scene.occ["l"],
            vec!["l.node0", "l.node1", "l.node2", "l.node3"]
        );
        assert_eq!(m.scene.occ["l#arrows"].len(), 9);
    }

    #[test]
    fn list_remove_front() {
        let m = movie("list(l,\"3 8 5\",(600,300)); remove(l,0);");
        assert_eq!(m.scene.occ["l"], vec!["l.node1", "l.node2"]);
        assert_eq!(m.scene.occ["l#arrows"].len(), 5);
    }

    #[test]
    fn list_remove_middle_bypasses() {
        let m = movie("list(l,\"3 8 5\",(600,300)); remove(l,1);");
        assert_eq!(m.scene.occ["l"], vec!["l.node0", "l.node2"]);
        assert_eq!(
            m.scene.occ["l#arrows"].len(),
            5,
            "5 arrows for 2 remaining nodes"
        );
        // the removed node fades out
        let sc = at(&m, 100.0);
        assert!(sc.get("l.node1").unwrap().opacity < 0.1);
    }

    #[test]
    fn list_remove_tail() {
        let m = movie("list(l,\"3 8 5\",(600,300)); remove(l,2);");
        assert_eq!(m.scene.occ["l"], vec!["l.node0", "l.node1"]);
        assert_eq!(m.scene.occ["l#arrows"].len(), 5);
    }

    #[test]
    fn list_insert_then_remove() {
        let m = movie("list(l,\"3 8 5\",(600,300)); insert(l,1,\"7\"); remove(l,0);");
        assert_eq!(m.scene.occ["l"], vec!["l.node1", "l.node3", "l.node2"]);
        assert_eq!(m.scene.occ["l#arrows"].len(), 7);
    }

    #[test]
    fn list_insert_out_of_range_errs() {
        assert!(crate::parse("list(l,\"1 2\",(0,0)); insert(l,5,\"x\");").is_err());
    }

    // ---- pointer / point ----
    #[test]
    fn pointer_moves_to_slot() {
        let m = movie("array(a,\"1 2 3\",(500,300),90,90); pointer(p,a,0,\"i\"); pointat(p,a,2);");
        let sc = at(&m, 100.0);
        let bx2 = sc.get("a.box2").unwrap().pos.x;
        assert!(
            close(sc.get("p").unwrap().pos.x, bx2),
            "pointer rides to slot 2"
        );
    }

    // ---- more edge cases ----
    #[test]
    fn list_insert_at_zero() {
        let m = movie("list(l,\"3 8 5\",(600,300)); insert(l,0,\"7\");");
        assert_eq!(
            m.scene.occ["l"],
            vec!["l.node0", "l.node3", "l.node1", "l.node2"]
        );
        assert_eq!(m.scene.occ["l#arrows"].len(), 9);
    }

    #[test]
    fn list_remove_to_empty() {
        let m = movie("list(l,\"9\",(600,300)); remove(l,0);");
        assert!(m.scene.occ["l"].is_empty());
        assert!(m.scene.occ["l#arrows"].is_empty());
    }

    #[test]
    fn array_swap_self_is_noop() {
        let m = movie("array(a,\"3 1 2\",(500,300)); swap(a,1,1);");
        assert_eq!(m.scene.occ["a"], vec!["a.c0", "a.c1", "a.c2"]);
    }

    #[test]
    fn stack_push_pop_to_empty() {
        let m = movie("stack(s,(300,500)); push(s,\"5\"); push(s,\"3\"); pop(s); pop(s);");
        assert!(m.scene.occ["s"].is_empty());
    }

    // ---- BFS / DFS (visited order captured by the readout text) ----
    const TREE: &str =
        "graph(g,\"a b c d e f g\",\"a-b a-c b-d b-e c-f c-g\",circular,(640,360),200);";

    #[test]
    fn bfs_visits_level_order() {
        let m = movie(&format!("{TREE} bfs(g,a);"));
        let sc = at(&m, 100.0);
        assert_eq!(text_of(&sc, "g.visited"), "visited: a b c d e f g");
        // every reached node ends in the done colour
        for n in ["a", "b", "c", "d", "e", "f", "g"] {
            let c = sc.get(&format!("g.{n}")).unwrap().color;
            assert!(
                (c.r - super::C_DONE.r).abs() < 0.01 && (c.g - super::C_DONE.g).abs() < 0.01,
                "{n} done"
            );
        }
    }

    #[test]
    fn dfs_visits_depth_order() {
        let m = movie(&format!("{TREE} dfs(g,a);"));
        let sc = at(&m, 100.0);
        assert_eq!(text_of(&sc, "g.visited"), "visited: a c g f b e d");
    }

    #[test]
    fn bfs_frontier_is_a_queue() {
        let m = movie(&format!("{TREE} bfs(g,a);"));
        let sc = at(&m, 100.0);
        // queue drains empty by the end
        assert_eq!(text_of(&sc, "g.frontier"), "queue:");
    }

    #[test]
    fn bfs_respects_edge_direction() {
        // a>b only: from b there is no outgoing edge, so only b is visited
        let m = movie("graph(g,\"a b\",\"a>b\",row,(640,360),120); bfs(g,b);");
        let sc = at(&m, 100.0);
        assert_eq!(text_of(&sc, "g.visited"), "visited: b");
    }

    #[test]
    fn bfs_unknown_start_errs() {
        assert!(crate::parse("graph(g,\"a b\",\"a-b\",row,(0,0),100); bfs(g,z);").is_err());
    }

    // ---- hashmap (byte-sum hash: "cat"=312%5=2, "act" anagram=2, "dog"=314%5=4) ----
    #[test]
    fn hashmap_put_hashes_to_bucket() {
        let m = movie("hashmap(hm,5,(300,300)); put(hm,\"cat\",\"7\");");
        assert_eq!(m.scene.occ["hm#b2"], vec!["hm.e0"]);
        assert_eq!(m.scene.occ["hm#bk2"], vec!["cat"]);
        assert!(m.scene.occ["hm#b0"].is_empty());
    }

    #[test]
    fn hashmap_collision_chains() {
        let m = movie("hashmap(hm,5,(300,300)); put(hm,\"cat\",\"1\"); put(hm,\"act\",\"2\");");
        assert_eq!(m.scene.occ["hm#b2"], vec!["hm.e0", "hm.e1"]);
        assert_eq!(m.scene.occ["hm#bk2"], vec!["cat", "act"]);
    }

    #[test]
    fn hashmap_distinct_buckets() {
        let m = movie("hashmap(hm,5,(300,300)); put(hm,\"cat\",\"1\"); put(hm,\"dog\",\"2\");");
        assert_eq!(m.scene.occ["hm#b2"], vec!["hm.e0"]); // cat
        assert_eq!(m.scene.occ["hm#b4"], vec!["hm.e1"]); // dog
    }

    #[test]
    fn hashmap_get_found_and_miss_ok() {
        // both lower without error; get never mutates occ
        let m = movie(
            "hashmap(hm,5,(300,300)); put(hm,\"cat\",\"7\"); get(hm,\"cat\"); get(hm,\"zzz\");",
        );
        assert_eq!(m.scene.occ["hm#b2"], vec!["hm.e0"]);
    }

    #[test]
    fn hashmap_put_without_map_errs() {
        assert!(crate::parse("put(hm,\"k\",\"v\");").is_err());
    }

    // ---- dijkstra (final distance shown in each node's `.d` label) ----
    // graph: a-b:1 a-c:4 b-c:2 b-d:5 c-d:1  ->  from a: a=0 b=1 c=3 d=4
    #[test]
    fn dijkstra_shortest_distances() {
        let m = movie(
            "graph(g,\"a b c d\",\"a-b:1 a-c:4 b-c:2 b-d:5 c-d:1\",circular,(640,360),200); dijkstra(g,a);",
        );
        let sc = at(&m, 100.0);
        assert_eq!(text_of(&sc, "g.a.d"), "0");
        assert_eq!(text_of(&sc, "g.b.d"), "1");
        assert_eq!(text_of(&sc, "g.c.d"), "3");
        assert_eq!(text_of(&sc, "g.d.d"), "4");
    }

    #[test]
    fn dijkstra_unreachable_stays_inf() {
        // z is isolated -> never relaxed
        let m = movie("graph(g,\"a b z\",\"a-b:2\",circular,(640,360),200); dijkstra(g,a);");
        let sc = at(&m, 100.0);
        assert_eq!(text_of(&sc, "g.b.d"), "2");
        assert_eq!(text_of(&sc, "g.z.d"), "inf");
    }

    #[test]
    fn graph_bad_weight_errs() {
        assert!(crate::parse("graph(g,\"a b\",\"a-b:xyz\",row,(0,0),100);").is_err());
    }
}
