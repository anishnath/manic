#!/usr/bin/env bash
# Package the repo `assets/` tree and install it on the render host under
# /usr/local/share/manic/assets — the SYSTEM_ASSET_ROOT where the manic binary
# resolves `asset:` URIs (diagram icons, fonts, models, …). This is the asset
# half of scripts/build-linux.sh, as a standalone one-shot.
#
# Usage:
#   ./scripts/deploy-assets.sh                        # host "ai", /usr/local/share/manic, sudo
#   MANIC_HOST=ubuntu@1.2.3.4 ./scripts/deploy-assets.sh
#   MANIC_ROOT=/opt/manic ./scripts/deploy-assets.sh  # different install root
#   SUDO= ./scripts/deploy-assets.sh                  # no sudo (login already owns the dir)
#
# Staged through /tmp then unpacked into $ROOT/assets, so it works whether or not
# the login can write the system path directly.
set -euo pipefail
cd "$(dirname "$0")/.."

HOST="${MANIC_HOST:-ai}"                 # ssh target (alias or user@host)
ROOT="${MANIC_ROOT:-/usr/local/share/manic}"
SUDO="${SUDO-sudo}"                      # set SUDO= to install without sudo
TARBALL="dist/manic-assets.tar.gz"
REMOTE_TMP="/tmp/manic-assets.tar.gz"

[ -d assets ] || { echo "error: no assets/ directory in $(pwd)" >&2; exit 1; }

echo "==> packing assets/ ($(du -sh assets | cut -f1)) -> $TARBALL"
mkdir -p dist
# macOS `tar` is bsdtar and stamps Apple xattrs (com.apple.provenance) as
# `LIBARCHIVE.xattr` PAX headers, which make GNU tar on the host print harmless
# "Ignoring unknown extended header keyword" warnings. --no-xattrs drops them
# (supported by both bsdtar and GNU tar); guarded so an ancient tar can't choke.
TAR_OPTS=()
if tar --no-xattrs -cf /dev/null -C assets README.md >/dev/null 2>&1; then
  TAR_OPTS+=(--no-xattrs)
fi
tar "${TAR_OPTS[@]}" -C assets -czf "$TARBALL" .   # contents of assets/, no top-level prefix
echo "    packed $(du -h "$TARBALL" | cut -f1)"

echo "==> copying to $HOST:$REMOTE_TMP"
scp "$TARBALL" "$HOST:$REMOTE_TMP"

echo "==> unpacking into $ROOT/assets on $HOST"
ssh "$HOST" "
  set -e
  $SUDO install -d '$ROOT/assets'
  $SUDO tar -xzf '$REMOTE_TMP' -C '$ROOT/assets'
  rm -f '$REMOTE_TMP'
  echo \"    installed \$(find '$ROOT/assets' -type f | wc -l | tr -d ' ') files under $ROOT/assets\"
"

echo "==> done. Point the render binary at it with MANIC_ASSETS_DIR=$ROOT/assets"
echo "    (or rely on the default $ROOT/assets), then render a diagram to verify."
