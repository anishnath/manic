//! Kits register domain vocabulary into the language's builtin registry.
//!
//! `std` is always on (generic shapes + animation verbs). Domain kits — `math`
//! today, `algo` next — add their nouns and verbs on top. A new domain is a
//! new file here plus one line in [`default_registry`]; the core never
//! changes.

pub mod algo;
pub mod brand;
pub mod creator;
pub mod geo;
pub mod map;
pub mod math;
pub mod ml;
pub mod ml_attention;
pub mod ml_decode;
pub mod ml_embedding;
pub mod ml_tensor;
pub mod ml_transformer;
pub mod optics;
pub mod physics;
pub mod stats;
pub mod std;
pub mod three;

use crate::lang::lower::Registry;

/// The default registry: std + every shipped domain kit.
pub fn default_registry() -> Registry {
    let mut r = Registry::new();
    std::register(&mut r);
    math::register(&mut r);
    ml::register(&mut r);
    ml_tensor::register(&mut r);
    ml_attention::register(&mut r);
    ml_embedding::register(&mut r);
    ml_transformer::register(&mut r);
    ml_decode::register(&mut r);
    algo::register(&mut r);
    geo::register(&mut r);
    map::register(&mut r);
    brand::register(&mut r);
    three::register(&mut r);
    stats::register(&mut r);
    physics::register(&mut r);
    optics::register(&mut r);
    creator::register(&mut r);
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
        let missing: Vec<_> = from_registry
            .iter()
            .filter(|e| !from_catalog.contains(e))
            .collect();
        let extra: Vec<_> = from_catalog
            .iter()
            .filter(|e| !from_registry.contains(e))
            .collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "catalog drift — missing from catalog: {missing:?}; not in registry: {extra:?}"
        );
    }

    /// Deeper guard than `catalog_matches_registry` (which only checks names):
    /// the catalog's declared *arity* must not be narrower than what each engine
    /// ctor actually reads, or the browser editor wrongly rejects valid calls
    /// ("`X` takes at most N argument(s)"). `scripts/audit-arity.py` compares
    /// every spec's param count to the highest `a.num/opt_num/…(i)` its ctor
    /// reads. Skipped only if no Python interpreter is available.
    #[test]
    fn catalog_arity_matches_engine() {
        use std::process::Command;
        let root = env!("CARGO_MANIFEST_DIR");
        let script = format!("{root}/scripts/audit-arity.py");
        let out = ["python3", "python"].iter().find_map(|py| {
            Command::new(py)
                .arg(&script)
                .current_dir(root)
                .output()
                .ok()
        });
        let Some(out) = out else {
            eprintln!("skipping arity audit — no python3/python on PATH");
            return;
        };
        assert!(
            out.status.success(),
            "catalog arity drift (see scripts/audit-arity.py):\n{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr),
        );
    }

    /// The authoritative editor guard: every shipped `.manic` (examples + book
    /// samples) must pass the same `check()` the browser editor runs — so a
    /// catalog/arity/syntax drift can't ship an example the editor rejects.
    /// Catches drift the static arity audit can't (e.g. verbs that read
    /// `dur`/`ease` through a shared helper).
    #[test]
    fn all_shipped_examples_pass_editor_check() {
        use std::fs;
        let root = env!("CARGO_MANIFEST_DIR");
        let mut offenders: Vec<String> = Vec::new();
        for sub in ["examples", "book/samples"] {
            let Ok(entries) = fs::read_dir(format!("{root}/{sub}")) else {
                continue;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("manic") {
                    continue;
                }
                // `test.manic` is a scratch/throwaway (often a raw model generation
                // under test) — don't let it gate the suite.
                if p.file_name().and_then(|x| x.to_str()) == Some("test.manic") {
                    continue;
                }
                let src = fs::read_to_string(&p).unwrap_or_default();
                if let Some(err) = manic_lang::services::check(&src)
                    .into_iter()
                    .find(|d| d.severity == "error")
                {
                    let name = p.file_name().unwrap().to_string_lossy();
                    offenders.push(format!("{name}: {}", err.message));
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "shipped examples the editor check() rejects (catalog/arity/syntax drift):\n  {}",
            offenders.join("\n  ")
        );
    }

    /// `polygon` is a variadic ctor (id + ≥3 points + optional colour). Guard
    /// the parse/catalog path: a well-formed call must produce no errors, and
    /// the builtin must be known to the editor.
    #[test]
    fn polygon_call_checks_clean() {
        let errs = |src: &str| {
            manic_lang::services::check(src)
                .into_iter()
                .filter(|d| d.severity == "error")
                .count()
        };
        assert_eq!(
            errs("canvas(\"16:9\");\npolygon(p, (0,0), (100,0), (50,80));\n"),
            0
        );
        assert_eq!(
            errs("canvas(\"16:9\");\npolygon(p, (0,0), (100,0), (50,80), lime);\n"),
            0
        );
        // an unknown name is still caught (sanity: check() isn't a no-op)
        assert!(errs("canvas(\"16:9\");\nnotabuiltin(p, (0,0));\n") > 0);
    }
}
