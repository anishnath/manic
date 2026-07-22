//! Systems Kit proof of concept: animated architecture, not static icon boards.
//!
//! Authors declare structure; the kit auto-lays it out and lowers it to ordinary
//! Manic entities. A request keeps one identity while `route` moves it through
//! named connections, so existing `step`, `flow`, captions, and camera verbs
//! remain the storytelling language.

use macroquad::prelude::{Color, Vec2};

use crate::easing::Easing;
use crate::lang::diag::Error;
use crate::lang::lower::{resolve_easing, Args, Registry};
use crate::primitives::{Entity, FontKind, Shape, StrokeStyle};
use crate::scene::Scene;
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TrackSpec, Value};

const NATIVE_NODE_KINDS: &[&str] = &[
    "client", "service", "gateway", "database", "cache", "queue", "storage", "external",
];

#[derive(Debug, Clone, Copy)]
enum NativeNodeIcon {
    Circle,
    Text(&'static str),
}

/// Friendly short kinds → the canonical reference-notation key in the diagram
/// catalog. Service names select **artwork only**: they never add routing,
/// queueing, balancing, broadcast, retry, or persistence semantics. Kinds that
/// already match a catalog key (`aws:lambda`, `gcp:bigquery`, …) pass straight
/// through, so every one of the 17 providers is reachable without a table here.
fn canonical_provider_kind(kind: &str) -> String {
    match kind.to_ascii_lowercase().as_str() {
        "aws:apigateway" => "aws:api-gateway".into(),
        "aws:route53" => "aws:route-53".into(),
        "aws:load-balancer" | "aws:elb" => "aws:elastic-load-balancing".into(),
        "aws:sqs" => "aws:simple-queue-service-sqs".into(),
        "aws:s3" => "aws:simple-storage-service-s3-bucket".into(),
        "aws:ecs" => "aws:elastic-container-service".into(),
        "aws:eks" => "aws:elastic-kubernetes-service".into(),
        other => other.to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct ArchitectureData {
    pub center: Vec2,
    pub width: f32,
    pub height: f32,
    pub horizontal: bool,
    /// Direct ownership children in declaration order. `nodes` remains the
    /// flattened node catalogue used by selectors such as `shop.nodes`.
    pub children: Vec<String>,
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SystemNodeData {
    pub architecture: String,
    pub parent: String,
    pub center: Vec2,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct SystemClusterData {
    pub architecture: String,
    pub parent: String,
    pub label: String,
    pub members: Vec<String>,
    pub center: Vec2,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct ConnectionData {
    pub from: String,
    pub to: String,
    pub path: String,
    pub start: Vec2,
    pub control: Vec2,
    pub end: Vec2,
    /// Every concrete node-to-node lane represented by this semantic
    /// connection. A node-to-cluster or cluster-to-node declaration expands
    /// here, so runtime verbs can choose a truthful physical route.
    pub lanes: Vec<ConnectionLaneData>,
}

#[derive(Debug, Clone)]
pub struct ConnectionLaneData {
    pub connection: String,
    pub from: String,
    pub to: String,
    pub path: String,
    pub hot_path: String,
    pub start: Vec2,
    pub control: Vec2,
    pub end: Vec2,
    /// Sampled motion geometry from the source node centre to the destination
    /// node centre. Curves and orthogonal connectors share this representation,
    /// so `route`/`hotpath` keep constant-speed identity across either style.
    pub motion_points: Vec<Vec2>,
    /// Orthogonal connectors render their terminal arrowhead as a small tagged
    /// companion entity. It lights only as the selected lane reaches its end.
    pub hot_arrow: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Port {
    Auto,
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy)]
enum ConnectionRouting {
    Curved(f32),
    Orthogonal { from: Port, to: Port },
}

fn parse_port(word: &str, args: &Args, index: usize) -> Result<Port, Error> {
    match word.to_ascii_lowercase().as_str() {
        "auto" => Ok(Port::Auto),
        "left" => Ok(Port::Left),
        "right" => Ok(Port::Right),
        "top" => Ok(Port::Top),
        "bottom" => Ok(Port::Bottom),
        _ => Err(Error::new(
            format!("unknown system port `{word}` (try: auto, left, right, top, bottom)"),
            args.span_of(index),
        )),
    }
}

fn connection_routing(args: &Args) -> Result<ConnectionRouting, Error> {
    if args.len() <= 3 {
        return Ok(ConnectionRouting::Curved(0.0));
    }
    if let Ok(bend) = args.num(3) {
        if args.len() > 4 {
            return Err(Error::new(
                "numeric connect bend does not take ports; use `orthogonal, from_port, to_port`",
                args.span_of(4),
            ));
        }
        if !bend.is_finite() {
            return Err(Error::new("connect bend must be finite", args.span_of(3)));
        }
        return Ok(ConnectionRouting::Curved(bend));
    }
    let routing = args.ident(3)?;
    if routing != "orthogonal" {
        return Err(Error::new(
            format!("unknown connect routing `{routing}` (try a numeric bend or `orthogonal`)"),
            args.span_of(3),
        ));
    }
    let from = if args.len() > 4 {
        parse_port(&args.ident(4)?, args, 4)?
    } else {
        Port::Auto
    };
    let to = if args.len() > 5 {
        parse_port(&args.ident(5)?, args, 5)?
    } else {
        Port::Auto
    };
    Ok(ConnectionRouting::Orthogonal { from, to })
}

/// Resolve a provider node kind (`aws:lambda`, `gcp:bigquery`, `onprem:redis`,
/// `k8s:pod`, `aws:network/internet-gateway`, …) to its asset-relative icon path
/// via the generated [`diagram_icons`](super::diagram_icons) catalog. Returns the
/// full `diagrams/...` path (already asset-relative).
fn provider_icon(kind: &str) -> Option<&'static str> {
    let canonical = canonical_provider_kind(kind);
    super::diagram_icons::path(&canonical)
}

fn native_node_icon(kind: &str) -> Option<NativeNodeIcon> {
    match kind.to_ascii_lowercase().as_str() {
        "client" => Some(NativeNodeIcon::Circle),
        "service" => Some(NativeNodeIcon::Text("SVC")),
        "gateway" => Some(NativeNodeIcon::Text("GW")),
        "database" => Some(NativeNodeIcon::Text("DB")),
        "cache" => Some(NativeNodeIcon::Text("C")),
        "queue" => Some(NativeNodeIcon::Text("Q")),
        "storage" => Some(NativeNodeIcon::Text("ST")),
        "external" => Some(NativeNodeIcon::Text("EXT")),
        _ => None,
    }
}

fn ensure_system_id_available(scene: &Scene, id: &str, args: &Args) -> Result<(), Error> {
    let semantic_owner = if scene.architectures.contains_key(id) {
        Some("architecture")
    } else if scene.system_clusters.contains_key(id) {
        Some("cluster")
    } else if scene.system_nodes.contains_key(id) {
        Some("node")
    } else if scene.system_connections.contains_key(id) {
        Some("connection")
    } else if scene.system_message_locations.contains_key(id) {
        Some("message")
    } else if scene.get(id).is_some() {
        Some("entity")
    } else {
        None
    };
    if let Some(owner) = semantic_owner {
        return Err(Error::new(
            format!("system id `{id}` is already used by a {owner}; choose a unique id"),
            args.span_of(0),
        ));
    }
    Ok(())
}

fn ensure_generated_ids_available(
    scene: &Scene,
    ids: impl IntoIterator<Item = String>,
    args: &Args,
) -> Result<(), Error> {
    for generated in ids {
        if scene.get(&generated).is_some() {
            return Err(Error::new(
                format!(
                    "system id `{}` would generate entity `{generated}`, but that entity already exists",
                    args.ident(0)?
                ),
                args.span_of(0),
            ));
        }
    }
    Ok(())
}

fn card_size(horizontal: bool) -> Vec2 {
    if horizontal {
        Vec2::new(150.0, 132.0)
    } else {
        Vec2::new(240.0, 86.0)
    }
}

fn auto_ports(delta: Vec2) -> (Port, Port) {
    if delta.x.abs() >= delta.y.abs() {
        if delta.x >= 0.0 {
            (Port::Right, Port::Left)
        } else {
            (Port::Left, Port::Right)
        }
    } else if delta.y >= 0.0 {
        (Port::Bottom, Port::Top)
    } else {
        (Port::Top, Port::Bottom)
    }
}

fn resolve_ports(from: Port, to: Port, delta: Vec2) -> (Port, Port) {
    let (auto_from, auto_to) = auto_ports(delta);
    (
        if from == Port::Auto { auto_from } else { from },
        if to == Port::Auto { auto_to } else { to },
    )
}

fn port_normal(port: Port) -> Vec2 {
    match port {
        Port::Left => Vec2::new(-1.0, 0.0),
        Port::Right => Vec2::new(1.0, 0.0),
        Port::Top => Vec2::new(0.0, -1.0),
        Port::Bottom => Vec2::new(0.0, 1.0),
        Port::Auto => Vec2::ZERO,
    }
}

fn port_point(center: Vec2, size: Vec2, port: Port) -> Vec2 {
    let half = size * 0.5;
    match port {
        Port::Left => center + Vec2::new(-half.x, 0.0),
        Port::Right => center + Vec2::new(half.x, 0.0),
        Port::Top => center + Vec2::new(0.0, -half.y),
        Port::Bottom => center + Vec2::new(0.0, half.y),
        Port::Auto => center,
    }
}

fn push_distinct(points: &mut Vec<Vec2>, point: Vec2) {
    if points
        .last()
        .is_none_or(|previous| previous.distance(point) > 0.5)
    {
        points.push(point);
    }
}

/// A compact Manhattan route between two resolved ports. Parallel opposing
/// ports share a centred bus; mixed ports get short outward stubs and one
/// deterministic corner. This is geometry only—never obstacle inference.
fn orthogonal_points(start: Vec2, end: Vec2, from: Port, to: Port) -> Vec<Vec2> {
    let from_horizontal = matches!(from, Port::Left | Port::Right);
    let to_horizontal = matches!(to, Port::Left | Port::Right);
    let mut points = vec![start];
    if from_horizontal == to_horizontal {
        if from_horizontal {
            let middle = (start.x + end.x) * 0.5;
            push_distinct(&mut points, Vec2::new(middle, start.y));
            push_distinct(&mut points, Vec2::new(middle, end.y));
        } else {
            let middle = (start.y + end.y) * 0.5;
            push_distinct(&mut points, Vec2::new(start.x, middle));
            push_distinct(&mut points, Vec2::new(end.x, middle));
        }
    } else {
        let gap = 28.0;
        let from_stub = start + port_normal(from) * gap;
        let to_stub = end + port_normal(to) * gap;
        push_distinct(&mut points, from_stub);
        let corner = if from_horizontal {
            Vec2::new(from_stub.x, to_stub.y)
        } else {
            Vec2::new(to_stub.x, from_stub.y)
        };
        push_distinct(&mut points, corner);
        push_distinct(&mut points, to_stub);
    }
    push_distinct(&mut points, end);
    points
}

fn cluster_axis(scene: &Scene, cluster: &SystemClusterData, horizontal: bool) -> bool {
    let only_nodes = !cluster.members.is_empty()
        && cluster
            .members
            .iter()
            .all(|member| scene.system_nodes.contains_key(member));
    if only_nodes && cluster.members.len() > 1 {
        !horizontal
    } else {
        horizontal
    }
}

fn item_size(scene: &Scene, id: &str, horizontal: bool) -> Vec2 {
    if scene.system_nodes.contains_key(id) {
        return card_size(horizontal);
    }
    let Some(cluster) = scene.system_clusters.get(id) else {
        return Vec2::ZERO;
    };
    if cluster.members.is_empty() {
        return if horizontal {
            Vec2::new(220.0, 110.0)
        } else {
            Vec2::new(280.0, 110.0)
        };
    }
    let along_horizontal = cluster_axis(scene, cluster, horizontal);
    let sizes: Vec<Vec2> = cluster
        .members
        .iter()
        .map(|member| item_size(scene, member, horizontal))
        .collect();
    let gap = 18.0;
    let nested = cluster
        .members
        .iter()
        .any(|member| scene.system_clusters.contains_key(member));
    let padding = if nested {
        Vec2::new(18.0, 16.0)
    } else {
        Vec2::new(22.0, 34.0)
    };
    let content = if along_horizontal {
        Vec2::new(
            sizes.iter().map(|size| size.x).sum::<f32>()
                + gap * sizes.len().saturating_sub(1) as f32,
            sizes.iter().map(|size| size.y).fold(0.0, f32::max),
        )
    } else {
        Vec2::new(
            sizes.iter().map(|size| size.x).fold(0.0, f32::max),
            sizes.iter().map(|size| size.y).sum::<f32>()
                + gap * sizes.len().saturating_sub(1) as f32,
        )
    };
    content + padding * 2.0
}

#[derive(Default)]
struct LayoutPlan {
    nodes: Vec<(String, Vec2)>,
    clusters: Vec<(String, Vec2, Vec2)>,
}

fn plan_item(scene: &Scene, id: &str, center: Vec2, horizontal: bool, plan: &mut LayoutPlan) {
    if scene.system_nodes.contains_key(id) {
        plan.nodes.push((id.to_string(), center));
        return;
    }
    let Some(cluster) = scene.system_clusters.get(id) else {
        return;
    };
    let size = item_size(scene, id, horizontal);
    plan.clusters.push((id.to_string(), center, size));
    if cluster.members.is_empty() {
        return;
    }
    let along_horizontal = cluster_axis(scene, cluster, horizontal);
    let sizes: Vec<Vec2> = cluster
        .members
        .iter()
        .map(|member| item_size(scene, member, horizontal))
        .collect();
    let gap = 18.0;
    let total = if along_horizontal {
        sizes.iter().map(|child| child.x).sum::<f32>()
    } else {
        sizes.iter().map(|child| child.y).sum::<f32>()
    } + gap * sizes.len().saturating_sub(1) as f32;
    let mut cursor = -total * 0.5;
    for ((member, child_size), index) in cluster.members.iter().zip(sizes).zip(0..) {
        let along = if along_horizontal {
            child_size.x
        } else {
            child_size.y
        };
        let offset = cursor + along * 0.5;
        let child_center = if along_horizontal {
            center + Vec2::new(offset, 0.0)
        } else {
            center + Vec2::new(0.0, offset)
        };
        plan_item(scene, member, child_center, horizontal, plan);
        cursor += along
            + if index + 1 < cluster.members.len() {
                gap
            } else {
                0.0
            };
    }
}

fn relayout(scene: &mut Scene, architecture: &str) {
    let Some(data) = scene.architectures.get(architecture).cloned() else {
        return;
    };
    if data.children.is_empty() {
        return;
    }
    let sizes: Vec<Vec2> = data
        .children
        .iter()
        .map(|child| item_size(scene, child, data.horizontal))
        .collect();
    let gap = 28.0;
    let total = if data.horizontal {
        sizes.iter().map(|size| size.x).sum::<f32>()
    } else {
        sizes.iter().map(|size| size.y).sum::<f32>()
    } + gap * sizes.len().saturating_sub(1) as f32;
    let mut cursor = -total * 0.5;
    let mut plan = LayoutPlan::default();
    for ((child, size), index) in data.children.iter().zip(sizes).zip(0..) {
        let along = if data.horizontal { size.x } else { size.y };
        let offset = cursor + along * 0.5;
        let center = if data.horizontal {
            data.center + Vec2::new(offset, 0.0)
        } else {
            data.center + Vec2::new(0.0, offset)
        };
        plan_item(scene, child, center, data.horizontal, &mut plan);
        cursor += along
            + if index + 1 < data.children.len() {
                gap
            } else {
                0.0
            };
    }
    for (node, center) in plan.nodes {
        place_node(scene, &node, center, data.horizontal);
    }
    for (cluster_id, center, size) in plan.clusters {
        if let Some(cluster) = scene.system_clusters.get_mut(&cluster_id) {
            cluster.center = center;
            cluster.width = size.x;
            cluster.height = size.y;
        }
        if let Some(frame) = scene.get_mut(&format!("{cluster_id}.frame")) {
            frame.pos = center;
            frame.shape = Shape::Rect {
                w: size.x,
                h: size.y,
            };
        }
        if let Some(label) = scene.get_mut(&format!("{cluster_id}.label")) {
            label.pos = center + Vec2::new(0.0, -size.y * 0.5 + 17.0);
        }
    }
}

fn place_node(scene: &mut Scene, node_id: &str, center: Vec2, horizontal: bool) {
    if let Some(node) = scene.system_nodes.get_mut(node_id) {
        node.center = center;
    }
    let placements = if horizontal {
        [
            (format!("{node_id}.card"), center),
            (format!("{node_id}.icon"), center + Vec2::new(0.0, -18.0)),
            (format!("{node_id}.label"), center + Vec2::new(0.0, 45.0)),
        ]
    } else {
        [
            (format!("{node_id}.card"), center),
            (format!("{node_id}.icon"), center + Vec2::new(-76.0, 0.0)),
            (format!("{node_id}.label"), center + Vec2::new(28.0, 0.0)),
        ]
    };
    for (entity_id, position) in placements {
        if let Some(entity) = scene.get_mut(&entity_id) {
            entity.pos = position;
        }
    }
}

/// `architecture(id, center, width, height)` — responsive system canvas.
fn c_architecture(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(4)?;
    let id = args.ident(0)?;
    ensure_system_id_available(scene, &id, args)?;
    ensure_generated_ids_available(scene, [format!("{id}.frame")], args)?;
    let center = args.pair(1)?;
    let width = args.num(2)?;
    let height = args.num(3)?;
    if !width.is_finite() || !height.is_finite() || width < 280.0 || height < 280.0 {
        return Err(Error::new(
            "architecture width and height must be finite and at least 280",
            args.span_of(2),
        ));
    }
    let horizontal = width >= height;
    let mut frame = Entity::new(
        format!("{id}.frame"),
        Shape::Rect {
            w: width,
            h: height,
        },
        center,
        style::PANEL,
    );
    frame.opacity = 0.45;
    frame.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 1.5,
        outline_color: Some(style::DIM),
    };
    frame.tags.push(id.clone());
    frame.z = -10;
    scene.add(frame);
    scene.architectures.insert(
        id,
        ArchitectureData {
            center,
            width,
            height,
            horizontal,
            children: Vec::new(),
            nodes: Vec::new(),
        },
    );
    Ok(())
}

fn parent_architecture(scene: &Scene, parent: &str) -> Option<String> {
    if scene.architectures.contains_key(parent) {
        Some(parent.to_string())
    } else {
        scene
            .system_clusters
            .get(parent)
            .map(|cluster| cluster.architecture.clone())
    }
}

fn remove_from_parent(scene: &mut Scene, child: &str) {
    for architecture in scene.architectures.values_mut() {
        architecture.children.retain(|item| item != child);
    }
    for cluster in scene.system_clusters.values_mut() {
        cluster.members.retain(|item| item != child);
    }
}

fn add_to_parent(scene: &mut Scene, parent: &str, child: String) {
    if let Some(architecture) = scene.architectures.get_mut(parent) {
        architecture.children.push(child);
    } else if let Some(cluster) = scene.system_clusters.get_mut(parent) {
        cluster.members.push(child);
    }
}

/// `node(id, parent, "kind", "label")` — auto-positioned component. `parent`
/// may be the architecture itself or a previously declared cluster.
fn c_node(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(4)?;
    let id = args.ident(0)?;
    ensure_system_id_available(scene, &id, args)?;
    ensure_generated_ids_available(
        scene,
        [
            format!("{id}.card"),
            format!("{id}.icon"),
            format!("{id}.label"),
        ],
        args,
    )?;
    let parent = args.ident(1)?;
    let kind = args.text(2)?;
    let label = args.text(3)?;
    let architecture = parent_architecture(scene, &parent).ok_or_else(|| {
        Error::new(
            format!("no architecture or cluster named `{parent}`"),
            args.span_of(1),
        )
    })?;
    let data = &scene.architectures[&architecture];
    let horizontal = data.horizontal;
    // A kind is either a native archetype (no colon) or a `provider:name` icon
    // from the diagram catalogue (17 providers). Unresolved kinds error at source.
    let is_provider = kind.contains(':');
    if is_provider && provider_icon(&kind).is_none() {
        return Err(Error::new(
            format!(
                "Systems Kit has no diagram icon for `{kind}`; use a catalogued \
                 `provider:name` (e.g. `aws:lambda`, `gcp:bigquery`, `onprem:redis`, \
                 `k8s:pod`) or `provider:category/name` to disambiguate"
            ),
            args.span_of(2),
        ));
    }
    if !is_provider && native_node_icon(&kind).is_none() {
        return Err(Error::new(
            format!(
                "unknown system node kind `{kind}`; use one of {} or a `provider:name` \
                 icon (aws/gcp/azure/onprem/k8s/ibm/oci/…)",
                NATIVE_NODE_KINDS.join(", ")
            ),
            args.span_of(2),
        ));
    }
    let center = data.center;
    let card = card_size(horizontal);
    let mut panel = Entity::new(
        format!("{id}.card"),
        Shape::Rect {
            w: card.x,
            h: card.y,
        },
        center,
        style::PANEL,
    );
    panel.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 2.0,
        outline_color: Some(style::DIM),
    };
    panel
        .tags
        .extend([id.clone(), format!("{architecture}.nodes")]);
    panel.z = -1;
    scene.add(panel);

    if let Some(relative) = provider_icon(&kind) {
        let uri = format!("asset:{relative}");
        let path = crate::assets::resolve(&uri)
            .map_err(|message| Error::new(message, args.span_of(2)))?
            .to_string_lossy()
            .into_owned();
        let mut icon = Entity::new(
            format!("{id}.icon"),
            Shape::Image {
                path,
                w: 56.0,
                h: 56.0,
                tint: false,
            },
            center,
            Color::new(1.0, 1.0, 1.0, 1.0),
        );
        icon.tags.extend([
            id.clone(),
            format!("{architecture}.nodes"),
            format!("{id}.visual"),
        ]);
        icon.z = 2;
        scene.add(icon);
    } else {
        let native = native_node_icon(&kind).expect("validated native kind");
        let shape = match native {
            NativeNodeIcon::Circle => Shape::Circle { r: 24.0 },
            NativeNodeIcon::Text(label) => Shape::Text {
                content: label.to_string(),
                size: if label.len() > 2 { 18.0 } else { 22.0 },
            },
        };
        let mut icon = Entity::new(format!("{id}.icon"), shape, center, style::CYAN);
        icon.font = FontKind::MonoBold;
        if matches!(native, NativeNodeIcon::Circle) {
            icon.stroke = StrokeStyle {
                fill: false,
                outline: true,
                width: 3.0,
                outline_color: Some(style::CYAN),
            };
        }
        icon.tags.extend([
            id.clone(),
            format!("{architecture}.nodes"),
            format!("{id}.visual"),
        ]);
        icon.z = 2;
        scene.add(icon);
    }

    let mut text = Entity::new(
        format!("{id}.label"),
        Shape::Text {
            content: label,
            size: if horizontal { 16.0 } else { 19.0 },
        },
        center,
        style::FG,
    );
    text.font = FontKind::MonoBold;
    text.tags
        .extend([id.clone(), format!("{architecture}.nodes")]);
    text.z = 3;
    scene.add(text);
    scene.system_nodes.insert(
        id.clone(),
        SystemNodeData {
            architecture: architecture.clone(),
            parent: parent.clone(),
            center,
            kind,
        },
    );
    scene
        .architectures
        .get_mut(&architecture)
        .expect("validated architecture")
        .nodes
        .push(id.clone());
    add_to_parent(scene, &parent, id);
    relayout(scene, &architecture);
    Ok(())
}

fn leaf_nodes(scene: &Scene, id: &str) -> Vec<String> {
    if scene.system_nodes.contains_key(id) {
        return vec![id.to_string()];
    }
    scene
        .system_clusters
        .get(id)
        .map(|cluster| {
            cluster
                .members
                .iter()
                .flat_map(|member| leaf_nodes(scene, member))
                .collect()
        })
        .unwrap_or_default()
}

/// `cluster(id, parent, "label")` — declare ownership before its children.
/// The former fourth-argument member list remains accepted for PoC sources.
fn c_cluster(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(4)?;
    let id = args.ident(0)?;
    ensure_system_id_available(scene, &id, args)?;
    ensure_generated_ids_available(scene, [format!("{id}.frame"), format!("{id}.label")], args)?;
    let parent = args.ident(1)?;
    let label = args.text(2)?;
    let architecture = parent_architecture(scene, &parent).ok_or_else(|| {
        Error::new(
            format!("no architecture or cluster named `{parent}`"),
            args.span_of(1),
        )
    })?;
    let members: Vec<String> = if args.len() > 3 {
        args.text(3)?
            .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == '|')
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect()
    } else {
        Vec::new()
    };
    for (index, member) in members.iter().enumerate() {
        if member == &id || members[..index].contains(member) {
            return Err(Error::new(
                format!("cluster member `{member}` is duplicated or self-referential"),
                args.span_of(3),
            ));
        }
        let member_architecture = scene
            .system_nodes
            .get(member)
            .map(|node| node.architecture.as_str())
            .or_else(|| {
                scene
                    .system_clusters
                    .get(member)
                    .map(|cluster| cluster.architecture.as_str())
            })
            .ok_or_else(|| {
                Error::new(
                    format!("cluster member `{member}` is not a system node or cluster"),
                    args.span_of(3),
                )
            })?;
        if member_architecture != architecture {
            return Err(Error::new(
                format!("cluster member `{member}` belongs to `{member_architecture}`, not `{architecture}`"),
                args.span_of(3),
            ));
        }
    }
    let center = scene
        .system_clusters
        .get(&parent)
        .map(|cluster| cluster.center)
        .unwrap_or(scene.architectures[&architecture].center);
    let size = if scene.architectures[&architecture].horizontal {
        Vec2::new(220.0, 110.0)
    } else {
        Vec2::new(280.0, 110.0)
    };

    let mut frame = Entity::new(
        format!("{id}.frame"),
        Shape::Rect {
            w: size.x,
            h: size.y,
        },
        center,
        style::PANEL,
    );
    frame.opacity = 0.22;
    frame.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 1.5,
        outline_color: Some(style::CYAN),
    };
    frame.tags.extend([
        id.clone(),
        format!("{id}.parts"),
        format!("{architecture}.clusters"),
    ]);
    frame.z = -6;
    scene.add(frame);

    let mut title = Entity::new(
        format!("{id}.label"),
        Shape::Text {
            content: label.clone(),
            size: 14.0,
        },
        center + Vec2::new(0.0, -size.y * 0.5 + 17.0),
        style::CYAN,
    );
    title.font = FontKind::MonoBold;
    title.tags.extend([
        id.clone(),
        format!("{id}.parts"),
        format!("{architecture}.clusters"),
    ]);
    title.z = 4;
    scene.add(title);
    scene.system_clusters.insert(
        id.clone(),
        SystemClusterData {
            architecture: architecture.clone(),
            parent: parent.clone(),
            label,
            members: Vec::new(),
            center,
            width: size.x,
            height: size.y,
        },
    );
    add_to_parent(scene, &parent, id.clone());
    for member in members {
        remove_from_parent(scene, &member);
        if let Some(node) = scene.system_nodes.get_mut(&member) {
            node.parent = id.clone();
        }
        if let Some(cluster) = scene.system_clusters.get_mut(&member) {
            cluster.parent = id.clone();
        }
        scene
            .system_clusters
            .get_mut(&id)
            .expect("cluster inserted")
            .members
            .push(member);
    }
    let nested = scene.system_clusters[&id]
        .members
        .iter()
        .any(|member| scene.system_clusters.contains_key(member));
    if nested {
        if let Some(frame) = scene.get_mut(&format!("{id}.frame")) {
            frame.z = -8;
        }
    }
    relayout(scene, &architecture);
    Ok(())
}

/// `connect(id, from, to, [bend|orthogonal], [from_port], [to_port])` — a stable
/// directed architecture edge. Signed bends and port-aware Manhattan routing
/// are explicit visual choices; neither infers obstacles or provider behavior.
fn c_connect(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(6)?;
    let id = args.ident(0)?;
    ensure_system_id_available(scene, &id, args)?;
    let from = args.ident(1)?;
    let to = args.ident(2)?;
    let routing = connection_routing(args)?;
    let from_nodes = leaf_nodes(scene, &from);
    let to_nodes = leaf_nodes(scene, &to);
    if from_nodes.is_empty() {
        return Err(Error::new(
            format!("no system node or cluster named `{from}`"),
            args.span_of(1),
        ));
    }
    if to_nodes.is_empty() {
        return Err(Error::new(
            format!("no system node or cluster named `{to}`"),
            args.span_of(2),
        ));
    }
    if from == to {
        return Err(Error::new(
            "connect needs two different nodes or clusters",
            args.span_of(2),
        ));
    }
    if from_nodes.len() > 1 && to_nodes.len() > 1 {
        return Err(Error::new(
            "connect between two multi-node clusters is ambiguous; connect through a queue or gateway node",
            args.span_of(2),
        ));
    }
    let pairs: Vec<(String, String)> = if from_nodes.len() == 1 {
        to_nodes
            .iter()
            .map(|to_node| (from_nodes[0].clone(), to_node.clone()))
            .collect()
    } else {
        from_nodes
            .iter()
            .map(|from_node| (from_node.clone(), to_nodes[0].clone()))
            .collect()
    };
    let physical_ids = (0..pairs.len()).flat_map(|index| {
        let path = if pairs.len() == 1 {
            id.clone()
        } else {
            format!("{id}.{index}")
        };
        match routing {
            ConnectionRouting::Curved(_) => vec![path.clone(), format!("{path}.hot")],
            ConnectionRouting::Orthogonal { .. } => vec![
                format!("{path}.body"),
                format!("{path}.hot.body"),
                format!("{path}.arrow"),
                format!("{path}.hot.arrow"),
            ],
        }
    });
    ensure_generated_ids_available(scene, physical_ids, args)?;
    let architecture = scene.system_nodes[&pairs[0].0].architecture.clone();
    let pair_count = pairs.len();
    let mut lanes = Vec::new();
    for (index, (from_node, to_node)) in pairs.into_iter().enumerate() {
        let a = scene.system_nodes[&from_node].center;
        let b = scene.system_nodes[&to_node].center;
        // A single edge keeps the convenient exact id. A fan-out/fan-in uses
        // numbered physical ids for every lane, leaving the public connection
        // id free to resolve as the shared tag. That makes draw/style/flow on
        // the declared connection address the complete lane group.
        let physical_id = if pair_count == 1 {
            id.clone()
        } else {
            format!("{id}.{index}")
        };
        let body_id = if matches!(routing, ConnectionRouting::Orthogonal { .. }) {
            format!("{physical_id}.body")
        } else {
            physical_id.clone()
        };
        let hot_body_id = if matches!(routing, ConnectionRouting::Orthogonal { .. }) {
            format!("{physical_id}.hot.body")
        } else {
            format!("{physical_id}.hot")
        };
        let horizontal = scene.architectures[&architecture].horizontal;
        let size = card_size(horizontal);
        let (control, end, motion_points, mut edge, arrow_id) = match routing {
            ConnectionRouting::Curved(bend) => {
                let direction = (b - a).normalize_or_zero();
                let trim = if (b - a).x.abs() >= (b - a).y.abs() {
                    76.0
                } else {
                    46.0
                };
                let start = a + direction * trim;
                let end = b - direction * trim;
                let delta = end - start;
                let perpendicular = Vec2::new(-delta.y, delta.x).normalize_or_zero();
                let control = (start + end) * 0.5 + perpendicular * bend;
                let route_control = (a + b) * 0.5 + perpendicular * bend;
                let motion_points = (0..=64)
                    .map(|sample| bezier(a, route_control, b, sample as f32 / 64.0))
                    .collect();
                let edge = Entity::new(
                    body_id.clone(),
                    Shape::Curve {
                        ctrl: control,
                        to: end,
                        arrow: true,
                    },
                    start,
                    style::DIM,
                );
                (route_control, end, motion_points, edge, None)
            }
            ConnectionRouting::Orthogonal { from, to } => {
                let (from_port, to_port) = resolve_ports(from, to, b - a);
                let start = port_point(a, size, from_port);
                let end = port_point(b, size, to_port);
                let visual_points = orthogonal_points(start, end, from_port, to_port);
                let mut motion_points = vec![a];
                for point in visual_points.iter().copied() {
                    push_distinct(&mut motion_points, point);
                }
                push_distinct(&mut motion_points, b);
                let control = visual_points
                    .get(visual_points.len() / 2)
                    .copied()
                    .unwrap_or((start + end) * 0.5);
                let edge = Entity::new(
                    body_id.clone(),
                    Shape::Polyline { pts: visual_points },
                    Vec2::ZERO,
                    style::DIM,
                );
                (
                    control,
                    end,
                    motion_points,
                    edge,
                    Some(format!("{physical_id}.arrow")),
                )
            }
        };
        edge.stroke = StrokeStyle {
            fill: false,
            outline: true,
            width: 2.5,
            outline_color: Some(style::DIM),
        };
        // Architecture edges are possibilities until runtime proves one is
        // active. The luminous flow overlay remains solid, so dashed topology
        // and a selected hot path read as two distinct layers.
        edge.dash = Some((12.0, 9.0));
        edge.tags.extend([
            id.clone(),
            format!("{architecture}.connections"),
            format!("{id}.path"),
        ]);
        edge.z = -2;
        let mut hot = edge.clone();
        hot.id = hot_body_id;
        hot.color = style::CYAN;
        hot.stroke.width = 4.0;
        hot.stroke.outline_color = Some(style::CYAN);
        hot.dash = None;
        hot.trace = 0.0;
        hot.flow = 0.0;
        hot.flow_back = 0.0;
        hot.tags = vec![format!("{id}.hot"), format!("{architecture}.hotpaths")];
        hot.z = -1;
        let hot_path = hot.id.clone();
        scene.add(edge);
        scene.add(hot);
        let hot_arrow = if let Some(arrow_id) = arrow_id {
            let direction = motion_points
                .windows(2)
                .rev()
                .find_map(|window| {
                    let delta = window[1] - window[0];
                    (delta.length() > 0.5).then_some(delta.normalize())
                })
                .unwrap_or(Vec2::X);
            let arrow_start = end - direction * 22.0;
            let mut arrow = Entity::new(
                arrow_id.clone(),
                Shape::Arrow { to: end },
                arrow_start,
                style::DIM,
            );
            arrow.stroke = StrokeStyle {
                fill: false,
                outline: true,
                width: 2.5,
                outline_color: Some(style::DIM),
            };
            arrow.dash = Some((12.0, 9.0));
            arrow.tags.extend([
                id.clone(),
                format!("{architecture}.connections"),
                format!("{id}.path"),
            ]);
            arrow.z = -2;
            let mut hot_arrow = arrow.clone();
            hot_arrow.id = format!("{physical_id}.hot.arrow");
            hot_arrow.color = style::CYAN;
            hot_arrow.stroke.width = 4.0;
            hot_arrow.stroke.outline_color = Some(style::CYAN);
            hot_arrow.dash = None;
            hot_arrow.trace = 0.0;
            hot_arrow.tags = vec![format!("{id}.hot"), format!("{architecture}.hotpaths")];
            hot_arrow.z = -1;
            let hot_arrow_id = hot_arrow.id.clone();
            scene.add(arrow);
            scene.add(hot_arrow);
            Some(hot_arrow_id)
        } else {
            None
        };
        lanes.push(ConnectionLaneData {
            connection: id.clone(),
            from: from_node,
            to: to_node,
            path: body_id,
            hot_path,
            start: a,
            control,
            end: b,
            motion_points,
            hot_arrow,
        });
    }
    let representative = lanes.first().expect("at least one pair").clone();
    scene.system_connections.insert(
        id,
        ConnectionData {
            from: representative.from,
            to: representative.to,
            path: representative.path,
            start: representative.start,
            control: representative.control,
            end: representative.end,
            lanes,
        },
    );
    Ok(())
}

/// `message(id, source_node, "label")` — one persistent system token.
/// `request` remains a compatible HTTP-friendly alias.
fn c_request(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(3)?;
    let id = args.ident(0)?;
    ensure_system_id_available(scene, &id, args)?;
    ensure_generated_ids_available(scene, [format!("{id}.label")], args)?;
    let source = args.ident(1)?;
    let label = args.text(2)?;
    let center = scene
        .system_nodes
        .get(&source)
        .ok_or_else(|| Error::new(format!("no system node named `{source}`"), args.span_of(1)))?
        .center;
    let mut token = Entity::new(id.clone(), Shape::Circle { r: 10.0 }, center, style::GOLD);
    token.stroke = StrokeStyle {
        fill: true,
        outline: true,
        width: 2.0,
        outline_color: Some(style::FG),
    };
    token
        .tags
        .extend([format!("{id}.token"), format!("{id}.parts")]);
    token.z = 12;
    scene.add(token);
    let mut text = Entity::new(
        format!("{id}.label"),
        Shape::Text {
            content: label,
            size: 14.0,
        },
        center + Vec2::new(0.0, -24.0),
        style::GOLD,
    );
    text.font = FontKind::MonoBold;
    text.follow = Some((id.clone(), Vec2::new(0.0, -24.0)));
    text.tags.extend([id.clone(), format!("{id}.parts")]);
    text.z = 13;
    scene.add(text);
    scene.system_message_locations.insert(id, source);
    Ok(())
}

fn bezier(a: Vec2, c: Vec2, b: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    a * (u * u) + c * (2.0 * u * t) + b * (t * t)
}

fn path_position(points: &[Vec2], t: f32) -> Vec2 {
    let Some(first) = points.first().copied() else {
        return Vec2::ZERO;
    };
    if points.len() == 1 {
        return first;
    }
    let lengths: Vec<f32> = points
        .windows(2)
        .map(|window| window[0].distance(window[1]))
        .collect();
    let total: f32 = lengths.iter().sum();
    if total <= 0.001 {
        return *points.last().unwrap_or(&first);
    }
    let mut target = t.clamp(0.0, 1.0) * total;
    for (index, length) in lengths.iter().copied().enumerate() {
        if target <= length || index + 1 == lengths.len() {
            let local = if length <= 0.001 {
                1.0
            } else {
                target / length
            };
            return points[index].lerp(points[index + 1], local.clamp(0.0, 1.0));
        }
        target -= length;
    }
    *points.last().unwrap_or(&first)
}

/// `route(request, connection, [duration], [ease])` — preserve request identity.
fn v_route(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(4)?;
    let request = args.ident(0)?;
    let connection = args.ident(1)?;
    if scene.get(&request).is_none() {
        return Err(Error::new(
            format!("no request named `{request}`"),
            args.span_of(0),
        ));
    }
    let connection_data = scene
        .system_connections
        .get(&connection)
        .cloned()
        .ok_or_else(|| {
            Error::new(
                format!("no system connection named `{connection}`"),
                args.span_of(1),
            )
        })?;
    let current = scene
        .system_message_locations
        .get(&request)
        .cloned()
        .ok_or_else(|| {
            Error::new(
                format!("`{request}` is not a Systems message; create it with message(...) or request(...)"),
                args.span_of(0),
            )
        })?;
    let edge = connection_data
        .lanes
        .iter()
        .find(|lane| lane.from == current)
        .cloned()
        .ok_or_else(|| {
            let mut starts: Vec<&str> = connection_data
                .lanes
                .iter()
                .map(|lane| lane.from.as_str())
                .collect();
            starts.sort_unstable();
            starts.dedup();
            let expectation = if starts.len() == 1 {
                format!("`{connection}` starts at `{}`", starts[0])
            } else {
                format!(
                    "`{connection}` starts at one of {}",
                    starts
                        .iter()
                        .map(|start| format!("`{start}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            Error::new(
                format!(
                    "`{request}` is currently at `{current}`, but {expectation}; route through a connecting edge"
                ),
                args.span_of(1),
            )
        })?;
    let duration = args.opt_num(2)?.unwrap_or(1.0);
    if !duration.is_finite() || duration <= 0.0 {
        return Err(Error::new(
            "route duration must be positive and finite",
            args.span_of(2),
        ));
    }
    let easing = if args.len() > 3 {
        resolve_easing(&args.ident(3)?, args.span_of(3))?
    } else {
        Easing::InOutCubic
    };
    let authored_start = scene
        .motion_pos
        .get(&request)
        .copied()
        .unwrap_or_else(|| scene.get(&request).expect("validated request").pos);
    let motion_points = if edge
        .motion_points
        .first()
        .is_some_and(|start| authored_start.distance(*start) < 2.0)
    {
        edge.motion_points.clone()
    } else {
        vec![authored_start, edge.end]
    };
    let segments = 32;
    let mut tracks: Vec<TrackSpec> = (1..=segments)
        .map(|index| {
            let t = easing.apply(index as f32 / segments as f32);
            TrackSpec {
                id: request.clone(),
                prop: Prop::Pos,
                target: TargetValue::Abs(Value::V(path_position(&motion_points, t))),
                start: duration * (index - 1) as f32 / segments as f32,
                dur: duration / segments as f32,
                easing: Easing::Linear,
            }
        })
        .collect();
    tracks.push(TrackSpec {
        id: edge.path.clone(),
        prop: Prop::Flow,
        target: TargetValue::Rel(Value::F(1.0)),
        start: 0.0,
        dur: duration,
        easing: Easing::Linear,
    });
    if let Some(hot_arrow) = edge.hot_arrow {
        tracks.push(TrackSpec {
            id: hot_arrow,
            prop: Prop::Trace,
            target: TargetValue::Abs(Value::F(1.0)),
            start: duration * 0.82,
            dur: duration * 0.18,
            easing: Easing::Linear,
        });
    }
    tracks.push(TrackSpec {
        id: edge.hot_path.clone(),
        prop: Prop::Trace,
        target: TargetValue::Abs(Value::F(1.0)),
        start: 0.0,
        dur: duration,
        easing: Easing::Linear,
    });
    scene.motion_pos.insert(request.clone(), edge.end);
    scene.system_message_locations.insert(request, edge.to);
    Ok(Clip {
        tracks,
        events: Vec::new(),
        dur: duration,
    })
}

fn lane_length(lane: &ConnectionLaneData) -> f32 {
    lane.motion_points
        .windows(2)
        .map(|window| window[0].distance(window[1]))
        .sum::<f32>()
        .max(1.0)
}

fn next_random(state: &mut u64) -> u64 {
    // Xorshift64*: small, deterministic, and identical on native and WASM.
    // Zero is remapped because it is the generator's absorbing state.
    if *state == 0 {
        *state = 0x9E37_79B9_7F4A_7C15;
    }
    *state ^= *state >> 12;
    *state ^= *state << 25;
    *state ^= *state >> 27;
    state.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

/// `hotpath(message, [duration], [seed])` — infer one complete valid route
/// from the message's current node to a reachable sink. At every fan-out a
/// seeded choice selects one physical lane; the same message then moves over
/// the chosen lanes without pauses while only those lanes illuminate.
fn v_hotpath(scene: &mut Scene, args: &Args) -> Result<Clip, Error> {
    args.max(3)?;
    let message = args.ident(0)?;
    if scene.get(&message).is_none() {
        return Err(Error::new(
            format!("no message named `{message}`"),
            args.span_of(0),
        ));
    }
    let mut current = scene
        .system_message_locations
        .get(&message)
        .cloned()
        .ok_or_else(|| {
            Error::new(
                format!(
                    "`{message}` is not a Systems message; create it with message(...) or request(...)"
                ),
                args.span_of(0),
            )
        })?;
    let architecture = scene.system_nodes[&current].architecture.clone();
    let duration = args.opt_num(1)?.unwrap_or(5.0);
    if !duration.is_finite() || duration <= 0.0 {
        return Err(Error::new(
            "hotpath duration must be positive and finite",
            args.span_of(1),
        ));
    }
    let seed_value = args.opt_num(2)?.unwrap_or(1.0);
    if !seed_value.is_finite() || seed_value < 0.0 {
        return Err(Error::new(
            "hotpath seed must be a finite non-negative number",
            args.span_of(2),
        ));
    }
    let mut random_state = seed_value.round() as u64;
    let mut used_paths = std::collections::HashSet::new();
    let mut visited_nodes = std::collections::HashSet::from([current.clone()]);
    let mut chosen = Vec::new();
    let maximum_hops = scene.system_nodes.len().max(1) * 2;

    for _ in 0..maximum_hops {
        let outgoing: Vec<ConnectionLaneData> = scene
            .system_connections
            .values()
            .flat_map(|connection| connection.lanes.iter())
            .filter(|lane| {
                lane.from == current
                    && scene.system_nodes[&lane.from].architecture == architecture
                    && !used_paths.contains(&lane.path)
            })
            .cloned()
            .collect();
        if outgoing.is_empty() {
            break;
        }
        let mut outgoing: Vec<ConnectionLaneData> = outgoing
            .into_iter()
            .filter(|lane| !visited_nodes.contains(&lane.to))
            .collect();
        if outgoing.is_empty() {
            return Err(Error::new(
                "hotpath reached a cycle; use explicit route(...) for retry or cyclic stories",
                args.span_of(0),
            ));
        }
        outgoing.sort_by(|a, b| a.path.cmp(&b.path));
        let index = (next_random(&mut random_state) as usize) % outgoing.len();
        let lane = outgoing.swap_remove(index);
        used_paths.insert(lane.path.clone());
        current = lane.to.clone();
        visited_nodes.insert(current.clone());
        chosen.push(lane);
    }

    if chosen.is_empty() {
        return Err(Error::new(
            format!("`{message}` is already at sink node `{current}`; hotpath needs a reachable outgoing connection"),
            args.span_of(0),
        ));
    }

    let total_length: f32 = chosen.iter().map(lane_length).sum();
    let mut elapsed = 0.0;
    let mut tracks = Vec::new();
    for (lane_index, lane) in chosen.iter().enumerate() {
        let lane_duration = if lane_index + 1 == chosen.len() {
            duration - elapsed
        } else {
            duration * lane_length(lane) / total_length
        };
        let segments = 32;
        for index in 1..=segments {
            let t = index as f32 / segments as f32;
            tracks.push(TrackSpec {
                id: message.clone(),
                prop: Prop::Pos,
                target: TargetValue::Abs(Value::V(path_position(&lane.motion_points, t))),
                start: elapsed + lane_duration * (index - 1) as f32 / segments as f32,
                dur: lane_duration / segments as f32,
                easing: Easing::Linear,
            });
        }
        tracks.push(TrackSpec {
            id: lane.path.clone(),
            prop: Prop::Flow,
            target: TargetValue::Rel(Value::F(1.0)),
            start: elapsed,
            dur: lane_duration,
            easing: Easing::Linear,
        });
        tracks.push(TrackSpec {
            id: lane.hot_path.clone(),
            prop: Prop::Trace,
            target: TargetValue::Abs(Value::F(1.0)),
            start: elapsed,
            dur: lane_duration,
            easing: Easing::Linear,
        });
        if let Some(hot_arrow) = &lane.hot_arrow {
            tracks.push(TrackSpec {
                id: hot_arrow.clone(),
                prop: Prop::Trace,
                target: TargetValue::Abs(Value::F(1.0)),
                start: elapsed + lane_duration * 0.82,
                dur: lane_duration * 0.18,
                easing: Easing::Linear,
            });
        }
        elapsed += lane_duration;
    }
    let destination = chosen.last().expect("non-empty path").to.clone();
    let end = chosen.last().expect("non-empty path").end;
    scene.motion_pos.insert(message.clone(), end);
    scene.system_message_locations.insert(message, destination);
    Ok(Clip {
        tracks,
        events: Vec::new(),
        dur: duration,
    })
}

pub fn register(registry: &mut Registry) {
    registry.ctor("architecture", c_architecture);
    registry.ctor("node", c_node);
    registry.ctor("cluster", c_cluster);
    registry.ctor("connect", c_connect);
    registry.ctor("message", c_request);
    registry.ctor("request", c_request);
    registry.mut_verb("route", v_route);
    registry.mut_verb("hotpath", v_hotpath);
}

#[cfg(test)]
mod tests {
    #[test]
    fn provider_neutral_node_archetypes_need_no_cloud_assets_or_semantics() {
        let kinds = [
            "client", "service", "gateway", "database", "cache", "queue", "storage", "external",
        ];
        let mut source = String::from("canvas(1600,900); architecture(system,(800,450),1450,560);");
        for (index, kind) in kinds.iter().enumerate() {
            source.push_str(&format!("node(n{index},system,\"{kind}\",\"{kind}\");"));
        }
        let movie = crate::parse(&source).expect("native system kinds should compile");
        assert_eq!(movie.scene.system_nodes.len(), kinds.len());
        for index in 0..kinds.len() {
            let icon = movie
                .scene
                .get(&format!("n{index}.icon"))
                .expect("every native node has a deterministic icon");
            assert!(
                !matches!(&icon.shape, crate::primitives::Shape::Image { .. }),
                "native archetypes must not depend on a provider asset"
            );
        }
    }

    #[test]
    fn provider_manifest_resolves_every_canonical_aws_asset_and_alias() {
        // the friendly AWS kinds authors use must all resolve to a packaged icon.
        for kind in [
            "aws:cloudfront", "aws:api-gateway", "aws:lambda", "aws:dynamodb",
            "aws:sqs", "aws:route53", "aws:s3", "aws:elb", "aws:ecs", "aws:eks",
            "aws:fargate", "aws:redshift", "aws:rds", "aws:elasticache",
        ] {
            let relative =
                super::provider_icon(kind).unwrap_or_else(|| panic!("kind `{kind}` must resolve"));
            let uri = format!("asset:{relative}");
            assert!(
                crate::assets::resolve(&uri).is_ok(),
                "provider icon `{kind}` must be packaged: {uri}"
            );
        }
        // aliases resolve to the same artwork as their canonical key.
        assert_eq!(super::provider_icon("aws:apigateway"), super::provider_icon("aws:api-gateway"));
        assert_eq!(super::provider_icon("aws:route53"), super::provider_icon("aws:route-53"));
        assert_eq!(super::provider_icon("aws:load-balancer"), super::provider_icon("aws:elb"));
        // the catalog now spans every provider, not just AWS.
        for kind in ["gcp:bigquery", "onprem:postgresql", "k8s:pod", "azure:app-services"] {
            if let Some(rel) = super::provider_icon(kind) {
                assert!(crate::assets::resolve(&format!("asset:{rel}")).is_ok());
            }
        }
    }

    #[test]
    fn structural_id_collisions_are_source_errors_instead_of_scene_panics() {
        for source in [
            "architecture(a,(400,400),500,500); architecture(a,(400,400),500,500);",
            "architecture(a,(400,400),500,500); node(x,a,\"service\",\"One\"); node(x,a,\"service\",\"Two\");",
            "architecture(a,(400,400),500,500); rect(x.card,(50,50),20,20); node(x,a,\"service\",\"One\");",
            "architecture(a,(400,400),500,500); node(x,a,\"service\",\"One\"); node(y,a,\"database\",\"Two\"); connect(path,x,y); connect(path,x,y);",
            "architecture(a,(400,400),500,500); node(x,a,\"service\",\"One\"); message(m,x,\"M\"); message(m,x,\"M2\");",
        ] {
            let error = match crate::parse(source) {
                Ok(_) => panic!("duplicate system ids must fail cleanly"),
                Err(error) => error,
            };
            assert!(
                error.to_string().contains("already"),
                "diagnostic should name the collision: {error}"
            );
        }
    }

    #[test]
    fn aws_architecture_auto_layouts_and_routes_one_persistent_request() {
        let source = r#"
            canvas(1280,720);
            architecture(shop,(640,360),1050,360);
            node(browser,shop,"client","Browser");
            node(edge,shop,"aws:cloudfront","CloudFront");
            node(api,shop,"aws:api-gateway","API Gateway");
            node(fn1,shop,"aws:lambda","Checkout");
            connect(a,browser,edge); connect(b,edge,api); connect(c,api,fn1);
            request(order,browser,"BUY");
            step("journey") { seq { route(order,a,0.3); route(order,b,0.3); route(order,c,0.3); } }
        "#;
        let movie = crate::parse(source).expect("Systems PoC should compile");
        assert_eq!(movie.scene.architectures["shop"].nodes.len(), 4);
        assert_eq!(movie.scene.system_connections.len(), 3);
        assert!(movie.scene.get("edge.icon").is_some());
        assert_eq!(
            movie.scene.motion_pos["order"],
            movie.scene.system_nodes["fn1"].center
        );
    }

    #[test]
    fn orthogonal_connect_is_one_port_aware_routable_identity() {
        let source = r#"
            canvas(1280,720);
            architecture(a,(640,360),1000,360);
            node(left,a,"client","Left");
            node(right,a,"service","Right");
            connect(bus,left,right,orthogonal,right,top);
            message(packet,left,"GET");
            step("move") { route(packet,bus,1,linear); }
        "#;
        let movie = crate::parse(source).expect("orthogonal connector should compile");
        assert!(
            movie.base().get("bus").is_none(),
            "the public id must remain a complete connector tag"
        );
        assert!(matches!(
            movie.base().get("bus.body").map(|entity| &entity.shape),
            Some(crate::primitives::Shape::Polyline { .. })
        ));
        assert!(movie.base().get("bus.arrow").is_some());
        assert!(movie.base().get("bus.hot.body").is_some());
        assert!(movie.base().get("bus.hot.arrow").is_some());

        let lane = &movie.base().system_connections["bus"].lanes[0];
        assert!(
            lane.motion_points.len() >= 5,
            "mixed ports need visible stubs and a corner"
        );
        assert_eq!(
            lane.motion_points[0],
            movie.base().system_nodes["left"].center
        );
        assert_eq!(
            *lane.motion_points.last().expect("destination"),
            movie.base().system_nodes["right"].center
        );
        let destination = movie.base().system_nodes["right"].center;
        assert!(
            lane.motion_points[3].x <= destination.x - super::card_size(true).x * 0.5 + 0.5,
            "right-to-top corner must remain on or outside the destination card"
        );
        assert!(
            lane.motion_points[4].y < destination.y - super::card_size(true).y * 0.5,
            "destination stub must approach the top port from outside"
        );

        let (base, timeline) = movie.finalize();
        let halfway = timeline.apply(&base, 0.5);
        let halfway_pos = halfway.get("packet").expect("packet").pos;
        assert_ne!(halfway_pos, movie.base().system_nodes["left"].center);
        assert_ne!(halfway_pos, movie.base().system_nodes["right"].center);
        let finished = timeline.apply(&base, 1.0);
        assert_eq!(
            finished.get("packet").expect("packet").pos,
            movie.base().system_nodes["right"].center
        );
        assert_eq!(finished.get("bus.hot.arrow").expect("hot arrow").trace, 1.0);
    }

    #[test]
    fn orthogonal_connect_reports_invalid_ports_without_panicking() {
        let source = r#"
            canvas(800,500); architecture(a,(400,250),650,280);
            node(x,a,"client","X"); node(y,a,"service","Y");
            connect(path,x,y,orthogonal,inside,left);
        "#;
        let error = match crate::parse(source) {
            Ok(_) => panic!("invalid port must fail"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("unknown system port `inside`"));
    }

    #[test]
    fn unknown_aws_kind_fails_with_catalog_help() {
        let source =
            r#"canvas(800,800); architecture(a,(400,400),500,500); node(x,a,"aws:not-real","X");"#;
        let error = match crate::parse(source) {
            Ok(_) => panic!("unknown kind must fail"),
            Err(error) => error,
        };
        let msg = error.to_string();
        assert!(
            msg.contains("no diagram icon for `aws:not-real`") && msg.contains("provider:name"),
            "diagnostic should name the missing kind and point to provider:name: {msg}"
        );
    }

    #[test]
    fn nested_clusters_expand_fan_out_and_keep_one_route_representative() {
        let source = r#"
            canvas(1280,720);
            architecture(events,(640,360),1120,500);
            node(source,events,"aws:eks","Source");
            cluster(flows,events,"EVENT FLOWS");
            cluster(workers,flows,"EVENT WORKERS");
            node(w1,workers,"aws:ecs","Worker 1");
            node(w2,workers,"aws:ecs","Worker 2");
            node(w3,workers,"aws:ecs","Worker 3");
            node(queue,flows,"aws:sqs","Queue");
            cluster(processing,flows,"PROCESSING");
            node(p1,processing,"aws:lambda","Processor 1");
            node(p2,processing,"aws:lambda","Processor 2");
            node(p3,processing,"aws:lambda","Processor 3");
            node(store,events,"aws:s3","Store");
            node(dw,events,"aws:redshift","Analytics");
            connect(toWorkers,source,workers);
            connect(toQueue,workers,queue);
            connect(toHandlers,queue,processing);
            connect(toStore,processing,store);
            message(event,source,"EVENT");
            untraced(toWorkers);
            step("move") { route(event,toWorkers,0.5); }
        "#;
        let movie = crate::parse(source).expect("nested Systems PoC should compile");
        assert_eq!(movie.scene.system_clusters.len(), 3);
        for connection in ["toWorkers", "toQueue", "toHandlers", "toStore"] {
            let physical_edges = movie
                .scene
                .entities
                .iter()
                .filter(|entity| entity.tags.iter().any(|tag| tag == connection))
                .count();
            assert_eq!(physical_edges, 3, "{connection} should expand three paths");
        }
        assert_eq!(
            movie.scene.motion_pos["event"], movie.scene.system_nodes["w1"].center,
            "route should use the first cluster leaf as its deterministic representative"
        );
        assert!(movie
            .scene
            .entities
            .iter()
            .filter(|entity| entity.tags.iter().any(|tag| tag == "toWorkers"))
            .all(|entity| entity.trace == 0.0));
        let (base, timeline) = movie.finalize();
        let active = timeline.apply(&base, 0.25);
        assert!(active.get("toWorkers.0").expect("chosen lane").flow > 0.0);
        assert_eq!(active.get("toWorkers.1").expect("idle lane").flow, 0.0);
        assert_eq!(active.get("toWorkers.2").expect("idle lane").flow, 0.0);
    }

    #[test]
    fn route_rejects_discontinuous_message_stories() {
        let source = r#"
            canvas(900,600); architecture(a,(450,300),760,360);
            node(x,a,"client","X"); node(y,a,"client","Y"); node(z,a,"client","Z");
            connect(xy,x,y); connect(xz,x,z); message(m,x,"M");
            step("bad") { seq { route(m,xy,0.2); route(m,xz,0.2); } }
        "#;
        let error = match crate::parse(source) {
            Ok(_) => panic!("a message must not teleport between disconnected routes"),
            Err(error) => error.to_string(),
        };
        assert!(error.contains("currently at `y`"));
        assert!(error.contains("`xz` starts at `x`"));
    }

    #[test]
    fn legacy_cluster_member_list_remains_accepted() {
        let source = r#"
            canvas(900,600); architecture(a,(450,300),760,360);
            node(x,a,"client","X"); node(y,a,"client","Y");
            cluster(pair,a,"PAIR","x y");
        "#;
        let movie = crate::parse(source).expect("legacy cluster syntax should remain compatible");
        assert_eq!(
            movie.scene.system_clusters["pair"].members,
            vec!["x".to_string(), "y".to_string()]
        );
    }

    #[test]
    fn hotpath_chooses_a_repeatable_valid_route_to_a_sink() {
        let source = r#"
            canvas(1280,720); architecture(events,(640,360),1120,500);
            node(source,events,"aws:eks","Source");
            cluster(workers,events,"WORKERS");
            node(w1,workers,"aws:ecs","Worker 1");
            node(w2,workers,"aws:ecs","Worker 2");
            node(queue,events,"aws:sqs","Queue");
            cluster(processors,events,"PROCESSORS");
            node(p1,processors,"aws:lambda","Processor 1");
            node(p2,processors,"aws:lambda","Processor 2");
            node(store,events,"aws:s3","Store");
            node(dw,events,"aws:redshift","Analytics");
            connect(a,source,workers); connect(b,workers,queue);
            connect(c,queue,processors); connect(d,processors,store);
            connect(toDw,processors,dw); message(event,source,"EVENT");
            step("runtime") { hotpath(event,4,27); }
        "#;
        let first = crate::parse(source).expect("hotpath story should compile");
        let second = crate::parse(source).expect("the same seed should compile identically");
        let destination = &first.scene.system_message_locations["event"];
        assert!(destination == "store" || destination == "dw");
        assert_eq!(destination, &second.scene.system_message_locations["event"]);
        assert_eq!(
            first.scene.motion_pos["event"],
            second.scene.motion_pos["event"]
        );
        assert!(first
            .scene
            .entities
            .iter()
            .filter(|entity| entity.tags.iter().any(|tag| tag == "events.connections"))
            .all(|entity| entity.dash == Some((12.0, 9.0))));
    }

    #[test]
    fn route_selects_the_lane_that_begins_at_the_messages_current_node() {
        let source = r#"
            canvas(900,600); architecture(a,(450,300),760,360);
            cluster(workers,a,"WORKERS");
            node(w1,workers,"client","W1"); node(w2,workers,"client","W2");
            node(q,a,"client","Q"); connect(fanin,workers,q); message(m,w2,"M");
            step("route") { route(m,fanin,0.5); }
        "#;
        let movie = crate::parse(source).expect("fan-in route should select the matching lane");
        assert_eq!(movie.scene.system_message_locations["m"], "q");
        let (base, timeline) = movie.finalize();
        let active = timeline.apply(&base, 0.25);
        assert_eq!(active.get("fanin.0").expect("first lane").flow, 0.0);
        assert!(active.get("fanin.1").expect("matching lane").flow > 0.0);
    }

    #[test]
    fn connect_accepts_an_explicit_visual_bend_and_route_follows_it() {
        let source = r#"
            canvas(900,600); architecture(a,(450,300),760,360);
            node(x,a,"client","X"); node(y,a,"client","Y");
            connect(curved,x,y,120); message(m,x,"M");
            step("route") { route(m,curved,0.5,smooth); }
        "#;
        let movie = crate::parse(source).expect("creator-authored bend should compile");
        let connection = &movie.scene.system_connections["curved"];
        let midpoint = (connection.start + connection.end) * 0.5;
        assert!(
            (connection.control - midpoint).length() > 100.0,
            "route metadata should preserve the authored curve"
        );
        let edge = movie.scene.get("curved").expect("physical edge");
        let crate::primitives::Shape::Curve { ctrl, .. } = edge.shape else {
            panic!("architecture connection should be a curve")
        };
        assert!((ctrl - midpoint).length() > 40.0);
        assert!(movie
            .scene
            .get("curved.hot")
            .expect("hot overlay")
            .tags
            .iter()
            .any(|tag| tag == "curved.hot"));
    }
}
