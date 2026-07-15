//! # manic
//!
//! A general-purpose 2D animation **language and engine** with foundational
//! hybrid 3D support, neon-terminal styled, deterministic, built on macroquad.
//!
//! Authors write animations in the `.manic` text language (see the `lang`
//! module) — no Rust required. The language lowers onto this engine, which
//! is organized around a small, scriptable pipeline:
//!
//! 1. Build a [`movie::Movie`] with a base scene.
//! 2. Declare visual entities with [`scene::SceneBuilder`].
//! 3. Add animation clips with [`animate::act`], [`seq!`], [`par!`], and
//!    [`stagger!`].
//! 4. Hand the movie to [`run`] for live preview or deterministic recording.
//!
//! ## Architecture
//!
//! The **core** (this crate's engine modules + the `lang` front end) is
//! domain-agnostic: it knows nothing about algorithms, math, or any other
//! subject. Domain vocabulary lives in **kits** that register builtins into
//! the language (see `kits`). The parser is generic — it knows no verb names;
//! call meaning is resolved at lowering time against a builtin registry, so a
//! new domain is a new file, not a core change.
//!
//! ## Core Concepts
//!
//! - [`movie::Movie`] stores the base scene, timeline clips, section jumps,
//!   and beat marks.
//! - [`scene`] owns entity declaration: circles, rectangles, lines, arrows,
//!   text, cells, code blocks, labels, tags, and follow relationships.
//! - [`animate`] provides the fluent verb DSL: move, fade, highlight, pulse,
//!   trace, type, retarget, and camera moves.
//! - [`timeline`] resolves clips into absolute tracks. Its evaluation is a
//!   pure function of time, so pause, scrub, frame stepping, and offline
//!   recording are deterministic.
//! - [`render`] turns a scene snapshot into macroquad draw calls with the
//!   neon-terminal style from [`style`].
//! - [`layout`] contains small coordinate helpers for rows, grids, trees,
//!   and rings.

pub mod animate;
pub mod branding;
pub mod easing;
pub mod geom;
pub mod kits;
pub mod lang;
pub mod layout;
pub mod movie;
pub mod ode;
pub mod player;
pub mod preset;
pub mod primitives;
pub mod primitives3d;
pub mod record;
pub mod render;
pub mod render3d;
pub mod scene;
pub mod style;
pub mod timeline;

use macroquad::prelude::Vec2;

/// Shorthand position constructor: `v(100., 200.)`.
pub fn v(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// Parse a `.manic` source string into a runnable [`movie::Movie`], using the
/// default kit registry (std + math + …). This is the language's front door;
/// the `manic` CLI is a thin wrapper over it.
pub fn parse(src: &str) -> Result<movie::Movie, lang::diag::Error> {
    lang::lower::lower(src, &kits::default_registry())
}

/// Open a window and run the movie (live preview, or `--record` offline).
///
/// Call this from a plain `fn main()` — no macroquad attribute needed.
pub fn run(movie: movie::Movie) {
    let opts = player::parse_opts();
    let conf = macroquad::window::Conf {
        window_title: movie.title.clone(),
        window_width: (movie.width as f32 * opts.scale) as i32,
        window_height: (movie.height as f32 * opts.scale) as i32,
        high_dpi: false,
        window_resizable: true,
        // 4x MSAA: smooth circle/line/diagonal edges
        sample_count: 4,
        ..Default::default()
    };
    macroquad::Window::from_config(conf, player::run_loop(movie));
}

/// Everything a movie script needs: `use manic::prelude::*;`
pub mod prelude {
    pub use crate::animate::{act, all, flash, stagger, wait, ActBuilder};
    pub use crate::easing::Easing::{self, *};
    pub use crate::layout;
    pub use crate::movie::Movie;
    pub use crate::style::{CYAN, DIM, FG, LIME, MAGENTA, PANEL, VOID};
    pub use crate::timeline::Clip;
    pub use crate::v;
    pub use crate::{par, seq, stagger};
    pub use macroquad::prelude::{Color, Vec2, Vec3};
}
