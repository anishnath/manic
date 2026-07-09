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
  (`hidden`, `untraced`, `color`, `outline`/`outlined`/`filled`, `size`,
  `stroke`, `glow`, `z`, `rot`, `opacity`, `bold`, `display`, `label` [offset],
  `tag`); ~20 verbs (`show`, `fade`, `move`, `shift`, `grow`, `draw`, `erase`,
  `type`, `say`, `recolor`, `flash`, `pulse`, `shake`, `scale`, `rotate`,
  `spin`, `to`/`set`, `cam`, `zoom`); boolean ops `union`/`intersect`/
  `difference`/`exclusion`.
- **math** — `axes` (optional ticks + labels), `plane`/`numberplane`,
  `complexplane`, `polarplane`, `plot` (12 named functions), `numberline`,
  `vector`, `arc`, `sector`, `annulus`, `pie`, `arrowfield` (8 named vector
  fields, magnitude-coloured), `matrix` (bracketed, row/column addressable via
  tags).
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
- Coordinate frames done: `axes` (ticks + integer labels), `plane`/
  `numberplane`, `complexplane`, `polarplane`. Still missing: custom
  tick-label values / non-integer steps, per-axis limits, multiple styled axes,
  3D axes (deferred).
- No area fill under a curve, legends, or data/scatter plots.
- Vector fields: `arrowfield` done; **`StreamLines`** (flowing-agent traces)
  not done — needs a flow simulation + the animation flow (a good fit for a
  future updater-driven feature).

### 3D — none (deferred by design)
`graph3` / `three` / `solids` / `tube` / `grid3` ≈ 96 asy files. Planned:
ROADMAP → "3D — planned" (Phase 1 additive camera + `Path3`, Phase 2 full Vec3).

### Generative / repetitive — none (language-level gap)
manic has **no variables, loops, arithmetic, or user functions**. Asymptote
uses `for` and recursion throughout `fractales`, `lsystem`, `tiling`,
`randomwalk`, and grid figures. Any figure built by iteration is currently
impossible in one `.manic` file. This is the biggest structural gap — closing
it means adding a small expression/loop layer to the language.

### Typography
- No LaTeX / math typesetting (`$…$`, `\frac`, matrices) — mono text only.
  Deferred (ROADMAP → "hard 20%").

## Where manic is ahead of Asymptote
- A **first-class animation timeline** — asy `animate` stitches frames; manic
  scripts beats (`par`/`seq`/`stagger`, sections, marker export) with
  deterministic recording.
- **Live dynamic constructions** — geo constructions and graph edges recompute
  as inputs move (GeoGebra-style), which static asy diagrams don't do.
