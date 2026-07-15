//! Generic explicit ODE integrators over an **n-dimensional** state vector.
//!
//! The `eval` closure receives the current `state` and must fill `deriv[i]` with
//! d(state[i])/dt. This mirrors the well-tested JS `core/solver.js` (RK4 / Euler)
//! and is the substrate for the physics kit: a system is a *named state vector*
//! plus its *derivative function*, pre-integrated once at build time into a
//! sampled trajectory that the stateless timeline can replay deterministically.
//!
//! **Time-dependent forcing.** RK4 here is autonomous — the closure sees only the
//! state, not an explicit `t`. Driven/damped systems (a forced pendulum, an AC
//! source) carry time *as a state component* with derivative 1 (the standard
//! "clock variable" trick): `d[TIME] = 1.0`, then reference `state[TIME]` in the
//! forcing term. This keeps one uniform integrator for every system.
//!
//! Numerics are `f32` to match the rest of the engine (`Vec2`, plots). Scratch is
//! allocated once per integration and reused across steps (no per-step allocation).

/// Reusable RK4 scratch buffers, sized to the state vector. Allocated once per
/// integration run and reused every step.
struct Scratch {
    k1: Vec<f32>,
    k2: Vec<f32>,
    k3: Vec<f32>,
    k4: Vec<f32>,
    tmp: Vec<f32>,
}

impl Scratch {
    fn new(n: usize) -> Self {
        Scratch {
            k1: vec![0.0; n],
            k2: vec![0.0; n],
            k3: vec![0.0; n],
            k4: vec![0.0; n],
            tmp: vec![0.0; n],
        }
    }
}

/// One classic 4th-order Runge–Kutta step, in place, using caller-provided scratch.
/// The single source of the RK4 math; `rk4_step` and the integrators all route here.
fn rk4_into<F>(state: &mut [f32], dt: f32, eval: &mut F, sc: &mut Scratch)
where
    F: FnMut(&[f32], &mut [f32]),
{
    let n = state.len();
    // k1 = f(y)
    eval(state, &mut sc.k1);
    // k2 = f(y + dt/2 · k1)
    for i in 0..n {
        sc.tmp[i] = state[i] + 0.5 * dt * sc.k1[i];
    }
    eval(&sc.tmp, &mut sc.k2);
    // k3 = f(y + dt/2 · k2)
    for i in 0..n {
        sc.tmp[i] = state[i] + 0.5 * dt * sc.k2[i];
    }
    eval(&sc.tmp, &mut sc.k3);
    // k4 = f(y + dt · k3)
    for i in 0..n {
        sc.tmp[i] = state[i] + dt * sc.k3[i];
    }
    eval(&sc.tmp, &mut sc.k4);
    // y += (dt/6)(k1 + 2k2 + 2k3 + k4)
    for i in 0..n {
        state[i] += dt / 6.0 * (sc.k1[i] + 2.0 * sc.k2[i] + 2.0 * sc.k3[i] + sc.k4[i]);
    }
}

/// One classic 4th-order Runge–Kutta step, in place. `dt` is the step size;
/// `eval(state, deriv)` fills `deriv` with the time-derivative of each component.
/// O(dt⁴) local error — the accuracy workhorse. (For many steps, prefer
/// [`integrate`] / [`integrate_sampled`], which reuse scratch across steps.)
pub fn rk4_step<F>(state: &mut [f32], dt: f32, eval: &mut F)
where
    F: FnMut(&[f32], &mut [f32]),
{
    let mut sc = Scratch::new(state.len());
    rk4_into(state, dt, eval, &mut sc);
}

/// One forward-Euler step (1st order) — cheaper, less accurate. Provided for
/// parity with the reference and for comparison/debug demos.
pub fn euler_step<F>(state: &mut [f32], dt: f32, eval: &mut F)
where
    F: FnMut(&[f32], &mut [f32]),
{
    let n = state.len();
    let mut d = vec![0.0f32; n];
    eval(state, &mut d);
    for i in 0..n {
        state[i] += dt * d[i];
    }
}

/// Integrate from `state0` for `steps` RK4 steps of size `dt`, returning the full
/// sampled trajectory — `steps + 1` snapshots including the initial state. Stops
/// early (returning what it has) if any component becomes non-finite, so a
/// blow-up degrades gracefully instead of poisoning the whole run.
pub fn integrate<F>(state0: &[f32], dt: f32, steps: usize, eval: F) -> Vec<Vec<f32>>
where
    F: FnMut(&[f32], &mut [f32]),
{
    integrate_sampled(state0, dt, 1, steps, eval)
}

/// Integrate with **substep control**: advance `substeps` RK4 steps of size `dt`
/// between each recorded snapshot, for `samples` snapshots. Returns `samples + 1`
/// state snapshots (including the initial state).
///
/// This is the physics-oriented API: simulate at a fine `dt` for stability/accuracy
/// while emitting only at animation-frame resolution, so the replayed trajectory
/// stays small. `substeps = 1` is equivalent to [`integrate`]. Stops early on a
/// non-finite state (checked every substep).
pub fn integrate_sampled<F>(
    state0: &[f32],
    dt: f32,
    substeps: usize,
    samples: usize,
    mut eval: F,
) -> Vec<Vec<f32>>
where
    F: FnMut(&[f32], &mut [f32]),
{
    let sub = substeps.max(1);
    let mut state = state0.to_vec();
    let mut sc = Scratch::new(state.len());
    let mut out = Vec::with_capacity(samples + 1);
    out.push(state.clone());
    'outer: for _ in 0..samples {
        for _ in 0..sub {
            rk4_into(&mut state, dt, &mut eval, &mut sc);
            if state.iter().any(|v| !v.is_finite()) {
                break 'outer;
            }
        }
        out.push(state.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// y' = −y, y(0) = 1  ⇒  y(t) = e^{−t}. RK4 should nail it.
    #[test]
    fn exponential_decay_matches_analytic() {
        let traj = integrate(&[1.0], 0.01, 100, |s, d| d[0] = -s[0]);
        assert_eq!(traj.len(), 101); // initial + 100 steps
        let got = traj.last().unwrap()[0];
        let want = (-1.0f32).exp(); // e^{-1} ≈ 0.367879
        assert!((got - want).abs() < 1e-4, "got {got}, want {want}");
    }

    /// Simple harmonic oscillator: x' = v, v' = −x, start (1, 0) ⇒ x = cos t,
    /// v = −sin t, period 2π, energy ½(x²+v²) = ½ conserved.
    #[test]
    fn sho_tracks_cosine_and_conserves_energy() {
        let dt = 0.001;
        let steps = (4.0 * PI / dt) as usize; // ~2 full periods
        let traj = integrate(&[1.0, 0.0], dt, steps, |s, d| {
            d[0] = s[1]; // x' = v
            d[1] = -s[0]; // v' = -x
        });
        let t_final = steps as f32 * dt;
        let last = traj.last().unwrap();
        let (x, v) = (last[0], last[1]);
        assert!((x - t_final.cos()).abs() < 1e-2, "x {x} vs cos {}", t_final.cos());
        assert!((v + t_final.sin()).abs() < 1e-2, "v {v} vs −sin {}", -t_final.sin());
        for s in &traj {
            let e = 0.5 * (s[0] * s[0] + s[1] * s[1]);
            assert!((e - 0.5).abs() < 1e-3, "energy drifted to {e}");
        }
    }

    /// **Time-dependent forcing via the clock-variable trick.** state = [y, t],
    /// y' = cos(t), t' = 1, y(0)=0 ⇒ y = sin(t). Checks driven systems integrate.
    #[test]
    fn time_dependent_forcing_via_clock_state() {
        let dt = 0.001;
        let steps = (PI / 2.0 / dt) as usize; // integrate to t = π/2
        let traj = integrate(&[0.0, 0.0], dt, steps, |s, d| {
            d[0] = s[1].cos(); // y' = cos(t)   (t is carried in s[1])
            d[1] = 1.0; // t' = 1  (the clock)
        });
        let y = traj.last().unwrap()[0];
        assert!((y - 1.0).abs() < 1e-3, "y {y} should approach sin(π/2)=1");
    }

    /// **Convergence:** halving dt must reduce the error (RK4 is 4th-order, so the
    /// error shrinks fast). We assert monotone improvement and small final error —
    /// robust in f32 (a strict 16× ratio is too brittle near f32 rounding).
    #[test]
    fn error_shrinks_as_dt_halves() {
        let exact = (-2.0f32).exp(); // solve y'=-y to t=2 ⇒ e^{-2}
        let err = |dt: f32| {
            let steps = (2.0 / dt).round() as usize;
            let traj = integrate(&[1.0], dt, steps, |s, d| d[0] = -s[0]);
            (traj.last().unwrap()[0] - exact).abs()
        };
        let (e2, e1, e05) = (err(0.2), err(0.1), err(0.05));
        assert!(e2 < 1e-2, "coarse error already too big: {e2}");
        assert!(e1 < e2, "halving dt (0.2→0.1) did not reduce error: {e1} vs {e2}");
        assert!(e05 < e1, "halving dt (0.1→0.05) did not reduce error: {e05} vs {e1}");
    }

    /// **Determinism:** identical inputs ⇒ bit-identical output. Core to manic's
    /// reproducible recordings.
    #[test]
    fn integration_is_deterministic() {
        let f = |s: &[f32], d: &mut [f32]| {
            d[0] = s[1];
            d[1] = -s[0] - 0.1 * s[1]; // damped oscillator
        };
        let a = integrate(&[1.0, 0.0], 0.01, 500, f);
        let b = integrate(&[1.0, 0.0], 0.01, 500, f);
        assert_eq!(a, b);
    }

    /// substeps = 1 reproduces [`integrate`] exactly; substeps > 1 gives the right
    /// snapshot count and still tracks the analytic solution at sample points.
    #[test]
    fn sampled_substeps_are_consistent() {
        let f = |s: &[f32], d: &mut [f32]| d[0] = -s[0];
        // substeps=1, 100 samples == integrate(100 steps)
        let via_integrate = integrate(&[1.0], 0.01, 100, f);
        let via_sampled = integrate_sampled(&[1.0], 0.01, 1, 100, f);
        assert_eq!(via_integrate, via_sampled);

        // substeps=4, 25 samples: same total time (100·0.01=1.0), coarser output
        let coarse = integrate_sampled(&[1.0], 0.01, 4, 25, f);
        assert_eq!(coarse.len(), 26); // samples + 1
        let want = (-1.0f32).exp();
        assert!((coarse.last().unwrap()[0] - want).abs() < 1e-4);
    }

    /// Euler is present and steps in the right direction (sanity, not accuracy).
    #[test]
    fn euler_step_advances() {
        let mut s = [1.0];
        euler_step(&mut s, 0.1, &mut |st, d| d[0] = -st[0]);
        assert!((s[0] - 0.9).abs() < 1e-6); // 1 + 0.1·(−1) = 0.9
    }

    /// y' = y², y(0)=1 blows up at t=1; integration should stop early on non-finite
    /// rather than return NaNs for the tail.
    #[test]
    fn blowup_stops_early() {
        let traj = integrate(&[1.0], 0.01, 500, |s, d| d[0] = s[0] * s[0]);
        assert!(traj.len() < 501, "should have stopped before all 500 steps");
        assert!(traj.iter().all(|s| s[0].is_finite()), "no non-finite snapshot retained");
    }
}
