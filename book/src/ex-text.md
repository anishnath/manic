# Text & UI

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## typewriter

Text revealed character by character.

```manic
{{#include ../../examples/typewriter.manic}}
```

<div class="manic-video" data-video="ex-typewriter"></div>

## captions

Karaoke / word-pop caption modes.

```manic
{{#include ../../examples/captions.manic}}
```

<div class="manic-video" data-video="ex-captions"></div>

## terminal_boot

The neon terminal template booting up.

```manic
{{#include ../../examples/terminal_boot.manic}}
```

<div class="manic-video" data-video="ex-terminal_boot"></div>

## brace

The curly-brace family.

```manic
{{#include ../../examples/brace.manic}}
```

<div class="manic-video" data-video="ex-brace"></div>

## banner

The manic logo / banner reveal.

```manic
{{#include ../../examples/banner.manic}}
```

<div class="manic-video" data-video="ex-banner"></div>

## equation

Display-quality LaTeX for fractions, roots, sums, powers and integrals, with semantic
colour and template tinting.

```manic
{{#include ../../examples/equation.manic}}
```

<div class="manic-video" data-video="ex-equation"></div>

## inline-math

Inline LaTeX mixed with ordinary prose, including wrapped explanatory text and a
standalone display equation.

```manic
{{#include ../../examples/inline-math.manic}}
```

<div class="manic-video" data-video="ex-inline-math"></div>

## image

Embed a raster image (PNG/JPG) with `image(id, (x,y), "path", w, h)` — a real file drawn
into the scene and animated like any entity (shown, spun, pulsed, moved). Unlocks logos,
avatars and photo backdrops (e.g. a creator's brand in a template).

```manic
{{#include ../../examples/image.manic}}
```

<div class="manic-video" data-video="ex-image"></div>
