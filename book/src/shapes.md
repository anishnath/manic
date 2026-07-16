# Shapes — the cast

Everything on screen is an **entity** with a **name** (its first argument). You
declare shapes once; the name is how you address them later in the script.

## The six primitives

Each line below is the whole call — copy it and tweak the numbers.

| shape | write | draws |
|---|---|---|
| **circle** | `circle(sun, (cx, cy), 90);` | a circle, radius 90, at the centre |
| **rect** | `rect(box, (cx, cy), 200, 120);` | a rectangle 200 wide, 120 tall |
| **line** | `line(edge, (100, 100), (400, 300));` | a line from point to point |
| **arrow** | `arrow(v, (100, 400), (400, 400));` | a line with an arrowhead at the end |
| **dot** | `dot(p, (cx, cy), 8);` | a small filled dot, radius 8 |
| **text** | `text(cap, (cx, 640), "hello");` | a text label anchored at a point |

Plus a few composite helpers built from those: `polygon`, `arc`/`sector`, `brace`/
`bracelabel`, `caption` (a row of words), and `support(id, (cx,cy), [len], ["dir"])`
— the hatched wall / ceiling / floor for mechanics & textbook diagrams (`"dir"` is
the open side: `"down"` ceiling, `"up"` floor, `"left"`/`"right"` walls).

Points are `(x, y)` in pixels, **origin top-left, y increasing downward**. Use
`cx`, `cy`, `w`, `h` to stay canvas-independent.

```manic
{{#include ../samples/shapes.manic}}
```

**▶ See it play:**

<div class="manic-video" data-video="shapes"></div>

## Modifiers — style a shape at t = 0

A shape starts plain. **Modifiers** change how it looks *before* the animation
begins. They take the entity name first, then a value:

| modifier | effect | example |
|---|---|---|
| `color(id, c)` | fill / stroke colour | `color(sun, cyan);` |
| `stroke(id, w)` | line thickness | `stroke(sun, 4);` |
| `size(id, n)` | text size | `size(cap, 30);` |
| `glow(id, n)` | neon halo strength | `glow(sun, 8);` |
| `opacity(id, 0..1)` | transparency | `opacity(sun, 0.5);` |
| `filled(id)` / `outlined(id)` | turn fill / outline on | `filled(box);` |
| `hue(id, deg)` | colour by an angle (0–360) — for gradients & loops | `hue(seg, 200);` |
| `z(id, n)` | draw order (higher = on top) | `z(box, 5);` |
| `sticky(id)` | pin to the screen so it stays put through a `cam`/`zoom` (a HUD) | `sticky(caption);` |

And two that decide *how a shape first appears*:

| modifier | pairs with | gives |
|---|---|---|
| `hidden(id)` | `show(id)` | a **fade-in** |
| `untraced(id)` | `draw(id)` | a **draw-on** (pen tracing the outline) |

> **Colours are a fixed palette:** `fg`, `void`, `cyan`, `magenta`, `lime`,
> `dim`, `panel`. For a computed colour (say, one per item in a loop) use
> `hue(id, degrees)`. More in [Colour & style](colour.md).

## Naming things in a loop

When you make many shapes with a `for` loop, give each a unique name with
**interpolation** — `{expr}` glued to the name:

```manic
for i in 0..5 {
  dot(p{i}, (200 + i*180, cy), 8);   // p0, p1, p2, p3, p4
}
```

That's your cast. Now let's make it move → [Verbs](verbs.md).
