# Creator formats — responsive social video

Creator Kit v2 turns a question, answers, illustration and reusable creator profile into a polished timed social clip—without hand-authoring the layout or timeline. The same format adapts to portrait 9:16, feed 4:5, square 1:1 and landscape 16:9. Its regions respect named Shorts, Reels and TikTok safe areas.

The default `studio` skin is restrained and editorial: rounded crisp panels, one accent colour and purposeful motion. Legacy `badge`, `minimal`, `glass` and `plain` skins remain available. A quiz's optional spec accepts both old reveal words and explicit v2 controls:

```manic
quiz(q, "Which glass separates blue and red light more?",
     "studio layout=media-first reveal=rise timer=bar density=comfortable motion=studio safe=shorts accent=cyan");
```

Controls are `skin`, `reveal`, `layout=auto|stack|grid|media-first`, `density=compact|comfortable|spacious`, `timer=ring|bar|number|none`, `motion=calm|studio|punch|cut`, `safe=shorts|reels|tiktok|clean`, and `accent`. `option` handles one to six answers and fits text to narrow cards; stack is intentionally capped at four while auto/grid support six. Mark exactly one answer correct. `run` plays the ask→choices→countdown→reveal beat. `explain` adds optional author-supplied answer context.

## creator-v2

The complete v2 core: responsive quiz, optics media, signature footer, explanation and final end card. Change the canvas in this example to `(1080,1350)`, `(1080,1080)` or `(1280,720)` to see the regions reflow.

```manic
{{#include ../../examples/creator-v2.manic}}
```

The expanded profile keeps identity in one reusable declaration:

```manic
creator(me, "@opticslab name=Optics_Lab tagline=Physics_made_visible \
logo=assets/manic-logo.png accent=cyan secondary=magenta \
footer=signature cta=Follow_for_more safe=shorts");

socials(me);                 // responsive selected footer
endcard(me);                 // hidden final lockup
// ... after the main beat:
show(me.endcard, 0.6);
```

Footer styles are `social`, `compact`, `signature`, and `none`. Exact logo/avatar art is supplied with `logo=`; manic does not bundle platform trademarks.

`safezone(id,"reels")` visualises a named platform safe area. `figure(group)` now measures text, images, equations and curves. For a live derived construction, tag all source dependencies; v2 reports a clear error when fitting an incomplete live group.

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## quiz-skins

The quiz Short in a dozen lines: `quiz`/`option`/`run` + a `creator`/`socials` footer.
Change the one style word on `quiz(...)` to switch a legacy card SKIN — `badge` (framed panel +
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
