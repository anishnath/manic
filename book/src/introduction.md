# manic — the manual

**manic is a tiny language for making animations.** You write a short text file;
manic renders a smooth, glowing video. No timeline scrubbing, no keyframes by
hand — you *describe* what's on screen and *when things happen*, and the engine
does the rest, deterministically.

It's built for **explainer videos** — math, algorithms, data structures, or
anything you can draw — and it's designed so a non-programmer can read and write
it. It draws in **2D** and, when you want depth, in **[3D](3d.md)** too.

## The whole idea in 30 seconds

A manic file has two parts:

1. **The cast** — the shapes on screen (a circle, a line, some text). You give
   each one a **name**.
2. **The script** — what happens over time, called out by name: *draw this*,
   *move that*, *flash it green*.

```manic
title("Hello");
canvas("16:9");

circle(sun, (640, 360), 90);   // the cast: a circle named `sun`
color(sun, cyan);

show(sun, 0.6);                 // the script: fade it in over 0.6s
pulse(sun);                     // then give it a little pulse
```

That's the entire model. The rest of this guide walks through the vocabulary —
one small, runnable example at a time — so by the end you can storyboard a video
in your head and type it out.

## How to read this book

Every section has a **runnable sample** and a short **video** of it playing, so
you see exactly what each word does. Copy any sample into a `.manic` file and:

```sh
manic yourfile.manic              # live preview window
manic yourfile.manic --record out # render to out/out.mp4
```

Ready? [Start with your first animation →](getting-started.md)

Making vertical social content? Take the production path directly:
[Create a polished Reel →](creator-reels.md)

Choosing the visual surface? Manic defaults to the professional black-and-white
`mono` look; compare every option in [Templates →](templates.md).
