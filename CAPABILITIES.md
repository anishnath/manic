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
  scale, angle, trace, color); `rotate`/`spin`; friendly easings; per-act
  duration.
- Updaters (pure functions of `t`): `follow` (ride a target), `link`
  (edge tracks two entities), and the general `derive` hook (dynamic
  constructions — drag a vertex and dependents recompute).

### Kits
- **std** — `dot`, `circle`, `rect`, `line`, `arrow`, `brace` / `bracelabel`
  (curly brace between two points, optional label), `text`; modifiers
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
- **Numeric labels** — `markangle` with a degree value, `distance` (16) —
  needs the numeric value-tracker.
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
Fully additive — expressions collapse to literals at lowering time, so kits are
unchanged and any loop-free `.manic` behaves exactly as before. See
`examples/area_under_curve.manic` (a Riemann n-sweep 5→10→20→40 built with `for`).
Still missing (Phase 2): **user macros / functions** (`def`) to name a
parametric group, **`if`** conditionals, stepped/`downto` ranges, nested-scope
niceties, and reductions (a running sum across a loop — so numeric readouts like
a live area total still can't be *computed* in-language). Recursion (fractals,
L-systems) needs `def` + self-call.

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

## Where manic is ahead of Asymptote
- A **first-class animation timeline** — asy `animate` stitches frames; manic
  scripts beats (`par`/`seq`/`stagger`, sections, marker export) with
  deterministic recording.
- **Live dynamic constructions** — geo constructions and graph edges recompute
  as inputs move (GeoGebra-style), which static asy diagrams don't do.
