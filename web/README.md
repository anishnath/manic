# manic — language-services prototype

A throwaway HTML/JS harness over the `manic-lang` **WASM** to demonstrate the
editor pipeline: **syntax highlighting**, **live error-checking with fixes**, and
**context-aware autocomplete**. This is *not* the production editor — that gets
its own design (CodeMirror/Monaco). It exists to prove the WASM services work in
a browser and to prototype against.

## Build the WASM

The front-end (`crates/manic-lang`) compiles to WASM with no engine/graphics
deps. Build it with the **rustup** toolchain (which has the `wasm32` target — a
Homebrew `rustc` won't):

```sh
# put the rustup toolchain first (it has wasm32-unknown-unknown)
export PATH="$(dirname $(rustup which rustc)):$HOME/.cargo/bin:$PATH"

wasm-pack build crates/manic-lang --target web --out-dir pkg --features wasm
```

This writes `crates/manic-lang/pkg/` (`manic_lang.js` + `manic_lang_bg.wasm`),
which `web/index.html` imports.

## Run

ES modules + WASM need to be served over HTTP (not `file://`):

```sh
python3 -m http.server        # from the repo root
open http://localhost:8000/web/
```

## What it exercises

`manic_lang.js` exports three functions (thin JSON wrappers over
`manic-lang/src/services.rs`, all unit-tested in `cargo test`):

| function | returns | used for |
|---|---|---|
| `tokenize(src)` | `[{start,len,kind}]` | highlighting (builtin / variable / color / keyword / comment …) |
| `check(src)` | `[{start,len,severity,message,fix?}]` | diagnostics + one-click fixes |
| `complete(src, offset)` | `[{label,kind,insert,detail,doc}]` | autocomplete (builtins, palette, file ids, presets) |

Everything is driven by the **catalog** (generated from the engine registry), so
the editor never drifts from what the renderer accepts.
