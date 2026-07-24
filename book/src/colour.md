# Colour & style

## The palette

manic uses a small set of semantic colour names. The default `mono` template
maps all of them to distinct black-and-white luminance levels; choose `plain`
or another colour template when hue should carry meaning. Their original neon
roles are:

| name | is | name | is |
|---|---|---|---|
| `cyan` | electric blue | `gold` | warm amber |
| `magenta` | hot pink | `red` | warm red |
| `lime` | green | `orange` | orange |
| `blue` | true blue (≠ cyan) | `dim` | muted grey-violet |
| `void` | the background | `fg` | near-white (default text) |
| `panel` | dark fill | | |

```manic
color(sun, cyan);
recolor(sun, magenta, 0.5);   // animate to a new palette colour
```

## Any colour, by hue

For a *computed* colour — one per item in a loop — use `hue`, which takes an
angle from 0 to 360:

```manic
hue(sun, 200);              // a fixed hue
for i in 0..24 {
  hue(p{i}, 360*i/24);      // a full rainbow around the loop
}
```

That's how the [rainbow-ring loop](language-layer.md) gets its colours.

## Gradient paint — computed, not painted

For a colour that *reads a quantity*, use `gradient`. One word covers multi-stop
ramps on fills and strokes; the mode (optional) picks the truth:

```manic
gradient(wave, blue, cyan, gold, 270);   // height of a plot
gradient(path, magenta, cyan);           // arc length along a stroke
gradient(well, panel, void, radial);     // centre → edge on a fill
gradient(p.path, blue, cyan, gold, "speed");      // true local speed (physics traj.)
gradient(swoop, dim, magenta, "curvature");       // how hard a path bends
```

Stops are palette names (≥2, evenly spaced) and stay template-aware.
`"speed"` only works on pre-simulated physics trajectories; `"curvature"` works
on any path. See the Modifiers table in [Shapes](shapes.md), and the demos
[gradient](ex-transforms.md#gradient),
[gradient-fastest-descent](ex-physics.md#gradient-fastest-descent), and the
shorts [gradient-fastest-descent-shorts](ex-creator.md#gradient-fastest-descent-shorts) /
[gradient-pendulum-shorts](ex-creator.md#gradient-pendulum-shorts).

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
canvas("16:9");        // 1280x720  (also: 4:5, square, portrait/9:16, 4:3, 1080p, 4k)
canvas(1280, 720);     // explicit
```

`portrait` / `9:16` is 1080×1920 — pair it with the `reel` render preset for
vertical / social clips.

Use `--canvas portrait|4:5|square|16:9|WIDTHxHEIGHT` to reframe one responsive
source without changing its `canvas(...)` line. The override is applied before
`w`, `h`, `cx`, `cy`, and build-time layout branches are evaluated.

## Templates — the whole look

For the complete selection guide, CLI override rules, mono tips and a runnable
black-and-white example, see [Templates — choose the whole visual system](templates.md).

`template("...")` sets the movie's **look** in one call: the background, how the
palette renders, the glow, and any page chrome. Put it near the top, after
`canvas(...)`.

```manic
canvas("16:9");
template("paper");     // a flat white exam-paper page
```

| template | look |
|---|---|
| `mono` (aliases `monochrome`, `bw`) | **default** — black-and-white editorial, near-black page, subtle glow |
| `plain` | original neon palette on near-black, no chrome |
| `terminal` | the neon-terminal frame (border, title, masthead) |
| `paper` (aliases `light`, `print`) | white page, dark ink — for print / textbook figures |
| `blueprint` (alias `blue`) | white & cyan lines on deep navy |
| `shorts` | restrained dark creator palette for Shorts and Reels |

Omitting `template(...)` is exactly the same as selecting `template("mono")`.
Use explicit `plain` when an older scene should keep the original neon colours.

The clever part is the **palette remap**: a template doesn't just change the
background, it re-maps every named palette colour to that template's role. So
on `mono`, `cyan`, `magenta`, `lime`, `gold`, `red`, `orange`, and `blue` become
carefully separated greys; on `paper`, `panel` → light box, `fg` → dark ink,
`lime` → forest green, and so on —
which means your existing scene renders legibly on the new page **without
recolouring anything**. That's why `template("paper")` alone turns a pulley,
a spring, or a linked list into a clean textbook figure (see
[Elevating a scene](elevating.md)). `paper`/`blueprint` also drop the glow for
crisp print output.

`hue(...)` and future explicit RGB colours are intentionally bespoke, so they
pass through instead of being forced to greyscale. Use the named palette colours
when a scene must remain strictly monochrome.

Next: loops, variables, and macros → [The language layer](language-layer.md).
