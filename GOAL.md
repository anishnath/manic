# manic — goal & plan

## What it is

**manic is a general-purpose 2D animation language and engine.** The product
is a *language*, not a Rust API: someone who is not a programmer writes a plain
text `.manic` file — the way you'd write Mermaid or TikZ — and manic renders it
to a live preview or a video.

The language is deliberately small and ASY-inspired: function-call statements,
`(x, y)` points, `;` terminators, `//` comments, and `par { }` / `seq { }` /
`stagger(d) { }` blocks. See [LANGUAGE.md](LANGUAGE.md).

## Why this shape

Existing tools each miss part of the target:

- **Mermaid / Graphviz** — declarative and approachable, but static diagrams,
  no timeline.
- **Manim** — beautiful animation, but you write Python; it's a programmer's
  tool.
- **TikZ / Asymptote** — precise and scriptable, but a steep, code-heavy
  surface and, again, no first-class notion of *animation over time*.

manic aims for the readability of Mermaid, the drawing vocabulary of ASY, and a
timeline as a first-class idea — authored by non-programmers.

## The load-bearing principle: domain-agnostic core + pluggable kits

manic is **not** a math tool, and **not** an algorithms tool. Those are
*domains*. Today the shipped domain is **math** (axes, plots, vectors, number
lines). Tomorrow it's **algorithms** (arrays, trees, graphs). After that,
whatever. The core must know nothing about any of them.

```
                       .manic  (text a non-programmer edits)
                          │
   ┌──────────────────────┼───────────────────────┐
   │  CORE (domain-agnostic)                        │
   │  lang/   lexer → parser → generic AST          │
   │             │   (calls: name + args + block)   │
   │             ▼   dispatched by name against a    │
   │          REGISTRY of builtins                  │
   │  engine/  primitives · scene · stateless       │
   │           timeline · animate · render · record  │
   └─────────────┬───────────────────────┬──────────┘
                 │                        │
        ┌────────▼────────┐      ┌────────▼─────────┐
        │  kit: std       │      │  kit: math       │
        │  (always on)    │      │  (a domain)      │
        │  shapes, verbs  │      │  axes, plot,     │
        │  modifiers      │      │  vector, …       │
        └─────────────────┘      └──────────────────┘
                                  (algo kit: next)
```

Two decisions make a new domain cheap:

1. **The parser is generic.** It parses grammar (calls, points, numbers,
   strings, blocks) and knows *zero* verb names. Meaning is resolved at
   lowering time against a builtin **registry**. A kit registers `plot`,
   `axes`, … without touching the lexer or parser.
2. **Primitives stay atomic and general** (point, line, rect, text, polygon,
   polyline). Domain shapes are *compositions* a kit registers — `axes` is two
   arrows, `plot` is a sampled polyline. New truly-atomic needs become core
   primitives only when a kit genuinely can't compose them.

The dependency direction is the real boundary: **kits depend on the core; the
core never depends on a kit.**

## Visual identity

One house style, defined once in `src/style.rs`: **neon terminal / synthwave**
— a deep indigo-black void, glowing cyan/magenta/lime strokes, all-monospace
type, a terminal-window frame (corner brackets, traffic-light dots, a fake
shell prompt), and an optional CRT post-process. Change it in one file and
every past and future animation follows.

## Non-goals

- Not a GUI editor — the point is a readable, diffable text language.
- Not audio/narration sync inside the engine — marker export
  (`markers.json`) is the hand-off to a video editor.

## Plan / sequencing

Building the harder domain (math) first stress-tests the core so the easier
domain (algo) falls out trivially. LaTeX/formula typesetting is the hard 20%
and is deliberately deferred — math v1 uses monospace labels. See
[ROADMAP.md](ROADMAP.md) for the concrete checklist and current status.
