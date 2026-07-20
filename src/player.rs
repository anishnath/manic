//! The runtime: live preview window with transport controls, or offline
//! recording, both driving the same pure `Timeline::apply(base, t)`.
//!
//! Every frame renders into an offscreen render target at the output
//! resolution; live mode blits it to the window (fit, centred, optional
//! CRT pass), record mode reads the pixels back. Window size never affects
//! recorded output.
//!
//! Live controls: `Space` pause, `←/→` frame step, `,`/`.` ±1 s, `1`–`9`
//! stage jump, clickable stage strip, `F` / `Ctrl+Cmd+F` fullscreen, `R`
//! restart, drag bottom bar to scrub. With no named stages, digits use sections.
//! The HUD is live-only.
//!
//! CLI flags (after `--`):
//! - `--record [dir]`  render offline (default sink: ffmpeg pipe → out.mp4)
//! - `--fps N`         output frame rate (default 60)
//! - `--scale F`       supersampling (default 1.5 recorded → 1080p, 1 live)
//! - `--from S --to S` record a time range (clips for social posts)
//! - `--stage NAME`     preview/record exactly one named story stage
//! - `--from-stage NAME --to-stage NAME` inclusive named story range
//! - `--frames N`      hard frame cap (smoke tests)
//! - `--still S`       export the single frame at time S as PNG and exit
//! - `--canvas FORMAT`  handled by the CLI before lowering; reframe responsive source
//! - `--alpha`         transparent background, no chrome, PNG sequence
//! - `--png`           force PNG sequence instead of the ffmpeg pipe
//! - `--gif`           pipe frames into out.gif instead of out.mp4
//! - `--crt`           CRT scanline + bloom + vignette post-process
//! - `--intro`         play the branding clip at the START (default is the END)

use macroquad::prelude::*;

use crate::movie::Movie;
use crate::record::Recorder;
use crate::render::{self, View};
use crate::style::{self, Fonts};

#[derive(Debug)]
pub(crate) struct Opts {
    pub record: Option<String>,
    pub fps: u32,
    pub max_frames: Option<u32>,
    pub scale: f32,
    pub still: Option<f32>,
    pub from: f32,
    pub from_set: bool,
    pub to: Option<f32>,
    pub stage: Option<String>,
    pub from_stage: Option<String>,
    pub to_stage: Option<String>,
    pub alpha: bool,
    pub png: bool,
    pub gif: bool,
    pub crt: bool,
    pub template: Option<String>,
    /// Apply branding (intro + watermark) — true only for a branded preset on a
    /// `--record`, and not disabled by `--no-brand`.
    pub branded: bool,
    /// Place the branding clip at the END (outro, default) or the START (intro).
    /// Default true = end; `--intro` puts it at the front.
    pub brand_end: bool,
}

pub(crate) fn parse_opts() -> Result<Opts, String> {
    let args: Vec<String> = std::env::args().collect();
    parse_opts_from(&args)
}

fn parse_opts_from(args: &[String]) -> Result<Opts, String> {
    // pre-scan the preset + branding toggle so they seed the defaults below
    let mut preset_name = String::from("studio");
    let mut no_brand = false;
    {
        let mut j = 1;
        while j < args.len() {
            match args[j].as_str() {
                "--preset" => {
                    if let Some(v) = args.get(j + 1) {
                        preset_name = v.clone();
                    }
                }
                "--no-brand" | "--nobrand" => no_brand = true,
                _ => {}
            }
            j += 1;
        }
    }
    let preset = crate::preset::by_name(&preset_name).unwrap_or_else(|| {
        eprintln!("unknown preset `{preset_name}` — using `studio`");
        crate::preset::default()
    });

    let mut opts = Opts {
        record: None,
        fps: preset.fps,
        max_frames: None,
        scale: 0.0,
        still: None,
        from: 0.0,
        from_set: false,
        to: None,
        stage: None,
        from_stage: None,
        to_stage: None,
        alpha: false,
        png: false,
        gif: preset.gif,
        crt: false,
        template: None,
        branded: false,
        brand_end: true, // default: branding plays as an OUTRO (`--intro` for front)
    };
    let mut i = 1;
    let value = |args: &[String], i: usize, flag: &str| -> Result<String, String> {
        match args.get(i + 1) {
            Some(value) if !value.starts_with("--") => Ok(value.clone()),
            _ => Err(format!("{flag} expects a value")),
        }
    };
    while i < args.len() {
        match args[i].as_str() {
            "--record" => {
                if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    opts.record = Some(args[i + 1].clone());
                    i += 1;
                } else {
                    opts.record = Some("frames".into());
                }
            }
            "--fps" => {
                opts.fps = value(args, i, "--fps")?
                    .parse()
                    .map_err(|_| "--fps expects a positive whole number".to_string())?;
                if opts.fps == 0 {
                    return Err("--fps expects a positive whole number".into());
                }
                i += 1;
            }
            "--scale" => {
                opts.scale = value(args, i, "--scale")?
                    .parse()
                    .map_err(|_| "--scale expects a finite number".to_string())?;
                if !opts.scale.is_finite() {
                    return Err("--scale expects a finite number".into());
                }
                i += 1;
            }
            "--frames" => {
                opts.max_frames = Some(
                    value(args, i, "--frames")?
                        .parse()
                        .map_err(|_| "--frames expects a whole number".to_string())?,
                );
                i += 1;
            }
            "--still" => {
                opts.still = Some(
                    value(args, i, "--still")?
                        .parse()
                        .map_err(|_| "--still expects seconds".to_string())?,
                );
                if opts.still.is_some_and(|still| !still.is_finite()) {
                    return Err("--still expects finite seconds".into());
                }
                i += 1;
            }
            "--from" => {
                opts.from = value(args, i, "--from")?
                    .parse()
                    .map_err(|_| "--from expects seconds".to_string())?;
                opts.from_set = true;
                i += 1;
            }
            "--to" => {
                opts.to = Some(
                    value(args, i, "--to")?
                        .parse()
                        .map_err(|_| "--to expects seconds".to_string())?,
                );
                i += 1;
            }
            "--stage" => {
                opts.stage = Some(value(args, i, "--stage")?);
                i += 1;
            }
            "--from-stage" => {
                opts.from_stage = Some(value(args, i, "--from-stage")?);
                i += 1;
            }
            "--to-stage" => {
                opts.to_stage = Some(value(args, i, "--to-stage")?);
                i += 1;
            }
            "--alpha" => opts.alpha = true,
            "--png" => opts.png = true,
            "--gif" => opts.gif = true,
            "--crt" => opts.crt = true,
            "--outro" | "--brand-end" => opts.brand_end = true,
            "--intro" | "--brand-front" => opts.brand_end = false,
            "--template" | "--theme" => {
                opts.template = Some(value(args, i, "--template")?);
                i += 1;
            }
            "--preset" => {
                let _ = value(args, i, "--preset")?;
                i += 1; // already handled in the pre-scan
            }
            "--no-brand" | "--nobrand" => {} // already handled in the pre-scan
            _ => {}
        }
        i += 1;
    }
    if opts.scale <= 0.0 {
        opts.scale = if opts.record.is_some() || opts.still.is_some() {
            preset.scale
        } else {
            1.0
        };
    }
    // branding: recorded output only, under a branded preset, unless --no-brand
    opts.branded = preset.branded && !no_brand && opts.record.is_some();
    Ok(opts)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlaybackWindow {
    pub from: f32,
    pub to: f32,
    pub label: Option<String>,
}

impl PlaybackWindow {
    fn duration(&self) -> f32 {
        self.to - self.from
    }

    fn progress(&self, t: f32) -> f32 {
        let duration = self.duration();
        if !duration.is_finite() || duration <= 0.0 || !t.is_finite() {
            return 0.0;
        }
        ((t - self.from) / duration).clamp(0.0, 1.0)
    }

    fn time_at(&self, fraction: f32) -> f32 {
        let duration = self.duration();
        if !duration.is_finite() || duration <= 0.0 || !fraction.is_finite() {
            return self.from;
        }
        self.from + fraction.clamp(0.0, 1.0) * duration
    }

    fn settle_time(&self, t: f32) -> (f32, bool) {
        if !t.is_finite() {
            return (self.from, true);
        }
        if t >= self.to {
            (self.to, true)
        } else {
            (t.max(self.from), false)
        }
    }
}

fn visible_stage_ranges(
    stages: &[crate::movie::StoryStage],
    playback: &PlaybackWindow,
) -> Vec<crate::movie::StoryStage> {
    stages
        .iter()
        .filter(|stage| stage.end > playback.from && stage.start < playback.to)
        .cloned()
        .collect()
}

fn stage_span(stage: &crate::movie::StoryStage, playback: &PlaybackWindow) -> (f32, f32) {
    (
        playback.progress(stage.start.max(playback.from)),
        playback.progress(stage.end.min(playback.to)),
    )
}

fn active_stage_index(
    stages: &[crate::movie::StoryStage],
    t: f32,
    playback: &PlaybackWindow,
) -> Option<usize> {
    stages
        .iter()
        .position(|stage| t >= stage.start.max(playback.from) && t < stage.end.min(playback.to))
        .or_else(|| {
            stages
                .last()
                .filter(|stage| t >= playback.to && stage.end >= playback.to)
                .map(|_| stages.len() - 1)
        })
}

fn stage_index_at_fraction(
    stages: &[crate::movie::StoryStage],
    playback: &PlaybackWindow,
    fraction: f32,
) -> Option<usize> {
    let fraction = fraction.clamp(0.0, 1.0);
    stages.iter().enumerate().find_map(|(index, stage)| {
        let (start, end) = stage_span(stage, playback);
        let is_last = index + 1 == stages.len();
        (fraction >= start && (fraction < end || (is_last && fraction <= end))).then_some(index)
    })
}

fn named_stage(movie: &Movie, name: &str) -> Result<crate::movie::StoryStage, String> {
    if let Some(stage) = movie.stage_range(name) {
        return Ok(stage);
    }
    let candidates: Vec<String> = movie
        .stage_ranges()
        .into_iter()
        .map(|stage| stage.name)
        .collect();
    let hint = crate::namehint::nearest_name(name, &candidates)
        .map(|near| format!(" Did you mean `{near}`?"))
        .unwrap_or_default();
    if candidates.is_empty() {
        Err("this movie has no named stages; add `step(\"name\") { ... }`".into())
    } else {
        Err(format!(
            "unknown stage `{name}`.{hint} Available stages: {}",
            candidates.join(", ")
        ))
    }
}

pub(crate) fn playback_window(
    movie: &Movie,
    opts: &Opts,
    timeline_end: f32,
) -> Result<PlaybackWindow, String> {
    if !timeline_end.is_finite() || timeline_end <= 0.0 {
        return Err("the movie timeline must have a finite positive duration".into());
    }
    if !opts.from.is_finite() || opts.to.is_some_and(|to| !to.is_finite()) {
        return Err("numeric `--from`/`--to` values must be finite seconds".into());
    }
    let uses_named_range =
        opts.stage.is_some() || opts.from_stage.is_some() || opts.to_stage.is_some();
    if uses_named_range && (opts.from_set || opts.to.is_some()) {
        return Err(
            "use either named stage ranges (`--stage`/`--from-stage`/`--to-stage`) or numeric `--from`/`--to`, not both"
                .into(),
        );
    }
    if opts.stage.is_some() && (opts.from_stage.is_some() || opts.to_stage.is_some()) {
        return Err("`--stage NAME` already selects one complete stage; do not combine it with `--from-stage` or `--to-stage`".into());
    }

    if let Some(name) = opts.stage.as_deref() {
        let stage = named_stage(movie, name)?;
        return Ok(PlaybackWindow {
            from: stage.start,
            to: stage.end,
            label: Some(stage.name),
        });
    }

    if uses_named_range {
        let from = match opts.from_stage.as_deref() {
            Some(name) => named_stage(movie, name)?.start,
            None => 0.0,
        };
        // `--to-stage` is inclusive: export through the end of that stage.
        let to = match opts.to_stage.as_deref() {
            Some(name) => named_stage(movie, name)?.end,
            None => movie.content_duration(),
        };
        if to <= from {
            return Err(format!(
                "the selected stage range is backwards or empty ({from:.2}s..{to:.2}s)"
            ));
        }
        let label = match (&opts.from_stage, &opts.to_stage) {
            (Some(a), Some(b)) => Some(format!("{a} → {b}")),
            (Some(a), None) => Some(format!("{a} → end")),
            (None, Some(b)) => Some(format!("start → {b}")),
            (None, None) => None,
        };
        return Ok(PlaybackWindow { from, to, label });
    }

    let from = opts.from.clamp(0.0, timeline_end);
    let to = opts.to.unwrap_or(timeline_end).clamp(0.0, timeline_end);
    if to <= from {
        return Err(format!(
            "the selected time range is backwards or empty ({from:.2}s..{to:.2}s)"
        ));
    }
    Ok(PlaybackWindow {
        from,
        to,
        label: None,
    })
}

const CRT_VERT: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
varying lowp vec2 uv;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
}"#;

// CRT: horizontal scanlines, a cheap 4-tap bloom on bright neon, a subtle
// chromatic offset at the edges, and a vignette.
const CRT_FRAG: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform vec2 TexSize;
void main() {
    vec2 px = 1.0 / TexSize;
    // chromatic aberration grows toward the edges
    vec2 d = uv - 0.5;
    float ca = 0.0018 * dot(d, d) * 4.0;
    vec3 c;
    c.r = texture2D(Texture, uv + vec2(ca, 0.0)).r;
    c.g = texture2D(Texture, uv).g;
    c.b = texture2D(Texture, uv - vec2(ca, 0.0)).b;
    // bloom: average a few neighbours, keep only the bright part, add back
    vec3 blur = texture2D(Texture, uv + vec2(px.x, 0.0)).rgb
              + texture2D(Texture, uv - vec2(px.x, 0.0)).rgb
              + texture2D(Texture, uv + vec2(0.0, px.y)).rgb
              + texture2D(Texture, uv - vec2(0.0, px.y)).rgb;
    blur *= 0.25;
    vec3 bright = max(blur - 0.35, 0.0);
    c += bright * 0.6;
    // scanlines
    float scan = 0.92 + 0.08 * sin(uv.y * TexSize.y * 3.14159);
    c *= scan;
    // vignette
    float vig = 1.0 - 0.25 * smoothstep(0.35, 0.95, length(d) * 1.4);
    c *= vig;
    gl_FragColor = vec4(c, 1.0);
}"#;

/// Mirror an image top-to-bottom in place (row-swap of RGBA pixels).
fn flip_vertical(img: &mut Image) {
    let w = img.width as usize;
    let h = img.height as usize;
    let stride = w * 4;
    let bytes = &mut img.bytes;
    for y in 0..h / 2 {
        let top = y * stride;
        let bot = (h - 1 - y) * stride;
        for i in 0..stride {
            bytes.swap(top + i, bot + i);
        }
    }
}

fn fullscreen_pressed() -> bool {
    let command_down = is_key_down(KeyCode::LeftSuper) || is_key_down(KeyCode::RightSuper);
    let control_down = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    is_key_pressed(KeyCode::F)
        || is_key_pressed(KeyCode::F11)
        || (command_down && control_down && is_key_pressed(KeyCode::F))
}

pub(crate) async fn run_loop(mut movie: Movie, opts: Opts) {
    let fonts = Fonts::load();
    // CLI template override (e.g. `--template terminal`)
    if let Some(name) = &opts.template {
        match crate::style::Template::by_name(name) {
            Some(t) => movie.template = t,
            None => eprintln!(
                "unknown template `{name}` — keeping `{}`",
                movie.template.name
            ),
        }
    }
    // branding (recorded output under a branded preset): pin the watermark into
    // the base scene, and build the pre-roll intro as its own little movie.
    if opts.branded {
        crate::branding::add_watermark(&mut movie);
    }
    let (mut base, mut timeline) = movie.finalize();
    let playback = playback_window(&movie, &opts, timeline.dur)
        .expect("stage/time range was validated before the preview window opened");
    let stage_ranges = movie.stage_ranges();
    let visible_stages = visible_stage_ranges(&stage_ranges, &playback);
    let title = movie.title.clone();
    let intro_tpl = crate::style::Template::plain();
    let intro: Option<(crate::scene::Scene, crate::timeline::Timeline)> = if opts.branded {
        let src = crate::branding::intro_source(movie.width, movie.height);
        match crate::parse(&src) {
            Ok(im) => Some(im.finalize()),
            Err(e) => {
                eprintln!("branding intro failed to build: {e:?}");
                None
            }
        }
    } else {
        None
    };
    // Rasterise LaTeX equations at the RENDER scale so they're pixel-sharp (1:1
    // with the display), not up/down-scaled from a fixed density. `opts.scale` is
    // the supersampling factor (view.ss); clamp to a sane range.
    {
        let dpr = opts.scale.clamp(1.5, 8.0);
        let pending = base.pending_eqs.clone();
        for (path, latex, size) in pending {
            // Keep the ordinary white texture available for inline-math runs.
            let _ = crate::latex::render_to_path(&latex, size, dpr, &path);
            if crate::latex::has_explicit_color(&latex) {
                let styled = crate::latex::with_palette_colors(&latex, &movie.template.palette);
                let styled_path = crate::latex::eq_path(&styled, size);
                let _ = crate::latex::render_to_path_preserve(&styled, size, dpr, &styled_path);
                // Whole equations use the template-resolved full-colour image;
                // mixed inline math deliberately retains its parent text tint.
                for e in &mut base.entities {
                    if let crate::primitives::Shape::Image { path: p, tint, .. } = &mut e.shape {
                        if *p == path {
                            *p = styled_path.clone();
                            *tint = false;
                        }
                    }
                }
                timeline.remap_image_path(&path, &styled_path);
            }
        }
        let pending_parts = base.pending_eq_parts.clone();
        for part in pending_parts {
            let _ = crate::latex::render_part_to_path(
                &part.latex,
                part.size,
                dpr,
                part.index,
                part.crop,
                false,
                &part.path,
            );
            if crate::latex::has_explicit_color(&part.latex) {
                let styled =
                    crate::latex::with_palette_colors(&part.latex, &movie.template.palette);
                let styled_path = crate::latex::eq_part_path(&styled, part.size, part.index);
                let _ = crate::latex::render_part_to_path(
                    &styled,
                    part.size,
                    dpr,
                    part.index,
                    part.crop,
                    true,
                    &styled_path,
                );
                for e in &mut base.entities {
                    if let crate::primitives::Shape::Image { path, tint, .. } = &mut e.shape {
                        if *path == part.path {
                            *path = styled_path.clone();
                            *tint = false;
                        }
                    }
                }
                timeline.remap_image_path(&part.path, &styled_path);
            }
        }
    }
    // preload any image textures referenced by the scene (+ the intro) once,
    // before the frame loop — `image(...)` entities draw from this cache
    {
        let mut paths: Vec<String> = Vec::new();
        let mut collect = |sc: &crate::scene::Scene| {
            for e in &sc.entities {
                match &e.shape {
                    crate::primitives::Shape::Image { path, .. } => paths.push(path.clone()),
                    crate::primitives::Shape::RichText { runs, .. } => {
                        for r in runs {
                            if let crate::primitives::TextRun::Math { path, .. } = r {
                                paths.push(path.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
        };
        collect(&base);
        if let Some((ib, _)) = &intro {
            collect(ib);
        }
        paths.extend(timeline.event_image_paths());
        render::preload_textures(paths).await;
    }

    let (w, h) = (movie.width as f32, movie.height as f32);
    let s = opts.scale;
    // Output dimensions MUST be even — h264/yuv420p rejects odd width/height
    // (an odd `--scale` like 0.6667 would otherwise make ffmpeg die → broken pipe).
    let even = |x: f32| -> f32 {
        let n = x.round().max(2.0) as u32;
        (n + (n & 1)) as f32
    };
    let (pw, ph) = (even(w * s), even(h * s));

    let rt = render_target_ex(
        pw as u32,
        ph as u32,
        RenderTargetParams {
            sample_count: 4,
            depth: true,
        },
    );
    rt.texture.set_filter(FilterMode::Linear);
    let rt_cam = Camera2D {
        zoom: vec2(2.0 / pw, 2.0 / ph),
        target: vec2(pw / 2.0, ph / 2.0),
        render_target: Some(rt.clone()),
        ..Default::default()
    };
    // second target for baking the CRT pass into recorded output
    let rt_post = render_target(pw as u32, ph as u32);
    rt_post.texture.set_filter(FilterMode::Linear);
    let rt_post_cam = Camera2D {
        zoom: vec2(2.0 / pw, 2.0 / ph),
        target: vec2(pw / 2.0, ph / 2.0),
        render_target: Some(rt_post.clone()),
        ..Default::default()
    };

    let crt = if opts.crt || movie.template.crt {
        load_material(
            ShaderSource::Glsl {
                vertex: CRT_VERT,
                fragment: CRT_FRAG,
            },
            MaterialParams {
                uniforms: vec![UniformDesc::new("TexSize", UniformType::Float2)],
                ..Default::default()
            },
        )
        .map_err(|e| eprintln!("crt shader failed to compile: {e}"))
        .ok()
    } else {
        None
    };
    if let Some(c) = &crt {
        c.set_uniform("TexSize", vec2(pw, ph));
    }

    let render_at = |base: &crate::scene::Scene,
                     tl: &crate::timeline::Timeline,
                     template: &style::Template,
                     t: f32| {
        set_camera(&rt_cam);
        let mut scene = tl.apply(base, t);
        let view = View::from_scene(&scene, w, h, s);
        if opts.alpha {
            clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        } else {
            render::clear_page_background(template);
        }
        if let Some(cam3) = crate::render3d::camera(&scene, rt.clone(), pw / ph) {
            set_camera(&cam3);
            crate::render3d::draw_scene(&scene, template);
        }
        set_camera(&rt_cam);
        // pin3: reproject each bound 3D point and glue its 2D label onto it, so
        // labels track the geometry as the camera orbits. Compute all screen
        // positions first (immutable borrows), then apply (mutable).
        if !scene.pins.is_empty() {
            let aspect = pw / ph;
            let mut updates: Vec<(String, Vec2, Option<f32>)> = Vec::new();
            let mut hide: Vec<String> = Vec::new();
            // Screen positions of decluttering labels already placed this frame.
            let mut placed: Vec<Vec2> = Vec::new();
            let min_gap = 26.0 * view.ss; // labels closer than this collide
            for pin in &scene.pins {
                let world = match &pin.target {
                    crate::scene::Pin3Target::Point(p) => Some(*p),
                    crate::scene::Pin3Target::Entity(id) => scene.get_3d(id).map(|e| e.pos),
                };
                let Some(world) = world else { continue };
                if let Some(px) = crate::render3d::project(&scene, aspect, world, pw, ph) {
                    // screen-space nudge (scaled with supersampling)
                    let px = px + pin.offset * view.ss;
                    if pin.declutter {
                        if placed.iter().any(|p| p.distance(px) < min_gap) {
                            hide.push(pin.label.clone());
                            continue;
                        }
                        placed.push(px);
                    }
                    // invert View::xform so the overlay lands exactly on `px`
                    let sp = (px / view.ss - view.center) / view.zoom + view.cam;
                    let scale = pin.world_height.and_then(|height| {
                        let pixels = crate::render3d::projected_world_height(
                            &scene, aspect, world, height, pw, ph,
                        )?;
                        let entity = scene.get(&pin.label)?;
                        let em = match entity.shape {
                            crate::primitives::Shape::Text { size, .. }
                            | crate::primitives::Shape::RichText { size, .. } => size,
                            _ => return None,
                        };
                        Some((pixels / (em * view.ss).max(1.0)).clamp(0.15, 8.0))
                    });
                    updates.push((pin.label.clone(), sp, scale));
                }
            }
            for (id, sp, scale) in updates {
                if let Some(e) = scene.get_mut(&id) {
                    e.pos = sp;
                    if let Some(scale) = scale {
                        e.scale = scale;
                    }
                }
            }
            // Suppress colliding declutter labels for this frame only.
            for id in hide {
                if let Some(e) = scene.get_mut(&id) {
                    e.opacity = 0.0;
                }
            }
        }
        if !opts.alpha {
            render::draw_page_chrome(template, &title, w, h, &fonts, &view);
        }
        render::draw_scene(&scene, &fonts, &view, template);
    };

    // crt bake: rt -> rt_post through the material; both passes flip, so
    // orientation matches the plain path.
    //
    // `get_texture_data` returns rows top-down — the correct order for a raw
    // RGBA frame piped to ffmpeg, so the recording path uses it as-is.
    // (macroquad's `Image::export_png` flips internally, so the still path
    // flips once before exporting; see below.)
    let capture = |crt: &Option<Material>| -> Image {
        if let Some(c) = crt {
            set_camera(&rt_post_cam);
            gl_use_material(c);
            draw_texture_ex(
                &rt.texture,
                0.0,
                0.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(pw, ph)),
                    ..Default::default()
                },
            );
            gl_use_default_material();
            set_default_camera();
            rt_post.texture.get_texture_data()
        } else {
            set_default_camera();
            rt.texture.get_texture_data()
        }
    };

    // ---- single still frame ----
    if let Some(ts) = opts.still {
        render_at(&base, &timeline, &movie.template, ts);
        next_frame().await;
        let mut img = capture(&crt);
        // export_png flips internally, so flip once here to cancel it
        flip_vertical(&mut img);
        let path = format!("still_{ts:.2}.png");
        img.export_png(&path);
        println!("wrote {path} ({pw}x{ph})");
        std::process::exit(0);
    }

    // ---- offline record ----
    if let Some(dir) = opts.record.clone() {
        let mut rec = Recorder::new(
            &dir,
            opts.fps,
            pw as u32,
            ph as u32,
            opts.png || opts.alpha,
            opts.gif,
        )
        .expect("cannot create record dir");
        // branding plays as an OUTRO (end) by default; `--intro` moves it to the
        // front. As an intro we trim the ~1s hold tail that `finalize` pads on
        // (clean cut to content); as an outro we play the full clip (it animates
        // in and holds briefly before the video finishes).
        if let (false, Some((ibase, itl))) = (opts.brand_end, &intro) {
            let idur = (itl.dur - 1.0).max(0.3);
            let iframes = (idur * opts.fps as f32).ceil() as u32;
            for f in 0..iframes {
                let t = f as f32 / opts.fps as f32;
                render_at(ibase, itl, &intro_tpl, t);
                let img = capture(&crt);
                rec.capture(&img);
                next_frame().await;
            }
        }
        let end_t = playback.to.min(timeline.dur);
        let total = (((end_t - playback.from).max(0.0) * opts.fps as f32).ceil() as u32)
            .min(opts.max_frames.unwrap_or(u32::MAX));
        for f in 0..total {
            let t = playback.from + f as f32 / opts.fps as f32;
            render_at(&base, &timeline, &movie.template, t);
            let img = capture(&crt); // top-down already — correct for ffmpeg
            rec.capture(&img);
            next_frame().await;
        }
        if let (true, Some((ibase, itl))) = (opts.brand_end, &intro) {
            let oframes = (itl.dur * opts.fps as f32).ceil() as u32;
            for f in 0..oframes {
                let t = f as f32 / opts.fps as f32;
                render_at(ibase, itl, &intro_tpl, t);
                let img = capture(&crt);
                rec.capture(&img);
                next_frame().await;
            }
        }
        rec.finish_range(
            &movie.sections,
            &movie.marks,
            &stage_ranges,
            playback.from,
            end_t,
        );
        std::process::exit(0);
    }

    // ---- live preview ----
    let mut t: f32 = playback.from;
    let mut paused = false;
    let mut fullscreen = false;
    let frame_dt = 1.0 / opts.fps as f32;

    loop {
        if fullscreen_pressed() {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        if is_key_pressed(KeyCode::Space) {
            paused = !paused;
        }
        if is_key_pressed(KeyCode::Right) {
            paused = true;
            t += frame_dt;
        }
        if is_key_pressed(KeyCode::Left) {
            paused = true;
            t -= frame_dt;
        }
        if is_key_pressed(KeyCode::Period) {
            t += 1.0;
        }
        if is_key_pressed(KeyCode::Comma) {
            t -= 1.0;
        }
        if is_key_pressed(KeyCode::R) {
            t = playback.from;
        }
        let digits = [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
            KeyCode::Key8,
            KeyCode::Key9,
        ];
        for (i, key) in digits.iter().enumerate() {
            if is_key_pressed(*key) {
                if let Some(stage) = visible_stages.get(i) {
                    t = stage.start.max(playback.from);
                } else if visible_stages.is_empty() {
                    if let Some((st, _)) = movie.sections.get(i) {
                        t = *st;
                    }
                }
            }
        }

        let (sw, sh) = (screen_width(), screen_height());
        let bar_y = sh - 26.0;
        let nav_y = bar_y - if visible_stages.is_empty() { 0.0 } else { 24.0 };
        let (mx, my) = mouse_position();
        if is_mouse_button_down(MouseButton::Left) && my >= bar_y {
            paused = true;
            t = playback.time_at(mx / sw);
        } else if is_mouse_button_pressed(MouseButton::Left) && my >= nav_y && my < bar_y {
            if let Some(index) = stage_index_at_fraction(&visible_stages, &playback, mx / sw) {
                t = visible_stages[index].start.max(playback.from);
                paused = true;
            }
        }

        if !paused {
            t += get_frame_time();
        }
        let (settled_t, reached_end) = playback.settle_time(t);
        t = settled_t;
        if reached_end {
            paused = true;
        }

        render_at(&base, &timeline, &movie.template, t);

        // blit to window: fit, centred, letterboxed
        set_default_camera();
        clear_background(BLACK);
        let fit = (sw / pw).min(sh / ph);
        let (dw, dh) = (pw * fit, ph * fit);
        let (dx, dy) = ((sw - dw) / 2.0, (sh - dh) / 2.0);
        if let Some(c) = &crt {
            gl_use_material(c);
        }
        draw_texture_ex(
            &rt.texture,
            dx,
            dy,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(dw, dh)),
                ..Default::default()
            },
        );
        if crt.is_some() {
            gl_use_default_material();
        }

        // ---- HUD (never recorded) ----
        // Intervals are half-open so an exact boundary belongs to the stage
        // beginning there. The selected range's final endpoint remains active
        // for the final visible stage.
        let active_stage_index = active_stage_index(&visible_stages, t, &playback);
        if !visible_stages.is_empty() {
            draw_rectangle(0.0, nav_y, sw, 24.0, Color::new(0.015, 0.015, 0.04, 0.94));
            for (index, stage) in visible_stages.iter().enumerate() {
                let (start, end) = stage_span(stage, &playback);
                let (x0, x1) = (sw * start, sw * end);
                let active = active_stage_index == Some(index);
                draw_rectangle(
                    x0 + 1.0,
                    nav_y + 1.0,
                    (x1 - x0 - 2.0).max(1.0),
                    22.0,
                    if active {
                        Color::new(0.10, 0.30, 0.36, 0.96)
                    } else {
                        Color::new(0.05, 0.06, 0.10, 0.92)
                    },
                );
                let max_chars = ((x1 - x0 - 16.0) / 7.0).floor().max(1.0) as usize;
                let mut label: String = stage.name.chars().take(max_chars).collect();
                if stage.name.chars().count() > max_chars && max_chars > 1 {
                    label.pop();
                    label.push('…');
                }
                draw_text_ex(
                    &format!("{} {label}", index + 1),
                    x0 + 6.0,
                    nav_y + 16.0,
                    TextParams {
                        font: fonts.mono.as_ref(),
                        font_size: 12,
                        color: if active { style::FG } else { style::DIM },
                        ..Default::default()
                    },
                );
            }
        }
        draw_rectangle(0.0, bar_y, sw, 26.0, Color::new(0.02, 0.02, 0.05, 0.9));
        let progress = playback.progress(t);
        draw_rectangle(0.0, bar_y, sw * progress, 3.0, style::MAGENTA);
        for stage in &visible_stages {
            let x = sw * stage_span(stage, &playback).0;
            draw_rectangle(x - 1.0, bar_y, 2.0, 8.0, style::CYAN);
        }
        let frame_no = (t * opts.fps as f32).round() as u32;
        let current_stage = active_stage_index
            .and_then(|index| visible_stages.get(index))
            .map(|stage| stage.name.as_str())
            .unwrap_or("—");
        let selection = playback.label.as_deref().unwrap_or("full story");
        let hud = format!(
            "{}  t={:6.2}s  frame={:5}  stage={}  range={}  [space] play/pause  [1-9] stages  [R] restart",
            if paused { "PAUSED " } else { "PLAYING" },
            t,
            frame_no,
            current_stage,
            selection,
        );
        draw_text_ex(
            &hud,
            10.0,
            bar_y + 18.0,
            TextParams {
                font: fonts.mono.as_ref(),
                font_size: 13,
                font_scale: 1.0,
                font_scale_aspect: 1.0,
                rotation: 0.0,
                color: style::FG,
            },
        );

        next_frame().await;
    }
}

#[cfg(test)]
mod stage_tests {
    use super::{
        active_stage_index, parse_opts_from, playback_window, stage_index_at_fraction, stage_span,
        visible_stage_ranges, Opts, PlaybackWindow,
    };

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn opts() -> Opts {
        Opts {
            record: None,
            fps: 60,
            max_frames: None,
            scale: 1.0,
            still: None,
            from: 0.0,
            from_set: false,
            to: None,
            stage: None,
            from_stage: None,
            to_stage: None,
            alpha: false,
            png: false,
            gif: false,
            crt: false,
            template: None,
            branded: false,
            brand_end: true,
        }
    }

    fn staged_movie() -> crate::movie::Movie {
        crate::parse(
            "step(\"question\") { wait(1); } wait(0.5);\n\
             step(\"experiment\") { wait(2); } wait(0.25);\n\
             step(\"proof\") { wait(1); } wait(0.5);",
        )
        .unwrap()
    }

    #[test]
    fn one_stage_and_inclusive_stage_ranges_resolve_to_exact_boundaries() {
        let movie = staged_movie();
        let mut one = opts();
        one.stage = Some("experiment".into());
        let window = playback_window(&movie, &one, 6.25).unwrap();
        assert_eq!((window.from, window.to), (1.5, 3.75));
        assert_eq!(window.label.as_deref(), Some("experiment"));

        let mut range = opts();
        range.from_stage = Some("experiment".into());
        range.to_stage = Some("proof".into());
        let window = playback_window(&movie, &range, 6.25).unwrap();
        assert_eq!((window.from, window.to), (1.5, 5.25));
        assert_eq!(window.label.as_deref(), Some("experiment → proof"));
    }

    #[test]
    fn named_stage_selection_reports_typos_conflicts_and_backwards_ranges() {
        let movie = staged_movie();
        let mut typo = opts();
        typo.stage = Some("experimnt".into());
        let error = playback_window(&movie, &typo, 6.25).unwrap_err();
        assert!(error.contains("Did you mean `experiment`"), "{error}");

        let mut conflict = opts();
        conflict.stage = Some("proof".into());
        conflict.from_set = true;
        assert!(playback_window(&movie, &conflict, 6.25)
            .unwrap_err()
            .contains("not both"));

        let mut backwards = opts();
        backwards.from_stage = Some("proof".into());
        backwards.to_stage = Some("question".into());
        assert!(playback_window(&movie, &backwards, 6.25)
            .unwrap_err()
            .contains("backwards or empty"));
    }

    #[test]
    fn stage_hud_is_safe_without_stages_and_uses_one_side_of_each_boundary() {
        let playback = PlaybackWindow {
            from: 0.0,
            to: 5.25,
            label: None,
        };
        assert_eq!(active_stage_index(&[], playback.to, &playback), None);

        let stages = staged_movie().stage_ranges();
        assert_eq!(active_stage_index(&stages, 1.5, &playback), Some(1));
        assert_eq!(active_stage_index(&stages, playback.to, &playback), Some(2));
    }

    #[test]
    fn unstaged_transport_covers_scrub_progress_and_the_final_frame_headlessly() {
        let movie = crate::parse("wait(4);").unwrap();
        let playback = playback_window(&movie, &opts(), 5.0).unwrap();
        let visible = visible_stage_ranges(&movie.stage_ranges(), &playback);

        assert!(visible.is_empty());
        assert_eq!(playback.time_at(-1.0), 0.0);
        assert_eq!(playback.time_at(0.5), 2.5);
        assert_eq!(playback.time_at(2.0), 5.0);
        assert_eq!(playback.progress(-1.0), 0.0);
        assert_eq!(playback.progress(2.5), 0.5);
        assert_eq!(playback.progress(9.0), 1.0);
        assert_eq!(playback.settle_time(-1.0), (0.0, false));
        assert_eq!(playback.settle_time(5.0), (5.0, true));
        assert_eq!(active_stage_index(&visible, 5.0, &playback), None);
        assert_eq!(stage_index_at_fraction(&visible, &playback, 1.0), None);
    }

    #[test]
    fn selected_stage_geometry_is_clipped_normalized_and_boundary_safe() {
        let movie = staged_movie();
        let mut selected = opts();
        selected.stage = Some("experiment".into());
        let playback = playback_window(&movie, &selected, 6.25).unwrap();
        let visible = visible_stage_ranges(&movie.stage_ranges(), &playback);

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].name, "experiment");
        assert_eq!(stage_span(&visible[0], &playback), (0.0, 1.0));
        assert_eq!(stage_index_at_fraction(&visible, &playback, 0.0), Some(0));
        assert_eq!(stage_index_at_fraction(&visible, &playback, 1.0), Some(0));

        let all = PlaybackWindow {
            from: 0.0,
            to: 5.25,
            label: None,
        };
        let visible = visible_stage_ranges(&movie.stage_ranges(), &all);
        let boundary = all.progress(1.5);
        assert_eq!(stage_index_at_fraction(&visible, &all, boundary), Some(1));
    }

    #[test]
    fn invalid_numeric_ranges_are_rejected_and_helpers_remain_total() {
        let movie = staged_movie();
        let mut invalid = opts();
        invalid.from = f32::NAN;
        invalid.from_set = true;
        assert!(playback_window(&movie, &invalid, 6.25)
            .unwrap_err()
            .contains("finite"));

        invalid = opts();
        invalid.to = Some(f32::INFINITY);
        assert!(playback_window(&movie, &invalid, 6.25)
            .unwrap_err()
            .contains("finite"));
        assert!(playback_window(&movie, &opts(), f32::NAN)
            .unwrap_err()
            .contains("finite positive"));

        let zero = PlaybackWindow {
            from: 2.0,
            to: 2.0,
            label: None,
        };
        assert_eq!(zero.progress(2.0), 0.0);
        assert_eq!(zero.time_at(0.5), 2.0);
        assert_eq!(zero.time_at(f32::NAN), 2.0);
        assert_eq!(zero.settle_time(f32::NAN), (2.0, true));
    }

    #[test]
    fn representative_shipped_movies_are_safe_at_transport_endpoints() {
        let cases = [
            (
                "reactive-math-notation",
                include_str!("../examples/reactive-math-notation.manic"),
                false,
            ),
            (
                "sine-wave",
                include_str!("../examples/sine_wave.manic"),
                false,
            ),
            (
                "reactive-world",
                include_str!("../examples/reactive-world.manic"),
                true,
            ),
            (
                "parameter-journeys",
                include_str!("../examples/parameter-journeys.manic"),
                true,
            ),
        ];

        for (name, source, expects_stages) in cases {
            let movie = crate::parse(source).unwrap_or_else(|error| panic!("{name}: {error:?}"));
            let timeline_end = movie.content_duration() + 1.0;
            let playback = playback_window(&movie, &opts(), timeline_end)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            let stages = visible_stage_ranges(&movie.stage_ranges(), &playback);

            assert_eq!(!stages.is_empty(), expects_stages, "{name}");
            assert_eq!(playback.progress(playback.from), 0.0, "{name}");
            assert_eq!(playback.progress(playback.to), 1.0, "{name}");
            assert_eq!(playback.settle_time(playback.to), (playback.to, true));
            let _ = active_stage_index(&stages, playback.to, &playback);
            let _ = stage_index_at_fraction(&stages, &playback, 1.0);
        }
    }

    #[test]
    fn cli_option_errors_are_reported_without_panics() {
        for (values, expected) in [
            (vec!["manic", "lesson.manic", "--stage"], "--stage expects"),
            (
                vec!["manic", "lesson.manic", "--stage", "--record"],
                "--stage expects",
            ),
            (vec!["manic", "lesson.manic", "--fps", "0"], "positive"),
            (vec!["manic", "lesson.manic", "--fps", "fast"], "positive"),
            (vec!["manic", "lesson.manic", "--from", "soon"], "seconds"),
            (vec!["manic", "lesson.manic", "--still", "NaN"], "finite"),
            (vec!["manic", "lesson.manic", "--scale", "inf"], "finite"),
        ] {
            let error = parse_opts_from(&args(&values)).unwrap_err();
            assert!(error.contains(expected), "{values:?}: {error}");
        }

        let parsed = parse_opts_from(&args(&[
            "manic",
            "lesson.manic",
            "--stage",
            "proof",
            "--fps",
            "30",
            "--from",
            "0",
        ]))
        .unwrap();
        assert_eq!(parsed.stage.as_deref(), Some("proof"));
        assert_eq!(parsed.fps, 30);
        assert!(parsed.from_set);
    }
}
