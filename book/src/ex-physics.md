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

## spring-annotated

Elevating the spring with a TYPEWRITER lab-note (`type` + `cursor`) and LIVE COUNTERS
(`counter` + `to(_, value, …)`) ticking k and the period up — Hooke's law → parabolic
well → SHM, with no stage-covering section cards. One of three elevation styles.

```manic
{{#include ../../examples/spring-annotated.manic}}
```

<div class="manic-video" data-video="ex-spring-annotated"></div>

## spring-paper

The SAME spring sim dressed as a textbook figure AND run: `template("paper")` inks it,
a hatched `support` wall, a forest-green coil and outlined mass box, Hooke's law and x₀
revealed, then `run` plays the SHM — the paper treatment on a LIVE sim (see pulley-paper).

```manic
{{#include ../../examples/spring-paper.manic}}
```

<div class="manic-video" data-video="ex-spring-paper"></div>

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

## piston

An engine piston: a spinning crank + connecting rod turn rotation into the piston's
up-and-down stroke — the slider-crank mechanism (`piston`).

```manic
{{#include ../../examples/piston.manic}}
```

<div class="manic-video" data-video="ex-piston"></div>

## molecule

A molecule as balls and springs — atoms bonded on every side, vibrating about their
equilibrium shape with the total energy conserved (`molecule`).

```manic
{{#include ../../examples/molecule.manic}}
```

<div class="manic-video" data-video="ex-molecule"></div>

## robot-arm

A two-link robot arm reaching for a target: the joint rates come from the analytic
inverse Jacobian, so the arm drives its end-effector to the goal and settles there —
inverse kinematics as a solved motion (`robotarm`).

```manic
{{#include ../../examples/robot-arm.manic}}
```

<div class="manic-video" data-video="ex-robot-arm"></div>

## pulley

The Atwood machine: two masses over one pulley, the heavier one accelerating down at
(m₁−m₂)g/(m₁+m₂). `energygraph` shows kinetic energy climbing as potential falls
(`pulley`).

```manic
{{#include ../../examples/pulley.manic}}
```

<div class="manic-video" data-video="ex-pulley"></div>

## pulley-scale

The surprise every physics class remembers: an in-line spring scale on an Atwood
machine reads the rope TENSION 2·m₁·m₂·g/(m₁+m₂) — not the sum of the two weights
(`pulleyscale`).

```manic
{{#include ../../examples/pulley-scale.manic}}
```

<div class="manic-video" data-video="ex-pulley-scale"></div>

## block-tackle

A compound pulley (block & tackle): a load on a movable block held by N rope strands,
pulled by an effort mass. N strands = a mechanical advantage of N — an effort of only
load/N balances the load, but the effort end travels N× as far (`blocktackle`).

```manic
{{#include ../../examples/block-tackle.manic}}
```

<div class="manic-video" data-video="ex-block-tackle"></div>

## compound-pulley

A compound pulley with a MOVABLE pulley: a fixed top pulley carries mass A on one side
and a movable lower pulley on the other; that pulley carries B and C. The string
constraints link them (a_A = −a_P, a_B + a_C = 2·a_P); static when mA = mB+mC
(`compoundpulley`).

```manic
{{#include ../../examples/compound-pulley.manic}}
```

<div class="manic-video" data-video="ex-compound-pulley"></div>

## pulley-annotated

The Atwood machine elevated with CAMERA work: `cam` + `zoom` push in on the two masses
for the imbalance beat and glow the heavier one, a `counter` ticks the acceleration up,
then it pulls back to release — cinematography instead of section cards.

```manic
{{#include ../../examples/pulley-annotated.manic}}
```

<div class="manic-video" data-video="ex-pulley-annotated"></div>

## pulley-paper

The SAME Atwood sim dressed as a textbook figure AND run: `template("paper")` inks it
automatically, a hatched `support` ceiling, a forest-green wheel and outlined mass boxes,
a base-manic reveal, then `run` plays the motion — the paper treatment on a LIVE sim.

```manic
{{#include ../../examples/pulley-paper.manic}}
```

<div class="manic-video" data-video="ex-pulley-paper"></div>

## ramp

A block sliding down an inclined plane with static/kinetic friction — the full force
model. Friction turns motion into heat, so the total-energy line steadily falls
(`ramp` + `energygraph`).

```manic
{{#include ../../examples/ramp.manic}}
```

<div class="manic-video" data-video="ex-ramp"></div>

## drop-mass

A mass dropped onto a spring-block STICKS — a perfectly inelastic collision. Watch the
total-energy line step down at impact, then the heavier combined mass oscillate about
a lower equilibrium (`dropmass` + `energygraph`).

```manic
{{#include ../../examples/drop-mass.manic}}
```

<div class="manic-video" data-video="ex-drop-mass"></div>

## raft-cm

A person walks back and forth on a floating raft; with no external force the centre of
mass stays fixed, so the raft glides the opposite way — momentum conservation you can
see (`raft`).

```manic
{{#include ../../examples/raft-cm.manic}}
```

<div class="manic-video" data-video="ex-raft-cm"></div>

## brachistochrone

Four beads race under gravity from A to B down a straight line, a circular arc, a
parabola, and a cycloid. The cycloid — the curve of fastest descent — wins, even
though it dips lower and travels farther (`brachistochrone`).

```manic
{{#include ../../examples/brachistochrone.manic}}
```

<div class="manic-video" data-video="ex-brachistochrone"></div>

## brachistochrone-annotated

The elevation recipe on a RACE, told with KINETIC TYPOGRAPHY: `wordpop` pops the
question in, `karaoke` sweeps a highlight across the four path names as the curves
sketch on, then `flash`/`glow` crown the cycloid — a third, distinct elevation style.

```manic
{{#include ../../examples/brachistochrone-annotated.manic}}
```

<div class="manic-video" data-video="ex-brachistochrone-annotated"></div>

## textbook-pulley

A physics-TEXTBOOK figure, manic style: the `template("paper")` white page, a hatched
`support` ceiling, a green pulley wheel, and outlined labelled mass boxes — the classic
m over 2m+3m arrangement, all base primitives.

```manic
{{#include ../../examples/textbook-pulley.manic}}
```

<div class="manic-video" data-video="ex-textbook-pulley"></div>

## textbook-tension

Another textbook figure: two support ropes at 60°/30° meeting a knot, a string over a
hanging pulley carrying 10 kg with the other end anchored to a hatched floor — `support`
+ `template("paper")` for the flat exam-paper look.

```manic
{{#include ../../examples/textbook-tension.manic}}
```

<div class="manic-video" data-video="ex-textbook-tension"></div>
