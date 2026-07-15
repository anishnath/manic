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

## linear-algebra-3d — the essence, in 3D

The 3D companion to the `linear-algebra` lesson: one matrix
`[[1,0,0],[0,3,1],[0,1,3]]` (det 8; eigenvalues 1, 2, 4) seen two ways on an
orbiting stage — first as a transformation (the unit cube → a parallelepiped
whose volume is the determinant), then through its eigenvectors (the invariant
axes that only stretch). Start here for 3D.

```manic
{{#include ../../examples/linear-algebra-3d.manic}}
```

<div class="manic-video" data-video="ex-linear-algebra-3d"></div>

## linear-map3

Linear algebra in 3D: a 3×3 matrix deforms the unit cube into a parallelepiped,
with basis arrows i/j/k landing on the matrix's columns and the enclosed volume
labelled as the determinant (`linmap3`). The 3D echo of `linear-map`.

```manic
{{#include ../../examples/linear-map3.manic}}
```

<div class="manic-video" data-video="ex-linear-map3"></div>

## eigen3

The real eigenvectors of a 3×3 matrix, in 3D: the invariant lines through the
origin that only stretch (by λ) when the matrix acts (`eigen3`). The 3D echo of
`eigen`. A symmetric matrix gives three perpendicular real eigen-axes; a rotation
leaves one real axis and two complex eigenvalues.

```manic
{{#include ../../examples/eigen3.manic}}
```

<div class="manic-video" data-video="ex-eigen3"></div>

## matrix3

A 3×3×3 block of cubes, with a shear matrix **M** and its inverse **M⁻¹** applied and undone.

```manic
{{#include ../../examples/matrix3.manic}}
```

<div class="manic-video" data-video="ex-matrix3"></div>

## double-integral3

Multivariable calculus: the volume under a surface as a limit of finer and
finer columns — a double integral, made solid. The coarse blocks refine until
they hug the surface.

```manic
{{#include ../../examples/double-integral3.manic}}
```

<div class="manic-video" data-video="ex-double-integral3"></div>
