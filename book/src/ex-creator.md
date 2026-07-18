# Creator formats — responsive social video

Creator Kit v2 turns a question, answers, illustration and reusable creator profile into a polished timed social clip—without hand-authoring the layout or timeline. The same format adapts to portrait 9:16, feed 4:5, square 1:1 and landscape 16:9. Its regions respect named Shorts, Reels and TikTok safe areas.

The global default `template("mono")` gives Creator scenes a professional
black-and-white surface without any template declaration. Use `shorts` when
channel accent hue matters; template choice never changes the responsive
regions or timing. See the [template selection guide](templates.md).

For the recommended idea-to-export workflow, phone-size design checklist, and
production-ready starter, begin with [Create a polished Reel](creator-reels.md).

The default `studio` skin is restrained and editorial: rounded crisp panels, one accent colour and purposeful motion. Legacy `badge`, `minimal`, `glass` and `plain` skins remain available. A quiz's optional spec accepts both old reveal words and explicit v2 controls:

```manic
quiz(q, "Which glass separates blue and red light more?",
     "studio layout=media-first reveal=rise timer=bar density=comfortable motion=studio safe=shorts accent=cyan");
```

Controls are `skin`, `reveal`, `layout=auto|stack|grid|media-first`, `density=compact|comfortable|spacious`, `labels=letters|numbers|none`, `timer=ring|bar|number|segments|ticks|pulse|none`, `motion=calm|studio|punch|cut`, `pace=quick|balanced|calm|dramatic`, `seconds`, `safe=shorts|reels|tiktok|clean`, and `accent`. `option` handles one to six answers and fits text to narrow cards; stack is intentionally capped at four while auto/grid support six. Mark exactly one answer correct. `run` plays the ask→choices→countdown→reveal beat. `explain` adds optional author-supplied answer context.

## Question and answer design

Letters are the default because they scan quickly in a timed quiz. Use
`labels=numbers` for ordered choices or `labels=none` for polls and short
statements. Every card reserves the same right-hand success zone, so a reveal
check never collides with long answer text; a fifth option is centred in its
last grid row instead of looking accidentally left-aligned.

The generated parts have stable semantic tags for safe custom styling and
animation. Question parts use `q.question`, `.question.panel`,
`.question.kicker`, `.question.rule`, and `.question.text`. Options use
`q.options`, `q.option.a` through `.option.f`, then role suffixes such as
`.card`, `.badge`, `.label`, `.text`, and `.check`. The correct choice also has
`q.option.correct`. Existing compact entity ids such as `q.q`, `q.c0`, and
`q.t0` remain valid.

### creator-v2-options-socials

An asset-free review scene for the professional A/B/C/D treatment and the
native YouTube, X, and web identity lockups:

```manic
{{#include ../../examples/creator-v2-options-socials.manic}}
```

## Timing v2 — choreography and appearance are independent

A plain `quiz(q,"...")` remains the recommended zero-config choice: it uses the balanced pace and polished draining ring. Inline `pace=` and `seconds=` cover quick changes. For full control, author each phase separately and choose a timer look without changing the timeline:

Timing v2 itself is format-neutral. The quiz form below is an adapter that gives
the common engine quiz-specific phase names; for `timing(clock,"intro=1 demo=6")`
with `timed`/`during` in any ordinary scene, see [Timing — Generic Timing v2](timing.md#generic-timing-v2--one-clock-for-any-scene).

```manic
timing(q, "calm ask=1.2 options=1.1 think=6 reveal=0.8 hold=2.2 stagger=0.07");
timerstyle(q, "look=segments position=media number=outside direction=drain \
size=large color=magenta label=THINK finish=pulse");
run(q); // exact authored total: 11.3 seconds
```

`timing` phases are `ask`, `options`, `think`, `reveal`, `hold`, and `stagger`. With a preset, `run(q,8)` proportionally scales the whole beat. Once any numeric phase is explicit, use `run(q)`; also supplying a duration would leave two competing sources of truth, so manic reports a clear error.

`timerstyle` accepts `look=ring|bar|number|segments|ticks|pulse|none`, `position=auto|header|media|below`, `number=inside|outside|none`, `direction=drain|fill`, `size`, `thickness`, `color`, `track`, `label`, `font=mono|display`, and `finish=fade|hold|flash|pulse`. The looks are native shapes, so they remain crisp, theme-aware and progress-animatable at every canvas size—no SVG is required.

Every timer exposes stable groups for ordinary modifiers: `q.timer`, `q.timer.track`, `q.timer.progress`, `q.timer.value`, `q.timer.label`, and `q.timer.effects`. Standalone `countdown(id,[at],[secs],["style"])` uses exactly the same vocabulary.

### creator-v2-timing

An explicitly choreographed portrait quiz with LaTeX, a media-first layout and segmented timer:

```manic
{{#include ../../examples/creator-v2-timing.manic}}
```

### creator-v2-timers

The six native timer looks running side by side in landscape:

```manic
{{#include ../../examples/creator-v2-timers.manic}}
```

### creator-v2-timing-square

A square feed card using a scaled dramatic preset and a filling tick timer:

```manic
{{#include ../../examples/creator-v2-timing-square.manic}}
```

## creator-v2

The complete v2 core: responsive quiz, optics media, signature footer, explanation and final end card. Change the canvas in this example to `(1080,1350)`, `(1080,1080)` or `(1280,720)` to see the regions reflow.

```manic
{{#include ../../examples/creator-v2.manic}}
```

The expanded profile keeps identity in one reusable declaration:

```manic
creator(me, "@anish2good name=Optics_Lab tagline=Physics_made_visible \
yt=zarigatongy x=@anish2good web=8gwifi.org/manic \
accent=cyan secondary=magenta \
footer=signature cta=Follow_for_more safe=shorts");

socials(me);                 // responsive selected footer
endcard(me);                 // hidden final lockup
// ... after the main beat:
show(me.endcard, 0.6);
```

Footer styles are `social`, `compact`, `signature`, and `none`. Social footers
use a consistent native-vector icon system, so no image or SVG assets are
needed. Supported keys and aliases are `yt|youtube`, `x|twitter`,
`ig|instagram`, `tt|tiktok`, `fb|facebook`, `li|linkedin`, `gh|github`,
`web|site|url`, and `email|mail`; unknown keys receive a neutral link icon.
With up to three identities, the configured values are shown beside their
icons. Larger sets collapse to an icon row plus the profile handle. Use
`logo=` only when you intentionally want a separate custom avatar or channel
mark in compact/signature layouts.

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
