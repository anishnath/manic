//! Recursive-descent parser: tokens → [`Program`].
//!
//! Grammar:
//!
//! ```text
//! program := stmt* EOF
//! stmt    := "let" IDENT "=" expr ";"
//!          | "for" IDENT "in" expr ".." expr block
//!          | IDENT arglist? ( block | ";" )
//! arglist := "(" ( expr ("," expr)* )? ")"
//! block   := "{" stmt* "}"
//! expr    := add
//! add     := mul (("+"|"-") mul)*
//! mul     := pow (("*"|"/") pow)*
//! pow     := unary ("^" pow)?          // right-associative
//! unary   := "-" unary | atom
//! atom    := NUM | STR
//!          | "(" expr ("," expr)? ")"  // grouping, or an (x,y) pair
//!          | IDENT "(" expr ")"        // math function call
//!          | IDENT template            // interpolated id: bar{i}
//!          | IDENT                     // plain name / variable
//! ```
//!
//! The parser validates *shape* only — it never checks whether a call name is
//! a known builtin, a color, or a bound variable. Meaning (and evaluation of
//! `let`/`for`/arithmetic) is the lowering pass's job, so this stays generic.

use super::ast::{BinOp, Ctrl, Expr, ExprKind, Program, ReduceOp, Seg, Stmt};
use super::diag::{Error, Span};
use super::lexer::{lex, Tok, Token};

/// Parse manic source into a [`Program`] (lexes first).
pub fn parse(src: &str) -> Result<Program, Error> {
    let toks = lex(src)?;
    let mut p = Parser { toks, pos: 0 };
    let stmts = p.program()?;
    Ok(Program { stmts })
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek_tok(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn span(&self) -> Span {
        self.toks[self.pos].span
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn program(&mut self) -> Result<Vec<Stmt>, Error> {
        let mut stmts = Vec::new();
        while self.peek_tok() != &Tok::Eof {
            stmts.push(self.stmt()?);
        }
        Ok(stmts)
    }

    fn stmt(&mut self) -> Result<Stmt, Error> {
        // name (or a `let` / `for` keyword — plain idents at this position)
        let (name, name_span) = match self.peek_tok().clone() {
            Tok::Ident(s) => {
                let sp = self.span();
                self.bump();
                (s, sp)
            }
            Tok::RBrace => {
                return Err(Error::new("unexpected `}` (no matching `{`)", self.span()))
            }
            other => {
                return Err(Error::new(
                    format!("expected a statement (a name like `move` or `circle`), found {}", describe(&other)),
                    self.span(),
                ))
            }
        };

        match name.as_str() {
            "let" => return self.let_stmt(name_span),
            "for" => return self.for_stmt(name_span),
            "def" => return self.def_stmt(name_span),
            "if" => return self.if_stmt(name_span),
            _ => {}
        }

        // optional arglist
        let args = if self.peek_tok() == &Tok::LParen {
            self.arglist()?
        } else {
            Vec::new()
        };

        // terminator: `{ block }` or `;`
        match self.peek_tok() {
            Tok::LBrace => {
                let block = self.block()?;
                Ok(Stmt {
                    name,
                    name_span,
                    args,
                    block: Some(block),
                    ctrl: None,
                })
            }
            Tok::Semi => {
                self.bump();
                Ok(Stmt {
                    name,
                    name_span,
                    args,
                    block: None,
                    ctrl: None,
                })
            }
            other => Err(Error::new(
                format!(
                    "expected `;` or `{{` after `{name}(...)`, found {}",
                    describe(other)
                ),
                self.span(),
            )),
        }
    }

    /// `let IDENT = expr ;`
    fn let_stmt(&mut self, name_span: Span) -> Result<Stmt, Error> {
        let var = self.ident_name("a variable name after `let`")?;
        self.expect_msg(&Tok::Eq, "expected `=` in `let name = value;`")?;
        let value = self.expr()?;
        self.expect_msg(&Tok::Semi, "expected `;` after `let name = value`")?;
        Ok(Stmt {
            name: "let".into(),
            name_span,
            args: Vec::new(),
            block: None,
            ctrl: Some(Ctrl::Let(var, value)),
        })
    }

    /// `for IDENT in expr .. expr { block }`
    fn for_stmt(&mut self, name_span: Span) -> Result<Stmt, Error> {
        let var = self.ident_name("a loop variable after `for`")?;
        match self.ident_name("`in`") {
            Ok(kw) if kw == "in" => {}
            _ => return Err(Error::new("expected `in` in `for i in a..b { }`", self.span())),
        }
        let start = self.expr()?;
        self.expect_msg(&Tok::DotDot, "expected `..` in the range of `for i in a..b`")?;
        let end = self.expr()?;
        let body = self.block()?;
        Ok(Stmt {
            name: "for".into(),
            name_span,
            args: Vec::new(),
            block: None,
            ctrl: Some(Ctrl::For {
                var,
                start,
                end,
                body,
            }),
        })
    }

    /// `def IDENT ( params ) { block }`
    fn def_stmt(&mut self, name_span: Span) -> Result<Stmt, Error> {
        let name = self.ident_name("a macro name after `def`")?;
        self.expect_msg(&Tok::LParen, "expected `(` after the macro name")?;
        let mut params = Vec::new();
        if self.peek_tok() != &Tok::RParen {
            loop {
                params.push(self.ident_name("a parameter name")?);
                match self.peek_tok() {
                    Tok::Comma => {
                        self.bump();
                    }
                    Tok::RParen => break,
                    other => {
                        return Err(Error::new(
                            format!("expected `,` or `)` in `def` parameters, found {}", describe(other)),
                            self.span(),
                        ))
                    }
                }
            }
        }
        self.expect(&Tok::RParen)?;
        let body = self.block()?;
        Ok(Stmt {
            name: "def".into(),
            name_span,
            args: Vec::new(),
            block: None,
            ctrl: Some(Ctrl::Def { name, params, body }),
        })
    }

    /// `if expr { block } [ else ( { block } | if ... ) ]`
    fn if_stmt(&mut self, name_span: Span) -> Result<Stmt, Error> {
        let cond = self.expr()?;
        let then_body = self.block()?;
        let else_body = if matches!(self.peek_tok(), Tok::Ident(s) if s == "else") {
            self.bump(); // `else`
            if matches!(self.peek_tok(), Tok::Ident(s) if s == "if") {
                // `else if …` — nest one if-statement as the else body
                let sp = self.span();
                self.bump();
                Some(vec![self.if_stmt(sp)?])
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Ok(Stmt {
            name: "if".into(),
            name_span,
            args: Vec::new(),
            block: None,
            ctrl: Some(Ctrl::If {
                cond,
                then_body,
                else_body,
            }),
        })
    }

    fn ident_name(&mut self, what: &str) -> Result<String, Error> {
        match self.peek_tok().clone() {
            Tok::Ident(s) => {
                self.bump();
                Ok(s)
            }
            other => Err(Error::new(
                format!("expected {what}, found {}", describe(&other)),
                self.span(),
            )),
        }
    }

    fn arglist(&mut self) -> Result<Vec<Expr>, Error> {
        self.expect(&Tok::LParen)?;
        let mut args = Vec::new();
        if self.peek_tok() == &Tok::RParen {
            self.bump();
            return Ok(args);
        }
        loop {
            args.push(self.expr()?);
            match self.peek_tok() {
                Tok::Comma => {
                    self.bump();
                }
                Tok::RParen => {
                    self.bump();
                    break;
                }
                other => {
                    return Err(Error::new(
                        format!("expected `,` or `)` in argument list, found {}", describe(other)),
                        self.span(),
                    ))
                }
            }
        }
        Ok(args)
    }

    fn block(&mut self) -> Result<Vec<Stmt>, Error> {
        self.expect(&Tok::LBrace)?;
        let mut stmts = Vec::new();
        while self.peek_tok() != &Tok::RBrace {
            if self.peek_tok() == &Tok::Eof {
                return Err(Error::new("unterminated `{ ... }` block", self.span()));
            }
            stmts.push(self.stmt()?);
        }
        self.bump(); // consume `}`
        Ok(stmts)
    }

    // ---- expressions ------------------------------------------------------

    fn expr(&mut self) -> Result<Expr, Error> {
        self.or()
    }

    fn or(&mut self) -> Result<Expr, Error> {
        let mut lhs = self.and()?;
        while self.peek_tok() == &Tok::OrOr {
            self.bump();
            let rhs = self.and()?;
            lhs = self.bin(BinOp::Or, lhs, rhs);
        }
        Ok(lhs)
    }

    fn and(&mut self) -> Result<Expr, Error> {
        let mut lhs = self.cmp()?;
        while self.peek_tok() == &Tok::AndAnd {
            self.bump();
            let rhs = self.cmp()?;
            lhs = self.bin(BinOp::And, lhs, rhs);
        }
        Ok(lhs)
    }

    fn cmp(&mut self) -> Result<Expr, Error> {
        let mut lhs = self.add()?;
        loop {
            let op = match self.peek_tok() {
                Tok::Lt => BinOp::Lt,
                Tok::Le => BinOp::Le,
                Tok::Gt => BinOp::Gt,
                Tok::Ge => BinOp::Ge,
                Tok::EqEq => BinOp::Eq,
                Tok::Ne => BinOp::Ne,
                _ => break,
            };
            self.bump();
            let rhs = self.add()?;
            lhs = self.bin(op, lhs, rhs);
        }
        Ok(lhs)
    }

    fn add(&mut self) -> Result<Expr, Error> {
        let mut lhs = self.mul()?;
        loop {
            let op = match self.peek_tok() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let rhs = self.mul()?;
            lhs = self.bin(op, lhs, rhs);
        }
        Ok(lhs)
    }

    fn mul(&mut self) -> Result<Expr, Error> {
        let mut lhs = self.pow()?;
        loop {
            let op = match self.peek_tok() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                _ => break,
            };
            self.bump();
            let rhs = self.pow()?;
            lhs = self.bin(op, lhs, rhs);
        }
        Ok(lhs)
    }

    fn pow(&mut self) -> Result<Expr, Error> {
        let base = self.unary()?;
        if self.peek_tok() == &Tok::Caret {
            self.bump();
            let exp = self.pow()?; // right-associative
            Ok(self.bin(BinOp::Pow, base, exp))
        } else {
            Ok(base)
        }
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        if self.peek_tok() == &Tok::Minus {
            let span = self.span();
            self.bump();
            let inner = self.unary()?;
            return Ok(Expr {
                kind: ExprKind::Neg(Box::new(inner)),
                span,
            });
        }
        self.atom()
    }

    fn atom(&mut self) -> Result<Expr, Error> {
        let span = self.span();
        match self.peek_tok().clone() {
            Tok::Num(n) => {
                self.bump();
                Ok(Expr {
                    kind: ExprKind::Num(n),
                    span,
                })
            }
            Tok::Str(s) => {
                self.bump();
                Ok(Expr {
                    kind: ExprKind::Str(s),
                    span,
                })
            }
            Tok::LParen => self.paren_or_pair(),
            Tok::Ident(s) => {
                self.bump();
                self.after_ident(s, span)
            }
            other => Err(Error::new(
                format!(
                    "expected a value (number, \"string\", name, or `(x,y)`), found {}",
                    describe(&other)
                ),
                span,
            )),
        }
    }

    /// Is the *upcoming* token glued directly onto `prev` (no whitespace)? Used
    /// so `bar{i}` interpolates but `n {` (a range end before a block) does not.
    fn glued(&self, prev: Span) -> bool {
        let sp = self.span();
        sp.line == prev.line && sp.col == prev.col + prev.len
    }

    /// After a leading identifier: a glued math call `f(x)`, a glued
    /// interpolated id `bar{i}`, or a plain name / variable.
    fn after_ident(&mut self, first: String, span: Span) -> Result<Expr, Error> {
        // reduction: sum/prod/min/max ( VAR in EXPR .. EXPR : EXPR )
        if let Some(op) = reduce_op(&first) {
            if self.peek_tok() == &Tok::LParen && self.glued(span) {
                return self.reduction(op, span);
            }
        }
        // math function call: IDENT "(" expr ")" — must be glued (`sin(x)`)
        if self.peek_tok() == &Tok::LParen && self.glued(span) {
            self.bump();
            let arg = self.expr()?;
            self.expect_msg(&Tok::RParen, "expected `)` to close a function call")?;
            return Ok(Expr {
                kind: ExprKind::Call(first, Box::new(arg)),
                span,
            });
        }
        // interpolation: glued `{expr}` or glued ident chunks continue the id
        let mut segs = vec![Seg::Lit(first.clone())];
        let mut prev = span;
        loop {
            if !self.glued(prev) {
                break;
            }
            match self.peek_tok().clone() {
                Tok::LBrace => {
                    self.bump();
                    let e = self.expr()?;
                    let rb = self.span();
                    self.expect_msg(&Tok::RBrace, "expected `}` to close `{...}` in an id")?;
                    segs.push(Seg::Ex(Box::new(e)));
                    prev = rb;
                }
                Tok::Ident(s) => {
                    let isp = self.span();
                    self.bump();
                    segs.push(Seg::Lit(s));
                    prev = isp;
                }
                _ => break,
            }
        }
        let kind = if segs.len() == 1 {
            ExprKind::Ident(first)
        } else {
            ExprKind::Interp(segs)
        };
        Ok(Expr { kind, span })
    }

    /// `sum ( VAR in EXPR .. EXPR : EXPR )` — a range reduction.
    fn reduction(&mut self, op: ReduceOp, span: Span) -> Result<Expr, Error> {
        self.expect(&Tok::LParen)?;
        let var = self.ident_name("a reduction variable, e.g. `sum(i in 0..n : ...)`")?;
        match self.ident_name("`in`") {
            Ok(kw) if kw == "in" => {}
            _ => return Err(Error::new("expected `in` in a reduction", self.span())),
        }
        let start = self.expr()?;
        self.expect_msg(&Tok::DotDot, "expected `..` in the reduction range")?;
        let end = self.expr()?;
        self.expect_msg(&Tok::Colon, "expected `:` before the reduction body")?;
        let body = self.expr()?;
        self.expect_msg(&Tok::RParen, "expected `)` to close the reduction")?;
        Ok(Expr {
            kind: ExprKind::Reduce {
                op,
                var,
                start: Box::new(start),
                end: Box::new(end),
                body: Box::new(body),
            },
            span,
        })
    }

    /// `(` already peeked: either `(expr)` grouping or `(x, y)` pair.
    fn paren_or_pair(&mut self) -> Result<Expr, Error> {
        let start = self.span();
        self.expect(&Tok::LParen)?;
        let first = self.expr()?;
        if self.peek_tok() == &Tok::Comma {
            self.bump();
            let second = self.expr()?;
            let end = self.span();
            self.expect_msg(&Tok::RParen, "expected `)` to close `(x, y)`")?;
            let len = if end.line == start.line {
                (end.col + end.len).saturating_sub(start.col)
            } else {
                start.len
            };
            Ok(Expr {
                kind: ExprKind::PairE(Box::new(first), Box::new(second)),
                span: Span::new(start.line, start.col, len.max(1)),
            })
        } else {
            self.expect_msg(&Tok::RParen, "expected `)` or `,` after `(`")?;
            Ok(first) // grouping
        }
    }

    fn bin(&self, op: BinOp, l: Expr, r: Expr) -> Expr {
        let span = l.span;
        Expr {
            kind: ExprKind::Bin(op, Box::new(l), Box::new(r)),
            span,
        }
    }

    fn expect(&mut self, want: &Tok) -> Result<(), Error> {
        if self.peek_tok() == want {
            self.bump();
            Ok(())
        } else {
            Err(Error::new(
                format!("expected {}, found {}", describe(want), describe(self.peek_tok())),
                self.span(),
            ))
        }
    }

    fn expect_msg(&mut self, want: &Tok, msg: &str) -> Result<(), Error> {
        if self.peek_tok() == want {
            self.bump();
            Ok(())
        } else {
            Err(Error::new(msg.to_string(), self.span()))
        }
    }
}

fn reduce_op(name: &str) -> Option<ReduceOp> {
    Some(match name {
        "sum" => ReduceOp::Sum,
        "prod" => ReduceOp::Prod,
        "min" => ReduceOp::Min,
        "max" => ReduceOp::Max,
        _ => return None,
    })
}

fn describe(t: &Tok) -> String {
    match t {
        Tok::Ident(s) => format!("`{s}`"),
        Tok::Num(n) => format!("number `{n}`"),
        Tok::Str(_) => "a string".to_string(),
        Tok::LParen => "`(`".to_string(),
        Tok::RParen => "`)`".to_string(),
        Tok::LBrace => "`{`".to_string(),
        Tok::RBrace => "`}`".to_string(),
        Tok::Comma => "`,`".to_string(),
        Tok::Semi => "`;`".to_string(),
        Tok::Plus => "`+`".to_string(),
        Tok::Minus => "`-`".to_string(),
        Tok::Star => "`*`".to_string(),
        Tok::Slash => "`/`".to_string(),
        Tok::Caret => "`^`".to_string(),
        Tok::Eq => "`=`".to_string(),
        Tok::Lt => "`<`".to_string(),
        Tok::Le => "`<=`".to_string(),
        Tok::Gt => "`>`".to_string(),
        Tok::Ge => "`>=`".to_string(),
        Tok::EqEq => "`==`".to_string(),
        Tok::Ne => "`!=`".to_string(),
        Tok::AndAnd => "`&&`".to_string(),
        Tok::OrOr => "`||`".to_string(),
        Tok::DotDot => "`..`".to_string(),
        Tok::Colon => "`:`".to_string(),
        Tok::Eof => "end of file".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_calls_pairs_and_blocks() {
        let p = parse(
            r#"
            title("Skip Lists");
            circle(A, (300, 400), 40);
            par {
              fade_in(e, 0.15);
              grow(e, (860, 400), 0.5, smooth);
            }
            "#,
        )
        .unwrap();
        assert_eq!(p.stmts.len(), 3);
        assert_eq!(p.stmts[0].name, "title");
        assert_eq!(p.stmts[1].name, "circle");
        assert_eq!(p.stmts[1].args.len(), 3);
        // (300, 400) parses as a pair expression
        assert!(matches!(p.stmts[1].args[1].kind, ExprKind::PairE(_, _)));
        let par = &p.stmts[2];
        assert_eq!(par.name, "par");
        assert_eq!(par.block.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn bare_block_call_no_args() {
        let p = parse("seq { wait(0.5); }").unwrap();
        assert_eq!(p.stmts[0].name, "seq");
        assert!(p.stmts[0].args.is_empty());
        assert_eq!(p.stmts[0].block.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn missing_semicolon_is_a_clear_error() {
        let e = parse("circle(A, (0,0), 40)").unwrap_err();
        assert!(e.msg.contains("expected `;` or `{`"), "{}", e.msg);
    }

    #[test]
    fn variable_coordinate_now_parses() {
        // `x` is a valid variable reference now; unbound-ness is caught later.
        let p = parse("move(A, (x, 0));").unwrap();
        assert!(matches!(p.stmts[0].args[1].kind, ExprKind::PairE(_, _)));
    }

    #[test]
    fn parses_let_and_for() {
        let p = parse("let n = 5;\nfor i in 0..n { rect(bar{i}, (i*10, 0), 8, 8); }").unwrap();
        assert!(matches!(p.stmts[0].ctrl, Some(Ctrl::Let(_, _))));
        match &p.stmts[1].ctrl {
            Some(Ctrl::For { var, body, .. }) => {
                assert_eq!(var, "i");
                assert_eq!(body.len(), 1);
                // the id `bar{i}` is an interpolation
                assert!(matches!(body[0].args[0].kind, ExprKind::Interp(_)));
            }
            _ => panic!("expected a for loop"),
        }
    }

    #[test]
    fn unterminated_block_errors() {
        let e = parse("par { wait(1);").unwrap_err();
        assert!(e.msg.contains("unterminated"), "{}", e.msg);
    }
}
