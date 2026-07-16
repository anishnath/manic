# Physics — simulations

Each simulation is **pre-simulated with RK4** at build time — deterministic and replayable — and its parts are ordinary manic entities the whole language composes with. The phase / time / well / energy views are optional and generic: any sim inherits them.

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## pendulum

One pendulum shown four ways from a single deterministic swing: the motion (with a
velocity arrow + KE/PE bars), the phase portrait (θ vs ω), a time series, the
potential-energy well, and energy over time (`pendulum` + `phase`/`timegraph`/
`well`/`energygraph` + `swing`).

```manic
{{#include ../../examples/pendulum.manic}}
```

<div class="manic-video" data-video="ex-pendulum"></div>

## pendulum-damped

The same four views with friction on (`damping`): the swing decays, the phase loop
spirals inward, the well ball settles, and the total-energy line drops — dissipation
told the same way by every panel.

```manic
{{#include ../../examples/pendulum-damped.manic}}
```

<div class="manic-video" data-video="ex-pendulum-damped"></div>

## pendulum-annotated

A guided anatomy lesson proving physics composes with base manic: `section` chapters,
`text` / `arrow` / `bracelabel` annotations, and `show`/`recolor`/`flash`/`pulse` all
driving the sim's parts — no special physics mode.

```manic
{{#include ../../examples/pendulum-annotated.manic}}
```

<div class="manic-video" data-video="ex-pendulum-annotated"></div>

## spring

A mass on a spring (simple harmonic motion) drawn with a real stretching coil — the
same generic views on a *different* system; note the energy well is a **parabola**
(½kx²) rather than the pendulum's cosine (`spring` + the views + `run`).

```manic
{{#include ../../examples/spring.manic}}
```

<div class="manic-video" data-video="ex-spring"></div>

## spring-damped

The damped spring: the coil's oscillation decays, the phase ellipse spirals in, the
ball settles in the parabola, and total energy bleeds away.

```manic
{{#include ../../examples/spring-damped.manic}}
```

<div class="manic-video" data-video="ex-spring-damped"></div>

## double-pendulum

Deterministic chaos: two arms hinged end-to-end whose outer bob traces a wild,
unrepeatable curve — yet the render is frame-identical every run. A 4-D system, so
it shows `phase` (θ₁ vs θ₂) and `energygraph` but has no potential `well`
(`doublependulum` + views + `run`).

```manic
{{#include ../../examples/double-pendulum.manic}}
```

<div class="manic-video" data-video="ex-double-pendulum"></div>

## spring-pendulum

An elastic pendulum — a bob on a springy rod (drawn as a stretching coil) that both
swings and bounces, energy sloshing between the two modes (`springpendulum`).

```manic
{{#include ../../examples/spring-pendulum.manic}}
```

<div class="manic-video" data-video="ex-spring-pendulum"></div>

## kapitza

The Kapitza pendulum: vibrate the pivot fast enough and the **inverted** position
becomes stable — the bob hovers near the top instead of falling (`kapitza`).

```manic
{{#include ../../examples/kapitza.manic}}
```

<div class="manic-video" data-video="ex-kapitza"></div>

## cart-pendulum

A pendulum on a spring-mounted cart rolling on a track — the classic control-theory
system; cart and bob trade momentum and energy (`cartpendulum`).

```manic
{{#include ../../examples/cart-pendulum.manic}}
```

<div class="manic-video" data-video="ex-cart-pendulum"></div>

## compare-pendulum

Sensitive dependence: two identical driven pendulums started 0.001 rad apart drift
onto completely different paths — the butterfly effect, watched in `phase`/`timegraph`
(`comparependulum`).

```manic
{{#include ../../examples/compare-pendulum.manic}}
```

<div class="manic-video" data-video="ex-compare-pendulum"></div>

## vertical-spring

A mass bobbing on a vertical spring under gravity — gravity shifts the equilibrium
but the energy well stays a parabola (`verticalspring`).

```manic
{{#include ../../examples/vertical-spring.manic}}
```

<div class="manic-video" data-video="ex-vertical-spring"></div>

## spring-incline

A mass on a spring on an inclined plane; gravity's along-ramp component sets a new
stretched rest point it oscillates about (`springincline`).

```manic
{{#include ../../examples/spring-incline.manic}}
```

<div class="manic-video" data-video="ex-spring-incline"></div>

## bungee

A bungee jump: free-fall, then a ONE-SIDED elastic cord (it only pulls) catches and
bounces the jumper — note the lopsided energy well (`bungee`).

```manic
{{#include ../../examples/bungee.manic}}
```

<div class="manic-video" data-video="ex-bungee"></div>

## resonance

A driven spring pushed near its natural frequency √(k/m): the amplitude climbs and
climbs — resonance, watched building up in `phase`/`energygraph` (`resonance`).

```manic
{{#include ../../examples/resonance.manic}}
```

<div class="manic-video" data-video="ex-resonance"></div>

## double-spring

Two masses coupled by springs between walls — push one and the energy sloshes back
and forth (beating); normal modes show as diagonals in `phase` (`doublespring`).

```manic
{{#include ../../examples/double-spring.manic}}
```

<div class="manic-video" data-video="ex-double-spring"></div>

## series-parallel-springs

The same mass on springs in series (soft, slow) vs parallel (stiff, fast), side by
side — the `timegraph` makes the frequency difference obvious (`seriesparallel`).

```manic
{{#include ../../examples/series-parallel-springs.manic}}
```

<div class="manic-video" data-video="ex-series-parallel-springs"></div>

## car-suspension

A quarter-car riding a scrolling road — a speed bump, a washboard stretch, and a
pothole — its spring+damper soaking up the ride (`carsuspension`).

```manic
{{#include ../../examples/car-suspension.manic}}
```

<div class="manic-video" data-video="ex-car-suspension"></div>
