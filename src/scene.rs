//! The [`Scene`]: an id-addressed store of entities, plus the chainable
//! [`SceneBuilder`] used to declare the time-zero state of a movie.

use std::collections::HashMap;

use macroquad::prelude::{Color, Vec2};

use crate::primitives::{Align, Entity, FontKind, Shape, StrokeStyle};
use crate::style;

/// An id-addressed collection of entities. This is the *base* state of the
/// world at t = 0; the timeline produces per-frame copies of it.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub entities: Vec<Entity>,
    index: HashMap<String, usize>,
    /// Build-time slot occupancy for stateful structures (e.g. `array`): maps a
    /// structure id to the entity id sitting in each slot. Seeded by the
    /// constructor and updated by mutating verbs like `swap`, so a chain of
    /// swaps knows the *current* occupant of each slot. Build-time only — the
    /// renderer never reads it.
    pub occ: HashMap<String, Vec<String>>,
}

impl Scene {
    pub fn new() -> Self {
        Scene::default()
    }

    /// Add an entity. Panics on duplicate id.
    pub fn add(&mut self, e: Entity) -> usize {
        assert!(
            !self.index.contains_key(&e.id),
            "duplicate entity id {:?}",
            e.id
        );
        let i = self.entities.len();
        self.index.insert(e.id.clone(), i);
        self.entities.push(e);
        i
    }

    pub fn get(&self, id: &str) -> Option<&Entity> {
        self.index.get(id).map(|&i| &self.entities[i])
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Entity> {
        self.index
            .get(id)
            .copied()
            .map(move |i| &mut self.entities[i])
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
