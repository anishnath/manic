# manic — status & roadmap

Where the project is and what's next. Every future item must preserve the core
invariant (timeline evaluation stays a pure function of `t`) and the boundary
(kits depend on the core; the core never depends on a kit).

## Status: the full vertical slice works

A `.manic` file lexes → parses → lowers through the registry → renders as a
neon frame, live or to mp4. 16 unit tests pass, no warnings.

### Done

- [x] **Engine** — primitives (circle, rect, line, arrow, curve, polygon,
      polyline, text), id-addressed scene, stateless keyframe timeline,
      resolve/apply, the verb DSL (`act`, `seq!`/`par!`, `stagger`), movie with
      cursor placement + sections + marks.
- [x] **Neon-terminal identity** — palette, embedded IBM Plex Mono, glow halos,
      terminal-window chrome (corner brackets, window dots, shell prompt,
      two-tone rule), verified on screen.
- [x] **Live player** — transport controls, fit/letterbox, HUD, section jumps.
- [x] **Deterministic recording** — fixed-timestep ffmpeg pipe → mp4/gif, PNG
      fallback, `markers.json`; render-target export flip fixed. `--crt` CRT
      scanline/bloom/vignette post-process. Verified: mp4 produced.
- [x] **Language front end** — lexer (no keywords), generic AST, recursive
      descent parser, line/col caret diagnostics.
- [x] **Registry + lowering** — two-phase lower (constructors → base scene,
      then verbs/blocks → timeline); reserved control flow; `Args` typed
      accessors; shared color/easing resolvers.
- [x] **std kit** — text/dot/circle/rect/line/arrow; modifiers; ~20 verbs
      including a general `to(id, property, value)` escape hatch (animate any of
      x/y/opacity/scale/angle/trace/color) plus `rotate`/`spin` — so authors
      can animate anything, not just a canned set.
- [x] **Rotation** — `Prop::Rot` animatable; render rotates rect/line/arrow/
      curve/polygon/polyline/text.
- [x] **math kit** — `axes`, `plot` (12 named functions), `vector`,
      `numberline`, and circular geometry `arc` / `sector` / `annulus` /
      `pie(n)` (built on a core `Arc` primitive covering Manim's
      Arc/Sector/Annulus/AnnularSector).
- [x] **CLI** — `manic FILE`, `manic check FILE`, pass-through record/still/crt
      flags.
- [x] **Example** — `examples/sine_wave.manic` (+ Rust `examples/smoke.rs`).
- [x] **Docs** — README, GOAL, ARCHITECTURE, LANGUAGE, this file, LICENSE(-FONTS).

## Next

### Near-term polish

- [ ] Friendlier CLI: `manic record FILE -o out.mp4` (a real `record`
      subcommand, not just the `--record` flag) and `manic still FILE -t 2 -o
      x.png`.
- [ ] `cargo install --path .` smoke check + a short "install" note.
- [ ] A second, richer math example (a derivative/tangent or transform demo).
- [ ] Golden-frame tests: render a few fixed frames and hash them to guard the
      renderer.

### Animation model — parity with Manim's `Animation` contract

manic's animation is declarative + stateless (`Timeline::apply(base, t)` is
pure), vs Manim's imperative per-object lifecycle (`begin`/`interpolate`/
`finish`, updaters, remover/introducer). Most of Manim's lifecycle surface is
intentionally absent (opacity model instead of add/remove; no per-frame
mutation). What already maps: `run_time` → `.dur`, `rate_func` → `.ease`,
`AnimationGroup`/`Succession`/`LaggedStart` → `par`/`seq`/`stagger`. Genuine
gaps to close:

- [x] **Updaters — geometric** — the general `derive` hook: an entity carries
      `deps` (input ids) + a `derive` fn, recomputed each frame in
      `Timeline::apply` (pure function of `t`, no mutation). The core stays
      domain-agnostic (it just calls the hook with resolved dep positions);
      kits supply the geometry. Powers all dynamic geo constructions;
      generalizes `follow` and `Link`.
- [ ] **Value-trackers — numeric / user-expressible** (still open) — a text
      that tweens a *displayed number* (a counter), a bar whose length reflects
      a value, an angle readout. Needs a way to express a derived *scalar* and
      bind it into text/geometry from the DSL, not just hard-coded `derive`
      fns in kits.
- [ ] **`lag_ratio` (fixed-duration stagger)** — a `stagger` variant that keeps
      total runtime constant and compresses members, matching Manim's
      `lag_ratio` semantics (ours currently extends duration).
- [ ] **More rate functions** — port Manim's `rate_functions`
      (there-and-back, wiggle, smootherstep, …) as new `Easing` variants.
- [ ] **`reverse` / `.reversed()`** on a verb (reverse the rate function).

### More math vocabulary

- [ ] `point(id, axes, (dataX, dataY))` — a labelled dot placed in *data*
      coordinates relative to an `axes` (needs axes to remember its scale).
- [ ] `angle`, `brace`, `bracket`, `grid` (lattice), `arc`.
- [ ] `plot` from a small arithmetic expression (`"x^2 - 3"`) — a tiny
      expression evaluator, so functions aren't limited to the named set.

### Shipped domains (since this section was written)

- [x] **`algo` kit** — graphs, arrays + sorting (`swap`/`compare`), linked lists
      (singly/doubly/circular), stacks/queues, hash maps, BFS/DFS, Dijkstra.
- [x] **`geo` kit** — olympiad constructions (see below).
- [x] **`stats` kit** — histograms, summary/boxplot/skew, bell curve + CLT,
      LLN, correlation, hypothesis testing, covariance, Bayes (13 builtins).
- [x] **`physics` kit** — n-dim RK4 + a `Sim` trait + four generic views
      (`phase`/`well`/`timegraph`/`energygraph`); ~25 sims: the pendulum family,
      the spring family, and mechanics (robot arm, pulleys incl. Atwood /
      block-and-tackle / compound, ramp, drop-mass, raft, brachistochrone,
      piston, molecule). Determinism preserved by pre-simulating at build time.
- [x] **Textbook / paper rendering** — `template("paper")` palette remap + the
      std `support` (hatched wall/ceiling/floor) and `sticky` (screen-pin)
      primitives compose over ANY kit (physics figures, a linked list, …).

### The `algo` kit — how it started

- [x] **`graph`** (Manim `Graph`/`DiGraph`) — labelled circle nodes + `a-b`
      (line) / `a>b` (arrow) edges trimmed to borders; `circular`/`row`/`grid`
      layouts; everything tagged `id`/`.nodes`/`.edges`. Proved the thesis:
      new file `src/kits/algo.rs` + one `default_registry()` line, zero core
      changes. See `examples/graph.manic`.
- [x] **Tag broadcast** (general language feature) — a verb/modifier whose
      first arg names a tag applies to the whole group; dotted ids
      (`g.nodes`, `g.a`) now lex. Makes multi-entity groups usable.
- [ ] `cells` (arrays / bit vectors), `tree`, `code_block` (per-line
      addressable), reusing `layout::{row,grid,ring,tree}`.

### Third domain — `geo` kit (olympiad geometry, started)

- [x] **`geo` kit** (à la Asymptote `olympiad.asy`/`cse5.asy`) — `point`,
      `segment` (reflowing), derived points `midpoint`/`centroid`/
      `circumcenter`/`incenter`/`orthocenter`/`foot`/`meet`, `circumcircle`,
      `incircle`, `anglemark`, `rightangle`. Constructions computed at build
      time from referenced points (static, like asy). `src/kits/geo.rs` + one
      registry line, zero core changes. See `examples/triangle.manic`.
- [x] **Dynamic constructions** — all geo constructions recompute each frame
      from their referenced points via the general `derive` hook (below), so
      dragging a vertex updates the circumcircle/incircle/foot/angle-mark and
      reflows the sides live (GeoGebra-style). See the "Drag a vertex" finale
      in `examples/triangle.manic`.
- [ ] Follow-ups: `tangent`/`tangentline`, `perpbisector`, `bisector`,
      `reflect`/`rotate about point`; label direction control (`dir`).
- [x] **Edges follow moving vertices** (Manim's updater behaviour) — an
      `Entity.link { from, to, trim }` derives a line/arrow's endpoints from
      two other entities every frame in `Timeline::apply`. The first concrete
      updater, expressed as a pure function of `t` (no mutation, keeps the
      invariant). See `examples/graph_moving.manic`.

### Boolean shape ops — done

- [x] **Boolean ops** (Manim's Union/Intersection/Difference/Exclusion) —
      `union` / `intersect` / `difference` / `exclusion(xor)` combine two
      fillable shapes into a `Region`. Robust 2D clipping via the `geo` crate
      (`i_overlay` under the hood); results (with holes + multiple pieces)
      triangulated with `earcutr` for fill. Core: `src/geom.rs` +
      `Shape::Region`. See `examples/boolean.manic`.
- [ ] Follow-ups: allow booleans on booleans (nested ops — needs structured
      Region data, not just baked triangles); `Ellipse` primitive (cheap).

### The hard 20%

- [ ] **LaTeX / math typesetting** — prerender formulas to glyphs/mesh and draw
      as a primitive. Deliberately deferred; math v1 uses mono labels. Likely a
      new core primitive + a build/asset step.

### 3D — planned (design agreed, deferred)

Matches Manim's `ThreeDCamera` (phi/theta/gamma, zoom, `project_point`). manic
was built for this: every world coordinate already flows through the single
`render::View` projection seam, and `sticky(id)` already == Manim's
`add_fixed_in_frame_mobjects`.

**Two implementation routes:**
- **(A) full `Vec3` positions** — `Entity.pos`, every shape's points, and the
  timeline's vector value become 3D. Fully general (3D objects can *move*), but
  touches the whole core (primitives, timeline, animate, render, every kit) and
  risks regressing 2D. → **Phase 2.**
- **(B) additive 3D layer** — leave the 2D core untouched; add 3D as new
  primitives + a camera projected at the seam, with *static* 3D geometry and an
  *animated camera orbit* (the core `ThreeDCamera` use case). → **Phase 1
  (do this first).**

**Phase 1 (additive) design:**
1. **Camera as an entity** — reserved `__cam3d` holds phi (`pos.x`), theta
   (`pos.y`), zoom (`scale`), so camera animation reuses existing `Pos`/`Scale`
   tracks — no new timeline machinery; orbiting is tweening those.
2. **Projection at the seam** — `render::View` gains a 3D mode:
   `project(Vec3) → screen` via a phi/theta rotation + orthographic projection
   + zoom (weak-perspective later). 2D entities and the terminal chrome keep
   rendering screen-space, so existing examples stay pixel-identical.
3. **New primitive `Path3 { pts: Vec<Vec3>, closed, arrow }`** — 3D lines,
   axes, parametric curves, surface wireframes; stroked, glow + draw-on trace.
4. **`anchor3: Option<Vec3>` on `Entity`** — pins any 2D shape (dot, text,
   circle) to a projected 3D point, always facing the viewer (== Manim's
   `add_fixed_orientation_mobjects`); reuses all 2D shapes as 3D markers/labels.
5. **Depth sort** — painter's algorithm on camera-space z for 3D content;
   chrome/UI stays on top.
6. **DSL** — parser accepts `(x, y, z)` triples (`(x, y)` ⇒ z 0);
   `camera3d(phi, theta, [zoom])` to enable; `orbit(Δtheta, [dur])` /
   `phi(deg,…)` / `theta(deg,…)` verbs. New **`math3d` kit**: `axes3d`,
   `line3`, `plot3` (parametric), `surface` (wireframe), `point3`/`label3`.

**Phase 1 sub-steps (each builds + tests green before the next):**
1. `Vec3` triples in lexer/parser/AST + lower (2D pairs still work).
2. `View` 3D projection + `__cam3d` + depth sort; verify `sine_wave` renders
   identically.
3. `Path3` primitive + render arm.
4. `anchor3` billboard pinning.
5. camera verbs (`camera3d`, `orbit`, `phi`, `theta`).
6. `math3d` kit + an orbiting `examples/axes3d.manic`.

**Phase 1 deliverable:** an orbiting 3D scene (axes + a parametric curve or
surface, with billboard x/y/z labels), recorded to mp4.

**Phase 2 (later):** the `Vec3` rewrite (route A) so 3D objects translate/morph
over time, not just the camera.

### Bigger swings (later)

- [ ] **Web playground (WASM)** — edit `.manic` in the browser with live
      preview, like the Mermaid live editor.
- [ ] **Shape morphing** — polygon → polygon interpolation.
- [ ] **Perspective + shading** — weak-perspective camera and depth/normal
      shading (Manim's `focal_distance` / `shading_factor`), once 3D lands.

## Explicitly not planned

- A GUI/editor for building animations — the readable text language is the
  point.
- Audio playback/sync inside the engine — that's post-production's job; marker
  export is the hand-off.
