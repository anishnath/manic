# Systems Kit — icon reference & aliases

Every Systems-kit node takes a **provider:name** string that selects *artwork only* — it never implies routing, balancing, queueing, or persistence. The catalogue covers **17 providers / 2461 components**, generated from `assets/diagrams/` by `scripts/gen-diagram-manifest.py` (Mingrammer/Diagrams icon set, disk-loaded at render — no binary bloat).

## Reference notation

- `provider:name` — e.g. `node(x, arch, "aws:lambda", "fn")`, `"gcp:bigquery"`, `"onprem:redis"`, `"k8s:pod"`. First matching category wins for the bare name.
- `provider:category/name` — disambiguates when the same name lives in two categories of one provider, e.g. `"aws:network/internet-gateway"` vs `"aws:general/internet-gateway"`.
- Native archetypes need **no** provider: `client`, `service`, `gateway`, `database`, `cache`, `queue`, `storage`, `external` (no cloud assets).

## Friendly aliases

Short, human names resolve to the exact catalogue key — same artwork either way:

| Alias | Resolves to |
|---|---|
| `aws:apigateway` | `aws:api-gateway` |
| `aws:route53` | `aws:route-53` |
| `aws:elb` | `aws:elastic-load-balancing` |
| `aws:load-balancer` | `aws:elastic-load-balancing` |
| `aws:sqs` | `aws:simple-queue-service-sqs` |
| `aws:s3` | `aws:simple-storage-service-s3-bucket` |
| `aws:ecs` | `aws:elastic-container-service` |
| `aws:eks` | `aws:elastic-kubernetes-service` |

Everything else passes through unchanged: `aws:lambda`, `gcp:*`, `azure:*`, `onprem:*`, `k8s:*`, `ibm:*`, `oci:*`, … all resolve directly.

## Colouring paths by relationship

A `connect(id, …)` exposes two colourable parts — the cold dashed line `id` and the hot overlay `id.hot`. Group connections by relationship and colour each group (`color`/`hue`/`glow`/`dashed`/`stroke`) so a dense diagram reads at a glance:

```manic
color(reqPath, cyan);   color(reqPath.hot, cyan);    // REQUEST
color(logPath, gold);   color(logPath.hot, gold);    // ANALYTICS
hue(scrape, 328);       hue(scrape.hot, 328);         // TELEMETRY (pink)
```

## Providers & components

Any listed name works as `provider:name`. Full machine-readable catalogue: `assets/diagrams-manifest.json`.

### `alibabacloud` — 92 components
`alibabacloud:alibabacloud`, `alibabacloud:analytic-db`, `alibabacloud:anti-bot-service`, `alibabacloud:anti-ddos-basic`, `alibabacloud:anti-ddos-pro`, `alibabacloud:antifraud-service`, `alibabacloud:api-gateway`, `alibabacloud:apsaradb-cassandra`, `alibabacloud:apsaradb-hbase`, `alibabacloud:apsaradb-memcache`, `alibabacloud:apsaradb-mongodb`, `alibabacloud:apsaradb-oceanbase`, `alibabacloud:apsaradb-polardb`, `alibabacloud:apsaradb-postgresql` … (+78 more)

### `aws` — 513 components
`aws:ad-connector`, `aws:alexa-for-business`, `aws:amazon-devops-guru`, `aws:amazon-managed-grafana`, `aws:amazon-managed-prometheus`, `aws:amazon-managed-workflows-apache-airflow`, `aws:amazon-opensearch-service`, `aws:amplify`, `aws:analytics`, `aws:apache-mxnet-on-aws`, `aws:api-gateway`, `aws:api-gateway-endpoint`, `aws:app-mesh`, `aws:app-runner` … (+499 more)

### `azure` — 690 components
`azure:aad-licenses`, `azure:abs-member`, `azure:access-review`, `azure:active-directory`, `azure:active-directory-connect-health`, `azure:activity-log`, `azure:ad-b2c`, `azure:ad-domain-services`, `azure:ad-identity-protection`, `azure:ad-privileged-identity-management`, `azure:administrative-units`, `azure:advisor`, `azure:ai-studio`, `azure:aks-istio` … (+676 more)

### `digitalocean` — 26 components
`digitalocean:certificate`, `digitalocean:containers`, `digitalocean:dbaas-primary`, `digitalocean:dbaas-primary-standby-more`, `digitalocean:dbaas-read-only`, `digitalocean:dbaas-standby`, `digitalocean:digitalocean`, `digitalocean:docker`, `digitalocean:domain`, `digitalocean:domain-registration`, `digitalocean:droplet`, `digitalocean:droplet-connect`, `digitalocean:droplet-snapshot`, `digitalocean:firewall` … (+12 more)

### `elastic` — 42 components
`elastic:agent`, `elastic:alerting`, `elastic:apm`, `elastic:app-search`, `elastic:auditbeat`, `elastic:beats`, `elastic:cloud`, `elastic:crawler`, `elastic:ece`, `elastic:eck`, `elastic:elastic`, `elastic:elasticsearch`, `elastic:endpoint`, `elastic:enterprise-search` … (+28 more)

### `firebase` — 22 components
`firebase:ab-testing`, `firebase:app-distribution`, `firebase:app-indexing`, `firebase:authentication`, `firebase:crash-reporting`, `firebase:crashlytics`, `firebase:dynamic-links`, `firebase:extensions`, `firebase:firebase`, `firebase:firestore`, `firebase:functions`, `firebase:hosting`, `firebase:in-app-messaging`, `firebase:invites` … (+8 more)

### `gcp` — 123 components
`gcp:access-context-manager`, `gcp:advanced-solutions-lab`, `gcp:ai-hub`, `gcp:ai-platform`, `gcp:ai-platform-data-labeling-service`, `gcp:api-gateway`, `gcp:apigee`, `gcp:app-engine`, `gcp:armor`, `gcp:assured-workloads`, `gcp:automl`, `gcp:automl-natural-language`, `gcp:automl-tables`, `gcp:automl-translation` … (+109 more)

### `generic` — 27 components
`generic:android`, `generic:blank`, `generic:centos`, `generic:datacenter`, `generic:debian`, `generic:firewall`, `generic:generic`, `generic:ios`, `generic:linux-general`, `generic:mobile`, `generic:qemu`, `generic:rack`, `generic:raspbian`, `generic:red-hat` … (+13 more)

### `gis` — 65 components
`gis:actinia`, `gis:addok`, `gis:ban`, `gis:baremaps`, `gis:cesium`, `gis:deegree`, `gis:g3w-suite`, `gis:gdal`, `gis:geohealthcheck`, `gis:geomapfish`, `gis:geomesa`, `gis:geonetwork`, `gis:geonode`, `gis:geopackage` … (+51 more)

### `ibm` — 164 components
`ibm:actionable-insight`, `ibm:alert-notification`, `ibm:analytics`, `ibm:annotate`, `ibm:api-developer-portal`, `ibm:api-management`, `ibm:api-polyglot-runtimes`, `ibm:api-security`, `ibm:app-server`, `ibm:application-logic`, `ibm:artifact-management`, `ibm:bare-metal-server`, `ibm:block-storage`, `ibm:blockchain` … (+150 more)

### `k8s` — 46 components
`k8s:api`, `k8s:c-c-m`, `k8s:c-m`, `k8s:c-role`, `k8s:chaos-mesh`, `k8s:cm`, `k8s:crb`, `k8s:crd`, `k8s:cronjob`, `k8s:deploy`, `k8s:ds`, `k8s:ep`, `k8s:etcd`, `k8s:external-dns` … (+32 more)

### `oci` — 141 components
`oci:alarm`, `oci:alarm-white`, `oci:api-gateway`, `oci:api-gateway-white`, `oci:api-service`, `oci:api-service-white`, `oci:audit`, `oci:audit-white`, `oci:autonomous`, `oci:autonomous-white`, `oci:autoscale`, `oci:autoscale-white`, `oci:backbone`, `oci:backbone-white` … (+127 more)

### `onprem` — 172 components
`onprem:activemq`, `onprem:aerospike`, `onprem:airflow`, `onprem:ambassador`, `onprem:ansible`, `onprem:apache`, `onprem:argocd`, `onprem:atlantis`, `onprem:awx`, `onprem:beam`, `onprem:bind-9`, `onprem:bitwarden`, `onprem:boundary`, `onprem:buzzfeed-sso` … (+158 more)

### `openstack` — 51 components
`openstack:ansible`, `openstack:barbican`, `openstack:blazar`, `openstack:charms`, `openstack:chef`, `openstack:cinder`, `openstack:cloudkitty`, `openstack:congress`, `openstack:cyborg`, `openstack:designate`, `openstack:ec2api`, `openstack:freezer`, `openstack:glance`, `openstack:heat` … (+37 more)

### `outscale` — 13 components
`outscale:client-vpn`, `outscale:compute`, `outscale:direct-connect`, `outscale:firewall`, `outscale:identity-and-access-management`, `outscale:internet-service`, `outscale:load-balancer`, `outscale:nat-service`, `outscale:net`, `outscale:outscale`, `outscale:simple-storage-service`, `outscale:site-to-site-vpng`, `outscale:storage`

### `programming` — 74 components
`programming:action`, `programming:angular`, `programming:backbone`, `programming:bash`, `programming:c`, `programming:camel`, `programming:collate`, `programming:cpp`, `programming:csharp`, `programming:dapr`, `programming:dart`, `programming:database`, `programming:decision`, `programming:delay` … (+60 more)

### `saas` — 40 components
`saas:adyen`, `saas:akamai`, `saas:amazon-pay`, `saas:auth0`, `saas:cloudflare`, `saas:cloudinary`, `saas:crowdstrike`, `saas:datadog`, `saas:dataform`, `saas:discord`, `saas:facebook`, `saas:fastly`, `saas:imperva`, `saas:intercom` … (+26 more)

