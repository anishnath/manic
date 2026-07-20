# Reactive stories — change the idea, not the scene

A polished Reel feels continuous: the equation changes, the point moves, the
graph responds, and the caption explains the same idea without cutting to a
blank screen. In Manic, build that as **one persistent visual world** and move it
forward with named `step` blocks.

This is the creator mental model:

1. Declare the cast once — equation, plot, diagram, labels, caption, profile.
2. Give each story beat a short, meaningful name.
3. Inside that step, describe only what changes.
4. Leave everything else alone; it remains on screen automatically.

The result is easier to write, easier to revise, and easier for a viewer's eye
to follow.

## One story, three layers

Think of a reactive video as three simple layers:

| layer | Manic vocabulary | creator decision |
|---|---|---|
| World | shapes, equations, plots, text, Creator profile | What should remain recognisable throughout? |
| Story | `step("question")`, `step("explain")`, `step("result")` | What changes in the viewer's understanding? |
| Motion | `rewrite`, `to`, `draw`, `say`, `show`, `fade` | How should that change become visible? |

`step` does not introduce a new kind of animation. It gives ordinary Manic
verbs a named conceptual boundary. Its children begin together, its duration is
the longest child, and any entity not mentioned persists.

```manic
step("measure-slope") {
  rewrite(work, `f'(x)=0.70x`, 0.90, smooth);
  to(tangent, x, 2.8, 3.20, smooth);
  to(rate, x, 2.8, 3.20, smooth);
  say(caption, "The tangent and its slope update together.", 0.40);
}
wait(0.60);
```

Here the formula, tangent, live value, and explanation share one beat. The
curve and every other unmentioned object remain exactly where they were.

## A creator-first workflow

### 1. Start with the promise

Before writing motion, write the story as four or five beat names:

```text
question → measure-slope → find-the-flat-point → see-the-derivative → takeaway
```

Use names that describe understanding, not implementation. `show-blue-line` is
fragile; `see-the-derivative` still makes sense after the design changes.

For a short educational Reel, a reliable shape is:

- **Hook:** pose one visual question in the first two seconds.
- **Explore:** change one or two connected representations.
- **Payoff:** make the important relationship visible.
- **Takeaway:** hold one memorable sentence or result.

The same structure works for geometry proofs, physics simulations, chemistry
reactions, data stories, algorithms, and product explainers. Reactive is a
story pattern, not a mathematics-only feature.

### 2. Declare the world once

Give every continuing visual a stable id. Do not create `equation1`,
`equation2`, and `equation3` when the viewer should perceive one evolving
equation. Keep `work` and rewrite it.

```manic
equation(work, (cx, 390), `f(x)=0.35x^2`, 51);
plot(curve, (cx, 1050), 115, 62, "0.35*x*x", (-3.5, 3.5));
text(caption, (cx, 1450), "How steep is this curve?");
```

Stable identity is what makes continuity possible. It also means a later style
change, position adjustment, or wording fix happens in one place.

### 3. Change only what matters

Inside a step, pair the representations that explain one another:

```manic
step("find-the-flat-point") {
  rewrite(work, `f'(0)=\textcolor{lime}{0}`, 0.85, smooth);
  to(tangent, x, 0, 1.80, smooth);
  to(rate, x, 0, 1.80, smooth);
  show(vertex, 0.40);
  say(caption, "At the vertex the tangent is flat.", 0.40);
}
```

Do not fade and rebuild the whole screen for every sentence. Preserve context,
move the smallest meaningful parts, and let the viewer compare before and
after. For equations, `rewrite` retains safely matched LaTeX pieces and brings
only the changed pieces in or out.

Manic animates the states you author; it does not solve or verify the maths.
Write each intermediate LaTeX state exactly as it should appear.

### 4. Give the result room to land

A step ends when its longest animation ends. Add a short `wait` after an
important step so the viewer can read the settled frame:

```manic
step("result") {
  rewrite(work, `\text{slope of }f=\textcolor{magenta}{f'}`, 0.95, smooth);
  pulse(curve, 0.70);
  pulse(derivative, 0.70);
  say(caption, "The derivative is the curve of all local slopes.", 0.40);
}
wait(1.80);
```

Use `seq { ... }` inside a step only when that conceptual beat genuinely needs
an internal order. Use `par` for anonymous choreography; use `step` when the
beat should have a name and meaning.

## Creator polish that matters

- Keep one focal change per step. Several synchronized views are fine when they
  all explain that one change.
- Reuse semantic colours: for example, cyan for the original function,
  magenta for its derivative, and lime for the key result.
- Put captions in a stable safe region. Let the words change with `say` instead
  of making the viewer search for a new text block.
- Prefer smooth movement and local equation rewrites over full-screen fades.
- Write for the final phone size. Short captions and one- or two-line equations
  beat dense slides.
- Keep the Creator profile and social footer consistent across a series, but do
  not let branding compete with the explanatory stage.
- Preview the hook, busiest transition, result, and end hold as still frames
  before recording the full video.

## Named steps are editing markers

Every step name is automatically exported at the step's start in
`markers.json`. Those semantic timestamps make it easier to:

- align narration, music, captions, and sound effects;
- review or seek directly to a conceptual beat;
- identify a clean hook, explanation, result, or takeaway;
- prepare the same story for future multi-format publishing tools.

Names must be non-empty, unique, and top-level. If you only need a timestamp
without a reactive state change, use `mark("name")` instead.

## Publish one story in four formats

A named-step story can now be reframed at render time without editing its
source. Use `--canvas` to override the logical canvas before Manic calculates
`w`, `h`, `cx`, `cy`, macros, and build-time layout branches:

```sh
manic examples/reactive-multiformat.manic --canvas portrait --record out-reel --preset reel
manic examples/reactive-multiformat.manic --canvas 4:5     --record out-feed
manic examples/reactive-multiformat.manic --canvas square  --record out-square
manic examples/reactive-multiformat.manic --canvas 16:9    --record out-lesson
```

The story's named steps, equation continuity, timing, and identity remain the
same. Only the logical layout changes. `--canvas` is separate from `--preset`:
the canvas controls composition; the preset controls recording quality, frame
rate, container, and engine branding.

Creator Kit regions already read the active canvas. For a hand-composed scene,
prefer `w`, `h`, `cx`, and `cy`, then use a small build-time branch when the
composition should genuinely change shape:

```manic
if h > 1.45*w {
  // vertical stack for a Reel or Short
}
else if w > 1.25*h {
  // explanation left, visual stage right
}
else {
  // compact square / 4:5 feed composition
}
```

An override cannot make hard-coded coordinates responsive by itself. The
author still decides the useful composition for each shape of screen; Manic
ensures every branch receives the correct dimensions before construction and
keeps the semantic story unchanged.

## Complete examples to learn from

- [Reactive world](ex-calculus.md#reactive-world) — the best compact starting
  point: one curve, one equation, one tangent, and five named steps.
- [Reactive multi-format](ex-creator.md#reactive-multiformat) — one source and
  one named timeline, with vertical, feed, square, and landscape compositions.
- [Parameter journeys](ex-creator.md#parameter-journeys) — one visible value
  drives a plot, analysis views, geometry, and a live readout without rebuilding
  the world.
- [Reactive integral](ex-calculus.md#reactive-integral) — an equation, integrand,
  antiderivative, and visual verification in one continuous world.
- [Reactive notation](ex-transforms.md#reactive-math-notation) — mathematics,
  physics, chemistry, biology, logic, and custom LaTeX notation.
- [Math journey](ex-transforms.md#reactive-math-journey) — a playful long-form
  progression from Class 1 arithmetic to PhD-level notation.
- [Quadratic continuity](ex-transforms.md#quadratic-formula-continuity) — a
  focused benchmark for retaining unchanged equation pieces.

Start by copying `reactive-world.manic`, rename its steps around your idea, and
replace one representation at a time. Keep the world; change the understanding.

Next: [Review every format before publishing →](creator-visual-checks.md)
