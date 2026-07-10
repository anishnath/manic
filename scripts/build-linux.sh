#!/usr/bin/env bash
# Cross-build the `manic` CLI for Linux — BOTH arches — into ./dist/, via Docker.
# Used to ship prebuilt binaries to Ubuntu EC2 boxes (Graviton = arm64, Intel/AMD
# = amd64). On an arm64 mac, arm64 builds natively (fast) and amd64 builds under
# emulation (slow but works); on an amd64 host it's the reverse.
#
#   ./scripts/build-linux.sh            # both arches
#   ./scripts/build-linux.sh arm64      # just one (arm64 | amd64)
#
# Then, on the target box:
#   scp dist/manic-linux-<arch> ubuntu@host:/tmp/manic
#   scp scripts/ec2-setup.sh    ubuntu@host:/tmp/
#   ssh ubuntu@host 'sudo mv /tmp/manic /usr/local/bin/manic && sudo chmod +x /usr/local/bin/manic && bash /tmp/ec2-setup.sh'
#   ssh ubuntu@host manic-render yourfile.manic --record out --fps 30
#
# Match the arch to `uname -m` on the box (aarch64 -> arm64, x86_64 -> amd64).
#
# glibc: the binary is built on Debian bookworm (glibc 2.36) — runs on Ubuntu
# 24.04. For Ubuntu 22.04 (glibc 2.35) change `rust:1-bookworm` in
# docker/Dockerfile to an older/ubuntu:22.04-based rust image and rebuild.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p dist

build() {
  local arch="$1" platform="linux/$1" out="dist/manic-linux-$1" img="manic-build-$1"
  echo ">> building $platform -> $out"
  docker build --platform "$platform" -f docker/Dockerfile --target build -t "$img" .
  local c; c="$(docker create --platform "$platform" "$img")"
  docker cp "$c:/app/target/release/manic" "$out"
  docker rm "$c" >/dev/null
  file "$out"
  # smoke-test: the binary executes + `check` works on a bare Linux image
  docker run --platform "$platform" --rm -v "$PWD/dist:/d" -v "$PWD/examples:/ex" \
    debian:bookworm-slim "/d/manic-linux-$arch" check /ex/hashmap.manic
}

case "${1:-both}" in
  arm64) build arm64 ;;
  amd64) build amd64 ;;
  both)  build arm64; build amd64 ;;
  *) echo "usage: $0 [arm64|amd64|both]" >&2; exit 2 ;;
esac
echo "done -> dist/"
