//! Creator-facing visual correctness checks.
//!
//! V1 deliberately audits settled semantic checkpoints rather than every
//! transition frame. That makes off-canvas, safe-area, overlap, and readability
//! diagnostics useful without flagging intentional entrances or rewrite
//! crossfades. The timeline is stateless, so every checkpoint is reproducible.

use std::collections::BTreeSet;

use macroquad::prelude::Vec2;

use crate::movie::Movie;
use crate::primitives::{Align, Entity, Shape, TextRun};
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
    let (l, r, t, b) = match safe {
        CreatorSafe::Shorts => (0.060, 0.090, 0.055, 0.110),
        CreatorSafe::Reels => (0.065, 0.105, 0.075, 0.135),
        CreatorSafe::Tiktok => (0.065, 0.145, 0.075, 0.155),
        CreatorSafe::Clean => (0.045, 0.045, 0.045, 0.045),
    };
    Bounds {
        lo: Vec2::new(canvas.x * l, canvas.y * t),
        hi: Vec2::new(canvas.x * (1.0 - r), canvas.y * (1.0 - b)),
    }
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
        let scene = timeline.apply(&base, at);
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
}
