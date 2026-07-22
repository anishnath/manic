//! The **expand pass**: `let` / `for` / `if` / `def` macros / reductions /
//! interpolation → a flat list of literal calls. Pure AST->AST (no engine
//! types), so it also runs in the browser (WASM) to validate control flow
//! before the renderer lowers the result to a Scene.

use std::collections::HashMap;

use crate::ast::{BinOp, Ctrl, Expr, ExprKind, Program, ReduceOp, Seg, Stmt};
use crate::catalog::{catalog, Ty};
use crate::diag::{Error, Span};

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
    /// Per-builtin, the param name of each `Str`-typed slot (`None` for non-string
    /// positions). Lets the arg pass flag bare, unquoted LaTeX/text — the classic
    /// `equation(q,(x,y),-i)` slip — with a "wrap in backticks" fix, instead of
    /// mis-evaluating it as arithmetic and guessing a nearest variable.
    str_slots: HashMap<&'static str, Vec<Option<&'static str>>>,
}

/// Evaluate `let`/`for`/`def`/`if` and every argument expression against an
/// environment, producing a program whose statements are plain calls with
/// *literal* args (`Num`/`Str`/`Ident`/`Pair`) and no control constructs. Kits
/// therefore never see an unevaluated expression, and programs that use none of
/// these features pass through unchanged.
pub fn expand(prog: &Program) -> Result<Program, Error> {
    expand_with_canvas(prog, None)
}

/// Expand a program while optionally replacing its logical canvas size.
///
/// The override is applied before `w`/`h`/`cx`/`cy`, conditionals, loops, and
/// macros are evaluated. Renderers can therefore reframe one responsive source
/// without rewriting its AST or changing ordinary editor/browser expansion.
pub fn expand_with_canvas(
    prog: &Program,
    canvas_override: Option<(u32, u32)>,
) -> Result<Program, Error> {
    let mut env = Env::new();
    // Seed canvas-relative variables so authors can position with `cx`/`cy`/
    // `w`/`h` and stay canvas-independent. A later `let w = ...` may shadow them.
    let (cw, ch) = match canvas_override {
        Some(size) => size,
        None => prog
            .stmts
            .iter()
            .find(|s| s.ctrl.is_none() && s.name == "canvas")
            .map(|s| canvas_dims(&s.args, s.name_span))
            .transpose()?
            .unwrap_or((1280, 720)),
    };
    env.insert("w".into(), cw as f32);
    env.insert("h".into(), ch as f32);
    env.insert("cx".into(), cw as f32 / 2.0);
    env.insert("cy".into(), ch as f32 / 2.0);

    let str_slots = catalog()
        .iter()
        .map(|s| {
            let slots = s
                .params
                .iter()
                .map(|p| (p.ty == Ty::Str).then_some(p.name))
                .collect();
            (s.name, slots)
        })
        .collect();
    let mut ctx = Ctx {
        macros: HashMap::new(),
        depth: 0,
        emitted: 0,
        str_slots,
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
pub fn canvas_dims(exprs: &[Expr], _span: Span) -> Result<(u32, u32), Error> {
    if let Some(first) = exprs.first() {
        if let ExprKind::Str(name) = &first.kind {
            return canvas_preset(name, first.span);
        }
    }
    let w = exprs.first().and_then(num_of).unwrap_or(1280.0);
    let h = exprs.get(1).and_then(num_of).unwrap_or(720.0);
    Ok((w.max(1.0) as u32, h.max(1.0) as u32))
}

/// Resolve a command-line canvas value such as `portrait`, `4:5`, `square`,
/// `16:9`, or `1080x1920` into logical pixels.
pub fn canvas_override_dims(spec: &str) -> Result<(u32, u32), String> {
    let clean = spec.trim().to_ascii_lowercase();
    if let Some(size) = canvas_preset_dims(&clean) {
        return Ok(size);
    }
    if let Some((w, h)) = clean.split_once('x') {
        let w: u32 = w
            .trim()
            .parse()
            .map_err(|_| format!("invalid width in `{spec}`"))?;
        let h: u32 = h
            .trim()
            .parse()
            .map_err(|_| format!("invalid height in `{spec}`"))?;
        if w == 0 || h == 0 {
            return Err("canvas width and height must be greater than zero".into());
        }
        return Ok((w, h));
    }
    Err(format!(
        "unknown format `{spec}` — try portrait, 4:5, square, 16:9, or WIDTHxHEIGHT"
    ))
}

/// Named canvas shapes so authors pick a format, not raw pixels.
fn canvas_preset(name: &str, span: Span) -> Result<(u32, u32), Error> {
    let clean = name.trim().to_ascii_lowercase();
    canvas_preset_dims(&clean).ok_or_else(|| {
        Error::new(
            format!(
                "unknown canvas preset {clean:?} — try 16:9, 1080p, 4k, square, portrait, 4:5, 4:3, or give width, height"
            ),
            span,
        )
    })
}

fn canvas_preset_dims(name: &str) -> Option<(u32, u32)> {
    Some(match name {
        "16:9" | "widescreen" | "landscape" | "720p" => (1280, 720),
        "1080p" | "fullhd" | "hd" => (1920, 1080),
        "4k" | "2160p" => (3840, 2160),
        "square" | "1:1" => (1080, 1080),
        "portrait" | "9:16" | "vertical" | "story" | "reel" => (1080, 1920),
        "4:5" | "feed" => (1080, 1350),
        "4:3" => (1280, 960),
        _ => return None,
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
                        format!(
                            "`for` range is too large ({} iterations, max {MAX_ITERS})",
                            hi - lo
                        ),
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
                            format!(
                                "macro `{}` recursed too deep ({MAX_DEPTH}) — missing a base case?",
                                s.name
                            ),
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
                let slots = ctx.str_slots.get(s.name.as_str());
                let args = s
                    .args
                    .iter()
                    .enumerate()
                    .map(
                        |(i, e)| match slots.and_then(|v| v.get(i)).copied().flatten() {
                            // a `Str`-typed slot: a bare identifier/expression here is
                            // unquoted LaTeX/text — flag it with a backtick fix rather
                            // than evaluating it as arithmetic.
                            Some(param) => resolve_str_arg(e, env, &s.name, param),
                            None => resolve_arg(e, env),
                        },
                    )
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
        ExprKind::Triple(x, y, z) => ExprKind::Triple(*x, *y, *z),
        ExprKind::Ident(name) => match constant(name).or_else(|| env.get(name).copied()) {
            Some(v) => ExprKind::Num(v),
            None => ExprKind::Ident(name.clone()),
        },
        ExprKind::PairE(a, b) => ExprKind::Pair(eval_expr(a, env)?, eval_expr(b, env)?),
        ExprKind::TripleE(a, b, c) => {
            ExprKind::Triple(eval_expr(a, env)?, eval_expr(b, env)?, eval_expr(c, env)?)
        }
        ExprKind::Interp(segs) => ExprKind::Ident(interp(segs, env)?),
        ExprKind::Bin(..) | ExprKind::Neg(_) | ExprKind::Call(..) | ExprKind::Reduce { .. } => {
            ExprKind::Num(eval_expr(e, env)?)
        }
    };
    Ok(Expr { kind, span: e.span })
}

/// Resolve an argument the builtin expects as a **string** (a `Str` slot: LaTeX
/// for `equation`/`rewrite`, the copy for `text`/`say`/`caption`, `data`/`layout`,
/// …). A real string literal passes straight through; a `(x,y)`/number leaves the
/// generic path (the engine reports the type mismatch). But a bare identifier or
/// expression — `equation(q,(x,y),-i)`, `text(t,(x,y),x^2)` — is unquoted
/// LaTeX/text: the engine can't tell it from arithmetic, so it would otherwise
/// die with a misleading "unknown variable `i` — did you mean `h`?". Catch it here
/// with the real diagnosis and a one-click backtick wrap.
fn resolve_str_arg(e: &Expr, env: &Env, builtin: &str, param: &str) -> Result<Expr, Error> {
    match &e.kind {
        // a genuine string, or a shape the generic path/engine already owns.
        ExprKind::Str(_)
        | ExprKind::Interp(_)
        | ExprKind::Num(_)
        | ExprKind::Pair(..)
        | ExprKind::PairE(..)
        | ExprKind::Triple(..)
        | ExprKind::TripleE(..) => resolve_arg(e, env),
        // A bare identifier or expression in a string slot. Some `Str` slots are
        // polymorphic and also accept a numeric value (`bind`'s from-value
        // `w*0.18`) — so if it EVALUATES, fold it to a number and pass it on. Only
        // when it does NOT evaluate (an unbound `i`, `-i`, `x^2`) is it certainly
        // unquoted LaTeX/text — then give the real diagnosis + a backtick fix.
        _ => match eval_expr(e, env) {
            Ok(n) => Ok(Expr {
                kind: ExprKind::Num(n),
                span: e.span,
            }),
            Err(_) => Err(bare_string_arg_error(e, builtin, param)),
        },
    }
}

/// The "this looks like bare LaTeX/text" error, with a backtick-wrap fix for the
/// shapes we can faithfully reconstruct (a name, or a negated name).
fn bare_string_arg_error(e: &Expr, builtin: &str, param: &str) -> Error {
    let msg = format!(
        "the `{param}` of `{builtin}` must be a STRING — wrap it in backticks (bare LaTeX/text isn't valid manic)"
    );
    match wrap_target(e) {
        Some((text, span)) => Error::new(format!("{msg}: `` `{text}` ``"), span)
            .with_fix(format!("Wrap in backticks: `{text}`"), format!("`{text}`")),
        None => Error::new(msg, e.span),
    }
}

/// Reconstruct the source text + full span of a simple scalar arg, so the fix can
/// wrap it in backticks. Only the shapes we can render 1:1 (a name, a negated
/// name); anything richer gets the message without an auto-fix.
fn wrap_target(e: &Expr) -> Option<(String, Span)> {
    match &e.kind {
        ExprKind::Ident(n) => Some((n.clone(), e.span)),
        // the `Neg` span covers only the `-`; extend it over the inner name so the
        // fix replaces the whole `-i`, not just the sign.
        ExprKind::Neg(inner) => match &inner.kind {
            ExprKind::Ident(n) => {
                let end = inner.span.col + inner.span.len;
                let span = Span::new(e.span.line, e.span.col, end.saturating_sub(e.span.col));
                Some((format!("-{n}"), span))
            }
            _ => None,
        },
        _ => None,
    }
}

fn eval_expr(e: &Expr, env: &Env) -> Result<f32, Error> {
    match &e.kind {
        ExprKind::Num(n) => Ok(*n),
        ExprKind::Ident(name) => match constant(name).or_else(|| env.get(name).copied()) {
            Some(v) => Ok(v),
            None => Err(unknown_var_error(name, env, e.span)),
        },
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
            call_fn(name, x).ok_or_else(|| unknown_fn_error(name, env, e.span))
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
        ExprKind::Pair(..) | ExprKind::PairE(..) | ExprKind::Triple(..) | ExprKind::TripleE(..) => {
            Err(Error::new("a point can't be used in arithmetic", e.span))
        }
        ExprKind::Interp(_) => Err(Error::new("an id can't be used in arithmetic", e.span)),
    }
}

/// Built-in numeric constants, reserved in expression contexts.
fn is_known_var(name: &str, env: &Env) -> bool {
    constant(name).is_some() || env.contains_key(name)
}

/// Build a helpful "unknown variable" error. Catches the common LLM/author slip
/// of running two variables together with no operator (`xvsx` → `xv * sx`, since
/// implicit multiply is number×variable only), else suggests the nearest known
/// name — both as a one-click fix.
fn unknown_var_error(name: &str, env: &Env, span: Span) -> Error {
    let base = format!("unknown variable `{name}`");
    // 1. two known variables concatenated (missing `*`)
    for k in 1..name.len() {
        if !name.is_char_boundary(k) {
            continue;
        }
        let (a, b) = name.split_at(k);
        if is_known_var(a, env) && is_known_var(b, env) {
            let repl = format!("{a} * {b}");
            return Error::new(
                format!("{base} — did you mean `{repl}`? (use `*` between variables)"),
                span,
            )
            .with_fix(format!("Change to `{repl}`"), repl);
        }
    }
    // 2. nearest defined name
    if let Some(sugg) = nearest_var(name, env) {
        return Error::new(format!("{base} — did you mean `{sugg}`?"), span)
            .with_fix(format!("Change to `{sugg}`"), sugg);
    }
    Error::new(base, span)
}

/// Build a helpful "unknown function" error. Catches the two classic glue slips
/// in a `name(args)` call: a variable run into a function (`g3rcos(x)` →
/// `g3r * cos(x)`) and a constant used as a function (`tau(x)` → `tau * (x)`).
/// The fix targets only the NAME (so the `(args)` is preserved).
fn unknown_fn_error(name: &str, env: &Env, span: Span) -> Error {
    // just the name portion of `name(args)` (the call span starts at the name)
    let name_span = Span::new(span.line, span.col, name.chars().count() as u32);
    let is_fn = |s: &str| call_fn(s, 1.0).is_some();
    // 1. `<var><fn>` glued — `g3rcos` = `g3r` (var) + `cos` (fn)
    for k in 1..name.len() {
        if !name.is_char_boundary(k) {
            continue;
        }
        let (a, b) = name.split_at(k);
        if is_known_var(a, env) && is_fn(b) {
            let repl = format!("{a} * {b}");
            return Error::new(
                format!("unknown function `{name}` — did you mean `{repl}(…)`? (use `*` between a variable and a function)"),
                name_span,
            )
            .with_fix(format!("Change to `{repl}`"), repl);
        }
    }
    // 2. a CONSTANT used as a function — `tau(x)` → `tau * (x)`
    if constant(name).is_some() {
        return Error::new(
            format!("`{name}` is a constant, not a function — did you mean `{name} * (…)`? (a number/constant before `(` must have an explicit `*`)"),
            name_span,
        )
        .with_fix(format!("Change to `{name} *`"), format!("{name} * "));
    }
    Error::new(format!("unknown function `{name}`"), span)
}

fn nearest_var(name: &str, env: &Env) -> Option<String> {
    let max = (name.len() / 2).max(1);
    let mut best: Option<(usize, String)> = None;
    for k in env.keys() {
        let d = levenshtein(name, k);
        if d <= max && best.as_ref().map_or(true, |(bd, _)| d < *bd) {
            best = Some((d, k.clone()));
        }
    }
    best.map(|(_, k)| k)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

fn constant(name: &str) -> Option<f32> {
    Some(match name {
        "pi" => std::f32::consts::PI,
        "tau" => std::f32::consts::TAU,
        "e" => std::f32::consts::E,
        // for `limit(id, curve, inf)` — a limit at infinity (horizontal asymptote)
        "inf" | "infinity" => f32::INFINITY,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ExprKind;
    use crate::parser::parse;

    fn num(e: &Expr) -> f32 {
        match &e.kind {
            ExprKind::Num(n) => *n,
            _ => panic!("expected number, got {:?}", e.kind),
        }
    }
    fn ident(e: &Expr) -> String {
        match &e.kind {
            ExprKind::Ident(n) => n.clone(),
            _ => panic!("expected ident, got {:?}", e.kind),
        }
    }

    #[test]
    fn for_loop_expands_and_interpolates() {
        let ex = expand(&parse("for i in 0..3 { dot(d{i}, (i*10, 0), 4); }").unwrap()).unwrap();
        let dots: Vec<_> = ex.stmts.iter().filter(|s| s.name == "dot").collect();
        assert_eq!(dots.len(), 3);
        let ids: Vec<String> = dots.iter().map(|s| ident(&s.args[0])).collect();
        assert_eq!(ids, vec!["d0", "d1", "d2"]);
        // arithmetic in the point is evaluated: d1 -> (10, 0)
        match &dots[1].args[1].kind {
            ExprKind::Pair(x, y) => {
                assert_eq!((*x, *y), (10.0, 0.0));
            }
            other => panic!("expected pair, got {other:?}"),
        }
    }

    #[test]
    fn let_arithmetic_folds() {
        let ex = expand(&parse("let r = 3 * 4 + 1; circle(c, (0,0), r);").unwrap()).unwrap();
        // the `let` is consumed; only `circle` remains, with r folded to 13
        assert!(ex.stmts.iter().all(|s| s.name != "let"));
        let c = ex.stmts.iter().find(|s| s.name == "circle").unwrap();
        assert_eq!(num(&c.args[2]), 13.0);
    }

    #[test]
    fn triple_components_expand() {
        let p = crate::parser::parse("let i = 2; point3(p, (i, i+1, i^2));").unwrap();
        let out = expand(&p).unwrap();
        match out.stmts[0].args[1].kind {
            ExprKind::Triple(x, y, z) => assert_eq!((x, y, z), (2.0, 3.0, 4.0)),
            ref other => panic!("expected triple, got {other:?}"),
        }
    }

    #[test]
    fn def_macro_expands() {
        // macro params are numeric; ids are built by interpolation (`r{k}`)
        let ex = expand(
            &parse("def sq(k, x) { rect(r{k}, (x, 0), 5, 5); }  sq(0, 3);  sq(1, 7);").unwrap(),
        )
        .unwrap();
        let rects: Vec<_> = ex.stmts.iter().filter(|s| s.name == "rect").collect();
        assert_eq!(rects.len(), 2);
        assert_eq!(ident(&rects[0].args[0]), "r0");
        match &rects[1].args[1].kind {
            ExprKind::Pair(x, _) => assert_eq!(*x, 7.0),
            other => panic!("expected pair, got {other:?}"),
        }
    }

    #[test]
    fn runaway_loop_is_bounded() {
        // a range far past MAX_ITERS must error, not hang
        assert!(expand(&parse("for i in 0..99999999 { dot(d{i}, (0,0), 1); }").unwrap()).is_err());
    }

    #[test]
    fn canvas_override_seeds_responsive_values_and_conditionals() {
        let src = "canvas(1280,720);\n\
                   if h > w { dot(layout,(cx,cy),9); }\n\
                   else { dot(layout,(w-10,h-20),4); }";
        let prog = parse(src).unwrap();

        let portrait = expand_with_canvas(&prog, Some((1080, 1920))).unwrap();
        let dot = portrait.stmts.iter().find(|s| s.name == "dot").unwrap();
        match dot.args[1].kind {
            ExprKind::Pair(x, y) => assert_eq!((x, y), (540.0, 960.0)),
            ref other => panic!("expected portrait point, got {other:?}"),
        }
        assert_eq!(num(&dot.args[2]), 9.0);

        let landscape = expand_with_canvas(&prog, Some((1920, 1080))).unwrap();
        let dot = landscape.stmts.iter().find(|s| s.name == "dot").unwrap();
        match dot.args[1].kind {
            ExprKind::Pair(x, y) => assert_eq!((x, y), (1910.0, 1060.0)),
            ref other => panic!("expected landscape point, got {other:?}"),
        }
        assert_eq!(num(&dot.args[2]), 4.0);
    }

    #[test]
    fn bare_latex_in_a_string_slot_suggests_backticks() {
        // the classic slip: LaTeX passed bare into `equation`'s `latex` slot.
        // `-i` doesn't evaluate → real diagnosis + a backtick-wrap fix over `-i`.
        let e = expand(&parse("equation(q, (0,0), -i, 34);").unwrap()).unwrap_err();
        assert!(e.msg.contains("must be a STRING"), "{}", e.msg);
        assert!(e.msg.contains("backticks"), "{}", e.msg);
        let (label, repl) = e.fix.expect("bare name should carry a wrap fix");
        assert_eq!(repl, "`-i`");
        assert!(label.contains("Wrap in backticks"), "{label}");
        // the fix span covers the whole `-i` (col 20), not just the `-`
        assert_eq!((e.span.col, e.span.len), (20, 2));

        // a bare single name is fixable too
        let e = expand(&parse("equation(q, (0,0), i, 34);").unwrap()).unwrap_err();
        assert_eq!(e.fix.unwrap().1, "`i`");
    }

    #[test]
    fn polymorphic_string_slot_still_accepts_a_numeric_expression() {
        // `bind`'s `formula_or_from` is a `Str` slot that ALSO takes a from-value;
        // an expression that evaluates must fold to a number, not get flagged.
        let ex = expand(&parse("canvas(1280,720); bind(a, mover, x, w*0.18, w*0.82);").unwrap())
            .unwrap();
        let bind = ex.stmts.iter().find(|s| s.name == "bind").unwrap();
        assert_eq!(num(&bind.args[3]), 1280.0 * 0.18);
    }

    #[test]
    fn command_line_canvas_specs_cover_social_formats_and_pixels() {
        assert_eq!(canvas_override_dims("portrait").unwrap(), (1080, 1920));
        assert_eq!(canvas_override_dims("4:5").unwrap(), (1080, 1350));
        assert_eq!(canvas_override_dims("square").unwrap(), (1080, 1080));
        assert_eq!(canvas_override_dims("16:9").unwrap(), (1280, 720));
        assert_eq!(canvas_override_dims("1440x2560").unwrap(), (1440, 2560));
        assert!(canvas_override_dims("0x1920").is_err());
        assert!(canvas_override_dims("cinema").is_err());
    }
}
