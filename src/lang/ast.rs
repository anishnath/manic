//! The manic AST: a program is a flat list of statements, each a *call*
//! (name + args + optional brace block). The AST is deliberately generic —
//! it carries no notion of what a call *means*. `move`, `par`, `section`,
//! `plot` are all just names here; the lowering pass resolves them against
//! the kit registry. That is the seam that lets a new domain be a new file.

use super::diag::Span;

/// A leaf argument value.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A numeric literal.
    Num(f32),
    /// A `"..."` string literal.
    Str(String),
    /// A bare word — an entity id, a color name, an easing name, etc.
    Ident(String),
    /// A `(x, y)` coordinate pair.
    Pair(f32, f32),
}

/// An argument expression with its source span (for error reporting).
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// One statement: `name(args) ;` or `name(args) { block }` or `name { block }`.
/// `block` is `Some` exactly when the statement used a brace block.
#[derive(Debug, Clone)]
pub struct Stmt {
    pub name: String,
    /// Span of the name token — where "unknown builtin" errors point.
    pub name_span: Span,
    pub args: Vec<Expr>,
    pub block: Option<Vec<Stmt>>,
}

/// A parsed manic program.
#[derive(Debug, Clone)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}
