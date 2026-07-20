#!/usr/bin/env bash
# Render the mdBook demo videos — UNBRANDED (no intro/watermark) at 1080p — for
# YouTube. Outputs book/videos-out/<name>.mp4. Names match book/videos.txt so the
# ids drop straight into the book.
#
#   ./scripts/render-samples.sh
#   # then upload each mp4, paste the id into book/videos.txt, run embed-videos.sh
#
# Uses `--preset studio --no-brand` (1080p, 60fps, unbranded). Needs a display
# (or run under xvfb on a server — see scripts/ec2-setup.sh).
set -euo pipefail
cd "$(dirname "$0")/.."
export MANIC_ASSETS_DIR="${MANIC_ASSETS_DIR:-$PWD/assets}"

MANIC="${MANIC:-target/release/manic}"
[ -x "$MANIC" ] || MANIC="$(command -v manic || echo target/debug/manic)"
OUT=book/videos-out
mkdir -p "$OUT"

render() { # <file> <out-name>
  # NB: separate `local` lines — a single `local a=$1 b=$OUT/$a` expands $a
  # before it's assigned under `set -u` (macOS bash 3.2) → "unbound variable".
  local file="$1"
  local name="$2"
  local out="$OUT/$name.mp4"
  local dir="$OUT/$name"
  # incremental: skip if the mp4 is already newer than its source (FORCE=1 to redo)
  if [ -z "${FORCE:-}" ] && [ -f "$out" ] && [ "$out" -nt "$file" ]; then
    echo "== $name  (up to date, skip)"
    return
  fi
  echo ">> $name  <-  $file"
  "$MANIC" "$file" --preset studio --no-brand --record "$dir" --fps 60
  mv "$dir/out.mp4" "$out" 2>/dev/null && rm -rf "$dir" || true
}

# guide samples -> name.mp4
for f in book/samples/*.manic; do
  render "$f" "$(basename "$f" .manic)"
done

# examples gallery -> ex-name.mp4  (matches data-video="ex-..." in examples.md).
# Skip the prompt-test-* diagnostic captures (not gallery demos).
for f in examples/*.manic; do
  name="$(basename "$f" .manic)"
  case "$name" in prompt-test*) continue ;; esac
  render "$f" "ex-$name"
done

echo
echo "✓ unbranded mp4s in $OUT/ — upload to YouTube, put the ids in book/videos.txt"
