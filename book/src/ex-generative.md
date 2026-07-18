# Generative & recursive

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## gun-shot

A pure-imagination SCENE — no physics kit, just storytelling: a gun fires, the camera
flies along with the bullet (`cam`/`zoom`), a block drops in out of nowhere, and BOOM —
`flash`/`shake`/`pulse` + a `for`-loop spark burst. manic as a movie language.

```manic
{{#include ../../examples/gun-shot.manic}}
```

<div class="manic-video" data-video="ex-gun-shot"></div>

## fractal_tree

One recursive `def`, drawn to depth 12.

```manic
{{#include ../../examples/fractal_tree.manic}}
```

<div class="manic-video" data-video="ex-fractal_tree"></div>

## particles-flow

Contained ambient motion and live curved connections in four generic words: `particles`,
`wander`, `link`, and `flow`. The ids supply the domain meaning.

```manic
{{#include ../../examples/particles-flow.manic}}
```

<div class="manic-video" data-video="ex-particles-flow"></div>

## hue_wave

An animated hue wave across a grid.

```manic
{{#include ../../examples/hue_wave.manic}}
```

<div class="manic-video" data-video="ex-hue_wave"></div>

## hill_run

A little scene animated with the language layer.

```manic
{{#include ../../examples/hill_run.manic}}
```

<div class="manic-video" data-video="ex-hill_run"></div>

## walk

An articulated stick figure walking down a road — legs swing, arms counter-swing, the body
bobs — built purely from the language layer (`let` + `for` + trig), no character rig.

```manic
{{#include ../../examples/walk.manic}}
```

<div class="manic-video" data-video="ex-walk"></div>

## two_person_walk

Two figures walk toward each other, MEET in the middle, shake hands, then continue past —
a little choreographed scene from loops and arithmetic alone (the language layer as animation).

```manic
{{#include ../../examples/two_person_walk.manic}}
```

<div class="manic-video" data-video="ex-two_person_walk"></div>

## equal_cuts

A circle halved again and again (pizza cuts).

```manic
{{#include ../../examples/equal_cuts.manic}}
```

<div class="manic-video" data-video="ex-equal_cuts"></div>

## archimedes_pi

Bounding pi with inscribed / circumscribed polygons.

```manic
{{#include ../../examples/archimedes_pi.manic}}
```

<div class="manic-video" data-video="ex-archimedes_pi"></div>

## pieday

A Pi Day card: a rainbow petal-flower built from a loop of circles, radial rays,
the digits of π, and the definition `circumference / diameter = pi`.

```manic
{{#include ../../examples/pieday.manic}}
```

<div class="manic-video" data-video="ex-pieday"></div>
