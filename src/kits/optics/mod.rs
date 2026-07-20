//! optics — light as geometry, made easy.
//!
//! A tiny, non-programmer-friendly kit: drop a `refract`/`lens`/`prism`/
//! `achromat` into a scene and watch light *do something*, with the **real
//! physics underneath** (Snell's law, Sellmeier dispersion) so the colours and
//! the focus are earned, not painted. This is emphatically *not* a lens-design
//! tool — the goldmine's engineering read-outs (surface tables, spot RMS, f/#)
//! are intentionally gone. The theme is manic; optics is just a kit.
//!
//! **Substrate — geometric, not RK4.** Optics has no time dimension: it is a
//! static, closed-form ray trace (like the collision sims' build-time
//! trajectories), producing ray polylines/arrows + glass tints + labels as
//! ordinary entities — so tag-broadcast, `cam`/`zoom`, `draw`/`show`, and
//! `template("paper")` all compose for free. **Animation is a parameter sweep:**
//! a ctor precomputes frames as one parameter varies (incidence angle, focal
//! length, wavelength) into a [`crate::scene::SimData`] playback, replayed by the
//! shared `run(id, [dur])` verb (registered by the physics kit; generic over any
//! sim in [`crate::scene::Scene::sims`]).
//!
//! **Modular by design (small files):**
//!   - [`trace`]  — the physics: 2-D Snell + TIR (later: ray–surface hits, ABCD).
//!   - `dispersion` (later) — Sellmeier n(λ) + the named glass catalog + λ→RGB.
//!   - [`builtins`] — the author-facing ctors.

mod builtins;
mod dispersion;
mod trace;

use crate::lang::lower::Registry;

pub fn register(r: &mut Registry) {
    r.ctor("refract", builtins::c_refract);
    r.ctor("lens", builtins::c_lens);
    r.ctor("prism", builtins::c_prism);
    r.ctor("achromat", builtins::c_achromat);
    r.ctor("lenssystem", builtins::c_lenssystem);
    r.ctor("rayfan", builtins::c_rayfan);
    r.ctor("spotdiagram", builtins::c_spotdiagram);
    r.ctor("fieldspot", builtins::c_fieldspot);
}

#[cfg(test)]
mod tests {
    /// `refract` lays out all parts and stores a sweep the shared `run` verb
    /// replays — proven by a clean parse + validate.
    #[test]
    fn refract_builds_parts_and_sweep() {
        let m = crate::parse("canvas(\"16:9\");\nrefract(r, (640, 360), 1.0, 1.5);\n").unwrap();
        let base = m.base();
        for sub in [
            "r.interface",
            "r.normal",
            "r.incident",
            "r.refracted",
            "r.reflected",
            "r.thetai",
            "r.thetat",
            "r.tir",
        ] {
            assert!(base.contains(sub), "missing entity `{sub}`");
        }
        // `run(r)` must resolve against the stored sweep
        let m2 = crate::parse("canvas(\"16:9\");\nrefract(r);\nrun(r, 6);\n").unwrap();
        assert!(
            m2.validate().is_ok(),
            "refract+run should validate: {:?}",
            m2.validate().err()
        );
    }

    /// `lens` lays out the axis, lens body, focal point, and both ray fans, and
    /// stores a focal-length sweep the shared `run` verb replays.
    #[test]
    fn lens_builds_rays_and_focus_sweep() {
        let m = crate::parse("canvas(\"16:9\");\nlens(l, (640, 360), 240);\n").unwrap();
        let base = m.base();
        for sub in [
            "l.axis", "l.lens", "l.focus", "l.flabel", "l.in0", "l.out0", "l.out6",
        ] {
            assert!(base.contains(sub), "missing entity `{sub}`");
        }
        let m2 = crate::parse("canvas(\"16:9\");\nlens(l);\nrun(l, 6);\n").unwrap();
        assert!(
            m2.validate().is_ok(),
            "lens+run should validate: {:?}",
            m2.validate().err()
        );
    }

    /// `prism` lays out the glass body, the white beam, and the per-colour in/out
    /// rays, and stores an incidence sweep the shared `run` verb replays. A named
    /// glass is accepted.
    #[test]
    fn prism_builds_spectrum_and_sweep() {
        let m = crate::parse("canvas(\"16:9\");\nprism(p, (540, 380), \"sf11\");\n").unwrap();
        let base = m.base();
        for sub in ["p.prism", "p.beam", "p.in0", "p.out0", "p.in8", "p.out8"] {
            assert!(base.contains(sub), "missing entity `{sub}`");
        }
        let m2 = crate::parse("canvas(\"16:9\");\nprism(p);\nrun(p, 6);\n").unwrap();
        assert!(
            m2.validate().is_ok(),
            "prism+run should validate: {:?}",
            m2.validate().err()
        );
    }

    /// `achromat` lays out both colour ray fans and the two foci, and stores the
    /// correction sweep the shared `run` verb replays.
    #[test]
    fn achromat_builds_two_foci_and_sweep() {
        let m = crate::parse("canvas(\"16:9\");\nachromat(ac, (540, 360), 120);\n").unwrap();
        let base = m.base();
        for sub in [
            "ac.lens", "ac.in0", "ac.r0", "ac.r1", "ac.b0", "ac.b1", "ac.fred", "ac.fblue",
        ] {
            assert!(base.contains(sub), "missing entity `{sub}`");
        }
        let m2 = crate::parse("canvas(\"16:9\");\nachromat(ac);\nrun(ac, 6);\n").unwrap();
        assert!(
            m2.validate().is_ok(),
            "achromat+run should validate: {:?}",
            m2.validate().err()
        );
    }

    /// `lenssystem` traces a real multi-surface prescription: it lays out glass
    /// elements + drawable rays + the sensor/spot read-outs, and each named
    /// preset (singlet/doublet/triplet) validates with `draw` + `run`.
    #[test]
    fn lenssystem_traces_presets() {
        let m =
            crate::parse("canvas(\"16:9\");\nlenssystem(ls, (640, 380), \"doublet\");\n").unwrap();
        let base = m.base();
        for sub in [
            "ls.elem0",
            "ls.axis",
            "ls.ray0",
            "ls.sensor",
            "ls.spot",
            "ls.fnum",
            "ls.na",
            "ls.bestfocus",
        ] {
            assert!(base.contains(sub), "missing entity `{sub}`");
        }
        for p in ["singlet", "doublet", "triplet"] {
            let src = format!("canvas(\"16:9\");\nlenssystem(ls, (640, 380), \"{p}\");\ndraw(ls.rays, 2);\nrun(ls, 5);\n");
            let m2 = crate::parse(&src).unwrap();
            assert!(
                m2.validate().is_ok(),
                "lenssystem {p} should validate: {:?}",
                m2.validate().err()
            );
        }
    }

    /// The T4 analysis views (`rayfan`, `spotdiagram`) build for each preset and
    /// draw clean — and a corrected doublet has a SMALLER RMS spot than a singlet.
    #[test]
    fn rayfan_and_spotdiagram_build() {
        let rf = crate::parse(
            "canvas(\"16:9\");\nrayfan(rf, (640, 360), \"singlet\");\ndraw(rf.curve, 2);\n",
        )
        .unwrap();
        assert!(rf.base().contains("rf.curve"), "rayfan curve missing");
        let sp = crate::parse(
            "canvas(\"16:9\");\nspotdiagram(sp, (640, 360), \"singlet\");\ndraw(sp.dots, 2);\n",
        )
        .unwrap();
        assert!(
            sp.base().contains("sp.ideal") && sp.base().contains("sp.dot0"),
            "spot dots missing"
        );
        // the physics: a doublet corrects — its RMS transverse aberration beats the singlet's
        let rms = |name: &str| -> f32 {
            let (dys, _, _) = super::builtins::analyze_preset(name, 0.0);
            (dys.iter().map(|(_, d)| d * d).sum::<f32>() / dys.len() as f32).sqrt()
        };
        assert!(
            rms("doublet") < rms("singlet"),
            "doublet should focus tighter than singlet"
        );
    }

    /// Lens prescriptions both ways: named presets (the easy path) AND a custom
    /// "radius thickness glass | …" string (full control) both trace + validate.
    /// The aspheric preset must actually correct spherical aberration (its RMS
    /// spot beats the same-shape spherical plano-convex).
    #[test]
    fn lenssystem_named_and_custom_prescriptions() {
        let rms = |name: &str| -> f32 {
            let (dys, _, _) = super::builtins::analyze_preset(name, 0.0);
            (dys.iter().map(|(_, d)| d * d).sum::<f32>() / dys.len() as f32).sqrt()
        };
        assert!(
            rms("aspheric") < rms("plano-convex") * 0.5,
            "asphere should roughly null spherical aberration"
        );
        for p in ["plano-convex", "meniscus", "achromat", "cooke", "singlet"] {
            let m = crate::parse(&format!(
                "canvas(\"16:9\");\nlenssystem(l, (640, 380), \"{p}\");\n"
            ))
            .unwrap();
            assert!(
                m.base().contains("l.elem0"),
                "named preset `{p}` should build glass"
            );
        }
        // custom prescription: crown + flint doublet by the numbers
        let src = "canvas(\"16:9\");\nlenssystem(l, (640, 380), \"160 26 bk7 | -140 8 f2 | -420 0 air\");\ndraw(l.rays, 2);\nrun(l, 5);\n";
        let m = crate::parse(src).unwrap();
        assert!(
            m.base().contains("l.elem0") && m.base().contains("l.ray0"),
            "custom prescription should build"
        );
        assert!(
            m.validate().is_ok(),
            "custom prescription should validate: {:?}",
            m.validate().err()
        );
    }

    /// `fieldspot` traces a 3-D pupil at a field angle: it builds the spot dots +
    /// the Airy ring for both on-axis and off-axis, and off-axis coma makes the
    /// singlet's spot larger than on-axis.
    #[test]
    fn fieldspot_on_and_off_axis() {
        for f in [0, 8] {
            let src = format!("canvas(\"16:9\");\nfieldspot(fs, (640, 360), \"singlet\", {f});\ndraw(fs.dots, 1);\n");
            let m = crate::parse(&src).unwrap();
            assert!(
                m.base().contains("fs.dot0") && m.base().contains("fs.airy"),
                "field {f} should build a spot + Airy ring"
            );
            assert!(
                m.validate().is_ok(),
                "fieldspot {f} should validate: {:?}",
                m.validate().err()
            );
        }
    }
}
