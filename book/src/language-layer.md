# The language layer

Everything so far has been static text. manic also has a small **computation
layer** that runs *before* the animation — variables, arithmetic, loops, and
macros. It lets one rule draw a hundred shapes.

> These are resolved at build time. By the time the animation plays, they've
> expanded into plain calls — so they cost nothing at render.

## Variables — `let`

```manic
let r = 120;
let gap = r * 2 + 40;
circle(a, (cx - gap, cy), r);
circle(b, (cx + gap, cy), r);
```

Arithmetic is what you'd expect: `+ - * / ^`, parentheses, and functions like
`sin`, `cos`, `sqrt`. Constants `pi`, `tau`, `e` are built in, as are the canvas
vars `w`, `h`, `cx`, `cy`.

## Loops — `for`

```manic
for i in 0..5 {
  dot(p{i}, (200 + i*180, cy), 8);   // p0 … p4
}
```

`p{i}` is **id interpolation** — `{expr}` glued to a name makes each entity
unique. Use `i` in the body to compute positions, sizes, hues…

```manic
{{#include ../samples/loop.manic}}
```

**▶ See it play:**

<div class="manic-video" data-video="loop"></div>

## Conditionals — `if`

```manic
if depth > 0 {
  line(seg{k}, (x, y), (x2, y2));
}
```

## Macros — `def`

A `def` is a reusable rule. Its parameters are **numbers**; build ids inside with
interpolation. It can even call itself (recursion) — that's how the fractal tree
in the gallery is one page of code:

```manic
def branch(k, x, y, ang, len, depth) {
  if depth > 0 && len > 2 {
    let x2 = x + len*cos(ang);
    let y2 = y - len*sin(ang);
    line(seg{k}, (x, y), (x2, y2));
    branch(2*k,     x2, y2, ang + 0.4, len*0.72, depth - 1);
    branch(2*k + 1, x2, y2, ang - 0.4, len*0.72, depth - 1);
  }
}
branch(1, cx, h - 40, 1.5708, 150, 12);
```

## Reductions

Fold a range into one number with `sum`, `prod`, `min`, `max`:

```manic
let total = sum(i in 1..n : i);   // 1 + 2 + … + (n-1)
```

That's the whole language. The rest is **kits** — bundles of higher-level
figures → [Kits](kits.md).
