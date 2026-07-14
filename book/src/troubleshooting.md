# Troubleshooting

manic checks your scene *as you type* — it underlines the spot and shows the
message, and where possible offers a one-click **Fix**. Here are the mistakes
people actually hit, the message you'll see, and the fix.

## Errors the editor catches

### 1. `unknown variable` — a missing `*`
The **#1 mistake.** Two names written together are read as a *single* word.

```manic
# ❌  unknown variable `idx`
dot(p, (cx + idx, cy), 6);
# ✅  put a * between the two names
dot(p, (cx + i*dx, cy), 6);
```

A **number** can hug a name (`2r`, `3(x+1)`), but **two names can't**. Add `*`
at every name-next-to-name: `i*dx`, `tau*i`, `xmid*sx`, and especially
`r*cos(t)` / `r*sin(t)` — `rcos`/`rsin` are the classic trap (they mean
"radius × cos", not a function).

### 2. `unknown function` in a plot — quote the formula
Only a short list of names work as **bare words** (`sin`, `cos`, `tan`,
`sqrt`, `abs`, `exp`, `log`, `parabola`, `cubic`, `gauss`, `sinc`, …). Anything
else — `acos`, `tanh`, `log10`, sums of terms — must be a **"quoted formula"**.

```manic
# ❌  unknown function `acos`
plot(f, (cx, cy), 80, 80, acos, (-1, 1));
# ✅  wrap it in quotes as a formula in x
plot(f, (cx, cy), 80, 80, "acos(x)", (-1, 1));
```

### 3. `needs at least N argument(s)` — you dropped the id (or an argument)
Every builtin's **first argument is its id** — a name *you* pick. Modifiers and
verbs need that id too.

```manic
# ❌  `size` needs at least 2 argument(s), got 1
size(30);
# ✅  say which entity
size(title, 30);

# ❌  `circle` needs at least 3 argument(s), got 2   (no radius)
circle(c, (cx, cy));
# ✅
circle(c, (cx, cy), 120);
```

### 4. `no entity named X` — a typo, or used too early
You referred to an id that doesn't exist — misspelled, or used before it's made.

```manic
text(title, (cx, 60), "Hello");
# ❌  no entity named `titel`
color(titel, cyan);
# ✅  match the id exactly
color(title, cyan);
```

### 5. `unknown colour` — use the palette (or `hue`)
Only the named palette colours work — no `red`, no `#ff0000`.

```manic
# ❌  unknown colour `red`
color(dot, red);
# ✅  a palette colour…
color(dot, magenta);
# ✅  …or a computed one, 0–360
hue(dot, 210);
```

Palette: `cyan  magenta  lime  gold  fg  dim  void  panel`.

### 6. `stroke is 2D-only` — a 2D styler on a 3D shape
Some styling is 2D-only. On 3D shapes use the 3D equivalent.

```manic
cube3(bx, (0, 0, 1), (2, 2, 2));
# ❌  `stroke` is 2D-only; for a 3D line/arrow/curve use `thick(bx, radius)`
stroke(bx, 3);
# ✅  thickness in world units
thick(bx, 0.1);
```

The message names the fix. Also: `hue` → use `color` on 3D entities.

### 7. `argument 1 should be a name` — a reserved word as an id
`pi`, `tau`, `e`, `inf`, `w`, `h`, `cx`, `cy` are built-in *values* — you can't
name an entity one of them.

```manic
# ❌  argument 1 of `dot` should be a name   (e is Euler's number)
dot(e, (cx, cy), 6);
# ✅  pick any other name
dot(pt, (cx, cy), 6);
```

### 8. `expected a statement … found ;` — a stray semicolon
Blocks (`par`, `seq`, `stagger`, `for`, `if`) end with `}` — **no** semicolon.

```manic
# ❌  expected a statement …, found `;`
par { show(a, 1); show(b, 1); };
# ✅
par { show(a, 1); show(b, 1); }
```

## No error — but it looks wrong

These pass the check, so watch for them yourself.

### 9. A curve appears all at once instead of drawing on
`draw` animates a shape that **starts hidden**. Declare it `untraced` first.

```manic
plot(f, (cx, cy), 80, 80, "sin(x)", (0, 6));
# ❌  f is already fully shown — draw does nothing visible
draw(f, 2);
# ✅  hide the line, then draw traces it on
untraced(f);
draw(f, 2);
```

### 10. Things land off-screen
The canvas is a **fixed logical size — 1280×720 for `16:9`** — *not* your
video's pixel size. Position with `cx`, `cy`, `w`, `h`, never hard-coded pixels.

```manic
# ❌  1700 is past the right edge (width is 1280)
text(t, (1700, 300), "hi");
# ✅  relative to the centre
text(t, (cx + 200, 300), "hi");
```

### 11. A matrix/table cell with a comma
Cells are single tokens split by spaces **or commas**, so a cell can't *contain*
a comma — `(0,0)` silently becomes two cells and the grid comes out malformed.

```manic
# ❌  no error, but the row breaks apart
matrix(m, "(0,0) (1,1)", (cx, cy));
# ✅  one token per cell
matrix(m, "0 1; 2 3", (cx, cy));
```

---

**Rules of thumb:** put a `*` between names · quote any formula that isn't a
bare-word function · give every entity an id · use `cx`/`cy`/`w`/`h` for
position · `untraced` + `draw` to trace a curve on.
