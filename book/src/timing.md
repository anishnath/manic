# Timing — flow, named phases & clocks

By default, verbs play **one after another**. Three wrappers change that — they
turn "then, then, then" into "together" or "cascading".

| wrapper | plays its steps… | use for |
|---|---|---|
| *(nothing)* | one after another | the normal flow |
| `par { … }` | all at the same instant | reveal a group at once |
| `seq { … }` | one after another (explicit) | grouping inside a `par` |
| `stagger(d) { … }` | each one `d` seconds after the last | cascades / waves |
| `step("name") { … }` | all at once, with a named exported start | change one reactive world across representations |

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

## Named reactive steps

Use `step` when a beat represents the world's **next conceptual state**, not
just anonymous timing. Its children start together like `par`; its duration is
the longest child; anything not mentioned remains exactly as it was. The unique
name is written to `markers.json` and becomes a first-class editing boundary:
`manic stages FILE.manic` lists durations, `--stage NAME` previews or records
one stage, and `--from-stage` / `--to-stage` export an inclusive arc.

```manic
step("explain") {
  rewrite(work, `f'(x)=2x`, 0.9, smooth);
  to(tangent, x, 2.5, 2.0, smooth);
  to(slopeValue, x, 2.5, 2.0, smooth);
  say(caption, "Every representation changes together.");
}
```

Steps are top-level and names must be non-empty and unique. Put `seq { … }`
inside a step when a small part needs ordered choreography.

An authored `wait` after a step remains part of that stage until the next step
begins. This lets a stage-only export keep the reading hold after its motion.
See [Story stages](creator-stages.md) for the live navigator and publishing
workflow.

## Generic Timing v2 — one clock for any scene

Use a generic timing controller when several parts of a scene must share one
exact schedule. It is format-neutral: the same controller can coordinate a
physics simulation, geometry proof, chart, caption sequence, or ordinary
shapes.

Think of it as four small pieces:

| piece | responsibility |
|---|---|
| `timing` | declares the phase names, durations, exact total, and optional clock position |
| `timerstyle` | changes only the visible clock; it never changes scene timing |
| `timed` | runs the clock and schedules the complete phase contract |
| `during` | contains the ordinary animation actions for one named phase |

The phase declaration is the source of truth. The clock is only one visual view
of that choreography.

### Quick reference

| form | use |
|---|---|
| `timing(clock,"intro=1 demo=6 result=2")` | declare named phases |
| `timing(clock,(1160,80),"...")` | declare phases and set the initial clock position |
| `timing(clock,"duration=6")` | shorthand for one phase named `main` |
| `timerstyle(clock,"...")` | change appearance without changing timing |
| `timerstyle(clock,(1160,80),"...")` | restyle and reposition the clock |
| `timed(clock) { ... }` | play the complete phase schedule and clock |
| `during("phase") { ... }` | author one phase inside `timed` |
| `run(clock)` | play the clock alone |
| `countdown(id,[at],[seconds],["style"])` | independent countdown without named phases |

Core pattern, after defining the referenced entities and simulation:

```manic
timing(clock, (1160,80), "intro=1 demo=6 result=2");
timerstyle(clock, "look=ring number=inside color=cyan");

timed(clock) {
  during("intro")  { show(title, 0.6); }
  during("demo")   { par { run(sim, 6); draw(sim.path, 6); } }
  during("result") { show(answer, 0.6); }
}
```

Use `countdown` when you only need an independent visual countdown. Use
`timing` when named phases must govern other animation.

### Compose inside phases

Each `during` block accepts the usual timeline language. In particular, use
`par` for actions that must occupy the same phase together:

```manic
timing(clock, "setup=1 experiment=6 result=2");

timed(clock) {
  during("setup") { show(title, 0.6); }

  during("experiment") {
    par {
      run(pendulum, 6);
      draw(pendulum.path, 6);
      karaoke(caption, 0.35);
    }
  }

  during("result") {
    par {
      show(formula, 0.6);
      show(explanation, 0.6);
    }
    wait(1.4);
  }
}
```

### Rules that prevent drift

- Give a generic controller a fresh id. Do not reuse an entity, quiz,
  simulation, or group id.
- Put constructors and style modifiers outside `timed`/`during`; put timeline
  actions such as `show`, `draw`, `run`, `wait`, `par`, and `seq` inside.
- A short phase block is padded automatically. A block that exceeds its phase
  is an error.
- A phase may have at most one `during` block. Combine related work inside it
  with `par` or `seq`.
- An omitted phase is valid and becomes a blank hold.
- Phase blocks may appear in any source order: they are placed at the absolute
  offsets declared by `timing`.
- `timed(clock)` already runs the clock. Do not also call `run(clock)` inside it.
- `run(clock)` is timer-only playback. `run(clock, dur)` is rejected because
  the phase declaration already owns the duration.

### Choose the clock's look

The clock uses native manic shapes, so it stays sharp at any output size and
can be targeted like the rest of the scene. No SVG workflow is required.

| look | best fit |
|---|---|
| `ring` | neutral default; compact and familiar |
| `bar` | long processes or wide layouts |
| `segments` | energetic stages and presentations |
| `ticks` | precise, technical, or measurement-led scenes |
| `number` | minimal layouts where the value is enough |
| `pulse` | short, urgent moments; use sparingly |
| `none` | keep exact phase choreography without showing a clock |

The main controls are:

- `number=inside|outside|none` and `direction=fill|drain`
- `size=small|medium|large|0.5..2.0` and `thickness=0.4..3.0`
- `color`, `track`, `label`, and `font=mono|display`
- `finish=fade|hold|flash|pulse` for the completion cue

Start with the default ring, then change the look only when it supports the
scene's meaning. For legibility, keep strong contrast between `color` and
`track`, and avoid combining a busy clock with dense content in the same corner.

Advanced styling can target the stable tags `clock.timer`,
`clock.timer.track`, `clock.timer.progress`, `clock.timer.value`,
`clock.timer.label`, and `clock.timer.effects` (replace `clock` with the
controller id).

### Generic controller or Creator quiz?

The same visual clock system serves two different timing contracts:

- `timing(fresh_id, "phase=seconds ...")` creates a generic controller used by
  `timed` and `during`.
- `timing(quiz_id, "calm ask=... think=... reveal=...")` configures a Creator
  quiz and is played with `run(quiz_id)`.

Do not wrap a quiz in `timed`; the quiz runner already owns its ask, options,
think, reveal, hold, and stagger phases.

### Common fixes

| message or symptom | fix |
|---|---|
| controller id is already in use | choose a fresh id for generic timing |
| a phase overruns | shorten its sequence, compose simultaneous work with `par`, or increase the declared phase |
| unknown phase | match the name in `during` to the `timing` declaration |
| duplicate phase block | combine the work into one `during` block |
| competing duration | remove the duration argument from `run(clock, dur)` |
| timer appears twice | remove `run(clock)` from inside `timed(clock)` |

Complete non-quiz example using a pendulum, caption, formula, and segmented
clock:

```manic
{{#include ../../examples/timing-v2-scene.manic}}
```

<div class="manic-video" data-video="ex-timing-v2-scene"></div>

Next: the palette, glow, and easings → [Colour & style](colour.md).
