# Timing — par, seq & stagger

By default, verbs play **one after another**. Three wrappers change that — they
turn "then, then, then" into "together" or "cascading".

| wrapper | plays its steps… | use for |
|---|---|---|
| *(nothing)* | one after another | the normal flow |
| `par { … }` | all at the same instant | reveal a group at once |
| `seq { … }` | one after another (explicit) | grouping inside a `par` |
| `stagger(d) { … }` | each one `d` seconds after the last | cascades / waves |

```manic
show(a);  show(b);              // a, THEN b
par    { show(a);  show(b); }   // a and b together
stagger(0.1) { show(a); show(b); show(c); }  // a, then b 0.1s later, then c…
```

Put a `for` loop *inside* one and it just works — the loop expands first, so all
its statements land in the wrapper:

```manic
par          { for i in 0..6 { show(a{i}); } }   // whole row at once
stagger(0.1) { for i in 0..6 { show(b{i}); } }   // whole row cascading
```

```manic
{{#include ../samples/timing.manic}}
```

**▶ See it play:**

<div class="manic-video" data-video="timing"></div>

## Beats & sections

Two more timing words structure a longer video:

```manic
wait(1.2);              // hold — nothing moves for 1.2s
section("Part Two");    // a titled marker (jump to it in preview with keys 1–9;
                        //   also exported for lining up narration)
mark("beat-3");         // a named timestamp for your editor
```

`wait` is your friend for pacing — a beat of stillness after something lands
reads far better than rushing to the next move.

Next: the palette, glow, and easings → [Colour & style](colour.md).
