# manic — capabilities & gaps

A snapshot of what manic can do today vs. what it can't, grounded against the
Asymptote example corpus (520 `.asy` files: 117 `geometry/`, ~197
`generalities/`, ~34 `graph/`, ~96 3D across `three`/`graph3`/`solids`/`tube`/
`grid3`, plus generative folders) and the Manim references. Usage counts below
are occurrences across the `geometry/` samples.

## Capabilities (implemented)

### Engine & language
- Stateless timeline (`Timeline::apply(base, t)` is pure) → free scrub/step,
  deterministic recording (mp4/gif/PNG), live preview, CRT post-process.
- ASY-like DSL: function-call statements, `(x, y)` points, `;` terminators,
  `//` comments, `par` / `seq` / `stagger` blocks, `section`, `wait`/`beat`,
  `mark`; dotted ids; **tag broadcast** (a verb/modifier on a tag hits the whole
  group); line/column error diagnostics.
- Animation: named verbs + a general `to(id, property, value)` (x, y, opacity,
  scale, angle, trace, color, **hue** — cycles around the colour wheel);
  `rotate`/`spin`; friendly easings; per-act duration.
- Updaters (pure functions of `t`): `follow` (ride a target), `link`
  (edge tracks two entities), and the general `derive` hook (dynamic
  constructions — drag a vertex and dependents recompute).

### Kits
- **std** — `dot`, `circle`, `rect`, `line`, `arrow`, `brace` / `bracelabel`
  (curly brace between two points, optional label), `text`, `counter` (live
  numeric readout); modifiers
  (`hidden`, `untraced`, `color`, `hue` (HSL, computable per-entity),
  `outline`/`outlined`/`filled`, `size`,
  `stroke`, `glow`, `z`, `rot`, `opacity`, `bold`, `display`, `label` [offset],
  `tag`); ~20 verbs (`show`, `fade`, `move`, `shift`, `grow`, `draw`, `erase`,
  `type`, `say`, `recolor`, `flash`, `pulse`, `shake`, `scale`, `rotate`,
  `spin`, `to`/`set`, `cam`, `zoom`); boolean ops `union`/`intersect`/
  `difference`/`exclusion`.
- **math** — `axes` (optional ticks + labels), `plane`/`numberplane`,
  `complexplane`, `polarplane`, `plot` (12 named functions), `numberline`,
  `vector`, `arc`, `sector`, `annulus`, `pie`, `arrowfield` (8 named vector
  fields, magnitude-coloured), `matrix` (bracketed, row/column addressable via
  tags), `table` (ruled grid + optional row/col labels; cells, rows, columns,
  labels and grid lines all addressable via tags).
- **algo** — `graph` (undirected `a-b` / directed `a>b`, circular/row/grid
  layouts, reflowing edges, tag groups).
- **geo** — `point`, `segment` (reflows), derived points `midpoint`,
  `centroid`, `circumcenter`, `incenter`, `orthocenter`, `foot`, `meet`;
  `circumcircle`, `incircle`; `anglemark`, `rightangle`. All constructions are
  **dynamic** (recompute as their inputs move).
- **brand** — `banner` (icon trio + "manic" wordmark, create→expand→unwrite)
  and `watermark` (screen-fixed persistent mark).

### Primitives (engine)
`Circle`, `Rect`, `Line`, `Arrow`, `Curve`, `Polygon`, `Polyline`, `Arc`
(arc/sector/annulus), `Region` (boolean result), `Text`.

## Gaps

### Geometry (olympiad) — core covered; missing
- **Intersections** beyond line∩line — line∩circle, circle∩circle
  (`intersectionpoints`, 73). We have only `meet` (line∩line).
- **Tangents** (`tangent`/`tangents`, 36) — tangent to a circle; tangents from
  an external point.
- **`bisector`** (14); **`reflect`**; rotate/transform a whole object.
- **Infinite `line`** that extends to the frame vs. our finite `segment`
  (`line`/`drawline`, ~68).
- **Conics:** `ellipse` (32), `parabola` (20), `hyperbola` (14) — none.
- **Point-on-curve by parameter** (`angpoint`/`curpoint`/`relpoint`/
  `*abscissa`, ~300 combined) — a point at an angle / arc-length / relative
  position along a path. Common in asy; advanced.
- **Skew coordinate systems** (`cartesiansystem`, 113) — niche.
- **Numeric labels** — `markangle` with a degree value, `distance` (16). The
  `counter` readout + `value` track cover *animated* / computed numbers; what's
  still missing is binding one to a *live geometry measurement* (a length that
  updates as a vertex is dragged) — would wire the `derive` hook into a counter.
- **Minor:** a `circle` centred on a *point* (dynamic) — cheap add.

### Graphing (math) — partial
- Expression plots DONE — `plot` takes a formula string in `x`/`t`
  (`"cos(x) + 0.5*cos(7*x)"`, arithmetic + ~20 functions), manic's
  `FunctionGraph`. (`arrowfield` still takes named fields only — its lambda
  would want the same evaluator extended to two variables.)
- `plot` range may be a scalar `domain` (symmetric) or an explicit `(x0, x1)`
  pair (one-sided) — `plot(g,(cx,cy),200,52,"x*x",(0,2.5))`.
- Coordinate frames done: `axes` (ticks + integer labels), `plane`/
  `numberplane`, `complexplane`, `polarplane`. Still missing: custom
  tick-label values / non-integer steps, per-axis limits, multiple styled axes,
  3D axes (deferred).
- **Area under a curve** works today via Riemann rectangles (`rect` bars under
  a `plot`) — see `examples/area_under_curve.manic` (midpoint sum converging to
  the integral). No smooth `Region`-based fill of the exact area yet, and — with
  no loops — the bars must be enumerated (that example is generated). No
  legends or data/scatter plots.
- Vector fields: `arrowfield` done; **`StreamLines`** (flowing-agent traces)
  not done — needs a flow simulation + the animation flow (a good fit for a
  future updater-driven feature).

### Transforms / morphing (Manim `Transform` family)
The dividing line: manic animates entity **properties** (position, endpoint,
color, scale, rotation, opacity, trace, hue, value) but does **not** morph one
shape's **geometry** into another's.

- **Have (full):**
  - `ApplyMethod` → our verbs `move`/`shift`/`scale`/`rotate`/`spin`/`recolor`/
    `to`/`set`.
  - `ScaleInPlace` → `scale(id, f)`; `ShrinkToCenter` → `scale(id, 0)`.
  - `FadeToColor` → `recolor`.
  - `MoveToTarget` → `to`/`move` straight to the target (no `generate_target`
    two-step, same result).
- **Partial (expressible, no dedicated builtin):**
  - `Swap`, `CyclicReplace` → hand-written `move`s (a `for` loop for cycles).
  - `FadeTransform` / `FadeTransformPieces` → crossfade `par { fade(a); show(b); }`
    — not point-matched.
  - `Restore` → the revert machinery exists internally (`pulse`/`flash`
    auto-restore) but there is no user-facing `save`/`restore` verb.
  - `ApplyMatrix`, `ApplyPointwiseFunction[ToCenter]`, `ApplyComplexFunction` →
    now expressible over a **set of dots/vectors** via the loop+expression layer
    (compute `M·p` / `f(z)` per point and `to` it). Cannot warp a single
    parametric shape, and there is no builtin (a `transform(group, a,b,c,d)`
    linear-transform verb is the highest-value candidate — 3b1b-style).
- **None (missing):**
  - `Transform` / `ReplacementTransform` — true shape morph (e.g. circle outline
    → square outline). Needs primitives to expose sampled points + a
    correspondence rule; the core prerequisite for this whole row.
  - `TransformFromCopy` — no entity-copy primitive.
  - `ClockwiseTransform` / `CounterclockwiseTransform` — depend on morph.
  - `TransformAnimations` — transforming between two animations; rare/advanced.

### Creation / reveal (Manim `Creation` family)
Built on manic's `trace` property (draw-on for strokes = fraction of path/
outline traced with fills fading in; for text = typewriter char count).

- **Have (full):**
  - `Create` → `draw(id)` (declare `untraced` first).
  - `Uncreate` → `erase(id)` (trace back to 0).
  - `ShowPartial` → the `trace` prop *is* this mechanism (animate `to(id,
    trace, u)` to any fraction).
  - `AddTextLetterByLetter` → `type(id)` (typewriter).
  - `RemoveTextLetterByLetter` → reverse typewriter (`erase` / `to(id, trace,
    0)` on text).
- **Partial (expressible / approximate, no dedicated builtin):**
  - `Write` / `Unwrite` → `draw`/`type` in, `erase` out. We do path-trace +
    fill-fade / typewriter, **not** calligraphic stroke-by-stroke handwriting of
    glyph outlines (that needs glyph-outline stroking, tied to the font/LaTeX
    work).
  - `DrawBorderThenFill` → `draw` traces the border and fades the fill *together*;
    sequencing border-fully-then-fill is scriptable but not one call.
  - `AddTextWordByWord` → typewriter is char-by-char only; no word granularity.
  - `TypeWithCursor` / `UntypeWithCursor` → typewriter without a cursor glyph.
  - `ShowIncreasingSubsets` → `stagger { for i in 0..n { show(x{i}); } }` over a
    tagged group (cumulative reveal).
  - `ShowSubmobjectsOneByOne` → a `seq` of show/hide (flipbook, one at a time).
- **None (missing):**
  - `SpiralIn` → no path-based entrance (positions interpolate linearly). Fakeable
    by placing pieces at spiral offsets and moving them in with a loop, but there
    is no spiral/path-motion builtin.

### Growing (Manim `Growing` family)
manic can animate `scale`, `spin`, and the line/arrow endpoint (`grow`), but has
no modifier to set an *initial* scale and no bounding box — so "appear by growing
out of nothing" and edge/point origins are scriptable rather than one call.

- **Have (full):**
  - `GrowArrow` → `grow(id, (x,y))` extends a line/arrow/curve endpoint to a
    point (declare it zero-length, then `grow` to full).
- **Partial (expressible, no dedicated builtin):**
  - `GrowFromCenter` → `scale` animates uniform scale, but there is no
    initial-scale modifier, so growing from nothing needs a
    `seq { scale(id,0,0); scale(id,1,d); }` trick.
  - `GrowFromPoint` → scale + a `move`/`shift` originating at the point.
  - `SpinInFromNothing` → `par { scale(id,1,d); spin(id,360,d); }` (compose the
    grow trick with `spin`).
- **None / needs prerequisites:**
  - `GrowFromEdge` → needs a bounding box to find the edge (same missing
    entity-bbox that blocks `Brace(mobject)` and `GrowFromPoint` automation);
    doable today only by supplying the edge point yourself.
- **Cheap win:** an initial `scale` modifier + a `growin`/`popin` verb (scale
  0→1 about the anchor) would move `GrowFromCenter` / `SpinInFromNothing` to
  full support in a few lines.

### 3D — none (deferred by design)
`graph3` / `three` / `solids` / `tube` / `grid3` ≈ 96 asy files. Planned:
ROADMAP → "3D — planned" (Phase 1 additive camera + `Path3`, Phase 2 full Vec3).

### Generative / repetitive — loops + variables + arithmetic DONE
manic now has a computation layer, evaluated before the scene is built:
- **`let name = expr;`** numeric variables;
- **arithmetic** (`+ - * / ^`, unary `-`, parens, `pi`/`e`/`tau`, ~20 functions)
  usable anywhere a number or `(x,y)` coordinate goes;
- **`for v in a..b { … }`** range loops;
- **id interpolation** `bar{i}` so a loop generates unique entities (then
  `tag` them into a group to animate together).
Plus, since Phase 2:
- **`def name(params) { … }`** macros — reusable parametric groups, and they may
  **recurse** (with a depth guard), so fractals/trees are a few lines
  (`examples/fractal_tree.manic`);
- **`if cond { } else { }`** (and `else if`) with comparisons `< <= > >= == !=`
  and logic `&& ||` — recursion base cases, conditional figures.
Fully additive — expressions collapse to literals at lowering time, so kits are
unchanged and any plain `.manic` behaves exactly as before. Examples:
`area_under_curve.manic` (a `for` n-sweep), `fractal_tree.manic` (recursive
`def`), `riemann_rainbow.manic` (loop + `hue` + `stagger`).
- **Reductions** — `sum(i in a..b : expr)` (also `prod`/`min`/`max`) aggregate
  over a range, so totals are computable in-language; paired with a `counter`
  entity + animatable `value` track, a computed number **counts up live** on
  screen (`examples/riemann_readout.manic`: a Riemann area summed and tweened).
Still missing: stepped/`downto` ranges, string/name variables (macro params are
numeric), and general per-frame data binding (a readout that reflects a moving
entity's live measured length/position — needs the `derive` hook to feed a
counter, not yet wired).

### Typography
- No LaTeX / math typesetting (`$…$`, `\frac`, matrices) — mono text only.
  **Approach under evaluation** (not yet decided):
  - **ReX** (`rex` crate) — pure-Rust math-mode LaTeX; lays out with an
    OpenType MATH font and emits glyph outlines + rules through a backend trait,
    so equations become ordinary manic paths (draw-on / color / glow for free).
    Self-contained; a subset of LaTeX; some API churn to track.
  - **`pulldown-latex`** — pure-Rust LaTeX-math → MathML; would need a MathML
    render step.
  - **Full TeX** (`latex` + `dvisvgm` → SVG → paths) — 100% fidelity but a
    per-user system dependency; possible as an *optional* backend when TeX is
    present, falling back to a pure-Rust path otherwise.
  - **mathtext-lite** — homegrown Unicode + super/subscript + `\frac`/`\sqrt`
    layout; least fidelity, zero deps, keeps the mono look.
- **Custom / selectable fonts — planned, not yet designed.** Today all text is
  IBM Plex Mono (regular/bold/display). A future capability: let the author pick
  fonts (per entity or globally) and load user-supplied font files. Tracked here
  so it isn't lost; no timeline yet. (Also unblocks a non-serif look for any
  future LaTeX backend.)

## What's required next & how to address it

The gaps above are the *symptoms*; this is the *plan*. **Guiding principle:
extend a few existing mechanisms so each covers a whole cluster of missing
features — do NOT add one builtin per Manim/Asymptote class.** Almost every gap
found so far maps onto one of six foundational extensions:

| # | Requirement | How to address (extend what) | Effort | Unlocks |
|---|---|---|---|---|
| 1 | **Entity bounding box** | Add a `bbox(entity)` in the engine, reusing the shape→points extraction already in `geom.rs` (used for boolean ops). | Small | `Brace(mobject)`, `GrowFromEdge`/`FromPoint`, `FocusOn`/`Circumscribe`, `next_to`-style relative placement, group framing. |
| 2 | **Entrance verbs** | New `growin`/`popin` verb + an initial-`scale` modifier (scale 0→1 about the anchor), on top of the existing `scale`/`spin`. | Tiny | `GrowFromCenter`, `SpinInFromNothing`, clean "appear from nothing". |
| 3 | **Move-along-path** | Extend the `Pos` track / `derive` updater to interpolate along a `Curve`/`Polyline` (both already exist) instead of a straight line. | Medium | `MoveAlongPath`, `SpiralIn`, orbit, and a **point riding a plotted curve** (tangent/particle — key for calculus). |
| 4 | **Linear-transform verb** | `transform(group, a,b,c,d)` that applies a 2×2 matrix to a tagged group over time — formalises what the loop layer can already compute (`M·p` per point). | Medium | `ApplyMatrix`, linear-algebra viz (grid shear, eigenvectors, det-as-area); stepping stone to `ApplyComplexFunction` (pass a formula). |
| 5 | **Live geometry readout** | Wire the `derive` hook to feed a `counter`'s `value` (both already exist) so a measured length/angle updates as inputs move. | Small | Olympiad numeric labels (`distance`, `markangle` value), any dragged-measurement display. |
| 6 | **Shape-morph (point sampling)** | Give primitives a sampled-point form + a correspondence rule — genuinely new machinery. | Large | `Transform`/`ReplacementTransform`/`TransformFromCopy`, calligraphic `Write`, morphing plots. **Deferred** — its everyday use (A→B) is fakeable with crossfade/redraw today. |

Recommended order: **2 → 1 → 5 → 3 → 4**, leaving **6** deferred. Items 1, 2,
and 5 are small and reuse machinery that already exists; together with 3 they
close most of the Growing / Creation / Indication gaps *and* deliver the
calculus "point on a curve" move — a dozen-plus Manim animations from a handful
of modest extensions. Two prerequisites recur and are the real leverage:
**bounding box (#1)** and **path-motion (#3)**.

Separately tracked, larger and orthogonal: **LaTeX/math typesetting** (approach
under evaluation, see Typography), **selectable fonts**, and **3D** (deferred by
design).

## Where manic is ahead of Asymptote
- A **first-class animation timeline** — asy `animate` stitches frames; manic
  scripts beats (`par`/`seq`/`stagger`, sections, marker export) with
  deterministic recording.
- **Live dynamic constructions** — geo constructions and graph edges recompute
  as inputs move (GeoGebra-style), which static asy diagrams don't do.
