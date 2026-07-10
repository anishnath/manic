//! Lowering: a parsed [`Program`] → a [`Movie`], via a builtin [`Registry`].
//!
//! This is the seam between the domain-agnostic front end and the kits. The
//! lowerer itself only knows a handful of reserved control-flow names
//! (`title`, `size`, `par`, `seq`, `stagger`, `section`, `wait`, `beat`,
//! `mark`). Every other call name is looked up in the registry, which kits
//! populate with **constructors** (declare entities at t=0) and **verbs**
//! (produce timeline clips). Meaning lives in the kits; structure lives here.
//!
//! Two passes over the top-level statements:
//! 1. **constructors** run in source order, building the base scene;
//! 2. **timeline** statements run in source order, appending clips.
//!
//! So an entity may be referenced by a beat that appears above its
//! declaration — order the cast and the script however reads best.

use std::collections::HashMap;

use macroquad::prelude::{Color, Vec2};

use crate::animate::{self, ActBuilder};
use crate::easing::Easing;
use crate::movie::Movie;
use crate::scene::Scene;
use crate::style;
use crate::timeline::Clip;

use super::ast::{BinOp, Ctrl, Expr, ExprKind, Program, ReduceOp, Seg, Stmt};
use super::diag::{Error, Span};
use super::parser::parse;

/// A constructor builtin: declare or modify entities in the base scene.
pub type CtorFn = fn(&mut Scene, &Args) -> Result<(), Error>;
/// A verb builtin: produce a timeline clip (reads the base scene for id lookups).
pub type VerbFn = fn(&Scene, &Args) -> Result<Clip, Error>;
/// A **mutating** verb builtin: produces a clip like a verb, but also gets
/// `&mut Scene` so it can carry build-time state forward between calls (e.g.
/// `swap` updating `Scene::occ`). This is what lets a chain of stateful steps —
/// sorting, stack push/pop, pointer moves — compose across the stateless
/// timeline. Usable inside `par`/`seq`/`stagger` too; block children lower in
/// source order, so the occupancy each one sees stays deterministic.
pub type MutVerbFn = fn(&mut Scene, &Args) -> Result<Clip, Error>;

/// The builtin table. Kits call [`Registry::ctor`] / [`Registry::verb`] to add
/// vocabulary; the lowerer dispatches call names through it.
#[derive(Default)]
pub struct Registry {
    ctors: HashMap<&'static str, CtorFn>,
    verbs: HashMap<&'static str, VerbFn>,
    mut_verbs: HashMap<&'static str, MutVerbFn>,
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    /// Register a constructor (declares/modifies entities at t=0).
    pub fn ctor(&mut self, name: &'static str, f: CtorFn) {
        self.ctors.insert(name, f);
    }

    /// Register a verb (produces a timeline clip).
    pub fn verb(&mut self, name: &'static str, f: VerbFn) {
        self.verbs.insert(name, f);
    }

    /// Register a mutating verb (produces a clip *and* may update `Scene::occ`).
    pub fn mut_verb(&mut self, name: &'static str, f: MutVerbFn) {
        self.mut_verbs.insert(name, f);
    }
}

/// Reserved control-flow names handled by the lowerer, never the registry.
fn is_reserved(name: &str) -> bool {
    matches!(
        name,
        "title"
            | "canvas"
            | "template"
            | "masthead"
            | "par"
            | "seq"
            | "stagger"
            | "section"
            | "wait"
            | "beat"
            | "mark"
    )
}

// ---- argument helpers -----------------------------------------------------

/// A typed, span-aware view over a call's arguments. Every accessor produces a
/// friendly error pointing at the right token when the shape is wrong.
pub struct Args<'a> {
    pub name: &'a str,
    pub name_span: Span,
    pub exprs: &'a [Expr],
}

impl<'a> Args<'a> {
    fn get(&self, i: usize) -> Result<&Expr, Error> {
        self.exprs.get(i).ok_or_else(|| {
            Error::new(
                format!(
                    "`{}` needs at least {} argument(s), got {}",
                    self.name,
                    i + 1,
                    self.exprs.len()
                ),
                self.name_span,
            )
        })
    }

    /// Number of arguments supplied.
    pub fn len(&self) -> usize {
        self.exprs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.exprs.is_empty()
    }

    /// A bare word argument (entity id, color name, easing name).
    pub fn ident(&self, i: usize) -> Result<String, Error> {
        let e = self.get(i)?;
        match &e.kind {
            ExprKind::Ident(s) => Ok(s.clone()),
            _ => Err(Error::new(
                format!("argument {} of `{}` should be a name", i + 1, self.name),
                e.span,
            )),
        }
    }

    /// A string-literal argument.
    pub fn text(&self, i: usize) -> Result<String, Error> {
        let e = self.get(i)?;
        match &e.kind {
            ExprKind::Str(s) => Ok(s.clone()),
            _ => Err(Error::new(
                format!("argument {} of `{}` should be a \"string\"", i + 1, self.name),
                e.span,
            )),
        }
    }

    /// A numeric argument.
    pub fn num(&self, i: usize) -> Result<f32, Error> {
        let e = self.get(i)?;
        match &e.kind {
            ExprKind::Num(n) => Ok(*n),
            _ => Err(Error::new(
                format!("argument {} of `{}` should be a number", i + 1, self.name),
                e.span,
            )),
        }
    }

    /// An optional numeric argument (returns `None` if absent).
    pub fn opt_num(&self, i: usize) -> Result<Option<f32>, Error> {
        match self.exprs.get(i) {
            None => Ok(None),
            Some(e) => match &e.kind {
                ExprKind::Num(n) => Ok(Some(*n)),
                _ => Err(Error::new(
                    format!("argument {} of `{}` should be a number", i + 1, self.name),
                    e.span,
                )),
            },
        }
    }

    /// A `(x, y)` coordinate pair.
    pub fn pair(&self, i: usize) -> Result<Vec2, Error> {
        let e = self.get(i)?;
        match &e.kind {
            ExprKind::Pair(x, y) => Ok(Vec2::new(*x, *y)),
            _ => Err(Error::new(
                format!("argument {} of `{}` should be a `(x, y)` point", i + 1, self.name),
                e.span,
            )),
        }
    }

    /// A point: either a literal `(x, y)` or the id of an existing entity
    /// (resolved to its current position).
    pub fn point(&self, i: usize, scene: &Scene) -> Result<Vec2, Error> {
        let e = self.get(i)?;
        match &e.kind {
            ExprKind::Pair(x, y) => Ok(Vec2::new(*x, *y)),
            ExprKind::Ident(id) => scene.get(id).map(|ent| ent.pos).ok_or_else(|| {
                Error::new(format!("no entity named `{id}` to point at"), e.span)
            }),
            _ => Err(Error::new(
                format!("argument {} of `{}` should be a `(x, y)` point or an entity name", i + 1, self.name),
                e.span,
            )),
        }
    }

    /// Error if there are more than `max` arguments.
    pub fn max(&self, max: usize) -> Result<(), Error> {
        if self.exprs.len() > max {
            Err(Error::new(
                format!("`{}` takes at most {} argument(s)", self.name, max),
                self.exprs[max].span,
            ))
        } else {
            Ok(())
        }
    }

    /// Span of argument `i` (falls back to the call name's span).
    pub fn span_of(&self, i: usize) -> Span {
        self.exprs.get(i).map(|e| e.span).unwrap_or(self.name_span)
    }
}

// ---- shared resolvers used by kits ---------------------------------------

/// Map a friendly color name to the neon palette. Kits use this for color
/// arguments so the whole language shares one vocabulary.
pub fn resolve_color(name: &str, span: Span) -> Result<Color, Error> {
    Ok(match name {
        "fg" | "white" => style::FG,
        "void" | "bg" => style::VOID,
        "cyan" | "blue" => style::CYAN,
        "magenta" | "pink" | "accent" | "red" => style::MAGENTA,
        "lime" | "green" => style::LIME,
        "dim" | "gray" | "grey" => style::DIM,
        "panel" => style::PANEL,
        other => {
            return Err(Error::new(
                format!(
                    "unknown color `{other}` (try: fg, cyan, magenta, lime, dim, panel, void)"
                ),
                span,
            ))
        }
    })
}

/// Map a friendly easing name to an [`Easing`] curve.
pub fn resolve_easing(name: &str, span: Span) -> Result<Easing, Error> {
    Ok(match name {
        "linear" => Easing::Linear,
        "smooth" | "inout" => Easing::InOutCubic,
        "in" => Easing::InCubic,
        "out" => Easing::OutCubic,
        "overshoot" | "back" => Easing::OutBack,
        "bounce" => Easing::OutBounce,
        "elastic" | "spring" => Easing::OutElastic,
        other => {
            return Err(Error::new(
                format!(
                    "unknown easing `{other}` (try: smooth, linear, in, out, overshoot, bounce, elastic)"
                ),
                span,
            ))
        }
    })
}

/// Apply optional trailing `duration` (number) and `easing` (name) arguments,
/// starting at argument index `from`, to an [`ActBuilder`]. This is the shared
/// `verb(id, target, [dur], [ease])` tail every motion verb uses.
pub fn apply_dur_ease(mut b: ActBuilder, a: &Args, from: usize) -> Result<ActBuilder, Error> {
    if let Some(d) = a.opt_num(from)? {
        b = b.dur(d);
    }
    if a.exprs.len() > from + 1 {
        let name = a.ident(from + 1)?;
        b = b.ease(resolve_easing(&name, a.span_of(from + 1))?);
    }
    Ok(b)
}

// ---- the lowerer ----------------------------------------------------------

/// Parse and lower manic source into a runnable [`Movie`], using `registry`
/// for all non-control-flow calls.
pub fn lower(src: &str, registry: &Registry) -> Result<Movie, Error> {
    let prog = parse(src)?;
    let prog = expand(&prog)?;
    lower_program(&prog, registry)
}

// ---- expand pass: resolve let/for/arithmetic/interpolation to literals -----

type Env = HashMap<String, f32>;

/// Iterations allowed per `for` loop — a guard against a runaway range.
const MAX_ITERS: i64 = 100_000;
/// Macro-call nesting allowed — a guard against non-terminating recursion.
const MAX_DEPTH: usize = 300;
/// Total statements the expand pass may emit — a runaway backstop.
const MAX_STMTS: usize = 500_000;

#[derive(Clone)]
struct Macro {
    params: Vec<String>,
    body: Vec<Stmt>,
}

/// Expansion-wide state: the macro table plus recursion/size guards.
struct Ctx {
    macros: HashMap<String, Macro>,
    depth: usize,
    emitted: usize,
}

/// Evaluate `let`/`for`/`def`/`if` and every argument expression against an
/// environment, producing a program whose statements are plain calls with
/// *literal* args (`Num`/`Str`/`Ident`/`Pair`) and no control constructs. Kits
/// therefore never see an unevaluated expression, and programs that use none of
/// these features pass through unchanged.
fn expand(prog: &Program) -> Result<Program, Error> {
    let mut env = Env::new();
    // Seed canvas-relative variables so authors can position with `cx`/`cy`/
    // `w`/`h` and stay canvas-independent. A later `let w = ...` may shadow them.
    let (cw, ch) = prog
        .stmts
        .iter()
        .find(|s| s.ctrl.is_none() && s.name == "canvas")
        .map(|s| canvas_dims(&s.args, s.name_span))
        .transpose()?
        .unwrap_or((1280, 720));
    env.insert("w".into(), cw as f32);
    env.insert("h".into(), ch as f32);
    env.insert("cx".into(), cw as f32 / 2.0);
    env.insert("cy".into(), ch as f32 / 2.0);

    let mut ctx = Ctx {
        macros: HashMap::new(),
        depth: 0,
        emitted: 0,
    };
    let stmts = expand_stmts(&prog.stmts, &mut env, &mut ctx)?;
    Ok(Program { stmts })
}

/// A numeric literal, if `e` is one.
fn num_of(e: &Expr) -> Option<f32> {
    match &e.kind {
        ExprKind::Num(n) => Some(*n),
        _ => None,
    }
}

/// Resolve a `canvas(...)` call's args to `(width, height)` — either a named
/// preset string or two numbers. Shared by the expand seed and movie setup.
fn canvas_dims(exprs: &[Expr], _span: Span) -> Result<(u32, u32), Error> {
    if let Some(first) = exprs.first() {
        if let ExprKind::Str(name) = &first.kind {
            return canvas_preset(name, first.span);
        }
    }
    let w = exprs.first().and_then(num_of).unwrap_or(1280.0);
    let h = exprs.get(1).and_then(num_of).unwrap_or(720.0);
    Ok((w.max(1.0) as u32, h.max(1.0) as u32))
}

/// Named canvas shapes so authors pick a format, not raw pixels.
fn canvas_preset(name: &str, span: Span) -> Result<(u32, u32), Error> {
    Ok(match name.trim().to_ascii_lowercase().as_str() {
        "16:9" | "widescreen" | "720p" => (1280, 720),
        "1080p" | "fullhd" | "hd" => (1920, 1080),
        "4k" | "2160p" => (3840, 2160),
        "square" | "1:1" => (1080, 1080),
        "portrait" | "9:16" | "vertical" | "story" | "reel" => (1080, 1920),
        "4:3" => (1280, 960),
        other => {
            return Err(Error::new(
                format!(
                    "unknown canvas preset {other:?} — try 16:9, 1080p, 4k, square, portrait, 4:3, or give width, height"
                ),
                span,
            ))
        }
    })
}

fn expand_stmts(stmts: &[Stmt], env: &mut Env, ctx: &mut Ctx) -> Result<Vec<Stmt>, Error> {
    let mut out = Vec::new();
    for s in stmts {
        match &s.ctrl {
            Some(Ctrl::Let(name, value)) => {
                let v = eval_expr(value, env)?;
                env.insert(name.clone(), v);
            }
            Some(Ctrl::For {
                var,
                start,
                end,
                body,
            }) => {
                let lo = eval_expr(start, env)?.round() as i64;
                let hi = eval_expr(end, env)?.round() as i64;
                if hi.saturating_sub(lo) > MAX_ITERS {
                    return Err(Error::new(
                        format!("`for` range is too large ({} iterations, max {MAX_ITERS})", hi - lo),
                        s.name_span,
                    ));
                }
                for k in lo..hi {
                    // each iteration gets its own scope so the loop var and any
                    // inner `let`s don't leak out
                    let mut child = env.clone();
                    child.insert(var.clone(), k as f32);
                    out.extend(expand_stmts(body, &mut child, ctx)?);
                }
            }
            Some(Ctrl::Def { name, params, body }) => {
                ctx.macros.insert(
                    name.clone(),
                    Macro {
                        params: params.clone(),
                        body: body.clone(),
                    },
                );
            }
            Some(Ctrl::If {
                cond,
                then_body,
                else_body,
            }) => {
                let chosen = if eval_expr(cond, env)?.abs() > 0.5 {
                    Some(then_body)
                } else {
                    else_body.as_ref()
                };
                if let Some(b) = chosen {
                    let mut child = env.clone();
                    out.extend(expand_stmts(b, &mut child, ctx)?);
                }
            }
            None => {
                // a call to a user macro?
                if let Some(m) = ctx.macros.get(&s.name).cloned() {
                    if s.args.len() != m.params.len() {
                        return Err(Error::new(
                            format!(
                                "macro `{}` takes {} argument(s), got {}",
                                s.name,
                                m.params.len(),
                                s.args.len()
                            ),
                            s.name_span,
                        ));
                    }
                    ctx.depth += 1;
                    if ctx.depth > MAX_DEPTH {
                        return Err(Error::new(
                            format!("macro `{}` recursed too deep ({MAX_DEPTH}) — missing a base case?", s.name),
                            s.name_span,
                        ));
                    }
                    // arguments evaluate in the caller's scope; the body runs in
                    // a fresh scope of the outer env overlaid with the params
                    let mut child = env.clone();
                    for (p, arg) in m.params.iter().zip(&s.args) {
                        let v = eval_expr(arg, env)?;
                        child.insert(p.clone(), v);
                    }
                    out.extend(expand_stmts(&m.body, &mut child, ctx)?);
                    ctx.depth -= 1;
                    continue;
                }
                // an ordinary kit call
                ctx.emitted += 1;
                if ctx.emitted > MAX_STMTS {
                    return Err(Error::new(
                        format!("too many statements generated (> {MAX_STMTS})"),
                        s.name_span,
                    ));
                }
                let args = s
                    .args
                    .iter()
                    .map(|e| resolve_arg(e, env))
                    .collect::<Result<Vec<_>, _>>()?;
                let block = match &s.block {
                    Some(b) => {
                        let mut child = env.clone();
                        Some(expand_stmts(b, &mut child, ctx)?)
                    }
                    None => None,
                };
                out.push(Stmt {
                    name: s.name.clone(),
                    name_span: s.name_span,
                    args,
                    block,
                    ctrl: None,
                });
            }
        }
    }
    Ok(out)
}

/// Resolve one argument expression to a literal. A bare `Ident` that names a
/// bound variable becomes its number; otherwise it stays a literal name (a
/// color, easing, entity id, or tag).
fn resolve_arg(e: &Expr, env: &Env) -> Result<Expr, Error> {
    let kind = match &e.kind {
        ExprKind::Num(n) => ExprKind::Num(*n),
        ExprKind::Str(s) => ExprKind::Str(s.clone()),
        ExprKind::Pair(x, y) => ExprKind::Pair(*x, *y),
        ExprKind::Ident(name) => match constant(name).or_else(|| env.get(name).copied()) {
            Some(v) => ExprKind::Num(v),
            None => ExprKind::Ident(name.clone()),
        },
        ExprKind::PairE(a, b) => ExprKind::Pair(eval_expr(a, env)?, eval_expr(b, env)?),
        ExprKind::Interp(segs) => ExprKind::Ident(interp(segs, env)?),
        ExprKind::Bin(..) | ExprKind::Neg(_) | ExprKind::Call(..) | ExprKind::Reduce { .. } => {
            ExprKind::Num(eval_expr(e, env)?)
        }
    };
    Ok(Expr { kind, span: e.span })
}

fn eval_expr(e: &Expr, env: &Env) -> Result<f32, Error> {
    match &e.kind {
        ExprKind::Num(n) => Ok(*n),
        ExprKind::Ident(name) => constant(name)
            .or_else(|| env.get(name).copied())
            .ok_or_else(|| Error::new(format!("unknown variable `{name}`"), e.span)),
        ExprKind::Neg(a) => Ok(-eval_expr(a, env)?),
        ExprKind::Bin(op, a, b) => {
            let (x, y) = (eval_expr(a, env)?, eval_expr(b, env)?);
            let b = |c: bool| if c { 1.0 } else { 0.0 };
            Ok(match op {
                BinOp::Add => x + y,
                BinOp::Sub => x - y,
                BinOp::Mul => x * y,
                BinOp::Div => x / y,
                BinOp::Pow => x.powf(y),
                BinOp::Lt => b(x < y),
                BinOp::Le => b(x <= y),
                BinOp::Gt => b(x > y),
                BinOp::Ge => b(x >= y),
                BinOp::Eq => b((x - y).abs() < 1e-9),
                BinOp::Ne => b((x - y).abs() >= 1e-9),
                BinOp::And => b(x.abs() > 0.5 && y.abs() > 0.5),
                BinOp::Or => b(x.abs() > 0.5 || y.abs() > 0.5),
            })
        }
        ExprKind::Call(name, arg) => {
            let x = eval_expr(arg, env)?;
            call_fn(name, x)
                .ok_or_else(|| Error::new(format!("unknown function `{name}`"), e.span))
        }
        ExprKind::Reduce {
            op,
            var,
            start,
            end,
            body,
        } => {
            let lo = eval_expr(start, env)?.round() as i64;
            let hi = eval_expr(end, env)?.round() as i64;
            if hi.saturating_sub(lo) > MAX_ITERS {
                return Err(Error::new(
                    format!("reduction range is too large (max {MAX_ITERS})"),
                    e.span,
                ));
            }
            let mut acc: Option<f32> = None;
            for k in lo..hi {
                let mut child = env.clone();
                child.insert(var.clone(), k as f32);
                let v = eval_expr(body, &child)?;
                acc = Some(match (op, acc) {
                    (_, None) => v,
                    (ReduceOp::Sum, Some(a)) => a + v,
                    (ReduceOp::Prod, Some(a)) => a * v,
                    (ReduceOp::Min, Some(a)) => a.min(v),
                    (ReduceOp::Max, Some(a)) => a.max(v),
                });
            }
            Ok(acc.unwrap_or(match op {
                ReduceOp::Prod => 1.0,
                _ => 0.0,
            }))
        }
        ExprKind::Str(_) => Err(Error::new("a string can't be used in arithmetic", e.span)),
        ExprKind::Pair(..) | ExprKind::PairE(..) => {
            Err(Error::new("a point can't be used in arithmetic", e.span))
        }
        ExprKind::Interp(_) => Err(Error::new("an id can't be used in arithmetic", e.span)),
    }
}

/// Built-in numeric constants, reserved in expression contexts.
fn constant(name: &str) -> Option<f32> {
    Some(match name {
        "pi" => std::f32::consts::PI,
        "tau" => std::f32::consts::TAU,
        "e" => std::f32::consts::E,
        _ => return None,
    })
}

fn call_fn(name: &str, x: f32) -> Option<f32> {
    Some(match name {
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "asin" => x.asin(),
        "acos" => x.acos(),
        "atan" => x.atan(),
        "sinh" => x.sinh(),
        "cosh" => x.cosh(),
        "tanh" => x.tanh(),
        "exp" => x.exp(),
        "sqrt" => x.sqrt(),
        "abs" => x.abs(),
        "ln" | "log" => x.ln(),
        "log10" => x.log10(),
        "log2" => x.log2(),
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "round" => x.round(),
        "sign" => x.signum(),
        _ => return None,
    })
}

fn interp(segs: &[Seg], env: &Env) -> Result<String, Error> {
    let mut s = String::new();
    for seg in segs {
        match seg {
            Seg::Lit(l) => s.push_str(l),
            Seg::Ex(e) => {
                let v = eval_expr(e, env)?;
                if (v.fract()).abs() < 1e-6 {
                    s.push_str(&format!("{}", v.round() as i64));
                } else {
                    s.push_str(&format!("{v}"));
                }
            }
        }
    }
    Ok(s)
}

fn args_of<'a>(s: &'a Stmt) -> Args<'a> {
    Args {
        name: &s.name,
        name_span: s.name_span,
        exprs: &s.args,
    }
}

fn lower_program(prog: &Program, registry: &Registry) -> Result<Movie, Error> {
    // phase 0 — movie metadata (title/size/template); first occurrence wins
    let mut title = "manic".to_string();
    let (mut w, mut h) = (1280u32, 720u32);
    let mut template: Option<(String, Span)> = None;
    let mut masthead: Option<(String, Option<String>)> = None;
    for s in &prog.stmts {
        match s.name.as_str() {
            "title" => title = args_of(s).text(0)?,
            "canvas" => {
                let (cw, ch) = canvas_dims(&s.args, s.name_span)?;
                w = cw;
                h = ch;
            }
            "template" => {
                if template.is_none() {
                    template = Some((args_of(s).text(0)?, s.name_span));
                }
            }
            "masthead" => {
                if masthead.is_none() {
                    let a = args_of(s);
                    let right = if a.len() > 1 { Some(a.text(1)?) } else { None };
                    masthead = Some((a.text(0)?, right));
                }
            }
            _ => {}
        }
    }
    let mut movie = Movie::new(&title, w, h);
    if let Some((name, span)) = template {
        movie.template = crate::style::Template::by_name(&name).ok_or_else(|| {
            Error::new(
                format!("unknown template `{name}` — try `plain` or `terminal`"),
                span,
            )
        })?;
    }
    if let Some((left, right)) = masthead {
        movie.template.masthead_left = left;
        if let Some(right) = right {
            movie.template.masthead_right = right;
        }
    }

    // classify + fail fast on unknown names
    for s in &prog.stmts {
        classify(&s.name, s.name_span, registry)?;
    }

    // phase A — constructors, in source order
    for s in &prog.stmts {
        if let Some(f) = registry.ctors.get(s.name.as_str()) {
            run_ctor(*f, &mut movie.scene, s)?;
        }
    }

    // phase B — timeline statements, in source order
    for s in &prog.stmts {
        match Class::of(&s.name, registry) {
            Class::Timeline => lower_top_timeline(&mut movie, s, registry)?,
            _ => {}
        }
    }

    Ok(movie)
}

enum Class {
    Meta,
    Ctor,
    Timeline,
    Unknown,
}

impl Class {
    fn of(name: &str, registry: &Registry) -> Class {
        if matches!(name, "title" | "canvas" | "template" | "masthead") {
            Class::Meta
        } else if registry.ctors.contains_key(name) {
            Class::Ctor
        } else if is_reserved(name)
            || registry.verbs.contains_key(name)
            || registry.mut_verbs.contains_key(name)
        {
            Class::Timeline
        } else {
            Class::Unknown
        }
    }
}

fn classify(name: &str, span: Span, registry: &Registry) -> Result<(), Error> {
    match Class::of(name, registry) {
        Class::Unknown => Err(Error::new(
            format!("unknown builtin `{name}` — no kit provides it"),
            span,
        )),
        _ => Ok(()),
    }
}

/// A top-level timeline statement: a section/beat/mark, a `par`/`seq`/`stagger`
/// block, or a verb — appended to the movie's timeline at the cursor.
fn lower_top_timeline(movie: &mut Movie, s: &Stmt, registry: &Registry) -> Result<(), Error> {
    let a = args_of(s);
    match s.name.as_str() {
        "section" => {
            movie.section(&a.text(0)?);
            Ok(())
        }
        "mark" => {
            movie.mark(&a.text(0)?);
            Ok(())
        }
        "wait" | "beat" => {
            movie.wait(a.num(0)?);
            Ok(())
        }
        "par" | "seq" | "stagger" => {
            let clip = build_block_scene(&mut movie.scene, s, registry)?;
            movie.play(clip);
            Ok(())
        }
        _ => {
            // a mutating verb carries state forward, so it gets `&mut scene`
            if let Some(f) = registry.mut_verbs.get(s.name.as_str()) {
                let clip = f(&mut movie.scene, &args_of(s))?;
                movie.play(clip);
                return Ok(());
            }
            // a plain verb; needs a read of the (now complete) base scene
            let f = registry.verbs.get(s.name.as_str()).expect("classified as verb");
            let clip = run_verb(*f, &movie.scene, s)?;
            movie.play(clip);
            Ok(())
        }
    }
}

/// Ids of entities carrying `tag` (empty if none).
fn tagged_ids(scene: &Scene, tag: &str) -> Vec<String> {
    scene
        .entities
        .iter()
        .filter(|e| e.tags.iter().any(|t| t == tag))
        .map(|e| e.id.clone())
        .collect()
}

/// Invoke a constructor/modifier, broadcasting over a tag group if the first
/// argument names a tag rather than an entity — so `hidden(g.nodes)` or
/// `color(g.edges, dim)` apply to the whole group at t=0.
fn run_ctor(f: CtorFn, scene: &mut Scene, s: &Stmt) -> Result<(), Error> {
    if let Some(first) = s.args.first() {
        if let ExprKind::Ident(name) = &first.kind {
            if !scene.contains(name) {
                let ids = tagged_ids(scene, name);
                if !ids.is_empty() {
                    for id in ids {
                        let mut args2 = s.args.clone();
                        args2[0] = Expr {
                            kind: ExprKind::Ident(id),
                            span: first.span,
                        };
                        let a2 = Args {
                            name: &s.name,
                            name_span: s.name_span,
                            exprs: &args2,
                        };
                        f(scene, &a2)?;
                    }
                    return Ok(());
                }
            }
        }
    }
    f(scene, &args_of(s))
}

/// Invoke a verb, broadcasting over a tag group if the first argument names a
/// tag rather than an entity. So `draw(g.edges)` runs `draw` on every entity
/// tagged `g.edges`, in parallel — the ergonomic that makes graphs, cells, and
/// other groups usable in the language.
fn run_verb(f: VerbFn, scene: &Scene, s: &Stmt) -> Result<Clip, Error> {
    if let Some(first) = s.args.first() {
        if let ExprKind::Ident(name) = &first.kind {
            if !scene.contains(name) {
                let ids = tagged_ids(scene, name);
                if !ids.is_empty() {
                    let mut clips = Vec::with_capacity(ids.len());
                    for id in ids {
                        let mut args2 = s.args.clone();
                        args2[0] = Expr {
                            kind: ExprKind::Ident(id),
                            span: first.span,
                        };
                        let a2 = Args {
                            name: &s.name,
                            name_span: s.name_span,
                            exprs: &args2,
                        };
                        clips.push(f(scene, &a2)?);
                    }
                    return Ok(Clip::par(clips));
                }
            }
        }
    }
    f(scene, &args_of(s))
}

/// Lower a statement that appears *inside* a `par`/`seq`/`stagger` block into a
/// clip. Only timeline-producing statements are legal here.
fn lower_inner(scene: &mut Scene, s: &Stmt, registry: &Registry) -> Result<Clip, Error> {
    match s.name.as_str() {
        "wait" | "beat" => Ok(Clip::wait(args_of(s).num(0)?)),
        "par" | "seq" | "stagger" => build_block_scene(scene, s, registry),
        "section" | "mark" => Err(Error::new(
            format!("`{}` can't appear inside a par/seq/stagger block", s.name),
            s.name_span,
        )),
        _ => {
            // a mutating verb carries state forward, so it gets `&mut scene`
            if let Some(f) = registry.mut_verbs.get(s.name.as_str()) {
                return f(scene, &args_of(s));
            }
            match registry.verbs.get(s.name.as_str()) {
                Some(f) => run_verb(*f, scene, s),
                None => {
                    // constructor or unknown used in a timeline block
                    if registry.ctors.contains_key(s.name.as_str()) {
                        Err(Error::new(
                            format!(
                                "`{}` declares an entity and can't appear inside a par/seq/stagger block",
                                s.name
                            ),
                            s.name_span,
                        ))
                    } else {
                        Err(Error::new(format!("unknown builtin `{}`", s.name), s.name_span))
                    }
                }
            }
        }
    }
}

fn build_block_scene(scene: &mut Scene, s: &Stmt, registry: &Registry) -> Result<Clip, Error> {
    let block = s
        .block
        .as_ref()
        .ok_or_else(|| Error::new(format!("`{}` needs a `{{ ... }}` block", s.name), s.name_span))?;
    let mut clips = Vec::with_capacity(block.len());
    for inner in block {
        clips.push(lower_inner(scene, inner, registry)?);
    }
    Ok(match s.name.as_str() {
        "par" => Clip::par(clips),
        "seq" => Clip::seq(clips),
        "stagger" => {
            let delay = args_of(s).num(0)?;
            animate::stagger(delay, clips)
        }
        _ => unreachable!(),
    })
}
