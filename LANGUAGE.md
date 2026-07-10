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
math kit flips y for you where it matters).

Statements fall into three groups:

- **Control / computation** — `let`, `for`, `if`, `def`, and macro calls. These
  are resolved **at build time** (see the computation layer below) and expand
  into the other two kinds; they produce nothing on their own.
- **Constructors** — build the cast at time 0 (shapes, modifiers, kit figures).
- **Timeline** — verbs and `par`/`seq`/`stagger` blocks, which play in order.

Constructors and timeline statements may appear in any order in the file — the
cast is gathered first, then the script runs — so you can reference an entity in
a beat written above its declaration.

---

## Program setup

| call | meaning |
|---|---|
| `title("...")` | window title + the masthead shown on every frame |
| `canvas(w, h)` | logical canvas size in pixels (default `1280, 720`). Origin `(0,0)` is top-left; x → right, y → down |
| `canvas("preset")` | pick a format instead of pixels: `"16:9"` (default), `"1080p"`, `"4k"`, `"square"` (1:1), `"portrait"` (9:16), `"4:3"` |
| `template("name")` | the overall look. `"plain"` (default) is a blank screen — background + your content only. `"terminal"` neon window chrome (border, dots, title, rule); `"paper"` ink on cream; `"blueprint"` white/cyan on navy. Each **retints** the palette (`cyan`/`fg`/`bg`…). Override at render with `--template <name>`. |
| `masthead("left", ["right"])` | your own header text in the top corners (shown by `terminal`). Empty by default — no engine branding is ever baked in. |

Put these first. (It's `canvas`, not `size` — `size` sets text size.)

**Canvas variables.** After `canvas`, four variables are predefined so you can
place things relative to the frame and stay canvas-independent: `w` (width),
`h` (height), `cx` (centre x = w/2), `cy` (centre y = h/2). Prefer these over
hard-coded pixels — then `canvas("square")` re-centres everything for free:

```
canvas("square");
text(title, (cx, cy), "Hello");       // always centred
dot(corner, (cx - w/4, cy - h/4));    // relative placement
```

---

## The computation layer (evaluated before the animation)

manic runs in two phases, and it helps to keep them separate:

1. **Computation layer** — variables, arithmetic, loops, conditionals, macros,
   reductions. Evaluated **once, at build time**, *before any frame is drawn*.
   It decides **what entities exist and where**. Everything here collapses to
   plain values, so it has no per-frame cost and **cannot refer to time**.
2. **Animation timeline** — verbs (`show`, `move`, `to`, …) that animate entity
   **properties over time**. This is the runtime part (see below).

> Rule of thumb: use the computation layer to *lay out* a scene; use the
> timeline to *animate* it. A `let` is a fixed build-time number — to make a
> number change on screen over time, use a `counter` + `to(id, value, …)`, not
> a `let`.

```
let n = 8;                 // a variable: a build-time number
let r = 220;

for i in 0..n {            // a loop: repeats the body for i = 0,1,...,n-1
  let a = tau * i / n;                          // arithmetic + a constant
  dot(p{i}, (cx + r*cos(a), cy + r*sin(a)));    // interpolation: p{i} -> p0,p1,...
  tag(p{i}, ring);
}
show(ring);                // animate the whole generated group by tag
```

### Values
Every expression evaluates to one of four things:
- **number** — the only thing arithmetic produces (booleans are numbers: `1`
  true, `0` false);
- **point** — an `(x, y)` pair, each component its own expression;
- **string** — `"..."`;
- **name** — a bare word that is *not* a bound variable: an entity id, colour,
  easing, or function name.

### Variable — `let name = expr;`
Binds `name` to the **number** that `expr` evaluates to; use it anywhere a
number or coordinate is expected. **Scope is lexical**: a top-level `let` is
visible to the statements after it; a `let` inside a `for` / `if` / block /
macro is confined to that body. Variables are **immutable** within a scope —
there is no reassignment; a later `let name = …` *shadows* the earlier one.
**Predefined:** `w`, `h`, `cx`, `cy` (from `canvas`) and the constants `pi`,
`e`, `tau` (a `let` of the same name shadows them).

### Expression & operators
Arithmetic `+ - * / ^` (`^` right-associative) and unary `-`; comparisons
`< <= > >= == !=` and logic `&& ||` (all yield `1`/`0`); parentheses; and the
functions `sin cos tan asin acos atan sinh cosh tanh exp ln log log10 log2 sqrt
abs floor ceil round sign`.

**Implicit multiplication** is allowed where it's unambiguous: a number or `)`
directly followed by a name or `(` multiplies — `2sx`, `3(x+1)`, `(a+b)c`,
`2pi` all mean what they look like. The one thing you *must* write with `*` is a
product of two variable names: `dx*sx`, because `dxsx` is a single identifier
(there's no boundary to split). Two number literals are never joined either, so
a missing comma like `(0 0)` stays a clear error.

### Loop — `for v in a..b { … }`
**Build-time repetition** (unrolling): expands the body once for each integer
`v` in `[a, b)` — i.e. `a, a+1, … b-1`. It is not a runtime loop; the body's
statements are generated before rendering.

### Conditional — `if cond { … } [else { … }]`
**Build-time branch**: keeps one arm's statements depending on `cond` (nonzero =
true). Chains with `else if`.

### Macro — `def name(p1, p2, …) { … }`
A named, parameterised **block of statements**. Calling `name(args)` **expands**
the body with each parameter bound to the corresponding argument number — a
macro *emits statements*, it is **not** a value-returning function. Parameters
are numbers. A macro **may call itself** (recursion), bounded by a depth guard,
so a self-recursive macro needs a base case via `if`.

### Reduction — `sum(v in a..b : expr)`
An **expression** (returns a number) that aggregates `expr` over the integer
range `[a, b)`; also `prod`, `min`, `max`. This is how you compute a total
in-language: `let area = sum(i in 0..n : f(i)*dx);`.

### Id interpolation — `name{expr}`
Builds an **identifier** by substituting the value of `{expr}` into it (glued,
no space — `foo {` with a space is still a block). Gives each loop iteration or
macro call a unique id; `tag` those into a group to animate together.

Everything here is additive: a program that uses none of it behaves exactly as
a plain list of calls. To show a computed number **counting up on screen**, pair
a reduction with a `counter`: `counter(total, (x,y), 0, 3, "area = ", "")` then
`to(total, value, area)` tweens the readout from 0 to `area`.

---

## Constructors — the cast (t = 0)

Every entity has a unique **id** (its first argument) that later statements
address.

| call | draws |
|---|---|
| `text(id, (x,y), "str")` | text centred at `(x,y)`, mono, size 28 |
| `counter(id, (x,y), value, [decimals], ["prefix"], ["suffix"])` | a numeric readout; animate with `to(id, value, target)` so it counts live |
| `caption(id, "some words", (x,y), [size], [color])` | lays words out in a centred row as `{id}.w0…` (tag `{id}.words`); animate with `karaoke`/`wordpop` |
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
| `cursor(id)` | give a text entity a `_` typewriter cursor (pairs with `type`/`trace`) |
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
| `transform(id, (ox,oy), a, b, c, d, [dur], [ease])` | apply the 2×2 matrix `[[a,b],[c,d]]` about origin `(ox,oy)` — broadcast over a tag to shear/rotate a whole grid + vectors (Manim `ApplyMatrix`) |
| `swap(a, b, [dur], [ease])` | animate two entities into each other's position (array form `swap(arr, i, j)` slides slot values & chains across a sort) |
| `karaoke(id, [delay], [color])` | highlight a `caption`'s words in sequence (lyrics-style) |
| `wordpop(id, [delay])` | pop a `caption`'s words in one at a time (TikTok-style; `hidden(id.words)` first) |
| `morph(a, b, [spin])` (constructor) + `to(a, morph, t, [dur])` | blend `a`'s outline into `b`'s (`t` 0→1). Optional `spin` degrees winds the blend (clockwise if positive). Outline-only; `a` becomes a stroked polyline (Manim `Transform`) |
| `copy(new, src)` (constructor) | duplicate entity `src` as `new` (standalone, no group tags) — copy then morph/move it while the original stays |

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

The animatable **properties** are `x`, `y`, `opacity` (alias `alpha`), `scale`,
`angle` (alias `rot`/`rotation`), `trace`, `color`, `hue`, and `value` (alias
`count`). `hue` travels around the colour wheel (set an initial hue with the
`hue` modifier first), so it cycles smoothly where `color` would interpolate
through grey; `value` drives a `counter`'s displayed number. Combine any of them
with `par`/`seq`/`stagger` and any easing — that is the full freedom to animate
however you like.

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

Blocks nest, and may contain verbs, `wait`, other blocks, and **control
constructs** (`for` / `if` / macro calls — which expand into verbs). They may
**not** contain constructors, `section`, or `mark`.

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
| `array(id, "5 2 8 1", (cx,cy), [cellw], [cellh])` | a row of value cells in fixed slot boxes |
| `pointer(id, arr, slot, [label])` | an index caret under a slot of `arr` |
| `stack(id, (x,y), [cw], [ch])` | an empty stack (bottom cell centre; grows up) |
| `queue(id, (x,y), [cw], [ch])` | an empty queue (front cell centre; grows right) |
| `caret(id, (x,y), "label", [dir])` | a labelled triangle marker (`dir` ∈ up/down/left/right) |
| `list(id, "3 8 5", (cx,cy), [kind], [cw], [ch])` | a linked list (`kind` ∈ singly/doubly/circular) |
| `hashmap(id, n, (cx,cy), [ew], [ch])` | `n` buckets (separate chaining) |

- **vertices** — a whitespace-separated string of names → nodes `{id}.{name}`
  (each with a name label).
- **edges** — whitespace/comma-separated tokens: `a-b` (undirected line) or
  `a>b` (directed arrow), trimmed to node borders → `{id}.{a}-{b}`. Add `:w` for
  a weight (`a-b:7`) — drawn as a midpoint label and read by `dijkstra`.
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

**`array`** lays out values in fixed slot boxes. Each value is a text cell
`{id}.c{k}` and each box is `{id}.box{k}` (tags `{id}.cells`, `{id}.boxes`). Two
verbs work by **slot index**:

| call | does |
|---|---|
| `compare(a, i, j, [color])` | flash the values *currently* in slots `i` and `j` |
| `swap(a, i, j, [dur])` | slide those two values into each other's slots |

`swap` is **stateful**: it updates the array's occupancy, so a whole chain of
swaps composes correctly (real in-place sorting) and `compare` always highlights
whatever value sits in a slot *now*:

```
array(a, "3 1 2", (cx, 360), 100, 100);
compare(a, 0, 1);  swap(a, 0, 1);   // 3 > 1 -> slide, now 1 3 2
compare(a, 1, 2);  swap(a, 1, 2);   // 3 > 2 -> slide, now 1 2 3
```

(`swap(a, b)` with two *entity ids* still does a plain position swap. The array
form is triggered when the first argument names an `array`.)

**`pointer`** drops a labelled index caret below a slot and `pointat(id, arr, slot)`
slides it to another (its `{id}.label` follows). Pointers track slot *positions*,
so they stay put as values swap through (the verb is `pointat`, not `point` —
geo's `point` owns that word):

```
pointer(lo, a, 0, "lo");   pointer(hi, a, 5, "hi");
par { pointat(lo, a, 1);  pointat(hi, a, 4); }   // both step inward
```

**`stack` / `queue`** are *dynamic*: `push(id, "v")` / `pop(id)` (stack, LIFO,
grows up) and `enqueue(id, "v")` / `dequeue(id)` (queue, FIFO, grows right) are
mutating verbs that add a cell and animate it in/out, tracking occupancy so a
chain of ops composes (`dequeue` also advances the cells behind the one that
left). Pair them with a `caret` — a rigid labelled marker you `move` to ride the
action point:

```
stack(st, (300, 500));
caret(top, (362, 500), "top", left);           // sits right of the column
push(st, "5");
par { push(st, "3");  move(top, (362, 436)); }  // caret rises with the top
pop(st);                                         // top value leaves
```

(Mutating verbs like `push`/`swap` may appear inside `par`/`seq`/`stagger` — block
steps lower in source order, so occupancy stays deterministic.)

**`list`** is a **linked list** with the classic node anatomy — framed boxes split
into compartments with pointer dots, a `head` pointer, and a `NULL` terminator (or
a wrap-to-head curve). `kind` ∈ `singly` (`[data│•]`, next + NULL), `doubly`
(`[•│data│•]`, next & prev, NULL both ends), `circular` (tail loops to head).
`insert(id, after, "v")` splices a node in **below** the gap and re-threads the
pointers (the row never shifts); `remove(id, i)` unlinks and re-points around it:

```
list(l, "3 8 5", (cx, cy), doubly);
insert(l, 1, "7");   // node 7 drops in below the 8-5 gap, pointers weave through
remove(l, 0);        // head unlinks; head pointer slides to the new first node
```

**`bfs` / `dfs`** run a traversal on a `graph` and animate it. They read the
graph's adjacency from its edges, so you just point them at a start node. Each
node cycles through colour states — discovered (cyan) → current (magenta) → done
(lime) — tree edges light up as they're taken, and two readouts track the
frontier (`queue:` for BFS, `stack:` for DFS) and the `visited:` order. BFS and
DFS are the *same* verb shape; only the frontier differs (queue vs stack):

```
graph(g, "a b c d e f g", "a-b a-c b-d b-e c-f c-g", circular, (cx,cy), 200);
bfs(g, a);   // level order: a, then b c, then d e f g
// ...recolor(g.nodes, panel) to reset, then:
dfs(g, a);   // depth first: dives down one branch before backtracking
```

Directed edges (`a>b`) are followed one way; undirected (`a-b`) both ways.

**Weighted edges + `dijkstra`.** Give an edge a weight with `a-b:7` (shown as a
midpoint label). `dijkstra(g, start)` runs single-source shortest paths: each
node carries a live distance label (`inf` → its final shortest distance), the
nearest unsettled node settles (magenta → lime) while its edges relax, and the
shortest-path-tree edges stay lit:

```
graph(g, "a b c d", "a-b:1 a-c:4 b-c:2 c-d:1", circular, (cx,cy), 200);
dijkstra(g, a);   // a=0, b=1, c=3, d=4
```

**`hashmap`** is separate chaining. `hashmap(id, n, (cx,cy))` draws `n` numbered
buckets; `put(id, "key", "val")` hashes the key (sum of its bytes, mod `n`) to a
bucket and chains the `key:val` entry on; `get(id, "key")` hashes, then scans
that bucket's chain, flashing entries until the key matches (lime) or the chain
runs out (bucket flashes magenta — a miss):

```
hashmap(ht, 5, (cx, cy));
put(ht, "cat", "7");   put(ht, "act", "9");   // anagrams collide -> same bucket, chained
get(ht, "act");        // scans bucket 2: cat, then act (found)
```

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
| `reflect(id, p, a, b)` | reflection of point `p` across line `ab` |
| `bisector(id, a, b, c)` | a point on the internal angle bisector at vertex `b` (draw `segment(b, id)`) |
| `rotpoint(id, p, center, deg)` | `p` rotated about `center` by `deg` degrees (e.g. an equilateral apex with `deg = -60`) |
| `between(id, a, b, t)` | the point a fraction `t` of the way from `a` to `b` (`t = 0.5` = midpoint) |
| `anglepoint(id, center, on, deg)` | a point on circle `(center, on)` at absolute angle `deg` |
| `fullline(id, a, b)` | a line through `a`,`b` extended across the frame (looks infinite) |
| `ellipse(id, (cx,cy), rx, ry, [deg])` | an ellipse outline, optionally rotated `deg` degrees |
| `parabola(id, (vx,vy), halfwidth, height)` | a parabola, vertex `(vx,vy)`, arms `height` px up at `±halfwidth` (negative opens down) |
| `hyperbola(id, (cx,cy), a, b, [range])` | a hyperbola, semi-axes `a`/`b`; two branches `{id}.r` / `{id}.l` (both tagged `id`) |
| `circle2(id, center, through)` | a circle centred at `center` passing through point `through` (radius = their distance) |
| `linecircle(id, a, b, center, through)` | the **two** points where line `ab` meets circle `(center, through)` → `{id}0`, `{id}1` |
| `circlecircle(id, o1, on1, o2, on2)` | the two intersection points of circles `(o1,on1)` and `(o2,on2)` → `{id}0`, `{id}1` |
| `tangent(id, from, center, through)` | the **two** tangent touch-points from external point `from` to circle `(center, through)` → `{id}0`, `{id}1` |
| `circumcircle(id, a, b, c)` | circle through the three points |
| `incircle(id, a, b, c)` | circle inscribed in the triangle |
| `anglemark(id, a, b, c)` | an arc marking the angle at vertex `b` |
| `rightangle(id, a, b, c)` | a small square marking a right angle at `b` |

Circles for `linecircle` / `circlecircle` / `tangent` are given as a **centre +
a point on the circle** (so the radius is dynamic too). Intersections and
tangents produce **two** points named `{id}0` and `{id}1`; draw or reference them
individually. All of these are dynamic — move an input and they recompute.

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
