//! Creator-facing visual correctness checks.
//!
//! V1 deliberately audits settled semantic checkpoints rather than every
//! transition frame. That makes off-canvas, safe-area, overlap, and readability
//! diagnostics useful without flagging intentional entrances or rewrite
//! crossfades. The timeline is stateless, so every checkpoint is reproducible.

use std::collections::BTreeSet;

use macroquad::prelude::{Vec2, Vec3};

use crate::movie::Movie;
use crate::primitives::{Align, Entity, Shape, TextRun};
use crate::primitives3d::Shape3D;
use crate::scene::{CreatorSafe, Scene};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VisualSeverity {
    Error,
    Warning,
}

impl VisualSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VisualIssue {
    OffCanvas,
    SafeArea,
    Overlap,
    Readability,
    /// Projected 3D content leaves its creator media rectangle while a camera
    /// transition is active.
    CameraBounds,
    /// An orbit/zoom/target transition is fast enough to read as a cut or jolt.
    CameraMotion,
    /// The camera eye enters drawable geometry.
    CameraPenetration,
    /// A live 3D relationship references a source that is no longer present.
    SpatialRelationship,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualDiagnostic {
    pub severity: VisualSeverity,
    pub issue: VisualIssue,
    pub format: String,
    pub stage: String,
    pub at: f32,
    pub entity: String,
    pub other: Option<String>,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    lo: Vec2,
    hi: Vec2,
}

impl Bounds {
    fn width(self) -> f32 {
        (self.hi.x - self.lo.x).max(0.0)
    }

    fn height(self) -> f32 {
        (self.hi.y - self.lo.y).max(0.0)
    }

    fn area(self) -> f32 {
        self.width() * self.height()
    }

    fn intersects(self, other: Self) -> bool {
        self.lo.x < other.hi.x
            && self.hi.x > other.lo.x
            && self.lo.y < other.hi.y
            && self.hi.y > other.lo.y
    }

    fn intersection(self, other: Self) -> Option<Self> {
        let out = Self {
            lo: self.lo.max(other.lo),
            hi: self.hi.min(other.hi),
        };
        (out.width() > 0.0 && out.height() > 0.0).then_some(out)
    }

    fn contains(self, other: Self, tolerance: f32) -> bool {
        other.lo.x >= self.lo.x - tolerance
            && other.lo.y >= self.lo.y - tolerance
            && other.hi.x <= self.hi.x + tolerance
            && other.hi.y <= self.hi.y + tolerance
    }

    fn inset(self, amount: f32) -> Self {
        let max_x = self.width() * 0.22;
        let max_y = self.height() * 0.22;
        let d = Vec2::new(amount.min(max_x), amount.min(max_y));
        Self {
            lo: self.lo + d,
            hi: self.hi - d,
        }
    }
}

fn rotated_about(p: Vec2, center: Vec2, degrees: f32) -> Vec2 {
    if degrees.abs() < 1e-3 {
        return p;
    }
    let (sn, cs) = degrees.to_radians().sin_cos();
    let d = p - center;
    center + Vec2::new(d.x * cs - d.y * sn, d.x * sn + d.y * cs)
}

/// Logical bounding box matching the Creator Kit's `figure` fit estimate.
fn entity_bounds(e: &Entity) -> Option<Bounds> {
    let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    let mut any = false;
    let mut acc = |p: Vec2| {
        lo = lo.min(p);
        hi = hi.max(p);
        any = true;
    };
    let scale = e.scale.abs().max(0.001);
    let mut point = |p: Vec2| acc(rotated_about(p, e.pos, e.rot));
    match &e.shape {
        Shape::Circle { r } | Shape::Arc { r, .. } => {
            let r = *r * scale;
            point(e.pos + Vec2::splat(r));
            point(e.pos - Vec2::splat(r));
        }
        Shape::Rect { w, h } | Shape::Image { w, h, .. } => {
            let half = Vec2::new(*w, *h) * scale * 0.5;
            for d in [
                Vec2::new(-half.x, -half.y),
                Vec2::new(half.x, -half.y),
                Vec2::new(half.x, half.y),
                Vec2::new(-half.x, half.y),
            ] {
                point(e.pos + d);
            }
        }
        Shape::Line { to } | Shape::Arrow { to } | Shape::Coil { to, .. } => {
            point(e.pos);
            point(*to);
        }
        Shape::Curve { ctrl, to, .. } => {
            point(e.pos);
            point(*ctrl);
            point(*to);
        }
        Shape::Polyline { pts } | Shape::Polygon { pts } => {
            for p in pts {
                point(e.pos + *p * scale);
            }
        }
        Shape::Region { tris, rings } => {
            for tri in tris {
                for p in tri {
                    point(e.pos + *p * scale);
                }
            }
            for ring in rings {
                for p in ring {
                    point(e.pos + *p * scale);
                }
            }
        }
        Shape::Text { content, size } => {
            let em = *size * scale;
            let rough = content.chars().count().max(1) as f32 * em * 0.61;
            let width = e
                .wrap
                .map(|w| rough.min(w * scale))
                .unwrap_or(rough)
                .max(em * 0.5);
            let lines = e
                .wrap
                .map(|w| (rough / (w * scale).max(1.0)).ceil())
                .unwrap_or(1.0)
                .max(1.0);
            let height = em * 1.25 * lines;
            add_text_box(&mut point, e, width, height);
        }
        Shape::RichText { runs, size } => {
            let em = *size * scale;
            let rough = runs
                .iter()
                .map(|run| match run {
                    TextRun::Text(text) => text.chars().count() as f32 * em * 0.61,
                    TextRun::Math { w, .. } => *w * scale,
                })
                .sum::<f32>()
                .max(em);
            let line_height = runs
                .iter()
                .map(|run| match run {
                    TextRun::Text(_) => em * 1.25,
                    TextRun::Math { h, .. } => *h * scale,
                })
                .fold(em, f32::max);
            let width = e.wrap.map(|w| rough.min(w * scale)).unwrap_or(rough);
            let lines = e
                .wrap
                .map(|w| (rough / (w * scale).max(1.0)).ceil())
                .unwrap_or(1.0)
                .max(1.0);
            let height = line_height * lines;
            add_text_box(&mut point, e, width, height);
        }
    }
    if !any {
        return None;
    }
    let pad = (e.stroke.width * scale * 0.5).max(1.0);
    Some(Bounds {
        lo: lo - Vec2::splat(pad),
        hi: hi + Vec2::splat(pad),
    })
}

fn add_text_box(point: &mut impl FnMut(Vec2), e: &Entity, width: f32, height: f32) {
    let left = if e.align == Align::Left {
        0.0
    } else {
        -width * 0.5
    };
    for d in [
        Vec2::new(left, -height * 0.5),
        Vec2::new(left + width, -height * 0.5),
        Vec2::new(left + width, height * 0.5),
        Vec2::new(left, height * 0.5),
    ] {
        point(e.pos + d);
    }
}

fn visible(e: &Entity) -> bool {
    e.opacity > 0.05 && !e.id.starts_with("__")
}

fn semantic_tag(e: &Entity) -> bool {
    e.tags.iter().any(|tag| {
        tag.contains(".question")
            || tag.contains(".option")
            || tag.ends_with(".footer")
            || tag.contains(".endcard")
    })
}

fn primary(e: &Entity) -> bool {
    visible(e) && (!e.id.contains('.') || semantic_tag(e))
}

fn content(e: &Entity) -> bool {
    primary(e)
        && matches!(
            e.shape,
            Shape::Text { .. } | Shape::RichText { .. } | Shape::Image { .. }
        )
}

fn safe_rect(canvas: Vec2, safe: CreatorSafe) -> Bounds {
    let (lo, hi) = safe.rect(canvas).edges();
    Bounds { lo, hi }
}

fn safe_bounds_for(scene: &Scene, e: &Entity) -> Option<Bounds> {
    let mut owned = Vec::new();
    for (id, quiz) in &scene.quizzes {
        if e.tags.iter().any(|tag| tag == id) {
            owned.push(quiz.safe);
        }
    }
    for (id, creator) in &scene.creators {
        if e.tags.iter().any(|tag| tag == id) {
            owned.push(creator.safe);
        }
    }
    if owned.is_empty() {
        owned.extend(scene.quizzes.values().map(|quiz| quiz.safe));
        owned.extend(scene.creators.values().map(|creator| creator.safe));
    }
    let mut rects = owned
        .into_iter()
        .map(|safe| safe_rect(scene.canvas(), safe));
    let first = rects.next()?;
    rects.try_fold(first, |acc, rect| acc.intersection(rect))
}

fn effective_size(e: &Entity) -> Option<f32> {
    match &e.shape {
        Shape::Text { size, .. } | Shape::RichText { size, .. } => Some(*size * e.scale.abs()),
        Shape::Image { h, tint: true, .. } => Some(*h * e.scale.abs()),
        _ => None,
    }
}

fn settled_checkpoints(movie: &Movie, authored_end: f32) -> Vec<(f32, String)> {
    let mut marks = movie.marks.clone();
    marks.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    marks.dedup();
    if marks.is_empty() {
        return vec![(authored_end.max(0.0), "final".into())];
    }
    marks
        .iter()
        .enumerate()
        .map(|(index, (start, name))| {
            let end = marks
                .get(index + 1)
                .map(|(next, _)| (*next - 0.01).max(*start))
                .unwrap_or(authored_end.max(*start));
            (end, name.clone())
        })
        .collect()
}

fn stage_ranges(movie: &Movie, authored_end: f32) -> Vec<(f32, f32, String)> {
    let mut marks = movie.marks.clone();
    marks.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    marks.dedup();
    if marks.is_empty() {
        return vec![(0.0, authored_end.max(0.0), "story".into())];
    }
    marks
        .iter()
        .enumerate()
        .map(|(index, (start, name))| {
            let end = marks
                .get(index + 1)
                .map(|(next, _)| *next)
                .unwrap_or(authored_end.max(*start));
            (*start, end.max(*start), name.clone())
        })
        .collect()
}

fn bounds_corners(lo: Vec3, hi: Vec3) -> [Vec3; 8] {
    [
        Vec3::new(lo.x, lo.y, lo.z),
        Vec3::new(hi.x, lo.y, lo.z),
        Vec3::new(lo.x, hi.y, lo.z),
        Vec3::new(hi.x, hi.y, lo.z),
        Vec3::new(lo.x, lo.y, hi.z),
        Vec3::new(hi.x, lo.y, hi.z),
        Vec3::new(lo.x, hi.y, hi.z),
        Vec3::new(hi.x, hi.y, hi.z),
    ]
}

/// Unlike the settled 2D checks, camera checks deliberately sample the active
/// transitions. A camera can start and end safely while crossing through a
/// solid or producing a one-frame visual jolt between those endpoints.
fn audit_camera_transitions(
    movie: &Movie,
    format: &str,
    base: &Scene,
    timeline: &crate::timeline::Timeline,
    authored_end: f32,
    diagnostics: &mut Vec<VisualDiagnostic>,
    seen: &mut BTreeSet<(VisualIssue, String, Option<String>)>,
) {
    if base.get_3d(crate::movie::CAMERA3_ID).is_none() {
        return;
    }
    let pw = movie.width as f32;
    let ph = movie.height as f32;
    let aspect = pw / ph.max(1.0);

    for (start, end, stage) in stage_ranges(movie, authored_end) {
        let span = (end - start).max(0.0);
        let samples = ((span * 12.0).ceil() as usize).clamp(2, 48);
        let mut previous: Option<(f32, Vec3, Vec3, f32)> = None;
        for index in 0..=samples {
            let at = if samples == 0 {
                start
            } else {
                start + span * index as f32 / samples as f32
            };
            let scene = timeline.apply(base, at);
            let Some(camera) = scene.get_3d(crate::movie::CAMERA3_ID) else {
                continue;
            };
            let eye = crate::render3d::eye_of(camera);
            let target = camera.pos;
            let radius = camera.scale.max(0.01);

            let camera_active = previous.is_some_and(|(_, pe, ptarget, pradius)| {
                eye.distance(pe) > 1e-4
                    || target.distance(ptarget) > 1e-4
                    || (radius - pradius).abs() > 1e-4
            });
            if let Some((pt, pe, ptarget, pradius)) = previous {
                let dt = (at - pt).max(1.0 / 120.0);
                let view_a = (ptarget - pe).normalize_or_zero();
                let view_b = (target - eye).normalize_or_zero();
                let angular = view_a.dot(view_b).clamp(-1.0, 1.0).acos().to_degrees() / dt;
                let zoom = (radius / pradius.max(0.01)).abs().ln().abs() / dt;
                let target_speed = target.distance(ptarget) / radius.max(pradius).max(0.1) / dt;
                if angular > 300.0 || zoom > 4.0 || target_speed > 3.5 {
                    push_unique(
                        diagnostics,
                        seen,
                        VisualDiagnostic {
                            severity: VisualSeverity::Warning,
                            issue: VisualIssue::CameraMotion,
                            format: format.into(),
                            stage: stage.clone(),
                            at,
                            entity: crate::movie::CAMERA3_ID.into(),
                            other: None,
                            message: format!(
                                "3D camera changes too abruptly ({angular:.0}°/s orbit, {zoom:.1}/s zoom)"
                            ),
                            suggestion: "lengthen orbit3/view3, use smooth, or split the camera move into two readable beats"
                                .into(),
                        },
                    );
                }
            }
            previous = Some((at, eye, target, radius));

            if !camera_active {
                continue;
            }

            let media = scene
                .creator_media_rect()
                .map(|rect| {
                    let (lo, hi) = rect.edges();
                    Bounds { lo, hi }
                })
                .unwrap_or(Bounds {
                    lo: Vec2::ZERO,
                    hi: Vec2::new(pw, ph),
                });
            for entity in scene.entities_3d.iter().filter(|entity| {
                entity.id != crate::movie::CAMERA3_ID
                    && entity.opacity > 0.05
                    && !matches!(entity.shape, Shape3D::Grid { .. })
            }) {
                let Some((lo, hi)) = entity.world_bounds() else {
                    continue;
                };
                if eye.x >= lo.x
                    && eye.x <= hi.x
                    && eye.y >= lo.y
                    && eye.y <= hi.y
                    && eye.z >= lo.z
                    && eye.z <= hi.z
                {
                    push_unique(
                        diagnostics,
                        seen,
                        VisualDiagnostic {
                            severity: VisualSeverity::Error,
                            issue: VisualIssue::CameraPenetration,
                            format: format.into(),
                            stage: stage.clone(),
                            at,
                            entity: crate::movie::CAMERA3_ID.into(),
                            other: Some(entity.id.clone()),
                            message: format!("camera enters `{}`", entity.id),
                            suggestion: "increase camera radius or use view3 to refit the subject before orbiting"
                                .into(),
                        },
                    );
                }

                let projected: Vec<_> = bounds_corners(lo, hi)
                    .into_iter()
                    .filter_map(|point| crate::render3d::project(&scene, aspect, point, pw, ph))
                    .collect();
                if projected.len() < 4 {
                    continue;
                }
                let projected_bounds = Bounds {
                    lo: projected
                        .iter()
                        .fold(Vec2::splat(f32::MAX), |acc, p| acc.min(*p)),
                    hi: projected
                        .iter()
                        .fold(Vec2::splat(f32::MIN), |acc, p| acc.max(*p)),
                };
                if !media.contains(projected_bounds, 3.0) {
                    push_unique(
                        diagnostics,
                        seen,
                        VisualDiagnostic {
                            severity: VisualSeverity::Warning,
                            issue: VisualIssue::CameraBounds,
                            format: format.into(),
                            stage: stage.clone(),
                            at,
                            entity: entity.id.clone(),
                            other: None,
                            message: "leaves the creator media area during a 3D camera transition".into(),
                            suggestion: "use view3 after changing the subject, increase its duration, or reduce the orbit radius change"
                                .into(),
                        },
                    );
                }
            }
        }
    }
}

/// Audit settled story checkpoints for one already-lowered format.
pub fn visual_diagnostics(movie: &Movie, format: &str) -> Vec<VisualDiagnostic> {
    let (base, timeline) = movie.finalize();
    let authored_end = (timeline.dur - 1.0).max(0.0);
    let checkpoints = settled_checkpoints(movie, authored_end);
    let canvas = Bounds {
        lo: Vec2::ZERO,
        hi: Vec2::new(movie.width as f32, movie.height as f32),
    };
    let minimum_size = 14.0
        * ((movie.width.min(movie.height) as f32 / 720.0)
            .sqrt()
            .clamp(1.0, 1.35));
    let safe_tolerance = (movie.width.min(movie.height) as f32 * 0.008).max(3.0);
    let mut seen = BTreeSet::new();
    let mut diagnostics = Vec::new();

    for (at, stage) in checkpoints {
        let mut scene = timeline.apply(&base, at);
        // pin3/label3 positions are player-time projections. Resolve the same
        // hook here before auditing their 2D text boxes; their authored (0,0)
        // placeholder is intentionally not a screen position.
        let pins = scene.pins.clone();
        let aspect = movie.width as f32 / movie.height.max(1) as f32;
        for pin in pins {
            let world = match pin.target {
                crate::scene::Pin3Target::Point(point) => Some(point),
                crate::scene::Pin3Target::Entity(id) => scene.get_3d(&id).map(|entity| entity.pos),
            };
            let Some(world) = world else { continue };
            let Some(position) = crate::render3d::project(
                &scene,
                aspect,
                world,
                movie.width as f32,
                movie.height as f32,
            ) else {
                continue;
            };
            let world_scale = pin.world_height.and_then(|height| {
                let pixels = crate::render3d::projected_world_height(
                    &scene,
                    aspect,
                    world,
                    height,
                    movie.width as f32,
                    movie.height as f32,
                )?;
                let entity = scene.get(&pin.label)?;
                let em = match entity.shape {
                    Shape::Text { size, .. } | Shape::RichText { size, .. } => size,
                    _ => return None,
                };
                Some((pixels / em.max(1.0)).clamp(0.15, 8.0))
            });
            if let Some(entity) = scene.get_mut(&pin.label) {
                entity.pos = position + pin.offset;
                if let Some(scale) = world_scale {
                    entity.scale = scale;
                }
            }
        }
        for entity in &scene.entities_3d {
            let missing = if let Some((target, _)) = &entity.follow {
                scene.get_3d(target).is_none().then(|| target.clone())
            } else if let Some(link) = &entity.link {
                [link.from.as_str(), link.to.as_str()]
                    .into_iter()
                    .find(|id| scene.get_3d(id).is_none())
                    .map(str::to_string)
            } else if let Some((source, _)) = &entity.projection {
                scene.get_3d(source).is_none().then(|| source.clone())
            } else {
                None
            };
            if let Some(missing) = missing {
                push_unique(
                    &mut diagnostics,
                    &mut seen,
                    VisualDiagnostic {
                        severity: VisualSeverity::Error,
                        issue: VisualIssue::SpatialRelationship,
                        format: format.into(),
                        stage: stage.clone(),
                        at,
                        entity: entity.id.clone(),
                        other: Some(missing.clone()),
                        message: format!("references missing 3D source `{missing}`"),
                        suggestion: "keep relationship sources in the scene, or release the relationship before replacing them"
                            .into(),
                    },
                );
            }

            if entity.id == crate::movie::CAMERA3_ID
                || entity.opacity <= 0.05
                || matches!(entity.shape, Shape3D::Grid { .. })
            {
                continue;
            }
            let Some((lo, hi)) = entity.world_bounds() else {
                continue;
            };
            let projected: Vec<_> = bounds_corners(lo, hi)
                .into_iter()
                .filter_map(|point| {
                    crate::render3d::project(
                        &scene,
                        aspect,
                        point,
                        movie.width as f32,
                        movie.height as f32,
                    )
                })
                .collect();
            if projected.len() < 4 {
                continue;
            }
            let projected_bounds = Bounds {
                lo: projected
                    .iter()
                    .fold(Vec2::splat(f32::MAX), |acc, point| acc.min(*point)),
                hi: projected
                    .iter()
                    .fold(Vec2::splat(f32::MIN), |acc, point| acc.max(*point)),
            };
            let media = scene
                .creator_media_rect()
                .map(|rect| {
                    let (lo, hi) = rect.edges();
                    Bounds { lo, hi }
                })
                .unwrap_or(canvas);
            if !media.contains(projected_bounds, 3.0) {
                push_unique(
                    &mut diagnostics,
                    &mut seen,
                    VisualDiagnostic {
                        severity: VisualSeverity::Warning,
                        issue: VisualIssue::CameraBounds,
                        format: format.into(),
                        stage: stage.clone(),
                        at,
                        entity: entity.id.clone(),
                        other: None,
                        message: "is outside the 3D creator media area at the settled checkpoint"
                            .into(),
                        suggestion:
                            "use view3 on this object or its tag after the preceding spatial change"
                                .into(),
                    },
                );
            }
        }
        let candidates: Vec<_> = scene
            .entities
            .iter()
            .filter(|entity| primary(entity))
            .filter_map(|entity| entity_bounds(entity).map(|bounds| (entity, bounds)))
            .collect();

        for (entity, bounds) in &candidates {
            let completely_outside = !canvas.intersects(*bounds);
            let content_overflow = content(entity) && !canvas.contains(*bounds, 2.0);
            if completely_outside || content_overflow {
                push_unique(
                    &mut diagnostics,
                    &mut seen,
                    VisualDiagnostic {
                        severity: VisualSeverity::Error,
                        issue: VisualIssue::OffCanvas,
                        format: format.into(),
                        stage: stage.clone(),
                        at,
                        entity: entity.id.clone(),
                        other: None,
                        message: if completely_outside {
                            "is completely outside the canvas".into()
                        } else {
                            "extends beyond the canvas edge".into()
                        },
                        suggestion:
                            "position it with w/h/cx/cy or add a format-specific layout branch"
                                .into(),
                    },
                );
            }

            if content(entity) {
                if let Some(safe) = safe_bounds_for(&scene, entity) {
                    if !safe.contains(*bounds, safe_tolerance) {
                        push_unique(
                            &mut diagnostics,
                            &mut seen,
                            VisualDiagnostic {
                                severity: VisualSeverity::Warning,
                                issue: VisualIssue::SafeArea,
                                format: format.into(),
                                stage: stage.clone(),
                                at,
                                entity: entity.id.clone(),
                                other: None,
                                message: "extends outside the selected creator safe area".into(),
                                suggestion: "move it inward, shorten/wrap it, or choose the correct safe profile"
                                    .into(),
                            },
                        );
                    }
                }

                if let Some(size) = effective_size(entity) {
                    if size + 0.1 < minimum_size {
                        push_unique(
                            &mut diagnostics,
                            &mut seen,
                            VisualDiagnostic {
                                severity: VisualSeverity::Warning,
                                issue: VisualIssue::Readability,
                                format: format.into(),
                                stage: stage.clone(),
                                at,
                                entity: entity.id.clone(),
                                other: None,
                                message: format!(
                                    "is about {size:.1}px; target at least {minimum_size:.1}px for this format"
                                ),
                                suggestion: "increase its size, reduce its copy, or give this format more room"
                                    .into(),
                            },
                        );
                    }
                }
            }
        }

        let content_boxes: Vec<_> = candidates
            .iter()
            .filter(|(entity, _)| content(entity))
            .collect();
        for i in 0..content_boxes.len() {
            for j in i + 1..content_boxes.len() {
                let (a, ab) = content_boxes[i];
                let (b, bb) = content_boxes[j];
                let Some(overlap) = ab.inset(2.0).intersection(bb.inset(2.0)) else {
                    continue;
                };
                let smaller = ab.area().min(bb.area()).max(1.0);
                if overlap.width() < 4.0
                    || overlap.height() < 4.0
                    || overlap.area() / smaller < 0.16
                {
                    continue;
                }
                push_unique(
                    &mut diagnostics,
                    &mut seen,
                    VisualDiagnostic {
                        severity: VisualSeverity::Warning,
                        issue: VisualIssue::Overlap,
                        format: format.into(),
                        stage: stage.clone(),
                        at,
                        entity: a.id.clone(),
                        other: Some(b.id.clone()),
                        message: format!("overlaps `{}`", b.id),
                        suggestion:
                            "separate the entities, shorten/wrap the text, or reflow this format"
                                .into(),
                    },
                );
            }
        }
    }

    audit_camera_transitions(
        movie,
        format,
        &base,
        &timeline,
        authored_end,
        &mut diagnostics,
        &mut seen,
    );

    diagnostics.sort_by(|a, b| {
        a.format
            .cmp(&b.format)
            .then_with(|| a.at.total_cmp(&b.at))
            .then_with(|| a.severity.cmp(&b.severity))
            .then_with(|| a.entity.cmp(&b.entity))
    });
    diagnostics
}

fn push_unique(
    out: &mut Vec<VisualDiagnostic>,
    seen: &mut BTreeSet<(VisualIssue, String, Option<String>)>,
    diagnostic: VisualDiagnostic,
) {
    let (entity, other) = match diagnostic.other.as_ref() {
        Some(other) if other < &diagnostic.entity => {
            (other.clone(), Some(diagnostic.entity.clone()))
        }
        _ => (diagnostic.entity.clone(), diagnostic.other.clone()),
    };
    if seen.insert((diagnostic.issue, entity, other)) {
        out.push(diagnostic);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_audit_names_the_stage_and_core_problem_types() {
        let movie = crate::parse(
            "canvas(1080,1920);\n\
             creator(me, \"@a footer=none safe=reels\");\n\
             text(outside,(-30,300),\"OFF SCREEN\"); size(outside,30);\n\
             text(a,(540,700),\"first\"); size(a,28);\n\
             text(b,(540,700),\"second\"); size(b,28);\n\
             text(tiny,(540,900),\"too small\"); size(tiny,8);\n\
             text(unsafe,(540,35),\"unsafe title\"); size(unsafe,30);\n\
             step(\"result\") { wait(0.5); }",
        )
        .unwrap();
        let diagnostics = visual_diagnostics(&movie, "portrait");
        assert!(diagnostics.iter().all(|d| d.stage == "result"));
        for issue in [
            VisualIssue::OffCanvas,
            VisualIssue::SafeArea,
            VisualIssue::Overlap,
            VisualIssue::Readability,
        ] {
            assert!(
                diagnostics.iter().any(|d| d.issue == issue),
                "missing {issue:?}: {diagnostics:#?}"
            );
        }
    }

    #[test]
    fn shipped_reactive_multiformat_story_is_clean_in_all_formats() {
        let src = include_str!("../examples/reactive-multiformat.manic");
        for (name, w, h) in [
            ("portrait", 1080, 1920),
            ("feed", 1080, 1350),
            ("square", 1080, 1080),
            ("landscape", 1280, 720),
        ] {
            let movie = crate::parse_with_canvas(src, w, h).unwrap();
            let diagnostics = visual_diagnostics(&movie, name);
            assert!(
                diagnostics.is_empty(),
                "{name} should be visually clean: {diagnostics:#?}"
            );
        }
    }

    #[test]
    fn camera_audit_samples_transitions_not_only_settled_frames() {
        let movie = crate::parse(
            "canvas(1080,1920); camera3((0,0,0),(0,20,3),35); \
             cube3(box,(0,0,0),(4,4,4),\"blue\"); \
             step(\"shock\") { orbit3(180,20,0.5,0.08,linear); }",
        )
        .unwrap();
        let diagnostics = visual_diagnostics(&movie, "portrait");
        assert!(
            diagnostics.iter().any(|d| matches!(
                d.issue,
                VisualIssue::CameraMotion | VisualIssue::CameraPenetration
            )),
            "{diagnostics:#?}"
        );
    }
}
