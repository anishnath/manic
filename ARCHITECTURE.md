# manic — architecture

Two halves: a **language front end** (`src/lang/`) that turns text into a
`Movie`, and an **engine** (the rest of `src/`) that turns a `Movie` into
pixels. Domain vocabulary lives in **kits** (`src/kits/`) that register
builtins into the language. The core depends on nothing above it; kits depend
on the core.

## The one invariant that matters

**The timeline is stateless.** `Timeline::apply(base_scene, t)` is a pure
function: it computes every animated property at absolute time `t` directly
from resolved keyframes. Nothing accumulates frame to frame.

Everything else falls out of this:

- **Pause / step / scrub** — evaluate any `t`, in any order.
- **Deterministic recording** — frame `f` is rendered at `t = f / fps`, wall
  clock ignored; output is bit-identical across runs.
- **No ordering bugs** — a property's value at `t` depends only on its own
  track list, never on what was drawn last frame.

Keep this invariant when extending. If a feature tempts you to mutate the scene
persistently mid-playback, express it as tracks/events instead.

## Module map

```
src/
├── lib.rs         re-exports, prelude, run(), parse() — the language front door
├── bin/manic.rs   the `manic` CLI (play / check / --record / --still …)
│
│  ENGINE (domain-agnostic core)
├── primitives.rs  Entity + Shape (circle, rect, line, arrow, curve, polygon,
│                  polyline, text) + StrokeStyle, FontKind, Align
├── scene.rs       Scene (id-addressed entity store) + SceneBuilder (Rust DSL)
├── easing.rs      Easing enum, pure f32 → f32 curves
├── timeline.rs    TrackSpec/Clip (unresolved) → Timeline (resolved), apply()
├── animate.rs     the Rust verb DSL: act(), ActBuilder, seq!/par!, stagger
├── movie.rs       Movie: base scene + cursor-based clip placement + sections
├── style.rs       the house style: neon palette, embedded fonts, chrome strings
├── render.rs      scene → macroquad draw calls; glow halos; terminal chrome
├── player.rs      live window w/ transport controls, or offline record loop;
│                  CRT shader; render-target capture (+ vertical flip on export)
├── record.rs      raw-RGBA → ffmpeg pipe (mp4/gif) or PNG sequence; markers.json
├── layout.rs      pure geometry helpers: row, grid, ring, tree
│
│  LANGUAGE FRONT END (domain-agnostic)
├── lang/
│   ├── diag.rs    Span + Error + render() (line/col caret diagnostics)
│   ├── lexer.rs   text → tokens (knows no keywords)
│   ├── ast.rs     Program = flat list of Stmt (name + args + optional block)
│   ├── parser.rs  recursive descent; validates shape only, not meaning
│   └── lower.rs   AST → Movie via a Registry; Args helpers; color/easing resolvers
│
│  KITS (domains register vocabulary here)
└── kits/
    ├── mod.rs     default_registry() = std + math (+ algo, later)
    ├── std.rs     always-on base: shapes, modifiers, animation verbs
    └── math.rs    a domain: axes, plot, vector, numberline
```

## Data flow

```
.manic text
   │  lang::lexer::lex          → Vec<Token>
   ▼
   │  lang::parser::parse       → Program (generic calls; no meaning yet)
   ▼
   │  lang::lower::lower(src, registry)
   │    phase 0: title/canvas → Movie::new
   │    phase A: constructors  → build the base Scene (t = 0)
   │    phase B: verbs/blocks  → Clips placed on the Movie timeline
   ▼
 Movie  (base Scene + placed Clips + sections + marks)
   │  Movie::finalize → Timeline::resolve (pins every track's `from`)
   ▼
per frame:  Timeline::apply(base, t) → Scene copy → render::draw_scene
   │  player: live blit to window, or record::Recorder → ffmpeg
   ▼
 window  |  out/out.mp4  +  out/markers.json
```

### How lowering resolves calls (the seam)

The lowerer knows only a handful of reserved control-flow names — `title`,
`canvas`, `par`, `seq`, `stagger`, `section`, `wait`, `beat`, `mark`. Every
other call name is looked up in the **`Registry`**, which kits populate with:

- **constructors** `fn(&mut Scene, &Args) -> Result<(), Error>` — declare or
  modify entities in the base scene (run in phase A, source order);
- **verbs** `fn(&Scene, &Args) -> Result<Clip, Error>` — produce a timeline
  clip (run in phase B; may read the finished base scene to resolve an id to a
  position).

Constructors run before verbs, so a beat may reference an entity declared
lower in the file — order the cast and the script however reads best. An
unknown call name is a build-time error (`unknown builtin`), pointed at the
exact token.

### How `resolve` works (the clever engine bit)

Verbs emit tracks with three target kinds: `Abs` (go to this value), `Rel` (go
to current + delta), and `Revert` (go back to the value before the previous
track — used by `flash`/`pulse` auto-restore). At finalize time, one forward
pass per (entity, property) in chronological order turns all of these into
concrete `from → to` pairs. After that, playback is dumb interpolation.

Corollary: two tracks animating the **same property of the same entity at
overlapping times** resolve in start order and the later one wins visually.
Don't overlap them deliberately; combine different properties instead (color +
scale + opacity coexist fine).

## Extension recipes

### Add a primitive

1. Add a variant to `Shape` in `primitives.rs`.
2. Add a match arm in `render::draw_entity`.
3. (Optional) If it has animatable geometry like `Line.to`, add a `Prop`
   variant and wire it in `timeline::{get_prop, set_prop}`.

Nothing else in the engine knows about shapes. (This is exactly how
`Shape::Polyline` was added for function plots.)

### Add an animation verb (Rust engine layer)

1. Add a variant to `Verb` in `animate.rs`.
2. Add a builder method on `ActBuilder` (the public Rust API).
3. Add a match arm in `build_clip` emitting one or more `TrackSpec`s.

Compound gestures are just multiple tracks: `pulse` = scale-up + `Revert`,
`shake` = six `Rel` segments summing to zero, `set_text` = fade-out + swap
event + fade-in.

### Add a builtin to the language

In a kit, write a `fn(&mut Scene, &Args)` (constructor) or `fn(&Scene, &Args)
-> Result<Clip>` (verb) and register it: `r.ctor("name", f)` / `r.verb("name",
f)`. Use the `Args` accessors (`ident`, `num`, `pair`, `point`, `text`,
`opt_num`) for typed, span-aware argument errors, and `resolve_color` /
`resolve_easing` / `apply_dur_ease` for the shared vocabulary.

### Add a whole domain (a kit)

1. New file `src/kits/mydomain.rs` with a `pub fn register(r: &mut Registry)`.
2. One line in `kits::default_registry()`.

No core file changes. That is the design goal — see [GOAL.md](GOAL.md).

## The house style

`style.rs` owns the identity: the neon palette (`VOID`, `FG`, `CYAN`,
`MAGENTA`, `LIME`, `DIM`, `PANEL`), the embedded IBM Plex Mono fonts (OFL,
compiled into the binary so renders are portable), and the masthead strings.
`render::draw_page_chrome` draws the terminal frame every frame; glow halos are
drawn behind fully-traced strokes and text. Change the look once here; every
animation follows.

## Recording pipeline

`--record DIR` renders at a fixed timestep and pipes raw RGBA straight into
ffmpeg (`DIR/out.mp4`), or writes a PNG sequence when ffmpeg is absent or
`--png`/`--alpha` is set. Note: macroquad render-target textures come back
bottom-up from `get_texture_data()`, so `player.rs` flips each captured frame
vertically — exports match the live window. `--fps 2` sparsely samples the
whole movie, a cheap visual proof-read of a full video.
