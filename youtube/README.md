# manic → YouTube (automated)

`manic_youtube.py` runs the full publish pipeline:

```
render (unbranded mp4s)  ->  upload (title/desc/tags/playlist)  ->  write video
ids back into book/videos.txt  ->  embed real players into the mdBook
```

The source of truth is `../book/videos.txt` (`name|id|title`). Only rows whose id
is still `PLACEHOLDER` **and** that have a render at `../book/videos-out/<name>.mp4`
are uploaded; each upload rewrites that row's id so `scripts/embed-videos.sh` can
embed it.

## One-time setup

```sh
cd youtube
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
```

**Credentials** (Google Cloud → APIs & Services → OAuth, *Desktop app*):

1. Enable the **YouTube Data API v3**.
2. Download the OAuth client JSON and drop it in this folder as
   `client_secret*.json` (the script auto-picks the newest match).
3. First run opens a browser to authorize; the token is saved to
   `youtube_credentials.json` here and reused. (Both are git-ignored.)

If you already have a `youtube_credentials.json` from another project, copy it
in — same OAuth scopes (`youtube.upload`, `youtube`).

## Usage

```sh
python manic_youtube.py --dry-run          # preview titles/descriptions/tags, upload nothing
python manic_youtube.py                     # upload pending clips (UNLISTED by default)
python manic_youtube.py --privacy public    # ...as public
python manic_youtube.py --only bubble_sort  # just one
python manic_youtube.py --playlists         # also add each to a per-topic playlist
python manic_youtube.py --all --privacy public   # render + upload + embed, end to end
```

Titles come from `book/videos.txt`; descriptions + tags are generated per topic
(with the manic link + hashtags). Optional thumbnail: put
`book/videos-out/<name>.png` next to the mp4 and it's set on upload.

## Never uploads the same thing twice

The script fingerprints each example's `.manic` source (sha256) at upload time in
`youtube/upload_state.json`. On the next run it only (re)uploads:

- **new** examples (no id yet), and
- examples whose **source changed** since their last upload.

Anything already uploaded and unchanged is skipped. (YouTube can't replace a
video's file via the API, so a changed example uploads a *new* video with a new
id — the script warns you; delete the old one if you don't want a duplicate.)

- `--force` re-uploads regardless of the fingerprint.
- **Commit `upload_state.json`** so the "already uploaded" record persists.
- `scripts/render-samples.sh` is likewise incremental (skips mp4s newer than
  their source; `FORCE=1 ./scripts/render-samples.sh` to re-render all).
