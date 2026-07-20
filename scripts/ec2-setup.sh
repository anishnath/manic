#!/usr/bin/env bash
# Provision an Ubuntu EC2 box to run the manic binary headlessly. yes
#
# macroquad loads OpenGL/X11 at runtime, so even the prebuilt binary needs these
# .so's present; rendering also needs a virtual display (Xvfb) + Mesa software GL
# + ffmpeg. Fonts are baked into the binary. Optional `asset:` resources belong
# under /usr/local/share/manic/assets (the deploy workflow installs them).
#
#   scp target/release/manic ubuntu@<host>:/usr/local/bin/manic   # arch-matched!
#   ssh ubuntu@<host> 'bash -s' < scripts/ec2-setup.sh
#   ssh ubuntu@<host> manic-render examples/hashmap.manic --record out --fps 30
set -euo pipefail

sudo apt-get update
# libasound2 is renamed libasound2t64 on Ubuntu 24.04+, so try both.
sudo apt-get install -y --no-install-recommends \
  xvfb xauth \
  libx11-6 libxi6 libxcursor1 libxrandr2 libxinerama1 \
  libgl1 libgl1-mesa-dri libglu1-mesa \
  ffmpeg \
  || true
sudo apt-get install -y --no-install-recommends libasound2t64 \
  || sudo apt-get install -y --no-install-recommends libasound2 \
  || true

# headless wrapper: virtual display + software GL, then exec manic
sudo tee /usr/local/bin/manic-render >/dev/null <<'EOF'
#!/usr/bin/env bash
export LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe
exec xvfb-run -a -s "-screen 0 1920x1080x24" manic "$@"
EOF
sudo chmod +x /usr/local/bin/manic-render

echo
echo "✓ ready. Ensure the manic binary is on PATH (e.g. /usr/local/bin/manic), then:"
echo "    manic-render yourfile.manic --record out --fps 30      # -> out/out.mp4"
echo "    manic-render yourfile.manic --still 2.0                 # -> a PNG"
echo "  (plain 'manic ... check' needs no display; rendering uses manic-render)"
if [ ! -d /usr/local/share/manic/assets ]; then
  echo "  note: install manic-assets.tar.gz under /usr/local/share/manic/assets for bundled asset: URIs"
fi
