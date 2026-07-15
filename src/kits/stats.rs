//! The **stats kit**: turn a dataset (or, later, a random process) into a
//! picture — shape, centre, spread, and the truths that appear at scale.
//!
//! Like every kit, these are *compositions* of core primitives: a histogram is
//! a row of `rect` bars, a number line is a `line` with markers. The dataset is
//! a plain number list (`"v1 v2 v3 …"`), parsed the same way `leastsquares`
//! reads its points. Tier 1 (describe a dataset): `histogram`.

use macroquad::prelude::{vec2, Color, Vec2};

use crate::lang::diag::Error;
use crate::lang::lower::{resolve_color, Args, Registry};
use crate::primitives::{Entity, FontKind, Shape};
use crate::scene::Scene;
use crate::style;

/// Parse a whitespace/comma-separated list of numbers.
fn parse_data(src: &str) -> Vec<f32> {
    src.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse::<f32>().ok())
        .collect()
}

/// Bin `data` into `bins` equal-width buckets over its [min, max] range.
/// Returns `(min, max, counts)`.
fn histogram_bins(data: &[f32], bins: usize) -> (f32, f32, Vec<i32>) {
    let lo = data.iter().copied().fold(f32::INFINITY, f32::min);
    let hi = data.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let range = (hi - lo).max(1e-6);
    let mut counts = vec![0i32; bins];
    for &v in data {
        let k = (((v - lo) / range) * bins as f32).floor() as i32;
        counts[k.clamp(0, bins as i32 - 1) as usize] += 1;
    }
    (lo, hi, counts)
}

/// Read a colour-or-`rainbow` slot at argument `idx`: returns `(rainbow?, base
/// colour)`. `rainbow` means "give each bar its own hue"; otherwise a colour word
/// (default cyan). Shared by the bar builtins.
fn color_or_rainbow(a: &Args, idx: usize) -> Result<(bool, Color), Error> {
    if a.len() > idx {
        let word = a.ident(idx)?;
        if word == "rainbow" {
            Ok((true, style::CYAN))
        } else {
            Ok((false, resolve_color(&word, a.span_of(idx))?))
        }
    } else {
        Ok((false, style::CYAN))
    }
}

/// The colour of bar `k` of `n`: its own hue across the spectrum when `rainbow`,
/// else the base colour.
fn bar_color(rainbow: bool, base: Color, k: usize, n: usize) -> Color {
    if rainbow {
        style::hsl(360.0 * k as f32 / n.max(1) as f32, 1.0, 0.6)
    } else {
        base
    }
}

/// Format a value for an axis label: integers plain, else one decimal.
fn fmt(v: f32) -> String {
    if (v - v.round()).abs() < 1e-4 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

fn add_line(s: &mut Scene, id: String, from: Vec2, to: Vec2, color: Color, width: f32, opacity: f32, z: i32, tags: Vec<String>) {
    let mut e = Entity::new(id, Shape::Line { to }, from, color);
    e.stroke.width = width;
    e.opacity = opacity;
    e.z = z;
    e.tags = tags;
    s.add(e);
}

fn add_label(s: &mut Scene, id: String, text: &str, pos: Vec2, size: f32, color: Color, tag: &str) {
    let mut t = Entity::new(id, Shape::Text { content: text.to_string(), size }, pos, color);
    t.font = FontKind::MonoBold;
    t.tags.push(tag.to_string());
    s.add(t);
}

/// `histogram(id, (cx,cy), "v1 v2 v3 …", [bins], [width], [height], [color])` —
/// bin a dataset into bars: the *shape* of the data. Bars are `{id}.bar{k}`
/// (tagged `{id}` and `{id}.bars`) so they stagger in and recolour as a group; a
/// gold `{id}.meanline` + `{id}.mean` marks the mean, and `{id}.min`/`{id}.max`
/// label the range. Default bin count ≈ √n (clamped 5–20).
fn c_histogram(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let data = parse_data(&a.text(2)?);
    if data.len() < 2 {
        return Err(Error::new(
            "histogram needs at least two numbers: \"v1 v2 v3 ...\"".to_string(),
            a.span_of(2),
        ));
    }
    let n = data.len();
    let bins = match a.opt_num(3)? {
        Some(v) => (v.round() as i32).clamp(2, 60),
        None => ((n as f32).sqrt().round() as i32).clamp(5, 20),
    } as usize;
    let w = a.opt_num(4)?.unwrap_or(460.0);
    let h = a.opt_num(5)?.unwrap_or(240.0);
    // the colour slot accepts a colour word OR `rainbow` (each bar its own hue)
    let (rainbow, base_color) = color_or_rainbow(a, 6)?;

    let (lo, hi, counts) = histogram_bins(&data, bins);
    let range = (hi - lo).max(1e-6);
    let maxc = *counts.iter().max().unwrap() as f32;

    let x0 = c.x - w / 2.0;
    let baseline = c.y + h / 2.0;
    let barw = w / bins as f32;
    let tags = || vec![id.clone(), format!("{id}.bars")];

    // baseline axis
    add_line(s, format!("{id}.axis"), vec2(x0, baseline), vec2(x0 + w, baseline), style::DIM, 2.0, 0.8, 0, vec![id.clone()]);

    // one bar per bin — always created (empty bins are height 0) so the count is
    // exactly `bins` and a `for k in 0..bins { draw(id.bar{k}) }` loop is safe.
    for (k, &count) in counts.iter().enumerate() {
        let color = bar_color(rainbow, base_color, k, bins);
        let bh = (count as f32 / maxc) * h;
        let bx = x0 + (k as f32 + 0.5) * barw;
        let by = baseline - bh / 2.0;
        let mut e = Entity::new(
            format!("{id}.bar{k}"),
            Shape::Rect { w: barw * 0.9, h: bh },
            vec2(bx, by),
            color,
        );
        e.stroke.fill = true;
        e.stroke.outline = true;
        e.stroke.outline_color = Some(color);
        e.opacity = 0.85;
        e.tags = tags();
        s.add(e);
    }

    // range labels under the ends
    add_label(s, format!("{id}.min"), &fmt(lo), vec2(x0, baseline + 22.0), 18.0, style::DIM, &id);
    add_label(s, format!("{id}.max"), &fmt(hi), vec2(x0 + w, baseline + 22.0), 18.0, style::DIM, &id);

    // mean marker (vertical line + label)
    let mean = data.iter().sum::<f32>() / n as f32;
    let mx = x0 + ((mean - lo) / range) * w;
    add_line(s, format!("{id}.meanline"), vec2(mx, baseline), vec2(mx, baseline - h - 12.0), style::GOLD, 2.0, 0.9, 1, vec![id.clone()]);
    add_label(s, format!("{id}.mean"), "mean", vec2(mx, baseline - h - 28.0), 18.0, style::GOLD, &id);

    Ok(())
}

/// Population covariance of `(x,y)` points: `Σ(x-x̄)(y-ȳ)/n`. Positive when the
/// two tend to rise together, negative when one rises as the other falls.
fn covariance_of(pts: &[Vec2]) -> f32 {
    let n = pts.len() as f32;
    let mx = pts.iter().map(|p| p.x).sum::<f32>() / n;
    let my = pts.iter().map(|p| p.y).sum::<f32>() / n;
    pts.iter().map(|p| (p.x - mx) * (p.y - my)).sum::<f32>() / n
}

/// `covariance(id, (cx,cy), unit, "x1 y1  x2 y2 …", [color])` — covariance as
/// signed area: a cross at the means splits the plane into four quadrants; each
/// point draws a rectangle to the mean-corner, cyan where `(x-x̄)(y-ȳ)>0` (the
/// agreeing quadrants) and magenta where it's negative. Their signed-area balance
/// IS the covariance. `{id}.points`, `{id}.rects`, `{id}.cross`, `{id}.cov`.
fn c_covariance(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let u = a.num(2)?;
    let pts = parse_points(&a.text(3)?);
    if pts.len() < 3 {
        return Err(Error::new(
            "covariance needs at least three points: \"x1 y1 x2 y2 ...\"".to_string(),
            a.span_of(3),
        ));
    }
    let dot_color = if a.len() > 4 {
        resolve_color(&a.ident(4)?, a.span_of(4))?
    } else {
        style::GOLD
    };
    let n = pts.len() as f32;
    let mx = pts.iter().map(|p| p.x).sum::<f32>() / n;
    let my = pts.iter().map(|p| p.y).sum::<f32>() / n;
    let sc = |p: Vec2| vec2(c.x + (p.x - mx) * u, c.y - (p.y - my) * u);
    let ext = pts
        .iter()
        .map(|p| (p.x - mx).abs().max((p.y - my).abs()))
        .fold(0.0f32, f32::max)
        + 1.0;

    // signed-area rectangles (behind), one per point
    for (i, p) in pts.iter().enumerate() {
        let col = if (p.x - mx) * (p.y - my) >= 0.0 { style::CYAN } else { style::MAGENTA };
        let a0 = sc(vec2(mx, my));
        let a1 = sc(*p);
        let mut e = Entity::new(
            format!("{id}.rect{i}"),
            Shape::Rect { w: (a1.x - a0.x).abs(), h: (a1.y - a0.y).abs() },
            vec2((a0.x + a1.x) / 2.0, (a0.y + a1.y) / 2.0),
            col,
        );
        e.stroke.fill = true;
        e.stroke.outline = false;
        e.opacity = 0.13;
        e.z = -1;
        e.tags = vec![id.clone(), format!("{id}.rects")];
        s.add(e);
    }
    // the mean cross
    add_line(s, format!("{id}.crossv"), sc(vec2(mx, my - ext)), sc(vec2(mx, my + ext)), style::DIM, 2.0, 0.8, 0, vec![id.clone(), format!("{id}.cross")]);
    add_line(s, format!("{id}.crossh"), sc(vec2(mx - ext, my)), sc(vec2(mx + ext, my)), style::DIM, 2.0, 0.8, 0, vec![id.clone(), format!("{id}.cross")]);
    // points on top
    for (i, p) in pts.iter().enumerate() {
        let mut e = Entity::new(format!("{id}.p{i}"), Shape::Circle { r: 7.0 }, sc(*p), dot_color);
        e.stroke.fill = true;
        e.z = 2;
        e.tags = vec![id.clone(), format!("{id}.points")];
        s.add(e);
    }
    let cov = covariance_of(&pts);
    let sign = if cov >= 0.0 { "positive relationship" } else { "negative relationship" };
    add_label(s, format!("{id}.cov"), &format!("cov = {cov:+.2}   ({sign})"), vec2(c.x, c.y + ext * u + 30.0), 22.0, if cov >= 0.0 { style::CYAN } else { style::MAGENTA }, &id);
    Ok(())
}

/// `bayes(id, (cx,cy), heads, tails, [width], [height])` — Bayesian updating for a
/// coin's bias: a mild **prior** belief (Beta(2,2), gold), the **likelihood** from
/// `heads`/`tails` (magenta), and the **posterior** (cyan) that combines them —
/// pulled toward the data and sharpening as evidence grows. `{id}.prior`,
/// `{id}.likelihood`, `{id}.posterior`, `{id}.mean`.
fn c_bayes(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let heads = a.num(2)?.max(0.0) as i32 as f32;
    let tails = a.num(3)?.max(0.0) as i32 as f32;
    let w = a.opt_num(4)?.unwrap_or(620.0);
    let h = a.opt_num(5)?.unwrap_or(260.0);
    let x0 = c.x - w / 2.0;
    let base = c.y + h / 2.0;
    let px = |p: f32| x0 + p * w;

    // unnormalised Beta-shaped density p^(a-1)(1-p)^(b-1), then peak-normalised curve
    let curve = |aa: f32, bb: f32| -> Vec<Vec2> {
        let cn = 160;
        let dens: Vec<f32> = (0..=cn)
            .map(|i| {
                let p = i as f32 / cn as f32;
                p.powf(aa - 1.0) * (1.0 - p).powf(bb - 1.0)
            })
            .collect();
        let peak = dens.iter().cloned().fold(1e-9f32, f32::max);
        dens.iter()
            .enumerate()
            .map(|(i, &d)| vec2(px(i as f32 / cn as f32), base - (d / peak) * h))
            .collect()
    };
    let draw_curve = |s: &mut Scene, nm: &str, pts: Vec<Vec2>, col, z: i32| {
        let mut e = Entity::new(format!("{id}.{nm}"), Shape::Polyline { pts }, vec2(0.0, 0.0), col);
        e.stroke.width = 3.0;
        e.z = z;
        e.tags.push(id.clone());
        s.add(e);
    };

    add_line(s, format!("{id}.axis"), vec2(px(0.0), base), vec2(px(1.0), base), style::DIM, 2.0, 0.8, 0, vec![id.clone()]);
    for (p, txt) in [(0.0, "0"), (0.5, "0.5"), (1.0, "1")] {
        add_label(s, format!("{id}.t{}", (p * 10.0) as i32), txt, vec2(px(p), base + 22.0), 16.0, style::DIM, &id);
    }
    // prior Beta(2,2); likelihood ∝ p^H(1-p)^T = Beta(H+1,T+1); posterior Beta(H+2,T+2)
    draw_curve(s, "prior", curve(2.0, 2.0), style::GOLD, 1);
    draw_curve(s, "likelihood", curve(heads + 1.0, tails + 1.0), style::MAGENTA, 2);
    draw_curve(s, "posterior", curve(heads + 2.0, tails + 2.0), style::CYAN, 3);
    // posterior mean = (H+2)/(H+T+4)
    let pm = (heads + 2.0) / (heads + tails + 4.0);
    add_line(s, format!("{id}.mean"), vec2(px(pm), base), vec2(px(pm), base - h), style::CYAN, 2.0, 0.7, 0, vec![id.clone()]);
    add_label(s, format!("{id}.priorlbl"), "prior", vec2(px(0.5), base - h * 0.28), 18.0, style::GOLD, &id);
    add_label(s, format!("{id}.postlbl"), &format!("posterior  (p ~ {pm:.2})"), vec2(px(pm), base - h - 16.0), 18.0, style::CYAN, &id);
    add_label(s, format!("{id}.datalbl"), &format!("{} heads, {} tails", heads as i32, tails as i32), vec2(c.x, base + 50.0), 20.0, style::MAGENTA, &id);
    Ok(())
}

/// Upper-tail area of the standard normal beyond `|z|` (numerical) — the
/// one-tailed p-value. `normal_tail(0) = 0.5`, `normal_tail(1.96) ≈ 0.025`.
fn normal_tail(z: f32) -> f32 {
    let z = z.abs();
    if z >= 6.0 {
        return 0.0;
    }
    let steps = 600;
    let dx = (6.0 - z) / steps as f32;
    let phi = |t: f32| (-0.5 * t * t).exp() / (2.0 * std::f32::consts::PI).sqrt();
    let mut sum = 0.5 * (phi(z) + phi(6.0));
    for i in 1..steps {
        sum += phi(z + i as f32 * dx);
    }
    sum * dx
}

/// `hypothesis(id, (cx,cy), z, [alpha], [unit])` — a two-tailed significance test:
/// under the null hypothesis the test statistic is standard-normal; the observed
/// `z` cuts off tails whose combined area is the **p-value**. Shades those tails,
/// marks ±z, and gives the verdict against `alpha` (default 0.05). `{id}.curve`,
/// `{id}.tails`, `{id}.zline`, `{id}.p`, `{id}.verdict`.
fn c_hypothesis(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let z = a.num(2)?.abs();
    let alpha = a.opt_num(3)?.unwrap_or(0.05);
    let unit = a.opt_num(4)?.unwrap_or(80.0);
    let h = 220.0f32;
    let zmax = 3.8;
    let baseline = c.y + h / 2.0;
    let bx = |t: f32| c.x + t * unit;
    let by = |t: f32| baseline - (-0.5 * t * t).exp() * h;

    add_line(s, format!("{id}.axis"), vec2(bx(-zmax), baseline), vec2(bx(zmax), baseline), style::DIM, 2.0, 0.8, 0, vec![id.clone()]);

    // shade the two tails beyond ±z (the p-value area)
    for (nm, sgn) in [("lo", -1.0f32), ("hi", 1.0f32)] {
        let (a0, a1) = if sgn > 0.0 { (z.min(zmax), zmax) } else { (-zmax, -z.max(-zmax)) };
        if a1 > a0 {
            let steps = 40;
            let mut pts = vec![vec2(bx(a0), baseline)];
            for i in 0..=steps {
                let t = a0 + (a1 - a0) * i as f32 / steps as f32;
                pts.push(vec2(bx(t), by(t)));
            }
            pts.push(vec2(bx(a1), baseline));
            let mut e = Entity::new(format!("{id}.tail{nm}"), Shape::Polygon { pts }, vec2(0.0, 0.0), style::MAGENTA);
            e.stroke.fill = true;
            e.stroke.outline = false;
            e.opacity = 0.5;
            e.z = 1;
            e.tags = vec![id.clone(), format!("{id}.tails")];
            s.add(e);
        }
    }

    // the null distribution (standard normal) on top
    let cn = 130;
    let mut curve = Vec::with_capacity(cn + 1);
    for i in 0..=cn {
        let t = -zmax + 2.0 * zmax * i as f32 / cn as f32;
        curve.push(vec2(bx(t), by(t)));
    }
    let mut ce = Entity::new(format!("{id}.curve"), Shape::Polyline { pts: curve }, vec2(0.0, 0.0), style::CYAN);
    ce.stroke.width = 3.0;
    ce.z = 2;
    ce.tags.push(id.clone());
    s.add(ce);

    // observed ±z lines
    add_line(s, format!("{id}.zlinehi"), vec2(bx(z), baseline), vec2(bx(z), by(z)), style::GOLD, 2.5, 1.0, 3, vec![id.clone(), format!("{id}.zline")]);
    add_line(s, format!("{id}.zlinelo"), vec2(bx(-z), baseline), vec2(bx(-z), by(-z)), style::GOLD, 2.5, 1.0, 3, vec![id.clone(), format!("{id}.zline")]);
    add_label(s, format!("{id}.zlbl"), &format!("z = {z:.2}"), vec2(bx(z), by(z) - 16.0), 18.0, style::GOLD, &id);

    let p = (2.0 * normal_tail(z)).min(1.0);
    add_label(s, format!("{id}.p"), &format!("p = {p:.3}"), vec2(c.x, baseline + 34.0), 22.0, style::MAGENTA, &id);
    let verdict = if p < alpha {
        format!("p < {alpha} — reject the null hypothesis")
    } else {
        format!("p >= {alpha} — fail to reject")
    };
    add_label(s, format!("{id}.verdict"), &verdict, vec2(c.x, baseline + 62.0), 20.0, style::FG, &id);
    Ok(())
}

/// `bellcurve(id, (cx,cy), mu, sigma, [unit], [color])` (alias `gaussian`) — the
/// normal / Gaussian bell curve for mean `mu`, std `sigma`, with the **68–95–99.7
/// rule** shaded: nested ±1σ / ±2σ / ±3σ bands under the curve, the mean line,
/// value ticks (μ, μ±σ, …), and the 68% / 95% / 99.7% labels. `unit` = pixels per
/// σ (default 80). The bell is standardised (its shape is universal); μ, σ set the
/// axis values. (Named `bellcurve`, not `normal`, to avoid the calculus `normal`
/// = perpendicular-line builtin.)
fn c_bellcurve(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let mu = a.num(2)?;
    let sigma = a.num(3)?;
    if sigma <= 0.0 {
        return Err(Error::new("normal: sigma must be positive".to_string(), a.span_of(3)));
    }
    let unit = a.opt_num(4)?.unwrap_or(80.0);
    let color = if a.len() > 5 {
        resolve_color(&a.ident(5)?, a.span_of(5))?
    } else {
        style::CYAN
    };
    let h = 220.0f32;
    let zmax = 3.6;
    let baseline = c.y + h / 2.0;
    let bx = |z: f32| c.x + z * unit;
    let by = |z: f32| baseline - (-0.5 * z * z).exp() * h;

    // baseline axis
    add_line(s, format!("{id}.axis"), vec2(bx(-zmax), baseline), vec2(bx(zmax), baseline), style::DIM, 2.0, 0.8, 0, vec![id.clone()]);

    // nested ±kσ bands (widest first, decreasing opacity → darker toward centre)
    for (k, kz, pct) in [(3usize, 3.0f32, "99.7%"), (2, 2.0, "95%"), (1, 1.0, "68%")] {
        let steps = 80;
        let mut pts = vec![vec2(bx(-kz), baseline)];
        for i in 0..=steps {
            let z = -kz + 2.0 * kz * i as f32 / steps as f32;
            pts.push(vec2(bx(z), by(z)));
        }
        pts.push(vec2(bx(kz), baseline));
        let mut e = Entity::new(format!("{id}.band{k}"), Shape::Polygon { pts }, vec2(0.0, 0.0), color);
        e.stroke.fill = true;
        e.stroke.outline = false;
        e.opacity = 0.16;
        e.z = -1;
        e.tags = vec![id.clone(), format!("{id}.bands")];
        s.add(e);
        // percentage label: for the core at centre, for the rings offset into the +side ring
        let lz = if k == 1 { 0.0 } else { kz - 0.5 };
        let ly = if k == 1 { baseline - h * 0.45 } else { baseline - 34.0 };
        add_label(s, format!("{id}.p{k}"), pct, vec2(bx(lz), ly), 20.0, color, &id);
    }

    // the bell curve on top
    let cn = 140;
    let mut curve = Vec::with_capacity(cn + 1);
    for i in 0..=cn {
        let z = -zmax + 2.0 * zmax * i as f32 / cn as f32;
        curve.push(vec2(bx(z), by(z)));
    }
    let mut ce = Entity::new(format!("{id}.curve"), Shape::Polyline { pts: curve }, vec2(0.0, 0.0), color);
    ce.stroke.width = 3.0;
    ce.z = 1;
    ce.tags.push(id.clone());
    s.add(ce);

    // mean line + value ticks
    add_line(s, format!("{id}.mean"), vec2(bx(0.0), baseline), vec2(bx(0.0), by(0.0)), style::GOLD, 2.0, 0.9, 1, vec![id.clone()]);
    for k in -3..=3 {
        let v = mu + k as f32 * sigma;
        let col = if k == 0 { style::GOLD } else { style::DIM };
        add_label(s, format!("{id}.t{k}"), &fmt(v), vec2(bx(k as f32), baseline + 22.0), 17.0, col, &id);
    }
    Ok(())
}

/// Descriptive statistics of a dataset.
struct Stats {
    mean: f32,
    median: f32,
    mode: Option<f32>, // most-frequent value; None if every value is unique
    var: f32,          // population variance (÷n)
    std: f32,
    lo: f32,
    hi: f32,
}

/// Compute mean / median / mode / variance / std / range for `data` (len ≥ 1).
fn describe(data: &[f32]) -> Stats {
    let n = data.len();
    let mean = data.iter().sum::<f32>() / n as f32;
    let var = data.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / n as f32;
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    };
    // mode = value with the longest run of equal values (sorted); None if all runs are 1
    let (mut best_val, mut best_run) = (sorted[0], 1usize);
    let (mut cur_val, mut cur_run) = (sorted[0], 1usize);
    for &v in &sorted[1..] {
        if (v - cur_val).abs() < 1e-6 {
            cur_run += 1;
        } else {
            cur_val = v;
            cur_run = 1;
        }
        if cur_run > best_run {
            best_run = cur_run;
            best_val = cur_val;
        }
    }
    Stats {
        mean,
        median,
        mode: if best_run >= 2 { Some(best_val) } else { None },
        var,
        std: var.sqrt(),
        lo: sorted[0],
        hi: sorted[n - 1],
    }
}

/// `summary(id, (cx,cy), "v1 v2 v3 …", [width], [color])` — the descriptive-stats
/// workhorse: the data as dots on a number line, with **mean** (gold), **median**
/// (magenta) and **mode** (lime) markers, a translucent **±1σ spread band**, and a
/// readout of **n / range / variance / std**. Covers most of central-tendency +
/// dispersion in one builtin. Markers `{id}.meanmark`/`.medianmark`/`.modemark`
/// (+ `.*lbl`), dots `{id}.dot{k}` (tagged `{id}.dots`), `{id}.readout`.
fn c_summary(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let data = parse_data(&a.text(2)?);
    if data.len() < 2 {
        return Err(Error::new(
            "summary needs at least two numbers: \"v1 v2 v3 ...\"".to_string(),
            a.span_of(2),
        ));
    }
    let w = a.opt_num(3)?.unwrap_or(640.0);
    let accent = if a.len() > 4 {
        resolve_color(&a.ident(4)?, a.span_of(4))?
    } else {
        style::CYAN
    };
    let st = describe(&data);
    let n = data.len();
    let y0 = c.y + 30.0;
    let pad = (st.hi - st.lo) * 0.08 + 0.5;
    let (lop, hip) = (st.lo - pad, st.hi + pad);
    let span = (hip - lop).max(1e-6);
    let sx = |v: f32| c.x - w / 2.0 + (v - lop) / span * w;

    // ±1σ spread band (behind everything)
    let (bx0, bx1) = (sx(st.mean - st.std), sx(st.mean + st.std));
    let mut band = Entity::new(
        format!("{id}.band"),
        Shape::Rect { w: (bx1 - bx0).abs(), h: 84.0 },
        vec2((bx0 + bx1) / 2.0, y0 - 34.0),
        accent,
    );
    band.stroke.fill = true;
    band.stroke.outline = false;
    band.opacity = 0.12;
    band.z = -1;
    band.tags.push(id.clone());
    s.add(band);

    add_line(s, format!("{id}.line"), vec2(sx(lop), y0), vec2(sx(hip), y0), style::DIM, 2.0, 0.8, 0, vec![id.clone()]);

    // the data cloud (translucent dots — overlaps darken where values cluster)
    for (k, &v) in data.iter().enumerate() {
        let mut e = Entity::new(format!("{id}.dot{k}"), Shape::Circle { r: 5.0 }, vec2(sx(v), y0), style::FG);
        e.stroke.fill = true;
        e.opacity = 0.45;
        e.z = 1;
        e.tags = vec![id.clone(), format!("{id}.dots")];
        s.add(e);
    }

    // central-tendency markers, at different heights so labels never collide
    let marker = |s: &mut Scene, nm: &str, v: f32, hgt: f32, col, label: String| {
        add_line(s, format!("{id}.{nm}mark"), vec2(sx(v), y0), vec2(sx(v), y0 - hgt), col, 2.5, 0.95, 2, vec![id.clone()]);
        let mut t = Entity::new(format!("{id}.{nm}lbl"), Shape::Text { content: label, size: 18.0 }, vec2(sx(v), y0 - hgt - 14.0), col);
        t.font = FontKind::MonoBold;
        t.tags.push(id.clone());
        s.add(t);
    };
    marker(s, "mean", st.mean, 92.0, style::GOLD, format!("mean {}", fmt(st.mean)));
    marker(s, "median", st.median, 66.0, style::MAGENTA, format!("median {}", fmt(st.median)));
    if let Some(m) = st.mode {
        marker(s, "mode", m, 40.0, style::LIME, format!("mode {}", fmt(m)));
    }

    add_label(s, format!("{id}.min"), &fmt(st.lo), vec2(sx(st.lo), y0 + 22.0), 16.0, style::DIM, &id);
    add_label(s, format!("{id}.max"), &fmt(st.hi), vec2(sx(st.hi), y0 + 22.0), 16.0, style::DIM, &id);

    let readout = format!("n = {}     range {}     variance {:.1}     std {:.1}", n, fmt(st.hi - st.lo), st.var, st.std);
    add_label(s, format!("{id}.readout"), &readout, vec2(c.x, y0 + 60.0), 20.0, accent, &id);

    Ok(())
}

/// Linear regression of `(x,y)` points: returns `(slope m, intercept k, Pearson
/// correlation r)`, or `None` if x (or y) has no spread. `r ∈ [-1, 1]` measures
/// how tightly the cloud hugs a straight line.
fn regression(pts: &[Vec2]) -> Option<(f32, f32, f32)> {
    let n = pts.len() as f32;
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0f32, 0.0, 0.0, 0.0, 0.0);
    for p in pts {
        sx += p.x;
        sy += p.y;
        sxx += p.x * p.x;
        syy += p.y * p.y;
        sxy += p.x * p.y;
    }
    let dx = n * sxx - sx * sx;
    let dy = n * syy - sy * sy;
    if dx.abs() < 1e-9 || dy.abs() < 1e-9 {
        return None;
    }
    let m = (n * sxy - sx * sy) / dx;
    let k = (sy - m * sx) / n;
    let r = (n * sxy - sx * sy) / (dx * dy).sqrt();
    Some((m, k, r))
}

/// Parse a flat "x1 y1 x2 y2 …" string into points; needs an even count ≥ 2 pairs.
fn parse_points(src: &str) -> Vec<Vec2> {
    let nums = parse_data(src);
    nums.chunks_exact(2).map(|p| vec2(p[0], p[1])).collect()
}

/// `correlation(id, (cx,cy), unit, "x1 y1  x2 y2 …", [color])` — how strongly two
/// variables move together: the scatter of points, the best-fit line, and the
/// **Pearson correlation r** (with a strong/moderate/weak · positive/negative
/// reading). `unit` = pixels per data unit. Points `{id}.p{k}` (`{id}.points`),
/// `{id}.line`, `{id}.r`.
fn c_correlation(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let u = a.num(2)?;
    let pts = parse_points(&a.text(3)?);
    if pts.len() < 3 {
        return Err(Error::new(
            "correlation needs at least three points: \"x1 y1 x2 y2 ...\"".to_string(),
            a.span_of(3),
        ));
    }
    let color = if a.len() > 4 {
        resolve_color(&a.ident(4)?, a.span_of(4))?
    } else {
        style::CYAN
    };
    // centre the cloud on (cx,cy): map data means to the centre
    let mx = pts.iter().map(|p| p.x).sum::<f32>() / pts.len() as f32;
    let my = pts.iter().map(|p| p.y).sum::<f32>() / pts.len() as f32;
    let sc = |p: Vec2| vec2(c.x + (p.x - mx) * u, c.y - (p.y - my) * u);

    let (m, k, r) = regression(&pts).ok_or_else(|| {
        Error::new("correlation: x or y has no spread".to_string(), a.span_of(3))
    })?;
    let x0 = pts.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let x1 = pts.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let pad = (x1 - x0) * 0.1 + 0.5;
    let (lx0, lx1) = (x0 - pad, x1 + pad);
    add_line(
        s,
        format!("{id}.line"),
        sc(vec2(lx0, m * lx0 + k)),
        sc(vec2(lx1, m * lx1 + k)),
        style::GOLD,
        3.0,
        1.0,
        0,
        vec![id.clone()],
    );
    for (i, p) in pts.iter().enumerate() {
        let mut e = Entity::new(format!("{id}.p{i}"), Shape::Circle { r: 7.0 }, sc(*p), color);
        e.stroke.fill = true;
        e.z = 2;
        e.tags = vec![id.clone(), format!("{id}.points")];
        s.add(e);
    }
    let strength = if r.abs() > 0.7 {
        "strong"
    } else if r.abs() > 0.4 {
        "moderate"
    } else {
        "weak"
    };
    let dir = if r >= 0.0 { "positive" } else { "negative" };
    add_label(s, format!("{id}.r"), &format!("r = {r:+.2}   ({strength} {dir})"), vec2(c.x, c.y + 200.0), 22.0, color, &id);
    Ok(())
}

/// The moment coefficient of skewness `g1 = m3 / m2^1.5` (population). Positive =
/// right-skewed (tail to the right, mean > median); negative = left-skewed; ≈ 0 =
/// symmetric.
fn skewness(data: &[f32]) -> f32 {
    let n = data.len() as f32;
    let mean = data.iter().sum::<f32>() / n;
    let m2 = data.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n;
    let m3 = data.iter().map(|x| (x - mean).powi(3)).sum::<f32>() / n;
    let s = m2.sqrt();
    if s < 1e-9 {
        0.0
    } else {
        m3 / (s * s * s)
    }
}

/// `skew(id, (cx,cy), "v1 v2 v3 …", [bins], [width], [height], [color])` — the
/// **shape** of a dataset: a histogram with the **mean** (gold) and **median**
/// (magenta) marked and a labelled skewness. When the mean sits to the right of
/// the median the tail (and skew) points right; to the left, left. Bars
/// `{id}.bar{k}` (tagged `{id}.bars`), `{id}.meanline`/`.medianline`, `{id}.skewlbl`.
fn c_skew(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let data = parse_data(&a.text(2)?);
    if data.len() < 3 {
        return Err(Error::new(
            "skew needs at least three numbers".to_string(),
            a.span_of(2),
        ));
    }
    let n = data.len();
    let bins = match a.opt_num(3)? {
        Some(v) => (v.round() as i32).clamp(2, 60),
        None => ((n as f32).sqrt().round() as i32).clamp(5, 20),
    } as usize;
    let w = a.opt_num(4)?.unwrap_or(520.0);
    let h = a.opt_num(5)?.unwrap_or(240.0);
    let (rainbow, base_color) = color_or_rainbow(a, 6)?;
    let (lo, hi, counts) = histogram_bins(&data, bins);
    let range = (hi - lo).max(1e-6);
    let maxc = *counts.iter().max().unwrap() as f32;
    let x0 = c.x - w / 2.0;
    let baseline = c.y + h / 2.0;
    let barw = w / bins as f32;
    let sx = |v: f32| x0 + ((v - lo) / range) * w;

    add_line(s, format!("{id}.axis"), vec2(x0, baseline), vec2(x0 + w, baseline), style::DIM, 2.0, 0.8, 0, vec![id.clone()]);
    for (k, &count) in counts.iter().enumerate() {
        let color = bar_color(rainbow, base_color, k, bins);
        let bh = (count as f32 / maxc) * h;
        let bx = x0 + (k as f32 + 0.5) * barw;
        let mut e = Entity::new(format!("{id}.bar{k}"), Shape::Rect { w: barw * 0.9, h: bh }, vec2(bx, baseline - bh / 2.0), color);
        e.stroke.fill = true;
        e.stroke.outline = true;
        e.stroke.outline_color = Some(color);
        e.opacity = 0.8;
        e.tags = vec![id.clone(), format!("{id}.bars")];
        s.add(e);
    }

    let mean = data.iter().sum::<f32>() / n as f32;
    let mut sorted = data.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = median_of(&sorted);
    add_line(s, format!("{id}.medianline"), vec2(sx(median), baseline), vec2(sx(median), baseline - h - 8.0), style::MAGENTA, 2.0, 0.9, 1, vec![id.clone()]);
    add_line(s, format!("{id}.meanline"), vec2(sx(mean), baseline), vec2(sx(mean), baseline - h - 8.0), style::GOLD, 2.0, 0.9, 1, vec![id.clone()]);
    add_label(s, format!("{id}.medianlbl"), "median", vec2(sx(median), baseline - h - 24.0), 16.0, style::MAGENTA, &id);
    add_label(s, format!("{id}.meanlbl"), "mean", vec2(sx(mean), baseline - h - 44.0), 16.0, style::GOLD, &id);

    let g1 = skewness(&data);
    let dir = if g1.abs() < 0.15 {
        "≈ symmetric"
    } else if g1 > 0.0 {
        "right-skewed"
    } else {
        "left-skewed"
    };
    add_label(s, format!("{id}.skewlbl"), &format!("skew = {g1:+.2}   ({dir})"), vec2(c.x, baseline + 32.0), 22.0, base_color, &id);
    Ok(())
}

/// Median of an already-sorted slice.
fn median_of(s: &[f32]) -> f32 {
    let n = s.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        s[n / 2]
    } else {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    }
}

/// The five-number summary `(min, Q1, median, Q3, max)` — quartiles by the Tukey
/// "median of halves" method (the middle value is excluded from the halves when
/// the count is odd).
fn five_number(data: &[f32]) -> (f32, f32, f32, f32, f32) {
    let mut s = data.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    let n = s.len();
    let med = median_of(&s);
    let (lower, upper) = if n % 2 == 0 {
        (&s[0..n / 2], &s[n / 2..])
    } else {
        (&s[0..n / 2], &s[n / 2 + 1..])
    };
    (s[0], median_of(lower), med, median_of(upper), s[n - 1])
}

/// `boxplot(id, (cx,cy), "v1 v2 v3 …", [width], [color])` — the five-number
/// summary as a box-and-whisker: the **box** spans Q1→Q3 (its width IS the
/// **interquartile range**), a line marks the median, whiskers reach the extreme
/// non-outliers (within 1.5·IQR of the box), and points beyond are **outliers**.
/// Pieces: `{id}.box`, `{id}.med`, `{id}.whiskerlo`/`.whiskerhi` (+ caps),
/// `{id}.out{k}` (tagged `{id}.outliers`), value labels, and `{id}.iqr`.
fn c_boxplot(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let data = parse_data(&a.text(2)?);
    if data.len() < 4 {
        return Err(Error::new(
            "boxplot needs at least four numbers (for quartiles)".to_string(),
            a.span_of(2),
        ));
    }
    let w = a.opt_num(3)?.unwrap_or(640.0);
    let accent = if a.len() > 4 {
        resolve_color(&a.ident(4)?, a.span_of(4))?
    } else {
        style::CYAN
    };
    let (mn, q1, med, q3, mx) = five_number(&data);
    let iqr = q3 - q1;
    let (flo, fhi) = (q1 - 1.5 * iqr, q3 + 1.5 * iqr);
    let wlo = data.iter().copied().filter(|&v| v >= flo).fold(f32::INFINITY, f32::min);
    let whi = data.iter().copied().filter(|&v| v <= fhi).fold(f32::NEG_INFINITY, f32::max);
    let (lo, hi) = (mn.min(wlo), mx.max(whi));
    let pad = (hi - lo) * 0.08 + 0.5;
    let (lop, hip) = (lo - pad, hi + pad);
    let span = (hip - lop).max(1e-6);
    let sx = |v: f32| c.x - w / 2.0 + (v - lop) / span * w;
    let y0 = c.y;
    let bh = 44.0;

    // whiskers first (behind the box)
    add_line(s, format!("{id}.whiskerlo"), vec2(sx(wlo), y0), vec2(sx(q1), y0), style::DIM, 2.0, 0.9, 0, vec![id.clone()]);
    add_line(s, format!("{id}.whiskerhi"), vec2(sx(q3), y0), vec2(sx(whi), y0), style::DIM, 2.0, 0.9, 0, vec![id.clone()]);
    add_line(s, format!("{id}.caplo"), vec2(sx(wlo), y0 - 20.0), vec2(sx(wlo), y0 + 20.0), style::DIM, 2.0, 0.9, 0, vec![id.clone()]);
    add_line(s, format!("{id}.caphi"), vec2(sx(whi), y0 - 20.0), vec2(sx(whi), y0 + 20.0), style::DIM, 2.0, 0.9, 0, vec![id.clone()]);

    // the IQR box
    let mut boxe = Entity::new(
        format!("{id}.box"),
        Shape::Rect { w: (sx(q3) - sx(q1)).max(1.0), h: 2.0 * bh },
        vec2((sx(q1) + sx(q3)) / 2.0, y0),
        accent,
    );
    boxe.stroke.fill = true;
    boxe.stroke.outline = true;
    boxe.stroke.outline_color = Some(accent);
    boxe.opacity = 0.2;
    boxe.z = 1;
    boxe.tags.push(id.clone());
    s.add(boxe);

    // median line inside the box
    add_line(s, format!("{id}.med"), vec2(sx(med), y0 - bh), vec2(sx(med), y0 + bh), style::GOLD, 3.0, 1.0, 2, vec![id.clone()]);

    // outlier dots
    for (k, &v) in data.iter().enumerate() {
        if v < flo || v > fhi {
            let mut e = Entity::new(format!("{id}.out{k}"), Shape::Circle { r: 6.0 }, vec2(sx(v), y0), style::MAGENTA);
            e.stroke.fill = true;
            e.z = 3;
            e.tags = vec![id.clone(), format!("{id}.outliers")];
            s.add(e);
        }
    }

    // labels: IQR above; the five numbers below (staggered into two rows)
    add_label(s, format!("{id}.iqr"), &format!("IQR = {}", fmt(iqr)), vec2((sx(q1) + sx(q3)) / 2.0, y0 - bh - 22.0), 20.0, accent, &id);
    let row = |s: &mut Scene, nm: &str, v: f32, dy: f32, col| {
        add_label(s, format!("{id}.l{nm}"), &fmt(v), vec2(sx(v), y0 + bh + dy), 17.0, col, &id);
    };
    row(s, "min", wlo, 24.0, style::DIM);
    row(s, "med", med, 24.0, style::GOLD);
    row(s, "max", whi, 24.0, style::DIM);
    row(s, "q1", q1, 46.0, accent);
    row(s, "q3", q3, 46.0, accent);
    Ok(())
}

/// One draw in `[0, 1)` from a seeded LCG — a *deterministic* PRNG so a sampling
/// scene renders the same frames every time (no system entropy).
fn lcg_next(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 40) as f32) / ((1u64 << 24) as f32)
}

/// Run `trials` experiments, each averaging `samplesize` dice (1–6) drawn from a
/// seeded LCG. Deterministic in `seed` — the heart of `clt`.
fn clt_means(n: usize, trials: usize, seed: u64) -> Vec<f32> {
    let mut state = seed.wrapping_add(1);
    let mut means = Vec::with_capacity(trials);
    for _ in 0..trials {
        let mut sum = 0.0f32;
        for _ in 0..n {
            sum += 1.0 + (lcg_next(&mut state) * 6.0).floor().min(5.0);
        }
        means.push(sum / n as f32);
    }
    means
}

fn factorial(k: u32) -> f64 {
    (1..=k as u64).product::<u64>() as f64
}

/// `distribution(id, (cx,cy), "kind", a, b, [color])` — a named probability
/// distribution: **uniform**(lo=a, hi=b) and **exponential**(rate=a) as density
/// curves; **binomial**(n=a, p=b) and **poisson**(mean=a) as probability bars.
fn c_distribution(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let kind = a.text(2)?;
    let p1 = a.num(3)?;
    let p2 = a.opt_num(4)?.unwrap_or(0.0);
    // colour word OR `rainbow` (per-bar hue for the discrete distributions)
    let (rainbow, color) = color_or_rainbow(a, 5)?;
    let (w, h) = (540.0f32, 240.0f32);
    let x0 = c.x - w / 2.0;
    let base = c.y + h / 2.0;

    // helper: draw a peak-normalised density curve sampled over [lo, hi]
    let draw_curve = |s: &mut Scene, lo: f32, hi: f32, f: &dyn Fn(f32) -> f32| {
        let cn = 160;
        let dens: Vec<f32> = (0..=cn).map(|i| f(lo + (hi - lo) * i as f32 / cn as f32)).collect();
        let peak = dens.iter().cloned().fold(1e-9, f32::max);
        let pts: Vec<Vec2> = dens.iter().enumerate().map(|(i, &d)| {
            vec2(x0 + i as f32 / cn as f32 * w, base - (d / peak) * h)
        }).collect();
        let mut e = Entity::new(format!("{id}.curve"), Shape::Polyline { pts }, vec2(0.0, 0.0), color);
        e.stroke.width = 3.0;
        e.z = 1;
        e.tags.push(id.clone());
        s.add(e);
    };
    // helper: draw probability bars for values 0..=kmax
    let bars = |s: &mut Scene, probs: &[f64]| {
        let maxp = probs.iter().cloned().fold(1e-12, f64::max) as f32;
        let bw = w / probs.len() as f32;
        let nb = probs.len();
        for (k, &p) in probs.iter().enumerate() {
            let bc = bar_color(rainbow, color, k, nb);
            let bh = (p as f32 / maxp) * h;
            let bx = x0 + (k as f32 + 0.5) * bw;
            let mut e = Entity::new(format!("{id}.bar{k}"), Shape::Rect { w: bw * 0.85, h: bh }, vec2(bx, base - bh / 2.0), bc);
            e.stroke.fill = true;
            e.stroke.outline = true;
            e.stroke.outline_color = Some(bc);
            e.opacity = 0.85;
            e.tags = vec![id.clone(), format!("{id}.bars")];
            s.add(e);
        }
    };
    add_line(s, format!("{id}.axis"), vec2(x0, base), vec2(x0 + w, base), style::DIM, 2.0, 0.8, 0, vec![id.clone()]);
    let label = match kind.as_str() {
        "uniform" => {
            let (lo, hi) = (p1, p2.max(p1 + 1e-3));
            let pad = (hi - lo) * 0.25;
            draw_curve(s, lo - pad, hi + pad, &move |x| if x >= lo && x <= hi { 1.0 } else { 0.0 });
            format!("uniform [{}, {}]", fmt(lo), fmt(hi))
        }
        "exponential" => {
            let rate = p1.max(1e-3);
            draw_curve(s, 0.0, 5.0 / rate, &move |x| rate * (-rate * x).exp());
            format!("exponential  (rate {})", fmt(rate))
        }
        "binomial" => {
            let n = (p1.round() as i64).clamp(1, 20) as u32;
            let p = p2.clamp(0.0, 1.0) as f64;
            let probs: Vec<f64> = (0..=n).map(|k| {
                factorial(n) / (factorial(k) * factorial(n - k)) * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32)
            }).collect();
            bars(s, &probs);
            format!("binomial  (n = {n}, p = {})", fmt(p as f32))
        }
        "poisson" => {
            let lam = p1.max(0.1) as f64;
            let kmax = ((lam * 2.5) as u32 + 6).min(30);
            let probs: Vec<f64> = (0..=kmax).map(|k| lam.powi(k as i32) * (-lam).exp() / factorial(k)).collect();
            bars(s, &probs);
            format!("poisson  (mean {})", fmt(lam as f32))
        }
        other => {
            return Err(Error::new(
                format!("unknown distribution `{other}` (uniform / exponential / binomial / poisson)"),
                a.span_of(2),
            ))
        }
    };
    add_label(s, format!("{id}.name"), &label, vec2(c.x, base + 34.0), 22.0, color, &id);
    Ok(())
}

/// `confidence(id, (cx,cy), mean, sd, n, [level], [width])` — a confidence
/// interval for a mean: the point estimate on a number line with an error bar of
/// ± z·sd/√n (z from `level`, default 95%). `{id}.estimate`, `{id}.bar` (+ caps),
/// `{id}.ci`.
fn c_confidence(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let mean = a.num(2)?;
    let sd = a.num(3)?;
    let n = a.num(4)?.max(1.0);
    let level = a.opt_num(5)?.unwrap_or(95.0);
    let w = a.opt_num(6)?.unwrap_or(560.0);
    let z = if level >= 99.0 { 2.576 } else if level >= 95.0 { 1.96 } else { 1.645 };
    let margin = z * sd / n.sqrt();
    let span = (margin * 3.0).max(1e-3);
    let sx = |v: f32| c.x + (v - mean) / span * (w / 2.0);
    let y0 = c.y;

    add_line(s, format!("{id}.line"), vec2(sx(mean - span), y0), vec2(sx(mean + span), y0), style::DIM, 2.0, 0.7, 0, vec![id.clone()]);
    // the CI error bar + caps
    add_line(s, format!("{id}.bar"), vec2(sx(mean - margin), y0), vec2(sx(mean + margin), y0), style::CYAN, 4.0, 1.0, 1, vec![id.clone()]);
    for e in [-margin, margin] {
        add_line(s, format!("{id}.cap{}", if e < 0.0 { "lo" } else { "hi" }), vec2(sx(mean + e), y0 - 18.0), vec2(sx(mean + e), y0 + 18.0), style::CYAN, 3.0, 1.0, 1, vec![id.clone()]);
        add_label(s, format!("{id}.b{}", if e < 0.0 { "lo" } else { "hi" }), &fmt(mean + e), vec2(sx(mean + e), y0 + 40.0), 17.0, style::DIM, &id);
    }
    // point estimate
    let mut dot = Entity::new(format!("{id}.estimate"), Shape::Circle { r: 8.0 }, vec2(sx(mean), y0), style::GOLD);
    dot.stroke.fill = true;
    dot.z = 2;
    dot.tags.push(id.clone());
    s.add(dot);
    add_label(s, format!("{id}.ci"), &format!("{}% CI:  {} ± {}", fmt(level), fmt(mean), fmt(margin)), vec2(c.x, y0 - 44.0), 22.0, style::GOLD, &id);
    Ok(())
}

/// `montecarlo(id, (cx,cy), points, [seed], [size])` — estimate π by darts: random
/// points in a square, those inside the inscribed circle in cyan, outside in
/// magenta; π ≈ 4·inside/total. Seeded → reproducible. `{id}.square`,
/// `{id}.circle`, `{id}.points`, `{id}.pi`.
fn c_montecarlo(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let points = (a.num(2)?.round() as i32).clamp(20, 4000) as usize;
    let seed = a.opt_num(3)?.map(|v| v as u64).unwrap_or(12345);
    let size = a.opt_num(4)?.unwrap_or(230.0);

    let mut sq = Entity::new(format!("{id}.square"), Shape::Rect { w: 2.0 * size, h: 2.0 * size }, c, style::DIM);
    sq.stroke.fill = false;
    sq.stroke.outline = true;
    sq.tags.push(id.clone());
    s.add(sq);
    let mut circ = Entity::new(format!("{id}.circle"), Shape::Circle { r: size }, c, style::GOLD);
    circ.stroke.fill = false;
    circ.stroke.outline = true;
    circ.stroke.outline_color = Some(style::GOLD);
    circ.tags.push(id.clone());
    s.add(circ);

    let mut state = seed.wrapping_add(1);
    let mut inside = 0usize;
    for i in 0..points {
        let x = lcg_next(&mut state) * 2.0 - 1.0;
        let y = lcg_next(&mut state) * 2.0 - 1.0;
        let hit = x * x + y * y <= 1.0;
        if hit {
            inside += 1;
        }
        let mut e = Entity::new(format!("{id}.pt{i}"), Shape::Circle { r: 3.5 }, vec2(c.x + x * size, c.y - y * size), if hit { style::CYAN } else { style::MAGENTA });
        e.stroke.fill = true;
        e.z = 1;
        e.tags = vec![id.clone(), format!("{id}.points")];
        s.add(e);
    }
    let pi = 4.0 * inside as f32 / points as f32;
    add_label(s, format!("{id}.pi"), &format!("pi ~ {pi:.3}   ({points} darts)"), vec2(c.x, c.y + size + 34.0), 22.0, style::GOLD, &id);
    Ok(())
}

/// `randomwalk(id, (cx,cy), steps, [seed], [scale])` — a 2-D random walk: from the
/// centre, each step heads a random direction. Draws the path, a start dot (lime)
/// and end dot (gold). Seeded → reproducible. `{id}.path`, `{id}.start`, `{id}.end`.
fn c_randomwalk(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let steps = (a.num(2)?.round() as i32).clamp(2, 3000) as usize;
    let seed = a.opt_num(3)?.map(|v| v as u64).unwrap_or(12345);
    let scale = a.opt_num(4)?.unwrap_or(14.0);
    let mut state = seed.wrapping_add(1);
    let mut pos = c;
    let mut path = vec![pos];
    for _ in 0..steps {
        let ang = lcg_next(&mut state) * std::f32::consts::TAU;
        pos = vec2(pos.x + scale * ang.cos(), pos.y + scale * ang.sin());
        path.push(pos);
    }
    let (start, end) = (path[0], *path.last().unwrap());
    let mut p = Entity::new(format!("{id}.path"), Shape::Polyline { pts: path }, vec2(0.0, 0.0), style::CYAN);
    p.stroke.width = 2.0;
    p.opacity = 0.9;
    p.tags.push(id.clone());
    s.add(p);
    for (nm, at, col) in [("start", start, style::LIME), ("end", end, style::GOLD)] {
        let mut d = Entity::new(format!("{id}.{nm}"), Shape::Circle { r: 7.0 }, at, col);
        d.stroke.fill = true;
        d.z = 2;
        d.tags.push(id.clone());
        s.add(d);
    }
    Ok(())
}

/// Running proportion of heads over `trials` seeded coin flips — settles onto 0.5
/// (the Law of Large Numbers). Deterministic in `seed`.
fn lln_proportions(trials: usize, seed: u64) -> Vec<f32> {
    let mut state = seed.wrapping_add(1);
    let mut heads = 0u32;
    let mut out = Vec::with_capacity(trials);
    for n in 1..=trials {
        if lcg_next(&mut state) < 0.5 {
            heads += 1;
        }
        out.push(heads as f32 / n as f32);
    }
    out
}

/// `lln(id, (cx,cy), trials, [seed], [width], [height])` — the Law of Large
/// Numbers: the running proportion of heads over many coin flips, wild at first,
/// **settling onto the true 0.5**. Draws the proportion curve (`{id}.curve`), the
/// true-probability reference line (`{id}.ref`), axis labels, and the final
/// value. Seeded → reproducible.
fn c_lln(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let trials = (a.num(2)?.round() as i32).clamp(10, 5000) as usize;
    let seed = a.opt_num(3)?.map(|v| v as u64).unwrap_or(12345);
    let w = a.opt_num(4)?.unwrap_or(680.0);
    let h = a.opt_num(5)?.unwrap_or(300.0);
    let props = lln_proportions(trials, seed);
    let x0 = c.x - w / 2.0;
    let base = c.y + h / 2.0; // p = 0
    let py = |p: f32| base - p * h; // p in [0,1]
    let px = |n: usize| x0 + (n as f32) / (trials as f32) * w;

    // frame: p=0 axis + the true-probability reference at p=0.5
    add_line(s, format!("{id}.axis"), vec2(x0, base), vec2(x0 + w, base), style::DIM, 2.0, 0.7, 0, vec![id.clone()]);
    add_line(s, format!("{id}.ref"), vec2(x0, py(0.5)), vec2(x0 + w, py(0.5)), style::GOLD, 2.0, 0.85, 0, vec![id.clone()]);
    for (p, txt) in [(0.0, "0"), (0.5, "0.5"), (1.0, "1")] {
        add_label(s, format!("{id}.y{}", (p * 10.0) as i32), txt, vec2(x0 - 26.0, py(p)), 17.0, style::DIM, &id);
    }
    add_label(s, format!("{id}.truelbl"), "true probability = 0.5", vec2(x0 + w - 130.0, py(0.5) - 18.0), 18.0, style::GOLD, &id);

    // the running-proportion curve
    let pts: Vec<Vec2> = props.iter().enumerate().map(|(i, &p)| vec2(px(i + 1), py(p))).collect();
    let mut curve = Entity::new(format!("{id}.curve"), Shape::Polyline { pts }, vec2(0.0, 0.0), style::CYAN);
    curve.stroke.width = 2.5;
    curve.z = 1;
    curve.tags.push(id.clone());
    s.add(curve);

    add_label(s, format!("{id}.finallbl"), &format!("after {} flips: {:.3}", trials, props[trials - 1]), vec2(c.x, base + 34.0), 20.0, style::CYAN, &id);
    Ok(())
}

/// `clt(id, (cx,cy), samplesize, trials, [seed], [width], [height])` — the Central
/// Limit Theorem: run `trials` experiments, each the average of `samplesize` dice
/// (1–6, uniform), and histogram those averages. However flat one die is, the
/// averages pile into a **bell**. Draws the histogram of sample means
/// (`{id}.bar{k}`, tagged `{id}.bars`), the theoretical normal it converges to
/// (`{id}.curve`), the population mean line (`{id}.mean`), value ticks, and an
/// `{id}.info` label. Seeded → reproducible.
fn c_clt(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let c = a.pair(1)?;
    let n = (a.num(2)?.round() as i32).clamp(1, 100) as usize;
    let trials = (a.num(3)?.round() as i32).clamp(10, 5000) as usize;
    let seed = a.opt_num(4)?.map(|v| v as u64).unwrap_or(12345);
    let w = a.opt_num(5)?.unwrap_or(560.0);
    let h = a.opt_num(6)?.unwrap_or(260.0);
    let (rainbow, color) = color_or_rainbow(a, 7)?;

    // run the experiments with the seeded PRNG → the sample means
    let means = clt_means(n, trials, seed);

    // histogram the means over the die range [1, 6]
    let bins = 30usize;
    let (lo, hi) = (1.0f32, 6.0f32);
    let mut counts = vec![0i32; bins];
    for &m in &means {
        let k = (((m - lo) / (hi - lo)) * bins as f32).floor() as i32;
        counts[k.clamp(0, bins as i32 - 1) as usize] += 1;
    }
    let maxc = *counts.iter().max().unwrap() as f32;

    let x0 = c.x - w / 2.0;
    let baseline = c.y + h / 2.0;
    let barw = w / bins as f32;
    let sx = |v: f32| x0 + ((v - lo) / (hi - lo)) * w;

    add_line(s, format!("{id}.axis"), vec2(x0, baseline), vec2(x0 + w, baseline), style::DIM, 2.0, 0.8, 0, vec![id.clone()]);

    // one bar per bin (always created so a 0..bins loop is safe)
    for (k, &count) in counts.iter().enumerate() {
        let bc = bar_color(rainbow, color, k, bins);
        let bh = (count as f32 / maxc) * h;
        let bx = x0 + (k as f32 + 0.5) * barw;
        let mut e = Entity::new(
            format!("{id}.bar{k}"),
            Shape::Rect { w: barw * 0.9, h: bh },
            vec2(bx, baseline - bh / 2.0),
            bc,
        );
        e.stroke.fill = true;
        e.stroke.outline = true;
        e.stroke.outline_color = Some(bc);
        e.opacity = 0.85;
        e.tags = vec![id.clone(), format!("{id}.bars")];
        s.add(e);
    }

    // the theoretical normal it converges to: mean 3.5, σ = σ_die / √n
    let mu = 3.5;
    let sig = (35.0f32 / 12.0).sqrt() / (n as f32).sqrt();
    let cn = 120;
    let mut curve = Vec::with_capacity(cn + 1);
    for i in 0..=cn {
        let v = lo + (hi - lo) * i as f32 / cn as f32;
        let z = (v - mu) / sig;
        curve.push(vec2(sx(v), baseline - (-0.5 * z * z).exp() * h));
    }
    let mut ce = Entity::new(format!("{id}.curve"), Shape::Polyline { pts: curve }, vec2(0.0, 0.0), style::GOLD);
    ce.stroke.width = 3.0;
    ce.z = 2;
    ce.tags.push(id.clone());
    s.add(ce);

    add_line(s, format!("{id}.mean"), vec2(sx(mu), baseline), vec2(sx(mu), baseline - h - 10.0), style::GOLD, 2.0, 0.9, 1, vec![id.clone()]);
    for v in 1..=6 {
        add_label(s, format!("{id}.t{v}"), &fmt(v as f32), vec2(sx(v as f32), baseline + 22.0), 17.0, style::DIM, &id);
    }
    add_label(s, format!("{id}.info"), &format!("average of {n} dice, {trials} times"), vec2(c.x, baseline - h - 30.0), 20.0, style::DIM, &id);

    Ok(())
}

/// Register the stats kit into `r`.
pub fn register(r: &mut Registry) {
    r.ctor("histogram", c_histogram);
    r.ctor("summary", c_summary);
    r.ctor("boxplot", c_boxplot);
    r.ctor("skew", c_skew);
    r.ctor("correlation", c_correlation);
    r.ctor("lln", c_lln);
    r.ctor("hypothesis", c_hypothesis);
    r.ctor("covariance", c_covariance);
    r.ctor("bayes", c_bayes);
    r.ctor("distribution", c_distribution);
    r.ctor("confidence", c_confidence);
    r.ctor("montecarlo", c_montecarlo);
    r.ctor("randomwalk", c_randomwalk);
    r.ctor("bellcurve", c_bellcurve);
    r.ctor("gaussian", c_bellcurve);
    r.ctor("clt", c_clt);
}

#[cfg(test)]
mod tests {
    use super::{histogram_bins, parse_data};

    #[test]
    fn parse_reads_numbers() {
        assert_eq!(parse_data("72 85, 90  68"), vec![72.0, 85.0, 90.0, 68.0]);
        assert!(parse_data("").is_empty());
    }

    #[test]
    fn monte_carlo_estimates_pi() {
        // reproduce the dart loop; 4·inside/n ≈ π for many seeded darts
        use super::lcg_next;
        let mut state = 5u64.wrapping_add(1);
        let (mut inside, n) = (0usize, 4000usize);
        for _ in 0..n {
            let x = lcg_next(&mut state) * 2.0 - 1.0;
            let y = lcg_next(&mut state) * 2.0 - 1.0;
            if x * x + y * y <= 1.0 {
                inside += 1;
            }
        }
        let pi = 4.0 * inside as f32 / n as f32;
        assert!((pi - std::f32::consts::PI).abs() < 0.15, "pi estimate = {pi}");
    }

    #[test]
    fn factorial_is_correct() {
        use super::factorial;
        assert_eq!(factorial(0), 1.0);
        assert_eq!(factorial(5), 120.0);
    }

    #[test]
    fn covariance_sign_tracks_the_relationship() {
        use super::covariance_of;
        use macroquad::prelude::vec2;
        assert!(covariance_of(&[vec2(1.0, 1.0), vec2(2.0, 2.0), vec2(3.0, 3.0)]) > 0.0);
        assert!(covariance_of(&[vec2(1.0, 3.0), vec2(2.0, 2.0), vec2(3.0, 1.0)]) < 0.0);
        // symmetric cloud → ~0
        assert!(covariance_of(&[vec2(-1.0, 1.0), vec2(1.0, 1.0), vec2(-1.0, -1.0), vec2(1.0, -1.0)]).abs() < 1e-4);
    }

    #[test]
    fn normal_tail_matches_known_critical_values() {
        use super::normal_tail;
        assert!((normal_tail(0.0) - 0.5).abs() < 1e-3);
        assert!((normal_tail(1.96) - 0.025).abs() < 2e-3); // 95% two-tailed critical
        assert!((normal_tail(1.645) - 0.05).abs() < 2e-3); // 90% two-tailed critical
        assert!(normal_tail(4.0) < 1e-3);
    }

    #[test]
    fn describe_matches_hand_computation() {
        use super::describe;
        // [2,4,4,4,5,5,7,9]: mean 5, median 4.5, mode 4, variance 4, std 2
        let st = describe(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        assert!((st.mean - 5.0).abs() < 1e-4);
        assert!((st.median - 4.5).abs() < 1e-4);
        assert_eq!(st.mode, Some(4.0));
        assert!((st.var - 4.0).abs() < 1e-4 && (st.std - 2.0).abs() < 1e-4);
        assert_eq!((st.lo, st.hi), (2.0, 9.0));
        // odd count → the middle value is the median
        assert!((describe(&[1.0, 3.0, 100.0]).median - 3.0).abs() < 1e-4);
        // all-unique → no mode
        assert_eq!(describe(&[1.0, 2.0, 3.0]).mode, None);
    }

    #[test]
    fn regression_recovers_line_and_correlation() {
        use super::regression;
        use macroquad::prelude::vec2;
        // points exactly on y = 2x + 1 → m=2, k=1, r=+1
        let up = [vec2(0.0, 1.0), vec2(1.0, 3.0), vec2(2.0, 5.0), vec2(3.0, 7.0)];
        let (m, k, r) = regression(&up).unwrap();
        assert!((m - 2.0).abs() < 1e-4 && (k - 1.0).abs() < 1e-4 && (r - 1.0).abs() < 1e-4);
        // a perfectly decreasing line → r = -1
        let down = [vec2(0.0, 5.0), vec2(1.0, 3.0), vec2(2.0, 1.0)];
        assert!((regression(&down).unwrap().2 + 1.0).abs() < 1e-4);
        // no x-spread → None
        assert!(regression(&[vec2(2.0, 1.0), vec2(2.0, 9.0)]).is_none());
    }

    #[test]
    fn skewness_detects_direction() {
        use super::skewness;
        // a right tail → positive skew
        assert!(skewness(&[1.0, 1.0, 1.0, 2.0, 2.0, 3.0, 10.0]) > 0.3);
        // its mirror → negative skew
        assert!(skewness(&[10.0, 10.0, 10.0, 9.0, 9.0, 8.0, 1.0]) < -0.3);
        // symmetric → ≈ 0
        assert!(skewness(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]).abs() < 0.1);
    }

    #[test]
    fn five_number_summary_matches_tukey() {
        use super::five_number;
        // 1..8 (even): min 1, Q1 2.5, median 4.5, Q3 6.5, max 8 → IQR 4
        let (mn, q1, med, q3, mx) = five_number(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!((mn, q1, med, q3, mx), (1.0, 2.5, 4.5, 6.5, 8.0));
        // odd count excludes the middle from the halves: 1..5 → Q1 1.5, med 3, Q3 4.5
        let (_, q1o, medo, q3o, _) = five_number(&[5.0, 3.0, 1.0, 4.0, 2.0]);
        assert_eq!((q1o, medo, q3o), (1.5, 3.0, 4.5));
    }

    #[test]
    fn lln_settles_onto_the_true_probability() {
        use super::lln_proportions;
        let p = lln_proportions(2000, 5);
        assert_eq!(p, lln_proportions(2000, 5)); // deterministic
        assert!((p[p.len() - 1] - 0.5).abs() < 0.05, "final proportion = {}", p[p.len() - 1]);
        // later estimates are calmer than the earliest one
        assert!((p[p.len() - 1] - 0.5).abs() <= (p[0] - 0.5).abs());
    }

    #[test]
    fn clt_is_deterministic_and_centred() {
        use super::clt_means;
        // same seed → identical results (reproducible renders)
        let a = clt_means(5, 500, 7);
        let b = clt_means(5, 500, 7);
        assert_eq!(a, b);
        // a different seed → a different sequence
        assert_ne!(clt_means(5, 500, 7), clt_means(5, 500, 8));
        // the mean of the sample means sits at the population mean (dice → 3.5)
        let m: f32 = a.iter().sum::<f32>() / a.len() as f32;
        assert!((m - 3.5).abs() < 0.15, "mean of means = {m}");
        // averaging more dice tightens the spread (σ shrinks with √n)
        let spread = |v: &[f32]| {
            let mu = v.iter().sum::<f32>() / v.len() as f32;
            (v.iter().map(|x| (x - mu) * (x - mu)).sum::<f32>() / v.len() as f32).sqrt()
        };
        assert!(spread(&clt_means(20, 500, 7)) < spread(&clt_means(2, 500, 7)));
    }

    #[test]
    fn binning_counts_every_value_once() {
        // 0..10, 5 bins → two values per bin; the max value lands in the last bin
        let data: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let (lo, hi, counts) = histogram_bins(&data, 5);
        assert_eq!((lo, hi), (0.0, 9.0));
        assert_eq!(counts.iter().sum::<i32>(), 10); // no value dropped or double-counted
        assert_eq!(counts.len(), 5);
        // a clustered set peaks in the right bin
        let (_, _, c2) = histogram_bins(&[1.0, 5.0, 5.0, 5.0, 9.0], 3);
        assert_eq!(c2[1], 3); // the three 5's fall in the middle bin
    }
}
