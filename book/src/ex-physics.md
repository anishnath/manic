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

## second-law-thermodynamics

Why entropy only grows, in five vignettes: gas particles mixing into far more
microstates, heat flowing until two sides equalize, free expansion into new volume,
a heat engine that must dump waste heat, and the reversible-vs-irreversible limit.

```manic
{{#include ../../examples/second-law-thermodynamics.manic}}
```

<div class="manic-video" data-video="ex-second-law-thermodynamics"></div>

## timing-v2-scene

Generic Timing v2 controlling an ordinary physics scene: one named-phase clock schedules
the intro, pendulum motion and finish independently from its native timer look.

```manic
{{#include ../../examples/timing-v2-scene.manic}}
```

<div class="manic-video" data-video="ex-timing-v2-scene"></div>

## zeroth-law-thermodynamics

The Zeroth Law told through three particle-filled bodies: thermal relations connect,
the bodies settle onto one temperature axis, and equilibrium becomes visible.

```manic
{{#include ../../examples/zeroth-law-thermodynamics.manic}}
```

<div class="manic-video" data-video="ex-zeroth-law-thermodynamics"></div>

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

## car-suspension-annotated

A marketing hero: the quarter-car suspension on a `template("paper")` brochure page,
elevated with generic base-manic — a live `counter` (sprung mass), leader-`arrow`
callouts, and an `energygraph` of the shock being absorbed — riding a scrolling road.

```manic
{{#include ../../examples/car-suspension-annotated.manic}}
```

<div class="manic-video" data-video="ex-car-suspension-annotated"></div>

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

## incline-pulley

The incline-Atwood: a block on an incline tied over a pulley at the top to a hanging
mass. m₂ outpulls m₁·sinθ, so the block climbs while the mass descends — `energygraph`
tracks the KE↔PE trade (`inclinepulley`).

```manic
{{#include ../../examples/incline-pulley.manic}}
```

<div class="manic-video" data-video="ex-incline-pulley"></div>

## double-incline

Two blocks on a wedge's two slopes, tied over a pulley at the apex (right slope rough).
The 70 kg block on the gentle 30° slope beats the 12 kg block on the steep 50° smooth
slope — connected motion on two inclines (`doubleincline`).

```manic
{{#include ../../examples/double-incline.manic}}
```

<div class="manic-video" data-video="ex-double-incline"></div>

## incline-bumper

A block slides down an incline into a spring bumper at the base, compresses it, and
launches back up — one-sided contact, gravity PE ↔ kinetic ↔ spring PE, energy
conserved (`inclinebumper`).

```manic
{{#include ../../examples/incline-bumper.manic}}
```

<div class="manic-video" data-video="ex-incline-bumper"></div>

## collide-blocks

The classic momentum demo: block 1 hangs on a spring to the wall, block 2 slides in and
they collide. A live Σp readout shows momentum conserved at every collision; elastic
(e=1) keeps total energy flat while it sloshs between KE and the spring (`collideblocks`).

```manic
{{#include ../../examples/collide-blocks.manic}}
```

<div class="manic-video" data-video="ex-collide-blocks"></div>

## collide-blocks-annotated

Conservation of momentum, the MANIC way — not a 1:1 port of the lab sim but a guided
lesson: the live Σp readout as the star, the KE↔spring-PE energy view, staged callouts,
and honest narration (with a wall-spring, Σp is conserved AT each collision, not constant).

```manic
{{#include ../../examples/collide-blocks-annotated.manic}}
```

<div class="manic-video" data-video="ex-collide-blocks-annotated"></div>

## bullet-block

A bullet fired into a block EMBEDS (perfectly inelastic). The flight is slow-mo so you can
watch it cross, then a live speed readout collapses from 40 m/s to ~1 — momentum survives,
energy does not. Uses `collide_1d(e=0)` (`bulletblock`).

```manic
{{#include ../../examples/bullet-block.manic}}
```

<div class="manic-video" data-video="ex-bullet-block"></div>

## bullet-impact

BEST OF BOTH: the cinematic gun-shot (gun · muzzle flash · a flying `cam`/`zoom` · BOOM)
wrapped around the REAL `bulletblock` physics — the collision is genuinely inelastic, the
live speed readout actually collapses 40 → ~1, and the BOOM is synced to the true impact.

```manic
{{#include ../../examples/bullet-impact.manic}}
```

<div class="manic-video" data-video="ex-bullet-impact"></div>

## bullet-block-annotated

The bullet's JOURNEY, the manic way: a gun fires, a muzzle flash, a glowing bullet crosses
the gap in slow-motion and embeds — the speed readout crashing 40 → ~1. A scene, not the
bare lab sim (base-manic staging over `bulletblock`).

```manic
{{#include ../../examples/bullet-block-annotated.manic}}
```

<div class="manic-video" data-video="ex-bullet-block-annotated"></div>

## newtons-cradle

Newton's cradle: pull one ball, one swings out the far side — momentum and energy pass
straight through the chain. An EVENT-DRIVEN sim (free-flight pendulums between elastic
collisions resolved by a shared 1-D impulse), the crowd-pleaser (`newtonscradle`).

```manic
{{#include ../../examples/newtons-cradle.manic}}
```

<div class="manic-video" data-video="ex-newtons-cradle"></div>

## string-wave

A wave on a plucked string: 36 masses on springs, both ends fixed (the discretised wave
equation). Pluck it off-centre and the pulse splits, travels, and reflects off the ends —
a rainbow chain that wiggles, pre-simulated with RK4 (`stringwave`).

```manic
{{#include ../../examples/string-wave.manic}}
```

<div class="manic-video" data-video="ex-string-wave"></div>

## loop-track

A ball rolls down a ramp and around a vertical LOOP-THE-LOOP — the curved-track case.
A bead energy solver (v = √(2g(H−y)) along the arc) so it visibly slows at the top;
release above 2·radius to clear it. `energygraph` tracks KE↔PE (`looptrack`).

```manic
{{#include ../../examples/loop-track.manic}}
```

<div class="manic-video" data-video="ex-loop-track"></div>

## loop-cinematic

The loop-the-loop as a MOVIE with real physics inside: the camera pushes in as the ball
climbs, and the tension is genuine — a modest release height means it truly crawls over
the top before rocketing out. `cam`/`zoom` beats synced to the `looptrack` sim.

```manic
{{#include ../../examples/loop-cinematic.manic}}
```

<div class="manic-video" data-video="ex-loop-cinematic"></div>

## spring-chain

Three blocks joined by two springs on an incline — coupled oscillators. Pull one and the
whole chain rings (normal modes / beating); shown in the incline's frame since uniform
gravity doesn't touch the internal motion (`springchain`).

```manic
{{#include ../../examples/spring-chain.manic}}
```

<div class="manic-video" data-video="ex-spring-chain"></div>

## incline-showcase

One paper page, FOUR live incline problems: a friction ramp, an incline+pulley, a
two-slope wedge, and a spring bumper — revealed one at a time with narration, then all
run in parallel. Real base-manic staging (`template("paper")` + `hidden`/`show` + `say`
+ `par`), not a physics dump.

```manic
{{#include ../../examples/incline-showcase.manic}}
```

<div class="manic-video" data-video="ex-incline-showcase"></div>

## textbook-incline-fbd

A block on an incline as a physics-class FREE-BODY DIAGRAM: the reusable `forces(id)`
view draws gravity/normal/friction/`a` vectors on the block, a second panel redraws them
from a point, and `template("paper")` inks it — then `run` slides the block (`ramp`).

```manic
{{#include ../../examples/textbook-incline-fbd.manic}}
```

<div class="manic-video" data-video="ex-textbook-incline-fbd"></div>

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

## gradient-fastest-descent

Bernoulli 1696, told THROUGH gradients: four wires race from A to B, each wearing a
3-stop vertical speedometer (`v = √(2gΔh)` — depth IS speed), then the cycloid's secret
is revealed by a `"curvature"` gradient — it bends hardest at the start. A 16:9 3B1B-
style story; companion Short is `gradient-fastest-descent-shorts`.

```manic
{{#include ../../examples/gradient-fastest-descent.manic}}
```

<div class="manic-video" data-video="ex-gradient-fastest-descent"></div>

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

## spinning-cylinder-flow

Potential flow around a spinning cylinder — WHY a spun ball curves. Parallel streamlines
(uniform flow) and concentric circles (a free vortex) superpose into the asymmetric flow
around the cylinder: bunched (fast, low-pressure) one side, spread (slow, high-pressure) the
other → a net Magnus force, `L=ρUΓ`. Every streamline is integrated straight from the
velocity field with `trajectory` — the asymmetry is computed, not drawn.

```manic
{{#include ../../examples/spinning-cylinder-flow.manic}}
```

<div class="manic-video" data-video="ex-spinning-cylinder-flow"></div>
