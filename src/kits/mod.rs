//! Kits register domain vocabulary into the language's builtin registry.
//!
//! `std` is always on (generic shapes + animation verbs). Domain kits — `math`
//! today, `algo` next — add their nouns and verbs on top. A new domain is a
//! new file here plus one line in [`default_registry`]; the core never
//! changes.

pub mod algo;
pub mod brand;
pub mod geo;
pub mod math;
pub mod std;

use crate::lang::lower::Registry;

/// The default registry: std + every shipped domain kit.
pub fn default_registry() -> Registry {
    let mut r = Registry::new();
    std::register(&mut r);
    math::register(&mut r);
    algo::register(&mut r);
    geo::register(&mut r);
    brand::register(&mut r);
    r
}

#[cfg(test)]
mod catalog_tests {
    use manic_lang::catalog::{catalog, Kind};

    /// The catalog and the live registry must describe exactly the same builtins
    /// — same names, same kinds. This is what guarantees the browser editor's
    /// highlighting/autocomplete never drift from what the engine accepts.
    #[test]
    fn catalog_matches_registry() {
        let reg = super::default_registry();
        let mut from_registry: Vec<(String, String)> = reg
            .builtins()
            .into_iter()
            .map(|(n, k)| (n.to_string(), k.to_string()))
            .collect();
        from_registry.sort();

        let kind_str = |k: Kind| match k {
            Kind::Ctor => "ctor",
            Kind::Verb => "verb",
            Kind::MutVerb => "mut_verb",
        };
        let mut from_catalog: Vec<(String, String)> = catalog()
            .iter()
            .map(|s| (s.name.to_string(), kind_str(s.kind).to_string()))
            .collect();
        from_catalog.sort();

        // pinpoint drift with a helpful message
        let missing: Vec<_> = from_registry.iter().filter(|e| !from_catalog.contains(e)).collect();
        let extra: Vec<_> = from_catalog.iter().filter(|e| !from_registry.contains(e)).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "catalog drift — missing from catalog: {missing:?}; not in registry: {extra:?}"
        );
    }
}
