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
- **Computation layer** (evaluated at build time): `let` variables; arithmetic
  `+ - * / ^` with **implicit multiplication** (`2sx`, `3(x+1)`), comparisons,
  logic, `pi`/`e`/`tau`, ~20 functions; `for v in a..b` loops; `if/else`;
  recursive `def` macros; reductions `sum`/`prod`/`min`/`max`; id interpolation
  (`bar{i}`). All collapse to literals before rendering — kits are unaffected.
- **Look / config**: `canvas` accepts pixels or presets (`"16:9"`/`"square"`/
  `"portrait"`/`"1080p"`/`"4k"`); `w`/`h`/`cx`/`cy` predefined. Selectable
  **templates** — `plain` (default, blank screen), `terminal`, `paper`,
  `blueprint` — each retints the palette and sets chrome/glow/CRT; author-set
  `masthead` (no engine branding baked in). Same content renders in any template.
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

### Primitives (engine)
`Circle`, `Rect`, `Line`, `Arrow`, `Curve`, `Polygon`, `Polyline`, `Arc`
(arc/sector/annulus), `Region` (boolean result), `Text`.

## Gaps

### Geometry (olympiad) — largely covered now
Done (all **dynamic** unless noted): `meet` (line∩line), **`linecircle`**
(line∩circle), **`circlecircle`** (circle∩circle) — the last two output two
points `{id}0/1`; **`tangent`** (two touch-points from an external point);
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

### Deeper math — numeric & symbolic (future, via a crate)
Not needed for anything built so far (our own expression evaluator covers
numeric plotting/computation, and shape-morph needs only lerp). Tracked so we
don't miss them when a feature calls for one:
- **Numeric linear algebra — `nalgebra`** (mature, low-risk). Add *when* we build
  linear-algebra visuals that need real matrix math: **eigenvectors/eigenvalues**
  of the `transform` matrix, determinant-as-signed-area, matrix
  decompositions, solving `Ax=b`. High value, contained; pull it in at that point,
  not speculatively.
- **Symbolic math / mini-CAS** (`symbolica` or hand-rolled) — a bigger, separate
  decision. Would enable **step-by-step algebra**, **auto-differentiation** (a
  tangent line straight from a formula), and **equation solving** shown on
  screen. The Rust symbolic ecosystem is thin and some options are commercially
  licensed, so this is a *direction* to commit to deliberately, only if
  "show the algebra" animations become a goal. Our numeric evaluator already
  handles evaluation; symbolic adds *manipulation*.
Precedent: we already add a focused crate when a feature needs it (`geo` for
booleans, `earcutr` for triangulation) — same policy here.

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

## Templates / themes

**Shipped (v1).** The look is now a selectable **template**, chosen with
`template("name")` (or `--template <name>` at render time). Chrome is driven by
`style::Template` (`Chrome::None|Minimal|Full` + background + masthead strings),
carried on the `Movie` and read by `render::draw_page_chrome`.
- **`plain` (default)** — a blank screen: background + content only, no frame /
  dots / masthead / rule. This is now the out-of-the-box look.
- **`terminal`** — the neon terminal-window chrome (border, corner brackets,
  window dots, centred title, masthead, two-tone rule), now opt-in.

**Runtime palette DONE.** Each template carries a `style::Palette` (bg/fg/cyan/
magenta/lime/dim/panel). The engine still bakes neon everywhere; the renderer
**remaps** each palette colour to the active template's at draw time
(`Palette::remap`, in `draw_entity`), so `--template` retints **content** too,
while bespoke colours (`hue`, explicit RGB) pass through. Templates: `plain`
(default, neon palette), `terminal` (neon + chrome), `paper` (ink on cream),
`blueprint` (white/cyan on navy). **Masthead is author-set** (`masthead(...)`),
empty by default — no `manic ~ %` / `60FPS` branding is baked into any template.

**Per-template glow + CRT DONE.** Each template has a `glow` multiplier (applied
to every entity's halo at render) and a `crt` default. `plain`/`terminal` glow
= 1 (neon); `paper`/`blueprint` glow = 0 (crisp, flat — right for print). `--crt`
still forces the post-process on regardless of the template default.

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

**What a template bundles:**
- palette + the named-colour map (what `cyan`/`magenta`/`lime`/`fg`/`dim`/`bg`/
  `panel` resolve to — each theme can retint these semantic roles);
- fonts (mono / display);
- chrome style (terminal window frame · plain · paper/notebook · blueprint);
- glow factor and CRT default (on for neon, off for a print look);
- masthead text/format.

**Chrome is developer branding — must be optional.** Today every frame bakes in
the terminal frame, traffic-light dots, the accent rule, and the masthead text
`manic ~ %` / `60FPS · DETERMINISTIC` (all in `render.rs::draw_page_chrome`).
A *content author* (the target user) doesn't want engine branding in their
video. So chrome needs levels — at minimum **`full`** (frame + dots + masthead,
today's look), **`minimal`** (masthead only, no window frame), and **`clean`**
(nothing but the author's content on the background). The masthead is
**author-set or empty** — never baked technical text like "60FPS ·
DETERMINISTIC". Selectable per-movie (`chrome("clean")` / part of the template)
and via a `--clean` CLI flag. This is small and independently useful — worth
shipping ahead of the full theme refactor.

**How to address it (extend, don't fork the renderer):**
1. Replace the `pub const` palette in `style.rs` with a runtime `Theme` struct;
   `Theme::neon()` holds today's exact values (zero visual change by default).
   Ship a few built-ins: `neon` (default), `paper` (light ink-on-cream, no glow/
   CRT), `blueprint` (cyan-on-navy grid), `slate` (muted dark).
2. Make colour resolution theme-aware: `resolve_color(name)` and kit default
   colours read the active theme's role map instead of the constants. (Kits keep
   using semantic names — `style::CYAN` becomes `theme.cyan`.)
3. Carry the chosen `Theme` on the `Movie`; `render.rs`/`player.rs` read chrome/
   glow/CRT from it instead of hard-coded values.
4. Selection: a top-level `template("neon")` statement (reserved control name)
   with a `--theme <name>` CLI override.

**Effort:** medium — a focused refactor (style → runtime `Theme`; thread through
`resolve_color`, `render`, `player`; one language keyword), not a rewrite.
**Note:** existing examples assume dark-bg + glow, so a light theme intentionally
changes their look — that's the feature; neon remains default so nothing breaks.
Composes with the separately-planned **selectable fonts** (a theme picks fonts;
custom fonts refine within a theme).

## Where manic is ahead of Asymptote
- A **first-class animation timeline** — asy `animate` stitches frames; manic
  scripts beats (`par`/`seq`/`stagger`, sections, marker export) with
  deterministic recording.
- **Live dynamic constructions** — geo constructions and graph edges recompute
  as inputs move (GeoGebra-style), which static asy diagrams don't do.
