# Publishing manic (docs + videos)

End-to-end flow for shipping the manual and its demo videos, and deploying the
built book into the playground site (`crypto-tool` → served at `/manic/docs`).

```
gen-gallery ─▶ mdbook build ─▶ render videos ─▶ upload to YouTube ─▶ embed players ─▶ deploy to /manic/docs
```

The source of truth for videos is `book/videos.txt` (`name | youtube_id | title`).

## Prerequisites

```sh
cargo install mdbook                 # the book builder
cargo build --release                # the manic renderer → target/release/manic
# ffmpeg on PATH (render → mp4);  youtube venv set up per youtube/README.md
```

## Production bundled assets

Manic keeps optional reusable files under `assets/`. A DSL reference such as
`asset:models/manic-pyramid.obj` is resolved independently of the process
working directory. Production installs the catalog at
`/usr/local/share/manic/assets`; `MANIC_ASSETS_DIR` can override that root for a
custom deployment. Ordinary `model3` filesystem paths remain supported for
backend-provisioned user files.

All packaging routes copy the complete tree, so adding a future asset requires
updating the catalog and tests—not another one-off pipeline rule:

- `docker/Dockerfile` installs `assets/` in the runtime image.
- `scripts/build-linux.sh` produces `dist/manic-assets.tar.gz` and smoke-tests a
  bundled model.
- `.github/workflows/deploy-manic.yml` installs that archive beside the binary.
- `scripts/gen-ui-index.py` copies the catalog into the playground snapshot.

For a manual Linux deployment:

```sh
scp dist/manic-linux-arm64 dist/manic-assets.tar.gz ubuntu@host:/tmp/
ssh ubuntu@host 'sudo install -m 0755 /tmp/manic-linux-arm64 /usr/local/bin/manic && sudo install -d -m 0755 /usr/local/share/manic/assets && sudo tar -xzf /tmp/manic-assets.tar.gz -C /usr/local/share/manic/assets'
```

The available asset list and contribution checklist live in `assets/README.md`
and the mdBook [Going 3D](book/src/3d.md) chapter.

## 1. (Re)generate the examples gallery — only if examples changed

Regenerates `book/src/ex-*.md`, `examples.md`, and `SUMMARY.md` from the
section/description map in the script:

```sh
python3 scripts/gen-gallery.py
```

## 2. Build the book

```sh
mdbook build book        # → book/book/   (the static site)
mdbook serve book        # optional: live preview at http://localhost:3000
```

## 3. Run the visual publishing audit

Before spending time on final renders, rebuild responsive creator examples at
the four common output shapes and audit their settled named stages:

```sh
manic check examples/reactive-multiformat.manic --canvas all
```

The command checks portrait, 4:5 feed, square, and 16:9 landscape for canvas
overflow, Creator safe-area overflow, substantial content overlap, and
unreadably small text/notation. Fix every reported format/stage/entity before
recording. Ordinary `manic check` remains the fast syntax/timeline check.

## 4. Render the demo videos

For a stage-specific social clip, inspect the semantic timeline and select by
name rather than copying seconds:

```sh
manic stages examples/reactive-world.manic
manic examples/reactive-world.manic --stage takeaway --record out-takeaway --preset reel
manic examples/reactive-world.manic --from-stage question --to-stage see-the-derivative --record out-arc
```

`--to-stage` is inclusive. The recording's `markers.json` contains the selected
source range plus clip-relative stage, section, and mark timing.

Renders each row in `book/videos.txt` to `book/videos-out/<name>.mp4`, **unbranded**
at 1080p/60 (`--preset studio --no-brand`). Incremental — re-renders only changed
sources (`FORCE=1` to redo all):

```sh
bash scripts/render-samples.sh
```

Needs a display. On a headless server, run under xvfb (see `scripts/ec2-setup.sh`):

```sh
xvfb-run -a bash scripts/render-samples.sh
```

## 5. Upload the videos to YouTube

Uploads every clip whose `book/videos.txt` id is still `PLACEHOLDER` (and that has a
render), writes the returned id back into `videos.txt`, and adds each to the **`manic`**
playlist. First run does the OAuth handshake — see `youtube/README.md` for credentials.

```sh
cd youtube && source venv/bin/activate
python manic_youtube.py --dry-run              # preview titles/descriptions/tags
python manic_youtube.py --privacy public       # upload (joins the "manic" playlist)
python manic_youtube.py --privacy public --playlists   # ...also file into per-topic playlists
cd ..
```

## 6. Embed the players + rebuild the book

Rebuilds the book and swaps each `data-video` placeholder for a real YouTube
iframe (rows still `PLACEHOLDER` keep the "coming soon" card). Idempotent — only
rewrites the built HTML:

```sh
bash scripts/embed-videos.sh     # → book/book/  with real players
```

> Shortcut for steps 4–6: `python youtube/manic_youtube.py --all --privacy public`
> runs render → upload → embed in one go.

## 6b. Publish a native Reddit video (optional)

The sibling Reddit publisher reuses the same title, rendered MP4, gallery
description, and exact source link. It defaults to `r/maniclang`, supports
community-specific title/body/flair overrides, and refuses accidental duplicate
posts through `reddit/post_state.json`:

```sh
python reddit/manic_reddit.py --only textbook-watermelon-sections --dry-run
python reddit/manic_reddit.py --only textbook-watermelon-sections
```

The first command is read-only and needs no credentials. Before the second,
create a Reddit script application and configure the `REDDIT_*` variables or an
ignored PRAW profile as described in [`reddit/README.md`](reddit/README.md).
Posting remains interactive unless `--yes` is explicitly supplied. Review each
target community's rules before reusing a post outside `r/maniclang`.

## 7. Deploy the built book into the playground → `/manic/docs`

Copy the generated site into the `crypto-tool` webapp; it's served at `/manic/docs/`:

```sh
cp -R book/book/. /Users/anish/git/crypto-tool/src/main/webapp/manic/docs/
```

Then rebuild/redeploy the `crypto-tool` WAR. The playground's **Docs ↗** button and
the welcome modal link to `/manic/docs/index.html`.

### Sibling playground assets (rebuild + copy when they change)

The book is one of several snapshots the playground (`crypto-tool`, served under
`/manic/…`) pulls from this repo. Set the destination once:

```sh
CT=/Users/anish/git/crypto-tool/src/main/webapp/manic
```

**a) Language-services WASM — how the editor gets its "brains".**
The playground editor's live syntax highlighting, inline error-checking and
autocomplete all run in the browser from `crates/manic-lang` compiled to WASM.
The UI (`manic/manic-editor.js`) does `import('<ctx>/manic/wasm/manic_lang.js')`
and instantiates `manic_lang_bg.wasm` next to it (`web.xml` serves `.wasm` as
`application/wasm`). So the deploy is: **build the WASM → copy both files into
`manic/wasm/`**. Rebuild whenever `crates/manic-lang/**` changes.

```sh
# one-time: cargo install wasm-pack
# Use the rustup toolchain — it has the wasm32 target; a Homebrew rustc does NOT.
export PATH="$(dirname "$(rustup which rustc)"):$HOME/.cargo/bin:$PATH"

wasm-pack build crates/manic-lang --target web --out-dir pkg --features wasm
#   → writes crates/manic-lang/pkg/{manic_lang.js, manic_lang_bg.wasm}

cp crates/manic-lang/pkg/manic_lang.js \
   crates/manic-lang/pkg/manic_lang_bg.wasm \
   "$CT/wasm/"
```

**b) AI system prompt** — the authoritative generation spec the AI assistant fetches.

```sh
cp SYSTEM_PROMPT.md "$CT/system-prompt.md"
```

**c) Examples + bundled assets** — the gallery `.manic` files used as playground
templates and the stable `asset:` catalog. One script reuses the same `SECTIONS`
table as `gen-gallery.py` (so the playground list can't drift from the book
gallery): it copies every `examples/*.manic` into `$CT/examples/`, writes
`$CT/examples/index.json` (grouped by category, 3d→`threed`), and mirrors
`assets/` to `$CT/assets/`. Run it after `gen-gallery.py`:

```sh
python3 scripts/gen-ui-index.py
```

Then rebuild/redeploy the `crypto-tool` WAR so the refreshed assets ship.

## One-shot recap

```sh
CT=/Users/anish/git/crypto-tool/src/main/webapp/manic

python3 scripts/gen-gallery.py                         # 1  gallery pages (if examples changed)
mdbook build book                                      # 2  build the book
manic check examples/reactive-multiformat.manic --canvas all # 3  visual publishing audit
python youtube/manic_youtube.py --all --privacy public # 4–6 render + upload + embed videos
#   (or `bash scripts/embed-videos.sh` alone to keep PLACEHOLDER "coming soon" cards)
cp -R book/book/. "$CT/docs/"                           # 7  deploy the book

# sibling playground assets (rebuild + copy when their sources change):
python3 scripts/gen-ui-index.py                         # c  examples + index.json + bundled assets
export PATH="$(dirname "$(rustup which rustc)"):$HOME/.cargo/bin:$PATH"
wasm-pack build crates/manic-lang --target web --out-dir pkg --features wasm  # a
cp crates/manic-lang/pkg/manic_lang.js crates/manic-lang/pkg/manic_lang_bg.wasm "$CT/wasm/"
cp SYSTEM_PROMPT.md "$CT/system-prompt.md"              # b  AI system prompt
# then rebuild/redeploy the crypto-tool WAR
```

**Videos are the one thing not yet done for the newer physics / textbook examples**
— those rows are `PLACEHOLDER` (coming-soon cards) in `book/videos.txt`. To ship
real players: `cargo build --release` (the release binary must include the current
kits), then steps 4–6.
