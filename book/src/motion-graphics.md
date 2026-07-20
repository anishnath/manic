# Motion graphics — move ideas, not layers

Most explanatory animation is not a sequence of disconnected scenes. A dot
becomes a data point, follows a curve, joins a ring, and then stops. Manic keeps
that object identity visible, while its timeline stays deterministic and freely
scrubbable.

## The small vocabulary that covers the common jobs

| Intent | Manic | What the viewer sees |
|---|---|---|
| Ambient life inside a region | `wander(group, dur)` | independent contained motion |
| Reorganize the same particles | `arrange(group, region, "random|grid|ring", dur, ease)` | a state change, not a crossfade |
| Move a real object along a path | `travel(marker, path, dur, ease)` | the marker arrives and remains at the endpoint |
| Send temporary emphasis along a path | `flow(path, dur)` | a pulse passes; the authored objects do not move |
| Preserve a path across ideas | `morph(from, to)` + `to(from, morph, 1, dur)` | one open path becomes another without a closing chord |
| Rotate or shear a whole group | `transform(tag, origin, a,b,c,d,dur,ease)` | every tagged object moves as one system |

`random` is seeded but not rigid. Each particle follows its own stable curved
route, so preview and export match exactly without the “assigned target on a
ruler” look. `grid` and `ring` remain direct, ordered layouts because their
structure is the point.

## Choose `travel` or `flow` deliberately

```manic
plot(curve, (180,620), 90, 140, "1-exp(-x)", (0,4));
dot(marker, (180,620), 7);

par {
  draw(curve, 2, out);
  travel(marker, curve, 2, out); // marker moves and stops at the end
  flow(curve, 1);                // temporary highlight only
}
```

Use `travel` when the moving thing has identity: a vehicle, probe, token,
particle, or graph marker. Use `flow` for energy, attention, traffic, or a signal
that should disappear after passing.

## Make arrival and rest both readable

Motion feels intentional when it has a settling frame. Arrange the objects,
give the group a short final turn, then leave a hold:

```manic
circle(orbit, (540,700), 240); hidden(orbit);

seq {
  arrange(dots, orbit, "ring", 1.1, smooth);
  transform(dots, (540,700), 0.966, -0.259, 0.259, 0.966, 0.5, out);
  wait(1.2); // the audience gets time to read the finished state
}
```

The matrix above is a 15° rotation. Use a small angle and `out` for a quick
decelerating settle; larger angles read as a spin rather than an arrival.

## Complete example

This scene combines organic particles, a persistent path marker, topology-safe
morphing, an ordered ring, and a brief final settle:

```manic
{{#include ../../examples/motion-graphics.manic}}
```

<div class="manic-video" data-video="ex-motion-graphics"></div>

Run it with:

```bash
manic examples/motion-graphics.manic
```

## Creator tips

- Run ambient motion in `par` with the explanation so it adds life without
  delaying the story.
- Use one persistent id across a change. Recreating the object loses continuity.
- Let important movement finish before the next sentence begins.
- Reserve `elastic` or `bounce` for playful subjects. `smooth` and `out` suit
  graphs, science, product demos, and professional explainers.
- Add a short `wait` after the final motion. Stopping is part of the animation.
