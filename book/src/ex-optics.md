# Optics — light as geometry

Easy builtins with the **real physics underneath** — Snell's law and Sellmeier dispersion — so the bending and the colours are earned, not painted. Each is static geometry that animates by a parameter sweep: call `run(id)`.

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## refraction

Snell's law you can watch: a ray crossing from air into glass bends toward the normal,
and `run` sweeps the incidence angle so the refracted ray swings — the live in/out
read-outs are the true angles. Start in the denser medium and it shows total internal
reflection past the critical angle (`refract`).

```manic
{{#include ../../examples/refraction.manic}}
```

<div class="manic-video" data-video="ex-refraction"></div>

## lens

A converging lens: a parallel beam bends to meet at the focal point F (ideal thin lens —
every ray passes through F). `run` sweeps the focal length, so the focus slides in as the
lens gets stronger (`lens`).

```manic
{{#include ../../examples/lens.manic}}
```

<div class="manic-video" data-video="ex-lens"></div>

## prism

White light into a prism, out as a RAINBOW — each colour traced through both faces with
its own refractive index (real Sellmeier dispersion), so blue bends more than red because
glass genuinely slows blue more. `run` sweeps the incidence angle and the fan widens (`prism`).

```manic
{{#include ../../examples/prism.manic}}
```

<div class="manic-video" data-video="ex-prism"></div>

## achromat

The optics capstone — chromatic aberration and its fix: a single lens focuses blue nearer
than red, so white light never comes to one point; `run` sweeps in the achromatic doublet
and the colours snap back to a single sharp focus (`achromat`).

```manic
{{#include ../../examples/achromat.manic}}
```

<div class="manic-video" data-video="ex-achromat"></div>

## refraction-paper

Snell's law as a `template("paper")` TEXTBOOK figure: inked media labels, the normal, and
the law itself, with a camera easing in on the bending point as `run` sweeps the angle —
the geometric builtins suit paper (`refract` + annotation + camera).

```manic
{{#include ../../examples/refraction-paper.manic}}
```

<div class="manic-video" data-video="ex-refraction-paper"></div>

## lens-paper

The converging lens inked on paper and narrated by a TYPEWRITER caption (a different
elevation lens): labelled parallel rays and focal point F while `run` slides the focus
(`lens` + `type`).

```manic
{{#include ../../examples/lens-paper.manic}}
```

<div class="manic-video" data-video="ex-lens-paper"></div>

## prism-cinematic

The prism on a dark optics bench where the spectrum GLOWS (a rainbow washes out on paper):
the colour names pop in word-by-word (`wordpop`) as the fan spreads and the camera flies
toward it — KINETIC-TYPE elevation (`prism`).

```manic
{{#include ../../examples/prism-cinematic.manic}}
```

<div class="manic-video" data-video="ex-prism-cinematic"></div>

## achromat-cinematic

The achromat with the CAMERA magnifying the focal region so the red/blue split is dramatic,
a `bracelabel` marking the aberration gap that closes as `run` sweeps in the doublet and
the colours merge (`achromat` + camera + brace).

```manic
{{#include ../../examples/achromat-cinematic.manic}}
```

<div class="manic-video" data-video="ex-achromat-cinematic"></div>

## lens-system

A REAL multi-element lens, ray-traced through its actual spherical surfaces (not the ideal
thin lens): the fast singlet reveals SPHERICAL ABERRATION — `draw` sketches the rays, then
`run` sweeps a sensor plane and the live spot read-out dips but never reaches a point,
because the outer rays focus short (`lenssystem`, presets singlet/doublet/triplet).

```manic
{{#include ../../examples/lens-system.manic}}
```

<div class="manic-video" data-video="ex-lens-system"></div>

## ray-fan

Reading an aberration: `rayfan` plots each ray's error at focus against where it entered the
lens. A flat line is a perfect lens — the singlet's cubic S-CURVE is textbook spherical
aberration (the edges bend too much), which a doublet flattens (`rayfan`).

```manic
{{#include ../../examples/ray-fan.manic}}
```

<div class="manic-video" data-video="ex-ray-fan"></div>

## spot-diagram

Lens quality as a picture: `spotdiagram` plots where a ray bundle actually lands at focus.
A fast single lens smears into a blur disc (the circle of least confusion), while a cemented
doublet collapses to a point — both to the same scale, RMS 4 px → under 1 px (`spotdiagram`).

```manic
{{#include ../../examples/spot-diagram.manic}}
```

<div class="manic-video" data-video="ex-spot-diagram"></div>

## lens-prescription

Type your OWN lens: `lenssystem` takes a design by name (`"plano-convex"`, `"doublet"`, …)
OR a custom PRESCRIPTION — the designer's surface table `"radius thickness glass [conic]
[aperture] | …"` — traced through the true surfaces with real Sellmeier glass (`lenssystem`).

```manic
{{#include ../../examples/lens-prescription.manic}}
```

<div class="manic-video" data-video="ex-lens-prescription"></div>

## aspheric-lens

How an ASPHERE kills spherical aberration: a spherical surface can't focus a wide beam to a
point (a blur, RMS 1.5 px), but reshaping it to the right conic — one constant in the
prescription — collapses every ray to a point (RMS 0.1 px). Two real ray-traced lenses,
spherical vs aspheric, side by side (`lenssystem` conic + `spotdiagram`).

```manic
{{#include ../../examples/aspheric-lens.manic}}
```

<div class="manic-video" data-video="ex-aspheric-lens"></div>

## off-axis

The hard test — light 8° OFF the axis. `fieldspot` traces a full 2-D pupil in 3-D: a single
lens flares into a COMA comet, while a doublet holds the spot near the Airy disk (the
diffraction limit). Real field aberration only a 3-D trace shows (`fieldspot`).

```manic
{{#include ../../examples/off-axis.manic}}
```

<div class="manic-video" data-video="ex-off-axis"></div>
