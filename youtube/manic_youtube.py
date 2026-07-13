#!/usr/bin/env python3
"""
manic → YouTube: the whole publish pipeline, automated.

    render (unbranded mp4s)  ->  upload (title/desc/tags/playlist)  ->  write the
    returned video ids back into book/videos.txt  ->  embed into the mdBook.

Source of truth is `book/videos.txt` (`name|id|title`). This script only touches
rows whose id is still `PLACEHOLDER` and that have a rendered mp4 in
`book/videos-out/<name>.mp4`; it uploads them, then rewrites that row's id, so
`scripts/embed-videos.sh` can drop real players into the book.

    cd youtube
    python3 -m venv venv && source venv/bin/activate
    pip install -r requirements.txt
    # put your OAuth `client_secret*.json` here (from Google Cloud console)

    python manic_youtube.py --dry-run        # preview titles/descriptions/tags
    python manic_youtube.py                   # upload pending (unlisted); joins the "manic" playlist
    python manic_youtube.py --all --privacy public   # render + upload + embed
    python manic_youtube.py --only bubble_sort
    python manic_youtube.py --playlists       # ALSO file each into manic — <Section>
    python manic_youtube.py --no-playlist     # skip the master playlist

Every upload is added to a single master playlist ("manic" by default; change with
--playlist NAME, disable with --no-playlist). --playlists additionally files each
video under a per-topic playlist. Playlists are de-duplicated by title and reused.

The upload/playlist/thumbnail helpers are adapted from the dsa-youtube-shorts
`youtube_upload.py`.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import List, Optional

# ---- paths ---------------------------------------------------------------
HERE = Path(__file__).resolve().parent
REPO = HERE.parent
VIDEOS_TXT = REPO / "book" / "videos.txt"
VIDEOS_OUT = REPO / "book" / "videos-out"
STATE_FILE = HERE / "upload_state.json"   # name -> source fingerprint at last upload
SITE = "https://8gwifi.org/manic"


# ---- change detection ----------------------------------------------------
def source_path(name: str) -> Path:
    """The .manic file behind a video (`ex-foo` -> examples/foo, else a sample)."""
    if name.startswith("ex-"):
        return REPO / "examples" / f"{name[3:]}.manic"
    return REPO / "book" / "samples" / f"{name}.manic"


def source_hash(name: str) -> str:
    p = source_path(name)
    return hashlib.sha256(p.read_bytes()).hexdigest() if p.exists() else ""


def load_state() -> dict:
    if STATE_FILE.exists():
        try:
            return json.loads(STATE_FILE.read_text())
        except Exception:
            return {}
    return {}


def save_state(state: dict) -> None:
    STATE_FILE.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n")

# ---- metadata templates --------------------------------------------------
BOILERPLATE = (
    "Made with manic — write animations as plain text, render to video.\n"
    f"▶ {SITE}"
)

BASE_TAGS = [
    "manic", "algorithm visualization", "computer science", "coding",
    "programming", "learn to code", "dsa", "data structures", "education",
    "explainer", "manim alternative", "animation",
]

# section keyword (from the `# ---- ... ----` comments in videos.txt) -> extras
TOPIC = {
    "guide samples":        (["creative coding", "tutorial", "text to animation"], "#creativecoding #coding #manic"),
    "algorithms":           (["sorting algorithm", "algorithms explained", "dsa"], "#algorithms #dsa #coding"),
    "graphs":               (["graph algorithms", "graph theory", "bfs", "dfs"], "#graphs #algorithms #coding"),
    "calculus":             (["calculus", "integral", "riemann sum", "math visualization"], "#calculus #math #manim"),
    "linear algebra":       (["linear algebra", "matrix", "matrices"], "#linearalgebra #math #coding"),
    "vectors":              (["vector field", "coordinates", "math visualization"], "#math #vectors #coding"),
    "geometry":             (["geometry", "euclidean geometry", "olympiad geometry"], "#geometry #math #manim"),
    "transforms":           (["linear transformation", "matrix transformation"], "#linearalgebra #math #coding"),
    "text":                 (["motion graphics", "typography", "creative coding"], "#motiongraphics #creativecoding #manic"),
    "generative":           (["creative coding", "generative art", "recursion", "fractal"], "#creativecoding #generative #manic"),
    "boolean":              (["boolean operations", "geometry", "creative coding"], "#geometry #creativecoding #manic"),
}
DEFAULT_TOPIC = (["creative coding", "coding"], "#coding #manim #manic")


def topic_for(section: str) -> tuple[list[str], str]:
    s = section.lower()
    for key, val in TOPIC.items():
        if key in s:
            return val
    return DEFAULT_TOPIC


def clean_title(title: str) -> str:
    """Drop the trailing ` — manic` / `(manic)` branding for the description hook."""
    return re.sub(r"\s*(—|-)\s*manic\s*$|\s*\(manic\)\s*$", "", title).strip()


def make_description(title: str, section: str) -> str:
    _, hashtags = topic_for(section)
    hook = clean_title(title)
    if not hook.endswith((".", "!", "?")):
        hook += "."
    return f"{hook}\n\n{BOILERPLATE}\n\n{hashtags}"


def make_tags(title: str, section: str) -> List[str]:
    extra, _ = topic_for(section)
    # a keyword from the title (e.g. "bubble sort") helps search
    kw = clean_title(title).lower()
    kw = re.sub(r"[^a-z0-9 ]", " ", kw)
    kw = re.sub(r"\s+", " ", kw).strip()[:40].strip()
    tags = [kw] + extra + BASE_TAGS
    seen, out = set(), []
    for t in tags:
        if t and t not in seen:
            seen.add(t)
            out.append(t)
    return out[:60]


# ---- videos.txt parsing / write-back ------------------------------------
def parse_videos(path: Path):
    section = "guide samples"
    rows = []
    for line in path.read_text().splitlines():
        s = line.strip()
        m = re.match(r"#\s*-+\s*(.*?)\s*-+\s*$", s)
        if m:
            section = re.sub(r"\(.*?\)", "", m.group(1)).replace("examples:", "").strip()
            continue
        if not s or s.startswith("#"):
            continue
        parts = [p.strip() for p in line.split("|")]
        if len(parts) >= 3:
            rows.append({"name": parts[0], "id": parts[1], "title": parts[2], "section": section})
    return rows


def write_back_id(path: Path, name: str, vid: str) -> None:
    lines = path.read_text().splitlines(keepends=True)
    for i, line in enumerate(lines):
        if line.lstrip().startswith("#"):
            continue
        parts = line.split("|")
        if len(parts) >= 3 and parts[0].strip() == name:
            parts[1] = f"{vid}"
            # preserve leading/trailing formatting of the title field
            lines[i] = f"{parts[0].rstrip()}|{vid}|{parts[2].lstrip()}"
            if not lines[i].endswith("\n"):
                lines[i] += "\n"
            break
    path.write_text("".join(lines))


# ---- YouTube (adapted from dsa-youtube-shorts/youtube_upload.py) ----------
SCOPES = [
    "https://www.googleapis.com/auth/youtube.upload",
    "https://www.googleapis.com/auth/youtube",
]


def _yt_imports():
    try:
        from googleapiclient.discovery import build
        from googleapiclient.errors import HttpError
        from googleapiclient.http import MediaFileUpload
        from google.oauth2.credentials import Credentials
        from google_auth_oauthlib.flow import InstalledAppFlow
        from google.auth.transport.requests import Request
        return build, HttpError, MediaFileUpload, Credentials, InstalledAppFlow, Request
    except ImportError:
        sys.exit("Missing deps. Run:  pip install -r requirements.txt")


def load_credentials(client_secret: Path, creds_path: Path):
    _, _, _, Credentials, InstalledAppFlow, Request = _yt_imports()
    creds = None
    if creds_path.exists():
        try:
            creds = Credentials.from_authorized_user_file(str(creds_path), SCOPES)
        except Exception:
            creds = None
    if not creds or not creds.valid:
        if creds and creds.expired and creds.refresh_token:
            try:
                creds.refresh(Request())
            except Exception:
                creds = None
        if not creds:
            flow = InstalledAppFlow.from_client_secrets_file(str(client_secret), SCOPES)
            creds = flow.run_local_server(port=0)
        creds_path.write_text(creds.to_json())
    return creds


def get_or_create_playlist(youtube, title: str, HttpError) -> str:
    try:
        resp = youtube.playlists().list(part="snippet", mine=True, maxResults=50).execute()
        for item in resp.get("items", []):
            if item["snippet"]["title"].lower() == title.lower():
                return item["id"]
    except HttpError as e:
        print(f"  ! playlist lookup failed: {e}")
    try:
        body = {"snippet": {"title": title, "description": f"manic — {title}"},
                "status": {"privacyStatus": "public"}}
        resp = youtube.playlists().insert(part="snippet,status", body=body).execute()
        print(f"  + created playlist: {title}")
        return resp.get("id", "")
    except HttpError as e:
        print(f"  ! playlist create failed: {e}")
        return ""


def add_to_playlist(youtube, HttpError, title: str, vid: str) -> None:
    pl = get_or_create_playlist(youtube, title, HttpError)
    if not pl:
        return
    try:
        youtube.playlistItems().insert(part="snippet", body={"snippet": {
            "playlistId": pl, "resourceId": {"kind": "youtube#video", "videoId": vid}}}).execute()
        print(f"   ✓ added to playlist: {title}")
    except HttpError as e:
        print(f"   ! playlist add failed ({title}): {e}")


def upload(youtube, MediaFileUpload, HttpError, mp4: Path, title: str,
           description: str, tags: List[str], privacy: str) -> str:
    body = {
        "snippet": {"title": title, "description": description, "tags": tags[:500],
                    "categoryId": "28"},  # Science & Technology
        "status": {"privacyStatus": privacy, "selfDeclaredMadeForKids": False},
    }
    media = MediaFileUpload(str(mp4), chunksize=-1, resumable=True, mimetype="video/mp4")
    req = youtube.videos().insert(part=",".join(body.keys()), body=body, media_body=media)
    resp = req.execute()
    vid = resp.get("id")
    if not vid:
        raise RuntimeError("upload succeeded but no video id returned")
    return vid


# ---- driver --------------------------------------------------------------
def run(cmd: list[str]):
    print(f"$ {' '.join(cmd)}")
    subprocess.run(cmd, cwd=REPO, check=True)


def main():
    ap = argparse.ArgumentParser(description="Automate manic → YouTube publishing.")
    ap.add_argument("--privacy", default="unlisted", choices=["private", "unlisted", "public"])
    ap.add_argument("--only", help="upload just this video name")
    ap.add_argument("--render", action="store_true", help="run scripts/render-samples.sh first")
    ap.add_argument("--embed", action="store_true", help="run scripts/embed-videos.sh after")
    ap.add_argument("--all", action="store_true", help="render + upload + embed")
    ap.add_argument("--playlist", default="manic",
                    help="master playlist every uploaded video joins (default: manic)")
    ap.add_argument("--no-playlist", action="store_true",
                    help="don't add uploads to the master playlist")
    ap.add_argument("--playlists", action="store_true",
                    help="ALSO add each video to a per-topic playlist (manic — <Section>)")
    ap.add_argument("--force", action="store_true", help="re-upload even if the source is unchanged")
    ap.add_argument("--dry-run", action="store_true", help="print metadata, upload nothing")
    ap.add_argument("--client-secret", type=Path, help="OAuth client_secret*.json (default: newest in ./)")
    ap.add_argument("--creds", type=Path, default=HERE / "youtube_credentials.json",
                    help="saved OAuth token path")
    args = ap.parse_args()

    if args.render or args.all:
        run(["bash", "scripts/render-samples.sh"])

    def wanted(name: str) -> bool:
        if not args.only:
            return True
        o = args.only
        return name == o or name == f"ex-{o}" or name.removeprefix("ex-") == o

    state = load_state()
    rows = [r for r in parse_videos(VIDEOS_TXT) if wanted(r["name"])]

    # decide what to (re)upload. Skip already-uploaded + unchanged; upload the
    # new ones and the ones whose .manic source changed since last upload.
    todo = []  # (row, reason, cur_hash)
    for r in rows:
        name, cur = r["name"], source_hash(r["name"])
        uploaded = r["id"] not in ("", "PLACEHOLDER")
        rec = state.get(name)
        if not uploaded:
            todo.append((r, "new", cur))
        elif args.force:
            todo.append((r, "forced", cur))
        elif rec is None:
            # uploaded before we tracked it — trust it, just record the fingerprint
            state[name] = cur
        elif rec != cur:
            todo.append((r, "source changed", cur))
        # else: uploaded + unchanged -> skip silently
    save_state(state)

    if not todo:
        print("up to date — nothing to upload (use --force to re-upload).")
    else:
        print(f"{len(todo)} video(s) to upload:\n")

    yt = None
    for r, reason, cur in todo:
        mp4 = VIDEOS_OUT / f"{r['name']}.mp4"
        title = r["title"]
        desc = make_description(title, r["section"])
        tags = make_tags(title, r["section"])

        print(f"── {r['name']}  [{r['section']}]  ({reason})")
        if reason == "source changed":
            print(f"   ⚠  re-upload creates a NEW video id; the old one stays on YouTube "
                  f"(delete it manually if you don't want a duplicate).")
        print(f"   title: {title}")
        print(f"   tags : {', '.join(tags[:8])} …(+{max(0,len(tags)-8)})")
        if args.dry_run:
            print("   description:")
            print("   " + desc.replace("\n", "\n   "))
            print()
            continue
        if not mp4.exists():
            print(f"   ! no render at {mp4} — run --render first. skipping.\n")
            continue

        if yt is None:
            build, HttpError, MediaFileUpload, *_ = _yt_imports()
            cs = args.client_secret or next(iter(sorted(HERE.glob("client_secret*.json"))), None)
            if not cs or not cs.exists():
                sys.exit("No OAuth client_secret*.json found in ./youtube/ (see README).")
            creds = load_credentials(cs, args.creds)
            yt = build("youtube", "v3", credentials=creds)
            _yt = (build, HttpError, MediaFileUpload)

        build, HttpError, MediaFileUpload = _yt
        try:
            vid = upload(yt, MediaFileUpload, HttpError, mp4, title, desc, tags, args.privacy)
            print(f"   ✅ https://youtube.com/watch?v={vid}")
            write_back_id(VIDEOS_TXT, r["name"], vid)
            state[r["name"]] = cur          # remember the source fingerprint
            save_state(state)
            # optional thumbnail: book/videos-out/<name>.png
            png = VIDEOS_OUT / f"{r['name']}.png"
            if png.exists():
                try:
                    yt.thumbnails().set(videoId=vid,
                        media_body=MediaFileUpload(str(png), mimetype="image/png")).execute()
                    print("   ✓ thumbnail set")
                except HttpError as e:
                    print(f"   ! thumbnail failed (needs verified channel): {e}")
            # every upload joins the master "manic" playlist by default;
            # --playlists ALSO files it under a per-topic playlist.
            targets = []
            if not args.no_playlist and args.playlist:
                targets.append(args.playlist)
            if args.playlists:
                targets.append(f"manic — {r['section'].title()}")
            for t in dict.fromkeys(targets):   # dedup, preserve order
                add_to_playlist(yt, HttpError, t, vid)
            print()
        except Exception as e:
            print(f"   ✗ upload failed: {e}\n")

    if args.embed or args.all:
        run(["bash", "scripts/embed-videos.sh"])


if __name__ == "__main__":
    main()
