# I Was Tired of Fighting After Effects. So I Started *Describing* My Animations Instead.

### A creator's journey with manic — the little language that turns plain text into precise, glowing explainer videos.

---

Every creator who's tried to make an explainer video knows the wall.

You have a clear idea in your head — *the tangent slides along the curve while the slope readout ticks down to zero at the peak.* Simple. Obvious. You can see it.

Then you open After Effects. Ninety minutes later you're nudging keyframes by hand, the "slope" is a text layer you're animating manually so it lies a little, and the curve is a Bézier path you eyeballed — so it's not even *true*. You wanted to explain an idea. Instead you became a timeline janitor.

There had to be a better way to go from **"here's what I want"** to **a finished video** without the scrubbing, the keyframes, and the quiet dishonesty of hand-drawn math.

That better way, for me, turned out to be **manic**.

---

## The whole idea in 30 seconds

manic is a tiny language for making animations. You write a short text file; it renders a smooth, glowing video. No timeline, no keyframes — you **describe** what's on screen and **when things happen**, and the engine does the rest, deterministically.

A manic file has just two parts:

1. **The cast** — the things on screen. You give each one a name.
2. **The script** — what happens over time, called out by name: *draw this, move that, flash it green.*

```manic
title("Hello");
canvas("16:9");

circle(sun, (640, 360), 90);   // the cast: a circle named `sun`
color(sun, cyan);

show(sun, 0.6);                 // the script: fade it in over 0.6s
pulse(sun);                     // then give it a little pulse
```

That's the entire model. If you've ever compared it to **Manim** (Grant Sanderson's math-animation library) — that instinct is right, but where Manim is Python you program *against*, manic is a small language you *read and write*. No boilerplate, no classes, no render loop. A non-programmer can pick up the whole vocabulary in an afternoon.

Here's what that first afternoon actually looks like.

---

## Beat 1 — Your first six lines

The smallest real animation: a title fades in, a circle draws itself, it pulses once.

```manic
text(head, (cx, cy), "Hello, manic"); color(head, cyan); hidden(head);
circle(sun, (cx, cy + 120), 80); untraced(sun);

show(head, 0.5);   // fade the text in
draw(sun, 1.2);    // trace the circle's outline on, like a pen
pulse(sun);        // a grow-and-settle attention pulse
```

Two things click immediately:

- **`cx`, `cy` are the canvas centre.** manic hands you `w`, `h`, `cx`, `cy` for free, so you place things by *meaning*, not by hard-coded pixels. Your scene works at any size.
- **The script runs top to bottom.** `show`, then `draw`, then `pulse` — one after another. Want things to happen *together*? Wrap them in `par { … }`. That's the whole timing model.

There's a single gotcha worth learning on day one, and it's a nice one because it maps to how things actually appear:

- **`hidden` + `show`** → a fade-in (text, filled shapes).
- **`untraced` + `draw`** → a draw-on, like a pen tracing a line (strokes, plots).

Get those two pairs right and everything else follows.

---

## Beat 2 — The diagram is actually *true*

This is the part that hooked me. When you plot a function in manic, you don't draw a curve that *looks* like a parabola — you give it the formula, and the engine computes it. The tangent line's tilt is read from the function itself, so it's the *real* slope, flat at the peaks. The area under the curve is the real integral, filling as the number climbs to its true value.

You're not faking the math for the camera. The animation is *correct by construction*. For anyone explaining STEM, that's not a nice-to-have — it's the whole point.

And when you need real notation, you write real LaTeX, and let one equation **evolve in place** instead of cutting between five static formulas:

```manic
equation(work, (cx, 520), `x^2 + 2x = 3`, 54);
rewrite(work, `x^2 + 2x + 1 = 4`, 0.85, smooth);
rewrite(work, `(x + 1)^2 = 4`, 0.85, smooth);
```

manic matches the rendered math by reading order and layout role, so the terms that *move* glide while the symbols that *stay* stay put. The viewer's eye follows the algebra the way it would if you were working it out on a board. You still own every step — the engine just makes the transition honest and smooth.

---

## Beat 3 — Make it a Reel, once, for every platform

Most of what I make now is vertical. manic's Creator Kit owns the parts that are tedious and easy to get wrong — responsive layout, safe areas, question hierarchy, answer cards, the reveal, the footer, the end card.

You write for a phone, not a slide: one question that fits in a line, one visual idea, one accent colour, one motion personality. Then — and this is the part that saved me hours — **you write the story once and reframe it at export.** The same source file becomes a 9:16 Reel, a 4:5 feed post, a square, or 16:9, and the layout reflows automatically:

```bash
manic story.manic --canvas 9:16     # the Reel
manic story.manic --canvas 4:5      # the feed post
manic story.manic --canvas 16:9     # the widescreen cut
```

No duplicated timelines. No re-editing four files when you change one word.

---

## Beat 4 — Change the idea, not the scene

The mental shift that made me *fast* was this: a polished explainer isn't a sequence of separate shots. It's **one persistent visual world** that you nudge forward.

You declare the cast once — the equation, the plot, the diagram, the caption. Then you give each story beat a short, meaningful name and, inside it, describe *only what changes.* Everything you don't mention simply stays on screen.

```manic
step("measure-slope") {
  rewrite(work, `f'(x) = 0.70x`, 0.90, smooth);
  to(tangent, x, 2.8, 3.20, smooth);
  to(rate,    x, 2.8, 3.20, smooth);
  say(caption, "The tangent and its slope update together.", 0.40);
}
```

The formula, the tangent, the live value, and the caption all move as one thought. The curve and everything else hold their place. It's easier to write, trivially easy to revise ("actually, make the slope 0.8"), and — because nothing ever cuts to black — genuinely nicer to watch.

---

## Why it *feels* different

A few things quietly add up to a completely different creative experience:

- **You describe outcomes, not frames.** "Move here over 0.9 seconds, smoothly." The engine interpolates. You never touch a keyframe.
- **It's deterministic.** The same file always produces the exact same video. Scrub to any moment; it's reproducible. Version-control your animations like code, because they *are* text.
- **It's honest.** Curves are computed, slopes are real, integrals are exact. Your explainer can't accidentally lie.
- **It's readable.** Six months later you can open the file and know exactly what it does — because it reads like a description, not a program.

Taking the timeline scrubbing out of the equation isn't a small win. It's the difference between "I'll make that video someday" and "I made it before lunch."

---

## Start here

You don't install anything to try it. Open the playground, type a few lines, watch it render:

**▶ [8gwifi.org/manic](https://8gwifi.org/manic)** — the browser playground
**▶ [8gwifi.org/manic/docs](https://8gwifi.org/manic/docs)** — the full guide + a gallery of examples, each with the exact script that made it

Copy any example. Change a number. Run it. That's the loop — and it's the whole reason I stopped fighting my tools and started describing what I actually wanted to see.

*Made with manic.*
