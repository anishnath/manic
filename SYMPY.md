# SymPy → manic (future: a generic step-animator)

> **Status: note only — not built yet.** `examples/rewrite-integration.manic` was
> produced *by hand* this once from a SymPy step dump. This file records how, and
> what a generic pipeline would need, so we can revisit and automate it later.

## The idea

SymPy's `manualintegrate` (and the wider `stepbystep` machinery) emits a tree of
named rules (`URule`, `RewriteRule`, `AddRule`, `ConstantTimesRule`,
`ReciprocalRule`, …), each with a LaTeX rendering of its current expression. That
maps almost 1:1 onto a manic scene:

- one `equation(work, …)` object, morphed step-by-step with `rewrite(work, `<latex>`, t, ease)`
- `stepTitle` / `ruleTag` captions driven by each step's `title` / `rule`
- a boxed final `RESULT`, revealed at the end

So a **generic generator** would take a CAS step dump (JSON) and emit a `.manic`
file — turning any worked solution (integrals today; derivatives, equation
solving, limits, series tomorrow) into a captioned rewrite animation. Not limited
to SymPy: anything that can produce `[{title, latex, rule}]` + `result` fits.

## What the manual pass had to handle (the automation must too)

1. **Double-JSON-escaped LaTeX.** The dump arrived as JSON-in-JSON, so the LaTeX
   was escaped twice. Decode with two passes of a JSON string-unescape (we used
   `json.loads` twice), *not* by eyeballing backslashes.

2. **Over-escaped control sequences.** After decoding, tokens like `\\int`,
   `\\frac`, `\\quad`, `\\,` came through as double-backslash where none were real
   LaTeX line breaks. We collapsed `\\` → `\`. A generic tool needs a rule for
   this: for single-line expressions, `\\` is never a break, so collapse; only
   preserve `\\` inside a multi-line env (`aligned`, `cases`, `matrix`).

3. **Backtick raw strings, not double quotes.** manic's `"…"` is LaTeX-safe but
   still treats `\\` and `\"` specially; backticks are fully raw. Emit LaTeX args
   in backticks so `\,`, fractions and subscripts survive verbatim. (If content
   could contain a backtick, escape/normalize first — CAS output generally won't.)

4. **Fit / sizing.** The widest step (a long integral equality) drove the font
   size down to ~34 to stay inside the panel. A generic tool should measure each
   step (we have `layout_dims` in the LaTeX path) and pick a size that fits the
   widest, or auto-shrink per step.

5. **CAS quirks to decide policy on:**
   - SymPy reuses `_u` for *every* nested substitution, so steps literally read
     "Let `u = u + 3/2`" and show `d_u`. We kept it verbatim (faithful). A generic
     mode could relabel nested substitutions `u, v, w, …` for readability — make
     it a flag.
   - Rule names are internal jargon (`URule`, `ConstantTimesRule`). Consider a
     display map (`URule → "u-substitution"`, `ConstantTimesRule → "constant
     factor"`, `ReciprocalRule → "∫1/u = ln|u|"`) — again, a flag.
   - The `RULES=` field in the dump is the full alternative tree; we only used the
     linear `STEPS` list. The tree could drive branching / "alternative method"
     views later.

## Sketch of a generic generator

```
cas-steps.json  ──►  gen-steps.(py|rs)  ──►  <name>.manic
   {title,latex,rule}[]                        equation(work,…) + N× rewrite()
   result, expr                                + captions + boxed result
   (+ optional flags: relabel-subs, rule-display-map, per-step-size)
```

Open questions for later:
- Where does the generator live — a `scripts/` helper, or a first-class `manic`
  subcommand / kit builtin that ingests a step list?
- Do we want a small manic *builtin* (e.g. `steps(work, <json-or-file>)`) that
  expands to the rewrite sequence at lower time, so authors don't post-process?
- Provenance: keep the CAS/source out of the *rendered* output (done — no on-screen
  credit), but record it in a comment or sidecar for reproducibility.

## Reference artifact

`examples/rewrite-integration.manic` — the hand-built exemplar this note is based
on (∫ 1/((x−1)(x+2)) dx, 21 steps, partial fractions by substitution).
