# Grids — pathfinding & automata

A first-class 2-D cell grid — the one primitive under tilemaps, spatial pathfinding (space, not `graph`'s topology), cellular automata and Wave Function Collapse. Cells address like `matrix`/`table` (`{id}.r{i}c{j}`); seed a maze from a compact `# . @ *` ASCII string; the pathfinders reuse the algo kit's exact colour grammar (discovered cyan → current magenta → done lime), and generation pre-simulates at build time then replays with `run`.

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## grid-astar

A* over a seeded ASCII maze: open cells flood by f-score (frontier cyan → current magenta), a live frontier/visited readout counts up, and the shortest route traces out in gold as `{id}.path`. `neighbors` picks 4- or 8-connectivity.

```manic
{{#include ../../examples/grid-astar.manic}}
```

<div class="manic-video" data-video="ex-grid-astar"></div>

## grid-life

Conway's Game of Life: a glider seeded with `setcell`, then `evolve` pre-simulates six generations at build time (alive = a filled cell, Conway's B3/S23) and `run` replays them — the glider walks diagonally across the grid.

```manic
{{#include ../../examples/grid-life.manic}}
```

<div class="manic-video" data-video="ex-grid-life"></div>

## grid-wfc

Wave Function Collapse: `collapse` pre-simulates a seeded, neighbour-constrained settling row by row, then `run` replays the grid resolving from empty into a finished maze — deterministic, so the same seed always settles the same way.

```manic
{{#include ../../examples/grid-wfc.manic}}
```

<div class="manic-video" data-video="ex-grid-wfc"></div>

## grid-life-zoo

Conway's Life's whole taxonomy — a still life (Block), an oscillator (Blinker) and a
spaceship (Glider) — three grids seeded with `setcell`, `evolve`d eight generations and
`run` in parallel: the Block holds, the Blinker flips, the Glider walks. The famous
pattern zoo, entirely in the grid kit.

```manic
{{#include ../../examples/grid-life-zoo.manic}}
```

<div class="manic-video" data-video="ex-grid-life-zoo"></div>
