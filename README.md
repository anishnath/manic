# manic

A general-purpose 2D animation **language and engine** — you write a short
`.manic` text file, it renders a glowing, neon-terminal animation to a live
window or an mp4. Deterministic, code-driven, built on
[macroquad](https://github.com/not-fl3/macroquad).

manic is not a Rust API you program against; it's a small language a
non-programmer can read and write (ASY-inspired: function calls, `(x,y)`
points, `;` terminators, `//` comments). It ships with **math** (axes, function
plots, vectors, number lines, calculus, linear algebra), **statistics** (histograms,
bell curves, the CLT, inference), **physics** (RK4 simulations — pendulums, springs,
pulleys, and more), **geometry** (olympiad constructions), and a broad
**algorithms** kit (graphs, arrays + sorting, linked lists, stacks/queues, BFS/DFS,
hash maps, Dijkstra); the architecture is domain-agnostic so new domains plug in
without touching the core.

Creator stories can also expose one bounded value and connect several views
without rebuilding the scene: `parameter(a,...)`,
`bind(a,curve,formula,"p*x*x")`, then ordinary named `step`s animate only
`to(a,value,...)`. See [`examples/parameter-journeys.manic`](examples/parameter-journeys.manic).

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

# publishing audit: settled stages across portrait, feed, square, and landscape
cargo run --bin manic -- check examples/reactive-multiformat.manic --canvas all

# inspect the story, then preview one named stage
cargo run --bin manic -- stages examples/reactive-world.manic
cargo run --bin manic -- examples/reactive-world.manic --stage find-the-flat-point

# export one still frame at t = 2.6s
cargo run --bin manic -- examples/sine_wave.manic --still 2.6

# render a video → out/out.mp4 (branded, 1080p; pipes to ffmpeg, PNG fallback)
cargo run --release --bin manic -- examples/sine_wave.manic --record out
```

After `cargo install --path .` the binary is just `manic examples/sine_wave.manic …`.
Examples that use `asset:` also need the repository `assets/` directory (or
`MANIC_ASSETS_DIR`) unless installed through the Docker/Linux production bundle.

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
| `reel` | vertical, branded — pair with `canvas("9:16")` or `--canvas portrait` | ✅ |

```sh
cargo run --release --bin manic -- examples/hashmap.manic --record out                 # studio (branded)
cargo run --release --bin manic -- examples/hashmap.manic --record out --preset test    # unbranded, fast
cargo run --release --bin manic -- examples/hashmap.manic --record out --preset studio --no-brand
```

### Branding

Recorded output under a branded preset automatically gets a short **intro**
(the neon fractal tree grows while the `Manic` wordmark types in over
<https://8gwifi.org/manic>) and a **"Made With Manic"** watermark. Preset
branding is added by the engine and need not appear in the `.manic` file; an
author can still add an intentional in-scene `watermark(...)`. Automatic preset
branding never appears in live preview or `--still`. Turn it off with
`--no-brand`.

### Recording flags

| flag | effect |
|---|---|
| `--preset NAME` | render preset: `studio` (default) · `test` · `reel` |
| `--no-brand` | disable the branding intro + watermark |
| `--record DIR` | render to `DIR/out.mp4` (ffmpeg pipe; deterministic, fixed timestep) |
| `--canvas FORMAT` | reframe one responsive source before layout: `portrait` · `4:5` · `square` · `16:9` · `WIDTHxHEIGHT` |
| `--fps N` | output frame rate (overrides the preset) |
| `--scale F` | supersampling; `1.5` → true 1080p from a 720p canvas, `2` → 1440p |
| `--stage NAME` | preview or record exactly one named `step` stage |
| `--from-stage NAME --to-stage NAME` | select an inclusive story-stage range |
| `--from S --to S` | record only a numeric time range (cannot mix with named ranges) |
| `--gif` | write `DIR/out.gif` instead of mp4 |
| `--png` / `--alpha` | PNG sequence (alpha = transparent, no chrome) |
| `--template NAME` | look/chrome: `mono` (default) · `plain` · `terminal` · `paper` · `blueprint` · `shorts` |
| `--crt` | bake in the CRT scanline + bloom + vignette look |
| `--still S` | export one PNG at time `S` and exit |

Flags override the preset — e.g. `--preset studio --gif` records a branded GIF.

`--canvas` is a layout override, not an output-quality preset. It changes the
logical canvas before `w`, `h`, `cx`, `cy`, and build-time layout branches are
evaluated, so one responsive story can produce each platform version:

```sh
manic examples/reactive-multiformat.manic --canvas portrait --record out-reel --preset reel
manic examples/reactive-multiformat.manic --canvas 4:5     --record out-feed
manic examples/reactive-multiformat.manic --canvas square  --record out-square
manic examples/reactive-multiformat.manic --canvas 16:9    --record out-lesson
```

Before recording every format, run
`manic check examples/reactive-multiformat.manic --canvas all`. It audits the
settled state of each named stage for canvas and Creator-safe-area overflow,
substantial content overlaps, and unreadably small text/notation. Diagnostics
name the format, stage, time, and entity and return a failing status until the
layout is clean. Ordinary `manic check` remains the fast parse-and-validation
path.

Run `manic stages FILE.manic` to inspect stage starts, ends, and durations before
opening a window. Recording writes `DIR/markers.json` with the selected source
range, clip-relative stage intervals, and filtered section/`mark(...)`
timestamps for lining narration up in an editor.

### Live transport controls

| key | action |
|---|---|
| `Space` | pause / play |
| `←` `→` | step one frame |
| `,` `.` | jump ±1 s |
| `1`–`9` | jump to named story stages (sections when no stages exist) |
| `F` / `F11` / `Ctrl`+`Cmd`+`F` | fullscreen |
| `R` | restart the selected stage/range |
| click stage strip | jump directly to that stage |
| drag bottom bar | scrub inside the selected stage/range |

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
- **[PUBLISHING.md](PUBLISHING.md)** — end-to-end publish flow: build the book,
  render + upload the demo videos, embed players, and deploy to `/manic/docs`.

## Requirements

- A recent Rust toolchain (`cargo`, `rustc`).
- `ffmpeg` on `PATH` for mp4/gif output (optional — without it, recording
  writes a PNG sequence and prints the stitch command).

## Linux binaries & deploy (Docker / EC2)

manic runs headless on Linux (e.g. an Ubuntu EC2 box). macroquad loads
OpenGL/X11 at runtime, so **rendering** needs a virtual display + software GL +
ffmpeg; **`check`** (parsing) needs none of that.

- **Docker** (self-contained render service): `docker build -t manic -f
  docker/Dockerfile .`, then `docker run --rm -v "$PWD/out:/work/out" manic
  manic examples/hashmap.manic --record out`. Recording is deterministic, so
  container output is byte-identical to a desktop run.
- **Prebuilt binary on a box:** cross-build both Linux arches into `dist/` with
  [`scripts/build-linux.sh`](scripts/build-linux.sh) (`arm64` for Graviton,
  `amd64` for Intel/AMD — match `uname -m`). The build also writes
  `dist/manic-assets.tar.gz`; install it under
  `/usr/local/share/manic/assets`, then run
  [`scripts/ec2-setup.sh`](scripts/ec2-setup.sh) (installs xvfb + mesa + ffmpeg
  and a `manic-render` wrapper). The GitHub deploy workflow performs both
  installs automatically. Fonts remain embedded; the asset catalog supplies
  optional stable `asset:` resources. (Binaries are built on glibc 2.36 →
  Ubuntu 24.04; for 22.04, rebuild on an older base.)

## License

MIT (see [LICENSE](LICENSE)). The embedded fonts are under the SIL Open Font
License — see [LICENSE-FONTS](LICENSE-FONTS).
