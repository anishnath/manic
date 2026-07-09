//! The **algo kit**: data-structure & algorithm vocabulary — the second
//! domain, proving the core is domain-agnostic (this file + one line in
//! `default_registry`, zero core changes).
//!
//! v1 centrepiece: `graph` (Manim's Graph / DiGraph). Nodes are labelled
//! circles, edges are lines (`a-b`) or arrows (`a>b`) trimmed to node borders,
//! laid out by a named layout. Everything is tagged so a whole graph animates
//! with one verb via tag-broadcast (`draw(g.edges)`, `flash(g.nodes, cyan)`).

use macroquad::prelude::Vec2;

use crate::lang::diag::Error;
use crate::lang::lower::{Args, Registry};
use crate::layout;
use crate::primitives::{Entity, FontKind, Link, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;

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

    let names: Vec<String> = verts_str.split_whitespace().map(|s| s.to_string()).collect();
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
        let mut node = Entity::new(nid.clone(), Shape::Circle { r: radius }, pos[i], style::PANEL);
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

    // edges, trimmed to node borders so arrowheads are visible
    for tok in edges_str
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
    {
        let directed = tok.contains('>');
        let sep = if directed { '>' } else { '-' };
        let parts: Vec<&str> = tok.split(sep).collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            return Err(Error::new(
                format!("bad edge `{tok}` — use `a-b` (line) or `a>b` (arrow)"),
                a.span_of(2),
            ));
        }
        let (u, v) = (parts[0], parts[1]);
        let iu = idx(u).ok_or_else(|| {
            Error::new(format!("edge `{tok}` uses unknown vertex `{u}`"), a.span_of(2))
        })?;
        let iv = idx(v).ok_or_else(|| {
            Error::new(format!("edge `{tok}` uses unknown vertex `{v}`"), a.span_of(2))
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
    }
    Ok(())
}

/// Register the algo kit into `r`.
pub fn register(r: &mut Registry) {
    r.ctor("graph", c_graph);
}
