//! **manic-lang** — the front-end of the [manic](https://github.com/anishnath/manic)
//! animation DSL: `text → tokens → AST`, plus precise diagnostics. It has **no
//! graphics dependency** (no macroquad), so it compiles to `wasm32` and powers
//! in-browser editor tooling — syntax highlighting, autocomplete, and
//! error-checking — with the *same* lexer and parser the renderer uses (one
//! grammar, no drift).
//!
//! The renderer's lowering (`text → Scene/Timeline`) and the kit registry live
//! in the `manic` engine crate, which depends on this one.

pub mod ast;
pub mod catalog;
pub mod diag;
pub mod expand;
pub mod lexer;
pub mod parser;
pub mod services;

#[cfg(feature = "wasm")]
pub mod wasm;

pub use diag::{Error, Span};
pub use parser::parse as parse_program;
