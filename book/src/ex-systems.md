# Diagrams

Animated diagrams — not another static boxes-and-arrows generator. Declare an architecture (or a `flowchart`) with auto-positioned nodes and directed connections — geometry is optional, so a diagram auto-fits the canvas and, when it grows dense, scales itself down as one to stay inside the frame (you never touch a coordinate) — then move one persistent request through the graph. A flowchart ranks its nodes top-down and runs: a token walks the process and takes a branch. It also speaks **C4** — `c4(id, level)` with `person`/`system`/`container`/`component` nodes draws Simon Brown's model in the conventional outline style, tiers people over internals over externals, auto-splits a dense tier into a balanced grid, and zooms from Context to Container to Component as a request flows through it. Node artwork comes from a string kind (`aws:lambda`, `gcp:bigquery`, `onprem:redis`, `k8s:pod` — 17 providers, see [icon reference & aliases](systems-icons.md)); paths are coloured by relationship. The kit never infers behaviour — the creator authors it with `route`, `flow`, and ordinary verbs.

Each block is the whole file — copy it into `x.manic` and run `manic x.manic` (live) or `--record out` (video).

## systems-foundation

Provider-neutral: only native `client`/`gateway`/`service`/`cache`/`database` archetypes — reveals cold topology, sends one persistent request forward, and returns the same identity over separately directed paths, with no cloud assets.

```manic
{{#include ../../examples/systems-foundation.manic}}
```

<div class="manic-video" data-video="ex-systems-foundation"></div>

## systems-architecture-poc

The first story: one Buy request travels Browser → CloudFront → API Gateway → Lambda → DynamoDB → SQS, auto-laid horizontally or vertically by the responsive region.

```manic
{{#include ../../examples/systems-architecture-poc.manic}}
```

<div class="manic-video" data-video="ex-systems-architecture-poc"></div>

## systems-arrow-patterns

The connection-grammar reference: one-way, parallel, round-trip, port-aware orthogonal, fan-out, and diagonal-duplex lanes.

```manic
{{#include ../../examples/systems-arrow-patterns.manic}}
```

<div class="manic-video" data-video="ex-systems-arrow-patterns"></div>

## microservices-platform

Auto-fit showcase: Route 53 → load balancer → gateway, three availability zones of ECS services, a replicated RDS database cluster, and CloudWatch monitoring — declared with zero coordinates. Add a tier or a zone and the whole diagram reflows and scales itself down to the frame; paths coloured by relationship (request · write · replication · telemetry).

```manic
{{#include ../../examples/microservices-platform.manic}}
```

<div class="manic-video" data-video="ex-microservices-platform"></div>

## factorial-flowchart

A flowchart that runs: `flowchart(fc)` auto-lays seven shape-nodes (terminator/io/process/decision) top-down with no coordinates, then a token walks the factorial loop — taking the yes branch, looping back, and exiting to the end. Node shapes are string kinds; branches are coloured and `annotate`d yes/no/loop, on a clean paper theme with a colour legend.

```manic
{{#include ../../examples/factorial-flowchart.manic}}
```

<div class="manic-video" data-video="ex-factorial-flowchart"></div>

## complex-flowchart

A big pipeline that builds itself, then runs: a 24-node CI/CD flow with 7 decisions and feedback loops auto-wraps into readable side-by-side columns (long loops routed around the perimeter). It first draws box-by-box in flow order, then pairs of commits race the pipeline in parallel to different outcomes — ship, rollback-to-start, held — twice. All `flowchart` + `route` + `par`, no coordinates.

```manic
{{#include ../../examples/complex-flowchart.manic}}
```

<div class="manic-video" data-video="ex-complex-flowchart"></div>

## c4-internet-banking

C4 Level 1 — System Context: the Internet Banking System in its world. A customer (a `person`, drawn as a box with a head), the system itself, and two external systems (e-mail, mainframe), joined by labelled relationships. Outline styling, `[Type]` tags and people-top tiers, all auto-laid with no coordinates.

```manic
{{#include ../../examples/c4-internet-banking.manic}}
```

<div class="manic-video" data-video="ex-c4-internet-banking"></div>

## c4-internet-banking-containers

C4 Level 2 — Containers: zoom inside the system to a single-page app, an API application and a database, each carrying its technology in a `[Container: tech]` tag; the mainframe stays external. Same `c4` container, one level down.

```manic
{{#include ../../examples/c4-internet-banking-containers.manic}}
```

<div class="manic-video" data-video="ex-c4-internet-banking-containers"></div>

## c4-internet-banking-components

C4 Level 3 — Components: inside the API application, sign-in and accounts controllers, a security component and a mainframe facade — declared so related pairs sit adjacent, so the `Uses` and `Reads/writes` edges never cross an intervening box.

```manic
{{#include ../../examples/c4-internet-banking-components.manic}}
```

<div class="manic-video" data-video="ex-c4-internet-banking-components"></div>

## c4-zoom

The C4 differentiator — it moves. One diagram zooms from System Context into its Containers: `zoom` into the centred system, `fade` the surroundings, then reveal the containers — author-composed with `zoom`/`fade`/`show` and `sticky` chrome, no new vocabulary.

```manic
{{#include ../../examples/c4-zoom.manic}}
```

<div class="manic-video" data-video="ex-c4-zoom"></div>

## c4-story

One system, at every altitude — a full end-to-end walkthrough. Context → zoom in → the Containers build along a `GET /accounts` request as it travels browser→API→database → zoom into the API → the Components build along a sign-in call → zoom back out. The flow IS the reveal, led by a moving token.

```manic
{{#include ../../examples/c4-story.manic}}
```

<div class="manic-video" data-video="ex-c4-story"></div>

## c4-test

The canonical bigbank Container diagram, translated straight from the Python `diagrams` library. Five containers auto-split into a balanced grid, and the long notification edge from the e-mail system back to the customer routes around the margin instead of bisecting the diagram — dense C4, still readable.

```manic
{{#include ../../examples/c4-test.manic}}
```

<div class="manic-video" data-video="ex-c4-test"></div>

## systems-rabbitmq-consumers

Authored one-of-many delivery: messages 101/102/103 explicitly select different workers — the kit never infers RabbitMQ or round-robin behaviour.

```manic
{{#include ../../examples/systems-rabbitmq-consumers.manic}}
```

<div class="manic-video" data-video="ex-systems-rabbitmq-consumers"></div>

## aws-three-tier-web-application

Presentation / Application / Data as three responsive tiers; one request travels Route 53 → CloudFront → ELB → ECS → ElastiCache → RDS.

```manic
{{#include ../../examples/aws-three-tier-web-application.manic}}
```

<div class="manic-video" data-video="ex-aws-three-tier-web-application"></div>

## aws-event-processing-clusters-poc

Nested clusters + parallel topology: EKS → three ECS workers → SQS → three Lambda processors → S3 & Redshift, with one seeded hot path through the fan-out.

```manic
{{#include ../../examples/aws-event-processing-clusters-poc.manic}}
```

<div class="manic-video" data-video="ex-aws-event-processing-clusters-poc"></div>

## aws-clustered-web-services

Route 53 → ELB → ECS pool with an RDS primary/replica cluster and ElastiCache; request and response follow separately coloured lanes.

```manic
{{#include ../../examples/aws-clustered-web-services.manic}}
```

<div class="manic-video" data-video="ex-aws-clustered-web-services"></div>

## gcp-clustered-web-services

The same clustered-web story on Google Cloud: Cloud DNS → Load Balancing → GKE pool → Cloud SQL cluster + Memorystore — a pure provider swap.

```manic
{{#include ../../examples/gcp-clustered-web-services.manic}}
```

<div class="manic-video" data-video="ex-gcp-clustered-web-services"></div>

## gcp-message-collecting

GCP IoT: three IoT Core devices publish to Pub/Sub, and Dataflow fans out to a data lake, a processing branch, and a serverless branch — three-level nested clusters.

```manic
{{#include ../../examples/gcp-message-collecting.manic}}
```

<div class="manic-video" data-video="ex-gcp-message-collecting"></div>

## k8s-three-tier

A three-tier app on Kubernetes: Ingress → web Deployment → api Service/Deployment → Redis + a PostgreSQL StatefulSet, following one checkout request.

```manic
{{#include ../../examples/k8s-three-tier.manic}}
```

<div class="manic-video" data-video="ex-k8s-three-tier"></div>

## k8s-stateful-architecture

A StatefulSet's storage: a Service, three pods, their PVCs, and the PV/StorageClass that provision them — generated with a `for` loop, coloured by access/ownership/provisioning.

```manic
{{#include ../../examples/k8s-stateful-architecture.manic}}
```

<div class="manic-video" data-video="ex-k8s-stateful-architecture"></div>

## k8s-cluster-components

The canonical Kubernetes components diagram — control plane (api hub, c-m, c-c-m, etcd, sched) + nodes + the cloud provider — drawn AND flowed with plain primitives, no systems kit at all.

```manic
{{#include ../../examples/k8s-cluster-components.manic}}
```

<div class="manic-video" data-video="ex-k8s-cluster-components"></div>

## serverless-processing

A mixed on-prem + AWS pipeline: Kafka + Docker engines → SQS (+ dead-letter) → Lambda → S3/Redshift, with a Fluentd→Kafka→Spark tap; paths coloured by relationship.

```manic
{{#include ../../examples/serverless-processing.manic}}
```

<div class="manic-video" data-video="ex-serverless-processing"></div>

## onprem-advanced-web-service

The on-prem stress test with native archetypes: Nginx, gRPC, Redis/PostgreSQL HA, Fluentd→Kafka→Spark, and Prometheus/Grafana — three runtime stories on one platform.

```manic
{{#include ../../examples/onprem-advanced-web-service.manic}}
```

<div class="manic-video" data-video="ex-onprem-advanced-web-service"></div>

## onprem-advanced-web-service-v2

The same platform with real provider icons and paths coloured by relationship (request · analytics · telemetry · replication).

```manic
{{#include ../../examples/onprem-advanced-web-service-v2.manic}}
```

<div class="manic-video" data-video="ex-onprem-advanced-web-service-v2"></div>
