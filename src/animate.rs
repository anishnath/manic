//! The animation DSL: verbs, `seq!`/`par!`, and beats.
//!
//! New verb = `Verb` variant + builder method + match arm in `build_clip`.

use macroquad::prelude::{Color, Vec2};

use crate::easing::Easing;
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TextEvent, TrackSpec, Value};

#[derive(Debug, Clone)]
enum Verb {
    MoveTo(String, Vec2),
    MoveBy(String, Vec2),
    FadeIn(String),
    FadeOut(String),
    ColorTo(String, Color),
    Highlight(String, Color),
    ScaleTo(String, f32),
    Pulse(String),
    Shake(String),
    GrowTo(String, Vec2),
    SetText(String, String),
    TraceTo(String, f32),
}

impl Verb {
    fn default_dur(&self) -> f32 {
        match self {
            Verb::FadeIn(_) | Verb::FadeOut(_) => 0.35,
            Verb::Highlight(..) => 1.0,
            Verb::Pulse(_) => 0.5,
            Verb::Shake(_) => 0.45,
            Verb::SetText(..) => 0.4,
            Verb::TraceTo(..) => 0.8,
            _ => 0.5,
        }
    }
}

/// Builder for one animation act. Create with [`act()`] (or
/// `Movie::act()`), pick a verb, then optionally `.dur()` / `.ease()`:
///
/// ```ignore
/// act().move_to("A", v(900., 400.)).dur(0.6).ease(OutBack)
/// ```
#[derive(Debug, Clone, Default)]
pub struct ActBuilder {
    verb: Option<Verb>,
    dur: Option<f32>,
    easing: Easing,
}

/// Start describing one animation act.
pub fn act() -> ActBuilder {
    ActBuilder::default()
}

/// A narration beat: an empty clip occupying `s` seconds. Usable inside
/// `seq!` to leave room for voiceover.
pub fn wait(s: f32) -> Clip {
    Clip::wait(s)
}

impl ActBuilder {
    fn verb(mut self, v: Verb) -> Self {
        self.verb = Some(v);
        self
    }

    // ---- verbs ---------------------------------------------------------

    /// Move an entity to an absolute position.
    pub fn move_to(self, id: &str, to: Vec2) -> Self {
        self.verb(Verb::MoveTo(id.into(), to))
    }

    /// Move an entity by a delta from wherever it is at that point.
    pub fn move_by(self, id: &str, by: Vec2) -> Self {
        self.verb(Verb::MoveBy(id.into(), by))
    }

    /// Animate opacity to 1.
    pub fn fade_in(self, id: &str) -> Self {
        self.verb(Verb::FadeIn(id.into()))
    }

    /// Animate opacity to 0.
    pub fn fade_out(self, id: &str) -> Self {
        self.verb(Verb::FadeOut(id.into()))
    }

    /// Permanently animate the fill color.
    pub fn color_to(self, id: &str, c: Color) -> Self {
        self.verb(Verb::ColorTo(id.into(), c))
    }

    /// Flash to `c`, hold, then restore the previous color automatically.
    /// The whole in-hold-out cycle spans the act's duration.
    pub fn highlight(self, id: &str, c: Color) -> Self {
        self.verb(Verb::Highlight(id.into(), c))
    }

    /// Animate uniform scale to an absolute factor.
    pub fn scale_to(self, id: &str, s: f32) -> Self {
        self.verb(Verb::ScaleTo(id.into(), s))
    }

    /// Quick grow-and-settle attention pulse (scale up ~18%, back).
    pub fn pulse(self, id: &str) -> Self {
        self.verb(Verb::Pulse(id.into()))
    }

    /// Horizontal shake — "no"/error gesture. Returns to origin.
    pub fn shake(self, id: &str) -> Self {
        self.verb(Verb::Shake(id.into()))
    }

    /// Animate the endpoint of a line/arrow to `to`. With the endpoint
    /// starting at the tail this *draws* the line; on an existing arrow it
    /// *retargets* it (e.g. Union-Find parent pointers).
    pub fn grow_to(self, id: &str, to: Vec2) -> Self {
        self.verb(Verb::GrowTo(id.into(), to))
    }

    /// Alias of [`grow_to`](Self::grow_to), reads better for pointer changes.
    pub fn retarget(self, id: &str, to: Vec2) -> Self {
        self.grow_to(id, to)
    }

    /// Crossfade a text entity to new content (fade out, swap, fade in).
    pub fn set_text(self, id: &str, text: &str) -> Self {
        self.verb(Verb::SetText(id.into(), text.into()))
    }

    /// Draw-on: trace a stroked shape's path/outline from its current
    /// progress to fully drawn. Declare the entity `.untraced()` first.
    pub fn trace_in(self, id: &str) -> Self {
        self.verb(Verb::TraceTo(id.into(), 1.0))
    }

    /// Reverse of [`trace_in`](Self::trace_in): erase back to nothing.
    pub fn trace_out(self, id: &str) -> Self {
        self.verb(Verb::TraceTo(id.into(), 0.0))
    }

    /// Typewriter: reveal a text entity character by character. Same track
    /// as `trace_in` but defaults to linear easing.
    pub fn type_in(mut self, id: &str) -> Self {
        self.easing = Easing::Linear;
        self.verb(Verb::TraceTo(id.into(), 1.0)).dur(1.5)
    }

    /// Pan the camera centre to `pos` (the camera is entity `"__cam"`).
    pub fn cam_to(self, pos: Vec2) -> Self {
        self.verb(Verb::MoveTo(crate::movie::CAMERA_ID.into(), pos))
    }

    /// Zoom the camera to factor `z` (1.0 = whole canvas).
    pub fn cam_zoom(self, z: f32) -> Self {
        self.verb(Verb::ScaleTo(crate::movie::CAMERA_ID.into(), z))
    }

    // ---- tuning --------------------------------------------------------

    /// Total duration of this act in seconds.
    pub fn dur(mut self, s: f32) -> Self {
        self.dur = Some(s);
        self
    }

    /// Easing curve (default: `InOutCubic`).
    pub fn ease(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }
}

fn track(
    id: &str,
    prop: Prop,
    target: TargetValue,
    start: f32,
    dur: f32,
    easing: Easing,
) -> TrackSpec {
    TrackSpec {
        id: id.into(),
        prop,
        target,
        start,
        dur,
        easing,
    }
}

fn build_clip(b: ActBuilder) -> Clip {
    let verb = b
        .verb
        .expect("ActBuilder used without a verb (e.g. .move_to(..))");
    let d = b.dur.unwrap_or_else(|| verb.default_dur());
    let e = b.easing;
    let mut clip = Clip {
        dur: d,
        ..Default::default()
    };

    match verb {
        Verb::MoveTo(id, to) => {
            clip.tracks.push(track(
                &id,
                Prop::Pos,
                TargetValue::Abs(Value::V(to)),
                0.0,
                d,
                e,
            ));
        }
        Verb::MoveBy(id, by) => {
            clip.tracks.push(track(
                &id,
                Prop::Pos,
                TargetValue::Rel(Value::V(by)),
                0.0,
                d,
                e,
            ));
        }
        Verb::FadeIn(id) => {
            clip.tracks.push(track(
                &id,
                Prop::Opacity,
                TargetValue::Abs(Value::F(1.0)),
                0.0,
                d,
                e,
            ));
        }
        Verb::FadeOut(id) => {
            clip.tracks.push(track(
                &id,
                Prop::Opacity,
                TargetValue::Abs(Value::F(0.0)),
                0.0,
                d,
                e,
            ));
        }
        Verb::ColorTo(id, c) => {
            clip.tracks.push(track(
                &id,
                Prop::Color,
                TargetValue::Abs(Value::C(c)),
                0.0,
                d,
                e,
            ));
        }
        Verb::Highlight(id, c) => {
            // in 25% — hold 50% — restore 25%
            clip.tracks.push(track(
                &id,
                Prop::Color,
                TargetValue::Abs(Value::C(c)),
                0.0,
                d * 0.25,
                Easing::OutQuad,
            ));
            clip.tracks.push(track(
                &id,
                Prop::Color,
                TargetValue::Revert,
                d * 0.75,
                d * 0.25,
                Easing::InQuad,
            ));
        }
        Verb::ScaleTo(id, s) => {
            clip.tracks.push(track(
                &id,
                Prop::Scale,
                TargetValue::Abs(Value::F(s)),
                0.0,
                d,
                e,
            ));
        }
        Verb::Pulse(id) => {
            clip.tracks.push(track(
                &id,
                Prop::Scale,
                TargetValue::Rel(Value::F(0.18)),
                0.0,
                d * 0.4,
                Easing::OutQuad,
            ));
            clip.tracks.push(track(
                &id,
                Prop::Scale,
                TargetValue::Revert,
                d * 0.4,
                d * 0.6,
                Easing::OutBack,
            ));
        }
        Verb::Shake(id) => {
            // offsets sum to zero so the entity lands back home
            let offs = [10.0, -16.0, 12.0, -8.0, 5.0, -3.0f32];
            let seg = d / offs.len() as f32;
            for (i, dx) in offs.iter().enumerate() {
                clip.tracks.push(track(
                    &id,
                    Prop::Pos,
                    TargetValue::Rel(Value::V(Vec2::new(*dx, 0.0))),
                    seg * i as f32,
                    seg,
                    Easing::InOutQuad,
                ));
            }
        }
        Verb::GrowTo(id, to) => {
            clip.tracks.push(track(
                &id,
                Prop::To,
                TargetValue::Abs(Value::V(to)),
                0.0,
                d,
                e,
            ));
        }
        Verb::TraceTo(id, f) => {
            clip.tracks.push(track(
                &id,
                Prop::Trace,
                TargetValue::Abs(Value::F(f)),
                0.0,
                d,
                e,
            ));
        }
        Verb::SetText(id, text) => {
            clip.tracks.push(track(
                &id,
                Prop::Opacity,
                TargetValue::Abs(Value::F(0.0)),
                0.0,
                d * 0.4,
                Easing::InQuad,
            ));
            clip.events.push(TextEvent {
                id: id.clone(),
                content: text,
                at: d * 0.5,
            });
            clip.tracks.push(track(
                &id,
                Prop::Opacity,
                TargetValue::Abs(Value::F(1.0)),
                d * 0.6,
                d * 0.4,
                Easing::OutQuad,
            ));
        }
    }
    clip
}

impl From<ActBuilder> for Clip {
    fn from(b: ActBuilder) -> Clip {
        build_clip(b)
    }
}

/// Convenience: a highlight in the house spot color (magenta).
pub fn flash(id: &str) -> ActBuilder {
    act().highlight(id, style::MAGENTA)
}

/// One clip per id from the same recipe, all in parallel. Pairs with
/// [`crate::movie::Movie::tagged`]:
///
/// ```ignore
/// m.play(all(&m.tagged("bits"), |id| act().fade_out(id).dur(0.4)));
/// ```
pub fn all<F>(ids: &[String], f: F) -> Clip
where
    F: Fn(&str) -> ActBuilder,
{
    Clip::par(ids.iter().map(|id| f(id).into()).collect())
}

/// Run clips in parallel, each starting `delay` seconds after the previous —
/// the cascade effect. Also available as `stagger![delay; a, b, c]`.
pub fn stagger(delay: f32, clips: Vec<Clip>) -> Clip {
    Clip::par(
        clips
            .into_iter()
            .enumerate()
            .map(|(i, c)| c.shift(delay * i as f32))
            .collect(),
    )
}

/// Run clips **one after another**. Accepts anything `Into<Clip>`
/// (acts, `wait(s)`, nested `seq!`/`par!`).
#[macro_export]
macro_rules! seq {
    ($($c:expr),* $(,)?) => {
        $crate::timeline::Clip::seq(vec![$(::std::convert::Into::<$crate::timeline::Clip>::into($c)),*])
    };
}

/// Run clips **at the same time** (duration = longest member).
#[macro_export]
macro_rules! par {
    ($($c:expr),* $(,)?) => {
        $crate::timeline::Clip::par(vec![$(::std::convert::Into::<$crate::timeline::Clip>::into($c)),*])
    };
}

/// Cascade: each clip starts `delay` after the previous.
/// `stagger![0.05; a, b, c]`.
#[macro_export]
macro_rules! stagger {
    ($d:expr; $($c:expr),* $(,)?) => {
        $crate::animate::stagger($d, vec![$(::std::convert::Into::<$crate::timeline::Clip>::into($c)),*])
    };
}
