# manic — capabilities & gaps

A snapshot of what manic can do today vs. what it can't, grounded against the
Asymptote example corpus (520 `.asy` files: 117 `geometry/`, ~197
`generalities/`, ~34 `graph/`, ~96 3D across `three`/`graph3`/`solids`/`tube`/
`grid3`, plus generative folders) and the Manim references. Usage counts below
are occurrences across the `geometry/` samples.

Status vocabulary used throughout this ledger:

- **✅ Shipped** — implemented, documented, tested, and represented by an example.
- **🟡 Foundation** — useful production surface exists, with explicitly listed
  layers still open.
- **⬜ Future** — accepted gap, not yet scheduled or promised as vocabulary.
- **🅿️ Parked** — deliberately deferred because correctness, scope, or teaching
  cost currently outweighs creator value.

The creator roadmap and the **Active work queue** below are authoritative.
Detailed benchmark/domain sections retain evidence and design context; they do
not create a second competing roadmap.

## Creator-first roadmap — priority order

These are the five priority product directions, ordered by dependency and creator
leverage. They are deliberately generic: each should strengthen every subject
without adding a separate vocabulary for algebra, chemistry, biology, physics,
or social video.

1. **Reactive worlds.** Declare a visual world once, then describe the next
   state of several existing entities together. Unmentioned entities persist;
   equations, plots, diagrams, labels, values, and the camera change in one
   continuous beat. **🟡 Foundation (2026-07):** a named `step("name") { ... }`
   block coordinates ordinary verbs in parallel and exports its start as a
   marker, while `rewrite` remains the structured equation transition. Rewrite
   matching is order-preserving, layout-role aware (main line, exponent,
   numerator, denominator, and structural rules), and math-style-depth aware,
   so an identical glyph cannot silently change mathematical jobs or jump
   between levels of a nested exponent. When a side gains a fraction, radical,
   or grouping structure, that side leaves and reforms locally while the
   compatible side and equality remain continuous. Empty, duplicate, and
   nested step names fail clearly; stateless seeking and ordinary non-step
   timelines remain unchanged. See `examples/reactive-world.manic`.
2. **Named story stages.** Make `question → intuition → experiment → proof →
   takeaway` first-class project structure. Stages must be readable in source,
   seekable in preview, and reusable by recording/publishing tools. **✅ Engine
   and CLI shipped (2026-07):** `step("name") { ... }` remains the only authoring
   vocabulary, while its metadata is promoted throughout the workflow:
   `manic stages FILE.manic` reports start/end/duration, `--stage NAME` previews
   or records exactly one stage, and inclusive `--from-stage` / `--to-stage`
   ranges export story slices. Live preview starts and restarts at the selected
   boundary, clamps scrubbing/playback to it, exposes a clickable stage strip,
   and uses number keys for direct stage jumps. Recording metadata is filtered,
   clipped, and shifted to the selected source range while retaining original
   `source_t` positions. No duplicate scene or timestamp DSL is introduced.
   **🟡 Production integration remains:** expose the same stage manifest and
   optional selection through a public runtime/backend request API so browser
   users never need CLI flags. A file-only run already shows the stage strip and
   plays the complete story.
3. **One story, multiple formats.** Reframe the same semantic stages for Reels,
   Shorts, landscape lessons, square posts, slides, thumbnails, and stills—same
   content identity, format-specific layout and pacing. **🟡 Foundation
   (2026-07):** `--canvas portrait|4:5|square|16:9|WIDTHxHEIGHT` overrides the
   logical canvas before expansion, so `w`/`h`/`cx`/`cy`, macros, and build-time
   layout branches reflow one source while named `step` markers and timing stay
   identical. The DSL's own `canvas(...)` remains the no-flag default. See
   `examples/reactive-multiformat.manic`.
4. **Visual correctness checks.** Detect unsafe-region overflow, overlaps,
   detached labels/links, unreadably fast changes, and equation/plot mismatches
   before publishing. Diagnostics should explain the problem and point to the
   responsible stage/entity. **🟡 Foundation (2026-07):**
   `manic check FILE.manic --canvas all` rebuilds portrait, 4:5 feed, square,
   and 16:9 landscape, then audits the settled state of every named stage for
   canvas overflow, Creator safe-area overflow, substantial content overlap,
   and unreadably small text/notation. Messages include format, stage, time,
   entity, and a suggested fix; findings return a failing status for publishing
   scripts. Transition-path collisions, detached dynamic links, reading speed,
   camera/3D bounds, and equation/plot semantic agreement remain future layers.
5. **Parameter journeys.** Let creators expose meaningful parameters and animate
   a family of cases—quadratic coefficients, damping, sample size, equilibrium,
   geometry inputs—without duplicating the story. **✅ Core shipped
   (2026-07):** `parameter(id,(x,y),initial,min,max,[label],[decimals])` creates
   a visible bounded control, while `bind(parameter,target,property,...)` maps
   it either through responsive numeric endpoints or a formula in live `p`.
   Ordinary `step` + `to(parameter,value,...)` then moves a whole connected
   family without rebuilding the scene. Bindings cover 2-D position, opacity,
   scale, angle, hue, trace, counters, and live plot formulas; a changing plot
   pushes its source into tangent/normal/slope/area/integral/mark views before
   they recompute. Evaluation is stateless, values clamp to the authored range,
   duplicate/invalid connections fail early, editor support and multi-format
   tests ship, and `examples/parameter-journeys.manic` passes all four visual
   audits. Expensive build-time constructor changes (re-simulating a physics
   system or changing generated object count) and live measured-geometry
   bindings remain future extensions rather than hidden magic.

### Active work queue

This is the single current priority list. Work below it is reference backlog,
not an instruction to implement everything.

| Priority | Status | Work | Creator value |
|---|---|---|---|
| P0 | ✅ | **Deterministic text + glyph engine** — bundled-only advanced Unicode shaping; one cached measurement/wrap/reveal/draw layout; checked render sessions; strict eight-face provenance; actionable diagnostics; pinned layout/pixel goldens; native/backend/WASM compile parity. | Prevents silent tofu boxes, broken symbols, incorrect wrapping, and machine-dependent text before Flowchart, Sequence Diagram, and further Systems work multiplies symbol-heavy content. |
| P0 | 🟡 | **Systems Foundation hardening** — make the shipped architecture/runtime model provider-neutral and reusable: stable native node archetypes, panic-free ownership/id diagnostics, a curated provider manifest, shared bounds/ports, responsive edge reflow, and explicit one/all/copy/merge choreography without inferred service semantics. | Turns the Systems PoC into the dependable geometry and flow substrate for architecture, flowchart, sequence, network, circuit, and future process kits. |
| P1 | ⬜ | **Production runtime contract** — public stage manifest plus optional stage/range/canvas/template selection shared by CLI, UI, and backend; full-story defaults require no options. | Makes shipped creator workflows usable from the production editor without leaking CLI concerns into the DSL. |
| P1 | 🟡 | **Motion Graphics V2Core** — ✅ authored endpoint/blueprint state plus `attach` / `become` / `turn` and shared pivots have shipped locally; normalized live path progress, general group bounds, and velocity-continuous generated motion remain. Existing `travel`, `flow`, `wander`, and `arrange` calls stay unchanged. | Lets non-programmers describe continuity and relationships instead of intermediate coordinates, while preserving deterministic playback and existing files. |
| P1 | ✅ | **Manic ML + transformers** — ML1–ML7 ship locally: feed-forward learning, tensor/CNN foundations, single-head attention, truthful tokenization/position, complete transformer blocks, and exact logits/temperature/sampling. | Lets educators follow modern ML from text to tokens, representations, attention, learning, and generation without Python animation code or visually plausible fake arithmetic. |
| P1 | ✅ | **3D V2Core** — creator-first authored 3D state and bounds, automatic camera composition, spatial path travel, timed attachment/release, exact-blueprint transformation, rigid axis turns, template-aware diagram lighting, tests, and creator examples ship. Surface: `view3`, `travel3`, `attach3`, `become3`, `turn3`; existing precise 3D calls remain unchanged. | Makes spatial explainers readable and cinematic without asking creators to calculate camera distance, Euler choreography, or intermediate coordinates. |
| P1 | 🟡 | **Visual audit layers** — transition-path collisions, detached dynamic links, reading speed, camera/3D bounds, and equation/plot semantic agreement. | Prevents bad exports, not merely bad settled frames. |
| P1 | 🟡 | **Multi-format composition** — reusable layout policies, format-specific pacing, thumbnail/still framing, and UI-controlled output variants. | Turns one semantic story into a dependable publishing system. |
| P2 | ⬜ | **General bounds + relative placement** — reusable entity/group bounding boxes, framing, edge origins, and `next_to`-style layout. | Removes manual coordinates and unlocks reliable automatic composition. |
| P2 | ⬜ | **Live geometry measurements** — bind derived lengths, angles, positions, and areas into counters/labels. | Makes olympiad, engineering, and interactive diagrams numerically truthful. |
| P2 | ⬜ | **Nonlinear remapping** — `travel(entity,path,…)` now ships for move-along-path; the remaining work is deforming grids/curves through a general authored map. | Extends the shipped path motion into advanced transformation stories without multiplying vocabulary. |
| P3 | ⬜ | **Typography/look extensions** — after P0 text correctness, selectable bundled branding fonts and an optional chalkboard/sketch renderer. | Broadens creator identity without confusing style choice with glyph reliability. |
| P3 | ⬜ | **3D V3 solid sections** — one generic solid-section/projection bridge that creates cut pieces and exposed faces while preserving their identities; V2 continues to use exact authored `param3` sections. | Extends textbook cutaway and cross-section stories beyond authored spheres without adding a boolean-node vocabulary. |

### Future creator support

After the five foundations above: reusable **“Why is this true?”** story
formats; misconception → diagnosis → correction stories; semantic identity
across equation/plot/diagram representations; visual experiment-first lessons;
“simple to surprising” narrative templates; domain-neutral visual-proof actions
(preserve, pair, rearrange, contradict, split cases, generalize); synchronized
split-screen comparisons; a timeline/entity visual debugger; community remix
inputs; and a consistent product promise—**describe how an idea changes, and
Manic keeps the visual world continuous**. A **Map / Geography kit** (map
explainer reels for non-coders) is explored and PoC-validated but **deferred** —
see [Map / Geography kit — ⬜ Deferred](#map--geography-kit--⬜-deferred-poc-validated-not-scheduled)
at the end of this document for the full findings and open forks.

## Visual Explanation Director — ⬜ Future cross-kit capability

**Product idea:** creators should describe what the audience needs to notice;
Manic should deterministically direct how that idea is presented. Today an
author often expresses both the semantic beat (for example, “the error travels
backward”) and its cinematography: camera coordinates, zoom, dimming, emphasis,
caption placement, restoration, and reading holds. The Director would compile
the semantic intention into those ordinary Manic tracks.

This is **not** another template and not an AI runtime. Templates control visual
identity—palette, type, chrome, and general look. The Director controls
attention over time—subject, context, framing, emphasis, pacing, and return to
the whole. Its output must remain deterministic, scrubbable, responsive,
reproducible, and manually overrideable.

The core directing model has three temporary visual roles:

- **Subject** — what the audience must understand now; receives primary framing
  and emphasis.
- **Context** — what makes the subject meaningful; remains visible but quieter.
- **Background** — currently irrelevant material; may be gently dimmed without
  destroying scene continuity.

A deliberately small conceptual surface is preferred:

```manic
focus(subject);          // make one idea primary while preserving useful context
compare(before, after); // compose and frame a readable comparison
overview(system);       // return to a stable view of the whole
```

Later intentions may include an unambiguous camera-tracking word, a subject-bound
explanation/callout, and a final-result treatment. They should be added only
when the first three cannot express a recurring creator need; existing structural
`follow` must not acquire a second, camera-related meaning.

Expected deterministic planning behavior:

1. Resolve the subject/tag and its current authored bounds.
2. Preserve enough related structure to maintain conceptual continuity.
3. Choose a canvas-safe frame for portrait, feed, square, or landscape.
4. Generate smooth camera/emphasis/quieting tracks with no abrupt jump.
5. Allocate a readable hold based on visible text and visual complexity.
6. Restore the previous emphasis and camera state, or settle into an explicit
   `overview`/takeaway state.

The Director should build on existing `step`, tags, authored endpoint state,
camera verbs, visual audits, and future general bounds. It should not create a
parallel timeline system. A good first acceptance story is one explanation
rendered across all four canvas formats where `focus → compare → overview`
keeps the same semantic identity, never blanks the scene, never hides required
context, and needs no hand-authored camera coordinates.

Longer-term Director quality checks belong in explanation linting: warn when too
many unrelated objects change simultaneously, a subject is too small, important
context disappears, a camera move is abrupt, a caption cannot be read in time,
or colour is the only carrier of meaning. This extends the current visual audit
from layout correctness toward explanation correctness.

## Future domain-kit candidates — ⬜ Exploration order

New kits should earn vocabulary by adding truthful domain meaning and computation;
ordinary Manic verbs should continue to provide presentation. The current value
ranking is:

| Rank | Candidate | Creator value | Fit / principal constraint |
|---:|---|---|---|
| 1 | **Systems / Architecture Kit** | Requests, services, databases, caches, queues, event streams, networks, cloud systems, failures, scaling, and distributed-system journeys. | Excellent procedural fit and immediate developer demand. The value is animated behavior, not another static boxes-and-arrows generator. |
| 2 | **Circuit Kit** | School electricity, electronics, digital logic, computer architecture, control systems, embedded tutorials, and engineering explainers. | Excellent fit with paths, flow, plots, equations, reactive parameters, and state changes. Numerical truth and standardized symbols are achievable without assets. |
| 3 | **Chemistry Reaction Kit** | Atoms, bonds, balancing, electron movement, molecular geometry, equilibrium, acids/bases, energy diagrams, and reaction mechanisms. | Strong fit with particles, rewrite, paths, and 3D; notation and chemical correctness require careful design. |
| 4 | **Biology Systems Kit** | Cells, DNA→RNA→protein, mitosis, neurons, circulation, immunity, transport, and pathways. | Very broad audience, but organic visuals may eventually need an asset/illustration strategy. |
| 5 | **Anatomy Kit** | Heart, lungs, digestion, muscles, joints, kidneys, and clinical education. | High educational value but strongly asset-dependent; defer until Manic has a deliberate organic-visual strategy. |

### Systems / Architecture Kit — ✅ V2 foundation complete

**Expansion gate (P0) — cleared:** the deterministic text + glyph engine now
ships with advanced shaping, technical-symbol fallback, multilingual script
faces, one cached measure/draw layout, checked render sessions, and parity
goldens. Systems, Flowchart, and Sequence Diagram work can build on that shared
foundation without spreading missing-glyph or layout instability.

#### Foundation hardening — active, in dependency order

The shipped V2 story surface stays `architecture`, `cluster`, `node`,
`connect`, `message`, `route`, and `hotpath`. Hardening is internal or additive;
existing `.manic` files must not change.

| Order | Foundation guarantee | Scope / acceptance |
|---:|---|---|
| 1 | ✅ **Provider-neutral structure and diagnostics — shipped** | Native visual archetypes (`client`, `service`, `gateway`, `database`, `cache`, `queue`, `storage`, `external`) work without cloud assets. Duplicate semantic/generated ids, invalid ownership, and unavailable provider kinds return source-located Manic errors rather than `Scene::add` panics. Every canonical AWS mapping resolves through a tested stable manifest; aliases map to the same artwork and never imply behavior. |
| 2 | 🟡 **Shared bounds, ports, and responsive edge geometry — port-aware orthogonal routing shipped** | `connect(path,a,b,orthogonal)` now chooses node-boundary ports; optional `left`/`right`/`top`/`bottom` ports give exact control. Direct, curved, and orthogonal lanes share one semantic identity and length-aware `route`/`hotpath` motion. The dense `onprem-advanced-web-service.manic` acceptance story proves motion continuity but exposes the remaining need for obstacle-aware lanes, general entity/group bounds, and post-declaration responsive edge reflow. |
| 3 | 🟡 **Explicit delivery choreography — authored one-of-many proven** | `systems-rabbitmq-consumers.manic` proves that persistent messages plus explicit routes can show successive queue jobs selecting different workers without implying broadcast or queue policy. Generic broadcast, fork/copy, and merge/join composition still remain. Icons and names never infer behavior; the creator selects it. |
| 4 | **Direction and round-trip grammar** | Forward, return, duplex, neutral relationships, and response identity remain readable without depending on colour. Incoming and returning paths may differ while retaining continuity and stateless seeking. |
| 5 | **Large-diagram composition** | Nested clusters, edge labels, camera focus, decluttering, and level-of-detail stay readable across portrait, square, and landscape without requiring manual coordinates for every node. Acceptance now explicitly includes fitting the on-prem stress test inside its architecture bounds, reserving nested-cluster title space, and detecting path/title, path/card, and message/card collisions in the visual audit. |
| 6 | **Distribution and ecosystem contract** | Curated provider assets, native fallbacks, mdBook/system-prompt guidance, editor catalog, package/deploy/WASM checks, and provider-manifest tests ship together. Additional providers extend data/assets, never the DSL grammar. |

The PoC proves the product distinction: static diagram-as-code describes what a
system **is**; Manic follows what the system **does**. Authors declare a bounded
architecture, auto-positioned nodes, and directed connections, then move one
persistent request through the graph while ordinary `step`, `flow`, captions,
and templates provide the story.

The deliberately small surface is `architecture`, `cluster`, `node`, `connect`,
`message`, `route`, and `hotpath`; `request` remains an HTTP-friendly
alias. Provider services remain string
data such as `"aws:lambda"`, never dedicated DSL words. The initial catalogue
maps Route 53, CloudFront, API Gateway, Elastic Load Balancing, Lambda, ECS,
Fargate, S3, DynamoDB, ElastiCache, RDS, Redshift, EKS, and SQS to the official
AWS Architecture Icon package under `assets/diagrams/aws`; a generic native
client symbol proves provider-neutral composition.

V2 ownership is declaration-first: `cluster(id,parent,"label")` creates the
boundary, and later clusters/nodes name it as their parent. The engine measures
descendants bottom-up, resizes every boundary, and lays children out along or
across the architecture's responsive main axis. The former fourth-argument
member list remains accepted for existing PoC sources but is no longer the
recommended authoring form.

`connect` renders possible relationships as cold dashed topology. `route`
preserves message identity, selects the physical lane beginning at the
message's actual current node, illuminates only that lane with a solid hot
overlay, and records where the message settles. A later discontinuous route is
rejected with the current node and expected connection source instead of
silently teleporting. `draw(connection)` reveals all possible lanes; grouped
`flow` remains aggregate untracked activity rather than pretending one message
is broadcast everywhere. Multi-lane connections use numbered internal paths so
the declared connection id always addresses the complete fan-out/fan-in group.
`connect(id,from,to,bend)` adds an optional signed, creator-authored curve for
visual routing around a known obstacle. `connect(id,from,to,orthogonal)` adds a
single Manhattan connector with automatic node-boundary ports; creators may add
explicit `from_port,to_port` values from `left`, `right`, `top`, and `bottom`.
Both forms change geometry only and perform no provider or obstacle inference.

`hotpath(message,duration,seed)` is the final runtime layer. It infers one
complete execution from the message's current node to a sink, chooses a valid
physical lane at every fan-out, and moves the same dot continuously end-to-end.
Unselected relationships remain dashed. Seeded branching is reproducible on
native, backend, and WASM renders while a changed seed demonstrates another
valid execution. Explicit `route` remains the right tool when each exact hop is
part of the lesson.

This graph convenience does **not** infer architecture semantics. Manic does not
know that an icon is a load balancer, queue, topic, database, or subscriber.
Names and provider artwork never imply one/all delivery, balancing, buffering,
retries, or broadcast. The creator authors those intentions with explicit
objects and ordinary `travel`, `flow`, `seq`, `par`, `stagger`, and tags.

Generic `flow(path,duration,direction,mode)` now supports `forward`/`reverse`/`both`
and `once`/`continuous`. Continuous flow computes length-aware complete cycles,
remains stateless and scrubbable, and drains on an integer boundary instead of
freezing halfway through a connection. Existing two-argument calls retain their
single-forward-pulse meaning.

`examples/systems-architecture-poc.manic` is the acceptance story: one Buy
request travels Browser → CloudFront → API Gateway → Lambda → DynamoDB → SQS.
The same source automatically lays nodes horizontally or vertically according
to its responsive architecture region.

`examples/systems-foundation.manic` is the provider-neutral acceptance story.
It uses only native `client`, `gateway`, `service`, `cache`, and `database`
archetypes, reveals cold topology, sends one persistent request forward, then
returns the same identity over separately directed curved paths. It proves that
structure, runtime motion, and direction remain available without cloud assets
or inferred service semantics.

`examples/aws-three-tier-web-application.manic` expands the visual test into a
textbook architecture story. Presentation, Application, and Data remain three
distinct responsive regions while one request travels Route 53 → CloudFront →
Elastic Load Balancing → ECS → ElastiCache → RDS. Landscape uses tier columns;
portrait uses tier rows without changing the semantic route.

`examples/aws-event-processing-clusters-poc.manic` is the next-level structural
test: EKS → three ECS workers → SQS → three Lambda processors → S3 and
Redshift. Clusters are declared before their children, so the same word
represents both Event Workers and the outer Event Flows boundary without a
member string.
When one endpoint of `connect` is a cluster, Manic expands the declaration into
parallel physical paths. The flagship story first reveals every relationship as
dashed topology, then `hotpath` chooses one worker, one processor, and one sink
while a persistent event and solid highlight travel together. Explicit
`flow(group,...)` animates the complete fan-out or fan-in only when aggregate
activity is the idea. This adds hierarchy and parallel topology without
provider-specific verbs.

`examples/aws-clustered-web-services.manic` faithfully translates a Mingrammer
Route 53 → ELB → ECS pool design with an RDS primary/read-only cluster and
ElastiCache. It proves the finalized distinction in one story: all relationships
remain cold and dashed, database and cache round trips use explicit forward and
return connections, and the neutral RDS link moves only when ordinary `flow`
asks it to. The same request identity becomes a response and returns along a
curved, dashed, glowing pink hot path; nothing plays a directed arrow backward.
Requests remain solid so direction is readable without relying on colour alone.
The cache fan-in uses the optional bend to remain readable around the DB cluster
on responsive canvases.

`examples/systems-arrow-patterns.manic` is the connection-grammar reference. It
covers one-way, parallel, round-trip, orthogonal, vertical-port, fan-out, and
diagonal-duplex patterns. This review led to the additive port-aware
`orthogonal` form and shared length-aware motion geometry; direct and curved
connections remain unchanged.

`examples/systems-rabbitmq-consumers.manic` is the focused delivery acceptance
story. Three cold queue-to-worker possibilities remain visible while messages
101, 102, and 103 explicitly select different workers and continue to one
database. It proves authored one-of-many delivery with the existing generic
vocabulary; Manic never infers RabbitMQ, round-robin, or Kubernetes behavior.

`examples/onprem-advanced-web-service.manic` is deliberately a stress test, not
a gallery-ready example. Its Nginx, gRPC, Redis, PostgreSQL, Fluentd, Kafka,
Spark, Prometheus, and Grafana topology proves persistent motion across mixed
paths, but human review exposes architecture overflow, nested-title contention,
path/card crossings, and message-label collisions that the current settled-frame
audit can miss. Those findings define the next large-diagram acceptance gate.

Review boundaries are intentional: no automatic obstacle-aware edge routing,
post-layout edge reflow, nested-title reservation, automatic cluster-to-cluster
bundles, connection-midpoint attachment, group-aware message copying,
broadcast/fork/merge choreography, full provider icon search, cyclic
retry/failure/scaling semantics, or provider-asset-specific WASM packaging yet.
Those should be designed only after each story is visually reviewed. The full
upstream AWS icon package remains untouched; authors see only the curated stable
manifest rather than its versioned filenames.

### Auto-fit & adjustment — ✅ Core shipped (optional hints deferred)

**The problem, observed across every dense example.** A big architecture (the
on-prem stress test, the k8s stateful set, the serverless pipeline) overflows the
frame, and the only fix today is to *hand-tune it*: split a cluster into extra
columns, nudge `architecture(id, (center), w, h)` numbers, shrink fonts. That is
the opposite of manic. The creator should describe **what** the system is; the
engine should make it **fit** — no coordinate math, no trial-and-error.

**The principle:** structure is authored, layout+fit is automatic. Writing a
diagram that fits must take *less* DSL, not more.

**The design (end-user-easy, engine does the work):**

1. **Auto-fit is the default. ✅ Shipped.** `architecture(id)` with **no**
   center/size fills the canvas minus safe margins and centres itself. Explicit
   `(center, w, h)` stays as an override for the rare hand-placed case, but is
   never required. The common case is one word.
2. **Grid-wrapping clusters + flow-wrapping rows. ✅ Shipped.** A leaf cluster
   with many children wraps into a rows×cols grid chosen from the available space
   and child count; top-level members flow-wrap into rows/columns instead of one
   overflowing line. This *automatically* does what we did by hand (splitting
   pods/claims into columns) — the #1 cause of overflow, gone.
3. **Measure, then scale to fit. ✅ Shipped.** `relayout` lays out at natural
   size, measures the packed content against the frame, and — when it still
   overflows — derives one uniform factor `s ≤ 1` (floored at a legible minimum)
   that contracts the whole diagram about its centre: positions, cards, icons,
   labels *and* connection ports/lanes together, in a single build-time pass so
   `route`/`flow` animate the scaled geometry. Content is guaranteed in-frame —
   nothing clips off-canvas. When it already fits, `s == 1` and tuned examples are
   byte-identical.
4. **Reserve heading and label space** during layout so cluster titles and node
   labels never collide with cards or lanes. 🟡 Partial — the fit pass keeps a
   frame margin; a nested cluster's title can still crowd a sub-cluster header
   (minor, cosmetic).
5. **Optional hints, never coordinates.** `density` (compact/comfortable) and an
   aspect/orientation hint may nudge the result; the author still never types a
   pixel to make it fit. 🟢 Deferred (not needed for fit; a future ergonomic).

**Why it is foundational (and first):** the same fit engine serves architecture,
flowchart (a wide decision tree overflows exactly the same way), sequence, and
any future diagram — so it lands *before* growing more diagram types. It is pure
layout under the hood: the DSL does not grow; authors write *less*.

**Acceptance — met.** `examples/microservices-platform.manic` declares Route 53 →
load balancer → gateway, three availability zones of services, a replicated
database cluster and monitoring with **zero coordinates**; adding a whole tier
reflows and scales the diagram to stay fully in-frame. The dense stress tests
(on-prem, k8s stateful set, serverless) render fully in-frame with **no** manual
column-splitting or size tuning — from structure alone. The one user-visible
promise, delivered: *it just fits.*

### Flowchart Foundation — ✅ Core shipped (C4 + sequence diagrams next)

The Diagrams kit does **architecture** and now **flowcharts**; sequence diagrams
are next. Flowcharts are the highest-demand diagram type — every algorithm,
process, decision tree, and onboarding explainer is a flowchart — and manic's
edge is that the chart *runs*: a token walks the process and takes a branch,
instead of a static Mermaid picture.

**Shipped this cycle** (`examples/factorial-flowchart.manic` is the flagship):
- **`flowchart(id, [TD|LR])` container** — an edge-ranked layout *mode* on the
  shared `ArchitectureData`, reusing bounds, ports and scale-to-fit (no second
  engine). BFS ranks nodes by connection direction; ranks lay out along the main
  axis, members spread on the cross axis; edges default to orthogonal elbow
  connectors along the rank direction. Re-ranks and rewires all lanes in place
  after every edge, so styling/labels survive.
- **Column-wrap layout** — with no direction, `flowchart(id)` lays the flow
  top-down and, when it's long, wraps it into side-by-side columns (count chosen to
  maximise the fit scale, re-decided on every add) so nodes stay full-size and
  readable — read down a column, across to the top of the next. `LR` forces a single
  row. Forward edges are orthogonal elbows; column wraps and short loops arc as
  curves; **long feedback edges route around the bottom-left perimeter** so a
  loop-to-start doesn't slash across the middle. Redundant arrowheads on curve edges
  are hidden (the curve draws its own head).
- **Readability guard** — a flowchart must be *readable*, not merely fit. `check`
  warns (never errors) when a flowchart exceeds a node limit — default 26 top-down/
  auto, 14 left-right (wrapping lets top-down hold far more) — pointing to
  **splitting into linked sub-flows** (a `connector` hand-off). `flowchart(id, dir,
  N)` overrides the limit. It's a language-level node count (layout/pixel size isn't
  known in `check`), so a proxy, surfaced live in the editor.
- **Node shapes as kinds** — `process` (rect), `decision` (diamond), `terminator`
  (pill), `io` (parallelogram), `subprocess` (rect; struck rails TODO), `connector`
  (circle). Selected by string on the ordinary `node`; the shape *is* the body with
  a centred label and no icon.
- **`annotate(edge, "text")`** — a caption at a lane midpoint (yes/no/loop),
  general to every diagram type. (The name `label` is the core std builtin.)
- The animation story works: reveal → `route` a token → it takes the coloured
  branch and loops, taken path glowing.

**Remaining polish (not blockers):** within-rank crossing reduction (dense
decision trees can look busy), a struck-rail `subprocess`, and `LR` still wants a
real-world example. Everything below is the design as shipped.

**Reuse first — a flowchart is another typed graph.** It shares the shipped
Diagrams substrate almost entirely; do **not** rebuild it:

- **Edges + motion** — `connect` (cold dashed topology), `route` (walk one path,
  identity preserved), `hotpath` (infer one execution through the branches),
  `flow` (aggregate), and **colour-by-relationship** (`id`/`id.hot`) all apply
  unchanged. A yes/no decision is just two connections the creator colours.
- **Grouping + story** — `cluster` for sub-processes/swimlanes; `step`/`par`/
  `seq`/`say`/`draw`/`pulse` for the walkthrough; responsive architecture bounds
  for layout regions.
- **Correctness** — the deterministic text/glyph engine (P0, shipped) already
  handles the symbol-heavy labels.

**Genuinely new — kept minimal:**

1. **Flowchart node *shapes*, as kinds not builtins. ✅ Decided.** Extend
   `node`'s kind vocabulary with the standard set — `process` (rectangle, already
   native), `decision` (diamond), `terminator` (stadium/pill for start/end), `io`
   (parallelogram), `subprocess` (double-struck rectangle), `connector` (small
   circle). Selected by string, exactly like the provider-neutral archetypes:
   `node(check, chart, "decision", "n % i == 0?")`. **Never a builtin per shape.**
   This is the deliberate ease-of-use call — a creator draws every flowchart node
   with the *one* `node(id, parent, "kind", "label")` form they already know, so
   there is nothing new to learn beyond the shape names. Remaining implementation
   detail (not a design fork): `diamond`/`parallelogram`/`stadium` likely need new
   `Shape` primitives if `polygon` can't render them cleanly at every size.
2. **Edge labels** — a decision's branches read "yes"/"no". Add an optional
   label to `connect` (or a thin `label(edge, "yes")`), rendered at the lane
   midpoint. General to all diagrams, not flowchart-specific.
3. **Directed ranked layout** — a top-down / left-right auto-layout that ranks
   nodes by edge direction (Mermaid `graph TD`/`LR`). The architecture layout is
   region/cluster-based; a flow needs edge-driven ranking. This is the one real
   layout addition — build it as a layout *mode*, reusing bounds/ports/reflow.

**The animation story (the differentiator):** reveal the chart, then `route` a
token from the start terminator; at each `decision`, the authored branch
illuminates (coloured yes/no) while the other stays cold; the token walks to an
end terminator. `hotpath` gives a seeded auto-walk. This turns a static
flowchart into "watch the algorithm actually run."

**Mermaid/authoring mapping:** `graph TD` → a flowchart layout mode; `A[Process]`
→ `node(a, chart, "process", "…")`; `B{Decision}` → `"decision"`; `A --> B` →
`connect`; `A -->|yes| B` → connect with an edge label. Faithful import without
teaching manic what a decision "means" — the creator authors which branch runs.

**Flagship demo:** a real algorithm as a running flowchart — e.g. binary search
or "is a number prime" — start → loop → decision (found? / divisible?) → the
token takes the true branch and reaches an end, with the taken path glowing.
If that reads clearly from one plain prompt, the foundation works.

**Proposed roadmap:** (1) node shape kinds (`decision`/`terminator`/`io`) +
any new `Shape` primitives; (2) edge labels on `connect`; (3) the ranked
TD/LR layout mode; (4) a flagship running-algorithm example + gallery/prompt
docs; (5) sequence-diagram foundation after this proves the shared model
stretches a second time.

**Decided:** node shapes are **kinds** on `node` (not builtins) — one authoring
form for every node, easy for creators.

**Decided (scheduling, this cycle):**
- **Edge labels → a `label(edge, "text")` verb**, not a `connect` arg. It stays
  general (labels any edge/lane midpoint across every diagram type) and keeps
  `connect`'s already-polymorphic args (bend / orthogonal / ports) clean, matching
  the post-hoc `color(edge, …)` styling pattern.
- **Ranked layout → a `flowchart(id, [TD|LR])` container**, not an `architecture`
  mode flag. It reads as authoring intent and defaults to top-down ranked layout,
  while **reusing the architecture substrate under the hood** (`ArchitectureData`
  + `relayout` + scale-to-fit + ports), so there is no second layout engine — a
  layout *mode* on the shared data, fronted by a clear builtin.

### C4 Model — ✅ Shipped (Context · Container · Component + auto-split + end-to-end animated story)

**Shipped (first draft):** `c4(id, [level])` container (reuses the architecture
region layout + scale-to-fit); C4 element **kinds on `node`** — `person`/`system`/
`container`/`component`/`external`, each an **outline-styled** rounded box (coloured
border + text, transparent fill, Structurizr-look) with **name / `[Type: tech]` /
description** (`node(id, parent, "kind", "name", [desc], [tech])`); a `person` gets
the classic **head circle atop the box**; internal elements in the accent colour,
externals greyed; relationships via `connect` + `annotate` (with a **backdrop chip**
so labels read over lanes/boxes — a general `annotate` win). **People-top C4
layout**: three tiers — people on top, internal systems/containers in the centre,
externals below (`relayout_c4`), which also puts the system dead-centre so `zoom`
can focus it. **Context→Container zoom** (the animated differentiator) works
author-composed — two overlapping `c4` diagrams, `zoom` into the centred system +
`fade` the surroundings, then `zoom` back and `show` the containers (`sticky` chrome
stays put); no new vocabulary. Relationship labels with a technology **auto-split**
to two lines (verb / `[tech]`) — the C4 convention, and narrow enough for a tight
gap. **Tier auto-split** (like flowchart column-wrap): a tier too wide to fit
wraps into balanced sub-rows (row-major, scale-to-fit) — 4 components fall into a
clean 2×2, 8 containers into 3-3-1 — so dense levels stay readable without author
layout math. Declaration order fills the grid row-major, so pairing declarations by
column (e.g. `signin, accounts` then `security, facade`) turns a 2×2 into two
crossing-free vertical flows. All three levels ship: `c4-internet-banking.manic`
(**Context**), `c4-internet-banking-containers.manic` (**Container**),
`c4-internet-banking-components.manic` (**Component**), and `c4-zoom.manic` (the
**zoom**). The capstone is **`c4-story.manic`** — a complete end-to-end animated
walkthrough: Context (pieces arrive) → zoom into the system → the Containers
**build along the flow** (a `GET /accounts` request leads; each box + its edge +
label materialises just as the dot arrives: browser → API → database) → zoom into
the API → the Components **build along the sign-in call** the same way → zoom back
out to Containers → out to the whole Context. Each drill-down is one continuous
build led by the moving dot (the flow *is* the reveal, not a separate beat after a
show-all); the return legs **sequence** the fade-out fully *before* revealing the
next level (no crossfade jumble) with a pull-back zoom. **Perimeter routing for
tier-spanning back-edges:** a C4 relationship that runs *upward* across tiers (e.g.
an external system's notification back to the person on top) would otherwise draw
straight through the centre and bisect the diagram; it now auto-bows out past the
box cluster to hug the margin (its label rides out with it), keeping the middle
readable — the same idea flowcharts use for long feedback loops. Verified on the
canonical bigbank container diagram (`c4-test.manic`, translated from the Python
`diagrams` library): the `E-mail System → Customer` edge routes around the side
instead of cutting through the API/SPA labels. (Implementation note: the
`annotate` label/chip carry the edge id only as a free tag, so a per-edge reveal
must `show({edge}.text)`/`show({edge}.text.bg)` explicitly — `show({edge})` alone
reveals the line but not its separately-added label.) All pass the editor gate. **To refine:**
within-tier ordering is author-controlled (declaration order = left→right/row-major,
so keep related boxes adjacent); auto-ordering by adjacency to cut crossings is a
future nicety. The design below is as shipped.



The Diagrams kit does **architecture** and **flowcharts**; the **C4 model** (Simon
Brown's Context → Container → Component → Code) is the standard way software teams
communicate architecture, and it's the highest-demand *notation* on top of the
substrate we already have. manic's edge is the same one that makes the flowchart
run: a C4 diagram here isn't a dead Structurizr/Mermaid picture — **the system box
zooms open into its containers, a container zooms into its components, and a real
request flows through it.** Architecture at every altitude, and it *moves*.

**Reuse first — C4 is boxes + labelled relationships + nested boundaries.** That is
almost exactly the shipped architecture substrate; do **not** rebuild it:

- **Boxes + layout** — `architecture`/`cluster`/`node` with auto-fit, scale-to-fit
  and ports already draw and place the elements.
- **Relationships** — `connect` (directed) + `annotate` (the C4 label: a verb
  phrase + technology, e.g. *"Reads/Writes"* `[SQL/TCP]`) apply unchanged.
- **Boundaries** — `cluster` is already a labelled, dashed grouping — a C4 system
  or container boundary.
- **Runtime story + reveal** — `route`/`flow` move a request through the
  containers; `step`/`say`/`draw`/camera narrate the walkthrough.

**Genuinely new — kept minimal (and consistent with the flowchart decisions):**

1. **C4 element *kinds* on `node`, not builtins.** Extend `node`'s kind vocabulary
   with `person`, `system`, `container`, `component`, and `external` (person or
   system). Each renders as its C4-styled box (a `person` gets a head/actor cap; an
   `external` is shaded) — selected by string exactly like the flowchart shapes:
   `node(api, sys, "container", "API")`. **Never a builtin per element.**
2. **Rich node content.** A C4 box shows three things — **name** (bold), a
   **`[type: technology]`** tag, and a one-line **description** — where today a node
   has a single label. This is the one real content addition. (Open: structured
   optional args `node(id, parent, "container", "API", "Spring Boot", "Handles …")`
   vs. an encoded multi-line label vs. thin `tech()`/`describe()` verbs.)
3. **C4 boundary style on `cluster`** — a dashed boundary tagged `[Software System]`
   / `[Container]`, i.e. a cluster style, not a new grouping primitive.
4. **The level zoom (the differentiator).** A `system` or `container` box expands to
   reveal its children — the Context→Container→Component transition, animated. First
   pass authored with existing `camera`/`show`/`fade`/`draw`; promoted to a helper
   only if it proves reusable.

**The animation story:** reveal **Context** (people, your system, external systems);
**zoom** the system box open into its **Containers**, relationships re-drawing;
optionally zoom a container into its **Components**; then `route` a request through
the containers ("watch a login travel web-app → API → database"). No static tool
does the zoom or the flow.

**Structurizr / authoring mapping:** `person` → `node "person"`; `softwareSystem` →
`"system"`; `container` → `"container"`; `component` → `"component"`; a relationship
`a -> b "Uses" "HTTPS"` → `connect(ab,a,b)` + `annotate(ab,"Uses [HTTPS]")`;
`systemBoundary`/`containerBoundary` → `cluster`. Faithful import, no new engine.

**Flagship demo:** the canonical C4 example — an *Internet Banking System* — drawn
at **Context** (customer · banking system · mainframe · email system) and then
**Container** level (web app · SPA · mobile app · API · database), with the system
box zooming from one to the next and a login request flowing through the containers.
If that reads from one plain prompt, the foundation works.

**Proposed roadmap:** (1) element kinds + rich node content (name/`[type: tech]`/
description) + boundary style; (2) labelled relationships via `connect`+`annotate`
(a two-line verb-plus-tech convention); (3) a Context flagship + a Container flagship
with a request flowing + gallery/prompt docs; (4) the Context→Container **zoom**
transition; (5) Component level + people-top/externals-around layout tuning.

**Decided (all, for ease of use):**
- C4 elements are **kinds on `node`** (one authoring form, as with flowchart
  shapes); relationships/boundaries/layout **reuse `connect`/`annotate`/`cluster`
  + the architecture substrate** — no second engine.
- **Rich node content = one call, optional trailing args:**
  `node(id, parent, "kind", "name", [description], [technology])` (Structurizr's
  order). Engine composes the box — **name** bold, `[Type: tech]` tag, description
  below. Degrades to `node(sys, bank, "system", "Internet Banking")`. No new verbs,
  no encoded labels.
- **Levels = a `c4(id, [level])` container** (level optional), the flowchart call:
  clear intent, reuses `ArchitectureData` + region layout + scale-to-fit. The C4
  element kinds are interpreted **in the context of a `c4` container** (so `external`
  reads as C4-external there, and the native archetype elsewhere — no prefix needed).
- **`person` = box-with-head** (Structurizr default) — fills a box with name +
  description and scales with the layout; not a stick figure.
- **Level zoom = author-composed first** (`camera`/`show`/`fade`/`draw`); promote to
  a helper only if one clean pattern recurs (reuse-first).

### Circuit Kit — recommended next domain exploration

Circuits are the highest-value candidate because they naturally form animated,
causal stories: **source → current flows → components respond → measured values
change → the system settles**. This exercises Manic's strengths instead of
introducing a disconnected drawing catalogue.

The kit should support three progressive layers:

1. **Textbook diagram:** standardized components, terminals, wires, polarity,
   values, labels, and automatic clean routing.
2. **Conceptual animation:** current/signal flow, switching, charging,
   component response, measurement, and synchronized explanation.
3. **Truthful computation:** a bounded set of solvable circuits whose displayed
   voltage/current values and plots come from the authored component values.

Keep the surface small and compositional. A possible design direction—not yet
accepted vocabulary—is a generic circuit/container, typed components, terminal
connections, probes, and a few semantic actions such as energize, measure, and
solve. Existing `flow`, `pulse`, `recolor`, `attach`, `plot`, `parameter`,
`rewrite`, `step`, and the future Director should handle most storytelling.
Avoid a bespoke animation verb for every component or circuit topology.

The flagship acceptance story should be an RC charging circuit. Closing one
switch must synchronously show charge/current flow, capacitor voltage rising,
current falling, and truthful plots/equations, with the Director guiding the
viewer from the physical circuit to the graphs and final time-constant insight.
Only after that representative story exposes genuine gaps should production
vocabulary be chosen.

## Manic ML kit — active implementation

**Status: ✅ ML1 feed-forward, ML2 learning + exact rollback, ML3 tensor/CNN foundations, ML4
single-head attention, ML5 token/embedding foundations, ML6 complete
transformer blocks, and ML7 exact decoding implemented locally.**
The kit makes an ML computation
understandable, not merely draw the familiar network silhouette. Its product
promise is:

> Give Manic a small model and an input. It computes the same values a learner
> sees, then reveals one meaningful flow at a time.

The supplied dense-network and transformer references define the visual bar,
not a request to copy their complexity. The dense graph shows why automatic
layout and weight styling are needed; the transformer explainer shows the
desired end state: tokens, Q/K/V, attention, residual flow, MLP, and output
probabilities remain spatially connected while the current computation is
bright and the surrounding model stays quiet.

### Design rules

1. **Progressive focus, not connection noise.** Inactive edges are faint or
   bundled. A forward/backward beat brightens only the active layer, receptive
   field, token, or attention head. Stroke width/opacity represents magnitude;
   a stable warm/cool palette represents sign. Colour never carries the only
   meaning.
2. **Truth before decoration.** Network outputs, activation values, gradients,
   convolutions, and pool selections are computed from the authored data. Shape
   errors, incompatible layers, invalid strides/windows, and non-finite values
   fail at the source location instead of producing plausible-looking art.
3. **Small vocabulary, reusable composition.** ML nouns describe computed
   structures; ML verbs describe computation direction. Existing `step`, `par`,
   `seq`, `stagger`, `flow`, `attach`, `become`, `turn`, `rewrite`, captions,
   plots, camera, and Creator templates remain the storytelling language. Do not
   add separate commands for every architecture or activation function.
4. **Deterministic and seekable.** Explicit values are preferred. Seeded sample
   values are reproducible. Forward and backward results are calculated at
   build time and animated as ordinary stateless tracks, so recording,
   scrubbing, stage export, and out-of-order frame requests agree exactly.
5. **Screen-aware by default.** Portrait may focus one layer/operator at a time;
   landscape may show the full pipeline. Node radius, spacing, labels, edge
   detail, feature-map tiles, and probability bars use level-of-detail rules and
   the existing safe-region/audit system. Large networks summarize rather than
   shrink into illegibility.
6. **Explanation, not a training framework.** V1 visualizes small explicit
   educational models. It does not load arbitrary PyTorch/TensorFlow programs,
   train large models, require a GPU, or pretend to be a general ML runtime.

### Creator surface

Keep the first surface near these nouns and verbs:

- `network` — **ML1 implemented** for layered feed-forward models with layer
  sizes, activations, deterministic seeded weights/biases, stable layer/node/
  connection tags, level-of-detail, and probability bars. Explicit authored
  weights and biases remain a planned extension.
- `tensor` and `kernel` — **ML3 implemented** as compact finite numeric grids
  with `;` rows and `|` channels, stable cell/slice tags, validated shared
  shapes, responsive cell sizing, and stacked multi-channel presentation.
- `activation` — **ML1 implemented** as a reusable truthful plot for `linear`,
  `relu`, `sigmoid`, and `tanh`; the network accepts activation names as data,
  including vector-valued `softmax`, rather than requiring a command per
  function.
- `convolve` and `pool` — **ML3 implemented** as derived tensor views with
  validated channel/kernel/window shapes, integer stride/padding, convolution
  bias/cellwise activation, multi-channel accumulation, max/average pooling,
  and deterministic first-row-major max ties. `pool(..., max, ...)` covers max
  pooling without a one-off `maxpool` word.
- `forward` — **ML1 implemented** with validated inputs, real affine/activation
  computation, stateless seekable tracks, progressive edge/node focus, and
  exact settled values.
- `loss` — **ML2 implemented** for mean-squared error and numerically stable
  softmax cross-entropy, with exact target-size/distribution validation and
  persistent prediction-versus-target readouts.
- `backward` and `update` — **ML2 implemented** as separate beats: exact
  reverse-mode gradients travel over the persistent graph, then one explicit
  gradient-descent update changes every weight/bias and recomputes the same
  input, output, and loss. No hidden training loop or optimizer catalogue.
- `checkpoint` and `restore` — **ML2 implemented** for one exact authored
  rollback: save weights, biases, prediction, target, and loss before an update,
  then restore every saved value with deterministic reverse-flow animation.
  This is explicitly not dataset-level machine unlearning.
- `scan` — **ML3 implemented** as one shared stateless animation for a
  convolution or pooling window: the source region, operator, truthful
  arithmetic summary, selected maximum, and output cell stay linked.
- `tokenize` and `embedding` — **ML5 implemented** for deterministic word,
  character, or authored token boundaries; explicit or stable seeded
  educational lookup vectors; and exact `none` or sinusoidal position values.
  Repeated token identities reuse one base vector before position is added.
- `transformer` and `encode` — **ML6 implemented** as one complete deterministic
  block over an ML5 embedding: 1–4 heads, full/causal masks, concatenation and
  output projection, pre/post layer normalization, both residuals, GELU/ReLU/
  SiLU/Tanh MLPs, and true seeded inverted dropout in explicit training mode.
- `logits` and `sample` — **ML7 implemented** as a separate educational LM
  projection over one final hidden row, complete temperature-scaled stable
  softmax, exact greedy/categorical/top-k/top-p decoding distributions,
  renormalization, and reproducible seeded selection.
- `attention`, `attend`, and `topk` — **ML4 implemented** as one deterministic
  scaled dot-product head from explicit embeddings, a stateless 1-based token
  focus, and a seeded residual-to-vocabulary projection whose displayed bars
  are exact full-softmax probabilities. This is an educational computation,
  not a pretrained-model claim.

This is the shipped ML1–ML7 vocabulary budget. Every word has catalog/editor
parity and creator examples proving that it composes with ordinary Manic
entities.

### Delivery plan

#### ML1 — feed-forward foundations

- ✅ Add an isolated `ml` kit and matching language-service catalog entries; no
  existing constructor or verb changes semantics.
- ✅ Implement auto-layout for input, hidden, and output layers with stable tags,
  collapsed-edge detail for large layers, labels, legends, and probability
  bars.
- ✅ Compute dense affine layers plus `linear`, `relu`, `sigmoid`, `tanh`, and
  numerically stable `softmax`; animate a forward pass from explicit inputs.
- ✅ Ship a beginner story: features → hidden evidence → class
  probabilities, with one selected prediction explained rather than every
  connection flashing together.
- ✅ Cover deterministic arithmetic, invalid dimensions/inputs, large-logit
  softmax stability, level-of-detail computation, direct seeking, catalog
  parity, all four canvas audits, creator documentation, and publishing
  metadata without changing an existing DSL call.

#### ML2 — backward pass and learning

- ✅ Add mean-squared-error and softmax cross-entropy loss views, exact local
  derivatives, and reverse-mode gradients for the ML1 operations.
- ✅ Animate loss → output gradient → hidden gradients → weight/bias
  gradients along the same persistent identities used by the forward pass.
- ✅ Show gradient sign/magnitude and the actual values at nodes, preserving
  very small gradients rather than exaggerating them; show an explicit
  representative old/new weight comparison. Keep gradient flow and parameter
  update as separate visual beats.
- ✅ Verify representative gradients against finite differences and ensure the
  animation ends on the exact computed values.
- ✅ Add zero-time supervised checkpoints and exact animated restore of weights,
  biases, node values, output bars, targets, and loss. Restored state clears
  gradients and requires another `backward` before updating; documentation
  distinguishes rollback from dataset-level unlearning.
- ✅ Ship a responsive creator story, language/system-prompt/mdBook guidance,
  gallery and publishing metadata, order/target/hyperparameter diagnostics, and
  direct-seeking regression coverage.

#### ML3 — convolution and pooling

- ✅ Add 2-D single/multi-channel tensors, kernels, valid/same-style authored
  padding, integer stride, bias, activation, and deterministic max/average
  pooling. Resolve max-pool ties by a documented stable rule.
- ✅ Make `scan` coordinate four things continuously: receptive-field highlight,
  moving kernel, arithmetic/value card, and destination feature-map cell.
- ✅ Ship a CNN story: image → edge/feature kernels → feature maps → ReLU →
  max pooling → compact evidence ready for a classifier. The creator can pause or focus any stage
  without manually moving dozens of squares.
- ✅ Cover exact convolution fixtures, multi-channel accumulation,
  stride/padding/bias/activation, exact max/average pooling, deterministic tie
  selection, malformed shapes/channels/windows, direct seeking, catalog parity,
  all four canvas audits, creator documentation, and publishing metadata.
- ✅ Keep the first surface explainable: one `convolve` produces one feature
  map; multiple filters are multiple named kernels/outputs rather than an
  opaque architecture constructor. Convolutional gradients remain later work.

#### ML4 — complex animation and transformer acceptance target

- ✅ Add computed connection fields, persistent residual lanes, fan-in focus,
  a labelled attention heatmap, weighted value outputs, and top-k probability
  bars without changing any existing DSL semantics.
- ✅ Compute seeded Q/K/V projections, `softmax(QK^T / sqrt(d))`, exact weighted
  value mixes, the selected residual, and a deterministic educational output
  projection from explicit embeddings and candidate labels.
- ✅ Ship the small public surface `attention`, `attend`, and `topk`; keep focus
  stateless and directly seekable, with stable token/role/weight/fan tags.
- ✅ Build and audit one responsive transformer story across all four canvases,
  document the pretrained-model boundary, and cover normalization, exact mixes,
  reproducibility, top-k probabilities, diagnostics, and catalog parity.
- ML4 remains the focused one-head teaching view; ML6 supplies complete
  multi-head residual/MLP blocks. Imported model weights, packaged/pretrained
  tokenizers, and large-model inference remain future work.

#### ML5 — tokens, embeddings, and position

- ✅ Add `tokenize` for three honest input modes: `authored` boundaries separated
  by `|`, deterministic Unicode-aware `word` splitting, and deterministic
  `character` splitting. Do not label a heuristic as BPE; real BPE waits for an
  explicit vocabulary/merge table or packaged model tokenizer.
- ✅ Add `embedding` as the bridge from a token sequence to numeric vectors. It
  accepts either explicit authored rows or a small deterministic seeded
  educational dimension. Generated vectors must be labelled as seeded
  educational values, never as weights from a pretrained model.
- ✅ Support `none` and exact sinusoidal positional encoding. Preserve separate
  token, token-vector, position-vector, and combined-vector identities so the
  learner can see `token embedding + position = model input` without a redraw.
- ✅ Keep the public surface to the two nouns above. Existing `step`, `show`,
  `draw`, `pulse`, `attach`, `become`, captions, equations, and tags own the
  storytelling; token splitting and vector addition do not need one verb per
  substage.
- ✅ Validate empty text, unsupported tokenizer modes, token/value row mismatch,
  ragged/non-finite rows, dimensions, and positional mode at the exact source
  argument. Cover Unicode/punctuation boundaries, sinusoidal fixtures,
  deterministic seeds, exact vector addition, direct seeking, all canvas
  formats, catalog parity, creator documentation, and one token-to-positioned-
  embedding story.

#### ML6 — complete transformer blocks

- ✅ Add one `transformer` noun and one `encode` staged computation verb on top of
  an ML5 `embedding`. The compact specification is
  `"heads=2 mask=causal mlp=12 activation=gelu norm=pre dropout=0 mode=inference seed=41"`;
  width and height remain ordinary optional layout arguments. This controls the
  complete block without adding a word for every internal operation.
- ✅ Compute every head as scaled Q/K scores → mask → stable softmax → weighted V,
  then concatenate heads and apply the output projection. Require `d_model` to
  divide exactly by the head count.
- ✅ Compute layer normalization, attention/MLP residuals, GELU/ReLU/SiLU/Tanh
  MLPs, and deterministic inverted dropout in explicitly authored training
  mode. Dropout is disabled during inference and is never simulated with mere
  opacity.
- ✅ Expose stable tags for heads, masks, matrices, concatenation, both residuals,
  both norms, MLP activation, dropout masks, and block output while keeping the
  inactive computation quiet.
- ✅ Ship catalog/editor parity, actionable diagnostics, seven numerical and
  direct-seeking regressions, a native-reviewed creator story, all four canvas
  audits, mdBook/system-prompt guidance, and gallery/publishing metadata.

#### ML7 — logits, temperature, and sampling

- ✅ Keep the architecture accurate: the transformer MLP produces a hidden
  representation; a separate language-model projection produces logits.
- ✅ Add one `logits` probability view that applies `logits / temperature` followed by a
  stable full softmax, then one deterministic sampling verb supporting greedy,
  categorical, top-k, and top-p selection.
- ✅ Temperature changes the complete distribution; top-k/top-p exclusions are
  exact zeros before renormalization, and sampled outcomes are reproducible from
  the same seed. Displayed values are computed educational probabilities unless
  a future explicit model package supplies real weights and tokenizer data.
- ✅ Keep the creator surface compact: `logits(next, block, token, at, labels,
  temperature, ...)` and `sample(next, "top-p 0.9 seed=17", ...)`. Expose stable
  projection, temperature, logit, probability, bar, and candidate tags.
- ✅ Cover exact projection arithmetic, stable full softmax, entropy changes,
  every filter, minimal top-p support, deterministic draws, invalid contracts,
  direct seeking, catalog parity, four-canvas layout, creator documentation,
  gallery metadata, and one cool-vs-warm next-token story.

#### Native ML1–ML7 visual re-review — completed 2026-07-21

The numerical/DSL milestones above remain complete. This review records a
separate creator-polish backlog; it does not reopen the compact vocabulary or
make framework import part of the first release.

| Story | What already reads well | Audit finding / next opportunity |
|---|---|---|
| Scalar → tensor | The rank progression, persistent values, arrows, and restrained semantic colour are immediately legible. | Let the same cells visibly `become` the next rank instead of relying mainly on reveal; a shallow parallax/depth cue can make channel stacking feel spatial without turning it into a 3-D spectacle. |
| Activation focus | The ReLU bend, probes, equation, and negative/positive comparison are the cleanest minimal ML lesson. | Travel one value marker into, through, and out of the curve; use a small camera push only at the bend, then settle back for the conclusion. |
| Forward pass | Computed node values and probability bars are truthful and readable. | The settled frame leaves large empty space between floating node columns and loses the causal path. Keep the full edge field as a very quiet scaffold, brighten only a bounded fan-in/fan-out, and send a few computed signal pulses along the strongest contributing edges. |
| Learning step | The same network survives prediction, loss, backward credit, and update without a redraw. | Gradient text can crowd node circles and a dense reverse edge bundle becomes noisy. Move gradient readouts to small outside badges, use sign plus magnitude styling, and focus one transition at a time while the remaining graph stays as context. |
| CNN edge story | Source, kernel, receptive field, feature map, arithmetic strip, and destination stay synchronized; this is already a strong operator story. | Briefly zoom into one receptive field, flow its nine values into the multiply/sum beat, and use a diverging negative/zero/positive palette before returning to the complete feature map. |
| Tokens + embedding | Token identity, repeated lookup rows, position, and combined model input remain aligned. | Under mono/low contrast, position and combined cells can look like empty colour boxes. Add compact values, bars, or phase glyphs for the focused dimension and animate `token + position → model input` with identity-preserving cell motion. |
| One attention head | Selected Q/K/V, the exact softmax row, weighted connections, and residual context are mathematically honest. | The weighted-V `mix` cards read as mostly empty destinations. Put a compact output-vector summary inside them, scale focused streams by attention weight, and let a small token/value packet travel only on the selected fan-in. |
| Complete transformer block | The complete order—MHA, projection, residual, norm, MLP, residual—fits on one persistent canvas. | This has the clearest empty-box problem: `ADD 1`, the MLP, and `ADD 2` are labelled containers without enough internal evidence. Replace placeholders with mini-views: residual bypass curves, before/after vector bars, normalization spread, and MLP expand–activate–contract motion. Follow one token through the block while other token lanes remain dim context. |
| Logits + sampling | The separate LM projection, temperature, exact probabilities, zeroed exclusions, renormalization, and selected token are clear. | Morph one distribution continuously between temperatures instead of fading between duplicate panels; a single seeded choice marker can travel from the retained probability mass into the selected token row. |

#### ML visual elevation pass — completed 2026-07-21

The first high-priority polish pass is implemented without adding ML-only
animation vocabulary:

- ✅ Weighted-attention destinations now contain compact summaries of their
  actual mixed vectors. Selected Q/K/V fan-in and the residual route carry
  directly seekable flow derived from the computed attention row.
- ✅ Transformer projection, residual, and MLP cards now contain signed
  value-driven mini bars rather than empty labelled panels. Two visible bypass
  curves explain residual identity; the main connectors and skips flow in
  computation order during `encode`.
- ✅ Position and final model-input cells now display their numeric values.
  Positive token, position, and combined values use distinct cyan, gold, and
  lime roles; signed negatives remain magenta, so the table stays meaningful
  under custom colour themes.
- ✅ A completed forward pass keeps a quiet contribution-weighted edge trace
  instead of returning every connection to the same low-opacity scaffold.
- ✅ `backward` preserves activation values inside neurons and places compact
  signed gradient badges outside them. `update` clears the badges before
  recomputing, retaining one persistent network without crowded `g...` labels.
- ✅ Activation, dense forward, backward learning, CNN scanning, attention, and
  the full transformer story use restrained responsive camera focus. Every
  move returns to the overview; scalar/tensor, embedding, and logits keep a
  fixed camera because their comparison is strongest as a complete still.
- ✅ Added visual-evidence regressions for attention summaries, embedding
  values, Transformer mini-views/bypasses, and gradient-badge lifecycle. All 51
  ML regressions pass, and all nine ML1–ML7 stories pass portrait, feed, square,
  and landscape publishing audits.

#### ML motion and visual-polish grammar

Do not add a new ML word for each effect. Compose the already shipped ML nouns
and computation verbs with the generic Manic motion surface:

- `flow` is the default for information moving along a connection; edge width,
  brightness, and pulse count should derive from a real activation, attention
  weight, or gradient magnitude.
- `travel` carries one persistent value/token marker through a path and stops at
  its exact computed destination. `attach` keeps its label or readout with it.
- `become` preserves identity when a scalar becomes a vector, a token vector
  gains position, a receptive field becomes an output cell, or logits become a
  filtered distribution.
- `particles` are semantic packets, not confetti. A bounded 3–8 particles may
  represent tokens, activation flow, weighted values, or a sampled draw only
  when their count/path has an explainable meaning. `wander` remains for genuine
  stochastic motion stories, not ordinary neural-network decoration.
- `par`, `seq`, and `stagger` remain the timing language: structure first,
  focused computation second, exact settled result third.

The existing `cam` and `zoom` words are sufficient for the first polish pass.
Examples should use them sparingly: one 1.08–1.25× push per conceptual beat,
smooth pan to the active operator, a short reading hold, then a settled overview.
Portrait should use smaller pushes and vertical pans. Camera movement must never
hide a caption, compete with dense text, or become necessary to understand the
final still. A later automatic framing policy may compute these targets from
stable tags and group bounds without adding another camera vocabulary.

#### Semantic colour system

Colour should explain computation rather than merely make the frame busier:

- input/data: cyan or cool blue;
- positive activation/retained signal: lime;
- negative contribution or reverse gradient: magenta/red;
- operator/transformation/current focus: gold;
- zero, masked, excluded, or inactive context: dim neutral;
- Q/K/V: three stable roles reused throughout the complete transformer story;
- selected prediction/sample: the strongest foreground plus one accent.

Every distinction must survive the default mono template through brightness,
stroke width, fill/pattern, labels, and motion direction. Custom templates may
retint the semantic roles, but a hard-coded rainbow is not the default. A
diverging scale is appropriate for signed kernels/gradients; a sequential scale
is appropriate for probabilities and attention weights.

#### Empty-container and focus rules

1. **No explanatory empty box.** A labelled operator visible for more than one
   beat must contain a value, compact diagram, progress trace, or moving identity.
   Otherwise collapse it to a small labelled connector until it becomes active.
2. **Overview → focus → overview.** Preserve the architecture, expand only the
   active stage, then settle on the complete result. Never clear the screen just
   to obtain focus.
3. **One hero identity.** Dense stories follow one token, neuron, receptive
   field, or candidate while the rest remain truthful low-opacity context.
4. **Motion is computed evidence.** Particle routes, edge emphasis, bar changes,
   and colour must be functions of the same retained values shown in text.
5. **Direct seeking stays pure.** Camera tracks, packets, and focused mini-views
   must remain deterministic at any requested time and hold exact endpoints.

The remaining optional polish is deliberately lower priority: identity-preserving
`become` journeys for scalar → tensor and embedding addition; a continuous
single-panel temperature morph for ML7; a travelling input probe through the
activation curve; and tag-bounds-driven automatic camera targets. These should
reuse the current generic verbs and must not destabilize the completed visual
or numerical contracts.

### End-goal visual design

The professional default is a persistent left-to-right computation canvas:
input/data on the left, the currently active operator in the visual centre, and
the derived value or decision on the right. A small explanation strip below
shows only the arithmetic for the highlighted unit/cell/token. Forward motion
uses a restrained directional pulse; backward motion follows the same geometry
in reverse with gradient styling; neither clears and redraws the whole scene.

For dense models, overview mode uses low-opacity connection bundles and opens
only the selected node's fan-in/fan-out. For CNNs, the kernel window and output
cell share one accent and move as an attached pair. For transformers, token
lanes persist through embeddings, Q/K/V, attention, residual, MLP, and
probabilities; zoom/focus changes emphasis without breaking those identities.
Every design must work in monochrome through width, pattern, labels, and
brightness even when the theme supplies colour.

### Acceptance and regression contract

- Unit tests: dense shapes and affine values; every supported activation and
  derivative; stable softmax/cross-entropy; convolution across channel,
  padding, and stride cases; pooling and deterministic ties; attention row
  normalization and exact weighted mixes; token boundaries, seeded lookup
  identity, sinusoidal fixtures, and exact position addition; malformed/
  non-finite diagnostics.
- Numerical tests: finite-difference checks for representative dense and
  convolution gradients, with documented tolerances and exact expected fixtures
  for small textbook examples.
- Timeline tests: exact endpoints, no blank frames between stages, deterministic
  seeds, direct seeking before/during/after `forward`, `backward`, `scan`, and
  `attend`, and parity between a full render and a selected named stage.
- Layout/audit tests: small and dense networks, portrait/square/landscape,
  readable labels, bounded edge detail, no unsafe-region overflow, and a
  meaningful low-detail fallback.
- Product artifacts: one focused example per milestone, one advanced connected
  story, mdBook creator documentation, catalog/system-prompt coverage, gallery
  and publishing entries, plus full `cargo test` regression before shipment.

### Deliberately later

Arbitrary framework import, large-model training, recurrent/stateful sequence
simulation, optimizer catalogues, automatic differentiation over general Manic
expressions, 3-D network spectacles, and architecture-specific dashboards stay
outside the first release. They should be reconsidered only after the shipped
milestone stories expose a repeated creator need that the compact surface
cannot express.

## Motion Graphics V2 — active implementation

**Status: 🟡 V2 relationship surface implemented locally.** `attach`, `become`,
and `turn` are valid DSL with native/editor catalog parity, focused Rust tests,
a generic example, system-prompt guidance, and mdBook documentation. The live
moving-path and generated-motion foundation described below remains active work.

Motion Graphics V2 is continuity infrastructure, not an effects catalogue. Its
product promise is:

> Describe what an object becomes, follows, or turns around. Manic preserves
> identity, motion continuity, and the exact settled frame.

The design deliberately adds only three creator-facing words. Existing motion
calls retain their signatures, ordinary files require no flag, and advanced
matrix/path controls remain available without becoming the default teaching
surface.

### Simplicity rules

1. **Relationships instead of intermediate coordinates.** An author states
   `attach(label, marker, ...)` or `become(curve, guide, ...)`; the engine owns
   the in-between frames.
2. **One obvious word per intent.** `travel` moves an object along a path,
   `flow` sends transient emphasis, `attach` keeps two objects together,
   `become` changes identity continuously, and `turn` rotates a whole layout.
3. **No domain vocabulary.** The same calls must serve graphs, UI cards,
   particles, geometry, algorithms, product demos, quizzes, and social video.
4. **Useful defaults, explicit escape hatches.** Default easing and duration
   remain familiar; authors can still pass duration/ease or use `transform` and
   `to` for low-level control.
5. **A still frame is part of motion design.** Every verb has an exact endpoint
   and holds it. Settling never depends on accumulated frame state.
6. **No production flags.** A `.manic` file behaves the same through native
   preview, recording, browser/editor, and backend execution.

### V2Core engine foundation — no new DSL

#### 1. Unified authored state

**✅ Implemented.** The partial position-only bookkeeping is now complemented by
one build-time authored-state record per 2-D entity. It tracks the endpoint needed
to compose subsequent actions: position, path endpoint, rotation, scale,
opacity, colour/hue, trace/morph state, and the current geometry blueprint.

Every geometry-aware verb reads the previous authored endpoint and writes its
own endpoint. The base scene remains the immutable `t=0` world and
`Timeline::apply(base,t)` remains pure. This is what makes the following chains
continuous rather than special-cased:

```text
move → travel       arrange → turn       transform → transform
grow → become      become → move        scale → attach
```

#### 2. Shared runtime path progress — planned

Give path-like entities one normalized arclength evaluator shared by `draw`,
`flow`, and `travel`. `travel(entity,path,dur,ease)` keeps its current syntax and
settled endpoint, but internally animates path progress rather than expanding
to dozens of unrelated position tracks.

The first release supports the existing path family—line, arrow, curve, plot,
spline, and arc—plus circle/rectangle/polygon boundaries where the direction is
unambiguous. A marker can follow a path while that path moves or morphs in the
same `par` block. Tangent-facing, partial ranges, reverse travel, and looping
remain later extensions unless a concrete example proves they need syntax.

#### 3. Group bounds and pivots — partial

**✅ Shared pivots shipped:** `turn` resolves one entity or every 2-D member of a
tag and moves positions, path endpoints, and curve controls along the same
circular turn. General deterministic group bounds/centre APIs remain planned
for relative placement and future visual audits.

#### 4. Velocity-continuous generated motion — planned

`wander` and unordered `arrange` remain seeded and scrubbable, but their sampled
paths must have continuous position and direction at segment boundaries.
Particles stay inside convex containers and finish exactly where the authored
layout says; V2 does not introduce a runtime particle simulator.

### The three V2Core creator words

#### `attach(child, target, [(dx,dy)])`

Keep an existing entity pinned to another entity with an optional screen-space
offset:

```manic
attach(name, marker, (0,-36));
travel(marker, curve, 2, smooth);
```

The child follows the target after ordinary tracks, derived geometry, links,
path travel, particle layout changes, and parameter bindings resolve. It also
inherits the target's visibility/opacity, matching the proven internal behavior
already used by labels. Attachment is a persistent scene relationship.
`attach(child, none)` releases it at the current authored position, avoiding a
second `detach` word while letting the child move, fade, or attach elsewhere.

#### `become(source, target, [duration], [ease])`

Continuously change one existing object into a declared target blueprint while
retaining the source id:

```manic
line(guide, (220,700), (860,700)); hidden(guide);
become(curve, guide, 0.8, smooth);
```

- compatible shapes interpolate geometry and style;
- open paths remain open and closed outlines remain closed;
- the settled source matches the target geometry, colour, stroke, and relevant
  shape styling exactly;
- unsupported pairs use a deterministic local crossfade instead of malformed
  geometry;
- the target's authored visibility is not changed—use a hidden target when it
  is only a blueprint; hidden blueprint opacity does not hide the source;
- equations continue to prefer `rewrite`, which preserves matching LaTeX parts;
  `become` may safely crossfade text/image content but does not pretend to
  understand mathematical semantics.

`morph` + `to(...,morph,...)` remain valid for explicit fraction-controlled or
spinning morphs. `become` is the common creator path, not a breaking replacement.

#### `turn(id_or_tag, pivot, degrees, [duration], [ease])`

Rotate one entity or a tagged layout around a literal point or another entity's
position:

```manic
turn(finalRing, finalOrbit, 18, 0.55, out);
```

Unlike `spin`, which rotates each addressed entity around its own anchor,
`turn` preserves group-local offsets and rotates the layout as one rigid system.
Unlike `transform`, it needs no matrix coefficients. Paths rotate both anchors
and endpoints; entity orientation rotates with the group. Existing `spin` and
`transform` behavior stays unchanged.

### Existing vocabulary after V2

The foundation is deliberately domain-neutral: **objects + paths + motion +
timing + composition**. `travel` moves any ordinary 2-D entity over a supported
path; `flow` animates identity-free activity over any path, including a free
curve connected to nothing. `seq` expresses creator-authored order and `par`
expresses creator-authored simultaneity. No object kind, icon, label, or filename
changes these rules. `examples/motion-flow-foundation.manic` is the acceptance
story for selected, ordered, simultaneous, and purely decorative flow.

| Intent | Call | V2 behavior |
|---|---|---|
| Move once along a path | `travel(entity,path,dur,ease)` | unchanged DSL and exact endpoint; live moving-path progress remains planned |
| Temporary or sustained path emphasis | `flow(path,dur,[direction],[mode])` | existing two-argument pulse is unchanged; `forward|reverse|both` and `once|continuous` add finite directional streaming without object identity |
| Ambient contained motion | `wander(group,dur)` | unchanged seeded, deterministic contained movement |
| Change particle layout | `arrange(group,container,"random|grid|ring",dur,ease)` | same DSL; persistent ids and exact final layout |
| Change an ordinary property | `to(id,property,value,dur,ease)` | remains the general escape hatch |
| Mathematical linear map | `transform(id,pivot,a,b,c,d,dur,ease)` | unchanged; still the precise matrix tool |
| Rotate in place | `spin(id,degrees,dur,ease)` | unchanged; no tag semantic regression |

No `enterLeft`, `floatUp`, `moleculeMotion`, `heatFlow`, or effect-specific
families are added. `par`, `seq`, `stagger`, `step`, ordinary movement, and the
three relationship verbs cover the choreography.

### V2 professional-polish layer — after V2Core

Only after the continuity foundation is stable:

- **`gradient(path, from, to)`** — arclength-based colour on stroked paths, so a
  creator does not split one curve into several plots to obtain a professional
  warm-to-cool stroke. It must compose with trace, dash, glow, and flow.
- **`trail(entity, seconds, [color])`** — deterministic recent motion history
  sampled from the resolved timeline, useful for a cursor, projectile, orbit,
  vehicle, or graph marker.
- **`motion("editorial|calm|snappy|playful")`** — an optional project-level
  default duration/easing profile. Explicit verb arguments always win and files
  without `motion(...)` render exactly as before.

Masking, arbitrary emitters, collisions, full particle physics, custom easing
editors, node timelines, and broad SVG animation are not V2Core. They require
separate evidence and must not delay continuity.

### Regression contract

Motion Graphics V2 cannot ship unless all of these hold:

1. Every existing `.manic` file parses without edits and retains its authored
   stage boundaries and total duration.
2. Existing verb signatures and tag-broadcast semantics remain valid.
3. Existing static-path `travel`, `flow`, `wander`, and `arrange` settle on the
   same final frames; enhanced behavior is visible only in newly supported
   combinations or new V2 calls.
4. Repeated/out-of-order `Timeline::apply(base,t)` produces identical frames.
5. Native and browser/editor catalogs expose the same arity, completions, and
   diagnostics; backend execution needs no additional options.
6. `become` always has an exact final blueprint and a safe crossfade fallback.
7. Attachment cycles, missing targets, invalid pivots, unsupported path types,
   non-positive durations, and new V2 operations that create incompatible
   simultaneous writes fail at parse/build time with source spans—never during
   rendering. Existing legal overlap/composition semantics remain unchanged.
8. Existing examples and the full Rust suite pass before visual improvements
   are judged. Numeric motion-continuity tests and milestone stills are added on
   top, not substituted for regressions.

### Delivery plan

1. **✅ V2.0 state foundation:** authored endpoint/blueprint record and
   chain-composition tests; incompatible/cyclic relationship diagnostics ship.
2. **🟡 V2.1 path + group foundation:** shared pivots ship; normalized live path
   dependency, general group bounds, and smoother generated motion remain.
3. **✅ V2.2 creator surface:** `attach`, `become`, and `turn` ship together with
   editor catalog/system-prompt parity and clear errors.
4. **✅ V2.3 proof examples:** `examples/motion-graphics-v2.manic` remains the
   compact three-verb acceptance scene, while
   `examples/motion-graphics-v2-story.manic` composes the relationship surface
   with `to`, `travel`, `flow`, `spin`, `arrange`, `wander`, `rewrite`, `seq`,
   `par`, and `stagger` in one continuous three-act creator story. Both ship in
   the gallery, publishing metadata, and the mdBook Motion Graphics chapter.
5. **V2.4 professional polish:** evaluate `gradient`, `trail`, and opt-in motion
   profiles independently; each must ship with its own tests and generic example.
6. **V2.5 publishing safety:** add transition-path collision, detached
   attachment, excessive speed/jerk, group-bound, and moving-camera checks to
   the visual audit.

The shipped relationship-surface acceptance example demonstrates, in one
generic file, a marker travelling along a path, a label remaining attached and
then releasing, the marker becoming a declared blueprint, particles arranging
into a ring, the ring turning briefly, and every element holding a clean final
frame. The advanced companion carries one question through attention, notation,
model, and coordinated-system acts without resetting the scene. Both use the
ordinary file-only production path with no flags. A marker following a
simultaneously changing path remains the acceptance test for the planned
normalized live-path foundation.

## 3D V2Core — ✅ shipped

**Status: ✅ V2.0–V2.5 implemented, documented, visually reviewed, and verified.**
The existing 3D engine already provides depth-tested primitives, curves,
surfaces, solids, extrusion, morphing, projected labels, deterministic camera
tracks, and stable pole-crossing orbits. V2 is therefore an authoring and
composition layer, not a second mesh engine.

The product promise is:

> Describe what travels, follows, transforms, turns, and deserves focus. Manic
> preserves the spatial world and composes the shot.

### Simplicity and compatibility rules

1. Every new spatial creator word keeps the existing `3` suffix convention.
2. Existing `camera3`, `move3`, `shift3`, `rotate3`, `orbit3`, `roll3`,
   `look3`, `follow3`, and `morph3` keep their signatures and settled frames.
3. V2 words are high-level compositions over shared authored state; they do not
   create a second timeline or require a mode/renderer flag.
4. Camera fitting uses real group bounds and a creator margin, not guessed
   object-type constants.
5. Spatial rotation follows one stable axis interpolation and must not shock,
   flip, or take a surprising Euler route.
6. Rendering polish comes from templates/defaults. V2Core does not expose a
   material graph, arbitrary light rig, texture system, or node editor.

### The five creator words

- **`view3(target_or_tag,"front|side|top|isometric|fit",[duration],[ease],[margin])`**
  aims the camera at the resolved target bounds and chooses a distance that
  keeps the subject framed. `fit` preserves the current viewing direction;
  named views select a familiar direction. Existing `look3` and `orbit3`
  remain the exact camera controls.
- **`travel3(entity,path3,[duration],[ease])`** moves one persistent 3D entity
  along a `line3`, `arrow3`, or `curve3` and leaves it at the exact endpoint.
- **`attach3(child,target,[(dx,dy,dz)])`** establishes a timed spatial
  relationship. **`attach3(child,none)`** releases at the resolved position
  without snapping. Constructor-time `follow3` remains useful for relationships
  that last for the entire movie.
- **`become3(source,blueprint,[duration],[ease])`** retains the source id and
  adopts the target geometry, transform, and style. Compatible families use
  the existing 3D morph machinery; unsupported pairs crossfade locally and
  still settle on the exact blueprint.
- **`turn3(id_or_tag,pivot,axis,degrees,[duration],[ease])`** rotates a spatial
  entity or group rigidly around one world-space pivot and axis, preserving
  member distances and orientations.

### V2Core engine foundation — no extra creator vocabulary

1. **Authored 3D endpoint/blueprint state.** Track the latest authored position,
   rotation, scale, endpoints, mesh/shape blueprint, style, and visibility
   without mutating the immutable `t=0` scene.
2. **Stable spatial rotation.** Use axis/quaternion interpolation internally for
   relational turns and camera transitions; keep existing Euler constructor and
   `rotate3` input compatibility.
3. **World/group bounds.** Resolve bounds after authored transforms for points,
   paths, surfaces, meshes, solids, and tag groups. Camera composition and later
   audit layers consume the same calculation.
4. **Deterministic 3D path sampling.** Convert supported authored paths into
   ordinary absolute-time position tracks so reverse scrubbing and recording
   produce identical frames.
5. **Exact visual transitions.** Reuse the shipped `morph3` resampling for
   compatible families and install the exact target blueprint at completion;
   use a bounded source-local crossfade otherwise.

### Delivery priority

1. **✅ V2.0:** authored 3D state, quaternion-backed stable axis rotation, and
   transformed world/group bounds.
2. **✅ V2.1:** `view3` camera composition and aspect-aware fit tests, including
   portrait bounds containment.
3. **✅ V2.2:** deterministic `travel3` plus timed `attach3`/release.
4. **✅ V2.3:** rigid `turn3` plus compatible morph and safe-crossfade
   exact-blueprint `become3`.
5. **✅ V2.4 core polish:** conservative template-aware ambient/key/fill diagram
   lighting with readable back faces. The follow-on creator roadmap adds smooth
   normals, mesh emphasis, depth/shadow cues, and bounded finishes through the
   single opt-in `finish3` modifier.
6. **✅ V2.5:** `examples/three-d-v2.manic` is the compact acceptance scene and
   `examples/three-d-v2-story.manic` is the continuous vertical creator story;
   mdBook/system-prompt/editor/publishing parity is complete. The full workspace
   suite passes 304 tests (258 engine/library, 2 CLI, 44 language/editor), and
   the engine suite validates every shipped example through the editor catalog.
7. **✅ V2 textbook dimension-story series:** eight approved portrait examples
   cover continuous 1D→2D→3D construction, nested distance, changing units,
   coordinate addresses, revolution into a solid, statistical dimensions, the
   reverse 3D→2D→1D journey, and curved-solid sections. The final
   `textbook-watermelon-sections.manic` uses bounded `param3` shells and section
   faces for horizontal/vertical halves and a quarter/three-quarter construction.
   Together they are the accepted V2 composition path for textbook spatial
   stories and mathematically exact authored curved sections; they add no new
   DSL word.

### Deferred to 3D V3 — generic solid sections

V2 deliberately stops at authored parametric sections. A future V3 may add one
engine-level solid-section or projection bridge that can cut an arbitrary solid,
generate the exposed faces, preserve the resulting piece identities, and move
smoothly between the 3D construction and its 2D textbook projection. That work
must reuse the current timeline, bounds, camera, and audit contracts; it must not
introduce a boolean-node vocabulary or change the settled output of V2 files.

### Regression contract

1. All existing 3D examples parse unchanged and retain their duration and
   authored final states.
2. Repeated or reverse-order `Timeline::apply(base,t)` remains deterministic.
3. `view3` always contains the requested bounds at its settled frame and never
   jumps through a pole or invalid camera basis.
4. `travel3` settles exactly on the path endpoint; `attach3` release preserves
   the last resolved world position.
5. `turn3` preserves pairwise distances and rotates member orientation around
   the same axis; `become3` installs the exact target blueprint.
6. Missing targets, attachment cycles, zero axes, unsupported paths,
   non-positive durations, and empty bounds fail during build with source spans.
7. Native and editor catalogs, the system prompt, mdBook, examples, and the full
   Rust workspace suite ship together before V2Core is marked complete.

## 3D creator roadmap 1–6 — ✅ shipped

**Status: 🟢 implemented.** The existing spatial language is now production-safe
and extensible without turning Manic into a material/node editor.

1. **✅ Production-safe camera composition.** `view3` automatically uses the
   active Creator/quiz media-safe rectangle, including its asymmetric platform
   insets, while files without Creator metadata retain full-canvas framing.
   Transition audits sample camera motion and projected 3D bounds between
   stages—not only at settled frames.
2. **✅ Stronger relationships.** `attach3` has an optional `rigid` mode
   that carries local offset and orientation; ordinary position-only attachment
   remains the default. `travel3` samples the path's resolved transform at
   playback time so a simultaneously moving/turning authored path stays live.
3. **✅ Spatial production audit.** Diagnostics cover 3D clipping, safe-region
   escape, camera speed/zoom shock, camera penetration, and broken spatial
   relationships. These reuse the same bounds/projection/attachment state as
   the engine rather than estimating a second world.
4. **✅ Rendering refinement through one modifier.** Added
   **`finish3(id,"...")`** for the small set of creator decisions that cannot be
   inferred: `shading=flat|smooth`, `mesh=0..1`, `material=matte|metal|glass`,
   `texture=solid|checker|stripes`, `depth=0..1`, and `shadow=0..1`. Templates
   continue to provide restrained defaults; there is no light graph.
5. **✅ Spatial explanation.** Added only four irreducible concepts:
   **`link3(id,a,b,[trim])`** for a live edge, **`project3(id,source,"xy|xz|yz")`**
   for a live orthogonal projection, **`contour3(id,surface,level)`** for a
   level curve, and **`label3(label,target,[world_height])`** for a projected
   label whose apparent size follows world depth.
6. **✅ Controlled asset/solid extension.**
   **`model3(id,"asset:models/name.obj"|"file.obj",center,[scale])`** imports
   deterministic OBJ geometry (vertices/faces/lines; no scripts). Documented
   `asset:` URIs resolve through the packaged production catalog independently
   of the working directory; ordinary paths remain available for backend-
   provisioned user models. Linux, Docker, EC2, and playground pipelines copy
   the complete `assets/` tree so future catalog entries need no one-off deploy
   rule. Meanwhile,
   **`tube3(id,path,"radius(t)",[sides])`** builds a variable-radius tube around
   an authored 3D path. `finish3` supplies the bounded material/texture treatment
   for both, avoiding separate shader vocabulary.

Native/editor catalog parity, precise invalid-input errors, headless Rust
coverage, the `three-d-v2-lab` creator example, mdBook/system-prompt/publishing
updates, and the full shipped-example regression are part of this release.

## Capabilities (implemented)

### Engine & language
- Stateless timeline (`Timeline::apply(base, t)` is pure) → free scrub/step,
  deterministic recording (mp4/gif/PNG), live preview, CRT post-process.
- Creator publishing audit: `manic check FILE.manic --canvas all` validates four
  common formats and reports settled-stage canvas, safe-area, overlap, and
  readability issues with entity-level repair guidance.
- ASY-like DSL: function-call statements, `(x, y)` points and `(x, y, z)` 3D
  points, `;` terminators,
  `//` comments, `par` / `seq` / `stagger` blocks, named reactive `step` blocks,
  `section`, `wait`/`beat`, `mark`; dotted ids; **tag broadcast** (a
  verb/modifier on a tag hits the whole
  group); line/column error diagnostics.
- **Computation layer** (evaluated at build time): `let` variables; arithmetic
  `+ - * / ^` with **implicit multiplication** (`2sx`, `3(x+1)`), comparisons,
  logic, `pi`/`e`/`tau`, ~20 functions; `for v in a..b` loops; `if/else`;
  recursive `def` macros; reductions `sum`/`prod`/`min`/`max`; id interpolation
  (`bar{i}`). All collapse to literals before rendering — kits are unaffected.
- **Look / config**: `canvas` accepts pixels or presets (`"16:9"`/`"square"`/
  `"portrait"`/`"4:5"`/`"1080p"`/`"4k"`); `w`/`h`/`cx`/`cy` predefined, and
  `--canvas` can reframe one responsive source before expansion. Selectable
  **templates** — `mono` (default black-and-white editorial), `plain`,
  `terminal`, `paper`, `blueprint`, `shorts` — each retints the palette and sets
  chrome/glow/CRT; author-set `masthead` (no engine branding baked in). Same
  content renders in any template.
- Animation: named verbs + a general `to(id, property, value)` (x, y, opacity,
  scale, angle, trace, color, **hue** — cycles around the colour wheel, and
  **value** — a live `counter`'s number); `rotate`/`spin`; camera `cam`/`zoom`;
  friendly easings; per-act duration.
- Motion Graphics V2 relationship surface: `attach(child,target,[offset])`
  with `attach(child,none)` release, exact-blueprint
  `become(source,target,[duration],[ease])` with safe crossfade fallback, and
  rigid shared-pivot `turn(id_or_tag,pivot,degrees,[duration],[ease])`. A
  build-time authored blueprint lets these compose after ordinary clips while
  `Timeline::apply(base,t)` stays pure.
- Updaters (pure functions of `t`): `follow` (ride a target), `link`
  (edge tracks two entities), and the general `derive` hook (dynamic
  constructions — drag a vertex and dependents recompute). Creator parameters
  add pure `bind` connections from one animated scalar to multiple properties,
  counters, or a resampled plot formula.

### Kits
- **std** — `dot`, `circle`, `rect`, `line`, `arrow`, `brace` / `bracelabel`
  (curly brace, optional label), `text`, `counter` (live numeric readout),
  `parameter` (bounded visible control) + `bind` (responsive range or formula
  mapping into several existing visuals),
  `morph` (set a shape up to morph into another), `copy` (duplicate an entity),
  `caption` (word-by-word text row + `karaoke`/`wordpop` verbs);
  modifiers (`hidden`, `untraced`, `cursor` (typewriter `_` on text), `color`,
  `hue` (HSL, computable per-entity), `outline`/`outlined`/`filled`, `size`,
  `stroke`, `dashed` (generic dash/gap pattern for path-like entities), `glow`,
  `z`, `rot`, `opacity`, `bold`, `display`, `label` [offset],
  `tag`); verbs (`show`, `fade`, `move`, `shift`, `grow`, `draw`, `erase`,
  `type`, `say`, `recolor`, `flash`, `pulse`, `shake`, `scale`, `rotate`,
  `spin`, `attach`, `become`, `turn`, `swap`, `transform` (2×2 matrix /
  ApplyMatrix), `to`/`set`, `cam`,
  `zoom`); boolean ops `union`/`intersect`/`difference`/`exclusion`.
- **math** — `axes` (optional ticks + labels), `plane`/`numberplane`,
  `complexplane`, `polarplane`, `plot` (named functions **or a formula string**
  like `"cos(x)+0.5*sin(3*x)"`; symmetric or one-sided `(x0,x1)` range),
  `numberline`, `vector`, `arc`, `sector`, `annulus`, `pie`, `arrowfield` (8
  named vector fields, magnitude-coloured), `matrix` (bracketed, row/column
  addressable via tags), `table` (ruled grid + optional row/col labels; cells,
  rows, columns, labels and grid lines all addressable via tags).
- **algo** — `graph` (undirected `a-b` / directed `a>b`, circular/row/grid
  layouts, reflowing edges, tag groups); `array` (row of fixed slot boxes
  `{id}.box{k}` + value cells `{id}.c{k}`) with `compare(a,i,j)` (flash the
  values now in two slots) and stateful `swap(a,i,j)` (slide them into the
  swapped slots, chaining correctly across a whole sort — see
  `examples/bubble_sort.manic`); `pointer(id, arr, slot, [label])` + `pointat(id,
  arr, slot)` — a labelled index caret that slides between slots (two-pointer /
  traversal, `examples/two_pointer.manic`); `stack`/`queue` with `push`/`pop`
  and `enqueue`/`dequeue` — dynamic structures that add cells and animate them
  in/out, tracking occupancy so chains of ops compose (`dequeue` also advances
  the cells behind); `caret(id, (x,y), "label", dir)` — a rigid labelled marker
  you `move` to track an action point (stack top, queue front/back). See
  `examples/stack_queue.manic`. `list(id, "3 8 5", (cx,cy), kind, [cw], [ch])` —
  a **linked list** with the classic node anatomy: framed boxes split into
  compartments (`[data│•next]` singly, `[•prev│data│next•]` doubly) with pointer
  dots, a `head` pointer and a `NULL` terminator (or a wrap-to-head curve).
  `kind` ∈ `singly`/`doubly`/`circular`. `insert(id, after, "v")` splices a node
  in below the gap and re-threads the pointers (no row shift); `remove(id, i)`
  unlinks and re-points around it. See `examples/linked_list.manic`. `bfs(g,
  start)` / `dfs(g, start)` — graph traversal: reads the graph's adjacency,
  runs the algorithm, and animates the classic states (discovered → current →
  done) with tree edges lighting up and live `queue:`/`stack:` + `visited:`
  readouts (BFS = queue, DFS = stack; directed edges followed one way). See
  `examples/bfs_dfs.manic`. **Weighted** edges: `a-b:7` gives an edge a weight
  (drawn as a midpoint label). `dijkstra(g, start)` — single-source shortest
  paths: each node shows a live distance (`inf` → final), the nearest unsettled
  node settles (magenta → lime), relaxed edges light, and the shortest-path-tree
  edges stay lit. See `examples/dijkstra.manic`. `hashmap(id, n, (cx,cy))` — `n`
  buckets in a column; `put(id, k, v)` hashes the key (byte-sum mod n) to a
  bucket and chains the `k:v` entry on (collisions extend the chain);
  `get(id, k)` hashes then scans that bucket's chain, flashing each entry until
  the key matches (lime) or the chain ends (miss). Separate chaining, composed
  from the array (buckets) + list (chains). See `examples/hashmap.manic`.
- **geo** — all **dynamic** (recompute as inputs move): `point`, `segment`;
  centres `midpoint`/`centroid`/`circumcenter`/`incenter`/`orthocenter`/`foot`;
  intersections `meet` (line∩line), `linecircle`, `circlecircle` (two points
  each); `tangent` (touch points from an external point); `reflect`, `bisector`,
  `rotpoint`, `between`, `anglepoint`; circles `circumcircle`/`incircle`/
  `circle2`; conics `ellipse`/`parabola`/`hyperbola`; `fullline` (infinite);
  `anglemark`, `rightangle`.
- **brand** — `banner` (icon trio + "manic" wordmark, create→expand→unwrite)
  and `watermark` (screen-fixed persistent mark with a responsive bottom-right
  default and exact-position override).
- **three** — hybrid depth-tested 3D under the normal 2D overlay: `camera3`
  (perspective/orthographic Z-up orbit camera), `point3`, `line3`, `arrow3`,
  `cube3`, `sphere3`, `grid3`, `axes3` (ticks + numbers), plus `pin3` (glue a 2D
  label to a 3D point), `follow3` (track another entity), `midpoint3` (derived
  point), `curve3` (parametric 3D curve), `surface3` (z=f(x,y) filled mesh), `param3` (parametric surface — tori/Möbius), `prism3`/`pyramid3`/`revolve3`
  (filled, flat-shaded solids), `extrude3` (extrude a 2D shape/boolean-region → CSG solids),
  `thick` (tube strokes); creator verbs `view3`, `travel3`, `attach3`, `become3`,
  `turn3`; precise verbs `move3`, `shift3`, `rotate3`, `grow3`, `orbit3`,
  `roll3`, `look3`. Shared modifiers/verbs (`color`, `opacity`,
  `hidden`, `untraced`, `tag`, `show`, `fade`, `draw`, `recolor`, `flash`,
  `pulse`, `scale`) also address 3D entities. See **3D foundation** below.

### Primitives (engine)
`Circle`, `Rect`, `Line`, `Arrow`, `Curve`, `Coil` (spring zigzag pos→to,
stretches via the `To` prop), `Polygon`, `Polyline`, `Arc`
(arc/sector/annulus), `Region` (boolean result), `Text`; 3D `Point`, `Line`,
`Arrow`, `Cube`, `Sphere`, and XY `Grid`.

### 3D foundation
- **Coordinates & scene model** — computed `(x,y,z)` values flow through the
  parser, macro expander, lowering, editor services, and runtime. 3D entities
  have stable ids and tags alongside the existing 2D scene.
- **Camera** — one Z-up orbit camera with perspective or orthographic
  projection. `camera3` sets its eye, target, and field of view (a single value,
  reused as the orthographic height), plus the projection; `orbit3` animates
  azimuth, elevation, and radius, `roll3` animates orientation around the view
  direction, and `look3` animates the target. An analytical pole-safe orbit
  frame keeps exact overhead/underside views stable and continuous through a
  pole crossing—there is no fallback-axis cutoff that can snap mid-turn.
- **Rendering & output** — depth-tested 3D renders beneath the normal 2D
  overlay. Preview, stills, CRT output, and recordings all use the same
  depth-enabled render target. Render-target Y correction keeps screen
  orientation consistent, with positive Z visibly pointing up.
- **Geometry** — points, lines, arrows, cubes, spheres, XY floor grids, and
  ticked, numbered XYZ axes (`axes3`, optional `step`). Objects support position,
  non-uniform scale, Euler rotation, color, opacity, visibility, and tracing state.
- **Animation** — deterministic `Vec3` timeline tracks drive `move3`, `shift3`,
  `rotate3`, and `grow3` (which retargets a `line3`/`arrow3` endpoint rather than
  scaling). Shared `show`, `fade`, `draw`, `recolor`, `flash`, `pulse`, and
  `scale` verbs also address 3D entities and tag groups.
- **Projected labels** — `pin3(label, point3 | entity3)` binds an existing 2D
  `text`/`label` to a 3D position; a world→screen projection reprojects it every
  frame, so the label stays glued as the camera orbits (or the target entity
  moves). The same hook powers the shipped ticked/numbered `axes3` labels.
- **Reference** — `examples/three_d.manic` exercises the camera, depth,
  primitives, axes, transforms, a pinned label, and hybrid 2D/3D composition.

## Coverage audit and remaining gaps

### Compact generic motion — shipped ✅

The generic-motion slice closes a creator-facing gap exposed by the Zeroth-Law
reference animation: authors no longer have to hand-place dozens of dots or fake
motion along curved connections. The vocabulary stays domain-neutral and small.

This slice deliberately keeps one small domain-neutral surface:

- **`particles(id, container, count, [radius], [seed], ["layout"])`** — create a seeded,
  deterministic group of small dots inside a circle or rectangle. The author's id
  supplies the meaning: `bubbles`, `dust`, `stars`, `data`, or `molecules` all
  use the same constructor. Children are `{id}.p0…`, tagged by bare `{id}`.
  `layout` is `random` by default, an ordered `grid` inside a rectangle, or an
  ordered `ring` inside a circle.
- **`wander(id, [duration])`** — give a particle group gentle ambient movement
  for the clip duration while keeping every child inside its source container.
  Evaluation remains pure by absolute time, so preview scrubbing and recording
  produce the same frame.
- **`arrange(id, container, ["random|grid|ring"], [duration], [ease])`** — move the
  same persistent children into another deterministic layout/container. This
  covers free expansion and exact `grid → random → grid` reversal without
  per-particle scripting or domain-specific entropy vocabulary. Random states
  use independent stable curved routes instead of a synchronized straight
  tween; `ring` adds a radial endpoint for clocks, orbits, state diagrams, and
  final-law frames.
- **`travel(entity, path, [duration], [ease])`** — move one real entity once
  along a line, arrow, curve, plot, spline, or arc, then hold it at the endpoint.
  This is the persistent-object complement to the transient `flow` pulse.
- **`link(id, a, b, [bend])`** — expose the engine's tracked-edge mechanism as
  public std vocabulary. `bend=0` is a straight link; non-zero bend produces a
  curve whose endpoints continue to follow moving entities.
- **`flow(path, [duration], [direction], [mode])`** — send a luminous emphasis
  pulse or finite stream over a line, curve, spline, or tracked link. Direction
  is `forward|reverse|both`; mode is `once|continuous`. Existing calls remain a
  single forward pulse. It expresses energy, a signal, traffic, data, or
  attention without inventing a domain-specific object.

Example target:

```manic
circle(glass, (cx,cy), 120);
particles(bubbles, glass, 30, 6, 7);
wander(bubbles, 6);

circle(tank, (cx+360,cy), 120);
link(pipe, glass, tank, 30);
untraced(pipe); draw(pipe); flow(pipe, 1.2);

dot(marker, (cx,cy), 6);
travel(marker, pipe, 1.2, smooth);
```

Non-goals for this slice: no `molecule`, `reservoir`, `heatflow`, or
`zerothlaw` builtin; no new word for shrinking three objects onto an axis
(`par` + `move` + `scale` already reads clearly); and no broad particle-effects
system with emitters, forces, collisions, or dozens of knobs. Further
3Blue1Brown-derived work must clear the same gate: recur across several lessons,
replace substantial manual scripting, and remain teachable in one sentence.

The shipped implementation keeps the containment promise exact: circles and
rectangles are convex, so both the sampled positions and every tween between
them stay inside. Concave-region path planning, collisions, emitters, forces,
and physics remain intentionally outside this small primitive. Dedicated tests
cover seeded repeatability, containment at sampled times, pure out-of-order
timeline evaluation, moving bent links, path travel/endpoint holding, open-path
morph topology, flow phase, and invalid targets.

Reference: `examples/particles-flow.manic` isolates ambient particles and path
flow; `examples/zeroth-law-thermodynamics.manic` uses them for thermal
equalisation; and `examples/second-law-thermodynamics.manic` uses persistent
rearrangement for mixing, free expansion, graph markers, graph-to-connector
morphs, a radial final state, and an exact statistical reversal.

### 3Blue1Brown benchmark audit — prioritized, vocabulary-gated

The audit compared repeated visual techniques, not subject-matter terms. Manic
already has strong coverage for formula plots, exact geometry, 2×2 transforms,
outline morphing, simulations, and a substantial 3-D layer. The remaining gaps
below are roadmap candidates, not promised builtins:

1. **Matching transforms for equation/text parts (highest leverage).** Official
   Manim examples use `TransformMatchingStrings` and `TransformMatchingShapes`
   so symbols can visibly retain identity through an algebraic rewrite. Manic's
   Ordinary LaTeX remains a fast single raster entity. **Creator-reactive v1
   SHIPPED ✅ (2026-07):** one opt-in, domain-neutral verb,
   `` rewrite(equation, `next latex`, [duration], [ease]) ``. The author supplies
   each mathematically correct state—Manic is not a CAS—and the engine matches
   RaTeX display items so unchanged glyphs retain identity, moved terms travel
   smoothly, new items enter locally, and removed items leave locally.
   **Continuity-safe matching shipped (2026-07):** common parts are now selected
   by an order-preserving sequence match with movement, row, scale, and neighbour
   context as tie-breakers. Identity also includes coarse mathematical layout
   role and RaTeX's exact math-style scale. The exponent in `x^2`, the coefficient
   in `2x`, a denominator `2`, and a deeper `2` in an exponent tower therefore do
   not become one another merely because they share a glyph. Repeated zeros,
   brackets, and variables retain reading order rather than being greedily paired
   with the nearest copy; genuinely unique compatible terms may still cross a
   relation when the authored algebra moves them.

   A side that gains or loses fraction, radical, or grouping topology uses a
   staged local replacement: the old side leaves, then its authored replacement
   enters, while the compatible side and equality remain continuous. Compatible
   additions still enter immediately. When unmatched source and target glyphs
   form a replacement (`2 → 3`, `u → x`), the old glyph leaves before the new
   glyph becomes readable instead of briefly displaying both as `23` or `ux`.
   A globally incompatible change uses two whole-equation layers with only a
   short, dim overlap rather than an equal-strength ghosted midpoint. The exact
   target RaTeX image is installed at the endpoint in every mode.

   Every transition receives a visual confidence score based on matched source
   and target area, travel distance, ordering inversions, mathematical structure,
   and matrix topology. This keeps the one-word DSL while preventing malformed
   matrix-to-formula morphs, misleading cross-role glyph jumps, and the old
   zero-opacity whole-equation frame.

   The shipped feature is deliberately regression-contained: existing `equation`,
   `show`, `fade`, `move`, `scale`, and LaTeX rendering do not change unless
   `rewrite` is used. Rewrites expand at build time into ordinary stateless
   position/scale/opacity tracks, preserving deterministic recording, seeking,
   and scrubbing. A chain remembers its authored LaTeX state while keeping one
   stable equation id. The first release holds a common scale and anchor across
   each transition, respects the existing canvas/safe-region layout, keeps
   semantic `\\textcolor` styling, and supports composition with plots,
   diagrams, captions, and `par` without adding `integral`, `quadratic`, `react`,
   `watch`, or CAS-specific vocabulary. A table-driven creator corpus now covers
   algebraic rearrangement; integrals/derivatives; fractions, radicals, powers,
   and limits; nested exponential towers; logarithms with compound bases;
   contour integrals; differential limits; ODE/PDE notation; trigonometric
   identities; sets/logic; sums/products; physics and units; probability;
   matrices/vectors; mixed prose/math; and creator-defined notation composed
   from renderer-supported LaTeX. It also retains the
   repeated-symbol, portrait-fit, exact-settled-image, out-of-order seeking, and
   malformed-LaTeX regressions. Frame-level regressions now cover immediate RHS
   entry, no-blank fallback opacity at 60 samples/second, stable quadratic RHS
   retention, ordered repeated matrix entries, nested-script scale identity,
   derivative-order separation, and upper/lower integral-limit separation.
   Reference scenes:
   `examples/quadratic-formula-continuity.manic` (completing-square acceptance
   benchmark) and `examples/reactive-integral.manic` (the same equation rewrite
   composed with plots, numerical differentiation, a moving tangent, a generic
   dashed antiderivative, and `+C`), plus
   `examples/reactive-math-notation.manic` (a 9:16 creator showcase spanning the
   full notation corpus plus chemistry and biology on one persistent stage).

   The common positional subset is already covered by generic
   `cycle(a,b,c,…,[duration],[arc],[ease])`: independently declared symbols move
   cyclically into one another's positions along an optional arc, matching
   Manim's `CyclicReplace` without adding algebra-specific vocabulary. See the
   [official example scenes](https://github.com/3b1b/manim/blob/master/example_scenes.py).
2. **General path remapping / nonlinear deformation.** The same examples expose
   arbitrary `apply_function` and `apply_complex_function`, while the
   [Fourier lesson](https://www.3blue1brown.com/lessons/fourier-transforms/)
   repeatedly winds an ordinary graph around a circle. Manic can apply a linear
   2×2 matrix and plot a formula, but cannot yet bend an existing grid, curve,
   or group through a reusable nonlinear map.
3. **Move an arbitrary entity along an existing path.** `flow` deliberately
   moves only an emphasis pulse. A dot, label, camera, or copied shape following
   a curve still needs sampled manual `move`s. This recurs in orbit, signal,
   tracing, and winding scenes, so one path-binding extension may eventually be
   justified after representative examples define its simplest semantics.
4. **Dense, data-driven connection fields.** The
   [neural-network lesson](https://www.3blue1brown.com/lessons/neural-networks/)
   uses large layered graphs whose edge brightness/color encode weights and
   whose activations propagate. This is now covered by the planned **Manic ML
   kit** above: reusable computed connection fields and progressive focus, not
   dozens of core words.
5. **Longer-horizon rendering capabilities.** Recursive path refinement in the
   [Hilbert-curve lesson](https://www.3blue1brown.com/lessons/hilbert-curve/),
   procedural fields in the
   [Newton-fractal lesson](https://www.3blue1brown.com/lessons/newtons-fractal/),
   and 4-D projection in the
   [quaternion lesson](https://www.3blue1brown.com/lessons/quaternions/)
   expose real gaps, but each has a higher implementation/teaching cost and
   lower creator frequency than items 1–3.

Roadmap rule: build a representative scene first; add vocabulary only when the
same operation recurs, removes substantial manual scripting, composes outside
its originating subject, and can still be explained in one sentence.

### Published benchmark 2 — inverse derivatives through a turning plane

`engine-test-2.mp4` derives `(ln x)' = 1/x` by treating an inverse function as
the same curve seen after the coordinate plane turns over: start with `y=e^x`,
build its `rise/run = y/1` tangent triangle, exchange the screen roles of `x`
and `y`, then read the reflected relationship as `y=ln(x)` with slope `1/x`.

Most of the scene is already ordinary vocabulary: `curve3` expresses both
parametric forms, `line3` builds the exact slope triangle, `pin3` attaches
labels, `morph3` carries the curve and triangle continuously between inverse
parameterisations, and `orbit3` supplies the spatial plane turn. The one real
camera gap was orientation:

- **`roll3(degrees, [duration], [ease])`** — rotate the 3-D camera's up vector
  around its viewing direction. This is general cinematography vocabulary, not
  inverse-function vocabulary. Combined with `orbit3`, it lets a plane pass
  continuously from an overhead view to its underside while deliberately
  exchanging which world axis is horizontal or vertical on screen.
- The camera frame must remain well-defined directly above/below a Z-up plane.
  The renderer derives its right/up basis analytically from azimuth/elevation,
  giving a continuous finite frame through the pole instead of switching to a
  fallback axis at a threshold (the old switch caused a visible mid-turn snap).

Reference-frame review exposed two further generic presentation gaps:

- **`cycle(a,b,c,…,[duration],[arc],[ease])`** moves each entity to the next
  entity's position and the last back to the first, following a circular arc
  (90 degrees by default). This is the small Manic equivalent of Manim's
  `CyclicReplace`; the `xy` plane label can therefore become `yx` by moving the
  actual `x` and `y` glyphs rather than crossfading two labels.
- `equation` now preserves standard LaTeX term colors such as
  `\textcolor{magenta}{\mathrm{slope}}` and
  `\textcolor{cyan}{x}`. Manic semantic color names are remapped through the
  selected template before rasterisation, while uncolored terms retain the
  template foreground. This keeps emphasis meaningful in `plain`, `paper`,
  `shorts`, and the default `mono` look.

No new `inverse`, `logproof`, `slopefraction`, or `swapaxes` builtin is planned.
Those ideas remain composition: geometry + camera + LaTeX. The acceptance test
is `examples/derivative-of-ln-x.manic`: no blank cuts during either plane turn, the
`x`/`y` glyphs visibly cycle, screen roles exchange continuously, semantic
equation terms keep their colors, unchanged algebra pieces retain identity
while only the rewritten terms animate, the explicit Manic watermark persists,
and the final curve/formula agree.

### Geometry (olympiad) — 🟡 Foundation
Done (all **dynamic** unless noted): `meet` (line∩line), **`linecircle`**
(line∩circle), **`circlecircle`** (circle∩circle) — the last two output two
points `{id}0/1`; **`tangent`** (two touch-points from an external point); **`commontangent`**
(a common tangent to TWO circles — external/direct or internal/transverse — as the
segment between the touch points, so its length is the tangent length `√(d²−(r₁∓r₂)²)`;
static);
**`reflect`** (point across a line); **`bisector`** (point on the internal angle
bisector); **`circle2`** (circle by centre + a point on it); **`rotpoint`**
(point rotated about another by θ — gives equilateral apexes, regular figures);
**`between`** (point at fraction `t` along a segment — relpoint); **`anglepoint`**
(point on a circle at an angle); **`fullline`** (line extended across the frame);
**`ellipse`** (rotatable outline, static). Circles are given as centre + a point
on them so the radius stays dynamic. Examples `examples/tangents.manic`.
**Conics complete:** `ellipse`, `parabola` (vertex + width/height), `hyperbola`
(two branches `{id}.r`/`{id}.l`) — see `examples/conics.manic`.
Still missing (minor):
- **Point-on-curve by arc-length** (`between` covers relative position on a
  segment; arc-length along an arbitrary path is not done).
- Foci/directrix as *constructed* elements of a conic (the conics are drawn
  outlines, not point-defined loci).
- **Skew coordinate systems** (`cartesiansystem`, 113) — niche.
- **Numeric labels** — `markangle` with a degree value, `distance` (16). The
  `counter` readout + `value` track cover *animated* / computed numbers; what's
  still missing is binding one to a *live geometry measurement* (a length that
  updates as a vertex is dragged) — would wire the `derive` hook into a counter.
Whole tagged constructions already rotate through `transform(group, origin,
a,b,c,d,...)` with a rotation matrix; no geometry-specific verb is needed.

### Graphing (math) — 🟡 Foundation
- Expression plots DONE — `plot` takes a formula string in `x`/`t`
  (`"cos(x) + 0.5*cos(7*x)"`, arithmetic + ~20 functions), manic's
  `FunctionGraph`. `arrowfield` still deliberately takes a small set of named
  vector fields; an authored two-component field expression remains future.
- `plot` range may be a scalar `domain` (symmetric) or an explicit `(x0, x1)`
  pair (one-sided) — `plot(g,(cx,cy),200,52,"x*x",(0,2.5))`.
- Coordinate frames done: `axes` (ticks + integer labels), `plane`/
  `numberplane`, `complexplane`, `polarplane`, plus foundational `axes3` and
  `grid3`. `axes3` already ships projected tick labels. Still missing: custom
  2-D tick-label values/non-integer steps, per-axis limits, and multiple styled
  axes in one constructor.
- **Area under a curve** ships both as the generic filled `area` graph view and
  as authored Riemann rectangles; `for` loops generate the converging bars in
  `examples/area_under_curve.manic`. Generic legends and an author-facing
  scatter/data-series constructor remain open (the stats kit has specialized
  scatter views such as `correlation`).
- Vector fields: `arrowfield` done; **`StreamLines`** (flowing-agent traces)
  not done — needs a flow simulation + the animation flow (a good fit for a
  future updater-driven feature).

### Transforms / morphing — ✅ Shipped core
Two kinds: **property** transforms (position, endpoint, colour, scale, rotation,
opacity, trace, hue, value) — all covered; and **geometry** transforms — a
linear map of space (`transform`), outline shape-morph (`morph`, with winding),
and entity `copy` — now covered too. Essentially the whole family; only
`TransformAnimations` is N/A by design (see below).

- **Have (full):**
  - `ApplyMethod` → our verbs `move`/`shift`/`scale`/`rotate`/`spin`/`recolor`/
    `to`/`set`.
  - `ScaleInPlace` → `scale(id, f)`; `ShrinkToCenter` → `scale(id, 0)`.
  - `FadeToColor` → `recolor`.
  - `MoveToTarget` → `to`/`move` straight to the target.
  - **`ApplyMatrix`** → **`transform(group, (ox,oy), a,b,c,d, [dur], [ease])`** —
    applies a 2×2 matrix about an origin to every entity in a tagged group
    (anchor + line/arrow endpoints), so a grid + basis vectors + points shear /
    rotate together (the 3b1b linear-map-of-space visual). See
    `examples/linear_transform.manic`. Correct for dots/lines/vectors/axes;
    curves/circles move by anchor only (approximate).
  - **`Transform` / `ReplacementTransform`** → **`morph(a, b, [spin])`** sets `a`
    up to morph into `b`'s outline (both sampled to the same points);
    `to(a, morph, t)` blends. See `examples/morph.manic`. Caveats: outline-only
    (stroke, not filled area); one target per setup; sampled at build time; naive
    index correspondence (slight rotational offset).
  - **`ClockwiseTransform` / `CounterclockwiseTransform`** → the optional `spin`
    on `morph(a, b, spin)` winds the blend (positive = clockwise, negative = CCW).
  - **`TransformFromCopy`** → **`copy(new, src)`** duplicates an entity (standalone,
    no group tags); `copy(c, a)` then morph/move `c` while `a` stays put.
  - **`Swap`** → **`swap(a, b, [dur], [ease])`** exchanges two entities' positions;
    the array form `swap(arr, i, j)` slides slot values and chains across a sort.
  - **`CyclicReplace`** → **`cycle(a, b, c, …, [dur], [arc], [ease])`** moves
    every entity into the next position and the last into the first along a
    circular path (`arc` degrees, default 90). Repeated calls compose.
- **Partial (expressible, no dedicated builtin):**
  - `FadeTransform` / `FadeTransformPieces` → crossfade `par { fade(a); show(b); }`
    — not point-matched.
  - Generic entity `Restore` → `checkpoint`/`restore` now ships for exact ML
    network rollback, while `pulse`/`flash` still auto-restore visual state.
    There is not yet a generic entity `save`/`restore` snapshot across every
    shape and property.
  - `ApplyPointwiseFunction[ToCenter]`, `ApplyComplexFunction` → expressible over
    a **set of dots** via the loop+expression layer (compute `f(z)` per point and
    `to` it); `transform` covers only the *linear* (2×2) case, not a general
    per-point formula.
- **N/A by design:**
  - `TransformAnimations` — Manim interpolates between two *animation objects*.
    manic's timeline is stateless property tracks with no first-class animation
    object to blend, so the literal form doesn't fit. The practical use —
    smoothly hand off / cross-blend two animations — is covered by `par`/`seq`
    composition plus `morph` / crossfade (`par { fade(a); show(b); }`).
- **Known `morph` limits:** naive index correspondence (mismatched topologies /
  holes can twist), and it can't morph *filled* regions or text glyphs.

### Creation / reveal — 🟡 Foundation
Built on manic's `trace` property (draw-on for strokes = fraction of path/
outline traced with fills fading in; for text = typewriter char count).

- **Have (full):**
  - `Create` → `draw(id)` (declare `untraced` first).
  - `Uncreate` → `erase(id)` (trace back to 0).
  - `ShowPartial` → the `trace` prop *is* this mechanism (animate `to(id,
    trace, u)` to any fraction).
  - `AddTextLetterByLetter` → `type(id)` (typewriter).
  - `RemoveTextLetterByLetter` → reverse typewriter (`erase` / `to(id, trace,
    0)` on text).
  - **`TypeWithCursor` / `UntypeWithCursor`** → the **`cursor(id)`** modifier adds
    a `_` typewriter cursor that rides the revealed text (terminal-prompt look).
  - **`AddTextWordByWord`** → **`caption(id, "words", (x,y))`** lays out the
    words, then **`wordpop(id)`** pops them in one at a time (TikTok style) or
    **`karaoke(id, [delay], [color])`** highlights them in sequence (lyrics
    style). See `examples/captions.manic`.
  - `ShowIncreasingSubsets` → `stagger { for i in 0..n { show(x{i}); } }` over a
    tagged group (cumulative reveal).
  - `ShowSubmobjectsOneByOne` → a `seq` of show/hide (flipbook, one at a time).
- **Partial / not one call:**
  - `DrawBorderThenFill` → `draw` traces the border and fades the fill *together*;
    sequencing border-fully-then-fill is scriptable (`seq`) but not one builtin
    (fill opacity isn't a track separate from `trace`).
- **Blocked / needs other machinery:**
  - `Write` / `Unwrite` → we do path-trace + typewriter, **not** calligraphic
    stroke-by-stroke handwriting of glyph outlines — needs glyph-outline stroking
    (tied to the font/LaTeX work).
  - `SpiralIn` → a path-based entrance. Needs **path-motion** (a `Pos` track that
    follows a curve) + the entrance/initial-state machinery (the Growing
    `growin`/`popin` cheap win). Fakeable today by loop-placing offsets + `move`.

### Growing — 🟡 Composable foundation
manic can animate `scale`, `spin`, and the line/arrow endpoint (`grow`), but has
no modifier to set an *initial* scale and no bounding box — so "appear by growing
out of nothing" and edge/point origins are scriptable rather than one call.

- **Have (full):**
  - `GrowArrow` → `grow(id, (x,y))` extends a line/arrow/curve endpoint to a
    point (declare it zero-length, then `grow` to full).
- **Partial (expressible, no dedicated builtin):**
  - `GrowFromCenter` → `scale` animates uniform scale, but there is no
    initial-scale modifier, so growing from nothing needs a
    `seq { scale(id,0,0); scale(id,1,d); }` trick.
  - `GrowFromPoint` → scale + a `move`/`shift` originating at the point.
  - `SpinInFromNothing` → `par { scale(id,1,d); spin(id,360,d); }` (compose the
    grow trick with `spin`).
- **None / needs prerequisites:**
  - `GrowFromEdge` → needs a bounding box to find the edge (same missing
    entity-bbox that blocks `Brace(mobject)` and `GrowFromPoint` automation);
    doable today only by supplying the edge point yourself.
- **Cheap win:** an initial `scale` modifier + a `growin`/`popin` verb (scale
  0→1 about the anchor) would move `GrowFromCenter` / `SpinInFromNothing` to
  full support in a few lines.

### Deeper math — how it can elevate the engine (mostly future)
The current evaluator is enough to calculate values and sample plots. Real math
elevates manic when it makes a diagram *depend on a mathematical truth*:
an intersection remains correct as inputs move, a tangent comes from the plotted
function, an eigenvector is computed rather than authored, or an optimisation
visibly converges. The goal is a small, dependable mathematical core, not a
general-purpose CAS embedded in the DSL.

**First rung shipped — a curve-analysis family.** `plot` now *remembers* its
function + screen mapping on the entity (`Entity::graph`), and a shared
`Entity::graph_view` (enum `GraphView`) drives four constructions that all
*query the curve the author already drew* and animate one moving parameter `x`
(`to(id, x, target, dur)` → `Prop::PlotX`):
- **`tangent(id, curve, x, [len])`** — tangent line + contact dot; slope from the
  function (numerical central difference), correct as it slides, honest at
  corners/asymptotes (dot only, no fake line).
- **`normal(id, curve, x, [len])`** — the perpendicular line + dot.
- **`slope(id, curve, x, [(dx,dy)])`** — a live slope *number* riding the point.
- **`area(id, curve, a, b, [n])`** — the filled region under the curve,
  sweepable open via `to(id, x, b, dur)`.
- **`integral(id, curve, a, b, [(px,py)])`** — a live number (composite Simpson)
  that climbs to the true integral as it sweeps, in step with `area`.
- **`roots(id, curve, [color])`** — a dot at every zero-crossing (sign-scan +
  bisection).
- **`newton(id, curve, x0, [steps])`** — the Newton's-method zig-zag from a guess,
  drawn on with `draw` to animate the walk to a root.

Beyond the curve-analysis family (these take points/formulas, not a `plot` id):
- **`spline(id, p0, p1, …)`** — a smooth Catmull-Rom curve through given points
  (interpolation), with knot dots.
- **`trajectory(id, "dx/dt", "dy/dt", (x0,y0), (cx,cy), scale, [steps])`** — an
  RK4-integrated ODE path (orbits, spirals, phase portraits).

See `examples/tangent.manic` and `examples/analysis.manic`; unit tests in
`kits::math::graph_tests` check the numbers against calculus (slope, ∫x²=8/3,
∫sin=2, normal ⟂ tangent). This is the pattern the rest should follow: query the
drawn function, return both a value and a drawable. Natural next step: expose the
integral/slope as a bindable value (`let a = area_of(f,0,2)`) once the arg
evaluator can reach the scene.

- **Robust numerical geometry** — tolerance-aware orientation, intersection,
  containment, root-finding, and curve-parameter routines would make dynamic
  constructions stable near parallel lines, tangencies, and degeneracies.
  This improves every geometry kit before adding any new notation.
- **Linear algebra** — ✅ *DONE — 2D Tiers 1–3 complete, plus the core 3D forms.*
  The unifying idea: a matrix *does something to space*, and the computed
  quantities (determinant, eigenvalues, solutions) are exposed visually — the
  2D/3D analog of what `GraphFn`/`SurfaceFn` did for calculus.
  - *Substrate (shipped):* a small **closed-form** numeric core — `det2`/`eig2`/
    `solve2` (2×2), `det3`/`eig3` (3×3, with a real-cubic root solver), `fit_line`
    (least-squares), `rref_steps` (Gauss-Jordan). **No `nalgebra`** — the 2×2/3×3
    cases are handled directly. The `MatrixFn` "matrix-remembers-its-numbers by
    id" idea was **closed as unneeded**: every builtin takes the matrix inline,
    and `let a = …` variables already give the define-once / reference-many
    ergonomic without coupling to the visual `matrix` entity. (A matrix-by-id
    binding could still be added later if a workflow wants it — the `surf`-on-
    entity pattern shows how — but nothing in Tiers 1–3 needed it.)
  - ✅ *Tier 1 — what a matrix IS (flagship trio, shipped):* **`linmap`** (the
    deformed grid + basis î,ĵ landing on the matrix's columns, over a faint
    identity grid); **`determinant`** (the unit square → parallelogram, area =
    det, flips colour when det<0, collapses to a line at det=0); **`eigen`** (the
    real eigenvector directions + eigenvalues; a note for complex/rotation).
    All math y-up via `det2`/`eig2` (closed-form 2×2 — no `nalgebra` yet). See
    `examples/linear-map.manic`.
  - ✅ *Tier 2 — systems, spans, rank (shipped):* **`linsolve`** (`Ax=b` as the
    row picture — the two rows as lines meeting at the solution, a gold dot + its
    coords; parallel rows = "no unique solution"); **`span`** (the line/plane a
    set of vectors reaches — two independent vectors → the whole plane, one or
    two parallel vectors → a line, i.e. the rank/collapse picture that ties to
    `determinant`). 2D via `solve2` (Cramer) + the cross-product test. See
    `examples/linear-system.manic`.
  - *Tier 3 — decompositions & operations:* ✅ **`diagonalise`** (shipped —
    `A = P D P⁻¹` made visual: the eigen-grid + unit eigen-cell and its image, a
    pure stretch by λ along each eigenvector, no shear; `eig2`-based, math y-up,
    complex/repeated → note; alias `diagonalize`; see `examples/diagonalise.manic`).
    ✅ **`rref`** (shipped — animated Gaussian elimination: one matrix per
    elimination state drawn in place, cross-faded `s{k-1}`→`s{k}` with the row-op
    captioned; the last state is the RREF, and for `[A|b]` its final column is the
    solution; `rref_steps` Gauss-Jordan core; see `examples/rref.manic`).
    ✅ **projection & least-squares** (shipped — `project` drops a vector onto a
    subspace line: the shadow `p = (b·a/a·a)a` and the residual `b−p` at a right
    angle; `leastsquares` fits `y = m x + c` to a point cloud with its vertical
    residuals — the same orthogonal-projection principle. See
    `examples/projection.manic`).
    **Tier 3 complete.**
  - *3D forms:* ✅ **`linmap3`** (shipped — a 3×3 matrix deforming the unit cube
    into a parallelepiped: basis arrows i/j/k on the columns, and the enclosed
    **volume = the determinant**, `det3`-based, colour flips on det < 0, collapses
    at det = 0; see `examples/linear-map3.manic`). ✅ **`eigen3`** (shipped — the
    real eigenvector directions of a 3×3 as invariant lines + λ labels; the
    characteristic cubic solved for real roots, eigenvectors via row cross
    products, complex eigenvalues noted; see `examples/eigen3.manic`). Remaining
    3D: **planes intersecting for a 3×3 solve** (the 3D row picture of `Ax = b`).
  - *3D lesson:* `examples/linear-algebra-3d.manic` ties the 3D forms together
    (one matrix, transformation then eigenvectors), the companion to the 2D
    `examples/linear-algebra.manic` five-idea lesson.
  - *Remaining (optional, not blocking "done"):* a 3D **`Ax=b` as three
    intersecting planes** viz would round out the 3D row picture; everything else
    in the rung is shipped.
- **Calculus and numerical analysis** — the numerical *operations* on a curve
  are shipped: differentiation (`tangent`/`slope`/`normal`/`deriv`), definite
  integration (`area`/`integral`/`accum`, composite Simpson), root-finding
  (`roots` bisection + `newton` zig-zag), interpolation (`spline`, Catmull-Rom),
  and ODE stepping (`trajectory`, RK4 — orbits/spirals/phase portraits). But
  calculus as
  a *subject* is only partly covered — the notable gaps:
  - ✅ *Shipped:* the **derivative as its own curve** (`deriv`) and the
    **accumulation function** `∫ₐˣ f` (`accum`) — together they *show the
    Fundamental Theorem* (`deriv(accum(f))` traces back onto `f`; see
    `examples/ftc.manic`). Both are first-class graphs (numerically sampled via
    `GraphSrc::Samples`), so `tangent`/`slope`/`area` work on them too. Also
    **`extrema`** (maxima/minima = roots of `f'`), **`inflections`** (concavity
    flips = roots of `f''`), and **`band`** (the filled region between two
    curves) — see `examples/curve-features.manic`, `examples/band.manic`.
  - ✅ *Shipped:* **limits** (`limit` — finite points show the value approached
    with an open circle + approaching dot, `examples/limit.manic`; and
    `limit(…, inf)` / `-inf` auto-detects and draws the **horizontal asymptote**,
    `examples/limit-infinity.manic` — `inf`/`infinity` is now a numeric constant)
    and **Taylor series** (`taylor` — the degree-n polynomial about `a`, growing
    to hug the curve; `examples/taylor.manic`). Both numerical.
  - ✅ *Multivariable (shipped):* `surface3` now remembers its `z(x,y)`
    (`Entity3D::surf: SurfaceFn`, the 3D analog of `GraphFn`), and on top of it —
    **`gradient3`** (steepest-ascent arrow, ∂f/∂x & ∂f/∂y), **`tangentplane3`**
    (the tangent plane patch), and **`volume3`** (the volume under the surface as
    a 3D Riemann-sum column grid = double integral). See
    `examples/multivariable3.manic`, `examples/volume3.manic`.
  - *Still to do:* sequence/series convergence (partial sums marching to a
    limit), directional derivatives, and vector-field divergence/curl.
  Status: single-variable calculus is complete, and the core of **multivariable**
  (gradient / partials / tangent plane / volume) now ships. Numerical methods
  were the right first step because their intermediate states are already an
  animation storyboard.
- **Statistics and probability** — ✅ *DONE — Tiers 1–5 all shipped (descriptive
  + shape + distributions + CLT/LLN/correlation + inference + confidence intervals
  + random processes); a new 17-builtin `stats` kit with a seeded PRNG.* The widest
  everyday-relevance rung and the biggest non-programmer audience. Unifying idea:
  turn **data** — or a **random process** — into a picture that reveals its
  shape, centre, and spread, plus the truths that only appear *at scale*
  (distributions, convergence, relationships). Animation-first, so each builtin
  shows a *process*, not a static chart: a histogram **builds up** bar by bar,
  sample means **pile into a bell**, a running proportion **settles** onto the
  true probability. Reuses much of what already ships: `plot`/`GraphFn` for
  distribution curves (the `gauss`/`bell` named functions already exist),
  `area`/`integral`/`accum` for probability-as-area and PDF→CDF, `leastsquares`
  for regression (already shipped), and the number-list parsing from
  `leastsquares` for datasets.
  - *Substrate (new):* a small stats core — mean / median / quantiles /
    variance-std, histogram binning, correlation `r` — plus distribution
    formulas (normal PDF/CDF, uniform, exponential, binomial, Poisson) as
    plottable curves. **Critical design constraint:** sampling demos need a
    **seeded, deterministic PRNG** (an LCG seeded from a DSL argument), NOT system
    entropy — a "1000 coin flips" scene must render the same frames every time
    (reproducible renders are core to the engine). Data is a number list
    (`"v1 v2 v3 …"`), reusing the `leastsquares` parser.
  - *Tier 1 — describe a dataset (flagship trio):* ✅ **`histogram`** (shipped —
    bins a number list into bars, the shape of the data, staggered in bar by bar;
    gold mean marker + range labels; bars tagged `{id}.bar{k}`/`{id}.bars`;
    `histogram_bins` core; new `stats` kit; see `examples/histogram.manic`).
    **`summary`** — the **descriptive-statistics** workhorse: the data as dots on
    a number line, with **mean / median / mode** markers and the **spread** (a
    ±σ band), plus live readouts of **range, variance, standard deviation**. One
    builtin covers most of central-tendency + dispersion. **`boxplot`** — the
    five-number summary (min · Q1 · median · Q3 · max) as a box-and-whisker, so
    the box *is* the **interquartile range (IQR)** and whiskers/outliers show
    tails. A tiny **`skew`** label (left / right / zero) can piggyback on
    `histogram` for **shape**. All cheap: reuse bars / number-line / point parsing.
    ✅ *Shipped:* **`summary`** (`describe` → mean/median/mode/range/variance/std)
    and **`boxplot`** (`five_number` → min·Q1·median·Q3·max, IQR box, 1.5·IQR
    outlier detection; see `examples/summary.manic`, `examples/boxplot.manic`)
    and **`skew`** (`skewness` moment coefficient, mean-vs-median tell, labelled
    right/left/symmetric; see `examples/skew.manic`) — **descriptive statistics
    and shape are complete** (central tendency + dispersion + skewness).
  - *Tier 2 — distributions:* ✅ **`bellcurve`** (shipped — the normal/Gaussian
    bell for μ, σ with the 68–95–99.7 rule shaded as nested ±1σ/±2σ/±3σ bands,
    mean line, % labels, value ticks; alias `gaussian`; named `bellcurve` not
    `normal` to avoid the calculus perpendicular-line builtin; see
    `examples/bellcurve.manic`); the other named
    distributions (uniform / exponential / binomial bars / Poisson);
    **probability = area** under the curve between `a` and `b` (reuses `area`);
    and **PDF → CDF** as the running integral of the density (reuses `accum`).
  - *Tier 3 — truths at scale:* ✅ **`clt`** (shipped — the Central Limit Theorem:
    histograms the averages of `samplesize` dice over `trials` runs → they pile
    into a bell that hugs the theoretical normal; **seeded LCG** (`lcg_next`,
    `clt_means`) so the render is reproducible — this is the promised seeded PRNG
    substrate; see `examples/clt.manic`). Remaining: the **Law of Large Numbers** (a
    running proportion/mean converging to the truth) — ✅ **`lln`** (shipped:
    `lln_proportions`, coin-flip proportion settling onto 0.5, seeded; see
    `examples/lln.manic`); ✅ **`correlation`** (shipped —
    scatter + best-fit line + the Pearson **r** with a strength/direction reading;
    `regression` helper returns `(m, k, r)`; see `examples/correlation.manic`); and
    ✅ **confidence intervals / error bars** (shipped as `confidence`, Tier 4).
  - *Tier 4 — random processes:* ✅ **shipped.** **`montecarlo`** (π by darts,
    seeded), **`randomwalk`** (2-D wandering path, seeded); plus **`distribution`**
    (uniform / exponential / binomial / poisson) and **`confidence`** (a CI ± z·sd/√n)
    round out the distributions/inference. See `examples/probability.manic` (a
    4-idea playground).
  - *Tier 5 — inference:* ✅ **shipped.** **`hypothesis`** (two-tailed z-test —
    p-value as shaded normal tails vs alpha; `normal_tail` numeric core),
    **`covariance`** (signed-area rectangles about the mean cross;
    `covariance_of`), and **`bayes`** (Beta-Bernoulli prior → likelihood →
    posterior for a coin's bias). See `examples/hypothesis.manic`,
    `examples/covariance.manic`, `examples/bayes.manic`.
  - *Recommended first slice:* the **Tier 1 trio** (`histogram`/`summary`/
    `boxplot`) — the "describe data" core, all cheap reuse — then **`normal`**
    (Tier 2), which reuses `plot` + `area` and unlocks the 68–95–99.7 rule. The
    **CLT** (Tier 3) is the flagship *payoff* once the PRNG + `histogram` exist,
    and the natural capstone lesson (`examples/statistics.manic`), mirroring the
    LA five-idea lessons.
  - *3D:* largely N/A / low priority (a bivariate-normal surface via `surface3`,
    or a 3D scatter — nice-to-have, not core to the rung).
- **Constraints and optimisation** — a small solver for distances, angles,
  incidence, and bounds would let authors state a construction's invariant
  instead of manually updating its points. It unlocks movable geometry,
  constrained mechanisms, fitting, gradient descent, and visual proofs by
  deformation. This needs explicit failure/degeneracy behavior, so it should
  follow robust predicates rather than precede them.
- **Symbolic algebra (CAS)** — 🅿️ *parked / design-only.* simplification,
  factoring, equation solving, and automatic differentiation would support
  step-by-step algebra and formula-led constructions. It is valuable when the
  explanation is about *manipulating an expression*, not merely plotting one.
  This is intentionally later: a CAS has a much larger correctness and
  product-scope cost than numeric math.
  - *Architecture (decided):* a **separate, pure, macroquad-free crate**
    `crates/manic-cas` — expression tree, simplify, differentiate, expand/factor,
    solve, and an ordered **step-list** — living at the language layer beside
    `manic-lang`, **not** in the engine. It returns plain **data** (a normalized
    result + the intermediate steps); a thin new engine **kit** (`kits/algebra.rs`)
    is the adapter that turns each step into a tagged `text` entity the author
    animates with existing verbs (`draw`/`stagger`/`morph`). Same "domain-agnostic
    core + pluggable kit" shape as `stats`. The engine depends on `manic-cas` and
    runs it at build/lowering time (like `plot`'s formula string); `manic-lang`
    needs only catalog specs for the new builtins in v1, and can add a dependency
    later for live browser-side symbolic preview (bigger WASM).
  - *End-to-end (author's view):* write an expression/equation string → the CAS
    derives the work → each step renders as an addressable entity → reveal them
    line-by-line like a teacher at the board. Uses: step-by-step **solve**
    (`2x+4=10 → 2x=6 → x=3`), a **symbolic derivative** that is both a formula
    label *and* a plottable curve (reuses `plot`/`GraphFn`), **expand/factor** as
    a `morph` between forms, **substitution** with highlighted replacement, and
    **equation-driven geometry** (exact solved intersections). Results are
    bindable (`let`) and flow into `counter`/downstream builtins like the numeric
    layer.
  - *Hard dependency:* the payoff lands only if the math **renders as math**
    (`x² + 2x + 1`, stacked fractions). ASCII (`x^2 + 2*x + 1`) undercuts the
    teaching benefit for the non-programmer audience.

**LaTeX / math typesetting — shipped ✅ (2026-07), on [RaTeX](https://github.com/erweixin/RaTeX), a core capability for all kits.**
`equation(id,(x,y),`latex`,[size])` typesets KaTeX-grade LaTeX (fractions, roots,
exponents, Greek, big operators) as a white-on-transparent PNG (RaTeX `embed-fonts`
→ self-contained binary, no font install), drawn via `Shape::Image { tint: true }`
so it takes the template colour and `color`/`recolor` work. LaTeX goes in **backtick
raw strings** (new lexer literal `` `...` ``) so `\frac`/`\theta`/`\neq` survive.
Display equations, inline `$...$` math, mixed text/math, semantic token colours,
and item-matched equation rewrites all ship. Equations are rasterized at output
scale, so ordinary `draw`/`fade`/`move`/`scale`/`recolor` animation remains crisp
at the target resolution. Native vector glyph/rule entities are an optional future
extension for calligraphic stroke-level draw-on, not a blocker for production use.
The original renderer decision is recorded below.

**Decision detail — adopt RaTeX, a CORE capability for ALL kits (not just creator):**
Every kit currently emits ASCII math (`x^2`, `pi*r^2*h`, `3600/47`, geo labels) — it
reads messy across the whole system, so this is engine-wide, not a creator add-on.
Chosen after surveying the field: browser-only MathML crates (katex-rs/pulldown-latex/
latex2mathml) can't render in native mp4; ReX is "not production"; embedding all of
Typst is overkill. **RaTeX** is pure-Rust, MIT, KaTeX-grade (>99.5% coverage), and
decomposed into `ratex-parser → ratex-layout → DisplayList → ratex-render`.
**Spike-validated** (2026-07, in-repo throwaway): the pipeline fetches, builds, and
renders textbook-quality output here (quadratic formula, Σ with limits, √ vinculum,
π/∠/°). Fonts = 20 KaTeX TTFs, 540 KB, MIT — bundle via `include_bytes!`.
Implementation record:
- **Phase 1 ✅:** `ratex-render` PNG → an `equation(id,(x,y),"latex",[size])`
  builtin using manic's existing `Shape::Image`. Full coverage immediately; bitmaps
  (fade/scale/move). Includes (both REQUIRED for Phase 1 to render at all):
  - **Bundle the fonts INTO the binary** — `include_bytes!` the 20 KaTeX TTFs
    (540 KB, MIT/OFL, ship their licence), like manic already embeds IBM Plex. NO
    system install, NO shipped font dir. `render_to_png` only accepts a `font_dir`,
    so extract the embedded bytes to an OS cache/temp dir once at startup and point
    `font_dir` there (the loader's global cache keys on the dir → one-time cost).
    Self-contained across EC2 headless, both Linux cross-builds, and WASM.
  - Render transparent-bg + template-fg colour (recolour DisplayList items; default
    is black-on-white).
- **Inline and mixed notation ✅:** `$...$` spans render inside ordinary text, with
  baseline-aware layout and semantic colouring.
- **Reactive rewrites ✅:** order-preserved equation items persist while changed
  terms enter, leave, or move; confidence-selected overlapping fallback prevents
  blank or structurally misleading intermediate frames.
- **Future ⬜:** consume the `DisplayList` as native manic glyph + rule entities for
  vector scaling and calligraphic stroke-level draw-on. A matching browser preview
  may use the same RaTeX pipeline when a renderer is added to the editor.
- **Probability and statistics** — ✅ *shipped* — deterministic (seeded) sampling,
  distributions, regression, histograms, and confidence intervals broadened the
  engine into data and algorithm explainers while retaining reproducible recordings.

*Status (2026-07):* root-finding, **calculus/numerical methods**, **linear algebra**
(2D Tiers 1–3 + the core 3D forms), and **statistics & probability** (Tiers 1–5)
ship. Constraints/optimisation and robust numerical geometry remain accepted
future math work; symbolic algebra stays parked because of its much larger
correctness and product scope. These domain items do not outrank the creator
queue above. Any later layer should expose computed values to the existing
timeline, counters, plots, geometry, and 3D scene instead of becoming a separate
math subsystem.
Typography is complementary but separate: LaTeX makes mathematics readable;
the capabilities above make it behave correctly.

### Physics — mechanics and waves shipped; advanced domains future

Physics is a major shipped domain (alongside `algo` and the math family — see
manic's "domain-agnostic core + pluggable kits" thesis), and it is exceptionally
well-timed: physics *is* applied calculus, and the calculus/ODE substrate already
ships. **Unifying idea:** a system **evolves under forces/rules**; show the motion
*and* the invisible quantities (velocity, force, energy, momentum) that govern it —
animation-first, so each builtin shows a *process*, not a static diagram.

**Seeded by a goldmine.** `crypto-tool` already holds **38 RK4 sims**
(`crypto-tool/src/main/webapp/physics/labs/js/sims/*.js`) plus a shared core
(`../core/solver.js` = generic n-dim RK4/Euler/midpoint, `state.js`,
`rigid-body.js`, `collision.js`) and reusable views (`energy-bar.js`,
`time-graph.js`, `potential-well.js`, `direction-field.js`). Each sim is a **uniform
declarative spec** that already splits *physics-as-data* from *rendering* — exactly
manic's kit shape. Per-sim fields: `vars` (state vector w/ index + symbol),
`params` (value/min/max/step/unit), `init(p)`, **`evaluate(vars,change,params)`**
(the ODE right-hand side — the physics), `energy()` (KE/PE/total),
`potentialEnergy`+`peWellConfig`, `theoreticalPeriod`, `trailPoint()` (body world
position), **`vectors()`** (velocity + acceleration at the body), `worldRect`
(world→screen map), `presets`, `views` (sim/phase/time/energy/well). The physics
(derivatives + energy formulas + world layout) is **language-agnostic** and
transcribes directly into manic sim definitions.

**The one real substrate change — generalize the integrator.** manic's `trajectory`
is a 2-var RK4 (`dx/dt`, `dy/dt`); these sims are **n-dimensional** state vectors
with an `evaluate` that fills `change[]`. So the single engine addition is a
**general n-dim RK4** (the JS `solver.js` is already generic on `vars.length` — a
direct reference). Everything else is reuse. **Determinism is preserved** by the
`trajectory` precedent: **pre-simulate the whole run at build time** into sampled
tracks, then the stateless timeline just replays them — so scrubbing and
frame-identical recordings still hold (a rare, valuable property: reproducible
physics videos).

**Reuse map (mostly existing machinery):**

| crypto-tool sim field | manic mechanism | New? |
|---|---|---|
| `evaluate` RHS | RK4 integrator (**generalize `trajectory` → n-dim**) | the one substrate change |
| `trailPoint()` / positions | drawn body (dot/rod) + traced path | reuse |
| `vectors()` (v, a) | `arrow`/`vector` glued to the body via updaters (`follow`/`derive`) | reuse |
| `energy()` | `counter` + energy bars | reuse |
| phase / time views | `plot` (x(t), v(t), E(t), phase portrait) | reuse |
| `worldRect` | plot-style screen mapping (pixels-per-metre) | reuse |

**Design boundaries:**
- **World units** — physics has real units (m, s, kg); needs a world→screen scale
  like `plot`'s mapping. `worldRect` in every sim already supplies it. Small.
- **One kit or several** — the shipped mechanics/waves simulations use one
  `physics` kit; genuinely different later domains may split out.
- **Sim spec in-engine vs authored** — the shipped simple interface is a named
  simulation registry. A general authored-system layer remains future.

**Shipped inventory (34 named simulations):**
- **Pendulum family ✅ COMPLETE:** pendulum, double-pendulum⭐, spring-pendulum,
  kapitza, cart-pendulum, compare-pendulum.
- **Spring family ✅ COMPLETE:** spring, vertical-spring, spring-incline, bungee,
  resonance, double-spring, series-parallel-springs, car-suspension, **spring-chain**
  (3-mass/2-spring coupled oscillators on an incline).
- **Pulley family ✅ COMPLETE:** pulley/Atwood, pulley-scale, block-tackle
  (N-strand block & tackle), compound-pulley (fixed + movable, A/B/C),
  incline-pulley (the incline-Atwood).
- **Inclines ✅:** ramp (+ `forces(id)` free-body diagram view), incline-bumper
  (slide into a spring), double-incline (two-slope wedge + apex pulley),
  **loop-track** (ramp → vertical loop-the-loop — the curved-track solver).
- **Other mechanics:** piston, molecule, robot-arm, drop-mass, raft-cm,
  brachistochrone.
- **Collisions ✅ (1-D core):** a shared impulse resolver `collide_1d` (elastic/inelastic, restitution e), event-driven; **newtons-cradle**, **collide-blocks** (elastic/inelastic + walls), **bullet-block** (embed) all ship on it. Remaining: billiards (needs a 2-D impulse extension).
- **Waves ✅:** string-wave (the discretised wave equation — N masses on springs,
  fixed ends; a plucked pulse travels and reflects).
All on the one `Sim` trait + n-dim RK4 + the four generic views (plus the
build-time energy/kinematic solvers for the event/curved-track cases). Textbook
rendering (`template("paper")` + `support`/`sticky`) composes over any of them.

**⬜ Deferred physics domains/simulations:**
- ⬜ **cart-pole** — needs a balancing controller (LQR/PD gains to tune).
- ⬜ **quadrotor** — 13-var control system.
- ⬜ **billiards** — 2-D collision; needs a 2-D impulse extension of `collide_1d`.
- ⬜ **E&M family** — generator, oscillating-charge, current-coil-magnetic-field,
  generator-3d (a new electromagnetism domain).
- ⬜ **Stretch / separate domains** — pile (rigid body), states-of-matter
  (thermodynamics), navier-stokes (fluids), circuit MNA, pycharge relativistic-EM.

**Tiered inventory (✅ = shipped; ⭐ = flagship):**
- **T1 · trivial (≈3-var):** spring ✅, vertical-spring ✅, spring-incline ✅,
  pendulum ✅, drop-mass ✅, pulley-scale ✅, bungee ✅.
- **T2 · oscillation / chaos:** ⭐**double-pendulum** ✅, double-spring ✅,
  spring-pendulum ✅, compare-pendulum ✅, kapitza-pendulum ✅, resonance ✅,
  series-parallel-springs ✅, spring-chain ✅.
- **T3 · coupled / control (bigger state):** cart-pendulum ✅, cart-pole (open),
  robot-arm ✅, quadrotor (open), brachistochrone ✅, piston ✅, car-suspension ✅,
  molecule ✅.
- **Pulleys / inclines ✅:** pulley/Atwood, block-tackle, compound-pulley,
  incline-pulley, ramp (+forces), incline-bumper, double-incline, loop-track.
- **T4 · collisions:** newtons-cradle ✅ (event-driven, via `collide_1d`); collide-blocks ✅, bullet-block ✅; billiards (open — 2-D impulse).
- **T5 · electromagnetism:** generator, oscillating-charge,
  current-coil-magnetic-field, generator-3d.
- **Stretch / separate domains:** string-wave (waves, 203 vars), pile (rigid body,
  100), states-of-matter (thermodynamics), navier-stokes (fluids), and the
  `circuit/` MNA simulator + `pycharge/` relativistic-EM subsystems — their own
  future domains, **not** RK4 point-mechanics.

**Why it fit:** it is mostly *reuse* (integrator generalization + drawables that exist)
sitting on a *ready, tuned* physics corpus; the double pendulum alone is a
standout demo; and it visibly *depends on* the shipped calculus/ODE core — the same
"the diagram is true, not drawn" thesis, applied to motion.

#### Architecture — "adapt, simulate, connect"

**Unifying model:** *a simulation = named state + their time-derivatives + a map
from state → drawables.* The shipped named-simulation layer supplies the simple
surface today; a formula-authored builder remains a possible future layer.

**Layer 1 — named sims (adapt-by-tweaking), for everyone.** The ~20 goldmine RK4
sims ship as named builtins; a non-programmer picks one and changes numbers/presets
— zero physics knowledge:
```
pendulum(p, center, length: 2, gravity: 9.81, angle0: 60);
draw(p);   // an ordinary entity → animate on the timeline
```

**Future Layer 2 — a system builder (author-your-own).** This conceptual surface
would express the same pendulum from equations—the author writes the math, not
the plumbing. It is design context, not currently accepted DSL:
```
system(s, center, scale: 120);
state(s, "theta", 60);  state(s, "omega", 0);
flow(s, "theta", "omega");                 // dθ/dt = ω
flow(s, "omega", "-(g/L)*sin(theta)");     // dω/dt = −(g/L)·sinθ
body(s, bob, "L*sin(theta)", "-L*cos(theta)");
rod(s, arm, origin, bob);
simulate(s, 12);                           // pre-integrate 12 s (RK4, deterministic)
```

**Three seams:**
1. **Simulate** — pre-integrate at build time into sampled state tracks (the
   `trajectory` precedent); the stateless timeline *replays* via a `time` track
   (`to(s, time, 12, dur)` → scrub / slow-mo / pause / replay for free).
2. **Animation engine** — every `body`/`rod`/`vector`/energy-bar is a tagged
   entity id → `draw`/`show`/`pulse`/`follow`/`section`/presets/branding all apply.
3. **Math engine** — a **shared world→screen mapping** (physics `scale`/world-rect
   *is* `plot`'s `GraphView`) + **bindable state** let physics and math combine on
   one stage: spring → `plot` x(t) → `tangent` = velocity; pendulum → (θ,ω) phase
   portrait via the `trajectory` plotter; damped spring → `leastsquares` the decay
   envelope; orbit → swept `area` = Kepler's 2nd law. One file, two kits, no glue.

**Adaptation ladder:** today authors can tweak a preset/parameter, restyle it, and
add trails, views, or force arrows. Equation overrides and a general
`state`+`flow` system remain future possibilities. The first rung needs no physics
knowledge.

**Scope status:** the named-simulation layer, n-dimensional deterministic RK4,
generic playback data, and reusable phase/well/time/energy views ship. The
pendulum remains the flagship and the double pendulum the chaos demo. A general
author-defined `state`/`flow` system builder was explored but is not part of the
current DSL; it should be added only when a creator use case justifies that extra
vocabulary.

#### Remaining physics work

- ⬜ Share a first-class world-units/coordinate mapping with `plot`, so a
  simulation and its mathematical graph can inhabit exactly the same frame.
- ⬜ Add 2-D collision impulses before the deferred billiards simulation.
- ⬜ Treat E&M, thermodynamics, fluids, controls, and circuit simulation as
  separate domain increments rather than expanding point mechanics indefinitely.
- ⬜ Consider an authored `system` builder later; named sims remain the simple,
  non-programmer-first interface today.

**Layer-1 status — the pendulum swings end-to-end.** ✅ The first named sim is
*shipped* as two builtins: **`pendulum(id,(cx,cy),[length],[angle0],[unit],
[damping])`** (ctor — builds the `Pendulum`, pre-simulates 240 RK4 frames at build
time, lays out `{id}.pivot`/`{id}.rod`/`{id}.bob`/`{id}.path` tagged bare `{id}`+
`{id}.parts`, stores the screen-space body path in a new `Scene.sims` side-table)
and **`swing(id,[dur])`** (verb — replays that path as a keyframed `Pos`/`To`
track chain; `swing` is in `verb_consumes_structure_id` so it doesn't broadcast
over the bare-id tag). This covers a pragmatic ③ (playback via the `Scene.sims`
side-table of typed `PlaybackTrack`s + `resolve`'s keyframe chaining — no new
`Prop` needed) and a minimal ④ (per-pendulum `unit` px/m). **Overlays shipped:**
the velocity arrow `{id}.vel` (gold, tangent, length ∝ speed) and the KE/PE energy
bars `{id}.ke`/`{id}.pe` (cyan/magenta, normalised to initial total energy so a
damped swing visibly bleeds energy) with labels, tagged `{id}.overlays`. **Args
are minimal-required:** only `id` is mandatory — `center` (default `(640,200)`),
`length`, `angle0`, `unit`, `damping` all default, so `pendulum(p); swing(p)`
works. `examples/pendulum.manic` renders deterministically. The surface is
registered, catalog-matched, arity-audited, editor-checked, and covered by the
workspace regression suite; docs are synced.
**Generic view layer shipped.** A sim's ctor now stores a reusable `SimData`
(raw state trajectory + `(KE,PE)` per frame + `dt` + var labels + phase/pos-var
metadata + a sampled well curve; in `scene.rs`), and **opt-in view builtins read
it generically** — `phase(id,(cx,cy),[size])` (phase portrait: closed loop vs
damped spiral) and `well(id,(cx,cy),[size])` (potential-energy well with the body
as a ball rolling in it). Each lays out its own auto-fit panel + curve + a marker
that it *appends to the sim's `swing` playback*, so all views animate together.
The `Sim` trait carries the view metadata as **defaulted methods**, so a sim
opts into a view just by overriding one (a sim that doesn't stays view-less) —
the "perfect baseline template" for future sims. **All four views ship:** `phase` (portrait), `well` (potential well),
`timegraph` (θ(t)/ω(t) with a sweep line), and `energygraph` (KE/PE/total over
time). The two graph views share an `add_time_view` helper (multi-curve + swept
"now" line). `examples/pendulum.manic` is a **four-view dashboard** (sim + 2×2
panels), renders deterministically.

**Second sim shipped — the baseline generalises.** `spring(id,[center],
[stiffness],[x0],[unit],[damping])` is a mass–spring (SHM) — a different system
(state `[x,v]`, motion along x, a **parabolic** well ½kx² vs the pendulum's
cosine) that inherits all four views *for free* via the same `Sim` trait. The
velocity-arrow + energy-bar overlays were extracted into a shared `add_overlays`
helper both sims call, and the playback verb was generalised to **`run(id,[dur])`**
(with **`swing`** kept as a pendulum-friendly alias — both map to `v_play`).
`examples/spring.manic` is a four-view spring dashboard; renders deterministically.
**Third sim shipped — the double pendulum⭐ (chaos).** `doublependulum(id,
[center],[angle1],[angle2],[unit])` — two arms hinged end-to-end, the coupled EOM
transcribed from the goldmine; deterministic yet sensitive to initial conditions.
Parts `{id}.pivot/.rod1/.bob1/.rod2/.bob2` + the outer bob's chaotic trail
`{id}.path` (trace it with `par { run(dp,d); draw(dp.path,d); }`). It's a 4-D
system, so `phase`(θ₁ vs θ₂)/`timegraph`/`energygraph` apply but **`well` is
refused** with a clear error (the generic view layer degrades gracefully — a sim
opts out of a view just by leaving its metadata empty). `examples/double-pendulum.manic`.
**Full pendulum family shipped** (all on the one `Sim` trait): `pendulum`,
`doublependulum` (chaos), `springpendulum` (elastic — swings + bounces, coil),
`kapitza` (inverted-stable via fast pivot vibration), `cartpendulum` (spring cart
+ pendulum), `comparependulum` (two 0.001-rad-apart pendulums diverging). **Full spring family shipped** too: `spring` (SHM), `verticalspring`,
`springincline`, `bungee` (one-sided cord), `resonance` (driven), `doublespring`
(coupled/beating), `seriesparallel` (series vs parallel), `carsuspension`
(quarter-car on a scrolling road) — springs drawn with the real stretching `Coil`
primitive. The complete shipped inventory is maintained above; each simulation is
roughly a struct plus a constructor and inherits the generic views where applicable.
Driven, higher-dimensional, and multi-body simulations opt out of views that do
not fit. A shared world→screen map with `plot` remains the meaningful Layer-1
polish item. A dedicated time-indexed playback `Prop` is only a future optimization
if the current per-frame track chain proves heavy.

### Optics — shipped ✅

**The theme is manic, not lens design.** The goldmine
(`crypto-tool/src/main/webapp/physics/js/optical-designer-{model,trace,render}.js`)
is a *serious* sequential lens-design ray-tracer — Sellmeier dispersion over a real
glass catalog, vector Snell's law with total-internal-reflection, closed-form
ray–conic intersection, ABCD paraxial matrices, spot diagrams and aberration plots.
Per **goldmine-reimagine-not-port**, we keep the **physics faithful** but **throw
away the engineering GUI** (surface tables, RMS sliders, f/# read-outs). What ships
is a handful of **dead-simple builtins a non-programmer can drop into a scene** —
`refract`, `lens`, `prism`, `achromat` — each showing light *doing something*, with
the true `n(λ)` underneath so the color effects are real, not painted.

**Substrate — geometric, not RK4.** Optics has **no time dimension**: it is a static
closed-form ray trace (like the collision sims' build-time trajectories), producing
ray **polylines** + **glass polygons** + a **focal dot** + light entities — all
ordinary manic entities, so tag-broadcast, `cam`/`zoom`, `draw`/`show`, and
`template("paper")` compose for free. **Animation = a parameter sweep** (build
`sweep` from day one): each builtin precomputes frames as one parameter
(**wavelength · incidence angle · lens radius/focal · object distance**) varies,
stored as a playback track and replayed with **`run(id,[dur])`** — the focus slides,
TIR switches on, the rainbow fans out. Same deterministic build-time-precompute
precedent as the physics `run`.

**Modular kit layout (keep files small).** Not one giant `optics.rs`. A module dir:
- `src/kits/optics/mod.rs` — kit registration + shared types (`Ray`, `Surface`, `Medium`).
- `src/kits/optics/dispersion.rs` — the glass catalog + `sellmeier_n(λ)` (faithful port).
- `src/kits/optics/trace.rs` — the physics engine: 2-D vector Snell + TIR, ray–surface
  (spherical/conic) intersection, `trace_sequential`, and the ABCD paraxial helper
  (reuses the linalg 2×2 mental model) for the focal point.
- `src/kits/optics/builtins.rs` — the author-facing ctors (`refract`/`lens`/`prism`/
  `achromat`), each emitting entities + a sweep playback track.

**Core builtins:**
| builtin | non-programmer's mental model | physics underneath |
|---|---|---|
| `refract(id,[n1],[n2],[angle])` | "a light ray bends crossing into glass/water" | 2-D Snell + TIR cutoff; sweep `angle` → watch TIR switch on |
| `lens(id,[center],[focal],[kind])` | "parallel rays focus to a point" | ray fan → real focal length; sweep `focal`/radius → focus slides |
| `prism(id,[center],[glass])` | "white light splits into a rainbow" | Sellmeier `n(λ)` per color → the spectrum fan (the iconic visual) |
| `achromat(id,…)` | "red and blue focus apart — then a doublet fixes it" | true axial chromatic Δf, then a BK7+SF2 doublet pulls them together (the capstone) |

`prism` is the **optics** builtin; the existing 3-D solid stays `prism3` (no clash).
A small named **glass catalog** (`bk7`, `sf11`, `f2`, `water`, `diamond`, …) selects
Sellmeier coefficients by name, so authors never touch numbers.

**Implementation tiers:**
- **T1 · foundations:** ✅ **`refract`** (Snell + TIR sweep — the modular kit
  `src/kits/optics/{mod,trace,builtins}.rs`; sweeps the incidence angle via a
  `SimData` playback replayed by `run`; `examples/refraction.manic`), ✅ **`lens`**
  (converging lens — a parallel beam focuses to F; sweeps the focal length so the
  focus slides; ideal thin lens; `examples/lens.manic`).
- **T2 · dispersion:** ✅ **`prism`** (Sellmeier rainbow — the new
  `src/kits/optics/dispersion.rs`: 3-term Sellmeier + a named glass catalog
  (`bk7`/`sf11`/`f2`/`diamond`/`water`/`sapphire`/`silica`) + wavelength→RGB; each
  colour traced through both prism faces with `refract_vec`+`ray_segment`; sweeps
  the incidence angle; `examples/prism.manic`), ✅ **`achromat`** (chromatic
  aberration → the doublet fix — real crown dispersion splits the red/blue foci,
  `run` sweeps in the correction and they merge to one sharp point;
  `examples/achromat.manic`). **T2 · dispersion COMPLETE — the through-dispersion
  first milestone is shipped.**
  - **Annotated/elevated examples (hybrid backdrop):** the *geometric* builtins get
    `template("paper")` textbook figures (`refraction-paper`, `lens-paper`); the
    *colour* builtins stay on a dark bench where light glows (`prism-cinematic`,
    `achromat-cinematic`) — a rainbow washes out on cream, so light is a
    dark-background subject. Each varies its elevation lens (camera / typewriter /
    wordpop / brace) per [[demo-elevation-controls]].
- **T3 · systems:** ✅ **`lenssystem`** (a REAL multi-element lens ray-traced
  through its actual spherical surfaces — presets singlet/doublet/triplet; the
  new `trace::trace_spherical` 2-D ray–sphere intersection; rays are drawable
  polylines and `run` sweeps a sensor plane + live spot-size read-out showing
  **spherical aberration**; f-number read-out; `examples/lens-system.manic`).
  "Best of both": faithful physics + manic animation. Now also **NA read-out +
  autofocus** (a magenta best-focus marker at the minimum-spot plane). **Lens
  prescriptions both ways:** pick a real design by NAME (singlet/biconvex,
  plano-convex, meniscus, doublet/achromat, triplet/cooke) OR write a CUSTOM
  prescription string `"radius thickness glass [conic] [aperture] | …"`
  (`resolve_prescription`/`parse_prescription` in `builtins.rs`;
  `examples/lens-prescription.manic`). **Full prescription surface fields shipped:**
  `trace::trace_conic` (2-D ray–conic intersection) gives **aspherics** — the
  `"aspheric"` preset's conic (K≈−0.55, an ellipsoid) nulls spherical aberration
  (RMS 1.5 px → 0.1 px, a real blur→point; `examples/aspheric-lens.manic`) —
  plus **per-surface aperture** (clips rays + sets element height) and an optional
  **finite object distance** (diverging point source; f/#/NA hidden off-axis of
  the collimated case).
- **Off-axis field aberrations ✅:** a **3-D conic tracer** (`trace::trace_conic_3d`
  + `refract_vec3`) powers **`fieldspot`** — a full 2-D pupil traced in 3-D at a
  field angle: symmetric on-axis, a **coma** comet + **astigmatic** stretch
  off-axis (singlet RMS ~7 px vs doublet ~1.4 px at 8°), with an **Airy-disk**
  diffraction-limit overlay that scales with f/# (small = geometry-limited, ~spot
  = diffraction-limited). `examples/off-axis.manic`. **Optics kit T1–T4 + full
  prescription + field aberrations COMPLETE.**
- **T4 · analysis ✅:** **`rayfan`** (the ray-fan aberration plot — the singlet's
  cubic spherical-aberration S-curve, flattened by the doublet; `examples/ray-fan.manic`)
  and **`spotdiagram`** (the spot at best focus — a blur disc for the singlet,
  a point for the doublet, RMS read-out + ideal-point marker; `examples/spot-diagram.manic`).
  Both share `optics::builtins::analyze_preset` (rotationally-symmetric on-axis
  transverse-aberration trace) and scale to the singlet so the correction reads.
  Off-axis field points and the Airy-disk overlay ship through `fieldspot` above.

**Why it fits:** a beautiful, genuinely-physical domain (the rainbow is *earned* by
`n(λ)`, the focus is *earned* by Snell), tiny author surface, and it reuses every
existing manic primitive — the same "the diagram is true, not drawn" thesis, now for
light. Follows the manic-builtin-checklist for each ctor (catalog + LANGUAGE +
SYSTEM_PROMPT + CAPABILITIES + test + example + WASM/system-prompt snapshots).

### 3D foundation — status (legacy roadmap #1–#6 all shipped)
The foundation roadmap below has shipped; the newer creator roadmap above adds
the production and vocabulary layer. Coverage against the
~96 Asymptote `graph3` / `three` / `solids` / `tube` examples is:
- **Geometry** — parametric **curves** (`curve3`), height-field **surfaces**
  (`surface3`), **general parametric surfaces** (`param3` — `x/y/z(u,v)`, so
  tori/Möbius/parametric spheres/shells), regular-polygon **solids**
  (`prism3`/`pyramid3`), and **solids of revolution** (`revolve3`) ship (surfaces
  and solids render **filled + flat-shaded**, not wireframe), arbitrary 2D
  shapes / boolean regions **extrude** into solids (`extrude3` — this doubles as
  **CSG solids**: extrude a `union`/`difference`/`intersect`/`xor` region), and
  `curve3`/`line3`/`arrow3` can be drawn as constant-radius shaded **tubes**
  (`thick`); `tube3` adds variable radius, `contour3` extracts height-field
  levels, and `model3` imports controlled geometry-only OBJ meshes from either
  documented production-bundled `asset:` URIs or provisioned ordinary paths.
- **Rendering** — template-aware ambient/key/fill diagram lighting + flat
  per-face shading ship for
  surfaces/meshes/`cube3`/`sphere3`, tube-style thick strokes ship for paths
  (`thick`), and intersecting translucent geometry is depth-sorted (opaque
  first, then translucent back-to-front). `finish3` adds opt-in smooth shading,
  mesh emphasis, bounded depth/shadow cues, matte/metal/glass treatments, and
  deterministic checker/stripe procedures. Arbitrary light/shader graphs and
  image textures intentionally remain out of scope.
- **Labels & graphing** — depth-aware projected labels (`pin3`) and fully
  ticked/labelled, auto-decluttering 3D axes (`axes3`) ship. `label3` adds
  natural depth scaling while remaining a crisp camera-facing projected label;
  extruded glyph geometry intentionally remains out of scope.
- **Dynamic constructions** — `follow3` + `midpoint3`, live `link3` edges, and
  `project3` principal-plane projections recompute as their sources move.
- **Animation breadth** — `morph3` blends curves, surfaces, and solids (solids
  reparameterised spherically), and `to` now animates 3D `morph`/`opacity`/
  `scale`/`trace`/`color`; the dedicated verbs (move3/rotate3/grow3/…) cover
  position, rotation, and size.

**3D roadmap (prioritized).** Same principle as the 2D plan — extend a few
existing mechanisms rather than add a builtin per Asymptote class. Two
prerequisites recur and are the real leverage: a **3D→screen projection hook**
(so the existing 2D `text`/`label`/`counter` overlay can pin to a projected 3D
point) and a **`Vec3` `derive`/updater** (mirror the 2D dependent-point path).

| # | Requirement | How to address (extend what) | Effort | Unlocks |
|---|---|---|---|---|
| 1 | **Ticked/labelled 3D axes + projected labels** ✅ **shipped** | `project()` world→screen hook; `pin3` (a 2D label glued to a 3D point/entity, reprojected each frame); `axes3` now emits tick marks + auto-`pin3`ed numbers (optional `step`). | Small | Readable 3D graphs + labelled points/vectors/axes. |
| 2 | **`Vec3` dynamic constructions** ✅ **shipped** | Added a 3D `derive`/`follow` resolve pass; `follow3` (track another entity + offset) and `midpoint3` (derived point) recompute each frame. `link3`/projections extend the same hook. | Medium | Live 3D geometry: dependent points + tracking that recompute as sources move. |
| 3 | **Parametric curve & surface** ✅ **shipped** | `curve3(id,"x(t)","y(t)","z(t)")` → drawn-on `Shape3D::Path`; `surface3(id,"z(x,y)",…)` → filled, flat-shaded `Shape3D::Surface`; `param3(id,"x(u,v)","y(u,v)","z(u,v)",…)` → a **general** parametric surface (tori/Möbius/shells — can wrap/close). The `plot` expr engine was widened to **two variables** (`x`/`y`, `u`/`v`). | Medium | Helices/Lissajous, `z=f(x,y)` surfaces, and closed/parametric surfaces (the full `graph3` corpus). |
| 4 | **Indexed meshes & solids** ✅ **shipped** | `Shape3D::Mesh` (verts + tri `faces` + wireframe fallback) + `prism3`/`pyramid3` (n-gon extrusion/apex) + `revolve3` (solids of revolution) + `extrude3` (extrude any 2D fillable shape or boolean `Region`). `extrude3` reuses `geom.rs` (`entity_to_multipolygon` + `earcutr`), so extruding a `union`/`difference`/`intersect`/`xor` region **is** boolean CSG (plate-with-hole, L-beams, …). | Large | Prisms/pyramids/cylinders/cones, vases/spheres/lathes, arbitrary/concave extrusions, and CSG solids. |
| 5 | **3D rendering upgrades** ✅ **shipped** | Surfaces/meshes/`cube3`/`sphere3` render **filled** with deterministic template-aware ambient/key/fill diagram lighting and readable back faces (chunked under the u16 index cap). `curve3`/`line3`/`arrow3` draw as shaded **tubes** via `thick(id,radius)` (rotation-minimising frame; arrows get a solid cone head). Translucent geometry is **depth-sorted** (opaque first, then translucent entities + their triangles back-to-front). The newer `finish3` layer supplies bounded materials/procedural textures and depth/shadow cues; arbitrary shader/light graphs remain out of scope. | Large | Solid-looking 3D, correct translucent overlaps, publication-quality output. |
| 6 | **3D morph / general `to`** ✅ **shipped** | `morph3(a,b,[spin])` samples both shapes to a shared form — curves→polyline, surfaces & solids→a filled/shaded grid (solids reparameterised onto a spherical `(θ,φ)` grid via bbox-centre raycasting, so cube↔sphere works). `to` extended to animate 3D `morph`/`opacity`/`scale`/`trace`/`color`. | Large | 3D `Transform` / `ReplacementTransform`, mesh/path morphing. |

Planned order (agreed): **1 ✅ → 2 ✅ → 3 ✅ → 4 ✅ → 5 ✅ → 6 ✅** — the full
3D roadmap has shipped. #4 shipped `Shape3D::Mesh` + `prism3`/`pyramid3`/`revolve3` + `extrude3`
(arbitrary/concave extrude **and** boolean CSG, both via `geom.rs`); #5 shipped
filled + flat-shaded faces with deterministic template-aware studio lighting
(surfaces/meshes/`cube3`/`sphere3`), tube strokes
(`thick`), and depth-sorted translucency. The creator roadmap later adds the
bounded `finish3` layer while keeping arbitrary shader/light graphs de-scoped.
#1 and #2 are mostly *reuse* (the projection hook + a `Vec3` updater)
and together make 3D genuinely usable for explainers; #3 brings the `graph3`
corpus within reach off the existing `plot` sampler. #4/#5 are the orthogonal
"real 3D engine" work — big, and only needed once the legible-diagram cases land.

### Generative / repetitive — ✅ Shipped
manic now has a computation layer, evaluated before the scene is built:
- **`let name = expr;`** numeric variables;
- **arithmetic** (`+ - * / ^`, unary `-`, parens, `pi`/`e`/`tau`, ~20 functions)
  usable anywhere a number or `(x,y)` coordinate goes;
- **`for v in a..b { … }`** range loops;
- **id interpolation** `bar{i}` so a loop generates unique entities (then
  `tag` them into a group to animate together).
Plus, since Phase 2:
- **`def name(params) { … }`** macros — reusable parametric groups, and they may
  **recurse** (with a depth guard), so fractals/trees are a few lines
  (`examples/fractal_tree.manic`);
- **`if cond { } else { }`** (and `else if`) with comparisons `< <= > >= == !=`
  and logic `&& ||` — recursion base cases, conditional figures.
Fully additive — expressions collapse to literals at lowering time, so kits are
unchanged and any plain `.manic` behaves exactly as before. Examples:
`area_under_curve.manic` (a `for` n-sweep), `fractal_tree.manic` (recursive
`def`), `riemann_rainbow.manic` (loop + `hue` + `stagger`).
- **Reductions** — `sum(i in a..b : expr)` (also `prod`/`min`/`max`) aggregate
  over a range, so totals are computable in-language; paired with a `counter`
  entity + animatable `value` track, a computed number **counts up live** on
  screen (`examples/riemann_readout.manic`: a Riemann area summed and tweened).
Still missing: stepped/`downto` ranges, string/name variables (macro params are
numeric), and a live **measured-geometry** binding (a readout that reflects a
moving entity's actual length/angle). General authored scalar binding now ships
through `parameter` + `bind`; measurement needs the `derive` hook to feed a
counter from geometry rather than from an authored parameter.

### Typography — math and deterministic glyph engine shipped
- ✅ **LaTeX / math typesetting:** display equations, inline `$…$`, fractions,
  matrices, mixed text/math, semantic colouring, and reactive rewrites ship on
  the bundled RaTeX renderer described above.
- ✅ **P0 deterministic text + glyph engine.** Every ordinary-text extended
  grapheme cluster is shaped and rasterized through an embedded-only chain:
  IBM Plex Mono (requested regular/bold style) → Noto Sans Math → Noto Sans →
  Noto Sans Symbols → Noto Sans Symbols 2, with Noto Sans Arabic and Noto Sans
  Devanagari as script-specific fallbacks. Plain text and text runs inside mixed
  text/LaTeX use the same cached layout for measurement, wrapping, alignment,
  baseline, grapheme reveal, glow, rotation, zoom, and drawing. Existing DSL
  files require no font option or new verb. The bundled manifest, immutable
  hashes, provenance, and OFL notices ship with all eight faces.

  `manic check` now scans base text plus timed `say`, shape replacement, and
  `become` text, reporting the owning entity, story stage when applicable,
  Unicode character/code point, and repair choices instead of silently accepting
  tofu. The initial regression corpus is
  `→ ← ↔ ⇒ ✓ ✗ ● ○ ◆ ◇ ∞ ≤ ≥`.

  The internal bundled-only `TextEngine` uses `cosmic-text` advanced shaping for
  GSUB/GPOS, bidi, RTL joining, Indic conjuncts, combining marks, and variation
  controls. Host font discovery is disabled by construction.

  **Font correctness issue register (blocks further Systems Kit expansion):**

  | ID | Status | Core issue | Required invariant / proof |
  |---|---|---|---|
  | FONT-01 | ✅ | Direct preview/record could bypass `Movie::validate`, so `manic check` might reject text that another production entry point still rendered as tofu. | Public `RenderSession::new(Movie)` is the checked production boundary; its inner movie is private and the frame loop accepts only a validated session. CLI/direct-run tests prove invalid Unicode cannot enter playback. |
  | FONT-02 | ✅ | Embedded Macroquad font loads used `.ok()`; failure silently selected Macroquad's built-in font. | Every bundled face is now required and initialization names the failed face. Measurement and drawing never opt into a renderer/default font. |
  | FONT-03 | ✅ | Fallback and typewriter reveal split individual Unicode scalars instead of shaped grapheme clusters. | UAX #29 boundaries plus `cosmic-text` advanced shaping keep decomposed accents, variation controls, RTL/Arabic joining, bidi order, and Indic conjuncts intact; reveal filters glyph jobs at grapheme boundaries without reshaping prefixes. |
  | FONT-04 | ✅ | Mixed-face bounds took independent maximum height/offset values, while rich text used primary `Xg` plus a fixed descent. | One `ShapedLayout` owns ascent/descent, visual lines, advances, wrapping, alignment, baseline and glyph jobs; measurement and final raster consume the same cached object. |
  | FONT-05 | ✅ | Font discovery, fallback selection, shaping, glow and repeated text changes could repeat work every frame. | One playback-thread `FontSystem`, bounded layout/raster/GPU texture caches, content-addressed `LayoutKey`/`RasterKey`, and deterministic `say`/rewrite seek tests provide reuse without host discovery. |
  | FONT-06 | ✅ | Earlier tests proved code-point coverage and parsing, not pixels or cross-runtime layout parity. | Pinned multilingual CPU-layout fingerprints and alpha-pixel goldens cover fallback and wrapping; transform-independent reuse covers rotation/glow/zoom/supersampling, native preview and backend recording share the checked draw path, and the exact bundled renderer compiles for `wasm32-unknown-unknown`. Full Rust and shipped-example regressions pass. |
  | FONT-07 | ✅ | Font assets recorded names/licences but not immutable provenance. | The manifest records source revision/version, SHA-256, role, order, and licence for every face. A Rust regression hashes the exact embedded/package bytes and requires the manifest and OFL notice; package inclusion covers all eight faces and documents. |
  | FONT-08 | ✅ | Renderer-only types such as fallback slots were wider public API than intended. | Font slots and selection helpers are crate-private; the stable author surface remains semantic `mono` / `bold` / `display` with no font-specific DSL vocabulary. |

  A fix is not complete merely because the symbol corpus passes `has_glyph`.
  The acceptance gate is the same resolved layout driving validation,
  measurement, wrapping, animation reveal, and final drawing in every runtime.

  The shipped engine is an internal `TextEngine` backed by
  `cosmic-text`, with a single shared `FontSystem`, shaped layout buffers, and a
  glyph raster/cache bridge into the existing Macroquad renderer. Production
  constructs the font system from bundled bytes only—never OS-installed
  fonts—so native preview, backend recording, and WASM render the same glyphs.
  IBM Plex Mono remains the primary visual identity. The shipped Noto Sans,
  Math, Symbols, Symbols 2, Arabic, and Devanagari faces cover extended Latin, common arrows,
  relations, geometric shapes, technical symbols, dingbats, and related
  monochrome glyphs. Later script fonts join through the bundled manifest, not
  renderer or DSL changes.

  One shaped layout must drive **measurement, wrapping, alignment, baseline,
  glow, and drawing**. Maintaining a fallback-aware draw path beside the old
  single-font measurement path is not acceptable because it would create new
  overlap and responsive-layout regressions. Inline/display LaTeX remains on
  RaTeX; structural arrows, lines, and diagram symbols should still use native
  Manic geometry when they carry semantic meaning.

  `manic check` must scan every ordinary and mixed-text run through the same font
  selection path. If no bundled face supports a grapheme, report the entity,
  character, Unicode code point, stage/format where relevant, and actionable
  remedies: use a native primitive, use LaTeX for math, or install/enable an
  approved bundled script face. Silent replacement boxes are a failed check.

  **P0 acceptance contract:**

  1. Existing IBM-supported examples remain visually and metrically stable.
  2. A symbol corpus including `→ ← ↔ ⇒ ✓ ✗ ● ○ ◆ ◇ ∞ ≤ ≥` renders without
     tofu and wraps from the same measured advances used for drawing.
  3. Mixed primary/fallback runs retain baseline, centering, rotation, glow,
     camera zoom, and supersampled recording quality.
  4. Repeated `say`/rewrite text changes invalidate and reuse glyph/layout caches
     deterministically without per-frame font discovery.
  5. Native, backend, and WASM fixtures select identical bundled faces and
     produce equivalent layout bounds.
  6. Unsupported glyphs fail clearly; colour emoji remains explicitly deferred
     until COLR/CPAL and bitmap-font rendering have a deliberate design.
  7. Font files, licences, manifest entries, mdBook guidance, system prompt,
     editor diagnostics, and Rust regression tests ship together.

- ⬜ **Native equation outlines:** optional future glyph/rule geometry for
  calligraphic stroke-level draw-on and resolution-independent authoring.
- ⬜ **Custom / selectable branding fonts — later, separate from P0.** P0 chooses
  deterministic bundled fallback automatically and introduces no DSL. A later
  creator feature may select approved faces per entity or template and may load
  user-supplied files after licensing, security, backend, and WASM behavior are
  designed. Font choice must not be required to repair missing glyphs.

## Engine extensions behind the active queue

The top-level **Active work queue** is authoritative. This dependency view maps
that product work to reusable engine mechanisms; it is not a second roadmap.
The guiding principle remains to extend a small number of general mechanisms,
not introduce one builtin for every reference-library animation.

| Priority | Requirement | Engine direction | Unlocks |
|---|---|---|---|
| P0 ✅ | **Deterministic text + glyph engine** | Bundled-only advanced shaping, one cached measure/wrap/reveal/draw layout, strict checked render sessions, immutable eight-face manifests, actionable diagnostics, pinned pixel/layout goldens, and native/backend/WASM compile parity ship. | Reliable symbols and text for Systems, Flowcharts, Sequence Diagrams, multilingual lessons, backend renders, and WASM. |
| P1 | **Production runtime contract** | Expose stage selection and time ranges through a stable runtime API with full-movie defaults. | UI/backend rendering of named stages without hidden CLI flags. |
| P1 | **Visual audit layers** | Add structural, resolved, and rendered-frame checks on top of the existing baseline auditor. | Safer automated generation and clearer creator diagnostics. |
| P1 | **Multi-format composition** | Make responsive regions, safe zones, typography, and pacing adapt across portrait, square, and landscape. | One story authored once and delivered to Shorts, posts, and lessons. |
| P2 | **General bounds + relative placement** | Promote the kit-level bbox work into a reliable engine service with anchors and group-aware placement. | `next_to`-style layout, framing, braces, collision avoidance, and responsive composition. |
| P2 | **Live geometry measurements** | Feed derived lengths/angles/areas into counters, equations, and bindings. | Truthful readouts on moving geometry and simulations. |
| P2 | **Path motion + nonlinear remapping** | Let position tracks sample curves and let groups transform through authored functions. | Curve-following particles, orbits, deformation, and richer calculus/linear-algebra explanations. |
| P3 | **Typography and look extensions** | After P0 correctness, add creator-selectable branding fonts, native equation outlines, and optional sketch styling. | More author identity and calligraphic math without changing scene semantics. |

Linear transforms, general 2-D shape morphing, LaTeX rendering and rewrites,
composable scale/spin entrance effects, and the prioritized 3-D roadmap already
ship; they are no longer listed as missing foundations.

### Stateful structures — ✅ Shipped

The timeline is a pure function of `t`, so an ordinary verb sees only the base
scene: a *chain* of swaps would each read stale positions. This is now solved
with a **mutating-verb** kind — `MutVerbFn = fn(&mut Scene, &Args) -> Clip` — and
a build-time occupancy map `Scene::occ` (structure id → entity per slot), plus
`Scene::motion_pos` for repeated positional cycles. A mutating verb produces
its clip and updates the relevant logical state, so the next step sees the
current arrangement. This composes across the stateless timeline without any
render-time state.

- **`swap(arr, i, j)`** (std, mutating) — the values in slots `i`/`j` **slide**
  past each other (one hops over the top) into the swapped slots, and `occ`
  updates so a whole sort chains correctly. `swap(a, b)` (two entity ids) still
  does the plain position swap.
- **`cycle(a, b, c, …)`** (std, mutating) — rotates ordinary entities through
  their logical positions along arcs; repeated cycles keep moving rather than
  rereading stale t=0 positions.
- **`compare(arr, i, j, [color])`** (algo) — flashes the values *currently* in
  those slots (reads live `occ`), the comparison step of a sort.

See `examples/bubble_sort.manic` — real in-place sort, no `say`.

## Presets & branding — ✅ Shipped

**Shipped.** Rendering is driven by named **presets** (`--preset <name>`) — the
baseline for quality, frame rate, container, and branding; any runtime flag
(`--scale`, `--fps`, `--gif`, `--no-brand`, …) overrides the preset's fields
(`src/preset.rs`).
- **`studio`** (default) — branded, `scale 1.5` (→1080p), 60fps, MP4.
- **`test`** — unbranded, `scale 1.0`, 30fps; the fast verify preset.
- **`reel`** — branded, for vertical/social clips (pair with a `canvas("9:16")`).

**Branding** (`src/branding.rs`) is injected by the **engine, never authored in
the DSL**, and applies only to **recorded** output under a branded preset (so the
live preview + stills stay clean and fast):
- a **pre-roll intro** — the hue-graded fractal tree grows (yellow trunk →
  magenta/blue tips) while the `Manic` wordmark typewrites in beside it over the
  link `https://8gwifi.org/manic`; authored internally in manic (a recursive
  `def`) and composed ahead of the user's timeline;
- a pinned **"Made With Manic"** watermark for the whole DSL portion.

Disable with `--no-brand`. (Also fixed: the `--png`/`--alpha` sequence now writes
frames upright — `export_png`'s internal flip is cancelled in `record.rs`.)

## Creator Kit v2 core — shipped ✅

The first Creator Kit shipped the complete quiz-Short loop (`creator`/`socials`,
`quiz`/`option`/`run`, countdown, safe-zone guide, figure auto-fit, four skins and
five question reveals). V2 is an intentional production redesign, not a second
pile of skins. Its shipped core contains three ordered slices:

### V2.1 — responsive layout and design foundations

- **Viewport-aware kit layout.** Creator constructors must read the actual canvas
  dimensions instead of baking `540`/`1920` coordinates. One format must adapt to
  portrait `9:16`, feed `4:5`, square `1:1`, and landscape `16:9` canvases.
- **Platform safe areas.** Named `shorts`, `reels`, `tiktok`, and `clean` guides
  provide top/bottom/side insets; all automatic format regions stay inside them.
- **Shared regions.** Header, media, choices, timer, caption and footer are derived
  from the safe content rectangle and density, rather than positioned separately.
- **Creator design tokens.** A small internal style model owns typography roles,
  spacing, card fill/edge, accent use, glow, option density, timer treatment, and
  motion recipe. The default is a restrained **studio/editorial** look: strong
  hierarchy, one accent, crisp panels and purposeful motion. `badge`, `minimal`,
  `glass`, and `plain` remain available and backwards compatible.
- **Reliable fitting.** `figure()` uses shared entity bounds and includes text,
  images/equations, curves, stroke and scale. It must fail clearly on an empty
  target and avoid silently producing a broken live construction.

### V2.2 — Quiz v2

- Preserve every v1 file unchanged: `quiz(q,"?")`, the old skin/reveal words,
  `option(...[,correct])`, and `run(q,dur)` remain valid.
- Extend the order-free quiz spec with explicit `key=value` options for
  `layout`, `density`, `timer`, `motion`, and `reveal`. Defaults stay concise.
- Responsive answer layouts cover 1–6 options (stack up to four; auto/grid up to six),
  long-answer wrapping, phone-readable minimum type, and deterministic overflow
  diagnostics instead of overlaps.
- Timer treatments: `ring`, `bar`, `number`, and `none`. Reveal treatments keep
  the correct answer legible, deliberately de-emphasise distractors, and allow an
  optional author-supplied explanation/source without inventing a solution act.
- Motion recipes: `calm`, `studio`, `punch`, and `cut`, with timing derived from
  the requested `run` duration rather than hard-coded absolute beats.

Representative v2 authoring surface (accepted keys are documented by parser
errors and tests):

```manic
canvas("9:16"); template("mono");
creator(me, "@anish2good name=Optics_Lab yt=zarigatongy x=@anish2good web=8gwifi.org/manic accent=cyan footer=compact");
quiz(q, "Which glass bends blue light more?",
     "studio layout=media-first reveal=rise timer=bar density=comfortable");
option(q, "Crown glass");
option(q, "Flint glass", correct);
option(q, "Both equally");
option(q, "Neither");
prism(p, (540, 650), "sf11");
figure(p);
explain(q, "Flint glass has stronger dispersion.");
run(q, 12);
socials(me);
```

### V2.3 — creator brand system

- Extend `creator(id,"spec")` without breaking existing specs: display name,
  handle, logo/avatar image, accent/secondary colours, tagline, website, footer
  style and default CTA live in one reusable profile.
- Footer variants: `compact`, `signature`, `social`, and `none`; automatic layout
  uses configured identity content and stays inside the active safe area.
- A reusable `endcard(profile, [spec])` produces a professional final creator
  lockup with optional CTA. Custom avatar/channel art remains optional through
  `logo=`; the social footer itself uses native vector marks.
- Brand choices are creator content, separate from manic's engine-level recorded
  watermark/pre-roll and from the global canvas `template()` palette.

### V2 core acceptance criteria

1. Old Creator Kit examples parse, validate, and retain their existing entity ids.
2. The same v2 quiz source lays out without overlap on 9:16, 4:5, 1:1, and 16:9.
3. Stress cases cover 2–6 choices, long text, inline math, light/dark templates,
   logo/no-logo profiles, and representative geo/physics/optics figures.
4. Unit tests cover spec parsing, layout regions, safe-area selection, backwards
   compatibility, profiles, footer variants, and end cards.
5. Representative frames are rendered and visually inspected at question,
   countdown, answer-reveal, and end-card moments before v2 is called complete.
6. `SYSTEM_PROMPT.md`, the creator book chapter, examples, and this capability
   ledger are updated together with the implementation.

**Deferred until after the v2 core:** fact-card, listicle, this-or-that and other
format families will reuse these foundations, but are not allowed to delay the
responsive quiz + brand-system release.

**Implementation result (2026-07-18):** ✅ logical canvas size now reaches every
kit through `Scene`; ✅ responsive header/media/choices/timer/footer regions adapt
across 9:16, 4:5, 1:1 and 16:9; ✅ named Shorts/Reels/TikTok/clean safe areas;
✅ rounded translucent-safe UI panels; ✅ a restrained studio palette under
`template("shorts")`; ✅ Studio is the new quiz default while all v1 skin/reveal
words and entity ids remain; ✅ v2 `layout`/`density`/`timer`/`motion`/`safe`/
`accent` parsing; ✅ width-aware answer type and 1–6 auto/grid layout (stack is
guarded at four); ✅ optional `explain`; ✅ expanded creator profile, four footer
styles and hidden `endcard`; ✅ improved `figure` bounds for paths/text/images/
equations plus live-dependency diagnostics; ✅ catalog, prompt, book, gallery and
`examples/creator-v2.manic` updated. Creator/Timing regression coverage includes
all four aspect ratios and generic named phases, and the full workspace suite
passes.
Question, choices/countdown, reveal, end-card, square and landscape frames were
rendered and visually inspected. That visual pass caught and fixed translucent
corner overdraw, timer/explanation collision, and narrow-card text overflow.

**Gold-path Reel documentation — shipped ✅ (2026-07):** mdBook now promotes a
first-class `Create a polished Reel` workflow directly after Getting Started.
It covers platform-safe composition, phone-first content hierarchy, layout and
motion choices, exact pacing, native timer selection, reusable branding,
end-card design, still-frame review, and Reel export. The copyable
`examples/perfect-reel.manic` starter is editor-checked and visually reviewed at
its hook, countdown, reveal, and end-card beats.

**Creator v2 + LaTeX review set — shipped ✅ (2026-07):** three focused examples
exercise inline and display math through the responsive Creator surface:
`examples/creator-v2-latex-calculus.manic` (9:16 studio),
`examples/creator-v2-latex-algebra.manic` (1:1 paper), and
`examples/creator-v2-latex-physics.manic` (16:9 studio). Portrait, square, and
landscape frames were rendered and visually inspected. The review also fixed
tintable equation images to use semantic template remapping, keeping formula
options legible on light templates.

### Creator v2.4 — questions, options and native socials shipped ✅ (2026-07)

This pass deliberately does **not** expand general image/asset support. It
polishes the high-frequency authored surfaces that should work from DSL alone:

- Question headers now allocate separate decoration and text regions, so the
  kicker/rule cannot collide with a wrapped prompt. Stable tags expose
  `{id}.question` plus `.panel`, `.kicker`, `.rule`, and `.text` roles while
  preserving existing ids such as `q.q` and `q.qrule`.
- `labels=letters|numbers|none` controls the option index treatment. Letters are
  the compatibility default; number/no-label modes suit ordered choices and
  polls. Answer cards reserve a uniform right-side check zone, auto-fit long
  text, and centre a single card in the final row of a five-choice grid.
- Options expose stable `{id}.options`, `{id}.option.a` through `.option.f`,
  role suffixes (`.card`, `.badge`, `.label`, `.text`, `.check`), and
  `{id}.option.correct`. This makes common A/B/C/D styling precise without
  depending on internal compact ids.
- The social footer uses one normalized native-vector registry for YouTube, X,
  Instagram, TikTok, Facebook, LinkedIn, GitHub, web, email, and a generic-link
  fallback. Common aliases normalize to stable tags such as
  `{id}.social.youtube` and `{id}.social.web`. Up to three configured values are
  displayed as professional icon+text lockups; larger sets remain responsive by
  collapsing to icons plus the profile handle.
- Maintained Creator examples now use `yt=zarigatongy`, `x=@anish2good`, and
  `web=8gwifi.org/manic`. The flagship v2 example is asset-free; optional
  `logo=` compatibility remains for authors who intentionally provide a custom
  avatar or channel mark.

Parser/layout/compatibility coverage includes label modes, semantic tags,
five-choice centring, canonical social aliases, exact profile values, and the
unknown-platform fallback. The mdBook, builtin catalog, system prompt, and
Creator/Reel examples document the same shipped surface. The full workspace
suite passes, including editor validation for every shipped example.

### Timing v2 core — generic + Creator adapters shipped ✅ (2026-07)

The original quiz timer deliberately shipped as a small surface
(`ring|bar|number|none`) with a fixed five-second display and motion-dependent
phase percentages. Timing v2 keeps the ring as the polished zero-config default
but separates **choreography** from **timer appearance**:

- `timing(clock,[(x,y)],"intro=1 demo=6 finish=1")` creates a
  format-neutral named-phase controller. `timed(clock) { during("intro") {
  ... } ... }` schedules any ordinary manic animation at exact phase offsets
  while running the native timer in parallel. Phase source order is irrelevant;
  short blocks are padded, while overruns, duplicates, and unknown phases fail
  clearly instead of drifting. `duration=6` is a one-phase `main` shorthand.
- `timing(q,"...")`: pace presets plus explicit `ask`, `options`, `think`,
  `reveal`, `hold`, and `stagger` phases. Explicit phases make `run(q)` derive
  the total duration; legacy `run(q,dur)` continues to scale the preset beat.
- `timerstyle(clock|q,[(x,y)],"...")`: native `ring`, `bar`, `number`,
  `segments`, `ticks`, `pulse`, and `none` looks with responsive position,
  count direction, size, thickness, semantic colours, optional label/digit
  placement, and finish cue. `run(clock)` is the timer-only form; a generic
  controller never accepts a competing `run(clock,dur)` duration.
- Stable timer-part tags expose track/progress/value/label/effects for ordinary
  modifiers. Standalone `countdown` uses the same visual vocabulary.
- SVG is intentionally deferred: native primitives already provide scalable,
  template-aware, progress-animatable timers. A future SVG feature should
  convert paths to native traceable geometry instead of rasterizing them into a
  non-animatable timer image.

Delivered with exact generic/quiz phase and counter tests, backward
compatibility, catalog and prompt coverage, a non-quiz physics example,
dedicated portrait/square/landscape examples, and the six-look comparison
gallery. The full workspace suite passes. Mid-countdown frames were rendered and
visually inspected at 9:16, 1:1, and 16:9; that review also corrected horizontal
timer digit/label spacing so segmented and bar looks stay inside their regions.

## Creator format templates — v1 shipped ✅

**A new audience: content creators, not just domain educators.** Every kit so far
adds a *domain* (math, physics, optics). This is **orthogonal** — a *format* layer:
opinionated, slot-filled, branded, pre-timed scene generators for social formats
(YouTube **Shorts** / Reels / TikTok). A creator picks a template, drops in content
(a question, four options, an answer) and their branding (handles, accent colour),
and manic produces a polished vertical clip — no timeline authoring, no design
skill. This turns manic from a *tool* into a *product creators return to*.

**Worked example — the quiz Short** (the format the request describes): a question
appears → an animated figure/illustration → four option cards (A–D) → a countdown
timer → time-out → the correct answer is revealed (right card glows, the rest dim)
→ a socials footer (handles + icons). Roughly:

```manic
canvas("9:16");                 // portrait 1080×1920 (already supported)
creator(me, "@anish2good yt=zarigatongy x=@anish2good web=8gwifi.org/manic accent=gold");

// FREEDOM path — builder verbs: any number of options, per-option media later
quiz(q, "Which glass bends BLUE light more?");
option(q, "Crown glass");
option(q, "Flint glass", correct);      // mark the right one
option(q, "Both equal");
option(q, "Neither");
figure(q, prism);               // optional illustration slot — ANY manic entity / kit sim
run(q, 12);                     // plays the whole beat: ask · countdown · reveal
socials(me);                    // the creator's footer, pinned in the safe zone

// EASY path — one-liner shorthand for the canonical 4-option quiz:
//   quiz(q, "Which glass bends BLUE light more?", "Crown", "Flint", "Both", "Neither", answer: 2);
```

**Mostly reuse — the foundation already ships.** Portrait canvas ✅
(`canvas("9:16")` → 1080×1920), the **`reel`** branded preset ✅, engine branding
for 1080×1920 ✅, `par`/`seq`/`wait`/`stagger` timing, `Counter` (a live 5→0
countdown digit), `Arc` (a shrinking timer ring), colour/theme, `banner`/
`watermark`. A countdown = a Counter `Value` track + an `Arc` sweep; a reveal =
`show`/`flash`/`color` on the right card — all existing verbs. **The template only
bakes the layout + the timeline.**

**The `figure` slot takes ANY manic entity** — it references an id, and everything
in manic is an entity, so a shape, a group, a kit sim (`prism`/`triangle`/
`pendulum`), a `def`, or even a **live-animating** sim can be the illustration
(the prism disperses / the geometry constructs *while* the question shows). Bare-id
tag-broadcast moves/scales a multi-part builtin into the slot as one. The only new
bit is **auto-fit**: compute the entity/group's 2-D bounding box (no general helper
today — reuse the footprint-bbox pattern in `three.rs`) and scale+translate it into
the figure region; `figure(q, fig)` auto-fits, or the creator places it and it's
just marked as the slot content.

**⬜ Tracked polish (do after the `creator` kit build):** the figure's small dot
markers (e.g. a circumcentre) are a touch small for a phone screen — bump their
size / add a thin ring so they pop in the `figure` slot.

**Prototype-first — SHIPPED:** the first quiz Short is hand-authored from shipped
primitives in **`examples/quiz-geometry.manic`** (9:16): typewriter `type`
question, an **animated geometry figure** (the geo kit constructs the Euler line —
which *is* the answer), four `rect` option cards, a countdown ring + `say`-driven
digit, a time-out reveal (correct card `recolor`→lime + `flash`/`pulse`, the rest
`fade`), over a `text` socials footer. ~20 s, renders under the `reel` preset. That
proven file is the reference the `quiz`/`countdown`/`socials` builtins will later
collapse to a few lines — the same "build by hand, then extract the builtin" path
the physics sims followed.

**What's genuinely new:**
1. **Reusable UI components** (a small `creator`/`ui` kit): `choices`/`card` (the
   A–D option cards), `countdown` (ring + digit), `reveal` (highlight-correct /
   dim-others beat), `socials` (a handle+icon footer). Useful well beyond quizzes.
   The `figure` slot auto-fits any entity (bounds→scale). **The POC is
   template-agnostic** — it uses only palette-semantic colours (`fg`/`cyan`/
   `magenta`/`lime`/`dim`/`panel`, which the template remaps) and outline-only
   chrome, so it renders with correct contrast on `paper` (light) AND `terminal`
   (dark); the fixed consts (`gold`/`red`/…) are avoided for contrast-critical bits.
2. **✅ Raster image embedding SHIPPED** —
   `image(id, (x,y), "asset:name.png"|"path", [w], [h])`
   (`Shape::Image` + a thread-local macroquad texture cache preloaded in
   `player::run_loop`, drawn in `render::draw_entity`; missing file → a crossed
   placeholder box). Loads real **logos / avatars / photo backdrops**, animates
   like any entity, `examples/image.manic` + bundled
   `asset:manic-logo.png`. A bundled URI resolves independently of the working
   directory; ordinary paths remain available for caller-provisioned files.
   Engine-only (no browser preview — the WASM front-end has no macroquad). The
   quiz POC keeps its *drawn* vector social icons (no trademark PNGs bundled),
   but a creator can now drop their own real logo/avatar in via `image(...)`.
3. **Format templates** — `quiz` first; then a family: `countdown` (N→0 hype),
   `factcard` (hook → fact → source), `listicle` (top-N reveal), `thisorthat`
   (A-vs-B poll), `hotseat` (rapid Q&A). One builtin per format.
4. **Shorts safe-zones** — a portrait layout that keeps content clear of the
   platform UI (bottom action bar, right rail, top clock): a `safezone` helper or
   an automatic inset the templates respect.
5. **A creator profile** — `creator(id, handle, x, yt, ig, tiktok, accent, logo)`
   set once (or in a small reusable file) and reused across every video; drives
   the `socials` footer + accent colour. Extends the brand kit.
6. **A `shorts` theme/preset** — punchy caption sizing, bold outlines, high
   contrast for tiny phone screens, safe-zone insets on by default.

**SHIPPED so far (`src/kits/creator.rs`):** ✅ **`creator(id, "spec")`** — a reusable
profile parsed from a space-separated spec (`@handle`, `yt=`/`x=`/`ig=`/`tt=`/
`fb=`/`li=`/`gh=`/`web=`/`email=` pairs, `accent=colour`), stored in
`Scene::creators`. ✅ **`socials(id, [at])`** — draws the footer using normalized
native platform marks and configured values; `at` defaults to the responsive
bottom safe region. It needs no downloads or image/SVG assets; `logo=` remains
available for a separate custom avatar in compact/signature layouts. ✅ **`quiz(id,
"question")`** + **`option(id, "text", [correct])`** — the question (typewriter,
wrapped) + auto **2×2** option grid + countdown widget; the correct option gets a
lime highlight. ✅ **`run(id, [dur])`** drives the whole **ask → countdown →
reveal** beat (the shared `run` verb dispatches to `build_quiz_clip` when the id is
a quiz — `Scene::quizzes`). `option`/`socials` opt out of tag-broadcast
(`consumes_structure_id`). Figure is author-supplied. **`examples/quiz-euler.manic`
= the ~60-line POC collapsed to `quiz` + 4 `option`s + `run`. FIRST KIT VERSION
COMPLETE.** **Production polish done:** cards **slide up + fade** in (Pos+Opacity),
long answers **wrap** within cards, the reveal **pops** the correct card (lime
highlight Scale-bump + a **drawn ✓**) and **dims** the wrong ones (0.28) instead of
vanishing, and the geo figure **dots are bigger** (`r` 5→7 — the tracked nit).
**Auto-layout done:** `run` lays the answers out by count — a centred column for
≤3, a 2×2 grid for 4+ (2/3/4 all verified) — by computing each slot from the final
count and sliding the cards in via Pos tracks (options are created at a neutral
spot; `run` knows the total). **All the structural features shipped too:**
✅ a **draining ring** (the countdown ring is a full-circle `Arc` whose `trace`
animates 1→0 — the Arc line already honours `trace`, no new prop needed);
✅ **`countdown(id, [at], [secs])`** standalone (draining ring + digit as a
`SimData` playback, `run`-driven); ✅ **`safezone(id, [inset])`** (a faint 9:16
content-safe guide); ✅ **`figure(target, [center], [size])`** (auto-fit: a 2-D
bbox over the group, then a uniform scale+translate of each entity's shape into the
zone — a kit sim / tagged group drops in without hand-placing); ✅ a **`shorts`
template** (neon-on-black, extra glow, no chrome — for phone screens). The
`reveal` beat stays folded into `quiz`'s `run` (no separate builtin needed).
**Creator kit: first production version + all planned features COMPLETE.**

**Production redesign — card SKINS (verified by still-render):** the quiz was
rebuilt from wireframe-grade to broadcast-grade with **4 selectable card skins**,
chosen via the `quiz` style spec (order-free with the reveal, e.g. `"glass fade"`):
`badge` (default — framed question panel + a "QUESTION" kicker pill + coloured
letter-badge answer cards), `minimal` (kicker + accent rule, outline rows), `glass`
(glowing borders, Reels look), and `plain` (flat). One `SkinSpec` table drives the
question header, cards, and reveal, so a new skin is one entry — and every skin
still works under any global `template()`. The reveal now tints + glows the correct
card, draws a check, and turns the correct **badge green**; a persistent faint
track ring means the countdown never decays to a lone digit. All skins verified by
headless `--still` PNG export.

> ⚠️ **Testing status — creative kits need more field testing (pre/post-deploy).**
> The creator kit + the **Shorts system-prompt guidance** are shipping, but they've
> only been exercised on a handful of prompts. The failure modes we've already
> caught-and-fixed are all layout/authoring judgement, not engine bugs: figures
> hand-plotted instead of kit-constructed, `figure()` misused on live geo,
> pre-solved coordinates, the worked-solution act shown unprompted, figure labels
> colliding with the answer cards, and geo point labels left at the 22px default.
> Each fix went into the system prompt, not the engine — which means **the quality
> bar here lives in the prompt and must be validated by generating real Shorts**
> (across models, topics, and question types) and rendering them, not by unit
> tests. **Action items:** (1) build a small regression set of representative
> Short prompts and eyeball the renders after each prompt change; (2) keep the
> `--still` visual-check loop in the deploy workflow; (3) apply the same
> generate-render-critique discipline to **every future creative kit** (new formats
> like countdown/factcard/listicle/this-or-that) before calling them production —
> expect the first cut to need prompt tuning, and budget for it.

**Current authoring model:**
- **Named components for creators:** start with `quiz(q,"question"[,"spec"])`,
  add each `option(q,"text"[,correct])`, then `run(q[,dur])`. Standalone
  `countdown`, generic `timing`, native `socials`, safe areas, and end cards use
  the same Creator system. A multi-option `quiz(...)` shorthand is not part of
  the current DSL.
- **Composition for studios:** ordinary `def` macros and parameters can package a
  channel's preferred components and timeline without introducing a second
  template language.
- **Reusable identity:** `creator(id,"spec")` stores the channel profile once;
  `socials(id)` and `endcard(id)` consume it. The `creator` kit remains separate
  from engine branding, which owns Manic's watermark and pre-roll.

**Why it fits:** the same "fill it in, get a correct animation" promise aimed at a
huge new audience; ~80% composition of shipped primitives; and the quiz Short
alone is a proven, repeatable viral format — a creator can make one a day.

## Templates / themes — ✅ Shipped

**Shipped.** The look is a selectable **template**, chosen with
`template("name")` (or `--template <name>` at render time). Chrome is driven by
`style::Template` (`Chrome::None|Minimal|Full` + background + masthead strings),
carried on the `Movie` and read by `render::draw_page_chrome`.
- **`mono` (default)** — restrained black-and-white editorial palette on a
  near-black blank screen, no frame/dots/masthead/rule, with a subtle glow. A
  DSL file that omits `template(...)` gets this look.
- **`plain`** — the original saturated neon palette on a blank screen, retained
  as an explicit compatibility choice.
- **`terminal`** — the neon terminal-window chrome (border, corner brackets,
  window dots, centred title, masthead, two-tone rule), now opt-in.

`mono` aliases are `monochrome`, `blackwhite`, `black-white`, and `bw`. Tests
cover the DSL default, explicit-template override, aliases, and greyscale
remapping of every named semantic colour. Both the explicit mono Timing v2
scene and a template-free sine-wave scene were rendered and visually inspected.

**mdBook template guide shipped (2026-07).** Templates now have a dedicated
navigation chapter with a runnable mono sample, selection matrix, aliases,
semantic-colour and `hue(...)` behavior, DSL-versus-CLI override rules,
Creator/Reel recommendations, and phone-size review tips. Getting Started,
Colour & Style, Creator formats, the Reel gold path, and the introduction link
back to the same guide.

**Runtime palette shipped.** Each template carries a `style::Palette` (bg/fg/cyan/
magenta/lime/gold/red/orange/blue/dim/panel). Constructors emit semantic named
colours, and the renderer
**remaps** each palette colour to the active template's at draw time
(`Palette::remap`, in `draw_entity`), so `--template` retints **content** too,
while bespoke colours (`hue`, explicit RGB) pass through. Templates: `plain`
(neon palette), `terminal` (neon + chrome), `paper` (ink on cream), `blueprint`
(white/cyan on navy), `shorts` (creator studio), and `mono` (default greyscale).
**Masthead is author-set** (`masthead(...)`), empty by default — no
`manic ~ %` / `60FPS` branding is baked into any template.

**Per-template glow + CRT shipped.** Each template has a `glow` multiplier (applied
to every entity's halo at render) and a `crt` default. `plain`/`terminal` glow
= 1 (neon), `mono` = 0.35 (subtle), `shorts` = 0.65, and
`paper`/`blueprint` = 0 (crisp, flat — right for print). `--crt` still forces
the post-process on regardless of the template default.

**Still to do:** template-controlled **fonts** (needs alternate font assets
bundled — the separate "selectable fonts" work); more palettes; a `minimal`
chrome level exposed as a template.

### Hand-drawn / chalkboard look — ⬜ Future
Requested idea: make the output *look* hand-drawn — chalk on a blackboard,
student/teacher style — not just clean neon geometry. Two independent layers:
- **Chalkboard colours** — a `chalkboard` **template** (dark slate bg + chalky
  off-white/pastel palette + glow off). Small; fits the current template
  structure. Gets the *vibe* but lines stay crisp.
- **Hand-drawn line quality** — a new **`sketch`/rough render style** (NOT
  built): at draw time, perturb every stroke's polyline points with a little
  noise so lines wobble like a human hand, vary width unevenly, and overlay a
  subtle chalk grain/texture (the RoughJS / Manim-xkcd effect). This is what
  actually makes it look hand-drawn. Doable as a render-time pass over paths +
  a grain overlay; medium effort.
- Note: the *motion* already reads as "being drawn" (`draw` traces strokes on,
  `type` reveals text like handwriting) — this is only about the static *texture*.
- The two compose: `chalkboard` template + `sketch` style = teacher-at-the-board.
Decide later.

**What a template bundles today:**
- palette + the complete named-colour map (`fg`, `dim`, `panel`, and every
  semantic accent);
- chrome style (none/minimal/full), glow factor, and CRT default;
- optional author-set masthead text.

Chrome and engine branding are independent. `mono`, `plain`, `paper`,
`blueprint`, and `shorts` have no page chrome; `terminal` opts into the full
window treatment. Recording-preset branding remains separately controllable
with `--no-brand`.

## Web / editor language services — ✅ Shipped prototype

The editor half of the beta: a browser-loadable build of manic's **language
front-end** that powers an in-page code editor — **syntax highlighting**,
**autocomplete / intelligence**, and **live error-checking with fix
suggestions** — so an author writes `.manic` in the browser and sees exactly
what the renderer would say.

**Status.** All four phases done:
1. `manic-lang` — a macroquad-free workspace crate (lexer/parser/ast/diag),
   publishable, native engine unchanged (depends on it via a re-export).
2. **catalog** — `BuiltinSpec` for every registered builtin + fixed vocab, kept honest by
   a test asserting the catalog == the live registry (zero drift).
3. **expand** extracted into `manic-lang` (so the browser runs `let`/`for`/`def`).
4. **WASM API** — `tokenize` / `check` / `complete` (`crates/manic-lang/src/services.rs`,
   thin `wasm-bindgen` JSON wrappers under `--features wasm`), built with
   `wasm-pack` (~190 KB), plus a throwaway HTML/JS harness in `web/` (see
   `web/README.md`). The real editor UI is a separate, later design.

All service logic is unit-tested natively and verified
end-to-end through the compiled WASM. What follows is the design rationale.

### Approach — compile the Rust front-end to WASM (single source of truth)

The parser is intentionally not re-ported to JavaScript. A hand-written JS parser would drift
from the Rust engine, and the whole point is that what the editor validates is
*exactly* what renders. The existing Rust lexer/parser/expander compiles to
`wasm32-unknown-unknown` and exposes a thin JS/TS API. One grammar,
one lexer, one set of diagnostics — no divergence, and new builtins light up in
the editor the moment they're added to the engine.

### Architecture — a macroquad-free `lang-core`

The renderer pulls in macroquad (graphics), which does not belong in a headless
parser. The pure front-end is split into a crate/feature with no macroquad
dependency:
- **in**: `lexer`, `parser`, `ast`, the **`expand`** pass of `lower`
  (`let`/`for`/`if`/`def`/reductions/interpolation — pure arithmetic over the
  AST), `diag`;
- **out**: `Scene`/`Entity`/`Clip`, `render`, `player`, and the ctor/verb
  *function bodies* (they touch macroquad types);
- the **catalog** (below) replaces the executable registry for validation.

This is the one real structural cost — and it cleanly separates "language" from
"engine", which the architecture already aspires to.

### The builtin catalog (the key new artifact)

Autocomplete and argument checking use machine-readable specs for every builtin
instead of relying only on doc comments and hand-written
`a.ident(0)?`/`a.num(1)?` calls. The structured catalog is:
`BuiltinSpec { name, kind: ctor|verb|mut_verb, params:[{name, ty:
name|num|str|point|color|ease|ident, optional}], summary, kit }` — plus the fixed
vocabularies already in the engine: **colors** (`fg void cyan magenta lime dim
panel`), **easings**, **canvas presets**, **template names**, **reserved vars**
(`w h cx cy pi e tau`). A catalog-vs-live-registry test keeps the source aligned
with executable registration so it cannot drift.

### WASM API (thin)

- `tokenize(src) -> [{kind, start, len}]` — from the lexer, for highlighting.
- `check(src) -> [{message, start, len, severity, fix?}]` — lex + parse + expand
  + name/arg validation; `fix = {label, replacement, range}` when auto-fixable.
- `complete(src, offset) -> [{label, kind, insertText, detail, doc}]` —
  context-aware (builtins at statement start; the param's type inside a call).
- `signature(src, offset) -> {label, params, activeParam}` — signature help.

### Language services (on CodeMirror 6 or Monaco)

- **Highlighting** — token kinds → classes (keyword `let/for/if/def`, builtin,
  number, string, ident, point punctuation, comment).
- **Diagnostics** — `diag::Error` already localizes precisely by span, and
  several messages already suggest (`try: circular, row, grid`); surface inline.
- **Autocomplete** — builtins by kit at statement start; inside a call the
  expected param type drives suggestions (palette after a color param, easings
  after an ease param, **in-file entity ids + tags** after an id param); reserved
  vars + constants.
- **Quick-fixes** (from the catalog): unknown builtin/color/easing → nearest by
  edit distance (`magena`→`magenta`); reserved id used as an entity name (`h`) →
  offer a rename; missing comma / unmatched paren or brace → insert; wrong arg
  count/type → show the signature and flag the offending arg.

### Boundaries

A language service, **not** a renderer: it validates *syntax, names, arg shape,
and the build-time `expand` pass* — it won't catch issues that only surface at
render (a circle radius overflowing the canvas). Full validation still comes from
`manic check` / a render. A WASM **renderer** (macroquad → WebGL) is a separate,
larger future step.

### Implementation order — completed

Catalog alignment → `lang-core` split → WASM API/build → editor glue. The separate
full browser renderer remains outside this completed language-service slice.

## Where manic is ahead of Asymptote
- A **first-class animation timeline** — asy `animate` stitches frames; manic
  scripts beats (`par`/`seq`/`stagger`, sections, marker export) with
  deterministic recording.
- **Live dynamic constructions** — geo constructions and graph edges recompute
  as inputs move (GeoGebra-style), which static asy diagrams don't do.

## Map / Geography kit — ⬜ Deferred (PoC validated, not scheduled)

> **Status: findings only — do NOT build yet.** A working proof of concept
> exists (`src/kits/map.rs`, `assets/maps/`, `examples/map-border-poc.manic`) and
> proves the core pipeline. This section records the exploration — the target
> format, a capability decomposition, the two hard forks, and pros/cons — so it
> can be picked up deliberately later. **Deferred** behind the currently-queued
> work (physics domain, creator formats). Nothing here is committed direction.

### End goal (the reason to eventually build it)

Reel makers — **non-coders** — produce **map explainer reels** ("spotlight a
region, move to the next, explain with data") entirely through the two no-code
levers manic already has: the **AI assistant** (plain prompt → map DSL) and
**creator templates** (fill slots). Success = *"a non-coder gets a polished map
reel from one sentence,"* not *"you can draw a border."* This makes the map kit a
**creator FORMAT**, not a geo-primitive library — so the DSL must be a pit of
success for AI generation + templating (few footguns, intent-shaped,
self-framing), the same discipline applied to LaTeX generation.

### What the PoC already proves ✅

Geographic data → **projected at build time** into ordinary `Shape::Polygon` /
`Shape::Polyline` (tagged `id.fill` / `id.border`) → animated with the **existing
verbs** (`draw`, `recolor`, `pulse`, `show`). Zero renderer changes — the same
"project into native primitives" pattern as physics (RK4→entities) and optics
(rays→lines). Uses the `geojson` crate over Natural Earth data (public domain).
Confirmed: a real country outline renders and the standard verbs compose with it
for free. Current PoC limits: one bundled country (India, hardcoded), naive
equirectangular projection normalized into a fixed box (aspect distortion), and
author-supplied lon/lat bounds.

### Reference target — a geopolitics/data countdown reel

The benchmark reel (a "top oil reserves" countdown, portrait ~22s) decomposes
into these capabilities, each mapped to manic's reality:

**Already native (manic has it):**
- **Karaoke captions** (word-by-word subtitles) — manic already ships karaoke /
  wordpop.
- **Portrait reel + branding + pacing** — the creator kit.
- **Ranked badges** (①②③), **labels with pointers** ("Canada", "Alberta") —
  text + a circle; trivial.
- **Big animated callouts** ("14× poorer", "266B BAR") — text + color + pulse
  (glow/gradient is styling polish).
- **Sticker / emoji overlays** (oil barrels, character stickers, $-signs) —
  `Shape::Image` already exists (from the LaTeX work); needs an emoji/PNG asset
  path.

**New engine work, feasible, on the roadmap:**
- **Universal country + admin-1 (state/province) coverage** — the PoC pipeline +
  a data strategy. "Any country, any state" = Natural Earth `ne_10m_admin_1`
  (~3,600 features, several MB) is the pivotal dataset.
- **Auto-framing + name lookup** — `spotlight("Kerala")` looks up, frames,
  projects, styles. A creator/AI must NEVER type a lon/lat box. Precondition for
  no-code, not a nicety.
- **Zoom/pan camera** (`zoomto(region, dur)`) — an **animatable viewport** whose
  bounds tween over time; every projected shape re-fits. This is the signature
  "move" of every map reel; belongs in the core, not deferred within the kit.
- **Aspect-correct projection**, glowing colored outlines (stroke styling).

**The two genuine departures (decide before building):**

1. **Satellite-imagery basemap** (the aerial photo under everything).
   - *Pro:* it's the look that performs on TikTok; creators may expect it.
   - *Con:* fundamentally against manic's model — vector, deterministic,
     self-contained (bundled assets, no network at render). Requires either a
     huge bundled world texture **or** live map tiles (Bing/Mapbox/OSM) → drags
     in **licensing + network-at-render + non-reproducible renders**. Heaviest
     engineering, most off-identity piece, *and the least essential to the
     storytelling.*
2. **Flag-texture country fill** (a country filled with its actual flag).
   - *Pro:* carries a lot of the genre's identity.
   - *Con:* needs ~250 flag assets (small) **plus** clipping a raster to a
     polygon — a new render capability (textured/clipped fill). Moderate lift.

### The aesthetic fork — the decision that shapes everything

The reference is **photorealistic** (satellite + flags + emoji). That is the
*opposite* of manic's elevated, truthful, vector/editorial identity. Two roads
(this is the `goldmine-reimagine-not-port` principle applied to maps):

- **Path A — replicate the popular look.** Satellite basemap + flag fills +
  stickers.
  - *Pro:* matches exactly what performs; least "translation" for the creator.
  - *Con:* breaks the vector aesthetic AND the deterministic/self-contained model
    (tiles = licensing + network + non-reproducible), heaviest to build.
- **Path B — reimagine in manic's voice.** Clean **vector** basemap
  (land/ocean/graticule in the palette), palette or flag-accent fills, but the
  **same motion and storytelling** — zoom, spotlight, rank badges, karaoke,
  stickers.
  - *Pro:* on-brand, deterministic, self-contained, tractable engine; every
    storytelling element is already Path-B-native.
  - *Con:* not the exact photoreal look some creators may want.

**Working recommendation (not a decision):** Path B for the core — the
storytelling (zoom + spotlight + badges + captions + stickers) is what makes these
reels work, and it's all Path-B-native. Treat the **satellite basemap as an
optional opt-in layer later** if creators genuinely demand photorealism — never
the foundation.

### Proposed phased roadmap (coverage-first, reel-maker-first)

- **Phase 0 — PoC ✅** (done): projection + one baked country + animate.
- **Phase 1 — reel-ready core:** universal country + admin-1 coverage; name
  lookup + auto-framing (`spotlight`); `zoomto` animatable camera;
  aspect-correct projection; portrait/reel format; **validate the no-code loop**
  (1 template + AI-prompt generation) end to end.
- **Phase 2 — the creator's story on the map:** `marker("Delhi")` by name,
  labels, `route(a,b)` great-circle arcs.
- **Phase 3 — thematic & compare:** `choropleth(data)`, animated spread,
  before/after split.
- **Phase 4 — format library:** map-reel templates (Region Spotlight / Journey /
  Data Map) + system-prompt training so the AI nails map reels from a bare prompt.

### Data strategy options (Phase 1's pivotal decision)

- **Bake everything** (110m countries + simplified 10m admin-1) via `include_str!`.
  - *Pro:* dead simple, offline, deterministic; server-side render makes binary
    size a non-issue (manic never renders in the browser).
  - *Con:* a few MB added to the binary; harder to reach admin-2/cities later.
- **Data-pack (load, not bake):** ship a `maps/` dir the render env loads.
  - *Pro:* lean binary, scales to cities/admin-2/custom regions.
  - *Con:* every deploy path (`build-linux.sh`, `ec2-setup.sh`, the onecompiler
    render env) must carry the data dir.
- **Curated subset:** bundle a hand-picked set, expand on demand. *Pro:* tiny
  binary. *Con:* breaks the "any country, any state" promise.
- *Lean:* bake 110m countries + simplified admin-1 (server-side render → size is
  cheap), design a loader hook for higher-res/admin-2 later.

### Open decisions to settle before scheduling

1. **Path A vs. B** — photoreal replica, or elevated-vector reimagining?
2. **Is the satellite basemap a hard requirement**, or is "clearly a map, clean
   and truthful" enough?
3. **Flag fills** — must-have, or palette/accent fill in manic's voice?
4. **No-code lever** — AI-from-prompt as the north star ("make anything"), with
   templates as quality anchors? Or templates-first?
5. **Political stance as kit policy** — "any country" = every disputed border
   globally; one source's viewpoint + a standing disclaimer, settled before ship.

### Why deferred

The engine work is tractable and the storytelling is largely native, but the
kit's success hinges on product decisions (aesthetic path, basemap, political
stance) that aren't urgent while the physics domain and creator formats are the
active queue. Revisit when a repeated creator demand for map reels justifies
picking the forks above.
