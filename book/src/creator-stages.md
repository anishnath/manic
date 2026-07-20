# Story stages — edit the idea, not the timestamp

A creator usually thinks in beats—**question**, **intuition**, **experiment**,
**proof**, **takeaway**—rather than `6.35 seconds`. Manic uses the names already
written in `step(...)` as the editing and publishing structure for the movie.
There is no second timeline file to maintain.

```manic
step("question") {
  show(prompt, 0.4);
  say(caption, "Where does the slope become zero?", 0.35);
}
wait(0.8);

step("experiment") {
  to(tangent, x, 0, 1.8, smooth);
  to(rate, x, 0, 1.8, smooth);
}
wait(0.7);

step("takeaway") {
  rewrite(work, `f'(0)=0`, 0.8, smooth);
}
wait(1.5);
```

The wait after a step belongs to that stage. This makes a stage export include
the transition **and** the time the viewer needs to absorb its settled result.
The next stage begins exactly where its next `step` begins.

## See the story before opening a window

```sh
manic stages examples/reactive-world.manic
```

The report lists every stage's start, end, and complete duration:

```text
stages — examples/reactive-world.manic (5 stages, 13.55s authored)

  #  stage                  start      end      duration
  1  question                0.00s     2.15s      2.15s
  2  measure-slope           2.15s     5.95s      3.80s
  3  find-the-flat-point     5.95s     8.40s      2.45s
  4  see-the-derivative      8.40s    10.80s      2.40s
  5  takeaway               10.80s    13.55s      2.75s
```

Use this before recording to catch a rushed hook, an overlong setup, or a
takeaway without enough reading time.

## Preview one stage

```sh
manic examples/reactive-world.manic --stage find-the-flat-point
```

Manic evaluates the complete persistent world at the stage's real start, then
limits playback and scrubbing to that stage. Nothing is reconstructed as a
separate scene, so incoming equation, plot, diagram, and parameter state remain
correct.

The live player adds a stage strip above the transport bar:

- click a stage segment to jump to it;
- use `1`–`9` to jump to visible stages;
- press `R` to restart at the selected range's beginning;
- drag the progress bar to scrub only inside the selected range;
- read the current stage and selected range in the HUD.

## Record a single stage

The same flag selects the recording range:

```sh
manic examples/reactive-world.manic \
  --stage see-the-derivative \
  --record out-derivative \
  --preset reel
```

For a clean editing clip without the preset's branding intro/outro:

```sh
manic examples/reactive-world.manic \
  --stage see-the-derivative \
  --record out-derivative \
  --preset test \
  --no-brand
```

## Export several stages

Use an inclusive named range when a clip should carry a short story arc:

```sh
manic examples/reactive-world.manic \
  --from-stage measure-slope \
  --to-stage takeaway \
  --record out-slope-to-takeaway
```

`--to-stage takeaway` includes the whole takeaway stage. With only
`--from-stage`, recording continues through the authored ending. With only
`--to-stage`, it begins at the start of the movie.

Named ranges and numeric `--from`/`--to` are intentionally separate. Use stage
names for story editing and numeric seconds only for a deliberately precise
technical trim. Unknown names report the available stages and suggest a close
match.

## Recording metadata stays useful

`markers.json` now includes:

- the selected source range;
- clipped stage intervals with relative `t`, `end`, and `duration`;
- each stage's original `source_t`;
- marks and sections filtered to the exported clip and shifted to clip time.

That means narration, captions, music cues, and downstream editing tools can
work from zero-based clip timing without losing the position in the source
story.

## Combine stages with output formats

Stage selection and canvas selection are independent:

```sh
manic examples/parameter-journeys.manic --stage takeaway --canvas portrait  --record out-reel
manic examples/parameter-journeys.manic --stage takeaway --canvas square    --record out-square
manic examples/parameter-journeys.manic --stage takeaway --canvas landscape --record out-lesson
```

The named idea stays the same while the responsive layout changes. Before the
batch render, run
`manic check examples/parameter-journeys.manic --canvas all` to review every
stage in every supported shape.

## Choosing useful names

- Name the change in understanding: `see-the-derivative`, not `show-pink-line`.
- Keep a short video to roughly three to six stages.
- Put several synchronized changes inside one stage only when they explain the
  same idea.
- Add a deliberate `wait` after the important result.
- Use `mark("cue")` for an editor timestamp that is not a story stage.
- Use `section("Part Two")` when the video itself needs a visible chapter card.

Start with [Reactive world](ex-calculus.md#reactive-world), run
`manic stages examples/reactive-world.manic`, and preview its
`find-the-flat-point` stage.

Next: [Review every format before publishing →](creator-visual-checks.md)
