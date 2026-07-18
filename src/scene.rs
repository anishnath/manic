//! The [`Scene`]: an id-addressed store of entities, plus the chainable
//! [`SceneBuilder`] used to declare the time-zero state of a movie.

use std::collections::HashMap;

use macroquad::prelude::{Color, Vec2, Vec3};

use crate::primitives::{Align, Entity, FontKind, Shape, StrokeStyle};
use crate::primitives3d::Entity3D;
use crate::style;
use crate::timeline::Prop;

/// One pre-simulated playback track: an entity sub-id, the property to drive
/// (`Pos`/`To`), and its per-frame screen positions. Physics ctors build these
/// and store them in [`Scene::sims`]; the playback verb (`swing`) replays each as
/// a keyframed track chain.
#[derive(Debug, Clone)]
pub struct PlaybackTrack {
    pub id: String,
    pub prop: Prop,
    pub points: Vec<Vec2>,
}

/// Everything a pre-simulated sim exposes — the reusable **baseline** for every
/// physics sim. A sim's ctor fills this once; the sim-view drawables replay via
/// `playback`, and the OPTIONAL view builtins (`phase`/`well`/…) read the raw
/// data series to render extra panels. Generic over the `Sim` trait, so any sim
/// (pendulum, spring, double-pendulum, …) gets the same views for free — a sim
/// that leaves a field empty simply doesn't support that view.
#[derive(Debug, Clone, Default)]
pub struct SimData {
    /// Screen-space keyframes replayed by `swing` (sim-view parts + any markers
    /// the view builtins append).
    pub playback: Vec<PlaybackTrack>,
    /// Raw state vector per frame (e.g. `[θ, ω, t]`).
    pub states: Vec<Vec<f32>>,
    /// `(kinetic, potential)` energy per frame.
    pub energy: Vec<(f32, f32)>,
    /// Simulated seconds per frame (the sample interval) — the `time`-view x axis.
    pub dt: f32,
    /// State-variable labels (`θ`, `ω`, …) for axis/legend text.
    pub labels: Vec<String>,
    /// State indices `(x, y)` to plot against each other in the phase portrait.
    pub phase_xy: Option<(usize, usize)>,
    /// The state index that is "position" (the well-view x axis).
    pub pos_var: Option<usize>,
    /// Sampled `(position, potential-energy)` for the potential-well curve.
    pub well: Vec<(f32, f32)>,
}

/// A 2D label glued to a 3D position (`pin3`). Reprojected every frame at
/// render time, so the label tracks the point as the camera orbits.
#[derive(Debug, Clone)]
pub struct Pin3 {
    /// id of the 2D entity (a `text`/`label`) to reposition.
    pub label: String,
    pub target: Pin3Target,
    /// Screen-space nudge (pixels, y-down) applied after projection, so a label
    /// can sit *beside* its anchor instead of on top of it. `axes3` uses it to
    /// fan each axis's tick numbers off the axis line in a distinct direction.
    pub offset: Vec2,
    /// If set, this label is hidden for any frame where it would collide with
    /// an already-placed decluttering label. `axes3` tick numbers use it so a
    /// foreshortened axis (pointing at the camera) doesn't stack its numbers;
    /// they reappear as the orbit spreads that axis out. User `pin3`s never
    /// declutter — they're always drawn where asked.
    pub declutter: bool,
}

#[derive(Debug, Clone)]
pub enum Pin3Target {
    /// A fixed world point.
    Point(Vec3),
    /// Track a 3D entity's current position.
    Entity(String),
}

#[derive(Debug, Clone, Copy)]
enum EntitySlot {
    D2(usize),
    D3(usize),
}

/// An id-addressed collection of entities. This is the *base* state of the
/// world at t = 0; the timeline produces per-frame copies of it.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    /// Logical canvas size. Kits use this for responsive layout; render scale is
    /// deliberately separate, so a 1080x1920 Short and its supersampled export
    /// share identical layout coordinates.
    pub canvas_size: macroquad::prelude::Vec2,
    pub entities: Vec<Entity>,
    pub entities_3d: Vec<Entity3D>,
    index: HashMap<String, EntitySlot>,
    /// Build-time slot occupancy for stateful structures (e.g. `array`): maps a
    /// structure id to the entity id sitting in each slot. Seeded by the
    /// constructor and updated by mutating verbs like `swap`, so a chain of
    /// swaps knows the *current* occupant of each slot. Build-time only — the
    /// renderer never reads it.
    pub occ: HashMap<String, Vec<String>>,
    /// 2D labels bound to 3D positions (`pin3`), applied per-frame by the player.
    pub pins: Vec<Pin3>,
    /// Pre-simulated playback for physics sims (`physics` kit): maps a sim id to
    /// its list of [`PlaybackTrack`]s (bob position, rod endpoint, velocity arrow,
    /// energy bars, …). The ctor (`pendulum`) pre-integrates and fills this; the
    /// playback verb (`swing`) reads it to emit the keyframed replay. Build-time only.
    pub sims: HashMap<String, SimData>,
    /// Creator social profiles (`creator` builtin): id → handle + platforms +
    /// accent. `socials` reads it to draw the footer. Build-time only.
    pub creators: HashMap<String, CreatorProfile>,
    /// Quiz-Short state (`quiz`/`option`): id → question/options/countdown.
    /// `run(id)` reads it to emit the ask → countdown → reveal beat. Build-time.
    pub quizzes: HashMap<String, QuizData>,
    /// Generic named-phase timing controllers. Unlike `QuizTiming`, these are
    /// format-neutral: `timed(id) { during("phase") { ... } }` can coordinate
    /// any ordinary scene while the same native timer widget runs alongside it.
    pub timings: HashMap<String, TimingData>,
    /// LaTeX equations to rasterise at the RENDER scale (so they're pixel-sharp,
    /// not up/down-scaled). Layout/size are fixed at build; the player renders the
    /// PNG at `dpr = render scale` before the frame loop. `(cache path, latex, em size)`.
    pub pending_eqs: Vec<(String, String, f32)>,
}

/// How a creator identity is presented at the bottom of a format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreatorFooter {
    /// Platform icons + handle: the v1 treatment and backwards-compatible default.
    #[default]
    Social,
    /// Small logo/name/handle lockup with no platform-icon row.
    Compact,
    /// Larger logo, display name and tagline lockup.
    Signature,
    /// Suppress the footer entirely.
    None,
}

/// Platform-safe content inset profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreatorSafe {
    #[default]
    Shorts,
    Reels,
    Tiktok,
    Clean,
}

/// A creator's reusable brand profile. Existing handle/platform/accent fields
/// remain; v2 adds identity and presentation fields for responsive footers and
/// end cards.
#[derive(Debug, Clone, Default)]
pub struct CreatorProfile {
    pub handle: String,
    pub display_name: String,
    pub tagline: String,
    pub logo: String,
    pub website: String,
    pub cta: String,
    pub platforms: Vec<(String, String)>,
    pub accent: Option<macroquad::prelude::Color>,
    pub secondary: Option<macroquad::prelude::Color>,
    pub footer: CreatorFooter,
    pub safe: CreatorSafe,
}

/// A centre/size rectangle used by the responsive creator layout.
#[derive(Debug, Clone, Copy, Default)]
pub struct CreatorRect {
    pub center: macroquad::prelude::Vec2,
    pub size: macroquad::prelude::Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuizLayout {
    #[default]
    Auto,
    Stack,
    Grid,
    MediaFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuizDensity {
    Compact,
    #[default]
    Comfortable,
    Spacious,
}

/// Visible option-index treatment. Letters remain the compatibility/default
/// choice; numbers and no labels are useful for polls and statement lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuizLabels {
    #[default]
    Letters,
    Numbers,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuizTimer {
    #[default]
    Ring,
    Bar,
    Number,
    Segments,
    Ticks,
    Pulse,
    None,
}

/// Named timing rhythms for a creator quiz. A preset is a convenient starting
/// point; `timing(...)` may override any individual phase afterward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreatorPace {
    Quick,
    #[default]
    Balanced,
    Calm,
    Dramatic,
}

/// Absolute phase durations (seconds) for the ask → options → think → reveal
/// → hold beat. `custom` records whether the author supplied numeric phases;
/// explicit phases intentionally reject a second total duration in `run`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuizTiming {
    pub pace: CreatorPace,
    pub ask: f32,
    pub options: f32,
    pub think: f32,
    pub reveal: f32,
    pub hold: f32,
    pub stagger: f32,
    pub custom: bool,
}

impl QuizTiming {
    pub fn preset(pace: CreatorPace) -> Self {
        let (ask, options, think, reveal, hold, stagger) = match pace {
            CreatorPace::Quick => (0.70, 0.70, 3.0, 0.50, 2.10, 0.045),
            CreatorPace::Balanced => (1.40, 1.20, 5.0, 0.80, 3.60, 0.065),
            CreatorPace::Calm => (1.80, 1.60, 7.0, 1.00, 3.60, 0.090),
            CreatorPace::Dramatic => (1.10, 1.40, 5.0, 0.90, 3.60, 0.075),
        };
        Self {
            pace,
            ask,
            options,
            think,
            reveal,
            hold,
            stagger,
            custom: false,
        }
    }

    pub fn total(self) -> f32 {
        self.ask + self.options + self.think + self.reveal + self.hold
    }
}

impl Default for QuizTiming {
    fn default() -> Self {
        Self::preset(CreatorPace::Balanced)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimerPosition {
    #[default]
    Auto,
    Header,
    Media,
    Below,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimerNumber {
    #[default]
    Inside,
    Outside,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimerDirection {
    #[default]
    Drain,
    Fill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimerFinish {
    #[default]
    Fade,
    Hold,
    Flash,
    Pulse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimerFont {
    #[default]
    Mono,
    Display,
}

/// Visual tokens for quiz and standalone countdown widgets. Colours are
/// optional semantic overrides; the quiz accent and DIM track remain defaults.
#[derive(Debug, Clone)]
pub struct CreatorTimerSpec {
    pub look: QuizTimer,
    pub position: TimerPosition,
    pub number: TimerNumber,
    pub direction: TimerDirection,
    pub finish: TimerFinish,
    pub font: TimerFont,
    pub size: f32,
    pub thickness: f32,
    pub color: Option<macroquad::prelude::Color>,
    pub track: Option<macroquad::prelude::Color>,
    pub label: String,
}

impl Default for CreatorTimerSpec {
    fn default() -> Self {
        Self {
            look: QuizTimer::Ring,
            position: TimerPosition::Auto,
            number: TimerNumber::Inside,
            direction: TimerDirection::Drain,
            finish: TimerFinish::Fade,
            font: TimerFont::Mono,
            size: 1.0,
            thickness: 1.0,
            color: None,
            track: None,
            label: String::new(),
        }
    }
}

/// One named, absolute-duration phase in a generic timing controller.
#[derive(Debug, Clone, PartialEq)]
pub struct TimingPhase {
    pub name: String,
    pub duration: f32,
}

/// Format-neutral Timing v2 data. `timing(id, ...)` creates this, `timerstyle`
/// controls its optional native clock, and the lowerer schedules `during`
/// blocks at the exact offsets derived from `phases`.
#[derive(Debug, Clone)]
pub struct TimingData {
    pub phases: Vec<TimingPhase>,
    pub timer_style: CreatorTimerSpec,
    pub timer_rect: CreatorRect,
    pub ui_scale: f32,
}

impl TimingData {
    pub fn total(&self) -> f32 {
        self.phases.iter().map(|phase| phase.duration).sum()
    }

    pub fn phase(&self, name: &str) -> Option<(f32, f32)> {
        let mut offset = 0.0;
        for phase in &self.phases {
            if phase.name == name {
                return Some((offset, phase.duration));
            }
            offset += phase.duration;
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreatorMotion {
    Calm,
    #[default]
    Studio,
    Punch,
    Cut,
}

/// A quiz-Short's state (`quiz`/`option` builtins): the question + its option
/// cards + the correct-answer highlight + countdown widget ids. `run(id)` reads
/// this to emit the whole ask → countdown → reveal beat. Build-time only.
#[derive(Debug, Clone, Default)]
pub struct QuizData {
    pub options: Vec<QuizOpt>,
    /// id of the lime highlight rect over the correct card (empty until set).
    pub highlight: String,
    /// how the question text reveals in (typewriter by default).
    pub reveal: QuizReveal,
    /// The card/question design skin (Studio by default in v2).
    pub skin: QuizSkin,
    pub layout: QuizLayout,
    pub density: QuizDensity,
    pub labels: QuizLabels,
    pub timer_style: CreatorTimerSpec,
    pub timing: QuizTiming,
    pub motion: CreatorMotion,
    pub safe: CreatorSafe,
    pub accent: Option<macroquad::prelude::Color>,
    /// Responsive layout snapshot computed when `quiz` is constructed.
    pub header: CreatorRect,
    pub media: CreatorRect,
    pub choices: CreatorRect,
    pub timer: CreatorRect,
    pub footer: CreatorRect,
    pub card_size: macroquad::prelude::Vec2,
    pub question_pos: macroquad::prelude::Vec2,
    pub ui_scale: f32,
    /// Optional author-supplied explanation/source entity ids.
    pub explanation: String,
    pub source: String,
}

/// The visual design of a quiz's question header + answer cards. Orthogonal to
/// the global `template()` (which retints the palette) — a skin picks the layout
/// and chrome. Default = `Studio`; all v1 skin names remain explicit options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuizSkin {
    /// Restrained editorial cards: rounded, crisp, one accent and clear type.
    #[default]
    Studio,
    /// Framed question panel + a filled letter-badge on each answer card. The
    /// bold, modern quiz-app v1 look.
    Badge,
    /// Editorial: a kicker over a thin accent rule, outline-only answer rows.
    Minimal,
    /// Dark glass panels with glowing accent borders (high-energy Reels look).
    Glass,
    /// The original flat cards with an inline letter (kept for back-compat).
    Plain,
}

/// How a quiz question's text is revealed. Default = `Type` (typewriter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuizReveal {
    /// Character-by-character draw-on (typewriter). The default.
    #[default]
    Type,
    /// Whole line fades up from transparent.
    Fade,
    /// Whole line slides up into place while fading in.
    Rise,
    /// Whole line pops in with a scale overshoot.
    Pop,
    /// Appears instantly on the first frame (no reveal).
    Cut,
}

#[derive(Debug, Clone)]
pub struct QuizOpt {
    pub card: String,
    pub text: String,
    pub correct: bool,
    /// Whether this option has a filled badge behind its A/B/C/D (or numeric)
    /// label. The runner uses this for the correct-state badge overlay.
    pub badge: bool,
    /// Every slide-in part of this card (card, badge, letter, text) paired with
    /// its offset from the card centre, so `run` can move + fade the whole card
    /// as a unit regardless of skin.
    pub parts: Vec<(String, macroquad::prelude::Vec2)>,
}

impl Scene {
    pub fn new() -> Self {
        Scene {
            canvas_size: macroquad::prelude::Vec2::new(1280.0, 720.0),
            ..Scene::default()
        }
    }

    /// Set the logical viewport before constructors run.
    pub fn set_canvas_size(&mut self, width: f32, height: f32) {
        self.canvas_size = macroquad::prelude::Vec2::new(width.max(1.0), height.max(1.0));
    }

    /// Logical viewport used by responsive kits.
    pub fn canvas(&self) -> macroquad::prelude::Vec2 {
        let v = self.canvas_size;
        if v.x > 0.0 && v.y > 0.0 {
            v
        } else {
            macroquad::prelude::Vec2::new(1280.0, 720.0)
        }
    }

    /// Add an entity. Panics on duplicate id.
    pub fn add(&mut self, e: Entity) -> usize {
        assert!(
            !self.index.contains_key(&e.id),
            "duplicate entity id {:?}",
            e.id
        );
        let i = self.entities.len();
        self.index.insert(e.id.clone(), EntitySlot::D2(i));
        self.entities.push(e);
        i
    }

    /// Add a 3D entity. Panics on an id already used by either dimension.
    pub fn add_3d(&mut self, e: Entity3D) -> usize {
        assert!(
            !self.index.contains_key(&e.id),
            "duplicate entity id {:?}",
            e.id
        );
        let i = self.entities_3d.len();
        self.index.insert(e.id.clone(), EntitySlot::D3(i));
        self.entities_3d.push(e);
        i
    }

    pub fn get(&self, id: &str) -> Option<&Entity> {
        match self.index.get(id) {
            Some(EntitySlot::D2(i)) => Some(&self.entities[*i]),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Entity> {
        match self.index.get(id).copied() {
            Some(EntitySlot::D2(i)) => Some(&mut self.entities[i]),
            _ => None,
        }
    }

    pub fn get_3d(&self, id: &str) -> Option<&Entity3D> {
        match self.index.get(id) {
            Some(EntitySlot::D3(i)) => Some(&self.entities_3d[*i]),
            _ => None,
        }
    }

    pub fn get_3d_mut(&mut self, id: &str) -> Option<&mut Entity3D> {
        match self.index.get(id).copied() {
            Some(EntitySlot::D3(i)) => Some(&mut self.entities_3d[i]),
            _ => None,
        }
    }

    pub fn contains(&self, id: &str) -> bool {
        self.index.contains_key(id)
    }
}

/// Chainable builder for declaring entities. Obtained from
/// [`crate::movie::Movie::scene`]. Shape methods (`circle`, `rect`, …) add an
/// entity; modifier methods (`color`, `outlined`, `z`, …) apply to the most
/// recently added one, so declarations read top-to-bottom:
///
/// ```ignore
/// m.scene()
///     .circle("A", v(300., 400.), 40.).outlined().label("A")
///     .text("cap", v(640., 650.), "hello").size(30.).hidden();
/// ```
pub struct SceneBuilder<'a> {
    scene: &'a mut Scene,
    last: Option<usize>,
}

impl<'a> SceneBuilder<'a> {
    pub(crate) fn new(scene: &'a mut Scene) -> Self {
        SceneBuilder { scene, last: None }
    }

    fn push(&mut self, e: Entity) -> &mut Self {
        self.last = Some(self.scene.add(e));
        self
    }

    fn last_mut(&mut self) -> &mut Entity {
        let i = self
            .last
            .expect("modifier called before any shape was added");
        &mut self.scene.entities[i]
    }

    // ---- shapes -------------------------------------------------------

    /// Circle centred at `pos` with radius `r`. Cyan-outlined over a dark
    /// panel fill by default (the house style for nodes).
    pub fn circle(&mut self, id: &str, pos: Vec2, r: f32) -> &mut Self {
        let mut e = Entity::new(id, Shape::Circle { r }, pos, style::PANEL);
        e.stroke = StrokeStyle {
            fill: true,
            outline: true,
            outline_color: Some(style::CYAN),
            ..Default::default()
        };
        self.push(e)
    }

    /// Rectangle centred at `pos`. Same default styling as `circle`.
    pub fn rect(&mut self, id: &str, pos: Vec2, w: f32, h: f32) -> &mut Self {
        let mut e = Entity::new(id, Shape::Rect { w, h }, pos, style::PANEL);
        e.stroke = StrokeStyle {
            fill: true,
            outline: true,
            outline_color: Some(style::CYAN),
            ..Default::default()
        };
        self.push(e)
    }

    /// Line from `from` to `to` (absolute coordinates).
    pub fn line(&mut self, id: &str, from: Vec2, to: Vec2) -> &mut Self {
        self.push(Entity::new(id, Shape::Line { to }, from, style::FG))
    }

    /// Arrow from `from` to `to`, head at `to`.
    pub fn arrow(&mut self, id: &str, from: Vec2, to: Vec2) -> &mut Self {
        self.push(Entity::new(id, Shape::Arrow { to }, from, style::FG))
    }

    /// Quadratic bézier from `from` to `to`, bowing sideways by `bend`
    /// pixels (positive = left of travel direction). Reveal with `trace_in`.
    pub fn curve(&mut self, id: &str, from: Vec2, to: Vec2, bend: f32) -> &mut Self {
        let mid = (from + to) / 2.0;
        let d = to - from;
        let len = d.length().max(1e-3);
        let perp = Vec2::new(-d.y, d.x) / len;
        let ctrl = mid + perp * bend;
        self.push(Entity::new(
            id,
            Shape::Curve {
                ctrl,
                to,
                arrow: false,
            },
            from,
            style::FG,
        ))
    }

    /// Curved arrow: [`curve`](Self::curve) with a head at `to`.
    pub fn curve_arrow(&mut self, id: &str, from: Vec2, to: Vec2, bend: f32) -> &mut Self {
        self.curve(id, from, to, bend);
        if let Shape::Curve { arrow, .. } = &mut self.last_mut().shape {
            *arrow = true;
        }
        self
    }

    /// Polygon with absolute points. Animate its `pos` to move it as a unit.
    pub fn polygon(&mut self, id: &str, pts: Vec<Vec2>) -> &mut Self {
        let mut e = Entity::new(id, Shape::Polygon { pts }, Vec2::ZERO, style::PANEL);
        e.stroke = StrokeStyle {
            fill: true,
            outline: true,
            outline_color: Some(style::CYAN),
            ..Default::default()
        };
        self.push(e)
    }

    /// Text centred at `pos`. Mono font, size 28, foreground by default.
    pub fn text(&mut self, id: &str, pos: Vec2, content: &str) -> &mut Self {
        self.push(Entity::new(
            id,
            Shape::Text {
                content: content.into(),
                size: 28.0,
            },
            pos,
            style::FG,
        ))
    }

    /// A row of `n` cells centred on `center`: rects `{prefix}{i}` (with
    /// `.label` children showing `labels[i]`, default empty) and faded index
    /// digits underneath. The bread and butter of bit arrays, hash tables
    /// and ring buffers. All cells carry tag `prefix`.
    pub fn cells(
        &mut self,
        prefix: &str,
        n: usize,
        center: Vec2,
        cell: Vec2,
        gap: f32,
        labels: Option<&[&str]>,
    ) -> &mut Self {
        let stride = cell.x + gap;
        let x0 = center.x - stride * (n as f32 - 1.0) / 2.0;
        for i in 0..n {
            let id = format!("{prefix}{i}");
            let pos = Vec2::new(x0 + stride * i as f32, center.y);
            self.rect(&id, pos, cell.x, cell.y)
                .color(style::PANEL)
                .outline_color(style::CYAN)
                .stroke(2.0)
                .tag(prefix)
                .label(labels.map_or("", |l| l[i]));
            self.text(&format!("{id}.idx"), Vec2::ZERO, &i.to_string())
                .size(14.0)
                .color(style::DIM)
                .follow(&id, Vec2::new(0.0, cell.y / 2.0 + 20.0));
        }
        self
    }

    /// A left-aligned monospace code block: one text entity per line, ids
    /// `{id}.line{i}`, all tagged `id` (so `all(&m.tagged(id), ...)` fades
    /// the whole block). Highlight a line with e.g.
    /// `act().highlight("code.line2", MAGENTA)`.
    pub fn code_block(&mut self, id: &str, pos: Vec2, lines: &[&str], size: f32) -> &mut Self {
        for (i, line) in lines.iter().enumerate() {
            self.text(
                &format!("{id}.line{i}"),
                Vec2::new(pos.x, pos.y + size * 1.6 * i as f32),
                line,
            )
            .size(size)
            .left()
            .tag(id);
        }
        self
    }

    // ---- modifiers (apply to the last shape added) ---------------------

    /// Set the primary (fill) color.
    pub fn color(&mut self, c: Color) -> &mut Self {
        self.last_mut().color = c;
        self
    }

    /// Outline only: no fill, cyan-colored stroke unless overridden.
    pub fn outlined(&mut self) -> &mut Self {
        let e = self.last_mut();
        e.stroke.fill = false;
        e.stroke.outline = true;
        self
    }

    /// Fill only, no outline.
    pub fn filled(&mut self) -> &mut Self {
        let e = self.last_mut();
        e.stroke.fill = true;
        e.stroke.outline = false;
        self
    }

    /// Outline thickness in pixels (also line/arrow thickness).
    pub fn stroke(&mut self, w: f32) -> &mut Self {
        self.last_mut().stroke.width = w;
        self
    }

    /// Outline color, independent of the fill color.
    pub fn outline_color(&mut self, c: Color) -> &mut Self {
        self.last_mut().stroke.outline_color = Some(c);
        self
    }

    /// Text size (points). Only meaningful for `text` entities.
    pub fn size(&mut self, s: f32) -> &mut Self {
        if let Shape::Text { size, .. } = &mut self.last_mut().shape {
            *size = s;
        }
        self
    }

    /// Use the heavy display font (headlines / banners).
    pub fn display(&mut self) -> &mut Self {
        self.last_mut().font = FontKind::Display;
        self
    }

    /// Use the bold mono font.
    pub fn mono_bold(&mut self) -> &mut Self {
        self.last_mut().font = FontKind::MonoBold;
        self
    }

    /// Draw order; higher on top.
    pub fn z(&mut self, z: i32) -> &mut Self {
        self.last_mut().z = z;
        self
    }

    /// Neon halo intensity multiplier (0 = crisp, no glow; 1 = house default).
    pub fn glow(&mut self, g: f32) -> &mut Self {
        self.last_mut().glow = g;
        self
    }

    /// Start invisible (opacity 0) — reveal later with `fade_in`.
    pub fn hidden(&mut self) -> &mut Self {
        self.last_mut().opacity = 0.0;
        self
    }

    /// Explicit starting opacity.
    pub fn opacity(&mut self, o: f32) -> &mut Self {
        self.last_mut().opacity = o;
        self
    }

    /// Rotation in degrees (text only — e.g. rubber stamps).
    pub fn rot(&mut self, deg: f32) -> &mut Self {
        self.last_mut().rot = deg;
        self
    }

    /// Word-wrap this text entity at `px` logical pixels; wrapped lines are
    /// centred as a block on the entity's position.
    pub fn wrap(&mut self, px: f32) -> &mut Self {
        self.last_mut().wrap = Some(px);
        self
    }

    /// Left-align this text entity on its position.
    pub fn left(&mut self) -> &mut Self {
        self.last_mut().align = Align::Left;
        self
    }

    /// Start with nothing drawn (trace 0) — reveal with `trace_in`
    /// (stroked shapes) or `type_in` (text).
    pub fn untraced(&mut self) -> &mut Self {
        self.last_mut().trace = 0.0;
        self
    }

    /// Add a group tag; address the group with `Movie::tagged` + `all(...)`.
    pub fn tag(&mut self, tag: &str) -> &mut Self {
        self.last_mut().tags.push(tag.into());
        self
    }

    /// Keep this entity fixed to screen coordinates while the camera pans or
    /// zooms. Useful for HUD-style overlays; normal page elements are not
    /// sticky by default.
    pub fn sticky(&mut self) -> &mut Self {
        self.last_mut().sticky = true;
        self
    }

    /// Pin this entity's position to another entity plus an offset. Its
    /// opacity is also multiplied by the followed entity's opacity.
    pub fn follow(&mut self, id: &str, offset: Vec2) -> &mut Self {
        self.last_mut().follow = Some((id.into(), offset));
        self
    }

    /// Attach a centred text label riding on this entity, addressable as
    /// `"{parent}.label"`.
    pub fn label(&mut self, text: &str) -> &mut Self {
        let (parent_id, parent_z, parent_sticky) = {
            let e = self.last_mut();
            (e.id.clone(), e.z, e.sticky)
        };
        let mut lbl = Entity::new(
            format!("{parent_id}.label"),
            Shape::Text {
                content: text.into(),
                size: 24.0,
            },
            Vec2::ZERO,
            style::FG,
        );
        lbl.font = FontKind::MonoBold;
        lbl.z = parent_z + 1;
        lbl.sticky = parent_sticky;
        lbl.follow = Some((parent_id, Vec2::ZERO));
        self.push(lbl)
    }
}
