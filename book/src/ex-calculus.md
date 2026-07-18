# Calculus & functions

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## calculus-demo

The flagship: two big ideas on one curve. A tangent slides along a bell curve
with a live slope readout (flat at the peak), then the area sweeps open while the
integral climbs to its true value — on properly numbered, scaled axes.

```manic
{{#include ../../examples/calculus-demo.manic}}
```

<div class="manic-video" data-video="ex-calculus-demo"></div>

## sine_wave

`axes` + `plot`, a curve traced on, then vectors.

```manic
{{#include ../../examples/sine_wave.manic}}
```

<div class="manic-video" data-video="ex-sine_wave"></div>

## function_graph

Plot an expression straight from a formula string.

```manic
{{#include ../../examples/function_graph.manic}}
```

<div class="manic-video" data-video="ex-function_graph"></div>

## area_under_curve

Riemann rectangles sweeping to the integral.

```manic
{{#include ../../examples/area_under_curve.manic}}
```

<div class="manic-video" data-video="ex-area_under_curve"></div>

## riemann_rainbow

Coloured Riemann rectangles revealed one by one.

```manic
{{#include ../../examples/riemann_rainbow.manic}}
```

<div class="manic-video" data-video="ex-riemann_rainbow"></div>

## riemann_readout

Running sums shown as a live computed number.

```manic
{{#include ../../examples/riemann_readout.manic}}
```

<div class="manic-video" data-video="ex-riemann_readout"></div>

## tangent

The tangent line to a curve, sliding along it — its tilt is read from the
function itself, so it's always the true slope (flat at the peaks).

```manic
{{#include ../../examples/tangent.manic}}
```

<div class="manic-video" data-video="ex-tangent"></div>

## analysis

Ask one curve everything at once — tangent, a live slope number, the normal, the
area sweeping open beneath it, and the integral climbing to its true value.

```manic
{{#include ../../examples/analysis.manic}}
```

<div class="manic-video" data-video="ex-analysis"></div>

## newton

Newton's method, drawn as a zig-zag: from a first guess, slide down each tangent
to the axis, back up to the curve, and watch the guesses walk to the root.

```manic
{{#include ../../examples/newton.manic}}
```

<div class="manic-video" data-video="ex-newton"></div>

## inverse-derivatives

Why a function and its inverse have reciprocal slopes: `e^x` and `ln x` mirrored
across `y = x`, with the slopes at matching points multiplying to 1.

```manic
{{#include ../../examples/inverse-derivatives.manic}}
```

<div class="manic-video" data-video="ex-inverse-derivatives"></div>

## spline

Interpolation: one smooth curve drawn through a scattered set of points — it
passes through every knot exactly.

```manic
{{#include ../../examples/spline.manic}}
```

<div class="manic-video" data-video="ex-spline"></div>

## trajectory

A phase portrait: three paths flowing under a differential system, each
spiralling into the sink at the origin.

```manic
{{#include ../../examples/trajectory.manic}}
```

<div class="manic-video" data-video="ex-trajectory"></div>

## band

The area trapped between two curves, filled directly with `band(top,bottom)` while both
boundary plots remain visible.

```manic
{{#include ../../examples/band.manic}}
```

<div class="manic-video" data-video="ex-band"></div>

## curve-features

Read a cubic by its geometry: maxima/minima where the slope is zero and an inflection
where the curve changes its bend (`extrema`, `inflections`).

```manic
{{#include ../../examples/curve-features.manic}}
```

<div class="manic-video" data-video="ex-curve-features"></div>

## ftc

The Fundamental Theorem of Calculus: accumulate the area under a curve, differentiate
that area function, and watch the original function return.

```manic
{{#include ../../examples/ftc.manic}}
```

<div class="manic-video" data-video="ex-ftc"></div>

## limit

A removable discontinuity visualized as an approaching point, open circle and live
finite limit at x→0.

```manic
{{#include ../../examples/limit.manic}}
```

<div class="manic-video" data-video="ex-limit"></div>

## limit-infinity

A rational function settling onto its horizontal asymptote, with `limit(...,inf)`
detecting and marking the value at infinity.

```manic
{{#include ../../examples/limit-infinity.manic}}
```

<div class="manic-video" data-video="ex-limit-infinity"></div>

## taylor

Taylor polynomials of increasing degree closing in on sin(x), one additional
approximation at a time.

```manic
{{#include ../../examples/taylor.manic}}
```

<div class="manic-video" data-video="ex-taylor"></div>
