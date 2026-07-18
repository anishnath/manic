# Elevating a scene

A one-line sim already animates:

```manic
spring(sp); run(sp, 8);
```

That's the *minimal* scene — correct, but bare. An **elevated** scene turns the
same sim into a narrated lesson: parts revealed one at a time, each labelled, the
governing law stated on screen, then the motion played across several synchronized
views.

The key fact that makes this possible: **a sim's parts are ordinary manic
entities.** `spring(sp, …)` lays out `sp.wall`, `sp.spring`, `sp.mass`, `sp.path`,
plus the tag `sp.parts` over all of them. Every base verb, modifier, and
annotation addresses those ids directly — there is no separate "physics mode".
So you elevate a scene with the vocabulary you already know.

## The three moves

### 1 · Stage — hide the parts, then reveal them in order

Build the sim, then hide each part (and any extra view) so you can bring them in
deliberately. `untraced` on a path keeps it in the scene at zero draw-progress so
you can `draw` it on later.

```manic
spring(sp, (360,300), 10, 1.4, 110);
hidden(sp.wall); hidden(sp.spring); hidden(sp.mass); hidden(sp.overlays);
untraced(sp.path);
well(sp, (1010,230), 120); hidden(sp.well);   // an extra "reading", revealed later
```

### 2 · Annotate — name the parts with base entities

Point at things with `text` + leader `arrow`, mark a length or displacement with
`bracelabel`, drop a reference `line`, and state the law with a `text`. Create
them all now, hidden, so the reveal order is yours to choreograph.

```manic
text(coilL,(300,205),"spring, stiffness k"); color(coilL,lime); display(coilL); hidden(coilL);
bracelabel(xb,(360,352),(514,352),"x₀",22);  color(xb,gold); hidden(xb); hidden(xb.label);
text(hooke,(360,150),"restoring force  F = −k·x"); color(hooke,gold); display(hooke); hidden(hooke);
```

### 3 · Choreograph — chapters, emphasis, then run

Narrate with `section` + `say`, reveal with `show`/`draw`, emphasize with
`flash`/`pulse`/`recolor`, `fade` the clutter before the motion, then `run`.

```manic
section("Hooke's law");
say(cap, "pull it x₀ from equilibrium — it pulls straight back, F = −k·x", 0.4);
show(sp.spring, 0.4); flash(sp.spring, gold); show(xb, 0.4); show(xb.label, 0.4);
wait(0.8);
fade(xb, 0.3); fade(xb.label, 0.3); fade(hooke, 0.3);   // declutter

section("Motion");
show(sp.overlays, 0.4); show(sp.well, 0.5); draw(sp.path, 0.6);
run(sp, 10);        // sim, overlays, well ball, energy sweep — all animate together
```

## The lever kit

Every one of these is base vocabulary that works on **any** entity — a sim part,
a shape, a label — so the same kit elevates a physics sim, a geometry
construction, or an algorithm trace.

| Lever | Builtins | Buys you |
|---|---|---|
| Stage the parts | `hidden(id.part)`, `untraced(id.path)`, then `show`/`draw` | reveal piece-by-piece instead of all at once |
| Narrate (light) | `say(cap,"…",dur)`, a small `text` kicker updated with `say`, `wait(dur)` | chapter the story **without** covering the stage |
| Typewriter | `type(id,dur)` + `cursor(id)` (set the string with `say(id,"…",0.1)` first) | a lab-note / terminal feel that types itself out |
| Live data | `counter(id,(x,y),start,decimals,"pre","suf")` + `to(id, value, target, dur)` | a number that ticks up (k, period, acceleration) |
| Camera | `cam((x,y),dur,ease)` + `zoom(factor,dur,ease)` | push in on a part, then pull back — cinematography |
| Pin a HUD | `sticky(id)` | keep a caption / counter fixed on screen while the camera `cam`/`zoom`s the world |
| Kinetic type | `caption(id,"…",(x,y),size,color)` + `wordpop(id,dur)` / `karaoke(id,dur,color)` | words pop in, or a highlight sweeps across them |
| Name the parts | `text`, leader `arrow`, `bracelabel`, reference `line` | say what each thing is |
| Emphasize | `flash(id,color)`, `pulse(id)`, `recolor(id,color,dur)`, `glow(id,amt)`, `shake(id)`, `spin(id,deg,dur)` | move the eye to the part being discussed |
| Add ambient life | `particles(id, circle_or_rect, count, radius, seed)` + `wander(id,dur)` | contained bubbles, dust, stars, data, or molecules without hand-animating dots |
| Show a transfer | `link(id,a,b,bend)` + `flow(id,dur)` | a tracked curved connection and a travelling emphasis pulse for energy, signals, traffic, or attention |
| Broadcast | `flash(id.parts, lime)` | hit **every** part of the sim at once |
| State the law | a `text` with the formula | tie the motion to the equation |
| Multiple readings | reveal `well` / `phase` / `timegraph` / `energygraph`, then `run` | one motion, several synchronized views |
| Declutter | `fade(...)` the static annotations | clean playback |

> **Careful with `section("…")`.** It drops a full-screen title *card* (an 820×240
> backdrop over the whole stage) and holds it ~2.2 s. Two or three of those and
> the interstitials start to *bury* the animation you're trying to show. Reach for
> it only for a genuine hard scene change; for beat-to-beat chapters prefer a
> lightweight persistent kicker/`caption` updated with `say`, or let the
> typewriter / camera / kinetic-type do the pacing. **Vary the lens per scene** —
> if every lesson uses the identical `section`+`say`+`show` loop, the medium looks
> one-note; the point of the kit above is that manic has range.

## Reveal-order gotchas

- **`show`/`fade` force opacity to 1 / 0.** They will override a deliberately
  faint entity (a dim reference line, a panel frame). To bring in something that
  should stay faint, `draw` it (traces the stroke, leaves opacity alone) or just
  `display` it faint from the start — don't `show` it.
- **`draw` is the nicest reveal for strokes** — lines, arrows, curves, and paths
  sketch themselves on. Set the entity `untraced` first, then `draw(id, dur)`.
- **`fade` the annotations before the motion** so the swing plays against a clean
  stage.
- **Loops (`+timeline`) reference only ids that exist** — build every part before
  the script that animates it.

## Worked examples

The gallery pairs each elevated lesson with its minimal reference — and each one
leads with a **different** lens, to show the range:

- **[Anatomy of a Spring](ex-physics.md)** (`spring-annotated`) — *typewriter +
  live data*: a `type`d lab-note with a `cursor`, and `counter`s that tick k and
  the period up, over Hooke's law → parabolic well → SHM.
- **[The Atwood Machine](ex-physics.md)** (`pulley-annotated`) — *camera*: `cam` +
  `zoom` push in on the masses for the imbalance beat, then pull back to release;
  the caption + counter are `sticky`, so they stay pinned through the zoom.
- **[Brachistochrone race](ex-physics.md)** (`brachistochrone-annotated`) —
  *kinetic typography* on a multi-body sim: `wordpop` the question, `karaoke` the
  path names as the curves sketch on, then crown the cycloid.

Notice none of them use `section` — the pacing comes from the lens instead, so the
motion is never hidden behind a card.

The three-move recipe is domain-agnostic: it elevates anything with addressable
parts, not just the physics kit.
