# Graphs

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## graph

Labelled nodes + edges, drawn on; reflowing links.

```manic
{{#include ../../examples/graph.manic}}
```

<div class="manic-video" data-video="ex-graph"></div>

## graph_moving

Drag a vertex and its incident edges follow.

```manic
{{#include ../../examples/graph_moving.manic}}
```

<div class="manic-video" data-video="ex-graph_moving"></div>

## bfs_dfs

The same graph, queue vs stack, with live frontier readouts.

```manic
{{#include ../../examples/bfs_dfs.manic}}
```

<div class="manic-video" data-video="ex-bfs_dfs"></div>

## dijkstra

Weighted edges, settling distances, a shortest-path tree.

```manic
{{#include ../../examples/dijkstra.manic}}
```

<div class="manic-video" data-video="ex-dijkstra"></div>
