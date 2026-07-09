# The manic language

A manic program is a list of **statements**. Each statement is a call —
a name, optional `(args)`, and either a `;` or a `{ block }`.

```
name(arg, arg, ...);     // a call, terminated by ;
name { ... }             // a block call (par / seq / stagger)
name(arg) { ... }        // a block call with args (stagger)
// comment to end of line
```

Arguments are:

| kind | example |
|---|---|
| number | `40`, `-5`, `2.5` |
| string | `"hello"` (escapes: `\n \t \" \\`) |
| name | `A`, `cyan`, `smooth` (an entity id, color, easing, or function) |
| point | `(300, 400)` — an `(x, y)` coordinate pair |

Coordinates are in pixels, origin **top-left**, y increases **downward** (the
math kit flips y for you where it matters). A statement may reference an entity
declared anywhere in the file — declare the cast and script the beats in
whatever order reads best.

There are two kinds of statement: **constructors**, which build the cast at
time 0, and **timeline** statements (verbs and blocks), which play in order.

---

## Program setup

| call | meaning |
|---|---|
| `title("...")` | window title + the masthead shown on every frame |
| `canvas(w, h)` | logical canvas size (default `1280, 720`) |

Put these first. (It's `canvas`, not `size` — `size` sets text size.)

---

## Constructors — the cast (t = 0)

Every entity has a unique **id** (its first argument) that later statements
address.

| call | draws |
|---|---|
| `text(id, (x,y), "str")` | text centred at `(x,y)`, mono, size 28 |
| `dot(id, (x,y), [r])` | small filled cyan dot, radius `r` (default 6) |
| `circle(id, (x,y), r)` | node: dark panel fill, glowing cyan ring |
| `rect(id, (x,y), w, h)` | rectangle, same node styling |
| `line(id, (x1,y1), (x2,y2))` | a straight line |
| `arrow(id, (x1,y1), (x2,y2))` | a line with an arrowhead at the second point |

### Modifiers (apply to an existing entity, at t = 0)

Each takes the target id as the first argument.

| call | effect |
|---|---|
| `hidden(id)` | start invisible (reveal later with `show`) |
| `untraced(id)` | start with the stroke undrawn (reveal with `draw`) |
| `rot(id, deg)` | start rotated by `deg` degrees |
| `opacity(id, n)` | explicit starting opacity 0..1 |
| `color(id, name)` | fill / primary color |
| `outline(id, name)` | outline color (and turn the outline on) |
| `size(id, n)` | text size (text entities only) |
| `stroke(id, n)` | stroke / outline width in px |
| `glow(id, n)` | neon halo intensity (0 = crisp, 1 = default) |
| `z(id, n)` | draw order (higher = on top) |
| `tag(id, name)` | group tag (for your own bookkeeping) |
| `bold(id)` | use the bold mono font |
| `display(id)` | use the heavy display font (headlines) |
| `label(id, "str")` | attach a bold-mono label that rides on the entity |

---

## Timeline — the script (verbs)

Verbs play **in source order**, each after the previous finishes. Motion verbs
take an optional trailing **duration** (seconds) and **easing** name:
`move(A, (x,y), 0.6, smooth)`.

| call | animates |
|---|---|
| `show(id, [dur])` | fade in |
| `fade(id, [dur])` | fade out |
| `move(id, target, [dur], [ease])` | move to a point **or another entity's position** |
| `shift(id, (dx,dy), [dur], [ease])` | move by a delta |
| `grow(id, target, [dur], [ease])` | move a line/arrow's endpoint (draws or retargets it) |
| `draw(id, [dur])` | trace a stroke on (declare `untraced` first) |
| `erase(id, [dur])` | trace a stroke off |
| `type(id, [dur])` | typewriter-reveal a text entity |
| `say(id, "str", [dur])` | crossfade a text entity to new content |
| `recolor(id, name, [dur])` | animate the fill color permanently |
| `flash(id, [name])` | flash to a color and auto-restore (default magenta) |
| `pulse(id, [dur])` | quick grow-and-settle attention pulse |
| `shake(id, [dur])` | horizontal shake, returns to origin |
| `scale(id, factor, [dur], [ease])` | animate uniform scale |
| `rotate(id, degrees, [dur], [ease])` | rotate to an absolute angle |
| `spin(id, degrees, [dur], [ease])` | rotate *by* a relative angle |
| `cam((x,y), [dur], [ease])` | pan the camera centre |
| `zoom(factor, [dur], [ease])` | zoom the camera (1.0 = whole canvas) |

`move`/`grow` accept an entity id as the target (`move(A, B)` moves A to B's
position); everything else takes a literal `(x, y)`.

### Animate anything — `to` / `set`

The named verbs above are ergonomic shortcuts. When you want to animate a
property directly — or one we didn't pre-name — use the general verb:

```
to(id, property, value, [dur], [ease])     // `set` is an alias

to(A, opacity, 0.3);              // fade to 30%
to(A, x, 300, 0.8, overshoot);    // slide the x-coordinate only
to(A, y, 120);
to(A, scale, 1.5, 0.6, bounce);
to(A, angle, 90);                 // rotate to 90°
to(A, color, magenta, 0.5);
to(A, trace, 0.5);                // half-draw a stroke
```

Animatable **properties**: `x`, `y`, `opacity` (`alpha`), `scale`, `angle`
(`rot`), `trace`, `color`. Combine any of them with `par`/`seq`/`stagger` and
any easing — that's the full freedom to animate however you like.

---

## Timeline — structure

| call | meaning |
|---|---|
| `wait(secs)` / `beat(secs)` | leave a gap (narration room); advances the cursor |
| `section("Title")` | a neon banner card + a jump marker (keys 1–9 in preview) |
| `mark("name")` | a named beat marker exported to `markers.json` |
| `par { ... }` | run the inner beats **at the same time** (duration = longest) |
| `seq { ... }` | run the inner beats **one after another** |
| `stagger(d) { ... }` | run in parallel, each starting `d` seconds after the previous |

Blocks nest, and may contain verbs, `wait`, and other blocks — but not
constructors, `section`, or `mark`.

```
par {
  show(v1, 0.4);
  pulse(v1);
}
stagger(0.08) {
  show(a);
  show(b);
  show(c);
}
```

---

## The math kit

Compositions for mathematical figures. `id` is a group id; some create child
entities named `{id}.x`, `{id}.tN`, etc.

| call | draws |
|---|---|
| `axes(id, (cx,cy), halfw, halfh)` | a coordinate cross with arrowheads on +x and +y |
| `plot(id, (cx,cy), sx, sy, fn, [domain])` | the named function `fn` over `x ∈ [-domain, domain]` (default 6), mapped as `(cx + x·sx, cy − f(x)·sy)`, as a glowing polyline |
| `vector(id, (cx,cy), (dx,dy), [color])` | an arrow from the origin to `(cx+dx, cy−dy)` (dy is up); default magenta |
| `numberline(id, (cx,cy), halfw, from, to, step)` | an axis with ticks and labels from `from` to `to` |
| `arc(id, (cx,cy), r, start, sweep)` | a circular arc line (angles in degrees) |
| `sector(id, (cx,cy), r, start, sweep)` | a filled pie slice |
| `annulus(id, (cx,cy), outer, inner)` | a filled ring between two radii |
| `pie(id, (cx,cy), r, n)` | a circle cut into `n` equal filled sectors, each addressable as `{id}0 … {id}{n-1}` (tag `id`) |

`plot` functions (`fn`): `sin`, `cos`, `tan`, `parabola` (`sq`, `square`),
`cubic` (`cube`), `line` (`id`, `identity`), `abs`, `exp`, `sqrt`, `log`
(`ln`), `recip` (`inv`), `gauss` (`bell`).

A `plot` curve renders instantly by default; declare it `untraced(id)` and use
`draw(id)` to trace it on.

---

## Colors

`fg` (foreground / `white`) · `void` (`bg`) · `cyan` (`blue`) · `magenta`
(`pink`, `accent`, `red`) · `lime` (`green`) · `dim` (`gray`, `grey`) ·
`panel`.

## Easings

`linear` · `smooth` (`inout`, the default) · `in` · `out` · `overshoot`
(`back`) · `bounce` · `elastic` (`spring`).

---

## A complete example

```
// examples/sine_wave.manic
title("The Sine Wave");
canvas(1280, 720);

// cast
axes(ax, (640, 380), 520, 240);
text(xlab, (1180, 410), "x");  color(xlab, dim);  size(xlab, 22);
text(ylab, (665, 152), "y");   color(ylab, dim);  size(ylab, 22);
plot(wave, (640, 380), 78, 120, sin, 6.6);  untraced(wave);
vector(v1, (640, 380), (122, 108));  hidden(v1);
text(head, (640, 118), "y = sin(x)");  display(head);  color(head, cyan);  size(head, 40);  hidden(head);
text(cap, (640, 662), "");  color(cap, dim);  size(cap, 22);

// script
show(head, 0.5);
say(cap, "a coordinate frame on the void");
draw(wave, 1.7);
say(cap, "y = sin(x), traced on");
wait(0.6);

section("Vectors");
say(cap, "a vector from the origin");
par {
  show(v1, 0.4);
  pulse(v1);
}
wait(1.2);
```

Run it:

```sh
manic examples/sine_wave.manic                 # live preview
manic examples/sine_wave.manic --record out    # → out/out.mp4
manic check examples/sine_wave.manic           # parse + report errors
```

## Errors

`manic check FILE` parses without opening a window and points at the exact
line and column:

```
error: unknown function `sine` (try: sin, cos, tan, parabola, cubic, …)
   --> line 8, col 30
    |
  8 | plot(wave, (640, 380), 78, 120, sine, 6.6);
    |                              ^^^^
```
