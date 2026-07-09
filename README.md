# manic

A general-purpose 2D animation **language and engine** — you write a short
`.manic` text file, it renders a glowing, neon-terminal animation to a live
window or an mp4. Deterministic, code-driven, built on
[macroquad](https://github.com/not-fl3/macroquad).

manic is not a Rust API you program against; it's a small language a
non-programmer can read and write (ASY-inspired: function calls, `(x,y)`
points, `;` terminators, `//` comments). The first domain it ships with is
**math** (axes, function plots, vectors, number lines); the architecture is
domain-agnostic so new domains — algorithms next — plug in without touching
the core.

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
# live preview window (transport controls below)
cargo run --bin manic -- examples/sine_wave.manic

# parse + report errors, no window
cargo run --bin manic -- check examples/sine_wave.manic

# export one still frame at t = 2.6s (1080p)
cargo run --bin manic -- examples/sine_wave.manic --still 2.6 --scale 1.5

# render a video → out/out.mp4 (pipes to ffmpeg; falls back to a PNG sequence)
cargo run --release --bin manic -- examples/sine_wave.manic --record out --fps 60
```

After `cargo install --path .` the binary is just `manic examples/sine_wave.manic …`.

### Recording flags

| flag | effect |
|---|---|
| `--record DIR` | render to `DIR/out.mp4` (ffmpeg pipe; deterministic, fixed timestep) |
| `--fps N` | output frame rate (default 60) |
| `--scale F` | supersampling; default 1.5 → true 1080p, `2` → 1440p |
| `--from S --to S` | record only a time range (clips) |
| `--gif` | write `DIR/out.gif` instead of mp4 |
| `--png` / `--alpha` | PNG sequence (alpha = transparent, no chrome) |
| `--crt` | bake in the CRT scanline + bloom + vignette look |
| `--still S` | export one PNG at time `S` and exit |

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
