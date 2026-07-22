# Diagrams — draw a system, then watch it work

Declare a technical diagram as text and manic animates it — **architecture
diagrams today**, with flows, sequences, and more to come. Declare what belongs
where, connect the topology, then follow one message through the design. Manic
handles nested responsive layout, provider artwork, parallel lanes, and
deterministic motion; ordinary `step`, `show`, `draw`, `flow`, camera, and text
verbs remain the presentation language.

Unlike static diagram-as-code (Mermaid, Mingrammer), a manic diagram *moves*:
the same source that draws the boxes also carries a request through them.

## The vocabulary

The whole kit is nine words — a behaviour layer over diagram structure, not a
catalogue of cloud verbs:

```text
architecture  cluster  node  connect  link
message       route    hotpath         flow
```

With them you get provider-neutral nodes (no assets) or real icons from **17
providers**; declaration-first nested clusters with responsive layout; direct,
curved, and orthogonal connections with node-boundary ports; cold dashed
topology kept separate from solid runtime motion; and one persistent message
whose identity survives every hop. The kit shines for one clear story — for a
very dense platform overview, split or place clusters yourself to keep it
readable.

## Possible vs. actual

The one rule to internalise: a **connection states what is possible; a moving
message states what happened.**

```manic
connect(toWorker1, queue, worker1, orthogonal, right, left);
connect(toWorker2, queue, worker2);
connect(toWorker3, queue, worker3, orthogonal, right, left);

message(job, queue, "101");
route(job, toWorker2, 0.9, linear);
route(job, worker2ToDatabase, 0.9, linear);
```

The three connections state what is possible. Only the two explicit `route`
calls state what happened. Manic does not claim RabbitMQ selected Worker 2; the
creator authored that selection.

## The mental model

| Layer | Words | Meaning |
|---|---|---|
| Ownership | `architecture`, `cluster`, `node` | what belongs where |
| Topology | `connect`, `link` | possible directed paths and neutral visual relationships |
| Runtime story | `message`, `route`, `hotpath` | what one persistent message actually does |
| Aggregate activity | `flow` | untracked traffic over one path or a path group |

`request` is an HTTP-friendly alias of `message`. Provider services are data,
not vocabulary: use `"aws:lambda"`, not a separate Lambda constructor.

Provider names are visual metadata only. Manic does not infer balancing,
queueing, broadcasting, retries, or any other behavior from an icon or label.
Use explicit `route`/`travel` for one authored journey, `seq` for authored order,
and `par` plus multiple objects when several journeys should happen together.
`hotpath` is only an optional seeded walk over declared graph geometry; it is
not a simulation of the services shown.

## Start without a cloud provider

The foundation includes native visual archetypes for `client`, `service`,
`gateway`, `database`, `cache`, `queue`, `storage`, and `external`. These names
select a compact visual only; they still imply no runtime behavior:

```manic
architecture(platform, (cx,cy), w*0.88, h*0.58);
node(user, platform, "client", "User");
node(edge, platform, "gateway", "Gateway");
node(api, platform, "service", "API Service");
node(db, platform, "database", "Database");
```

Use provider names only when recognizable artwork helps the lesson. A generic
diagram remains fully portable and needs no image assets.

The complete foundation story sends one request forward and returns the same
identity over separately directed paths:

```manic
{{#include ../../examples/systems-foundation.manic}}
```

## Declare the parent first

A node's second argument is its parent architecture or cluster. Clusters may
themselves belong to an earlier cluster, giving hierarchy without member lists
or special block syntax:

```manic
architecture(events, (cx,cy), w*0.9, h*0.7);

node(source, events, "aws:eks", "K8s Source");
cluster(flows, events, "EVENT FLOWS");

cluster(workers, flows, "EVENT WORKERS");
node(worker1, workers, "aws:ecs", "Worker 1");
node(worker2, workers, "aws:ecs", "Worker 2");
node(worker3, workers, "aws:ecs", "Worker 3");

node(queue, flows, "aws:sqs", "Event Queue");

cluster(processing, flows, "PROCESSING");
node(proc1, processing, "aws:lambda", "Processor 1");
node(proc2, processing, "aws:lambda", "Processor 2");
node(proc3, processing, "aws:lambda", "Processor 3");

node(store, events, "aws:s3", "Events Store");
node(analytics, events, "aws:redshift", "Analytics");
```

Landscape uses a horizontal system flow and stacks replicated members inside
their clusters. Portrait rotates the main flow and places replicated members
across the available width. Cluster frames resize from their descendants.

## Cold topology, then hot behaviour

```manic
connect(toWorkers, source, workers);
connect(toQueue, workers, queue);
connect(toProcessors, queue, processing);
connect(toStore, processing, store);

message(event, source, "EVENT");

step("process") {
  seq {
    route(event, toWorkers, 0.8, smooth);
    route(event, toQueue, 0.8, smooth);
    route(event, toProcessors, 0.8, smooth);
  }
}
```

Connecting a node to a cluster creates the possible lane to every member.
Connections are dashed by default: they explain a relationship without falsely
claiming that data is flowing. The hot overlay stays solid when activity begins.

The declared connection name addresses the complete fan-out or fan-in group, so
`draw(toWorkers)` and `flow(toWorkers, ...)` affect every physical lane. `route`
still selects only the lane beginning at the message's current node.

`route` is the explicit form. It chooses the physical lane that begins at the
message's current node, illuminates that lane, moves the same message identity,
and records its semantic destination. A later route that does not start there
fails during `manic check` instead of teleporting.

Use `draw(toWorkers)` to reveal every possible lane. Use `route` for one real
message. Use `flow(toWorkers, ...)` only when the idea is aggregate activity
across the worker pool.

## Route around a visual obstacle

Connections remain direct unless the creator supplies a signed bend in canvas
units:

```manic
connect(toDatabase, services, database);
connect(toCache, services, cache, 145*u);
```

The optional fourth argument changes only the drawn curve and the route that
travels over it. Positive and negative values curve on opposite sides. It does
not infer obstacles, architecture semantics, or provider behavior; use the
smallest bend that keeps the diagram readable on every target canvas.

## Use one orthogonal connector

For architecture diagrams that need right-angle routing, keep the connection
as one semantic identity:

```manic
connect(toDatabase, cache, database, orthogonal);
connect(returnPath, database, api, orthogonal, bottom, right);

message(packet, cache, "GET");
route(packet, toDatabase, 0.9, linear);
```

`orthogonal` uses automatic node-boundary ports by default. Add explicit source
and destination ports only when the composition needs them: `left`, `right`,
`top`, or `bottom`. The engine builds the internal elbows, but `draw`, `flow`,
`route`, and `hotpath` still address the declared connection id. One message
moves across every segment without replacement objects or animation gaps.

This is deterministic Manhattan geometry, not obstacle avoidance. Manic does
not inspect service kinds or guess what the connection means.

## The connection & arrow grammar

`systems-arrow-patterns.manic` is the visual reference for every way one
connection can be drawn. All six share one rule: the **connection describes a
possible relationship**; direction and meaning come from the *moving* object
(`route`/`flow`) and from creator-chosen **colour**, never from the arrowhead
alone.

| # | Pattern | How to author it |
|---|---|---|
| 1 | **One-way** — one source, one destination | `connect(a2b, a, b)` |
| 2 | **Round-trip** — request and response as *separate*, honest lanes | `connect(fwd, a, b)` + `connect(ret, b, a)` (style/colour them differently) |
| 3 | **Orthogonal** — right-angle Manhattan routing, one identity | `connect(a2b, a, b, orthogonal)` |
| 4 | **Vertical ports** — the lane enters/leaves a chosen face | `connect(a2b, a, b, orthogonal, top, bottom)` |
| 5 | **Fan-out** — N *explicitly authored* deliveries (not inferred broadcast) | `connect(toW1, q, w1)` … `connect(toW3, q, w3)`, or one `connect(toWorkers, q, workers)` to a cluster |
| 6 | **Diagonal duplex** — two directions with distinct styling | two `connect`s + per-lane `color`/`hue`/`dashed` |

Colour the lanes by relationship so a dense diagram reads at a glance — each
connection exposes the cold line `id` and the hot overlay `id.hot`:

```manic
color(fwd, cyan);   color(fwd.hot, cyan);      // request
hue(ret, 328);      hue(ret.hot, 328);         // response (pink)
```

<div class="manic-video" data-video="ex-systems-arrow-patterns"></div>

Nothing here implies balancing, buffering, or broadcast. A fan-out is *possible*
lanes; the message that moves — and the colour you give each lane — is what tells
the runtime story.

## Translate a Mingrammer diagram

The structural mapping is deliberately small:

| Mingrammer | Manic |
|---|---|
| `Diagram(...)` | `architecture(...)` |
| `Cluster(...)` | `cluster(...)` |
| `ECS(...)`, `RDS(...)`, and other provider objects | `node(..., "aws:kind", ...)` |
| `a >> b` | `connect(path, a, b)` |
| `a - b` | `link(path, a.card, b.card)` |
| runtime behavior | explicit `message`, `route`, `travel`, or `flow` |

This keeps the imported design faithful without turning labels such as “lb,”
“queue,” or “topic” into hidden behavior. The creator decides which object
moves, which path it takes, and whether several motions use `seq` or `par`.

The clustered web-services example recreates Route 53, ELB, three ECS services,
an RDS primary/read-only pair, and ElastiCache. It then distinguishes all cold
relationships from one database round trip, one cache round trip, and a
separately authored database-link pulse. Return journeys use separately
directed connections and recolour the same persistent identity rather than
playing the incoming path backward. The example gives responses a second visual
grammar—curved pink dashed arrows, a heavier glow, and a matching response
identity—while requests remain clean solid strokes:

```manic
{{#include ../../examples/aws-clustered-web-services.manic}}
```

## Infer one complete hot path

For the common architecture-explainer shot, Manic can follow the graph itself:

```manic
message(event, source, "EVENT");

step("runtime") {
  hotpath(event, 6.0, 27);
}
```

`hotpath(message, [duration], [seed])` begins at the message's current node,
finds valid outgoing physical lanes, chooses one at every fan-out, and keeps the
same dot moving until it reaches a sink. Only the selected lanes illuminate;
all other relationships stay dashed and quiet.

The optional seed controls the branch choices. The result feels random to a
viewer but is deterministic across previews, backend renders, and WASM. Change
the seed to show another valid execution without rewriting the story. Use
`route` when the exact service sequence is part of the lesson; use `hotpath`
when the lesson is “one possible runtime through this topology.”

## Directional and continuous flow

```manic
flow(path, 0.8);                         // forward, one clean pulse
flow(path, 4.0, forward, continuous);    // finite repeating stream
flow(returnPath, 1.0, reverse, once);    // reverse pulse on generic geometry
flow(syncPath, 4.0, both, continuous);   // independent duplex streams
```

The full signature is:

```text
flow(path, [duration], [forward|reverse|both], [once|continuous])
```

Continuous flow chooses an integer number of length-aware cycles. It begins
empty and ends on a cycle boundary, so seeking remains deterministic and the
stream drains instead of freezing halfway along the connection. In a Systems
story, prefer a separately directed return connection when the topology truly
contains a response path.

## Complete event-processing story

```manic
{{#include ../../examples/aws-event-processing-clusters-poc.manic}}
```

Run and audit it directly:

```bash
manic examples/aws-event-processing-clusters-poc.manic
manic check examples/aws-event-processing-clusters-poc.manic --canvas all
```

## What the kit will not do for you

By design, the kit never infers behaviour from an icon or a label. It will not
decide that a node is a load balancer, a queue, or a database, and it never
simulates balancing, buffering, broadcast, retries, or failure. You author what
happens with explicit `route`, `flow`, `seq`, `par`, colour, and ordinary verbs
— that is what keeps the vocabulary small and the diagram honest.

Two practical tips:

- **Draw the return path, don't reverse the arrow.** A response is its own
  connection with its own colour, not the request lane played backward.
- **For very dense diagrams, shape the layout yourself.** Group nodes into
  clusters and split wide rows into columns so nothing overflows the frame.
