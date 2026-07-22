# Motion graphics — move ideas, not layers

Motion Graphics V2 is built around continuity. Keep one object alive while it
moves, follows another object, changes visual form, joins a layout, or turns as
part of a system. The timeline remains deterministic and freely scrubbable.

There is no V2 mode or production flag. Write an ordinary `.manic` file and use
the relationship only where the story needs it.

## The complete everyday vocabulary

| Creator intent | Manic | Result |
|---|---|---|
| Keep one object beside another | `attach(child, target, [(dx,dy)])` | the child follows every resolved target position |
| Stop following | `attach(child, none)` | the child releases at the settled position without a second verb |
| Change visual identity | `become(source, blueprint, [dur], [ease])` | the source keeps its id and settles exactly on the blueprint |
| Turn a whole arrangement | `turn(id_or_tag, pivot, degrees, [dur], [ease])` | every member follows the same circular pivot motion |
| Move a real object on a path | `travel(object, path, dur, ease)` | the object arrives and remains at the endpoint |
| Send temporary or sustained path emphasis | `flow(path, dur, [forward|reverse|both], [once|continuous])` | a clean directional pulse or finite draining stream passes without moving an authored object |
| Add contained ambient life | `wander(particles, dur)` | seeded, repeatable motion within the container |
| Reorganize the same particles | `arrange(particles, region, "random|grid|ring", dur, ease)` | identity-preserving layout change |
| Change an ordinary property | `to(id, property, value, dur, ease)` | the general escape hatch remains available |

The three V2 words add relationships; they do not replace `move`, `travel`,
`flow`, `arrange`, `spin`, `transform`, `rewrite`, or `morph`.

## Manic does not infer the subject

An icon, label, or filename never changes motion semantics. Manic does not know
that an object represents a load balancer, queue, topic, photon, vehicle, blood
cell, or decorative spark. The creator supplies that meaning through ordinary
composition:

```manic
// One selected route
par { travel(packet, lane2, 1.2, smooth); flow(lane2, 1.2); }

// Three authored deliveries together
par {
  travel(copy1, lane1, 1.0, smooth);
  travel(copy2, lane2, 1.0, smooth);
  travel(copy3, lane3, 1.0, smooth);
}

// A path connected to nothing—motion used purely as design
flow(ribbon, 4.0, both, continuous);
```

`seq` creates an authored order. `par` creates simultaneous motion. `travel`
moves any ordinary 2-D entity and preserves its identity. `flow` requires only
a path; it does not require endpoints, architecture metadata, or an object to
carry. Tags let one `flow` address several paths when the creator wants them to
act together.

## `attach` — author the relationship

```manic
dot(marker, (180,620), 8);
text(readout, (180,580), "sample A");
plot(curve, (180,620), 90,140,"1-exp(-x)",(0,4));

attach(readout, marker, (0,-40));
travel(marker, curve, 2, smooth);
attach(readout, none); // release at the settled endpoint
```

The child follows after normal tracks, reactive bindings, derived geometry,
links, particle layouts, and path travel have resolved. Its opacity is
multiplied by the target opacity, so a label naturally disappears with the
thing it explains.

Use an offset to keep the child readable. Release at the end of a movement,
then `move`, `fade`, or reuse the child normally. Attachment cycles are rejected
while building the movie rather than failing during rendering.

## `become` — preserve identity across a visual change

Declare the destination like any other entity and hide it when it is only a
blueprint:

```manic
circle(seed, (220,700), 16); color(seed, cyan);
circle(node, (820,700), 62); color(node, magenta);
outlined(node); stroke(node, 7); hidden(node);

become(seed, node, 0.9, smooth);
```

Compatible geometry interpolates continuously, including circle, rectangle,
line, arrow, curve, coil, arc, and equal-topology polygon/polyline pairs.
Unsupported pairs use a local fade/swap/fade instead of producing broken
geometry. Both paths settle on the exact target geometry and styling while the
source id remains alive. A hidden blueprint does not make the transformed
source disappear, and the blueprint's own visibility is never changed.

Use `rewrite` for equations because it understands matching LaTeX visual parts.
Use `morph` plus `to(..., morph, ...)` when you explicitly need a fraction-driven
point morph or winding angle.

## `turn` — rotate the system, not every member

```manic
circle(orbit, (540,700), 220); hidden(orbit);
particles(dots, orbit, 16, 7, 42, "ring");

turn(dots, orbit, 24, 0.65, out);
```

The first argument may be one entity or a tag. The pivot may be a point or an
entity. Positions follow circular paths, path endpoints and curve controls turn
with their paths, and ordinary shapes retain their group-local orientation.
Because the target is resolved from the latest authored state, `turn` composes
after `move`, `travel`, `arrange`, or an earlier `turn` without snapping back.

Use `spin` for one object's in-place rotation. Use `transform` when the actual
idea is a matrix, shear, reflection, or other precise linear map.

## `travel` versus `flow`

```manic
par {
  draw(curve, 2, out);
  travel(marker, curve, 2, out); // the marker moves and stays
  flow(curve, 1);                // temporary travelling emphasis
}
```

Use `travel` for a vehicle, probe, token, particle, or graph marker. Use `flow`
for energy, attention, traffic, or a signal that should disappear after passing.
The default remains one forward pulse. For sustained activity use
`flow(curve, 4, forward, continuous)`; Manic chooses length-aware complete
cycles, so the stream begins empty and drains cleanly at the end. Use
`flow(curve, 1, reverse, once)` for generic reverse motion and `both` for two
independent opposing streams.

## Motion-flow foundation example

This example proves the same vocabulary across four different intentions: one
selected path, three motions authored in order, three motions authored together,
and a free spline used only as visual design.

```manic
{{#include ../../examples/motion-flow-foundation.manic}}
```

Run it directly or audit every target format:

```bash
manic examples/motion-flow-foundation.manic
manic check examples/motion-flow-foundation.manic --canvas all
```

## Complete V2 example

This generic Reel demonstrates all three relationship words, release, path
travel, an identity-preserving blueprint change, particle arrangement, a shared
pivot turn, and a readable final hold:

```manic
{{#include ../../examples/motion-graphics-v2.manic}}
```

<div class="manic-video" data-video="ex-motion-graphics-v2"></div>

Run it directly—no extra runtime option is required:

```bash
manic examples/motion-graphics-v2.manic
```

## Advanced story — compose the whole motion language

The compact example above answers “what do the three new words do?” This
advanced Reel answers the more important creator question: “how do they form a
story with the motion vocabulary I already know?”

It keeps one visual world alive across three acts:

1. A question travels while its label stays attached; a path pulse guides the
   eye and surrounding facts wander.
2. The notation rewrites, the same question becomes a model, and the same facts
   arrange into a system. The model then spins locally.
3. Labels and particles turn around one shared pivot while the equation reaches
   the story's final meaning.

`seq` creates cause and effect, `par` groups changes that express one idea, and
`stagger` prevents a dense scene from arriving as a visual shock.

```manic
{{#include ../../examples/motion-graphics-v2-story.manic}}
```

<div class="manic-video" data-video="ex-motion-graphics-v2-story"></div>

Run the advanced story through the same file-only production path:

```bash
manic examples/motion-graphics-v2-story.manic
```

The original essentials example remains useful when learning `wander`,
`travel`, explicit `morph`, and particle arrangement:

```manic
{{#include ../../examples/motion-graphics.manic}}
```

<div class="manic-video" data-video="ex-motion-graphics"></div>

## Professional motion checklist

- Author relationships and final states; avoid hand-keyframing intermediate
  coordinates.
- Use `par` for changes that belong to one idea and `seq` for cause-and-effect.
- Prefer `smooth` for explanatory transformations and `out` for a short settle.
- Keep an important object id alive instead of fading it out and rebuilding it.
- Leave a final `wait` so the audience can read the state motion created.
- Preview by named `step` and scrub backwards: the same time must always produce
  the same frame.
