//! Recursive-descent parser: tokens → [`Program`].
//!
//! Grammar (deliberately tiny):
//!
//! ```text
//! program := stmt* EOF
//! stmt    := IDENT arglist? ( block | ";" )
//! arglist := "(" ( expr ("," expr)* )? ")"
//! block   := "{" stmt* "}"
//! expr    := NUM | STR | IDENT | pair
//! pair    := "(" NUM "," NUM ")"
//! ```
//!
//! The parser validates *shape* only — it never checks whether a call name is
//! a known builtin, or whether an ident is a valid color. Those are the
//! lowering pass's job, so this stays domain-agnostic.

use super::ast::{Expr, ExprKind, Program, Stmt};
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
        // name
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
                })
            }
            Tok::Semi => {
                self.bump();
                Ok(Stmt {
                    name,
                    name_span,
                    args,
                    block: None,
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

    fn expr(&mut self) -> Result<Expr, Error> {
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
            Tok::Ident(s) => {
                self.bump();
                Ok(Expr {
                    kind: ExprKind::Ident(s),
                    span,
                })
            }
            Tok::LParen => self.pair(),
            other => Err(Error::new(
                format!(
                    "expected an argument (number, \"string\", name, or `(x,y)`), found {}",
                    describe(&other)
                ),
                span,
            )),
        }
    }

    fn pair(&mut self) -> Result<Expr, Error> {
        let start = self.span();
        self.expect(&Tok::LParen)?;
        let x = self.number("coordinate x")?;
        self.expect_msg(&Tok::Comma, "expected `,` between the two coordinates of `(x, y)`")?;
        let y = self.number("coordinate y")?;
        let end = self.span();
        self.expect_msg(&Tok::RParen, "expected `)` to close `(x, y)`")?;
        let len = if end.line == start.line {
            (end.col + end.len).saturating_sub(start.col)
        } else {
            start.len
        };
        Ok(Expr {
            kind: ExprKind::Pair(x, y),
            span: Span::new(start.line, start.col, len.max(1)),
        })
    }

    fn number(&mut self, what: &str) -> Result<f32, Error> {
        match self.peek_tok().clone() {
            Tok::Num(n) => {
                self.bump();
                Ok(n)
            }
            other => Err(Error::new(
                format!("expected {what} (a number), found {}", describe(&other)),
                self.span(),
            )),
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
        // circle args: Ident(A), Pair(300,400), Num(40)
        assert_eq!(p.stmts[1].args.len(), 3);
        assert_eq!(p.stmts[1].args[1].kind, ExprKind::Pair(300.0, 400.0));
        // par is a block statement with 2 inner stmts
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
    fn non_numeric_coordinate_errors() {
        let e = parse("move(A, (x, 0));").unwrap_err();
        assert!(e.msg.contains("number"), "{}", e.msg);
    }

    #[test]
    fn unterminated_block_errors() {
        let e = parse("par { wait(1);").unwrap_err();
        assert!(e.msg.contains("unterminated"), "{}", e.msg);
    }
}
