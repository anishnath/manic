//! The [`Movie`]: top-level container tying a scene to a timeline, with a
//! cursor-based sequencing model.
//!
//! `play(clip)` appends at the current cursor and advances it; `at(t, clip)`
//! places a clip at an absolute time without moving the cursor (for lining
//! up with narration beats); `wait(s)` leaves silence.

use macroquad::prelude::Vec2;

use crate::animate::{act, ActBuilder};
use crate::primitives::{Entity, Shape};
use crate::scene::{Scene, SceneBuilder};
use crate::style;
use crate::timeline::{Clip, TextEvent, Timeline, TrackSpec};

/// Reserved id of the animatable camera entity every movie carries.
/// Animate it with `act().cam_to(pos)` / `act().cam_zoom(z)`.
pub const CAMERA_ID: &str = "__cam";
/// Reserved id of the optional animatable 3D orbit camera.
pub const CAMERA3_ID: &str = "__cam3";

/// If `s` (trimmed) is exactly one `$…$` span with no interior `$`, return the
/// inner LaTeX. Phase 2a handles WHOLE-label math only (a caption that *is* a
/// formula); mixed runs (`"KE = $\frac12 mv^2$"`) come in 2b. Returns `None` for
/// plain text — so it's never touched.
pub(crate) fn whole_math_span(s: &str) -> Option<&str> {
    let inner = s.trim().strip_prefix('$')?.strip_suffix('$')?;
    (!inner.is_empty() && !inner.contains('$')).then_some(inner)
}

/// A build-time run before math is rendered: plain text or a LaTeX span.
enum RawRun {
    Text(String),
    Math(String),
}

/// Split a string into alternating text / `$…$` math runs. `\$` is a literal
/// dollar; every other backslash is kept (for LaTeX). An unmatched `$` and its
/// tail fall back to literal text (never drops characters).
fn split_math_runs(s: &str) -> Vec<RawRun> {
    let mut runs = Vec::new();
    let mut buf = String::new();
    let mut in_math = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'$') => {
                buf.push('$'); // escaped literal dollar
                chars.next();
            }
            '\\' => buf.push('\\'), // keep the backslash for LaTeX
            '$' => {
                if in_math {
                    runs.push(RawRun::Math(std::mem::take(&mut buf)));
                } else if !buf.is_empty() {
                    runs.push(RawRun::Text(std::mem::take(&mut buf)));
                }
                in_math = !in_math;
            }
            _ => buf.push(c),
        }
    }
    if in_math {
        runs.push(RawRun::Text(format!("${buf}"))); // unmatched → literal
    } else if !buf.is_empty() {
        runs.push(RawRun::Text(buf));
    }
    runs
}

/// A complete animation: base scene + placed clips + metadata.
pub struct Movie {
    /// Shown in the header of every frame and as the window title.
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub(crate) scene: Scene,
    placed: Vec<(f32, Clip)>,
    cursor: f32,
    /// (time, name) markers — the player jumps to these with keys 1–9.
    pub sections: Vec<(f32, String)>,
    /// Named beat markers from [`Movie::mark`], exported to `markers.json`
    /// alongside recordings for narration alignment.
    pub marks: Vec<(f32, String)>,
    /// The visual template (look/chrome). Defaults to `mono`.
    pub template: style::Template,
    section_n: usize,
}

impl Movie {
    /// New movie with a canvas size. 1280×720 keeps live preview snappy;
    /// recordings supersample to 1080p regardless.
    pub fn new(title: &str, width: u32, height: u32) -> Movie {
        let mut scene = Scene::new();
        scene.set_canvas_size(width as f32, height as f32);
        let mut cam = Entity::new(
            CAMERA_ID,
            Shape::Circle { r: 0.0 },
            Vec2::new(width as f32 / 2.0, height as f32 / 2.0),
            style::VOID,
        );
        cam.opacity = 0.0;
        scene.add(cam);
        Movie {
            title: title.into(),
            width,
            height,
            scene,
            placed: Vec::new(),
            cursor: 0.0,
            sections: Vec::new(),
            marks: Vec::new(),
            template: style::Template::default(),
            section_n: 0,
        }
    }

    /// Declare entities (the world at t = 0). Call as many times as you like.
    pub fn scene(&mut self) -> SceneBuilder<'_> {
        SceneBuilder::new(&mut self.scene)
    }

    /// Read-only access to the base scene (used by kits / the lowering pass
    /// to look up existing entities, e.g. resolving an id to a position).
    pub fn base(&self) -> &Scene {
        &self.scene
    }

    /// Build post-pass — **inline LaTeX in ANY text**. Every base-scene
    /// `Shape::Text` whose content is a single `$…$` math span (a label/caption
    /// that *is* a formula) is typeset via RaTeX and becomes a colour-tinted
    /// equation image. This is what makes LaTeX holistic: `text`/`caption`/`say`
    /// and every kit label (geo points, quiz options, …) get it for free, with
    /// **zero per-kit code**. Plain text (no `$`) is byte-identically untouched;
    /// bad LaTeX is left as literal text (never panics or aborts the build).
    pub(crate) fn typeset_inline_math(&mut self) {
        use crate::primitives::TextRun;
        // Lay out one `$…$` span now (LOGICAL px) and queue it for the player to
        // rasterise at render scale. Returns None on bad LaTeX (left as text).
        let mut pending: Vec<(String, String, f32)> = Vec::new();
        let mut layout = |latex: &str, size: f32| -> Option<(String, f32, f32, f32)> {
            let (w, h, base) = crate::latex::layout_dims(latex, size).ok()?;
            let path = crate::latex::eq_path(latex, size);
            pending.push((path.clone(), latex.to_string(), size));
            Some((path, w, h, base))
        };
        for e in self.scene.entities.iter_mut() {
            let (content, size) = match &e.shape {
                Shape::Text { content, size } => (content.clone(), *size),
                _ => continue,
            };
            // 2a — a WHOLE-`$…$` label becomes an equation image. It keeps the
            // entity's alignment (the Image draw honours Left vs Center), so a
            // left-aligned option label stays right of its badge.
            if let Some(inner) = whole_math_span(&content) {
                if let Some((path, w, h, _)) = layout(inner, size) {
                    e.shape = Shape::Image { path, w, h, tint: true };
                }
                continue;
            }
            // 2b — MIXED text + `$…$` on one line becomes RichText (inline runs).
            let raw = split_math_runs(&content);
            if !raw.iter().any(|r| matches!(r, RawRun::Math(_))) {
                continue; // no math → plain text, untouched
            }
            let mut runs = Vec::with_capacity(raw.len());
            let mut ok = true;
            for r in raw {
                match r {
                    RawRun::Text(t) => runs.push(TextRun::Text(t)),
                    RawRun::Math(m) => match layout(&m, size) {
                        Some((path, w, h, baseline)) => runs.push(TextRun::Math { path, w, h, baseline }),
                        None => {
                            ok = false; // bad LaTeX in a run → leave the whole line as text
                            break;
                        }
                    },
                }
            }
            if ok {
                e.shape = Shape::RichText { runs, size };
            }
        }
        self.scene.pending_eqs.extend(pending);
    }

    /// Start describing an animation act (same as the free [`act()`]).
    pub fn act(&self) -> ActBuilder {
        act()
    }

    /// Append a clip at the cursor; the cursor advances past it.
    pub fn play(&mut self, clip: impl Into<Clip>) {
        let clip = clip.into();
        self.cursor = self.cursor.max(0.0);
        let end = self.cursor + clip.dur;
        self.placed.push((self.cursor, clip));
        self.cursor = end;
    }

    /// Place a clip at an absolute time. Does not move the cursor.
    pub fn at(&mut self, t: f32, clip: impl Into<Clip>) {
        self.placed.push((t, clip.into()));
    }

    /// Advance the cursor by `s` seconds of nothing — a narration beat.
    pub fn wait(&mut self, s: f32) {
        self.cursor += s;
    }

    /// Current cursor time (useful for noting narration timestamps).
    pub fn now(&self) -> f32 {
        self.cursor
    }

    /// Drop a named beat marker at the cursor. Markers (plus sections) are
    /// written to `markers.json` next to recorded frames.
    pub fn mark(&mut self, name: &str) {
        self.marks.push((self.cursor, name.to_string()));
    }

    /// Ids of all entities carrying `tag`. Pair with [`crate::animate::all`]:
    /// `m.play(all(&m.tagged("bits"), |id| act().fade_out(id)))`.
    pub fn tagged(&self, tag: &str) -> Vec<String> {
        let mut ids: Vec<String> = self
            .scene
            .entities
            .iter()
            .filter(|e| e.tags.iter().any(|t| t == tag))
            .map(|e| e.id.clone())
            .collect();
        ids.extend(
            self.scene
                .entities_3d
                .iter()
                .filter(|e| e.tags.iter().any(|t| t == tag))
                .map(|e| e.id.clone()),
        );
        ids
    }

    /// Section break: fades in a display headline with a neon rule (terminal
    /// banner), holds, fades out. Also records a marker the player can jump to
    /// with number keys.
    pub fn section(&mut self, title: &str) {
        self.section_n += 1;
        let n = self.section_n;
        let cx = self.width as f32 / 2.0;
        let cy = self.height as f32 / 2.0;

        let head_id = format!("__section{n}");
        let rule_id = format!("__section{n}.rule");
        let kicker_id = format!("__section{n}.kicker");
        let bg_id = format!("__section{n}.bg");
        {
            let mut s = self.scene();
            // backdrop keeps the card legible over a busy stage
            s.rect(&bg_id, Vec2::new(cx, cy - 10.0), 820.0, 240.0)
                .color(style::PANEL)
                .outline_color(style::CYAN)
                .stroke(1.5)
                .z(88)
                .hidden();
            s.text(&head_id, Vec2::new(cx, cy - 10.0), title)
                .display()
                .size(58.0)
                .color(style::CYAN)
                .z(90)
                .hidden();
            s.line(
                &rule_id,
                Vec2::new(cx - 140.0, cy + 36.0),
                Vec2::new(cx + 140.0, cy + 36.0),
            )
            .color(style::MAGENTA)
            .stroke(3.0)
            .z(90)
            .hidden();
            s.text(
                &kicker_id,
                Vec2::new(cx, cy - 64.0),
                &format!("» SECTION {n:02}"),
            )
            .mono_bold()
            .size(20.0)
            .color(style::LIME)
            .z(90)
            .hidden();
        }

        self.sections.push((self.cursor, title.to_string()));
        let clip = crate::seq![
            crate::par![
                act().fade_in(&bg_id).dur(0.4),
                act().fade_in(&head_id).dur(0.4),
                act().fade_in(&rule_id).dur(0.4),
                act().fade_in(&kicker_id).dur(0.4),
            ],
            crate::timeline::Clip::wait(1.4),
            crate::par![
                act().fade_out(&bg_id).dur(0.4),
                act().fade_out(&head_id).dur(0.4),
                act().fade_out(&rule_id).dur(0.4),
                act().fade_out(&kicker_id).dur(0.4),
            ],
        ];
        self.play(clip);
    }

    /// Fade out every entity currently declared (a "clear the stage" scene
    /// change). Entities declared *after* this call are unaffected.
    pub fn clear_all(&mut self, dur: f32) {
        let ids: Vec<String> = self
            .scene
            .entities
            .iter()
            .filter(|e| !e.id.starts_with("__"))
            .map(|e| e.id.clone())
            .chain(
                self.scene
                    .entities_3d
                    .iter()
                    .filter(|e| !e.id.starts_with("__"))
                    .map(|e| e.id.clone()),
            )
            .collect();
        let clips: Vec<Clip> = ids
            .iter()
            .map(|id| act().fade_out(id).dur(dur).into())
            .collect();
        self.play(Clip::par(clips));
    }

    /// Flatten placed clips into absolute-time specs and resolve keyframes.
    /// Called by the player; you rarely need it directly.
    pub fn finalize(&self) -> (Scene, Timeline) {
        let mut specs: Vec<TrackSpec> = Vec::new();
        let mut events: Vec<TextEvent> = Vec::new();
        let mut end = self.cursor;
        for (start, clip) in &self.placed {
            end = end.max(start + clip.dur);
            for t in &clip.tracks {
                let mut t = t.clone();
                t.start += start;
                specs.push(t);
            }
            for e in &clip.events {
                let mut e = e.clone();
                e.shift(*start);
                events.push(e);
            }
        }
        let tl = Timeline::resolve(&self.scene, specs, events, end + 1.0);
        (self.scene.clone(), tl)
    }

    /// Whole-file sanity check: verify every animation (track) and text event
    /// references an entity that actually exists in the scene — the same
    /// condition [`Timeline::resolve`] asserts, surfaced as a recoverable error
    /// so `manic check` catches it *before* render instead of panicking at
    /// finalize. Returns the sorted, de-duplicated list of offending ids.
    ///
    /// No false positives: tag-broadcast has already expanded groups to concrete
    /// ids by this point, so any track/event id absent from the scene is a real
    /// mistake (a typo, or animating a builtin's bare id when only sub-ids exist).
    pub fn validate(&self) -> Result<(), String> {
        use std::collections::BTreeSet;
        let mut unknown: BTreeSet<String> = BTreeSet::new();
        for (_, clip) in &self.placed {
            for t in &clip.tracks {
                if !self.scene.contains(&t.id) {
                    unknown.insert(t.id.clone());
                }
            }
            for e in &clip.events {
                if !self.scene.contains(e.id()) {
                    unknown.insert(e.id().to_string());
                }
            }
        }
        if unknown.is_empty() {
            return Ok(());
        }
        // Every entity id and tag an author could legitimately have meant.
        let candidates = crate::namehint::candidate_names(&self.scene);
        let mut lines = vec![format!(
            "{} unknown entity id(s) in animations — nothing is created with {}:",
            unknown.len(),
            if unknown.len() == 1 { "this name" } else { "these names" }
        )];
        for id in &unknown {
            match crate::namehint::nearest_name(id, &candidates) {
                Some(sugg) => lines.push(format!("  • `{id}` — did you mean `{sugg}`?")),
                None => lines.push(format!(
                    "  • `{id}` — create it before animating it (a shape/text/… with this id or tag)"
                )),
            }
        }
        Err(lines.join("\n"))
    }
}

#[cfg(test)]
mod validate_tests {
    /// Unknown ids are now caught at PARSE time (in lowering) with a spanned
    /// diagnostic — before finalize — so every command (run/record/check) reports
    /// the exact line + a "did you mean".
    #[test]
    fn parse_flags_animation_on_unknown_id() {
        let src = "dot(a, (100,100), 5);\nshow(b, 0.5);";
        let err = crate::parse(src).err().expect("expected a parse error");
        let msg = crate::lang::diag::render(src, &err);
        assert!(msg.contains('b'), "error should name the unknown id:\n{msg}");
        assert!(msg.contains("not created"), "should explain the problem:\n{msg}");
    }

    #[test]
    fn parse_passes_when_all_ids_exist() {
        let m = crate::parse("dot(a, (100,100), 5);\nshow(a, 0.5);").unwrap();
        assert!(m.validate().is_ok(), "all ids exist → should validate");
    }

    #[test]
    fn dsl_defaults_to_mono_but_respects_an_explicit_template() {
        let default_movie = crate::parse("dot(a, (100,100), 5);").unwrap();
        assert_eq!(default_movie.template.name, "mono");

        let neon_movie = crate::parse("template(\"plain\"); dot(a, (100,100), 5);").unwrap();
        assert_eq!(neon_movie.template.name, "plain");
    }

    /// `Movie::validate` (the `manic check` net) directly flags a bad track even
    /// one hand-built past the parse-time check (e.g. a mut-verb path).
    #[test]
    fn validate_catches_hand_built_bad_track() {
        use crate::easing::Easing;
        use crate::timeline::{Clip, Prop, TargetValue, TrackSpec};
        let mut m = crate::parse("dot(a, (0,0), 5);").unwrap();
        m.play(Clip {
            tracks: vec![TrackSpec {
                id: "nope".into(),
                prop: Prop::Opacity,
                target: TargetValue::Revert,
                start: 0.0,
                dur: 0.5,
                easing: Easing::default(),
            }],
            events: vec![],
            dur: 0.5,
        });
        let err = m.validate().unwrap_err();
        assert!(err.contains("nope"), "validate should flag it: {err}");
    }
}
