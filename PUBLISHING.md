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

## 3. Render the demo videos

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

## 4. Upload the videos to YouTube

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

## 5. Embed the players + rebuild the book

Rebuilds the book and swaps each `data-video` placeholder for a real YouTube
iframe (rows still `PLACEHOLDER` keep the "coming soon" card). Idempotent — only
rewrites the built HTML:

```sh
bash scripts/embed-videos.sh     # → book/book/  with real players
```

> Shortcut for steps 3–5: `python youtube/manic_youtube.py --all --privacy public`
> runs render → upload → embed in one go.

## 6. Deploy the built book into the playground → `/manic/docs`

Copy the generated site into the `crypto-tool` webapp; it's served at `/manic/docs/`:

```sh
cp -R book/book/. /Users/anish/git/crypto-tool/src/main/webapp/manic/docs/
```

Then rebuild/redeploy the `crypto-tool` WAR. The playground's **Docs ↗** button and
the welcome modal link to `/manic/docs/index.html`.

### Sibling playground assets (refresh when they change)

The book is one of several snapshots the playground pulls from this repo. Refresh
the others the same way when they change:

```sh
CT=/Users/anish/git/crypto-tool/src/main/webapp/manic

# language-services WASM (rebuild first: see web/README.md)
cp crates/manic-lang/pkg/manic_lang.js crates/manic-lang/pkg/manic_lang_bg.wasm "$CT/wasm/"

# AI system prompt (authoritative generation spec)
cp SYSTEM_PROMPT.md "$CT/system-prompt.md"

# examples used as playground templates (copies files + writes index.json)
# generator lives with the crypto-tool tooling; see manic-playground-ui notes
```

## One-shot recap

```sh
python3 scripts/gen-gallery.py                         # 1  gallery pages (if examples changed)
bash scripts/render-samples.sh                         # 3  render mp4s
python youtube/manic_youtube.py --all --privacy public # 3–5 render + upload + embed
cp -R book/book/. /Users/anish/git/crypto-tool/src/main/webapp/manic/docs/   # 6 deploy
```
