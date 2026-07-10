//! The manic language front end: text → tokens → AST → (lowering to a
//! [`crate::movie::Movie`], added next). Domain-agnostic — the parser knows
//! no verb names; meaning is resolved against the kit registry at lower time.

// The front-end (lexer/parser/ast/diag) now lives in the macroquad-free
// `manic-lang` crate so it can also target WASM for browser editor tooling.
// Re-exported here so the rest of the engine keeps using `crate::lang::…`.
pub use manic_lang::{ast, diag, lexer, parser};

pub mod lower;
