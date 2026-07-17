# Creator formats — Shorts

Vertical (9:16) social-video formats a creator fills in. The `creator` kit turns a question, a few answers and a social profile into a branded, timed Short — no timeline authoring. `quiz` picks a card **skin** (`badge` default · `minimal` · `glass` · `plain`) and a question **reveal** (`type` default · `fade` · `rise` · `pop` · `cut`) from one order-free style string, e.g. `"glass fade"`; `option` cards auto-lay-out by count (centred column ≤3, 2×2 for 4); `run` plays the whole ask→countdown→reveal beat; `figure` auto-fits an illustration between the header and the cards.

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## quiz-skins

The quiz Short in a dozen lines: `quiz`/`option`/`run` + a `creator`/`socials` footer.
Change the one style word on `quiz(...)` to switch card SKIN — `badge` (framed panel +
coloured letter badges), `minimal`, `glass` (glowing borders) or `plain` — and add a
question REVEAL in the same string (e.g. `"glass fade"`). The correct card lights up with a
green badge + check on reveal; a draining ring counts the timer down.

```manic
{{#include ../../examples/quiz-skins.manic}}
```

<div class="manic-video" data-video="ex-quiz-skins"></div>

## quiz-euler

A quiz Short with an ANIMATED figure: the geo kit constructs the Euler line (the answer),
and `figure(...)` AUTO-FITS the whole triangle+circumcircle into the zone between the
question header and the answer cards — no coordinate tuning. The question, four cards, the
countdown and the whole ask→countdown→reveal beat are just `quiz`/`option`/`run`.

```manic
{{#include ../../examples/quiz-euler.manic}}
```

<div class="manic-video" data-video="ex-quiz-euler"></div>

## quiz-geometry

The hand-authored proof behind the kit (≈60 lines from shipped primitives): a question, an
animated geometry figure, four option cards, a countdown and a time-out reveal. Useful to
see what `quiz`/`option`/`run` automate under the hood.

```manic
{{#include ../../examples/quiz-geometry.manic}}
```

<div class="manic-video" data-video="ex-quiz-geometry"></div>

## quiz-geometry-2

A layout stress-test: a different olympiad question with TWO figures side by side (an acute
triangle with its circumcentre INSIDE vs an obtuse one with it OUTSIDE), proving the 2×2
options, countdown and footer keep their spacing for richer figure content.

```manic
{{#include ../../examples/quiz-geometry-2.manic}}
```

<div class="manic-video" data-video="ex-quiz-geometry-2"></div>
