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

## Variables, arithmetic, loops & macros

manic has a small computation layer. It runs *before* the animation is built —
every expression is evaluated to a plain number/point/name, so anywhere a
number or `(x, y)` goes you can write a formula instead.

```
let n  = 8;              // a numeric variable
let r  = 220;
let cx = 640;  let cy = 380;

for i in 0..n {          // i walks the integers 0, 1, ... n-1
  let a = 6.283185 * i / n;             // arithmetic: + - * / ^ and functions
  dot(p{i}, (cx + r*cos(a), cy + r*sin(a)));   // id interpolation: p{i} -> p0, p1, ...
  tag(p{i}, ring);
}
show(ring);              // the whole generated group, by tag
```

- **`let name = expr;`** binds a number. Bindings are visible to later
  statements; a `let` inside a loop/block is scoped to it.
- **Expressions** support arithmetic `+ - * / ^` and unary `-`, comparisons
  `< <= > >= == !=`, logic `&& ||` (all yielding `1`/`0`), parentheses, the
  constants `pi` / `e` / `tau`, and the functions `sin cos tan asin acos atan
  sinh cosh tanh exp ln log log10 log2 sqrt abs floor ceil round sign`.
  Multiplication is explicit (`3*i`, not `3i`). A bare word is a *variable* if
  bound, otherwise a literal name (a colour, easing, id, or tag).
- **`for v in a..b { … }`** repeats the body with `v = a, a+1, … b-1` (integers).
- **`if cond { … } else { … }`** (and `else if`) run a branch when `cond` is
  nonzero — the base case for recursion.
- **`def name(p1, p2, …) { … }`** defines a reusable macro; call it like any
  other statement. Params are numbers; a macro may call itself (recursion), so
  fractals and trees are a few lines. (Give a base case, or it stops at the
  recursion-depth guard.)
- **`bar{i}`** — id interpolation: `{expr}` glued into an identifier is replaced
  by its value, so each iteration/call makes a unique id (then `tag` them into a
  group to animate together). Glued only — `foo {` with a space is still a block.
- **Reductions** — `sum(i in a..b : expr)` aggregates `expr` over the range;
  also `prod`, `min`, `max`. Use in a `let`: `let area = sum(i in 0..n :
  f(i)*dx);`. This is how you compute a total in-language.

It's all additive: a program using none of these behaves exactly as before.

To show a computed number that **counts up on screen**, pair a reduction with a
`counter` (below): `counter(total, (x,y), 0, 3, "area = ", "")` then
`to(total, value, area)` tweens the readout from 0 to `area`.

---

## Constructors — the cast (t = 0)

Every entity has a unique **id** (its first argument) that later statements
address.

| call | draws |
|---|---|
| `text(id, (x,y), "str")` | text centred at `(x,y)`, mono, size 28 |
| `counter(id, (x,y), value, [decimals], ["prefix"], ["suffix"])` | a numeric readout; animate with `to(id, value, target)` so it counts live |
| `dot(id, (x,y), [r])` | small filled cyan dot, radius `r` (default 6) |
| `circle(id, (x,y), r)` | node: dark panel fill, glowing cyan ring |
| `rect(id, (x,y), w, h)` | rectangle, same node styling |
| `line(id, (x1,y1), (x2,y2))` | a straight line |
| `arrow(id, (x1,y1), (x2,y2))` | a line with an arrowhead at the second point |
| `brace(id, (x1,y1), (x2,y2), [depth])` | a curly brace spanning the two points, bulging `depth` px to one side (default 22; negative flips the side) |
| `bracelabel(id, (x1,y1), (x2,y2), "text", [depth])` (alias `bracetext`) | a brace with a text label centred just beyond its cusp; child `{id}.label` |

### Modifiers (apply to an existing entity, at t = 0)

Each takes the target id as the first argument.

| call | effect |
|---|---|
| `hidden(id)` | start invisible (reveal later with `show`) |
| `untraced(id)` | start with the stroke undrawn (reveal with `draw`) |
| `rot(id, deg)` | start rotated by `deg` degrees |
| `opacity(id, n)` | explicit starting opacity 0..1 |
| `color(id, name)` | fill / primary color |
| `outlined(id)` | outline only (no fill) |
| `filled(id)` | fill only (no outline) |
| `outline(id, name)` | outline color (and turn the outline on) |
| `hue(id, deg, [sat], [light])` | set the color from an HSL hue in degrees (sat 1.0, light 0.6 by default) — computable, so `hue(bar{i}, 360*i/n)` gives each looped entity its own color |
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
to(A, hue, 480, 2, linear);       // cycle colour around the wheel (needs hue set)
```

The animatable properties are `x`, `y`, `opacity`, `scale`, `angle`, `trace`,
`color`, `hue`, and `value`. `hue` travels around the colour wheel (set an
initial hue with the `hue` modifier first), so it cycles smoothly where `color`
would interpolate through grey. `value` drives a `counter`'s displayed number.

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
| `axes(id, (cx,cy), halfw, halfh, [unit])` | a coordinate cross with arrowheads on +x and +y; with `unit` (px per step) it also gets tick marks and integer labels. Children `{id}.x` / `{id}.y`; tags `{id}.ticks` / `{id}.labels` |
| `plane(id, (cx,cy), halfw, halfh, [unit])` (alias `numberplane`) | a NumberPlane: a faint grid every `unit` px (default 50) with brighter axes through the centre. Grid tagged `{id}.grid`; axes `{id}.x` / `{id}.y` |
| `complexplane(id, (cx,cy), halfw, halfh, [unit])` | a NumberPlane labelled with cyan `Re` / `Im` axes |
| `polarplane(id, (cx,cy), radius, [rings], [spokes])` | a PolarPlane: faint concentric rings (default 4) and radial spokes (default 12), tagged `{id}.grid` |
| `plot(id, (cx,cy), sx, sy, fn, [range])` | plot `fn`, mapped as `(cx + x·sx, cy − f(x)·sy)`, as a glowing polyline. `fn` is either a **named** function (below) or a **formula string** in `x` (alias `t`) — e.g. `"cos(x) + 0.5*cos(7*x)"` (manic's `FunctionGraph`). `range` is a scalar `domain` → `x ∈ [-domain, domain]` (default 6) **or** an explicit pair `(x0, x1)` for a one-sided range, e.g. `plot(g,(cx,cy),200,52,"x*x",(0,2.5))` |
| `vector(id, (cx,cy), (dx,dy), [color])` | an arrow from the origin to `(cx+dx, cy−dy)` (dy is up); default magenta |
| `numberline(id, (cx,cy), halfw, from, to, step)` | an axis with ticks and labels from `from` to `to` |
| `arrowfield(id, (cx,cy), halfw, halfh, field, [n])` | a grid of arrows sampling a named vector `field`, coloured by magnitude (cyan→lime→magenta); `n` arrows across |
| `matrix(id, "a b; c d", (cx,cy), [cellw], [cellh])` | a bracketed matrix (rows split by `;`, entries by space/comma); entry `{id}.r{i}c{j}`, tags `{id}.row{i}` / `{id}.col{j}` / `{id}.entries`, brackets `{id}.lbrack`/`{id}.rbrack` |
| `table(id, "a b; c d", (cx,cy), [cellw], [cellh], [col-labels], [row-labels])` (aliases `mathtable`/`decimaltable`/`integertable`) | a ruled grid of single-token entries; body cell `{id}.r{i}c{j}` (tags `{id}.row{i}` / `{id}.col{j}` / `{id}.entries`); optional header strings add a top label row (`{id}.collabel{j}`) / left label column (`{id}.rowlabel{i}`), tagged `{id}.labels`; grid lines `{id}.h{k}` / `{id}.v{k}`, tagged `{id}.hlines` / `{id}.vlines` / `{id}.lines` |
| `arc(id, (cx,cy), r, start, sweep)` | a circular arc line (angles in degrees) |
| `sector(id, (cx,cy), r, start, sweep)` | a filled pie slice |
| `annulus(id, (cx,cy), outer, inner)` | a filled ring between two radii |
| `pie(id, (cx,cy), r, n)` | a circle cut into `n` equal filled sectors, each addressable as `{id}0 … {id}{n-1}` (tag `id`) |

Named `plot` functions (`fn`): `sin`, `cos`, `tan`, `parabola` (`sq`, `square`),
`cubic` (`cube`), `line` (`id`, `identity`), `abs`, `exp`, `sqrt`, `log`
(`ln`), `recip` (`inv`), `gauss` (`bell`).

Formula strings accept the variable `x` (alias `t`); constants `pi`, `e`, `tau`;
operators `+ - * / ^` and unary `-`; and the functions `sin`, `cos`, `tan`,
`asin`, `acos`, `atan`, `sinh`, `cosh`, `tanh`, `exp`, `ln`/`log`, `log10`,
`log2`, `sqrt`, `abs`, `floor`, `ceil`, `round`, `sign`. Multiplication is
explicit (`7*x`, not `7x`). Example: `plot(f,(640,384),70,70,"cos(x) +
0.5*cos(7*x) + (1/7)*cos(14*x)", 7)`.

A `plot` curve renders instantly by default; declare it `untraced(id)` and use
`draw(id)` to trace it on.

`arrowfield` functions (`field`): `radial` (`source`/`out`), `sink`
(`attract`/`in`), `swirl` (`rotational`/`curl`), `saddle`, `wave`, `shear`,
`uniform` (`flow`), `spiral`.

A `matrix`'s rows and columns are tag groups, so you colour or highlight a
whole column with `recolor(m.col1, magenta)` (= `set_column_colors`) or a row
with `flash(m.row0, cyan)` (= `set_row_colors`). Entries are mono text (no
LaTeX yet — write `pi` or a literal `π`, not `\pi`).

---

## The algo kit

Data-structure & algorithm vocabulary. v1 centrepiece: **`graph`** (Manim's
`Graph` / `DiGraph`).

| call | draws |
|---|---|
| `graph(id, "v1 v2 …", "edges", layout, (cx,cy), scale, [radius])` | a graph of labelled circle nodes + edges |

- **vertices** — a whitespace-separated string of names → nodes `{id}.{name}`
  (each with a name label).
- **edges** — whitespace/comma-separated tokens: `a-b` (undirected line) or
  `a>b` (directed arrow), trimmed to node borders → `{id}.{a}-{b}`.
- **layout** — `circular`, `row`, or `grid`.
- Every entity is tagged `id`, `{id}.nodes`, and `{id}.edges`.
- Edges **reflow automatically**: `move(g.1, (x,y))` drags a vertex and its
  incident edges stretch to follow (trimmed to the borders).

```
graph(g, "a b c d e f", "a>b b>c c>d d>e e>f f>a a>d", circular, (640,384), 210);
hidden(g.nodes);   untraced(g.edges);   // broadcast at t=0
show(g.nodes);     draw(g.edges);       // reveal the whole group
flash(g.a, magenta);                    // address one node by id
```

## The geo kit (olympiad geometry)

High-level Euclidean constructions in the spirit of Asymptote's
`olympiad.asy` / `cse5.asy` — you write the *geometry*, not coordinates. Every
construction reads points **declared earlier** and is **dynamic**: it recomputes
as those points move, so `move(C, …)` drags a vertex and the circumcircle,
incircle, centroid, foot, angle mark, and all sides update live.

| call | makes |
|---|---|
| `point(id, (x,y), ["L"])` | a dot, optionally labelled `L` |
| `segment(id, a, b)` | a line joining points `a`,`b` (reflows) |
| `midpoint(id, a, b)` | midpoint of `a`,`b` |
| `centroid(id, a, b, c)` | triangle centroid |
| `circumcenter(id, a, b, c)` | centre of the circumscribed circle |
| `incenter(id, a, b, c)` | centre of the inscribed circle |
| `orthocenter(id, a, b, c)` | intersection of the altitudes |
| `foot(id, p, a, b)` | foot of the perpendicular from `p` to line `ab` |
| `meet(id, a, b, c, d)` | intersection of lines `ab` and `cd` |
| `circumcircle(id, a, b, c)` | circle through the three points |
| `incircle(id, a, b, c)` | circle inscribed in the triangle |
| `anglemark(id, a, b, c)` | an arc marking the angle at vertex `b` |
| `rightangle(id, a, b, c)` | a small square marking a right angle at `b` |

```
point(A, (380,560), "A");  point(B, (900,560), "B");  point(C, (640,140), "C");
segment(ab, A, B);  segment(bc, B, C);  segment(ca, C, A);
circumcircle(cc, A, B, C);   incircle(ic, A, B, C);   centroid(G, A, B, C);
foot(F, C, A, B);   segment(alt, C, F);   anglemark(angC, A, C, B);
```

## Banner & watermark (brand kit)

manic's own logo and mark (à la `ManimBanner`).

| call | makes |
|---|---|
| `banner(id, (cx,cy), [scale])` | the manic logo: a cyan circle + magenta square + lime triangle icon trio (`{id}.dot`/`{id}.sq`/`{id}.tri`, tag `{id}.icon`) and the "manic" wordmark (`{id}.word`) |
| `watermark(id, (x,y), ["text"])` | a small, glowing, **screen-fixed** mark that ignores camera moves and persists |

Animate it `create → expand → unwrite` like the reference banner:

```
banner(logo, (600, 360), 1.1);
untraced(logo.icon);  hidden(logo.word);
watermark(wm, (1120, 690), "manic // synthwave");

draw(logo.icon);      // create — trace the icons on (broadcasts over the trio)
show(logo.word);      // expand — reveal the wordmark
fade(logo.icon);  fade(logo.word);   // unwrite
```

## Groups & tag broadcast

Any verb or modifier whose **first argument names a tag** (rather than a single
entity) applies to *every* entity carrying that tag — in parallel for verbs.
So `draw(g.edges)`, `flash(g.nodes, cyan)`, `hidden(g.nodes)` operate on the
whole group. Individual members are still addressable by their dotted id
(`g.a`, `g.a-b`). This is what makes graphs, cells, and other multi-entity
groups practical to animate.

## Boolean shape ops

Combine two **fillable** shapes (circle, rect, polygon, filled sector/annulus)
into a new filled region:

| call | result |
|---|---|
| `union(id, a, b, [color])` | `a ∪ b` |
| `intersect(id, a, b, [color])` (alias `intersection`) | `a ∩ b` |
| `difference(id, a, b, [color])` (alias `subtract`) | `a − b` |
| `exclusion(id, a, b, [color])` (alias `xor`) | `a ⊕ b` (both, minus overlap) |

Operands `a` and `b` must be **declared before** the op — booleans read their
geometry at build time. The result is a `Region` entity (default color lime,
holes and multiple pieces handled) that you can `move` / `scale` / `rotate` /
`show` / `fade` as one shape.

```
rect(sq, (330, 300), 130, 130);   outlined(sq);
circle(cr, (400, 250), 78);       outlined(cr);
difference(bite, sq, cr, lime);   // the square with a circular bite removed
```

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
