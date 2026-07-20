# Create a polished Reel

Creator Kit v2 is the fastest path from an idea to a finished vertical video.
It owns the responsive layout, safe areas, question hierarchy, answer cards,
timer, reveal, creator footer, and end card. You provide the content and choose
how restrained or energetic it should feel.

This chapter gives one dependable production recipe. Start here, then use the
[Creator reference and examples](ex-creator.md) when you need every option.

## The production recipe

| step | recommended starting choice | why |
|---|---|---|
| Format | `canvas("9:16")` | native vertical composition |
| Surface | omit the call or use `template("mono")` | professional black-and-white default; use `shorts` when accent hue matters |
| Safe area | `safe=reels` on the creator and quiz | protects important content from platform UI |
| Hierarchy | `studio layout=media-first density=comfortable` | question first, one focal visual, readable answers |
| Motion | `motion=calm` or `motion=studio` | purposeful movement without visual noise |
| Timing | 9–13 seconds for one quiz beat | enough time to read, think, and absorb the reveal |
| Close | signature footer plus one end-card CTA | consistent identity and one clear next action |

For Shorts or TikTok, change both safe-area declarations to `safe=shorts` or
`safe=tiktok`. The same source can also be reframed for `4:5`, `square`, or
`16:9`; the Creator regions reflow automatically. Use `--canvas 4:5`,
`--canvas square`, or `--canvas 16:9` at preview/record time so the file stays
unchanged. The [Reactive stories](creator-reactive.md#publish-one-story-in-four-formats)
chapter shows the full workflow and the responsive manual-layout pattern.

## 1. Write for a phone, not a slide

A strong first Reel usually has:

- one question or promise that fits in one or two lines;
- one visual idea—an equation, diagram, image, chart, or simulation;
- three or four short answer choices;
- one accent colour and one motion personality;
- a brief explanation and a single call to action.

Prefer direct wording. Move supporting context into `explain`; do not make the
viewer read a paragraph before the timer starts. Use LaTeX for mathematical
notation so formulas stay crisp and compact.

### Gold pattern: let one equation evolve

For a solution Reel, do not stack five complete formulas or cut to a blank
screen between steps. Declare one equation and rewrite it in place:

```manic
equation(work, (cx, 520), `x^2+2x=3`, 54);
rewrite(work, `x^2+2x+1=4`, 0.85, smooth);
wait(0.8);
rewrite(work, `(x+1)^2=4`, 0.85, smooth);
```

The viewer's eye can follow the terms that move while unchanged symbols stay
put. Manic matches the rendered LaTeX; you still supply and own every correct
step. For phone video, use one meaningful rewrite per narration beat, keep the
equation in the media safe region, pause after the important result, and place
any related plot or diagram motion beside the rewrite in `par`.

## 2. Let the format own layout

```manic
canvas("9:16");
template("mono"); // optional: mono is the DSL default

creator(me, "@anish2good name=Proof_Daily tagline=Think_then_prove \
yt=zarigatongy x=@anish2good web=8gwifi.org/manic \
accent=cyan secondary=magenta footer=signature cta=Save_and_share safe=reels");

quiz(q, "Your short question",
     "studio layout=media-first density=comfortable motion=calm safe=reels accent=cyan");
```

Use `layout=media-first` when the visual is part of the question, `stack` for up
to four text-led answers, `grid` for four to six compact answers, and `auto`
when you want manic to decide. Pass the visual to `figure(id)` instead of tuning
its coordinates for each aspect ratio.

Temporarily add `safezone(guide,"reels")` while previewing a new design. It is a
visible diagnostic overlay, so remove it before the final record. The
`safe=reels` layout setting should remain.

## 3. Give reading, thinking, and payoff separate beats

The default `run(q,10)` is a good first draft. For a repeatable series, declare
the exact beat:

```manic
timing(q, "calm ask=1.1 options=1 think=5.5 reveal=0.75 hold=2.15 stagger=0.06");
timerstyle(q, "look=ring position=below direction=drain color=cyan finish=pulse");
run(q);
```

A useful starting range is 1–1.5 seconds for the hook, 4–6 seconds to think,
0.6–0.9 seconds for the reveal, and 1.5–2.5 seconds to hold the explanation.
Adjust for the actual reading load; the right pace is the one that remains clear
on a phone.

`timing` owns choreography and `timerstyle` owns appearance. Change the ring to
`bar`, `segments`, `ticks`, `number`, `pulse`, or `none` without moving a single
beat. With explicit phases, always use `run(q)`—adding `run(q,dur)` would create
a second competing duration.

Design shortcut: keep the default ring for most educational Reels. Use a bar
for a longer process, segments for staged challenges, ticks for technical work,
and pulse only for genuinely urgent moments.

## 4. Build a recognisable close

Call `socials(profile)` before the main animation so the selected footer is
present throughout. `endcard(profile)` creates a hidden final lockup; fade the
content, then reveal it for about one second or more.

Keep the CTA singular: “Save this”, “Try the next one”, or “Follow for more”. A
creator profile is reusable across a whole series, so brand decisions live in
one line rather than every scene. Inside a profile spec, use underscores where
a value needs spaces: `name=Proof_Daily cta=Save_and_share`.

For a channel-link row without external assets, choose `footer=social` and set
the platform values directly: `yt=zarigatongy x=@anish2good
web=8gwifi.org/manic`. Manic draws matching normalized icons and keeps the text
inside the responsive footer. Use two or three identities for the clearest
phone-size close; a larger platform set automatically becomes icon-only.

## Complete Reel

This example uses exact phases, LaTeX media, the Reels safe area, a restrained
native timer, a signature footer, timeline markers, and a final end card:

```manic
{{#include ../../examples/perfect-reel.manic}}
```

<div class="manic-video" data-video="ex-perfect-reel"></div>

## Preview and export

```sh
manic check examples/perfect-reel.manic
manic check examples/perfect-reel.manic --canvas all
manic examples/perfect-reel.manic

# Inspect representative phone frames: hook, countdown, and answer hold.
manic examples/perfect-reel.manic --still 0.8
manic examples/perfect-reel.manic --still 6.0
manic examples/perfect-reel.manic --still 10.0

# Vertical render with only your Creator profile branding.
manic examples/perfect-reel.manic --record out --preset reel --no-brand
```

Omit `--no-brand` if you also want the Manic intro and watermark. Recording
writes `markers.json`; use its `mark(...)` timestamps to align narration, music,
or captions in an editor.

## Final phone-size review

Before publishing, check:

- Does `manic check your-file.manic --canvas all` pass portrait, feed, square,
  and landscape? See [Visual checks](creator-visual-checks.md) for fixes.
- Can the hook be understood during the first beat?
- Is every important word and the CTA inside the selected safe area?
- Is there one obvious focal point, with no competing motion?
- Are the choices short, distinct, and marked with exactly one correct answer?
- Does the timer contrast with its track without overpowering the content?
- Does the explanation remain visible long enough to read aloud?
- Does the final card ask for one action and hold long enough to register?
- Are logos, photos, fonts, narration, and music yours or licensed for use?

Once those pass, keep the same profile, motion, colour, and closing pattern
across the series. Consistency will make the next Reel both faster to produce
and easier to recognise.

Next: [Turn one polished scene into a reactive story →](creator-reactive.md)

Reference: [Explore every Creator control and format variant](ex-creator.md).
