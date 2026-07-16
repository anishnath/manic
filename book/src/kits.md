# Kits — math, geometry, algorithms

The words so far (`circle`, `move`, `flash`, `for`…) are the **core**. On top of
that, manic ships **kits** — bundles of higher-level figures for a domain. You
use them exactly like any other call.

## math

Coordinate frames, function plots, vectors, tables:

```manic
axes(ax, (cx, cy), 520, 240);            // a coordinate frame
plot(wave, (cx, cy), 78, 120, "sin(x)"); // y = f(x) from a formula
tangent(t, wave, 0.5);                    // the tangent line + dot at x = 0.5
vector(v, (cx, cy), (120, -90));         // an arrow from an origin
matrix(m, "1 0; 0 1", (cx, cy));         // a bracketed matrix
```

## geo

Olympiad-style constructions — you write the *geometry*, not coordinates, and
everything is **live** (drag a point and the circumcircle, centroid, angles all
recompute):

```manic
point(A, (300, 500));  point(B, (900, 500));  point(C, (620, 180));
circumcircle(cc, A, B, C);   // recomputes if A/B/C move
midpoint(m, A, B);
```

## algo

Data structures and algorithms — arrays + sorting, linked lists, stacks/queues,
graphs, hash maps, BFS/DFS, Dijkstra:

```manic
array(a, "5 2 8 1", (cx, cy));  compare(a, 0, 1);  swap(a, 0, 1);
graph(g, "a b c d", "a-b:2 b-c:1 c-d:3", circular, (cx, cy), 200);
dijkstra(g, a);                 // animates shortest paths
```

Groups make these one-liners: a graph tags its nodes and edges, so
`draw(g.edges)` or `flash(g.nodes, cyan)` animates the whole set.

## three (3D)

A whole second world — a camera, solids, surfaces, and curves in real 3D space,
which you spin and morph. Every 3D word ends in `3`:

```manic
camera3((8, -10, 6), (0, 0, 1), 45);      // an eye to look through
cube3(box, (0, 0, 1), (2, 2, 2));         // a shaded box
revolve3(vase, (3, 0, 1.5), "0.7+0.4*sin(t*2)", (0, 3));  // spin a profile
orbit3(70, 25, 12, 4, smooth);            // swing the camera around
```

It has its own chapter — see [Going 3D](3d.md).

## stats

Turn **data** — or a random process — into a picture that reveals its shape,
centre, and spread. Each builtin animates a *process*: a histogram builds up bar
by bar, sample means pile into a bell, a running proportion settles onto the truth.

```manic
histogram(h, (cx, cy), "72 85 90 68 95 88 76 91 83", 8, 640, 300, rainbow);
bellcurve(b, (cx, cy), 100, 15);          // the 68-95-99.7 rule
clt(c, (cx, cy), 5, 1200);                 // the Central Limit Theorem
```

`histogram` · `summary` · `boxplot` · `skew` · `bellcurve` · `correlation` ·
`lln` · `clt` · `hypothesis` · `covariance` · `bayes` · `distribution` ·
`confidence` · `montecarlo` · `randomwalk`. Seeded, so renders are reproducible.

## physics

Simulations built from their physics and **pre-simulated with RK4** at build time,
so every render is deterministic. Each sim's parts are ordinary manic entities, and
the optional views (`phase` · `well` · `timegraph` · `energygraph`) show the same
motion as math panels. `run(id)` (alias `swing`) plays it.

```manic
pendulum(p, (cx, 200), 2, 50);   phase(p, (980, 200), 120);
well(p, (980, 470), 120);        run(p, 8);   // one swing, three views
doublependulum(dp, (400, 240));  par { run(dp, 12); draw(dp.path, 12); }  // chaos
```

Pendulum family: `pendulum` · `doublependulum` · `springpendulum` · `kapitza` ·
`cartpendulum` · `comparependulum`. Spring family: `spring` · `verticalspring` ·
`springincline` · `bungee` · `resonance` · `doublespring` · `seriesparallel` ·
`carsuspension`. Mechanics: `robotarm` · `piston` · `molecule` · `ramp` (with a `forces(id)`
free-body diagram) · `inclinepulley` · `doubleincline` · `inclinebumper` ·
`springchain` · `looptrack` (a curved-track loop-the-loop) · `dropmass` · `raft` ·
`brachistochrone`. Pulleys: `pulley` (Atwood) ·
`pulleyscale` (reads the tension) · `blocktackle` (N-strand block & tackle) ·
`compoundpulley` (fixed + movable, masses A/B/C).

Because a sim's parts are ordinary entities, any base look composes over them —
e.g. `template("paper")` + a hatched `support` turns a pulley or spring into a
textbook figure (see [Elevating a scene](elevating.md) and the `*-paper` examples).

---

Each kit has a full reference at <https://8gwifi.org/manic>, and you can see them
all in motion in the [Examples gallery](examples.md).
