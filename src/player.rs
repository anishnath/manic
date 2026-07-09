//! The runtime: live preview window with transport controls, or offline
//! recording, both driving the same pure `Timeline::apply(base, t)`.
//!
//! Every frame renders into an offscreen render target at the output
//! resolution; live mode blits it to the window (fit, centred, optional
//! CRT pass), record mode reads the pixels back. Window size never affects
//! recorded output.
//!
//! Live controls: `Space` pause, `←/→` frame step, `,`/`.` ±1 s, `1`–`9`
//! section jump, `F` / `Ctrl+Cmd+F` fullscreen, `R` restart, drag bottom bar to scrub.
//! The HUD is live-only.
//!
//! CLI flags (after `--`):
//! - `--record [dir]`  render offline (default sink: ffmpeg pipe → out.mp4)
//! - `--fps N`         output frame rate (default 60)
//! - `--scale F`       supersampling (default 1.5 recorded → 1080p, 1 live)
//! - `--from S --to S` record a time range (clips for social posts)
//! - `--frames N`      hard frame cap (smoke tests)
//! - `--still S`       export the single frame at time S as PNG and exit
//! - `--alpha`         transparent background, no chrome, PNG sequence
//! - `--png`           force PNG sequence instead of the ffmpeg pipe
//! - `--gif`           pipe frames into out.gif instead of out.mp4
//! - `--crt`           CRT scanline + bloom + vignette post-process

use macroquad::prelude::*;

use crate::movie::Movie;
use crate::record::Recorder;
use crate::render::{self, View};
use crate::style::{self, Fonts};

pub(crate) struct Opts {
    pub record: Option<String>,
    pub fps: u32,
    pub max_frames: Option<u32>,
    pub scale: f32,
    pub still: Option<f32>,
    pub from: f32,
    pub to: Option<f32>,
    pub alpha: bool,
    pub png: bool,
    pub gif: bool,
    pub crt: bool,
    pub template: Option<String>,
}

pub(crate) fn parse_opts() -> Opts {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = Opts {
        record: None,
        fps: 60,
        max_frames: None,
        scale: 0.0,
        still: None,
        from: 0.0,
        to: None,
        alpha: false,
        png: false,
        gif: false,
        crt: false,
        template: None,
    };
    let mut i = 1;
    let value = |args: &[String], i: usize, flag: &str| -> String {
        args.get(i + 1)
            .unwrap_or_else(|| panic!("{flag} expects a value"))
            .clone()
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
                opts.fps = value(&args, i, "--fps").parse().expect("--fps: number");
                i += 1;
            }
            "--scale" => {
                opts.scale = value(&args, i, "--scale").parse().expect("--scale: number");
                i += 1;
            }
            "--frames" => {
                opts.max_frames = Some(
                    value(&args, i, "--frames")
                        .parse()
                        .expect("--frames: number"),
                );
                i += 1;
            }
            "--still" => {
                opts.still = Some(
                    value(&args, i, "--still")
                        .parse()
                        .expect("--still: seconds"),
                );
                i += 1;
            }
            "--from" => {
                opts.from = value(&args, i, "--from").parse().expect("--from: seconds");
                i += 1;
            }
            "--to" => {
                opts.to = Some(value(&args, i, "--to").parse().expect("--to: seconds"));
                i += 1;
            }
            "--alpha" => opts.alpha = true,
            "--png" => opts.png = true,
            "--gif" => opts.gif = true,
            "--crt" => opts.crt = true,
            "--template" | "--theme" => {
                opts.template = Some(value(&args, i, "--template"));
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    if opts.scale <= 0.0 {
        opts.scale = if opts.record.is_some() || opts.still.is_some() {
            1.5
        } else {
            1.0
        };
    }
    opts
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

pub async fn run_loop(mut movie: Movie) {
    let fonts = Fonts::load();
    let (base, timeline) = movie.finalize();
    let opts = parse_opts();
    // CLI template override (e.g. `--template terminal`)
    if let Some(name) = &opts.template {
        match crate::style::Template::by_name(name) {
            Some(t) => movie.template = t,
            None => eprintln!("unknown template `{name}` — keeping `{}`", movie.template.name),
        }
    }
    let (w, h) = (movie.width as f32, movie.height as f32);
    let s = opts.scale;
    let (pw, ph) = ((w * s).round(), (h * s).round());

    let rt = render_target(pw as u32, ph as u32);
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

    let render_canvas = |t: f32| {
        set_camera(&rt_cam);
        let scene = timeline.apply(&base, t);
        let view = View::from_scene(&scene, w, h, s);
        if opts.alpha {
            clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        } else {
            render::draw_page_chrome(&movie.template, &movie.title, w, h, &fonts, &view);
        }
        render::draw_scene(&scene, &fonts, &view, &movie.template);
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
        render_canvas(ts);
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
        let end_t = opts.to.unwrap_or(timeline.dur).min(timeline.dur);
        let total = (((end_t - opts.from).max(0.0) * opts.fps as f32).ceil() as u32)
            .min(opts.max_frames.unwrap_or(u32::MAX));
        for f in 0..total {
            let t = opts.from + f as f32 / opts.fps as f32;
            render_canvas(t);
            let img = capture(&crt); // top-down already — correct for ffmpeg
            rec.capture(&img);
            next_frame().await;
        }
        rec.finish(&movie.sections, &movie.marks);
        std::process::exit(0);
    }

    // ---- live preview ----
    let mut t: f32 = 0.0;
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
            t = 0.0;
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
                if let Some((st, _)) = movie.sections.get(i) {
                    t = *st;
                }
            }
        }

        let (sw, sh) = (screen_width(), screen_height());
        let bar_y = sh - 26.0;
        let (mx, my) = mouse_position();
        if is_mouse_button_down(MouseButton::Left) && my >= bar_y {
            paused = true;
            t = (mx / sw).clamp(0.0, 1.0) * timeline.dur;
        }

        if !paused {
            t += get_frame_time();
        }
        t = t.clamp(0.0, timeline.dur);

        render_canvas(t);

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
        draw_rectangle(0.0, bar_y, sw, 26.0, Color::new(0.02, 0.02, 0.05, 0.9));
        draw_rectangle(0.0, bar_y, sw * (t / timeline.dur), 3.0, style::MAGENTA);
        for (st, _) in &movie.sections {
            draw_rectangle(sw * (st / timeline.dur) - 1.0, bar_y, 2.0, 8.0, style::CYAN);
        }
        let frame_no = (t * opts.fps as f32).round() as u32;
        let hud = format!(
            "{}  t={:6.2}s  frame={:5}  [space] play/pause  [</>] step  [,/.] +/-1s  [1-9] sections  [F] fullscreen  [R] restart",
            if paused { "PAUSED " } else { "PLAYING" },
            t,
            frame_no
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
