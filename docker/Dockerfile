# manic — headless render container.
#
# macroquad needs OpenGL, so we render on a virtual display (Xvfb) with Mesa's
# software GL (llvmpipe) and pipe frames to ffmpeg. Recording is deterministic
# (fixed timestep), so output is identical to a desktop run. Fonts are embedded
# in the binary (include_bytes!), so nothing but the .manic files is needed.
#
#   docker build -t manic -f docker/Dockerfile .
#   # render a built-in example to ./out/out.mp4
#   docker run --rm -v "$PWD/out:/work/out" manic \
#       manic examples/hashmap.manic --record out --fps 30
#   # or a still, or your own file (mount it)
#   docker run --rm -v "$PWD:/work" manic manic myfile.manic --still 2.0

# ---- build ----
FROM rust:1-bookworm AS build
WORKDIR /app
COPY . .
RUN cargo build --release --bin manic

# ---- runtime ----
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
      xvfb xauth \
      libx11-6 libxi6 libxcursor1 libxrandr2 libxinerama1 \
      libgl1 libgl1-mesa-dri libglu1-mesa \
      libasound2 \
      ffmpeg ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/manic /usr/local/bin/manic
COPY examples /work/examples
WORKDIR /work

# force software OpenGL (no GPU in the container)
ENV LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe

# wrap every command in a virtual X display
ENTRYPOINT ["xvfb-run", "-a", "-s", "-screen 0 1920x1080x24"]
CMD ["manic", "examples/hashmap.manic", "--record", "out", "--fps", "30", "--frames", "90"]
