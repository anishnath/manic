# Verbs — bringing it to life

**Verbs are the script.** Each one names an entity and animates it. They run
**top-to-bottom, one after another** (use [`par`](timing.md) for simultaneous).

Almost every verb takes two optional trailing arguments:

```manic
move(sun, (900, 400), 0.8, smooth);
//                     ^dur  ^easing
```

- **`[dur]`** — how long, in seconds (there's a sensible default).
- **`[ease]`** — the motion curve: `linear`, `smooth`, `in`, `out`, `back`,
  `bounce`, `elastic`, `spring` (see [Colour & style](colour.md#easings)).

## Reveal & hide

```manic
draw(sun, 1.2);   // -> trace a stroke on (needs `untraced` first)
erase(sun);       // -> the reverse: un-draw it
show(cap, 0.5);   // -> fade in (needs `hidden` first)
fade(cap);        // -> fade out
type(cap);        // -> typewriter: reveal text character by character
```

<div class="watch">▶ Watch: reveal</div>
<div class="manic-video" data-video="reveal"></div>

## Attention

```manic
flash(sun, cyan);  // -> flash to a colour, then restore
pulse(sun);        // -> quick grow-and-settle "look here"
shake(sun);        // -> horizontal shake, an "error/no" gesture
spin(sun, 360);    // -> spin about its centre
```

## Motion

```manic
move(p,  (900, 400));   // -> glide to an absolute point
shift(p, (0, -120));    // -> move by a delta (relative)
scale(r, 1.4);          // -> animate uniform scale to 1.4x
rotate(r, 45);          // -> rotate by 45 degrees
grow(arrow, (500, 200));// -> animate a line/arrow endpoint (draws or retargets)
```

<div class="watch">▶ Watch: motion</div>
<div class="manic-video" data-video="motion"></div>

## Content & colour

```manic
say(cap, "next step");     // -> crossfade a text entity to new words
recolor(sun, lime, 0.5);   // -> permanently animate the colour
```

<div class="watch">▶ Watch: text</div>
<div class="manic-video" data-video="text"></div>

## The escape hatch — `to` / `set`

Named verbs are shortcuts. `to` animates *any* single property, for whatever
isn't pre-named:

```manic
to(sun, opacity, 0.3, 0.5);   // animate opacity to 0.3 over 0.5s
to(sun, rot, 90);             // rotation to 90 degrees
```

Properties: `pos`, `color`, `opacity`, `scale`, `rot`, `trace`, `hue`.

## Move the camera

```manic
cam((300, 200), 1.0);   // pan the camera centre
zoom(1.6, 0.8);         // zoom to 1.6x
```

The camera moves the whole world, so a caption or counter would slide off with it.
Mark it `sticky(id)` to pin it to the screen and keep it readable through the move.

## One verb, a whole group

If a name refers to a **tag** (a group) instead of a single entity, the verb
runs on *every* member at once. This is how you animate a whole graph, table, or
loop-generated set in one line:

```manic
hidden(g.nodes);     // every node, at t=0
draw(g.edges);       // trace every edge on together
flash(a.cells, cyan);// flash all array cells
```

More on grouping in the [Kits](kits.md) chapter. Next, control *when* things
happen → [Timing](timing.md).
