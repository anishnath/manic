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
use crate::primitives::{Entity, FontKind, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;

/// Evaluate a named function at `x`. Returns `None` for names we don't know
/// and non-finite results (asymptotes), so the caller can break the polyline.
fn eval(name: &str, x: f32) -> Option<f32> {
    let y = match name {
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "parabola" | "sq" | "square" => x * x,
        "cubic" | "cube" => x * x * x,
        "line" | "id" | "identity" => x,
        "abs" => x.abs(),
        "exp" => x.exp(),
        "sqrt" if x >= 0.0 => x.sqrt(),
        "log" | "ln" if x > 0.0 => x.ln(),
        "recip" | "inv" if x.abs() > 1e-3 => 1.0 / x,
        "gauss" | "bell" => (-x * x).exp(),
        _ => return None,
    };
    y.is_finite().then_some(y)
}

fn known_fn(name: &str) -> bool {
    matches!(
        name,
        "sin" | "cos" | "tan" | "parabola" | "sq" | "square" | "cubic" | "cube" | "line" | "id"
            | "identity" | "abs" | "exp" | "sqrt" | "log" | "ln" | "recip" | "inv" | "gauss"
            | "bell"
    )
}

/// A tiny single-variable expression evaluator, so `plot` can take a formula
/// string like `"cos(t) + 0.5*cos(7*t) + (1/7)*cos(14*t)"` — manic's answer to
/// Manim's `FunctionGraph(lambda t: ...)`. The variable is `x` (alias `t`);
/// constants `pi`, `e`, `tau`; operators `+ - * / ^` (unary `-`); functions
/// sin/cos/tan/asin/acos/atan/sinh/cosh/tanh/exp/ln/log/log10/log2/sqrt/abs/
/// floor/ceil/round/sign. Compiled once to a tree, then sampled per point.
/// This is deliberately NOT the language's (still-deferred) variable/loop
/// layer — it is a leaf evaluator scoped to a single plotted curve.
mod expr {
    pub enum Node {
        Num(f32),
        Var,
        Neg(Box<Node>),
        Add(Box<Node>, Box<Node>),
        Sub(Box<Node>, Box<Node>),
        Mul(Box<Node>, Box<Node>),
        Div(Box<Node>, Box<Node>),
        Pow(Box<Node>, Box<Node>),
        Call(fn(f32) -> f32, Box<Node>),
    }

    impl Node {
        pub fn eval(&self, x: f32) -> f32 {
            match self {
                Node::Num(n) => *n,
                Node::Var => x,
                Node::Neg(a) => -a.eval(x),
                Node::Add(a, b) => a.eval(x) + b.eval(x),
                Node::Sub(a, b) => a.eval(x) - b.eval(x),
                Node::Mul(a, b) => a.eval(x) * b.eval(x),
                Node::Div(a, b) => a.eval(x) / b.eval(x),
                Node::Pow(a, b) => a.eval(x).powf(b.eval(x)),
                Node::Call(f, a) => f(a.eval(x)),
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
            let id = std::str::from_utf8(&self.s[start..self.i]).unwrap().to_string();
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
                "x" | "t" => Ok(Node::Var),
                "pi" => Ok(Node::Num(std::f32::consts::PI)),
                "e" => Ok(Node::Num(std::f32::consts::E)),
                "tau" => Ok(Node::Num(std::f32::consts::TAU)),
                _ => Err(format!(
                    "unknown name `{id}` (use x/t, pi, e, tau, or a function)"
                )),
            }
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

    // arg 4 is either a "formula string" or a bare named-function word.
    enum F {
        Named(String),
        Expr(expr::Node),
    }
    let f = if let Ok(src) = a.text(4) {
        expr::compile(&src)
            .map(F::Expr)
            .map_err(|m| Error::new(format!("in plot formula: {m}"), a.span_of(4)))?
    } else {
        let name = a.ident(4)?;
        if !known_fn(&name) {
            return Err(Error::new(
                format!(
                    "unknown function `{name}` — use a named one (sin, cos, tan, parabola, cubic, line, abs, exp, sqrt, log, recip, gauss) or a \"formula\" like \"cos(x)+0.5*sin(3*x)\""
                ),
                a.span_of(4),
            ));
        }
        F::Named(name)
    };
    let sample = |x: f32| -> Option<f32> {
        match &f {
            F::Named(n) => eval(n, x),
            F::Expr(node) => node.eval(x).is_finite().then(|| node.eval(x)),
        }
    };

    const N: usize = 600;
    let mut pts = Vec::with_capacity(N + 1);
    for i in 0..=N {
        let x = x0 + (x1 - x0) * i as f32 / N as f32;
        if let Some(y) = sample(x) {
            pts.push(Vec2::new(c.x + x * sx, c.y - y * sy));
        }
    }
    let mut e = Entity::new(id.clone(), Shape::Polyline { pts }, Vec2::ZERO, style::CYAN);
    e.stroke = stroked(style::CYAN, 3.0);
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
    let mut e = Entity::new(id, Shape::Arc { r, inner: 0.0, start, sweep }, c, style::CYAN);
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
    let mut e = Entity::new(id, Shape::Arc { r, inner, start, sweep }, c, style::PANEL);
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
        return Err(Error::new(
            "matrix rows must all have the same number of entries",
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
    let col_labels = if a.len() > 5 { tokens(&a.text(5)?) } else { vec![] };
    let row_labels = if a.len() > 6 { tokens(&a.text(6)?) } else { vec![] };

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
        return Err(Error::new(
            "table rows must all have the same number of entries",
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
    let cell = |r: usize, col: usize| {
        Vec2::new(x0 + (col as f32 + 0.5) * cw, y0 + (r as f32 + 0.5) * ch)
    };
    let txt = |s: &mut Scene, id: String, content: String, pos: Vec2, color: Color, tags: Vec<String>| {
        let mut e = Entity::new(id, Shape::Text { content, size: 26.0 }, pos, color);
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

/// Register the math kit into `r`.
pub fn register(r: &mut Registry) {
    r.ctor("axes", c_axes);
    r.ctor("plane", c_plane);
    r.ctor("numberplane", c_plane);
    r.ctor("complexplane", c_complexplane);
    r.ctor("polarplane", c_polarplane);
    r.ctor("plot", c_plot);
    r.ctor("vector", c_vector);
    r.ctor("numberline", c_numberline);
    r.ctor("arc", c_arc);
    r.ctor("sector", c_sector);
    r.ctor("annulus", c_annulus);
    r.ctor("pie", c_pie);
    r.ctor("arrowfield", c_arrowfield);
    r.ctor("vectorfield", c_arrowfield);
    r.ctor("matrix", c_matrix);
    r.ctor("table", c_table);
    r.ctor("mathtable", c_table);
    r.ctor("decimaltable", c_table);
    r.ctor("integertable", c_table);
}
