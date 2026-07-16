//! physics — a new domain kit (Layer 1, in progress).
//!
//! A simulation is a **named state vector** evolving under its equations of
//! motion, pre-integrated once at build time (via [`crate::ode`]) into a sampled
//! trajectory the stateless timeline replays — deterministic, so recordings are
//! reproducible. This file establishes the declarative **sim model** (mirroring
//! the uniform specs in the crypto-tool RK4 goldmine) and the first named sim,
//! the pendulum. The drawable/replay wiring and the `pendulum(...)` builtin
//! (Layer-1 ctor) land next; nothing is registered into the vocabulary yet.

use crate::ode;
use crate::lang::lower::{Args, Registry};
use crate::primitives::{Entity, Shape};
use crate::scene::{PlaybackTrack, Scene, SimData};
use crate::style;
use crate::timeline::{Clip, Prop, TargetValue, TrackSpec, Value};
use crate::{easing::Easing, lang::diag::Error};
use macroquad::prelude::Vec2;

/// Kinetic + potential energy of a state — for energy bars and conservation checks.
#[derive(Debug, Clone, Copy)]
pub struct Energy {
    pub kinetic: f32,
    pub potential: f32,
}

impl Energy {
    pub fn total(&self) -> f32 {
        self.kinetic + self.potential
    }
}

/// A named physical system: an n-dimensional state vector evolving under `deriv`,
/// with derived quantities (energy) and a map from state → the body's world
/// position. Every named sim implements this; the ctor layer turns a `Sim` +
/// its integrated trajectory into tagged, animatable entities.
pub trait Sim {
    /// Initial state vector at t = 0.
    fn state0(&self) -> Vec<f32>;
    /// Fill `d[i]` with d(state[i])/dt — the equations of motion (the physics).
    fn deriv(&self, state: &[f32], d: &mut [f32]);
    /// Kinetic + potential energy for a state.
    fn energy(&self, state: &[f32]) -> Energy;
    /// The primary body's position in world (metre) coordinates, y-up.
    fn body(&self, state: &[f32]) -> (f32, f32);

    // ── optional view metadata (defaults ⇒ the sim doesn't offer that view) ──

    /// State-variable labels (θ, ω, …), one per state index.
    fn labels(&self) -> Vec<String> {
        Vec::new()
    }
    /// State indices `(x, y)` to plot against each other in the phase portrait.
    fn phase_xy(&self) -> Option<(usize, usize)> {
        None
    }
    /// The state index that acts as "position" (the potential-well x axis).
    fn pos_var(&self) -> Option<usize> {
        None
    }
    /// Sampled `(position, potential-energy)` describing the potential-well curve.
    fn well_curve(&self) -> Vec<(f32, f32)> {
        Vec::new()
    }
}

/// Pre-integrate a sim into a sampled trajectory: `samples + 1` state snapshots,
/// each advancing `substeps` RK4 steps of size `dt`. Deterministic. This is the
/// bridge between the sim model and [`crate::ode`] — and the same call the ctor
/// layer will make before turning the samples into a replayable animation.
pub fn simulate(sim: &dyn Sim, dt: f32, substeps: usize, samples: usize) -> Vec<Vec<f32>> {
    ode::integrate_sampled(&sim.state0(), dt, substeps, samples, |s, d| sim.deriv(s, d))
}

/// A simple / damped / driven pendulum. State = `[θ, ω, t]` — θ measured from the
/// downward vertical, ω its rate, and `t` a clock carried in the state so the
/// optional sinusoidal drive term stays inside the autonomous integrator:
///
/// θ″ = −(g/L)·sinθ − (b/mL²)·ω + (A/mL²)·cos(f·t)
///
/// Transcribed from the goldmine `sims/pendulum.js`.
pub struct Pendulum {
    pub g: f32,
    pub length: f32,
    pub mass: f32,
    pub damping: f32,
    pub drive_amp: f32,
    pub drive_freq: f32,
    pub theta0: f32,
}

impl Default for Pendulum {
    fn default() -> Self {
        Pendulum {
            g: 9.81,
            length: 1.0,
            mass: 1.0,
            damping: 0.0,
            drive_amp: 0.0,
            drive_freq: 0.667,
            theta0: std::f32::consts::FRAC_PI_3, // 60°
        }
    }
}

impl Pendulum {
    /// The small-angle period 2π·√(L/g) — the textbook reference the simulation
    /// should reproduce for small swings.
    pub fn small_angle_period(&self) -> f32 {
        std::f32::consts::TAU * (self.length / self.g).sqrt()
    }
}

impl Sim for Pendulum {
    fn state0(&self) -> Vec<f32> {
        vec![self.theta0, 0.0, 0.0]
    }

    fn deriv(&self, state: &[f32], d: &mut [f32]) {
        let (theta, omega, t) = (state[0], state[1], state[2]);
        let ml2 = self.mass * self.length * self.length;
        d[0] = omega;
        d[1] = -(self.g / self.length) * theta.sin() - (self.damping / ml2) * omega
            + (self.drive_amp / ml2) * (self.drive_freq * t).cos();
        d[2] = 1.0; // the clock always advances
    }

    fn energy(&self, state: &[f32]) -> Energy {
        let (theta, omega) = (state[0], state[1]);
        let kinetic = 0.5 * self.mass * (self.length * omega).powi(2);
        let potential = self.mass * self.g * self.length * (1.0 - theta.cos());
        Energy { kinetic, potential }
    }

    fn body(&self, state: &[f32]) -> (f32, f32) {
        let theta = state[0];
        (self.length * theta.sin(), -self.length * theta.cos())
    }

    // the pendulum offers every view: phase (θ vs ω) and the well U(θ)
    fn labels(&self) -> Vec<String> {
        vec!["θ".into(), "ω".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 1))
    }
    fn pos_var(&self) -> Option<usize> {
        Some(0)
    }
    fn well_curve(&self) -> Vec<(f32, f32)> {
        // U(θ) = m·g·L·(1 − cosθ), a bit past the release amplitude
        let amp = (self.theta0.abs() * 1.15).clamp(0.3, std::f32::consts::PI);
        let n = 64usize;
        (0..=n)
            .map(|i| {
                let th = -amp + 2.0 * amp * (i as f32 / n as f32);
                (th, self.mass * self.g * self.length * (1.0 - th.cos()))
            })
            .collect()
    }
}

impl Pendulum {
    /// World-frame velocity of the bob (m/s), y-up: v = L·ω·(cosθ, sinθ).
    pub fn body_velocity(&self, state: &[f32]) -> (f32, f32) {
        let (theta, omega) = (state[0], state[1]);
        (self.length * omega * theta.cos(), self.length * omega * theta.sin())
    }
}


/// A mass on a spring — a simple / damped harmonic oscillator. State = `[x, v, t]`
/// with `x` the displacement from equilibrium: x″ = −(k/m)·x − (b/m)·v. Its energy
/// well is a **parabola** U(x)=½kx² (vs the pendulum's cosine well) — the clean
/// contrast that shows the view baseline generalises. Motion is along the x axis.
pub struct Spring {
    pub k: f32,
    pub mass: f32,
    pub damping: f32,
    pub x0: f32,
}

impl Spring {
    /// World-frame velocity of the mass (m/s), horizontal: (v, 0).
    pub fn body_velocity(&self, state: &[f32]) -> (f32, f32) {
        (state[1], 0.0)
    }
    /// The undamped period 2π·√(m/k) — the SHM reference.
    pub fn period(&self) -> f32 {
        std::f32::consts::TAU * (self.mass / self.k).sqrt()
    }
}

impl Sim for Spring {
    fn state0(&self) -> Vec<f32> {
        vec![self.x0, 0.0, 0.0]
    }
    fn deriv(&self, state: &[f32], d: &mut [f32]) {
        let (x, v) = (state[0], state[1]);
        d[0] = v;
        d[1] = -(self.k / self.mass) * x - (self.damping / self.mass) * v;
        d[2] = 1.0;
    }
    fn energy(&self, state: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * self.mass * state[1] * state[1],
            potential: 0.5 * self.k * state[0] * state[0],
        }
    }
    fn body(&self, state: &[f32]) -> (f32, f32) {
        (state[0], 0.0)
    }
    fn labels(&self) -> Vec<String> {
        vec!["x".into(), "v".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 1))
    }
    fn pos_var(&self) -> Option<usize> {
        Some(0)
    }
    fn well_curve(&self) -> Vec<(f32, f32)> {
        // U(x) = ½·k·x² — a parabola, a bit past the amplitude
        let amp = (self.x0.abs() * 1.15).max(0.2);
        let n = 64usize;
        (0..=n)
            .map(|i| {
                let x = -amp + 2.0 * amp * (i as f32 / n as f32);
                (x, 0.5 * self.k * x * x)
            })
            .collect()
    }
}

/// A **double pendulum** — the classic chaotic system. State = `[θ₁, ω₁, θ₂, ω₂, t]`
/// (two arms hinged end-to-end). Its motion is deterministic but exquisitely
/// sensitive to initial conditions. Coupled equations of motion transcribed from
/// the goldmine `sims/double-pendulum.js`. It's 4-D in phase space, so it has no
/// single-variable potential well (the `well` view doesn't apply), but `phase`
/// (θ₁ vs θ₂), `timegraph`, and `energygraph` do.
pub struct DoublePendulum {
    pub g: f32,
    pub l1: f32,
    pub l2: f32,
    pub m1: f32,
    pub m2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl DoublePendulum {
    /// Inner bob (bob 1) world position, y-up.
    pub fn body1(&self, state: &[f32]) -> (f32, f32) {
        let th1 = state[0];
        (self.l1 * th1.sin(), -self.l1 * th1.cos())
    }
    /// Outer bob (bob 2) world velocity — the tip whose overlay arrow we draw.
    pub fn body_velocity(&self, state: &[f32]) -> (f32, f32) {
        let (th1, w1, th2, w2) = (state[0], state[1], state[2], state[3]);
        let vx = self.l1 * w1 * th1.cos() + self.l2 * w2 * th2.cos();
        let vy = self.l1 * w1 * th1.sin() + self.l2 * w2 * th2.sin();
        (vx, vy)
    }
}

impl Sim for DoublePendulum {
    fn state0(&self) -> Vec<f32> {
        vec![self.a1, 0.0, self.a2, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let (th1, dth1, th2, dth2) = (s[0], s[1], s[2], s[3]);
        let (g, l1, l2, m1, m2) = (self.g, self.l1, self.l2, self.m1, self.m2);
        let delta = th1 - th2;
        let (sin_d, sin2d) = (delta.sin(), (2.0 * delta).sin());
        let cos_d = delta.cos();
        let denom = 2.0 * m1 + m2 - m2 * (2.0 * delta).cos();

        d[0] = dth1;
        let num1 = -g * (2.0 * m1 + m2) * th1.sin()
            - g * m2 * (th1 - 2.0 * th2).sin()
            - 2.0 * m2 * dth2 * dth2 * l2 * sin_d
            - m2 * dth1 * dth1 * l1 * sin2d;
        d[1] = num1 / (l1 * denom);

        d[2] = dth2;
        let num2 = 2.0
            * sin_d
            * ((m1 + m2) * dth1 * dth1 * l1
                + g * (m1 + m2) * th1.cos()
                + m2 * dth2 * dth2 * l2 * cos_d);
        d[3] = num2 / (l2 * denom);

        d[4] = 1.0; // clock
    }
    fn energy(&self, s: &[f32]) -> Energy {
        let (th1, dth1, th2, dth2) = (s[0], s[1], s[2], s[3]);
        let (g, l1, l2, m1, m2) = (self.g, self.l1, self.l2, self.m1, self.m2);
        let (x1, y1) = (l1 * th1.sin(), -l1 * th1.cos());
        let y2 = y1 - l2 * th2.cos();
        let (vx1, vy1) = (l1 * dth1 * th1.cos(), l1 * dth1 * th1.sin());
        let (vx2, vy2) = (vx1 + l2 * dth2 * th2.cos(), vy1 + l2 * dth2 * th2.sin());
        let _ = x1;
        let kinetic = 0.5 * m1 * (vx1 * vx1 + vy1 * vy1) + 0.5 * m2 * (vx2 * vx2 + vy2 * vy2);
        // PE zero when both hang straight down
        let potential = g * m1 * (y1 + l1) + g * m2 * (y2 + l1 + l2);
        Energy { kinetic, potential }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        // the OUTER bob (bob 2) is the primary body (its trail is the chaos curve)
        let (th1, th2) = (s[0], s[2]);
        (self.l1 * th1.sin() + self.l2 * th2.sin(), -self.l1 * th1.cos() - self.l2 * th2.cos())
    }
    fn labels(&self) -> Vec<String> {
        vec!["θ₁".into(), "ω₁".into(), "θ₂".into(), "ω₂".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 2)) // θ₁ vs θ₂
    }
    // no single-variable well (4-D system) → `well` view unavailable
}

/// An **elastic (spring) pendulum**: a bob on a springy rod that both swings and
/// bounces. State = `[r, ṙ, θ, ω, t]`. Quasi-periodic/chaotic energy sloshing
/// between the radial and angular modes. From goldmine `sims/spring-pendulum.js`.
pub struct SpringPendulum {
    pub g: f32,
    pub k: f32,
    pub l0: f32,
    pub m: f32,
    pub damping: f32,
    pub a0: f32,
    pub stretch0: f32,
}
impl SpringPendulum {
    fn eq_r(&self) -> f32 {
        self.l0 + self.m * self.g / self.k
    }
    pub fn body_velocity(&self, s: &[f32]) -> (f32, f32) {
        let (r, vr, th, w) = (s[0], s[1], s[2], s[3]);
        (vr * th.sin() + r * w * th.cos(), -vr * th.cos() + r * w * th.sin())
    }
}
impl Sim for SpringPendulum {
    fn state0(&self) -> Vec<f32> {
        vec![self.eq_r() + self.stretch0, 0.0, self.a0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let (r, vr, th, w) = (s[0], s[1], s[2], s[3]);
        let rs = r.max(0.01);
        d[0] = vr;
        d[1] = rs * w * w - (self.k / self.m) * (rs - self.l0) + self.g * th.cos()
            - (self.damping / self.m) * vr;
        d[2] = w;
        d[3] = -(self.g / rs) * th.sin() - (2.0 / rs) * vr * w - (self.damping / self.m) * w;
        d[4] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        let (r, vr, th, w) = (s[0], s[1], s[2], s[3]);
        Energy {
            kinetic: 0.5 * self.m * (vr * vr + r * r * w * w),
            potential: 0.5 * self.k * (r - self.l0).powi(2) - self.m * self.g * r * th.cos(),
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[0] * s[2].sin(), -s[0] * s[2].cos())
    }
    fn labels(&self) -> Vec<String> {
        vec!["r".into(), "ṙ".into(), "θ".into(), "ω".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((2, 3))
    }
}

/// A **Kapitza pendulum**: a pivot vibrated fast enough (vertically) that the
/// *inverted* position becomes stable. State = `[θ, ω, t]`; the pivot motion
/// enters as a time-dependent effective gravity `g + A·sin(ω_v·t)`. Driven — energy
/// isn't conserved. From goldmine `sims/kapitza-pendulum.js`.
pub struct Kapitza {
    pub g: f32,
    pub l: f32,
    pub m: f32,
    pub damping: f32,
    pub vibe_amp: f32,
    pub vibe_freq: f32,
    pub a0: f32,
}
impl Kapitza {
    /// Pivot vertical displacement at time `t` (twice-integrated `A·sin(ω_v t)`).
    pub fn pivot_y(&self, t: f32) -> f32 {
        if self.vibe_freq <= 0.0 {
            0.0
        } else {
            -(self.vibe_amp / (self.vibe_freq * self.vibe_freq)) * (self.vibe_freq * t).sin()
        }
    }
    pub fn body_velocity(&self, s: &[f32]) -> (f32, f32) {
        (self.l * s[1] * s[0].cos(), self.l * s[1] * s[0].sin())
    }
}
impl Sim for Kapitza {
    fn state0(&self) -> Vec<f32> {
        vec![self.a0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let (th, w, t) = (s[0], s[1], s[2]);
        let g_eff = self.g + self.vibe_amp * (self.vibe_freq * t).sin();
        d[0] = w;
        d[1] = -(g_eff / self.l) * th.sin() - (self.damping / (self.m * self.l * self.l)) * w;
        d[2] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * self.m * (self.l * s[1]).powi(2),
            potential: self.m * self.g * self.l * (1.0 - s[0].cos()),
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (self.l * s[0].sin(), -self.l * s[0].cos())
    }
    fn labels(&self) -> Vec<String> {
        vec!["θ".into(), "ω".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 1))
    }
}

/// A **cart-pendulum**: a pendulum hinged on a spring-mounted cart that rolls on a
/// track — the classic control-theory system. State = `[x, θ, v, ω, t]`. Coupled
/// cart↔pendulum dynamics. From goldmine `sims/cart-pendulum.js`.
pub struct CartPendulum {
    pub g: f32,
    pub l: f32,
    pub mcart: f32,
    pub mbob: f32,
    pub k: f32,
    pub cart_damp: f32,
    pub bob_damp: f32,
    pub a0: f32,
}
impl CartPendulum {
    pub fn body_velocity(&self, s: &[f32]) -> (f32, f32) {
        let (th, v, w) = (s[1], s[2], s[3]);
        (v + self.l * w * th.cos(), self.l * w * th.sin())
    }
}
impl Sim for CartPendulum {
    fn state0(&self) -> Vec<f32> {
        vec![0.0, self.a0, 0.0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let (x, h, v, w) = (s[0], s[1], s[2], s[3]);
        let (bm, cm, ll, g, k) = (self.mbob, self.mcart, self.l, self.g, self.k);
        let (sin_h, cos_h) = (h.sin(), h.cos());
        let denom = cm + bm * sin_h * sin_h;
        d[0] = v;
        d[1] = w;
        d[2] = (bm * w * w * ll * sin_h + bm * g * sin_h * cos_h - k * x - self.cart_damp * v
            + self.bob_damp * w * cos_h / ll)
            / denom;
        d[3] = (-bm * w * w * ll * sin_h * cos_h + k * x * cos_h - (cm + bm) * g * sin_h
            + self.cart_damp * v * cos_h
            - (cm + bm) * self.bob_damp * w / (bm * ll))
            / (ll * denom);
        d[4] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        let (x, th, v, w) = (s[0], s[1], s[2], s[3]);
        let (bvx, bvy) = (v + self.l * w * th.cos(), self.l * w * th.sin());
        Energy {
            kinetic: 0.5 * self.mcart * v * v + 0.5 * self.mbob * (bvx * bvx + bvy * bvy),
            potential: 0.5 * self.k * x * x + self.mbob * self.g * self.l * (1.0 - th.cos()),
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[0] + self.l * s[1].sin(), -self.l * s[1].cos())
    }
    fn labels(&self) -> Vec<String> {
        vec!["x".into(), "θ".into(), "v".into(), "ω".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((1, 3))
    }
}

/// **Two chaotic pendulums** started a hair apart (Δθ ≈ 0.001 rad) — a
/// sensitive-dependence demo: identical driven-damped physics, yet they diverge
/// completely. State = `[θ_A, ω_A, θ_B, ω_B, t]`. From `sims/compare-pendulum.js`.
pub struct ComparePendulum {
    pub g: f32,
    pub l: f32,
    pub m: f32,
    pub damping: f32,
    pub drive_amp: f32,
    pub drive_freq: f32,
    pub a0: f32,
    pub delta: f32,
}
impl ComparePendulum {
    fn accel(&self, th: f32, w: f32, t: f32) -> f32 {
        let ml2 = self.m * self.l * self.l;
        -(self.g / self.l) * th.sin() - (self.damping / ml2) * w
            + (self.drive_amp / ml2) * (self.drive_freq * t).cos()
    }
    pub fn bob_a(&self, s: &[f32]) -> (f32, f32) {
        (self.l * s[0].sin(), -self.l * s[0].cos())
    }
    pub fn bob_b(&self, s: &[f32]) -> (f32, f32) {
        (self.l * s[2].sin(), -self.l * s[2].cos())
    }
}
impl Sim for ComparePendulum {
    fn state0(&self) -> Vec<f32> {
        vec![self.a0, 0.0, self.a0 + self.delta, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let t = s[4];
        d[0] = s[1];
        d[1] = self.accel(s[0], s[1], t);
        d[2] = s[3];
        d[3] = self.accel(s[2], s[3], t);
        d[4] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * self.m * (self.l * s[1]).powi(2),
            potential: self.m * self.g * self.l * (1.0 - s[0].cos()),
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        self.bob_a(s)
    }
    fn labels(&self) -> Vec<String> {
        vec!["θA".into(), "ωA".into(), "θB".into(), "ωB".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 2)) // θA vs θB — starts on the diagonal, then scatters
    }
}

/// A **vertical spring**: a mass hanging on a spring under gravity, bobbing about
/// its stretched equilibrium. State = `[d, ḋ, t]`, `d` the downward distance from
/// the anchor. Parabolic well (shifted by gravity). From `sims/vertical-spring.js`.
pub struct VerticalSpring {
    pub g: f32,
    pub k: f32,
    pub l0: f32,
    pub m: f32,
    pub damping: f32,
    pub stretch0: f32,
}
impl VerticalSpring {
    fn eq_d(&self) -> f32 {
        self.l0 + self.m * self.g / self.k
    }
    pub fn body_velocity(&self, s: &[f32]) -> (f32, f32) {
        (0.0, -s[1])
    }
}
impl Sim for VerticalSpring {
    fn state0(&self) -> Vec<f32> {
        vec![self.eq_d() + self.stretch0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        d[0] = s[1];
        d[1] = self.g - (self.k / self.m) * (s[0] - self.l0) - (self.damping / self.m) * s[1];
        d[2] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * self.m * s[1] * s[1],
            potential: 0.5 * self.k * (s[0] - self.l0).powi(2) - self.m * self.g * s[0],
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (0.0, -s[0])
    }
    fn labels(&self) -> Vec<String> {
        vec!["d".into(), "ḋ".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 1))
    }
    fn pos_var(&self) -> Option<usize> {
        Some(0)
    }
    fn well_curve(&self) -> Vec<(f32, f32)> {
        let (c, amp) = (self.eq_d(), (self.stretch0.abs() * 1.3).max(0.3));
        (0..=64)
            .map(|i| {
                let d = c - amp + 2.0 * amp * (i as f32 / 64.0);
                (d, 0.5 * self.k * (d - self.l0).powi(2) - self.m * self.g * d)
            })
            .collect()
    }
}

/// A **mass on a spring on an inclined plane**. State = `[s, ṡ, t]`, `s` the
/// position down the ramp. Gravity's along-ramp component shifts the equilibrium.
/// From `sims/spring-incline.js`.
pub struct SpringIncline {
    pub g: f32,
    pub k: f32,
    pub l0: f32,
    pub m: f32,
    pub damping: f32,
    pub angle: f32, // radians
    pub stretch0: f32,
}
impl SpringIncline {
    fn eq_s(&self) -> f32 {
        self.l0 + self.m * self.g * self.angle.sin() / self.k
    }
    pub fn body_velocity(&self, s: &[f32]) -> (f32, f32) {
        (s[1] * self.angle.cos(), -s[1] * self.angle.sin())
    }
}
impl Sim for SpringIncline {
    fn state0(&self) -> Vec<f32> {
        vec![self.eq_s() + self.stretch0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        d[0] = s[1];
        d[1] = (-self.k * (s[0] - self.l0) + self.m * self.g * self.angle.sin()
            - self.damping * s[1])
            / self.m;
        d[2] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * self.m * s[1] * s[1],
            potential: 0.5 * self.k * (s[0] - self.l0).powi(2)
                - self.m * self.g * s[0] * self.angle.sin(),
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[0] * self.angle.cos(), -s[0] * self.angle.sin())
    }
    fn labels(&self) -> Vec<String> {
        vec!["s".into(), "ṡ".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 1))
    }
    fn pos_var(&self) -> Option<usize> {
        Some(0)
    }
    fn well_curve(&self) -> Vec<(f32, f32)> {
        let (c, amp) = (self.eq_s(), (self.stretch0.abs() * 1.3).max(0.3));
        let sn = self.angle.sin();
        (0..=64)
            .map(|i| {
                let s = c - amp + 2.0 * amp * (i as f32 / 64.0);
                (s, 0.5 * self.k * (s - self.l0).powi(2) - self.m * self.g * s * sn)
            })
            .collect()
    }
}

/// A **bungee jump**: free-fall, then a one-sided elastic cord (only pulls, never
/// pushes) catches and bounces the jumper. State = `[y, ẏ, t]`, `y` the downward
/// distance fallen from the platform. From `sims/bungee.js`.
pub struct Bungee {
    pub g: f32,
    pub cord: f32,
    pub k: f32,
    pub m: f32,
    pub damping: f32,
}
impl Bungee {
    pub fn body_velocity(&self, s: &[f32]) -> (f32, f32) {
        (0.0, -s[1])
    }
    fn eq_y(&self) -> f32 {
        self.cord + self.m * self.g / self.k
    }
}
impl Sim for Bungee {
    fn state0(&self) -> Vec<f32> {
        vec![0.0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let stretch = (s[0] - self.cord).max(0.0);
        d[0] = s[1];
        d[1] = self.g - (self.k / self.m) * stretch - (self.damping / self.m) * s[1];
        d[2] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        let stretch = (s[0] - self.cord).max(0.0);
        Energy {
            kinetic: 0.5 * self.m * s[1] * s[1],
            potential: -self.m * self.g * s[0] + 0.5 * self.k * stretch * stretch,
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (0.0, -s[0])
    }
    fn labels(&self) -> Vec<String> {
        vec!["y".into(), "ẏ".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 1))
    }
    fn pos_var(&self) -> Option<usize> {
        Some(0)
    }
    fn well_curve(&self) -> Vec<(f32, f32)> {
        // asymmetric well: linear (free fall) then parabola (cord) — min at eq_y
        let hi = (self.eq_y() * 1.6).max(self.cord + 1.0);
        (0..=80)
            .map(|i| {
                let y = hi * (i as f32 / 80.0);
                let st = (y - self.cord).max(0.0);
                (y, -self.m * self.g * y + 0.5 * self.k * st * st)
            })
            .collect()
    }
}

/// A **driven (resonant) spring**: a mass on a spring pushed by a periodic force.
/// State = `[x, ẋ, t]`. When the drive frequency approaches the natural frequency
/// √(k/m), the amplitude grows large — resonance. Driven, so energy isn't
/// conserved. (The essence of `sims/resonance.js`, as a single time-domain sim.)
pub struct Resonance {
    pub k: f32,
    pub m: f32,
    pub damping: f32,
    pub drive_amp: f32,
    pub drive_freq: f32,
}
impl Resonance {
    pub fn natural_freq(&self) -> f32 {
        (self.k / self.m).sqrt()
    }
    pub fn body_velocity(&self, s: &[f32]) -> (f32, f32) {
        (s[1], 0.0)
    }
}
impl Sim for Resonance {
    fn state0(&self) -> Vec<f32> {
        vec![0.0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        d[0] = s[1];
        d[1] = -(self.k / self.m) * s[0] - (self.damping / self.m) * s[1]
            + (self.drive_amp / self.m) * (self.drive_freq * s[2]).cos();
        d[2] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * self.m * s[1] * s[1],
            potential: 0.5 * self.k * s[0] * s[0],
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[0], 0.0)
    }
    fn labels(&self) -> Vec<String> {
        vec!["x".into(), "ẋ".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 1))
    }
}

/// **Two masses coupled by springs** between two walls (three springs) — coupled
/// oscillators that exchange energy (beating) and reveal normal modes. State =
/// `[x₁, x₂, v₁, v₂, t]`. From `sims/double-spring.js`.
pub struct DoubleSpring {
    pub m1: f32,
    pub m2: f32,
    pub k: f32,
    pub r: f32,
    pub w1: f32,
    pub w2: f32,
    pub damping: f32,
    pub x1_0: f32,
    pub x2_0: f32,
}
impl Sim for DoubleSpring {
    fn state0(&self) -> Vec<f32> {
        vec![self.x1_0, self.x2_0, 0.0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let (x1, x2, v1, v2) = (s[0], s[1], s[2], s[3]);
        let l1 = (x1 - self.w1) - self.r;
        let l2 = (x2 - x1) - self.r;
        let l3 = (self.w2 - x2) - self.r;
        d[0] = v1;
        d[1] = v2;
        d[2] = (-self.k * l1 + self.k * l2 - self.damping * v1) / self.m1;
        d[3] = (-self.k * l2 + self.k * l3 - self.damping * v2) / self.m2;
        d[4] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        let (x1, x2, v1, v2) = (s[0], s[1], s[2], s[3]);
        let l1 = (x1 - self.w1) - self.r;
        let l2 = (x2 - x1) - self.r;
        let l3 = (self.w2 - x2) - self.r;
        Energy {
            kinetic: 0.5 * self.m1 * v1 * v1 + 0.5 * self.m2 * v2 * v2,
            potential: 0.5 * self.k * (l1 * l1 + l2 * l2 + l3 * l3),
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[0], 0.0)
    }
    fn labels(&self) -> Vec<String> {
        vec!["x₁".into(), "x₂".into(), "v₁".into(), "v₂".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 1)) // x₁ vs x₂ — modes show as diagonals
    }
}

/// **Springs in series vs parallel**: two identical masses, one on two springs in
/// series (softer), one in parallel (stiffer) — they oscillate at different rates.
/// State = `[d_s, ḋ_s, d_p, ḋ_p, t]`. From `sims/series-parallel-springs.js`.
pub struct SeriesParallel {
    pub g: f32,
    pub k1: f32,
    pub k2: f32,
    pub m: f32,
    pub damping: f32,
    pub l0: f32,
    pub stretch0: f32,
}
impl SeriesParallel {
    fn ks(&self) -> f32 {
        self.k1 * self.k2 / (self.k1 + self.k2)
    }
    fn kp(&self) -> f32 {
        self.k1 + self.k2
    }
    fn eq_s(&self) -> f32 {
        2.0 * self.l0 + self.m * self.g / self.ks()
    }
    fn eq_p(&self) -> f32 {
        self.l0 + self.m * self.g / self.kp()
    }
}
impl Sim for SeriesParallel {
    fn state0(&self) -> Vec<f32> {
        vec![self.eq_s() + self.stretch0, 0.0, self.eq_p() + self.stretch0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        d[0] = s[1];
        d[1] = self.g - (self.ks() / self.m) * (s[0] - 2.0 * self.l0) - (self.damping / self.m) * s[1];
        d[2] = s[3];
        d[3] = self.g - (self.kp() / self.m) * (s[2] - self.l0) - (self.damping / self.m) * s[3];
        d[4] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * self.m * (s[1] * s[1] + s[3] * s[3]),
            potential: 0.5 * self.ks() * (s[0] - 2.0 * self.l0).powi(2)
                + 0.5 * self.kp() * (s[2] - self.l0).powi(2)
                - self.m * self.g * (s[0] + s[2]),
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (0.0, -s[0])
    }
    fn labels(&self) -> Vec<String> {
        vec!["y_s".into(), "ẏ_s".into(), "y_p".into(), "ẏ_p".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 2)) // series vs parallel position
    }
}

/// Road profile for the car suspension: a speed bump, a washboard stretch, and a
/// pothole, repeating every 60 m (from `sims/car-suspension.js`).
fn road_height(x: f32, h: f32) -> f32 {
    use std::f32::consts::PI;
    let xm = x.rem_euclid(60.0);
    if (8.0..=10.0).contains(&xm) {
        h * (PI * (xm - 8.0) / 2.0).sin()
    } else if (20.0..=40.0).contains(&xm) {
        0.5 * h * (2.0 * PI * (xm - 20.0) / 5.0).sin()
    } else if (48.0..=50.0).contains(&xm) {
        -h * (PI * (xm - 48.0) / 2.0).sin()
    } else {
        0.0
    }
}
fn road_slope(x: f32, h: f32) -> f32 {
    use std::f32::consts::PI;
    let xm = x.rem_euclid(60.0);
    if (8.0..=10.0).contains(&xm) {
        h * (PI / 2.0) * (PI * (xm - 8.0) / 2.0).cos()
    } else if (20.0..=40.0).contains(&xm) {
        0.5 * h * (2.0 * PI / 5.0) * (2.0 * PI * (xm - 20.0) / 5.0).cos()
    } else if (48.0..=50.0).contains(&xm) {
        -h * (PI / 2.0) * (PI * (xm - 48.0) / 2.0).cos()
    } else {
        0.0
    }
}

/// A **quarter-car suspension**: a sprung mass on a spring + damper riding over a
/// road profile (speed bump, washboard, pothole). State = `[y, ẏ, x, t]` — `y` the
/// body's vertical displacement, `x` the distance travelled. From
/// `sims/car-suspension.js`.
pub struct CarSuspension {
    pub m: f32,
    pub k: f32,
    pub damping: f32,
    pub speed: f32,
    pub bump: f32,
}
impl Sim for CarSuspension {
    fn state0(&self) -> Vec<f32> {
        vec![0.0, 0.0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let yr = road_height(s[2], self.bump);
        let yr_dot = road_slope(s[2], self.bump) * self.speed;
        d[0] = s[1];
        d[1] = (-self.k * (s[0] - yr) - self.damping * (s[1] - yr_dot)) / self.m;
        d[2] = self.speed;
        d[3] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        let yr = road_height(s[2], self.bump);
        Energy {
            kinetic: 0.5 * self.m * s[1] * s[1],
            potential: 0.5 * self.k * (s[0] - yr).powi(2),
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (0.0, s[0])
    }
    fn labels(&self) -> Vec<String> {
        vec!["y".into(), "ẏ".into(), "x".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((2, 0)) // body response vs distance travelled
    }
}

// ── shared sim overlays (velocity arrow + KE/PE energy bars) ────────────────

/// Standard overlays reused by every sim: a velocity arrow riding the body and
/// KE/PE energy bars (normalised to `e_max`) with labels. Given the body's
/// per-frame screen positions, world velocities, and energies, it lays out the
/// entities and returns the `(KE,PE)` series (for `SimData`) + the overlay
/// playback tracks (to append). `overlay_tags` is applied to each overlay entity.
#[allow(clippy::too_many_arguments)]
fn add_overlays(
    s: &mut Scene,
    id: &str,
    center: Vec2,
    unit: f32,
    e_max: f32,
    body_pts: &[Vec2],
    vel_world: &[(f32, f32)],
    energies: &[Energy],
    overlay_tags: &[String],
) -> (Vec<(f32, f32)>, Vec<PlaybackTrack>) {
    let e_base = center.y + E_BASE_DY;
    let (ke_x, pe_x) = (center.x + KE_DX, center.x + PE_DX);
    let n = body_pts.len();
    let mut vel_tip = Vec::with_capacity(n);
    let mut ke_tip = Vec::with_capacity(n);
    let mut pe_tip = Vec::with_capacity(n);
    let mut energy_series = Vec::with_capacity(n);
    for i in 0..n {
        let b = body_pts[i];
        let (vx, vy) = vel_world[i];
        let e = energies[i];
        vel_tip.push(Vec2::new(b.x + vx * unit * VEL_SCALE, b.y - vy * unit * VEL_SCALE));
        ke_tip.push(Vec2::new(ke_x, e_base - (e.kinetic / e_max).clamp(0.0, 1.0) * BAR_MAX));
        pe_tip.push(Vec2::new(pe_x, e_base - (e.potential / e_max).clamp(0.0, 1.0) * BAR_MAX));
        energy_series.push((e.kinetic, e.potential));
    }

    let mut vel = Entity::new(format!("{id}.vel"), Shape::Arrow { to: vel_tip[0] }, body_pts[0], style::GOLD);
    vel.stroke.width = 3.0;
    vel.tags = overlay_tags.to_vec();
    s.add(vel);
    let mut ke = Entity::new(format!("{id}.ke"), Shape::Line { to: ke_tip[0] }, Vec2::new(ke_x, e_base), style::CYAN);
    ke.stroke.width = 12.0;
    ke.tags = overlay_tags.to_vec();
    s.add(ke);
    let mut pe = Entity::new(format!("{id}.pe"), Shape::Line { to: pe_tip[0] }, Vec2::new(pe_x, e_base), style::MAGENTA);
    pe.stroke.width = 12.0;
    pe.tags = overlay_tags.to_vec();
    s.add(pe);
    for (lid, lx, txt, col) in [
        (format!("{id}.kelbl"), ke_x, "KE", style::CYAN),
        (format!("{id}.pelbl"), pe_x, "PE", style::MAGENTA),
    ] {
        let mut lbl = Entity::new(lid, Shape::Text { content: txt.to_string(), size: 16.0 }, Vec2::new(lx, e_base + 18.0), col);
        lbl.tags = overlay_tags.to_vec();
        s.add(lbl);
    }

    let tracks = vec![
        PlaybackTrack { id: format!("{id}.vel"), prop: Prop::Pos, points: body_pts.to_vec() },
        PlaybackTrack { id: format!("{id}.vel"), prop: Prop::To, points: vel_tip },
        PlaybackTrack { id: format!("{id}.ke"), prop: Prop::To, points: ke_tip },
        PlaybackTrack { id: format!("{id}.pe"), prop: Prop::To, points: pe_tip },
    ];
    (energy_series, tracks)
}

// ── Layer-1 builtins ───────────────────────────────────────────────────────

/// Frames pre-simulated per pendulum (≈ `SAMPLES · SUB · DT` seconds of motion).
const SAMPLES: usize = 240;

/// Fraction of a view panel used for content (leaves a margin inside the frame).
const PANEL_MARGIN: f32 = 0.85;

// Standard sim-overlay layout (relative to the sim's `center`), shared by every sim.
const BAR_MAX: f32 = 160.0; // full-energy bar height, px
const E_BASE_DY: f32 = 230.0; // energy-bar baseline below center
const KE_DX: f32 = 210.0; // KE bar x offset from center
const PE_DX: f32 = 250.0; // PE bar x offset from center
const VEL_SCALE: f32 = 0.15; // velocity arrow px per (m/s · unit)

/// `pendulum(id, [center], [length], [angle0], [unit], [damping])` — a swinging
/// pendulum. Only `id` is required. `center` is the pivot in screen coords
/// (default `(640, 200)`, the top-centre of the default 16:9 stage — pass one for
/// other canvases); `length` in metres (default 1); `angle0` the release angle in
/// DEGREES from vertical (default 30); `unit` px-per-metre (default 150);
/// `damping` (default 0). Pre-simulates the motion with RK4 and lays out
/// `{id}.pivot`, `{id}.rod`, `{id}.bob`, the faint swing arc `{id}.path`, plus
/// overlays (tagged `{id}.overlays`): the velocity arrow `{id}.vel` and the KE/PE
/// energy bars `{id}.ke`/`{id}.pe` with labels. Everything is tagged bare `{id}` +
/// `{id}.parts`, so `show(id)`/`draw(id)` address the whole thing (and
/// `hidden(id.overlays)` drops the readouts). Animate it with `swing(id, [dur])`.
fn c_pendulum(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    // `center` is optional: present at index 1 when given, else default.
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 200.0) };
    let length = a.opt_num(2)?.unwrap_or(1.0).max(0.05);
    let angle0 = a.opt_num(3)?.unwrap_or(30.0).to_radians();
    let unit = a.opt_num(4)?.unwrap_or(150.0);
    let damping = a.opt_num(5)?.unwrap_or(0.0).max(0.0);

    let p = Pendulum {
        g: 9.81,
        length,
        mass: 1.0,
        damping,
        drive_amp: 0.0,
        drive_freq: 0.0,
        theta0: angle0,
    };

    // Fine sub-stepping for accuracy; 240 frames of ≈0.025 s = ~6 s of motion.
    let (sim_dt, substeps) = (0.005f32, 5usize);
    let states = simulate(&p, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);

    // per-frame body positions / velocities / energies (normalised to the initial
    // total energy so a damped swing visibly loses energy)
    let e_max = (p.mass * p.g * p.length * (1.0 - angle0.cos())).max(1e-3);
    let bob_pts: Vec<Vec2> = states.iter().map(|st| to_screen(p.body(st))).collect();
    let vel_world: Vec<(f32, f32)> = states.iter().map(|st| p.body_velocity(st)).collect();
    let energies: Vec<Energy> = states.iter().map(|st| p.energy(st)).collect();
    let bob0 = bob_pts[0];

    let parts = format!("{id}.parts");
    let core_tags = || vec![id.clone(), parts.clone()];
    let overlay_tags = vec![id.clone(), parts.clone(), format!("{id}.overlays")];

    // faint arc the bob will trace
    let mut arc = Entity::new(
        format!("{id}.path"),
        Shape::Polyline { pts: bob_pts.clone() },
        Vec2::ZERO,
        style::DIM,
    );
    arc.stroke.width = 2.0;
    arc.opacity = 0.35;
    arc.tags = core_tags();
    s.add(arc);

    // rod: pivot → bob
    let mut rod = Entity::new(format!("{id}.rod"), Shape::Line { to: bob0 }, center, style::FG);
    rod.stroke.width = 3.0;
    rod.tags = core_tags();
    s.add(rod);

    // pivot dot
    let mut pivot = Entity::new(format!("{id}.pivot"), Shape::Circle { r: 6.0 }, center, style::DIM);
    pivot.stroke.fill = true;
    pivot.stroke.outline = false;
    pivot.tags = core_tags();
    s.add(pivot);

    // bob
    let mut bob = Entity::new(format!("{id}.bob"), Shape::Circle { r: 16.0 }, bob0, style::CYAN);
    bob.stroke.fill = true;
    bob.stroke.outline = false;
    bob.tags = core_tags();
    s.add(bob);

    // velocity arrow + KE/PE energy bars (shared across all sims)
    let (energy_series, overlay_tracks) =
        add_overlays(s, &id, center, unit, e_max, &bob_pts, &vel_world, &energies, &overlay_tags);

    let mut playback = vec![
        PlaybackTrack { id: format!("{id}.bob"), prop: Prop::Pos, points: bob_pts.clone() },
        PlaybackTrack { id: format!("{id}.rod"), prop: Prop::To, points: bob_pts },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: p.labels(),
            phase_xy: p.phase_xy(),
            pos_var: p.pos_var(),
            well: p.well_curve(),
            energy: energy_series,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `run(id, [dur])` (alias `swing`) — replay a sim's pre-simulated motion over
/// `dur` seconds (default 6): every part + view marker animates along it. A
/// keyframed replay (one segment per frame) of every stored [`PlaybackTrack`].
fn v_play(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let sim = s.sims.get(&id).ok_or_else(|| {
        Error::new(format!("no sim `{id}` — call `pendulum(...)` (or another sim) first"), a.span_of(0))
    })?;
    let frames = sim.playback.iter().map(|p| p.points.len()).max().unwrap_or(0);
    if frames < 2 {
        return Err(Error::new(format!("`{id}` has no motion to swing"), a.span_of(0)));
    }
    let dur = a.opt_num(1)?.unwrap_or(6.0).max(0.1);
    let frame = dur / (frames - 1) as f32;
    let mut tracks = Vec::new();
    for pt in &sim.playback {
        for k in 1..pt.points.len() {
            tracks.push(TrackSpec {
                id: pt.id.clone(),
                prop: pt.prop,
                target: TargetValue::Abs(Value::V(pt.points[k])),
                start: (k - 1) as f32 * frame,
                dur: frame,
                easing: Easing::Linear,
            });
        }
    }
    Ok(Clip { tracks, events: vec![], dur })
}

/// `spring(id, [center], [stiffness], [x0], [unit], [damping])` — a mass on a
/// spring. Only `id` is required. `center` is the equilibrium position (default
/// `(640, 320)`); `stiffness` k (default 10); `x0` the initial displacement in
/// metres (default 1.3); `unit` px-per-metre (default 110); `damping` (default 0).
/// Lays out `{id}.wall`, `{id}.spring`, `{id}.mass`, the range `{id}.path`, plus
/// the shared velocity arrow + energy bars — and stores the data for all four
/// views (its well is a parabola). Animate with `run(id, [dur])`.
fn c_spring(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 320.0) };
    let k = a.opt_num(2)?.unwrap_or(10.0).max(0.1);
    let x0 = a.opt_num(3)?.unwrap_or(1.3);
    let unit = a.opt_num(4)?.unwrap_or(110.0);
    let damping = a.opt_num(5)?.unwrap_or(0.0).max(0.0);

    let sp = Spring { k, mass: 1.0, damping, x0 };
    let (sim_dt, substeps) = (0.005f32, 5usize);
    let states = simulate(&sp, sim_dt, substeps, SAMPLES);
    // mass moves horizontally: world (x, 0) → screen at center.y
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let e_max = (0.5 * k * x0 * x0).max(1e-3);
    let mass_pts: Vec<Vec2> = states.iter().map(|st| to_screen(sp.body(st))).collect();
    let vel_world: Vec<(f32, f32)> = states.iter().map(|st| sp.body_velocity(st)).collect();
    let energies: Vec<Energy> = states.iter().map(|st| sp.energy(st)).collect();
    let mass0 = mass_pts[0];
    let wall_x = center.x - x0.abs() * unit - 60.0;

    let parts = format!("{id}.parts");
    let core_tags = || vec![id.clone(), parts.clone()];
    let overlay_tags = vec![id.clone(), parts.clone(), format!("{id}.overlays")];

    // fixed wall
    let mut wall = Entity::new(
        format!("{id}.wall"),
        Shape::Line { to: Vec2::new(wall_x, center.y + 34.0) },
        Vec2::new(wall_x, center.y - 34.0),
        style::DIM,
    );
    wall.stroke.width = 5.0;
    wall.tags = core_tags();
    s.add(wall);
    // spring: wall → mass, drawn as a real coil that stretches with the motion
    let mut spring = Entity::new(
        format!("{id}.spring"),
        Shape::Coil { to: mass0, turns: 12 },
        Vec2::new(wall_x, center.y),
        style::LIME,
    );
    spring.stroke.width = 3.0;
    spring.tags = core_tags();
    s.add(spring);
    // the mass block
    let mut mass = Entity::new(format!("{id}.mass"), Shape::Rect { w: 40.0, h: 40.0 }, mass0, style::CYAN);
    mass.stroke.fill = true;
    mass.stroke.outline = false;
    mass.tags = core_tags();
    s.add(mass);
    // faint range-of-motion path
    let mut path = Entity::new(format!("{id}.path"), Shape::Polyline { pts: mass_pts.clone() }, Vec2::ZERO, style::DIM);
    path.stroke.width = 2.0;
    path.opacity = 0.3;
    path.tags = core_tags();
    s.add(path);

    let (energy_series, overlay_tracks) =
        add_overlays(s, &id, center, unit, e_max, &mass_pts, &vel_world, &energies, &overlay_tags);

    let mut playback = vec![
        PlaybackTrack { id: format!("{id}.mass"), prop: Prop::Pos, points: mass_pts.clone() },
        PlaybackTrack { id: format!("{id}.spring"), prop: Prop::To, points: mass_pts },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: sp.labels(),
            phase_xy: sp.phase_xy(),
            pos_var: sp.pos_var(),
            well: sp.well_curve(),
            energy: energy_series,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `doublependulum(id, [center], [angle1], [angle2], [unit])` — the chaotic double
/// pendulum. Only `id` is required. `center` is the pivot (default `(640, 240)`);
/// `angle1`/`angle2` the release angles in DEGREES from vertical (default 90 each);
/// `unit` px-per-metre (default 110). Lays out `{id}.pivot`, `{id}.rod1`,
/// `{id}.bob1`, `{id}.rod2`, `{id}.bob2`, and the outer bob's chaotic trail
/// `{id}.path`, plus the shared velocity arrow + energy bars. Supports `phase`
/// (θ₁ vs θ₂), `timegraph`, `energygraph` (not `well` — it's a 4-D system).
/// Animate with `run(id, [dur])`.
fn c_doublependulum(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 240.0) };
    let a1 = a.opt_num(2)?.unwrap_or(90.0).to_radians();
    let a2 = a.opt_num(3)?.unwrap_or(90.0).to_radians();
    let unit = a.opt_num(4)?.unwrap_or(110.0);

    let dp = DoublePendulum { g: 9.8, l1: 1.0, l2: 1.0, m1: 2.0, m2: 2.0, a1, a2 };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&dp, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let e_max = dp.energy(&states[0]).total().max(1e-3);

    let bob1_pts: Vec<Vec2> = states.iter().map(|st| to_screen(dp.body1(st))).collect();
    let bob2_pts: Vec<Vec2> = states.iter().map(|st| to_screen(dp.body(st))).collect();
    let vel_world: Vec<(f32, f32)> = states.iter().map(|st| dp.body_velocity(st)).collect();
    let energies: Vec<Energy> = states.iter().map(|st| dp.energy(st)).collect();
    let (bob1_0, bob2_0) = (bob1_pts[0], bob2_pts[0]);

    let parts = format!("{id}.parts");
    let core_tags = || vec![id.clone(), parts.clone()];
    let overlay_tags = vec![id.clone(), parts.clone(), format!("{id}.overlays")];

    // outer bob's chaotic trail (traced during `run`)
    let mut trail = Entity::new(format!("{id}.path"), Shape::Polyline { pts: bob2_pts.clone() }, Vec2::ZERO, style::MAGENTA);
    trail.stroke.width = 2.0;
    trail.opacity = 0.6;
    trail.tags = core_tags();
    s.add(trail);
    // pivot
    let mut pivot = Entity::new(format!("{id}.pivot"), Shape::Circle { r: 6.0 }, center, style::DIM);
    pivot.stroke.fill = true;
    pivot.stroke.outline = false;
    pivot.tags = core_tags();
    s.add(pivot);
    // arm 1: pivot → bob1
    let mut rod1 = Entity::new(format!("{id}.rod1"), Shape::Line { to: bob1_0 }, center, style::FG);
    rod1.stroke.width = 3.0;
    rod1.tags = core_tags();
    s.add(rod1);
    // arm 2: bob1 → bob2 (both ends move)
    let mut rod2 = Entity::new(format!("{id}.rod2"), Shape::Line { to: bob2_0 }, bob1_0, style::FG);
    rod2.stroke.width = 3.0;
    rod2.tags = core_tags();
    s.add(rod2);
    // bobs
    let mut bob1 = Entity::new(format!("{id}.bob1"), Shape::Circle { r: 12.0 }, bob1_0, style::CYAN);
    bob1.stroke.fill = true;
    bob1.stroke.outline = false;
    bob1.tags = core_tags();
    s.add(bob1);
    let mut bob2 = Entity::new(format!("{id}.bob2"), Shape::Circle { r: 14.0 }, bob2_0, style::LIME);
    bob2.stroke.fill = true;
    bob2.stroke.outline = false;
    bob2.tags = core_tags();
    s.add(bob2);

    let (energy_series, overlay_tracks) =
        add_overlays(s, &id, center, unit, e_max, &bob2_pts, &vel_world, &energies, &overlay_tags);

    let mut playback = vec![
        PlaybackTrack { id: format!("{id}.bob1"), prop: Prop::Pos, points: bob1_pts.clone() },
        PlaybackTrack { id: format!("{id}.bob2"), prop: Prop::Pos, points: bob2_pts.clone() },
        PlaybackTrack { id: format!("{id}.rod1"), prop: Prop::To, points: bob1_pts.clone() },
        PlaybackTrack { id: format!("{id}.rod2"), prop: Prop::Pos, points: bob1_pts },
        PlaybackTrack { id: format!("{id}.rod2"), prop: Prop::To, points: bob2_pts },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: dp.labels(),
            phase_xy: dp.phase_xy(),
            pos_var: dp.pos_var(),
            well: dp.well_curve(),
            energy: energy_series,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// Peak of `max(KE, PE≥0)` over a trajectory — the energy-bar normaliser (robust
/// to a negative gravitational-PE reference, e.g. the elastic pendulum).
fn peak_energy(energies: &[Energy]) -> f32 {
    energies
        .iter()
        .map(|e| e.kinetic.max(e.potential.max(0.0)))
        .fold(1e-3, f32::max)
}

/// `springpendulum(id, [center], [angle0], [stretch0], [unit], [damping])` — an
/// elastic pendulum (swings AND bounces), drawn with a real stretching coil.
fn c_springpendulum(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 240.0) };
    let a0 = a.opt_num(2)?.unwrap_or(30.0).to_radians();
    let stretch0 = a.opt_num(3)?.unwrap_or(0.3);
    let unit = a.opt_num(4)?.unwrap_or(110.0);
    let damping = a.opt_num(5)?.unwrap_or(0.1).max(0.0);
    let sp = SpringPendulum { g: 9.81, k: 40.0, l0: 1.5, m: 1.0, damping, a0, stretch0 };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&sp, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let bob_pts: Vec<Vec2> = states.iter().map(|st| to_screen(sp.body(st))).collect();
    let vel_world: Vec<(f32, f32)> = states.iter().map(|st| sp.body_velocity(st)).collect();
    let energies: Vec<Energy> = states.iter().map(|st| sp.energy(st)).collect();
    let e_max = peak_energy(&energies);
    let bob0 = bob_pts[0];
    let parts = format!("{id}.parts");
    let core_tags = || vec![id.clone(), parts.clone()];
    let overlay_tags = vec![id.clone(), parts.clone(), format!("{id}.overlays")];
    let mut trail = Entity::new(format!("{id}.path"), Shape::Polyline { pts: bob_pts.clone() }, Vec2::ZERO, style::DIM);
    trail.stroke.width = 2.0;
    trail.opacity = 0.35;
    trail.tags = core_tags();
    s.add(trail);
    let mut spring = Entity::new(format!("{id}.spring"), Shape::Coil { to: bob0, turns: 10 }, center, style::LIME);
    spring.stroke.width = 3.0;
    spring.tags = core_tags();
    s.add(spring);
    let mut pivot = Entity::new(format!("{id}.pivot"), Shape::Circle { r: 6.0 }, center, style::DIM);
    pivot.stroke.fill = true;
    pivot.stroke.outline = false;
    pivot.tags = core_tags();
    s.add(pivot);
    let mut bob = Entity::new(format!("{id}.bob"), Shape::Circle { r: 15.0 }, bob0, style::CYAN);
    bob.stroke.fill = true;
    bob.stroke.outline = false;
    bob.tags = core_tags();
    s.add(bob);
    let (energy_series, overlay_tracks) =
        add_overlays(s, &id, center, unit, e_max, &bob_pts, &vel_world, &energies, &overlay_tags);
    let mut playback = vec![
        PlaybackTrack { id: format!("{id}.bob"), prop: Prop::Pos, points: bob_pts.clone() },
        PlaybackTrack { id: format!("{id}.spring"), prop: Prop::To, points: bob_pts },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(id, SimData {
        playback, labels: sp.labels(), phase_xy: sp.phase_xy(), pos_var: sp.pos_var(),
        well: sp.well_curve(), energy: energy_series, dt: sim_dt * substeps as f32, states,
    });
    Ok(())
}

/// `kapitza(id, [center], [angle0], [vibeamp], [unit])` — a pendulum whose pivot
/// vibrates fast enough that the INVERTED position becomes stable. `angle0` in
/// degrees (default 165, near inverted), `vibeamp` the drive strength (default 220).
fn c_kapitza(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 400.0) };
    let a0 = a.opt_num(2)?.unwrap_or(165.0).to_radians();
    let vibe_amp = a.opt_num(3)?.unwrap_or(220.0).max(0.0);
    let unit = a.opt_num(4)?.unwrap_or(150.0);
    let kp = Kapitza { g: 9.81, l: 1.0, m: 1.0, damping: 0.1, vibe_amp, vibe_freq: 30.0, a0 };
    let (sim_dt, substeps) = (0.002f32, 12usize); // fast vibration ⇒ fine dt
    let states = simulate(&kp, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let pivot_pts: Vec<Vec2> = states.iter().map(|st| to_screen((0.0, kp.pivot_y(st[2])))).collect();
    let bob_pts: Vec<Vec2> = states
        .iter()
        .map(|st| to_screen((kp.l * st[0].sin(), kp.pivot_y(st[2]) - kp.l * st[0].cos())))
        .collect();
    let vel_world: Vec<(f32, f32)> = states.iter().map(|st| kp.body_velocity(st)).collect();
    let energies: Vec<Energy> = states.iter().map(|st| kp.energy(st)).collect();
    let e_max = peak_energy(&energies);
    let (pivot0, bob0) = (pivot_pts[0], bob_pts[0]);
    let parts = format!("{id}.parts");
    let core_tags = || vec![id.clone(), parts.clone()];
    let overlay_tags = vec![id.clone(), parts.clone(), format!("{id}.overlays")];
    let mut trail = Entity::new(format!("{id}.path"), Shape::Polyline { pts: bob_pts.clone() }, Vec2::ZERO, style::DIM);
    trail.stroke.width = 2.0;
    trail.opacity = 0.3;
    trail.tags = core_tags();
    s.add(trail);
    let mut rod = Entity::new(format!("{id}.rod"), Shape::Line { to: bob0 }, pivot0, style::FG);
    rod.stroke.width = 3.0;
    rod.tags = core_tags();
    s.add(rod);
    let mut pivot = Entity::new(format!("{id}.pivot"), Shape::Circle { r: 7.0 }, pivot0, style::GOLD);
    pivot.stroke.fill = true;
    pivot.stroke.outline = false;
    pivot.tags = core_tags();
    s.add(pivot);
    let mut bob = Entity::new(format!("{id}.bob"), Shape::Circle { r: 15.0 }, bob0, style::CYAN);
    bob.stroke.fill = true;
    bob.stroke.outline = false;
    bob.tags = core_tags();
    s.add(bob);
    let (energy_series, overlay_tracks) =
        add_overlays(s, &id, center, unit, e_max, &bob_pts, &vel_world, &energies, &overlay_tags);
    let mut playback = vec![
        PlaybackTrack { id: format!("{id}.pivot"), prop: Prop::Pos, points: pivot_pts.clone() },
        PlaybackTrack { id: format!("{id}.rod"), prop: Prop::Pos, points: pivot_pts },
        PlaybackTrack { id: format!("{id}.rod"), prop: Prop::To, points: bob_pts.clone() },
        PlaybackTrack { id: format!("{id}.bob"), prop: Prop::Pos, points: bob_pts },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(id, SimData {
        playback, labels: kp.labels(), phase_xy: kp.phase_xy(), pos_var: kp.pos_var(),
        well: kp.well_curve(), energy: energy_series, dt: sim_dt * substeps as f32, states,
    });
    Ok(())
}

/// `cartpendulum(id, [center], [angle0], [unit])` — a pendulum on a spring-mounted
/// cart rolling on a track (the classic control system). `angle0` in degrees.
fn c_cartpendulum(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 340.0) };
    let a0 = a.opt_num(2)?.unwrap_or(45.0).to_radians();
    let unit = a.opt_num(3)?.unwrap_or(110.0);
    let cp = CartPendulum { g: 9.8, l: 1.0, mcart: 1.0, mbob: 1.0, k: 6.0, cart_damp: 0.0, bob_damp: 0.0, a0 };
    let (sim_dt, substeps) = (0.005f32, 5usize);
    let states = simulate(&cp, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let cart_pts: Vec<Vec2> = states.iter().map(|st| to_screen((st[0], 0.0))).collect();
    let bob_pts: Vec<Vec2> = states.iter().map(|st| to_screen(cp.body(st))).collect();
    let vel_world: Vec<(f32, f32)> = states.iter().map(|st| cp.body_velocity(st)).collect();
    let energies: Vec<Energy> = states.iter().map(|st| cp.energy(st)).collect();
    let e_max = peak_energy(&energies);
    let (cart0, bob0) = (cart_pts[0], bob_pts[0]);
    let wall_x = center.x - 230.0;
    let parts = format!("{id}.parts");
    let core_tags = || vec![id.clone(), parts.clone()];
    let overlay_tags = vec![id.clone(), parts.clone(), format!("{id}.overlays")];
    let mut track = Entity::new(format!("{id}.track"), Shape::Line { to: Vec2::new(center.x + 230.0, center.y + 22.0) }, Vec2::new(wall_x, center.y + 22.0), style::DIM);
    track.stroke.width = 2.0;
    track.opacity = 0.5;
    track.tags = core_tags();
    s.add(track);
    let mut wall = Entity::new(format!("{id}.wall"), Shape::Line { to: Vec2::new(wall_x, center.y + 22.0) }, Vec2::new(wall_x, center.y - 34.0), style::DIM);
    wall.stroke.width = 5.0;
    wall.tags = core_tags();
    s.add(wall);
    let mut spring = Entity::new(format!("{id}.spring"), Shape::Coil { to: cart0, turns: 10 }, Vec2::new(wall_x, center.y), style::LIME);
    spring.stroke.width = 3.0;
    spring.tags = core_tags();
    s.add(spring);
    let mut cart = Entity::new(format!("{id}.cart"), Shape::Rect { w: 52.0, h: 34.0 }, cart0, style::PANEL);
    cart.stroke.fill = true;
    cart.stroke.outline = true;
    cart.tags = core_tags();
    s.add(cart);
    let mut rod = Entity::new(format!("{id}.rod"), Shape::Line { to: bob0 }, cart0, style::FG);
    rod.stroke.width = 3.0;
    rod.tags = core_tags();
    s.add(rod);
    let mut bob = Entity::new(format!("{id}.bob"), Shape::Circle { r: 14.0 }, bob0, style::CYAN);
    bob.stroke.fill = true;
    bob.stroke.outline = false;
    bob.tags = core_tags();
    s.add(bob);
    let (energy_series, overlay_tracks) =
        add_overlays(s, &id, center, unit, e_max, &bob_pts, &vel_world, &energies, &overlay_tags);
    let mut playback = vec![
        PlaybackTrack { id: format!("{id}.cart"), prop: Prop::Pos, points: cart_pts.clone() },
        PlaybackTrack { id: format!("{id}.spring"), prop: Prop::To, points: cart_pts.clone() },
        PlaybackTrack { id: format!("{id}.rod"), prop: Prop::Pos, points: cart_pts },
        PlaybackTrack { id: format!("{id}.rod"), prop: Prop::To, points: bob_pts.clone() },
        PlaybackTrack { id: format!("{id}.bob"), prop: Prop::Pos, points: bob_pts },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(id, SimData {
        playback, labels: cp.labels(), phase_xy: cp.phase_xy(), pos_var: cp.pos_var(),
        well: cp.well_curve(), energy: energy_series, dt: sim_dt * substeps as f32, states,
    });
    Ok(())
}

/// `comparependulum(id, [center], [angle0], [unit])` — two driven-damped pendulums
/// started ≈0.001 rad apart: sensitive dependence, they diverge completely.
fn c_comparependulum(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 240.0) };
    let a0 = a.opt_num(2)?.unwrap_or(10.0).to_radians();
    let unit = a.opt_num(3)?.unwrap_or(130.0);
    let cmp = ComparePendulum { g: 9.81, l: 1.0, m: 1.0, damping: 0.5, drive_amp: 10.0, drive_freq: 0.667, a0, delta: 0.001 };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&cmp, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let a_pts: Vec<Vec2> = states.iter().map(|st| to_screen(cmp.bob_a(st))).collect();
    let b_pts: Vec<Vec2> = states.iter().map(|st| to_screen(cmp.bob_b(st))).collect();
    let energy_series: Vec<(f32, f32)> = states.iter().map(|st| { let e = cmp.energy(st); (e.kinetic, e.potential) }).collect();
    let (a0p, b0p) = (a_pts[0], b_pts[0]);
    let parts = format!("{id}.parts");
    let core_tags = || vec![id.clone(), parts.clone()];
    let mut ta = Entity::new(format!("{id}.pathA"), Shape::Polyline { pts: a_pts.clone() }, Vec2::ZERO, style::CYAN);
    ta.stroke.width = 2.0;
    ta.opacity = 0.25;
    ta.tags = core_tags();
    s.add(ta);
    let mut tb = Entity::new(format!("{id}.pathB"), Shape::Polyline { pts: b_pts.clone() }, Vec2::ZERO, style::MAGENTA);
    tb.stroke.width = 2.0;
    tb.opacity = 0.25;
    tb.tags = core_tags();
    s.add(tb);
    let mut pivot = Entity::new(format!("{id}.pivot"), Shape::Circle { r: 6.0 }, center, style::DIM);
    pivot.stroke.fill = true;
    pivot.stroke.outline = false;
    pivot.tags = core_tags();
    s.add(pivot);
    let mut rod_a = Entity::new(format!("{id}.rodA"), Shape::Line { to: a0p }, center, style::CYAN);
    rod_a.stroke.width = 3.0;
    rod_a.tags = core_tags();
    s.add(rod_a);
    let mut bob_a = Entity::new(format!("{id}.bobA"), Shape::Circle { r: 13.0 }, a0p, style::CYAN);
    bob_a.stroke.fill = true;
    bob_a.stroke.outline = false;
    bob_a.tags = core_tags();
    s.add(bob_a);
    let mut rod_b = Entity::new(format!("{id}.rodB"), Shape::Line { to: b0p }, center, style::MAGENTA);
    rod_b.stroke.width = 3.0;
    rod_b.tags = core_tags();
    s.add(rod_b);
    let mut bob_b = Entity::new(format!("{id}.bobB"), Shape::Circle { r: 13.0 }, b0p, style::MAGENTA);
    bob_b.stroke.fill = true;
    bob_b.stroke.outline = false;
    bob_b.tags = core_tags();
    s.add(bob_b);
    let playback = vec![
        PlaybackTrack { id: format!("{id}.rodA"), prop: Prop::To, points: a_pts.clone() },
        PlaybackTrack { id: format!("{id}.bobA"), prop: Prop::Pos, points: a_pts },
        PlaybackTrack { id: format!("{id}.rodB"), prop: Prop::To, points: b_pts.clone() },
        PlaybackTrack { id: format!("{id}.bobB"), prop: Prop::Pos, points: b_pts },
    ];
    s.sims.insert(id, SimData {
        playback, labels: cmp.labels(), phase_xy: cmp.phase_xy(), pos_var: cmp.pos_var(),
        well: cmp.well_curve(), energy: energy_series, dt: sim_dt * substeps as f32, states,
    });
    Ok(())
}

/// `verticalspring(id, [center], [stretch0], [unit], [damping])` — a mass bobbing
/// on a vertical spring under gravity (coil drawn from an anchor above).
fn c_verticalspring(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 170.0) };
    let stretch0 = a.opt_num(2)?.unwrap_or(0.6);
    let unit = a.opt_num(3)?.unwrap_or(120.0);
    let damping = a.opt_num(4)?.unwrap_or(0.2).max(0.0);
    let vs = VerticalSpring { g: 9.81, k: 20.0, l0: 1.0, m: 1.0, damping, stretch0 };
    sim_spring_like(s, &id, center, unit, &vs, |st| vs.body_velocity(st), true)
}

/// `springincline(id, [center], [angle], [unit], [damping])` — a mass on a spring
/// on an inclined plane (`angle` in degrees). Coil + bob run down the ramp.
fn c_springincline(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(430.0, 190.0) };
    let angle = a.opt_num(2)?.unwrap_or(30.0).to_radians();
    let unit = a.opt_num(3)?.unwrap_or(120.0);
    let damping = a.opt_num(4)?.unwrap_or(0.3).max(0.0);
    let si = SpringIncline { g: 9.81, k: 20.0, l0: 1.5, m: 1.0, damping, angle, stretch0: 0.6 };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&si, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let bob_pts: Vec<Vec2> = states.iter().map(|st| to_screen(si.body(st))).collect();
    let vel_world: Vec<(f32, f32)> = states.iter().map(|st| si.body_velocity(st)).collect();
    let energies: Vec<Energy> = states.iter().map(|st| si.energy(st)).collect();
    let e_max = peak_energy(&energies);
    let parts = format!("{id}.parts");
    let core_tags = || vec![id.clone(), parts.clone()];
    let overlay_tags = vec![id.clone(), parts.clone(), format!("{id}.overlays")];
    // ramp surface (a long line down-right through the anchor)
    let far = to_screen((3.2 * angle.cos(), -3.2 * angle.sin()));
    let mut ramp = Entity::new(format!("{id}.ramp"), Shape::Line { to: far }, center, style::DIM);
    ramp.stroke.width = 3.0;
    ramp.opacity = 0.6;
    ramp.tags = core_tags();
    s.add(ramp);
    let mut trail = Entity::new(format!("{id}.path"), Shape::Polyline { pts: bob_pts.clone() }, Vec2::ZERO, style::DIM);
    trail.stroke.width = 2.0;
    trail.opacity = 0.3;
    trail.tags = core_tags();
    s.add(trail);
    let mut spring = Entity::new(format!("{id}.spring"), Shape::Coil { to: bob_pts[0], turns: 10 }, center, style::LIME);
    spring.stroke.width = 3.0;
    spring.tags = core_tags();
    s.add(spring);
    let mut anchor = Entity::new(format!("{id}.anchor"), Shape::Circle { r: 6.0 }, center, style::GOLD);
    anchor.stroke.fill = true;
    anchor.stroke.outline = false;
    anchor.tags = core_tags();
    s.add(anchor);
    let mut bob = Entity::new(format!("{id}.bob"), Shape::Circle { r: 15.0 }, bob_pts[0], style::CYAN);
    bob.stroke.fill = true;
    bob.stroke.outline = false;
    bob.tags = core_tags();
    s.add(bob);
    let (energy_series, overlay_tracks) =
        add_overlays(s, &id, center, unit, e_max, &bob_pts, &vel_world, &energies, &overlay_tags);
    let mut playback = vec![
        PlaybackTrack { id: format!("{id}.bob"), prop: Prop::Pos, points: bob_pts.clone() },
        PlaybackTrack { id: format!("{id}.spring"), prop: Prop::To, points: bob_pts },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(id, SimData {
        playback, labels: si.labels(), phase_xy: si.phase_xy(), pos_var: si.pos_var(),
        well: si.well_curve(), energy: energy_series, dt: sim_dt * substeps as f32, states,
    });
    Ok(())
}

/// `bungee(id, [center], [unit], [damping])` — a bungee jump: free-fall then a
/// one-sided elastic cord catches and bounces the jumper.
fn c_bungee(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 120.0) };
    let unit = a.opt_num(2)?.unwrap_or(28.0);
    let damping = a.opt_num(3)?.unwrap_or(50.0).max(0.0);
    let bg = Bungee { g: 9.81, cord: 4.0, k: 800.0, m: 70.0, damping };
    let (sim_dt, substeps) = (0.002f32, 12usize); // stiff cord ⇒ fine dt
    let states = simulate(&bg, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let jumper_pts: Vec<Vec2> = states.iter().map(|st| to_screen(bg.body(st))).collect();
    let vel_world: Vec<(f32, f32)> = states.iter().map(|st| bg.body_velocity(st)).collect();
    let energies: Vec<Energy> = states.iter().map(|st| bg.energy(st)).collect();
    let e_max = peak_energy(&energies);
    let parts = format!("{id}.parts");
    let core_tags = || vec![id.clone(), parts.clone()];
    let overlay_tags = vec![id.clone(), parts.clone(), format!("{id}.overlays")];
    // platform
    let mut plat = Entity::new(format!("{id}.platform"), Shape::Line { to: Vec2::new(center.x + 70.0, center.y) }, Vec2::new(center.x - 70.0, center.y), style::DIM);
    plat.stroke.width = 5.0;
    plat.tags = core_tags();
    s.add(plat);
    // elastic cord platform → jumper
    let mut cord = Entity::new(format!("{id}.cord"), Shape::Line { to: jumper_pts[0] }, center, style::LIME);
    cord.stroke.width = 2.5;
    cord.tags = core_tags();
    s.add(cord);
    let mut jumper = Entity::new(format!("{id}.jumper"), Shape::Circle { r: 14.0 }, jumper_pts[0], style::CYAN);
    jumper.stroke.fill = true;
    jumper.stroke.outline = false;
    jumper.tags = core_tags();
    s.add(jumper);
    let (energy_series, overlay_tracks) =
        add_overlays(s, &id, center, unit, e_max, &jumper_pts, &vel_world, &energies, &overlay_tags);
    let mut playback = vec![
        PlaybackTrack { id: format!("{id}.jumper"), prop: Prop::Pos, points: jumper_pts.clone() },
        PlaybackTrack { id: format!("{id}.cord"), prop: Prop::To, points: jumper_pts },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(id, SimData {
        playback, labels: bg.labels(), phase_xy: bg.phase_xy(), pos_var: bg.pos_var(),
        well: bg.well_curve(), energy: energy_series, dt: sim_dt * substeps as f32, states,
    });
    Ok(())
}

/// `resonance(id, [center], [drivefreq], [unit], [damping])` — a driven spring; a
/// drive frequency near the natural √(k/m) pumps the amplitude up (resonance).
fn c_resonance(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 300.0) };
    let drive_freq = a.opt_num(2)?.unwrap_or(3.8);
    let unit = a.opt_num(3)?.unwrap_or(90.0);
    let damping = a.opt_num(4)?.unwrap_or(0.3).max(0.0);
    let rs = Resonance { k: 16.0, m: 1.0, damping, drive_amp: 2.0, drive_freq };
    sim_spring_like(s, &id, center, unit, &rs, |st| rs.body_velocity(st), false)
}

/// Shared ctor for a **horizontal or vertical single mass on a coil** (spring,
/// vertical-spring, resonance): wall/anchor + coil + mass + trail + overlays.
/// `vertical = true` anchors above and hangs down; `false` anchors left, moves
/// right. Returns after inserting the `SimData`.
fn sim_spring_like<S: Sim, F: Fn(&[f32]) -> (f32, f32)>(
    s: &mut Scene,
    id: &str,
    center: Vec2,
    unit: f32,
    sim: &S,
    vel: F,
    vertical: bool,
) -> Result<(), Error> {
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(sim, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let body_pts: Vec<Vec2> = states.iter().map(|st| to_screen(sim.body(st))).collect();
    let vel_world: Vec<(f32, f32)> = states.iter().map(|st| vel(st)).collect();
    let energies: Vec<Energy> = states.iter().map(|st| sim.energy(st)).collect();
    let e_max = peak_energy(&energies);
    let body0 = body_pts[0];
    // anchor: above (vertical) or to the left of the motion range (horizontal)
    let anchor = if vertical {
        center
    } else {
        let min_x = body_pts.iter().map(|p| p.x).fold(f32::MAX, f32::min);
        Vec2::new(min_x - 60.0, center.y)
    };
    let parts = format!("{id}.parts");
    let core_tags = || vec![id.to_string(), parts.clone()];
    let overlay_tags = vec![id.to_string(), parts.clone(), format!("{id}.overlays")];
    let mut trail = Entity::new(format!("{id}.path"), Shape::Polyline { pts: body_pts.clone() }, Vec2::ZERO, style::DIM);
    trail.stroke.width = 2.0;
    trail.opacity = 0.3;
    trail.tags = core_tags();
    s.add(trail);
    let mut coil = Entity::new(format!("{id}.spring"), Shape::Coil { to: body0, turns: 11 }, anchor, style::LIME);
    coil.stroke.width = 3.0;
    coil.tags = core_tags();
    s.add(coil);
    // a wall (horizontal) or anchor dot (vertical)
    if vertical {
        let mut anc = Entity::new(format!("{id}.anchor"), Shape::Circle { r: 6.0 }, anchor, style::GOLD);
        anc.stroke.fill = true;
        anc.stroke.outline = false;
        anc.tags = core_tags();
        s.add(anc);
    } else {
        let mut wall = Entity::new(format!("{id}.wall"), Shape::Line { to: Vec2::new(anchor.x, center.y + 34.0) }, Vec2::new(anchor.x, center.y - 34.0), style::DIM);
        wall.stroke.width = 5.0;
        wall.tags = core_tags();
        s.add(wall);
    }
    let mut mass = Entity::new(format!("{id}.mass"), Shape::Circle { r: 15.0 }, body0, style::CYAN);
    mass.stroke.fill = true;
    mass.stroke.outline = false;
    mass.tags = core_tags();
    s.add(mass);
    let (energy_series, overlay_tracks) =
        add_overlays(s, id, center, unit, e_max, &body_pts, &vel_world, &energies, &overlay_tags);
    let mut playback = vec![
        PlaybackTrack { id: format!("{id}.mass"), prop: Prop::Pos, points: body_pts.clone() },
        PlaybackTrack { id: format!("{id}.spring"), prop: Prop::To, points: body_pts },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(id.to_string(), SimData {
        playback, labels: sim.labels(), phase_xy: sim.phase_xy(), pos_var: sim.pos_var(),
        well: sim.well_curve(), energy: energy_series, dt: sim_dt * substeps as f32, states,
    });
    Ok(())
}

/// `doublespring(id, [center], [unit])` — two masses coupled by three springs
/// between walls; energy sloshes between them (beating / normal modes).
fn c_doublespring(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(430.0, 320.0) };
    let unit = a.opt_num(2)?.unwrap_or(85.0);
    let ds = DoubleSpring { m1: 1.0, m2: 1.0, k: 20.0, r: 1.8, w1: 0.0, w2: 6.0, damping: 0.0, x1_0: 2.4, x2_0: 4.0 };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&ds, sim_dt, substeps, SAMPLES);
    let sx = |x: f32| Vec2::new(center.x + x * unit, center.y);
    let b1: Vec<Vec2> = states.iter().map(|st| sx(st[0])).collect();
    let b2: Vec<Vec2> = states.iter().map(|st| sx(st[1])).collect();
    let energy: Vec<(f32, f32)> = states.iter().map(|st| { let e = ds.energy(st); (e.kinetic, e.potential) }).collect();
    let (w1s, w2s) = (sx(ds.w1), sx(ds.w2));
    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut mk_wall = |suffix: &str, at: Vec2| {
        let mut w = Entity::new(format!("{id}.{suffix}"), Shape::Line { to: Vec2::new(at.x, at.y + 40.0) }, Vec2::new(at.x, at.y - 40.0), style::DIM);
        w.stroke.width = 5.0;
        w.tags = ct();
        s.add(w);
    };
    mk_wall("wall1", w1s);
    mk_wall("wall2", w2s);
    for (sid, from, to0, col) in [
        (format!("{id}.spring1"), w1s, b1[0], style::LIME),
        (format!("{id}.spring2"), b1[0], b2[0], style::LIME),
        (format!("{id}.spring3"), b2[0], w2s, style::LIME),
    ] {
        let mut c = Entity::new(sid, Shape::Coil { to: to0, turns: 8 }, from, col);
        c.stroke.width = 3.0;
        c.tags = ct();
        s.add(c);
    }
    for (sid, at, col) in [(format!("{id}.block1"), b1[0], style::CYAN), (format!("{id}.block2"), b2[0], style::MAGENTA)] {
        let mut bl = Entity::new(sid, Shape::Rect { w: 34.0, h: 34.0 }, at, col);
        bl.stroke.fill = true;
        bl.stroke.outline = false;
        bl.tags = ct();
        s.add(bl);
    }
    let playback = vec![
        PlaybackTrack { id: format!("{id}.block1"), prop: Prop::Pos, points: b1.clone() },
        PlaybackTrack { id: format!("{id}.block2"), prop: Prop::Pos, points: b2.clone() },
        PlaybackTrack { id: format!("{id}.spring1"), prop: Prop::To, points: b1.clone() },
        PlaybackTrack { id: format!("{id}.spring2"), prop: Prop::Pos, points: b1 },
        PlaybackTrack { id: format!("{id}.spring2"), prop: Prop::To, points: b2.clone() },
        PlaybackTrack { id: format!("{id}.spring3"), prop: Prop::Pos, points: b2 },
    ];
    s.sims.insert(id, SimData {
        playback, labels: ds.labels(), phase_xy: ds.phase_xy(), pos_var: ds.pos_var(),
        well: ds.well_curve(), energy, dt: sim_dt * substeps as f32, states,
    });
    Ok(())
}

/// `seriesparallel(id, [center], [unit])` — two identical masses, one on springs in
/// series (soft, slow) and one in parallel (stiff, fast), bobbing side by side.
fn c_seriesparallel(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 150.0) };
    let unit = a.opt_num(2)?.unwrap_or(70.0);
    let sp = SeriesParallel { g: 9.81, k1: 20.0, k2: 20.0, m: 1.0, damping: 0.0, l0: 0.8, stretch0: 0.5 };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&sp, sim_dt, substeps, SAMPLES);
    let (xs, xp) = (center.x - 140.0, center.x + 140.0);
    let sm: Vec<Vec2> = states.iter().map(|st| Vec2::new(xs, center.y + st[0] * unit)).collect();
    let jn: Vec<Vec2> = states.iter().map(|st| Vec2::new(xs, center.y + st[0] * 0.5 * unit)).collect();
    let pm: Vec<Vec2> = states.iter().map(|st| Vec2::new(xp, center.y + st[2] * unit)).collect();
    let energy: Vec<(f32, f32)> = states.iter().map(|st| { let e = sp.energy(st); (e.kinetic, e.potential) }).collect();
    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    // anchors + labels
    for (sid, at, txt) in [(format!("{id}.anchorS"), Vec2::new(xs, center.y), "series"), (format!("{id}.anchorP"), Vec2::new(xp, center.y), "parallel")] {
        let mut anc = Entity::new(format!("{sid}.dot"), Shape::Circle { r: 5.0 }, at, style::GOLD);
        anc.stroke.fill = true; anc.stroke.outline = false; anc.tags = ct(); s.add(anc);
        let mut lbl = Entity::new(sid, Shape::Text { content: txt.into(), size: 16.0 }, Vec2::new(at.x, at.y - 20.0), style::DIM);
        lbl.tags = ct(); s.add(lbl);
    }
    // series: two stacked coils via the junction
    let mut c1 = Entity::new(format!("{id}.sCoil1"), Shape::Coil { to: jn[0], turns: 6 }, Vec2::new(xs, center.y), style::LIME);
    c1.stroke.width = 3.0; c1.tags = ct(); s.add(c1);
    let mut c2 = Entity::new(format!("{id}.sCoil2"), Shape::Coil { to: sm[0], turns: 6 }, jn[0], style::LIME);
    c2.stroke.width = 3.0; c2.tags = ct(); s.add(c2);
    // parallel: two side-by-side coils to the one mass
    for (sid, ax) in [(format!("{id}.pCoilL"), xp - 15.0), (format!("{id}.pCoilR"), xp + 15.0)] {
        let mut c = Entity::new(sid, Shape::Coil { to: pm[0], turns: 6 }, Vec2::new(ax, center.y), style::LIME);
        c.stroke.width = 3.0; c.tags = ct(); s.add(c);
    }
    for (sid, at, col) in [(format!("{id}.massS"), sm[0], style::CYAN), (format!("{id}.massP"), pm[0], style::MAGENTA)] {
        let mut m = Entity::new(sid, Shape::Rect { w: 40.0, h: 32.0 }, at, col);
        m.stroke.fill = true; m.stroke.outline = false; m.tags = ct(); s.add(m);
    }
    let playback = vec![
        PlaybackTrack { id: format!("{id}.massS"), prop: Prop::Pos, points: sm.clone() },
        PlaybackTrack { id: format!("{id}.massP"), prop: Prop::Pos, points: pm.clone() },
        PlaybackTrack { id: format!("{id}.sCoil1"), prop: Prop::To, points: jn.clone() },
        PlaybackTrack { id: format!("{id}.sCoil2"), prop: Prop::Pos, points: jn },
        PlaybackTrack { id: format!("{id}.sCoil2"), prop: Prop::To, points: sm },
        PlaybackTrack { id: format!("{id}.pCoilL"), prop: Prop::To, points: pm.clone() },
        PlaybackTrack { id: format!("{id}.pCoilR"), prop: Prop::To, points: pm },
    ];
    s.sims.insert(id, SimData {
        playback, labels: sp.labels(), phase_xy: sp.phase_xy(), pos_var: sp.pos_var(),
        well: sp.well_curve(), energy, dt: sim_dt * substeps as f32, states,
    });
    Ok(())
}

/// `carsuspension(id, [center], [unit])` — a quarter-car riding a scrolling road
/// (speed bump, washboard, pothole); the body bobs on its spring+damper.
fn c_carsuspension(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 { a.pair(1)? } else { Vec2::new(640.0, 430.0) };
    let _unit = a.opt_num(2)?;
    let cs = CarSuspension { m: 500.0, k: 20000.0, damping: 4000.0, speed: 8.0, bump: 0.08 };
    let (sim_dt, substeps) = (0.002f32, 12usize);
    let states = simulate(&cs, sim_dt, substeps, SAMPLES);
    let (uy, ux, rest) = (500.0f32, 12.0f32, 100.0f32); // vertical amp, road scale, rest suspension px
    let gy = center.y;
    // road polyline (world rx 0..130), scrolled by animating Pos.x
    let road_pts: Vec<Vec2> = (0..=260).map(|i| { let rx = i as f32 * 0.5; Vec2::new(center.x + rx * ux, gy - road_height(rx, cs.bump) * uy) }).collect();
    let road_pos: Vec<Vec2> = states.iter().map(|st| Vec2::new(-st[2] * ux, 0.0)).collect();
    let wheel: Vec<Vec2> = states.iter().map(|st| Vec2::new(center.x, gy - road_height(st[2], cs.bump) * uy)).collect();
    let body: Vec<Vec2> = states.iter().map(|st| Vec2::new(center.x, gy - rest - st[0] * uy)).collect();
    let energy: Vec<(f32, f32)> = states.iter().map(|st| { let e = cs.energy(st); (e.kinetic, e.potential) }).collect();
    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut road = Entity::new(format!("{id}.road"), Shape::Polyline { pts: road_pts }, Vec2::ZERO, style::DIM);
    road.stroke.width = 3.0;
    road.tags = ct();
    s.add(road);
    let mut susp = Entity::new(format!("{id}.suspension"), Shape::Coil { to: body[0], turns: 6 }, wheel[0], style::LIME);
    susp.stroke.width = 3.0;
    susp.tags = ct();
    s.add(susp);
    let mut wh = Entity::new(format!("{id}.wheel"), Shape::Circle { r: 16.0 }, wheel[0], style::GOLD);
    wh.stroke.fill = false;
    wh.stroke.outline = true;
    wh.stroke.width = 4.0;
    wh.tags = ct();
    s.add(wh);
    let mut car = Entity::new(format!("{id}.body"), Shape::Rect { w: 90.0, h: 44.0 }, body[0], style::CYAN);
    car.stroke.fill = true;
    car.stroke.outline = false;
    car.tags = ct();
    s.add(car);
    let playback = vec![
        PlaybackTrack { id: format!("{id}.road"), prop: Prop::Pos, points: road_pos },
        PlaybackTrack { id: format!("{id}.wheel"), prop: Prop::Pos, points: wheel.clone() },
        PlaybackTrack { id: format!("{id}.body"), prop: Prop::Pos, points: body.clone() },
        PlaybackTrack { id: format!("{id}.suspension"), prop: Prop::Pos, points: wheel },
        PlaybackTrack { id: format!("{id}.suspension"), prop: Prop::To, points: body },
    ];
    s.sims.insert(id, SimData {
        playback, labels: cs.labels(), phase_xy: cs.phase_xy(), pos_var: cs.pos_var(),
        well: cs.well_curve(), energy, dt: sim_dt * substeps as f32, states,
    });
    Ok(())
}

// ── generic sim views (opt-in; work for any sim that stored the data) ────────

/// Map data-space points into a `2·half`-px square panel centred at `center`
/// (y-up), with a margin — the shared fit for the phase/well view panels.
fn panel_fit(pts: &[(f32, f32)], center: Vec2, half: f32) -> impl Fn(f32, f32) -> Vec2 {
    let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for &(x, y) in pts {
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
        y1 = y1.max(y);
    }
    let xr = (x1 - x0).max(1e-6);
    let yr = (y1 - y0).max(1e-6);
    let m = PANEL_MARGIN;
    move |x: f32, y: f32| {
        Vec2::new(
            center.x + ((x - x0) / xr - 0.5) * 2.0 * half * m,
            center.y - ((y - y0) / yr - 0.5) * 2.0 * half * m,
        )
    }
}

/// A faint square frame + title for a view panel (adds `{base}.frame`/`{base}.title`).
fn add_panel(s: &mut Scene, base: &str, center: Vec2, half: f32, title: &str, tags: &[String]) {
    let mut frame = Entity::new(
        format!("{base}.frame"),
        Shape::Rect { w: 2.0 * half, h: 2.0 * half },
        center,
        style::DIM,
    );
    frame.stroke.outline = true;
    frame.stroke.fill = false;
    frame.stroke.width = 1.5;
    frame.opacity = 0.5;
    frame.tags = tags.to_vec();
    s.add(frame);
    let mut lbl = Entity::new(
        format!("{base}.title"),
        Shape::Text { content: title.to_string(), size: 15.0 },
        Vec2::new(center.x, center.y - half - 15.0),
        style::DIM,
    );
    lbl.tags = tags.to_vec();
    s.add(lbl);
}

/// `phase(id, (cx,cy), [size])` — the **phase portrait** of a sim: its two phase
/// variables (e.g. θ vs ω) plotted against each other — a closed loop when energy
/// is conserved, an inward spiral when damped. A dot rides the curve during
/// `swing`. Works for any sim exposing `phase_xy` (call the sim ctor first).
fn c_phase(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = a.pair(1)?;
    let half = a.opt_num(2)?.unwrap_or(120.0).max(30.0);
    let (pts, xl, yl) = {
        let sim = s.sims.get(&id).ok_or_else(|| {
            Error::new(format!("no sim `{id}` — call a sim ctor (e.g. `pendulum`) before `phase`"), a.span_of(0))
        })?;
        let (xi, yi) = sim
            .phase_xy
            .ok_or_else(|| Error::new(format!("sim `{id}` has no phase portrait"), a.span_of(0)))?;
        if sim.states.len() < 2 {
            return Err(Error::new(format!("sim `{id}` has no trajectory"), a.span_of(0)));
        }
        let l = &sim.labels;
        (
            sim.states.iter().map(|st| (st[xi], st[yi])).collect::<Vec<_>>(),
            l.get(xi).cloned().unwrap_or_default(),
            l.get(yi).cloned().unwrap_or_default(),
        )
    };
    let fit = panel_fit(&pts, center, half);
    let screen: Vec<Vec2> = pts.iter().map(|&(x, y)| fit(x, y)).collect();
    let base = format!("{id}.phase");
    let tags = vec![id.clone(), format!("{id}.parts"), base.clone()];
    add_panel(s, &base, center, half, &format!("phase: {yl} vs {xl}"), &tags);

    let mut curve = Entity::new(
        format!("{base}.curve"),
        Shape::Polyline { pts: screen.clone() },
        Vec2::ZERO,
        style::LIME,
    );
    curve.stroke.width = 2.0;
    curve.opacity = 0.75;
    curve.tags = tags.clone();
    s.add(curve);

    let mut dot = Entity::new(format!("{base}.dot"), Shape::Circle { r: 6.0 }, screen[0], style::GOLD);
    dot.stroke.fill = true;
    dot.stroke.outline = false;
    dot.tags = tags.clone();
    s.add(dot);

    if let Some(sim) = s.sims.get_mut(&id) {
        sim.playback.push(PlaybackTrack { id: format!("{base}.dot"), prop: Prop::Pos, points: screen });
    }
    Ok(())
}

/// `well(id, (cx,cy), [size])` — the **potential-energy well**: the sim's U(pos)
/// curve with the body drawn as a ball rolling in it (its height = current PE),
/// so a swing looks like a marble in a bowl. The ball rides the curve during
/// `swing`. Works for any sim exposing a `well_curve` + `pos_var`.
fn c_well(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = a.pair(1)?;
    let half = a.opt_num(2)?.unwrap_or(120.0).max(30.0);
    let (curve_pts, ball_pts) = {
        let sim = s.sims.get(&id).ok_or_else(|| {
            Error::new(format!("no sim `{id}` — call a sim ctor (e.g. `pendulum`) before `well`"), a.span_of(0))
        })?;
        if sim.well.is_empty() {
            return Err(Error::new(format!("sim `{id}` has no potential well"), a.span_of(0)));
        }
        let posi = sim
            .pos_var
            .ok_or_else(|| Error::new(format!("sim `{id}` has no position variable"), a.span_of(0)))?;
        let ball: Vec<(f32, f32)> = sim
            .states
            .iter()
            .zip(sim.energy.iter())
            .map(|(st, e)| (st[posi], e.1)) // (position, potential energy) = point on U
            .collect();
        (sim.well.clone(), ball)
    };
    let fit = panel_fit(&curve_pts, center, half);
    let curve_screen: Vec<Vec2> = curve_pts.iter().map(|&(x, y)| fit(x, y)).collect();
    let ball_screen: Vec<Vec2> = ball_pts.iter().map(|&(x, y)| fit(x, y)).collect();
    let base = format!("{id}.well");
    let tags = vec![id.clone(), format!("{id}.parts"), base.clone()];
    add_panel(s, &base, center, half, "energy well  U(x)", &tags);

    let mut curve = Entity::new(
        format!("{base}.curve"),
        Shape::Polyline { pts: curve_screen },
        Vec2::ZERO,
        style::MAGENTA,
    );
    curve.stroke.width = 2.5;
    curve.tags = tags.clone();
    s.add(curve);

    let mut ball = Entity::new(format!("{base}.ball"), Shape::Circle { r: 8.0 }, ball_screen[0], style::CYAN);
    ball.stroke.fill = true;
    ball.stroke.outline = false;
    ball.tags = tags.clone();
    s.add(ball);

    if let Some(sim) = s.sims.get_mut(&id) {
        sim.playback.push(PlaybackTrack { id: format!("{base}.ball"), prop: Prop::Pos, points: ball_screen });
    }
    Ok(())
}

/// Lay out a **time-series panel**: each `(color, values)` series drawn as a
/// curve over time (shared time x-axis + shared y-range), plus a vertical sweep
/// line marking "now" that rides across during `swing`. Shared by `timegraph`
/// and `energygraph`.
fn add_time_view(
    s: &mut Scene,
    sim_id: &str,
    base: &str,
    center: Vec2,
    half: f32,
    title: &str,
    dt: f32,
    series: &[(macroquad::prelude::Color, Vec<f32>)],
    tags: &[String],
) {
    let n = series.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
    if n < 2 {
        return;
    }
    // range over every (t, v) so all series share one mapping
    let mut all = Vec::new();
    for (_, vals) in series {
        for (k, &v) in vals.iter().enumerate() {
            all.push((k as f32 * dt, v));
        }
    }
    let fit = panel_fit(&all, center, half);
    add_panel(s, base, center, half, title, tags);

    for (i, (col, vals)) in series.iter().enumerate() {
        let pts: Vec<Vec2> = vals.iter().enumerate().map(|(k, &v)| fit(k as f32 * dt, v)).collect();
        let mut e = Entity::new(format!("{base}.c{i}"), Shape::Polyline { pts }, Vec2::ZERO, *col);
        e.stroke.width = 2.0;
        e.tags = tags.to_vec();
        s.add(e);
    }

    // vertical sweep line spanning the content area, swept left→right
    let top = center.y - half * PANEL_MARGIN;
    let bot = center.y + half * PANEL_MARGIN;
    let xs: Vec<f32> = (0..n).map(|k| fit(k as f32 * dt, 0.0).x).collect();
    let mut sweep = Entity::new(
        format!("{base}.sweep"),
        Shape::Line { to: Vec2::new(xs[0], bot) },
        Vec2::new(xs[0], top),
        style::DIM,
    );
    sweep.stroke.width = 2.0;
    sweep.opacity = 0.7;
    sweep.tags = tags.to_vec();
    s.add(sweep);

    if let Some(sim) = s.sims.get_mut(sim_id) {
        sim.playback.push(PlaybackTrack {
            id: format!("{base}.sweep"),
            prop: Prop::Pos,
            points: xs.iter().map(|&x| Vec2::new(x, top)).collect(),
        });
        sim.playback.push(PlaybackTrack {
            id: format!("{base}.sweep"),
            prop: Prop::To,
            points: xs.iter().map(|&x| Vec2::new(x, bot)).collect(),
        });
    }
}

/// `timegraph(id, (cx,cy), [size])` — the sim's phase variables as **curves over
/// time** (e.g. θ(t) cyan, ω(t) magenta) with a sweep line marking "now" during
/// `swing`. Works for any sim exposing `phase_xy`.
fn c_timegraph(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = a.pair(1)?;
    let half = a.opt_num(2)?.unwrap_or(120.0).max(30.0);
    let (series, dt, title) = {
        let sim = s.sims.get(&id).ok_or_else(|| {
            Error::new(format!("no sim `{id}` — call a sim ctor before `timegraph`"), a.span_of(0))
        })?;
        let (xi, yi) = sim
            .phase_xy
            .ok_or_else(|| Error::new(format!("sim `{id}` has no time-series variables"), a.span_of(0)))?;
        if sim.states.len() < 2 {
            return Err(Error::new(format!("sim `{id}` has no trajectory"), a.span_of(0)));
        }
        let l = &sim.labels;
        let xs: Vec<f32> = sim.states.iter().map(|st| st[xi]).collect();
        let ys: Vec<f32> = sim.states.iter().map(|st| st[yi]).collect();
        let title = format!(
            "time: {} & {}",
            l.get(xi).cloned().unwrap_or_default(),
            l.get(yi).cloned().unwrap_or_default()
        );
        (vec![(style::CYAN, xs), (style::MAGENTA, ys)], sim.dt, title)
    };
    let tags = vec![id.clone(), format!("{id}.parts"), format!("{id}.time")];
    add_time_view(s, &id, &format!("{id}.time"), center, half, &title, dt, &series, &tags);
    Ok(())
}

/// `energygraph(id, (cx,cy), [size])` — kinetic (cyan), potential (magenta), and
/// total (gold) energy as **curves over time**: total is flat when conserved,
/// decaying when damped. A sweep line marks "now" during `swing`.
fn c_energygraph(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = a.pair(1)?;
    let half = a.opt_num(2)?.unwrap_or(120.0).max(30.0);
    let (series, dt) = {
        let sim = s.sims.get(&id).ok_or_else(|| {
            Error::new(format!("no sim `{id}` — call a sim ctor before `energygraph`"), a.span_of(0))
        })?;
        if sim.energy.len() < 2 {
            return Err(Error::new(format!("sim `{id}` has no energy series"), a.span_of(0)));
        }
        let ke: Vec<f32> = sim.energy.iter().map(|e| e.0).collect();
        let pe: Vec<f32> = sim.energy.iter().map(|e| e.1).collect();
        let total: Vec<f32> = sim.energy.iter().map(|e| e.0 + e.1).collect();
        (vec![(style::CYAN, ke), (style::MAGENTA, pe), (style::GOLD, total)], sim.dt)
    };
    let tags = vec![id.clone(), format!("{id}.parts"), format!("{id}.energy")];
    add_time_view(s, &id, &format!("{id}.energy"), center, half, "energy: KE PE total", dt, &series, &tags);
    Ok(())
}

/// Register the physics kit's vocabulary.
pub fn register(r: &mut Registry) {
    // sims
    r.ctor("pendulum", c_pendulum);
    r.ctor("spring", c_spring);
    r.ctor("doublependulum", c_doublependulum);
    r.ctor("springpendulum", c_springpendulum);
    r.ctor("kapitza", c_kapitza);
    r.ctor("cartpendulum", c_cartpendulum);
    r.ctor("comparependulum", c_comparependulum);
    r.ctor("verticalspring", c_verticalspring);
    r.ctor("springincline", c_springincline);
    r.ctor("bungee", c_bungee);
    r.ctor("resonance", c_resonance);
    r.ctor("doublespring", c_doublespring);
    r.ctor("seriesparallel", c_seriesparallel);
    r.ctor("carsuspension", c_carsuspension);
    // playback (`run` is the generic name; `swing` is a pendulum-friendly alias)
    r.verb("run", v_play);
    r.verb("swing", v_play);
    // generic views (any sim)
    r.ctor("phase", c_phase);
    r.ctor("well", c_well);
    r.ctor("timegraph", c_timegraph);
    r.ctor("energygraph", c_energygraph);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small-swing pendulum reproduces the textbook period 2π√(L/g), measured
    /// from the simulation's own zero-crossings.
    #[test]
    fn pendulum_reproduces_small_angle_period() {
        let p = Pendulum { theta0: 0.05, ..Default::default() };
        let dt = 0.0005;
        let steps = 14_000; // ~3.5 periods at L=1, g=9.81
        let traj = ode::integrate(&p.state0(), dt, steps, |s, d| p.deriv(s, d));
        // upward zero-crossings of θ (−→+) occur once per period
        let mut crossings = Vec::new();
        for i in 1..traj.len() {
            if traj[i - 1][0] < 0.0 && traj[i][0] >= 0.0 {
                crossings.push(i as f32 * dt);
            }
        }
        assert!(crossings.len() >= 2, "expected ≥2 periods of swing");
        let period = crossings[1] - crossings[0];
        let expected = p.small_angle_period();
        assert!(
            (period - expected).abs() / expected < 0.02,
            "measured period {period} vs textbook {expected}"
        );
    }

    /// Undamped, unforced: total mechanical energy is conserved along the path.
    #[test]
    fn undamped_pendulum_conserves_energy() {
        let p = Pendulum::default(); // 60° swing, no damping/drive
        let traj = ode::integrate(&p.state0(), 0.0005, 8_000, |s, d| p.deriv(s, d));
        let e0 = p.energy(&traj[0]).total();
        for s in &traj {
            let e = p.energy(s).total();
            assert!((e - e0).abs() / e0 < 0.01, "energy drifted: {e} vs {e0}");
        }
    }

    /// The `pendulum` ctor lays out all parts (incl. overlays) and stores the
    /// six playback tracks the `swing` verb replays.
    #[test]
    fn pendulum_ctor_builds_parts_and_playback() {
        let m = crate::parse("canvas(\"16:9\");\npendulum(p, (640, 200), 2, 50);\n").unwrap();
        let base = m.base();
        for sub in ["p.pivot", "p.rod", "p.bob", "p.path", "p.vel", "p.ke", "p.pe"] {
            assert!(base.contains(sub), "missing entity `{sub}`");
        }
        // `swing(p)` must resolve against those — proven by a clean parse+validate
        let m2 = crate::parse("canvas(\"16:9\");\npendulum(p);\nswing(p, 6);\n").unwrap();
        assert!(m2.validate().is_ok(), "pendulum+swing should validate: {:?}", m2.validate().err());
    }

    /// The generic views (`phase`/`well`) lay out their panels + curve + marker
    /// off a sim's stored data, and the marker joins the `swing` playback.
    #[test]
    fn phase_and_well_views_build_and_animate() {
        let src = "canvas(\"16:9\");\npendulum(p, (300, 200), 1.5, 55);\n\
                   phase(p, (900, 200), 120);\nwell(p, (900, 480), 120);\nswing(p, 8);\n";
        let m = crate::parse(src).unwrap();
        let base = m.base();
        for sub in ["p.phase.curve", "p.phase.dot", "p.well.curve", "p.well.ball"] {
            assert!(base.contains(sub), "missing view entity `{sub}`");
        }
        assert!(m.validate().is_ok(), "views + swing should validate: {:?}", m.validate().err());
    }

    /// The time-graph views (`timegraph`/`energygraph`) build curves + a sweep
    /// line off the sim's series, and the sweep joins the `swing` playback.
    #[test]
    fn time_and_energy_graphs_build() {
        let src = "canvas(\"16:9\");\npendulum(p, (280, 210), 1.3, 55);\n\
                   timegraph(p, (900, 200), 100);\nenergygraph(p, (900, 470), 100);\nswing(p, 8);\n";
        let m = crate::parse(src).unwrap();
        let base = m.base();
        for sub in ["p.time.c0", "p.time.c1", "p.time.sweep", "p.energy.c0", "p.energy.c2", "p.energy.sweep"] {
            assert!(base.contains(sub), "missing graph entity `{sub}`");
        }
        assert!(m.validate().is_ok(), "graphs + swing should validate: {:?}", m.validate().err());
    }

    /// The spring reproduces the SHM period 2π√(m/k) from its own zero-crossings.
    #[test]
    fn spring_reproduces_shm_period() {
        let sp = Spring { k: 12.0, mass: 1.0, damping: 0.0, x0: 0.1 };
        let dt = 0.0005;
        let traj = ode::integrate(&sp.state0(), dt, 12_000, |s, d| sp.deriv(s, d));
        let mut crossings = Vec::new();
        for i in 1..traj.len() {
            if traj[i - 1][0] < 0.0 && traj[i][0] >= 0.0 {
                crossings.push(i as f32 * dt);
            }
        }
        assert!(crossings.len() >= 2, "expected ≥2 oscillations");
        let period = crossings[1] - crossings[0];
        assert!(
            (period - sp.period()).abs() / sp.period() < 0.02,
            "measured {period} vs SHM {}",
            sp.period()
        );
    }

    /// Undamped spring conserves energy; the ctor builds its parts and inherits
    /// the generic views (parabolic well, phase ellipse).
    #[test]
    fn spring_conserves_energy_and_inherits_views() {
        let sp = Spring { k: 10.0, mass: 1.0, damping: 0.0, x0: 1.0 };
        let traj = ode::integrate(&sp.state0(), 0.0005, 8_000, |s, d| sp.deriv(s, d));
        let e0 = sp.energy(&traj[0]).total();
        for s in &traj {
            assert!((sp.energy(s).total() - e0).abs() / e0 < 0.01, "energy drifted");
        }
        let src = "canvas(\"16:9\");\nspring(sp, (360,300), 10, 1.2);\n\
                   phase(sp,(900,200),110);\nwell(sp,(900,470),110);\nrun(sp, 8);\n";
        let m = crate::parse(src).unwrap();
        let base = m.base();
        for sub in ["sp.wall", "sp.spring", "sp.mass", "sp.phase.curve", "sp.well.curve"] {
            assert!(base.contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok(), "spring + views + run should validate: {:?}", m.validate().err());
    }

    /// The undamped double pendulum conserves total energy (a good check on the
    /// coupled equations of motion), and its ctor builds the two arms + inherits
    /// phase/energy views — but `well` is refused (4-D system).
    #[test]
    fn double_pendulum_conserves_energy_and_has_no_well() {
        let dp = DoublePendulum { g: 9.8, l1: 1.0, l2: 1.0, m1: 2.0, m2: 2.0, a1: 1.2, a2: 0.7 };
        let traj = ode::integrate(&dp.state0(), 0.002, 6_000, |s, d| dp.deriv(s, d));
        let e0 = dp.energy(&traj[0]).total();
        for s in &traj {
            assert!((dp.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02, "energy drifted");
        }
        assert!(dp.well_curve().is_empty(), "double pendulum should have no well curve");

        let ok = "canvas(\"16:9\");\ndoublependulum(dp, (400,240), 120, 100);\n\
                  phase(dp,(940,200),110);\nenergygraph(dp,(940,470),110);\nrun(dp, 8);\n";
        let m = crate::parse(ok).unwrap();
        let base = m.base();
        for sub in ["dp.rod1", "dp.bob1", "dp.rod2", "dp.bob2", "dp.path", "dp.phase.curve"] {
            assert!(base.contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok(), "double pendulum + views should validate");
        // `well` on a 4-D sim is refused at parse time
        assert!(crate::parse("canvas(\"16:9\");\ndoublependulum(d);\nwell(d,(0,0),100);\n").is_err());
    }

    /// Elastic pendulum: undamped conserves energy; ctor builds its coil + views.
    #[test]
    fn spring_pendulum_conserves_energy() {
        let sp = SpringPendulum { g: 9.81, k: 40.0, l0: 1.5, m: 1.0, damping: 0.0, a0: 0.5, stretch0: 0.4 };
        let traj = ode::integrate(&sp.state0(), 0.001, 8_000, |s, d| sp.deriv(s, d));
        let e0 = sp.energy(&traj[0]).total();
        for s in &traj {
            assert!((sp.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02, "energy drifted");
        }
        let m = crate::parse("canvas(\"16:9\");\nspringpendulum(sp,(400,240),30,0.3);\nphase(sp,(900,200),110);\nrun(sp,8);\n").unwrap();
        for sub in ["sp.spring", "sp.bob", "sp.phase.curve"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Cart-pendulum: undamped conserves energy; ctor builds cart + coil + rod.
    #[test]
    fn cart_pendulum_conserves_energy() {
        let cp = CartPendulum { g: 9.8, l: 1.0, mcart: 1.0, mbob: 1.0, k: 6.0, cart_damp: 0.0, bob_damp: 0.0, a0: 0.7 };
        let traj = ode::integrate(&cp.state0(), 0.001, 8_000, |s, d| cp.deriv(s, d));
        let e0 = cp.energy(&traj[0]).total();
        for s in &traj {
            assert!((cp.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02, "energy drifted");
        }
        let m = crate::parse("canvas(\"16:9\");\ncartpendulum(cp,(500,340),45);\nrun(cp,8);\n").unwrap();
        for sub in ["cp.cart", "cp.spring", "cp.rod", "cp.bob"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Compare-pendulum: two starts 0.001 rad apart diverge (sensitive dependence).
    #[test]
    fn compare_pendulum_diverges() {
        let cmp = ComparePendulum { g: 9.81, l: 1.0, m: 1.0, damping: 0.5, drive_amp: 10.0, drive_freq: 0.667, a0: 0.1, delta: 0.001 };
        let traj = ode::integrate(&cmp.state0(), 0.002, 8_000, |s, d| cmp.deriv(s, d));
        let d0 = (traj[0][2] - traj[0][0]).abs();
        let dn = (traj.last().unwrap()[2] - traj.last().unwrap()[0]).abs();
        assert!(d0 < 0.01, "should start close: {d0}");
        // exponential separation (positive Lyapunov) — the signature of chaos,
        // even when the absolute gap is still modest over the window
        assert!(dn > 20.0 * d0, "should diverge exponentially: start {d0}, end {dn}");
        let m = crate::parse("canvas(\"16:9\");\ncomparependulum(cm,(400,240),10);\nphase(cm,(900,200),110);\nrun(cm,10);\n").unwrap();
        for sub in ["cm.rodA", "cm.bobA", "cm.rodB", "cm.bobB", "cm.phase.curve"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Kapitza: builds + supports phase, but `well` is refused (driven system).
    #[test]
    fn kapitza_builds_no_well() {
        let m = crate::parse("canvas(\"16:9\");\nkapitza(kp,(640,420),165,220);\nphase(kp,(1000,300),120);\nrun(kp,8);\n").unwrap();
        for sub in ["kp.pivot", "kp.rod", "kp.bob", "kp.phase.curve"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
        assert!(crate::parse("canvas(\"16:9\");\nkapitza(k);\nwell(k,(0,0),100);\n").is_err());
    }

    /// The single-mass spring variants each conserve energy undamped and build.
    #[test]
    fn single_mass_spring_variants() {
        let vs = VerticalSpring { g: 9.81, k: 20.0, l0: 1.0, m: 1.0, damping: 0.0, stretch0: 0.6 };
        let t1 = ode::integrate(&vs.state0(), 0.001, 6_000, |s, d| vs.deriv(s, d));
        let e0 = vs.energy(&t1[0]).total();
        for s in &t1 { assert!((vs.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02); }
        for (src, subs) in [
            ("verticalspring(vs);well(vs,(900,300),100);run(vs,8);", vec!["vs.spring", "vs.mass", "vs.well.curve"]),
            ("springincline(si,(400,190),30);run(si,8);", vec!["si.ramp", "si.spring", "si.bob"]),
            ("bungee(bg);run(bg,8);", vec!["bg.platform", "bg.cord", "bg.jumper"]),
            ("resonance(rs);run(rs,8);", vec!["rs.wall", "rs.spring", "rs.mass"]),
        ] {
            let m = crate::parse(&format!("canvas(\"16:9\");\n{src}")).unwrap();
            for sub in subs { assert!(m.base().contains(sub), "missing `{sub}`"); }
            assert!(m.validate().is_ok(), "{src} should validate");
        }
    }

    /// Multi-body springs: double-spring conserves energy; series/parallel and the
    /// car all build their parts.
    #[test]
    fn multi_body_spring_sims() {
        let ds = DoubleSpring { m1: 1.0, m2: 1.0, k: 20.0, r: 1.8, w1: 0.0, w2: 6.0, damping: 0.0, x1_0: 2.4, x2_0: 4.0 };
        let t = ode::integrate(&ds.state0(), 0.001, 6_000, |s, d| ds.deriv(s, d));
        let e0 = ds.energy(&t[0]).total();
        for s in &t { assert!((ds.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02, "double-spring energy drift"); }
        for (src, subs) in [
            ("doublespring(dd);run(dd,8);", vec!["dd.block1", "dd.block2", "dd.spring2"]),
            ("seriesparallel(sp);run(sp,8);", vec!["sp.massS", "sp.massP", "sp.sCoil1"]),
            ("carsuspension(car);run(car,8);", vec!["car.road", "car.wheel", "car.body", "car.suspension"]),
        ] {
            let m = crate::parse(&format!("canvas(\"16:9\");\n{src}")).unwrap();
            for sub in subs { assert!(m.base().contains(sub), "missing `{sub}`"); }
            assert!(m.validate().is_ok(), "{src} should validate");
        }
    }

    /// Damping bleeds energy: the swing loses mechanical energy over time.
    #[test]
    fn damping_bleeds_energy() {
        let p = Pendulum { damping: 0.5, ..Default::default() };
        let traj = ode::integrate(&p.state0(), 0.001, 5_000, |s, d| p.deriv(s, d));
        let e0 = p.energy(&traj[0]).total();
        let e_end = p.energy(traj.last().unwrap()).total();
        assert!(e_end < 0.5 * e0, "damped energy {e_end} should be well below {e0}");
    }
}
