//! Lowering: a parsed [`Program`] → a [`Movie`], via a builtin [`Registry`].
//!
//! This is the seam between the domain-agnostic front end and the kits. The
//! lowerer itself only knows a handful of reserved control-flow names
//! (`title`, `size`, `par`, `seq`, `stagger`, `section`, `wait`, `beat`,
//! `mark`). Every other call name is looked up in the registry, which kits
//! populate with **constructors** (declare entities at t=0) and **verbs**
//! (produce timeline clips). Meaning lives in the kits; structure lives here.
//!
//! Two passes over the top-level statements:
//! 1. **constructors** run in source order, building the base scene;
//! 2. **timeline** statements run in source order, appending clips.
//!
//! So an entity may be referenced by a beat that appears above its
//! declaration — order the cast and the script however reads best.

use std::collections::HashMap;

use macroquad::prelude::{Color, Vec2, Vec3};

use crate::animate::{self, ActBuilder};
use crate::easing::Easing;
use crate::movie::Movie;
use crate::scene::Scene;
use crate::style;
use crate::timeline::Clip;

use super::ast::{Expr, ExprKind, Program, Stmt};
use super::diag::{Error, Span};
use super::parser::parse;
use manic_lang::expand::{canvas_dims, expand};

/// A constructor builtin: declare or modify entities in the base scene.
pub type CtorFn = fn(&mut Scene, &Args) -> Result<(), Error>;
/// A verb builtin: produce a timeline clip (reads the base scene for id lookups).
pub type VerbFn = fn(&Scene, &Args) -> Result<Clip, Error>;
/// A **mutating** verb builtin: produces a clip like a verb, but also gets
/// `&mut Scene` so it can carry build-time state forward between calls (e.g.
/// `swap` updating `Scene::occ`). This is what lets a chain of stateful steps —
/// sorting, stack push/pop, pointer moves — compose across the stateless
/// timeline. Usable inside `par`/`seq`/`stagger` too; block children lower in
/// source order, so the occupancy each one sees stays deterministic.
pub type MutVerbFn = fn(&mut Scene, &Args) -> Result<Clip, Error>;

/// The builtin table. Kits call [`Registry::ctor`] / [`Registry::verb`] to add
/// vocabulary; the lowerer dispatches call names through it.
#[derive(Default)]
pub struct Registry {
    ctors: HashMap<&'static str, CtorFn>,
    verbs: HashMap<&'static str, VerbFn>,
    mut_verbs: HashMap<&'static str, MutVerbFn>,
}

impl Registry {
    pub fn new() -> Registry {
        Registry::default()
    }

    /// Register a constructor (declares/modifies entities at t=0).
    pub fn ctor(&mut self, name: &'static str, f: CtorFn) {
        self.ctors.insert(name, f);
    }

    /// Register a verb (produces a timeline clip).
    pub fn verb(&mut self, name: &'static str, f: VerbFn) {
        self.verbs.insert(name, f);
    }

    /// Register a mutating verb (produces a clip *and* may update `Scene::occ`).
    pub fn mut_verb(&mut self, name: &'static str, f: MutVerbFn) {
        self.mut_verbs.insert(name, f);
    }

    /// `(name, kind)` for every registered builtin (`kind` ∈ `ctor` / `verb` /
    /// `mut_verb`), sorted. Used to keep the `manic-lang` catalog honest — a test
    /// asserts the catalog's name set equals this, so highlighting/autocomplete
    /// never drift from what the engine actually accepts.
    pub fn builtins(&self) -> Vec<(&'static str, &'static str)> {
        let mut v: Vec<(&'static str, &'static str)> = self
            .ctors
            .keys()
            .map(|k| (*k, "ctor"))
            .chain(self.verbs.keys().map(|k| (*k, "verb")))
            .chain(self.mut_verbs.keys().map(|k| (*k, "mut_verb")))
            .collect();
        v.sort();
        v
    }
}

/// Reserved control-flow names handled by the lowerer, never the registry.
fn is_reserved(name: &str) -> bool {
    matches!(
        name,
        "title"
            | "canvas"
            | "template"
            | "masthead"
            | "par"
            | "seq"
            | "stagger"
            | "section"
            | "wait"
            | "beat"
            | "mark"
    )
}

// ---- argument helpers -----------------------------------------------------

/// A typed, span-aware view over a call's arguments. Every accessor produces a
/// friendly error pointing at the right token when the shape is wrong.
pub struct Args<'a> {
    pub name: &'a str,
    pub name_span: Span,
    pub exprs: &'a [Expr],
}

impl<'a> Args<'a> {
    fn get(&self, i: usize) -> Result<&Expr, Error> {
        self.exprs.get(i).ok_or_else(|| {
            Error::new(
                format!(
                    "`{}` needs at least {} argument(s), got {}",
                    self.name,
                    i + 1,
                    self.exprs.len()
                ),
                self.name_span,
            )
        })
    }

    /// Number of arguments supplied.
    pub fn len(&self) -> usize {
        self.exprs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.exprs.is_empty()
    }

    /// A bare word argument (entity id, color name, easing name).
    pub fn ident(&self, i: usize) -> Result<String, Error> {
        let e = self.get(i)?;
        match &e.kind {
            ExprKind::Ident(s) => Ok(s.clone()),
            _ => Err(Error::new(
                format!("argument {} of `{}` should be a name", i + 1, self.name),
                e.span,
            )),
        }
    }

    /// A string-literal argument.
    pub fn text(&self, i: usize) -> Result<String, Error> {
        let e = self.get(i)?;
        match &e.kind {
            ExprKind::Str(s) => Ok(s.clone()),
            _ => Err(Error::new(
                format!(
                    "argument {} of `{}` should be a \"string\"",
                    i + 1,
                    self.name
                ),
                e.span,
            )),
        }
    }

    /// A numeric argument.
    pub fn num(&self, i: usize) -> Result<f32, Error> {
        let e = self.get(i)?;
        match &e.kind {
            ExprKind::Num(n) => Ok(*n),
            _ => Err(Error::new(
                format!("argument {} of `{}` should be a number", i + 1, self.name),
                e.span,
            )),
        }
    }

    /// An optional numeric argument (returns `None` if absent).
    pub fn opt_num(&self, i: usize) -> Result<Option<f32>, Error> {
        match self.exprs.get(i) {
            None => Ok(None),
            Some(e) => match &e.kind {
                ExprKind::Num(n) => Ok(Some(*n)),
                _ => Err(Error::new(
                    format!("argument {} of `{}` should be a number", i + 1, self.name),
                    e.span,
                )),
            },
        }
    }

    /// A `(x, y)` coordinate pair.
    pub fn pair(&self, i: usize) -> Result<Vec2, Error> {
        let e = self.get(i)?;
        match &e.kind {
            ExprKind::Pair(x, y) => Ok(Vec2::new(*x, *y)),
            _ => Err(Error::new(
                format!(
                    "argument {} of `{}` should be a `(x, y)` point",
                    i + 1,
                    self.name
                ),
                e.span,
            )),
        }
    }

    /// A `(x, y, z)` coordinate triple.
    pub fn triple(&self, i: usize) -> Result<Vec3, Error> {
        let e = self.get(i)?;
        match &e.kind {
            ExprKind::Triple(x, y, z) => Ok(Vec3::new(*x, *y, *z)),
            _ => Err(Error::new(
                format!(
                    "argument {} of `{}` should be a `(x, y, z)` point",
                    i + 1,
                    self.name
                ),
                e.span,
            )),
        }
    }

    /// A point: either a literal `(x, y)` or the id of an existing entity
    /// (resolved to its current position).
    pub fn point(&self, i: usize, scene: &Scene) -> Result<Vec2, Error> {
        let e = self.get(i)?;
        match &e.kind {
            ExprKind::Pair(x, y) => Ok(Vec2::new(*x, *y)),
            ExprKind::Ident(id) => scene
                .get(id)
                .map(|ent| ent.pos)
                .ok_or_else(|| Error::new(format!("no entity named `{id}` to point at"), e.span)),
            _ => Err(Error::new(
                format!(
                    "argument {} of `{}` should be a `(x, y)` point or an entity name",
                    i + 1,
                    self.name
                ),
                e.span,
            )),
        }
    }

    /// Error if there are more than `max` arguments.
    pub fn max(&self, max: usize) -> Result<(), Error> {
        if self.exprs.len() > max {
            Err(Error::new(
                format!("`{}` takes at most {} argument(s)", self.name, max),
                self.exprs[max].span,
            ))
        } else {
            Ok(())
        }
    }

    /// Span of argument `i` (falls back to the call name's span).
    pub fn span_of(&self, i: usize) -> Span {
        self.exprs.get(i).map(|e| e.span).unwrap_or(self.name_span)
    }
}

// ---- shared resolvers used by kits ---------------------------------------

/// Map a friendly color name to the neon palette. Kits use this for color
/// arguments so the whole language shares one vocabulary.
pub fn resolve_color(name: &str, span: Span) -> Result<Color, Error> {
    Ok(match name {
        "fg" | "white" => style::FG,
        "void" | "bg" => style::VOID,
        "cyan" | "blue" => style::CYAN,
        "magenta" | "pink" | "accent" | "red" => style::MAGENTA,
        "lime" | "green" => style::LIME,
        "gold" | "amber" | "yellow" => style::GOLD,
        "dim" | "gray" | "grey" => style::DIM,
        "panel" => style::PANEL,
        // `rainbow` is a per-element spectrum used by bar builtins (histogram,
        // …); where a single colour is needed it falls back to cyan.
        "rainbow" => style::CYAN,
        other => {
            return Err(Error::new(
                format!(
                    "unknown color `{other}` (try: fg, cyan, magenta, lime, gold, dim, panel, void)"
                ),
                span,
            ))
        }
    })
}

/// Map a friendly easing name to an [`Easing`] curve.
pub fn resolve_easing(name: &str, span: Span) -> Result<Easing, Error> {
    Ok(match name {
        "linear" => Easing::Linear,
        "smooth" | "inout" => Easing::InOutCubic,
        "in" => Easing::InCubic,
        "out" => Easing::OutCubic,
        "overshoot" | "back" => Easing::OutBack,
        "bounce" => Easing::OutBounce,
        "elastic" | "spring" => Easing::OutElastic,
        other => {
            return Err(Error::new(
                format!(
                    "unknown easing `{other}` (try: smooth, linear, in, out, overshoot, bounce, elastic)"
                ),
                span,
            ))
        }
    })
}

/// Apply optional trailing `duration` (number) and `easing` (name) arguments,
/// starting at argument index `from`, to an [`ActBuilder`]. This is the shared
/// `verb(id, target, [dur], [ease])` tail every motion verb uses.
pub fn apply_dur_ease(mut b: ActBuilder, a: &Args, from: usize) -> Result<ActBuilder, Error> {
    if let Some(d) = a.opt_num(from)? {
        b = b.dur(d);
    }
    if a.exprs.len() > from + 1 {
        let name = a.ident(from + 1)?;
        b = b.ease(resolve_easing(&name, a.span_of(from + 1))?);
    }
    Ok(b)
}

// ---- the lowerer ----------------------------------------------------------

/// Parse and lower manic source into a runnable [`Movie`], using `registry`
/// for all non-control-flow calls.
pub fn lower(src: &str, registry: &Registry) -> Result<Movie, Error> {
    let prog = parse(src)?;
    let prog = expand(&prog)?;
    lower_program(&prog, registry)
}

// The expand pass (let/for/if/def/reductions/interpolation) lives in the
// macroquad-free `manic-lang` crate so the browser can run it too.
// Re-exported via `crate::lang`; used here by `lower()` + the canvas scan.

fn args_of<'a>(s: &'a Stmt) -> Args<'a> {
    Args {
        name: &s.name,
        name_span: s.name_span,
        exprs: &s.args,
    }
}

fn lower_program(prog: &Program, registry: &Registry) -> Result<Movie, Error> {
    // phase 0 — movie metadata (title/size/template); first occurrence wins
    let mut title = "manic".to_string();
    let (mut w, mut h) = (1280u32, 720u32);
    let mut template: Option<(String, Span)> = None;
    let mut masthead: Option<(String, Option<String>)> = None;
    for s in &prog.stmts {
        match s.name.as_str() {
            "title" => title = args_of(s).text(0)?,
            "canvas" => {
                let (cw, ch) = canvas_dims(&s.args, s.name_span)?;
                w = cw;
                h = ch;
            }
            "template" => {
                if template.is_none() {
                    template = Some((args_of(s).text(0)?, s.name_span));
                }
            }
            "masthead" => {
                if masthead.is_none() {
                    let a = args_of(s);
                    let right = if a.len() > 1 { Some(a.text(1)?) } else { None };
                    masthead = Some((a.text(0)?, right));
                }
            }
            _ => {}
        }
    }
    let mut movie = Movie::new(&title, w, h);
    if let Some((name, span)) = template {
        movie.template = crate::style::Template::by_name(&name).ok_or_else(|| {
            Error::new(
                format!("unknown template `{name}` — try `plain` or `terminal`"),
                span,
            )
        })?;
    }
    if let Some((left, right)) = masthead {
        movie.template.masthead_left = left;
        if let Some(right) = right {
            movie.template.masthead_right = right;
        }
    }

    // classify + fail fast on unknown names
    for s in &prog.stmts {
        classify(&s.name, s.name_span, registry)?;
    }

    // phase A — constructors, in source order
    for s in &prog.stmts {
        if let Some(f) = registry.ctors.get(s.name.as_str()) {
            run_ctor(*f, &mut movie.scene, s)?;
        }
    }

    // phase B — timeline statements, in source order
    for s in &prog.stmts {
        match Class::of(&s.name, registry) {
            Class::Timeline => lower_top_timeline(&mut movie, s, registry)?,
            _ => {}
        }
    }

    Ok(movie)
}

enum Class {
    Meta,
    Ctor,
    Timeline,
    Unknown,
}

impl Class {
    fn of(name: &str, registry: &Registry) -> Class {
        if matches!(name, "title" | "canvas" | "template" | "masthead") {
            Class::Meta
        } else if registry.ctors.contains_key(name) {
            Class::Ctor
        } else if is_reserved(name)
            || registry.verbs.contains_key(name)
            || registry.mut_verbs.contains_key(name)
        {
            Class::Timeline
        } else {
            Class::Unknown
        }
    }
}

fn classify(name: &str, span: Span, registry: &Registry) -> Result<(), Error> {
    match Class::of(name, registry) {
        Class::Unknown => Err(Error::new(
            format!("unknown builtin `{name}` — no kit provides it"),
            span,
        )),
        _ => Ok(()),
    }
}

/// A top-level timeline statement: a section/beat/mark, a `par`/`seq`/`stagger`
/// block, or a verb — appended to the movie's timeline at the cursor.
fn lower_top_timeline(movie: &mut Movie, s: &Stmt, registry: &Registry) -> Result<(), Error> {
    let a = args_of(s);
    match s.name.as_str() {
        "section" => {
            movie.section(&a.text(0)?);
            Ok(())
        }
        "mark" => {
            movie.mark(&a.text(0)?);
            Ok(())
        }
        "wait" | "beat" => {
            movie.wait(a.num(0)?);
            Ok(())
        }
        "par" | "seq" | "stagger" => {
            let clip = build_block_scene(&mut movie.scene, s, registry)?;
            movie.play(clip);
            Ok(())
        }
        _ => {
            // a mutating verb carries state forward, so it gets `&mut scene`
            if let Some(f) = registry.mut_verbs.get(s.name.as_str()) {
                let clip = f(&mut movie.scene, &args_of(s))?;
                movie.play(clip);
                return Ok(());
            }
            // a plain verb; needs a read of the (now complete) base scene
            let f = registry
                .verbs
                .get(s.name.as_str())
                .expect("classified as verb");
            let clip = run_verb(*f, &movie.scene, s)?;
            movie.play(clip);
            Ok(())
        }
    }
}

/// Ids of entities carrying `tag` (empty if none).
fn tagged_ids(scene: &Scene, tag: &str) -> Vec<String> {
    scene
        .entities
        .iter()
        .filter(|e| e.tags.iter().any(|t| t == tag))
        .map(|e| e.id.clone())
        .chain(
            scene
                .entities_3d
                .iter()
                .filter(|e| e.tags.iter().any(|t| t == tag))
                .map(|e| e.id.clone()),
        )
        .collect()
}

/// Invoke a constructor/modifier, broadcasting over a tag group if the first
/// argument names a tag rather than an entity — so `hidden(g.nodes)` or
/// `color(g.edges, dim)` apply to the whole group at t=0.
fn run_ctor(f: CtorFn, scene: &mut Scene, s: &Stmt) -> Result<(), Error> {
    if let Some(first) = s.args.first() {
        if let ExprKind::Ident(name) = &first.kind {
            if !scene.contains(name) {
                let ids = tagged_ids(scene, name);
                if !ids.is_empty() {
                    for id in ids {
                        let mut args2 = s.args.clone();
                        args2[0] = Expr {
                            kind: ExprKind::Ident(id),
                            span: first.span,
                        };
                        let a2 = Args {
                            name: &s.name,
                            name_span: s.name_span,
                            exprs: &args2,
                        };
                        f(scene, &a2)?;
                    }
                    return Ok(());
                }
            }
        }
    }
    f(scene, &args_of(s))
}

/// Verbs whose first argument is a *structure id they consume* (not a broadcast
/// target): `karaoke`/`wordpop` operate on a `caption` as a whole and look up its
/// `{id}.w0…` words themselves. Because `caption` tags the bare `{id}` (so generic
/// verbs like `show`/`draw` broadcast over its words), these must be excluded from
/// broadcast — otherwise `karaoke(cap)` would fan out to `karaoke(cap.w0)`, …
fn verb_consumes_structure_id(verb: &str) -> bool {
    matches!(verb, "karaoke" | "wordpop")
}

/// Invoke a verb, broadcasting over a tag group if the first argument names a
/// tag rather than an entity. So `draw(g.edges)` runs `draw` on every entity
/// tagged `g.edges`, in parallel — the ergonomic that makes graphs, cells, and
/// other groups usable in the language.
fn run_verb(f: VerbFn, scene: &Scene, s: &Stmt) -> Result<Clip, Error> {
    if let Some(first) = s.args.first() {
        if let ExprKind::Ident(name) = &first.kind {
            if !verb_consumes_structure_id(&s.name) && !scene.contains(name) {
                let ids = tagged_ids(scene, name);
                if !ids.is_empty() {
                    let mut clips = Vec::with_capacity(ids.len());
                    for id in ids {
                        let mut args2 = s.args.clone();
                        args2[0] = Expr {
                            kind: ExprKind::Ident(id),
                            span: first.span,
                        };
                        let a2 = Args {
                            name: &s.name,
                            name_span: s.name_span,
                            exprs: &args2,
                        };
                        clips.push(f(scene, &a2)?);
                    }
                    return Ok(Clip::par(clips));
                }
            }
        }
    }
    let clip = f(scene, &args_of(s))?;
    check_clip_targets(scene, &clip, s)?;
    Ok(clip)
}

/// After a verb runs, verify its clip animates only entities that exist — turning
/// the render-time "unknown entity id" panic into a precise **parse-time**
/// diagnostic that points at the offending id and suggests the nearest real name.
/// Broadcast already expanded tags to real ids, so this only bites a mistyped or
/// absent bare id (e.g. `show(ghost)` or `show(cap)` when only `cap.words` exists).
fn check_clip_targets(scene: &Scene, clip: &Clip, s: &Stmt) -> Result<(), Error> {
    for id in clip
        .tracks
        .iter()
        .map(|t| &t.id)
        .chain(clip.events.iter().map(|e| &e.id))
    {
        if !scene.contains(id) {
            return Err(unknown_id_error(scene, id, s));
        }
    }
    Ok(())
}

/// Build the "unknown id" diagnostic: point at the id token the author typed (the
/// first argument, when it matches), else the call name, and offer a "did you
/// mean" suggestion drawn from the scene's entity ids + tags.
fn unknown_id_error(scene: &Scene, id: &str, s: &Stmt) -> Error {
    let span = s
        .args
        .first()
        .filter(|f| matches!(&f.kind, ExprKind::Ident(n) if n == id))
        .map(|f| f.span)
        .unwrap_or(s.name_span);
    let base = format!("`{id}` is not created — no entity or tag has this id");
    match crate::namehint::nearest_name(id, &crate::namehint::candidate_names(scene)) {
        Some(sugg) => Error::new(format!("{base}; did you mean `{sugg}`?"), span),
        None => Error::new(format!("{base} — create it (a shape/text/…) before animating it"), span),
    }
}

/// Lower a statement that appears *inside* a `par`/`seq`/`stagger` block into a
/// clip. Only timeline-producing statements are legal here.
fn lower_inner(scene: &mut Scene, s: &Stmt, registry: &Registry) -> Result<Clip, Error> {
    match s.name.as_str() {
        "wait" | "beat" => Ok(Clip::wait(args_of(s).num(0)?)),
        "par" | "seq" | "stagger" => build_block_scene(scene, s, registry),
        "section" | "mark" => Err(Error::new(
            format!("`{}` can't appear inside a par/seq/stagger block", s.name),
            s.name_span,
        )),
        _ => {
            // a mutating verb carries state forward, so it gets `&mut scene`
            if let Some(f) = registry.mut_verbs.get(s.name.as_str()) {
                return f(scene, &args_of(s));
            }
            match registry.verbs.get(s.name.as_str()) {
                Some(f) => run_verb(*f, scene, s),
                None => {
                    // constructor or unknown used in a timeline block
                    if registry.ctors.contains_key(s.name.as_str()) {
                        Err(Error::new(
                            format!(
                                "`{}` declares an entity and can't appear inside a par/seq/stagger block",
                                s.name
                            ),
                            s.name_span,
                        ))
                    } else {
                        Err(Error::new(
                            format!("unknown builtin `{}`", s.name),
                            s.name_span,
                        ))
                    }
                }
            }
        }
    }
}

fn build_block_scene(scene: &mut Scene, s: &Stmt, registry: &Registry) -> Result<Clip, Error> {
    let block = s.block.as_ref().ok_or_else(|| {
        Error::new(
            format!("`{}` needs a `{{ ... }}` block", s.name),
            s.name_span,
        )
    })?;
    let mut clips = Vec::with_capacity(block.len());
    for inner in block {
        clips.push(lower_inner(scene, inner, registry)?);
    }
    Ok(match s.name.as_str() {
        "par" => Clip::par(clips),
        "seq" => Clip::seq(clips),
        "stagger" => {
            let delay = args_of(s).num(0)?;
            animate::stagger(delay, clips)
        }
        _ => unreachable!(),
    })
}
