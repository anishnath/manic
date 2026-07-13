#!/usr/bin/env bash
# Build the mdBook and embed real YouTube players from book/videos.txt.
#
#   ./scripts/embed-videos.sh          # -> book/book/  (open index.html)
#
# For every row whose id is set (not PLACEHOLDER), the matching
# <div class="manic-video" data-video="NAME"></div> placeholder is replaced with
# a YouTube <iframe>. Rows still PLACEHOLDER keep the "coming soon" card. The
# book src is never modified — this only rewrites the built HTML — so it's
# idempotent (always builds fresh, then embeds).
set -euo pipefail
cd "$(dirname "$0")/.."

MAP=book/videos.txt
OUT=book/book

command -v mdbook >/dev/null || { echo "mdbook not found (cargo install mdbook)"; exit 1; }
mdbook build book

count=0
while IFS='|' read -r name id title; do
  name="${name//[[:space:]]/}"
  [[ -z "$name" || "$name" == \#* ]] && continue
  id="${id//[[:space:]]/}"
  [[ -z "$id" || "$id" == "PLACEHOLDER" ]] && continue
  title="${title#"${title%%[![:space:]]*}"}"   # trim leading space

  export V_NAME="$name" V_ID="$id" V_TITLE="$title"
  find "$OUT" -name '*.html' -print0 | xargs -0 perl -0pi -e '
    my $n = quotemeta($ENV{V_NAME});
    my $frame = "<div class=\"manic-video\"><iframe src=\"https://www.youtube.com/embed/"
      . $ENV{V_ID} . "\" title=\"" . $ENV{V_TITLE}
      . "\" allowfullscreen loading=\"lazy\"></iframe></div>";
    s{<div class="manic-video" data-video="$n"></div>}{$frame}g;
  '
  count=$((count + 1))
done < "$MAP"

echo "✓ built $OUT/ ($count video(s) embedded; rest are placeholders)"
echo "  open $OUT/index.html"
