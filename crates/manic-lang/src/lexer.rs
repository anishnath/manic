//! The manic lexer: source text → a flat token stream.
//!
//! The language is ASY-inspired: function-call statements, `(x,y)` pairs,
//! `;` terminators, `{ }` blocks, `//` line comments. The lexer knows no
//! keywords — `move`, `par`, `magenta`, `smooth` are all just identifiers;
//! their meaning is resolved later against the builtin registry. That is what
//! keeps the front end domain-agnostic.

use crate::diag::{Error, Span};

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
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
    EqEq,
    Ne,
    AndAnd,
    OrOr,
    /// `..` — a range, used by `for`.
    DotDot,
    /// `:` — separates the range from the body in a reduction (`sum(i in a..b: e)`).
    Colon,
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

    fn peek2(&self) -> Option<char> {
        self.chars.clone().nth(1)
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
    // `.` lets ids address kit-generated children and tag groups
    // (`g.nodes`, `code.line1`, `run0a.tag`).
    c.is_ascii_alphanumeric() || c == '_' || c == '.'
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
                Some('/') if lx.peek2() == Some('/') => {
                    // line comment `//`: consume to end of line
                    lx.bump();
                    lx.bump();
                    while let Some(c) = lx.peek() {
                        if c == '\n' {
                            break;
                        }
                        lx.bump();
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
            ':' => {
                lx.bump();
                Tok::Colon
            }
            '+' => {
                lx.bump();
                Tok::Plus
            }
            '-' => {
                lx.bump();
                Tok::Minus
            }
            '*' => {
                lx.bump();
                Tok::Star
            }
            '/' => {
                lx.bump();
                Tok::Slash
            }
            '^' => {
                lx.bump();
                Tok::Caret
            }
            '=' => {
                lx.bump();
                if lx.peek() == Some('=') {
                    lx.bump();
                    Tok::EqEq
                } else {
                    Tok::Eq
                }
            }
            '<' => {
                lx.bump();
                if lx.peek() == Some('=') {
                    lx.bump();
                    Tok::Le
                } else {
                    Tok::Lt
                }
            }
            '>' => {
                lx.bump();
                if lx.peek() == Some('=') {
                    lx.bump();
                    Tok::Ge
                } else {
                    Tok::Gt
                }
            }
            '!' => {
                lx.bump();
                if lx.peek() == Some('=') {
                    lx.bump();
                    Tok::Ne
                } else {
                    return Err(Error::new("unexpected `!` (did you mean `!=`?)", start));
                }
            }
            '&' => {
                lx.bump();
                if lx.peek() == Some('&') {
                    lx.bump();
                    Tok::AndAnd
                } else {
                    return Err(Error::new("unexpected `&` (did you mean `&&`?)", start));
                }
            }
            '|' => {
                lx.bump();
                if lx.peek() == Some('|') {
                    lx.bump();
                    Tok::OrOr
                } else {
                    return Err(Error::new("unexpected `|` (did you mean `||`?)", start));
                }
            }
            // `..` range, or a `.5`-style number, or an error
            '.' => {
                lx.bump(); // first '.'
                if lx.peek() == Some('.') {
                    lx.bump();
                    Tok::DotDot
                } else if matches!(lx.peek(), Some(d) if d.is_ascii_digit()) {
                    let mut s = String::from("0.");
                    while let Some(ch) = lx.peek() {
                        if ch.is_ascii_digit() {
                            s.push(ch);
                            lx.bump();
                        } else {
                            break;
                        }
                    }
                    let n: f32 = s
                        .parse()
                        .map_err(|_| Error::new(format!("invalid number `{s}`"), start))?;
                    out.push(Token {
                        tok: Tok::Num(n),
                        span: Span::new(start.line, start.col, s.chars().count() as u32),
                    });
                    continue;
                } else {
                    return Err(Error::new("unexpected `.`", start));
                }
            }
            '"' => {
                lx.bump(); // opening quote
                let mut s = String::new();
                loop {
                    match lx.bump() {
                        Some('"') => break,
                        // LaTeX-safe strings: keep backslashes verbatim so
                        // `"\frac{1}{2}"`, `"\theta"`, `"\int"` all survive. Only
                        // `\"` (a literal quote) and `\\` (a literal backslash)
                        // are special; every other backslash is preserved for
                        // LaTeX. (Backticks are still fully raw, incl. quotes.)
                        Some('\\') => match lx.peek() {
                            Some('"') => {
                                lx.bump();
                                s.push('"');
                            }
                            Some('\\') => {
                                lx.bump();
                                s.push('\\');
                            }
                            _ => s.push('\\'), // keep it; the next char lexes normally
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
            '`' => {
                // Raw string: NO escape processing (backslashes kept verbatim), for
                // LaTeX in `equation(...)` — `\frac{1}{2}`, `\times`, `\theta` all
                // survive intact. Same `Str` token as `"..."`.
                lx.bump(); // opening backtick
                let mut s = String::new();
                loop {
                    match lx.bump() {
                        Some('`') => break,
                        Some(ch) => s.push(ch),
                        None => return Err(Error::new("unterminated raw string literal", start)),
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
            c if c.is_ascii_digit() => {
                let mut s = String::new();
                let mut seen_dot = false;
                while let Some(ch) = lx.peek() {
                    if ch.is_ascii_digit() {
                        s.push(ch);
                        lx.bump();
                    } else if ch == '.' && !seen_dot && lx.peek2() != Some('.') {
                        // a lone `.` is a decimal point; `..` is a range (leave it)
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
            '\\' => {
                return Err(Error::new(
                    "unexpected `\\` — LaTeX must be inside a STRING. Wrap it in double quotes (or backticks): equation(q,(x,y),\"\\frac{1}{2}\")",
                    start,
                ))
            }
            other => return Err(Error::new(format!("unexpected character `{other}`"), start)),
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
        // LaTeX-safe strings keep the backslash: `"hi\n"` is `hi` + `\` + `n`.
        assert!(toks.contains(&Tok::Str("hi\\n".into())));
        // `-5` now lexes as a Minus operator + Num(5) (unary minus in the parser)
        assert!(toks.contains(&Tok::Minus));
        assert!(toks.contains(&Tok::Num(5.0)));
        assert_eq!(*toks.last().unwrap(), Tok::Eof);
    }

    #[test]
    fn strings_keep_latex_backslashes() {
        // BOTH `"..."` and `` `...` `` keep backslashes verbatim (LaTeX-safe), so
        // `\theta`/`\frac`/`\neq` survive. `"..."` only treats `\"` and `\\`.
        let quoted = kinds(r#"f("\theta = \frac{\pi}{4} \neq \tan x")"#);
        assert!(
            quoted.contains(&Tok::Str(r"\theta = \frac{\pi}{4} \neq \tan x".into())),
            "double quotes should keep LaTeX backslashes: {quoted:?}"
        );
        let raw = kinds("f(`\\theta = \\frac{\\pi}{4} \\neq \\tan x`)");
        assert!(
            raw.contains(&Tok::Str(r"\theta = \frac{\pi}{4} \neq \tan x".into())),
            "raw string should keep backslashes verbatim: {raw:?}"
        );
        // `\"` still escapes a quote inside a double-quoted string
        let esc = kinds(r#"f("a\"b")"#);
        assert!(esc.contains(&Tok::Str("a\"b".into())), "\\\" should escape a quote: {esc:?}");
    }

    #[test]
    fn lexes_operators_and_ranges() {
        let toks = kinds("let n = 2 + 3*4 / 2 ^ i .. m;");
        assert!(toks.contains(&Tok::Eq));
        assert!(toks.contains(&Tok::Plus));
        assert!(toks.contains(&Tok::Star));
        assert!(toks.contains(&Tok::Slash));
        assert!(toks.contains(&Tok::Caret));
        assert!(toks.contains(&Tok::DotDot));
    }

    #[test]
    fn tracks_line_and_col() {
        let toks = lex("a;\n  b;").unwrap();
        // `b` is on line 2, col 3
        let b = toks
            .iter()
            .find(|t| t.tok == Tok::Ident("b".into()))
            .unwrap();
        assert_eq!((b.span.line, b.span.col), (2, 3));
    }

    #[test]
    fn single_slash_is_division() {
        let toks = kinds("a / b");
        assert_eq!(
            toks,
            vec![
                Tok::Ident("a".into()),
                Tok::Slash,
                Tok::Ident("b".into()),
                Tok::Eof,
            ]
        );
    }
}
