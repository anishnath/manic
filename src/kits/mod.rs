//! Kits register domain vocabulary into the language's builtin registry.
//!
//! `std` is always on (generic shapes + animation verbs). Domain kits — `math`
//! today, `algo` next — add their nouns and verbs on top. A new domain is a
//! new file here plus one line in [`default_registry`]; the core never
//! changes.

pub mod math;
pub mod std;

use crate::lang::lower::Registry;

/// The default registry: std + every shipped domain kit.
pub fn default_registry() -> Registry {
    let mut r = Registry::new();
    std::register(&mut r);
    math::register(&mut r);
    r
}
