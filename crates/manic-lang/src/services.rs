//! **Editor language services** — pure functions the browser calls (via the
//! WASM wrapper) to power a manic code editor:
//!
//! - [`tokenize`] → semantic tokens for **highlighting** (builtin vs variable vs
//!   colour vs keyword vs comment …);
//! - [`check`] → **diagnostics** (lex/parse/expand errors + name/arg validation),
//!   each with an optional auto-**fix**;
//! - [`complete`] → context-aware **autocomplete** (builtins at statement start;
//!   the right values inside a call, incl. the file's own ids).
//!
//! All logic is here and unit-tested in plain Rust; the WASM layer is a thin
//! JSON marshaller over these.

use crate::ast::{Expr, ExprKind, Stmt};
use crate::catalog::{
    catalog, BuiltinSpec, Ty, CANVAS_PRESETS, COLORS, EASINGS, KEYWORDS, NAMED_FNS, RESERVED_VARS,
    TEMPLATES,
};
use crate::diag::{Error, Span};
use crate::expand::expand;
use crate::lexer::{lex, Tok};
use crate::parser::parse;

/// Control-flow / meta names handled by the lowerer, not the registry.
const RESERVED_CONTROL: &[&str] = &[
    "par", "seq", "stagger", "section", "wait", "beat", "mark", "title", "canvas", "template",
    "masthead",
];

// ---- output shapes --------------------------------------------------------

/// A highlight token: `[start, start+len)` (char offsets) with a semantic class.
#[derive(Debug, Clone, PartialEq)]
pub struct SemToken {
    pub start: u32,
    pub len: u32,
    /// `builtin` `call` `keyword` `constant` `color` `ease` `number` `string`
    /// `variable` `comment`
    pub kind: &'static str,
}

/// A suggested edit attached to a diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub struct Fix {
    pub label: String,
    pub replacement: String,
    pub start: u32,
    pub len: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub start: u32,
    pub len: u32,
    /// `error` (only kind for now).
    pub severity: &'static str,
    pub message: String,
    pub fix: Option<Fix>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub label: String,
    /// `builtin` `keyword` `color` `ease` `id` `snippet` `preset`
    pub kind: &'static str,
    pub insert: String,
    pub detail: String,
    pub doc: String,
}

// ---- offset helpers -------------------------------------------------------

fn line_starts(src: &str) -> Vec<usize> {
    let mut v = vec![0usize];
    for (i, c) in src.chars().enumerate() {
        if c == '\n' {
            v.push(i + 1);
        }
    }
    v
}

fn range(starts: &[usize], sp: &Span) -> (u32, u32) {
    let li = ((sp.line.max(1) - 1) as usize).min(starts.len().saturating_sub(1));
    let start = starts[li] + (sp.col.max(1) - 1) as usize;
    (start as u32, sp.len)
}

// ---- highlighting ---------------------------------------------------------

fn builtin_names() -> Vec<&'static str> {
    catalog().into_iter().map(|s| s.name).collect()
}

fn classify_ident(name: &str, is_call: bool, builtins: &[&str]) -> &'static str {
    if KEYWORDS.contains(&name) {
        "keyword"
    } else if is_call {
        if builtins.contains(&name) {
            "builtin"
        } else {
            "call" // a user `def` macro, or (if unknown) flagged by `check`
        }
    } else if RESERVED_VARS.contains(&name) {
        "constant"
    } else if COLORS.contains(&name) {
        "color"
    } else if EASINGS.contains(&name) {
        "ease"
    } else {
        "variable"
    }
}

/// Line comments (`// …` outside a string), scanned directly since the lexer
/// discards them.
fn scan_comments(src: &str, out: &mut Vec<SemToken>) {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut in_str = false;
    while i < chars.len() {
        let c = chars[i];
        if in_str {
            if c == '"' {
                in_str = false;
            }
            i += 1;
        } else if c == '"' {
            in_str = true;
            i += 1;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            let start = i;
            let mut j = i;
            while j < chars.len() && chars[j] != '\n' {
                j += 1;
            }
            out.push(SemToken {
                start: start as u32,
                len: (j - start) as u32,
                kind: "comment",
            });
            i = j;
        } else {
            i += 1;
        }
    }
}

/// Semantic tokens for highlighting. A sparse overlay — only the meaningful
/// spans are returned; the editor leaves everything else default.
pub fn tokenize(src: &str) -> Vec<SemToken> {
    let starts = line_starts(src);
    let builtins = builtin_names();
    let mut out = Vec::new();
    scan_comments(src, &mut out);
    if let Ok(toks) = lex(src) {
        for (i, t) in toks.iter().enumerate() {
            let kind = match &t.tok {
                Tok::Num(_) => Some("number"),
                Tok::Str(_) => Some("string"),
                Tok::Ident(name) => {
                    let is_call = matches!(toks.get(i + 1).map(|x| &x.tok), Some(Tok::LParen));
                    Some(classify_ident(name, is_call, &builtins))
                }
                _ => None,
            };
            if let Some(kind) = kind {
                let (start, len) = range(&starts, &t.span);
                out.push(SemToken { start, len, kind });
            }
        }
    }
    out.sort_by_key(|t| t.start);
    out
}

// ---- diagnostics ----------------------------------------------------------

fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur.push((prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost));
        }
        prev = cur;
    }
    prev[b.len()]
}

fn nearest<'a>(name: &str, cands: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    cands
        .map(|c| (edit_distance(name, c), c))
        .filter(|(d, _)| *d <= 2)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

fn diag_from_error(starts: &[usize], e: &Error) -> Diagnostic {
    let (start, len) = range(starts, &e.span);
    Diagnostic {
        start,
        len,
        severity: "error",
        message: e.msg.clone(),
        fix: e.fix.as_ref().map(|(label, replacement)| Fix {
            label: label.clone(),
            replacement: replacement.clone(),
            start,
            len,
        }),
    }
}

/// Lex → parse → expand → name/arg validation. Returns the first structural
/// error (lex/parse/expand) if any, else every name/arg problem found.
pub fn check(src: &str) -> Vec<Diagnostic> {
    let starts = line_starts(src);
    if let Err(e) = lex(src) {
        return vec![diag_from_error(&starts, &e)];
    }
    let prog = match parse(src) {
        Ok(p) => p,
        Err(e) => return vec![diag_from_error(&starts, &e)],
    };
    let expanded = match expand(&prog) {
        Ok(p) => p,
        Err(e) => return vec![diag_from_error(&starts, &e)],
    };
    let cat = catalog();
    let names: Vec<&str> = cat.iter().map(|s| s.name).collect();
    let mut out = Vec::new();
    for s in &expanded.stmts {
        validate_stmt(s, &cat, &names, &starts, &mut out);
    }
    out
}

fn validate_stmt(
    s: &Stmt,
    cat: &[BuiltinSpec],
    names: &[&str],
    starts: &[usize],
    out: &mut Vec<Diagnostic>,
) {
    let name = s.name.as_str();
    if RESERVED_CONTROL.contains(&name) {
        return; // engine-handled; not in the catalog
    }
    let Some(spec) = cat.iter().find(|b| b.name == name) else {
        // unknown builtin — suggest the nearest known name
        let (start, len) = range(starts, &s.name_span);
        let fix = nearest(name, names.iter().copied()).map(|sug| Fix {
            label: format!("Change to `{sug}`"),
            replacement: sug.to_string(),
            start,
            len,
        });
        out.push(Diagnostic {
            start,
            len,
            severity: "error",
            message: format!("unknown builtin `{name}`"),
            fix,
        });
        return;
    };
    // arg-count (only where we have authored params)
    if !spec.params.is_empty() {
        let required = spec.params.iter().filter(|p| !p.optional).count();
        let (start, len) = range(starts, &s.name_span);
        if s.args.len() < required {
            out.push(Diagnostic {
                start,
                len,
                severity: "error",
                message: format!(
                    "`{name}` needs {required} argument(s), got {}",
                    s.args.len()
                ),
                fix: None,
            });
            return;
        }
        if s.args.len() > spec.params.len() {
            out.push(Diagnostic {
                start,
                len,
                severity: "error",
                message: format!(
                    "`{name}` takes at most {} argument(s), got {}",
                    spec.params.len(),
                    s.args.len()
                ),
                fix: None,
            });
        }
    }
    // colour / easing value checks
    for (arg, param) in s.args.iter().zip(spec.params.iter()) {
        let (vocab, what): (&[&str], &str) = match param.ty {
            Ty::Color => (COLORS, "colour"),
            Ty::Ease => (EASINGS, "easing"),
            Ty::Fn => (NAMED_FNS, "function"),
            _ => continue,
        };
        if let ExprKind::Ident(v) = &arg.kind {
            if !vocab.contains(&v.as_str()) {
                let (start, len) = range(starts, &arg.span);
                let fix = nearest(v, vocab.iter().copied()).map(|sug| Fix {
                    label: format!("Change to `{sug}`"),
                    replacement: sug.to_string(),
                    start,
                    len,
                });
                out.push(Diagnostic {
                    start,
                    len,
                    severity: "error",
                    message: format!("unknown {what} `{v}`"),
                    fix,
                });
            }
        }
    }
}

// ---- autocomplete ---------------------------------------------------------

/// Entity ids declared in the file (first arg of any ctor call), so id-param
/// completion can offer them. Works over the *expanded* program, so loop- and
/// macro-generated ids are included.
fn file_ids(src: &str) -> Vec<String> {
    // parse only the completed statements (up to the last `;`) so a half-typed
    // current line doesn't break extraction of the ids declared above it
    let prefix = match src.rfind(';') {
        Some(i) => &src[..=i],
        None => "",
    };
    let Ok(prog) = parse(prefix).and_then(|p| expand(&p)) else {
        return Vec::new();
    };
    let cat = catalog();
    let ctors: Vec<&str> = cat
        .iter()
        .filter(|b| matches!(b.kind, crate::catalog::Kind::Ctor))
        .map(|b| b.name)
        .collect();
    let mut ids = Vec::new();
    for s in &prog.stmts {
        if ctors.contains(&s.name.as_str()) {
            if let Some(Expr {
                kind: ExprKind::Ident(id),
                ..
            }) = s.args.first()
            {
                if !ids.contains(id) {
                    ids.push(id.clone());
                }
            }
        }
    }
    ids
}

fn signature(spec: &BuiltinSpec) -> String {
    let ps: Vec<String> = spec
        .params
        .iter()
        .map(|p| {
            if p.optional {
                format!("[{}]", p.name)
            } else {
                p.name.to_string()
            }
        })
        .collect();
    format!("{}({})", spec.name, ps.join(", "))
}

/// Context-aware completions at `offset` (a char offset into `src`).
pub fn complete(src: &str, offset: u32) -> Vec<Completion> {
    let cat = catalog();
    let starts = line_starts(src);

    // walk tokens up to the cursor, tracking the enclosing call + param index
    let toks = lex(src).unwrap_or_default();
    let mut frames: Vec<(Option<String>, usize)> = Vec::new();
    let mut prev_ident: Option<String> = None;
    let mut prefix = String::new();
    for t in &toks {
        let (ts, tl) = range(&starts, &t.span);
        if ts >= offset {
            break;
        }
        // capture the partial identifier the cursor sits inside
        if ts < offset && offset <= ts + tl {
            if let Tok::Ident(n) = &t.tok {
                prefix = n.chars().take((offset - ts) as usize).collect();
            }
        }
        match &t.tok {
            Tok::LParen => frames.push((prev_ident.take(), 0)),
            Tok::RParen => {
                frames.pop();
                prev_ident = None;
            }
            Tok::Comma => {
                if let Some(f) = frames.last_mut() {
                    f.1 += 1;
                }
                prev_ident = None;
            }
            Tok::Semi | Tok::LBrace | Tok::RBrace => {
                frames.clear();
                prev_ident = None;
            }
            Tok::Ident(n) => prev_ident = Some(n.clone()),
            _ => prev_ident = None,
        }
    }

    let starts_with = |s: &str| prefix.is_empty() || s.starts_with(&prefix);

    match frames.last() {
        // statement start → builtins + keywords
        None => {
            let mut out: Vec<Completion> = cat
                .iter()
                .filter(|b| starts_with(b.name))
                .map(|b| Completion {
                    label: b.name.to_string(),
                    kind: "builtin",
                    insert: b.name.to_string(),
                    detail: signature(b),
                    doc: b.summary.to_string(),
                })
                .collect();
            for k in KEYWORDS.iter().filter(|k| starts_with(k)) {
                out.push(Completion {
                    label: k.to_string(),
                    kind: "keyword",
                    insert: k.to_string(),
                    detail: String::new(),
                    doc: String::new(),
                });
            }
            out
        }
        // inside a call — offer values appropriate to the current parameter
        Some((call, param_idx)) => {
            // meta calls (`canvas` / `template`) take a preset name string
            let preset_list = |v: &[&str], kind: &'static str| -> Vec<Completion> {
                v.iter()
                    .filter(|x| starts_with(x))
                    .map(|x| Completion {
                        label: x.to_string(),
                        kind,
                        insert: format!("\"{x}\""),
                        detail: String::new(),
                        doc: String::new(),
                    })
                    .collect()
            };
            match call.as_deref() {
                Some("canvas") => return preset_list(CANVAS_PRESETS, "preset"),
                Some("template") => return preset_list(TEMPLATES, "preset"),
                _ => {}
            }
            let spec = call
                .as_ref()
                .and_then(|c| cat.iter().find(|b| b.name == *c));
            let ty = spec.and_then(|s| s.params.get(*param_idx)).map(|p| p.ty);
            let vocab = |v: &[&str], kind: &'static str| -> Vec<Completion> {
                v.iter()
                    .filter(|x| starts_with(x))
                    .map(|x| Completion {
                        label: x.to_string(),
                        kind,
                        insert: x.to_string(),
                        detail: String::new(),
                        doc: String::new(),
                    })
                    .collect()
            };
            match ty {
                Some(Ty::Color) => vocab(COLORS, "color"),
                Some(Ty::Ease) => vocab(EASINGS, "ease"),
                Some(Ty::Fn) => vocab(NAMED_FNS, "function"),
                Some(Ty::Ident) => file_ids(src)
                    .into_iter()
                    .filter(|id| starts_with(id))
                    .map(|id| Completion {
                        label: id.clone(),
                        kind: "id",
                        insert: id,
                        detail: String::new(),
                        doc: String::new(),
                    })
                    .collect(),
                Some(Ty::Point) => vec![Completion {
                    label: "(x, y)".into(),
                    kind: "snippet",
                    insert: "(x, y)".into(),
                    detail: "point literal".into(),
                    doc: String::new(),
                }],
                Some(Ty::Point3) => vec![Completion {
                    label: "(x, y, z)".into(),
                    kind: "snippet",
                    insert: "(x, y, z)".into(),
                    detail: "3D point literal".into(),
                    doc: String::new(),
                }],
                // a bare `(` group (not a known call) → offer the file's ids as a
                // sensible default (verbs reference existing entities)
                None if call.is_none() => Vec::new(),
                _ => file_ids(src)
                    .into_iter()
                    .filter(|id| starts_with(id))
                    .map(|id| Completion {
                        label: id.clone(),
                        kind: "id",
                        insert: id,
                        detail: String::new(),
                        doc: String::new(),
                    })
                    .collect(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds_at<'a>(toks: &'a [SemToken], src: &str, word: &str) -> Option<&'a str> {
        let at = src.find(word)? as u32;
        toks.iter().find(|t| t.start == at).map(|t| t.kind)
    }

    #[test]
    fn tokenize_classifies() {
        let src = "circle(sun, (0,0), 40);  flash(sun, cyan);  // hi";
        let toks = tokenize(src);
        assert_eq!(kinds_at(&toks, src, "circle"), Some("builtin"));
        assert_eq!(kinds_at(&toks, src, "sun"), Some("variable"));
        assert_eq!(kinds_at(&toks, src, "cyan"), Some("color"));
        assert_eq!(kinds_at(&toks, src, "40"), Some("number"));
        assert_eq!(kinds_at(&toks, src, "// hi"), Some("comment"));
    }

    #[test]
    fn tokenize_keyword() {
        let src = "for i in 0..3 { dot(d{i}, (0,0), 2); }";
        let toks = tokenize(src);
        assert_eq!(kinds_at(&toks, src, "for"), Some("keyword"));
    }

    #[test]
    fn check_clean_program() {
        let d = check("circle(sun, (0,0), 40);");
        assert!(d.is_empty(), "expected no diagnostics, got {d:?}");
    }

    #[test]
    fn check_flags_unknown_plot_function() {
        // an unknown bareword function is flagged (was silently accepted → Render
        // wrongly enabled; the `acos` drift). Suggestion offered.
        let d = check("plot(f, (0,0), 80, 60, foobar);");
        let err = d.iter().find(|x| x.severity == "error").expect("expected an error");
        assert!(err.message.contains("function"), "msg: {}", err.message);
        // a valid bareword (incl. the newly-added inverse trig) is accepted
        assert!(check("plot(f, (0,0), 80, 60, acos);")
            .iter()
            .all(|x| x.severity != "error"));
        // a formula string is fine — only barewords are name-checked
        assert!(check("plot(f, (0,0), 80, 60, \"acos(x)\");")
            .iter()
            .all(|x| x.severity != "error"));
    }

    #[test]
    fn check_unknown_builtin_suggests() {
        let d = check("crcle(sun, (0,0), 40);");
        assert_eq!(d.len(), 1);
        assert!(d[0].message.contains("unknown builtin"));
        assert_eq!(d[0].fix.as_ref().unwrap().replacement, "circle");
    }

    #[test]
    fn check_unknown_color_suggests() {
        let d = check("circle(s,(0,0),5); flash(s, cyn);");
        let color = d.iter().find(|x| x.message.contains("colour")).unwrap();
        assert_eq!(color.fix.as_ref().unwrap().replacement, "cyan");
    }

    #[test]
    fn glued_variables_suggest_a_star() {
        // `xvsx` = `xv` + `sx` run together (missing `*`) — the common LLM slip.
        let d = check("let xv = 1;\nlet sx = 2;\ndot(p, (xvsx, 0), 3);");
        let v = d.iter().find(|x| x.message.contains("xvsx")).unwrap();
        assert!(v.message.contains("xv * sx"), "msg: {}", v.message);
        assert_eq!(v.fix.as_ref().unwrap().replacement, "xv * sx");
    }

    #[test]
    fn check_too_few_args() {
        let d = check("circle(sun);");
        assert!(d.iter().any(|x| x.message.contains("argument")));
    }

    #[test]
    fn check_parse_error_reported() {
        let d = check("circle(sun, (0 0), 40);"); // missing comma
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].severity, "error");
    }

    #[test]
    fn complete_statement_start() {
        let out = complete("ci", 2);
        assert!(out
            .iter()
            .any(|c| c.label == "circle" && c.kind == "builtin"));
    }

    #[test]
    fn complete_color_param() {
        let src = "circle(s,(0,0),5);\nflash(s, ";
        let out = complete(src, src.len() as u32);
        assert!(out.iter().any(|c| c.label == "cyan" && c.kind == "color"));
        assert!(!out.iter().any(|c| c.label == "circle"));
    }

    #[test]
    fn complete_id_param_offers_file_ids() {
        let src = "circle(sun,(0,0),5);\ndot(moon,(1,1),3);\nflash(";
        let out = complete(src, src.len() as u32);
        let labels: Vec<_> = out.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"sun") && labels.contains(&"moon"));
    }

    #[test]
    fn complete_presets_exist() {
        // sanity: the fixed vocab lists are wired
        assert!(!CANVAS_PRESETS.is_empty() && !TEMPLATES.is_empty());
    }
}
