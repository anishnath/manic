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

/// Flowchart node shapes, selected by string `kind` on the ordinary `node`
/// builtin (never a builtin-per-shape). Each is the node *body* itself — the
/// label sits centred inside and there is no provider icon — so the standard
/// flowchart vocabulary is one authoring form the creator already knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlowShape {
    /// `process` — a plain rectangle (a step/action).
    Process,
    /// `decision` — a diamond (a yes/no branch).
    Decision,
    /// `terminator` — a stadium/pill (start / end).
    Terminator,
    /// `io` — a parallelogram (input / output).
    Io,
    /// `subprocess` — a rectangle with struck side rails (a predefined process).
    Subprocess,
    /// `connector` — a small circle (an on-page join).
    Connector,
}

const FLOW_SHAPE_KINDS: &[&str] = &[
    "process",
    "decision",
    "terminator",
    "io",
    "subprocess",
    "connector",
];

fn flow_shape(kind: &str) -> Option<FlowShape> {
    match kind.to_ascii_lowercase().as_str() {
        "process" => Some(FlowShape::Process),
        "decision" => Some(FlowShape::Decision),
        "terminator" => Some(FlowShape::Terminator),
        "io" => Some(FlowShape::Io),
        "subprocess" => Some(FlowShape::Subprocess),
        "connector" => Some(FlowShape::Connector),
        _ => None,
    }
}

/// Natural (unscaled) body size per flowchart shape. Diamonds get extra height
/// so a decision's question fits; a connector is a small dot.
fn flow_card_size(shape: FlowShape) -> Vec2 {
    match shape {
        FlowShape::Process | FlowShape::Subprocess => Vec2::new(178.0, 66.0),
        FlowShape::Io => Vec2::new(188.0, 66.0),
        FlowShape::Terminator => Vec2::new(156.0, 58.0),
        FlowShape::Decision => Vec2::new(184.0, 112.0),
        FlowShape::Connector => Vec2::new(50.0, 50.0),
    }
}

/// The body [`Shape`] for a flowchart node at a given size, centred on `pos`
/// (polygon points are offsets from `pos`). Returns the corner radius the entity
/// should carry (non-zero only for the pill terminator).
fn flow_body_shape(shape: FlowShape, size: Vec2) -> (Shape, f32) {
    let (hw, hh) = (size.x * 0.5, size.y * 0.5);
    match shape {
        FlowShape::Process | FlowShape::Subprocess => {
            (Shape::Rect { w: size.x, h: size.y }, 0.0)
        }
        FlowShape::Terminator => (Shape::Rect { w: size.x, h: size.y }, hh),
        FlowShape::Connector => (Shape::Circle { r: hw.min(hh) }, 0.0),
        FlowShape::Decision => (
            Shape::Polygon {
                pts: vec![
                    Vec2::new(0.0, -hh),
                    Vec2::new(hw, 0.0),
                    Vec2::new(0.0, hh),
                    Vec2::new(-hw, 0.0),
                ],
            },
            0.0,
        ),
        FlowShape::Io => {
            let skew = hh * 0.85;
            (
                Shape::Polygon {
                    pts: vec![
                        Vec2::new(-hw + skew, -hh),
                        Vec2::new(hw, -hh),
                        Vec2::new(hw - skew, hh),
                        Vec2::new(-hw, hh),
                    ],
                },
                0.0,
            )
        }
    }
}

/// C4-model element kinds, selected by string `kind` on `node` **inside a `c4`
/// container** (so `external` reads as a C4 external system here, and the native
/// archetype elsewhere — no prefix needed). Each renders as a filled rounded box
/// carrying a name, a `[Type: technology]` tag, and a description.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum C4Kind {
    Person,
    System,
    Container,
    Component,
    External,
}

const C4_KINDS: &[&str] = &["person", "system", "container", "component", "external"];

fn c4_kind(kind: &str) -> Option<C4Kind> {
    match kind.to_ascii_lowercase().as_str() {
        "person" => Some(C4Kind::Person),
        "system" => Some(C4Kind::System),
        "container" => Some(C4Kind::Container),
        "component" => Some(C4Kind::Component),
        "external" => Some(C4Kind::External),
        _ => None,
    }
}

/// Natural (unscaled) C4 box size — big enough for name + type tag + a wrapped
/// description line.
fn c4_box_size() -> Vec2 {
    Vec2::new(232.0, 138.0)
}

/// The `[Type]` / `[Type: technology]` tag shown under a C4 element's name.
fn c4_type_tag(kind: C4Kind, technology: Option<&str>) -> String {
    let base = match kind {
        C4Kind::Person => "Person",
        C4Kind::System => "Software System",
        C4Kind::Container => "Container",
        C4Kind::Component => "Component",
        C4Kind::External => "External System",
    };
    match technology {
        Some(t) if !t.is_empty() => format!("[{base}: {t}]"),
        _ => format!("[{base}]"),
    }
}

/// Line + text colour for a C4 element (drawn outline-style, like Structurizr):
/// the accent for internal elements, greyed for an external one.
fn c4_line(kind: C4Kind) -> Color {
    match kind {
        C4Kind::External => style::DIM,
        _ => style::CYAN,
    }
}

/// Head-circle radius for a `person` element, atop its box.
fn c4_head_radius() -> f32 {
    30.0
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

/// Rank direction for a flowchart: top-down or left-right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RankDir {
    Down,
    Right,
}

/// How a diagram container lays out its children. `Region` is the architecture's
/// nested cluster/flow-wrap layout; `Ranked` is the flowchart's edge-driven
/// rank layout (Mermaid `graph TD`/`LR`). Both share the same bounds, scale-to-fit
/// and ports — a layout *mode* on one data structure, not a second engine.
/// `Ranked(None)` is **auto**: the layout picks the orientation (TD/LR) that fits
/// the frame best and re-decides on every added node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayoutMode {
    Region,
    Ranked(Option<RankDir>),
}

#[derive(Debug, Clone)]
pub struct ArchitectureData {
    pub center: Vec2,
    pub width: f32,
    pub height: f32,
    pub horizontal: bool,
    mode: LayoutMode,
    /// Uniform scale-to-fit factor (≤ 1.0) applied to the whole diagram by
    /// [`relayout`] when the natural layout would overflow the frame. `connect`
    /// reads it so port geometry lands on the shrunk cards. 1.0 = fits as-is.
    pub scale: f32,
    /// True for a `c4` container: node kinds are read as C4 elements
    /// (person/system/container/component/external) rather than provider/native.
    c4: bool,
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
    /// `Some` for a flowchart node whose body *is* a shape (diamond, pill, …)
    /// with a centred label and no icon; `None` for an ordinary card node.
    flow: Option<FlowShape>,
    /// `Some` for a C4 element box (filled rounded rect, multi-line label, no icon).
    c4: Option<C4Kind>,
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
    /// The routing style this connection was declared with, so a ranked
    /// (flowchart) relayout can recompute its lane geometry after nodes move.
    routing: ConnectionRouting,
    /// True when a flowchart edge took the default routing (no explicit ports):
    /// its ports follow the auto-chosen rank direction and are recomputed on each
    /// relayout. An explicit `orthogonal, <port>, <port>` edge (e.g. a loop) keeps
    /// its declared ports.
    default_routed: bool,
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

/// Leaf clusters with more than this many node children wrap into a grid.
const GRID_WRAP_MIN: usize = 4;

/// Grid `(cols, rows)` for `n` cells: squarish, but biased so the longer run
/// follows the architecture's main axis, keeping the cross axis (the one that
/// overflows) compact.
fn grid_shape(n: usize, horizontal: bool) -> (usize, usize) {
    let main = (n as f32).sqrt().ceil().max(1.0) as usize; // count along the main axis
    let cross = (n + main - 1) / main; // count across it
    if horizontal {
        (main, cross)
    } else {
        (cross, main)
    }
}

fn item_size(scene: &Scene, id: &str, horizontal: bool) -> Vec2 {
    if let Some(node) = scene.system_nodes.get(id) {
        if node.c4.is_some() {
            return c4_box_size();
        }
        return match node.flow {
            Some(shape) => flow_card_size(shape),
            None => card_size(horizontal),
        };
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
    // Grid-wrap: a leaf cluster (all members are nodes) with many children packs
    // into a rows×cols grid instead of one overflowing line, so a dense cluster
    // stays compact instead of shooting off the frame.
    if !nested && cluster.members.len() > GRID_WRAP_MIN {
        let (cols, rows) = grid_shape(cluster.members.len(), horizontal);
        let cell = card_size(horizontal);
        let content = Vec2::new(
            cols as f32 * cell.x + gap * cols.saturating_sub(1) as f32,
            rows as f32 * cell.y + gap * rows.saturating_sub(1) as f32,
        );
        return content + padding * 2.0;
    }
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
    // Grid-wrap placement: mirror the grid measured in `item_size` for a dense
    // leaf cluster, laying its node children row-major.
    let nested = cluster
        .members
        .iter()
        .any(|member| scene.system_clusters.contains_key(member));
    if !nested && cluster.members.len() > GRID_WRAP_MIN {
        let n = cluster.members.len();
        let (cols, rows) = grid_shape(n, horizontal);
        let cell = card_size(horizontal);
        let gap = 18.0;
        let grid_w = cols as f32 * cell.x + gap * cols.saturating_sub(1) as f32;
        let grid_h = rows as f32 * cell.y + gap * rows.saturating_sub(1) as f32;
        let x0 = center.x - grid_w * 0.5 + cell.x * 0.5;
        let y0 = center.y - grid_h * 0.5 + cell.y * 0.5;
        for (i, member) in cluster.members.iter().enumerate() {
            let col = (i % cols) as f32;
            let row = (i / cols) as f32;
            let child_center =
                Vec2::new(x0 + col * (cell.x + gap), y0 + row * (cell.y + gap));
            plan_item(scene, member, child_center, horizontal, plan);
        }
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

/// Inner fit margin: content is scaled to fill this fraction of the frame,
/// leaving a small border so nothing kisses the edge.
const FIT_MARGIN: f32 = 0.94;
/// The smallest scale-to-fit factor. Fitting is the promise, so this floor is
/// low enough to contain even a very dense diagram (e.g. a 20+ node flowchart);
/// past it, a diagram is simply too big to be legible at this canvas and should
/// be split or rendered taller — but it still stays inside the frame.
const MIN_SCALE: f32 = 0.2;

fn relayout(scene: &mut Scene, architecture: &str) {
    let Some(data) = scene.architectures.get(architecture).cloned() else {
        return;
    };
    // C4 lays out by element type in tiers (people on top, systems/containers in
    // the middle, externals below) — the conventional C4 reading.
    if data.c4 {
        relayout_c4(scene, architecture);
        return;
    }
    // Flowcharts rank by edge direction — a wholly different placement — but reuse
    // the same bounds, scale-to-fit and ports.
    if let LayoutMode::Ranked(dir) = data.mode {
        relayout_ranked(scene, architecture, dir);
        return;
    }
    if data.children.is_empty() {
        return;
    }
    let sizes: Vec<Vec2> = data
        .children
        .iter()
        .map(|child| item_size(scene, child, data.horizontal))
        .collect();
    // C4 boxes are large and joined by labelled relationships, so they need much
    // more room between them for the edge + its annotation to sit clearly.
    let gap = if data.c4 { 130.0 } else { 28.0 };
    // main() runs along the architecture's main axis; cross() is perpendicular.
    let main = |s: &Vec2| if data.horizontal { s.x } else { s.y };
    let cross = |s: &Vec2| if data.horizontal { s.y } else { s.x };

    // Flow-wrap the top-level children: pack them along the main axis and wrap to
    // a new line whenever they would exceed the box, so a wide diagram stacks into
    // rows (or columns) and fits instead of running off the frame. When everything
    // fits in one line this is identical to the old single-line layout, so tuned
    // examples are untouched.
    let avail = main(&Vec2::new(data.width, data.height));
    let mut lines: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut cur_main = 0.0f32;
    for (i, size) in sizes.iter().enumerate() {
        let add = main(size) + if cur.is_empty() { 0.0 } else { gap };
        if !cur.is_empty() && cur_main + add > avail {
            lines.push(std::mem::take(&mut cur));
            cur_main = 0.0;
        }
        cur_main += main(size) + if cur.is_empty() { 0.0 } else { gap };
        cur.push(i);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    let line_main: Vec<f32> = lines
        .iter()
        .map(|ln| {
            ln.iter().map(|&i| main(&sizes[i])).sum::<f32>()
                + gap * ln.len().saturating_sub(1) as f32
        })
        .collect();
    let line_cross: Vec<f32> = lines
        .iter()
        .map(|ln| ln.iter().map(|&i| cross(&sizes[i])).fold(0.0, f32::max))
        .collect();
    let total_cross =
        line_cross.iter().sum::<f32>() + gap * lines.len().saturating_sub(1) as f32;

    // Scale-to-fit safety net: measure the natural content against the frame and,
    // if it overflows, derive one uniform factor `s ≤ 1` that shrinks the whole
    // diagram — positions, cards, icons, labels and (via `data.scale`) the
    // connection ports — so a dense or deeply-nested diagram always fits without
    // the author touching a coordinate. When the content already fits, `s == 1`
    // and every downstream step is a no-op, leaving tuned examples untouched.
    let content_main = line_main.iter().cloned().fold(0.0_f32, f32::max);
    let content_cross = total_cross;
    let (content_w, content_h) = if data.horizontal {
        (content_main, content_cross)
    } else {
        (content_cross, content_main)
    };
    let usable_w = data.width * FIT_MARGIN;
    let usable_h = data.height * FIT_MARGIN;
    let sx = if content_w > 1.0 { usable_w / content_w } else { 1.0 };
    let sy = if content_h > 1.0 { usable_h / content_h } else { 1.0 };
    let s = sx.min(sy).min(1.0).max(MIN_SCALE);
    if let Some(arch) = scene.architectures.get_mut(architecture) {
        arch.scale = s;
    }

    let mut plan = LayoutPlan::default();
    let mut cross_cursor = -total_cross * 0.5;
    for (li, ln) in lines.iter().enumerate() {
        let cross_center = cross_cursor + line_cross[li] * 0.5;
        let mut main_cursor = -line_main[li] * 0.5;
        for &i in ln {
            let sz = sizes[i];
            let main_center = main_cursor + main(&sz) * 0.5;
            let center = if data.horizontal {
                data.center + Vec2::new(main_center, cross_center)
            } else {
                data.center + Vec2::new(cross_center, main_center)
            };
            plan_item(scene, &data.children[i], center, data.horizontal, &mut plan);
            main_cursor += main(&sz) + gap;
        }
        cross_cursor += line_cross[li] + gap;
    }
    // Apply the fit factor as one uniform transform about the frame centre over
    // the fully-built natural plan: every node/cluster centre contracts toward
    // the centre and cluster frames shrink, so nested structure scales coherently.
    for (node, center) in plan.nodes {
        let scaled = data.center + (center - data.center) * s;
        place_node(scene, &node, scaled, data.horizontal, s);
    }
    for (cluster_id, center, size) in plan.clusters {
        let scaled_center = data.center + (center - data.center) * s;
        let scaled_size = size * s;
        if let Some(cluster) = scene.system_clusters.get_mut(&cluster_id) {
            cluster.center = scaled_center;
            cluster.width = scaled_size.x;
            cluster.height = scaled_size.y;
        }
        if let Some(frame) = scene.get_mut(&format!("{cluster_id}.frame")) {
            frame.pos = scaled_center;
            frame.shape = Shape::Rect {
                w: scaled_size.x,
                h: scaled_size.y,
            };
        }
        if let Some(label) = scene.get_mut(&format!("{cluster_id}.label")) {
            label.pos = scaled_center + Vec2::new(0.0, -scaled_size.y * 0.5 + 17.0 * s);
            label.scale = s;
        }
    }
}

/// C4 layout: three tiers — people on top, internal systems/containers/components
/// in the middle, externals below — each a centred row, then scale-to-fit. This
/// keeps the internal system in the frame centre, so `zoom` focuses it for a
/// Context→Container reveal. Connections bake from these final positions in
/// `connect` (architecture-style), so no rewire is needed.
fn relayout_c4(scene: &mut Scene, architecture: &str) {
    let Some(data) = scene.architectures.get(architecture).cloned() else {
        return;
    };
    let nodes = data.nodes.clone();
    if nodes.is_empty() {
        return;
    }
    let bx = c4_box_size();
    let node_gap = 130.0;
    let row_gap = 108.0;
    let head = c4_head_radius() + 8.0; // top overhang so person heads clear the edge
    let tier_of = |k: Option<C4Kind>| match k {
        Some(C4Kind::Person) => 0usize,
        Some(C4Kind::External) => 2usize,
        _ => 1usize,
    };
    let mut tiers: [Vec<usize>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for (i, id) in nodes.iter().enumerate() {
        let k = scene.system_nodes.get(id).and_then(|n| n.c4);
        tiers[tier_of(k)].push(i);
    }
    let usable_w = data.width * FIT_MARGIN;
    let usable_h = data.height * FIT_MARGIN;
    // Auto-split: a tier with many entries wraps into balanced sub-rows so it never
    // runs off the frame (each row holds at most what fits the width at full size);
    // the tiers stay ordered people → internal → external.
    let per_row = (((usable_w + node_gap) / (bx.x + node_gap)).floor() as usize).max(1);
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for tier in tiers.iter() {
        if tier.is_empty() {
            continue;
        }
        let n = tier.len();
        let n_rows = n.div_ceil(per_row);
        let cols = n.div_ceil(n_rows); // balance the rows
        let mut i = 0;
        while i < n {
            let end = (i + cols).min(n);
            rows.push(tier[i..end].to_vec());
            i = end;
        }
    }
    if rows.is_empty() {
        return;
    }
    let row_w: Vec<f32> = rows
        .iter()
        .map(|r| r.len() as f32 * bx.x + node_gap * r.len().saturating_sub(1) as f32)
        .collect();
    let content_w = row_w.iter().cloned().fold(0.0, f32::max);
    let content_h =
        rows.len() as f32 * bx.y + row_gap * rows.len().saturating_sub(1) as f32 + head;
    let sx = if content_w > 1.0 { usable_w / content_w } else { 1.0 };
    let sy = if content_h > 1.0 { usable_h / content_h } else { 1.0 };
    let s = sx.min(sy).min(1.0).max(MIN_SCALE);
    if let Some(a) = scene.architectures.get_mut(architecture) {
        a.scale = s;
    }
    // Rows top→bottom, each centred; the top row leaves head-room for a person.
    let mut y = -content_h * 0.5 + head + bx.y * 0.5;
    for (ri, row) in rows.iter().enumerate() {
        let mut x = -row_w[ri] * 0.5 + bx.x * 0.5;
        for &i in row.iter() {
            let natural = data.center + Vec2::new(x, y);
            let scaled = data.center + (natural - data.center) * s;
            place_node(scene, &nodes[i], scaled, data.horizontal, s);
            x += bx.x + node_gap;
        }
        y += bx.y + row_gap;
    }
}

/// Flowchart layout: rank nodes by connection direction (BFS distance from the
/// sources), place each rank along the main axis with its members spread on the
/// cross axis, reuse scale-to-fit, then rewire every lane from the new positions.
fn relayout_ranked(scene: &mut Scene, architecture: &str, dir: Option<RankDir>) {
    let Some(data) = scene.architectures.get(architecture).cloned() else {
        return;
    };
    let nodes = data.nodes.clone();
    if nodes.is_empty() {
        return;
    }
    let n = nodes.len();
    let index: std::collections::HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut indeg = vec![0usize; n];
    for conn in scene.system_connections.values() {
        for lane in &conn.lanes {
            if let (Some(&u), Some(&v)) =
                (index.get(lane.from.as_str()), index.get(lane.to.as_str()))
            {
                adj[u].push(v);
                indeg[v] += 1;
            }
        }
    }
    // BFS layering from every source (in-degree 0). Shortest-distance ranking is
    // cycle-safe: a loop's back-edge targets an already-ranked node and is skipped.
    let mut rank = vec![usize::MAX; n];
    let mut queue: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    for &i in &queue {
        rank[i] = 0;
    }
    if queue.is_empty() {
        rank[0] = 0;
        queue.push(0);
    }
    let mut head = 0;
    while head < queue.len() {
        let u = queue[head];
        head += 1;
        for k in 0..adj[u].len() {
            let v = adj[u][k];
            if rank[v] == usize::MAX {
                rank[v] = rank[u] + 1;
                queue.push(v);
            }
        }
    }
    for r in rank.iter_mut() {
        if *r == usize::MAX {
            *r = 0; // disconnected node → first rank
        }
    }
    let max_rank = *rank.iter().max().unwrap();
    let mut ranks: Vec<Vec<usize>> = vec![Vec::new(); max_rank + 1];
    for i in 0..n {
        ranks[rank[i]].push(i); // declaration order preserved within a rank
    }

    let sizes: Vec<Vec2> = nodes
        .iter()
        .map(|id| item_size(scene, id, data.horizontal))
        .collect();
    let rank_gap = 56.0;
    let node_gap = 40.0;
    let col_gap = 84.0;
    let usable_w = data.width * FIT_MARGIN;
    let usable_h = data.height * FIT_MARGIN;
    let r_count = ranks.len();

    // Explicit LR lays the ranks in a single left-to-right row. Everything else
    // (auto and TD) uses top-down columns that WRAP: a long flow splits into
    // side-by-side columns — the count chosen to fill the frame — so nodes stay
    // large and readable instead of shrinking to a ribbon. Consecutive ranks stay
    // connected bottom→top of the next column (the default TD ports do this for
    // free), which reads like a multi-column paper flowchart.
    if dir == Some(RankDir::Right) {
        // rank thickness = along x (max node width); span = across, along y.
        let thick: Vec<f32> = ranks
            .iter()
            .map(|row| row.iter().map(|&i| sizes[i].x).fold(0.0, f32::max))
            .collect();
        let span: Vec<f32> = ranks
            .iter()
            .map(|row| {
                row.iter().map(|&i| sizes[i].y).sum::<f32>()
                    + node_gap * row.len().saturating_sub(1) as f32
            })
            .collect();
        let total = thick.iter().sum::<f32>() + rank_gap * r_count.saturating_sub(1) as f32;
        let cross = span.iter().cloned().fold(0.0, f32::max);
        let sx = if total > 1.0 { usable_w / total } else { 1.0 };
        let sy = if cross > 1.0 { usable_h / cross } else { 1.0 };
        let s = sx.min(sy).min(1.0).max(MIN_SCALE);
        if let Some(arch) = scene.architectures.get_mut(architecture) {
            arch.scale = s;
        }
        let mut x = -total * 0.5;
        for (r, row) in ranks.iter().enumerate() {
            let rx = x + thick[r] * 0.5;
            let mut y = -span[r] * 0.5;
            for &i in row {
                let cy = y + sizes[i].y * 0.5;
                let scaled = data.center + Vec2::new(rx, cy) * s;
                place_node(scene, &nodes[i], scaled, data.horizontal, s);
                y += sizes[i].y + node_gap;
            }
            x += thick[r] + rank_gap;
        }
        rewire_ranked(scene, architecture, RankDir::Right);
        return;
    }

    // Top-down / auto with column wrapping.
    // rank thickness = along the flow (y, max node height); span = across (x).
    let thick: Vec<f32> = ranks
        .iter()
        .map(|row| row.iter().map(|&i| sizes[i].y).fold(0.0, f32::max))
        .collect();
    let span: Vec<f32> = ranks
        .iter()
        .map(|row| {
            row.iter().map(|&i| sizes[i].x).sum::<f32>()
                + node_gap * row.len().saturating_sub(1) as f32
        })
        .collect();
    // Metrics for `c` columns (ranks split into `c` contiguous groups).
    let column_metrics = |c: usize| -> (f32, Vec<f32>, Vec<f32>) {
        let per = r_count.div_ceil(c);
        let mut col_w = Vec::with_capacity(c);
        let mut col_h = Vec::with_capacity(c);
        for col in 0..c {
            let lo = col * per;
            let hi = ((col + 1) * per).min(r_count);
            if lo >= hi {
                col_w.push(0.0);
                col_h.push(0.0);
                continue;
            }
            let w = (lo..hi).map(|r| span[r]).fold(0.0, f32::max);
            let h = (lo..hi).map(|r| thick[r]).sum::<f32>()
                + rank_gap * (hi - lo).saturating_sub(1) as f32;
            col_w.push(w);
            col_h.push(h);
        }
        let content_w = col_w.iter().sum::<f32>() + col_gap * c.saturating_sub(1) as f32;
        let content_h = col_h.iter().cloned().fold(0.0, f32::max);
        let sx = if content_w > 1.0 { usable_w / content_w } else { 1.0 };
        let sy = if content_h > 1.0 { usable_h / content_h } else { 1.0 };
        (sx.min(sy).min(1.0).max(MIN_SCALE), col_w, col_h)
    };
    // Pick the column count that fills the frame best. More columns shorten a deep
    // flow (bigger nodes) until width becomes the limit; ties prefer fewer columns.
    let max_cols = r_count.min(8).max(1);
    let mut cols = 1;
    let mut best_s = column_metrics(1).0;
    for c in 2..=max_cols {
        let s = column_metrics(c).0;
        if s > best_s + 0.001 {
            best_s = s;
            cols = c;
        }
    }
    let (s, col_w, col_h) = column_metrics(cols);
    if let Some(arch) = scene.architectures.get_mut(architecture) {
        arch.scale = s;
    }
    let per = r_count.div_ceil(cols);
    let content_w = col_w.iter().sum::<f32>() + col_gap * cols.saturating_sub(1) as f32;
    let content_h = col_h.iter().cloned().fold(0.0, f32::max);
    let mut x = -content_w * 0.5;
    for col in 0..cols {
        let cx = x + col_w[col] * 0.5;
        let lo = col * per;
        let hi = ((col + 1) * per).min(r_count);
        let mut y = -content_h * 0.5; // top-align every column at the block top
        for r in lo..hi {
            let ry = y + thick[r] * 0.5;
            let mut mx = -span[r] * 0.5;
            for &i in &ranks[r] {
                let ncx = cx + mx + sizes[i].x * 0.5;
                let scaled = data.center + Vec2::new(ncx, ry) * s;
                place_node(scene, &nodes[i], scaled, data.horizontal, s);
                mx += sizes[i].x + node_gap;
            }
            y += thick[r] + rank_gap;
        }
        x += col_w[col] + col_gap;
    }
    rewire_ranked(scene, architecture, RankDir::Down);
}

/// Fully-computed geometry for one lane, so a flowchart relayout can rebuild an
/// existing edge in place after nodes move (no entity churn — styling survives).
struct LaneGeom {
    start: Vec2,
    control: Vec2,
    end: Vec2,
    motion_points: Vec<Vec2>,
    body_pos: Vec2,
    body: Shape,
    /// `(arrow_start, tip)` for an orthogonal lane's terminal arrowhead entity.
    arrow: Option<(Vec2, Vec2)>,
}

/// Trim a lane back to a node's card edge along the travel direction.
fn edge_trim(size: Vec2, dir: Vec2) -> f32 {
    (if dir.x.abs() >= dir.y.abs() { size.x } else { size.y }) * 0.5 + 6.0
}

/// The pure geometry of one lane between two node centres of the given sizes —
/// mirrors `c_connect`'s per-lane math so ranked relayout can recompute it.
fn lane_geometry(a: Vec2, b: Vec2, a_size: Vec2, b_size: Vec2, routing: ConnectionRouting) -> LaneGeom {
    match routing {
        ConnectionRouting::Curved(bend) => {
            let direction = (b - a).normalize_or_zero();
            // Trim each end back to its card edge, but never so far that the two
            // trims cross on a short/diagonal edge (which would reverse the line
            // into a stub) — clamp their sum to a fraction of the span.
            let distance = (b - a).length().max(1.0);
            let mut trim_a = edge_trim(a_size, direction);
            let mut trim_b = edge_trim(b_size, direction);
            let budget = distance * 0.7;
            if trim_a + trim_b > budget {
                let k = budget / (trim_a + trim_b);
                trim_a *= k;
                trim_b *= k;
            }
            let start = a + direction * trim_a;
            let end = b - direction * trim_b;
            let delta = end - start;
            let perpendicular = Vec2::new(-delta.y, delta.x).normalize_or_zero();
            let control = (start + end) * 0.5 + perpendicular * bend;
            let route_control = (a + b) * 0.5 + perpendicular * bend;
            let motion_points = (0..=64)
                .map(|sample| bezier(a, route_control, b, sample as f32 / 64.0))
                .collect();
            LaneGeom {
                start: a,
                control: route_control,
                end: b,
                motion_points,
                body_pos: start,
                body: Shape::Curve {
                    ctrl: control,
                    to: end,
                    arrow: true,
                },
                arrow: None,
            }
        }
        ConnectionRouting::Orthogonal { from, to } => {
            let (from_port, to_port) = resolve_ports(from, to, b - a);
            let start = port_point(a, a_size, from_port);
            let end = port_point(b, b_size, to_port);
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
            let arrow_dir = motion_points
                .windows(2)
                .rev()
                .find_map(|window| {
                    let delta = window[1] - window[0];
                    (delta.length() > 0.5).then_some(delta.normalize())
                })
                .unwrap_or(Vec2::X);
            LaneGeom {
                start: a,
                control,
                end: b,
                motion_points,
                body_pos: Vec2::ZERO,
                body: Shape::Polyline { pts: visual_points },
                arrow: Some((end - arrow_dir * 22.0, end)),
            }
        }
    }
}

/// A long feedback lane routed around the diagram's bottom-left perimeter: down
/// out of the source, along the margin below every column, up the left margin, and
/// into the target's left side — the classic "back to an earlier step" rail that
/// stays clear of the flow instead of cutting across it.
fn perimeter_lane(a: Vec2, b: Vec2, a_size: Vec2, b_size: Vec2, below_y: f32, left_x: f32) -> LaneGeom {
    let start = a + Vec2::new(0.0, a_size.y * 0.5); // exit source bottom
    let end = b - Vec2::new(b_size.x * 0.5, 0.0); // enter target left side
    let pts = vec![
        start,
        Vec2::new(a.x, below_y),
        Vec2::new(left_x, below_y),
        Vec2::new(left_x, b.y),
        end,
    ];
    let mut motion_points = vec![a];
    motion_points.extend(pts.iter().copied());
    motion_points.push(b);
    LaneGeom {
        start: a,
        control: Vec2::new(left_x, below_y),
        end: b,
        motion_points,
        body_pos: Vec2::ZERO,
        body: Shape::Polyline { pts },
        arrow: Some((end - Vec2::new(22.0, 0.0), end)),
    }
}

/// Port size for a node when attaching a lane — the (fit-scaled) body extent.
fn node_port_size(node: &SystemNodeData, fit: f32, horizontal: bool) -> Vec2 {
    match node.flow {
        Some(shape) => flow_card_size(shape) * fit,
        None => card_size(horizontal) * fit,
    }
}

/// Rebuild every lane of a ranked (flowchart) architecture from the current node
/// positions, updating the existing edge/hot/arrow entities in place so colours
/// and labels applied to them survive. Runs after `relayout_ranked` moves nodes.
fn rewire_ranked(scene: &mut Scene, architecture: &str, dir: RankDir) {
    let fit = scene.architectures[architecture].scale;
    let horizontal = scene.architectures[architecture].horizontal;
    // Default-routed flowchart edges take their ports from the (possibly
    // auto-chosen) rank direction: TD leaves the bottom into the top, LR leaves
    // the right into the left. Explicit-port edges keep their declared routing.
    let dir_routing = match dir {
        RankDir::Down => ConnectionRouting::Orthogonal {
            from: Port::Bottom,
            to: Port::Top,
        },
        RankDir::Right => ConnectionRouting::Orthogonal {
            from: Port::Right,
            to: Port::Left,
        },
    };
    // Bottom-left margins for routing long feedback edges around the perimeter.
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for node in scene.system_nodes.values() {
        if node.architecture != architecture {
            continue;
        }
        let half = node_port_size(node, fit, horizontal) * 0.5;
        min = min.min(node.center - half);
        max = max.max(node.center + half);
    }
    let below_y = max.y + 34.0 * fit;
    let left_x = min.x - 34.0 * fit;

    struct Plan {
        conn: String,
        lane_idx: usize,
        path: String,
        hot_path: String,
        hot_arrow: Option<String>,
        geom: LaneGeom,
    }
    let mut plans: Vec<Plan> = Vec::new();
    for (cid, conn) in &scene.system_connections {
        let belongs = conn
            .lanes
            .first()
            .and_then(|lane| scene.system_nodes.get(&lane.from))
            .map(|node| node.architecture == architecture)
            .unwrap_or(false);
        if !belongs {
            continue;
        }
        for (lane_idx, lane) in conn.lanes.iter().enumerate() {
            let (Some(fa), Some(ta)) = (
                scene.system_nodes.get(&lane.from),
                scene.system_nodes.get(&lane.to),
            ) else {
                continue;
            };
            let (a, b) = (fa.center, ta.center);
            let a_size = node_port_size(fa, fit, horizontal);
            let b_size = node_port_size(ta, fit, horizontal);
            // A long feedback edge — one whose target sits well to the *left* (an
            // earlier column), like a loop back to the start — routes around the
            // bottom-left perimeter instead of cutting diagonally across the whole
            // chart. Shorter backward edges (a one-step loop, or a bottom→top column
            // wrap) arc as a gentle curve. Everything forward is a clean elbow.
            let geom = if conn.default_routed
                && dir == RankDir::Down
                && b.x < a.x - a_size.x * 1.2
            {
                perimeter_lane(a, b, a_size, b_size, below_y, left_x)
            } else {
                let routing = if conn.default_routed {
                    let backward = match dir {
                        RankDir::Down => b.y < a.y - 4.0,
                        RankDir::Right => b.x < a.x - 4.0,
                    };
                    if backward {
                        let bend = ((b - a).length() * 0.10).clamp(24.0, 60.0);
                        ConnectionRouting::Curved(bend)
                    } else {
                        dir_routing
                    }
                } else {
                    conn.routing
                };
                lane_geometry(a, b, a_size, b_size, routing)
            };
            plans.push(Plan {
                conn: cid.clone(),
                lane_idx,
                path: lane.path.clone(),
                hot_path: lane.hot_path.clone(),
                hot_arrow: lane.hot_arrow.clone(),
                geom,
            });
        }
    }

    for plan in &plans {
        for id in [&plan.path, &plan.hot_path] {
            if let Some(entity) = scene.get_mut(id) {
                entity.pos = plan.geom.body_pos;
                entity.shape = plan.geom.body.clone();
            }
        }
        if let Some(hot_arrow_id) = &plan.hot_arrow {
            let cold_arrow_id = hot_arrow_id.replace(".hot.arrow", ".arrow");
            match plan.geom.arrow {
                // Polyline/orthogonal lanes carry a separate terminal arrowhead.
                Some((arrow_start, tip)) => {
                    for id in [cold_arrow_id.as_str(), hot_arrow_id.as_str()] {
                        if let Some(entity) = scene.get_mut(id) {
                            entity.pos = arrow_start;
                            entity.shape = Shape::Arrow { to: tip };
                            entity.opacity = 1.0;
                        }
                    }
                }
                // A curve draws its own head, so hide the (now redundant) arrow
                // entity — otherwise it lingers at a stale spot as a stray head.
                None => {
                    for id in [cold_arrow_id.as_str(), hot_arrow_id.as_str()] {
                        if let Some(entity) = scene.get_mut(id) {
                            entity.opacity = 0.0;
                        }
                    }
                }
            }
        }
        if let Some(conn) = scene.system_connections.get_mut(&plan.conn) {
            if let Some(lane) = conn.lanes.get_mut(plan.lane_idx) {
                lane.start = plan.geom.start;
                lane.control = plan.geom.control;
                lane.end = plan.geom.end;
                lane.motion_points = plan.geom.motion_points.clone();
            }
            if plan.lane_idx == 0 {
                conn.start = plan.geom.start;
                conn.control = plan.geom.control;
                conn.end = plan.geom.end;
            }
        }
    }
}

/// Position (and, under scale-to-fit, resize) a node's card/icon/label. `center`
/// is already the fit-scaled node centre; `s` is the architecture's uniform fit
/// factor — the intra-node offsets scale by `s` and each part carries `scale = s`
/// so the card, icon glyph/image and label shrink together. At `s == 1` this is
/// the natural placement, unchanged.
fn place_node(scene: &mut Scene, node_id: &str, center: Vec2, horizontal: bool, s: f32) {
    let node_ref = scene.system_nodes.get(node_id);
    let flow = node_ref.and_then(|node| node.flow);
    let is_c4 = node_ref.map(|node| node.c4.is_some()).unwrap_or(false);
    if let Some(node) = scene.system_nodes.get_mut(node_id) {
        node.center = center;
    }
    // C4 box: a fit-scaled rounded rect with a centred multi-line label, no icon;
    // a `person` also carries a head circle atop the box.
    if is_c4 {
        let base = c4_box_size();
        let size = base * s;
        if let Some(card) = scene.get_mut(&format!("{node_id}.card")) {
            card.pos = center;
            card.shape = Shape::Rect { w: size.x, h: size.y };
            card.corner_radius = 10.0 * s;
            card.scale = 1.0;
        }
        if let Some(label) = scene.get_mut(&format!("{node_id}.label")) {
            label.pos = center;
            label.scale = s;
            label.wrap = Some((base.x - 26.0) * s);
        }
        if let Some(head) = scene.get_mut(&format!("{node_id}.head")) {
            let r = c4_head_radius();
            head.pos = center + Vec2::new(0.0, -(base.y * 0.5 + r * 0.55)) * s;
            head.scale = s;
        }
        return;
    }
    // Flowchart node: the body is a shape whose geometry encodes its size, so we
    // rebuild it at the fit-scaled size (`scale` stays 1 to avoid double-shrink),
    // and the label centres inside it. No icon entity exists.
    if let Some(shape) = flow {
        let (body, corner) = flow_body_shape(shape, flow_card_size(shape) * s);
        if let Some(card) = scene.get_mut(&format!("{node_id}.card")) {
            card.pos = center;
            card.shape = body;
            card.corner_radius = corner;
            card.scale = 1.0;
        }
        if let Some(label) = scene.get_mut(&format!("{node_id}.label")) {
            label.pos = center;
            label.scale = s;
        }
        return;
    }
    let placements = if horizontal {
        [
            (format!("{node_id}.card"), center),
            (format!("{node_id}.icon"), center + Vec2::new(0.0, -18.0) * s),
            (format!("{node_id}.label"), center + Vec2::new(0.0, 45.0) * s),
        ]
    } else {
        [
            (format!("{node_id}.card"), center),
            (format!("{node_id}.icon"), center + Vec2::new(-76.0, 0.0) * s),
            (format!("{node_id}.label"), center + Vec2::new(28.0, 0.0) * s),
        ]
    };
    for (entity_id, position) in placements {
        if let Some(entity) = scene.get_mut(&entity_id) {
            entity.pos = position;
            entity.scale = s;
        }
    }
}

/// `architecture(id, [center], [width], [height])` — responsive diagram canvas.
/// Geometry is optional: with none, the diagram **auto-fits** the canvas (centred,
/// minus safe margins), so `architecture(id)` is enough. Explicit `(center, w, h)`
/// remains an override for a hand-placed diagram.
fn c_architecture(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(4)?;
    let id = args.ident(0)?;
    ensure_system_id_available(scene, &id, args)?;
    ensure_generated_ids_available(scene, [format!("{id}.frame")], args)?;
    let canvas = scene.canvas();
    // Auto-fit default leaves a title band at the top and a caption band at the
    // bottom, so a diagram declared with no geometry does not collide with the
    // scene's headline/caption text.
    let center = if args.len() >= 2 {
        args.pair(1)?
    } else {
        Vec2::new(canvas.x * 0.5, canvas.y * 0.545)
    };
    let width = args.opt_num(2)?.unwrap_or(canvas.x * 0.92);
    let height = args.opt_num(3)?.unwrap_or(canvas.y * 0.70);
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
            mode: LayoutMode::Region,
            scale: 1.0,
            c4: false,
            children: Vec::new(),
            nodes: Vec::new(),
        },
    );
    Ok(())
}

/// `flowchart(id, [TD|LR])` — an edge-ranked diagram (Mermaid `graph TD`/`LR`).
/// Nodes rank by connection direction and auto-fit the canvas; it reuses the
/// architecture bounds, ports and scale-to-fit — a layout *mode*, not a second
/// engine. **With no direction it auto-orients**: the layout picks TD or LR by
/// whichever fits the frame larger and re-decides on every added node, so a
/// growing flow reflows (and even flips orientation) on its own; connections
/// follow. Give nodes flowchart shape kinds (`terminator`/`process`/`decision`/
/// `io`/…) and connect them; a token then `route`s the path.
fn c_flowchart(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    // (id, [dir], [max_nodes]); max_nodes is the readability split limit, enforced
    // by the language `check` (a warning), so the engine just accepts it here.
    args.max(3)?;
    let id = args.ident(0)?;
    ensure_system_id_available(scene, &id, args)?;
    ensure_generated_ids_available(scene, [format!("{id}.frame")], args)?;
    let dir: Option<RankDir> = if args.len() >= 2 {
        match args.ident(1)?.to_ascii_uppercase().as_str() {
            "TD" | "TB" | "DOWN" => Some(RankDir::Down),
            "LR" | "RIGHT" => Some(RankDir::Right),
            "AUTO" => None,
            other => {
                return Err(Error::new(
                    format!("unknown flowchart direction `{other}`; use `TD` (top-down), `LR` (left-right), or `auto`"),
                    args.span_of(1),
                ))
            }
        }
    } else {
        None
    };
    let canvas = scene.canvas();
    let center = Vec2::new(canvas.x * 0.5, canvas.y * 0.545);
    let width = canvas.x * 0.92;
    let height = canvas.y * 0.70;
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
            // `horizontal` only sizes card-nodes (flowchart shape-nodes ignore it)
            // and seeds a default; the live rank direction is chosen at layout.
            horizontal: matches!(dir, Some(RankDir::Right)),
            mode: LayoutMode::Ranked(dir),
            scale: 1.0,
            c4: false,
            children: Vec::new(),
            nodes: Vec::new(),
        },
    );
    Ok(())
}

/// `c4(id, [level])` — a C4-model diagram canvas (Context / Container / Component).
/// Reuses the architecture region layout, ports and scale-to-fit; **inside it**,
/// `node` kinds are read as C4 elements (`person`/`system`/`container`/`component`/
/// `external`), each a filled box with a name, a `[Type: technology]` tag and a
/// description. `level` is optional and advisory. Connect elements with `connect`
/// and label the relationship with `annotate`.
fn c_c4(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(2)?;
    let id = args.ident(0)?;
    ensure_system_id_available(scene, &id, args)?;
    ensure_generated_ids_available(scene, [format!("{id}.frame")], args)?;
    if args.len() >= 2 {
        let level = args.ident(1)?.to_ascii_lowercase();
        if !matches!(
            level.as_str(),
            "context" | "container" | "component" | "code" | "system"
        ) {
            return Err(Error::new(
                format!("unknown C4 level `{level}` (use `context`, `container`, or `component`)"),
                args.span_of(1),
            ));
        }
    }
    let canvas = scene.canvas();
    let center = Vec2::new(canvas.x * 0.5, canvas.y * 0.545);
    let width = canvas.x * 0.92;
    let height = canvas.y * 0.70;
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
            horizontal: true,
            mode: LayoutMode::Region,
            scale: 1.0,
            c4: true,
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
    // (id, parent, "kind", "name/label", [description], [technology]) — the last
    // two are the C4 box's description + technology (ignored by non-C4 nodes).
    args.max(6)?;
    let id = args.ident(0)?;
    ensure_system_id_available(scene, &id, args)?;
    ensure_generated_ids_available(
        scene,
        [
            format!("{id}.card"),
            format!("{id}.icon"),
            format!("{id}.label"),
            format!("{id}.head"),
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
    let is_c4 = data.c4;
    // Inside a `c4` container the kind is a C4 element; otherwise a flowchart shape
    // (`decision`/…), a native archetype (`service`/…), or a `provider:name` icon.
    let c4 = if is_c4 { c4_kind(&kind) } else { None };
    let is_provider = !is_c4 && kind.contains(':');
    let flow = if is_c4 || is_provider {
        None
    } else {
        flow_shape(&kind)
    };
    if is_c4 {
        if c4.is_none() {
            return Err(Error::new(
                format!(
                    "unknown C4 element kind `{kind}`; inside a `c4` diagram use one of {}",
                    C4_KINDS.join(", ")
                ),
                args.span_of(2),
            ));
        }
    } else if is_provider && provider_icon(&kind).is_none() {
        return Err(Error::new(
            format!(
                "Systems Kit has no diagram icon for `{kind}`; use a catalogued \
                 `provider:name` (e.g. `aws:lambda`, `gcp:bigquery`, `onprem:redis`, \
                 `k8s:pod`) or `provider:category/name` to disambiguate"
            ),
            args.span_of(2),
        ));
    } else if !is_provider && flow.is_none() && native_node_icon(&kind).is_none() {
        return Err(Error::new(
            format!(
                "unknown system node kind `{kind}`; use a flowchart shape ({}), a native \
                 archetype ({}), or a `provider:name` icon (aws/gcp/azure/onprem/k8s/ibm/oci/…)",
                FLOW_SHAPE_KINDS.join(", "),
                NATIVE_NODE_KINDS.join(", ")
            ),
            args.span_of(2),
        ));
    }
    let center = data.center;
    if let Some(k) = c4 {
        // C4 element: an outline-styled rounded box (coloured border + text,
        // transparent fill) with a centred multi-line label (name / [Type: tech] /
        // description) and no icon. A `person` also gets a head: an outlined circle
        // sitting atop the box. `place_node` rebuilds all of it at the fit scale.
        let size = c4_box_size();
        let line = c4_line(k);
        let mut panel = Entity::new(
            format!("{id}.card"),
            Shape::Rect {
                w: size.x,
                h: size.y,
            },
            center,
            line,
        );
        panel.corner_radius = 10.0;
        panel.stroke = StrokeStyle {
            fill: false,
            outline: true,
            width: 2.5,
            outline_color: Some(line),
        };
        panel
            .tags
            .extend([id.clone(), format!("{architecture}.nodes")]);
        panel.z = -1;
        scene.add(panel);
        if k == C4Kind::Person {
            let r = c4_head_radius();
            let mut head = Entity::new(
                format!("{id}.head"),
                Shape::Circle { r },
                center + Vec2::new(0.0, -(size.y * 0.5 + r * 0.55)),
                line,
            );
            head.stroke = StrokeStyle {
                fill: false,
                outline: true,
                width: 2.5,
                outline_color: Some(line),
            };
            head.tags
                .extend([id.clone(), format!("{architecture}.nodes")]);
            head.z = -1;
            scene.add(head);
        }
    } else if let Some(shape) = flow {
        // Flowchart node: the body *is* the shape, with a centred label and no
        // icon. Tagged `{id}.card` like any node so relayout/scale-to-fit reuse
        // applies; `place_node` rebuilds its geometry at the fit-scaled size.
        let (body, corner) = flow_body_shape(shape, flow_card_size(shape));
        let mut panel = Entity::new(format!("{id}.card"), body, center, style::PANEL);
        panel.corner_radius = corner;
        panel.stroke = StrokeStyle {
            fill: true,
            outline: true,
            width: 2.5,
            outline_color: Some(style::CYAN),
        };
        panel
            .tags
            .extend([id.clone(), format!("{architecture}.nodes")]);
        panel.z = -1;
        scene.add(panel);
    } else {
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
    }

    // For a C4 box the label is `name / [Type: tech] / description`, wrapped to the
    // box; otherwise it's the plain node label.
    let (label_content, label_size, label_wrap) = if let Some(k) = c4 {
        let description = if args.len() > 4 { args.text(4)? } else { String::new() };
        let technology = if args.len() > 5 { args.text(5)? } else { String::new() };
        let tag = c4_type_tag(
            k,
            if technology.is_empty() {
                None
            } else {
                Some(technology.as_str())
            },
        );
        let mut content = format!("{label}\n{tag}");
        if !description.is_empty() {
            content.push('\n');
            content.push_str(&description);
        }
        (content, 15.0, Some(c4_box_size().x - 26.0))
    } else if flow.is_some() {
        (label, 15.0, None)
    } else if horizontal {
        (label, 16.0, None)
    } else {
        (label, 19.0, None)
    };
    let label_color = c4.map(c4_line).unwrap_or(style::FG);
    let mut text = Entity::new(
        format!("{id}.label"),
        Shape::Text {
            content: label_content,
            size: label_size,
        },
        center,
        label_color,
    );
    text.font = FontKind::MonoBold;
    text.wrap = label_wrap;
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
            flow,
            c4,
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
    // In a flowchart, an unqualified edge is an elbow connector whose ports follow
    // the (possibly auto-chosen) rank direction — recomputed at layout, so it stays
    // right even when the chart auto-flips TD↔LR. An explicit routing arg still
    // wins. A placeholder orthogonal routing here fixes the edge's entity-id shape.
    let is_ranked = matches!(
        scene
            .system_nodes
            .get(&from_nodes[0])
            .and_then(|node| scene.architectures.get(&node.architecture))
            .map(|arch| arch.mode),
        Some(LayoutMode::Ranked(_))
    );
    let default_routed = args.len() <= 3 && is_ranked;
    let routing = if default_routed {
        ConnectionRouting::Orthogonal {
            from: Port::Bottom,
            to: Port::Top,
        }
    } else {
        routing
    };
    // C4 perimeter routing. In the people-top C4 layout a relationship that spans
    // *upward* across tiers (e.g. an external system's notification back to the
    // person on top) would draw straight through the centre, bisecting the diagram
    // and colliding with the labels there. Bow it out past the box cluster so it
    // hugs the margin instead — the conventional C4 way to keep the read clear.
    // Only the bend changes: it stays a Curved edge, so entity ids, the arrow and
    // the label (which rides the control point) are all unchanged, just swung out.
    let routing = {
        let arch_name = scene.system_nodes[&from_nodes[0]].architecture.clone();
        let arch = &scene.architectures[&arch_name];
        let single = from_nodes.len() == 1 && to_nodes.len() == 1 && args.len() <= 3;
        let a = scene.system_nodes[&from_nodes[0]].center;
        let b = scene.system_nodes[&to_nodes[0]].center;
        let s = arch.scale;
        let box_size = c4_box_size() * s;
        // "Upward" more than a tier (y grows downward, so a higher tier has a
        // smaller y). One box-height of slack keeps adjacent-tier edges straight.
        if arch.c4 && single && b.y < a.y - box_size.y * 1.3 {
            let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
            for node in scene.system_nodes.values() {
                if node.architecture == arch_name {
                    min_x = min_x.min(node.center.x);
                    max_x = max_x.max(node.center.x);
                }
            }
            let half = box_size.x * 0.5;
            let clearance = 70.0 * s;
            let m = (a + b) * 0.5;
            // Route around whichever side the edge already leans toward.
            let margin_x = if m.x <= arch.center.x {
                min_x - half - clearance
            } else {
                max_x + half + clearance
            };
            let dir = (b - a).normalize_or_zero();
            let perp = Vec2::new(-dir.y, dir.x);
            // Choose the bend so the arc's *midpoint* — not just its control point,
            // which a quadratic bezier only pulls halfway toward — reaches the margin.
            if perp.x.abs() > 0.05 {
                ConnectionRouting::Curved(2.0 * (margin_x - m.x) / perp.x)
            } else {
                routing
            }
        } else {
            routing
        }
    };
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
        // Nodes may have been shrunk by scale-to-fit; attach ports to the actual
        // (scaled) card so lanes meet the card edge, not a phantom full-size box.
        let fit = scene.architectures[&architecture].scale;
        let size = card_size(horizontal) * fit;
        let (control, end, motion_points, mut edge, arrow_id) = match routing {
            ConnectionRouting::Curved(bend) => {
                let direction = (b - a).normalize_or_zero();
                // Trim the lane back to the (scaled) card edge before the arc.
                let trim = fit
                    * if (b - a).x.abs() >= (b - a).y.abs() {
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
            routing,
            default_routed,
            lanes,
        },
    );
    // A flowchart ranks nodes by edge direction, so every new edge can change
    // node positions. Re-run the ranked layout (which also rewires all lanes from
    // the new positions). Architecture mode bakes once and is left untouched.
    if let LayoutMode::Ranked(_) = scene.architectures[&architecture].mode {
        relayout(scene, &architecture);
    }
    Ok(())
}

/// `annotate(edge, "text")` — a small caption at a connection's midpoint (a
/// decision's "yes"/"no", or any edge annotation). General to every diagram type,
/// not flowchart-specific. Creates `{edge}.text`, tagged with the edge id so it
/// reveals/hides with the connection. Call it after the edge's final layout.
/// (The name `label` is the core std builtin that pins text to an entity.)
fn c_annotate(scene: &mut Scene, args: &Args) -> Result<(), Error> {
    args.max(2)?;
    let edge = args.ident(0)?;
    let raw = args.text(1)?;
    // A relationship label with a technology reads (and fits) best on two lines —
    // the verb phrase, then the `[tech]` — matching the C4 convention and keeping
    // the label narrow enough for a tight gap. Only splits an unbroken label.
    let text = match (raw.contains('\n'), raw.find(" [")) {
        (false, Some(i)) => format!("{}\n{}", &raw[..i], &raw[i + 1..]),
        _ => raw,
    };
    let conn = scene.system_connections.get(&edge).ok_or_else(|| {
        Error::new(
            format!("no connection named `{edge}` to label"),
            args.span_of(0),
        )
    })?;
    let (control, start, end, from) = (conn.control, conn.start, conn.end, conn.from.clone());
    // Match the diagram's fit scale so the caption shrinks with a dense diagram
    // (node labels already do). `annotate` is called after the edge's final
    // layout, so the scale is settled.
    let owner = scene.system_nodes.get(&from).map(|node| node.architecture.clone());
    let fit = owner
        .as_ref()
        .and_then(|arch| scene.architectures.get(arch))
        .map(|arch| arch.scale)
        .unwrap_or(1.0);
    // Sit just off the lane midpoint, on the perpendicular, so it never lands on
    // the line itself.
    let direction = (end - start).normalize_or_zero();
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let pos = control + perpendicular * 18.0 * fit;
    let size = 15.0 * fit;
    let label_id = format!("{edge}.text");
    let bg_id = format!("{edge}.text.bg");
    ensure_generated_ids_available(scene, [label_id.clone(), bg_id.clone()], args)?;
    let group = owner.as_ref().map(|arch| format!("{arch}.connections"));

    // A backdrop chip sized to the (monospace) text, so the label reads cleanly
    // even where it crosses a lane or a box — it masks whatever is behind it.
    let lines = text.split('\n');
    let max_chars = lines.clone().map(|l| l.chars().count()).max().unwrap_or(0) as f32;
    let n_lines = text.split('\n').count().max(1) as f32;
    let chip_w = max_chars * size * 0.60 + 16.0 * fit;
    let chip_h = n_lines * size * 1.30 + 8.0 * fit;
    let mut chip = Entity::new(
        bg_id.clone(),
        Shape::Rect {
            w: chip_w,
            h: chip_h,
        },
        pos,
        style::PANEL,
    );
    chip.corner_radius = 5.0 * fit;
    chip.opacity = 0.92;
    chip.stroke = StrokeStyle {
        fill: true,
        outline: false,
        width: 0.0,
        outline_color: None,
    };
    chip.tags.extend([edge.clone(), bg_id]);
    if let Some(g) = &group {
        chip.tags.push(g.clone());
    }
    chip.z = 5;
    scene.add(chip);

    let mut caption = Entity::new(
        label_id.clone(),
        Shape::Text {
            content: text,
            size,
        },
        pos,
        style::FG,
    );
    caption.font = FontKind::MonoBold;
    caption.tags.extend([edge.clone(), label_id]);
    // Also join the diagram's connection group so revealing/hiding the edges (e.g.
    // switching between split sub-flows) carries the caption with them.
    if let Some(g) = group {
        caption.tags.push(g);
    }
    caption.z = 6;
    scene.add(caption);
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
    registry.ctor("flowchart", c_flowchart);
    registry.ctor("c4", c_c4);
    registry.ctor("node", c_node);
    registry.ctor("cluster", c_cluster);
    registry.ctor("connect", c_connect);
    registry.ctor("annotate", c_annotate);
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
