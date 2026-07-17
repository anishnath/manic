//! Optics physics — faithful, closed-form ports of the goldmine's ray engine
//! (`crypto-tool/.../optical-designer-trace.js`). No ODE, no time: light is
//! geometry. These helpers are the shared physics every optics builtin draws on
//! (`refract`/`lens` today; `prism`/`achromat` next), kept in one small module so
//! the author-facing `builtins.rs` stays about *staging*, not equations.

use macroquad::prelude::{Vec2, Vec3};

/// Vector Snell's law in 3-D — the field-aberration companion to [`refract_vec`].
/// `d` incident unit dir, `n` an either-facing unit normal, `eta = n1/n2`.
pub fn refract_vec3(d: Vec3, n: Vec3, eta: f32) -> Option<Vec3> {
    let mut nrm = n;
    let mut cosi = -d.dot(nrm);
    if cosi < 0.0 {
        nrm = -nrm;
        cosi = -cosi;
    }
    let sin2t = eta * eta * (1.0 - cosi * cosi);
    if sin2t > 1.0 {
        return None;
    }
    let cost = (1.0 - sin2t).sqrt();
    Some((d * eta + nrm * (eta * cosi - cost)).normalize())
}

/// Trace a ray against one **conic** surface in 3-D — the full rotationally
/// symmetric surface `(1+k)·X² − 2r·X + (Y² + Z²) = 0` about the x-axis (vertex
/// at `x = vx`, axis through the origin in y,z). This is what off-axis field
/// tracing needs (coma/astigmatism break the meridional symmetry the 2-D
/// [`trace_conic`] relies on). Returns the hit and refracted unit dir, or `None`.
pub fn trace_conic_3d(o: Vec3, d: Vec3, vx: f32, r: f32, k: f32, n1: f32, n2: f32) -> Option<(Vec3, Vec3)> {
    if r.abs() > 1.0e6 {
        if d.x.abs() < 1e-9 {
            return None;
        }
        let t = (vx - o.x) / d.x;
        if t <= 1e-3 {
            return None;
        }
        let hit = o + d * t;
        return refract_vec3(d, Vec3::new(1.0, 0.0, 0.0), n1 / n2).map(|nd| (hit, nd));
    }
    let ox = o.x - vx;
    let k1 = 1.0 + k;
    let a = k1 * d.x * d.x + d.y * d.y + d.z * d.z;
    let b = 2.0 * (k1 * ox * d.x + o.y * d.y + o.z * d.z) - 2.0 * r * d.x;
    let c = k1 * ox * ox - 2.0 * r * ox + o.y * o.y + o.z * o.z;
    let roots: Vec<f32> = if a.abs() < 1e-9 {
        if b.abs() < 1e-12 {
            return None;
        }
        vec![-c / b]
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            return None;
        }
        let sq = disc.sqrt();
        vec![(-b - sq) / (2.0 * a), (-b + sq) / (2.0 * a)]
    };
    let mut best: Option<(f32, Vec3)> = None;
    for t in roots {
        if t <= 1e-3 {
            continue;
        }
        let hit = o + d * t;
        let dvx = (hit.x - vx).abs();
        if best.map_or(true, |(bd, _)| dvx < bd) {
            best = Some((dvx, hit));
        }
    }
    let (_, hit) = best?;
    let nrm = Vec3::new(k1 * (hit.x - vx) - r, hit.y, hit.z).normalize();
    refract_vec3(d, nrm, n1 / n2).map(|nd| (hit, nd))
}

/// 2-D cross product `a × b` (a scalar) — used for ray/segment intersection.
fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

/// Vector form of Snell's law in 2-D. `d` is the incident **unit** direction,
/// `n` an (either-facing) unit surface normal, and `eta = n1/n2`. Returns the
/// refracted unit direction, or `None` on total internal reflection. (This is
/// the 2-D reduction of the goldmine's 3-D `traceRay3D` refraction step.)
pub fn refract_vec(d: Vec2, n: Vec2, eta: f32) -> Option<Vec2> {
    let mut nrm = n;
    let mut cosi = -d.dot(nrm);
    if cosi < 0.0 {
        nrm = -nrm; // flip the normal to face the incoming ray
        cosi = -cosi;
    }
    let sin2t = eta * eta * (1.0 - cosi * cosi);
    if sin2t > 1.0 {
        return None; // TIR
    }
    let cost = (1.0 - sin2t).sqrt();
    Some((d * eta + nrm * (eta * cosi - cost)).normalize())
}

/// Intersect a ray (origin `o`, unit dir `d`) with the segment `a→b`. Returns
/// the hit point when the ray crosses the segment strictly ahead of the origin.
pub fn ray_segment(o: Vec2, d: Vec2, a: Vec2, b: Vec2) -> Option<Vec2> {
    let e = b - a;
    let denom = cross(d, e);
    if denom.abs() < 1e-6 {
        return None; // parallel
    }
    let diff = a - o;
    let t = cross(diff, e) / denom;
    let u = cross(diff, d) / denom;
    if t > 1e-3 && (-1e-3..=1.0 + 1e-3).contains(&u) {
        Some(o + d * t)
    } else {
        None
    }
}

/// Snell's law in 2-D. Given an incidence angle `theta_i` (radians, measured from
/// the surface normal) crossing from index `n1` into `n2`, returns the refraction
/// angle `theta_t` (radians) — or `None` for **total internal reflection** (the
/// refracted ray ceases to exist, `sin θ_t > 1`).
pub fn snell(theta_i: f32, n1: f32, n2: f32) -> Option<f32> {
    let s = (n1 / n2) * theta_i.sin();
    if s.abs() > 1.0 {
        None
    } else {
        Some(s.asin())
    }
}

/// The critical angle for total internal reflection crossing `n1 → n2` (radians),
/// or `None` when `n1 <= n2` (no TIR — refraction always succeeds). Used by the
/// dispersion/lens builtins to mark the TIR onset; part of the shared engine.
#[allow(dead_code)]
pub fn critical_angle(n1: f32, n2: f32) -> Option<f32> {
    if n1 > n2 {
        Some((n2 / n1).asin())
    } else {
        None
    }
}

/// Trace a ray against one **conic** refracting surface of a lens system (the
/// multi-surface `lenssystem` builtin). The vertex sits on the axis at
/// `(vx, axis_y)`; `r` is the signed radius of curvature (`|r|` huge ⇒ flat);
/// `k` is the conic constant (0 = sphere, −1 = parabola, `< −1` = hyperbola,
/// `−1 < k < 0` = prolate ellipse, `> 0` = oblate). The rotationally-symmetric
/// surface obeys `(1+k)·X² − 2r·X + Y² = 0` (X = x−vx, Y = y−axis_y). Returns the
/// hit point and refracted unit direction (`n1 → n2`), or `None` on miss/TIR.
/// This is the 2-D reduction of the goldmine's conic `traceRay2D`.
pub fn trace_conic(
    o: Vec2,
    d: Vec2,
    vx: f32,
    r: f32,
    k: f32,
    axis_y: f32,
    n1: f32,
    n2: f32,
) -> Option<(Vec2, Vec2)> {
    if r.abs() > 1.0e6 {
        // flat surface: intersect the vertical line x = vx, normal along the axis
        if d.x.abs() < 1e-9 {
            return None;
        }
        let t = (vx - o.x) / d.x;
        if t <= 1e-3 {
            return None;
        }
        let hit = o + d * t;
        return refract_vec(d, Vec2::new(1.0, 0.0), n1 / n2).map(|nd| (hit, nd));
    }
    // substitute X = ox + t·dx, Y = oy + t·dy into (1+k)X² − 2rX + Y² = 0
    let (ox, oy) = (o.x - vx, o.y - axis_y);
    let k1 = 1.0 + k;
    let a = k1 * d.x * d.x + d.y * d.y;
    let b = 2.0 * (k1 * ox * d.x + oy * d.y) - 2.0 * r * d.x;
    let c = k1 * ox * ox - 2.0 * r * ox + oy * oy;
    // candidate t roots (linear if a≈0, e.g. a parabola hit by an axial ray)
    let roots: Vec<f32> = if a.abs() < 1e-9 {
        if b.abs() < 1e-12 {
            return None;
        }
        vec![-c / b]
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            return None;
        }
        let sq = disc.sqrt();
        vec![(-b - sq) / (2.0 * a), (-b + sq) / (2.0 * a)]
    };
    // keep the forward intersection nearest the vertex (the near face)
    let mut best: Option<(f32, Vec2)> = None;
    for t in roots {
        if t <= 1e-3 {
            continue;
        }
        let hit = o + d * t;
        let dvx = (hit.x - vx).abs();
        if best.map_or(true, |(bd, _)| dvx < bd) {
            best = Some((dvx, hit));
        }
    }
    let (_, hit) = best?;
    // normal ∝ ∇F = ((1+k)X − r, Y); refract_vec re-faces it to the ray
    let nrm = Vec2::new(k1 * (hit.x - vx) - r, hit.y - axis_y).normalize();
    refract_vec(d, nrm, n1 / n2).map(|nd| (hit, nd))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Entering a denser medium bends the ray TOWARD the normal (θt < θi); the
    /// exact Snell ratio holds.
    #[test]
    fn refraction_bends_toward_normal() {
        let ti = 40f32.to_radians();
        let tt = snell(ti, 1.0, 1.5).expect("air→glass always refracts");
        assert!(tt < ti, "should bend toward the normal: {tt} !< {ti}");
        // n1·sin θi = n2·sin θt (Snell) to within float tolerance
        assert!((1.0 * ti.sin() - 1.5 * tt.sin()).abs() < 1e-5);
    }

    /// Past the critical angle, light in the denser medium is totally internally
    /// reflected — no transmitted ray.
    #[test]
    fn tir_beyond_critical_angle() {
        let tc = critical_angle(1.5, 1.0).expect("glass→air has a critical angle");
        assert!((tc.to_degrees() - 41.81).abs() < 0.1, "critical angle ≈ 41.8°");
        assert!(snell(tc - 0.02, 1.5, 1.0).is_some(), "just under: still refracts");
        assert!(snell(tc + 0.02, 1.5, 1.0).is_none(), "just over: TIR");
        assert!(critical_angle(1.0, 1.5).is_none(), "no TIR entering a denser medium");
    }

    /// A convex air→glass surface bends a parallel ray above the axis DOWNWARD
    /// (toward the axis) — the converging action of a positive lens.
    #[test]
    fn convex_surface_converges() {
        let o = Vec2::new(0.0, 10.0);
        let d = Vec2::new(1.0, 0.0); // parallel to the axis, above it
        let (hit, nd) = trace_conic(o, d, 100.0, 50.0, 0.0, 0.0, 1.0, 1.5).expect("ray should hit");
        assert!((hit.x - 100.0).abs() < 6.0, "hit near the vertex");
        assert!(nd.y < 0.0, "should bend toward the axis (downward): {nd:?}");
    }

    /// The conic term matters: a strong conic (k) changes where a marginal ray
    /// lands vs a sphere (k = 0) — the mechanism that lets an asphere cancel
    /// spherical aberration.
    #[test]
    fn conic_changes_the_bend() {
        let o = Vec2::new(0.0, 40.0);
        let d = Vec2::new(1.0, 0.0);
        let (_, sphere) = trace_conic(o, d, 100.0, 120.0, 0.0, 0.0, 1.0, 1.5).unwrap();
        let (_, hyper) = trace_conic(o, d, 100.0, 120.0, -3.0, 0.0, 1.0, 1.5).unwrap();
        assert!((sphere.y - hyper.y).abs() > 1e-3, "conic should change the refracted slope");
    }
}
