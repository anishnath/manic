# Diagrams

Animated diagrams — not another static boxes-and-arrows generator. Declare a bounded architecture with auto-positioned nodes and directed connections, then move one persistent request through the graph. Node artwork comes from a string kind (`aws:lambda`, `gcp:bigquery`, `onprem:redis`, `k8s:pod` — 17 providers, see [icon reference & aliases](systems-icons.md)); paths are coloured by relationship. The kit never infers behaviour — the creator authors it with `route`, `flow`, and ordinary verbs.

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
