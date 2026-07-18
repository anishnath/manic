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
- ASY-like DSL: function-call statements, `(x, y)` points and `(x, y, z)` 3D
  points, `;` terminators,
  `//` comments, `par` / `seq` / `stagger` blocks, `section`, `wait`/`beat`,
  `mark`; dotted ids; **tag broadcast** (a verb/modifier on a tag hits the whole
  group); line/column error diagnostics.
- **Computation layer** (evaluated at build time): `let` variables; arithmetic
  `+ - * / ^` with **implicit multiplication** (`2sx`, `3(x+1)`), comparisons,
  logic, `pi`/`e`/`tau`, ~20 functions; `for v in a..b` loops; `if/else`;
  recursive `def` macros; reductions `sum`/`prod`/`min`/`max`; id interpolation
  (`bar{i}`). All collapse to literals before rendering — kits are unaffected.
- **Look / config**: `canvas` accepts pixels or presets (`"16:9"`/`"square"`/
  `"portrait"`/`"1080p"`/`"4k"`); `w`/`h`/`cx`/`cy` predefined. Selectable
  **templates** — `mono` (default black-and-white editorial), `plain`,
  `terminal`, `paper`, `blueprint`, `shorts` — each retints the palette and sets
  chrome/glow/CRT; author-set `masthead` (no engine branding baked in). Same
  content renders in any template.
- Animation: named verbs + a general `to(id, property, value)` (x, y, opacity,
  scale, angle, trace, color, **hue** — cycles around the colour wheel, and
  **value** — a live `counter`'s number); `rotate`/`spin`; camera `cam`/`zoom`;
  friendly easings; per-act duration.
- Updaters (pure functions of `t`): `follow` (ride a target), `link`
  (edge tracks two entities), and the general `derive` hook (dynamic
  constructions — drag a vertex and dependents recompute).

### Kits
- **std** — `dot`, `circle`, `rect`, `line`, `arrow`, `brace` / `bracelabel`
  (curly brace, optional label), `text`, `counter` (live numeric readout),
  `morph` (set a shape up to morph into another), `copy` (duplicate an entity),
  `caption` (word-by-word text row + `karaoke`/`wordpop` verbs);
  modifiers (`hidden`, `untraced`, `cursor` (typewriter `_` on text), `color`,
  `hue` (HSL, computable per-entity), `outline`/`outlined`/`filled`, `size`,
  `stroke`, `glow`, `z`, `rot`, `opacity`, `bold`, `display`, `label` [offset],
  `tag`); verbs (`show`, `fade`, `move`, `shift`, `grow`, `draw`, `erase`,
  `type`, `say`, `recolor`, `flash`, `pulse`, `shake`, `scale`, `rotate`,
  `spin`, `swap`, `transform` (2×2 matrix / ApplyMatrix), `to`/`set`, `cam`,
  `zoom`); boolean ops `union`/`intersect`/`difference`/`exclusion`.
- **math** — `axes` (optional ticks + labels), `plane`/`numberplane`,
  `complexplane`, `polarplane`, `plot` (named functions **or a formula string**
  like `"cos(x)+0.5*sin(3*x)"`; symmetric or one-sided `(x0,x1)` range),
  `numberline`, `vector`, `arc`, `sector`, `annulus`, `pie`, `arrowfield` (8
  named vector fields, magnitude-coloured), `matrix` (bracketed, row/column
  addressable via tags), `table` (ruled grid + optional row/col labels; cells,
  rows, columns, labels and grid lines all addressable via tags).
- **algo** — `graph` (undirected `a-b` / directed `a>b`, circular/row/grid
  layouts, reflowing edges, tag groups); `array` (row of fixed slot boxes
  `{id}.box{k}` + value cells `{id}.c{k}`) with `compare(a,i,j)` (flash the
  values now in two slots) and stateful `swap(a,i,j)` (slide them into the
  swapped slots, chaining correctly across a whole sort — see
  `examples/bubble_sort.manic`); `pointer(id, arr, slot, [label])` + `pointat(id,
  arr, slot)` — a labelled index caret that slides between slots (two-pointer /
  traversal, `examples/two_pointer.manic`); `stack`/`queue` with `push`/`pop`
  and `enqueue`/`dequeue` — dynamic structures that add cells and animate them
  in/out, tracking occupancy so chains of ops compose (`dequeue` also advances
  the cells behind); `caret(id, (x,y), "label", dir)` — a rigid labelled marker
  you `move` to track an action point (stack top, queue front/back). See
  `examples/stack_queue.manic`. `list(id, "3 8 5", (cx,cy), kind, [cw], [ch])` —
  a **linked list** with the classic node anatomy: framed boxes split into
  compartments (`[data│•next]` singly, `[•prev│data│next•]` doubly) with pointer
  dots, a `head` pointer and a `NULL` terminator (or a wrap-to-head curve).
  `kind` ∈ `singly`/`doubly`/`circular`. `insert(id, after, "v")` splices a node
  in below the gap and re-threads the pointers (no row shift); `remove(id, i)`
  unlinks and re-points around it. See `examples/linked_list.manic`. `bfs(g,
  start)` / `dfs(g, start)` — graph traversal: reads the graph's adjacency,
  runs the algorithm, and animates the classic states (discovered → current →
  done) with tree edges lighting up and live `queue:`/`stack:` + `visited:`
  readouts (BFS = queue, DFS = stack; directed edges followed one way). See
  `examples/bfs_dfs.manic`. **Weighted** edges: `a-b:7` gives an edge a weight
  (drawn as a midpoint label). `dijkstra(g, start)` — single-source shortest
  paths: each node shows a live distance (`inf` → final), the nearest unsettled
  node settles (magenta → lime), relaxed edges light, and the shortest-path-tree
  edges stay lit. See `examples/dijkstra.manic`. `hashmap(id, n, (cx,cy))` — `n`
  buckets in a column; `put(id, k, v)` hashes the key (byte-sum mod n) to a
  bucket and chains the `k:v` entry on (collisions extend the chain);
  `get(id, k)` hashes then scans that bucket's chain, flashing each entry until
  the key matches (lime) or the chain ends (miss). Separate chaining, composed
  from the array (buckets) + list (chains). See `examples/hashmap.manic`.
- **geo** — all **dynamic** (recompute as inputs move): `point`, `segment`;
  centres `midpoint`/`centroid`/`circumcenter`/`incenter`/`orthocenter`/`foot`;
  intersections `meet` (line∩line), `linecircle`, `circlecircle` (two points
  each); `tangent` (touch points from an external point); `reflect`, `bisector`,
  `rotpoint`, `between`, `anglepoint`; circles `circumcircle`/`incircle`/
  `circle2`; conics `ellipse`/`parabola`/`hyperbola`; `fullline` (infinite);
  `anglemark`, `rightangle`.
- **brand** — `banner` (icon trio + "manic" wordmark, create→expand→unwrite)
  and `watermark` (screen-fixed persistent mark).
- **three** — hybrid depth-tested 3D under the normal 2D overlay: `camera3`
  (perspective/orthographic Z-up orbit camera), `point3`, `line3`, `arrow3`,
  `cube3`, `sphere3`, `grid3`, `axes3` (ticks + numbers), plus `pin3` (glue a 2D
  label to a 3D point), `follow3` (track another entity), `midpoint3` (derived
  point), `curve3` (parametric 3D curve), `surface3` (z=f(x,y) filled mesh), `param3` (parametric surface — tori/Möbius), `prism3`/`pyramid3`/`revolve3`
  (filled, flat-shaded solids), `extrude3` (extrude a 2D shape/boolean-region → CSG solids),
  `thick` (tube strokes); verbs `move3`, `shift3`, `rotate3`,
  `grow3`, `orbit3`, `look3`. Shared modifiers/verbs (`color`, `opacity`,
  `hidden`, `untraced`, `tag`, `show`, `fade`, `draw`, `recolor`, `flash`,
  `pulse`, `scale`) also address 3D entities. See **3D foundation** below.

### Primitives (engine)
`Circle`, `Rect`, `Line`, `Arrow`, `Curve`, `Coil` (spring zigzag pos→to,
stretches via the `To` prop), `Polygon`, `Polyline`, `Arc`
(arc/sector/annulus), `Region` (boolean result), `Text`; 3D `Point`, `Line`,
`Arrow`, `Cube`, `Sphere`, and XY `Grid`.

### 3D foundation
- **Coordinates & scene model** — computed `(x,y,z)` values flow through the
  parser, macro expander, lowering, editor services, and runtime. 3D entities
  have stable ids and tags alongside the existing 2D scene.
- **Camera** — one Z-up orbit camera with perspective or orthographic
  projection. `camera3` sets its eye, target, and field of view (a single value,
  reused as the orthographic height), plus the projection; `orbit3` animates
  azimuth, elevation, and radius, while `look3` animates the target.
- **Rendering & output** — depth-tested 3D renders beneath the normal 2D
  overlay. Preview, stills, CRT output, and recordings all use the same
  depth-enabled render target. Render-target Y correction keeps screen
  orientation consistent, with positive Z visibly pointing up.
- **Geometry** — points, lines, arrows, cubes, spheres, XY floor grids, and
  ticked, numbered XYZ axes (`axes3`, optional `step`). Objects support position,
  non-uniform scale, Euler rotation, color, opacity, visibility, and tracing state.
- **Animation** — deterministic `Vec3` timeline tracks drive `move3`, `shift3`,
  `rotate3`, and `grow3` (which retargets a `line3`/`arrow3` endpoint rather than
  scaling). Shared `show`, `fade`, `draw`, `recolor`, `flash`, `pulse`, and
  `scale` verbs also address 3D entities and tag groups.
- **Projected labels** — `pin3(label, point3 | entity3)` binds an existing 2D
  `text`/`label` to a 3D position; a world→screen projection reprojects it every
  frame, so the label stays glued as the camera orbits (or the target entity
  moves). This is the reusable hook behind future ticked/numbered 3D axes.
- **Reference** — `examples/three_d.manic` exercises the camera, depth,
  primitives, axes, transforms, a pinned label, and hybrid 2D/3D composition.

## Gaps

### Geometry (olympiad) — largely covered now
Done (all **dynamic** unless noted): `meet` (line∩line), **`linecircle`**
(line∩circle), **`circlecircle`** (circle∩circle) — the last two output two
points `{id}0/1`; **`tangent`** (two touch-points from an external point); **`commontangent`**
(a common tangent to TWO circles — external/direct or internal/transverse — as the
segment between the touch points, so its length is the tangent length `√(d²−(r₁∓r₂)²)`;
static);
**`reflect`** (point across a line); **`bisector`** (point on the internal angle
bisector); **`circle2`** (circle by centre + a point on it); **`rotpoint`**
(point rotated about another by θ — gives equilateral apexes, regular figures);
**`between`** (point at fraction `t` along a segment — relpoint); **`anglepoint`**
(point on a circle at an angle); **`fullline`** (line extended across the frame);
**`ellipse`** (rotatable outline, static). Circles are given as centre + a point
on them so the radius stays dynamic. Examples `examples/tangents.manic`.
**Conics complete:** `ellipse`, `parabola` (vertex + width/height), `hyperbola`
(two branches `{id}.r`/`{id}.l`) — see `examples/conics.manic`.
Still missing (minor):
- **Rotate a whole construction** at once (you can `rotpoint` each vertex).
- **Point-on-curve by arc-length** (`between` covers relative position on a
  segment; arc-length along an arbitrary path is not done).
- Foci/directrix as *constructed* elements of a conic (the conics are drawn
  outlines, not point-defined loci).
- **Skew coordinate systems** (`cartesiansystem`, 113) — niche.
- **Numeric labels** — `markangle` with a degree value, `distance` (16). The
  `counter` readout + `value` track cover *animated* / computed numbers; what's
  still missing is binding one to a *live geometry measurement* (a length that
  updates as a vertex is dragged) — would wire the `derive` hook into a counter.

### Graphing (math) — partial
- Expression plots DONE — `plot` takes a formula string in `x`/`t`
  (`"cos(x) + 0.5*cos(7*x)"`, arithmetic + ~20 functions), manic's
  `FunctionGraph`. (`arrowfield` still takes named fields only — its lambda
  would want the same evaluator extended to two variables.)
- `plot` range may be a scalar `domain` (symmetric) or an explicit `(x0, x1)`
  pair (one-sided) — `plot(g,(cx,cy),200,52,"x*x",(0,2.5))`.
- Coordinate frames done: `axes` (ticks + integer labels), `plane`/
  `numberplane`, `complexplane`, `polarplane`, plus foundational `axes3` and
  `grid3`. Still missing: custom tick-label values / non-integer steps,
  per-axis limits, multiple styled axes, and labelled/ticked 3D axes.
- **Area under a curve** works today via Riemann rectangles (`rect` bars under
  a `plot`) — see `examples/area_under_curve.manic` (midpoint sum converging to
  the integral). No smooth `Region`-based fill of the exact area yet, and — with
  no loops — the bars must be enumerated (that example is generated). No
  legends or data/scatter plots.
- Vector fields: `arrowfield` done; **`StreamLines`** (flowing-agent traces)
  not done — needs a flow simulation + the animation flow (a good fit for a
  future updater-driven feature).

### Transforms / morphing (Manim `Transform` family)
Two kinds: **property** transforms (position, endpoint, colour, scale, rotation,
opacity, trace, hue, value) — all covered; and **geometry** transforms — a
linear map of space (`transform`), outline shape-morph (`morph`, with winding),
and entity `copy` — now covered too. Essentially the whole family; only
`TransformAnimations` is N/A by design (see below).

- **Have (full):**
  - `ApplyMethod` → our verbs `move`/`shift`/`scale`/`rotate`/`spin`/`recolor`/
    `to`/`set`.
  - `ScaleInPlace` → `scale(id, f)`; `ShrinkToCenter` → `scale(id, 0)`.
  - `FadeToColor` → `recolor`.
  - `MoveToTarget` → `to`/`move` straight to the target.
  - **`ApplyMatrix`** → **`transform(group, (ox,oy), a,b,c,d, [dur], [ease])`** —
    applies a 2×2 matrix about an origin to every entity in a tagged group
    (anchor + line/arrow endpoints), so a grid + basis vectors + points shear /
    rotate together (the 3b1b linear-map-of-space visual). See
    `examples/linear_transform.manic`. Correct for dots/lines/vectors/axes;
    curves/circles move by anchor only (approximate).
  - **`Transform` / `ReplacementTransform`** → **`morph(a, b, [spin])`** sets `a`
    up to morph into `b`'s outline (both sampled to the same points);
    `to(a, morph, t)` blends. See `examples/morph.manic`. Caveats: outline-only
    (stroke, not filled area); one target per setup; sampled at build time; naive
    index correspondence (slight rotational offset).
  - **`ClockwiseTransform` / `CounterclockwiseTransform`** → the optional `spin`
    on `morph(a, b, spin)` winds the blend (positive = clockwise, negative = CCW).
  - **`TransformFromCopy`** → **`copy(new, src)`** duplicates an entity (standalone,
    no group tags); `copy(c, a)` then morph/move `c` while `a` stays put.
  - **`Swap`** → **`swap(a, b, [dur], [ease])`** exchanges two entities' positions;
    the array form `swap(arr, i, j)` slides slot values and chains across a sort.
- **Partial (expressible, no dedicated builtin):**
  - `CyclicReplace` → a `for` loop of `move`s.
  - `FadeTransform` / `FadeTransformPieces` → crossfade `par { fade(a); show(b); }`
    — not point-matched.
  - `Restore` → the revert machinery exists internally (`pulse`/`flash`
    auto-restore) but there is no user-facing `save`/`restore` verb.
  - `ApplyPointwiseFunction[ToCenter]`, `ApplyComplexFunction` → expressible over
    a **set of dots** via the loop+expression layer (compute `f(z)` per point and
    `to` it); `transform` covers only the *linear* (2×2) case, not a general
    per-point formula.
- **N/A by design:**
  - `TransformAnimations` — Manim interpolates between two *animation objects*.
    manic's timeline is stateless property tracks with no first-class animation
    object to blend, so the literal form doesn't fit. The practical use —
    smoothly hand off / cross-blend two animations — is covered by `par`/`seq`
    composition plus `morph` / crossfade (`par { fade(a); show(b); }`).
- **Known `morph` limits:** naive index correspondence (mismatched topologies /
  holes can twist), and it can't morph *filled* regions or text glyphs.

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
  - **`TypeWithCursor` / `UntypeWithCursor`** → the **`cursor(id)`** modifier adds
    a `_` typewriter cursor that rides the revealed text (terminal-prompt look).
  - **`AddTextWordByWord`** → **`caption(id, "words", (x,y))`** lays out the
    words, then **`wordpop(id)`** pops them in one at a time (TikTok style) or
    **`karaoke(id, [delay], [color])`** highlights them in sequence (lyrics
    style). See `examples/captions.manic`.
  - `ShowIncreasingSubsets` → `stagger { for i in 0..n { show(x{i}); } }` over a
    tagged group (cumulative reveal).
  - `ShowSubmobjectsOneByOne` → a `seq` of show/hide (flipbook, one at a time).
- **Partial / not one call:**
  - `DrawBorderThenFill` → `draw` traces the border and fades the fill *together*;
    sequencing border-fully-then-fill is scriptable (`seq`) but not one builtin
    (fill opacity isn't a track separate from `trace`).
- **Blocked / needs other machinery:**
  - `Write` / `Unwrite` → we do path-trace + typewriter, **not** calligraphic
    stroke-by-stroke handwriting of glyph outlines — needs glyph-outline stroking
    (tied to the font/LaTeX work).
  - `SpiralIn` → a path-based entrance. Needs **path-motion** (a `Pos` track that
    follows a curve) + the entrance/initial-state machinery (the Growing
    `growin`/`popin` cheap win). Fakeable today by loop-placing offsets + `move`.

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

### Deeper math — how it can elevate the engine (mostly future)
The current evaluator is enough to calculate values and sample plots. Real math
elevates manic when it makes a diagram *depend on a mathematical truth*:
an intersection remains correct as inputs move, a tangent comes from the plotted
function, an eigenvector is computed rather than authored, or an optimisation
visibly converges. The goal is a small, dependable mathematical core, not a
general-purpose CAS embedded in the DSL.

**First rung shipped — a curve-analysis family.** `plot` now *remembers* its
function + screen mapping on the entity (`Entity::graph`), and a shared
`Entity::graph_view` (enum `GraphView`) drives four constructions that all
*query the curve the author already drew* and animate one moving parameter `x`
(`to(id, x, target, dur)` → `Prop::PlotX`):
- **`tangent(id, curve, x, [len])`** — tangent line + contact dot; slope from the
  function (numerical central difference), correct as it slides, honest at
  corners/asymptotes (dot only, no fake line).
- **`normal(id, curve, x, [len])`** — the perpendicular line + dot.
- **`slope(id, curve, x, [(dx,dy)])`** — a live slope *number* riding the point.
- **`area(id, curve, a, b, [n])`** — the filled region under the curve,
  sweepable open via `to(id, x, b, dur)`.
- **`integral(id, curve, a, b, [(px,py)])`** — a live number (composite Simpson)
  that climbs to the true integral as it sweeps, in step with `area`.
- **`roots(id, curve, [color])`** — a dot at every zero-crossing (sign-scan +
  bisection).
- **`newton(id, curve, x0, [steps])`** — the Newton's-method zig-zag from a guess,
  drawn on with `draw` to animate the walk to a root.

Beyond the curve-analysis family (these take points/formulas, not a `plot` id):
- **`spline(id, p0, p1, …)`** — a smooth Catmull-Rom curve through given points
  (interpolation), with knot dots.
- **`trajectory(id, "dx/dt", "dy/dt", (x0,y0), (cx,cy), scale, [steps])`** — an
  RK4-integrated ODE path (orbits, spirals, phase portraits).

See `examples/tangent.manic` and `examples/analysis.manic`; unit tests in
`kits::math::graph_tests` check the numbers against calculus (slope, ∫x²=8/3,
∫sin=2, normal ⟂ tangent). This is the pattern the rest should follow: query the
drawn function, return both a value and a drawable. Natural next step: expose the
integral/slope as a bindable value (`let a = area_of(f,0,2)`) once the arg
evaluator can reach the scene.

- **Robust numerical geometry** — tolerance-aware orientation, intersection,
  containment, root-finding, and curve-parameter routines would make dynamic
  constructions stable near parallel lines, tangencies, and degeneracies.
  This improves every geometry kit before adding any new notation.
- **Linear algebra** — ✅ *DONE — 2D Tiers 1–3 complete, plus the core 3D forms.*
  The unifying idea: a matrix *does something to space*, and the computed
  quantities (determinant, eigenvalues, solutions) are exposed visually — the
  2D/3D analog of what `GraphFn`/`SurfaceFn` did for calculus.
  - *Substrate (shipped):* a small **closed-form** numeric core — `det2`/`eig2`/
    `solve2` (2×2), `det3`/`eig3` (3×3, with a real-cubic root solver), `fit_line`
    (least-squares), `rref_steps` (Gauss-Jordan). **No `nalgebra`** — the 2×2/3×3
    cases are handled directly. The `MatrixFn` "matrix-remembers-its-numbers by
    id" idea was **closed as unneeded**: every builtin takes the matrix inline,
    and `let a = …` variables already give the define-once / reference-many
    ergonomic without coupling to the visual `matrix` entity. (A matrix-by-id
    binding could still be added later if a workflow wants it — the `surf`-on-
    entity pattern shows how — but nothing in Tiers 1–3 needed it.)
  - ✅ *Tier 1 — what a matrix IS (flagship trio, shipped):* **`linmap`** (the
    deformed grid + basis î,ĵ landing on the matrix's columns, over a faint
    identity grid); **`determinant`** (the unit square → parallelogram, area =
    det, flips colour when det<0, collapses to a line at det=0); **`eigen`** (the
    real eigenvector directions + eigenvalues; a note for complex/rotation).
    All math y-up via `det2`/`eig2` (closed-form 2×2 — no `nalgebra` yet). See
    `examples/linear-map.manic`.
  - ✅ *Tier 2 — systems, spans, rank (shipped):* **`linsolve`** (`Ax=b` as the
    row picture — the two rows as lines meeting at the solution, a gold dot + its
    coords; parallel rows = "no unique solution"); **`span`** (the line/plane a
    set of vectors reaches — two independent vectors → the whole plane, one or
    two parallel vectors → a line, i.e. the rank/collapse picture that ties to
    `determinant`). 2D via `solve2` (Cramer) + the cross-product test. See
    `examples/linear-system.manic`.
  - *Tier 3 — decompositions & operations:* ✅ **`diagonalise`** (shipped —
    `A = P D P⁻¹` made visual: the eigen-grid + unit eigen-cell and its image, a
    pure stretch by λ along each eigenvector, no shear; `eig2`-based, math y-up,
    complex/repeated → note; alias `diagonalize`; see `examples/diagonalise.manic`).
    ✅ **`rref`** (shipped — animated Gaussian elimination: one matrix per
    elimination state drawn in place, cross-faded `s{k-1}`→`s{k}` with the row-op
    captioned; the last state is the RREF, and for `[A|b]` its final column is the
    solution; `rref_steps` Gauss-Jordan core; see `examples/rref.manic`).
    ✅ **projection & least-squares** (shipped — `project` drops a vector onto a
    subspace line: the shadow `p = (b·a/a·a)a` and the residual `b−p` at a right
    angle; `leastsquares` fits `y = m x + c` to a point cloud with its vertical
    residuals — the same orthogonal-projection principle. See
    `examples/projection.manic`).
    **Tier 3 complete.**
  - *3D forms:* ✅ **`linmap3`** (shipped — a 3×3 matrix deforming the unit cube
    into a parallelepiped: basis arrows i/j/k on the columns, and the enclosed
    **volume = the determinant**, `det3`-based, colour flips on det < 0, collapses
    at det = 0; see `examples/linear-map3.manic`). ✅ **`eigen3`** (shipped — the
    real eigenvector directions of a 3×3 as invariant lines + λ labels; the
    characteristic cubic solved for real roots, eigenvectors via row cross
    products, complex eigenvalues noted; see `examples/eigen3.manic`). Remaining
    3D: **planes intersecting for a 3×3 solve** (the 3D row picture of `Ax = b`).
  - *3D lesson:* `examples/linear-algebra-3d.manic` ties the 3D forms together
    (one matrix, transformation then eigenvectors), the companion to the 2D
    `examples/linear-algebra.manic` five-idea lesson.
  - *Remaining (optional, not blocking "done"):* a 3D **`Ax=b` as three
    intersecting planes** viz would round out the 3D row picture; everything else
    in the rung is shipped.
- **Calculus and numerical analysis** — the numerical *operations* on a curve
  are shipped: differentiation (`tangent`/`slope`/`normal`/`deriv`), definite
  integration (`area`/`integral`/`accum`, composite Simpson), root-finding
  (`roots` bisection + `newton` zig-zag), interpolation (`spline`, Catmull-Rom),
  and ODE stepping (`trajectory`, RK4 — orbits/spirals/phase portraits). But
  calculus as
  a *subject* is only partly covered — the notable gaps:
  - ✅ *Shipped:* the **derivative as its own curve** (`deriv`) and the
    **accumulation function** `∫ₐˣ f` (`accum`) — together they *show the
    Fundamental Theorem* (`deriv(accum(f))` traces back onto `f`; see
    `examples/ftc.manic`). Both are first-class graphs (numerically sampled via
    `GraphSrc::Samples`), so `tangent`/`slope`/`area` work on them too. Also
    **`extrema`** (maxima/minima = roots of `f'`), **`inflections`** (concavity
    flips = roots of `f''`), and **`band`** (the filled region between two
    curves) — see `examples/curve-features.manic`, `examples/band.manic`.
  - ✅ *Shipped:* **limits** (`limit` — finite points show the value approached
    with an open circle + approaching dot, `examples/limit.manic`; and
    `limit(…, inf)` / `-inf` auto-detects and draws the **horizontal asymptote**,
    `examples/limit-infinity.manic` — `inf`/`infinity` is now a numeric constant)
    and **Taylor series** (`taylor` — the degree-n polynomial about `a`, growing
    to hug the curve; `examples/taylor.manic`). Both numerical.
  - ✅ *Multivariable (shipped):* `surface3` now remembers its `z(x,y)`
    (`Entity3D::surf: SurfaceFn`, the 3D analog of `GraphFn`), and on top of it —
    **`gradient3`** (steepest-ascent arrow, ∂f/∂x & ∂f/∂y), **`tangentplane3`**
    (the tangent plane patch), and **`volume3`** (the volume under the surface as
    a 3D Riemann-sum column grid = double integral). See
    `examples/multivariable3.manic`, `examples/volume3.manic`.
  - *Still to do:* sequence/series convergence (partial sums marching to a
    limit), directional derivatives, and vector-field divergence/curl.
  Status: single-variable calculus is complete, and the core of **multivariable**
  (gradient / partials / tangent plane / volume) now ships. Numerical methods
  were the right first step because their intermediate states are already an
  animation storyboard.
- **Statistics and probability** — ✅ *DONE — Tiers 1–5 all shipped (descriptive
  + shape + distributions + CLT/LLN/correlation + inference + confidence intervals
  + random processes); a new 17-builtin `stats` kit with a seeded PRNG.* The widest
  everyday-relevance rung and the biggest non-programmer audience. Unifying idea:
  turn **data** — or a **random process** — into a picture that reveals its
  shape, centre, and spread, plus the truths that only appear *at scale*
  (distributions, convergence, relationships). Animation-first, so each builtin
  shows a *process*, not a static chart: a histogram **builds up** bar by bar,
  sample means **pile into a bell**, a running proportion **settles** onto the
  true probability. Reuses much of what already ships: `plot`/`GraphFn` for
  distribution curves (the `gauss`/`bell` named functions already exist),
  `area`/`integral`/`accum` for probability-as-area and PDF→CDF, `leastsquares`
  for regression (already shipped), and the number-list parsing from
  `leastsquares` for datasets.
  - *Substrate (new):* a small stats core — mean / median / quantiles /
    variance-std, histogram binning, correlation `r` — plus distribution
    formulas (normal PDF/CDF, uniform, exponential, binomial, Poisson) as
    plottable curves. **Critical design constraint:** sampling demos need a
    **seeded, deterministic PRNG** (an LCG seeded from a DSL argument), NOT system
    entropy — a "1000 coin flips" scene must render the same frames every time
    (reproducible renders are core to the engine). Data is a number list
    (`"v1 v2 v3 …"`), reusing the `leastsquares` parser.
  - *Tier 1 — describe a dataset (flagship trio):* ✅ **`histogram`** (shipped —
    bins a number list into bars, the shape of the data, staggered in bar by bar;
    gold mean marker + range labels; bars tagged `{id}.bar{k}`/`{id}.bars`;
    `histogram_bins` core; new `stats` kit; see `examples/histogram.manic`).
    **`summary`** — the **descriptive-statistics** workhorse: the data as dots on
    a number line, with **mean / median / mode** markers and the **spread** (a
    ±σ band), plus live readouts of **range, variance, standard deviation**. One
    builtin covers most of central-tendency + dispersion. **`boxplot`** — the
    five-number summary (min · Q1 · median · Q3 · max) as a box-and-whisker, so
    the box *is* the **interquartile range (IQR)** and whiskers/outliers show
    tails. A tiny **`skew`** label (left / right / zero) can piggyback on
    `histogram` for **shape**. All cheap: reuse bars / number-line / point parsing.
    ✅ *Shipped:* **`summary`** (`describe` → mean/median/mode/range/variance/std)
    and **`boxplot`** (`five_number` → min·Q1·median·Q3·max, IQR box, 1.5·IQR
    outlier detection; see `examples/summary.manic`, `examples/boxplot.manic`)
    and **`skew`** (`skewness` moment coefficient, mean-vs-median tell, labelled
    right/left/symmetric; see `examples/skew.manic`) — **descriptive statistics
    and shape are complete** (central tendency + dispersion + skewness).
  - *Tier 2 — distributions:* ✅ **`bellcurve`** (shipped — the normal/Gaussian
    bell for μ, σ with the 68–95–99.7 rule shaded as nested ±1σ/±2σ/±3σ bands,
    mean line, % labels, value ticks; alias `gaussian`; named `bellcurve` not
    `normal` to avoid the calculus perpendicular-line builtin; see
    `examples/bellcurve.manic`); the other named
    distributions (uniform / exponential / binomial bars / Poisson);
    **probability = area** under the curve between `a` and `b` (reuses `area`);
    and **PDF → CDF** as the running integral of the density (reuses `accum`).
  - *Tier 3 — truths at scale:* ✅ **`clt`** (shipped — the Central Limit Theorem:
    histograms the averages of `samplesize` dice over `trials` runs → they pile
    into a bell that hugs the theoretical normal; **seeded LCG** (`lcg_next`,
    `clt_means`) so the render is reproducible — this is the promised seeded PRNG
    substrate; see `examples/clt.manic`). Remaining: the **Law of Large Numbers** (a
    running proportion/mean converging to the truth) — ✅ **`lln`** (shipped:
    `lln_proportions`, coin-flip proportion settling onto 0.5, seeded; see
    `examples/lln.manic`); ✅ **`correlation`** (shipped —
    scatter + best-fit line + the Pearson **r** with a strength/direction reading;
    `regression` helper returns `(m, k, r)`; see `examples/correlation.manic`); and
    ✅ **confidence intervals / error bars** (shipped as `confidence`, Tier 4).
  - *Tier 4 — random processes:* ✅ **shipped.** **`montecarlo`** (π by darts,
    seeded), **`randomwalk`** (2-D wandering path, seeded); plus **`distribution`**
    (uniform / exponential / binomial / poisson) and **`confidence`** (a CI ± z·sd/√n)
    round out the distributions/inference. See `examples/probability.manic` (a
    4-idea playground).
  - *Tier 5 — inference:* ✅ **shipped.** **`hypothesis`** (two-tailed z-test —
    p-value as shaded normal tails vs alpha; `normal_tail` numeric core),
    **`covariance`** (signed-area rectangles about the mean cross;
    `covariance_of`), and **`bayes`** (Beta-Bernoulli prior → likelihood →
    posterior for a coin's bias). See `examples/hypothesis.manic`,
    `examples/covariance.manic`, `examples/bayes.manic`.
  - *Recommended first slice:* the **Tier 1 trio** (`histogram`/`summary`/
    `boxplot`) — the "describe data" core, all cheap reuse — then **`normal`**
    (Tier 2), which reuses `plot` + `area` and unlocks the 68–95–99.7 rule. The
    **CLT** (Tier 3) is the flagship *payoff* once the PRNG + `histogram` exist,
    and the natural capstone lesson (`examples/statistics.manic`), mirroring the
    LA five-idea lessons.
  - *3D:* largely N/A / low priority (a bivariate-normal surface via `surface3`,
    or a 3D scatter — nice-to-have, not core to the rung).
- **Constraints and optimisation** — a small solver for distances, angles,
  incidence, and bounds would let authors state a construction's invariant
  instead of manually updating its points. It unlocks movable geometry,
  constrained mechanisms, fitting, gradient descent, and visual proofs by
  deformation. This needs explicit failure/degeneracy behavior, so it should
  follow robust predicates rather than precede them.
- **Symbolic algebra (CAS)** — 🅿️ *parked / design-only.* simplification,
  factoring, equation solving, and automatic differentiation would support
  step-by-step algebra and formula-led constructions. It is valuable when the
  explanation is about *manipulating an expression*, not merely plotting one.
  This is intentionally later: a CAS has a much larger correctness and
  product-scope cost than numeric math.
  - *Architecture (decided):* a **separate, pure, macroquad-free crate**
    `crates/manic-cas` — expression tree, simplify, differentiate, expand/factor,
    solve, and an ordered **step-list** — living at the language layer beside
    `manic-lang`, **not** in the engine. It returns plain **data** (a normalized
    result + the intermediate steps); a thin new engine **kit** (`kits/algebra.rs`)
    is the adapter that turns each step into a tagged `text` entity the author
    animates with existing verbs (`draw`/`stagger`/`morph`). Same "domain-agnostic
    core + pluggable kit" shape as `stats`. The engine depends on `manic-cas` and
    runs it at build/lowering time (like `plot`'s formula string); `manic-lang`
    needs only catalog specs for the new builtins in v1, and can add a dependency
    later for live browser-side symbolic preview (bigger WASM).
  - *End-to-end (author's view):* write an expression/equation string → the CAS
    derives the work → each step renders as an addressable entity → reveal them
    line-by-line like a teacher at the board. Uses: step-by-step **solve**
    (`2x+4=10 → 2x=6 → x=3`), a **symbolic derivative** that is both a formula
    label *and* a plottable curve (reuses `plot`/`GraphFn`), **expand/factor** as
    a `morph` between forms, **substitution** with highlighted replacement, and
    **equation-driven geometry** (exact solved intersections). Results are
    bindable (`let`) and flow into `counter`/downstream builtins like the numeric
    layer.
  - *Hard dependency:* the payoff lands only if the math **renders as math**
    (`x² + 2x + 1`, stacked fractions). ASCII (`x^2 + 2*x + 1`) undercuts the
    teaching benefit for the non-programmer audience.

**LaTeX / math typesetting — Phase 1 SHIPPED ✅ (2026-07), on [RaTeX](https://github.com/erweixin/RaTeX), a CORE capability for ALL kits.**
`equation(id,(x,y),`latex`,[size])` typesets KaTeX-grade LaTeX (fractions, roots,
exponents, Greek, big operators) as a white-on-transparent PNG (RaTeX `embed-fonts`
→ self-contained binary, no font install), drawn via `Shape::Image { tint: true }`
so it takes the template colour and `color`/`recolor` work. LaTeX goes in **backtick
raw strings** (new lexer literal `` `...` ``) so `\frac`/`\theta`/`\neq` survive.
Verified by render (`examples/equation.manic`); 177 tests. Remaining: Phase 2
(DisplayList → native manic glyph/rule entities for draw-on animation + vector
scaling), then migrate kit ASCII labels and drop the old "No LaTeX" gotcha (done in
SYSTEM_PROMPT). Original decision + survey below.

**Decision detail — adopt RaTeX, a CORE capability for ALL kits (not just creator):**
Every kit currently emits ASCII math (`x^2`, `pi*r^2*h`, `3600/47`, geo labels) — it
reads messy across the whole system, so this is engine-wide, not a creator add-on.
Chosen after surveying the field: browser-only MathML crates (katex-rs/pulldown-latex/
latex2mathml) can't render in native mp4; ReX is "not production"; embedding all of
Typst is overkill. **RaTeX** is pure-Rust, MIT, KaTeX-grade (>99.5% coverage), and
decomposed into `ratex-parser → ratex-layout → DisplayList → ratex-render`.
**Spike-validated** (2026-07, in-repo throwaway): the pipeline fetches, builds, and
renders textbook-quality output here (quadratic formula, Σ with limits, √ vinculum,
π/∠/°). Fonts = 20 KaTeX TTFs, 540 KB, MIT — bundle via `include_bytes!`. Plan:
- **Phase 1 (fast):** `ratex-render` PNG → an `equation(id,(x,y),"latex",[size])`
  builtin using manic's existing `Shape::Image`. Full coverage immediately; bitmaps
  (fade/scale/move). Includes (both REQUIRED for Phase 1 to render at all):
  - **Bundle the fonts INTO the binary** — `include_bytes!` the 20 KaTeX TTFs
    (540 KB, MIT/OFL, ship their licence), like manic already embeds IBM Plex. NO
    system install, NO shipped font dir. `render_to_png` only accepts a `font_dir`,
    so extract the embedded bytes to an OS cache/temp dir once at startup and point
    `font_dir` there (the loader's global cache keys on the dir → one-time cost).
    Self-contained across EC2 headless, both Linux cross-builds, and WASM.
  - Render transparent-bg + template-fg colour (recolour DisplayList items; default
    is black-on-white).
- **Phase 2 (native, the manic way):** consume the `DisplayList` → emit manic glyph +
  rule entities (bundled KaTeX fonts) → equations become first-class, theme-coloured,
  **drawn-on stroke by stroke**. Same layout, native rendering.
- **Bonus:** `ratex-wasm` gives the SAME engine in the playground editor → preview
  matches the render exactly.
Once shipped, retire the "No LaTeX" gotcha and migrate kit equation labels off ASCII.
- **Probability and statistics** — ✅ *shipped* — deterministic (seeded) sampling,
  distributions, regression, histograms, and confidence intervals broadened the
  engine into data and algorithm explainers while retaining reproducible recordings.

Recommended order: **robust predicates/root finding → linear algebra →
calculus/numerical methods → constraints/optimisation → symbolic algebra**.
*Status (2026-07):* root-finding, **calculus/numerical methods**, **linear algebra**
(2D Tiers 1–3 + the core 3D forms), and **statistics & probability** (Tiers 1–5)
are all **shipped** — we took calculus ahead of linear algebra, then added the
stats rung. **The active next rung is constraints/optimisation** (then symbolic
algebra); the robust-predicates/numerical-geometry work remains valuable
groundwork underneath both. Each layer should expose computed values to the
existing timeline, counters, plots, geometry, and 3D scene rather than becoming
a separate math subsystem.
Typography is complementary but separate: LaTeX makes mathematics readable;
the capabilities above make it behave correctly.

### Physics — a new domain (in progress) 🚧

Physics is the natural **next domain** (alongside `algo` and the math family — see
manic's "domain-agnostic core + pluggable kits" thesis), and it is exceptionally
well-timed: physics *is* applied calculus, and the calculus/ODE substrate already
ships. **Unifying idea:** a system **evolves under forces/rules**; show the motion
*and* the invisible quantities (velocity, force, energy, momentum) that govern it —
animation-first, so each builtin shows a *process*, not a static diagram.

**Seeded by a goldmine.** `crypto-tool` already holds **38 RK4 sims**
(`crypto-tool/src/main/webapp/physics/labs/js/sims/*.js`) plus a shared core
(`../core/solver.js` = generic n-dim RK4/Euler/midpoint, `state.js`,
`rigid-body.js`, `collision.js`) and reusable views (`energy-bar.js`,
`time-graph.js`, `potential-well.js`, `direction-field.js`). Each sim is a **uniform
declarative spec** that already splits *physics-as-data* from *rendering* — exactly
manic's kit shape. Per-sim fields: `vars` (state vector w/ index + symbol),
`params` (value/min/max/step/unit), `init(p)`, **`evaluate(vars,change,params)`**
(the ODE right-hand side — the physics), `energy()` (KE/PE/total),
`potentialEnergy`+`peWellConfig`, `theoreticalPeriod`, `trailPoint()` (body world
position), **`vectors()`** (velocity + acceleration at the body), `worldRect`
(world→screen map), `presets`, `views` (sim/phase/time/energy/well). The physics
(derivatives + energy formulas + world layout) is **language-agnostic** and
transcribes directly into manic sim definitions.

**The one real substrate change — generalize the integrator.** manic's `trajectory`
is a 2-var RK4 (`dx/dt`, `dy/dt`); these sims are **n-dimensional** state vectors
with an `evaluate` that fills `change[]`. So the single engine addition is a
**general n-dim RK4** (the JS `solver.js` is already generic on `vars.length` — a
direct reference). Everything else is reuse. **Determinism is preserved** by the
`trajectory` precedent: **pre-simulate the whole run at build time** into sampled
tracks, then the stateless timeline just replays them — so scrubbing and
frame-identical recordings still hold (a rare, valuable property: reproducible
physics videos).

**Reuse map (mostly existing machinery):**

| crypto-tool sim field | manic mechanism | New? |
|---|---|---|
| `evaluate` RHS | RK4 integrator (**generalize `trajectory` → n-dim**) | the one substrate change |
| `trailPoint()` / positions | drawn body (dot/rod) + traced path | reuse |
| `vectors()` (v, a) | `arrow`/`vector` glued to the body via updaters (`follow`/`derive`) | reuse |
| `energy()` | `counter` + energy bars | reuse |
| phase / time views | `plot` (x(t), v(t), E(t), phase portrait) | reuse |
| `worldRect` | plot-style screen mapping (pixels-per-metre) | reuse |

**Open decisions:**
- **World units** — physics has real units (m, s, kg); needs a world→screen scale
  like `plot`'s mapping. `worldRect` in every sim already supplies it. Small.
- **One kit or several** — start with a single `physics` kit (like `stats`), split
  by category later (mechanics / E&M / waves) if it grows.
- **Sim spec in-engine vs authored** — likely a declarative sim registry in the kit
  (mirroring the JS specs), with the author choosing which to stage + how to pace it.

**SHIPPED so far (35 sims):**
- **Pendulum family ✅ COMPLETE:** pendulum, double-pendulum⭐, spring-pendulum,
  kapitza, cart-pendulum, compare-pendulum.
- **Spring family ✅ COMPLETE:** spring, vertical-spring, spring-incline, bungee,
  resonance, double-spring, series-parallel-springs, car-suspension, **spring-chain**
  (3-mass/2-spring coupled oscillators on an incline).
- **Pulley family ✅ COMPLETE:** pulley/Atwood, pulley-scale, block-tackle
  (N-strand block & tackle), compound-pulley (fixed + movable, A/B/C),
  incline-pulley (the incline-Atwood).
- **Inclines ✅:** ramp (+ `forces(id)` free-body diagram view), incline-bumper
  (slide into a spring), double-incline (two-slope wedge + apex pulley),
  **loop-track** (ramp → vertical loop-the-loop — the curved-track solver).
- **Other mechanics:** piston, molecule, robot-arm, drop-mass, raft-cm,
  brachistochrone.
- **Collisions ✅ (started):** a shared 1-D impulse resolver `collide_1d` (elastic/inelastic, restitution e), event-driven; **newtons-cradle**, **collide-blocks** (elastic/inelastic + walls), **bullet-block** (embed) all ship on it. Remaining: billiards (needs a 2-D impulse extension).
- **Waves ✅:** string-wave (the discretised wave equation — N masses on springs,
  fixed ends; a plucked pulse travels and reflects).
All on the one `Sim` trait + n-dim RK4 + the four generic views (plus the
build-time energy/kinematic solvers for the event/curved-track cases). Textbook
rendering (`template("paper")` + `support`/`sticky`) composes over any of them.

**⬜ TODO — physics sims not yet built (deferred, pick up later):**
- ⬜ **cart-pole** — needs a balancing controller (LQR/PD gains to tune).
- ⬜ **quadrotor** — 13-var control system.
- ⬜ **billiards** — 2-D collision; needs a 2-D impulse extension of `collide_1d`.
- ⬜ **E&M family** — generator, oscillating-charge, current-coil-magnetic-field,
  generator-3d (a new electromagnetism domain).
- ⬜ **Stretch / separate domains** — pile (rigid body), states-of-matter
  (thermodynamics), navier-stokes (fluids), circuit MNA, pycharge relativistic-EM.

**Tiered inventory (✅ = shipped; ⭐ = flagship):**
- **T1 · trivial (≈3-var):** spring ✅, vertical-spring ✅, spring-incline ✅,
  pendulum ✅, drop-mass ✅, pulley-scale ✅, bungee ✅.
- **T2 · oscillation / chaos:** ⭐**double-pendulum** ✅, double-spring ✅,
  spring-pendulum ✅, compare-pendulum ✅, kapitza-pendulum ✅, resonance ✅,
  series-parallel-springs ✅, spring-chain ✅.
- **T3 · coupled / control (bigger state):** cart-pendulum ✅, cart-pole (open),
  robot-arm ✅, quadrotor (open), brachistochrone ✅, piston ✅, car-suspension ✅,
  molecule ✅.
- **Pulleys / inclines ✅:** pulley/Atwood, block-tackle, compound-pulley,
  incline-pulley, ramp (+forces), incline-bumper, double-incline, loop-track.
- **T4 · collisions:** newtons-cradle ✅ (event-driven, via `collide_1d`); collide-blocks ✅, bullet-block ✅; billiards (open — 2-D impulse).
- **T5 · electromagnetism:** generator, oscillating-charge,
  current-coil-magnetic-field, generator-3d.
- **Stretch / separate domains:** string-wave (waves, 203 vars), pile (rigid body,
  100), states-of-matter (thermodynamics), navier-stokes (fluids), and the
  `circuit/` MNA simulator + `pycharge/` relativistic-EM subsystems — their own
  future domains, **not** RK4 point-mechanics.

**Why now:** it is mostly *reuse* (integrator generalization + drawables that exist)
sitting on a *ready, tuned* physics corpus; the double pendulum alone is a
standout demo; and it visibly *depends on* the shipped calculus/ODE core — the same
"the diagram is true, not drawn" thesis, applied to motion.

#### Design — "adapt, simulate, connect"

**Unifying model:** *a simulation = named state + their time-derivatives + a map
from state → drawables — all expressed with the formula strings manic already
evaluates.* That single model gives two layers of ease and three integration seams.

**Layer 1 — named sims (adapt-by-tweaking), for everyone.** The ~20 goldmine RK4
sims ship as named builtins; a non-programmer picks one and changes numbers/presets
— zero physics knowledge:
```
pendulum(p, center, length: 2, gravity: 9.81, angle0: 60);
draw(p);   // an ordinary entity → animate on the timeline
```

**Layer 2 — the system builder (author-your-own), for the creative user.** The same
pendulum from its equations — you write the *math*, not the plumbing:
```
system(s, center, scale: 120);
state(s, "theta", 60);  state(s, "omega", 0);
flow(s, "theta", "omega");                 // dθ/dt = ω
flow(s, "omega", "-(g/L)*sin(theta)");     // dω/dt = −(g/L)·sinθ
body(s, bob, "L*sin(theta)", "-L*cos(theta)");
rod(s, arm, origin, bob);
simulate(s, 12);                           // pre-integrate 12 s (RK4, deterministic)
```

**Three seams:**
1. **Simulate** — pre-integrate at build time into sampled state tracks (the
   `trajectory` precedent); the stateless timeline *replays* via a `time` track
   (`to(s, time, 12, dur)` → scrub / slow-mo / pause / replay for free).
2. **Animation engine** — every `body`/`rod`/`vector`/energy-bar is a tagged
   entity id → `draw`/`show`/`pulse`/`follow`/`section`/presets/branding all apply.
3. **Math engine** — a **shared world→screen mapping** (physics `scale`/world-rect
   *is* `plot`'s `GraphView`) + **bindable state** let physics and math combine on
   one stage: spring → `plot` x(t) → `tangent` = velocity; pendulum → (θ,ω) phase
   portrait via the `trajectory` plotter; damped spring → `leastsquares` the decay
   envelope; orbit → swept `area` = Kepler's 2nd law. One file, two kits, no glue.

**Adaptation ladder (why it's "easy"):** 1. tweak a preset/param → 2. restyle / add
a trail or force arrows → 3. override one equation of a built-in → 4. author a new
system from `state`+`flow`. Rung 1 needs no physics.

**Scope decisions (agreed):** ship **named sims + a minimal builder** in v1; builder
surface = `state`/`flow`/`body`/`rod`/`vector`/`energy`/`simulate` (collisions /
constraints later); **one `physics` kit** (split by category only if it grows);
**pendulum is the flagship** (cleanest teaching arc), double-pendulum the wow demo.

#### What needs to be done (ordered)

1. ✅ **Generalize RK4 → n-dim** — `src/ode.rs`: generic `rk4_step`/`euler_step`,
   `integrate` (every step) and `integrate_sampled` (**substep control** — sim at a
   fine `dt`, emit at frame resolution) over an n-vector state with an
   `eval(state, deriv)` closure (modeled on the well-tested `core/solver.js`).
   Reused scratch (no per-step alloc); stops on non-finite. **Time-dependent
   forcing** supported via the clock-variable trick (`d[TIME]=1`) — driven/damped
   systems just carry time in the state. Unit-tested against analytic truth (exp
   decay = e⁻ᵗ; SHO tracks cos/−sin + conserves energy; driven `y'=cos(t)`⇒sin;
   convergence order as `dt` halves; **determinism**; sampled/substep consistency;
   blow-up stops early). The 2-var `rk4_path` (trajectory) now delegates to it — the
   n = 2 special case (dedup + validation). 8 tests; 116 engine tests green, no
   regression.
2. **Extend the formula evaluator to named variables** — today `expr::Node::eval`
   is 2-var (`x`,`y`); physics needs evaluation over an arbitrary named-state env
   (`theta`,`omega`,…, plus `let` params). This is what lets `flow`/`body` take
   formula strings.
3. **Sim-container entity + `time` playback param** — holds the sampled tracks and
   a playback cursor (mirror `plot`'s `PlotX` / `trajectory`'s pre-integration), so
   `to(s, time, …)` drives every bound body/vector/plot.
4. **Shared world-units mapping with `plot`** — the math↔physics seam (pixels-per-
   metre == `GraphView`).
5. 🚧 **Named-sim registry** — transcribe the goldmine specs
   (`evaluate`/`energy`/`trailPoint`/`vectors`/`worldRect`/`presets`) into the kit.
   *Started:* `src/kits/physics.rs` has the declarative **`Sim` trait**
   (`state0`/`deriv`/`energy`/`body`) + `simulate()` (bridges to `crate::ode`) +
   the first sim **`Pendulum`** (simple/damped/driven, clock-variable time),
   physics-checked (reproduces 2π√(L/g); conserves energy undamped; damping bleeds
   it) — 3 tests. No builtin registered yet. NB: Layer-1 named sims are Rust
   closures, so task ② (formula evaluator) is **not** needed for them — only for
   the Layer-2 builder.
6. **Builder builtins** — `system`/`state`/`flow`/`body`/`rod`/`vector`/`energy`/
   `simulate` in a new `src/kits/physics.rs`.
7. **Math-seam demos + examples + a lesson** (pendulum flagship; the combo demos).
8. **Checklist wiring per builtin** — catalog + LANGUAGE + SYSTEM_PROMPT +
   CAPABILITIES + tests + example + WASM rebuild + UI snapshot (the builtin
   checklist).

**Layer-1 status — the pendulum swings end-to-end.** ✅ The first named sim is
*shipped* as two builtins: **`pendulum(id,(cx,cy),[length],[angle0],[unit],
[damping])`** (ctor — builds the `Pendulum`, pre-simulates 240 RK4 frames at build
time, lays out `{id}.pivot`/`{id}.rod`/`{id}.bob`/`{id}.path` tagged bare `{id}`+
`{id}.parts`, stores the screen-space body path in a new `Scene.sims` side-table)
and **`swing(id,[dur])`** (verb — replays that path as a keyframed `Pos`/`To`
track chain; `swing` is in `verb_consumes_structure_id` so it doesn't broadcast
over the bare-id tag). This covers a pragmatic ③ (playback via the `Scene.sims`
side-table of typed `PlaybackTrack`s + `resolve`'s keyframe chaining — no new
`Prop` needed) and a minimal ④ (per-pendulum `unit` px/m). **Overlays shipped:**
the velocity arrow `{id}.vel` (gold, tangent, length ∝ speed) and the KE/PE energy
bars `{id}.ke`/`{id}.pe` (cyan/magenta, normalised to initial total energy so a
damped swing visibly bleeds energy) with labels, tagged `{id}.overlays`. **Args
are minimal-required:** only `id` is mandatory — `center` (default `(640,200)`),
`length`, `angle0`, `unit`, `damping` all default, so `pendulum(p); swing(p)`
works. `examples/pendulum.manic` renders deterministically. Registered,
catalog-matched, arity-audited, editor-checked; 124 engine + 36 manic-lang tests
green; WASM rebuilt+copied; docs synced.
**Generic view layer shipped.** A sim's ctor now stores a reusable `SimData`
(raw state trajectory + `(KE,PE)` per frame + `dt` + var labels + phase/pos-var
metadata + a sampled well curve; in `scene.rs`), and **opt-in view builtins read
it generically** — `phase(id,(cx,cy),[size])` (phase portrait: closed loop vs
damped spiral) and `well(id,(cx,cy),[size])` (potential-energy well with the body
as a ball rolling in it). Each lays out its own auto-fit panel + curve + a marker
that it *appends to the sim's `swing` playback*, so all views animate together.
The `Sim` trait carries the view metadata as **defaulted methods**, so a sim
opts into a view just by overriding one (a sim that doesn't stays view-less) —
the "perfect baseline template" for future sims. **All four views ship:** `phase` (portrait), `well` (potential well),
`timegraph` (θ(t)/ω(t) with a sweep line), and `energygraph` (KE/PE/total over
time). The two graph views share an `add_time_view` helper (multi-curve + swept
"now" line). `examples/pendulum.manic` is a **four-view dashboard** (sim + 2×2
panels), renders deterministically.

**Second sim shipped — the baseline generalises.** `spring(id,[center],
[stiffness],[x0],[unit],[damping])` is a mass–spring (SHM) — a different system
(state `[x,v]`, motion along x, a **parabolic** well ½kx² vs the pendulum's
cosine) that inherits all four views *for free* via the same `Sim` trait. The
velocity-arrow + energy-bar overlays were extracted into a shared `add_overlays`
helper both sims call, and the playback verb was generalised to **`run(id,[dur])`**
(with **`swing`** kept as a pendulum-friendly alias — both map to `v_play`).
`examples/spring.manic` is a four-view spring dashboard; renders deterministically.
**Third sim shipped — the double pendulum⭐ (chaos).** `doublependulum(id,
[center],[angle1],[angle2],[unit])` — two arms hinged end-to-end, the coupled EOM
transcribed from the goldmine; deterministic yet sensitive to initial conditions.
Parts `{id}.pivot/.rod1/.bob1/.rod2/.bob2` + the outer bob's chaotic trail
`{id}.path` (trace it with `par { run(dp,d); draw(dp.path,d); }`). It's a 4-D
system, so `phase`(θ₁ vs θ₂)/`timegraph`/`energygraph` apply but **`well` is
refused** with a clear error (the generic view layer degrades gracefully — a sim
opts out of a view just by leaving its metadata empty). `examples/double-pendulum.manic`.
**Full pendulum family shipped** (all on the one `Sim` trait): `pendulum`,
`doublependulum` (chaos), `springpendulum` (elastic — swings + bounces, coil),
`kapitza` (inverted-stable via fast pivot vibration), `cartpendulum` (spring cart
+ pendulum), `comparependulum` (two 0.001-rad-apart pendulums diverging). **Full spring family shipped** too: `spring` (SHM), `verticalspring`,
`springincline`, `bungee` (one-sided cord), `resonance` (driven), `doublespring`
(coupled/beating), `seriesparallel` (series vs parallel), `carsuspension`
(quarter-car on a scrolling road) — springs drawn with the real stretching `Coil`
primitive. **13 sims total**; each is ~a struct + a ctor; all inherit the four
views where they apply (driven/4-D/multi-body sims skip `well`; multi-body sims
skip the single-body overlays and compute their energy series inline). ~17 physics
examples in the gallery. **Remaining Layer-1 polish:** a shared world→screen map
with `plot` (so a sim + a graph share coordinates). A dedicated time-indexed
playback `Prop` stays a future optimization if the per-frame track chain proves
heavy.

### Optics — a new kit (planned) 🚧

**The theme is manic, not lens design.** The goldmine
(`crypto-tool/src/main/webapp/physics/js/optical-designer-{model,trace,render}.js`)
is a *serious* sequential lens-design ray-tracer — Sellmeier dispersion over a real
glass catalog, vector Snell's law with total-internal-reflection, closed-form
ray–conic intersection, ABCD paraxial matrices, spot diagrams and aberration plots.
Per **goldmine-reimagine-not-port**, we keep the **physics faithful** but **throw
away the engineering GUI** (surface tables, RMS sliders, f/# read-outs). What ships
is a handful of **dead-simple builtins a non-programmer can drop into a scene** —
`refract`, `lens`, `prism`, `achromat` — each showing light *doing something*, with
the true `n(λ)` underneath so the color effects are real, not painted.

**Substrate — geometric, not RK4.** Optics has **no time dimension**: it is a static
closed-form ray trace (like the collision sims' build-time trajectories), producing
ray **polylines** + **glass polygons** + a **focal dot** + light entities — all
ordinary manic entities, so tag-broadcast, `cam`/`zoom`, `draw`/`show`, and
`template("paper")` compose for free. **Animation = a parameter sweep** (build
`sweep` from day one): each builtin precomputes frames as one parameter
(**wavelength · incidence angle · lens radius/focal · object distance**) varies,
stored as a playback track and replayed with **`run(id,[dur])`** — the focus slides,
TIR switches on, the rainbow fans out. Same deterministic build-time-precompute
precedent as the physics `run`.

**Modular kit layout (keep files small).** Not one giant `optics.rs`. A module dir:
- `src/kits/optics/mod.rs` — kit registration + shared types (`Ray`, `Surface`, `Medium`).
- `src/kits/optics/dispersion.rs` — the glass catalog + `sellmeier_n(λ)` (faithful port).
- `src/kits/optics/trace.rs` — the physics engine: 2-D vector Snell + TIR, ray–surface
  (spherical/conic) intersection, `trace_sequential`, and the ABCD paraxial helper
  (reuses the linalg 2×2 mental model) for the focal point.
- `src/kits/optics/builtins.rs` — the author-facing ctors (`refract`/`lens`/`prism`/
  `achromat`), each emitting entities + a sweep playback track.

**Builtins — first milestone (through dispersion):**
| builtin | non-programmer's mental model | physics underneath |
|---|---|---|
| `refract(id,[n1],[n2],[angle])` | "a light ray bends crossing into glass/water" | 2-D Snell + TIR cutoff; sweep `angle` → watch TIR switch on |
| `lens(id,[center],[focal],[kind])` | "parallel rays focus to a point" | ray fan → real focal length; sweep `focal`/radius → focus slides |
| `prism(id,[center],[glass])` | "white light splits into a rainbow" | Sellmeier `n(λ)` per color → the spectrum fan (the iconic visual) |
| `achromat(id,…)` | "red and blue focus apart — then a doublet fixes it" | true axial chromatic Δf, then a BK7+SF2 doublet pulls them together (the capstone) |

`prism` is the **optics** builtin; the existing 3-D solid stays `prism3` (no clash).
A small named **glass catalog** (`bk7`, `sf11`, `f2`, `water`, `diamond`, …) selects
Sellmeier coefficients by name, so authors never touch numbers.

**Tiers (build one at a time):**
- **T1 · foundations:** ✅ **`refract`** (Snell + TIR sweep — the modular kit
  `src/kits/optics/{mod,trace,builtins}.rs`; sweeps the incidence angle via a
  `SimData` playback replayed by `run`; `examples/refraction.manic`), ✅ **`lens`**
  (converging lens — a parallel beam focuses to F; sweeps the focal length so the
  focus slides; ideal thin lens; `examples/lens.manic`).
- **T2 · dispersion:** ✅ **`prism`** (Sellmeier rainbow — the new
  `src/kits/optics/dispersion.rs`: 3-term Sellmeier + a named glass catalog
  (`bk7`/`sf11`/`f2`/`diamond`/`water`/`sapphire`/`silica`) + wavelength→RGB; each
  colour traced through both prism faces with `refract_vec`+`ray_segment`; sweeps
  the incidence angle; `examples/prism.manic`), ✅ **`achromat`** (chromatic
  aberration → the doublet fix — real crown dispersion splits the red/blue foci,
  `run` sweeps in the correction and they merge to one sharp point;
  `examples/achromat.manic`). **T2 · dispersion COMPLETE — the through-dispersion
  first milestone is shipped.**
  - **Annotated/elevated examples (hybrid backdrop):** the *geometric* builtins get
    `template("paper")` textbook figures (`refraction-paper`, `lens-paper`); the
    *colour* builtins stay on a dark bench where light glows (`prism-cinematic`,
    `achromat-cinematic`) — a rainbow washes out on cream, so light is a
    dark-background subject. Each varies its elevation lens (camera / typewriter /
    wordpop / brace) per [[demo-elevation-controls]].
- **T3 · systems:** ✅ **`lenssystem`** (a REAL multi-element lens ray-traced
  through its actual spherical surfaces — presets singlet/doublet/triplet; the
  new `trace::trace_spherical` 2-D ray–sphere intersection; rays are drawable
  polylines and `run` sweeps a sensor plane + live spot-size read-out showing
  **spherical aberration**; f-number read-out; `examples/lens-system.manic`).
  "Best of both": faithful physics + manic animation. Now also **NA read-out +
  autofocus** (a magenta best-focus marker at the minimum-spot plane). **Lens
  prescriptions both ways:** pick a real design by NAME (singlet/biconvex,
  plano-convex, meniscus, doublet/achromat, triplet/cooke) OR write a CUSTOM
  prescription string `"radius thickness glass [conic] [aperture] | …"`
  (`resolve_prescription`/`parse_prescription` in `builtins.rs`;
  `examples/lens-prescription.manic`). **Full prescription surface fields shipped:**
  `trace::trace_conic` (2-D ray–conic intersection) gives **aspherics** — the
  `"aspheric"` preset's conic (K≈−0.55, an ellipsoid) nulls spherical aberration
  (RMS 1.5 px → 0.1 px, a real blur→point; `examples/aspheric-lens.manic`) —
  plus **per-surface aperture** (clips rays + sets element height) and an optional
  **finite object distance** (diverging point source; f/#/NA hidden off-axis of
  the collimated case).
- **Off-axis field aberrations ✅:** a **3-D conic tracer** (`trace::trace_conic_3d`
  + `refract_vec3`) powers **`fieldspot`** — a full 2-D pupil traced in 3-D at a
  field angle: symmetric on-axis, a **coma** comet + **astigmatic** stretch
  off-axis (singlet RMS ~7 px vs doublet ~1.4 px at 8°), with an **Airy-disk**
  diffraction-limit overlay that scales with f/# (small = geometry-limited, ~spot
  = diffraction-limited). `examples/off-axis.manic`. **Optics kit T1–T4 + full
  prescription + field aberrations COMPLETE.**
- **T4 · analysis ✅:** **`rayfan`** (the ray-fan aberration plot — the singlet's
  cubic spherical-aberration S-curve, flattened by the doublet; `examples/ray-fan.manic`)
  and **`spotdiagram`** (the spot at best focus — a blur disc for the singlet,
  a point for the doublet, RMS read-out + ideal-point marker; `examples/spot-diagram.manic`).
  Both share `optics::builtins::analyze_preset` (rotationally-symmetric on-axis
  transverse-aberration trace) and scale to the singlet so the correction reads.
  ⬜ off-axis field points (coma/astigmatism) + Airy-disk overlay still open.

**Why it fits:** a beautiful, genuinely-physical domain (the rainbow is *earned* by
`n(λ)`, the focus is *earned* by Snell), tiny author surface, and it reuses every
existing manic primitive — the same "the diagram is true, not drawn" thesis, now for
light. Follows the manic-builtin-checklist for each ctor (catalog + LANGUAGE +
SYSTEM_PROMPT + CAPABILITIES + test + example + WASM/system-prompt snapshots).

### 3D — status (roadmap #1–#6 all shipped)
The foundation and the full 3D roadmap below have shipped. Coverage against the
~96 Asymptote `graph3` / `three` / `solids` / `tube` examples is:
- **Geometry** — parametric **curves** (`curve3`), height-field **surfaces**
  (`surface3`), **general parametric surfaces** (`param3` — `x/y/z(u,v)`, so
  tori/Möbius/parametric spheres/shells), regular-polygon **solids**
  (`prism3`/`pyramid3`), and **solids of revolution** (`revolve3`) ship (surfaces
  and solids render **filled + flat-shaded**, not wireframe), arbitrary 2D
  shapes / boolean regions **extrude** into solids (`extrude3` — this doubles as
  **CSG solids**: extrude a `union`/`difference`/`intersect`/`xor` region), and
  `curve3`/`line3`/`arrow3` can be drawn as shaded **tubes** (`thick`). Still
  pending: imported 3D models, variable-radius tubes (the Asymptote `tube`
  corpus — `thick` is constant-radius), contour/level curves on surfaces.
- **Rendering** — a single baked light + flat per-face shading ship for
  surfaces/meshes/`cube3`/`sphere3`, tube-style thick strokes ship for paths
  (`thick`), and intersecting translucent geometry is depth-sorted (opaque
  first, then translucent back-to-front). Still pending: **adjustable/multiple
  lights** (direction + intensity; today it's one hard-coded light), smooth
  (Gouraud) shading, **visible surface mesh/grid lines** on filled surfaces,
  **depth cueing / fog**, material/texture shaders, and shadows.
- **Labels & graphing** — depth-aware projected labels (`pin3`) and fully
  ticked/labelled, auto-decluttering 3D axes (`axes3`) ship. Still pending:
  **true 3D-embedded text** that lives in the scene and scales with distance
  (today's labels are screen-space, constant size).
- **Dynamic constructions** — `follow3` + `midpoint3` (3D `follow`/`derive`) ship;
  still to come: `link3` (reflowing 3D edges) and richer derived/constrained
  geometry that recomputes as source points move.
- **Animation breadth** — `morph3` blends curves, surfaces, and solids (solids
  reparameterised spherically), and `to` now animates 3D `morph`/`opacity`/
  `scale`/`trace`/`color`; the dedicated verbs (move3/rotate3/grow3/…) cover
  position, rotation, and size.

**3D roadmap (prioritized).** Same principle as the 2D plan — extend a few
existing mechanisms rather than add a builtin per Asymptote class. Two
prerequisites recur and are the real leverage: a **3D→screen projection hook**
(so the existing 2D `text`/`label`/`counter` overlay can pin to a projected 3D
point) and a **`Vec3` `derive`/updater** (mirror the 2D dependent-point path).

| # | Requirement | How to address (extend what) | Effort | Unlocks |
|---|---|---|---|---|
| 1 | **Ticked/labelled 3D axes + projected labels** ✅ **shipped** | `project()` world→screen hook; `pin3` (a 2D label glued to a 3D point/entity, reprojected each frame); `axes3` now emits tick marks + auto-`pin3`ed numbers (optional `step`). | Small | Readable 3D graphs + labelled points/vectors/axes. |
| 2 | **`Vec3` dynamic constructions** ✅ **shipped** | Added a 3D `derive`/`follow` resolve pass; `follow3` (track another entity + offset) and `midpoint3` (derived point) recompute each frame. `link3`/projections extend the same hook. | Medium | Live 3D geometry: dependent points + tracking that recompute as sources move. |
| 3 | **Parametric curve & surface** ✅ **shipped** | `curve3(id,"x(t)","y(t)","z(t)")` → drawn-on `Shape3D::Path`; `surface3(id,"z(x,y)",…)` → filled, flat-shaded `Shape3D::Surface`; `param3(id,"x(u,v)","y(u,v)","z(u,v)",…)` → a **general** parametric surface (tori/Möbius/shells — can wrap/close). The `plot` expr engine was widened to **two variables** (`x`/`y`, `u`/`v`). | Medium | Helices/Lissajous, `z=f(x,y)` surfaces, and closed/parametric surfaces (the full `graph3` corpus). |
| 4 | **Indexed meshes & solids** ✅ **shipped** | `Shape3D::Mesh` (verts + tri `faces` + wireframe fallback) + `prism3`/`pyramid3` (n-gon extrusion/apex) + `revolve3` (solids of revolution) + `extrude3` (extrude any 2D fillable shape or boolean `Region`). `extrude3` reuses `geom.rs` (`entity_to_multipolygon` + `earcutr`), so extruding a `union`/`difference`/`intersect`/`xor` region **is** boolean CSG (plate-with-hole, L-beams, …). | Large | Prisms/pyramids/cylinders/cones, vases/spheres/lathes, arbitrary/concave extrusions, and CSG solids. |
| 5 | **3D rendering upgrades** ✅ **shipped** | Surfaces/meshes/`cube3`/`sphere3` render **filled** with a single baked light and flat per-face lambert shading (`abs(n·l)`, no black back-faces; chunked under the u16 index cap). `curve3`/`line3`/`arrow3` draw as shaded **tubes** via `thick(id,radius)` (rotation-minimising frame; arrows get a solid cone head). Translucent geometry is **depth-sorted** (opaque first, then translucent entities + their triangles back-to-front). **Remaining (moved out of scope):** material/texture shaders and shadows. | Large | Solid-looking 3D, correct translucent overlaps, publication-quality output. |
| 6 | **3D morph / general `to`** ✅ **shipped** | `morph3(a,b,[spin])` samples both shapes to a shared form — curves→polyline, surfaces & solids→a filled/shaded grid (solids reparameterised onto a spherical `(θ,φ)` grid via bbox-centre raycasting, so cube↔sphere works). `to` extended to animate 3D `morph`/`opacity`/`scale`/`trace`/`color`. | Large | 3D `Transform` / `ReplacementTransform`, mesh/path morphing. |

Planned order (agreed): **1 ✅ → 2 ✅ → 3 ✅ → 4 ✅ → 5 ✅ → 6 ✅** — the full
3D roadmap has shipped. #4 shipped `Shape3D::Mesh` + `prism3`/`pyramid3`/`revolve3` + `extrude3`
(arbitrary/concave extrude **and** boolean CSG, both via `geom.rs`); #5 shipped
filled + flat-shaded faces (surfaces/meshes/`cube3`/`sphere3`), tube strokes
(`thick`), and depth-sorted translucency (de-scoped: texture/material shaders +
shadows). #1 and #2 are mostly *reuse* (the projection hook + a `Vec3` updater)
and together make 3D genuinely usable for explainers; #3 brings the `graph3`
corpus within reach off the existing `plot` sampler. #4/#5 are the orthogonal
"real 3D engine" work — big, and only needed once the legible-diagram cases land.

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
under evaluation, see Typography), **selectable fonts**, and the advanced 3D
geometry/rendering work listed above.

### Stateful structures — done (mutating verbs)

The timeline is a pure function of `t`, so an ordinary verb sees only the base
scene: a *chain* of swaps would each read stale positions. This is now solved
with a **mutating-verb** kind — `MutVerbFn = fn(&mut Scene, &Args) -> Clip` — and
a build-time occupancy map `Scene::occ` (structure id → entity per slot). A
mutating verb produces its clip *and* updates `occ`, so the next step sees the
current state. This is the general primitive for stateful data structures
(sorting today; stack push/pop, queue, pointer moves next), and it composes
across the stateless timeline without any render-time state.

- **`swap(arr, i, j)`** (std, mutating) — the values in slots `i`/`j` **slide**
  past each other (one hops over the top) into the swapped slots, and `occ`
  updates so a whole sort chains correctly. `swap(a, b)` (two entity ids) still
  does the plain position swap.
- **`compare(arr, i, j, [color])`** (algo) — flashes the values *currently* in
  those slots (reads live `occ`), the comparison step of a sort.

See `examples/bubble_sort.manic` — real in-place sort, no `say`.

## Presets & branding (output)

**Shipped.** Rendering is driven by named **presets** (`--preset <name>`) — the
baseline for quality, frame rate, container, and branding; any runtime flag
(`--scale`, `--fps`, `--gif`, `--no-brand`, …) overrides the preset's fields
(`src/preset.rs`).
- **`studio`** (default) — branded, `scale 1.5` (→1080p), 60fps, MP4.
- **`test`** — unbranded, `scale 1.0`, 30fps; the fast verify preset.
- **`reel`** — branded, for vertical/social clips (pair with a `canvas("9:16")`).

**Branding** (`src/branding.rs`) is injected by the **engine, never authored in
the DSL**, and applies only to **recorded** output under a branded preset (so the
live preview + stills stay clean and fast):
- a **pre-roll intro** — the hue-graded fractal tree grows (yellow trunk →
  magenta/blue tips) while the `Manic` wordmark typewrites in beside it over the
  link `https://8gwifi.org/manic`; authored internally in manic (a recursive
  `def`) and composed ahead of the user's timeline;
- a pinned **"Made With Manic"** watermark for the whole DSL portion.

Disable with `--no-brand`. (Also fixed: the `--png`/`--alpha` sequence now writes
frames upright — `export_png`'s internal flip is cancelled in `record.rs`.)

## Creator Kit v2 core — shipped ✅

The first Creator Kit shipped the complete quiz-Short loop (`creator`/`socials`,
`quiz`/`option`/`run`, countdown, safe-zone guide, figure auto-fit, four skins and
five question reveals). V2 is an intentional production redesign, not a second
pile of skins. Its shipped core contains three ordered slices:

### V2.1 — responsive layout and design foundations

- **Viewport-aware kit layout.** Creator constructors must read the actual canvas
  dimensions instead of baking `540`/`1920` coordinates. One format must adapt to
  portrait `9:16`, feed `4:5`, square `1:1`, and landscape `16:9` canvases.
- **Platform safe areas.** Named `shorts`, `reels`, `tiktok`, and `clean` guides
  provide top/bottom/side insets; all automatic format regions stay inside them.
- **Shared regions.** Header, media, choices, timer, caption and footer are derived
  from the safe content rectangle and density, rather than positioned separately.
- **Creator design tokens.** A small internal style model owns typography roles,
  spacing, card fill/edge, accent use, glow, option density, timer treatment, and
  motion recipe. The default is a restrained **studio/editorial** look: strong
  hierarchy, one accent, crisp panels and purposeful motion. `badge`, `minimal`,
  `glass`, and `plain` remain available and backwards compatible.
- **Reliable fitting.** `figure()` uses shared entity bounds and includes text,
  images/equations, curves, stroke and scale. It must fail clearly on an empty
  target and avoid silently producing a broken live construction.

### V2.2 — Quiz v2

- Preserve every v1 file unchanged: `quiz(q,"?")`, the old skin/reveal words,
  `option(...[,correct])`, and `run(q,dur)` remain valid.
- Extend the order-free quiz spec with explicit `key=value` options for
  `layout`, `density`, `timer`, `motion`, and `reveal`. Defaults stay concise.
- Responsive answer layouts cover 1–6 options (stack up to four; auto/grid up to six),
  long-answer wrapping, phone-readable minimum type, and deterministic overflow
  diagnostics instead of overlaps.
- Timer treatments: `ring`, `bar`, `number`, and `none`. Reveal treatments keep
  the correct answer legible, deliberately de-emphasise distractors, and allow an
  optional author-supplied explanation/source without inventing a solution act.
- Motion recipes: `calm`, `studio`, `punch`, and `cut`, with timing derived from
  the requested `run` duration rather than hard-coded absolute beats.

Proposed v2 authoring surface (the exact accepted keys are documented by parser
errors and tests):

```manic
canvas("9:16"); template("mono");
creator(me, "@anish2good name=Optics_Lab yt=zarigatongy x=@anish2good web=8gwifi.org/manic accent=cyan footer=compact");
quiz(q, "Which glass bends blue light more?",
     "studio layout=media-first reveal=rise timer=bar density=comfortable");
option(q, "Crown glass");
option(q, "Flint glass", correct);
option(q, "Both equally");
option(q, "Neither");
prism(p, (540, 650), "sf11");
figure(p);
explain(q, "Flint glass has stronger dispersion.");
run(q, 12);
socials(me);
```

### V2.3 — creator brand system

- Extend `creator(id,"spec")` without breaking existing specs: display name,
  handle, logo/avatar image, accent/secondary colours, tagline, website, footer
  style and default CTA live in one reusable profile.
- Footer variants: `compact`, `signature`, `social`, and `none`; automatic layout
  uses configured identity content and stays inside the active safe area.
- A reusable `endcard(profile, [spec])` produces a professional final creator
  lockup with optional CTA. Custom avatar/channel art remains optional through
  `logo=`; the social footer itself uses native vector marks.
- Brand choices are creator content, separate from manic's engine-level recorded
  watermark/pre-roll and from the global canvas `template()` palette.

### V2 core acceptance criteria

1. Old Creator Kit examples parse, validate, and retain their existing entity ids.
2. The same v2 quiz source lays out without overlap on 9:16, 4:5, 1:1, and 16:9.
3. Stress cases cover 2–6 choices, long text, inline math, light/dark templates,
   logo/no-logo profiles, and representative geo/physics/optics figures.
4. Unit tests cover spec parsing, layout regions, safe-area selection, backwards
   compatibility, profiles, footer variants, and end cards.
5. Representative frames are rendered and visually inspected at question,
   countdown, answer-reveal, and end-card moments before v2 is called complete.
6. `SYSTEM_PROMPT.md`, the creator book chapter, examples, and this capability
   ledger are updated together with the implementation.

**Deferred until after the v2 core:** fact-card, listicle, this-or-that and other
format families will reuse these foundations, but are not allowed to delay the
responsive quiz + brand-system release.

**Implementation result (2026-07-18):** ✅ logical canvas size now reaches every
kit through `Scene`; ✅ responsive header/media/choices/timer/footer regions adapt
across 9:16, 4:5, 1:1 and 16:9; ✅ named Shorts/Reels/TikTok/clean safe areas;
✅ rounded translucent-safe UI panels; ✅ a restrained studio palette under
`template("shorts")`; ✅ Studio is the new quiz default while all v1 skin/reveal
words and entity ids remain; ✅ v2 `layout`/`density`/`timer`/`motion`/`safe`/
`accent` parsing; ✅ width-aware answer type and 1–6 auto/grid layout (stack is
guarded at four); ✅ optional `explain`; ✅ expanded creator profile, four footer
styles and hidden `endcard`; ✅ improved `figure` bounds for paths/text/images/
equations plus live-dependency diagnostics; ✅ catalog, prompt, book, gallery and
`examples/creator-v2.manic` updated. Sixteen Creator/Timing tests cover the v2
surface, including all four aspect ratios and generic named phases; the complete
193-test library suite passes.
Question, choices/countdown, reveal, end-card, square and landscape frames were
rendered and visually inspected. That visual pass caught and fixed translucent
corner overdraw, timer/explanation collision, and narrow-card text overflow.

**Gold-path Reel documentation — shipped ✅ (2026-07):** mdBook now promotes a
first-class `Create a polished Reel` workflow directly after Getting Started.
It covers platform-safe composition, phone-first content hierarchy, layout and
motion choices, exact pacing, native timer selection, reusable branding,
end-card design, still-frame review, and Reel export. The copyable
`examples/perfect-reel.manic` starter is editor-checked and visually reviewed at
its hook, countdown, reveal, and end-card beats.

**Creator v2 + LaTeX review set — shipped ✅ (2026-07):** three focused examples
exercise inline and display math through the responsive Creator surface:
`examples/creator-v2-latex-calculus.manic` (9:16 studio),
`examples/creator-v2-latex-algebra.manic` (1:1 paper), and
`examples/creator-v2-latex-physics.manic` (16:9 studio). Portrait, square, and
landscape frames were rendered and visually inspected. The review also fixed
tintable equation images to use semantic template remapping, keeping formula
options legible on light templates.

### Creator v2.4 — questions, options and native socials shipped ✅ (2026-07)

This pass deliberately does **not** expand general image/asset support. It
polishes the high-frequency authored surfaces that should work from DSL alone:

- Question headers now allocate separate decoration and text regions, so the
  kicker/rule cannot collide with a wrapped prompt. Stable tags expose
  `{id}.question` plus `.panel`, `.kicker`, `.rule`, and `.text` roles while
  preserving existing ids such as `q.q` and `q.qrule`.
- `labels=letters|numbers|none` controls the option index treatment. Letters are
  the compatibility default; number/no-label modes suit ordered choices and
  polls. Answer cards reserve a uniform right-side check zone, auto-fit long
  text, and centre a single card in the final row of a five-choice grid.
- Options expose stable `{id}.options`, `{id}.option.a` through `.option.f`,
  role suffixes (`.card`, `.badge`, `.label`, `.text`, `.check`), and
  `{id}.option.correct`. This makes common A/B/C/D styling precise without
  depending on internal compact ids.
- The social footer uses one normalized native-vector registry for YouTube, X,
  Instagram, TikTok, Facebook, LinkedIn, GitHub, web, email, and a generic-link
  fallback. Common aliases normalize to stable tags such as
  `{id}.social.youtube` and `{id}.social.web`. Up to three configured values are
  displayed as professional icon+text lockups; larger sets remain responsive by
  collapsing to icons plus the profile handle.
- Maintained Creator examples now use `yt=zarigatongy`, `x=@anish2good`, and
  `web=8gwifi.org/manic`. The flagship v2 example is asset-free; optional
  `logo=` compatibility remains for authors who intentionally provide a custom
  avatar or channel mark.

Parser/layout/compatibility coverage includes label modes, semantic tags,
five-choice centring, canonical social aliases, exact profile values, and the
unknown-platform fallback. The mdBook, builtin catalog, system prompt, and
Creator/Reel examples document the same shipped surface. The complete 196-test
library suite passes, including editor validation for every shipped example.

### Timing v2 core — generic + Creator adapters shipped ✅ (2026-07)

The original quiz timer deliberately shipped as a small surface
(`ring|bar|number|none`) with a fixed five-second display and motion-dependent
phase percentages. Timing v2 keeps the ring as the polished zero-config default
but separates **choreography** from **timer appearance**:

- `timing(clock,[(x,y)],"intro=1 demo=6 finish=1")` creates a
  format-neutral named-phase controller. `timed(clock) { during("intro") {
  ... } ... }` schedules any ordinary manic animation at exact phase offsets
  while running the native timer in parallel. Phase source order is irrelevant;
  short blocks are padded, while overruns, duplicates, and unknown phases fail
  clearly instead of drifting. `duration=6` is a one-phase `main` shorthand.
- `timing(q,"...")`: pace presets plus explicit `ask`, `options`, `think`,
  `reveal`, `hold`, and `stagger` phases. Explicit phases make `run(q)` derive
  the total duration; legacy `run(q,dur)` continues to scale the preset beat.
- `timerstyle(clock|q,[(x,y)],"...")`: native `ring`, `bar`, `number`,
  `segments`, `ticks`, `pulse`, and `none` looks with responsive position,
  count direction, size, thickness, semantic colours, optional label/digit
  placement, and finish cue. `run(clock)` is the timer-only form; a generic
  controller never accepts a competing `run(clock,dur)` duration.
- Stable timer-part tags expose track/progress/value/label/effects for ordinary
  modifiers. Standalone `countdown` uses the same visual vocabulary.
- SVG is intentionally deferred: native primitives already provide scalable,
  template-aware, progress-animatable timers. A future SVG feature should
  convert paths to native traceable geometry instead of rasterizing them into a
  non-animatable timer image.

Delivered with exact generic/quiz phase and counter tests, backward
compatibility, catalog and prompt coverage, a non-quiz physics example,
dedicated portrait/square/landscape examples, and the six-look comparison
gallery. All 193 tests pass. Mid-countdown frames were rendered and
visually inspected at 9:16, 1:1, and 16:9; that review also corrected horizontal
timer digit/label spacing so segmented and bar looks stay inside their regions.

## Creator format templates — manic for social creators (v1 shipped)

**A new audience: content creators, not just domain educators.** Every kit so far
adds a *domain* (math, physics, optics). This is **orthogonal** — a *format* layer:
opinionated, slot-filled, branded, pre-timed scene generators for social formats
(YouTube **Shorts** / Reels / TikTok). A creator picks a template, drops in content
(a question, four options, an answer) and their branding (handles, accent colour),
and manic produces a polished vertical clip — no timeline authoring, no design
skill. This turns manic from a *tool* into a *product creators return to*.

**Worked example — the quiz Short** (the format the request describes): a question
appears → an animated figure/illustration → four option cards (A–D) → a countdown
timer → time-out → the correct answer is revealed (right card glows, the rest dim)
→ a socials footer (handles + icons). Roughly:

```manic
canvas("9:16");                 // portrait 1080×1920 (already supported)
creator(me, "@anish2good yt=zarigatongy x=@anish2good web=8gwifi.org/manic accent=gold");

// FREEDOM path — builder verbs: any number of options, per-option media later
quiz(q, "Which glass bends BLUE light more?");
option(q, "Crown glass");
option(q, "Flint glass", correct);      // mark the right one
option(q, "Both equal");
option(q, "Neither");
figure(q, prism);               // optional illustration slot — ANY manic entity / kit sim
run(q, 12);                     // plays the whole beat: ask · countdown · reveal
socials(me);                    // the creator's footer, pinned in the safe zone

// EASY path — one-liner shorthand for the canonical 4-option quiz:
//   quiz(q, "Which glass bends BLUE light more?", "Crown", "Flint", "Both", "Neither", answer: 2);
```

**Mostly reuse — the foundation already ships.** Portrait canvas ✅
(`canvas("9:16")` → 1080×1920), the **`reel`** branded preset ✅, engine branding
for 1080×1920 ✅, `par`/`seq`/`wait`/`stagger` timing, `Counter` (a live 5→0
countdown digit), `Arc` (a shrinking timer ring), colour/theme, `banner`/
`watermark`. A countdown = a Counter `Value` track + an `Arc` sweep; a reveal =
`show`/`flash`/`color` on the right card — all existing verbs. **The template only
bakes the layout + the timeline.**

**The `figure` slot takes ANY manic entity** — it references an id, and everything
in manic is an entity, so a shape, a group, a kit sim (`prism`/`triangle`/
`pendulum`), a `def`, or even a **live-animating** sim can be the illustration
(the prism disperses / the geometry constructs *while* the question shows). Bare-id
tag-broadcast moves/scales a multi-part builtin into the slot as one. The only new
bit is **auto-fit**: compute the entity/group's 2-D bounding box (no general helper
today — reuse the footprint-bbox pattern in `three.rs`) and scale+translate it into
the figure region; `figure(q, fig)` auto-fits, or the creator places it and it's
just marked as the slot content.

**⬜ Tracked polish (do after the `creator` kit build):** the figure's small dot
markers (e.g. a circumcentre) are a touch small for a phone screen — bump their
size / add a thin ring so they pop in the `figure` slot.

**Prototype-first — SHIPPED:** the first quiz Short is hand-authored from shipped
primitives in **`examples/quiz-geometry.manic`** (9:16): typewriter `type`
question, an **animated geometry figure** (the geo kit constructs the Euler line —
which *is* the answer), four `rect` option cards, a countdown ring + `say`-driven
digit, a time-out reveal (correct card `recolor`→lime + `flash`/`pulse`, the rest
`fade`), over a `text` socials footer. ~20 s, renders under the `reel` preset. That
proven file is the reference the `quiz`/`countdown`/`socials` builtins will later
collapse to a few lines — the same "build by hand, then extract the builtin" path
the physics sims followed.

**What's genuinely new:**
1. **Reusable UI components** (a small `creator`/`ui` kit): `choices`/`card` (the
   A–D option cards), `countdown` (ring + digit), `reveal` (highlight-correct /
   dim-others beat), `socials` (a handle+icon footer). Useful well beyond quizzes.
   The `figure` slot auto-fits any entity (bounds→scale). **The POC is
   template-agnostic** — it uses only palette-semantic colours (`fg`/`cyan`/
   `magenta`/`lime`/`dim`/`panel`, which the template remaps) and outline-only
   chrome, so it renders with correct contrast on `paper` (light) AND `terminal`
   (dark); the fixed consts (`gold`/`red`/…) are avoided for contrast-critical bits.
2. **✅ Raster image embedding SHIPPED** — `image(id, (x,y), "path", [w], [h])`
   (`Shape::Image` + a thread-local macroquad texture cache preloaded in
   `player::run_loop`, drawn in `render::draw_entity`; missing file → a crossed
   placeholder box). Loads real **logos / avatars / photo backdrops**, animates
   like any entity, `examples/image.manic` + bundled `assets/manic-logo.png`.
   Engine-only (no browser preview — the WASM front-end has no macroquad). The
   quiz POC keeps its *drawn* vector social icons (no trademark PNGs bundled),
   but a creator can now drop their own real logo/avatar in via `image(...)`.
2. **Format templates** — `quiz` first; then a family: `countdown` (N→0 hype),
   `factcard` (hook → fact → source), `listicle` (top-N reveal), `thisorthat`
   (A-vs-B poll), `hotseat` (rapid Q&A). One builtin per format.
3. **Shorts safe-zones** — a portrait layout that keeps content clear of the
   platform UI (bottom action bar, right rail, top clock): a `safezone` helper or
   an automatic inset the templates respect.
4. **A creator profile** — `creator(id, handle, x, yt, ig, tiktok, accent, logo)`
   set once (or in a small reusable file) and reused across every video; drives
   the `socials` footer + accent colour. Extends the brand kit.
5. **A `shorts` theme/preset** — punchy caption sizing, bold outlines, high
   contrast for tiny phone screens, safe-zone insets on by default.

**SHIPPED so far (`src/kits/creator.rs`):** ✅ **`creator(id, "spec")`** — a reusable
profile parsed from a space-separated spec (`@handle`, `yt=`/`x=`/`ig=`/`tt=`/
`fb=`/`li=`/`gh=`/`web=`/`email=` pairs, `accent=colour`), stored in
`Scene::creators`. ✅ **`socials(id, [at])`** — draws the footer using normalized
native platform marks and configured values; `at` defaults to the responsive
bottom safe region. It needs no downloads or image/SVG assets; `logo=` remains
available for a separate custom avatar in compact/signature layouts. ✅ **`quiz(id,
"question")`** + **`option(id, "text", [correct])`** — the question (typewriter,
wrapped) + auto **2×2** option grid + countdown widget; the correct option gets a
lime highlight. ✅ **`run(id, [dur])`** drives the whole **ask → countdown →
reveal** beat (the shared `run` verb dispatches to `build_quiz_clip` when the id is
a quiz — `Scene::quizzes`). `option`/`socials` opt out of tag-broadcast
(`consumes_structure_id`). Figure is author-supplied. **`examples/quiz-euler.manic`
= the ~60-line POC collapsed to `quiz` + 4 `option`s + `run`. FIRST KIT VERSION
COMPLETE.** **Production polish done:** cards **slide up + fade** in (Pos+Opacity),
long answers **wrap** within cards, the reveal **pops** the correct card (lime
highlight Scale-bump + a **drawn ✓**) and **dims** the wrong ones (0.28) instead of
vanishing, and the geo figure **dots are bigger** (`r` 5→7 — the tracked nit).
**Auto-layout done:** `run` lays the answers out by count — a centred column for
≤3, a 2×2 grid for 4+ (2/3/4 all verified) — by computing each slot from the final
count and sliding the cards in via Pos tracks (options are created at a neutral
spot; `run` knows the total). **All the structural features shipped too:**
✅ a **draining ring** (the countdown ring is a full-circle `Arc` whose `trace`
animates 1→0 — the Arc line already honours `trace`, no new prop needed);
✅ **`countdown(id, [at], [secs])`** standalone (draining ring + digit as a
`SimData` playback, `run`-driven); ✅ **`safezone(id, [inset])`** (a faint 9:16
content-safe guide); ✅ **`figure(target, [center], [size])`** (auto-fit: a 2-D
bbox over the group, then a uniform scale+translate of each entity's shape into the
zone — a kit sim / tagged group drops in without hand-placing); ✅ a **`shorts`
template** (neon-on-black, extra glow, no chrome — for phone screens). The
`reveal` beat stays folded into `quiz`'s `run` (no separate builtin needed).
**Creator kit: first production version + all planned features COMPLETE.**

**Production redesign — card SKINS (verified by still-render):** the quiz was
rebuilt from wireframe-grade to broadcast-grade with **4 selectable card skins**,
chosen via the `quiz` style spec (order-free with the reveal, e.g. `"glass fade"`):
`badge` (default — framed question panel + a "QUESTION" kicker pill + coloured
letter-badge answer cards), `minimal` (kicker + accent rule, outline rows), `glass`
(glowing borders, Reels look), and `plain` (flat). One `SkinSpec` table drives the
question header, cards, and reveal, so a new skin is one entry — and every skin
still works under any global `template()`. The reveal now tints + glows the correct
card, draws a check, and turns the correct **badge green**; a persistent faint
track ring means the countdown never decays to a lone digit. All skins verified by
headless `--still` PNG export.

> ⚠️ **Testing status — creative kits need more field testing (pre/post-deploy).**
> The creator kit + the **Shorts system-prompt guidance** are shipping, but they've
> only been exercised on a handful of prompts. The failure modes we've already
> caught-and-fixed are all layout/authoring judgement, not engine bugs: figures
> hand-plotted instead of kit-constructed, `figure()` misused on live geo,
> pre-solved coordinates, the worked-solution act shown unprompted, figure labels
> colliding with the answer cards, and geo point labels left at the 22px default.
> Each fix went into the system prompt, not the engine — which means **the quality
> bar here lives in the prompt and must be validated by generating real Shorts**
> (across models, topics, and question types) and rendering them, not by unit
> tests. **Action items:** (1) build a small regression set of representative
> Short prompts and eyeball the renders after each prompt change; (2) keep the
> `--still` visual-check loop in the deploy workflow; (3) apply the same
> generate-render-critique discipline to **every future creative kit** (new formats
> like countdown/factcard/listicle/this-or-that) before calling them production —
> expect the first cut to need prompt tuning, and budget for it.

**Two layers (mirrors the physics kit's Layer 1 / Layer 2):**
- **Layer 1 — named templates**, for creators: pick `quiz`/`countdown`/…, fill the
  slots, `run`. Zero design skill.
- **Layer 2 — author-your-own template**, for designers: define a reusable format
  (named slots + layout + a default timeline) with `def` + parameters, so a studio
  ships its own branded template and reuses it across a channel.

**Decided (locked):**
- **Content input — freedom + easy:** primarily **builder verbs** (`quiz(q,"?")`
  then `option(q,"text"[,correct])`) so a creator gets full freedom — any number of
  options, per-option media later — AND an **easy one-liner shorthand**
  (`quiz(q,"?","A","B","C","D",answer:2)`) for the common 4-option case. Both map
  to the same quiz structure.
- **Branding — a reusable creator profile:** `creator(id, handle, x, yt, ig,
  tiktok, accent, logo)` set once, ideally in a small file a channel reuses across
  every video; `socials(id)` drops the safe-zone footer. One place to edit brand.
- **A new `creator` kit** (`src/kits/creator.rs`), separate from `brand` (which
  stays about manic's own watermark/intro). Holds `creator`/`socials`/`quiz` +
  the shared UI components + `safezone`.
- **First deliverable — the full `quiz` Short end-to-end** (9:16 layout · question
  · option cards · countdown ring+digit · time-out reveal · socials footer), as
  the proof of the whole format; the reusable components (`countdown`/`choices`/
  `reveal`/`socials`) fall out of it and get exposed standalone.

**First-build sequence (when we start):** `creator` kit skeleton + `creator`
profile + `socials` footer → `safezone` insets for 9:16 → `countdown` (Counter
`Value` + `Arc` sweep) → `choices`/`card` (A–D option cards) → `reveal`
(highlight-correct/dim-others) → the `quiz` template (both input paths) + `run`
beat → a `shorts` theme → one example + book gallery + the builtin checklist.

**Why it fits:** the same "fill it in, get a correct animation" promise aimed at a
huge new audience; ~80% composition of shipped primitives; and the quiz Short
alone is a proven, repeatable viral format — a creator can make one a day.

## Templates / themes

**Shipped.** The look is a selectable **template**, chosen with
`template("name")` (or `--template <name>` at render time). Chrome is driven by
`style::Template` (`Chrome::None|Minimal|Full` + background + masthead strings),
carried on the `Movie` and read by `render::draw_page_chrome`.
- **`mono` (default)** — restrained black-and-white editorial palette on a
  near-black blank screen, no frame/dots/masthead/rule, with a subtle glow. A
  DSL file that omits `template(...)` gets this look.
- **`plain`** — the original saturated neon palette on a blank screen, retained
  as an explicit compatibility choice.
- **`terminal`** — the neon terminal-window chrome (border, corner brackets,
  window dots, centred title, masthead, two-tone rule), now opt-in.

`mono` aliases are `monochrome`, `blackwhite`, `black-white`, and `bw`. Tests
cover the DSL default, explicit-template override, aliases, and greyscale
remapping of every named semantic colour. Both the explicit mono Timing v2
scene and a template-free sine-wave scene were rendered and visually inspected.

**mdBook template guide shipped (2026-07).** Templates now have a dedicated
navigation chapter with a runnable mono sample, selection matrix, aliases,
semantic-colour and `hue(...)` behavior, DSL-versus-CLI override rules,
Creator/Reel recommendations, and phone-size review tips. Getting Started,
Colour & Style, Creator formats, the Reel gold path, and the introduction link
back to the same guide.

**Runtime palette DONE.** Each template carries a `style::Palette` (bg/fg/cyan/
magenta/lime/gold/red/orange/blue/dim/panel). The engine still bakes neon everywhere; the renderer
**remaps** each palette colour to the active template's at draw time
(`Palette::remap`, in `draw_entity`), so `--template` retints **content** too,
while bespoke colours (`hue`, explicit RGB) pass through. Templates: `plain`
(neon palette), `terminal` (neon + chrome), `paper` (ink on cream), `blueprint`
(white/cyan on navy), `shorts` (creator studio), and `mono` (default greyscale).
**Masthead is author-set** (`masthead(...)`), empty by default — no
`manic ~ %` / `60FPS` branding is baked into any template.

**Per-template glow + CRT DONE.** Each template has a `glow` multiplier (applied
to every entity's halo at render) and a `crt` default. `plain`/`terminal` glow
= 1 (neon), `mono` = 0.35 (subtle), `shorts` = 0.65, and
`paper`/`blueprint` = 0 (crisp, flat — right for print). `--crt` still forces
the post-process on regardless of the template default.

**Still to do:** template-controlled **fonts** (needs alternate font assets
bundled — the separate "selectable fonts" work); more palettes; a `minimal`
chrome level exposed as a template.

### Hand-drawn / chalkboard look (planned, undecided)
Requested idea: make the output *look* hand-drawn — chalk on a blackboard,
student/teacher style — not just clean neon geometry. Two independent layers:
- **Chalkboard colours** — a `chalkboard` **template** (dark slate bg + chalky
  off-white/pastel palette + glow off). Small; fits the current template
  structure. Gets the *vibe* but lines stay crisp.
- **Hand-drawn line quality** — a new **`sketch`/rough render style** (NOT
  built): at draw time, perturb every stroke's polyline points with a little
  noise so lines wobble like a human hand, vary width unevenly, and overlay a
  subtle chalk grain/texture (the RoughJS / Manim-xkcd effect). This is what
  actually makes it look hand-drawn. Doable as a render-time pass over paths +
  a grain overlay; medium effort.
- Note: the *motion* already reads as "being drawn" (`draw` traces strokes on,
  `type` reveals text like handwriting) — this is only about the static *texture*.
- The two compose: `chalkboard` template + `sketch` style = teacher-at-the-board.
Decide later.

**What a template bundles today:**
- palette + the complete named-colour map (`fg`, `dim`, `panel`, and every
  semantic accent);
- chrome style (none/minimal/full), glow factor, and CRT default;
- optional author-set masthead text.

Chrome and engine branding are independent. `mono`, `plain`, `paper`,
`blueprint`, and `shorts` have no page chrome; `terminal` opts into the full
window treatment. Recording-preset branding remains separately controllable
with `--no-brand`.

## Web / editor language services — **shipped** (prototype UI)

The editor half of the beta: a browser-loadable build of manic's **language
front-end** that powers an in-page code editor — **syntax highlighting**,
**autocomplete / intelligence**, and **live error-checking with fix
suggestions** — so an author writes `.manic` in the browser and sees exactly
what the renderer would say.

**Status.** All four phases done:
1. `manic-lang` — a macroquad-free workspace crate (lexer/parser/ast/diag),
   publishable, native engine unchanged (depends on it via a re-export).
2. **catalog** — `BuiltinSpec` for all 130 builtins + fixed vocab, kept honest by
   a test asserting the catalog == the live registry (zero drift).
3. **expand** extracted into `manic-lang` (so the browser runs `let`/`for`/`def`).
4. **WASM API** — `tokenize` / `check` / `complete` (`crates/manic-lang/src/services.rs`,
   thin `wasm-bindgen` JSON wrappers under `--features wasm`), built with
   `wasm-pack` (~190 KB), plus a throwaway HTML/JS harness in `web/` (see
   `web/README.md`). The real editor UI is a separate, later design.

All service logic is unit-tested natively (31 `manic-lang` tests) and verified
end-to-end through the compiled WASM. What follows is the design rationale.

### Approach — compile the Rust front-end to WASM (single source of truth)

**Do not re-port the parser to JavaScript.** A hand-written JS parser would drift
from the Rust engine, and the whole point is that what the editor validates is
*exactly* what renders. Instead compile the existing Rust lexer / parser /
expander to `wasm32-unknown-unknown` and expose a thin JS/TS API. One grammar,
one lexer, one set of diagnostics — no divergence, and new builtins light up in
the editor the moment they're added to the engine.

### Prerequisite refactor — a macroquad-free `lang-core`

The renderer pulls in macroquad (graphics), which shouldn't compile into a
headless parser. Split the pure front-end into a crate/feature with no macroquad
dependency:
- **in**: `lexer`, `parser`, `ast`, the **`expand`** pass of `lower`
  (`let`/`for`/`if`/`def`/reductions/interpolation — pure arithmetic over the
  AST), `diag`;
- **out**: `Scene`/`Entity`/`Clip`, `render`, `player`, and the ctor/verb
  *function bodies* (they touch macroquad types);
- the **catalog** (below) replaces the executable registry for validation.

This is the one real structural cost — and it cleanly separates "language" from
"engine", which the architecture already aspires to.

### The builtin catalog (the key new artifact)

Autocomplete + arg-checking need machine-readable specs for every builtin; today
those live in doc comments and hand-written `a.ident(0)?`/`a.num(1)?` calls.
Introduce a structured catalog —
`BuiltinSpec { name, kind: ctor|verb|mut_verb, params:[{name, ty:
name|num|str|point|color|ease|ident, optional}], summary, kit }` — plus the fixed
vocabularies already in the engine: **colors** (`fg void cyan magenta lime dim
panel`), **easings**, **canvas presets**, **template names**, **reserved vars**
(`w h cx cy pi e tau`). Source of truth: generate it where kits already register
(a registration macro that records the signature next to the fn, or a build step
emitting catalog JSON consumed by *both* Rust checks and the WASM API) so it
can't drift.

### WASM API (thin)

- `tokenize(src) -> [{kind, start, len}]` — from the lexer, for highlighting.
- `check(src) -> [{message, start, len, severity, fix?}]` — lex + parse + expand
  + name/arg validation; `fix = {label, replacement, range}` when auto-fixable.
- `complete(src, offset) -> [{label, kind, insertText, detail, doc}]` —
  context-aware (builtins at statement start; the param's type inside a call).
- `signature(src, offset) -> {label, params, activeParam}` — signature help.

### Language services (on CodeMirror 6 or Monaco)

- **Highlighting** — token kinds → classes (keyword `let/for/if/def`, builtin,
  number, string, ident, point punctuation, comment).
- **Diagnostics** — `diag::Error` already localizes precisely by span, and
  several messages already suggest (`try: circular, row, grid`); surface inline.
- **Autocomplete** — builtins by kit at statement start; inside a call the
  expected param type drives suggestions (palette after a color param, easings
  after an ease param, **in-file entity ids + tags** after an id param); reserved
  vars + constants.
- **Quick-fixes** (from the catalog): unknown builtin/color/easing → nearest by
  edit distance (`magena`→`magenta`); reserved id used as an entity name (`h`) →
  offer a rename; missing comma / unmatched paren or brace → insert; wrong arg
  count/type → show the signature and flag the offending arg.

### Boundaries

A language service, **not** a renderer: it validates *syntax, names, arg shape,
and the build-time `expand` pass* — it won't catch issues that only surface at
render (a circle radius overflowing the canvas). Full validation still comes from
`manic check` / a render. A WASM **renderer** (macroquad → WebGL) is a separate,
larger future step.

### Effort / order

Catalog + registration macro (medium, touches each kit once) → `lang-core` split
(medium; the front-end is already fairly decoupled) → WASM API + build (small) →
editor glue (small–medium).

## Where manic is ahead of Asymptote
- A **first-class animation timeline** — asy `animate` stitches frames; manic
  scripts beats (`par`/`seq`/`stagger`, sections, marker export) with
  deterministic recording.
- **Live dynamic constructions** — geo constructions and graph edges recompute
  as inputs move (GeoGebra-style), which static asy diagrams don't do.
