# Creator formats — responsive social video

Creator Kit v2 turns a question, answers, media and a reusable creator profile into a polished timed social clip. The same source adapts to 9:16, 4:5, 1:1 and 16:9 with platform-safe regions. `studio` plus a balanced ring is the restrained default; `timing` controls the beat independently from `timerstyle`, whose native ring, bar, number, segments, ticks and pulse looks remain crisp at every size. Explicit `layout`, `density`, `labels`, `motion`, `safe` and `accent` controls customise the rest. Responsive native social icons, optional explanations and final end cards share the same brand profile.

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## perfect-reel

The gold-path production starter: phone-safe composition, real LaTeX, exact pacing,
professional mono styling, creator identity, timeline markers and a focused end card.

```manic
{{#include ../../examples/perfect-reel.manic}}
```

<div class="manic-video" data-video="ex-perfect-reel"></div>

## reactive-multiformat

One named reactive story rendered as portrait, 4:5 feed, square, or landscape with the
`--canvas` override. Responsive variables and layout branches reflow before construction,
while the same steps, timing, equation continuity and creator identity remain intact.

```manic
{{#include ../../examples/reactive-multiformat.manic}}
```

<div class="manic-video" data-video="ex-reactive-multiformat"></div>

## parameter-journeys

One visible parameter drives a quadratic plot, its live tangent and slope, a geometric
position, scale and a derived numeric readout. Named steps animate only the value; `bind`
keeps every representation continuous and the source reflows across all four formats.

```manic
{{#include ../../examples/parameter-journeys.manic}}
```

<div class="manic-video" data-video="ex-parameter-journeys"></div>

## pascal-triangle

A non-quiz Creator v2 Short built entirely from the computation layer: each cell's
binomial coefficient is a `prod` reduction, the triangle reveals row by row, the
sum-of-two-parents rule is highlighted, and colouring the odd cells uncovers Sierpinski's
triangle — all inside a branded 9:16 shorts frame with creator identity, socials and an
end card (no grid kit needed: the triangle is triangular).

```manic
{{#include ../../examples/pascal-triangle.manic}}
```

<div class="manic-video" data-video="ex-pascal-triangle"></div>

## creator-lattice-paths

The rectangular cousin of Pascal, on a real grid-kit lattice: 'how many ways from corner
to corner moving only right and down?' Every cell's path-count is a `prod` reduction
(C(i+j,i)), the same above-plus-left rule is highlighted, the far corner holds the total,
and one actual monotone path is traced with a `spline`. Blueprint template, and a
different typewriter beat — the question erases and retypes itself into the answer.

```manic
{{#include ../../examples/creator-lattice-paths.manic}}
```

<div class="manic-video" data-video="ex-creator-lattice-paths"></div>

## creator-rule90-sierpinski

A QUIZ-style Short: Rule 90 (each new cell = its two upper neighbours XOR'd) draws
Sierpinski's triangle from a single dot — because XOR of two parents is exactly Pascal's
triangle mod 2 (a cell is lit iff C(n,k) is odd). The gasket is the quiz's media (fit with
`figure`), building as you're asked to predict it, then the correct card and the reason
reveal. Full Creator v2 quiz: question, options, think timer, explanation and end card.

```manic
{{#include ../../examples/creator-rule90-sierpinski.manic}}
```

<div class="manic-video" data-video="ex-creator-rule90-sierpinski"></div>

## creator-heightmap-world

The Grid→3D bridge as a Short: a grid-kit WFC map settles in 2D, then the SAME grid rises
into 3D terrain via `heightmap3` — the camera pulls back to reveal the whole world, rotates,
then flies in low over the peaks. One grid, two dimensions, inside a 9:16 creator frame with
typed hook, socials and end card.

```manic
{{#include ../../examples/creator-heightmap-world.manic}}
```

<div class="manic-video" data-video="ex-creator-heightmap-world"></div>

## creator-noise-story

How Noise Builds Worlds — a Short walking procedural noise from 1D to fractal: raw `rand(x)`
(jagged) vs smooth `noise(x)` (Perlin), then `noise(x,y)` tilting from a flat field into a 3D
surface, then `fbm` stacking octaves into fractal terrain. Every visual is one formula in the
shared expression engine — the arc that motivated adding `rand`/`noise`/`fbm`.

```manic
{{#include ../../examples/creator-noise-story.manic}}
```

<div class="manic-video" data-video="ex-creator-noise-story"></div>

## creator-v2-options-socials

The asset-free v2.4 review scene: collision-safe question hierarchy, professional A/B/C/D
cards, uniform correct-state spacing, and native YouTube/X/web identity lockups.

```manic
{{#include ../../examples/creator-v2-options-socials.manic}}
```

<div class="manic-video" data-video="ex-creator-v2-options-socials"></div>

## creator-v2

The complete v2 core: responsive studio quiz, optics media, width-aware answer cards, a
signature creator footer, optional explanation and a branded final end card.

```manic
{{#include ../../examples/creator-v2.manic}}
```

<div class="manic-video" data-video="ex-creator-v2"></div>

## creator-v2-timing

Timing v2 in a portrait quiz: exact ask/options/think/reveal/hold phases, LaTeX media, and a
segmented timer whose presentation can change without changing the choreography.

```manic
{{#include ../../examples/creator-v2-timing.manic}}
```

<div class="manic-video" data-video="ex-creator-v2-timing"></div>

## creator-v2-timers

All six native Timing v2 looks—ring, bar, number, segments, ticks and pulse—running side by
side. Native shapes keep every look scalable, theme-aware and progress-animatable.

```manic
{{#include ../../examples/creator-v2-timers.manic}}
```

<div class="manic-video" data-video="ex-creator-v2-timers"></div>

## creator-v2-timing-square

A square feed-card variant with a scaled dramatic preset and a filling tick timer, showing
that timing and timer placement reflow independently across formats.

```manic
{{#include ../../examples/creator-v2-timing-square.manic}}
```

<div class="manic-video" data-video="ex-creator-v2-timing-square"></div>

## creator-v2-olympiad-geometry

An olympiad-level geometry Reel built as pause → predict → prove, with a responsive
construction, authored explanation and reusable creator identity.

```manic
{{#include ../../examples/creator-v2-olympiad-geometry.manic}}
```

<div class="manic-video" data-video="ex-creator-v2-olympiad-geometry"></div>

## creator-v2-latex-calculus

Portrait Creator v2 with inline and display LaTeX: a calculus question, fitted formula
answers and crisp typesetting throughout the timed reveal.

```manic
{{#include ../../examples/creator-v2-latex-calculus.manic}}
```

<div class="manic-video" data-video="ex-creator-v2-latex-calculus"></div>

## creator-v2-latex-algebra

Square Creator v2 on a paper surface, checking that algebraic LaTeX and answer cards
remain balanced and readable outside the vertical format.

```manic
{{#include ../../examples/creator-v2-latex-algebra.manic}}
```

<div class="manic-video" data-video="ex-creator-v2-latex-algebra"></div>

## creator-v2-latex-physics

Landscape Creator v2 with a physics equation, proving the same LaTeX quiz system
reflows cleanly for widescreen explainers.

```manic
{{#include ../../examples/creator-v2-latex-physics.manic}}
```

<div class="manic-video" data-video="ex-creator-v2-latex-physics"></div>

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
