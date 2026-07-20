//! Keyframe timeline: tracks, clips, and stateless evaluation.
//!
//! Invariant: `Timeline::apply(base, t)` is a pure function of absolute time.
//! No state accumulates between frames, which is what makes pause, stepping,
//! scrubbing, and deterministic recording possible.

use std::collections::HashMap;

use macroquad::prelude::{Color, EulerRot, Quat, Vec2, Vec3};

use crate::easing::Easing;
use crate::primitives::{
    BoundProperty, Entity, GraphSrc, GraphView, ParameterBinding, ParameterMap, Shape,
};
use crate::scene::Scene;

/// A dynamically-typed animatable value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    F(f32),
    V(Vec2),
    V3(Vec3),
    Q(Quat),
    C(Color),
}

impl Value {
    fn add(self, other: Value) -> Value {
        match (self, other) {
            (Value::F(a), Value::F(b)) => Value::F(a + b),
            (Value::V(a), Value::V(b)) => Value::V(a + b),
            (Value::V3(a), Value::V3(b)) => Value::V3(a + b),
            _ => other,
        }
    }

    fn lerp(a: Value, b: Value, u: f32) -> Value {
        match (a, b) {
            (Value::F(x), Value::F(y)) => Value::F(x + (y - x) * u),
            (Value::V(x), Value::V(y)) => Value::V(x + (y - x) * u),
            (Value::V3(x), Value::V3(y)) => Value::V3(x + (y - x) * u),
            (Value::Q(x), Value::Q(y)) => Value::Q(x.slerp(y, u).normalize()),
            (Value::C(x), Value::C(y)) => Value::C(Color::new(
                x.r + (y.r - x.r) * u,
                x.g + (y.g - x.g) * u,
                x.b + (y.b - x.b) * u,
                x.a + (y.a - x.a) * u,
            )),
            _ => b,
        }
    }
}

/// Which property of an entity a track animates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prop {
    Pos,
    /// Endpoint of a `Line`/`Arrow`/`Curve` shape.
    To,
    /// Control point of a quadratic `Curve` shape.
    Ctrl,
    Color,
    Opacity,
    Scale,
    /// Rotation in degrees ([`crate::primitives::Entity::rot`]).
    Rot,
    /// Euler rotation in degrees for a 3D entity.
    Rot3,
    /// Stable additional quaternion orientation for V2 axis turns.
    Orient3,
    /// Camera azimuth/elevation. Kept separate from roll so `orbit3` and
    /// `roll3` can run concurrently without overwriting each other.
    Orbit3,
    /// Camera roll in degrees around the viewing direction.
    Roll3,
    /// Camera vertical field of view, or visible height in orthographic mode.
    Fov3,
    /// Draw-on / typewriter progress ([`crate::primitives::Entity::trace`]).
    Trace,
    /// Monotonic path-flow phase; the renderer uses its fractional part for a
    /// transient travelling emphasis pulse.
    Flow,
    /// HSL hue angle in degrees — drives `color` for colour cycling.
    Hue,
    /// A live numeric readout ([`crate::primitives::Counter::value`]); the
    /// text content re-renders each frame as it tweens.
    Value,
    /// Shape-morph fraction `0→1` — blends the entity's `Polyline` between the
    /// two outlines in [`crate::primitives::Entity::morph`].
    Morph,
    /// A tangent's touch position `x` in the graph's own units
    /// ([`crate::primitives::Tangent::x`]); the segment + contact dot recompute
    /// each frame as it slides along the curve.
    PlotX,
}

/// Where a track ends up. `Rel` and `Revert` are resolved to absolute values
/// in [`Timeline::resolve`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetValue {
    Abs(Value),
    /// Current value + delta (`move_by`, `pulse`, `shake`).
    Rel(Value),
    /// Rotate the current 2-D point about a shared pivot. Unlike an absolute
    /// endpoint this resolves from the preceding track's settled value, so a
    /// `turn` composes after `move`, `travel`, or another `turn` without a
    /// constructor-position snap.
    RotateAround {
        pivot: Vec2,
        degrees: f32,
    },
    /// Rotate the current 3-D point about a world-space pivot and axis.
    RotateAround3 {
        pivot: Vec3,
        axis: Vec3,
        degrees: f32,
    },
    /// The value the property had before the previous track started
    /// (`highlight`/`pulse` auto-restore).
    Revert,
}

/// One property animation; `start` is relative to the enclosing [`Clip`].
#[derive(Debug, Clone)]
pub struct TrackSpec {
    pub id: String,
    pub prop: Prop,
    pub target: TargetValue,
    pub start: f32,
    pub dur: f32,
    pub easing: Easing,
}

/// An instantaneous, stateless scene change. Shape replacement is used by the
/// final frame of `rewrite`: moving pieces disappear and the exact RaTeX image
/// becomes the entity's new settled shape while retaining the same public id.
#[derive(Debug, Clone)]
pub enum TimelineEvent {
    Text {
        id: String,
        content: String,
        at: f32,
    },
    Shape {
        id: String,
        shape: Shape,
        at: f32,
    },
    /// A time-scoped 2-D attachment relationship. `target: None` releases the
    /// child; the accompanying zero-duration position track emitted by the DSL
    /// keeps the release frame exact and lets later movement compose normally.
    Attachment {
        id: String,
        target: Option<String>,
        offset: Vec2,
        at: f32,
    },
    /// A visual-blueprint transition used by `become`. Animatable scalar
    /// properties travel on ordinary tracks; this event owns geometry and the
    /// remaining shape styling, then installs the exact target at `u = 1`.
    Become {
        id: String,
        from: Box<Entity>,
        to: Box<Entity>,
        at: f32,
        dur: f32,
        easing: Easing,
        crossfade: bool,
    },
    /// Time-scoped 3-D relationship. Release is paired with an exact authored
    /// position track just like the 2-D attachment surface.
    Attachment3 {
        id: String,
        target: Option<String>,
        offset: Vec3,
        rigid: bool,
        relative_orientation: Quat,
        at: f32,
    },
    /// Runtime-sampled travel along another 3D entity's transformed path. The
    /// path is read every frame during the move, then sampled once at the end
    /// so later path motion does not drag the traveller with it.
    Travel3 {
        id: String,
        path: String,
        at: f32,
        dur: f32,
        easing: Easing,
    },
    /// Identity-preserving 3-D blueprint transition. `morph` is precomputed at
    /// build time for compatible families; `None` is the safe local crossfade.
    Become3 {
        id: String,
        from: Box<crate::primitives3d::Entity3D>,
        to: Box<crate::primitives3d::Entity3D>,
        morph: Option<crate::primitives3d::Morph3>,
        at: f32,
        dur: f32,
        easing: Easing,
        crossfade: bool,
    },
}

impl TimelineEvent {
    pub fn text(id: String, content: String, at: f32) -> Self {
        Self::Text { id, content, at }
    }

    pub fn shape(id: String, shape: Shape, at: f32) -> Self {
        Self::Shape { id, shape, at }
    }

    pub fn attachment(id: String, target: Option<String>, offset: Vec2, at: f32) -> Self {
        Self::Attachment {
            id,
            target,
            offset,
            at,
        }
    }

    pub fn visual_transition(
        id: String,
        from: Entity,
        to: Entity,
        dur: f32,
        easing: Easing,
        crossfade: bool,
    ) -> Self {
        Self::Become {
            id,
            from: Box::new(from),
            to: Box::new(to),
            at: 0.0,
            dur,
            easing,
            crossfade,
        }
    }

    pub fn attachment3(
        id: String,
        target: Option<String>,
        offset: Vec3,
        rigid: bool,
        relative_orientation: Quat,
        at: f32,
    ) -> Self {
        Self::Attachment3 {
            id,
            target,
            offset,
            rigid,
            relative_orientation,
            at,
        }
    }

    pub fn visual_transition3(
        id: String,
        from: crate::primitives3d::Entity3D,
        to: crate::primitives3d::Entity3D,
        morph: Option<crate::primitives3d::Morph3>,
        dur: f32,
        easing: Easing,
        crossfade: bool,
    ) -> Self {
        Self::Become3 {
            id,
            from: Box::new(from),
            to: Box::new(to),
            morph,
            at: 0.0,
            dur,
            easing,
            crossfade,
        }
    }

    pub fn travel3(id: String, path: String, dur: f32, easing: Easing) -> Self {
        Self::Travel3 {
            id,
            path,
            at: 0.0,
            dur,
            easing,
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::Text { id, .. }
            | Self::Shape { id, .. }
            | Self::Attachment { id, .. }
            | Self::Become { id, .. }
            | Self::Attachment3 { id, .. }
            | Self::Travel3 { id, .. }
            | Self::Become3 { id, .. } => id,
        }
    }

    pub fn at(&self) -> f32 {
        match self {
            Self::Text { at, .. }
            | Self::Shape { at, .. }
            | Self::Attachment { at, .. }
            | Self::Become { at, .. }
            | Self::Attachment3 { at, .. }
            | Self::Travel3 { at, .. }
            | Self::Become3 { at, .. } => *at,
        }
    }

    pub fn shift(&mut self, dt: f32) {
        match self {
            Self::Text { at, .. }
            | Self::Shape { at, .. }
            | Self::Attachment { at, .. }
            | Self::Become { at, .. }
            | Self::Attachment3 { at, .. }
            | Self::Travel3 { at, .. }
            | Self::Become3 { at, .. } => *at += dt,
        }
    }
}

/// Backwards-compatible internal name used by existing text-producing kits.
pub type TextEvent = TimelineEvent;

/// A composable bundle of tracks/events with a known duration. Times inside
/// are relative to clip start; `seq!`/`par!` build on these.
#[derive(Debug, Clone, Default)]
pub struct Clip {
    pub tracks: Vec<TrackSpec>,
    pub events: Vec<TextEvent>,
    pub dur: f32,
}

impl Clip {
    /// An empty clip occupying `d` seconds.
    pub fn wait(d: f32) -> Clip {
        Clip {
            dur: d,
            ..Default::default()
        }
    }

    /// Shift everything in this clip later by `dt`.
    pub fn shift(mut self, dt: f32) -> Clip {
        for t in &mut self.tracks {
            t.start += dt;
        }
        for e in &mut self.events {
            e.shift(dt);
        }
        self.dur += dt;
        self
    }

    /// Run clips one after another. Total duration = sum.
    pub fn seq(clips: Vec<Clip>) -> Clip {
        let mut out = Clip::default();
        for c in clips {
            let offset = out.dur;
            for mut t in c.tracks {
                t.start += offset;
                out.tracks.push(t);
            }
            for mut e in c.events {
                e.shift(offset);
                out.events.push(e);
            }
            out.dur += c.dur;
        }
        out
    }

    /// Run clips at the same time. Total duration = longest.
    pub fn par(clips: Vec<Clip>) -> Clip {
        let mut out = Clip::default();
        for c in clips {
            out.tracks.extend(c.tracks);
            out.events.extend(c.events);
            out.dur = out.dur.max(c.dur);
        }
        out
    }
}

/// A resolved track: `from` is concrete, so evaluation is direct interpolation.
#[derive(Debug, Clone)]
struct Track {
    from: Value,
    to: Value,
    start: f32,
    dur: f32,
    easing: Easing,
}

/// The resolved, immutable animation program for a movie.
#[derive(Debug, Clone, Default)]
pub struct Timeline {
    /// Per (entity, property), sorted by start time.
    tracks: HashMap<(String, Prop), Vec<Track>>,
    events: Vec<TextEvent>,
    /// Total duration in seconds.
    pub dur: f32,
}

fn get_prop(scene: &Scene, id: &str, prop: Prop) -> Option<Value> {
    if let Some(e) = scene.get(id) {
        return Some(match prop {
            Prop::Pos => Value::V(e.pos),
            Prop::Color => Value::C(e.color),
            Prop::Opacity => Value::F(e.opacity),
            Prop::Scale => Value::F(e.scale),
            Prop::Rot => Value::F(e.rot),
            Prop::Trace => Value::F(e.trace),
            Prop::Flow => Value::F(e.flow),
            Prop::Hue => Value::F(e.hue.unwrap_or(0.0)),
            Prop::Value => Value::F(e.counter.as_ref().map(|c| c.value).unwrap_or(0.0)),
            Prop::Morph => {
                if e.morph.is_none() {
                    return None;
                }
                Value::F(0.0)
            }
            Prop::To => match &e.shape {
                Shape::Line { to }
                | Shape::Arrow { to }
                | Shape::Curve { to, .. }
                | Shape::Coil { to, .. } => Value::V(*to),
                _ => return None,
            },
            Prop::Ctrl => match &e.shape {
                Shape::Curve { ctrl, .. } => Value::V(*ctrl),
                _ => return None,
            },
            Prop::PlotX => match &e.graph_view {
                Some(gv) => Value::F(gv.x()),
                None => return None,
            },
            Prop::Rot3 | Prop::Orient3 | Prop::Orbit3 | Prop::Roll3 | Prop::Fov3 => return None,
        });
    }
    let e = scene.get_3d(id)?;
    Some(match prop {
        Prop::Pos => Value::V3(e.pos),
        Prop::Color => Value::C(e.color),
        Prop::Opacity => Value::F(e.opacity),
        Prop::Scale => Value::F(e.scale),
        Prop::Rot3 => Value::V3(e.rotation),
        Prop::Orient3 => Value::Q(e.orientation),
        Prop::Orbit3 => Value::V3(e.rotation),
        Prop::Roll3 => Value::F(e.rotation.z),
        Prop::Fov3 => match e.shape {
            crate::primitives3d::Shape3D::Camera { fov, .. } => Value::F(fov),
            _ => return None,
        },
        Prop::Trace => Value::F(e.trace),
        Prop::To => match &e.shape {
            crate::primitives3d::Shape3D::Line { to }
            | crate::primitives3d::Shape3D::Arrow { to } => Value::V3(*to),
            _ => return None,
        },
        Prop::Morph => {
            if e.morph3.is_none() {
                return None;
            }
            Value::F(0.0)
        }
        Prop::Rot | Prop::Ctrl | Prop::Hue | Prop::Value | Prop::PlotX | Prop::Flow => return None,
    })
}

fn set_prop(scene: &mut Scene, id: &str, prop: Prop, v: Value) {
    if let Some(e) = scene.get_mut(id) {
        match (prop, v) {
            (Prop::Pos, Value::V(p)) => e.pos = p,
            (Prop::Color, Value::C(c)) => e.color = c,
            (Prop::Opacity, Value::F(o)) => e.opacity = o,
            (Prop::Scale, Value::F(s)) => e.scale = s,
            (Prop::Rot, Value::F(r)) => e.rot = r,
            (Prop::Trace, Value::F(f)) => e.trace = f,
            (Prop::Flow, Value::F(f)) => e.flow = f,
            (Prop::Hue, Value::F(h)) => {
                e.hue = Some(h);
                let c = crate::style::hsl(h, 1.0, 0.6);
                e.color = c;
                if e.stroke.outline_color.is_some() {
                    e.stroke.outline_color = Some(c);
                }
            }
            (Prop::Value, Value::F(v)) => {
                let v = e
                    .parameter
                    .map(|parameter| v.clamp(parameter.min, parameter.max))
                    .unwrap_or(v);
                if let Some(c) = &mut e.counter {
                    c.value = v;
                    let text = c.render();
                    if let Shape::Text { content, .. } = &mut e.shape {
                        *content = text;
                    }
                }
            }
            (Prop::Morph, Value::F(f)) => {
                let rebuilt = e.morph.as_ref().map(|(from, to, spin)| {
                    let mut pts: Vec<Vec2> = from
                        .iter()
                        .zip(to)
                        .map(|(a, b)| *a + (*b - *a) * f)
                        .collect();
                    // winding: rotate the blend by `f * spin` about its centroid
                    if spin.abs() > 1e-3 && !pts.is_empty() {
                        let c = pts.iter().copied().sum::<Vec2>() / pts.len() as f32;
                        let (s, cc) = (f * *spin).to_radians().sin_cos();
                        for p in &mut pts {
                            let d = *p - c;
                            *p = c + Vec2::new(d.x * cc - d.y * s, d.x * s + d.y * cc);
                        }
                    }
                    pts
                });
                if let Some(pts) = rebuilt {
                    e.shape = Shape::Polyline { pts };
                }
            }
            (Prop::To, Value::V(p)) => {
                if let Shape::Line { to }
                | Shape::Arrow { to }
                | Shape::Curve { to, .. }
                | Shape::Coil { to, .. } = &mut e.shape
                {
                    *to = p;
                }
            }
            (Prop::Ctrl, Value::V(p)) => {
                if let Shape::Curve { ctrl, .. } = &mut e.shape {
                    *ctrl = p;
                }
            }
            (Prop::PlotX, Value::F(nx)) => {
                // move the view's parameter, then recompute the entity from it
                if let Some(gv) = e.graph_view.as_mut() {
                    gv.set_x(nx);
                }
                refresh_graph_view(e);
            }
            _ => {}
        }
        return;
    }
    let Some(e) = scene.get_3d_mut(id) else {
        return;
    };
    match (prop, v) {
        (Prop::Pos, Value::V3(p)) => e.pos = p,
        (Prop::Color, Value::C(c)) => e.color = c,
        (Prop::Opacity, Value::F(o)) => e.opacity = o,
        (Prop::Scale, Value::F(s)) => e.scale = s,
        (Prop::Rot3, Value::V3(r)) => e.rotation = r,
        (Prop::Orient3, Value::Q(q)) => e.orientation = q.normalize(),
        (Prop::Orbit3, Value::V3(r)) => {
            e.rotation.x = r.x;
            e.rotation.y = r.y;
        }
        (Prop::Roll3, Value::F(r)) => e.rotation.z = r,
        (Prop::Fov3, Value::F(value)) => {
            if let crate::primitives3d::Shape3D::Camera { fov, .. } = &mut e.shape {
                *fov = value.max(0.01);
            }
        }
        (Prop::Trace, Value::F(f)) => e.trace = f,
        (Prop::To, Value::V3(p)) => {
            if let crate::primitives3d::Shape3D::Line { to }
            | crate::primitives3d::Shape3D::Arrow { to } = &mut e.shape
            {
                *to = p;
            }
        }
        (Prop::Ctrl, _) => {}
        (Prop::Morph, Value::F(f)) => {
            use crate::primitives3d::{Morph3Kind, Shape3D};
            if let Some(m) = &e.morph3 {
                let mut pts: Vec<Vec3> = m
                    .from
                    .iter()
                    .zip(&m.to)
                    .map(|(a, b)| *a + (*b - *a) * f)
                    .collect();
                // winding: rotate the blend about the vertical (Z) axis
                if m.spin.abs() > 1e-3 && !pts.is_empty() {
                    let mut c = Vec3::ZERO;
                    for p in &pts {
                        c += *p;
                    }
                    c /= pts.len() as f32;
                    let (s, cc) = (f * m.spin).to_radians().sin_cos();
                    for p in &mut pts {
                        let d = *p - c;
                        p.x = c.x + d.x * cc - d.y * s;
                        p.y = c.y + d.x * s + d.y * cc;
                    }
                }
                e.shape = match m.kind {
                    Morph3Kind::Path => Shape3D::Path { points: pts },
                    Morph3Kind::Surface { nu, nv } => Shape3D::Surface { pts, nu, nv },
                };
            }
        }
        _ => {}
    }
}

fn refresh_graph_view(e: &mut Entity) {
    let Some(gv) = e.graph_view.clone() else {
        return;
    };
    match &gv {
        GraphView::Tangent { .. } | GraphView::Normal { .. } => {
            if let Some((tail, head)) = gv.segment() {
                e.pos = if tail.x.is_finite() && tail.y.is_finite() {
                    tail
                } else {
                    gv.touch()
                };
                if let Shape::Line { to } = &mut e.shape {
                    *to = head;
                }
            }
        }
        GraphView::Slope { .. } | GraphView::Integral { .. } => {
            let value = gv.value();
            if let Some(counter) = &mut e.counter {
                counter.value = value;
                let text = counter.render();
                if let Shape::Text { content, .. } = &mut e.shape {
                    *content = text;
                }
            }
            e.pos = gv.readout_pos();
        }
        GraphView::Area { .. } => {
            let (tris, rings) = gv.region();
            e.shape = Shape::Region { tris, rings };
        }
        GraphView::Mark { .. } => e.pos = gv.touch(),
    }
}

fn sample_graph(graph: &crate::primitives::GraphFn) -> Vec<Vec2> {
    const SAMPLES: usize = 600;
    let mut points = Vec::with_capacity(SAMPLES + 1);
    for i in 0..=SAMPLES {
        let x = graph.x0 + (graph.x1 - graph.x0) * i as f32 / SAMPLES as f32;
        let point = graph.point(x);
        if point.x.is_finite() && point.y.is_finite() {
            points.push(point);
        }
    }
    points
}

fn parameter_state(scene: &Scene, binding: &ParameterBinding) -> Option<(f32, f32, f32)> {
    let source = scene.get(&binding.source)?;
    let parameter = source.parameter?;
    let value = source
        .counter
        .as_ref()?
        .value
        .clamp(parameter.min, parameter.max);
    Some((value, parameter.min, parameter.max))
}

fn mapped_parameter_value(map: &ParameterMap, p: f32, min: f32, max: f32) -> f32 {
    match map {
        ParameterMap::Range { from, to } => {
            let u = ((p - min) / (max - min)).clamp(0.0, 1.0);
            from + (to - from) * u
        }
        ParameterMap::Formula(node) => node.eval(0.0, p),
    }
}

fn apply_parameter_binding(scene: &mut Scene, binding: &ParameterBinding) {
    let Some((p, min, max)) = parameter_state(scene, binding) else {
        return;
    };
    let Some(target) = scene.get_mut(&binding.target) else {
        return;
    };
    if binding.property == BoundProperty::Formula {
        let ParameterMap::Formula(node) = &binding.map else {
            return;
        };
        let Some(graph) = target.graph.as_mut() else {
            return;
        };
        graph.src = GraphSrc::ParameterExpr {
            node: node.clone(),
            p,
        };
        target.shape = Shape::Polyline {
            pts: sample_graph(graph),
        };
        return;
    }

    let value = mapped_parameter_value(&binding.map, p, min, max);
    if !value.is_finite() {
        return;
    }
    match binding.property {
        BoundProperty::X => {
            if let Some(view) = target.graph_view.as_mut() {
                view.set_x(value);
            } else {
                target.pos.x = value;
            }
        }
        BoundProperty::Y => target.pos.y = value,
        BoundProperty::Opacity => target.opacity = value.clamp(0.0, 1.0),
        BoundProperty::Scale => target.scale = value,
        BoundProperty::Rot => target.rot = value,
        BoundProperty::Hue => {
            target.hue = Some(value);
            let color = crate::style::hsl(value, 1.0, 0.6);
            target.color = color;
            if target.stroke.outline_color.is_some() {
                target.stroke.outline_color = Some(color);
            }
        }
        BoundProperty::Value => {
            if let Some(counter) = &mut target.counter {
                counter.value = value;
                if let Shape::Text { content, .. } = &mut target.shape {
                    *content = counter.render();
                }
            }
        }
        BoundProperty::Trace => target.trace = value.clamp(0.0, 1.0),
        BoundProperty::Formula => {}
    }
}

fn apply_parameter_bindings(scene: &mut Scene) {
    let bindings = scene.parameter_bindings.clone();
    // A changing plot must be rebuilt before its analysis views copy the source.
    for binding in bindings
        .iter()
        .filter(|binding| binding.property == BoundProperty::Formula)
    {
        apply_parameter_binding(scene, binding);
    }
    for binding in bindings
        .iter()
        .filter(|binding| binding.property != BoundProperty::Formula)
    {
        apply_parameter_binding(scene, binding);
    }
}

fn sync_parameter_widgets(scene: &mut Scene) {
    let parameters: Vec<_> = scene
        .entities
        .iter()
        .filter_map(|entity| {
            let parameter = entity.parameter?;
            let value = entity.counter.as_ref()?.value;
            Some((entity.id.clone(), parameter, value))
        })
        .collect();
    for (id, parameter, value) in parameters {
        let Some((left, right)) = scene.get(&format!("{id}.track")).and_then(|track| {
            let Shape::Line { to } = track.shape else {
                return None;
            };
            Some((track.pos, to))
        }) else {
            continue;
        };
        let u = ((value - parameter.min) / (parameter.max - parameter.min)).clamp(0.0, 1.0);
        let live = left.lerp(right, u);
        if let Some(fill) = scene.get_mut(&format!("{id}.fill")) {
            fill.pos = left;
            if let Shape::Line { to } = &mut fill.shape {
                *to = live;
            }
        }
        if let Some(dot) = scene.get_mut(&format!("{id}.dot")) {
            dot.pos = live;
        }
    }
}

fn sync_parameter_graph_views(scene: &mut Scene) {
    let bound_plots: Vec<_> = scene
        .parameter_bindings
        .iter()
        .filter(|binding| binding.property == BoundProperty::Formula)
        .map(|binding| binding.target.as_str())
        .collect();
    let updates: Vec<_> = scene
        .entities
        .iter()
        .enumerate()
        .filter_map(|(index, entity)| {
            let source = entity.graph_source.as_ref()?;
            if !bound_plots.iter().any(|bound| *bound == source) {
                return None;
            }
            let graph = scene.get(source)?.graph.clone()?;
            Some((index, graph))
        })
        .collect();
    for (index, graph) in updates {
        if let Some(view) = scene.entities[index].graph_view.as_mut() {
            view.set_graph(graph);
            refresh_graph_view(&mut scene.entities[index]);
        }
    }
}

fn linked_boundary_trim(e: &Entity, dir_world: Vec2, fallback: f32) -> f32 {
    match &e.shape {
        Shape::Circle { r } => r * e.scale.abs(),
        Shape::Rect { w, h } => {
            let (sn, cs) = (-e.rot).to_radians().sin_cos();
            let d = Vec2::new(
                dir_world.x * cs - dir_world.y * sn,
                dir_world.x * sn + dir_world.y * cs,
            );
            let hw = w * e.scale.abs() * 0.5;
            let hh = h * e.scale.abs() * 0.5;
            let tx = if d.x.abs() > 1e-5 {
                hw / d.x.abs()
            } else {
                f32::INFINITY
            };
            let ty = if d.y.abs() > 1e-5 {
                hh / d.y.abs()
            } else {
                f32::INFINITY
            };
            tx.min(ty)
        }
        _ => fallback,
    }
}

/// Whether two shapes have a topology-preserving direct interpolation. Other
/// pairs still work through `become`, but use its local fade/swap/fade fallback.
pub(crate) fn shape_transition_compatible(from: &Shape, to: &Shape) -> bool {
    match (from, to) {
        (Shape::Circle { .. }, Shape::Circle { .. })
        | (Shape::Rect { .. }, Shape::Rect { .. })
        | (Shape::Line { .. }, Shape::Line { .. })
        | (Shape::Arrow { .. }, Shape::Arrow { .. })
        | (Shape::Curve { .. }, Shape::Curve { .. })
        | (Shape::Arc { .. }, Shape::Arc { .. }) => true,
        (Shape::Coil { turns: a, .. }, Shape::Coil { turns: b, .. }) => a == b,
        (Shape::Polygon { pts: a }, Shape::Polygon { pts: b })
        | (Shape::Polyline { pts: a }, Shape::Polyline { pts: b }) => {
            a.len() == b.len() && !a.is_empty()
        }
        _ => false,
    }
}

fn lerp_shape(from: &Shape, to: &Shape, u: f32) -> Option<Shape> {
    let f = |a: f32, b: f32| a + (b - a) * u;
    let v = |a: Vec2, b: Vec2| a.lerp(b, u);
    Some(match (from, to) {
        (Shape::Circle { r: a }, Shape::Circle { r: b }) => Shape::Circle { r: f(*a, *b) },
        (Shape::Rect { w: aw, h: ah }, Shape::Rect { w: bw, h: bh }) => Shape::Rect {
            w: f(*aw, *bw),
            h: f(*ah, *bh),
        },
        (Shape::Line { to: a }, Shape::Line { to: b }) => Shape::Line { to: v(*a, *b) },
        (Shape::Arrow { to: a }, Shape::Arrow { to: b }) => Shape::Arrow { to: v(*a, *b) },
        (
            Shape::Curve {
                ctrl: ac,
                to: at,
                arrow: aa,
            },
            Shape::Curve {
                ctrl: bc,
                to: bt,
                arrow: ba,
            },
        ) if aa == ba => Shape::Curve {
            ctrl: v(*ac, *bc),
            to: v(*at, *bt),
            arrow: *aa,
        },
        (Shape::Coil { to: a, turns: at }, Shape::Coil { to: b, turns: bt }) if at == bt => {
            Shape::Coil {
                to: v(*a, *b),
                turns: *at,
            }
        }
        (Shape::Polygon { pts: a }, Shape::Polygon { pts: b }) if a.len() == b.len() => {
            Shape::Polygon {
                pts: a.iter().zip(b).map(|(a, b)| v(*a, *b)).collect(),
            }
        }
        (Shape::Polyline { pts: a }, Shape::Polyline { pts: b }) if a.len() == b.len() => {
            Shape::Polyline {
                pts: a.iter().zip(b).map(|(a, b)| v(*a, *b)).collect(),
            }
        }
        (
            Shape::Arc {
                r: ar,
                inner: ai,
                start: ast,
                sweep: asw,
            },
            Shape::Arc {
                r: br,
                inner: bi,
                start: bst,
                sweep: bsw,
            },
        ) => Shape::Arc {
            r: f(*ar, *br),
            inner: f(*ai, *bi),
            start: f(*ast, *bst),
            sweep: f(*asw, *bsw),
        },
        _ => return None,
    })
}

fn lerp_color(a: Color, b: Color, u: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * u,
        a.g + (b.g - a.g) * u,
        a.b + (b.b - a.b) * u,
        a.a + (b.a - a.a) * u,
    )
}

fn apply_become_visual(entity: &mut Entity, from: &Entity, to: &Entity, u: f32, crossfade: bool) {
    if u >= 1.0 {
        entity.shape = to.shape.clone();
    } else if crossfade {
        entity.shape = if u < 0.5 {
            from.shape.clone()
        } else {
            to.shape.clone()
        };
    } else if let Some(shape) = lerp_shape(&from.shape, &to.shape, u) {
        entity.shape = shape;
    }

    entity.stroke.width = from.stroke.width + (to.stroke.width - from.stroke.width) * u;
    entity.stroke.fill = if u < 0.5 {
        from.stroke.fill
    } else {
        to.stroke.fill
    };
    entity.stroke.outline = if u < 0.5 {
        from.stroke.outline
    } else {
        to.stroke.outline
    };
    entity.stroke.outline_color = match (from.stroke.outline_color, to.stroke.outline_color) {
        (Some(a), Some(b)) => Some(lerp_color(a, b, u)),
        (a, b) => {
            if u < 0.5 {
                a
            } else {
                b
            }
        }
    };
    entity.glow = from.glow + (to.glow - from.glow) * u;
    entity.corner_radius = from.corner_radius + (to.corner_radius - from.corner_radius) * u;
    entity.wrap = match (from.wrap, to.wrap) {
        (Some(a), Some(b)) => Some(a + (b - a) * u),
        (a, b) => {
            if u < 0.5 {
                a
            } else {
                b
            }
        }
    };
    entity.dash = match (from.dash, to.dash) {
        (Some((ad, ag)), Some((bd, bg))) => Some((ad + (bd - ad) * u, ag + (bg - ag) * u)),
        (a, b) => {
            if u < 0.5 {
                a
            } else {
                b
            }
        }
    };
    if u >= 0.5 {
        entity.font = to.font;
        entity.align = to.align;
        entity.z = to.z;
        entity.type_cursor = to.type_cursor;
    }
    if u >= 1.0 {
        entity.hue = to.hue;
    }
}

fn apply_become3_visual(
    entity: &mut crate::primitives3d::Entity3D,
    from: &crate::primitives3d::Entity3D,
    to: &crate::primitives3d::Entity3D,
    morph: Option<&crate::primitives3d::Morph3>,
    u: f32,
    crossfade: bool,
) {
    use crate::primitives3d::{Morph3Kind, Shape3D};
    if u >= 1.0 {
        entity.shape = to.shape.clone();
        entity.thickness = to.thickness;
        entity.surf = to.surf.clone();
        entity.morph3 = to.morph3.clone();
        return;
    }
    if let Some(morph) = morph {
        let pts = morph
            .from
            .iter()
            .zip(&morph.to)
            .map(|(a, b)| *a + (*b - *a) * u)
            .collect();
        entity.shape = match morph.kind {
            Morph3Kind::Path => Shape3D::Path { points: pts },
            Morph3Kind::Surface { nu, nv } => Shape3D::Surface { pts, nu, nv },
        };
    } else if crossfade {
        entity.shape = if u < 0.5 {
            from.shape.clone()
        } else {
            to.shape.clone()
        };
    }
    entity.thickness = from.thickness + (to.thickness - from.thickness) * u;
    entity.finish = if u < 0.5 { from.finish } else { to.finish };
}

impl Timeline {
    fn entity3_at_tracks(
        &self,
        base: &Scene,
        id: &str,
        t: f32,
    ) -> Option<crate::primitives3d::Entity3D> {
        let mut entity = base.get_3d(id)?.clone();
        for prop in [Prop::Pos, Prop::Scale, Prop::Rot3, Prop::Orient3] {
            let Some(base_value) = get_prop(base, id, prop) else {
                continue;
            };
            let value = self.value_at(id, prop, base_value, t);
            let mut scratch = Scene::new();
            scratch.add_3d(entity);
            set_prop(&mut scratch, id, prop, value);
            entity = scratch.entities_3d.remove(0);
        }
        Some(entity)
    }

    /// Replace a cached image path in future shape events after the player has
    /// resolved semantic LaTeX colours through the selected template.
    pub fn remap_image_path(&mut self, from: &str, to: &str) {
        for event in &mut self.events {
            let remap = |shape: &mut Shape| {
                if let Shape::Image { path, .. } = shape {
                    if path == from {
                        *path = to.to_string();
                    }
                }
            };
            match event {
                TimelineEvent::Shape { shape, .. } => remap(shape),
                TimelineEvent::Become {
                    from: source,
                    to: target,
                    ..
                } => {
                    remap(&mut source.shape);
                    remap(&mut target.shape);
                }
                _ => {}
            }
        }
    }

    /// Image paths introduced by future shape events, for texture preloading.
    pub fn event_image_paths(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|event| match event {
                TimelineEvent::Shape {
                    shape: Shape::Image { path, .. },
                    ..
                } => Some(path.clone()),
                TimelineEvent::Become { to, .. } => match &to.shape {
                    Shape::Image { path, .. } => Some(path.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    /// Resolve absolute-time track specs against the base scene.
    ///
    /// One chronological pass per (entity, property) pins each track's `from`
    /// to the previous track's end (or the base value), turns `Rel` deltas
    /// into absolute targets, and gives `Revert` the value the property had
    /// before the preceding track began.
    ///
    /// Panics on an unknown entity id: better to fail at build time than to
    /// render a movie with a silent no-op animation.
    pub fn resolve(
        base: &Scene,
        specs: Vec<TrackSpec>,
        mut events: Vec<TextEvent>,
        dur: f32,
    ) -> Timeline {
        let mut grouped: HashMap<(String, Prop), Vec<TrackSpec>> = HashMap::new();
        for s in specs {
            assert!(
                base.contains(&s.id),
                "animation references unknown entity id {:?}",
                s.id
            );
            grouped.entry((s.id.clone(), s.prop)).or_default().push(s);
        }

        let mut tracks: HashMap<(String, Prop), Vec<Track>> = HashMap::new();
        for ((id, prop), mut specs) in grouped {
            specs.sort_by(|a, b| a.start.total_cmp(&b.start));
            let base_val = get_prop(base, &id, prop)
                .unwrap_or_else(|| panic!("entity {id:?} has no property {prop:?}"));
            let mut cur = base_val;
            let mut prev_from: Option<Value> = None;
            let mut resolved = Vec::with_capacity(specs.len());
            for s in specs {
                let from = cur;
                let to = match s.target {
                    TargetValue::Abs(v) => v,
                    TargetValue::Rel(v) => from.add(v),
                    TargetValue::RotateAround { pivot, degrees } => match from {
                        Value::V(point) => {
                            let (sn, cs) = degrees.to_radians().sin_cos();
                            let d = point - pivot;
                            Value::V(pivot + Vec2::new(d.x * cs - d.y * sn, d.x * sn + d.y * cs))
                        }
                        _ => from,
                    },
                    TargetValue::RotateAround3 {
                        pivot,
                        axis,
                        degrees,
                    } => match from {
                        Value::V3(point) => {
                            let q = Quat::from_axis_angle(
                                axis.normalize_or_zero(),
                                degrees.to_radians(),
                            );
                            Value::V3(pivot + q * (point - pivot))
                        }
                        _ => from,
                    },
                    TargetValue::Revert => prev_from.unwrap_or(base_val),
                };
                resolved.push(Track {
                    from,
                    to,
                    start: s.start,
                    dur: s.dur,
                    easing: s.easing,
                });
                prev_from = Some(from);
                cur = to;
            }
            tracks.insert((id, prop), resolved);
        }

        events.sort_by(|a, b| a.at().total_cmp(&b.at()));
        Timeline {
            tracks,
            events,
            dur,
        }
    }

    fn value_at(&self, id: &str, prop: Prop, base: Value, t: f32) -> Value {
        let Some(tracks) = self.tracks.get(&(id.to_string(), prop)) else {
            return base;
        };
        let mut value = base;
        for tr in tracks {
            if t < tr.start {
                break;
            }
            if t < tr.start + tr.dur && tr.dur > 0.0 {
                let u = tr.easing.apply((t - tr.start) / tr.dur);
                return Value::lerp(tr.from, tr.to, u);
            }
            value = tr.to;
        }
        value
    }

    /// Evaluate the world at absolute time `t`: a fresh copy of the base
    /// scene with every animated property set. Pure — call with any `t` in
    /// any order.
    pub fn apply(&self, base: &Scene, t: f32) -> Scene {
        let mut scene = base.clone();

        for (id, prop) in self.tracks.keys() {
            if let Some(base_val) = get_prop(base, id, *prop) {
                let v = self.value_at(id, *prop, base_val, t);
                set_prop(&mut scene, id, *prop, v);
            }
        }

        for ev in &self.events {
            if ev.at() > t {
                break;
            }
            match ev {
                TimelineEvent::Text { id, content, .. } => {
                    if let Some(e) = scene.get_mut(id) {
                        if let Shape::Text { content: text, .. } = &mut e.shape {
                            *text = content.clone();
                        }
                    }
                }
                TimelineEvent::Shape { id, shape, .. } => {
                    if let Some(e) = scene.get_mut(id) {
                        e.shape = shape.clone();
                    }
                }
                TimelineEvent::Attachment {
                    id, target, offset, ..
                } => {
                    if let Some(e) = scene.get_mut(id) {
                        e.follow = target.clone().map(|target| (target, *offset));
                    }
                }
                TimelineEvent::Become {
                    id,
                    from,
                    to,
                    at,
                    dur,
                    easing,
                    crossfade,
                } => {
                    let u = if *dur <= 0.0 {
                        1.0
                    } else {
                        easing.apply(((t - *at) / *dur).clamp(0.0, 1.0))
                    };
                    if let Some(e) = scene.get_mut(id) {
                        apply_become_visual(e, from, to, u, *crossfade);
                    }
                }
                TimelineEvent::Attachment3 {
                    id,
                    target,
                    offset,
                    rigid,
                    relative_orientation,
                    ..
                } => {
                    if let Some(e) = scene.get_3d_mut(id) {
                        e.follow = target.clone().map(|target| (target, *offset));
                        e.follow_local = target.is_some() && *rigid;
                        e.follow_orientation = if target.is_some() && *rigid {
                            *relative_orientation
                        } else {
                            Quat::IDENTITY
                        };
                    }
                }
                TimelineEvent::Travel3 {
                    id,
                    path,
                    at,
                    dur,
                    easing,
                } => {
                    let later_position_track =
                        self.tracks
                            .get(&(id.clone(), Prop::Pos))
                            .and_then(|tracks| {
                                tracks.iter().find(|track| track.start + 1e-5 >= *at + *dur)
                            });
                    if let Some(track) = later_position_track {
                        if t >= track.start + track.dur {
                            continue;
                        }
                    }
                    let u = if *dur <= 0.0 {
                        1.0
                    } else {
                        easing.apply(((t - *at) / *dur).clamp(0.0, 1.0))
                    };
                    let path_entity = if t < *at + *dur {
                        scene.get_3d(path).cloned()
                    } else {
                        self.entity3_at_tracks(base, path, *at + *dur)
                    };
                    if let Some(path_entity) = path_entity {
                        let points: Option<Vec<Vec3>> = match &path_entity.shape {
                            crate::primitives3d::Shape3D::Line { to }
                            | crate::primitives3d::Shape3D::Arrow { to } => Some(vec![
                                path_entity.world_point(Vec3::ZERO),
                                path_entity.world_point(*to),
                            ]),
                            crate::primitives3d::Shape3D::Path { points } => Some(
                                points
                                    .iter()
                                    .map(|point| path_entity.world_point(*point))
                                    .collect(),
                            ),
                            _ => None,
                        };
                        if let (Some(points), Some(entity)) = (points, scene.get_3d_mut(id)) {
                            let endpoint = point_along_runtime3(&points, u);
                            entity.pos = if let Some(track) =
                                later_position_track.filter(|track| t >= track.start)
                            {
                                let tu = if track.dur <= 0.0 {
                                    1.0
                                } else {
                                    track
                                        .easing
                                        .apply(((t - track.start) / track.dur).clamp(0.0, 1.0))
                                };
                                match track.to {
                                    Value::V3(target) => endpoint.lerp(target, tu),
                                    _ => endpoint,
                                }
                            } else {
                                endpoint
                            };
                        }
                    }
                }
                TimelineEvent::Become3 {
                    id,
                    from,
                    to,
                    morph,
                    at,
                    dur,
                    easing,
                    crossfade,
                } => {
                    let u = if *dur <= 0.0 {
                        1.0
                    } else {
                        easing.apply(((t - *at) / *dur).clamp(0.0, 1.0))
                    };
                    if let Some(e) = scene.get_3d_mut(id) {
                        apply_become3_visual(e, from, to, morph.as_ref(), u, *crossfade);
                    }
                }
            }
        }

        // A parameter is an ordinary animated counter; these pure connections
        // turn its current value into a coordinated family of visual states.
        apply_parameter_bindings(&mut scene);
        sync_parameter_widgets(&mut scene);
        sync_parameter_graph_views(&mut scene);

        // --- constraint resolution, each a pure function of t ---

        // 1. Derived constructions: recompute an entity from its deps' current
        //    positions (e.g. a circumcircle tracks its three vertices). A few
        //    passes let a construction that depends on another settle.
        for _ in 0..3 {
            for i in 0..scene.entities.len() {
                let Some(f) = scene.entities[i].derive else {
                    continue;
                };
                let deps = scene.entities[i].deps.clone();
                let mut pts = Vec::with_capacity(deps.len());
                let mut ok = true;
                for d in &deps {
                    match scene.get(d) {
                        Some(e) => pts.push(e.pos),
                        None => {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok {
                    f(&mut scene.entities[i], &pts);
                }
            }
        }

        // 2. Linked edges: endpoints follow two entities, trimmed inward, so an
        //    edge/segment reflows when its endpoints (including derived ones)
        //    move.
        for i in 0..scene.entities.len() {
            let Some(link) = scene.entities[i].link.clone() else {
                continue;
            };
            let (Some(a_entity), Some(b_entity)) = (scene.get(&link.from), scene.get(&link.to))
            else {
                continue;
            };
            let (a, b) = (a_entity.pos, b_entity.pos);
            let dir = (b - a).normalize_or_zero();
            let trim_from = if link.auto_trim {
                linked_boundary_trim(a_entity, dir, link.trim_from)
            } else {
                link.trim_from
            };
            let trim_to = if link.auto_trim {
                linked_boundary_trim(b_entity, -dir, link.trim_to)
            } else {
                link.trim_to
            };
            let from = a + dir * trim_from;
            let to = b - dir * trim_to;
            scene.entities[i].pos = from;
            match &mut scene.entities[i].shape {
                Shape::Line { to: t } | Shape::Arrow { to: t } => *t = to,
                Shape::Curve { ctrl, to: t, .. } => {
                    *t = to;
                    let delta = to - from;
                    let perp = Vec2::new(-delta.y, delta.x).normalize_or_zero();
                    *ctrl = (from + to) * 0.5 + perp * link.bend;
                }
                _ => {}
            }
        }

        // 3. Followers pin to a target's position + offset and inherit its
        //    opacity. Two passes so a follower-of-a-follower settles. Last, so
        //    labels sit on the final positions of everything above.
        for _ in 0..2 {
            for i in 0..scene.entities.len() {
                let Some((target, offset)) = scene.entities[i].follow.clone() else {
                    continue;
                };
                let Some(te) = scene.get(&target) else {
                    continue;
                };
                let (target_pos, target_opacity) = (te.pos, te.opacity);
                let own = match self.value_at(
                    &scene.entities[i].id,
                    Prop::Opacity,
                    Value::F(base.entities[i].opacity),
                    t,
                ) {
                    Value::F(o) => o,
                    _ => base.entities[i].opacity,
                };
                scene.entities[i].pos = target_pos + offset;
                scene.entities[i].opacity = own * target_opacity;
            }
        }

        // --- 3D constraint resolution (mirror of the 2D passes, in Vec3) ---
        // Derived 3D points (midpoint3, …) first, a few passes to settle.
        for _ in 0..3 {
            for i in 0..scene.entities_3d.len() {
                let Some(f) = scene.entities_3d[i].derive else {
                    continue;
                };
                let deps = scene.entities_3d[i].deps.clone();
                let mut pts = Vec::with_capacity(deps.len());
                let mut ok = true;
                for d in &deps {
                    match scene.get_3d(d) {
                        Some(e) => pts.push(e.pos),
                        None => {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok {
                    f(&mut scene.entities_3d[i], &pts);
                }
            }
        }
        // 3D followers (follow3): track target position + offset, inherit opacity.
        for _ in 0..2 {
            for i in 0..scene.entities_3d.len() {
                let Some((target, offset)) = scene.entities_3d[i].follow.clone() else {
                    continue;
                };
                let Some(te) = scene.get_3d(&target).cloned() else {
                    continue;
                };
                if scene.entities_3d[i].follow_local {
                    scene.entities_3d[i].pos = te.pos + te.rotation_quat() * offset;
                    let child = &mut scene.entities_3d[i];
                    let r = Vec3::new(
                        child.rotation.x.to_radians(),
                        child.rotation.y.to_radians(),
                        child.rotation.z.to_radians(),
                    );
                    let authored_euler = Quat::from_euler(EulerRot::ZYX, r.z, r.y, r.x);
                    child.orientation =
                        te.rotation_quat() * child.follow_orientation * authored_euler.inverse();
                } else {
                    scene.entities_3d[i].pos = te.pos + offset;
                }
                scene.entities_3d[i].opacity *= te.opacity;
            }
        }

        // Live 3D projections and edges resolve after followers so they see
        // the final relationship positions for this frame.
        for i in 0..scene.entities_3d.len() {
            if let Some((source, plane)) = scene.entities_3d[i].projection.clone() {
                if let Some(source) = scene.get_3d(&source) {
                    let p = source.pos;
                    scene.entities_3d[i].pos = match plane {
                        crate::primitives3d::ProjectionPlane3::Xy => Vec3::new(p.x, p.y, 0.0),
                        crate::primitives3d::ProjectionPlane3::Xz => Vec3::new(p.x, 0.0, p.z),
                        crate::primitives3d::ProjectionPlane3::Yz => Vec3::new(0.0, p.y, p.z),
                    };
                }
            }
        }
        for i in 0..scene.entities_3d.len() {
            let Some(link) = scene.entities_3d[i].link.clone() else {
                continue;
            };
            let Some(a) = scene.get_3d(&link.from).map(|entity| entity.pos) else {
                continue;
            };
            let Some(b) = scene.get_3d(&link.to).map(|entity| entity.pos) else {
                continue;
            };
            let direction = (b - a).normalize_or_zero();
            let from = a + direction * link.trim;
            let to = b - direction * link.trim;
            scene.entities_3d[i].pos = from;
            scene.entities_3d[i].shape = crate::primitives3d::Shape3D::Line { to: to - from };
        }

        scene
    }
}

fn point_along_runtime3(points: &[Vec3], u: f32) -> Vec3 {
    if points.len() <= 1 {
        return points.first().copied().unwrap_or(Vec3::ZERO);
    }
    let lengths: Vec<f32> = std::iter::once(0.0)
        .chain(points.windows(2).scan(0.0, |total, pair| {
            *total += pair[0].distance(pair[1]);
            Some(*total)
        }))
        .collect();
    let total = *lengths.last().unwrap_or(&0.0);
    if total <= 1e-6 {
        return points[0];
    }
    let distance = total * u.clamp(0.0, 1.0);
    let segment = lengths
        .windows(2)
        .position(|span| distance <= span[1])
        .unwrap_or(points.len() - 2);
    let span = (lengths[segment + 1] - lengths[segment]).max(1e-6);
    points[segment].lerp(points[segment + 1], (distance - lengths[segment]) / span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::easing::Easing;
    use crate::primitives::Entity;
    use macroquad::prelude::Vec2;

    fn scene_with_dot() -> Scene {
        let mut s = Scene::new();
        s.add(Entity::new(
            "dot",
            Shape::Circle { r: 10.0 },
            Vec2::new(0.0, 0.0),
            Color::new(0.0, 0.0, 0.0, 1.0),
        ));
        s
    }

    fn spec(prop: Prop, target: TargetValue, start: f32, dur: f32) -> TrackSpec {
        TrackSpec {
            id: "dot".into(),
            prop,
            target,
            start,
            dur,
            easing: Easing::Linear,
        }
    }

    #[test]
    fn abs_track_interpolates_and_holds() {
        let base = scene_with_dot();
        let tl = Timeline::resolve(
            &base,
            vec![spec(
                Prop::Pos,
                TargetValue::Abs(Value::V(Vec2::new(100.0, 0.0))),
                1.0,
                2.0,
            )],
            vec![],
            5.0,
        );
        assert_eq!(tl.apply(&base, 0.0).get("dot").unwrap().pos.x, 0.0);
        assert_eq!(tl.apply(&base, 2.0).get("dot").unwrap().pos.x, 50.0);
        assert_eq!(tl.apply(&base, 4.0).get("dot").unwrap().pos.x, 100.0);
    }

    #[test]
    fn rel_chains_from_previous_end_and_revert_restores() {
        let base = scene_with_dot();
        let tl = Timeline::resolve(
            &base,
            vec![
                spec(
                    Prop::Pos,
                    TargetValue::Rel(Value::V(Vec2::new(100.0, 0.0))),
                    0.0,
                    1.0,
                ),
                spec(Prop::Pos, TargetValue::Revert, 2.0, 1.0),
            ],
            vec![],
            5.0,
        );
        assert_eq!(tl.apply(&base, 1.5).get("dot").unwrap().pos.x, 100.0);
        assert_eq!(tl.apply(&base, 4.0).get("dot").unwrap().pos.x, 0.0);
    }

    #[test]
    fn evaluation_is_order_independent() {
        let base = scene_with_dot();
        let tl = Timeline::resolve(
            &base,
            vec![spec(
                Prop::Opacity,
                TargetValue::Abs(Value::F(0.0)),
                0.5,
                1.0,
            )],
            vec![],
            3.0,
        );
        let forward: Vec<f32> = (0..30)
            .map(|i| tl.apply(&base, i as f32 * 0.1).get("dot").unwrap().opacity)
            .collect();
        let mut backward: Vec<f32> = (0..30)
            .rev()
            .map(|i| tl.apply(&base, i as f32 * 0.1).get("dot").unwrap().opacity)
            .collect();
        backward.reverse();
        assert_eq!(forward, backward);
    }

    #[test]
    fn easing_endpoints_are_exact() {
        use crate::easing::Easing::*;
        for e in [
            Linear, InQuad, OutQuad, InOutQuad, InCubic, OutCubic, InOutCubic, OutBack, OutElastic,
            OutBounce,
        ] {
            assert!((e.apply(0.0)).abs() < 1e-4, "{e:?} at 0");
            assert!((e.apply(1.0) - 1.0).abs() < 1e-4, "{e:?} at 1");
        }
    }

    #[test]
    #[should_panic(expected = "unknown entity")]
    fn unknown_id_fails_at_resolve_not_playback() {
        let base = scene_with_dot();
        Timeline::resolve(
            &base,
            vec![TrackSpec {
                id: "typo".into(),
                prop: Prop::Opacity,
                target: TargetValue::Abs(Value::F(0.0)),
                start: 0.0,
                dur: 1.0,
                easing: Easing::Linear,
            }],
            vec![],
            1.0,
        );
    }

    #[test]
    fn vec3_track_is_deterministic() {
        let mut base = Scene::new();
        base.add_3d(crate::primitives3d::Entity3D::new(
            "cube",
            crate::primitives3d::Shape3D::Cube { size: Vec3::ONE },
            Vec3::ZERO,
            Color::new(0.0, 1.0, 1.0, 1.0),
        ));
        let tl = Timeline::resolve(
            &base,
            vec![TrackSpec {
                id: "cube".into(),
                prop: Prop::Pos,
                target: TargetValue::Abs(Value::V3(Vec3::new(2.0, 4.0, 6.0))),
                start: 0.0,
                dur: 2.0,
                easing: Easing::Linear,
            }],
            vec![],
            2.0,
        );
        assert_eq!(
            tl.apply(&base, 1.0).get_3d("cube").unwrap().pos,
            Vec3::new(1.0, 2.0, 3.0)
        );
        assert_eq!(
            tl.apply(&base, 1.0).get_3d("cube").unwrap().pos,
            Vec3::new(1.0, 2.0, 3.0)
        );
    }
}
