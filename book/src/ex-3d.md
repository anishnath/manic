# 3D scenes

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video). See the [Going 3D](3d.md) chapter for the words used here.

## three_d

Cubes, spheres, arrows, a curve, a surface and solids together — the 3D basics on one stage.

```manic
{{#include ../../examples/three_d.manic}}
```

<div class="manic-video" data-video="ex-three_d"></div>

## solids3

Filled, shaded solids: a prism, a cone, and a lathed vase.

```manic
{{#include ../../examples/solids3.manic}}
```

<div class="manic-video" data-video="ex-solids3"></div>

## param3

Parametric surfaces a height field can't make — a torus, a sphere, and a Möbius strip.

```manic
{{#include ../../examples/param3.manic}}
```

<div class="manic-video" data-video="ex-param3"></div>

## extrude3

Lifting flat shapes into solids, including a boolean cut-out (a plate with a hole) and an L-beam.

```manic
{{#include ../../examples/extrude3.manic}}
```

<div class="manic-video" data-video="ex-extrude3"></div>

## morph3

Morphing across families — a cube into a sphere, a saddle into a bowl, a helix into a ring.

```manic
{{#include ../../examples/morph3.manic}}
```

<div class="manic-video" data-video="ex-morph3"></div>

## matrix3

A 3×3×3 block of cubes, with a shear matrix **M** and its inverse **M⁻¹** applied and undone.

```manic
{{#include ../../examples/matrix3.manic}}
```

<div class="manic-video" data-video="ex-matrix3"></div>
