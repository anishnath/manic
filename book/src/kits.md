# Kits — math, geometry, algorithms

The words so far (`circle`, `move`, `flash`, `for`…) are the **core**. On top of
that, manic ships **kits** — bundles of higher-level figures for a domain. You
use them exactly like any other call.

## math

Coordinate frames, function plots, vectors, tables:

```manic
axes(ax, (cx, cy), 520, 240);            // a coordinate frame
plot(wave, (cx, cy), 78, 120, "sin(x)"); // y = f(x) from a formula
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

---

Each kit has a full reference at <https://8gwifi.org/manic>, and you can see them
all in motion in the [Examples gallery](examples.md).
