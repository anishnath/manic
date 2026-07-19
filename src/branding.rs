//! Engine **branding** — applied to recorded output under a branded preset,
//! never authored in (or visible to) the DSL.
//!
//! Two pieces:
//! - a short **branding clip** — placed at the end (outro, default) or, with
//!   `--intro`, at the start — the neon `manic` wordmark typed out over the
//!   project link, authored here *in manic* so it reuses the same engine but
//!   stays completely separate from the user's file;
//! - a pinned **"Made With Manic" watermark** for the whole DSL portion.
//!
//! The intro is built to match the target canvas, then composed ahead of the
//! user's timeline by the player at record time.

use macroquad::prelude::Vec2;

use crate::movie::Movie;
use crate::primitives::{Entity, FontKind, Shape};
use crate::style;

/// The project link shown in the intro / watermark.
pub const LINK: &str = "https://8gwifi.org/manic";

/// A self-contained manic program for the pre-roll, sized to `w`×`h`: the
/// hue-graded fractal tree grows (yellow trunk → magenta/blue tips), the
/// `Manic` wordmark typewrites in beside it over the link, then all fade out.
/// Authored here in manic so it reuses the engine but is wholly separate from
/// the user's DSL.
pub fn intro_source(w: u32, h: u32) -> String {
    let (wf, hf) = (w as f32, h as f32);
    let cx = wf / 2.0;
    let ry = hf * 0.965; // trunk root near the bottom
    let len = hf * 0.21; // first branch length
    let mx = wf * 0.62; // "Manic" sits to the right of the trunk
    let my = hf * 0.74;
    let mksz = hf * 0.09;
    let ly = hf - 42.0; // link near the bottom
    format!(
        "title(\"manic\");\n\
         canvas({w}, {h});\n\
         def branch(k, x, y, ang, len, depth) {{\n\
         \x20 if depth > 0 && len > 2 {{\n\
         \x20   let x2 = x + len*cos(ang);\n\
         \x20   let y2 = y - len*sin(ang);\n\
         \x20   line(seg{{k}}, (x, y), (x2, y2));\n\
         \x20   stroke(seg{{k}}, 1 + depth*0.8);\n\
         \x20   hue(seg{{k}}, 250 + depth*8.5);\n\
         \x20   untraced(seg{{k}});  tag(seg{{k}}, tree);\n\
         \x20   branch(2*k,     x2, y2, ang + 0.42, len*0.72, depth - 1);\n\
         \x20   branch(2*k + 1, x2, y2, ang - 0.42, len*0.72, depth - 1);\n\
         \x20 }}\n\
         }}\n\
         branch(1, {cx}, {ry}, 1.5708, {len}, 12);\n\
         text(mk, ({mx}, {my}), \"Manic\");  color(mk, magenta);  size(mk, {mksz});  glow(mk, 8);  untraced(mk);\n\
         text(lk, ({cx}, {ly}), \"{LINK}\");  color(lk, dim);  size(lk, 30);  hidden(lk);\n\
         par {{ draw(tree, 2.0);  seq {{ wait(0.8);  type(mk); }} }}\n\
         show(lk, 0.5);\n\
         wait(0.8);\n\
         par {{ fade(tree);  fade(mk);  fade(lk); }}\n"
    )
}

/// Pin a small glowing "Made With Manic" watermark, screen-fixed, for the whole
/// video. Added to the base scene so it persists across the DSL timeline.
pub fn add_watermark(m: &mut Movie) {
    let (w, h) = (m.width as f32, m.height as f32);
    let mut wm = Entity::new(
        "__brand.wm".to_string(),
        Shape::Text {
            content: "Made With Manic".to_string(),
            size: 22.0,
        },
        Vec2::new(w - 150.0, h - 26.0),
        style::DIM,
    );
    wm.font = FontKind::MonoBold;
    wm.z = 9999;
    wm.sticky = true;
    wm.glow = 0.7;
    m.scene.add(wm);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intro_parses_for_common_sizes() {
        for (w, h) in [(1280, 720), (1920, 1080), (1080, 1920)] {
            assert!(
                crate::parse(&intro_source(w, h)).is_ok(),
                "intro should parse for {w}x{h}"
            );
        }
    }

    #[test]
    fn watermark_added_to_scene() {
        let mut m = Movie::new("t", 1280, 720);
        add_watermark(&mut m);
        assert!(m.scene.contains("__brand.wm"));
    }
}
