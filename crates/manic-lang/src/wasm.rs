//! The **WASM boundary** — thin `wasm-bindgen` exports over [`crate::services`].
//! Each returns a JSON string (the browser does `JSON.parse`), so there are no
//! serde/serde-wasm-bindgen dependencies and the surface stays tiny.
//!
//! Build with: `wasm-pack build crates/manic-lang --target web --features wasm`.

use wasm_bindgen::prelude::*;

use crate::services::{self, Completion, Diagnostic, SemToken};

/// Semantic tokens for highlighting — `[{start,len,kind}]`.
#[wasm_bindgen]
pub fn tokenize(src: &str) -> String {
    let toks = services::tokenize(src);
    let items: Vec<String> = toks
        .iter()
        .map(|t: &SemToken| format!("{{\"start\":{},\"len\":{},\"kind\":\"{}\"}}", t.start, t.len, t.kind))
        .collect();
    format!("[{}]", items.join(","))
}

/// Diagnostics — `[{start,len,severity,message,fix?}]`.
#[wasm_bindgen]
pub fn check(src: &str) -> String {
    let diags = services::check(src);
    let items: Vec<String> = diags.iter().map(diag_json).collect();
    format!("[{}]", items.join(","))
}

/// Completions at a char `offset` — `[{label,kind,insert,detail,doc}]`.
#[wasm_bindgen]
pub fn complete(src: &str, offset: u32) -> String {
    let comps = services::complete(src, offset);
    let items: Vec<String> = comps.iter().map(comp_json).collect();
    format!("[{}]", items.join(","))
}

fn diag_json(d: &Diagnostic) -> String {
    let fix = match &d.fix {
        Some(f) => format!(
            ",\"fix\":{{\"label\":\"{}\",\"replacement\":\"{}\",\"start\":{},\"len\":{}}}",
            esc(&f.label),
            esc(&f.replacement),
            f.start,
            f.len
        ),
        None => String::new(),
    };
    format!(
        "{{\"start\":{},\"len\":{},\"severity\":\"{}\",\"message\":\"{}\"{}}}",
        d.start,
        d.len,
        d.severity,
        esc(&d.message),
        fix
    )
}

fn comp_json(c: &Completion) -> String {
    format!(
        "{{\"label\":\"{}\",\"kind\":\"{}\",\"insert\":\"{}\",\"detail\":\"{}\",\"doc\":\"{}\"}}",
        esc(&c.label),
        c.kind,
        esc(&c.insert),
        esc(&c.detail),
        esc(&c.doc)
    )
}

/// Minimal JSON string escaping.
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}
