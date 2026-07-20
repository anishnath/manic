//! The **math kit**: coordinate frames, function plots, vectors, number
//! lines. The first domain built on the manic core.
//!
//! Everything here is a *composition* of core primitives registered as a
//! constructor — `axes` is two arrows, `plot` is a sampled polyline, a
//! `vector` is an arrow. Adding this kit touches no core code, which is the
//! whole point of the registry design. LaTeX typesetting is a later addition;
//! labels are mono text for now.

use macroquad::prelude::{Color, Vec2};

use crate::lang::diag::Error;
use crate::lang::lower::{resolve_color, Args, Registry};
use crate::primitives::{Counter, Entity, FontKind, GraphFn, GraphSrc, GraphView, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;

/// **The bareword named-function family** — the single source of truth. Each
/// entry maps a bareword (and its aliases) to an equivalent formula in `x`, so
/// `plot(f, …, cos, …)` becomes exactly the graph `"cos(x)"` would: it samples
/// the same way and is queryable by `tangent`/`slope`/`deriv` etc. Returns
/// `None` for an unknown name.
///
/// To ADD a variant, add one arm here (any valid formula-string function is
/// allowed — see `expr::func`); everything else (validation, sampling, the
/// stored `GraphFn`) derives from this automatically.
/// The canonical `(bareword, formula)` table. `named_formula`, the error-message
/// list, and the editor's `catalog::NAMED_FNS` vocab all derive from these names;
/// a sync test (`catalog_named_fns_match_engine`) keeps engine and catalog equal.
pub(crate) const NAMED_FORMULAS: &[(&str, &str)] = &[
    ("sin", "sin(x)"),
    ("cos", "cos(x)"),
    ("tan", "tan(x)"),
    // inverse trig (the evaluator already supports asin/acos/atan)
    ("asin", "asin(x)"),
    ("arcsin", "asin(x)"),
    ("acos", "acos(x)"),
    ("arccos", "acos(x)"),
    ("atan", "atan(x)"),
    ("arctan", "atan(x)"),
    ("parabola", "x*x"),
    ("sq", "x*x"),
    ("square", "x*x"),
    ("cubic", "x*x*x"),
    ("cube", "x*x*x"),
    ("line", "x"),
    ("id", "x"),
    ("identity", "x"),
    ("abs", "abs(x)"),
    ("exp", "exp(x)"),
    ("sqrt", "sqrt(x)"),
    ("log", "ln(x)"),
    ("ln", "ln(x)"),
    ("recip", "1/x"),
    ("inv", "1/x"),
    ("gauss", "exp(-x*x)"),
    ("bell", "exp(-x*x)"),
    ("sinc", "sin(x)/x"), // the cardinal sine
    ("sigmoid", "1/(1 + exp(-x))"),
    ("logistic", "1/(1 + exp(-x))"),
    ("relu", "0.5*(x + abs(x))"),
    ("step", "0.5*(1 + sign(x))"),
    ("heaviside", "0.5*(1 + sign(x))"),
];

fn named_formula(name: &str) -> Option<&'static str> {
    NAMED_FORMULAS.iter().find(|(n, _)| *n == name).map(|(_, f)| *f)
}

/// A human-readable list of the bareword names, for error messages.
const NAMED_FN_LIST: &str = "sin, cos, tan, asin, acos, atan, parabola, cubic, \
    line, abs, exp, sqrt, log, recip, gauss, sinc, sigmoid, relu, step";

/// A tiny single-variable expression evaluator, so `plot` can take a formula
/// string like `"cos(t) + 0.5*cos(7*t) + (1/7)*cos(14*t)"` — manic's answer to
/// Manim's `FunctionGraph(lambda t: ...)`. The variable is `x` (alias `t`);
/// constants `pi`, `e`, `tau`; operators `+ - * / ^` (unary `-`); functions
/// sin/cos/tan/asin/acos/atan/sinh/cosh/tanh/exp/ln/log/log10/log2/sqrt/abs/
/// floor/ceil/round/sign. Compiled once to a tree, then sampled per point.
/// This is deliberately NOT the language's (still-deferred) variable/loop
/// layer — it is a leaf evaluator scoped to a single plotted curve.
pub(crate) mod expr {
    #[derive(Debug, Clone)]
    pub enum Node {
        Num(f32),
        Var,
        VarY,
        Neg(Box<Node>),
        Add(Box<Node>, Box<Node>),
        Sub(Box<Node>, Box<Node>),
        Mul(Box<Node>, Box<Node>),
        Div(Box<Node>, Box<Node>),
        Pow(Box<Node>, Box<Node>),
        Call(fn(f32) -> f32, Box<Node>),
    }

    impl Node {
        pub fn eval(&self, x: f32, y: f32) -> f32 {
            match self {
                Node::Num(n) => *n,
                Node::Var => x,
                Node::VarY => y,
                Node::Neg(a) => -a.eval(x, y),
                Node::Add(a, b) => a.eval(x, y) + b.eval(x, y),
                Node::Sub(a, b) => a.eval(x, y) - b.eval(x, y),
                Node::Mul(a, b) => a.eval(x, y) * b.eval(x, y),
                Node::Div(a, b) => a.eval(x, y) / b.eval(x, y),
                Node::Pow(a, b) => a.eval(x, y).powf(b.eval(x, y)),
                Node::Call(f, a) => f(a.eval(x, y)),
            }
        }
    }

    fn func(name: &str) -> Option<fn(f32) -> f32> {
        Some(match name {
            "sin" => f32::sin as fn(f32) -> f32,
            "cos" => f32::cos,
            "tan" => f32::tan,
            "asin" => f32::asin,
            "acos" => f32::acos,
            "atan" => f32::atan,
            "sinh" => f32::sinh,
            "cosh" => f32::cosh,
            "tanh" => f32::tanh,
            "exp" => f32::exp,
            "sqrt" => f32::sqrt,
            "abs" => f32::abs,
            "ln" | "log" => f32::ln,
            "log10" => f32::log10,
            "log2" => f32::log2,
            "floor" => f32::floor,
            "ceil" => f32::ceil,
            "round" => f32::round,
            "sign" => f32::signum,
            _ => return None,
        })
    }

    struct Parser<'a> {
        s: &'a [u8],
        i: usize,
    }

    /// Parse a formula into a tree, or return a human-readable error.
    pub fn compile(src: &str) -> Result<Node, String> {
        let mut p = Parser {
            s: src.as_bytes(),
            i: 0,
        };
        let n = p.expr()?;
        p.ws();
        if p.i != p.s.len() {
            return Err(format!("unexpected `{}`", &src[p.i..]));
        }
        Ok(n)
    }

    impl Parser<'_> {
        fn ws(&mut self) {
            while self.i < self.s.len() && (self.s[self.i] as char).is_whitespace() {
                self.i += 1;
            }
        }
        fn peek(&self) -> Option<u8> {
            self.s.get(self.i).copied()
        }
        fn eat(&mut self, c: u8) -> bool {
            self.ws();
            if self.peek() == Some(c) {
                self.i += 1;
                true
            } else {
                false
            }
        }

        // expr := term (('+'|'-') term)*
        fn expr(&mut self) -> Result<Node, String> {
            let mut lhs = self.term()?;
            loop {
                self.ws();
                match self.peek() {
                    Some(b'+') => {
                        self.i += 1;
                        lhs = Node::Add(Box::new(lhs), Box::new(self.term()?));
                    }
                    Some(b'-') => {
                        self.i += 1;
                        lhs = Node::Sub(Box::new(lhs), Box::new(self.term()?));
                    }
                    _ => break,
                }
            }
            Ok(lhs)
        }

        // term := unary (('*'|'/') unary)*
        fn term(&mut self) -> Result<Node, String> {
            let mut lhs = self.unary()?;
            loop {
                self.ws();
                match self.peek() {
                    Some(b'*') => {
                        self.i += 1;
                        lhs = Node::Mul(Box::new(lhs), Box::new(self.unary()?));
                    }
                    Some(b'/') => {
                        self.i += 1;
                        lhs = Node::Div(Box::new(lhs), Box::new(self.unary()?));
                    }
                    // implicit multiplication before a name or `(`: `2x`, `2pi`,
                    // `3(x+1)` — matches the main DSL. Not before a digit.
                    Some(c) if (c as char).is_ascii_alphabetic() || c == b'_' || c == b'(' => {
                        lhs = Node::Mul(Box::new(lhs), Box::new(self.unary()?));
                    }
                    _ => break,
                }
            }
            Ok(lhs)
        }

        // unary := ('-'|'+') unary | power
        fn unary(&mut self) -> Result<Node, String> {
            self.ws();
            match self.peek() {
                Some(b'-') => {
                    self.i += 1;
                    Ok(Node::Neg(Box::new(self.unary()?)))
                }
                Some(b'+') => {
                    self.i += 1;
                    self.unary()
                }
                _ => self.power(),
            }
        }

        // power := atom ('^' unary)?   (right-associative)
        fn power(&mut self) -> Result<Node, String> {
            let base = self.atom()?;
            self.ws();
            if self.peek() == Some(b'^') {
                self.i += 1;
                Ok(Node::Pow(Box::new(base), Box::new(self.unary()?)))
            } else {
                Ok(base)
            }
        }

        fn atom(&mut self) -> Result<Node, String> {
            self.ws();
            match self.peek() {
                Some(b'(') => {
                    self.i += 1;
                    let n = self.expr()?;
                    if !self.eat(b')') {
                        return Err("expected `)`".into());
                    }
                    Ok(n)
                }
                Some(c) if c.is_ascii_digit() || c == b'.' => self.number(),
                Some(c) if (c as char).is_ascii_alphabetic() || c == b'_' => self.name(),
                Some(c) => Err(format!("unexpected `{}`", c as char)),
                None => Err("unexpected end of expression".into()),
            }
        }

        fn number(&mut self) -> Result<Node, String> {
            let start = self.i;
            let mut dot = false;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.i += 1;
                } else if c == b'.' && !dot {
                    dot = true;
                    self.i += 1;
                } else {
                    break;
                }
            }
            let txt = std::str::from_utf8(&self.s[start..self.i]).unwrap();
            txt.parse::<f32>()
                .map(Node::Num)
                .map_err(|_| format!("bad number `{txt}`"))
        }

        fn name(&mut self) -> Result<Node, String> {
            let start = self.i;
            while let Some(c) = self.peek() {
                if (c as char).is_ascii_alphanumeric() || c == b'_' {
                    self.i += 1;
                } else {
                    break;
                }
            }
            let id = std::str::from_utf8(&self.s[start..self.i])
                .unwrap()
                .to_string();
            self.ws();
            if self.peek() == Some(b'(') {
                self.i += 1;
                let arg = self.expr()?;
                if !self.eat(b')') {
                    return Err(format!("expected `)` after `{id}(`"));
                }
                let f = func(&id).ok_or_else(|| format!("unknown function `{id}`"))?;
                return Ok(Node::Call(f, Box::new(arg)));
            }
            match id.as_str() {
                "x" | "t" | "u" => Ok(Node::Var),
                "y" | "v" | "p" => Ok(Node::VarY),
                "pi" => Ok(Node::Num(std::f32::consts::PI)),
                "e" => Ok(Node::Num(std::f32::consts::E)),
                "tau" => Ok(Node::Num(std::f32::consts::TAU)),
                _ => {
                    // `pit` / `vv` / `piu` — adjacent names glued without `*`.
                    // If the name splits cleanly into known ones, suggest it.
                    fn split_glue(id: &str) -> Option<String> {
                        let names = ["tau", "pi", "x", "y", "t", "u", "v", "p", "e"];
                        let mut rest = id;
                        let mut parts = Vec::new();
                        while !rest.is_empty() {
                            let m = names.iter().copied().find(|n| rest.starts_with(*n))?;
                            parts.push(m);
                            rest = &rest[m.len()..];
                        }
                        (parts.len() >= 2).then(|| parts.join("*"))
                    }
                    Err(match split_glue(&id) {
                        Some(s) => format!(
                            "unknown name `{id}` — did you mean `{s}`? put `*` between names"
                        ),
                        None => format!(
                            "unknown name `{id}` (use x/y, t, u/v, p, pi, e, tau, or a function)"
                        ),
                    })
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::compile;

        #[test]
        fn glued_names_suggest_a_star() {
            // `pit` = pi*t, `vv` = v*v — adjacent names need `*` between them.
            let e = compile("0.1*sin(pit)").err().unwrap();
            assert!(e.contains("pi*t"), "got: {e}");
            assert!(compile("v*v - vv").err().unwrap().contains("v*v"));
            // a genuinely unknown name gets no bogus suggestion
            let g = compile("foo(x)").err().unwrap();
            assert!(g.contains("unknown function"), "got: {g}");
        }

        #[test]
        fn implicit_multiplication_still_parses() {
            // `2x`, `pi*t`, `sin(x)y`, `3(x+1)` are all valid
            assert!(compile("2x + pi*t + sin(x)y + 3(x+1)").is_ok());
        }
    }
}

fn stroked(color: macroquad::prelude::Color, width: f32) -> StrokeStyle {
    StrokeStyle {
        fill: false,
        outline: true,
        width,
        outline_color: Some(color),
    }
}

fn fmt_num(v: f32) -> String {
    if (v.fract()).abs() < 1e-4 {
        format!("{}", v as i64)
    } else {
        format!("{v:.1}")
    }
}

// ---- builtins -------------------------------------------------------------

fn add_line(
    s: &mut Scene,
    id: String,
    from: Vec2,
    to: Vec2,
    color: Color,
    width: f32,
    opacity: f32,
    z: i32,
    tags: Vec<String>,
) {
    let mut e = Entity::new(id, Shape::Line { to }, from, color);
    e.stroke.width = width;
    e.opacity = opacity;
    e.glow = if opacity < 0.9 { 0.0 } else { 1.0 };
    e.z = z;
    e.tags = tags;
    s.add(e);
}

/// Tick marks + integer labels every `unit` px along both axes.
fn add_ticks(s: &mut Scene, id: &str, c: Vec2, hw: f32, hh: f32, unit: f32) {
    let ticks = format!("{id}.ticks");
    let labels = format!("{id}.labels");
    let kx = (hw / unit).floor() as i32;
    for k in -kx..=kx {
        if k == 0 {
            continue;
        }
        let x = c.x + k as f32 * unit;
        add_line(
            s,
            format!("{id}.xt{k}"),
            Vec2::new(x, c.y - 6.0),
            Vec2::new(x, c.y + 6.0),
            style::DIM,
            2.0,
            1.0,
            1,
            vec![id.to_string(), ticks.clone()],
        );
        let mut lbl = Entity::new(
            format!("{id}.xl{k}"),
            Shape::Text {
                content: k.to_string(),
                size: 15.0,
            },
            Vec2::new(x, c.y + 24.0),
            style::DIM,
        );
        lbl.z = 1;
        lbl.tags = vec![id.to_string(), labels.clone()];
        s.add(lbl);
    }
    let ky = (hh / unit).floor() as i32;
    for k in -ky..=ky {
        if k == 0 {
            continue;
        }
        let y = c.y - k as f32 * unit; // +k up the screen
        add_line(
            s,
            format!("{id}.yt{k}"),
            Vec2::new(c.x - 6.0, y),
            Vec2::new(c.x + 6.0, y),
            style::DIM,
            2.0,
            1.0,
            1,
            vec![id.to_string(), ticks.clone()],
        );
        let mut lbl = Entity::new(
            format!("{id}.yl{k}"),
            Shape::Text {
                content: k.to_string(),
                size: 15.0,
            },
            Vec2::new(c.x - 26.0, y),
            style::DIM,
        );
        lbl.z = 1;
        lbl.tags = vec![id.to_string(), labels.clone()];
        s.add(lbl);
    }
}

/// `axes(id, (cx,cy), halfw, halfh, [unit])` — a cyan-dim coordinate cross with
/// arrowheads on the +x and +y ends. With `unit` (px per step) it also gets
/// tick marks + integer labels. Children `{id}.x`, `{id}.y`; tags `id`,
/// `{id}.ticks`, `{id}.labels`.
fn c_axes(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let hw = a.num(2)?;
    let hh = a.num(3)?;

    let mut x = Entity::new(
        format!("{id}.x"),
        Shape::Arrow {
            to: Vec2::new(c.x + hw, c.y),
        },
        Vec2::new(c.x - hw, c.y),
        style::DIM,
    );
    x.stroke.width = 2.0;
    x.tags.push(id.clone());
    s.add(x);

    // y grows up the screen (smaller y), so the head is at the top
    let mut y = Entity::new(
        format!("{id}.y"),
        Shape::Arrow {
            to: Vec2::new(c.x, c.y - hh),
        },
        Vec2::new(c.x, c.y + hh),
        style::DIM,
    );
    y.stroke.width = 2.0;
    y.tags.push(id.clone());
    s.add(y);

    if let Some(unit) = a.opt_num(4)? {
        if unit > 1.0 {
            add_ticks(s, &id, c, hw, hh, unit);
        }
    }
    Ok(())
}

/// `plane(id, (cx,cy), halfw, halfh, [unit])` — a NumberPlane: a faint grid
/// every `unit` px (default 50) with brighter axes through the centre. Grid
/// tagged `{id}.grid`; axes `{id}.x` / `{id}.y`.
fn c_plane(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let hw = a.num(2)?;
    let hh = a.num(3)?;
    let unit = a.opt_num(4)?.unwrap_or(50.0).max(4.0);
    let grid = format!("{id}.grid");

    let kx = (hw / unit).floor() as i32;
    for k in -kx..=kx {
        if k == 0 {
            continue;
        }
        let x = c.x + k as f32 * unit;
        add_line(
            s,
            format!("{id}.gv{k}"),
            Vec2::new(x, c.y - hh),
            Vec2::new(x, c.y + hh),
            style::DIM,
            1.0,
            0.35,
            0,
            vec![id.clone(), grid.clone()],
        );
    }
    let ky = (hh / unit).floor() as i32;
    for k in -ky..=ky {
        if k == 0 {
            continue;
        }
        let y = c.y + k as f32 * unit;
        add_line(
            s,
            format!("{id}.gh{k}"),
            Vec2::new(c.x - hw, y),
            Vec2::new(c.x + hw, y),
            style::DIM,
            1.0,
            0.35,
            0,
            vec![id.clone(), grid.clone()],
        );
    }
    let mut x = Entity::new(
        format!("{id}.x"),
        Shape::Arrow {
            to: Vec2::new(c.x + hw, c.y),
        },
        Vec2::new(c.x - hw, c.y),
        style::FG,
    );
    x.stroke.width = 2.0;
    x.z = 1;
    x.tags.push(id.clone());
    s.add(x);
    let mut y = Entity::new(
        format!("{id}.y"),
        Shape::Arrow {
            to: Vec2::new(c.x, c.y - hh),
        },
        Vec2::new(c.x, c.y + hh),
        style::FG,
    );
    y.stroke.width = 2.0;
    y.z = 1;
    y.tags.push(id);
    s.add(y);
    Ok(())
}

/// `complexplane(id, (cx,cy), halfw, halfh, [unit])` — a NumberPlane labelled
/// with `Re` and `Im` axes.
fn c_complexplane(s: &mut Scene, a: &Args) -> Result<(), Error> {
    c_plane(s, a)?;
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let hw = a.num(2)?;
    let hh = a.num(3)?;
    let mut re = Entity::new(
        format!("{id}.re"),
        Shape::Text {
            content: "Re".into(),
            size: 20.0,
        },
        Vec2::new(c.x + hw - 16.0, c.y - 20.0),
        style::CYAN,
    );
    re.z = 2;
    re.tags.push(id.clone());
    s.add(re);
    let mut im = Entity::new(
        format!("{id}.im"),
        Shape::Text {
            content: "Im".into(),
            size: 20.0,
        },
        Vec2::new(c.x + 22.0, c.y - hh + 14.0),
        style::CYAN,
    );
    im.z = 2;
    im.tags.push(id);
    s.add(im);
    Ok(())
}

/// `polarplane(id, (cx,cy), radius, [rings], [spokes])` — a PolarPlane: faint
/// concentric rings and radial spokes. Tagged `{id}.grid`.
fn c_polarplane(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let radius = a.num(2)?;
    let rings = a.opt_num(3)?.unwrap_or(4.0).max(1.0) as i32;
    let spokes = a.opt_num(4)?.unwrap_or(12.0).max(2.0) as i32;
    let grid = format!("{id}.grid");

    for i in 1..=rings {
        let r = radius * i as f32 / rings as f32;
        let mut e = Entity::new(format!("{id}.ring{i}"), Shape::Circle { r }, c, style::DIM);
        e.stroke = StrokeStyle {
            fill: false,
            outline: true,
            width: 1.0,
            outline_color: Some(style::DIM),
        };
        e.opacity = 0.4;
        e.glow = 0.0;
        e.tags = vec![id.clone(), grid.clone()];
        s.add(e);
    }
    for j in 0..spokes {
        let ang = std::f32::consts::TAU * j as f32 / spokes as f32;
        let to = Vec2::new(c.x + ang.cos() * radius, c.y + ang.sin() * radius);
        add_line(
            s,
            format!("{id}.spoke{j}"),
            c,
            to,
            style::DIM,
            1.0,
            0.4,
            0,
            vec![id.clone(), grid.clone()],
        );
    }
    Ok(())
}

/// `plot(id, (cx,cy), sx, sy, fn, [domain])` — sample a function over
/// `x ∈ [-domain, domain]` (default 6) and draw it as a glowing polyline in
/// screen space: `(cx + x*sx, cy - f(x)*sy)`. `fn` is either a **named**
/// function (`sin`, `cos`, `parabola`, …) or a **formula string** in the
/// variable `x`/`t` — e.g. `plot(f,(cx,cy),40,40,"cos(x)+0.5*cos(7*x)",7)`,
/// manic's `FunctionGraph(lambda t: …)`. The range arg may also be an
/// **asymmetric pair** `(x0, x1)` — e.g. `plot(g,(cx,cy),200,52,"x*x",(0,2.5))`.
fn c_plot(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let sx = a.num(2)?;
    let sy = a.num(3)?;
    // range: either a scalar `domain` -> [-d, d], or an explicit `(x0, x1)`
    let (x0, x1) = if let Ok(p) = a.pair(5) {
        (p.x, p.y)
    } else {
        let d = a.opt_num(5)?.unwrap_or(6.0);
        (-d, d)
    };

    // arg 4 is either a "formula string" or a bareword named function — both
    // compile to one expression tree, so sampling and the stored `GraphFn` share
    // a single representation (the author never retypes the formula).
    let node = if let Ok(src) = a.text(4) {
        expr::compile(&src)
            .map_err(|m| Error::new(format!("in plot formula: {m}"), a.span_of(4)))?
    } else {
        let name = a.ident(4)?;
        let formula = named_formula(&name).ok_or_else(|| {
            Error::new(
                format!(
                    "unknown function `{name}` — use a named one ({NAMED_FN_LIST}) or a \"formula\" like \"cos(x)+0.5*sin(3*x)\""
                ),
                a.span_of(4),
            )
        })?;
        expr::compile(formula).expect("named-function formula always compiles")
    };

    const N: usize = 600;
    let mut pts = Vec::with_capacity(N + 1);
    for i in 0..=N {
        let x = x0 + (x1 - x0) * i as f32 / N as f32;
        let y = node.eval(x, 0.0);
        // non-finite (asymptotes, sqrt/log out of range) breaks the polyline
        if y.is_finite() {
            pts.push(Vec2::new(c.x + x * sx, c.y - y * sy));
        }
    }
    let mut e = Entity::new(id.clone(), Shape::Polyline { pts }, Vec2::ZERO, style::CYAN);
    e.stroke = stroked(style::CYAN, 3.0);
    e.graph = Some(GraphFn {
        src: GraphSrc::Expr(node),
        center: c,
        sx,
        sy,
        x0,
        x1,
    });
    e.tags.push(id);
    s.add(e);
    Ok(())
}

/// `tangent(id, graph, x, [len])` — the tangent line to a plotted function
/// (`graph`, a `plot` id) at `x` in the graph's own units, with a contact dot.
/// The slope is measured from the function itself (numerically), so it's correct
/// as the touch point slides: `x` is animatable via `to(id, x, target, dur)`.
/// `len` is the on-screen segment length in px (default 120). At a corner or
/// asymptote the slope is undefined and only the dot is drawn — never a fake
/// line.
/// Fetch a plotted function's remembered graph (`plot` stored it on the entity),
/// or a clear error naming the caller. Lets the analysis family query a curve by
/// id — the author never retypes the formula.
fn fetch_graph(
    s: &Scene,
    a: &Args,
    idx: usize,
    who: &str,
) -> Result<crate::primitives::GraphFn, Error> {
    let src = a.ident(idx)?;
    s.get(&src).and_then(|e| e.graph.clone()).ok_or_else(|| {
        Error::new(
            format!(
                "`{who}` needs a plotted function as argument {}, but `{src}` isn't a `plot` (draw the curve with `plot` first)",
                idx + 1
            ),
            a.span_of(idx),
        )
    })
}

/// Shared body for `tangent`/`normal`: a line grazing (or perpendicular to) the
/// curve at `x`, with a contact dot. `x` is animatable via `to(id, x, …)`.
fn line_view(
    s: &mut Scene,
    a: &Args,
    who: &str,
    normal: bool,
    color: Color,
) -> Result<(), Error> {
    let id = a.ident(0)?;
    let source = a.ident(1)?;
    let graph = fetch_graph(s, a, 1, who)?;
    let x = a.num(2)?;
    let half = a.opt_num(3)?.unwrap_or(120.0) * 0.5;
    let gv = if normal {
        GraphView::Normal { graph, x, half }
    } else {
        GraphView::Tangent { graph, x, half }
    };
    let (tail, head) = gv.segment().unwrap();
    let pos = if tail.x.is_finite() && tail.y.is_finite() {
        tail
    } else {
        gv.touch()
    };
    let mut e = Entity::new(id.clone(), Shape::Line { to: head }, pos, color);
    e.stroke.width = 3.0;
    e.graph_view = Some(gv);
    e.graph_source = Some(source);
    e.tags.push(id);
    s.add(e);
    Ok(())
}

/// `tangent(id, curve, x, [len])` — the tangent line + contact dot to a plotted
/// function at `x` (its own units). Slope read from the function itself, so it
/// stays true as `x` slides (`to(id, x, target, dur)`); at a corner/asymptote
/// only the dot draws. (Overloaded — see `geo::c_tangent` — with three name
/// args it's the circle construction.)
pub(crate) fn c_graph_tangent(s: &mut Scene, a: &Args) -> Result<(), Error> {
    line_view(s, a, "tangent", false, style::GOLD)
}

/// `normal(id, curve, x, [len])` — the normal (perpendicular) line + contact dot
/// to a plotted function at `x`. Animatable exactly like `tangent`.
fn c_normal(s: &mut Scene, a: &Args) -> Result<(), Error> {
    line_view(s, a, "normal", true, style::MAGENTA)
}

/// `slope(id, curve, x, [(dx,dy)])` — a live number showing the slope of a
/// plotted function at `x`, riding just off the point (offset `(dx,dy)` px).
/// Animate `to(id, x, target, dur)` and the readout climbs/falls with the curve.
fn c_slope(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let source = a.ident(1)?;
    let graph = fetch_graph(s, a, 1, "slope")?;
    let x = a.num(2)?;
    let off = a.pair(3).unwrap_or(Vec2::new(16.0, -20.0));
    let gv = GraphView::Slope { graph, x, off };
    let counter = Counter {
        value: gv.value(),
        decimals: 2,
        prefix: String::new(),
        suffix: String::new(),
    };
    let content = counter.render();
    let mut e = Entity::new(
        id.clone(),
        Shape::Text {
            content,
            size: 24.0,
        },
        gv.readout_pos(),
        style::GOLD,
    );
    e.font = FontKind::MonoBold;
    e.counter = Some(counter);
    e.graph_view = Some(gv);
    e.graph_source = Some(source);
    e.tags.push(id);
    s.add(e);
    Ok(())
}

/// `integral(id, curve, a, b, [(px,py)])` — a live readout of the definite
/// integral of a plotted function from `a` to `b`, pinned at screen `(px,py)`
/// (defaults above the curve). Animate `to(id, x, b, dur)` — in step with an
/// `area` sweep — and the number climbs to the true integral.
fn c_integral(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let source = a.ident(1)?;
    let graph = fetch_graph(s, a, 1, "integral")?;
    let av = a.num(2)?;
    let bv = a.num(3)?;
    let at = a
        .pair(4)
        .unwrap_or(Vec2::new(graph.center.x, graph.center.y - 170.0));
    let gv = GraphView::Integral {
        graph,
        a: av,
        x: bv,
        n: 80,
        at,
    };
    let counter = Counter {
        value: gv.value(),
        decimals: 3,
        prefix: String::new(),
        suffix: String::new(),
    };
    let content = counter.render();
    let mut e = Entity::new(
        id.clone(),
        Shape::Text {
            content,
            size: 26.0,
        },
        at,
        style::LIME,
    );
    e.font = FontKind::MonoBold;
    e.counter = Some(counter);
    e.graph_view = Some(gv);
    e.graph_source = Some(source);
    e.tags.push(id);
    s.add(e);
    Ok(())
}

/// `area(id, curve, a, b, [n])` — the filled region under a plotted function
/// from `a` to `b` (its own units), sampled with `n` strips (default 60).
/// Translucent so the curve reads through. To sweep it open dramatically, start
/// collapsed (`area(r, f, 1, 1)`) and animate the right bound: `to(r, x, 4, 3)`.
fn c_area(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let source = a.ident(1)?;
    let graph = fetch_graph(s, a, 1, "area")?;
    let av = a.num(2)?;
    let bv = a.num(3)?;
    let n = a.opt_num(4)?.map(|v| v as u32).unwrap_or(60);
    let gv = GraphView::Area {
        graph,
        a: av,
        x: bv,
        n,
    };
    let (tris, rings) = gv.region();
    let mut e = Entity::new(id.clone(), Shape::Region { tris, rings }, Vec2::ZERO, style::CYAN);
    e.opacity = 0.30;
    e.stroke.fill = true;
    e.graph_view = Some(gv);
    e.graph_source = Some(source);
    e.tags.push(id);
    s.add(e);
    Ok(())
}

/// Build a numerically-sampled curve entity (`deriv`/`accum`), mapped through
/// the source graph `g` so it overlays the plot, and carrying its own `GraphFn`
/// so it's first-class (tangent/slope/area work on it too).
fn add_sample_curve(
    s: &mut Scene,
    id: String,
    g: &GraphFn,
    xs: Vec<f32>,
    ys: Vec<f32>,
    color: Color,
) {
    let pts: Vec<Vec2> = xs
        .iter()
        .zip(&ys)
        .map(|(&x, &y)| Vec2::new(g.center.x + x * g.sx, g.center.y - y * g.sy))
        .collect();
    let mut e = Entity::new(id.clone(), Shape::Polyline { pts }, Vec2::ZERO, color);
    e.stroke.width = 3.0;
    e.graph = Some(GraphFn {
        src: GraphSrc::Samples { xs, ys },
        center: g.center,
        sx: g.sx,
        sy: g.sy,
        x0: g.x0,
        x1: g.x1,
    });
    e.tags.push(id);
    s.add(e);
}

/// `deriv(id, curve, [color])` — the derivative `f'` of a plotted function,
/// measured numerically across its domain and drawn as its own curve on the
/// same axes. It's a first-class graph, so `tangent`/`area` work on it too.
fn c_deriv(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let g = fetch_graph(s, a, 1, "deriv")?;
    let color = if a.len() > 2 {
        resolve_color(&a.ident(2)?, a.span_of(2))?
    } else {
        style::GOLD
    };
    const N: usize = 240;
    let mut xs = Vec::with_capacity(N + 1);
    let mut ys = Vec::with_capacity(N + 1);
    for i in 0..=N {
        let x = g.x0 + (g.x1 - g.x0) * i as f32 / N as f32;
        xs.push(x);
        ys.push(g.slope(x));
    }
    add_sample_curve(s, id, &g, xs, ys, color);
    Ok(())
}

/// `accum(id, curve, [a], [color])` — the accumulation function
/// `F(x) = ∫ₐˣ f dt` (default `a` = the curve's left edge), drawn as its own
/// curve. By the Fundamental Theorem its slope is `f`, so a `tangent` on it
/// reads back `f(x)` — plot `f` and `accum(f)` together to *show* the theorem.
fn c_accum(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let g = fetch_graph(s, a, 1, "accum")?;
    let a0 = a.opt_num(2)?.unwrap_or(g.x0);
    let color = if a.len() > 3 {
        resolve_color(&a.ident(3)?, a.span_of(3))?
    } else {
        style::LIME
    };
    const N: usize = 240;
    let mut xs = Vec::with_capacity(N + 1);
    let mut ys = Vec::with_capacity(N + 1);
    for i in 0..=N {
        let x = g.x0 + (g.x1 - g.x0) * i as f32 / N as f32;
        xs.push(x);
        ys.push(crate::primitives::integrate(&g, a0, x, 48));
    }
    add_sample_curve(s, id, &g, xs, ys, color);
    Ok(())
}

/// Place a dot at `graph.point(x)` for each x — the shared body of
/// `roots`/`extrema`/`inflections` (children `{id}0…`, tagged `id`).
fn mark_points(s: &mut Scene, id: &str, graph: &GraphFn, xs: &[f32], color: Color) {
    for (k, &x) in xs.iter().enumerate() {
        let p = graph.point(x);
        if !(p.x.is_finite() && p.y.is_finite()) {
            continue;
        }
        let mut e = Entity::new(format!("{id}{k}"), Shape::Circle { r: 6.0 }, p, color);
        e.stroke.fill = true;
        e.stroke.outline = false;
        e.tags.push(id.to_string());
        s.add(e);
    }
}

/// The x-values where `f(x)` (sampled across `g`'s domain) crosses zero — a
/// sampled `GraphFn` reused through `roots()`. Powers `extrema` (zeros of `f'`)
/// and `inflections` (zeros of `f''`).
fn sampled_zeros(g: &GraphFn, f: impl Fn(f32) -> f32) -> Vec<f32> {
    const N: usize = 400;
    let (mut xs, mut ys) = (Vec::with_capacity(N + 1), Vec::with_capacity(N + 1));
    for i in 0..=N {
        let x = g.x0 + (g.x1 - g.x0) * i as f32 / N as f32;
        xs.push(x);
        ys.push(f(x));
    }
    GraphFn {
        src: GraphSrc::Samples { xs, ys },
        center: g.center,
        sx: g.sx,
        sy: g.sy,
        x0: g.x0,
        x1: g.x1,
    }
    .roots()
}

/// `taylor(id, curve, a, n, [color])` — the degree-`n` Taylor polynomial of a
/// plotted function about `x = a`, drawn as its own curve on the same axes.
/// Reveal `taylor(…,1)`, `taylor(…,3)`, `taylor(…,5)` in turn to watch the
/// polynomial hug the curve over a widening interval. (Coefficients come from
/// numerical derivatives, so keep `n` modest — high orders get noisy.)
fn c_taylor(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let g = fetch_graph(s, a, 1, "taylor")?;
    let ctr = a.num(2)?;
    let deg = (a.num(3)?.round() as i32).clamp(0, 12) as u32;
    let color = if a.len() > 4 {
        resolve_color(&a.ident(4)?, a.span_of(4))?
    } else {
        style::GOLD
    };
    // Taylor coefficients c_k = f⁽ᵏ⁾(a) / k!
    let mut coef = Vec::with_capacity(deg as usize + 1);
    let mut fact = 1.0f32;
    for k in 0..=deg {
        if k > 0 {
            fact *= k as f32;
        }
        coef.push(g.nth_deriv(ctr, k) / fact);
    }
    const N: usize = 240;
    let (mut xs, mut ys) = (Vec::with_capacity(N + 1), Vec::with_capacity(N + 1));
    for i in 0..=N {
        let x = g.x0 + (g.x1 - g.x0) * i as f32 / N as f32;
        let dx = x - ctr;
        let (mut p, mut pw) = (0.0f32, 1.0f32);
        for &c in &coef {
            p += c * pw;
            pw *= dx;
        }
        xs.push(x);
        ys.push(p);
    }
    add_sample_curve(s, id, &g, xs, ys, color);
    Ok(())
}

/// `limit(id, curve, a, [color])` — visualise `lim(x→a) f(x)`: an open circle at
/// the value `L` the curve approaches (found numerically from both sides),
/// dashed guides to the axes, the value, and a dot that rides the curve — slide
/// it in with `to(id, x, a, dur)`. Works even where `f(a)` is undefined (a
/// removable hole, e.g. `sin(x)/x` at 0).
fn c_limit(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let source = a.ident(1)?;
    let g = fetch_graph(s, a, 1, "limit")?;
    let at = a.num(2)?;
    let color = if a.len() > 3 {
        resolve_color(&a.ident(3)?, a.span_of(3))?
    } else {
        style::GOLD
    };
    // limit at infinity (`limit(id, curve, inf)` / `-inf`): the horizontal
    // asymptote y = L, found by sampling far out along the curve
    if at.is_infinite() {
        let sign = at.signum();
        let mut l = f32::NAN;
        for &m in &[1e3f32, 1e4, 1e5, 1e6] {
            let v = g.y(sign * m);
            if v.is_finite() {
                l = v;
            }
        }
        if !l.is_finite() {
            return Err(Error::new(
                "`limit`: no finite horizontal asymptote as x → ±inf",
                a.span_of(2),
            ));
        }
        let ys = g.center.y - l * g.sy;
        let (xl, xr) = (g.center.x + g.x0 * g.sx, g.center.x + g.x1 * g.sx);
        let mut lbl = Entity::new(
            format!("{id}.val"),
            Shape::Text {
                content: format!("y = {l:.2}"),
                size: 22.0,
            },
            Vec2::new(xr - 40.0, ys - 26.0),
            color,
        );
        lbl.font = FontKind::MonoBold;
        lbl.tags.push(id.clone());
        s.add(lbl);
        let mut line = Entity::new(
            id.clone(),
            Shape::Line { to: Vec2::new(xr, ys) },
            Vec2::new(xl, ys),
            color,
        );
        line.stroke.width = 2.5;
        line.tags.push(id);
        s.add(line);
        return Ok(());
    }
    let span = (g.x1 - g.x0).abs();
    let eps = (span * 1e-3).max(1e-4);
    let (yl, yr) = (g.y(at - eps), g.y(at + eps));
    let l = match (yl.is_finite(), yr.is_finite()) {
        (true, true) => 0.5 * (yl + yr),
        (true, false) => yl,
        (false, true) => yr,
        _ => {
            return Err(Error::new(
                format!("`limit`: f doesn't approach a finite value at x = {at}"),
                a.span_of(2),
            ))
        }
    };
    let target = Vec2::new(g.center.x + at * g.sx, g.center.y - l * g.sy);
    // dashed-ish guide lines (dim) down to the x-axis and across to the y-axis
    let mut gx = Entity::new(
        format!("{id}.gx"),
        Shape::Line { to: Vec2::new(target.x, g.center.y) },
        target,
        style::DIM,
    );
    gx.stroke.width = 1.5;
    gx.tags.push(id.clone());
    s.add(gx);
    let mut gy = Entity::new(
        format!("{id}.gy"),
        Shape::Line { to: Vec2::new(g.center.x, target.y) },
        target,
        style::DIM,
    );
    gy.stroke.width = 1.5;
    gy.tags.push(id.clone());
    s.add(gy);
    // the value L, just above-right of the target
    let mut lbl = Entity::new(
        format!("{id}.val"),
        Shape::Text {
            content: format!("{l:.2}"),
            size: 22.0,
        },
        Vec2::new(target.x + 34.0, target.y - 22.0),
        color,
    );
    lbl.font = FontKind::MonoBold;
    lbl.tags.push(id.clone());
    s.add(lbl);
    // the open circle at (a, L) — the value approached
    let mut mark = Entity::new(format!("{id}.mark"), Shape::Circle { r: 10.0 }, target, color);
    mark.stroke.fill = false;
    mark.stroke.outline = true;
    mark.stroke.width = 3.0;
    mark.tags.push(id.clone());
    s.add(mark);
    // the approaching dot (main entity): rides the curve, slide it via to(id,x,a)
    let start = if at - span * 0.3 >= g.x0 {
        at - span * 0.3
    } else {
        at + span * 0.3
    };
    let mut dot = Entity::new(id.clone(), Shape::Circle { r: 7.0 }, g.point(start), color);
    dot.stroke.fill = true;
    dot.graph_view = Some(GraphView::Mark {
        graph: g,
        x: start,
    });
    dot.graph_source = Some(source);
    dot.tags.push(id);
    s.add(dot);
    Ok(())
}

/// `roots(id, curve, [color])` — mark every place a plotted function crosses
/// zero with a dot (`{id}0`, `{id}1`, … tagged `id`). Found by scanning the
/// curve's domain for sign changes and refining each by bisection.
fn c_roots(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let graph = fetch_graph(s, a, 1, "roots")?;
    let color = if a.len() > 2 {
        resolve_color(&a.ident(2)?, a.span_of(2))?
    } else {
        style::LIME
    };
    mark_points(s, &id, &graph, &graph.roots(), color);
    Ok(())
}

/// `extrema(id, curve, [color])` — dots at the maxima and minima of a plotted
/// function (the critical points, where the slope is zero). Just the roots of
/// `f'`.
fn c_extrema(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let g = fetch_graph(s, a, 1, "extrema")?;
    let color = if a.len() > 2 {
        resolve_color(&a.ident(2)?, a.span_of(2))?
    } else {
        style::GOLD
    };
    let xs = sampled_zeros(&g, |x| g.slope(x));
    mark_points(s, &id, &g, &xs, color);
    Ok(())
}

/// `inflections(id, curve, [color])` — dots where a plotted function changes
/// concavity (the inflection points, where `f''` is zero). Roots of `f''`.
fn c_inflections(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let g = fetch_graph(s, a, 1, "inflections")?;
    let color = if a.len() > 2 {
        resolve_color(&a.ident(2)?, a.span_of(2))?
    } else {
        style::MAGENTA
    };
    let xs = sampled_zeros(&g, |x| g.second(x));
    mark_points(s, &id, &g, &xs, color);
    Ok(())
}

/// `band(id, top, bottom, [color])` — the filled (translucent) region between
/// two plotted curves over the x-range they share. Each curve is drawn where its
/// own mapping puts it, so the band hugs both.
fn c_band(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let top = fetch_graph(s, a, 1, "band")?;
    let bot = fetch_graph(s, a, 2, "band")?;
    let color = if a.len() > 3 {
        resolve_color(&a.ident(3)?, a.span_of(3))?
    } else {
        style::CYAN
    };
    let (x0, x1) = (top.x0.max(bot.x0), top.x1.min(bot.x1));
    if x1 <= x0 {
        return Err(Error::new(
            "`band` needs two curves that overlap in x",
            a.name_span,
        ));
    }
    const N: usize = 160;
    let mut up = Vec::with_capacity(N + 1);
    let mut dn = Vec::with_capacity(N + 1);
    for i in 0..=N {
        let x = x0 + (x1 - x0) * i as f32 / N as f32;
        let (pt, pb) = (top.point(x), bot.point(x));
        if pt.x.is_finite() && pt.y.is_finite() && pb.y.is_finite() {
            up.push(pt);
            dn.push(pb);
        }
    }
    if up.len() < 2 {
        return Err(Error::new("`band` produced no fillable region", a.name_span));
    }
    let mut tris = Vec::with_capacity((up.len() - 1) * 2);
    for i in 0..up.len() - 1 {
        tris.push([up[i], up[i + 1], dn[i + 1]]);
        tris.push([up[i], dn[i + 1], dn[i]]);
    }
    let mut ring = up.clone();
    for p in dn.iter().rev() {
        ring.push(*p);
    }
    let mut e = Entity::new(
        id.clone(),
        Shape::Region { tris, rings: vec![ring] },
        Vec2::ZERO,
        color,
    );
    e.opacity = 0.28;
    e.stroke.fill = true;
    e.tags.push(id);
    s.add(e);
    Ok(())
}

/// `newton(id, curve, x0, [steps])` — Newton's method starting from guess `x0`,
/// drawn as the classic zig-zag: from the curve, down each tangent to the x-axis
/// (the next guess), back up to the curve, converging on a root. Declare
/// `untraced(id)` and `draw(id, dur)` to watch the guesses walk to the root.
fn c_newton(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let graph = fetch_graph(s, a, 1, "newton")?;
    let x0 = a.num(2)?;
    let steps = a.opt_num(3)?.map(|v| v as u32).unwrap_or(6);
    let pts = graph.newton_path(x0, steps);
    let mut e = Entity::new(id.clone(), Shape::Polyline { pts }, Vec2::ZERO, style::GOLD);
    e.stroke.width = 2.5;
    e.tags.push(id);
    s.add(e);
    Ok(())
}

/// One Catmull-Rom point on the segment `p1→p2` (with neighbours `p0`,`p3`) at
/// parameter `t ∈ [0,1]` — the standard tension-½ interpolant.
fn catmull_point(p0: Vec2, p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> Vec2 {
    let (t2, t3) = (t * t, t * t * t);
    0.5 * (2.0 * p1
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// Sample a Catmull-Rom spline through `pts` (it passes through every point),
/// `seg` samples per segment. Endpoint neighbours are reflected so the ends
/// aren't clamped flat.
fn catmull_rom(pts: &[Vec2], seg: usize) -> Vec<Vec2> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let mut out = Vec::with_capacity((n - 1) * seg + 1);
    for i in 0..n - 1 {
        let p0 = if i == 0 { 2.0 * pts[0] - pts[1] } else { pts[i - 1] };
        let p1 = pts[i];
        let p2 = pts[i + 1];
        let p3 = if i + 2 < n {
            pts[i + 2]
        } else {
            2.0 * pts[n - 1] - pts[n - 2]
        };
        for s in 0..seg {
            out.push(catmull_point(p0, p1, p2, p3, s as f32 / seg as f32));
        }
    }
    out.push(pts[n - 1]);
    out
}

/// Integrate the autonomous system `dx/dt = fx(x,y)`, `dy/dt = fy(x,y)` from
/// `(x0,y0)` with classic RK4 (`steps` steps of size `dt`), returning the path
/// in math coords. Stops early on a non-finite state.
fn rk4_path(
    fx: &expr::Node,
    fy: &expr::Node,
    x0: f32,
    y0: f32,
    dt: f32,
    steps: u32,
) -> Vec<(f32, f32)> {
    // The 2-var phase-plane flow is the n = 2 case of the generic integrator.
    let traj = crate::ode::integrate(&[x0, y0], dt, steps as usize, |st, d| {
        d[0] = fx.eval(st[0], st[1]);
        d[1] = fy.eval(st[0], st[1]);
    });
    traj.into_iter().map(|s| (s[0], s[1])).collect()
}

/// `spline(id, p0, p1, …)` — a smooth Catmull-Rom curve passing through every
/// given point (screen coords), with a dot at each knot (`{id}.k0`, … tagged
/// `{id}.knots`). Declare `untraced(id)` + `draw(id, dur)` to trace it on.
/// manic's answer to "draw a smooth curve through these data points."
fn c_spline(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let mut pts = Vec::new();
    for i in 1..a.len() {
        pts.push(a.pair(i)?);
    }
    if pts.len() < 2 {
        return Err(Error::new(
            "`spline` needs at least 2 points: `spline(id, (x0,y0), (x1,y1), …)`",
            a.name_span,
        ));
    }
    for (k, p) in pts.iter().enumerate() {
        let mut d = Entity::new(format!("{id}.k{k}"), Shape::Circle { r: 5.0 }, *p, style::MAGENTA);
        d.stroke.fill = true;
        d.stroke.outline = false;
        d.tags.push(format!("{id}.knots"));
        s.add(d);
    }
    let curve = catmull_rom(&pts, 24);
    let mut e = Entity::new(id.clone(), Shape::Polyline { pts: curve }, Vec2::ZERO, style::CYAN);
    e.stroke.width = 3.0;
    e.tags.push(id);
    s.add(e);
    Ok(())
}

/// `trajectory(id, "dx/dt", "dy/dt", (x0,y0), (cx,cy), scale, [steps])` — the
/// path a point follows under the differential system `dx/dt = fx(x,y)`,
/// `dy/dt = fy(x,y)`, integrated (RK4) from math point `(x0,y0)` and mapped to
/// screen as `(cx + x*scale, cy − y*scale)`. Phase portraits, orbits, spirals.
/// (For `dy/dx = f(x,y)`, use `"1"` and `"f(x,y)"`.) Declare `untraced(id)` +
/// `draw(id, dur)` to watch the point flow along it.
fn c_trajectory(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let fx = expr::compile(&a.text(1)?)
        .map_err(|m| Error::new(format!("in trajectory dx/dt: {m}"), a.span_of(1)))?;
    let fy = expr::compile(&a.text(2)?)
        .map_err(|m| Error::new(format!("in trajectory dy/dt: {m}"), a.span_of(2)))?;
    let start = a.pair(3)?;
    let center = a.pair(4)?;
    let scale = a.num(5)?;
    let steps = a.opt_num(6)?.map(|v| v as u32).unwrap_or(400).clamp(2, 5000);
    let path = rk4_path(&fx, &fy, start.x, start.y, 0.02, steps);
    let pts: Vec<Vec2> = path
        .iter()
        .map(|&(x, y)| Vec2::new(center.x + x * scale, center.y - y * scale))
        .collect();
    let mut e = Entity::new(id.clone(), Shape::Polyline { pts }, Vec2::ZERO, style::LIME);
    e.stroke.width = 3.0;
    e.tags.push(id);
    s.add(e);
    Ok(())
}

/// `vector(id, (cx,cy), (dx,dy), [color])` — a glowing arrow from the origin
/// to origin + (dx, -dy) (dy is up). Defaults to magenta.
fn c_vector(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let o = a.pair(1)?;
    let d = a.pair(2)?;
    let color = if a.len() > 3 {
        resolve_color(&a.ident(3)?, a.span_of(3))?
    } else {
        style::MAGENTA
    };
    let mut e = Entity::new(
        id,
        Shape::Arrow {
            to: Vec2::new(o.x + d.x, o.y - d.y),
        },
        o,
        color,
    );
    e.stroke.width = 3.0;
    s.add(e);
    Ok(())
}

/// `numberline(id, (cx,cy), halfw, from, to, step)` — a dim axis with ticks
/// and labels. Children `{id}.axis`, `{id}.tN`, `{id}.lN`, tag `id`.
fn c_numberline(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let hw = a.num(2)?;
    let from = a.num(3)?;
    let to = a.num(4)?;
    let step = a.num(5)?;
    if step <= 0.0 || to <= from {
        return Err(Error::new(
            "numberline needs step > 0 and to > from",
            a.name_span,
        ));
    }

    let mut axis = Entity::new(
        format!("{id}.axis"),
        Shape::Arrow {
            to: Vec2::new(c.x + hw, c.y),
        },
        Vec2::new(c.x - hw, c.y),
        style::DIM,
    );
    axis.stroke.width = 2.0;
    axis.tags.push(id.clone());
    s.add(axis);

    let span = to - from;
    let mut v = from;
    let mut i = 0;
    // guard against float drift producing a runaway loop
    while v <= to + 1e-4 && i < 1000 {
        let x = c.x - hw + (v - from) / span * (2.0 * hw);
        let mut tick = Entity::new(
            format!("{id}.t{i}"),
            Shape::Line {
                to: Vec2::new(x, c.y + 8.0),
            },
            Vec2::new(x, c.y - 8.0),
            style::DIM,
        );
        tick.stroke.width = 2.0;
        tick.tags.push(id.clone());
        s.add(tick);

        let mut lbl = Entity::new(
            format!("{id}.l{i}"),
            Shape::Text {
                content: fmt_num(v),
                size: 18.0,
            },
            Vec2::new(x, c.y + 30.0),
            style::FG,
        );
        lbl.font = FontKind::Mono;
        lbl.tags.push(id.clone());
        s.add(lbl);

        v += step;
        i += 1;
    }
    Ok(())
}

/// `arc(id, (cx,cy), r, start, sweep)` — a plain circular arc line (degrees).
fn c_arc(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let r = a.num(2)?;
    let start = a.num(3)?;
    let sweep = a.num(4)?;
    let mut e = Entity::new(
        id,
        Shape::Arc {
            r,
            inner: 0.0,
            start,
            sweep,
        },
        c,
        style::CYAN,
    );
    e.stroke = StrokeStyle {
        fill: false,
        outline: true,
        width: 3.0,
        outline_color: Some(style::CYAN),
    };
    s.add(e);
    Ok(())
}

fn neon_sector(id: String, c: Vec2, r: f32, inner: f32, start: f32, sweep: f32) -> Entity {
    let mut e = Entity::new(
        id,
        Shape::Arc {
            r,
            inner,
            start,
            sweep,
        },
        c,
        style::PANEL,
    );
    e.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 2.5,
        outline_color: Some(style::CYAN),
    };
    e
}

/// `sector(id, (cx,cy), r, start, sweep)` — a filled pie slice (degrees).
fn c_sector(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let r = a.num(2)?;
    let start = a.num(3)?;
    let sweep = a.num(4)?;
    s.add(neon_sector(id, c, r, 0.0, start, sweep));
    Ok(())
}

/// `annulus(id, (cx,cy), outer, inner)` — a full ring.
fn c_annulus(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let outer = a.num(2)?;
    let inner = a.num(3)?;
    if inner >= outer {
        return Err(Error::new("annulus needs outer > inner", a.name_span));
    }
    s.add(neon_sector(id, c, outer, inner, 0.0, 360.0));
    Ok(())
}

/// `pie(id, (cx,cy), r, n)` — a circle cut into `n` equal filled sectors,
/// each addressable as `{id}0 … {id}{n-1}` (tag `id`). The one-liner behind
/// "cut a circle equally".
fn c_pie(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let r = a.num(2)?;
    let n = a.num(3)? as i64;
    if n < 1 || n > 360 {
        return Err(Error::new("pie needs 1..=360 slices", a.span_of(3)));
    }
    let step = 360.0 / n as f32;
    for k in 0..n {
        let mut e = neon_sector(format!("{id}{k}"), c, r, 0.0, k as f32 * step, step);
        e.tags.push(id.clone());
        s.add(e);
    }
    Ok(())
}

// ---- vector fields --------------------------------------------------------

/// Evaluate a named 2D vector field at math coords `(u, v)`. Returns the
/// change vector `(du, dv)`, or `None` for unknown names.
fn field(name: &str, u: f32, v: f32) -> Option<(f32, f32)> {
    Some(match name {
        "radial" | "source" | "out" => (u, v),
        "sink" | "attract" | "in" => (-u, -v),
        "swirl" | "rotational" | "curl" | "rotate" => (-v, u),
        "saddle" => (u, -v),
        "wave" => (v.sin(), u.cos()),
        "shear" => (v, 0.0),
        "uniform" | "flow" => (1.0, 0.0),
        "spiral" => (-v + u * 0.4, u + v * 0.4),
        _ => return None,
    })
}

fn known_field(name: &str) -> bool {
    field(name, 0.0, 0.0).is_some()
}

fn lerp_col(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        1.0,
    )
}

/// Neon magnitude gradient: cyan → lime → magenta.
fn grad(t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        lerp_col(style::CYAN, style::LIME, t * 2.0)
    } else {
        lerp_col(style::LIME, style::MAGENTA, (t - 0.5) * 2.0)
    }
}

/// `arrowfield(id, (cx,cy), halfw, halfh, field, [n])` — a grid of arrows
/// sampling a named vector field, coloured by magnitude (cyan→lime→magenta).
/// Arrows `{id}.a{i}`, tag `id`.
fn c_arrowfield(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let hw = a.num(2)?;
    let hh = a.num(3)?;
    let f = a.ident(4)?;
    let cols = a.opt_num(5)?.unwrap_or(13.0).max(2.0) as usize;
    if !known_field(&f) {
        return Err(Error::new(
            format!(
                "unknown field `{f}` (try: radial, sink, swirl, saddle, wave, shear, uniform, spiral)"
            ),
            a.span_of(4),
        ));
    }
    let rows = ((cols as f32) * hh / hw).round().max(2.0) as usize;
    let sxg = 2.0 * hw / cols as f32;
    let syg = 2.0 * hh / rows as f32;
    let unit = hw / 3.0; // math range u,v ∈ [-3, 3] across the box
    let maxlen = sxg.min(syg) * 0.46;

    // pass 1: sample, find the largest magnitude for normalisation
    let mut cells: Vec<(Vec2, Vec2, f32)> = Vec::with_capacity(cols * rows);
    let mut maxmag = 1e-6f32;
    for j in 0..rows {
        for i in 0..cols {
            let sp = Vec2::new(
                c.x - hw + sxg * (i as f32 + 0.5),
                c.y - hh + syg * (j as f32 + 0.5),
            );
            let (u, v) = ((sp.x - c.x) / unit, -(sp.y - c.y) / unit);
            let (du, dv) = field(&f, u, v).unwrap();
            let mag = (du * du + dv * dv).sqrt();
            let dir = Vec2::new(du, -dv).normalize_or_zero(); // dv up → screen down
            cells.push((sp, dir, mag));
            maxmag = maxmag.max(mag);
        }
    }

    // pass 2: draw an arrow per cell, length/colour by magnitude
    let tag = id.clone();
    for (k, (sp, dir, mag)) in cells.into_iter().enumerate() {
        let t = mag / maxmag;
        let len = maxlen * t;
        if len < 1.5 {
            continue; // skip near-zero vectors (e.g. a field's centre)
        }
        let to = sp + dir * len;
        let mut e = Entity::new(format!("{id}.a{k}"), Shape::Arrow { to }, sp, grad(t));
        e.stroke.width = 2.0;
        e.z = 1;
        e.tags.push(tag.clone());
        s.add(e);
    }
    Ok(())
}

/// `matrix(id, "a b; c d", (cx,cy), [cellw], [cellh])` — a bracketed matrix.
/// Rows are separated by `;`, entries by whitespace/commas. Entry `{id}.r{i}c{j}`
/// is tagged `{id}`, `{id}.entries`, `{id}.row{i}`, `{id}.col{j}` — so
/// `recolor(m.col1, …)` colours a column and `flash(m.row0, …)` a row. Brackets
/// are `{id}.lbrack` / `{id}.rbrack`.
fn c_matrix(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let src = a.text(1)?;
    let c = a.pair(2)?;
    let cw = a.opt_num(3)?.unwrap_or(88.0);
    let ch = a.opt_num(4)?.unwrap_or(70.0);

    let rows: Vec<Vec<String>> = src
        .split(';')
        .map(|r| {
            r.split(|c: char| c == ',' || c.is_whitespace())
                .filter(|t| !t.is_empty())
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
        })
        .filter(|r| !r.is_empty())
        .collect();
    if rows.is_empty() {
        return Err(Error::new("matrix has no entries", a.span_of(1)));
    }
    let ncols = rows.iter().map(|r| r.len()).max().unwrap();
    if rows.iter().any(|r| r.len() != ncols) {
        let counts: Vec<usize> = rows.iter().map(|r| r.len()).collect();
        return Err(Error::new(
            format!(
                "matrix rows must all have the same number of entries — got {counts:?} \
                 (entries split on whitespace AND commas, so a value like `(0,0)` counts as two)"
            ),
            a.span_of(1),
        ));
    }
    let nrows = rows.len();
    let totalw = (ncols as f32 - 1.0) * cw;
    let totalh = (nrows as f32 - 1.0) * ch;
    let x0 = c.x - totalw / 2.0;
    let y0 = c.y - totalh / 2.0;

    for (i, row) in rows.iter().enumerate() {
        for (j, val) in row.iter().enumerate() {
            let pos = Vec2::new(x0 + cw * j as f32, y0 + ch * i as f32);
            let mut e = Entity::new(
                format!("{id}.r{i}c{j}"),
                Shape::Text {
                    content: val.clone(),
                    size: 30.0,
                },
                pos,
                style::FG,
            );
            e.font = FontKind::MonoBold;
            e.z = 2;
            e.tags = vec![
                id.clone(),
                format!("{id}.entries"),
                format!("{id}.row{i}"),
                format!("{id}.col{j}"),
            ];
            s.add(e);
        }
    }

    // brackets flanking the grid (open polylines with serifs)
    let pad = ch * 0.45;
    let serif = 14.0;
    let margin = cw * 0.5;
    let (top, bot) = (y0 - pad, y0 + totalh + pad);
    let lx = x0 - margin;
    let mut lb = Entity::new(
        format!("{id}.lbrack"),
        Shape::Polyline {
            pts: vec![
                Vec2::new(lx + serif, top),
                Vec2::new(lx, top),
                Vec2::new(lx, bot),
                Vec2::new(lx + serif, bot),
            ],
        },
        Vec2::ZERO,
        style::CYAN,
    );
    lb.stroke.width = 3.0;
    lb.tags.push(id.clone());
    s.add(lb);
    let rx = x0 + totalw + margin;
    let mut rb = Entity::new(
        format!("{id}.rbrack"),
        Shape::Polyline {
            pts: vec![
                Vec2::new(rx - serif, top),
                Vec2::new(rx, top),
                Vec2::new(rx, bot),
                Vec2::new(rx - serif, bot),
            ],
        },
        Vec2::ZERO,
        style::CYAN,
    );
    rb.stroke.width = 3.0;
    rb.tags.push(id);
    s.add(rb);
    Ok(())
}

/// Split a whitespace/comma-separated string into non-empty tokens.
fn tokens(src: &str) -> Vec<String> {
    src.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

/// `table(id, "a b; c d", (cx,cy), [cellw], [cellh], [col-labels], [row-labels])`
/// — a ruled grid of single-token entries. Body cell `{id}.r{i}c{j}` is tagged
/// `{id}`, `{id}.entries`, `{id}.row{i}`, `{id}.col{j}`. Optional header strings
/// add a top label row (`{id}.collabel{j}`) and/or a left label column
/// (`{id}.rowlabel{i}`), tagged `{id}.labels`. Grid lines are `{id}.h{k}` /
/// `{id}.v{k}`, tagged `{id}.hlines` / `{id}.vlines` / `{id}.lines` — so
/// `recolor(t.hlines, …)` colours the rules. Aliases: `mathtable`,
/// `decimaltable`, `integertable` (entries are plain tokens either way).
fn c_table(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let src = a.text(1)?;
    let c = a.pair(2)?;
    let cw = a.opt_num(3)?.unwrap_or(120.0);
    let ch = a.opt_num(4)?.unwrap_or(64.0);
    let col_labels = if a.len() > 5 {
        tokens(&a.text(5)?)
    } else {
        vec![]
    };
    let row_labels = if a.len() > 6 {
        tokens(&a.text(6)?)
    } else {
        vec![]
    };

    let body: Vec<Vec<String>> = src
        .split(';')
        .map(|r| tokens(r))
        .filter(|r| !r.is_empty())
        .collect();
    if body.is_empty() {
        return Err(Error::new("table has no entries", a.span_of(1)));
    }
    let ncols = body.iter().map(|r| r.len()).max().unwrap();
    if body.iter().any(|r| r.len() != ncols) {
        let counts: Vec<usize> = body.iter().map(|r| r.len()).collect();
        return Err(Error::new(
            format!(
                "table rows must all have the same number of cells — got {counts:?} \
                 (cells split on whitespace AND commas, so a coord like `(0,0)` counts as two)"
            ),
            a.span_of(1),
        ));
    }
    let nrows = body.len();
    let has_col = if col_labels.is_empty() { 0 } else { 1 };
    let has_row = if row_labels.is_empty() { 0 } else { 1 };
    let gcols = ncols + has_row;
    let grows = nrows + has_col;

    let totalw = gcols as f32 * cw;
    let totalh = grows as f32 * ch;
    let x0 = c.x - totalw / 2.0;
    let y0 = c.y - totalh / 2.0;
    // centre of full-grid cell (r, c)
    let cell =
        |r: usize, col: usize| Vec2::new(x0 + (col as f32 + 0.5) * cw, y0 + (r as f32 + 0.5) * ch);
    let txt =
        |s: &mut Scene, id: String, content: String, pos: Vec2, color: Color, tags: Vec<String>| {
            let mut e = Entity::new(
                id,
                Shape::Text {
                    content,
                    size: 26.0,
                },
                pos,
                color,
            );
            e.font = FontKind::MonoBold;
            e.z = 2;
            e.tags = tags;
            s.add(e);
        };

    // body entries
    for (i, row) in body.iter().enumerate() {
        for (j, val) in row.iter().enumerate() {
            txt(
                s,
                format!("{id}.r{i}c{j}"),
                val.clone(),
                cell(i + has_col, j + has_row),
                style::FG,
                vec![
                    id.clone(),
                    format!("{id}.entries"),
                    format!("{id}.row{i}"),
                    format!("{id}.col{j}"),
                ],
            );
        }
    }
    // column labels across the top
    for (j, lbl) in col_labels.iter().enumerate().take(ncols) {
        txt(
            s,
            format!("{id}.collabel{j}"),
            lbl.clone(),
            cell(0, j + has_row),
            style::CYAN,
            vec![id.clone(), format!("{id}.labels")],
        );
    }
    // row labels down the left
    for (i, lbl) in row_labels.iter().enumerate().take(nrows) {
        txt(
            s,
            format!("{id}.rowlabel{i}"),
            lbl.clone(),
            cell(i + has_col, 0),
            style::CYAN,
            vec![id.clone(), format!("{id}.labels")],
        );
    }

    // grid lines (outer included)
    let line = |s: &mut Scene, id: String, from: Vec2, to: Vec2, tags: Vec<String>| {
        let mut e = Entity::new(id, Shape::Line { to }, from, style::DIM);
        e.stroke.width = 1.5;
        e.glow = 0.0;
        e.tags = tags;
        s.add(e);
    };
    for k in 0..=grows {
        let y = y0 + k as f32 * ch;
        line(
            s,
            format!("{id}.h{k}"),
            Vec2::new(x0, y),
            Vec2::new(x0 + totalw, y),
            vec![id.clone(), format!("{id}.hlines"), format!("{id}.lines")],
        );
    }
    for k in 0..=gcols {
        let x = x0 + k as f32 * cw;
        line(
            s,
            format!("{id}.v{k}"),
            Vec2::new(x, y0),
            Vec2::new(x, y0 + totalh),
            vec![id.clone(), format!("{id}.vlines"), format!("{id}.lines")],
        );
    }
    Ok(())
}

// ===================== linear algebra (flagship trio) =====================
// A 2×2 matrix [[a,b],[c,d]] *does something to space*. These draw it in math
// y-up (grid point (gx,gy) → screen (cx + gx*u, cy − gy*u)), so î,ĵ land on the
// matrix's columns, the area scales by the determinant, and eigenvectors are
// the real invariant directions.

/// The 2×2 determinant.
fn det2(a: f32, b: f32, c: f32, d: f32) -> f32 {
    a * d - b * c
}

/// Solve `[[a,b],[c,d]] x = (e,f)` by Cramer's rule; `None` when the matrix is
/// singular (det ≈ 0 — the two rows are parallel lines, no unique solution).
fn solve2(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Option<(f32, f32)> {
    let det = a * d - b * c;
    if det.abs() < 1e-6 {
        return None;
    }
    Some(((e * d - b * f) / det, (a * f - e * c) / det))
}

/// Real eigenpairs `(λ, unit eigenvector)` of `[[a,b],[c,d]]`; empty when the
/// eigenvalues are complex (a rotation — no real invariant line).
fn eig2(a: f32, b: f32, c: f32, d: f32) -> Vec<(f32, Vec2)> {
    let (tr, det) = (a + d, a * d - b * c);
    let disc = tr * tr - 4.0 * det;
    if disc < -1e-6 {
        return Vec::new();
    }
    let sq = disc.max(0.0).sqrt();
    let lambdas: Vec<f32> = if sq < 1e-5 {
        vec![tr / 2.0]
    } else {
        vec![(tr + sq) / 2.0, (tr - sq) / 2.0]
    };
    lambdas
        .into_iter()
        .map(|l| {
            let v = if b.abs() > 1e-6 {
                Vec2::new(b, l - a)
            } else if c.abs() > 1e-6 {
                Vec2::new(l - d, c)
            } else if (l - a).abs() < (l - d).abs() {
                Vec2::new(1.0, 0.0)
            } else {
                Vec2::new(0.0, 1.0)
            };
            let n = v.length();
            (l, if n > 1e-6 { v / n } else { Vec2::new(1.0, 0.0) })
        })
        .collect()
}

/// `linmap(id, (cx,cy), unit, a, b, c, d, [span])` — a 2×2 matrix applied to the
/// plane: a faint identity grid under the deformed (cyan) grid, with the basis
/// î (gold) and ĵ (magenta) landing on the matrix's columns `(a,c)` and `(b,d)`.
fn c_linmap(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let u = a.num(2)?;
    let (m11, m12, m21, m22) = (a.num(3)?, a.num(4)?, a.num(5)?, a.num(6)?);
    let span = a.opt_num(7)?.map(|v| v as i32).unwrap_or(4).clamp(1, 10);
    let sc = |gx: f32, gy: f32| Vec2::new(c.x + gx * u, c.y - gy * u);
    let map = |gx: f32, gy: f32| (m11 * gx + m12 * gy, m21 * gx + m22 * gy);
    let sp = span as f32;
    for k in -span..=span {
        let g = k as f32;
        // faint identity grid
        add_line(s, format!("{id}.ih{k}"), sc(-sp, g), sc(sp, g), style::DIM, 1.0, 0.2, -2, vec![id.clone()]);
        add_line(s, format!("{id}.iv{k}"), sc(g, -sp), sc(g, sp), style::DIM, 1.0, 0.2, -2, vec![id.clone()]);
        // deformed grid = identity mapped by M
        let (hx0, hy0) = map(-sp, g);
        let (hx1, hy1) = map(sp, g);
        add_line(s, format!("{id}.h{k}"), sc(hx0, hy0), sc(hx1, hy1), style::CYAN, 1.5, 0.85, -1, vec![id.clone()]);
        let (vx0, vy0) = map(g, -sp);
        let (vx1, vy1) = map(g, sp);
        add_line(s, format!("{id}.v{k}"), sc(vx0, vy0), sc(vx1, vy1), style::CYAN, 1.5, 0.85, -1, vec![id.clone()]);
    }
    // basis vectors, landing on the columns
    for (nm, tox, toy, col, lab) in [
        ("i", m11, m21, style::GOLD, "i"),
        ("j", m12, m22, style::MAGENTA, "j"),
    ] {
        let mut arr = Entity::new(format!("{id}.{nm}"), Shape::Arrow { to: sc(tox, toy) }, sc(0.0, 0.0), col);
        arr.stroke.width = 4.0;
        arr.tags.push(id.clone());
        s.add(arr);
        let mut t = Entity::new(
            format!("{id}.l{nm}"),
            Shape::Text { content: lab.to_string(), size: 22.0 },
            sc(tox, toy) + Vec2::new(14.0, -12.0),
            col,
        );
        t.font = FontKind::MonoBold;
        t.tags.push(id.clone());
        s.add(t);
    }
    Ok(())
}

/// `determinant(id, (cx,cy), unit, a, b, c, d, [color])` — the unit square (faint)
/// and its image under the matrix (a filled parallelogram), labelled with the
/// signed area = det. The fill flips colour when det < 0 (orientation reversed);
/// at det = 0 the parallelogram collapses to a line.
fn c_determinant(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let u = a.num(2)?;
    let (m11, m12, m21, m22) = (a.num(3)?, a.num(4)?, a.num(5)?, a.num(6)?);
    let det = det2(m11, m12, m21, m22);
    let color = if a.len() > 7 {
        resolve_color(&a.ident(7)?, a.span_of(7))?
    } else if det < 0.0 {
        style::MAGENTA
    } else {
        style::LIME
    };
    let sc = |gx: f32, gy: f32| Vec2::new(c.x + gx * u, c.y - gy * u);
    // faint unit square (identity)
    let unit_sq = vec![sc(0.0, 0.0), sc(1.0, 0.0), sc(1.0, 1.0), sc(0.0, 1.0)];
    let mut usq = Entity::new(format!("{id}.unit"), Shape::Polygon { pts: unit_sq }, Vec2::ZERO, style::DIM);
    usq.stroke.fill = false;
    usq.stroke.outline = true;
    usq.opacity = 0.35;
    usq.tags.push(id.clone());
    s.add(usq);
    // image parallelogram: columns (m11,m21) and (m12,m22)
    let para = vec![sc(0.0, 0.0), sc(m11, m21), sc(m11 + m12, m21 + m22), sc(m12, m22)];
    let mut e = Entity::new(id.clone(), Shape::Polygon { pts: para }, Vec2::ZERO, color);
    e.stroke.fill = true;
    e.stroke.outline = true;
    e.stroke.outline_color = Some(color);
    e.opacity = 0.45;
    e.tags.push(id.clone());
    s.add(e);
    let mid = sc(0.5 * (m11 + m12), 0.5 * (m21 + m22));
    let mut lbl = Entity::new(
        format!("{id}.val"),
        Shape::Text { content: format!("det = {det:.2}"), size: 24.0 },
        mid,
        color,
    );
    lbl.font = FontKind::MonoBold;
    lbl.tags.push(id);
    s.add(lbl);
    Ok(())
}

/// `eigen(id, (cx,cy), unit, a, b, c, d, [color])` — the matrix's real
/// eigenvectors as lines through the origin (the invariant directions — a vector
/// on them only stretches, by the eigenvalue λ, shown). Complex eigenvalues (a
/// rotation) leave a short note instead.
fn c_eigen(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let u = a.num(2)?;
    let (m11, m12, m21, m22) = (a.num(3)?, a.num(4)?, a.num(5)?, a.num(6)?);
    let color = if a.len() > 7 {
        resolve_color(&a.ident(7)?, a.span_of(7))?
    } else {
        style::GOLD
    };
    let sc = |gx: f32, gy: f32| Vec2::new(c.x + gx * u, c.y - gy * u);
    let pairs = eig2(m11, m12, m21, m22);
    if pairs.is_empty() {
        let mut note = Entity::new(
            format!("{id}.note"),
            Shape::Text { content: "complex eigenvalues (a rotation)".to_string(), size: 20.0 },
            c + Vec2::new(0.0, u * 2.6),
            style::DIM,
        );
        note.tags.push(id);
        s.add(note);
        return Ok(());
    }
    let ext = 4.0;
    for (k, (l, v)) in pairs.iter().enumerate() {
        add_line(
            s,
            format!("{id}.line{k}"),
            sc(-v.x * ext, -v.y * ext),
            sc(v.x * ext, v.y * ext),
            color,
            3.0,
            1.0,
            2,
            vec![id.clone()],
        );
        let mut lbl = Entity::new(
            format!("{id}.l{k}"),
            Shape::Text { content: format!("lambda = {l:.2}"), size: 20.0 },
            sc(v.x * (ext - 0.6), v.y * (ext - 0.6)) + Vec2::new(0.0, -14.0),
            color,
        );
        lbl.font = FontKind::MonoBold;
        lbl.tags.push(id.clone());
        s.add(lbl);
    }
    Ok(())
}

/// `diagonalise(id, (cx,cy), unit, a, b, c, d, [color])` — the eigendecomposition
/// `A = P D P⁻¹` made visual: in the **eigenbasis** the matrix is a pure diagonal
/// stretch. Draws the (generally skewed) eigen-grid, the eigen-axes, and the unit
/// eigen-cell together with its **image** under A — a cell stretched by λ₁ along
/// e₁ and λ₂ along e₂ with NO shear (its edges stay parallel to the eigenvectors).
/// Complex/repeated eigenvalues (no real 2-D eigenbasis) leave a short note.
fn c_diagonalise(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let u = a.num(2)?;
    let (m11, m12, m21, m22) = (a.num(3)?, a.num(4)?, a.num(5)?, a.num(6)?);
    let color = if a.len() > 7 {
        resolve_color(&a.ident(7)?, a.span_of(7))?
    } else {
        style::CYAN
    };
    let sc = |p: Vec2| Vec2::new(c.x + p.x * u, c.y - p.y * u);
    let pairs = eig2(m11, m12, m21, m22);
    if pairs.len() < 2 {
        let mut note = Entity::new(
            format!("{id}.note"),
            Shape::Text {
                content: "no real eigenbasis (complex or repeated eigenvalues)".to_string(),
                size: 20.0,
            },
            c + Vec2::new(0.0, u * 2.6),
            style::DIM,
        );
        note.tags.push(id);
        s.add(note);
        return Ok(());
    }
    let (l1, e1) = pairs[0];
    let (l2, e2) = pairs[1];
    let span = 3i32;
    let sp = span as f32;
    // faint eigen-grid: the (skewed) coordinate frame of the eigenbasis.
    for k in -span..=span {
        let kf = k as f32;
        add_line(s, format!("{id}.g{k}a"), sc(-sp * e1 + kf * e2), sc(sp * e1 + kf * e2), style::DIM, 1.0, 0.2, -2, vec![id.clone()]);
        add_line(s, format!("{id}.g{k}b"), sc(kf * e1 - sp * e2), sc(kf * e1 + sp * e2), style::DIM, 1.0, 0.2, -2, vec![id.clone()]);
    }
    // eigen-axes through the origin (the invariant directions)
    let ext = sp + 0.5;
    add_line(s, format!("{id}.axis1"), sc(-ext * e1), sc(ext * e1), style::GOLD, 2.0, 0.8, -1, vec![id.clone()]);
    add_line(s, format!("{id}.axis2"), sc(-ext * e2), sc(ext * e2), style::MAGENTA, 2.0, 0.8, -1, vec![id.clone()]);
    // unit eigen-cell (faint) and its image under A: stretched by λ along each axis.
    let cell = vec![sc(Vec2::ZERO), sc(e1), sc(e1 + e2), sc(e2)];
    let mut ce = Entity::new(format!("{id}.cell"), Shape::Polygon { pts: cell }, Vec2::ZERO, style::DIM);
    ce.stroke.fill = false;
    ce.stroke.outline = true;
    ce.opacity = 0.5;
    ce.tags.push(id.clone());
    s.add(ce);
    let img = vec![sc(Vec2::ZERO), sc(l1 * e1), sc(l1 * e1 + l2 * e2), sc(l2 * e2)];
    let mut ie = Entity::new(format!("{id}.img"), Shape::Polygon { pts: img }, Vec2::ZERO, color);
    ie.stroke.fill = true;
    ie.stroke.outline = true;
    ie.stroke.outline_color = Some(color);
    ie.opacity = 0.4;
    ie.tags.push(id.clone());
    s.add(ie);
    // the eigenvectors' images λ·e as arrows (only stretch, never turn)
    for (nm, l, e, col) in [("v1", l1, e1, style::GOLD), ("v2", l2, e2, style::MAGENTA)] {
        let mut arr = Entity::new(format!("{id}.{nm}"), Shape::Arrow { to: sc(l * e) }, sc(Vec2::ZERO), col);
        arr.stroke.width = 4.0;
        arr.tags.push(id.clone());
        s.add(arr);
        let mut lbl = Entity::new(
            format!("{id}.{nm}l"),
            Shape::Text { content: format!("lambda = {l:.2}"), size: 20.0 },
            sc(l * e) + Vec2::new(10.0, -12.0),
            col,
        );
        lbl.font = FontKind::MonoBold;
        lbl.tags.push(id.clone());
        s.add(lbl);
    }
    Ok(())
}

/// Format a value for a matrix cell: integers plain, otherwise up to 2 decimals
/// with trailing zeros trimmed. Snaps tiny values (incl. -0.0) to "0".
fn fmt_cell(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r.abs() < 1e-6 {
        return "0".to_string();
    }
    if (r - r.round()).abs() < 1e-6 {
        format!("{}", r.round() as i64)
    } else {
        format!("{r:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Gauss-Jordan elimination: returns each intermediate matrix state paired with
/// the row operation that produced it. The first entry is the untouched input
/// ("start"); the last is the reduced row-echelon form.
fn rref_steps(m0: Vec<Vec<f32>>) -> Vec<(Vec<Vec<f32>>, String)> {
    let mut m = m0;
    let nrows = m.len();
    let ncols = m[0].len();
    let mut steps = vec![(m.clone(), "start".to_string())];
    let mut pr = 0usize; // current pivot row
    for col in 0..ncols {
        if pr >= nrows {
            break;
        }
        // partial pivot: the largest |value| at or below pr in this column
        let mut piv = pr;
        for r in (pr + 1)..nrows {
            if m[r][col].abs() > m[piv][col].abs() {
                piv = r;
            }
        }
        if m[piv][col].abs() < 1e-9 {
            continue; // no pivot available in this column
        }
        if piv != pr {
            m.swap(piv, pr);
            steps.push((m.clone(), format!("swap R{} <-> R{}", pr + 1, piv + 1)));
        }
        let pv = m[pr][col];
        if (pv - 1.0).abs() > 1e-9 {
            for j in 0..ncols {
                m[pr][j] /= pv;
            }
            steps.push((m.clone(), format!("R{} -> R{} / {}", pr + 1, pr + 1, fmt_cell(pv))));
        }
        for r in 0..nrows {
            if r == pr {
                continue;
            }
            let f = m[r][col];
            if f.abs() > 1e-9 {
                for j in 0..ncols {
                    m[r][j] -= f * m[pr][j];
                }
                let sign = if f > 0.0 { "-" } else { "+" };
                steps.push((
                    m.clone(),
                    format!("R{} -> R{} {} {} R{}", r + 1, r + 1, sign, fmt_cell(f.abs()), pr + 1),
                ));
            }
        }
        pr += 1;
    }
    steps
}

/// `rref(id, "2 1 5 ; 1 3 10", (cx,cy), [cellw], [rowh])` — animated Gaussian
/// elimination. The matrix (rows split on `;`) is reduced to reduced row-echelon
/// form one row operation at a time. Every intermediate state is drawn at the
/// SAME spot as its own matrix, tagged `{id}.s{k}` (all hidden but the brackets),
/// with the row-op text as `{id}.op{k}`. Reveal them in order (cross-fade
/// `s{k-1}`→`s{k}`) to watch the numbers transform in place; `{id}.s0` is the
/// untouched input, the last `s{k}` is the RREF (for an augmented `A|b`, its last
/// column is the solution).
fn c_rref(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let src = a.text(1)?;
    let c = a.pair(2)?;
    let cw = a.opt_num(3)?.unwrap_or(96.0);
    let ch = a.opt_num(4)?.unwrap_or(64.0);
    let mut rows: Vec<Vec<f32>> = Vec::new();
    for seg in src.split(';') {
        let toks = tokens(seg);
        if toks.is_empty() {
            continue;
        }
        let mut row = Vec::with_capacity(toks.len());
        for t in &toks {
            match t.parse::<f32>() {
                Ok(v) => row.push(v),
                Err(_) => {
                    return Err(Error::new(
                        format!("rref entry `{t}` is not a number (rows separated by `;`)"),
                        a.span_of(1),
                    ))
                }
            }
        }
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(Error::new("rref has no entries".to_string(), a.span_of(1)));
    }
    let ncols = rows[0].len();
    if rows.iter().any(|r| r.len() != ncols) {
        return Err(Error::new(
            "rref rows must all have the same length".to_string(),
            a.span_of(1),
        ));
    }
    let nrows = rows.len();
    let steps = rref_steps(rows);
    let totalw = (ncols as f32 - 1.0) * cw;
    let totalh = (nrows as f32 - 1.0) * ch;
    let x0 = c.x - totalw / 2.0;
    let y0 = c.y - totalh / 2.0;
    // static brackets flanking the grid (always visible — the frame the numbers fill)
    let pad = ch * 0.45;
    let serif = 14.0;
    let margin = cw * 0.5;
    let (top, bot) = (y0 - pad, y0 + totalh + pad);
    for (nm, bx, dir) in [("lbrack", x0 - margin, 1.0f32), ("rbrack", x0 + totalw + margin, -1.0)] {
        let mut b = Entity::new(
            format!("{id}.{nm}"),
            Shape::Polyline {
                pts: vec![
                    Vec2::new(bx + dir * serif, top),
                    Vec2::new(bx, top),
                    Vec2::new(bx, bot),
                    Vec2::new(bx + dir * serif, bot),
                ],
            },
            Vec2::ZERO,
            style::CYAN,
        );
        b.stroke.width = 3.0;
        b.tags.push(id.clone());
        s.add(b);
    }
    // each elimination state, stacked at the same center, hidden until revealed
    for (k, (grid, _op)) in steps.iter().enumerate() {
        for (i, row) in grid.iter().enumerate() {
            for (j, val) in row.iter().enumerate() {
                let pos = Vec2::new(x0 + cw * j as f32, y0 + ch * i as f32);
                let mut e = Entity::new(
                    format!("{id}.s{k}r{i}c{j}"),
                    Shape::Text { content: fmt_cell(*val), size: 30.0 },
                    pos,
                    style::FG,
                );
                e.font = FontKind::MonoBold;
                e.z = 2;
                e.opacity = 0.0;
                e.tags = vec![id.clone(), format!("{id}.s{k}")];
                s.add(e);
            }
        }
    }
    // the row-operation caption for each state (overlaid below the matrix), hidden
    for (k, (_grid, op)) in steps.iter().enumerate() {
        let mut t = Entity::new(
            format!("{id}.op{k}"),
            Shape::Text { content: op.clone(), size: 24.0 },
            Vec2::new(c.x, y0 + totalh + ch * 0.95),
            style::GOLD,
        );
        t.font = FontKind::MonoBold;
        t.opacity = 0.0;
        t.tags.push(id.clone());
        s.add(t);
    }
    Ok(())
}

/// `project(id, (cx,cy), unit, (bx,by), (ax,ay), [color])` — the orthogonal
/// projection of vector **b** onto the line spanned by **a**: draws the subspace
/// line, b, its shadow `p = (b·a / a·a) a` on the line, and the residual `b - p`
/// meeting the line at a right angle (the shortest distance from b to the space).
fn c_project(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let u = a.num(2)?;
    let b = a.pair(3)?;
    let av = a.pair(4)?;
    let color = if a.len() > 5 {
        resolve_color(&a.ident(5)?, a.span_of(5))?
    } else {
        style::CYAN
    };
    let sc = |p: Vec2| Vec2::new(c.x + p.x * u, c.y - p.y * u);
    let denom = av.x * av.x + av.y * av.y;
    if denom < 1e-9 {
        return Err(Error::new(
            "project: the subspace vector (ax,ay) cannot be zero".to_string(),
            a.span_of(4),
        ));
    }
    let t = (b.x * av.x + b.y * av.y) / denom; // b·a / a·a
    let p = Vec2::new(t * av.x, t * av.y); // the projection point
    // the subspace: the line spanned by a, through the origin
    let n = denom.sqrt();
    let ah = av / n;
    let ext = 4.5;
    add_line(s, format!("{id}.line"), sc(-ext * ah), sc(ext * ah), style::DIM, 2.0, 0.7, -1, vec![id.clone()]);
    // b (the vector), p (its shadow), and the residual b - p
    let arrow = |s: &mut Scene, nm: &str, to: Vec2, col| {
        let mut e = Entity::new(format!("{id}.{nm}"), Shape::Arrow { to: sc(to) }, sc(Vec2::ZERO), col);
        e.stroke.width = 4.0;
        e.tags.push(id.clone());
        s.add(e);
    };
    arrow(s, "b", b, color);
    arrow(s, "p", p, style::GOLD);
    add_line(s, format!("{id}.res"), sc(p), sc(b), style::MAGENTA, 2.5, 0.9, 0, vec![id.clone()]);
    // right-angle mark at p, in the corner between the line and the residual
    let eh = {
        let d = b - p;
        let l = d.length().max(1e-6);
        d / l
    };
    let sq = 0.3;
    let d1 = ah * (if t >= 0.0 { -sq } else { sq });
    let d2 = eh * sq;
    let mut rt = Entity::new(
        format!("{id}.rt"),
        Shape::Polyline { pts: vec![sc(p + d1), sc(p + d1 + d2), sc(p + d2)] },
        Vec2::ZERO,
        style::DIM,
    );
    rt.stroke.width = 2.0;
    rt.tags.push(id.clone());
    s.add(rt);
    // labels
    for (nm, at, txt, col) in [
        ("blabel", b, "b", color),
        ("plabel", p, "proj", style::GOLD),
    ] {
        let mut lbl = Entity::new(
            format!("{id}.{nm}"),
            Shape::Text { content: txt.to_string(), size: 22.0 },
            sc(at) + Vec2::new(12.0, -12.0),
            col,
        );
        lbl.font = FontKind::MonoBold;
        lbl.tags.push(id.clone());
        s.add(lbl);
    }
    Ok(())
}

/// Least-squares fit `y = m x + k` through points; `None` if every point shares
/// one x (a vertical line, not expressible as `y = m x + k`).
fn fit_line(pts: &[Vec2]) -> Option<(f32, f32)> {
    let n = pts.len() as f32;
    let (mut sx, mut sy, mut sxx, mut sxy) = (0.0f32, 0.0, 0.0, 0.0);
    for p in pts {
        sx += p.x;
        sy += p.y;
        sxx += p.x * p.x;
        sxy += p.x * p.y;
    }
    let d = n * sxx - sx * sx;
    if d.abs() < 1e-9 {
        return None;
    }
    let m = (n * sxy - sx * sy) / d;
    Some((m, (sy - m * sx) / n))
}

/// `leastsquares(id, (cx,cy), unit, "x1 y1  x2 y2  …", [color])` — the best-fit
/// line through a point cloud. Draws the points, the line `y = m x + c` that
/// minimises the sum of squared **vertical residuals**, and each residual as a
/// thin segment from its point to the line. Also known as linear regression.
fn c_leastsquares(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let u = a.num(2)?;
    let src = a.text(3)?;
    let color = if a.len() > 4 {
        resolve_color(&a.ident(4)?, a.span_of(4))?
    } else {
        style::CYAN
    };
    let nums: Vec<f32> = tokens(&src).iter().filter_map(|t| t.parse::<f32>().ok()).collect();
    if nums.len() < 4 || nums.len() % 2 != 0 {
        return Err(Error::new(
            "leastsquares needs an even list of at least two points: \"x1 y1 x2 y2 ...\"".to_string(),
            a.span_of(3),
        ));
    }
    let pts: Vec<Vec2> = nums.chunks(2).map(|p| Vec2::new(p[0], p[1])).collect();
    let sc = |p: Vec2| Vec2::new(c.x + p.x * u, c.y - p.y * u);
    let (m, k) = fit_line(&pts).ok_or_else(|| {
        Error::new(
            "leastsquares: all points share one x — no y = m x + c fit (a vertical line)".to_string(),
            a.span_of(3),
        )
    })?;
    let x0 = pts.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let x1 = pts.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let pad = (x1 - x0) * 0.12 + 0.5;
    let (lx0, lx1) = (x0 - pad, x1 + pad);
    add_line(
        s,
        format!("{id}.line"),
        sc(Vec2::new(lx0, m * lx0 + k)),
        sc(Vec2::new(lx1, m * lx1 + k)),
        style::GOLD,
        3.0,
        1.0,
        0,
        vec![id.clone()],
    );
    // residuals (thin vertical segments) then the points on top
    for (i, p) in pts.iter().enumerate() {
        let yhat = m * p.x + k;
        add_line(s, format!("{id}.r{i}"), sc(*p), sc(Vec2::new(p.x, yhat)), style::MAGENTA, 2.0, 0.8, -1, vec![id.clone(), format!("{id}.residuals")]);
    }
    for (i, p) in pts.iter().enumerate() {
        let mut e = Entity::new(format!("{id}.p{i}"), Shape::Circle { r: 7.0 }, sc(*p), color);
        e.stroke.fill = true;
        e.z = 2;
        e.tags = vec![id.clone(), format!("{id}.points")];
        s.add(e);
    }
    let sign = if k >= 0.0 { "+" } else { "-" };
    let midx = (lx0 + lx1) / 2.0;
    let mut lbl = Entity::new(
        format!("{id}.eq"),
        Shape::Text {
            content: format!("y = {:.2} x {} {:.2}", m, sign, k.abs()),
            size: 22.0,
        },
        sc(Vec2::new(midx, m * midx + k)) + Vec2::new(-40.0, -34.0),
        style::GOLD,
    );
    lbl.font = FontKind::MonoBold;
    lbl.tags.push(id);
    s.add(lbl);
    Ok(())
}

/// Two math-coord endpoints of the line `A·x + B·y = C`, spanning ±`ext` along
/// whichever axis keeps the segment in view. `None` for a degenerate equation.
fn line_eq_pts(a: f32, b: f32, cc: f32, ext: f32) -> Option<(Vec2, Vec2)> {
    if a.abs() < 1e-6 && b.abs() < 1e-6 {
        return None;
    }
    if b.abs() >= a.abs() {
        Some((
            Vec2::new(-ext, (cc + a * ext) / b),
            Vec2::new(ext, (cc - a * ext) / b),
        ))
    } else {
        Some((
            Vec2::new((cc + b * ext) / a, -ext),
            Vec2::new((cc - b * ext) / a, ext),
        ))
    }
}

/// `linsolve(id, (cx,cy), unit, a, b, c, d, e, f, [span])` — the *row picture* of
/// `Ax=b`: `a·x+b·y=e` and `c·x+d·y=f` drawn as two lines; their intersection is
/// the solution (a gold dot + its coords). Parallel lines (det = 0) → a note.
fn c_linsolve(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let u = a.num(2)?;
    let (m11, m12, m21, m22) = (a.num(3)?, a.num(4)?, a.num(5)?, a.num(6)?);
    let (e, f) = (a.num(7)?, a.num(8)?);
    let ext = a.opt_num(9)?.map(|v| v as f32).unwrap_or(5.0);
    let sc = |p: Vec2| Vec2::new(c.x + p.x * u, c.y - p.y * u);
    if let Some((p0, p1)) = line_eq_pts(m11, m12, e, ext) {
        add_line(s, format!("{id}.r1"), sc(p0), sc(p1), style::CYAN, 3.0, 1.0, 0, vec![id.clone()]);
    }
    if let Some((p0, p1)) = line_eq_pts(m21, m22, f, ext) {
        add_line(s, format!("{id}.r2"), sc(p0), sc(p1), style::MAGENTA, 3.0, 1.0, 0, vec![id.clone()]);
    }
    let (sx, sy) = match solve2(m11, m12, m21, m22, e, f) {
        Some(xy) => xy,
        None => {
            let mut note = Entity::new(
                format!("{id}.note"),
                Shape::Text { content: "no unique solution (parallel lines)".to_string(), size: 20.0 },
                c + Vec2::new(0.0, u * 2.6),
                style::DIM,
            );
            note.tags.push(id);
            s.add(note);
            return Ok(());
        }
    };
    let mut dot = Entity::new(id.clone(), Shape::Circle { r: 8.0 }, sc(Vec2::new(sx, sy)), style::GOLD);
    dot.stroke.fill = true;
    dot.tags.push(id.clone());
    s.add(dot);
    let mut lbl = Entity::new(
        format!("{id}.val"),
        Shape::Text { content: format!("({sx:.2}, {sy:.2})"), size: 22.0 },
        sc(Vec2::new(sx, sy)) + Vec2::new(18.0, -18.0),
        style::GOLD,
    );
    lbl.font = FontKind::MonoBold;
    lbl.tags.push(id);
    s.add(lbl);
    Ok(())
}

/// `span(id, (cx,cy), unit, (vx,vy), [(wx,wy)], [color])` — the span of one or two
/// vectors: one vector (or two dependent ones) spans a **line** through the
/// origin; two independent vectors span the **whole plane** (a faint region).
/// The dependent case is the rank/collapse picture.
fn c_span(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let u = a.num(2)?;
    let v = a.pair(3)?;
    // arg 4 is either the second vector (a pair) or the colour
    let (w, cidx) = match a.pair(4) {
        Ok(p) => (Some(p), 5),
        Err(_) => (None, 4),
    };
    let color = if a.len() > cidx {
        resolve_color(&a.ident(cidx)?, a.span_of(cidx))?
    } else {
        style::GOLD
    };
    let sc = |p: Vec2| Vec2::new(c.x + p.x * u, c.y - p.y * u);
    let o = sc(Vec2::ZERO);
    let ext = 5.0;
    let arrow = |s: &mut Scene, nm: &str, p: Vec2, col| {
        let mut e = Entity::new(format!("{id}.{nm}"), Shape::Arrow { to: sc(p) }, o, col);
        e.stroke.width = 4.0;
        e.tags.push(id.clone());
        s.add(e);
    };
    arrow(s, "v", v, color);
    let span_line = |s: &mut Scene, dir: Vec2| {
        let n = dir.length().max(1e-6);
        let d = dir / n * ext;
        add_line(s, format!("{id}.line"), sc(-d), sc(d), style::DIM, 2.0, 0.85, -1, vec![id.clone()]);
    };
    match w {
        None => span_line(s, v),
        Some(w) => {
            arrow(s, "w", w, style::MAGENTA);
            let cross = v.x * w.y - v.y * w.x;
            if cross.abs() < 1e-6 {
                span_line(s, v); // dependent → a line (rank 1)
            } else {
                // independent → the whole plane (a faint region)
                let sq = vec![
                    sc(Vec2::new(-ext, -ext)),
                    sc(Vec2::new(ext, -ext)),
                    sc(Vec2::new(ext, ext)),
                    sc(Vec2::new(-ext, ext)),
                ];
                let mut e = Entity::new(format!("{id}.plane"), Shape::Polygon { pts: sq }, Vec2::ZERO, style::CYAN);
                e.stroke.fill = true;
                e.stroke.outline = false;
                e.opacity = 0.14;
                e.z = -2;
                e.tags.push(id.clone());
                s.add(e);
            }
        }
    }
    Ok(())
}

/// Register the math kit into `r`.
pub fn register(r: &mut Registry) {
    r.ctor("axes", c_axes);
    r.ctor("plane", c_plane);
    r.ctor("numberplane", c_plane);
    r.ctor("complexplane", c_complexplane);
    r.ctor("polarplane", c_polarplane);
    r.ctor("plot", c_plot);
    // `tangent` is registered by the geo kit, which dispatches the calculus
    // (curve) form here — see `geo::c_tangent`.
    r.ctor("normal", c_normal);
    r.ctor("slope", c_slope);
    r.ctor("area", c_area);
    r.ctor("integral", c_integral);
    r.ctor("roots", c_roots);
    r.ctor("newton", c_newton);
    r.ctor("spline", c_spline);
    r.ctor("trajectory", c_trajectory);
    r.ctor("deriv", c_deriv);
    r.ctor("accum", c_accum);
    r.ctor("extrema", c_extrema);
    r.ctor("inflections", c_inflections);
    r.ctor("band", c_band);
    r.ctor("taylor", c_taylor);
    r.ctor("limit", c_limit);
    r.ctor("vector", c_vector);
    r.ctor("numberline", c_numberline);
    r.ctor("arc", c_arc);
    r.ctor("sector", c_sector);
    r.ctor("annulus", c_annulus);
    r.ctor("pie", c_pie);
    r.ctor("arrowfield", c_arrowfield);
    r.ctor("vectorfield", c_arrowfield);
    r.ctor("linmap", c_linmap);
    r.ctor("determinant", c_determinant);
    r.ctor("eigen", c_eigen);
    r.ctor("linsolve", c_linsolve);
    r.ctor("span", c_span);
    r.ctor("diagonalise", c_diagonalise);
    r.ctor("diagonalize", c_diagonalise);
    r.ctor("rref", c_rref);
    r.ctor("project", c_project);
    r.ctor("leastsquares", c_leastsquares);
    r.ctor("matrix", c_matrix);
    r.ctor("table", c_table);
    r.ctor("mathtable", c_table);
    r.ctor("decimaltable", c_table);
    r.ctor("integertable", c_table);
}

#[cfg(test)]
mod graph_tests {
    use super::expr::compile;
    use crate::primitives::{GraphFn, GraphSrc, GraphView};
    use macroquad::math::Vec2;

    fn graph(src: &str) -> GraphFn {
        GraphFn {
            src: GraphSrc::Expr(compile(src).unwrap()),
            center: Vec2::ZERO,
            sx: 100.0,
            sy: 100.0,
            x0: 0.0,
            x1: 6.3,
        }
    }

    #[test]
    fn determinant_and_eigen_match_linear_algebra() {
        use super::{det2, eig2};
        assert!((det2(1.0, 2.0, 3.0, 4.0) + 2.0).abs() < 1e-5); // 1*4 - 2*3 = -2
        // symmetric [[2,1],[1,2]]: eigenvalues 3 and 1, eigenvectors (1,1) & (1,-1)
        let e = eig2(2.0, 1.0, 1.0, 2.0);
        assert_eq!(e.len(), 2);
        assert!(e.iter().any(|(l, _)| (l - 3.0).abs() < 1e-3));
        assert!(e.iter().any(|(l, _)| (l - 1.0).abs() < 1e-3));
        // the λ=3 eigenvector points along (1,1)
        let (_, v3) = e.iter().find(|(l, _)| (l - 3.0).abs() < 1e-3).unwrap();
        assert!((v3.x - v3.y).abs() < 1e-3);
        // a 90° rotation [[0,-1],[1,0]] has no real eigenvectors
        assert!(eig2(0.0, -1.0, 1.0, 0.0).is_empty());
        // a shear [[1,1],[0,1]] preserves area (det 1) and fixes the x-axis
        assert!((det2(1.0, 1.0, 0.0, 1.0) - 1.0).abs() < 1e-5);
        let sh = eig2(1.0, 1.0, 0.0, 1.0);
        assert!(sh.iter().all(|(_, v)| v.y.abs() < 1e-3)); // eigenvector on the x-axis
    }

    #[test]
    fn diagonalise_eigenpairs_only_stretch() {
        use super::eig2;
        // the property `diagonalise` draws: A·e = λ·e (each eigenvector only scales)
        let (a, b, c, d) = (2.0, 1.0, 1.0, 2.0);
        let pairs = eig2(a, b, c, d);
        assert_eq!(pairs.len(), 2);
        for (l, e) in pairs {
            let (aex, aey) = (a * e.x + b * e.y, c * e.x + d * e.y); // A·e
            assert!((aex - l * e.x).abs() < 1e-3 && (aey - l * e.y).abs() < 1e-3);
        }
        // a pure rotation has no real eigenbasis → diagonalise draws its note
        assert!(eig2(0.0, -1.0, 1.0, 0.0).is_empty());
    }

    #[test]
    fn rref_reduces_to_identity_and_solution() {
        use super::rref_steps;
        // [2 1 | 5 ; 1 3 | 10]  (the system 2x+y=5, x+3y=10) reduces to
        // [1 0 | 1 ; 0 1 | 3] — identity on the left, solution (1,3) on the right.
        let steps = rref_steps(vec![vec![2.0, 1.0, 5.0], vec![1.0, 3.0, 10.0]]);
        let (last, _) = steps.last().unwrap();
        assert!((last[0][0] - 1.0).abs() < 1e-4 && last[0][1].abs() < 1e-4 && (last[0][2] - 1.0).abs() < 1e-4);
        assert!(last[1][0].abs() < 1e-4 && (last[1][1] - 1.0).abs() < 1e-4 && (last[1][2] - 3.0).abs() < 1e-4);
        assert!(steps.len() >= 4); // start + real row operations, not a no-op
        // a singular left block leaves a row of zeros (no unique solution)
        let sing = rref_steps(vec![vec![1.0, 2.0, 3.0], vec![2.0, 4.0, 7.0]]);
        let (ls, _) = sing.last().unwrap();
        assert!(ls[1][0].abs() < 1e-4 && ls[1][1].abs() < 1e-4); // bottom row's A-part is zero
    }

    #[test]
    fn projection_residual_is_perpendicular_to_the_subspace() {
        // p = (b·a / a·a) a; the residual b - p must be orthogonal to a
        let (b, av) = (Vec2::new(1.0, 3.0), Vec2::new(3.0, 1.0));
        let t = (b.x * av.x + b.y * av.y) / (av.x * av.x + av.y * av.y);
        let p = Vec2::new(t * av.x, t * av.y);
        let e = b - p;
        assert!((e.x * av.x + e.y * av.y).abs() < 1e-5, "residual not perpendicular");
        // a vector already on the line projects to itself
        let on = Vec2::new(6.0, 2.0); // = 2·a
        let t2 = (on.x * av.x + on.y * av.y) / (av.x * av.x + av.y * av.y);
        assert!((t2 * av.x - on.x).abs() < 1e-4 && (t2 * av.y - on.y).abs() < 1e-4);
    }

    #[test]
    fn leastsquares_recovers_a_known_line() {
        use super::fit_line;
        // points exactly on y = 2x + 1 → the fit recovers m = 2, k = 1
        let pts = [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 3.0),
            Vec2::new(2.0, 5.0),
            Vec2::new(3.0, 7.0),
        ];
        let (m, k) = fit_line(&pts).unwrap();
        assert!((m - 2.0).abs() < 1e-4 && (k - 1.0).abs() < 1e-4);
        // a vertical cloud has no y = m x + k fit
        assert!(fit_line(&[Vec2::new(2.0, 0.0), Vec2::new(2.0, 5.0)]).is_none());
    }

    #[test]
    fn linsolve_intersection_solves_the_system() {
        use super::{line_eq_pts, solve2};
        // [[2,1],[1,3]] x = (5,10): unique solution (1, 3)
        let (x, y) = solve2(2.0, 1.0, 1.0, 3.0, 5.0, 10.0).unwrap();
        assert!((x - 1.0).abs() < 1e-4 && (y - 3.0).abs() < 1e-4);
        // the solution satisfies both equations
        assert!((2.0 * x + 1.0 * y - 5.0).abs() < 1e-4);
        assert!((1.0 * x + 3.0 * y - 10.0).abs() < 1e-4);
        // parallel rows (det 0) → no unique solution
        assert!(solve2(1.0, 2.0, 2.0, 4.0, 3.0, 9.0).is_none());
        // both endpoints of a drawn row lie on that row's line: 2x + 1y = 5
        let (p0, p1) = line_eq_pts(2.0, 1.0, 5.0, 5.0).unwrap();
        assert!((2.0 * p0.x + p0.y - 5.0).abs() < 1e-3);
        assert!((2.0 * p1.x + p1.y - 5.0).abs() < 1e-3);
    }

    #[test]
    fn named_functions_resolve_and_evaluate() {
        use super::named_formula;
        assert!(named_formula("sinc").is_some() && named_formula("relu").is_some());
        assert!(named_formula("definitely-not-a-fn").is_none());
        // dropped: rcos/rsin (collide with the r*cos/r*sin glued-name typo)
        assert!(named_formula("rcos").is_none() && named_formula("rsin").is_none());
        let f = |n: &str| compile(named_formula(n).unwrap()).unwrap();
        assert!(f("relu").eval(-2.0, 0.0).abs() < 1e-4 && (f("relu").eval(3.0, 0.0) - 3.0).abs() < 1e-4);
        assert!((f("sigmoid").eval(0.0, 0.0) - 0.5).abs() < 1e-4);
        assert!((f("step").eval(2.0, 0.0) - 1.0).abs() < 1e-4 && f("step").eval(-2.0, 0.0).abs() < 1e-4);
    }

    /// The editor's `NAMED_FNS` vocab (manic-lang catalog) must equal exactly the
    /// engine's `named_formula` names — otherwise the browser would flag a valid
    /// bareword (false error) or miss an invalid one (Render wrongly enabled, the
    /// `acos` drift). Also asserts every formula compiles + evaluates.
    #[test]
    fn catalog_named_fns_match_engine() {
        use super::{named_formula, NAMED_FORMULAS};
        use std::collections::BTreeSet;
        let engine: BTreeSet<&str> = NAMED_FORMULAS.iter().map(|(n, _)| *n).collect();
        let cat: BTreeSet<&str> = manic_lang::catalog::NAMED_FNS.iter().copied().collect();
        assert_eq!(
            engine, cat,
            "named-fn vocab drift — only in engine: {:?}; only in catalog: {:?}",
            engine.difference(&cat).collect::<Vec<_>>(),
            cat.difference(&engine).collect::<Vec<_>>(),
        );
        for (n, formula) in NAMED_FORMULAS {
            let g = compile(formula).unwrap_or_else(|e| panic!("{n}: `{formula}` bad: {e}"));
            assert!(g.eval(0.5, 0.0).is_finite(), "{n} did not evaluate finitely");
        }
        // the newly-added inverse trig is correct
        assert!(compile(named_formula("acos").unwrap()).unwrap().eval(1.0, 0.0).abs() < 1e-4); // acos 1 = 0
        assert!(compile(named_formula("asin").unwrap()).unwrap().eval(0.0, 0.0).abs() < 1e-4); // asin 0 = 0
    }

    #[test]
    fn slope_matches_calculus() {
        let g = graph("sin(x)");
        // d/dx sin = cos: slope at 0 ≈ 1, at π/2 ≈ 0, at π ≈ -1
        assert!((g.slope(0.0) - 1.0).abs() < 1e-2, "sin' (0) = {}", g.slope(0.0));
        assert!(g.slope(std::f32::consts::FRAC_PI_2).abs() < 1e-2);
        assert!((g.slope(std::f32::consts::PI) + 1.0).abs() < 1e-2);
    }

    #[test]
    fn tangent_is_flat_at_a_peak() {
        // at the peak x = π/2 the tangent segment is horizontal: endpoints share y
        let gv = GraphView::Tangent {
            graph: graph("sin(x)"),
            x: std::f32::consts::FRAC_PI_2,
            half: 80.0,
        };
        let (a, b) = gv.segment().unwrap();
        assert!((a.y - b.y).abs() < 0.5, "peak tangent not flat: {a:?} {b:?}");
        assert!((b.x - a.x - 160.0).abs() < 1.0, "segment length wrong");
    }

    #[test]
    fn normal_is_perpendicular_to_tangent() {
        let g = graph("x*x"); // slope 2 at x=1 (screen dir not axis-aligned)
        let t = GraphView::Tangent { graph: g.clone(), x: 1.0, half: 50.0 };
        let n = GraphView::Normal { graph: g, x: 1.0, half: 50.0 };
        let (ta, tb) = t.segment().unwrap();
        let (na, nb) = n.segment().unwrap();
        let dt = tb - ta;
        let dn = nb - na;
        assert!(dt.dot(dn).abs() < 1.0, "normal not ⟂ tangent: dot = {}", dt.dot(dn));
    }

    #[test]
    fn undefined_slope_draws_no_line() {
        // 1/x has an asymptote at 0: the segment collapses to the touch point
        let gv = GraphView::Tangent { graph: graph("1/x"), x: 0.0, half: 80.0 };
        let (a, b) = gv.segment().unwrap();
        assert_eq!(a, b, "a fake tangent was drawn at an asymptote");
    }

    #[test]
    fn area_integral_matches_calculus() {
        // ∫₀² x² dx = 8/3 ≈ 2.667
        let gv = GraphView::Area { graph: graph("x*x"), a: 0.0, x: 2.0, n: 100 };
        assert!((gv.value() - 8.0 / 3.0).abs() < 1e-2, "∫x² = {}", gv.value());
        // ∫₀^π sin = 2
        let gv = GraphView::Area { graph: graph("sin(x)"), a: 0.0, x: std::f32::consts::PI, n: 100 };
        assert!((gv.value() - 2.0).abs() < 1e-2, "∫sin = {}", gv.value());
    }

    #[test]
    fn slope_readout_tracks_the_curve() {
        let gv = GraphView::Slope { graph: graph("sin(x)"), x: 0.0, off: Vec2::ZERO };
        assert!((gv.value() - 1.0).abs() < 1e-2); // cos(0) = 1
    }

    // build a numerically-sampled GraphFn like `deriv`/`accum` do
    fn sampled(f: impl Fn(f32) -> f32, x0: f32, x1: f32) -> GraphFn {
        let n = 240;
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        for i in 0..=n {
            let x = x0 + (x1 - x0) * i as f32 / n as f32;
            xs.push(x);
            ys.push(f(x));
        }
        GraphFn { src: GraphSrc::Samples { xs, ys }, center: Vec2::ZERO, sx: 100.0, sy: 100.0, x0, x1 }
    }

    #[test]
    fn deriv_curve_is_the_derivative() {
        // deriv of sin sampled ≈ cos
        let g = graph("sin(x)");
        let d = sampled(|x| g.slope(x), 0.0, 6.3); // what c_deriv builds
        assert!((d.y(1.0) - 1f32.cos()).abs() < 1e-2, "deriv(sin)(1) = {}", d.y(1.0));
        assert!((d.y(3.0) - 3f32.cos()).abs() < 1e-2);
    }

    #[test]
    fn fundamental_theorem_slope_of_accum_recovers_f() {
        // F(x) = ∫₀ˣ sin ; by the FTC, F'(x) = sin(x). Slope of the sampled
        // accumulation curve must return f.
        let g = graph("sin(x)");
        let acc = sampled(|x| crate::primitives::integrate(&g, 0.0, x, 48), 0.0, 6.3);
        for &x in &[0.7f32, 1.5, 2.5, 4.0] {
            assert!((acc.slope(x) - x.sin()).abs() < 2e-2, "F'({x}) = {}, want sin = {}", acc.slope(x), x.sin());
        }
    }

    #[test]
    fn integral_readout_matches_area() {
        // ∫₀² x² dx = 8/3 — the readout value equals the swept integral
        let gv = GraphView::Integral { graph: graph("x*x"), a: 0.0, x: 2.0, n: 80, at: Vec2::ZERO };
        assert!((gv.value() - 8.0 / 3.0).abs() < 1e-2, "∫x² readout = {}", gv.value());
    }

    #[test]
    fn extrema_are_zeros_of_the_slope() {
        // sin over (0, 2π): max at π/2, min at 3π/2 — zeros of the derivative
        let g = graph("sin(x)");
        let crit = super::sampled_zeros(&g, |x| g.slope(x));
        assert!(crit.iter().any(|&x| (x - std::f32::consts::FRAC_PI_2).abs() < 2e-2), "no max: {crit:?}");
        assert!(crit.iter().any(|&x| (x - 3.0 * std::f32::consts::FRAC_PI_2).abs() < 2e-2), "no min: {crit:?}");
    }

    #[test]
    fn inflections_are_zeros_of_the_second_derivative() {
        // sin'' = -sin, zero at π (interior of 0..2π)
        let g = graph("sin(x)");
        let infl = super::sampled_zeros(&g, |x| g.second(x));
        assert!(infl.iter().any(|&x| (x - std::f32::consts::PI).abs() < 3e-2), "no inflection at π: {infl:?}");
    }

    #[test]
    fn taylor_coefficients_match_sin() {
        // sin about 0: c0=0, c1=1, c2=0, c3=-1/6, c5=1/120
        let g = graph("sin(x)");
        let c = |k| g.nth_deriv(0.0, k);
        assert!(c(1).abs() > 0.97 && c(1) < 1.03, "f'(0) = {}", c(1)); // 1
        assert!((c(3) / 6.0 + 1.0 / 6.0).abs() < 3e-2, "c3 = {}", c(3) / 6.0); // -1/6
        // and the degree-5 polynomial at x=1 ≈ sin(1)
        let mut fact = 1.0f32;
        let mut p = 0.0f32;
        for k in 0..=5u32 {
            if k > 0 { fact *= k as f32; }
            p += g.nth_deriv(0.0, k) / fact * 1f32.powi(k as i32);
        }
        assert!((p - 1f32.sin()).abs() < 2e-2, "P5(1) = {p}, sin 1 = {}", 1f32.sin());
    }

    #[test]
    fn limit_at_infinity_finds_the_asymptote() {
        // (5x^3-2x+7)/(x^3+4x^2+3) -> 5 as x -> inf: sample far out
        let g = graph("(5*x*x*x - 2*x + 7)/(x*x*x + 4*x*x + 3)");
        assert!((g.y(1e5) - 5.0).abs() < 1e-2, "asymptote = {}", g.y(1e5));
    }

    #[test]
    fn limit_of_removable_hole_is_finite() {
        // lim(x->0) sin(x)/x = 1, even though f(0) is 0/0
        let g = graph("sin(x)/x");
        let eps = 1e-3;
        let l = 0.5 * (g.y(-eps) + g.y(eps));
        assert!((l - 1.0).abs() < 1e-2, "lim sin(x)/x = {l}");
        assert!(!g.y(0.0).is_finite(), "f(0) should be 0/0 = non-finite");
    }

    #[test]
    fn roots_finds_zero_crossings() {
        // sin over (0, 6.3): zeros at 0…, π, 2π — π and 2π are interior crossings
        let rs = graph("sin(x)").roots();
        assert!(rs.iter().any(|&r| (r - std::f32::consts::PI).abs() < 1e-2), "no π root: {rs:?}");
        assert!(rs.iter().any(|&r| (r - std::f32::consts::TAU).abs() < 1e-2), "no 2π root: {rs:?}");
    }

    #[test]
    fn newton_converges_to_a_root() {
        // x² − 2 from x0 = 3 → √2 ≈ 1.4142; last curve point maps back to it
        let g = graph("x*x - 2");
        let pts = g.newton_path(3.0, 20);
        let last = pts.last().unwrap();
        let x = (last.x - g.center.x) / g.sx; // screen → math
        assert!((x - 2f32.sqrt()).abs() < 1e-2, "Newton landed at x = {x}, want √2");
    }

    #[test]
    fn spline_passes_through_its_knots() {
        let knots = [
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 80.0),
            Vec2::new(220.0, -30.0),
            Vec2::new(300.0, 50.0),
        ];
        let curve = super::catmull_rom(&knots, 16);
        // each knot appears exactly at a segment boundary (i*seg)
        for (i, k) in knots.iter().enumerate() {
            let at = &curve[(i * 16).min(curve.len() - 1)];
            assert!(at.distance(*k) < 1e-2, "knot {i} missed: {at:?} vs {k:?}");
        }
    }

    #[test]
    fn trajectory_orbit_stays_on_its_circle() {
        // dx/dt = -y, dy/dt = x is a pure rotation: |(x,y)| is conserved
        let fx = compile("-y").unwrap();
        let fy = compile("x").unwrap();
        let path = super::rk4_path(&fx, &fy, 2.0, 0.0, 0.02, 400);
        for &(x, y) in &path {
            assert!(((x * x + y * y).sqrt() - 2.0).abs() < 1e-2, "left the r=2 orbit at ({x},{y})");
        }
    }
}
