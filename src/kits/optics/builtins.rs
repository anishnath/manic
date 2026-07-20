//! Optics — the author-facing constructors. Each is a *dead-simple* scene a
//! non-programmer can drop in: name it, maybe tweak a number, then `run(id)` to
//! sweep the parameter. The physics (Snell, dispersion) lives in `trace`/
//! `dispersion`; here we only stage entities + a playback track.

use crate::lang::diag::Error;
use crate::lang::lower::Args;
use crate::primitives::{Counter, Entity, Shape};
use crate::scene::{PlaybackTrack, Scene, SimData};
use crate::style;
use crate::timeline::Prop;
use macroquad::prelude::{Color, Vec2, Vec3};

use super::{dispersion, trace};

/// A faint tint for a medium, its strength scaled by how much it bends light
/// (index above vacuum). Air (`n≈1`) reads as nothing; glass a soft wash;
/// diamond a deep one.
fn medium_tint(n: f32) -> f32 {
    ((n - 1.0) * 0.18).clamp(0.0, 0.26)
}

/// `refract(id, [center], [n1], [n2], [angle])` — a single light ray meeting the
/// boundary between two media and **bending** (Snell's law). The top medium has
/// index `n1` (default `1.0`, air), the bottom `n2` (default `1.5`, glass). With
/// no `angle`, `run(id)` **sweeps the incidence angle** from shallow to steep so
/// you watch the refracted ray swing — and, when the light starts in the denser
/// medium (`n1 > n2`), watch **total internal reflection** switch on past the
/// critical angle (the refracted ray vanishes, the reflected ray goes full).
/// Give `angle` (degrees) to freeze a single incidence instead.
///
/// Parts: `{id}.interface`, `{id}.normal`, `{id}.medium1`/`.medium2` (tints),
/// `{id}.incident` (gold, into the surface), `{id}.refracted` (cyan, bent),
/// `{id}.reflected` (orange, faint until TIR), and live readouts `{id}.thetai`/
/// `{id}.thetat`. All tagged bare `{id}` so `color`/`show`/`hidden` broadcast.
pub fn c_refract(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 360.0)
    };
    let n1 = a.opt_num(2)?.unwrap_or(1.0).max(1.0);
    let n2 = a.opt_num(3)?.unwrap_or(1.5).max(1.0);
    let fixed_angle = a.opt_num(4)?;

    let l = 300.0f32; // ray length (px)
    let hw = 380.0f32; // interface half-width
    let (cx, cy) = (center.x, center.y);
    let hit = center;
    let tag = || vec![id.clone(), format!("{id}.parts")];

    // ---- precompute the sweep (or a 2-frame hold at a fixed angle) ----
    let (a0, a1, n) = match fixed_angle {
        Some(deg) => (deg.clamp(1.0, 88.0), deg.clamp(1.0, 88.0), 2usize),
        None => (8.0, 84.0, 96usize),
    };
    let mut src_pts = Vec::with_capacity(n); // incident tail
    let mut refr_to = Vec::with_capacity(n); // refracted head
    let mut refl_to = Vec::with_capacity(n); // reflected head
    let mut refr_op = Vec::with_capacity(n); // refracted ray + its readout (1 refracting, 0 at TIR)
    let mut refl_op = Vec::with_capacity(n);
    let mut tir_op = Vec::with_capacity(n); // the "total internal reflection" callout
    let mut ti_val = Vec::with_capacity(n);
    let mut tt_val = Vec::with_capacity(n);

    for k in 0..n {
        let f = if n <= 1 {
            0.0
        } else {
            k as f32 / (n - 1) as f32
        };
        let deg = a0 + (a1 - a0) * f;
        let th = deg.to_radians();
        let (si, co) = (th.sin(), th.cos());
        // incident arrives from the upper-left, head pinned at the hit point
        src_pts.push(Vec2::new(hit.x - si * l, hit.y - co * l));
        // reflected bounces to the upper-right
        refl_to.push(Vec2::new(hit.x + si * l, hit.y - co * l));
        match trace::snell(th, n1, n2) {
            Some(tt) => {
                let (st, ct) = (tt.sin(), tt.cos());
                refr_to.push(Vec2::new(hit.x + st * l, hit.y + ct * l));
                refr_op.push(Vec2::new(1.0, 0.0));
                refl_op.push(Vec2::new(0.30, 0.0));
                tir_op.push(Vec2::new(0.0, 0.0));
                tt_val.push(Vec2::new(tt.to_degrees(), 0.0));
            }
            None => {
                // total internal reflection: no refracted ray, full reflection
                refr_to.push(hit);
                refr_op.push(Vec2::new(0.0, 0.0));
                refl_op.push(Vec2::new(1.0, 0.0));
                tir_op.push(Vec2::new(1.0, 0.0));
                tt_val.push(Vec2::new(90.0, 0.0));
            }
        }
        ti_val.push(Vec2::new(deg, 0.0));
    }

    // ---- static scenery ----
    let tint = |strength: f32, y: f32, part: &str| -> Entity {
        let mut m = Entity::new(
            format!("{id}.{part}"),
            Shape::Rect {
                w: hw * 2.0,
                h: 230.0,
            },
            Vec2::new(cx, y),
            style::CYAN,
        );
        m.stroke.fill = true;
        m.stroke.outline = false;
        m.opacity = strength;
        m.tags = tag();
        m
    };
    s.add(tint(medium_tint(n1), cy - 118.0, "medium1"));
    s.add(tint(medium_tint(n2), cy + 118.0, "medium2"));

    let mut iface = Entity::new(
        format!("{id}.interface"),
        Shape::Line {
            to: Vec2::new(cx + hw, cy),
        },
        Vec2::new(cx - hw, cy),
        style::FG,
    );
    iface.stroke.width = 2.0;
    iface.tags = tag();
    s.add(iface);

    let mut normal = Entity::new(
        format!("{id}.normal"),
        Shape::Line {
            to: Vec2::new(cx, cy + 168.0),
        },
        Vec2::new(cx, cy - 168.0),
        style::DIM,
    );
    normal.stroke.width = 1.5;
    normal.tags = tag();
    s.add(normal);

    // ---- the three rays ----
    let mut ray =
        |part: &str, from: Vec2, to: Vec2, color: Color, width: f32, glow: f32, op: f32| {
            let mut e = Entity::new(format!("{id}.{part}"), Shape::Arrow { to }, from, color);
            e.stroke.width = width;
            e.glow = glow;
            e.opacity = op;
            e.tags = tag();
            s.add(e);
        };
    ray("incident", src_pts[0], hit, style::GOLD, 3.0, 1.4, 1.0); // tail sweeps, head at hit
    ray(
        "refracted",
        hit,
        refr_to[0],
        style::CYAN,
        3.0,
        1.4,
        refr_op[0].x,
    ); // head sweeps
    ray(
        "reflected",
        hit,
        refl_to[0],
        style::ORANGE,
        2.5,
        1.0,
        refl_op[0].x,
    );

    // ---- live angle readouts ----
    let mut readout = |part: &str, v: f32, prefix: &str, at: Vec2, color: Color| {
        let counter = Counter {
            value: v,
            decimals: 0,
            prefix: prefix.into(),
            suffix: "°".into(),
        };
        let mut e = Entity::new(
            format!("{id}.{part}"),
            Shape::Text {
                content: counter.render(),
                size: 22.0,
            },
            at,
            color,
        );
        e.counter = Some(counter);
        e.tags = tag();
        s.add(e);
    };
    readout(
        "thetai",
        ti_val[0].x,
        "in = ",
        Vec2::new(cx - 150.0, cy - 44.0),
        style::GOLD,
    );
    readout(
        "thetat",
        tt_val[0].x,
        "out = ",
        Vec2::new(cx + 155.0, cy + 66.0),
        style::CYAN,
    );

    // TIR callout — hidden except while the light is totally internally reflected
    let mut tir = Entity::new(
        format!("{id}.tir"),
        Shape::Text {
            content: "total internal reflection".into(),
            size: 22.0,
        },
        Vec2::new(cx, cy + 120.0),
        style::ORANGE,
    );
    tir.opacity = tir_op[0].x;
    tir.tags = tag();
    s.add(tir);

    // ---- playback (the parameter sweep) ----
    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.incident"),
            prop: Prop::Pos,
            points: src_pts,
        },
        PlaybackTrack {
            id: format!("{id}.refracted"),
            prop: Prop::To,
            points: refr_to,
        },
        PlaybackTrack {
            id: format!("{id}.refracted"),
            prop: Prop::Opacity,
            points: refr_op.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.reflected"),
            prop: Prop::To,
            points: refl_to,
        },
        PlaybackTrack {
            id: format!("{id}.reflected"),
            prop: Prop::Opacity,
            points: refl_op,
        },
        PlaybackTrack {
            id: format!("{id}.thetai"),
            prop: Prop::Value,
            points: ti_val,
        },
        PlaybackTrack {
            id: format!("{id}.thetat"),
            prop: Prop::Value,
            points: tt_val,
        },
        // the "out" readout hides at TIR (same on/off as the refracted ray)
        PlaybackTrack {
            id: format!("{id}.thetat"),
            prop: Prop::Opacity,
            points: refr_op,
        },
        PlaybackTrack {
            id: format!("{id}.tir"),
            prop: Prop::Opacity,
            points: tir_op,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["θi".into(), "θt".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy: Vec::new(),
            dt: 1.0 / n as f32,
            states: Vec::new(),
        },
    );
    Ok(())
}

/// `lens(id, [center], [focal], [aperture])` — a **converging lens**: a beam of
/// rays travelling parallel to the axis is bent by the lens to meet at the
/// **focal point** F. This is the ideal thin lens (Gaussian optics — every
/// parallel ray passes through F; the multi-surface `lenssystem` will later add
/// the real spherical aberration). `center` is the lens on the axis (default
/// `(640, 360)`); `focal` the focal length in px (default 240); `aperture` the
/// beam half-height (default 150). With no `focal`, `run(id)` **sweeps the focal
/// length** from long to short so you watch the focus slide IN toward the lens —
/// a shorter focal length is a stronger lens. Give `focal` to freeze one lens.
///
/// Parts: `{id}.axis`, `{id}.lens`, `{id}.focus` (the F dot), `{id}.flabel`,
/// the parallel input rays `{id}.in{i}` and the converging output rays
/// `{id}.out{i}`. All tagged bare `{id}` so `color`/`show`/`hidden` broadcast.
pub fn c_lens(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 360.0)
    };
    let fixed_focal = a.opt_num(2)?;
    let ap = a.opt_num(3)?.unwrap_or(150.0).clamp(40.0, 320.0);
    let (cx, cy) = (center.x, center.y);
    let tag = || vec![id.clone(), format!("{id}.parts")];

    let x_left = cx - 400.0; // where the parallel beam enters
    let x_max = cx + 540.0; // right edge the output rays run to
    let nrays = 7usize; // odd → a chief ray on the axis
    let ys: Vec<f32> = (0..nrays)
        .map(|i| {
            let t = if nrays <= 1 {
                0.0
            } else {
                i as f32 / (nrays - 1) as f32
            };
            -ap + 2.0 * ap * t
        })
        .collect();

    // ---- precompute the focal-length sweep ----
    let (f0, f1, n) = match fixed_focal {
        Some(f) => (f.max(40.0), f.max(40.0), 2usize),
        None => (360.0, 140.0, 90usize), // long → short: the focus slides in
    };
    let mut focus_pts = Vec::with_capacity(n);
    let mut flabel_pts = Vec::with_capacity(n);
    // one To-track of output-ray endpoints per ray
    let mut out_to: Vec<Vec<Vec2>> = vec![Vec::with_capacity(n); nrays];
    for k in 0..n {
        let t = if n <= 1 {
            0.0
        } else {
            k as f32 / (n - 1) as f32
        };
        let f = f0 + (f1 - f0) * t;
        focus_pts.push(Vec2::new(cx + f, cy));
        flabel_pts.push(Vec2::new(cx + f, cy - 26.0));
        // each parallel ray at height y exits the lens vertex heading through F;
        // extend it to the right edge (it crosses the axis at F, then diverges)
        let scale = (x_max - cx) / f;
        for (i, &y) in ys.iter().enumerate() {
            out_to[i].push(Vec2::new(x_max, cy + y * (1.0 - scale)));
        }
    }

    // ---- static scenery ----
    let mut axis = Entity::new(
        format!("{id}.axis"),
        Shape::Line {
            to: Vec2::new(x_max, cy),
        },
        Vec2::new(x_left, cy),
        style::DIM,
    );
    axis.stroke.width = 1.0;
    axis.tags = tag();
    s.add(axis);

    // a biconvex lens outline (two parabolic arcs), faintly filled glass
    let bulge = 24.0f32;
    let steps = 26usize;
    let mut lpts = Vec::with_capacity(2 * steps + 2);
    for k in 0..=steps {
        let y = ap - 2.0 * ap * k as f32 / steps as f32; // top → bottom, right edge
        let b = bulge * (1.0 - (y / ap) * (y / ap));
        lpts.push(Vec2::new(cx + b, cy + y));
    }
    for k in 0..=steps {
        let y = -ap + 2.0 * ap * k as f32 / steps as f32; // bottom → top, left edge
        let b = bulge * (1.0 - (y / ap) * (y / ap));
        lpts.push(Vec2::new(cx - b, cy + y));
    }
    let mut lens = Entity::new(
        format!("{id}.lens"),
        Shape::Polygon { pts: lpts },
        Vec2::ZERO,
        style::CYAN,
    );
    lens.stroke.fill = true;
    lens.stroke.outline = true;
    lens.stroke.width = 2.0;
    lens.opacity = 0.85;
    // fill reads faint via a low-alpha override on the glass body
    lens.stroke.outline_color = Some(style::CYAN);
    lens.tags = tag();
    // give the glass a faint fill by lowering the body colour's alpha
    lens.color = Color::new(style::CYAN.r, style::CYAN.g, style::CYAN.b, 0.14);
    s.add(lens);

    // ---- input rays (parallel, static) + output rays (converging, swept) ----
    for (i, &y) in ys.iter().enumerate() {
        let mut inray = Entity::new(
            format!("{id}.in{i}"),
            Shape::Line {
                to: Vec2::new(cx, cy + y),
            },
            Vec2::new(x_left, cy + y),
            style::GOLD,
        );
        inray.stroke.width = 2.0;
        inray.glow = 1.2;
        inray.tags = tag();
        s.add(inray);

        let mut outray = Entity::new(
            format!("{id}.out{i}"),
            Shape::Line { to: out_to[i][0] },
            Vec2::new(cx, cy + y),
            style::CYAN,
        );
        outray.stroke.width = 2.0;
        outray.glow = 1.2;
        outray.tags = tag();
        s.add(outray);
    }

    // ---- the focal point + its label ----
    let mut focus = Entity::new(
        format!("{id}.focus"),
        Shape::Circle { r: 6.0 },
        focus_pts[0],
        style::GOLD,
    );
    focus.stroke.fill = true;
    focus.stroke.outline = false;
    focus.glow = 2.0;
    focus.tags = tag();
    s.add(focus);

    let mut flabel = Entity::new(
        format!("{id}.flabel"),
        Shape::Text {
            content: "F".into(),
            size: 26.0,
        },
        flabel_pts[0],
        style::GOLD,
    );
    flabel.tags = tag();
    s.add(flabel);

    // ---- playback (the focal-length sweep) ----
    let mut playback = vec![
        PlaybackTrack {
            id: format!("{id}.focus"),
            prop: Prop::Pos,
            points: focus_pts,
        },
        PlaybackTrack {
            id: format!("{id}.flabel"),
            prop: Prop::Pos,
            points: flabel_pts,
        },
    ];
    for (i, track) in out_to.into_iter().enumerate() {
        playback.push(PlaybackTrack {
            id: format!("{id}.out{i}"),
            prop: Prop::To,
            points: track,
        });
    }
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["f".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy: Vec::new(),
            dt: 1.0 / n as f32,
            states: Vec::new(),
        },
    );
    Ok(())
}

/// `prism(id, [center], [glass])` — white light entering a triangular prism and
/// splitting into a **spectrum**. Each colour is traced through both faces with
/// its own refractive index (real Sellmeier dispersion), so the fan is earned:
/// blue bends more than red because glass really does slow blue more. `glass`
/// names the material (`"bk7"` crown by default; also `"sf11"`, `"f2"`,
/// `"diamond"`, `"water"`, `"sapphire"`, `"silica"`). `run(id)` **sweeps the
/// incidence angle**, so the whole rainbow fan swings and its spread breathes
/// (narrowest near minimum deviation).
///
/// Parts: `{id}.prism`, the white `{id}.beam`, the in-glass rays `{id}.in{c}`
/// and the dispersed exit rays `{id}.out{c}` (c = 0 red … 8 violet). All tagged
/// bare `{id}` so `color`/`show`/`hidden` broadcast.
pub fn c_prism(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(540.0, 380.0)
    };
    let glass = if a.len() >= 3 {
        a.text(2)?
    } else {
        "bk7".to_string()
    };
    let (cx, cy) = (center.x, center.y);
    let tag = || vec![id.clone(), format!("{id}.parts")];

    // equilateral prism: apex up, base down
    let apex = Vec2::new(cx, cy - 140.0);
    let bl = Vec2::new(cx - 162.0, cy + 140.0);
    let br = Vec2::new(cx + 162.0, cy + 140.0);
    let rot90 = |v: Vec2| Vec2::new(-v.y, v.x);
    let n_l = rot90((bl - apex).normalize()); // left-face normal (line; sign auto-flipped)
    let n_r = rot90((br - apex).normalize()); // right-face normal
    let e = apex + (bl - apex) * 0.55; // entry point on the left face

    // visible wavelengths (nm): red … violet
    let waves = [
        660.0f32, 625.0, 592.0, 560.0, 535.0, 510.0, 482.0, 455.0, 430.0,
    ];
    let nc = waves.len();
    let x_target = cx + 560.0; // exit rays run out to here

    // ---- sweep the incidence angle ----
    // Range chosen so the internal ray always strikes the far (right) face: near
    // 0° the passage is close to minimum deviation (narrow fan); tilting the beam
    // up raises the incidence angle and the spectrum fans WIDER.
    let (b0, b1, n) = (-3.0f32.to_radians(), -16.0f32.to_radians(), 90usize);
    let mut src_pts = Vec::with_capacity(n); // white beam tail
    let mut in_to: Vec<Vec<Vec2>> = vec![Vec::with_capacity(n); nc];
    let mut out_pos: Vec<Vec<Vec2>> = vec![Vec::with_capacity(n); nc];
    let mut out_to: Vec<Vec<Vec2>> = vec![Vec::with_capacity(n); nc];
    for k in 0..n {
        let t = k as f32 / (n - 1) as f32;
        let beta = b0 + (b1 - b0) * t;
        let d0 = Vec2::new(beta.cos(), beta.sin());
        src_pts.push(e - d0 * 360.0);
        for (ci, &nm) in waves.iter().enumerate() {
            let ni = dispersion::glass_n(&glass, nm / 1000.0);
            let d_in = trace::refract_vec(d0, n_l, 1.0 / ni).unwrap_or(d0);
            let h2 = trace::ray_segment(e, d_in, apex, br).unwrap_or(e + d_in * 220.0);
            let d_out = trace::refract_vec(d_in, n_r, ni).unwrap_or(d_in);
            let s_ext = if d_out.x.abs() > 1e-3 {
                ((x_target - h2.x) / d_out.x).clamp(60.0, 1400.0)
            } else {
                500.0
            };
            in_to[ci].push(h2);
            out_pos[ci].push(h2);
            out_to[ci].push(h2 + d_out * s_ext);
        }
    }

    // ---- static scenery: the prism body ----
    let mut prism = Entity::new(
        format!("{id}.prism"),
        Shape::Polygon {
            pts: vec![apex, br, bl],
        },
        Vec2::ZERO,
        Color::new(style::CYAN.r, style::CYAN.g, style::CYAN.b, 0.12),
    );
    prism.stroke.fill = true;
    prism.stroke.outline = true;
    prism.stroke.width = 2.0;
    prism.stroke.outline_color = Some(style::CYAN);
    prism.tags = tag();
    s.add(prism);

    // ---- the white incoming beam (tail sweeps, head pinned at the entry) ----
    let mut beam = Entity::new(
        format!("{id}.beam"),
        Shape::Line { to: e },
        src_pts[0],
        style::FG,
    );
    beam.stroke.width = 4.0;
    beam.glow = 1.6;
    beam.tags = tag();
    s.add(beam);

    // ---- per-colour in-glass + dispersed exit rays ----
    for (ci, &nm) in waves.iter().enumerate() {
        let col = dispersion::wavelength_rgb(nm);
        let mut inray = Entity::new(
            format!("{id}.in{ci}"),
            Shape::Line { to: in_to[ci][0] },
            e,
            col,
        );
        inray.stroke.width = 2.0;
        inray.glow = 1.2;
        inray.tags = tag();
        s.add(inray);

        let mut outray = Entity::new(
            format!("{id}.out{ci}"),
            Shape::Line { to: out_to[ci][0] },
            out_pos[ci][0],
            col,
        );
        outray.stroke.width = 2.5;
        outray.glow = 1.5;
        outray.tags = tag();
        s.add(outray);
    }

    // ---- playback (the incidence-angle sweep) ----
    let mut playback = vec![PlaybackTrack {
        id: format!("{id}.beam"),
        prop: Prop::Pos,
        points: src_pts,
    }];
    for ci in 0..nc {
        playback.push(PlaybackTrack {
            id: format!("{id}.in{ci}"),
            prop: Prop::To,
            points: in_to[ci].clone(),
        });
        playback.push(PlaybackTrack {
            id: format!("{id}.out{ci}"),
            prop: Prop::Pos,
            points: out_pos[ci].clone(),
        });
        playback.push(PlaybackTrack {
            id: format!("{id}.out{ci}"),
            prop: Prop::To,
            points: out_to[ci].clone(),
        });
    }
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["incidence".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy: Vec::new(),
            dt: 1.0 / n as f32,
            states: Vec::new(),
        },
    );
    Ok(())
}

/// `achromat(id, [center], [aperture])` — the capstone: **chromatic aberration**
/// and its fix. A single lens focuses blue light nearer than red (its index is
/// higher for blue — real dispersion from the glass catalog), so white light
/// never comes to ONE focus; the colours smear along the axis. `run(id)` then
/// **sweeps in the achromatic correction** (a cemented crown+flint doublet),
/// and you watch the red and blue foci slide back together to a single sharp
/// point. The chromatic-aberration direction and relative size are real
/// (Sellmeier); the axial gap is exaggerated for visibility.
///
/// Parts: `{id}.axis`, `{id}.lens`, the white input rays `{id}.in0/.in1`, the
/// red exit rays `{id}.r0/.r1`, the blue exit rays `{id}.b0/.b1`, and the two
/// foci `{id}.fred`/`{id}.fblue`. All tagged bare `{id}`.
pub fn c_achromat(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(540.0, 360.0)
    };
    let ap = a.opt_num(2)?.unwrap_or(120.0).clamp(50.0, 260.0);
    let (cx, cy) = (center.x, center.y);
    let tag = || vec![id.clone(), format!("{id}.parts")];

    // real crown-glass dispersion sets the focal split (f ∝ 1/(n−1))
    let n_red = dispersion::glass_n("bk7", 0.66);
    let n_blue = dispersion::glass_n("bk7", 0.47);
    let target_f = 340.0f32; // the corrected (achromatic) focal length, px
    let raw_r = 1.0 / (n_red - 1.0);
    let raw_b = 1.0 / (n_blue - 1.0);
    let scale = target_f / (0.5 * (raw_r + raw_b));
    let exag = 9.0f32; // blow the (tiny, real) axial gap up so it reads
    let f_r0 = raw_r * scale; // red singlet focal (farther)
    let f_b0 = raw_b * scale; // blue singlet focal (nearer)

    let x_max = cx + 580.0;
    let heights = [ap, -ap]; // two parallel rays; they cross at the focus

    // ---- sweep the correction: α 0 (single lens) → 1 (doublet) ----
    let n = 90usize;
    let red = dispersion::wavelength_rgb(660.0);
    let blue = dispersion::wavelength_rgb(470.0);
    let endpoint = |y: f32, f: f32| Vec2::new(x_max, cy + y * (1.0 - (x_max - cx) / f));
    let mut r_to: Vec<Vec<Vec2>> = vec![Vec::with_capacity(n); 2];
    let mut b_to: Vec<Vec<Vec2>> = vec![Vec::with_capacity(n); 2];
    let mut fred_pts = Vec::with_capacity(n);
    let mut fblue_pts = Vec::with_capacity(n);
    for k in 0..n {
        let alpha = k as f32 / (n - 1) as f32;
        let fr = target_f + (f_r0 - target_f) * exag * (1.0 - alpha);
        let fb = target_f + (f_b0 - target_f) * exag * (1.0 - alpha);
        fred_pts.push(Vec2::new(cx + fr, cy));
        fblue_pts.push(Vec2::new(cx + fb, cy));
        for (i, &y) in heights.iter().enumerate() {
            r_to[i].push(endpoint(y, fr));
            b_to[i].push(endpoint(y, fb));
        }
    }

    // ---- static scenery ----
    let mut axis = Entity::new(
        format!("{id}.axis"),
        Shape::Line {
            to: Vec2::new(x_max, cy),
        },
        Vec2::new(cx - 400.0, cy),
        style::DIM,
    );
    axis.stroke.width = 1.0;
    axis.tags = tag();
    s.add(axis);

    // a biconvex lens body
    let bulge = 24.0f32;
    let steps = 26usize;
    let mut lpts = Vec::with_capacity(2 * steps + 2);
    for k in 0..=steps {
        let y = ap - 2.0 * ap * k as f32 / steps as f32;
        let b = bulge * (1.0 - (y / ap) * (y / ap));
        lpts.push(Vec2::new(cx + b, cy + y));
    }
    for k in 0..=steps {
        let y = -ap + 2.0 * ap * k as f32 / steps as f32;
        let b = bulge * (1.0 - (y / ap) * (y / ap));
        lpts.push(Vec2::new(cx - b, cy + y));
    }
    let mut lens = Entity::new(
        format!("{id}.lens"),
        Shape::Polygon { pts: lpts },
        Vec2::ZERO,
        Color::new(style::CYAN.r, style::CYAN.g, style::CYAN.b, 0.14),
    );
    lens.stroke.fill = true;
    lens.stroke.outline = true;
    lens.stroke.width = 2.0;
    lens.stroke.outline_color = Some(style::CYAN);
    lens.tags = tag();
    s.add(lens);

    // ---- white input rays (parallel, static) ----
    for (i, &y) in heights.iter().enumerate() {
        let mut inray = Entity::new(
            format!("{id}.in{i}"),
            Shape::Line {
                to: Vec2::new(cx, cy + y),
            },
            Vec2::new(cx - 400.0, cy + y),
            style::FG,
        );
        inray.stroke.width = 2.0;
        inray.glow = 1.3;
        inray.tags = tag();
        s.add(inray);
    }

    // ---- red + blue exit rays (converge to their own foci) ----
    for (i, &y) in heights.iter().enumerate() {
        for (name, col, ends) in [("r", red, &r_to), ("b", blue, &b_to)] {
            let mut e = Entity::new(
                format!("{id}.{name}{i}"),
                Shape::Line { to: ends[i][0] },
                Vec2::new(cx, cy + y),
                col,
            );
            e.stroke.width = 2.2;
            e.glow = 1.4;
            e.tags = tag();
            s.add(e);
        }
    }

    // ---- the two focal points ----
    let mut mkdot = |name: &str, at: Vec2, col: Color| {
        let mut d = Entity::new(format!("{id}.{name}"), Shape::Circle { r: 6.0 }, at, col);
        d.stroke.fill = true;
        d.stroke.outline = false;
        d.glow = 2.0;
        d.tags = tag();
        s.add(d);
    };
    mkdot("fred", fred_pts[0], red);
    mkdot("fblue", fblue_pts[0], blue);

    // ---- playback (the correction sweep) ----
    let mut playback = vec![
        PlaybackTrack {
            id: format!("{id}.fred"),
            prop: Prop::Pos,
            points: fred_pts,
        },
        PlaybackTrack {
            id: format!("{id}.fblue"),
            prop: Prop::Pos,
            points: fblue_pts,
        },
    ];
    for i in 0..heights.len() {
        playback.push(PlaybackTrack {
            id: format!("{id}.r{i}"),
            prop: Prop::To,
            points: r_to[i].clone(),
        });
        playback.push(PlaybackTrack {
            id: format!("{id}.b{i}"),
            prop: Prop::To,
            points: b_to[i].clone(),
        });
    }
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["correction".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy: Vec::new(),
            dt: 1.0 / n as f32,
            states: Vec::new(),
        },
    );
    Ok(())
}

/// One refracting surface of a system prescription: signed radius `r` (px;
/// `+` bulges toward the incoming light, huge = flat), axial `thick` to the next
/// vertex (px), the glass FOLLOWING it (`""` = air), the **conic** constant `k`
/// (0 = sphere, `< 0` toward parabola/hyperbola — asphere), and the surface
/// **aperture** (semi-diameter px; `0` = use the system beam aperture).
struct Surf {
    r: f32,
    thick: f32,
    glass: String,
    conic: f32,
    aperture: f32,
}

fn sf(r: f32, thick: f32, glass: &str) -> Surf {
    Surf {
        r,
        thick,
        glass: glass.to_string(),
        conic: 0.0,
        aperture: 0.0,
    }
}

/// A surface with a conic constant (an asphere).
fn sfc(r: f32, thick: f32, glass: &str, conic: f32) -> Surf {
    Surf {
        r,
        thick,
        glass: glass.to_string(),
        conic,
        aperture: 0.0,
    }
}

const FLAT: f32 = 1.0e7;

/// Resolve the `preset` argument: either a **named** design (the easy path — pick
/// a lens by word) or a **custom prescription** string (any arg containing `|`):
/// surfaces separated by `|`, each written `radius thickness glass [conic]
/// [aperture]` (radius in px, `+`/`-`/`flat`; glass name or `air`; optional conic
/// constant and semi-diameter) — e.g. `"200 30 bk7 | -200 0 air"` or an asphere
/// `"190 28 bk7 -2.3 | flat 0 air"`.
fn resolve_prescription(spec: &str) -> (Vec<Surf>, f32, String) {
    if spec.contains('|') {
        return parse_prescription(spec);
    }
    let (surfs, ap, label) = preset(spec);
    (surfs, ap, label.to_string())
}

/// Parse a custom prescription string (see [`resolve_prescription`]). Forgiving:
/// malformed surfaces are skipped; fewer than two falls back to the singlet.
fn parse_prescription(spec: &str) -> (Vec<Surf>, f32, String) {
    let mut surfs = Vec::new();
    for seg in spec.split('|') {
        let toks: Vec<&str> = seg.split_whitespace().collect();
        if toks.len() < 2 {
            continue;
        }
        let r = match toks[0].to_ascii_lowercase().as_str() {
            "flat" | "inf" | "plano" | "planar" => FLAT,
            other => match other.parse::<f32>() {
                Ok(v) if v.abs() > 1e-3 => v,
                _ => FLAT,
            },
        };
        let thick = toks[1].parse::<f32>().unwrap_or(0.0).max(0.0);
        let glass = match toks.get(2).copied().unwrap_or("air") {
            "air" | "" => "",
            g => g,
        };
        let conic = toks
            .get(3)
            .and_then(|t| t.parse::<f32>().ok())
            .unwrap_or(0.0);
        let aperture = toks
            .get(4)
            .and_then(|t| t.parse::<f32>().ok())
            .unwrap_or(0.0)
            .max(0.0);
        surfs.push(Surf {
            r,
            thick,
            glass: glass.to_string(),
            conic,
            aperture,
        });
    }
    if surfs.len() < 2 {
        let (s, ap, _) = preset("singlet");
        return (s, ap, "singlet (fallback)".into());
    }
    (surfs, 120.0, "custom prescription".into())
}

/// Named lens prescriptions (px-scaled, axis-relative). Returns the surfaces,
/// the beam half-aperture, and a human label.
fn preset(name: &str) -> (Vec<Surf>, f32, &'static str) {
    match name {
        // flat back, convex front — the classic plano-convex
        "plano-convex" | "planoconvex" | "plano" => (
            vec![sf(190.0, 28.0, "bk7"), sf(FLAT, 0.0, "")],
            120.0,
            "plano-convex",
        ),
        // same plano-convex but the convex face is a HYPERBOLIC asphere (conic
        // ≈ −n²) that nulls spherical aberration — a fast lens that still focuses
        // to a point
        "aspheric" | "asphere" => (
            vec![sfc(190.0, 28.0, "bk7", -0.55), sf(FLAT, 0.0, "")],
            120.0,
            "aspheric singlet (corrected)",
        ),
        // both surfaces curve the same way — a converging meniscus
        "meniscus" => (
            vec![sf(150.0, 24.0, "bk7"), sf(320.0, 0.0, "")],
            110.0,
            "meniscus",
        ),
        // cemented achromatic-style doublet — power split over 3 surfaces, so it
        // bends each ray less → far less spherical aberration than the singlet
        "doublet" | "achromat" | "achromatic-doublet" => (
            vec![
                sf(300.0, 30.0, "bk7"),
                sf(-210.0, 14.0, "f2"),
                sf(-560.0, 0.0, ""),
            ],
            120.0,
            "doublet (crown + flint)",
        ),
        // three spaced elements — a Cooke-style triplet
        "triplet" | "cooke" | "cooke-triplet" => (
            vec![
                sf(240.0, 24.0, "bk7"),
                sf(2600.0, 30.0, ""),
                sf(-300.0, 14.0, "f2"),
                sf(240.0, 30.0, ""),
                sf(520.0, 24.0, "bk7"),
                sf(-240.0, 0.0, ""),
            ],
            118.0,
            "triplet (3 elements)",
        ),
        // a fast biconvex singlet — its outer rays focus short (spherical aberration)
        _ => (
            vec![sf(200.0, 30.0, "bk7"), sf(-200.0, 0.0, "")],
            120.0,
            "singlet (one element)",
        ),
    }
}

/// `lenssystem(id, [center], [preset])` — a REAL multi-element lens, ray-traced
/// through its actual spherical surfaces (not the ideal thin lens of `lens`).
/// `preset` picks the design: `"singlet"` (default — a fast single element),
/// `"doublet"` (cemented crown + flint), or `"triplet"` (three spaced elements).
/// A parallel beam is traced surface-by-surface with real glass; sketch the rays
/// on with `draw(id.rays)`, and `run(id)` sweeps a **sensor** plane along the
/// axis while the live **spot-size** read-out dips to its minimum at best focus —
/// non-zero for the singlet (**spherical aberration**: outer rays focus short),
/// tight for the doublet/triplet. An f-number read-out sits in the corner.
///
/// Parts: `{id}.elem{k}` (glass), `{id}.axis`, `{id}.ray{i}` (tagged `{id}.rays`),
/// `{id}.sensor` (the sweeping plane), `{id}.spot` (spot µm-ish read-out),
/// `{id}.fnum` (f-number). All tagged bare `{id}`.
pub fn c_lenssystem(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 380.0)
    };
    let name = if a.len() >= 3 {
        a.text(2)?
    } else {
        "singlet".to_string()
    };
    let (surfs, ap, label) = resolve_prescription(&name);
    let (cx, cy) = (center.x, center.y);
    let tag = || vec![id.clone(), format!("{id}.parts")];

    // an optional finite object distance (px) launches a diverging point source;
    // omitted ⇒ a collimated beam (object at infinity)
    let object = a.opt_num(3)?;
    let lx = cx - 150.0; // first vertex x
    let lens_len: f32 = surfs.iter().map(|s| s.thick).sum();
    let x_in = lx - 260.0; // parallel beam entry
                           // a finite (real-image) object throws the image farther out — extend the field
    let x_end = lx + lens_len + if object.is_some() { 620.0 } else { 420.0 };

    // surface vertex x positions
    let mut vxs = Vec::with_capacity(surfs.len());
    let mut acc = lx;
    for sf in &surfs {
        vxs.push(acc);
        acc += sf.thick;
    }
    let last_vx = *vxs.last().unwrap();

    // ---- trace the beam (single wavelength, 546 nm) ----
    let nrays = 11usize;
    let lam = 0.546;
    let heights: Vec<f32> = (0..nrays)
        .map(|i| -ap + 2.0 * ap * i as f32 / (nrays - 1) as f32)
        .collect();
    // per ray: the drawn polyline, plus its final (hit, dir) for the sensor sweep
    let mut ray_polys: Vec<Vec<Vec2>> = Vec::with_capacity(nrays);
    let mut ray_tail: Vec<(Vec2, Vec2)> = Vec::with_capacity(nrays); // (last point, dir)
    for &y in &heights {
        // launch: collimated, or diverging from an on-axis object point
        let (mut o, mut d) = match object {
            Some(u) if u > 1.0 => {
                let p = Vec2::new(lx - u, cy);
                (p, (Vec2::new(lx, cy + y) - p).normalize())
            }
            _ => (Vec2::new(x_in, cy + y), Vec2::new(1.0, 0.0)),
        };
        // draw from the left viewport edge (a distant object point is off-screen)
        let start = if o.x < x_in && d.x.abs() > 1e-4 {
            o + d * ((x_in - o.x) / d.x)
        } else {
            o
        };
        let mut n1 = 1.0f32;
        let mut poly = vec![start];
        let mut ok = true;
        for (i, sf) in surfs.iter().enumerate() {
            let n2 = if sf.glass.is_empty() {
                1.0
            } else {
                dispersion::glass_n(&sf.glass, lam)
            };
            let surf_ap = if sf.aperture > 0.0 {
                sf.aperture
            } else {
                ap * 1.2
            };
            match trace::trace_conic(o, d, vxs[i], sf.r, sf.conic, cy, n1, n2) {
                Some((hit, nd)) if (hit.y - cy).abs() <= surf_ap => {
                    poly.push(hit);
                    o = hit;
                    d = nd;
                    n1 = n2;
                }
                _ => {
                    ok = false; // missed the surface or clipped at its aperture
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        // extend straight out to the far edge
        let end = if d.x.abs() > 1e-4 {
            let t = (x_end - o.x) / d.x;
            o + d * t
        } else {
            o + d * 400.0
        };
        poly.push(end);
        ray_polys.push(poly);
        ray_tail.push((o, d));
    }

    // ---- find best focus + spot size along the axis (sensor sweep data) ----
    let y_at = |tail: &(Vec2, Vec2), x: f32| -> f32 {
        let (p, d) = tail;
        if d.x.abs() < 1e-5 {
            p.y
        } else {
            p.y + (x - p.x) * d.y / d.x
        }
    };
    let x0 = last_vx + 30.0;
    let x1 = x_end - 30.0;
    let nsweep = 90usize;
    let mut sensor_pts = Vec::with_capacity(nsweep);
    let mut spot_pts = Vec::with_capacity(nsweep);
    let mut best_x = x0;
    let mut best_spot = f32::MAX;
    for k in 0..nsweep {
        let x = x0 + (x1 - x0) * k as f32 / (nsweep - 1) as f32;
        let ys: Vec<f32> = ray_tail.iter().map(|t| y_at(t, x)).collect();
        let spot = ys.iter().cloned().fold(f32::MIN, f32::max)
            - ys.iter().cloned().fold(f32::MAX, f32::min);
        if spot < best_spot {
            best_spot = spot;
            best_x = x;
        }
        sensor_pts.push(Vec2::new(x, cy));
        spot_pts.push(Vec2::new(spot, 0.0));
    }
    let focal = best_x - last_vx;
    let fnum = focal / (2.0 * ap);

    // ---- draw the glass elements (each glass run = one lens body) ----
    // conic sag at height yy: x = vx + sign·yy² / (|r| + √(r² − (1+k)·yy²))
    let arc_pts = |vx: f32, r: f32, k: f32, half: f32, top_down: bool| -> Vec<Vec2> {
        let steps = 22i32;
        let mut v = Vec::new();
        for j in 0..=steps {
            let f = j as f32 / steps as f32;
            let yy = if top_down {
                half - 2.0 * half * f
            } else {
                -half + 2.0 * half * f
            };
            let disc = (r * r - (1.0 + k) * yy * yy).max(0.0);
            let sag = r.signum() * yy * yy / (r.abs() + disc.sqrt());
            v.push(Vec2::new(vx + sag, cy + yy));
        }
        v
    };
    let surf_half = |sf: &Surf| if sf.aperture > 0.0 { sf.aperture } else { ap };
    let mut elem = 0;
    for (i, sf) in surfs.iter().enumerate() {
        if sf.glass.is_empty() {
            continue; // air gap — no body starts here
        }
        // this glass body spans surface i (front) to surface i+1 (back)
        if i + 1 >= surfs.len() {
            break;
        }
        let mut body = arc_pts(vxs[i], sf.r, sf.conic, surf_half(sf), true); // front, top→bottom
        body.extend(arc_pts(
            vxs[i + 1],
            surfs[i + 1].r,
            surfs[i + 1].conic,
            surf_half(&surfs[i + 1]),
            false,
        )); // back
        let mut e = Entity::new(
            format!("{id}.elem{elem}"),
            Shape::Polygon { pts: body },
            Vec2::ZERO,
            Color::new(style::CYAN.r, style::CYAN.g, style::CYAN.b, 0.13),
        );
        e.stroke.fill = true;
        e.stroke.outline = true;
        e.stroke.width = 2.0;
        e.stroke.outline_color = Some(style::CYAN);
        e.tags = tag();
        s.add(e);
        elem += 1;
    }

    // optical axis
    let mut axis = Entity::new(
        format!("{id}.axis"),
        Shape::Line {
            to: Vec2::new(x_end, cy),
        },
        Vec2::new(x_in, cy),
        style::DIM,
    );
    axis.stroke.width = 1.0;
    axis.tags = tag();
    s.add(axis);

    // ---- the traced rays (polylines, drawable) ----
    for (i, poly) in ray_polys.iter().enumerate() {
        let mut r = Entity::new(
            format!("{id}.ray{i}"),
            Shape::Polyline { pts: poly.clone() },
            Vec2::ZERO,
            style::CYAN,
        );
        r.stroke.width = 1.6;
        r.glow = 1.3;
        r.tags = vec![id.clone(), format!("{id}.parts"), format!("{id}.rays")];
        s.add(r);
    }

    // ---- the sweeping sensor plane + live spot read-out ----
    let mut sensor = Entity::new(
        format!("{id}.sensor"),
        Shape::Line {
            to: Vec2::new(sensor_pts[0].x, cy + ap * 0.7),
        },
        Vec2::new(sensor_pts[0].x, cy - ap * 0.7),
        style::GOLD,
    );
    sensor.stroke.width = 2.0;
    sensor.tags = tag();
    s.add(sensor);
    // the sensor's `to` endpoint also rides the sweep (a vertical bar)
    let sensor_to: Vec<Vec2> = sensor_pts
        .iter()
        .map(|p| Vec2::new(p.x, cy + ap * 0.7))
        .collect();

    let spot_counter = crate::primitives::Counter {
        value: spot_pts[0].x,
        decimals: 0,
        prefix: "spot = ".into(),
        suffix: " px".into(),
    };
    let mut spot = Entity::new(
        format!("{id}.spot"),
        Shape::Text {
            content: spot_counter.render(),
            size: 22.0,
        },
        Vec2::new(cx + 40.0, cy - ap - 40.0),
        style::GOLD,
    );
    spot.counter = Some(spot_counter);
    spot.tags = tag();
    s.add(spot);

    // f-number + NA read-outs — defined for a collimated (object-at-infinity)
    // beam; a finite object is a different conjugate, so they're omitted there
    if object.is_none() {
        let fcounter = crate::primitives::Counter {
            value: fnum,
            decimals: 1,
            prefix: "f/".into(),
            suffix: "".into(),
        };
        let mut fnume = Entity::new(
            format!("{id}.fnum"),
            Shape::Text {
                content: fcounter.render(),
                size: 24.0,
            },
            Vec2::new(x_in + 40.0, cy - ap - 40.0),
            style::FG,
        );
        fnume.counter = Some(fcounter);
        fnume.tags = tag();
        s.add(fnume);

        // numerical aperture (paraxial): NA ≈ 1 / (2·f/#)
        let na = if fnum > 1e-3 { 1.0 / (2.0 * fnum) } else { 0.0 };
        let ncounter = crate::primitives::Counter {
            value: na,
            decimals: 2,
            prefix: "NA ".into(),
            suffix: "".into(),
        };
        let mut nae = Entity::new(
            format!("{id}.na"),
            Shape::Text {
                content: ncounter.render(),
                size: 20.0,
            },
            Vec2::new(x_in + 40.0, cy - ap - 12.0),
            style::DIM,
        );
        nae.counter = Some(ncounter);
        nae.tags = tag();
        s.add(nae);
    }

    // autofocus: mark the best-focus plane (the sensor's minimum-spot position)
    let mut bestf = Entity::new(
        format!("{id}.bestfocus"),
        Shape::Line {
            to: Vec2::new(best_x, cy + ap * 0.55),
        },
        Vec2::new(best_x, cy - ap * 0.55),
        style::MAGENTA,
    );
    bestf.stroke.width = 1.5;
    bestf.opacity = 0.5;
    bestf.tags = tag();
    s.add(bestf);
    let mut bflab = Entity::new(
        format!("{id}.bestfocuslabel"),
        Shape::Text {
            content: "best focus".into(),
            size: 16.0,
        },
        Vec2::new(best_x, cy + ap * 0.55 + 18.0),
        style::MAGENTA,
    );
    bflab.opacity = 0.7;
    bflab.tags = tag();
    s.add(bflab);

    let mut lbl = Entity::new(
        format!("{id}.label"),
        Shape::Text {
            content: label.into(),
            size: 22.0,
        },
        Vec2::new(cx, cy + ap + 56.0),
        style::DIM,
    );
    lbl.tags = tag();
    s.add(lbl);

    // ---- playback: the sensor sweeps, the spot read-out tracks it ----
    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.sensor"),
            prop: Prop::Pos,
            points: sensor_pts
                .iter()
                .map(|p| Vec2::new(p.x, cy - ap * 0.7))
                .collect(),
        },
        PlaybackTrack {
            id: format!("{id}.sensor"),
            prop: Prop::To,
            points: sensor_to,
        },
        PlaybackTrack {
            id: format!("{id}.spot"),
            prop: Prop::Value,
            points: spot_pts,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["sensor".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy: Vec::new(),
            dt: 1.0 / nsweep as f32,
            states: Vec::new(),
        },
    );
    Ok(())
}

/// Shared T4 analysis: trace a fan of parallel rays through a preset's real
/// spherical surfaces and return, at best focus, each ray's `(pupil_fraction,
/// transverse_aberration_px)` — plus the focal length and aperture (px). This is
/// the goldmine's `computeRayAberration`/`computeSpotDiagram` core, reduced to
/// the rotationally-symmetric on-axis 2-D case both T4 views draw from.
pub(super) fn analyze_preset(name: &str, cy: f32) -> (Vec<(f32, f32)>, f32, f32) {
    let (surfs, ap, _) = resolve_prescription(name);
    let mut vxs = Vec::with_capacity(surfs.len());
    let mut acc = 0.0f32;
    for sf in &surfs {
        vxs.push(acc);
        acc += sf.thick;
    }
    let last_vx = *vxs.last().unwrap();
    let x_in = -260.0f32;
    let lam = 0.546;
    let nrays = 21usize;
    let mut tails: Vec<(f32, Vec2, Vec2)> = Vec::new(); // (hfrac, last pt, dir)
    for i in 0..nrays {
        let hfrac = -1.0 + 2.0 * i as f32 / (nrays - 1) as f32;
        let mut o = Vec2::new(x_in, cy + hfrac * ap);
        let mut d = Vec2::new(1.0, 0.0);
        let mut n1 = 1.0f32;
        let mut ok = true;
        for (j, sf) in surfs.iter().enumerate() {
            let n2 = if sf.glass.is_empty() {
                1.0
            } else {
                dispersion::glass_n(&sf.glass, lam)
            };
            match trace::trace_conic(o, d, vxs[j], sf.r, sf.conic, cy, n1, n2) {
                Some((h, nd)) => {
                    o = h;
                    d = nd;
                    n1 = n2;
                }
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            tails.push((hfrac, o, d));
        }
    }
    let y_at = |t: &(f32, Vec2, Vec2), x: f32| -> f32 {
        let (_, p, d) = t;
        if d.x.abs() < 1e-5 {
            p.y
        } else {
            p.y + (x - p.x) * d.y / d.x
        }
    };
    // best focus = minimum transverse spread
    let (x0, x1) = (last_vx + 30.0, last_vx + 700.0);
    let (mut best_x, mut best) = (x0, f32::MAX);
    for k in 0..200 {
        let x = x0 + (x1 - x0) * k as f32 / 199.0;
        let ys: Vec<f32> = tails.iter().map(|t| y_at(t, x)).collect();
        let spread = ys.iter().cloned().fold(f32::MIN, f32::max)
            - ys.iter().cloned().fold(f32::MAX, f32::min);
        if spread < best {
            best = spread;
            best_x = x;
        }
    }
    let dys: Vec<(f32, f32)> = tails.iter().map(|t| (t.0, y_at(t, best_x) - cy)).collect();
    (dys, best_x - last_vx, ap)
}

/// `rayfan(id, [center], [preset])` — the **ray-fan aberration plot**: transverse
/// ray error at best focus vs pupil height. A flat line at zero is a perfect
/// lens; the singlet's cubic S-curve is textbook **spherical aberration**; the
/// doublet/triplet flatten it. Sketch it on with `draw(id.curve)`. `preset` is
/// `"singlet"`/`"doublet"`/`"triplet"`. Parts `{id}.box/.zerox/.zeroy/.curve/…`.
pub fn c_rayfan(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 360.0)
    };
    let name = if a.len() >= 3 {
        a.text(2)?
    } else {
        "singlet".to_string()
    };
    let (dys, _focal, _ap) = analyze_preset(&name, 0.0);
    let (cx, cy) = (center.x, center.y);
    let tag = || vec![id.clone(), format!("{id}.parts")];
    let (hw, hh) = (320.0f32, 170.0f32); // plot half-width/height

    // scale to the SINGLET's aberration (same for every preset) so a doublet/
    // triplet reads as visibly flatter — not re-normalised to fill the box
    let (ref_dys, _, _) = analyze_preset("singlet", 0.0);
    let ref_max = ref_dys.iter().map(|(_, d)| d.abs()).fold(1.0f32, f32::max);
    let yscale = hh * 0.85 / ref_max;

    // frame + zero axes
    let mut frame = Entity::new(
        format!("{id}.box"),
        Shape::Rect {
            w: hw * 2.0,
            h: hh * 2.0,
        },
        center,
        style::DIM,
    );
    frame.stroke.outline = true;
    frame.stroke.fill = false;
    frame.stroke.width = 1.5;
    frame.tags = tag();
    s.add(frame);
    for (part, from, to) in [
        ("zerox", Vec2::new(cx - hw, cy), Vec2::new(cx + hw, cy)),
        ("zeroy", Vec2::new(cx, cy - hh), Vec2::new(cx, cy + hh)),
    ] {
        let mut ln = Entity::new(format!("{id}.{part}"), Shape::Line { to }, from, style::DIM);
        ln.stroke.width = 1.0;
        ln.opacity = 0.6;
        ln.tags = tag();
        s.add(ln);
    }

    // the aberration curve
    let pts: Vec<Vec2> = dys
        .iter()
        .map(|(h, d)| Vec2::new(cx + h * hw, cy - d * yscale))
        .collect();
    let mut curve = Entity::new(
        format!("{id}.curve"),
        Shape::Polyline { pts },
        Vec2::ZERO,
        style::CYAN,
    );
    curve.stroke.width = 3.0;
    curve.glow = 1.4;
    curve.tags = vec![id.clone(), format!("{id}.parts"), format!("{id}.curve")];
    s.add(curve);

    // labels
    let mklab = |s: &mut Scene, part: &str, at: Vec2, txt: &str, sz: f32| {
        let mut e = Entity::new(
            format!("{id}.{part}"),
            Shape::Text {
                content: txt.into(),
                size: sz,
            },
            at,
            style::DIM,
        );
        e.tags = vec![id.clone(), format!("{id}.parts")];
        s.add(e);
    };
    mklab(
        s,
        "xlabel",
        Vec2::new(cx, cy + hh + 26.0),
        "pupil height  (edge → edge)",
        18.0,
    );
    mklab(
        s,
        "ylabel",
        Vec2::new(cx - hw - 4.0, cy - hh - 22.0),
        "ray error at focus",
        18.0,
    );
    mklab(
        s,
        "title",
        Vec2::new(cx, cy - hh - 30.0),
        &format!("ray-fan: {name}"),
        22.0,
    );
    Ok(())
}

/// `spotdiagram(id, [center], [preset])` — the **spot diagram**: where a bundle
/// of rays actually lands at best focus. A perfect lens makes a point; the
/// singlet smears into a disc (the circle of least confusion from **spherical
/// aberration**), tightest for the doublet/triplet. A green dot marks the ideal
/// (point) focus; an **RMS** read-out gives the blur radius. `draw`/`show` reveal
/// the dots. Parts `{id}.ideal/.rms/.dot{k}/…`.
pub fn c_spotdiagram(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 360.0)
    };
    let name = if a.len() >= 3 {
        a.text(2)?
    } else {
        "singlet".to_string()
    };
    let (dys, _focal, _ap) = analyze_preset(&name, 0.0);
    let (cx, cy) = (center.x, center.y);
    let tag = || vec![id.clone(), format!("{id}.parts")];

    // scale to the SINGLET's aberration (same for every preset), so a corrected
    // doublet/triplet forms a visibly TIGHTER spot than the singlet's blur disc
    let (ref_dys, _, _) = analyze_preset("singlet", 0.0);
    let ref_max = ref_dys
        .iter()
        .map(|(_, d)| d.abs())
        .fold(0.001f32, f32::max);
    let disp = 140.0 / ref_max;
    let rms_px = (dys.iter().map(|(_, d)| d * d).sum::<f32>() / dys.len() as f32).sqrt();

    // reference crosshair
    for (part, from, to) in [
        (
            "crossx",
            Vec2::new(cx - 170.0, cy),
            Vec2::new(cx + 170.0, cy),
        ),
        (
            "crossy",
            Vec2::new(cx, cy - 170.0),
            Vec2::new(cx, cy + 170.0),
        ),
    ] {
        let mut ln = Entity::new(format!("{id}.{part}"), Shape::Line { to }, from, style::DIM);
        ln.stroke.width = 1.0;
        ln.opacity = 0.4;
        ln.tags = tag();
        s.add(ln);
    }

    // rings of dots — each pupil height maps to a radius (rotational symmetry);
    // a per-ring phase offset breaks the dots off radial spokes into a blur
    let azimuths = 10;
    let mut k = 0;
    let mut ring = 0;
    for (h, d) in dys.iter() {
        if *h <= 0.02 {
            continue; // use the positive half; h≈0 is the centre
        }
        let radius = d.abs() * disp;
        let phase = ring as f32 * 0.55;
        ring += 1;
        for j in 0..azimuths {
            let ang = j as f32 * std::f32::consts::TAU / azimuths as f32 + phase;
            let at = Vec2::new(cx + radius * ang.cos(), cy + radius * ang.sin());
            let mut dot = Entity::new(
                format!("{id}.dot{k}"),
                Shape::Circle { r: 3.0 },
                at,
                style::CYAN,
            );
            dot.stroke.fill = true;
            dot.stroke.outline = false;
            dot.glow = 1.3;
            dot.tags = vec![id.clone(), format!("{id}.parts"), format!("{id}.dots")];
            s.add(dot);
            k += 1;
        }
    }

    // ideal (point) focus + RMS read-out
    let mut ideal = Entity::new(
        format!("{id}.ideal"),
        Shape::Circle { r: 5.0 },
        center,
        style::LIME,
    );
    ideal.stroke.fill = true;
    ideal.stroke.outline = false;
    ideal.glow = 2.0;
    ideal.tags = tag();
    s.add(ideal);

    let rms_counter = crate::primitives::Counter {
        value: rms_px,
        decimals: 1,
        prefix: "RMS ".into(),
        suffix: " px".into(),
    };
    let mut rms = Entity::new(
        format!("{id}.rms"),
        Shape::Text {
            content: rms_counter.render(),
            size: 22.0,
        },
        Vec2::new(cx, cy - 190.0),
        style::GOLD,
    );
    rms.counter = Some(rms_counter);
    rms.tags = tag();
    s.add(rms);

    let mut lbl = Entity::new(
        format!("{id}.label"),
        Shape::Text {
            content: format!("spot at focus: {name}"),
            size: 20.0,
        },
        Vec2::new(cx, cy + 195.0),
        style::DIM,
    );
    lbl.tags = tag();
    s.add(lbl);
    Ok(())
}

/// `fieldspot(id, [center], [preset], [field])` — the **off-axis spot diagram**:
/// how a lens images a point OFF the axis. A 2-D pupil is traced in full 3-D at
/// the field angle `field` (degrees, default 5°) and its hits are plotted on the
/// image plane. On-axis (`field 0`) the spot is symmetric; off-axis it flares
/// into a **coma** comet and stretches with **astigmatism** — real aberrations
/// only a 3-D trace shows. A dashed **Airy-disk** circle marks the diffraction
/// limit (1 px ≈ 1 µm at the image); when the geometric blur shrinks below it the
/// lens is diffraction-limited. Parts `{id}.dot{k}` (tagged `{id}.dots`),
/// `{id}.airy`, `{id}.rms`, `{id}.crossx/.crossy/.label`.
pub fn c_fieldspot(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 360.0)
    };
    let name = if a.len() >= 3 {
        a.text(2)?
    } else {
        "doublet".to_string()
    };
    let field = a.opt_num(3)?.unwrap_or(5.0).to_radians();
    let (surfs, ap, _) = resolve_prescription(&name);
    let (cx, cy) = (center.x, center.y);
    let tag = || vec![id.clone(), format!("{id}.parts")];

    // vertices (axis through the origin; local coords)
    let mut vxs = Vec::with_capacity(surfs.len());
    let mut acc = 0.0f32;
    for sf in &surfs {
        vxs.push(acc);
        acc += sf.thick;
    }
    let last_vx = *vxs.last().unwrap();
    let x_in = -260.0f32;
    let lam = 0.546;
    let gridn = 6i32;

    // trace a full 2-D pupil disc at a field angle; return each ray's (last pt, dir)
    let trace_pupil = |theta: f32| -> Vec<(Vec3, Vec3)> {
        let d0 = Vec3::new(theta.cos(), theta.sin(), 0.0);
        let mut tails = Vec::new();
        for iy in -gridn..=gridn {
            for iz in -gridn..=gridn {
                let y = ap * iy as f32 / gridn as f32;
                let z = ap * iz as f32 / gridn as f32;
                if y * y + z * z > ap * ap {
                    continue;
                }
                let mut o = Vec3::new(x_in, y, z);
                let mut d = d0;
                let mut n1 = 1.0f32;
                let mut ok = true;
                for (j, sf) in surfs.iter().enumerate() {
                    let n2 = if sf.glass.is_empty() {
                        1.0
                    } else {
                        dispersion::glass_n(&sf.glass, lam)
                    };
                    let surf_ap = if sf.aperture > 0.0 {
                        sf.aperture
                    } else {
                        ap * 1.3
                    };
                    match trace::trace_conic_3d(o, d, vxs[j], sf.r, sf.conic, n1, n2) {
                        Some((h, nd)) if (h.y * h.y + h.z * h.z).sqrt() <= surf_ap => {
                            o = h;
                            d = nd;
                            n1 = n2;
                        }
                        _ => {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok {
                    tails.push((o, d));
                }
            }
        }
        tails
    };
    let yz_at = |t: &(Vec3, Vec3), x: f32| -> (f32, f32) {
        let (p, d) = t;
        if d.x.abs() < 1e-5 {
            (p.y, p.z)
        } else {
            let sca = (x - p.x) / d.x;
            (p.y + sca * d.y, p.z + sca * d.z)
        }
    };
    let centroid = |pts: &[(f32, f32)]| -> (f32, f32) {
        let n = pts.len().max(1) as f32;
        (
            pts.iter().map(|p| p.0).sum::<f32>() / n,
            pts.iter().map(|p| p.1).sum::<f32>() / n,
        )
    };

    // best (on-axis) focus = minimum radial RMS of the on-axis pupil
    let onax = trace_pupil(0.0);
    let (fx0, fx1) = (last_vx + 30.0, last_vx + 700.0);
    let (mut best_x, mut best) = (fx0, f32::MAX);
    for k in 0..160 {
        let x = fx0 + (fx1 - fx0) * k as f32 / 159.0;
        let pts: Vec<(f32, f32)> = onax.iter().map(|t| yz_at(t, x)).collect();
        let (cy_, cz_) = centroid(&pts);
        let rms = (pts
            .iter()
            .map(|(y, z)| (y - cy_).powi(2) + (z - cz_).powi(2))
            .sum::<f32>()
            / pts.len().max(1) as f32)
            .sqrt();
        if rms < best {
            best = rms;
            best_x = x;
        }
    }
    let focal = best_x - last_vx;
    let fnum = focal / (2.0 * ap);

    // the field spot at that plane
    let field_tails = trace_pupil(field);
    let spots: Vec<(f32, f32)> = field_tails.iter().map(|t| yz_at(t, best_x)).collect();
    let (c0y, c0z) = centroid(&spots);
    let offs: Vec<(f32, f32)> = spots.iter().map(|(y, z)| (y - c0y, z - c0z)).collect();
    let max_r = offs
        .iter()
        .map(|(y, z)| (y * y + z * z).sqrt())
        .fold(0.5f32, f32::max);
    let disp = 130.0 / max_r; // fit this spot to the panel
    let rms_px =
        (offs.iter().map(|(y, z)| y * y + z * z).sum::<f32>() / offs.len().max(1) as f32).sqrt();
    let airy_disp = (1.22 * 0.546 * fnum) * disp; // 1 px ≈ 1 µm at the image

    // crosshair
    for (part, from, to) in [
        (
            "crossx",
            Vec2::new(cx - 170.0, cy),
            Vec2::new(cx + 170.0, cy),
        ),
        (
            "crossy",
            Vec2::new(cx, cy - 170.0),
            Vec2::new(cx, cy + 170.0),
        ),
    ] {
        let mut ln = Entity::new(format!("{id}.{part}"), Shape::Line { to }, from, style::DIM);
        ln.stroke.width = 1.0;
        ln.opacity = 0.4;
        ln.tags = tag();
        s.add(ln);
    }

    // the spot: local (Δy, Δz) → screen (x, y)
    for (k, (dy, dz)) in offs.iter().enumerate() {
        let at = Vec2::new(cx + dy * disp, cy + dz * disp);
        let mut dot = Entity::new(
            format!("{id}.dot{k}"),
            Shape::Circle { r: 2.6 },
            at,
            style::CYAN,
        );
        dot.stroke.fill = true;
        dot.stroke.outline = false;
        dot.glow = 1.3;
        dot.tags = vec![id.clone(), format!("{id}.parts"), format!("{id}.dots")];
        s.add(dot);
    }

    // Airy-disk reference (diffraction limit) — dashed-look faint ring
    let mut airy = Entity::new(
        format!("{id}.airy"),
        Shape::Arc {
            r: airy_disp.max(4.0),
            inner: 0.0,
            start: 0.0,
            sweep: 360.0,
        },
        center,
        style::GOLD,
    );
    airy.stroke.outline = true;
    airy.stroke.fill = false;
    airy.stroke.width = 1.5;
    airy.opacity = 0.7;
    airy.tags = tag();
    s.add(airy);

    let rms_counter = crate::primitives::Counter {
        value: rms_px,
        decimals: 1,
        prefix: "RMS ".into(),
        suffix: " px".into(),
    };
    let mut rms = Entity::new(
        format!("{id}.rms"),
        Shape::Text {
            content: rms_counter.render(),
            size: 22.0,
        },
        Vec2::new(cx, cy - 190.0),
        style::GOLD,
    );
    rms.counter = Some(rms_counter);
    rms.tags = tag();
    s.add(rms);

    let fdeg = field.to_degrees();
    let mut lbl = Entity::new(
        format!("{id}.label"),
        Shape::Text {
            content: format!("spot at {fdeg:.0}° off-axis: {name}"),
            size: 20.0,
        },
        Vec2::new(cx, cy + 195.0),
        style::DIM,
    );
    lbl.tags = tag();
    s.add(lbl);

    let mut ai = Entity::new(
        format!("{id}.airylabel"),
        Shape::Text {
            content: "circle = Airy (diffraction limit)".into(),
            size: 15.0,
        },
        Vec2::new(cx, cy - 165.0),
        style::DIM,
    );
    ai.opacity = 0.75;
    ai.tags = tag();
    s.add(ai);
    Ok(())
}
