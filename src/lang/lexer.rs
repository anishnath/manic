//! The manic lexer: source text → a flat token stream.
//!
//! The language is ASY-inspired: function-call statements, `(x,y)` pairs,
//! `;` terminators, `{ }` blocks, `//` line comments. The lexer knows no
//! keywords — `move`, `par`, `magenta`, `smooth` are all just identifiers;
//! their meaning is resolved later against the builtin registry. That is what
//! keeps the front end domain-agnostic.

use super::diag::{Error, Span};

/// A lexical token.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    /// `move`, `A`, `magenta`, `smooth` — any bare word.
    Ident(String),
    /// A numeric literal (ints and floats both land here as `f32`).
    Num(f32),
    /// A `"..."` string literal (unescaped contents).
    Str(String),
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semi,
    Eof,
}

/// A token plus where it came from.
#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: u32,
    col: u32,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer {
            chars: src.chars().peekable(),
            line: 1,
            col: 1,
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next();
        match c {
            Some('\n') => {
                self.line += 1;
                self.col = 1;
            }
            Some(_) => self.col += 1,
            None => {}
        }
        c
    }

    fn here(&self) -> Span {
        Span::new(self.line, self.col, 1)
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Tokenize `src`. Returns tokens ending in a single [`Tok::Eof`], or the
/// first lexical error encountered.
pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let mut lx = Lexer::new(src);
    let mut out = Vec::new();

    loop {
        // skip whitespace and // line comments
        loop {
            match lx.peek() {
                Some(c) if c.is_whitespace() => {
                    lx.bump();
                }
                Some('/') => {
                    // could be a comment `//`; peek the char after
                    let start = lx.here();
                    lx.bump(); // consume first '/'
                    if lx.peek() == Some('/') {
                        // line comment: consume to end of line
                        while let Some(c) = lx.peek() {
                            if c == '\n' {
                                break;
                            }
                            lx.bump();
                        }
                    } else {
                        return Err(Error::new(
                            "unexpected `/` (did you mean `//` for a comment?)",
                            start,
                        ));
                    }
                }
                _ => break,
            }
        }

        let start = lx.here();
        let Some(c) = lx.peek() else {
            out.push(Token {
                tok: Tok::Eof,
                span: start,
            });
            return Ok(out);
        };

        let tok = match c {
            '(' => {
                lx.bump();
                Tok::LParen
            }
            ')' => {
                lx.bump();
                Tok::RParen
            }
            '{' => {
                lx.bump();
                Tok::LBrace
            }
            '}' => {
                lx.bump();
                Tok::RBrace
            }
            ',' => {
                lx.bump();
                Tok::Comma
            }
            ';' => {
                lx.bump();
                Tok::Semi
            }
            '"' => {
                lx.bump(); // opening quote
                let mut s = String::new();
                loop {
                    match lx.bump() {
                        Some('"') => break,
                        Some('\\') => match lx.bump() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('"') => s.push('"'),
                            Some('\\') => s.push('\\'),
                            Some(other) => s.push(other),
                            None => {
                                return Err(Error::new("unterminated string literal", start))
                            }
                        },
                        Some(ch) => s.push(ch),
                        None => return Err(Error::new("unterminated string literal", start)),
                    }
                }
                let len = (lx.col.saturating_sub(start.col)).max(1);
                out.push(Token {
                    tok: Tok::Str(s),
                    span: Span::new(start.line, start.col, len),
                });
                continue;
            }
            c if is_ident_start(c) => {
                let mut s = String::new();
                while let Some(ch) = lx.peek() {
                    if is_ident_continue(ch) {
                        s.push(ch);
                        lx.bump();
                    } else {
                        break;
                    }
                }
                let len = s.chars().count() as u32;
                out.push(Token {
                    tok: Tok::Ident(s),
                    span: Span::new(start.line, start.col, len),
                });
                continue;
            }
            c if c.is_ascii_digit() || c == '-' || c == '.' => {
                let mut s = String::new();
                if c == '-' {
                    s.push('-');
                    lx.bump();
                    // must be followed by a digit or dot to be a number
                    match lx.peek() {
                        Some(d) if d.is_ascii_digit() || d == '.' => {}
                        _ => return Err(Error::new("expected a number after `-`", start)),
                    }
                }
                let mut seen_dot = false;
                while let Some(ch) = lx.peek() {
                    if ch.is_ascii_digit() {
                        s.push(ch);
                        lx.bump();
                    } else if ch == '.' && !seen_dot {
                        seen_dot = true;
                        s.push(ch);
                        lx.bump();
                    } else {
                        break;
                    }
                }
                let n: f32 = s
                    .parse()
                    .map_err(|_| Error::new(format!("invalid number `{s}`"), start))?;
                let len = s.chars().count() as u32;
                out.push(Token {
                    tok: Tok::Num(n),
                    span: Span::new(start.line, start.col, len),
                });
                continue;
            }
            other => {
                return Err(Error::new(
                    format!("unexpected character `{other}`"),
                    start,
                ))
            }
        };

        out.push(Token { tok, span: start });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<Tok> {
        lex(src).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn lexes_a_move_statement() {
        let toks = kinds("move(A, (300, 250.5));");
        assert_eq!(
            toks,
            vec![
                Tok::Ident("move".into()),
                Tok::LParen,
                Tok::Ident("A".into()),
                Tok::Comma,
                Tok::LParen,
                Tok::Num(300.0),
                Tok::Comma,
                Tok::Num(250.5),
                Tok::RParen,
                Tok::RParen,
                Tok::Semi,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn handles_negatives_strings_comments_blocks() {
        let toks = kinds("// hi\npar {\n  say(cap, \"hi\\n\"); move(a,(-5,0));\n}");
        assert_eq!(toks[0], Tok::Ident("par".into()));
        assert_eq!(toks[1], Tok::LBrace);
        assert!(toks.contains(&Tok::Str("hi\n".into())));
        assert!(toks.contains(&Tok::Num(-5.0)));
        assert_eq!(*toks.last().unwrap(), Tok::Eof);
    }

    #[test]
    fn tracks_line_and_col() {
        let toks = lex("a;\n  b;").unwrap();
        // `b` is on line 2, col 3
        let b = toks.iter().find(|t| t.tok == Tok::Ident("b".into())).unwrap();
        assert_eq!((b.span.line, b.span.col), (2, 3));
    }

    #[test]
    fn bare_slash_is_an_error() {
        assert!(lex("a / b").is_err());
    }
}
