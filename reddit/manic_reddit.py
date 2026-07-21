#!/usr/bin/env python3
"""Publish rendered Manic examples as native Reddit video posts.

Catalog mode reuses the existing publishing data:

* title and rendered-video name: ``book/videos.txt``
* video: ``book/videos-out/<name>.mp4``
* human description and gallery link: ``scripts/gen-gallery.py``

Examples::

    python reddit/manic_reddit.py --only textbook-watermelon-sections --dry-run
    python reddit/manic_reddit.py --only textbook-watermelon-sections
    python reddit/manic_reddit.py --video clip.mp4 --title "My title" \
        --description-file post.md

Posting is deliberately interactive unless ``--yes`` is supplied. A per-subreddit
state file prevents accidental duplicates; use ``--force`` to post one again.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


HERE = Path(__file__).resolve().parent
REPO = HERE.parent
VIDEOS_TXT = REPO / "book" / "videos.txt"
VIDEOS_OUT = REPO / "book" / "videos-out"
GALLERY_SCRIPT = REPO / "scripts" / "gen-gallery.py"
STATE_FILE = HERE / "post_state.json"
SITE = "https://8gwifi.org/manic"


@dataclass
class Target:
    name: str
    title: str
    description: str
    video: Path
    thumbnail: Path | None
    section: str


def parse_videos(path: Path) -> list[dict[str, str]]:
    """Read ``name|youtube_id|title`` rows while retaining their section."""
    section = "guide samples"
    rows: list[dict[str, str]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        match = re.match(r"#\s*-+\s*(.*?)\s*-+\s*$", stripped)
        if match:
            section = (
                re.sub(r"\(.*?\)", "", match.group(1))
                .replace("examples:", "")
                .strip()
            )
            continue
        if not stripped or stripped.startswith("#"):
            continue
        parts = [part.strip() for part in line.split("|")]
        if len(parts) >= 3:
            rows.append(
                {
                    "name": parts[0],
                    "youtube_id": parts[1],
                    "title": parts[2],
                    "section": section,
                }
            )
    return rows


def load_gallery() -> dict[str, dict[str, str]]:
    """Load gallery descriptions without running its generation entry point."""
    spec = importlib.util.spec_from_file_location("manic_gallery", GALLERY_SCRIPT)
    if spec is None or spec.loader is None:
        return {}
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    gallery: dict[str, dict[str, str]] = {}
    for section in module.SECTIONS:
        for item in section["items"]:
            example, description = item[0], item[1]
            gallery[f"ex-{example}"] = {
                "description": description,
                "slug": section["slug"],
                "example": example,
            }
    return gallery


def clean_title(title: str) -> str:
    return re.sub(r"\s*(—|-)\s*manic\s*$|\s*\(manic\)\s*$", "", title).strip()


def compact(text: str) -> str:
    """Keep Markdown but turn authored gallery line wrapping into paragraphs."""
    paragraphs = []
    for paragraph in re.split(r"\n\s*\n", text.strip()):
        paragraphs.append(re.sub(r"\s*\n\s*", " ", paragraph).strip())
    return "\n\n".join(part for part in paragraphs if part)


def make_description(row: dict[str, str], gallery: dict[str, dict[str, str]]) -> str:
    """Generate a concise, Reddit-native body with the exact copyable source."""
    meta = gallery.get(row["name"])
    if meta:
        opening = compact(meta["description"])
        source = f"{SITE}/docs/ex-{meta['slug']}.html#{meta['example']}"
    else:
        opening = clean_title(row["title"])
        source = f"{SITE}/docs"
    return (
        f"{opening}\n\n"
        "This animation is written as a plain-text `.manic` scene—no video "
        "timeline or hand-positioned keyframes.\n\n"
        f"**Copy the complete source:** {source}\n\n"
        f"**Try Manic in the browser:** {SITE}"
    )


def wanted(name: str, only: str | None) -> bool:
    if not only:
        return True
    return name == only or name == f"ex-{only}" or name.removeprefix("ex-") == only


def catalog_targets(
    rows: list[dict[str, str]],
    gallery: dict[str, dict[str, str]],
    only: str | None,
) -> list[Target]:
    targets = []
    for row in rows:
        if not wanted(row["name"], only):
            continue
        video = VIDEOS_OUT / f"{row['name']}.mp4"
        # With --only, retain the target so the error names the missing render.
        if not only and not video.exists():
            continue
        thumbnail = VIDEOS_OUT / f"{row['name']}.png"
        targets.append(
            Target(
                name=row["name"],
                title=row["title"],
                description=make_description(row, gallery),
                video=video,
                thumbnail=thumbnail if thumbnail.exists() else None,
                section=row["section"],
            )
        )
    return targets


def load_state(path: Path) -> dict[str, Any]:
    if not path.exists():
        return {}
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else {}
    except (OSError, json.JSONDecodeError):
        return {}


def save_state(path: Path, state: dict[str, Any]) -> None:
    path.write_text(json.dumps(state, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def fingerprint(target: Target) -> str:
    digest = hashlib.sha256()
    source = (
        REPO / "examples" / f"{target.name.removeprefix('ex-')}.manic"
        if target.name.startswith("ex-")
        else REPO / "book" / "samples" / f"{target.name}.manic"
    )
    payload = source if source.exists() else target.video
    if payload.exists():
        with payload.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
    digest.update(target.title.encode("utf-8"))
    digest.update(target.description.encode("utf-8"))
    return digest.hexdigest()


def reddit_imports():
    try:
        import praw
        from praw.models import PostMedia

        return praw, PostMedia
    except ImportError:
        sys.exit("Missing dependency. Run: pip install -r reddit/requirements.txt")


def reddit_client(profile: str | None):
    praw, _ = reddit_imports()
    if profile:
        # PRAW discovers praw.ini relative to the current process directory.
        # Make `reddit/praw.ini` work even when this script is launched at the
        # repository root, then restore the caller's directory immediately.
        if (HERE / "praw.ini").exists():
            previous = Path.cwd()
            try:
                os.chdir(HERE)
                return praw.Reddit(profile)
            finally:
                os.chdir(previous)
        return praw.Reddit(profile)

    keys = {
        "client_id": "REDDIT_CLIENT_ID",
        "client_secret": "REDDIT_CLIENT_SECRET",
        "username": "REDDIT_USERNAME",
        "password": "REDDIT_PASSWORD",
    }
    values = {argument: os.environ.get(variable) for argument, variable in keys.items()}
    missing = [variable for argument, variable in keys.items() if not values[argument]]
    if missing:
        sys.exit(
            "Missing Reddit credentials: "
            + ", ".join(missing)
            + ". Set them or pass --profile for a praw.ini profile."
        )
    user_agent = os.environ.get(
        "REDDIT_USER_AGENT",
        f"manic-reddit-publisher/1.0 by u/{values['username']}",
    )
    return praw.Reddit(user_agent=user_agent, **values)


def parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(
        description="Publish rendered Manic examples as native Reddit videos."
    )
    ap.add_argument("--subreddit", default="maniclang", help="target without r/ (default: maniclang)")
    ap.add_argument("--only", help="catalog name, with or without the ex- prefix")
    ap.add_argument("--video", type=Path, help="one-off MP4 instead of a catalog row")
    ap.add_argument("--title", help="title override; required with --video")
    body = ap.add_mutually_exclusive_group()
    body.add_argument("--description", help="Markdown body override")
    body.add_argument("--description-file", type=Path, help="read Markdown body from this file")
    ap.add_argument("--thumbnail", type=Path, help="PNG/JPG thumbnail override")
    ap.add_argument("--videogif", action="store_true", help="post as a silent looping videogif")
    ap.add_argument("--flair-id")
    ap.add_argument("--flair-text")
    ap.add_argument("--nsfw", action="store_true")
    ap.add_argument("--spoiler", action="store_true")
    ap.add_argument("--no-replies", action="store_true", help="disable inbox replies")
    ap.add_argument("--timeout", type=int, default=60, help="media completion timeout (default: 60s)")
    ap.add_argument("--profile", help="PRAW profile in praw.ini instead of REDDIT_* variables")
    ap.add_argument("--state", type=Path, default=STATE_FILE, help="duplicate-prevention state file")
    ap.add_argument("--force", action="store_true", help="allow a duplicate of an already recorded post")
    ap.add_argument("--yes", action="store_true", help="skip the final posting confirmation")
    ap.add_argument("--dry-run", action="store_true", help="print the exact post; contact nothing")
    return ap


def main() -> None:
    ap = parser()
    args = ap.parse_args()
    if args.flair_text and not args.flair_id:
        ap.error("--flair-text requires --flair-id")
    if args.timeout <= 0:
        ap.error("--timeout must be positive")

    description_override = args.description
    if args.description_file:
        if not args.description_file.exists():
            ap.error(f"description file does not exist: {args.description_file}")
        description_override = args.description_file.read_text(encoding="utf-8").strip()

    if args.video:
        if args.only:
            ap.error("use either --video or --only, not both")
        if not args.title:
            ap.error("--video requires --title")
        thumb = args.thumbnail
        targets = [
            Target(
                name=f"manual-{args.video.stem}",
                title=args.title,
                description=description_override
                or f"Made with Manic.\n\n**Try it:** {SITE}",
                video=args.video.expanduser().resolve(),
                thumbnail=thumb.expanduser().resolve() if thumb else None,
                section="manual",
            )
        ]
    else:
        targets = catalog_targets(parse_videos(VIDEOS_TXT), load_gallery(), args.only)
        if not targets:
            hint = f" matching --only {args.only!r}" if args.only else " with rendered MP4s"
            sys.exit(f"No catalog rows{hint}.")
        if any((args.title, description_override, args.thumbnail)) and len(targets) != 1:
            ap.error("title/description/thumbnail overrides require exactly one target (use --only)")
        if args.title:
            targets[0].title = args.title
        if description_override is not None:
            targets[0].description = description_override
        if args.thumbnail:
            targets[0].thumbnail = args.thumbnail.expanduser().resolve()

    subreddit = args.subreddit.removeprefix("r/").strip("/")
    if not subreddit:
        ap.error("--subreddit cannot be empty")

    state = load_state(args.state)
    pending: list[tuple[Target, str, str]] = []
    for target in targets:
        if len(target.title) > 300:
            ap.error(f"Reddit titles are limited to 300 characters: {target.name}")
        if not target.video.exists():
            ap.error(f"video does not exist: {target.video}")
        if target.thumbnail and not target.thumbnail.exists():
            ap.error(f"thumbnail does not exist: {target.thumbnail}")
        key = f"{subreddit.lower()}:{target.name}"
        current = fingerprint(target)
        previous = state.get(key)
        if previous and not args.force:
            changed = isinstance(previous, dict) and previous.get("fingerprint") != current
            suffix = " (content changed; use --force to post a new submission)" if changed else ""
            print(f"skip {target.name}: already posted to r/{subreddit}{suffix}")
            continue
        pending.append((target, key, current))

    if not pending:
        print("up to date — no Reddit posts to create.")
        return

    for target, _, _ in pending:
        print(f"── r/{subreddit} · {target.name} [{target.section}]")
        print(f"   video: {target.video}")
        if target.thumbnail:
            print(f"   thumbnail: {target.thumbnail}")
        print(f"   title: {target.title}")
        print("   description:")
        print("   " + target.description.replace("\n", "\n   "))
        print()

    if args.dry_run:
        print(f"dry run — {len(pending)} post(s) previewed; Reddit was not contacted.")
        return
    if not args.yes:
        answer = input(f"Create {len(pending)} native video post(s) in r/{subreddit}? [y/N] ")
        if answer.strip().lower() not in {"y", "yes"}:
            sys.exit("aborted.")

    reddit = reddit_client(args.profile)
    account = reddit.user.me()
    if account is None:
        sys.exit("Reddit authentication is read-only; a posting account is required.")
    print(f"authenticated as u/{account}")
    _, PostMedia = reddit_imports()
    community = reddit.subreddit(subreddit)

    posted = failed = 0
    for target, key, current in pending:
        video: Any = PostMedia(str(target.video))
        if args.videogif or target.thumbnail:
            video = {"media": video, "gif": args.videogif}
            if target.thumbnail:
                video["thumbnail"] = PostMedia(str(target.thumbnail))
        try:
            submission = community.submit(
                target.title,
                video=video,
                selftext=target.description,
                flair_id=args.flair_id,
                flair_text=args.flair_text,
                nsfw=args.nsfw,
                spoiler=args.spoiler,
                send_replies=not args.no_replies,
                timeout=args.timeout,
            )
            if submission is None:
                raise RuntimeError("Reddit completed the upload but returned no submission")
            url = f"https://www.reddit.com{submission.permalink}"
            state[key] = {
                "fingerprint": current,
                "post_id": submission.id,
                "title": target.title,
                "url": url,
            }
            save_state(args.state, state)
            print(f"✓ {target.name}: {url}")
            posted += 1
        except Exception as error:  # PRAW exposes several API/media exception types.
            print(f"✗ {target.name}: {error}", file=sys.stderr)
            failed += 1

    print(f"done — {posted} posted, {failed} failed.")
    if failed:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
