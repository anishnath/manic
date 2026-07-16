# Colour & style

## The palette

manic uses a small, fixed set of colour names — no hex, no RGB. They're tuned to
glow on the dark default background:

| name | is | name | is |
|---|---|---|---|
| `cyan` | electric blue | `dim` | muted grey-violet |
| `magenta` | hot pink | `fg` | near-white (default text) |
| `lime` | green | `panel` | dark fill |
| `void` | the background | | |

```manic
color(sun, cyan);
recolor(sun, magenta, 0.5);   // animate to a new palette colour
```

## Any colour, by hue

For a *computed* colour — a gradient, or one per item in a loop — use `hue`,
which takes an angle from 0 to 360:

```manic
hue(sun, 200);              // a fixed hue
for i in 0..24 {
  hue(p{i}, 360*i/24);      // a full rainbow around the loop
}
```

That's how the [rainbow-ring loop](language-layer.md) gets its colours.

## Glow

Every entity has a neon `glow` (0 = crisp, higher = brighter halo):

```manic
glow(sun, 8);   // strong halo
glow(grid, 0);  // crisp, no halo — good for fine detail
```

## Easings

The optional last argument of a motion verb is the **easing** — the shape of the
motion over time:

| easing | feel |
|---|---|
| `linear` | constant speed (mechanical) |
| `smooth` | ease in and out (the default, natural) |
| `in` / `out` | accelerate / decelerate |
| `back` | overshoot slightly and settle |
| `bounce` | bounce at the end |
| `elastic` / `spring` | wobble / springy settle |

```manic
move(p, (900, 400), 0.8, bounce);
move(p, (900, 400), 0.8, smooth);   // usually what you want
```

## Canvas & size

`canvas(...)` sets the frame. Give it a preset or explicit pixels:

```manic
canvas("16:9");        // 1280x720  (also: 1080p, 4k, square, portrait/9:16, 4:3)
canvas(1280, 720);     // explicit
```

`portrait` / `9:16` is 1080×1920 — pair it with the `reel` render preset for
vertical / social clips.

## Templates — the whole look

`template("...")` sets the movie's **look** in one call: the background, how the
palette renders, the glow, and any page chrome. Put it near the top, after
`canvas(...)`.

```manic
canvas("16:9");
template("paper");     // a flat white exam-paper page
```

| template | look |
|---|---|
| `plain` | the default — neon on near-black, no chrome |
| `terminal` | the neon-terminal frame (border, title, masthead) |
| `paper` (aliases `light`, `print`) | white page, dark ink — for print / textbook figures |
| `blueprint` (alias `blue`) | white & cyan lines on deep navy |

The clever part is the **palette remap**: a template doesn't just change the
background, it re-maps every palette colour to that template's role. So on
`paper`, `panel` → light box, `fg` → dark ink, `lime` → forest green, and so on —
which means your existing scene renders legibly on the new page **without
recolouring anything**. That's why `template("paper")` alone turns a pulley,
a spring, or a linked list into a clean textbook figure (see
[Elevating a scene](elevating.md)). `paper`/`blueprint` also drop the glow for
crisp print output.

Next: loops, variables, and macros → [The language layer](language-layer.md).
