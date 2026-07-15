# Linear algebra & tables

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## linear-algebra — the whole subject in five ideas

A guided lesson, not a feature demo: five chapters that build linear algebra as
one connected story. Chapters 1–3 view the **same** matrix `[[2,1],[1,2]]`
through three lenses — a transformation of space (`linmap`), the determinant as
area scaling (`determinant`), and its eigenvectors / diagonalisation
(`diagonalise`) — then it moves on to solving `Ax = b` (`linsolve` → `rref`) and
projection / least-squares (`project`). Start here.

```manic
{{#include ../../examples/linear-algebra.manic}}
```

<div class="manic-video" data-video="ex-linear-algebra"></div>

## linear-map

What a 2×2 matrix does to space: the grid deforms and the basis lands on its
columns (`linmap`), the unit square's area becomes the determinant
(`determinant`), and two directions only stretch — the eigenvectors (`eigen`).

```manic
{{#include ../../examples/linear-map.manic}}
```

<div class="manic-video" data-video="ex-linear-map"></div>

## linear-system

The geometry of solving and spanning, in three panels: a 2×2 system as two lines
crossing at the solution (`linsolve`), two independent vectors reaching the whole
plane, and two parallel vectors collapsing to a line — rank 1 (`span`).

```manic
{{#include ../../examples/linear-system.manic}}
```

<div class="manic-video" data-video="ex-linear-system"></div>

## diagonalise

`A = P D P⁻¹` made visual: every real-diagonalisable matrix has a basis — its
eigenvectors — in which it does nothing but *stretch* each axis. The unit
eigen-cell stretches by λ along each eigenvector, with no rotation or shear
(`diagonalise`).

```manic
{{#include ../../examples/diagonalise.manic}}
```

<div class="manic-video" data-video="ex-diagonalise"></div>

## rref

Gaussian elimination, animated: an augmented matrix `[A | b]` is reduced to
reduced row-echelon form one row operation at a time, the numbers transforming
in place until the left block is the identity and the last column is the
solution (`rref`).

```manic
{{#include ../../examples/rref.manic}}
```

<div class="manic-video" data-video="ex-rref"></div>

## projection

One idea, two faces: orthogonal **projection** drops a vector onto a subspace
(the shadow is the closest point, the error meets the space at a right angle),
and **least-squares** fits a line to data the same way — minimising the squared
residuals (`project`, `leastsquares`).

```manic
{{#include ../../examples/projection.manic}}
```

<div class="manic-video" data-video="ex-projection"></div>

## matrix

A bracketed matrix, rows/columns addressable via tags.

```manic
{{#include ../../examples/matrix.manic}}
```

<div class="manic-video" data-video="ex-matrix"></div>

## matrix_addition

Two matrices summed, cell by cell.

```manic
{{#include ../../examples/matrix_addition.manic}}
```

<div class="manic-video" data-video="ex-matrix_addition"></div>

## matrix_addition_plane

The same sum, laid out on a coordinate plane.

```manic
{{#include ../../examples/matrix_addition_plane.manic}}
```

<div class="manic-video" data-video="ex-matrix_addition_plane"></div>

## linear_transform

A 2x2 matrix shearing a grid + basis vectors.

```manic
{{#include ../../examples/linear_transform.manic}}
```

<div class="manic-video" data-video="ex-linear_transform"></div>

## table

A ruled table; cells, rows, columns, labels all addressable.

```manic
{{#include ../../examples/table.manic}}
```

<div class="manic-video" data-video="ex-table"></div>

## table_braces

A table annotated with braces.

```manic
{{#include ../../examples/table_braces.manic}}
```

<div class="manic-video" data-video="ex-table_braces"></div>
