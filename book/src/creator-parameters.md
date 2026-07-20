# Parameter journeys — one value, many connected views

A parameter journey shows how an idea behaves across a family of cases. Instead
of rebuilding the scene for “negative”, “zero”, and “positive”, declare one
visible value, connect it to the existing world once, and animate only that
value.

```manic
parameter(a, (cx, 140), -1.2, -1.5, 1.5, "a", 2);

plot(curve, (cx, 720), 110, 58, "x*x", (-3.3,3.3));
circle(point, (cx, 1200), 22);
counter(magnitude, (cx, 1360), 0, 2, "a² = ", "");

bind(a, curve, formula, "0.22*p*x*x");
bind(a, point, x, w*0.18, w*0.82);
bind(a, point, scale, "0.78+0.22*abs(p)");
bind(a, magnitude, value, "p*p");

step("flatten") {
  to(a, value, 0, 2.2, smooth);
}
step("opens-up") {
  to(a, value, 1.25, 2.4, smooth);
}
```

The plot, point, scale, number, and native parameter control move on the same
continuous clock. Nothing is replaced and no scene construction is duplicated.

## The three-part mental model

| part | vocabulary | creator decision |
|---|---|---|
| Expose | `parameter` | Which value should the viewer follow? |
| Connect | `bind` | Which representations explain that value? |
| Travel | `step` + `to(parameter,value,…)` | Which cases tell the clearest story? |

This pattern is domain-neutral. The parameter can be a quadratic coefficient,
damping factor in a formula plot, probability, sample-size readout, geometric
position, opacity comparison, colour phase, or any other scalar that makes the
idea easier to see.

## Declare the visible parameter

```manic
parameter(id, (x,y), initial, min, max, ["label"], [decimals]);
```

`parameter` produces a typeset numeric readout and a compact track/dot widget.
The initial value must be inside a finite `min..max` range. Animated values are
clamped to that range, which prevents an accidental journey from leaving the
meaningful domain.

The whole widget is tagged `id.widget`, so it can enter with the rest of the
story:

```manic
hidden(a.widget);
show(a.widget, 0.4);
```

It is an authored animation control, not an interactive slider in the exported
video. Use ordinary `to` to move it; stateless evaluation keeps preview seeking
and recording deterministic.

## Connect with a responsive range

The simplest binding maps the parameter's declared minimum and maximum onto two
output values:

```manic
bind(a, point, x, w*0.18, w*0.82);
bind(a, label, opacity, 0.25, 1);
bind(a, arrow, angle, -25, 25);
```

This form is especially useful for positions because the endpoints are normal
Manic expressions. `w`, `h`, `cx`, and `cy` are evaluated before the binding is
created, so the same source can give each output format an appropriate range.

## Connect with a formula

Use a string formula for a nonlinear relationship:

```manic
bind(a, point, scale, "0.8+0.2*abs(p)");
bind(a, result, value, "p*p");
bind(damping, wave, formula, "exp(-p*abs(x))*cos(4*x)");
```

Inside a binding formula, `p` is the live parameter. A plot-formula binding also
provides `x` as the plot coordinate. The usual arithmetic, constants, and
functions—`sin`, `cos`, `exp`, `sqrt`, `abs`, `round`, and others—work here.

Bindable properties are `x`, `y`, `opacity`, `scale`, `angle`, `hue`, `value`,
`trace`, and a plot's `formula`. A `value` target must be a `counter`. A formula
target must be a `plot`.

When a plot formula changes, its existing `tangent`, `normal`, `slope`, `area`,
`integral`, and moving mark views receive the new function before they recompute.
That is the important reactive promise: the analysis stays mathematically tied
to the curve the viewer sees.

## Choose cases, not frames

Use named steps to describe meaningful cases:

```manic
step("underdamped") {
  to(damping, value, 0.12, 2.0, smooth);
  say(caption, "The oscillation survives for longer.", 0.35);
}
step("strong-damping") {
  to(damping, value, 0.85, 2.0, smooth);
  say(caption, "The same response now settles quickly.", 0.35);
}
```

The stage names should explain the cases to a human. The parameter supplies the
continuous motion between them; `bind` keeps every connected view synchronized.

## Creator design tips

- Follow one primary parameter per short scene. Several controls are possible,
  but the audience should know which value is changing.
- Connect two or three complementary representations—a formula and plot, a
  geometry and measurement, or a simulation curve and readout.
- Give the parameter a meaningful range. Avoid travelling through singular or
  irrelevant values merely because the engine allows them.
- Use responsive range bindings for positions and formula bindings for meaning.
- Hold the important cases briefly. Smooth motion reveals the trend; the hold
  lets the viewer name it.
- Keep stable ids. Parameter journeys are an extension of the persistent-world
  model, not a reason to create replacement scenes.
- Run `manic check FILE.manic --canvas all` after the journey is complete.

## Complete example

[Parameter journeys](ex-creator.md#parameter-journeys) is the polished reference:
one coefficient drives a quadratic, live tangent and slope, moving point,
changing scale, and `a²` readout across portrait, feed, square, and landscape.

Current bindings target 2D entity properties and formula plots. Constructors
that perform expensive build-time work—such as re-simulating a full physics
system or changing the number of generated objects—remain build-time choices;
represent those journeys with a formula/readout today rather than implying that
Manic silently recomputed the model.

Next: [Meet the shapes that make up the persistent world →](shapes.md)
