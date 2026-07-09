//! The manic language front end: text → tokens → AST → (lowering to a
//! [`crate::movie::Movie`], added next). Domain-agnostic — the parser knows
//! no verb names; meaning is resolved against the kit registry at lower time.

pub mod ast;
pub mod diag;
pub mod lexer;
pub mod lower;
pub mod parser;
