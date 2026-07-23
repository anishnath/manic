# 3D scenes

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video). See the [Going 3D](3d.md) chapter for the words used here. `three-d-v2-lab` uses the bundled geometry-only asset `asset:models/manic-pyramid.obj`; production packages install it automatically, or you can replace its `model3` call with a built-in solid.

## three-d-v2

The compact 3D V2 reference: frame a tagged craft, attach its parts, travel one
persistent subject along a spatial route, deploy the rig with a rigid turn, and
become the final blueprint — five creator words in one continuous scene.

```manic
{{#include ../../examples/three-d-v2.manic}}
```

<div class="manic-video" data-video="ex-three-d-v2"></div>

## three-d-v2-story

A vertical creator story about a satellite finding orbit. The same mission survives
assembly, launch, transformation, deployment, and screen-aware camera composition
without a scene reset.

```manic
{{#include ../../examples/three-d-v2-story.manic}}
```

<div class="manic-video" data-video="ex-three-d-v2-story"></div>

## three-d-v2-lab

A creator-first spatial lab: safe-aware framing, rigid assembly, a live projection
and edge, a moving route, surface contour, depth-scaled label, bounded finishes,
variable tube, and controlled OBJ geometry in one continuous story.

```manic
{{#include ../../examples/three-d-v2-lab.manic}}
```

<div class="manic-video" data-video="ex-three-d-v2-lab"></div>

## trapped-light-dimensions

A photon escapes one dimension at a time: first a 5-unit line, then the 5–12–13
diagonal of a plane, and finally the 13–84–85 diagonal through a volume. One
persistent light beam makes the generalized Pythagorean idea visible.

```manic
{{#include ../../examples/trapped-light-dimensions.manic}}
```

<div class="manic-video" data-video="ex-trapped-light-dimensions"></div>

## dimensions-unfold

A point stretches into a line, the line sweeps sideways into a plane, and the plane
lifts into a room. The geometry grows continuously instead of resetting between
1D, 2D, and 3D.

```manic
{{#include ../../examples/dimensions-unfold.manic}}
```

<div class="manic-video" data-video="ex-dimensions-unfold"></div>

## textbook-length-area-volume

Why units become cm, cm², and cm³: one measured segment sweeps out a rectangle,
then the rectangle rises into a cuboid. A textbook measurement story built from
extrusion rather than three disconnected formulas.

```manic
{{#include ../../examples/textbook-length-area-volume.manic}}
```

<div class="manic-video" data-video="ex-textbook-length-area-volume"></div>

## textbook-coordinate-worlds

A point earns a longer address as dimensions unlock: x on a line, (x,y) on a
plane, then (x,y,z) in space. Coordinates remain attached to the same idea while
the world expands around it.

```manic
{{#include ../../examples/textbook-coordinate-worlds.manic}}
```

<div class="manic-video" data-video="ex-textbook-coordinate-worlds"></div>

## textbook-function-to-solid

A diameter becomes a semicircle and the semicircle revolves into a sphere. The
story links a 1D domain, a 2D graph, and a 3D solid through one continuous
generating motion.

```manic
{{#include ../../examples/textbook-function-to-solid.manic}}
```

<div class="manic-video" data-video="ex-textbook-function-to-solid"></div>

## textbook-statistical-dimensions

Data grows from a one-variable number line to a two-variable scatter plot and a
three-variable point cloud. The axes and observations evolve together so statistical
dimension reads as information, not decoration.

```manic
{{#include ../../examples/textbook-statistical-dimensions.manic}}
```

<div class="manic-video" data-video="ex-textbook-statistical-dimensions"></div>

## textbook-geometry-dimension-reduction

The reverse journey: a sphere reveals a great-circle section, then that circle
collapses to its diameter. A 3D→2D→1D geometry lesson that makes section and
projection relationships explicit.

```manic
{{#include ../../examples/textbook-geometry-dimension-reduction.manic}}
```

<div class="manic-video" data-video="ex-textbook-geometry-dimension-reduction"></div>

## textbook-watermelon-sections

A paper-style spatial lesson that turns sphere sections into a continuous story:
horizontal and vertical great-circle cuts make two halves, then perpendicular cuts
separate one quarter from the three-quarter remainder. The section faces are bounded
parametric surfaces, so the authored geometry is exact rather than a flat overlay.

```manic
{{#include ../../examples/textbook-watermelon-sections.manic}}
```

<div class="manic-video" data-video="ex-textbook-watermelon-sections"></div>

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

## multivariable3

Calculus on a surface: a smooth hill, its tangent plane and the gradient arrow at a
chosen point, inspected with an orbiting camera.

```manic
{{#include ../../examples/multivariable3.manic}}
```

<div class="manic-video" data-video="ex-multivariable3"></div>

## volume3

The volume under a surface represented as solid 3D Riemann-sum columns, turning a
double integral into visible geometry.

```manic
{{#include ../../examples/volume3.manic}}
```

<div class="manic-video" data-video="ex-volume3"></div>

## heightmap3

The Grid Kit → 3D bridge: `heightmap3(land, grid, "z(x,y,h)")` lifts a 2-D grid's per-cell
state into a surface3-style terrain mesh. A seeded Wave Function Collapse settles a map,
then its walls rise into an island terrain a camera orbits — the grid kit stays entirely
3D-unaware.

```manic
{{#include ../../examples/heightmap3.manic}}
```

<div class="manic-video" data-video="ex-heightmap3"></div>

## heightmap3-world

The creative payoff: a grid-kit WFC map settles in 2-D, then the very same grid lifts into
a 3-D world as the camera tilts down — one grid, two dimensions. `h` (the cell value) is a
third formula variable added to the expression engine for exactly this.

```manic
{{#include ../../examples/heightmap3-world.manic}}
```

<div class="manic-video" data-video="ex-heightmap3-world"></div>

## noise-terrain

Procedural generation from a single formula: `noise(x,y)` and `fbm(x,y)` (fractal Brownian
motion) are now formula functions beside sin/cos, so `surface3(land, "fbm(x*0.9,y*0.9)*2.4")`
sculpts an organic fractal landscape the camera tours — no new kit, just two functions the
shared expression evaluator now understands.

```manic
{{#include ../../examples/noise-terrain.manic}}
```

<div class="manic-video" data-video="ex-noise-terrain"></div>
