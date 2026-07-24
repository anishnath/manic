# Transforms & morphing

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## gradient

Gradient paint, the acceptance demo: one word `gradient(id, c1, c2, …, [mode])` covers
a radial well, a three-stop height-colored plot, a spline colored by `"curvature"`, an
RK4 free kick colored by `"speed"`, and an arc-length arrow whose head takes the tip
color. The color is computed, not painted — and every stop stays template-aware.

```manic
{{#include ../../examples/gradient.manic}}
```

<div class="manic-video" data-video="ex-gradient"></div>

## motion-graphics-v2

The generic Motion Graphics V2 acceptance scene: one persistent marker carries an attached
label along a path, becomes a declared visual blueprint, releases the label, gathers the
same particles into a ring, and turns the whole arrangement around one shared pivot. Uses
`attach`, `become`, and `turn` with no renderer flags or subject-specific vocabulary.

```manic
{{#include ../../examples/motion-graphics-v2.manic}}
```

<div class="manic-video" data-video="ex-motion-graphics-v2"></div>

## motion-graphics-v2-story

The advanced composition example: one question travels through a field of facts, its `WHY?`
label follows, notation rewrites into a pattern, the question becomes a model, and the same
facts arrange and turn as one knowledge system. Combines `attach`, `become`, and `turn` with
`to`, `travel`, `flow`, `spin`, `arrange`, `wander`, `rewrite`, `seq`, `par`, and `stagger`.

```manic
{{#include ../../examples/motion-graphics-v2-story.manic}}
```

<div class="manic-video" data-video="ex-motion-graphics-v2-story"></div>

## reactive-math-journey

A playful vertical journey from `1+1` in Class 1 through fractions, algebra,
geometry, calculus, linear algebra, probability and Fourier analysis to a PhD-level
functional integral — then back to the curiosity that started it all.

```manic
{{#include ../../examples/reactive-math-journey.manic}}
```

<div class="manic-video" data-video="ex-reactive-math-journey"></div>

## reactive-math-notation

One Reels-ready stage exercises structured LaTeX across thirteen notation worlds:
algebra, calculus, limits, trigonometry, logic, sums/products, physics, chemistry,
biology, probability, matrices/vectors, mixed prose/math, and creator notation.

```manic
{{#include ../../examples/reactive-math-notation.manic}}
```

<div class="manic-video" data-video="ex-reactive-math-notation"></div>

## quadratic-formula-continuity

The quadratic formula by completing the square with one persistent LaTeX equation.
Each authored `rewrite` retains unchanged symbols, moves reused terms, and introduces
only the new notation — the acceptance benchmark for structured formula motion.

```manic
{{#include ../../examples/quadratic-formula-continuity.manic}}
```

<div class="manic-video" data-video="ex-quadratic-formula-continuity"></div>

## transforms

Apply a 2x2 matrix (ApplyMatrix) to a group.

```manic
{{#include ../../examples/transforms.manic}}
```

<div class="manic-video" data-video="ex-transforms"></div>

## transform_copy

Duplicate an entity, then transform the copy.

```manic
{{#include ../../examples/transform_copy.manic}}
```

<div class="manic-video" data-video="ex-transform_copy"></div>

## morph

A sampled-point shape morph from A to B.

```manic
{{#include ../../examples/morph.manic}}
```

<div class="manic-video" data-video="ex-morph"></div>
