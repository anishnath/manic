//! Source spans and human-facing error diagnostics for the manic language.
//!
//! Errors carry a [`Span`] so the CLI can point the author at the exact line
//! and column, with a caret under the offending token — the language is meant
//! for non-programmers, so a bad message is a bad product.

/// A 1-based location in the source, plus a length in characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: u32,
    pub col: u32,
    pub len: u32,
}

impl Span {
    pub fn new(line: u32, col: u32, len: u32) -> Span {
        Span { line, col, len }
    }
}

/// A language error (lex, parse, or lower) tied to a source span.
#[derive(Debug, Clone)]
pub struct Error {
    pub msg: String,
    pub span: Span,
}

impl Error {
    pub fn new(msg: impl Into<String>, span: Span) -> Error {
        Error {
            msg: msg.into(),
            span,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "error: {} (line {}, col {})",
            self.msg, self.span.line, self.span.col
        )
    }
}

impl std::error::Error for Error {}

/// Render an error against the source with a caret, e.g.
///
/// ```text
/// error: unknown builtin `moove`
///   --> line 12, col 1
///    |
/// 12 | moove(A, (300,250));
///    | ^^^^^
/// ```
pub fn render(src: &str, err: &Error) -> String {
    let line_idx = err.span.line.saturating_sub(1) as usize;
    let line_text = src.lines().nth(line_idx).unwrap_or("");
    let gutter = err.span.line.to_string();
    let pad = " ".repeat(gutter.len());
    let caret_pad = " ".repeat(err.span.col.saturating_sub(1) as usize);
    let caret = "^".repeat(err.span.len.max(1) as usize);
    format!(
        "error: {msg}\n{pad} --> line {line}, col {col}\n{pad} |\n{gutter} | {line_text}\n{pad} | {caret_pad}{caret}",
        msg = err.msg,
        line = err.span.line,
        col = err.span.col,
    )
}
