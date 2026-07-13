# Getting started

Let's make the smallest real animation: a title fades in, a circle draws itself,
and it pulses once.

```manic
{{#include ../samples/hello.manic}}
```

**▶ See it play:**

<div class="manic-video" data-video="hello"></div>

## What each line is doing

| line | meaning |
|---|---|
| `title("Hello, manic")` | the window/file title (metadata) |
| `canvas("16:9")` | the frame size — `16:9` is 1280×720 (see [Colour & style](colour.md)) |
| `text(head, (cx, cy)…)` | **cast:** a text entity named `head` at the canvas centre |
| `color / size / hidden` | **modifiers** — style `head`, and start it invisible |
| `circle(sun, …)` | **cast:** a circle named `sun` |
| `untraced(sun)` | start with the stroke *undrawn*, ready to trace on |
| `show(head, 0.5)` | **script:** fade `head` in over 0.5s |
| `draw(sun, 1.2)` | **script:** trace `sun`'s outline on over 1.2s |
| `pulse(sun)` | **script:** grow-and-settle attention pulse |
| `wait(1.0)` | hold for a second at the end |

Two things worth internalising right away:

- **`cx`, `cy` are the canvas centre.** manic gives you `w`, `h`, `cx`, `cy` for
  free so you can place things without hard-coding pixels. `(cx, cy)` is always
  the middle.
- **The order of the cast doesn't matter, but the script runs top-to-bottom.**
  `show`, then `draw`, then `pulse` play *one after another*. To make things
  happen *at the same time*, you wrap them in `par { … }` — that's the
  [Timing](timing.md) chapter.

## Two ways to appear

Notice `head` uses `hidden` + `show`, but `sun` uses `untraced` + `draw`. That's
the one gotcha worth learning early:

- **`hidden` + `show`** → a fade-in (good for text and filled shapes).
- **`untraced` + `draw`** → a *draw-on*, like a pen tracing the outline (good for
  strokes, lines, plots).

Get those two pairs right and everything else clicks. Next: [the shapes you can
put on screen →](shapes.md)
