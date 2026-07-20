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
| string | `"hello"` — keeps backslashes verbatim so **LaTeX works directly** (`"\frac{1}{2}"`, `"\theta"`); only `\"` (a literal quote) and `\\` are special. **`\n` is a hard line break** in `text`/`caption` (text also auto-wraps if you `wrap(id,w)`). Backticks `` `...` `` are also raw and additionally let the LaTeX contain a `"`. |
| name | `A`, `cyan`, `smooth` (an entity id, color, easing, or function) |
| point | `(300, 400)` — an `(x, y)` coordinate pair |
| 3D point | `(2, -1, 3)` — an `(x, y, z)` coordinate triple |

Coordinates are in pixels, origin **top-left**, y increases **downward** (the
math kit flips y for you where it matters).

The 3D kit uses a separate right-handed, **Z-up** world measured in logical
units: x/y form the ground plane and +z points upward. A `camera3` projects that
world into the same canvas; ordinary 2D entities and chrome draw over it.

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
| `canvas("preset")` | pick a format instead of pixels: `"16:9"` (default), `"1080p"`, `"4k"`, `"square"` (1:1), `"portrait"` (9:16), `"4:5"` (feed), `"4:3"` |
| `template("name")` | the overall look. `"mono"` is the **default** when omitted: black-and-white editorial on near-black with subtle glow (aliases `monochrome`, `blackwhite`, `black-white`, `bw`). `"plain"` keeps the original neon palette; `"terminal"` adds neon window chrome; `"paper"` is ink on cream; `"blueprint"` is white/cyan on navy; `"shorts"` is a restrained colour Creator surface. Each remaps every named semantic colour (`cyan`/`magenta`/`lime`/`fg`/`dim`/`panel`/…). Bespoke `hue(...)` colours pass through. Override the DSL choice for one render with `--template <name>`. |
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

Override that logical canvas for one check, preview, still, or recording with
`--canvas portrait|4:5|square|16:9|WIDTHxHEIGHT`. The override is applied before
the computation layer, so the responsive variables and `if h > w { ... }`
layout branches see the requested format. Without the flag, `canvas(...)`
behaves exactly as authored.

For a publishing pass, `manic check FILE.manic --canvas all` rebuilds portrait,
4:5 feed, square, and 16:9 landscape and checks the settled state of each named
stage. It reports canvas overflow, Creator safe-area overflow, substantial
content overlap, and unreadably small text/notation with the format, stage,
time, entity, and a suggested fix. Ordinary `manic check FILE.manic` remains a
parse-and-timeline validation only.

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
Every expression evaluates to one of five things:
- **number** — the only thing arithmetic produces (booleans are numbers: `1`
  true, `0` false);
- **point** — an `(x, y)` pair, each component its own expression;
- **3D point** — an `(x, y, z)` triple, each component its own expression;
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
| `parameter(id, (x,y), initial, min, max, ["label"], [decimals])` | a visible bounded creator value: numeric readout + native track/dot widget. Animate its `value`; the range clamps the journey. All widget parts carry tag `{id}.widget` |
| `bind(parameter, target, property, "formula")` | connect live parameter `p` to `x`, `y`, `opacity`, `scale`, `angle`, `hue`, `value`, `trace`, or a plot `formula`. A plot formula also has coordinate `x` |
| `bind(parameter, target, property, from, to)` | linearly map the parameter's declared min/max to responsive output endpoints; ideal for positions using `w`/`h`/`cx`/`cy` |
| `caption(id, "some words", (x,y), [size], [color])` | lays words out in a centred row as `{id}.w0…` (tagged bare `{id}` + `{id}.words`); `show`/`draw`/`hidden(id)` broadcast over the whole caption, or animate with `karaoke`/`wordpop` |
| `dot(id, (x,y), [r])` | small filled cyan dot, radius `r` (default 6) |
| `circle(id, (x,y), r)` | node: dark panel fill, glowing cyan ring |
| `rect(id, (x,y), w, h)` | rectangle, same node styling |
| `particles(id, container, count, [radius], [seed], ["layout"])` | a deterministic group of small dots inside a `circle` or `rect`; `layout` is `"random"` (default), `"grid"` (rectangle), or `"ring"` (circle). Children are `{id}.p0…` and the bare id addresses the group. The id supplies the meaning—`bubbles`, `dust`, `stars`, `data`, etc. |
| `image(id, (x,y), "path", [w], [h])` | a **raster image** (PNG/JPG) from a file path, centred at `(x,y)`, `w`×`h` px (default 300 square; `h` defaults to `w`). Loaded once at render start; animates like any entity (`show`/`move`/`fade`/`spin`/…). A missing file draws a crossed placeholder box. (Engine-only — the browser editor knows the builtin but won't preview the raster.) |
| `equation(id, (x,y), \`latex\`, [size])` | typeset a **LaTeX math** string (real fractions, roots, exponents, Greek, big operators — KaTeX-grade, via RaTeX) centred at `(x,y)`; `size` is the em height in px (default 48). Put the LaTeX in **backticks** (a raw string) so `\`-commands survive: `` equation(q, (cx,cy), `x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}`) ``. Color individual terms with standard LaTeX and Manic palette names: `` `\textcolor{magenta}{\mathrm{slope}}=\textcolor{cyan}{x}` ``; semantic colors follow the active template and uncolored terms use its foreground. Ordinary single-color equations still follow `color`/`recolor`. It's an image → `show`/`fade`/`move`/`scale` animate it, but not `draw` (trace); use `rewrite` below for a continuous step-by-step derivation. Fonts are baked in (self-contained). **Inline shorthand:** wrap math in `` `$…$` `` inside any `text`/`caption`/kit label and it auto-typesets, taking the entity colour — no `equation` call. Works for a WHOLE label (`` text(l,(x,y),`$E=mc^2$`) ``, `` option(q,`$\tfrac12$`) ``, `` point(A,(x,y),`$\alpha$`) ``) **and for MIXED text+math on one line** (`` text(t,(x,y),`The area is $\pi r^2$ units`) ``) — plain words and inline formulas baseline-aligned, and **mixed lines wrap** at word boundaries (math stays inline). Plain strings (no `$`) are unchanged; a literal `$` is `\$`. (Inline math is an image: `show`/`fade`/`move` animate it, not typewriter/`trace`.) |
| `line(id, (x1,y1), (x2,y2))` | a straight line |
| `link(id, from, to, [bend])` | a line that stays attached to two moving entities; it meets circle/rectangle boundaries automatically. `bend=0` is straight, positive/negative values bow to opposite sides. |
| `support(id, (cx,cy), [len], ["dir"])` | a **hatched fixed support** (wall / ceiling / floor) for mechanics diagrams — a baseline `{id}.line` + diagonal hatch ticks `{id}.tick{i}`. `"dir"` is the OPEN side: `"down"` (ceiling, default), `"up"` (floor), `"left"`/`"right"` (walls). Tagged bare `{id}` + `{id}.parts`. Pairs with `template("paper")` for a textbook look |
| `polygon(id, (x1,y1), (x2,y2), (x3,y3), …, [color])` | a filled polygon through ≥ 3 points (a trailing colour word is optional). Filled with a matching outline; drop `opacity(id, 0.2)` for a translucent region, or `outline(id)` for edges only. Tagged `id`. |
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
| `sticky(id)` | pin to screen coordinates so it stays fixed while the camera `cam`/`zoom`s (a HUD overlay — captions, counters, readouts). Broadcasts over a tag |
| `rot(id, deg)` | start rotated by `deg` degrees |
| `opacity(id, n)` | explicit starting opacity 0..1 |
| `color(id, name)` | fill / primary color |
| `outlined(id)` | outline only (no fill) |
| `filled(id)` | fill only (no outline) |
| `outline(id, name)` | outline color (and turn the outline on) |
| `hue(id, deg, [sat], [light])` | set the color from an HSL hue in degrees (sat 1.0, light 0.6 by default) — computable, so `hue(bar{i}, 360*i/n)` gives each looped entity its own color |
| `size(id, n)` | text size (text entities only) |
| `stroke(id, n)` | stroke / outline width in px |
| `dashed(id, [dash], [gap])` | repeat a dash/gap pattern on a path-like entity (defaults 16/10 px); works on plots, lines, links, arrows, curves, splines, coils, and arcs |
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
| `travel(id, path, [dur], [ease])` | move one persistent entity once along an existing line, arrow, curve, plot, spline, or arc; it stops and holds at the endpoint |
| `wander(id, [dur])` | gently move a `particles` group while keeping every dot inside its original circle/rectangle; seeded and deterministic |
| `arrange(id, container, ["random|grid|ring"], [dur], [ease])` | move one persistent particle set into a deterministic layout inside `container`; random layouts use independent organic curved routes, a larger rectangle creates expansion, `grid → random → grid` gives exact reversal, and a circle plus `ring` gives a radial endpoint |
| `flow(path, [dur])` | send one luminous emphasis pulse over a line, arrow, curve, spline, arc, or `link` |
| `draw(id, [dur])` | trace a stroke on (declare `untraced` first) |
| `erase(id, [dur])` | trace a stroke off |
| `type(id, [dur])` | typewriter-reveal a text entity |
| `say(id, "str", [dur])` | crossfade a text entity to new content |
| `` rewrite(id, `latex`, [dur], [ease]) `` | smoothly transform an existing `equation` into the next **authored** LaTeX state. Equal RaTeX glyphs/rules retain identity and move; additions/removals enter or leave locally; the settled frame is the exact target LaTeX. Manic animates the math you provide—it does not solve or verify it. Repeated calls keep the same id, semantic `\textcolor` roles, deterministic seeking, and a stable screen-aware scale. If a visual item cannot be matched safely, that item crossfades; existing equation behavior is unchanged unless `rewrite` is used. Example: `` rewrite(work, `x^2+2x+1=(x+1)^2`, 0.9, smooth) ``. |
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
| `cycle(a, b, c, …, [dur], [arc], [ease])` | move each entity into the next one's position and the last into the first one's position (Manim `CyclicReplace`). `arc` is degrees and defaults to 90; use `0` for straight paths. Repeated cycles compose statefully. |
| `karaoke(id, [delay], [color])` | highlight a `caption`'s words in sequence (lyrics-style) |
| `wordpop(id, [delay])` | pop a `caption`'s words in one at a time (TikTok-style; `hidden(id.words)` first) |
| `morph(a, b, [spin])` (constructor) + `to(a, morph, t, [dur])` | blend `a`'s outline into `b`'s (`t` 0→1). Optional `spin` degrees winds the blend (clockwise if positive). Open paths remain open and closed outlines remain closed, avoiding a false diagonal chord during graph/line transforms. Outline-only; `a` becomes a stroked polyline (Manim `Transform`) |
| `copy(new, src)` (constructor) | duplicate entity `src` as `new` (standalone, no group tags) — copy then morph/move it while the original stays |

`move`/`grow` accept an entity id as the target (`move(A, B)` moves A to B's
position); everything else takes a literal `(x, y)`.

### Contained motion, rearrangement, and live connections

A compact generic vocabulary replaces the common pattern of hand-placing and
hand-moving dozens of dots. None of these words is chemistry-specific:

```manic
circle(glass, (400,300), 100);
particles(bubbles, glass, 24, 5, 7);

rect(tank, (750,300), 180, 160);
particles(data, tank, 18, 4, 19, "grid");

link(pipe, glass, tank, 35);
untraced(pipe);

par {
  wander(bubbles, 6);
  arrange(data, tank, "random", 3, smooth);
  seq { draw(pipe, 0.8); flow(pipe, 1.2); }
}

dot(marker, (400,300), 6);
travel(marker, pipe, 1.2, smooth); // moves the dot and leaves it at the end
```

The optional seed makes placement and motion repeat exactly in preview, stills,
and recordings. `arrange` preserves every child id while changing its layout or
container. Its random transition gives each particle a stable curved route, so
the result stays deterministic without looking like one synchronized straight
tween; a later `arrange(...,"grid")` can still reconstruct the exact ordered
state. A circular target plus `"ring"` gives the same particles a radial final
layout. `wander` occupies its requested duration, so put it in `par` with the
story beats it should accompany. A `link` follows `move`d endpoints; `flow` is
transient emphasis and does not alter the path itself, while `travel` moves a
real entity once along that path and stops at its endpoint.

### Parameter journeys

Declare one meaningful scalar, connect it to the persistent world once, then
animate only that value inside named steps:

```manic
parameter(a, (cx,120), -1.2, -1.5, 1.5, "a", 2);
plot(curve, (cx,500), 90, 45, "x*x", (-3,3));
counter(result, (cx,800), 0, 2, "a² = ", "");

bind(a, curve, formula, "p*x*x");
bind(a, result, value, "p*p");

step("flatten") { to(a, value, 0, 2, smooth); }
step("opens-up") { to(a, value, 1.25, 2, smooth); }
```

Formula bindings use `p` for the live parameter. A bound plot uses both `x`
(plot coordinate) and `p`; its existing tangent, normal, slope, area, integral,
and moving-mark views update from the new function automatically. Range
bindings map the parameter min/max onto two ordinary numeric expressions, so
`bind(a, point, x, w*0.2, w*0.8)` stays responsive.

Bindings are pure per-frame connections: seeking and recording are
deterministic. They target existing 2-D properties and formula plots; they do
not silently rerun expensive build-time constructors or change generated
entity counts. See `examples/parameter-journeys.manic`.

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
| `step("name") { ... }` | a named reactive world transition and first-class story stage: children start together, unmentioned entities persist, and the start is exported as a marker |
| `par { ... }` | run the inner beats **at the same time** (duration = longest) |
| `seq { ... }` | run the inner beats **one after another** |
| `stagger(d) { ... }` | run in parallel, each starting `d` seconds after the previous |

`step` is the story-level form of `par`: give the world's next state a unique,
readable name and place its changes inside. Its duration is the longest child.
Use a nested `seq` when part of the step needs internal order; named steps stay
top-level so their exported timestamps remain unambiguous.

```manic
step("explain") {
  rewrite(work, `f'(x)=2x`, 0.9, smooth);
  to(tangent, x, 2.5, 2.0, smooth);
  to(slopeValue, x, 2.5, 2.0, smooth);
  say(caption, "The formula, tangent and readout change together.");
}
```

Stage names are also creator workflow controls—no timestamps need to be copied:

```sh
manic stages examples/reactive-world.manic
manic examples/reactive-world.manic --stage find-the-flat-point
manic examples/reactive-world.manic --stage see-the-derivative --record out-derivative
manic examples/reactive-world.manic --from-stage measure-slope --to-stage takeaway --record out-arc
```

`manic stages` reports each start, end, and duration. A stage extends until the
next `step`, so an authored `wait` after its transition is included as reading
time. `--to-stage` is inclusive. In live preview the selected range controls
restart and scrubbing, while the stage strip and keys `1`–`9` jump between its
visible stages. Named ranges cannot be mixed with numeric `--from`/`--to`.

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

### Generic Timing v2 — named phases for any scene

`timing` is not limited to quizzes. With a fresh id it creates a format-neutral
clock whose phase names are yours:

| call | meaning |
|---|---|
| `timing(id, [(x,y)], "intro=1 demo=6 finish=1")` | declare exact named phase durations and create a native timer; `duration=6` is shorthand for one `main` phase |
| `timed(id) { ... }` | schedule the clock and its `during` blocks together |
| `during("phase") { ... }` | place one block at that phase's absolute start; source order does not affect timing, short blocks are padded, overruns/duplicates/unknown phases error |
| `run(id)` | timer-only playback when no authored phase blocks are needed; generic exact clocks reject a competing `run(id,dur)` |

Restyle the clock with the same `timerstyle` vocabulary documented in the
Creator kit: `ring`, `bar`, `number`, `segments`, `ticks`, `pulse`, or `none`.

```manic
canvas("16:9");
text(heading, (cx, 100), "ONE CLOCK · THREE PHASES"); hidden(heading);
circle(ball, (260, 420), 48);
timing(clock, (1160, 80), "intro=1 motion=4 finish=1");
timerstyle(clock, "look=ticks direction=fill label=SCENE finish=hold");
timed(clock) {
  during("intro")  { show(heading, 0.5); }
  during("motion") { move(ball, (900, 420), 3.5); }
  during("finish") { pulse(ball); }
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
| `tangent(id, curve, x, [len])` | the tangent line to a plotted `curve` at `x` (in the curve's own units), with a contact dot — one entity. The slope is read from the function itself, so it stays correct as `x` moves: animate it with `to(id, x, target, [dur])` to slide the tangent along. `len` is the on-screen segment length in px (default 120). At a corner/asymptote the slope is undefined and only the dot draws. (`tangent` is overloaded: with three **name** args, `tangent(id, p, c, thru)`, it's the geometry construction — tangent points from an external point to a circle.) |
| `normal(id, curve, x, [len])` | the normal (perpendicular) line + contact dot to a plotted `curve` at `x`. Animatable exactly like `tangent` (`to(id, x, …)`). |
| `slope(id, curve, x, [(dx,dy)])` | a live number showing the **slope** of a plotted `curve` at `x`, riding just off the point (offset `(dx,dy)` px). Animate `to(id, x, …)` and the number climbs/falls with the curve. |
| `area(id, curve, a, b, [n])` | the filled (translucent) region under a plotted `curve` from `a` to `b`, sampled with `n` strips (default 60). To sweep it open, start collapsed (`area(r, f, 1, 1)`) and animate the right bound `to(r, x, b, [dur])`. |
| `integral(id, curve, a, b, [(px,py)])` | a live **number** showing the definite integral of a plotted `curve` from `a` to `b`, pinned at screen `(px,py)`. Animate `to(id, x, b, …)` in step with an `area` sweep and it climbs to the true integral. |
| `roots(id, curve, [color])` | a dot at every place a plotted `curve` crosses zero (found by scanning its domain for sign changes + bisection). Dots are `{id}0`, `{id}1`, … tagged `id`. |
| `newton(id, curve, x0, [steps])` | Newton's method from starting guess `x0`, drawn as the classic zig-zag (curve → down each tangent to the x-axis → back up to the curve → …), converging on a root. `steps` default 6. Declare `untraced(id)` + `draw(id, dur)` to watch the guesses walk to the root. |
| `deriv(id, curve, [color])` | the derivative `f'` of a plotted `curve`, measured numerically and drawn as its own curve on the same axes. It's itself a graph, so `tangent`/`slope`/`area` work on it too. |
| `accum(id, curve, [a], [color])` | the accumulation function `F(x) = ∫ₐˣ f` (default `a` = the curve's left edge), drawn as its own curve. By the Fundamental Theorem `F' = f`, so `deriv(accum(f))` traces back onto `f` (and a `tangent` on it reads back `f(x)`). |
| `extrema(id, curve, [color])` | dots at a plotted `curve`'s maxima and minima (its critical points, where the slope is 0). Children `{id}0…` tagged `id`. |
| `inflections(id, curve, [color])` | dots where a plotted `curve` changes concavity (its inflection points, where `f''` is 0). Children `{id}0…` tagged `id`. |
| `band(id, top, bottom, [color])` | the filled (translucent) region between two plotted curves over the x-range they share — the area between two curves. |
| `taylor(id, curve, a, n, [color])` | the degree-`n` Taylor polynomial of a plotted `curve` about `x = a`, drawn as its own curve. Reveal `taylor(…,1)`, `taylor(…,3)`, `taylor(…,5)` in turn to watch it hug the curve over a widening interval. Coefficients are numerical, so keep `n` modest (≤ ~8). |
| `limit(id, curve, a, [color])` | visualise `lim(x→a) f(x)`: an open circle at the value `L` the curve approaches (found from both sides), guides to the axes, the value, and a dot that rides the curve — slide it in with `to(id, x, a, dur)`. Works at a removable hole (`sin(x)/x` at 0). Use **`a = inf`** (or `-inf`) for a limit at infinity — it auto-detects and draws the horizontal asymptote `y = L`. |
| `spline(id, p0, p1, …)` | a smooth curve (Catmull-Rom) passing through every given point — manic's "draw a smooth curve through these data points." Knot dots are `{id}.k0`, … (tagged `{id}.knots`). `untraced(id)` + `draw(id, dur)` traces it on. |
| `trajectory(id, "dx/dt", "dy/dt", (x0,y0), (cx,cy), scale, [steps])` | the path a point follows under the ODE system `dx/dt = fx(x,y)`, `dy/dt = fy(x,y)`, integrated (RK4) from math point `(x0,y0)` and drawn as `(cx + x·scale, cy − y·scale)` — orbits, spirals, phase portraits. For `dy/dx = f(x,y)`, pass `"1"` and `"f(x,y)"`. `untraced(id)` + `draw(id, dur)` flows the point along it. |
| `vector(id, (cx,cy), (dx,dy), [color])` | an arrow from the origin to `(cx+dx, cy−dy)` (dy is up); default magenta |
| `linmap(id, (cx,cy), unit, a, b, c, d, [span])` | the 2×2 matrix `[[a,b],[c,d]]` applied to the plane (math y-up, `unit` px per unit): a faint identity grid under the deformed (cyan) grid, with basis î (gold) and ĵ (magenta) landing on the columns `(a,c)`, `(b,d)`. Tagged `id`. |
| `determinant(id, (cx,cy), unit, a, b, c, d, [color])` | the unit square and its image under the matrix (a filled parallelogram), labelled **area = det**. Fill flips colour when det < 0; collapses to a line at det = 0. |
| `eigen(id, (cx,cy), unit, a, b, c, d, [color])` | the matrix's **real eigenvector directions** as lines through the origin (invariant — only stretch by the eigenvalue λ, shown). Complex eigenvalues leave a short note. |
| `linsolve(id, (cx,cy), unit, a, b, c, d, e, f, [span])` | the **row picture** of `Ax=b`: `a·x+b·y=e` (cyan) and `c·x+d·y=f` (magenta) drawn as two lines; their intersection is the solution, marked with a gold dot and its coords. Parallel rows (det = 0) leave a "no unique solution" note. |
| `span(id, (cx,cy), unit, (vx,vy), [(wx,wy)], [color])` | the **span** of one or two vectors (as arrows from the origin): one vector — or two parallel ones — spans a **line** (the rank-1 collapse); two independent vectors span the **whole plane** (a faint region). |
| `diagonalise(id, (cx,cy), unit, a, b, c, d, [color])` | `A = P D P⁻¹` made visual: draws the (skewed) **eigen-grid**, the eigen-axes, and the unit eigen-cell together with its image under `A` — a pure **stretch** by λ along each eigenvector, no shear (A is *diagonal* in its own basis). `diagonalize` is an alias. Complex/repeated eigenvalues leave a note. |
| `rref(id, "2 1 5 ; 1 3 10", (cx,cy), [cellw], [rowh])` | **animated Gaussian elimination**: reduces the matrix (rows split on `;`) to reduced row-echelon form one row operation at a time. Draws static brackets + one matrix per state `{id}.s0`, `{id}.s1`, … (all hidden) at the same spot, with each row-op text as `{id}.op0`, `{id}.op1`, …. Reveal the states in order (cross-fade `s{k-1}`→`s{k}`) to watch the numbers change in place. For an augmented `[A\|b]`, the last state's final column is the solution. |
| `project(id, (cx,cy), unit, (bx,by), (ax,ay), [color])` | **orthogonal projection** of vector `b` onto the line spanned by `a`: the subspace line (`{id}.line`), `b` (`{id}.b`), its shadow `p = (b·a / a·a) a` (`{id}.p`), the residual `b − p` (`{id}.res`) meeting the line at a right angle (`{id}.rt`), plus labels. The nearest point of the subspace to `b`. |
| `leastsquares(id, (cx,cy), unit, "x1 y1  x2 y2  …", [color])` | the **best-fit line** through a point cloud (linear regression): the points (`{id}.points`), the line `y = m x + c` minimising the squared vertical residuals (`{id}.line` + `{id}.eq`), and each residual (`{id}.residuals`). The same principle as `project`. |
| `numberline(id, (cx,cy), halfw, from, to, step)` | an axis with ticks and labels from `from` to `to` |
| `arrowfield(id, (cx,cy), halfw, halfh, field, [n])` | a grid of arrows sampling a named vector `field`, coloured by magnitude (cyan→lime→magenta); `n` arrows across |
| `matrix(id, "a b; c d", (cx,cy), [cellw], [cellh])` | a bracketed matrix (rows split by `;`, entries by space **or comma** — so no comma inside an entry, and every row must have the same number of entries); entry `{id}.r{i}c{j}`, tags `{id}.row{i}` / `{id}.col{j}` / `{id}.entries`, brackets `{id}.lbrack`/`{id}.rbrack` |
| `table(id, "a b; c d", (cx,cy), [cellw], [cellh], [col-labels], [row-labels])` (aliases `mathtable`/`decimaltable`/`integertable`) | a ruled grid of single-token entries (rows split by `;`, cells by space **or comma** — so no comma inside a cell like `(0,0)`, and every row must have the same number of cells); body cell `{id}.r{i}c{j}` (tags `{id}.row{i}` / `{id}.col{j}` / `{id}.entries`); optional header strings add a top label row (`{id}.collabel{j}`) / left label column (`{id}.rowlabel{i}`), tagged `{id}.labels`; grid lines `{id}.h{k}` / `{id}.v{k}`, tagged `{id}.hlines` / `{id}.vlines` / `{id}.lines` |
| `arc(id, (cx,cy), r, start, sweep)` | a circular arc line (angles in degrees) |
| `sector(id, (cx,cy), r, start, sweep)` | a filled pie slice |
| `annulus(id, (cx,cy), outer, inner)` | a filled ring between two radii |
| `pie(id, (cx,cy), r, n)` | a circle cut into `n` equal filled sectors, each addressable as `{id}0 … {id}{n-1}` (tag `id`) |

Named `plot` functions (`fn`): `sin`, `cos`, `tan`, `asin` (`arcsin`), `acos`
(`arccos`), `atan` (`arctan`), `parabola` (`sq`, `square`),
`cubic` (`cube`), `line` (`id`, `identity`), `abs`, `exp`, `sqrt`, `log`
(`ln`), `recip` (`inv`), `gauss` (`bell`), `sinc` (`sin x / x`), `sigmoid`
(`logistic`), `relu`, `step` (`heaviside`). These are *plot barewords* only —
each is shorthand for a formula (e.g. `sinc` = `"sin(x)/x"`); to use one inside
an expression, write that formula. The whole family lives in one table
(`named_formula` in `src/kits/math.rs`) — add an arm there and everything else
derives from it.

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
| `commontangent(id, oA, aOn, oB, bOn, ["type"])` | a **common tangent to two circles** (each = centre + a point on it). `type` = `"external"`/`"direct"` (default) or `"internal"`/`"transverse"`. Draws the segment `{id}` **between the touch points** (its length = the tangent length: external `√(d²−(r₁−r₂)²)`, internal `√(d²−(r₁+r₂)²)`); touch dots `{id}.a`/`{id}.b`. Errors if the circles are too close for that tangent. |
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

## The stats kit

Turn a dataset into a picture. The dataset is a plain number list (`"v1 v2 v3 …"`,
parsed like `leastsquares`). Tier 1 — *describe a dataset*:

| builtin | what it draws |
| --- | --- |
| `histogram(id, (cx,cy), "v1 v2 …", [bins], [width], [height], [color])` | bins the numbers into bars — the **shape** of the data. Bars are `{id}.bar0`, `{id}.bar1`, … (tagged `{id}` and `{id}.bars`, exactly `bins` of them so a `for k in 0..bins { draw(id.bar{k}) }` loop is safe) so they stagger in and recolour as a group. A gold `{id}.meanline` + `{id}.mean` mark the mean; `{id}.min` / `{id}.max` label the range. Default bin count ≈ √n (clamped 5–20). Pass `rainbow` as the colour to give every bar its own hue (no loop needed). |
| `summary(id, (cx,cy), "v1 v2 …", [width], [color])` | **describe a dataset**: the data as dots on a number line (`{id}.dots`) with **mean** (gold), **median** (magenta) and **mode** (lime) markers (`{id}.meanmark`/`.medianmark`/`.modemark` + `.*lbl`), a translucent **±1σ band** (`{id}.band`), and a readout of **n / range / variance / std** (`{id}.readout`). Central tendency + dispersion in one call. |
| `skew(id, (cx,cy), "v1 v2 …", [bins], [width], [height], [color])` | the **shape** of a dataset: a histogram with the **mean** (gold) and **median** (magenta) marked and a labelled **skewness** (`{id}.skewlbl` — right / left / ≈ symmetric). Mean right of median ⇒ right-skewed. Bars `{id}.bar{k}` (`{id}.bars`); pass `rainbow` for per-bar hues. |
| `boxplot(id, (cx,cy), "v1 v2 …", [width], [color])` | the **five-number summary** as a box-and-whisker: the box (`{id}.box`) spans Q1→Q3 (its width IS the **IQR**), `{id}.med` marks the median, whiskers (`{id}.whiskerlo`/`.whiskerhi` + caps) reach the extreme non-outliers (within 1.5·IQR), and points beyond are **outliers** (`{id}.out{k}`, tagged `{id}.outliers`). Value labels + `{id}.iqr`. |
| `correlation(id, (cx,cy), unit, "x1 y1  x2 y2 …", [color])` | how strongly two variables move together: the **scatter** (`{id}.p{k}`, tagged `{id}.points`), the best-fit **line** (`{id}.line`), and the **Pearson correlation r** (`{id}.r`, with a strong/moderate/weak · positive/negative reading). `unit` = px per data unit (x & y share it — use comparable ranges). |
| `bellcurve(id, (cx,cy), mu, sigma, [unit], [color])` | the normal / Gaussian **bell curve** with the **68-95-99.7 rule** shaded: `{id}.curve`, nested ±1σ/±2σ/±3σ bands (`{id}.band1/2/3`, tagged `{id}.bands`), `{id}.mean`, the `{id}.p1/2/3` percentages, and value ticks `{id}.t-3…t3` (μ, μ±σ, …). `unit` = px per σ (default 80); the bell is standardised, μ/σ set the axis values. Alias `gaussian`. |
| `hypothesis(id, (cx,cy), z, [alpha], [unit])` | a two-tailed **significance test**: the standard-normal null distribution with the tails beyond ±z shaded (`{id}.tails`) — their area is the **p-value** (`{id}.p`), compared to `alpha` (default 0.05) for the verdict (`{id}.verdict`). |
| `covariance(id, (cx,cy), unit, "x1 y1  x2 y2 …", [color])` | **covariance** as signed area: a cross at the means, and per-point rectangles (`{id}.rects`) cyan where `(x-x̄)(y-ȳ)>0`, magenta where negative — their balance is the covariance (`{id}.cov`). |
| `bayes(id, (cx,cy), heads, tails, [width], [height])` | **Bayesian updating** for a coin's bias: a mild **prior** (`{id}.prior`), the **likelihood** from the data (`{id}.likelihood`), and the **posterior** (`{id}.posterior`) between them — pulled toward the data, sharpening with evidence. |
| `distribution(id, (cx,cy), "kind", a, [b], [color])` | a named distribution: **uniform**(lo=a, hi=b) and **exponential**(rate=a) as density curves; **binomial**(n=a, p=b) and **poisson**(mean=a) as probability bars (`{id}.bars`). Pass `rainbow` for per-bar hues. |
| `confidence(id, (cx,cy), mean, sd, n, [level], [width])` | a **confidence interval** for a mean: the estimate on a number line (`{id}.estimate`) with an error bar of ± z·sd/√n (z from `level`, default 95%). `{id}.bar` + caps, `{id}.ci`. |
| `montecarlo(id, (cx,cy), points, [seed], [size])` | estimate **π by darts**: random points in a square, inside the circle cyan / outside magenta (`{id}.points`); π ≈ 4·inside/total (`{id}.pi`). Seeded / reproducible. |
| `randomwalk(id, (cx,cy), steps, [seed], [scale])` | a 2-D **random walk**: from the centre, each step a random direction — the wandering `{id}.path` with `{id}.start` (lime) and `{id}.end` (gold). Seeded. |
| `lln(id, (cx,cy), trials, [seed], [width], [height])` | the **Law of Large Numbers**: the running proportion of heads over many coin flips (`{id}.curve`) — wild at first, settling onto the true 0.5 (`{id}.ref`). Seeded / deterministic. Draw the curve in to *watch* it converge. |
| `clt(id, (cx,cy), samplesize, trials, [seed], [width], [height], [color])` | the **Central Limit Theorem**: `trials` experiments, each the average of `samplesize` dice, histogrammed — the averages pile into a bell. Draws the histogram of sample means (`{id}.bar{k}`, tagged `{id}.bars`, exactly 30), the normal curve they converge to (`{id}.curve`), the mean line, ticks 1–6, and `{id}.info`. **Seeded** (deterministic — same `seed` → identical render). Pass `rainbow` for per-bar hues. |

```manic
histogram(h, (cx, cy), "72 85 90 68 95 88 76 91 83 79 84 60 97 81", 10);
untraced(h.bars);
stagger(0.06) { for k in 0..10 { draw(h.bar{k}, 0.35); } }   // bars build up
```

## The physics kit

A **simulation** is built from its physics and PRE-SIMULATED (RK4) at build time,
so the render is deterministic (frame-identical every run). The pendulum is the
first named sim; more follow the same shape.

| call | makes |
|---|---|
| `pendulum(id, [center], [length], [angle0], [unit], [damping])` | a swinging pendulum. Only `id` is required. `center` is the pivot (default `(640,200)`); `length` in metres (default 1), `angle0` the release angle in **degrees** from vertical (default 30), `unit` px-per-metre (default 150), `damping` (default 0). Lays out `{id}.pivot`, `{id}.rod`, `{id}.bob`, the faint swing arc `{id}.path`, plus overlays (`{id}.overlays`): the velocity arrow `{id}.vel` and the KE/PE energy bars `{id}.ke`/`{id}.pe` + labels. All tagged bare `{id}` + `{id}.parts`. Static until you call `swing`. |
| `spring(id, [center], [stiffness], [x0], [unit], [damping])` | a mass on a spring. Only `id` required. `center` = equilibrium (default `(640,320)`); `stiffness` k (default 10); `x0` initial displacement m (default 1.3); `unit` px/m (default 110); `damping` (default 0). Lays out `{id}.wall`, `{id}.spring`, `{id}.mass`, `{id}.path` + the shared velocity arrow / energy bars. Its energy well is a **parabola** ½kx². Animate with `run(id)`. |
| `doublependulum(id, [center], [angle1], [angle2], [unit])` | the chaotic double pendulum (two arms hinged end-to-end). Only `id` required. `angle1`/`angle2` release angles in **degrees** (default 90 each). Parts `{id}.pivot/.rod1/.bob1/.rod2/.bob2` + the outer bob's trail `{id}.path`. A 4-D system — supports `phase`/`timegraph`/`energygraph` but **not** `well`. Animate with `run(id)`. |
| `springpendulum(id, [center], [angle0], [stretch0], [unit], [damping])` | an **elastic pendulum** (swings + bounces); the springy rod is drawn as a stretching coil. Energy sloshes between swing and bounce. |
| `kapitza(id, [center], [angle0], [vibeamp], [unit])` | a **Kapitza pendulum** — a strong enough `vibeamp` (pivot vibration) stabilises the **inverted** position (start `angle0` near 165–180°). Driven, so energy isn't conserved. |
| `cartpendulum(id, [center], [angle0], [unit])` | a pendulum on a **spring-mounted cart** rolling on a track (`{id}.track/.wall/.spring/.cart/.rod/.bob`). |
| `comparependulum(id, [center], [angle0], [unit])` | **two chaotic pendulums** started ≈0.001 rad apart — sensitive dependence; they diverge (`{id}.rodA/.bobA` cyan, `{id}.rodB/.bobB` magenta). Watch it in `phase`/`timegraph`. |
| `verticalspring(id, [center], [stretch0], [unit], [damping])` | a mass bobbing on a **vertical** spring under gravity (parabolic well, shifted). |
| `springincline(id, [center], [angle], [unit], [damping])` | a mass on a spring on an **inclined plane** (`angle` in degrees). |
| `bungee(id, [center], [unit], [damping])` | a **bungee jump** — free-fall, then a one-sided elastic cord (`{id}.platform/.cord/.jumper`); lopsided energy well. |
| `resonance(id, [center], [drivefreq], [unit], [damping])` | a **driven spring** — a drive near the natural √(k/m) pumps the amplitude up (resonance). |
| `doublespring(id, [center], [unit])` | **two coupled masses** between walls (three coils) — energy sloshes (beating); `phase` shows normal modes. |
| `seriesparallel(id, [center], [unit])` | springs in **series vs parallel** side by side — soft/slow vs stiff/fast (see it in `timegraph`). |
| `carsuspension(id, [center], [unit])` | a **quarter-car** riding a scrolling road (bump/washboard/pothole) on a spring+damper. |
| `piston(id, [center], [rpm], [unit])` | an **engine piston** (slider-crank): a spinning crank + rod drive a piston up/down in a cylinder. Kinematic — no phase/energy views. |
| `molecule(id, [center], [atoms], [unit])` | **N atoms bonded by springs**, vibrating about their shape (`{id}.atom{i}`, `{id}.bond{i}{j}` coils); `energygraph` shows the conserved energy. |
| `robotarm(id, [center], [mode], [unit])` | a **two-link arm** tracking a target by inverse-kinematics velocity control (`{id}.base/.link1/.elbow/.link2/.ee/.target`, trail `{id}.path`). `mode` **1 = trace a circle** (default), 2 = figure-8, 0 = reach a fixed point and settle. In modes 1/2 the gripper follows the moving target for the whole run. |
| `pulley(id, [center], [m1], [m2], [unit])` | a vertical **Atwood machine** — two masses over one pulley, accelerating at (m₁−m₂)g/(m₁+m₂) (`{id}.wheel/.mass1/.mass2/.ropeL/.ropeR`). `energygraph` shows the KE↔PE trade. |
| `pulleyscale(id, [center], [m1], [m2], [unit])` | an Atwood machine over **two** pulleys with an in-line **spring scale** reading the rope tension 2·m₁·m₂·g/(m₁+m₂) — not the sum of weights (`{id}.scale/.reading/.mass1/.mass2`). |
| `blocktackle(id, [center], [load], [effort], [strands], [unit])` | a **compound pulley** (block & tackle): a `load` on a movable block held by `strands` = N rope segments, pulled by an `effort` mass. N gives a **mechanical advantage of N** — an effort of only load/N balances the load, and the effort end travels N× as far. N=1 is the Atwood. `{id}.fixed/.movable/.load/.strand{i}/.effort`. |
| `compoundpulley(id, [center], [mA], [mB], [mC], [unit])` | a **compound pulley with a movable pulley**: a fixed top pulley carries mass A and a movable lower pulley; the movable pulley carries B and C. String constraints link them (a_A = −a_P, a_B + a_C = 2·a_P; T₁ = 2·T₂). **Static when mA = mB + mC.** `{id}.top/.mov/.massA/.massB/.massC/.rope*`. |
| `ramp(id, [center], [angle], [mass], [applied], [unit])` | a block sliding on an **inclined plane** with static/kinetic **friction** (`angle` in degrees; optional horizontal `applied` force). Friction dissipates energy, so `energygraph`'s total decays (`{id}.incline/.surface/.block`). Reveal its **free-body diagram** with `forces(id)`. |
| `inclinepulley(id, [center], [angle], [m1], [m2], [unit])` | the **incline-Atwood**: a block `m1` on an incline tied over a pulley at the top to a hanging mass `m2`. a = (m₂g − m₁g·sinθ)/(m₁+m₂). `{id}.incline/.pulley/.block/.rope1/.rope2/.mass2`. |
| `doubleincline(id, [center], [angle1], [angle2], [m1], [m2], [unit])` | two blocks on a **wedge's two slopes**, tied over an apex pulley (right slope rough). Slides toward the heavier/steeper side. `{id}.wedge/.pulley/.mass1/.mass2`. |
| `inclinebumper(id, [center], [angle], [mass], [stiffness], [unit])` | a block **slides down an incline into a spring bumper** at the base (one-sided contact), compresses it, and launches back — free-slide then spring. `{id}.incline/.spring/.plate/.block`. |
| `springchain(id, [center], [angle], [unit])` | **three blocks joined by two springs** on an incline — coupled oscillators / normal modes (shown in the incline's frame; uniform gravity doesn't affect the internal motion). `{id}.block1..3/.spring1/.spring2`. |
| `looptrack(id, [center], [radius], [height], [unit])` | a ball rolls down a ramp and around a vertical **loop-the-loop** (a curved track). Energy solver v=√(2g(H−y)) along the arc — it slows at the top; `height` must exceed 2·`radius`. `{id}.ramp/.loop/.ball`. |
| `collideblocks(id, [center], [m1], [m2], [restitution], [unit])` | the classic momentum demo: **block 1 on a spring** to the wall, block 2 slides in and they collide with restitution `e` (1 = elastic → total energy conserved; <1 → lost). A live **Σp readout** shows momentum conserved at each collision. `{id}.spring/.block1/.block2/.mom` + walls/floor. |
| `bulletblock(id, [center], [bulletmass], [speed], [blockmass], [unit])` | a bullet fired into a block **embeds** (perfectly inelastic). The combined mass crawls off at m_b·v_b/(m_b+M) — most kinetic energy is lost (`energygraph`'s total STEPS DOWN). `{id}.floor/.block/.bullet`. |
| `newtonscradle(id, [center], [balls], [pulled])` | **Newton's cradle** — a row of equal pendulum balls; pull `pulled` back and the same number swing out the far side. An **event-driven** sim: free-flight between elastic collisions resolved by the shared 1-D impulse. `{id}.bar/.string{i}/.ball{i}`. |
| `stringwave(id, [center], [width], [amp], [pluck])` | a **wave on a plucked string** — N masses on springs, both ends fixed (the discretised wave equation). The pulse splits, travels, and reflects. Drawn as a rainbow chain of segments `{id}.seg{i}`. `pluck` (0..1) sets the initial peak. |
| `dropmass(id, [center], [dropheight], [unit])` | a mass dropped onto a spring-block that **sticks** (inelastic collision) — `energygraph`'s total **steps down** at impact, then the heavier mass oscillates about a lower equilibrium (`{id}.spring/.block/.drop/.eq1/.eq2`). |
| `raft(id, [center], [personmass], [raftmass], [unit])` | a person walking on a **floating raft** — momentum conservation keeps the **centre of mass fixed**, so the raft slides the opposite way (`{id}.water/.cm/.raft/.body/.head`). Kinematic — no energy/phase views. |
| `brachistochrone(id, [center], [unit])` | four beads **race under gravity** from A to B down four curves (straight/arc/parabola/**cycloid**); the cycloid wins. Each is a full RK4 bead-on-wire integration (`{id}.straight/.circle/.parabola/.cycloid`, beads `{id}.bead_*`). |
| `run(id, [dur])` (alias `swing`) | replay a sim's pre-simulated motion over `dur` seconds (default 6) — every part, velocity arrow, energy bar, **and view marker** animates along it. Works for **any** sim (the whole pendulum + spring families). |
| `forces(id, [dur])` | reveal a sim's **free-body force diagram** — the force vectors on the body (for `ramp`: gravity `mg`, normal `N`, friction `f`, and the acceleration `a`), which then ride the body during `run`. Currently provided by `ramp`. |
| `phase(id, (cx,cy), [size])` | **phase portrait** of a sim (e.g. θ vs ω) in a `2·size` panel at `(cx,cy)` — a closed loop when energy is conserved, an inward spiral when damped. A dot rides the curve during `swing`. Call the sim ctor first. |
| `well(id, (cx,cy), [size])` | the **potential-energy well** U(pos) of a sim, with the body as a ball rolling in it (its height = current PE) — a marble-in-a-bowl view. The ball rides the curve during `swing`. |
| `timegraph(id, (cx,cy), [size])` | the sim's phase variables as **curves over time** (θ(t) cyan, ω(t) magenta) with a vertical sweep line marking "now" during `swing`. |
| `energygraph(id, (cx,cy), [size])` | **KE (cyan) / PE (magenta) / total (gold)** energy as curves over time — total flat when conserved, decaying when damped — with a sweep line. |

Views are **optional and generic** — any sim that stores the needed data (a
phase pair, a well curve) supports them; a sim that doesn't simply can't use that
view. They read the sim's pre-simulated data, so they cost nothing at render time.

```manic
pendulum(p, (cx, 190), 2, 50);   // length 2 m, released from 50°
untraced(p.path);
draw(p.path, 1.0);               // trace the arc it will follow
swing(p, 8);                     // then play the motion over 8 s
```

## The optics kit

Light as geometry — easy builtins with the **real physics underneath** (Snell's
law, and, coming next, Sellmeier dispersion). Like the physics sims, an optics
builtin is **static geometry that animates by sweeping a parameter**: `run(id)`
replays the sweep (incidence angle today; focal length / wavelength next).

| builtin | what it does |
|---|---|
| `refract(id, [center], [n1], [n2], [angle])` | a light ray meeting the boundary between two media and **bending** (Snell's law). Top medium index `n1` (default 1.0 = air), bottom `n2` (default 1.5 = glass). With no `angle`, `run(id)` **sweeps the incidence angle** — the refracted ray swings, the live `in`/`out` read-outs are the true Snell angles, and when light starts in the denser medium (`n1 > n2`) it shows **total internal reflection** past the critical angle. Give `angle` (degrees) to freeze one incidence. Parts `{id}.interface/.normal/.incident/.refracted/.reflected/.thetai/.thetat/.tir`. |
| `lens(id, [center], [focal], [aperture])` | a **converging lens** focusing a parallel beam to the **focal point** F (ideal thin lens — every parallel ray passes through F). `focal` in px (default 240), `aperture` the beam half-height (default 150). With no `focal`, `run(id)` **sweeps the focal length** so the focus slides in toward the lens (shorter `focal` = stronger lens). Parts `{id}.axis/.lens/.focus/.flabel/.in{i}/.out{i}`. |
| `prism(id, [center], [glass])` | white light entering a triangular prism and splitting into a **spectrum** — each colour traced through both faces with its true `n(λ)` (real **Sellmeier dispersion**), so blue bends more than red because it genuinely does. `glass` names the material (`"bk7"` crown default; `"sf11"`, `"f2"`, `"diamond"`, `"water"`, `"sapphire"`, `"silica"`). `run(id)` **sweeps the incidence angle** — the fan swings and its spread widens away from minimum deviation. Parts `{id}.prism/.beam/.in{c}/.out{c}` (c = 0 red … 8 violet). |
| `achromat(id, [center], [aperture])` | **chromatic aberration and its fix.** A single lens focuses blue nearer than red (real dispersion — its index is higher for blue), so white light never comes to one focus. `run(id)` **sweeps in the achromatic doublet** (crown + flint) and the red & blue foci slide back together to a single sharp point. Parts `{id}.axis/.lens/.in{i}/.r{i}/.b{i}/.fred/.fblue`. (CA direction/relative size are real; the axial gap is exaggerated for visibility.) |
| `lenssystem(id, [center], [preset], [object])` | a **real multi-element lens**, ray-traced through its actual **spherical/aspheric surfaces** (not the ideal thin lens of `lens`). `preset` is a lens **by name** — `"singlet"`/`"biconvex"`, `"plano-convex"`, `"aspheric"` (a conic that nulls spherical aberration), `"meniscus"`, `"doublet"`/`"achromat"`, `"triplet"`/`"cooke"` — **or a full custom prescription** (any string with `\|`): a surface table `"radius thickness glass [conic] [aperture] \| …"` — radius px (`+`/`-`/`flat`), glass name or `air`, optional **conic** constant (asphere) and **semi-diameter**, e.g. `"200 30 bk7 \| -200 0 air"` or an asphere `"190 28 bk7 -0.55 \| flat 0 air"`. Optional `object` = a finite object distance (px) for a diverging point source (omit ⇒ collimated). Sketch the rays with `draw(id.rays)`; `run(id)` sweeps a **sensor** plane while a live **spot-size** read-out dips to its minimum at best focus. **f-number** + **NA** read-outs (collimated only) and a magenta **best-focus** marker. Parts `{id}.elem{k}/.axis/.ray{i}` (tagged `{id}.rays`) `/.sensor/.spot/.fnum/.na/.bestfocus/.label`. |
| `rayfan(id, [center], [preset])` | the **ray-fan aberration plot** of a preset: transverse ray error at best focus (y) vs pupil height (x). A flat line is a perfect lens; the singlet's cubic **S-curve** is textbook spherical aberration; the doublet/triplet flatten it (all drawn to the singlet's scale). Sketch it on with `draw(id.curve)`. Parts `{id}.box/.zerox/.zeroy/.curve/.title/…`. |
| `spotdiagram(id, [center], [preset])` | the **spot diagram** at best focus: where a bundle of rays actually lands. A perfect lens makes a point; the singlet smears into a disc (**circle of least confusion**), the doublet/triplet stay tight — all to one scale. A green dot marks the ideal point focus; an **RMS** read-out gives the blur radius. Reveal the dots with `draw(id.dots)`. Parts `{id}.ideal/.rms/.dot{k}` (tagged `{id}.dots`) `/.crossx/.crossy/.label`. |
| `fieldspot(id, [center], [preset], [field])` | the **off-axis spot diagram** — a full 2-D pupil traced in **3-D** at a field angle `field` (degrees, default 5). On-axis the spot is symmetric; off-axis it flares into a **coma** comet and stretches with **astigmatism**. A dashed **Airy-disk** circle marks the diffraction limit (1 px ≈ 1 µm at the image) — when the geometric blur shrinks to it, the lens is diffraction-limited. Reveal with `draw(id.dots)`. Parts `{id}.dot{k}` (tagged `{id}.dots`) `/.airy/.rms/.crossx/.crossy/.label`. |

```manic
refract(r, (640, 380), 1.0, 1.52);   // air → crown glass
run(r, 7);                           // sweep the incidence angle

lens(l, (620, 360));                 // a converging lens
run(l, 7);                           // sweep the focal length — the focus slides

prism(p, (560, 400), "sf11");        // white light → a rainbow (real dispersion)
run(p, 7);                           // sweep the incidence angle — the fan swings
```

## The creator kit

**Responsive social-video formats** — one source adapts to 9:16, 4:5, 1:1 and
16:9. Creator owns safe layout regions, question hierarchy, fitted options,
timing, timer presentation, reusable identity and end cards. The global
`mono` template is the professional black-and-white default; use `shorts` when
accent hue matters.

| builtin | what it does |
|---|---|
| `creator(id, "spec")` | store one reusable profile. The first bare token is its handle; keys include `name`, `tagline`, `accent`, `secondary`, `footer=social|compact|signature|none`, `cta`, `safe=shorts|reels|tiktok|clean`, and optional custom `logo`. Platform values use `yt|youtube`, `x|twitter`, `ig|instagram`, `tt|tiktok`, `fb|facebook`, `li|linkedin`, `gh|github`, `web|site|url`, and `email|mail`. Use underscores for spaces inside values. Creates no drawables itself. |
| `socials(id, [at])` | draw the selected responsive footer. Social mode uses normalized native YouTube/X/Instagram/TikTok/Facebook/LinkedIn/GitHub/web/email marks—no image or SVG assets required. Up to three configured values appear beside their icons; larger sets collapse to icons plus the profile handle. Tags include `{id}.footer`, `{id}.socials`, `{id}.social.<platform>` and `.icon`/`.label`. |
| `quiz(id, "question", ["spec"])` | start a responsive quiz. The zero-config default is the professional `studio` skin, typewriter question, letter labels, balanced pace and draining ring. The order-free spec accepts legacy skin/reveal words and `skin`, `reveal=type|fade|rise|pop|cut`, `layout=auto|stack|grid|media-first`, `density=compact|comfortable|spacious`, `labels=letters|numbers|none`, `timer=ring|bar|number|segments|ticks|pulse|none`, `motion=calm|studio|punch|cut`, `pace=quick|balanced|calm|dramatic`, `seconds`, `safe`, and `accent`. Question tags are `{id}.question` plus `.panel`, `.kicker`, `.rule`, `.text`; compact ids such as `{id}.q` remain compatible. |
| `option(id, "text", [correct])` | add one of 1–6 answers. Stack supports up to four; auto/grid support six and centre a single final-row card. Type auto-fits, every card reserves a right-side check zone, and exactly one trailing `correct` choice receives the reveal highlight/check while distractors dim. Stable tags are `{id}.options`, `{id}.option.a`…`.f`, role suffixes `.card`/`.badge`/`.label`/`.text`/`.check`, and `{id}.option.correct`. |
| `timing(quiz, "spec")` | configure the quiz choreography independently: a `quick|balanced|calm|dramatic` preset and optional exact `ask`, `options`, `think`, `reveal`, `hold`, `stagger` or `seconds` values. A preset may be scaled by `run(q,dur)`; after any numeric phase use `run(q)` so there is only one duration source. |
| `timerstyle(id, "spec")` | style a quiz or generic Timing v2 clock without changing its phases: `look`, `position=auto|header|media|below`, `number=inside|outside|none`, `direction=drain|fill`, `size`, `thickness`, `color`, `track`, `label`, `font=mono|display`, and `finish=fade|hold|flash|pulse`. Stable groups are `{id}.timer`, `.timer.track`, `.timer.progress`, `.timer.value`, `.timer.label`, `.timer.effects`. |
| `countdown(id, [at], [secs], ["style"])` | standalone timer using the same native looks and style vocabulary. Play it with `run(id, secs)`. |
| `safezone(id, [inset|"profile"])` | visualize a numeric inset or named `shorts`/`reels`/`tiktok`/`clean` safe area while composing. Remove or hide the guide for export. |
| `figure(target, [center], [size])` | auto-fit any entity/group—including text, images, LaTeX equations and paths—into the responsive media zone. Tag every source dependency of a live construction; incomplete live groups fail clearly. |
| `explain(id, "text", ["source"])` | optional author-supplied answer context shown at reveal. Do not add it unless the explanation/source is actually authored. |
| `endcard(profile, ["cta=... safe=..."])` | build a hidden responsive creator lockup; reveal it at the close with `show(profile.endcard)`. |

```manic
canvas("9:16");
template("mono");
creator(me, "@anish2good name=Olympiad_Minute yt=zarigatongy x=@anish2good web=8gwifi.org/manic footer=social safe=reels");
quiz(q, "In a cyclic quadrilateral, angle A is 68 degrees. Find angle C.",
     "studio labels=letters layout=auto safe=reels");
option(q, "68 degrees");
option(q, "102 degrees");
option(q, "112 degrees", correct);
option(q, "122 degrees");
timing(q, "balanced ask=1.2 options=1 think=4.8 reveal=0.8 hold=2.2");
timerstyle(q, "look=bar direction=drain label=THINK finish=pulse");
socials(me);
run(q);
```

## The 3D kit

3D is rendered with depth testing beneath the normal 2D scene. Declare one
camera before using 3D objects. Camera and object animation remain stateless and
scrubbable like every other Manic track.

| call | makes |
|---|---|
| `camera3((ex,ey,ez), (tx,ty,tz), [fov], [projection])` | orbit camera from eye to target; `fov` is vertical degrees for `perspective` (default), visible world height for `orthographic` |
| `point3(id, (x,y,z), [radius])` | small sphere marking a 3D point |
| `line3(id, from, to)` | depth-tested 3D segment |
| `arrow3(id, from, to)` | depth-tested 3D vector |
| `cube3(id, center, (sx,sy,sz))` | cuboid centred at `center` |
| `linmap3(id, (cx,cy,cz), a,b,c,d,e,f,g,h,i, [color])` | a 3×3 matrix `[[a,b,c],[d,e,f],[g,h,i]]` applied to space (the 3-D echo of `linmap`/`determinant`): the unit cube (faint wireframe `{id}.ref`) becomes a parallelepiped (`{id}`), with basis arrows `{id}.i`/`.j`/`.k` on the matrix's columns. The enclosed **volume = the determinant** (`{id}.val`) — colour flips when det < 0, collapses flat at det = 0. |
| `eigen3(id, (cx,cy,cz), a,b,c,d,e,f,g,h,i, [color])` | the real **eigenvectors** of a 3×3 matrix as invariant lines through the origin (`{id}.axis0…`) with `lambda = λ` labels — the directions a vector only *stretches*, never turns (3-D echo of `eigen`). A real 3×3 always has ≥ 1 real eigenvector; complex eigenvalues (a rotation) get a note. |
| `sphere3(id, center, radius)` | sphere centred at `center` |
| `grid3(id, center, half, [spacing])` | XY ground grid from `-half` to `half` |
| `axes3(id, origin, length, [step])` | cyan x, magenta y, lime z arrows **with tick marks + numbers** every `step` (default 1; `step ≤ 0` = plain arrows), tagged as `id`. Tick numbers sit off each axis in a distinct direction and auto-declutter per frame (a number is hidden while it would collide with another), so a foreshortened or short axis stays readable and the numbers reappear as the orbit spreads it out |
| `pin3(label, (x,y,z) \| entity3)` | glue an existing 2D `text`/`label` to a 3D point (or a 3D entity); reprojected each frame so it tracks the camera |
| `follow3(id, target, [(dx,dy,dz)])` | make a 3D entity track another's position (+ offset), recomputed each frame |
| `midpoint3(id, a, b)` | a point at the midpoint of two 3D entities, recomputed as they move |
| `curve3(id, "x(t)", "y(t)", "z(t)", [(t0,t1)])` | parametric 3D curve sampled from three formulas of `t` (default range `0..2π`), drawn as a polyline (thin by default; give it body with `thick`) |
| `surface3(id, "z(x,y)", (x0,x1), (y0,y1), [res])` | height-field surface `z = f(x,y)` sampled over the x/y rectangle into a `(res+1)²` filled, flat-shaded mesh (default `res` 20) |
| `param3(id, "x(u,v)", "y(u,v)", "z(u,v)", (u0,u1), (v0,v1), [res])` | **general parametric surface** — three formulas of `u`,`v` sampled into a `(res+1)²` filled, flat-shaded mesh (default `res` 24). Unlike `surface3` it can wrap and close: **tori** (`(R+r·cos v)·cos u`, …), parametric **spheres**, **Möbius strips**, shells |
| `gradient3(id, surface, x, y, [color])` | an arrow on a plotted `surface3` at `(x,y)` pointing in the direction of **steepest ascent** (the gradient ∇f), its length growing with the slope — multivariable calculus |
| `tangentplane3(id, surface, x, y, [color])` | the **plane tangent** to a plotted `surface3` at `(x,y)` (`z = f + fx·(u−x) + fy·(v−y)`), a small translucent patch — the 3D analog of the tangent line |
| `volume3(id, surface, [res], [color])` | the **volume under** a plotted `surface3` as a `res×res` grid of columns from `z=0` to the surface (a 3D Riemann sum = double integral). Columns `{id}0…` tagged `id` |
| `prism3(id, (cx,cy,cz), sides, radius, height)` | regular n-gon **prism** as a filled, shaded solid, centred on its position (`sides ≥ 3`; many sides ≈ a cylinder) |
| `pyramid3(id, (cx,cy,cz), sides, radius, height)` | regular n-gon **pyramid** as a filled, shaded solid (many sides ≈ a cone) |
| `revolve3(id, (cx,cy,cz), "r(t)", (t0,t1), [sides])` | **solid of revolution**: sweep the radius profile `r(t)` over height `t∈[t0,t1]` around the vertical axis (filled, shaded) — vases, spheres (`sqrt(1-t*t)`), cones (`t`); default 32 sides |
| `extrude3(id, source, height, [(cx,cy,cz)])` | **extrude** a 2D fillable shape (`rect`/`circle`/`sector`/`annulus`/`polygon`) or a boolean `Region` (`union`/`difference`/`intersect`/`xor`) straight up into a solid of `height`, centred on `(cx,cy,cz)`. Extruding a boolean region gives **CSG solids** (e.g. plate `difference` hole → a plate with a hole; concave `union` → an L-beam). The 2D `source` is auto-hidden — it's only the cross-section recipe (its own boolean operands are not, so `hidden(…)` them if unwanted) |
| `morph3(a, b, [spin])` | set 3D entity `a` up to **morph** into `b`'s shape, then animate with `to(a, morph, 1, dur)`. Both are sampled now to a shared form: two **curves** blend as a polyline; two **surfaces** (`surface3`/`revolve3`) or two **solids** (`cube3`/`sphere3`/`prism3`/`pyramid3`/`extrude3`) blend as a filled, shaded grid — solids are reparameterised spherically so e.g. a cube can become a sphere (approximate near sharp corners). `spin` adds a winding rotation about the vertical axis. Both operands must be the same family |
| `thick(id, radius)` | give a 3D `curve3`/`line3`/`arrow3` real thickness — renders it as a shaded **tube** of the given world-space radius (arrows get a solid cone head) instead of a 1px line; `0` restores the thin line. (`stroke` is the 2D equivalent and only works on 2D shapes) |

| verb | animation |
|---|---|
| `move3(id, to, [dur], [ease])` | absolute 3D movement |
| `shift3(id, delta, [dur], [ease])` | relative 3D movement |
| `rotate3(id, (xdeg,ydeg,zdeg), [dur], [ease])` | Euler rotation in Z-Y-X order |
| `grow3(id, to, [dur], [ease])` | retarget a `line3` or `arrow3` endpoint |
| `orbit3(azimuth, elevation, radius, [dur], [ease])` | animate the camera orbit |
| `roll3(degrees, [dur], [ease])` | roll the camera around its viewing direction; compose with `orbit3` in `par` for turning-plane and cinematic banking shots |
| `look3(target, [dur], [ease])` | animate the camera target |

### What works on 3D entities (and what doesn't)

Not every 2D modifier/verb applies to 3D — a 3D entity lives in the separate
Z-up world, so 2D-only ones **error** when aimed at a 3D id (with a message that
says so and lists the alternatives).

| Applies to 3D | 2D-only (errors on a 3D id → use the alt) |
|---|---|
| **modifiers:** `color`, `opacity`, `hidden`, `untraced`, `tag`, `thick` | `hue` → use `color` with a palette name · `stroke` → use `thick` · `glow`, `z`, `size`, `bold`, `outlined`/`filled`/`outline` |
| **verbs:** `show`, `fade`, `draw`, `flash`, `pulse`, `recolor`, `scale`, and `to(id, morph\|opacity\|scale\|trace\|color, …)` | `move`/`shift`/`rotate`/`spin` → use `move3`/`shift3`/`rotate3` · `cam`/`zoom` → use `camera3`/`orbit3`/`roll3`/`look3` · `transform` (2D matrix) · `morph` → use `morph3` · `to(x/y)` (no 3D single-axis form) |

Spatial tracks always go through the explicit `move3`/`rotate3`/`grow3` family.

```
camera3((8,-10,6), (0,0,1), 45);
grid3(floor, (0,0,0), 5, 1);  color(floor, dim);
axes3(world, (0,0,0), 4);
cube3(box, (0,0,1), (2,2,2)); color(box, magenta);
par { rotate3(box, (0,0,360), 4, linear); orbit3(70,28,11,4,smooth); }
```

Current limits: no parametric surfaces/arbitrary meshes, lighting, model
loading, projected 3D labels, or robust intersecting-transparency sorting yet.

## Banner & watermark (brand kit)

manic's own logo and mark (à la `ManimBanner`).

| call | makes |
|---|---|
| `banner(id, (cx,cy), [scale])` | the manic logo: a cyan circle + magenta square + lime triangle icon trio (`{id}.dot`/`{id}.sq`/`{id}.tri`, tag `{id}.icon`) and the "manic" wordmark (`{id}.word`) |
| `watermark(id, [(x,y)], ["text"])` | a small, glowing, **screen-fixed** mark that ignores camera moves and persists. With no point it responsively sits bottom-right; pass a point to protect important content or platform UI. Default text: `Made With Manic`. |

Animate it `create → expand → unwrite` like the reference banner:

```
banner(logo, (600, 360), 1.1);
untraced(logo.icon);  hidden(logo.word);
watermark(autoMark);                              // responsive bottom-right
watermark(wm, (150, 48), "manic // synthwave"); // exact composition control

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

`fg` (foreground / `white`) · `void` (`bg`) · `cyan` · `magenta` (`pink`,
`accent`) · `lime` (`green`) · `gold` (`amber`, `yellow`) · `red` (`crimson`) ·
`orange` · `blue` (`azure`, a true blue — distinct from `cyan`) ·
`dim` (`gray`, `grey`) · `panel`.

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
