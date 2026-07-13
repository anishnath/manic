//! The manic AST: a program is a flat list of statements, each a *call*
//! (name + args + optional brace block). The AST is deliberately generic —
//! it carries no notion of what a call *means*. `move`, `par`, `section`,
//! `plot` are all just names here; the lowering pass resolves them against
//! the kit registry. That is the seam that lets a new domain be a new file.
//!
//! Expressions can be *unevaluated* after parsing (`Bin`, `Neg`, `Call`,
//! `PairE`, `Interp`, or an `Ident` that names a variable). The **expand** pass
//! in `lower` evaluates them against an environment built from `let` bindings
//! and `for` loops, collapsing every expression to a literal (`Num`, `Str`,
//! `Ident`, `Pair`) before a kit ever sees it. So kits stay literal-only.

use crate::diag::Span;

/// A binary operator. Comparisons and logicals evaluate to `1.0` / `0.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

/// One piece of an interpolated identifier like `bar{i}` or `cell{i}x{j}`.
#[derive(Debug, Clone, PartialEq)]
pub enum Seg {
    /// A literal chunk (`bar`, `cell`, `x`).
    Lit(String),
    /// A `{ expr }` chunk, formatted as a number (integers lose the `.0`).
    Ex(Box<Expr>),
}

/// An argument value — a literal after expansion, possibly a computation before.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A numeric literal.
    Num(f32),
    /// A `"..."` string literal.
    Str(String),
    /// A bare word — an entity id, color/easing name, or (in an arithmetic
    /// context, or if bound by `let`/`for`) a variable reference.
    Ident(String),
    /// A `(x, y)` coordinate pair of literals (post-expand form).
    Pair(f32, f32),
    /// A `(x, y, z)` coordinate triple of literals (post-expand form).
    Triple(f32, f32, f32),

    // ---- pre-expand forms (gone after `expand`) ----
    /// `a <op> b`.
    Bin(BinOp, Box<Expr>, Box<Expr>),
    /// Unary minus.
    Neg(Box<Expr>),
    /// A math function applied to one argument, e.g. `sin(x)`.
    Call(String, Box<Expr>),
    /// A `(x, y)` pair whose components are (possibly) computed.
    PairE(Box<Expr>, Box<Expr>),
    /// A `(x, y, z)` triple whose components are computed expressions.
    TripleE(Box<Expr>, Box<Expr>, Box<Expr>),
    /// An interpolated identifier, e.g. `bar{i}`.
    Interp(Vec<Seg>),
    /// A reduction over a range: `sum(i in a..b : body)` (also `prod`/`min`/`max`).
    Reduce {
        op: ReduceOp,
        var: String,
        start: Box<Expr>,
        end: Box<Expr>,
        body: Box<Expr>,
    },
}

/// A range reduction operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReduceOp {
    Sum,
    Prod,
    Min,
    Max,
}

/// An argument expression with its source span (for error reporting).
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

/// A control construct that the **expand** pass resolves away (it never reaches
/// lowering). Attached to a [`Stmt`] via `ctrl`.
#[derive(Debug, Clone)]
pub enum Ctrl {
    /// `let name = value;`
    Let(String, Expr),
    /// `for var in start..end { body }` — `var` walks the integers
    /// `[start, end)`.
    For {
        var: String,
        start: Expr,
        end: Expr,
        body: Vec<Stmt>,
    },
    /// `def name(params) { body }` — a reusable macro (params are numbers).
    Def {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    /// `if cond { then } [else { otherwise }]`.
    If {
        cond: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<Vec<Stmt>>,
    },
}

/// One statement: `name(args) ;` / `name(args) { block }` / `name { block }`,
/// or — when `ctrl` is `Some` — a `let` / `for` control construct.
#[derive(Debug, Clone)]
pub struct Stmt {
    pub name: String,
    /// Span of the name token — where "unknown builtin" errors point.
    pub name_span: Span,
    pub args: Vec<Expr>,
    pub block: Option<Vec<Stmt>>,
    /// A control construct (`let`/`for`), resolved by the expand pass.
    pub ctrl: Option<Ctrl>,
}

/// A parsed manic program.
#[derive(Debug, Clone)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}
