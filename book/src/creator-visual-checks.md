# Visual checks — review every format before publishing

A responsive Manic story can share its content and timing across a Reel, feed
post, square video, and landscape lesson. Before recording those versions, run
one publishing audit:

```sh
manic check examples/reactive-multiformat.manic --canvas all
```

Manic rebuilds the source at four logical canvases—portrait, 4:5 feed, square,
and 16:9 landscape—then checks the settled frame of every named story stage.
A clean result looks like this:

```text
ok — examples/reactive-multiformat.manic [portrait]: visual checks passed
ok — examples/reactive-multiformat.manic [feed]: visual checks passed
ok — examples/reactive-multiformat.manic [square]: visual checks passed
ok — examples/reactive-multiformat.manic [landscape]: visual checks passed
ok — examples/reactive-multiformat.manic: visual audit passed all 4 formats
```

Use this after the story and layout feel right, but before spending time on the
final render.

## What the first audit catches

Visual-check v1 reports four common publishing problems:

| check | what it protects |
|---|---|
| Canvas bounds | text, equations, images, or primary objects outside the frame |
| Creator safe area | content underneath platform controls or too close to a protected edge |
| Content overlap | two substantial text, equation, or image boxes competing for the same space |
| Readability | text or rendered notation that is too small for the selected format |

Each message names the **format**, **story stage**, **time**, and responsible
**entity**, then suggests a practical fix:

```text
warning [square · takeaway @ 8.45s]: `caption` overlaps `result`
  entities: `caption` and `result`
  suggestion: separate the entities, shorten/wrap the text, or reflow this format
```

The command exits unsuccessfully when it finds either an error or warning, so
it can guard a publishing script or CI job. Ordinary `manic check file.manic`
remains the fast parse-and-validation check. To validate only one responsive
layout, keep using `manic check file.manic --canvas square`.

## Why named steps improve the result

The audit reviews the settled state at the end of each `step("name")` (or
marker-defined stage), not every frame of a transition. That avoids treating a
deliberate entrance, exit, or equation rewrite crossfade as a layout failure.
It also makes a diagnosis useful: `takeaway` tells you which idea needs fixing,
not merely that something happened at 8.45 seconds.

If a file has no named stage, Manic checks its final authored frame. For a
creator story, prefer meaningful names such as `question`, `experiment`,
`proof`, and `takeaway`; they help editing, seeking, publishing, and review.

## Fixes that stay reusable

When the audit finds a problem, preserve one responsive source:

- Position with `w`, `h`, `cx`, and `cy` instead of copying pixel coordinates
  between files.
- Give continuing content stable ids so a message points to the same entity in
  every stage.
- Shorten copy or use `wrap(id, width)` before reducing the type size.
- Use one small `if h > 1.45*w { ... } else if w > 1.25*h { ... } else { ... }`
  layout branch when the composition genuinely needs to reflow.
- Choose the correct Creator `safe=shorts|reels|tiktok|clean` profile. Use
  `safezone(...)` while previewing the protected area; it is a design guide,
  not content for the finished video.
- Leave enough room for the busiest settled stage, not only the opening frame.

The gold reference is
[`reactive-multiformat.manic`](ex-creator.md#reactive-multiformat). It uses one
timeline and passes the audit in all four formats.

## What still needs human review

Visual-check v1 is intentionally conservative. It does not yet judge collision
paths during transitions, camera-transformed world or 3D bounds, detached
labels and links, reading speed, or whether an equation agrees mathematically
with a plot. It also cannot decide whether the hierarchy is beautiful or the
hook is compelling.

After the automated audit passes, preview the hook, busiest transition, result,
and end hold at phone size. The command catches mechanical layout mistakes;
the creator still owns clarity, rhythm, and taste.

Next: [Animate one value through many connected views →](creator-parameters.md)
