//! The **brand kit**: manic's own banner and watermark (à la `ManimBanner`).
//!
//! - `banner(id, (cx,cy), [scale])` — the neon logo: a circle / square /
//!   triangle icon trio + the "manic" wordmark. Tagged `{id}.icon` (the three
//!   shapes) and `id`; the wordmark is `{id}.word`. Animate create→expand with
//!   `draw({id}.icon)` then `show({id}.word)`.
//! - `watermark(id, (x,y), ["text"])` — a small, glowing, screen-fixed mark
//!   that stays put through camera moves; drop it once and it persists.

use macroquad::prelude::Vec2;

use crate::lang::diag::Error;
use crate::lang::lower::{Args, Registry};
use crate::primitives::{Entity, FontKind, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;

fn neon(color: macroquad::prelude::Color) -> StrokeStyle {
    StrokeStyle {
        fill: true,
        outline: true,
        width: 3.0,
        outline_color: Some(color),
    }
}

/// `banner(id, (cx,cy), [scale])` — the manic logo/banner.
fn c_banner(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let k = a.opt_num(2)?.unwrap_or(1.0);
    let icon = format!("{id}.icon");

    // icon trio: cyan circle, magenta square, lime triangle
    let mut circ = Entity::new(
        format!("{id}.dot"),
        Shape::Circle { r: 26.0 * k },
        Vec2::new(c.x - 230.0 * k, c.y),
        style::PANEL,
    );
    circ.stroke = neon(style::CYAN);
    circ.z = 5;
    circ.tags = vec![id.clone(), icon.clone()];
    s.add(circ);

    let mut sq = Entity::new(
        format!("{id}.sq"),
        Shape::Rect {
            w: 48.0 * k,
            h: 48.0 * k,
        },
        Vec2::new(c.x - 150.0 * k, c.y),
        style::PANEL,
    );
    sq.stroke = neon(style::MAGENTA);
    sq.z = 5;
    sq.tags = vec![id.clone(), icon.clone()];
    s.add(sq);

    let ct = Vec2::new(c.x - 70.0 * k, c.y);
    let tri_pts = vec![
        Vec2::new(ct.x, ct.y - 30.0 * k),
        Vec2::new(ct.x - 28.0 * k, ct.y + 22.0 * k),
        Vec2::new(ct.x + 28.0 * k, ct.y + 22.0 * k),
    ];
    let mut tri = Entity::new(
        format!("{id}.tri"),
        Shape::Polygon { pts: tri_pts },
        Vec2::ZERO,
        style::PANEL,
    );
    tri.stroke = neon(style::LIME);
    tri.z = 5;
    tri.tags = vec![id.clone(), icon.clone()];
    s.add(tri);

    // wordmark
    let mut word = Entity::new(
        format!("{id}.word"),
        Shape::Text {
            content: "manic".into(),
            size: 64.0 * k,
        },
        Vec2::new(c.x + 110.0 * k, c.y),
        style::CYAN,
    );
    word.font = FontKind::Display;
    word.z = 6;
    word.tags = vec![id];
    s.add(word);
    Ok(())
}

/// `watermark(id, (x,y), ["text"])` — a persistent, screen-fixed brand mark.
fn c_watermark(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let pos = a.pair(1)?;
    let text = if a.len() > 2 {
        a.text(2)?
    } else {
        "manic".to_string()
    };
    let mut e = Entity::new(
        id,
        Shape::Text {
            content: text,
            size: 20.0,
        },
        pos,
        style::DIM,
    );
    e.font = FontKind::MonoBold;
    e.sticky = true; // fixed to the screen, ignores camera pan/zoom
    e.z = 200;
    e.glow = 0.8;
    s.add(e);
    Ok(())
}

/// Register the brand kit into `r`.
pub fn register(r: &mut Registry) {
    r.ctor("banner", c_banner);
    r.ctor("watermark", c_watermark);
}
