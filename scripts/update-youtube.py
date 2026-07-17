#!/usr/bin/env python3
"""
update-youtube.py — push manic's CTR titles + description to the already-published
YouTube videos, driven by `book/videos.txt` (the single source of truth).

For every row `name | id | title` whose id is real (not PLACEHOLDER), this sets
the video's:
  • title       ← the CTR title from videos.txt
  • description ← the "YOUTUBE DESCRIPTION TEMPLATE" comment block in videos.txt,
                  with {TITLE} filled in (manic blurb + 8gwifi links + hashtags)
  • tags        ← the hashtags from that template
  • categoryId  ← kept as-is, or 27 (Education) if unset

Safe by default:
  --dry-run            print exactly what WOULD change; no API calls, no creds
  --descriptions-only  update descriptions + tags but LEAVE titles untouched
                       (titles on already-ranked videos can disturb SEO)
  --only NAME[,NAME]   restrict to specific rows (by `name`, e.g. ex-loop-track)
  --limit N            cap how many videos to touch

Auth (only needed for a live run, not --dry-run): OAuth 2.0 as the channel owner
(scope youtube.force-ssl). Provide an OAuth *client secrets* JSON (Desktop app)
via --client-secrets or $YT_CLIENT_SECRETS; the user token is cached at
~/.manic-youtube-token.json. The client secret is never committed.

    python3 scripts/update-youtube.py --dry-run
    python3 scripts/update-youtube.py --descriptions-only --client-secrets client_secret.json
    python3 scripts/update-youtube.py --client-secrets client_secret.json     # title + description

Quota: videos.update ≈ 50 units each (+1 to read the current snippet); ~134
videos ≈ ~6.8k units, under the default 10k/day.
"""
import argparse
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VIDEOS_TXT = ROOT / "book" / "videos.txt"
TOKEN_PATH = Path(os.path.expanduser("~/.manic-youtube-token.json"))
SCOPES = ["https://www.googleapis.com/auth/youtube.force-ssl"]
CATEGORY_EDUCATION = "27"
PLACEHOLDER = "PLACEHOLDER"

# YouTube hard limits
TITLE_MAX = 100
DESC_MAX = 5000
TAGS_TOTAL_MAX = 500  # sum of tag characters


def parse_videos(path):
    """Return [(name, video_id, title)] for rows with a real id."""
    rows = []
    for line in path.read_text().splitlines():
        s = line.strip()
        if not s or s.startswith("#"):
            continue
        parts = line.split("|")
        if len(parts) != 3:
            continue
        name, vid, title = (p.strip() for p in parts)
        if not vid or vid == PLACEHOLDER:
            continue
        rows.append((name, vid, title))
    return rows


def parse_template(path):
    """Extract the description template (with {TITLE}) + tags from the
    'YOUTUBE DESCRIPTION TEMPLATE' comment block in videos.txt."""
    lines = path.read_text().splitlines()
    # find the block: a '# ----' line after the TEMPLATE header, until the next '# ===='
    start = None
    for i, line in enumerate(lines):
        if "YOUTUBE DESCRIPTION TEMPLATE" in line:
            for j in range(i + 1, len(lines)):
                if re.match(r"^#\s*-{5,}", lines[j]):
                    start = j + 1
                    break
            break
    if start is None:
        sys.exit("could not find the YOUTUBE DESCRIPTION TEMPLATE block in videos.txt")
    body = []
    for line in lines[start:]:
        if re.match(r"^#\s*={5,}", line):
            break
        # strip the leading '#' and up to two spaces of comment indentation
        c = re.sub(r"^#", "", line)
        c = re.sub(r"^ {1,2}", "", c)
        body.append(c)
    # trim leading/trailing blank lines
    while body and not body[0].strip():
        body.pop(0)
    while body and not body[-1].strip():
        body.pop()
    template = "\n".join(body)
    if "{TITLE}" not in template:
        sys.exit("the description template has no {TITLE} placeholder")
    tags = re.findall(r"#(\w+)", template)
    return template, tags


def build_description(template, title):
    desc = template.replace("{TITLE}", title)
    return desc[:DESC_MAX]


def clamp_tags(tags):
    out, total = [], 0
    for t in tags:
        # YouTube counts a quoted tag (with spaces) with +2; ours are single words
        cost = len(t) + 1
        if total + cost > TAGS_TOTAL_MAX:
            break
        out.append(t)
        total += cost
    return out


def load_youtube(client_secrets):
    try:
        from google.auth.transport.requests import Request
        from google.oauth2.credentials import Credentials
        from google_auth_oauthlib.flow import InstalledAppFlow
        from googleapiclient.discovery import build
    except ImportError:
        sys.exit(
            "missing deps — install:\n"
            "  pip install google-api-python-client google-auth-oauthlib google-auth-httplib2"
        )
    creds = None
    if TOKEN_PATH.exists():
        creds = Credentials.from_authorized_user_file(str(TOKEN_PATH), SCOPES)
    if not creds or not creds.valid:
        if creds and creds.expired and creds.refresh_token:
            creds.refresh(Request())
        else:
            if not client_secrets or not Path(client_secrets).exists():
                sys.exit(
                    "need an OAuth client secrets JSON (Desktop app). Pass "
                    "--client-secrets PATH or set $YT_CLIENT_SECRETS.\n"
                    "Create one at https://console.cloud.google.com/apis/credentials "
                    "(OAuth client ID → Desktop app), with the YouTube Data API v3 enabled."
                )
            flow = InstalledAppFlow.from_client_secrets_file(client_secrets, SCOPES)
            creds = flow.run_local_server(port=0)
        TOKEN_PATH.write_text(creds.to_json())
        try:
            os.chmod(TOKEN_PATH, 0o600)
        except OSError:
            pass
    return build("youtube", "v3", credentials=creds)


def main():
    ap = argparse.ArgumentParser(description="Update published manic YouTube videos from videos.txt")
    ap.add_argument("--dry-run", action="store_true", help="print planned changes; no API calls, no creds")
    ap.add_argument("--descriptions-only", action="store_true", help="update descriptions + tags but not titles")
    ap.add_argument("--only", default="", help="comma-separated row names to restrict to (e.g. ex-loop-track)")
    ap.add_argument("--limit", type=int, default=0, help="cap the number of videos touched")
    ap.add_argument("--client-secrets", default=os.environ.get("YT_CLIENT_SECRETS", ""), help="OAuth client secrets JSON")
    ap.add_argument("--yes", action="store_true", help="skip the confirmation prompt")
    args = ap.parse_args()

    rows = parse_videos(VIDEOS_TXT)
    if args.only:
        want = {n.strip() for n in args.only.split(",")}
        rows = [r for r in rows if r[0] in want]
    if args.limit > 0:
        rows = rows[: args.limit]
    if not rows:
        sys.exit("no published rows matched (all PLACEHOLDER, or --only filtered everything out)")

    template, tags = parse_template(VIDEOS_TXT)
    tags = clamp_tags(tags)
    mode = "descriptions + tags (titles untouched)" if args.descriptions_only else "title + description + tags"
    print(f"videos.txt: {len(rows)} published video(s) · mode: {mode}")
    print(f"tags: {', '.join(tags)}\n")

    if args.dry_run:
        for name, vid, title in rows:
            print(f"── {name}  [{vid}]")
            if not args.descriptions_only:
                print(f"   title: {title[:TITLE_MAX]}")
            first = build_description(template, title).splitlines()[0]
            print(f"   desc first line: {first}")
        print(f"\n(dry run — nothing sent. {len(rows)} video(s) would be updated.)")
        return

    if not args.yes:
        resp = input(f"About to update {len(rows)} LIVE YouTube video(s) — proceed? [y/N] ").strip().lower()
        if resp not in ("y", "yes"):
            sys.exit("aborted.")

    yt = load_youtube(args.client_secrets)
    from googleapiclient.errors import HttpError

    ok, failed = 0, 0
    for name, vid, title in rows:
        try:
            resp = yt.videos().list(part="snippet", id=vid).execute()
            items = resp.get("items", [])
            if not items:
                print(f"!! {name} [{vid}]: not found / not owned — skipped")
                failed += 1
                continue
            snip = items[0]["snippet"]
            if not args.descriptions_only:
                snip["title"] = title[:TITLE_MAX]
            snip["description"] = build_description(template, title)
            snip["tags"] = tags
            snip["categoryId"] = snip.get("categoryId") or CATEGORY_EDUCATION
            yt.videos().update(part="snippet", body={"id": vid, "snippet": snip}).execute()
            print(f"✓ {name} [{vid}]")
            ok += 1
        except HttpError as e:
            print(f"!! {name} [{vid}]: {e}")
            failed += 1

    print(f"\ndone — {ok} updated, {failed} failed.")


if __name__ == "__main__":
    main()
