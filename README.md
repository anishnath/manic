# manic

A general-purpose 2D animation **language and engine** — you write a short
`.manic` text file, it renders a glowing, neon-terminal animation to a live
window or an mp4. Deterministic, code-driven, built on
[macroquad](https://github.com/not-fl3/macroquad).

manic is not a Rust API you program against; it's a small language a
non-programmer can read and write (ASY-inspired: function calls, `(x,y)`
points, `;` terminators, `//` comments). It ships with **math** (axes, function
plots, vectors, number lines), **geometry** (olympiad constructions), and a
broad **algorithms** kit (graphs, arrays + sorting, linked lists, stacks/queues,
BFS/DFS, hash maps, Dijkstra); the architecture is domain-agnostic so new
domains plug in without touching the core.

```
title("The Sine Wave");
canvas(1280, 720);

axes(ax, (640, 380), 520, 240);        // a coordinate frame
plot(wave, (640, 380), 78, 120, sin);  // y = sin(x) as a curve
untraced(wave);                        // hidden stroke, ready to draw on

draw(wave, 1.7);                       // trace it on over 1.7s
section("Vectors");
vector(v1, (640, 380), (122, 108));
```

## Run it

```sh
# live preview window (fast, unbranded — the main verify loop; controls below)
cargo run --bin manic -- examples/sine_wave.manic

# parse + report errors, no window
cargo run --bin manic -- check examples/sine_wave.manic

# export one still frame at t = 2.6s
cargo run --bin manic -- examples/sine_wave.manic --still 2.6

# render a video → out/out.mp4 (branded, 1080p; pipes to ffmpeg, PNG fallback)
cargo run --release --bin manic -- examples/sine_wave.manic --record out
```

After `cargo install --path .` the binary is just `manic examples/sine_wave.manic …`.

Any file in [`examples/`](examples) runs the same way — e.g.
`cargo run --bin manic -- examples/bfs_dfs.manic` or `examples/hashmap.manic`.

### Presets

A **preset** sets the render defaults (quality, frame rate, format, branding);
any flag below overrides its fields. Pick one with `--preset <name>` — the
default is `studio`.

| preset | output | branding |
|---|---|---|
| `studio` *(default)* | 1080p, 60fps, MP4 | ✅ intro + watermark |
| `test` | 720p, 30fps, fast | ❌ (for quick checks) |
| `reel` | vertical, branded — pair with `canvas("9:16")` | ✅ |

```sh
cargo run --release --bin manic -- examples/hashmap.manic --record out                 # studio (branded)
cargo run --release --bin manic -- examples/hashmap.manic --record out --preset test    # unbranded, fast
cargo run --release --bin manic -- examples/hashmap.manic --record out --preset studio --no-brand
```

### Branding

Recorded output under a branded preset automatically gets a short **intro**
(the neon fractal tree grows while the `Manic` wordmark types in over
<https://8gwifi.org/manic>) and a **"Made With Manic"** watermark. Branding is added by the engine — it's **never part of your
`.manic` file** — and never appears in the live preview or `--still` (those stay
clean for iteration). Turn it off with `--no-brand`.

### Recording flags

| flag | effect |
|---|---|
| `--preset NAME` | render preset: `studio` (default) · `test` · `reel` |
| `--no-brand` | disable the branding intro + watermark |
| `--record DIR` | render to `DIR/out.mp4` (ffmpeg pipe; deterministic, fixed timestep) |
| `--fps N` | output frame rate (overrides the preset) |
| `--scale F` | supersampling; `1.5` → true 1080p from a 720p canvas, `2` → 1440p |
| `--from S --to S` | record only a time range (clips) |
| `--gif` | write `DIR/out.gif` instead of mp4 |
| `--png` / `--alpha` | PNG sequence (alpha = transparent, no chrome) |
| `--template NAME` | look/chrome: `plain` (default) · `terminal` · `paper` · `blueprint` |
| `--crt` | bake in the CRT scanline + bloom + vignette look |
| `--still S` | export one PNG at time `S` and exit |

Flags override the preset — e.g. `--preset studio --gif` records a branded GIF.

Recording also writes `DIR/markers.json` — section and `mark(...)` timestamps
for lining narration up in an editor.

### Live transport controls

| key | action |
|---|---|
| `Space` | pause / play |
| `←` `→` | step one frame |
| `,` `.` | jump ±1 s |
| `1`–`9` | jump to section markers |
| `F` / `F11` / `Ctrl`+`Cmd`+`F` | fullscreen |
| `R` | restart |
| drag bottom bar | scrub |

## Documentation

- **[GOAL.md](GOAL.md)** — what manic is, why, and the plan.
- **[LANGUAGE.md](LANGUAGE.md)** — the complete language reference: every
  builtin, colors, easings, structure.
- **[CAPABILITIES.md](CAPABILITIES.md)** — what's implemented (kits, presets,
  branding) and what's planned.
- **[SYSTEM_PROMPT.md](SYSTEM_PROMPT.md)** — the guide for generating valid
  `.manic` files with an LLM.
- **[ARCHITECTURE.md](ARCHITECTURE.md)** — how the engine and the language fit
  together, the invariants, and how to add a primitive, a verb, or a whole kit.
- **[ROADMAP.md](ROADMAP.md)** — current status and what's next.

## Requirements

- A recent Rust toolchain (`cargo`, `rustc`).
- `ffmpeg` on `PATH` for mp4/gif output (optional — without it, recording
  writes a PNG sequence and prints the stitch command).

## License

MIT (see [LICENSE](LICENSE)). The embedded fonts are under the SIL Open Font
License — see [LICENSE-FONTS](LICENSE-FONTS).
