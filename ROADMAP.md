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

### More math vocabulary

- [ ] `point(id, axes, (dataX, dataY))` — a labelled dot placed in *data*
      coordinates relative to an `axes` (needs axes to remember its scale).
- [ ] `angle`, `brace`, `bracket`, `grid` (lattice), `arc`.
- [ ] `plot` from a small arithmetic expression (`"x^2 - 3"`) — a tiny
      expression evaluator, so functions aren't limited to the named set.

### The next domain

- [ ] **`algo` kit** — `cells` (arrays / bit vectors), `tree`, `graph`,
      `code_block` (per-line addressable), reusing `layout::{row,grid,ring,
      tree}`. Proves the "tomorrow it's algorithms" claim: a new file +
      one line in `default_registry()`, zero core changes.

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

### Bigger swings (later)

- [ ] **3D** — `render::View::xform` is the single projection seam; a camera
      matrix and a z on `Entity.pos` go there. Nothing in scene/timeline/lang
      cares.
- [ ] **Web playground (WASM)** — edit `.manic` in the browser with live
      preview, like the Mermaid live editor.
- [ ] **Shape morphing** — polygon → polygon interpolation.

## Explicitly not planned

- A GUI/editor for building animations — the readable text language is the
  point.
- Audio playback/sync inside the engine — that's post-production's job; marker
  export is the hand-off.
