# Templates — choose the whole visual system

A template controls the background, semantic palette, glow character and any
page chrome for the entire movie. Choose it once near the top of the file:

```manic
canvas("9:16");
template("mono");
```

`mono` is the default. If a DSL file does not call `template(...)`, it renders
exactly as if `template("mono")` had been written. Keeping the call explicit is
useful in shared examples; omitting it is convenient for ordinary authoring.

## Available templates

| template | best for | character |
|---|---|---|
| `mono` | professional explainers, proofs and restrained Reels | black-and-white editorial surface, clear luminance hierarchy, subtle glow |
| `plain` | legacy or intentionally colourful scenes | original neon semantic colours on near-black |
| `terminal` | code, algorithms and technical demos | neon terminal frame and stronger chrome |
| `paper` | textbook figures, worksheets and print | white page, dark ink, crisp low-glow rendering |
| `blueprint` | geometry, engineering and construction | cyan/white drafting marks on deep navy |
| `shorts` | energetic social content where hue matters | restrained dark creator palette |

Useful aliases are `monochrome`, `blackwhite`, `black-white`, and `bw` for
`mono`; `light` and `print` for `paper`; and `blue` for `blueprint`.

## Mono in practice

```manic
{{#include ../samples/template-mono.manic}}
```

<div class="manic-video" data-video="template-mono"></div>

Named colours remain meaningful under mono. `fg`, `dim`, `panel`, `cyan`,
`magenta`, `lime`, `gold`, `red`, `orange`, `blue`, `teal`, `violet`, `coral`,
`indigo`, and `mint` are mapped to deliberately different greys instead of
collapsing to identical white. This preserves hierarchy and correct-answer
contrast while keeping the export monochrome.

Use named colours for template-aware work:

```manic
color(answer, lime);       // success role; becomes a bright mono tone
color(note, dim);          // secondary role; remains visually quiet
```

`hue(...)` is intentionally an explicit colour choice and bypasses semantic
palette remapping. Avoid it when the output must remain strictly black and
white.

## DSL selection versus export override

The DSL call travels with the scene. The command-line option is useful for a
one-off alternate render:

```sh
manic scene.manic --still 4.0
manic scene.manic --record out --template paper
```

An explicit `--template NAME` export option overrides the DSL template for that
run. This is a quick way to proof the same semantic scene on mono, paper and
blueprint without editing the source.

## Creator and Reel guidance

Start with mono for a polished black-and-white identity. It works especially
well for mathematics, question cards, native social icons and technical timers:

```manic
canvas("9:16");
// template("mono") is optional because mono is the default.
creator(me, "@anish2good yt=zarigatongy x=@anish2good web=8gwifi.org/manic footer=social");
quiz(q, "Which statement is true?");
```

Switch to `shorts` when accent hue communicates part of the channel identity,
or to `paper` for an exam-sheet or textbook treatment. Template choice does not
change Creator layout, safe areas, timing, option labels or social-platform
selection—it changes only their visual system.

## Practical checks

- Review one reading frame and one reveal frame at phone size.
- In mono, check luminance contrast rather than relying on colour names.
- Prefer one semantic accent role; do not make every object equally bright.
- Use `plain` when migrating an older example that must preserve its neon hue.
- Use `paper` or `blueprint` to test whether a construction remains legible on a
  very different surface.

Next: [Colour, glow and semantic roles →](colour.md)
