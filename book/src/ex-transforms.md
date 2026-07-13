# Transforms & morphing

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

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
