//! physics — a new domain kit (Layer 1, in progress).
//!
//! A simulation is a **named state vector** evolving under its equations of
//! motion, pre-integrated once at build time (via [`crate::ode`]) into a sampled
//! trajectory the stateless timeline replays — deterministic, so recordings are
//! reproducible. This file establishes the declarative **sim model** (mirroring
//! the uniform specs in the crypto-tool RK4 goldmine) and the first named sim,
//! the pendulum. The drawable/replay wiring and the `pendulum(...)` builtin
//! (Layer-1 ctor) land next; nothing is registered into the vocabulary yet.

use crate::lang::lower::{Args, Registry};
use crate::ode;
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
        (
            self.length * omega * theta.cos(),
            self.length * omega * theta.sin(),
        )
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
        (
            self.l1 * th1.sin() + self.l2 * th2.sin(),
            -self.l1 * th1.cos() - self.l2 * th2.cos(),
        )
    }
    fn labels(&self) -> Vec<String> {
        vec![
            "θ₁".into(),
            "ω₁".into(),
            "θ₂".into(),
            "ω₂".into(),
            "t".into(),
        ]
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
        (
            vr * th.sin() + r * w * th.cos(),
            -vr * th.cos() + r * w * th.sin(),
        )
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
        vec![
            "θA".into(),
            "ωA".into(),
            "θB".into(),
            "ωB".into(),
            "t".into(),
        ]
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
                (
                    d,
                    0.5 * self.k * (d - self.l0).powi(2) - self.m * self.g * d,
                )
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
                (
                    s,
                    0.5 * self.k * (s - self.l0).powi(2) - self.m * self.g * s * sn,
                )
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
        vec![
            "x₁".into(),
            "x₂".into(),
            "v₁".into(),
            "v₂".into(),
            "t".into(),
        ]
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
        vec![
            self.eq_s() + self.stretch0,
            0.0,
            self.eq_p() + self.stretch0,
            0.0,
            0.0,
        ]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        d[0] = s[1];
        d[1] =
            self.g - (self.ks() / self.m) * (s[0] - 2.0 * self.l0) - (self.damping / self.m) * s[1];
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
        vec![
            "y_s".into(),
            "ẏ_s".into(),
            "y_p".into(),
            "ẏ_p".into(),
            "t".into(),
        ]
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

/// An **automotive piston** — a slider-crank mechanism. The crank spins at a
/// constant rate; the piston's height follows H(θ) = a·cosθ + √(L² − a²sin²θ).
/// State = `[θ, t]` (kinematic — driven, not force-integrated). From
/// `sims/piston.js`. Dimensions in mm.
pub struct Piston {
    pub a: f32,   // crank radius
    pub l: f32,   // connecting-rod length
    pub rpm: f32, // crank speed
}
impl Piston {
    fn omega(&self) -> f32 {
        self.rpm * std::f32::consts::TAU / 60.0
    }
    /// Crank-pin world position (mm), crank centre at origin.
    pub fn pin(&self, th: f32) -> (f32, f32) {
        (self.a * th.sin(), self.a * th.cos())
    }
    /// Piston height above the crank centre (mm).
    pub fn height(&self, th: f32) -> f32 {
        self.a * th.cos()
            + (self.l * self.l - self.a * self.a * th.sin() * th.sin())
                .max(0.0)
                .sqrt()
    }
}
impl Sim for Piston {
    fn state0(&self) -> Vec<f32> {
        vec![0.0, 0.0]
    }
    fn deriv(&self, _s: &[f32], d: &mut [f32]) {
        d[0] = self.omega();
        d[1] = 1.0;
    }
    fn energy(&self, _s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.0,
            potential: 0.0,
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (0.0, self.height(s[0]))
    }
    fn labels(&self) -> Vec<String> {
        vec!["θ".into(), "t".into()]
    }
    // kinematic mechanism — no phase/well/energy views
}

/// A **vibrating molecule**: N atoms in 2D connected by springs on every bond,
/// oscillating about their equilibrium shape (gravity off, so they vibrate in
/// place). State = `[x₁,y₁,vx₁,vy₁, …, t]`. From `sims/molecule.js`.
pub struct Molecule {
    pub n: usize,
    pub k: f32,
    pub rest: f32,
    pub mass: f32,
    pub damping: f32,
}
impl Sim for Molecule {
    fn state0(&self) -> Vec<f32> {
        use std::f32::consts::{FRAC_PI_2, TAU};
        let mut st = vec![0.0; 4 * self.n + 1];
        let radius = self.rest * 0.8;
        for i in 0..self.n {
            let a = TAU * i as f32 / self.n as f32 - FRAC_PI_2;
            let r = if i == 0 { radius * 1.4 } else { radius }; // atom 0 pulled out → vibration
            st[i * 4] = r * a.cos();
            st[i * 4 + 1] = r * a.sin();
        }
        st
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let n = self.n;
        for i in 0..n {
            d[i * 4] = s[i * 4 + 2];
            d[i * 4 + 1] = s[i * 4 + 3];
            d[i * 4 + 2] = -self.damping * s[i * 4 + 2] / self.mass;
            d[i * 4 + 3] = -self.damping * s[i * 4 + 3] / self.mass;
        }
        for i in 0..n {
            for j in (i + 1)..n {
                let dx = s[j * 4] - s[i * 4];
                let dy = s[j * 4 + 1] - s[i * 4 + 1];
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < 1e-6 {
                    continue;
                }
                let fmag = self.k * (dist - self.rest) / dist;
                let (fx, fy) = (fmag * dx, fmag * dy);
                d[i * 4 + 2] += fx / self.mass;
                d[i * 4 + 3] += fy / self.mass;
                d[j * 4 + 2] -= fx / self.mass;
                d[j * 4 + 3] -= fy / self.mass;
            }
        }
        d[4 * n] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        let mut ke = 0.0;
        for i in 0..self.n {
            ke += 0.5 * self.mass * (s[i * 4 + 2].powi(2) + s[i * 4 + 3].powi(2));
        }
        let mut pe = 0.0;
        for i in 0..self.n {
            for j in (i + 1)..self.n {
                let dx = s[j * 4] - s[i * 4];
                let dy = s[j * 4 + 1] - s[i * 4 + 1];
                pe += 0.5 * self.k * ((dx * dx + dy * dy).sqrt() - self.rest).powi(2);
            }
        }
        Energy {
            kinetic: ke,
            potential: pe,
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[0], s[1])
    }
    // 2-D N-body — no phase/well; `energygraph` still works (energy is conserved)
}

// ── mechanics: arms, pulleys, inclines, curves ──────────────────────────────

/// A two-link **robot arm** reaching for a target by inverse-kinematics velocity
/// control. State = `[θ₁, θ₂, t]`; the joint rates are the analytic 2×2 inverse
/// Jacobian times a proportional gain on the end-effector error, so the arm
/// drives its gripper onto the target. First-order control (velocities, not
/// torques). The target can be fixed (`mode 0` — reach and settle) or moving
/// (`mode 1` circle, `mode 2` figure-8) so the arm tracks it continuously.
/// Transcribed from `sims/robot-arm.js`.
pub struct RobotArm {
    pub l1: f32,
    pub l2: f32,
    pub gain: f32,
    pub mode: u8, // 0 = fixed target, 1 = trace a circle, 2 = trace a figure-8
    pub tx: f32,  // fixed-target position (mode 0)
    pub ty: f32,
}
impl RobotArm {
    /// The target position at time `t` (moving for modes 1/2).
    pub fn target(&self, t: f32) -> (f32, f32) {
        use std::f32::consts::TAU;
        match self.mode {
            1 => {
                let w = TAU / 5.0;
                (0.7 + 0.3 * (w * t).cos(), 0.7 + 0.3 * (w * t).sin())
            }
            2 => {
                let w = TAU / 6.0;
                (0.7 + 0.4 * (w * t).sin(), 0.5 + 0.3 * (2.0 * w * t).sin())
            }
            _ => (self.tx, self.ty),
        }
    }
    /// Elbow (joint-1) world position.
    pub fn elbow(&self, s: &[f32]) -> (f32, f32) {
        (self.l1 * s[0].cos(), self.l1 * s[0].sin())
    }
}
impl Sim for RobotArm {
    fn state0(&self) -> Vec<f32> {
        use std::f32::consts::FRAC_PI_4;
        vec![FRAC_PI_4, -FRAC_PI_4, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let (t1, t2) = (s[0], s[1]);
        let (s1, c1) = (t1.sin(), t1.cos());
        let (s12, c12) = ((t1 + t2).sin(), (t1 + t2).cos());
        let xe = self.l1 * c1 + self.l2 * c12;
        let ye = self.l1 * s1 + self.l2 * s12;
        let (tx, ty) = self.target(s[2]);
        let (ex, ey) = (tx - xe, ty - ye);
        // 2×2 Jacobian of (xe, ye) w.r.t (θ₁, θ₂)
        let j11 = -self.l1 * s1 - self.l2 * s12;
        let j12 = -self.l2 * s12;
        let j21 = self.l1 * c1 + self.l2 * c12;
        let j22 = self.l2 * c12;
        let mut det = j11 * j22 - j12 * j21; // = l1·l2·sinθ₂
        let min_det = 0.01; // singularity floor (near full-stretch / fold)
        if det.abs() < min_det {
            det = if det < 0.0 { -min_det } else { min_det };
        }
        // inverse-Jacobian velocity command, gain·error, clamped
        let w1 = self.gain * (j22 * ex - j12 * ey) / det;
        let w2 = self.gain * (-j21 * ex + j11 * ey) / det;
        d[0] = w1.clamp(-8.0, 8.0);
        d[1] = w2.clamp(-8.0, 8.0);
        d[2] = 1.0;
    }
    fn energy(&self, _s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.0,
            potential: 0.0,
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        let (t1, t2) = (s[0], s[1]);
        (
            self.l1 * t1.cos() + self.l2 * (t1 + t2).cos(),
            self.l1 * t1.sin() + self.l2 * (t1 + t2).sin(),
        )
    }
    fn labels(&self) -> Vec<String> {
        vec!["θ₁".into(), "θ₂".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 1)) // joint-space path to the target
    }
    // kinematic controller — no potential well, no mechanical energy
}

/// A vertical **Atwood machine**: two masses on a light string over a single
/// pulley. State = `[s, v, t]` — `s` is how far m₁ has descended (m₂ rises the
/// same distance), `v` its rate. Constant acceleration a = ((m₁−m₂)g − b·v)/
/// (m₁+m₂). The steady rope tension is 2·m₁·m₂·g/(m₁+m₂) — *between* the two
/// weights, the point the scale variant makes visible. From `sims/pulley-scale.js`.
pub struct Pulley {
    pub m1: f32,
    pub m2: f32,
    pub g: f32,
    pub damping: f32,
}
impl Pulley {
    /// Steady-state rope tension 2·m₁·m₂·g/(m₁+m₂).
    pub fn tension(&self) -> f32 {
        2.0 * self.m1 * self.m2 * self.g / (self.m1 + self.m2)
    }
}
impl Sim for Pulley {
    fn state0(&self) -> Vec<f32> {
        vec![0.0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let v = s[1];
        d[0] = v;
        d[1] = ((self.m1 - self.m2) * self.g - self.damping * v) / (self.m1 + self.m2);
        d[2] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * (self.m1 + self.m2) * s[1] * s[1],
            potential: (self.m2 - self.m1) * self.g * s[0],
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (0.0, -s[0]) // m₁ descends
    }
    fn labels(&self) -> Vec<String> {
        vec!["s".into(), "v".into(), "t".into()]
    }
    // monotone accel — no phase/well; energygraph shows the KE↔PE trade
}

/// A **compound pulley** (fixed pulley + movable pulley, three masses): a fixed
/// top pulley whose rope carries mass A on one side and a MOVABLE lower pulley P
/// on the other; P's rope carries masses B and C. State = `[xA, vA, xB, vB, xC,
/// vC, t]`, downward positive. The string constraints tie them together —
/// a_A = −a_P and a_B + a_C = 2·a_P — and the massless movable pulley gives
/// T₁ = 2·T₂, so the accelerations are constant:
///   T₂ = 4g/(4/mA + 1/mB + 1/mC),  a_A = g − 2T₂/mA,  a_B = g − T₂/mB,  a_C = g − T₂/mC.
/// It is static exactly when mA = mB + mC. This is the classic A/B/C figure.
pub struct CompoundPulley {
    pub ma: f32,
    pub mb: f32,
    pub mc: f32,
    pub g: f32,
}
impl CompoundPulley {
    fn tension2(&self) -> f32 {
        4.0 * self.g / (4.0 / self.ma + 1.0 / self.mb + 1.0 / self.mc)
    }
    /// The three (constant) mass accelerations `(a_A, a_B, a_C)`, down positive.
    pub fn accels(&self) -> (f32, f32, f32) {
        let t2 = self.tension2();
        (
            self.g - 2.0 * t2 / self.ma,
            self.g - t2 / self.mb,
            self.g - t2 / self.mc,
        )
    }
}
impl Sim for CompoundPulley {
    fn state0(&self) -> Vec<f32> {
        vec![0.0; 7]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let (aa, ab, ac) = self.accels();
        d[0] = s[1];
        d[1] = aa;
        d[2] = s[3];
        d[3] = ab;
        d[4] = s[5];
        d[5] = ac;
        d[6] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * (self.ma * s[1] * s[1] + self.mb * s[3] * s[3] + self.mc * s[5] * s[5]),
            potential: -self.g * (self.ma * s[0] + self.mb * s[2] + self.mc * s[4]),
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (0.0, -s[0]) // mass A
    }
    fn labels(&self) -> Vec<String> {
        vec![
            "x_A".into(),
            "v_A".into(),
            "x_B".into(),
            "v_B".into(),
            "x_C".into(),
            "v_C".into(),
            "t".into(),
        ]
    }
    // 3-mass constrained system — no phase/well; energygraph shows KE↔PE
}

/// A **block-and-tackle** (compound pulley): a load `M` on a movable block held
/// by `N` rope strands, pulled by an effort mass `m`. State = `[x, v, t]`, x the
/// load's rise. The `N` strands give a **mechanical advantage of N** — the effort
/// end moves N× as far as the load, so an effort of only `M/N` balances the load.
/// Constraint dynamics: a = (N·m − M)·g / (M + N²·m). N = 1 recovers the Atwood.
pub struct BlockTackle {
    pub load: f32,
    pub effort: f32,
    pub strands: f32, // N — the number of supporting rope segments = the advantage
    pub g: f32,
}
impl BlockTackle {
    fn accel(&self) -> f32 {
        let n = self.strands;
        (n * self.effort - self.load) * self.g / (self.load + n * n * self.effort)
    }
}
impl Sim for BlockTackle {
    fn state0(&self) -> Vec<f32> {
        vec![0.0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        d[0] = s[1];
        d[1] = self.accel();
        d[2] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        let n = self.strands;
        Energy {
            // effort end moves N·v, so its KE carries the N² factor
            kinetic: 0.5 * (self.load + n * n * self.effort) * s[1] * s[1],
            potential: (self.load - n * self.effort) * self.g * s[0],
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (0.0, s[0]) // the load rises
    }
    fn labels(&self) -> Vec<String> {
        vec!["x".into(), "v".into(), "t".into()]
    }
    // monotone accel — no phase/well; energygraph shows the KE↔PE trade
}

/// A block on an **inclined plane** with friction. State = `[s, v, t]` — `s` is
/// distance up the incline, `v` its rate. Full force model: gravity component
/// down the slope, normal force, a static/kinetic friction switch, and an
/// optional horizontal applied force. Friction dissipates mechanical energy, so
/// the energy total decays. Transcribed from `sims/ramp.js`.
pub struct Ramp {
    pub g: f32,
    pub angle: f32, // incline, radians
    pub mass: f32,
    pub mu_s: f32,
    pub mu_k: f32,
    pub applied: f32, // horizontal applied force, N
    pub s0: f32,      // start distance up the incline
}
impl Ramp {
    /// Normal force magnitude N = m·g·cosθ + F_applied·sinθ (clamped ≥ 0).
    fn normal(&self) -> f32 {
        (self.mass * self.g * self.angle.cos() + self.applied * self.angle.sin()).max(0.0)
    }
    /// Friction force along the slope (signed, +s = up the incline).
    fn friction_along(&self, v: f32) -> f32 {
        let n = self.normal();
        let fnet_nf = self.applied * self.angle.cos() - self.mass * self.g * self.angle.sin();
        if v.abs() > 1e-3 {
            -v.signum() * self.mu_k * n // kinetic: opposes velocity
        } else {
            let fs_max = self.mu_s * n; // static: cancels the net, up to its budget
            if fnet_nf.abs() <= fs_max {
                -fnet_nf
            } else {
                -fnet_nf.signum() * self.mu_k * n
            }
        }
    }
    fn accel(&self, v: f32) -> f32 {
        let fnet_nf = self.applied * self.angle.cos() - self.mass * self.g * self.angle.sin();
        (fnet_nf + self.friction_along(v)) / self.mass
    }
    /// The free-body force vectors at velocity `v`, each `(label, world (fx,fy) in
    /// newtons, y-up)`: gravity (down), normal (⟂ to the slope), friction (along it).
    pub fn force_vectors(&self, v: f32) -> [(&'static str, f32, f32); 3] {
        let (st, ct) = (self.angle.sin(), self.angle.cos());
        let n = self.normal();
        let ff = self.friction_along(v);
        [
            ("mg", 0.0, -self.mass * self.g), // gravity, straight down
            ("N", -n * st, n * ct),           // normal, out of the slope (up-left)
            ("f", ff * ct, ff * st),          // friction, along the slope
        ]
    }
}
impl Sim for Ramp {
    fn state0(&self) -> Vec<f32> {
        vec![self.s0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        d[0] = s[1];
        d[1] = self.accel(s[1]);
        d[2] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * self.mass * s[1] * s[1],
            potential: self.mass * self.g * s[0] * self.angle.sin(), // height = s·sinθ
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[0] * self.angle.cos(), s[0] * self.angle.sin())
    }
    fn labels(&self) -> Vec<String> {
        vec!["s".into(), "v".into(), "t".into()]
    }
    fn phase_xy(&self) -> Option<(usize, usize)> {
        Some((0, 1))
    }
    // the "well" here is a straight ramp (linear PE), so no well view
}

/// Static/kinetic friction force (signed): kinetic opposes velocity; static
/// cancels the net force up to its budget, else breaks away to kinetic.
fn friction_switch(v: f32, fnet_nf: f32, n: f32, mu_k: f32, mu_s: f32) -> f32 {
    if v.abs() > 1e-3 {
        -v.signum() * mu_k * n
    } else if fnet_nf.abs() <= mu_s * n {
        -fnet_nf
    } else {
        -fnet_nf.signum() * mu_k * n
    }
}

/// A block on an **incline connected over a pulley to a hanging mass** (the
/// incline-Atwood). State = `[s, v, t]`, `s` = distance the incline block has
/// moved UP the slope (the hanging mass descends the same). With +s up-slope:
/// a = (m₂g − m₁g·sinθ − friction) / (m₁ + m₂). From the goldmine `pulley.js`.
pub struct InclinePulley {
    pub g: f32,
    pub angle: f32,
    pub m1: f32, // block on the incline
    pub m2: f32, // hanging mass
    pub mu_k: f32,
    pub mu_s: f32,
}
impl InclinePulley {
    fn accel(&self, v: f32) -> f32 {
        let n = self.m1 * self.g * self.angle.cos();
        let fnet_nf = self.m2 * self.g - self.m1 * self.g * self.angle.sin();
        (fnet_nf + friction_switch(v, fnet_nf, n, self.mu_k, self.mu_s)) / (self.m1 + self.m2)
    }
}
impl Sim for InclinePulley {
    fn state0(&self) -> Vec<f32> {
        vec![0.0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        d[0] = s[1];
        d[1] = self.accel(s[1]);
        d[2] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * (self.m1 + self.m2) * s[1] * s[1],
            // m₁ rises s·sinθ (gains PE); m₂ descends s (loses PE)
            potential: (self.m1 * self.angle.sin() - self.m2) * self.g * s[0],
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[0] * self.angle.cos(), s[0] * self.angle.sin())
    }
    fn labels(&self) -> Vec<String> {
        vec!["s".into(), "v".into(), "t".into()]
    }
    // monotone accel — no phase/well; energygraph shows the KE↔PE trade
}

/// Two blocks on a **wedge with two slopes**, connected over a pulley at the apex.
/// State = `[s, v, t]`, `s` = how far m₂ has slid DOWN the right slope (m₁ moves
/// up the left). With +s that way: a = (m₂g·sinθ₂ − m₁g·sinθ₁ − friction)/(m₁+m₂),
/// friction acting on the (rough) right slope. Matches the classic A–B–C wedge.
pub struct DoubleIncline {
    pub g: f32,
    pub a1: f32, // left slope
    pub a2: f32, // right slope (the rough one)
    pub m1: f32, // left block
    pub m2: f32, // right block
    pub mu_k: f32,
    pub mu_s: f32,
}
impl DoubleIncline {
    fn accel(&self, v: f32) -> f32 {
        let n2 = self.m2 * self.g * self.a2.cos();
        let fnet_nf = self.m2 * self.g * self.a2.sin() - self.m1 * self.g * self.a1.sin();
        (fnet_nf + friction_switch(v, fnet_nf, n2, self.mu_k, self.mu_s)) / (self.m1 + self.m2)
    }
}
impl Sim for DoubleIncline {
    fn state0(&self) -> Vec<f32> {
        vec![0.0, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        d[0] = s[1];
        d[1] = self.accel(s[1]);
        d[2] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * (self.m1 + self.m2) * s[1] * s[1],
            // m₁ rises s·sinθ₁ (gains), m₂ descends s·sinθ₂ (loses)
            potential: (self.m1 * self.a1.sin() - self.m2 * self.a2.sin()) * self.g * s[0],
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[0], 0.0)
    }
    fn labels(&self) -> Vec<String> {
        vec!["s".into(), "v".into(), "t".into()]
    }
}

/// A block **sliding down an incline into a spring bumper** at the base (one-sided
/// contact — the spring only pushes while touched). State = `[s, v, t]`, `s` =
/// distance up the slope from the base. Free slide (a = −g·sinθ) until `s` drops
/// below the spring's free end `s_contact`, where the spring pushes back with
/// (k/m)·(s_contact − s); it compresses, then launches the block back up.
pub struct InclineBumper {
    pub g: f32,
    pub angle: f32,
    pub m: f32,
    pub k: f32,
    pub mu_k: f32,
    pub s_contact: f32, // spring free-end position along the slope
    pub s0: f32,        // release position (up the slope)
}
impl Sim for InclineBumper {
    fn state0(&self) -> Vec<f32> {
        vec![self.s0, 0.0, 0.0]
    }
    fn deriv(&self, st: &[f32], d: &mut [f32]) {
        let (s, v) = (st[0], st[1]);
        let mut a = -self.g * self.angle.sin(); // gravity, down the slope
        if s < self.s_contact {
            a += (self.k / self.m) * (self.s_contact - s); // spring push, up-slope
        }
        if self.mu_k > 0.0 && v.abs() > 1e-3 {
            a -= self.mu_k * self.g * self.angle.cos() * v.signum();
        }
        d[0] = v;
        d[1] = a;
        d[2] = 1.0;
    }
    fn energy(&self, st: &[f32]) -> Energy {
        let s = st[0];
        let spring = if s < self.s_contact {
            0.5 * self.k * (self.s_contact - s).powi(2)
        } else {
            0.0
        };
        Energy {
            kinetic: 0.5 * self.m * st[1] * st[1],
            potential: self.m * self.g * s * self.angle.sin() + spring,
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[0] * self.angle.cos(), s[0] * self.angle.sin())
    }
    fn labels(&self) -> Vec<String> {
        vec!["s".into(), "v".into(), "t".into()]
    }
}

/// Three masses in a chain joined by two springs, on an incline — coupled
/// oscillators. State = `[x₁,v₁,x₂,v₂,x₃,v₃,t]`, positions ALONG the slope. A
/// uniform gravity component acts equally on all three, so it never changes the
/// internal (relative) motion — the chain is shown in the incline's frame (centre
/// of mass held). The classic normal-modes demo, extended from `doublespring`.
pub struct SpringChain {
    pub m: f32,
    pub k: f32,
    pub rest: f32,
}
impl Sim for SpringChain {
    fn state0(&self) -> Vec<f32> {
        // equilibrium spacing `rest`, with the left mass pulled out to seed motion
        vec![-self.rest - 0.6, 0.0, 0.0, 0.0, self.rest, 0.0, 0.0]
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        let f12 = self.k * (s[2] - s[0] - self.rest);
        let f23 = self.k * (s[4] - s[2] - self.rest);
        d[0] = s[1];
        d[1] = f12 / self.m;
        d[2] = s[3];
        d[3] = (-f12 + f23) / self.m;
        d[4] = s[5];
        d[5] = -f23 / self.m;
        d[6] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        Energy {
            kinetic: 0.5 * self.m * (s[1] * s[1] + s[3] * s[3] + s[5] * s[5]),
            potential: 0.5
                * self.k
                * ((s[2] - s[0] - self.rest).powi(2) + (s[4] - s[2] - self.rest).powi(2)),
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[2], 0.0) // the middle mass
    }
    fn labels(&self) -> Vec<String> {
        vec![
            "x₁".into(),
            "v₁".into(),
            "x₂".into(),
            "v₂".into(),
            "x₃".into(),
            "v₃".into(),
            "t".into(),
        ]
    }
    // coupled oscillators — energygraph shows the KE↔PE exchange (beating)
}

/// A **wave on a string**: N interior masses joined by springs to their
/// neighbours, both ends fixed — the discretised wave equation. State =
/// `[y₁,v₁,…,y_N,v_N,t]` (transverse displacements). A pluck (triangular initial
/// shape) splits into two pulses that travel out, reflect (inverting) off the
/// fixed ends, and recombine. RK4-clean — just a big state vector.
pub struct StringWave {
    pub n: usize,
    pub k: f32,
    pub m: f32,
    pub damping: f32,
    pub pluck: f32, // 0..1, where along the string the initial peak sits
}
impl StringWave {
    fn y0(&self, i: usize) -> f32 {
        // triangular pluck rising to `pluck`, then falling to the far end
        let x = i as f32 / (self.n as f32 + 1.0);
        if x <= self.pluck {
            x / self.pluck
        } else {
            (1.0 - x) / (1.0 - self.pluck)
        }
    }
}
impl Sim for StringWave {
    fn state0(&self) -> Vec<f32> {
        let mut st = vec![0.0; 2 * self.n + 1];
        for i in 1..=self.n {
            st[(i - 1) * 2] = self.y0(i);
        }
        st
    }
    fn deriv(&self, s: &[f32], d: &mut [f32]) {
        for i in 1..=self.n {
            let (yi, vi) = (s[(i - 1) * 2], s[(i - 1) * 2 + 1]);
            let yl = if i == 1 { 0.0 } else { s[(i - 2) * 2] };
            let yr = if i == self.n { 0.0 } else { s[i * 2] };
            d[(i - 1) * 2] = vi;
            d[(i - 1) * 2 + 1] = (self.k * (yl - 2.0 * yi + yr) - self.damping * vi) / self.m;
        }
        d[2 * self.n] = 1.0;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        let (mut ke, mut pe, mut prev) = (0.0, 0.0, 0.0);
        for i in 1..=self.n {
            let (yi, vi) = (s[(i - 1) * 2], s[(i - 1) * 2 + 1]);
            ke += 0.5 * self.m * vi * vi;
            pe += 0.5 * self.k * (yi - prev).powi(2);
            prev = yi;
        }
        pe += 0.5 * self.k * prev.powi(2); // segment to the fixed right end
        Energy {
            kinetic: ke,
            potential: pe,
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (0.0, s[(self.n / 2).max(1).saturating_sub(1) * 2])
    }
    fn labels(&self) -> Vec<String> {
        vec!["y".into(), "v".into(), "t".into()]
    }
    // a standing/travelling wave — energygraph shows KE↔PE; no phase/well
}

/// One of the racing descent curves for the brachistochrone. Each provides
/// `y(x)`, `y'(x)`, `y''(x)` on `x ∈ [0, D]`, with **y measured downward** (so
/// gravity acts in +y). The cycloid — the true brachistochrone — is built by a
/// Newton solve for its parameter range and inverted through a lookup table.
#[derive(Clone)]
pub enum Curve {
    Straight { m: f32 },
    Parabola { h: f32, d: f32 },
    Circle { r: f32 },
    Cycloid { r: f32, table: Vec<(f32, f32)> }, // (x, θ) samples
}
impl Curve {
    fn y(&self, x: f32) -> f32 {
        match self {
            Curve::Straight { m } => m * x,
            Curve::Parabola { h, d } => h * (1.0 - (1.0 - x / d).powi(2)),
            Curve::Circle { r } => r - (r * r - x * x).max(0.0).sqrt(),
            Curve::Cycloid { r, table, .. } => {
                let th = theta_from_x(x, table);
                r * (1.0 - th.cos())
            }
        }
    }
    fn dy(&self, x: f32) -> f32 {
        match self {
            Curve::Straight { m } => *m,
            Curve::Parabola { h, d } => 2.0 * h * (1.0 - x / d) / d,
            Curve::Circle { r } => x / (r * r - x * x).max(0.01).sqrt(),
            Curve::Cycloid { table, .. } => {
                let th = theta_from_x(x, table);
                (th.sin() / (1.0 - th.cos()).max(1e-6)).clamp(-2000.0, 2000.0)
            }
        }
    }
    fn d2y(&self, x: f32) -> f32 {
        match self {
            Curve::Straight { .. } => 0.0,
            Curve::Parabola { h, d } => -2.0 * h / (d * d),
            Curve::Circle { r } => {
                let sq = (r * r - x * x).max(0.01);
                r * r / (sq * sq.sqrt())
            }
            Curve::Cycloid { r, table, .. } => {
                let th = theta_from_x(x, table);
                (-1.0 / (r * (1.0 - th.cos()).max(1e-6).powi(2))).max(-1e6)
            }
        }
    }
}

/// Invert `x = r(θ − sinθ)` via a monotone `(x, θ)` table (linear interpolation).
fn theta_from_x(x: f32, table: &[(f32, f32)]) -> f32 {
    if x <= table[0].0 {
        return table[0].1;
    }
    let last = table[table.len() - 1];
    if x >= last.0 {
        return last.1;
    }
    for w in table.windows(2) {
        let (x0, t0) = w[0];
        let (x1, t1) = w[1];
        if x <= x1 {
            let f = (x - x0) / (x1 - x0 + 1e-12);
            return t0 + f * (t1 - t0);
        }
    }
    last.1
}

/// Build the cycloid through (0,0)→(d,h): Newton-solve θ_end so `r(θ_end−sinθ_end)=d`
/// with `r = h/(1−cosθ_end)`, then sample an `(x, θ)` inversion table.
fn build_cycloid(d: f32, h: f32) -> Curve {
    use std::f32::consts::{PI, TAU};
    let mut theta_end = PI;
    for _ in 0..20 {
        let r = h / (1.0 - theta_end.cos());
        let x_end = r * (theta_end - theta_end.sin());
        let dx = r * (1.0 - theta_end.cos());
        theta_end -= (x_end - d) / dx;
        theta_end = theta_end.clamp(0.1, TAU);
    }
    let r = h / (1.0 - theta_end.cos());
    let n = 200usize;
    let table: Vec<(f32, f32)> = (0..=n)
        .map(|i| {
            let th = theta_end * i as f32 / n as f32;
            (r * (th - th.sin()), th)
        })
        .collect();
    Curve::Cycloid { r, table }
}

/// A **bead sliding on a wire** under gravity — the brachistochrone racer. State
/// = `[x, ẋ, t]` in the horizontal coordinate x; the constrained equation of
/// motion is ẍ = (g·f′ − 2ẋ²·f′·f″)/(1+f′²) − b·ẋ for the curve `y = f(x)`. Four
/// beads on four curves race from A=(0,0) to B=(D,H); the cycloid wins. From
/// `sims/brachistochrone.js`.
pub struct Bead {
    pub g: f32,
    pub b: f32, // damping
    pub d: f32, // horizontal span D
    pub h: f32, // vertical drop H
    pub curve: Curve,
}
impl Sim for Bead {
    fn state0(&self) -> Vec<f32> {
        vec![0.02, 0.0, 0.0] // start just off the origin to dodge the cusp singularity
    }
    fn deriv(&self, s: &[f32], dd: &mut [f32]) {
        let (x, v) = (s[0], s[1]);
        dd[2] = 1.0;
        if x >= self.d {
            dd[0] = 0.0;
            dd[1] = 0.0;
            return;
        }
        let fp = self.curve.dy(x);
        let fpp = self.curve.d2y(x);
        let denom = 1.0 + fp * fp;
        dd[0] = v;
        dd[1] = (self.g * fp - 2.0 * v * v * fp * fpp) / denom - self.b * v;
    }
    fn energy(&self, s: &[f32]) -> Energy {
        let (x, v) = (s[0], s[1]);
        let fp = self.curve.dy(x);
        Energy {
            kinetic: 0.5 * v * v * (1.0 + fp * fp), // true along-curve speed²
            potential: self.g * (self.h - self.curve.y(x)),
        }
    }
    fn body(&self, s: &[f32]) -> (f32, f32) {
        (s[0], -self.curve.y(s[0])) // f is y-down → negate for the y-up ctor mapping
    }
    fn labels(&self) -> Vec<String> {
        vec!["x".into(), "v".into(), "t".into()]
    }
    // several beads under one id — no per-bead phase/well
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
        vel_tip.push(Vec2::new(
            b.x + vx * unit * VEL_SCALE,
            b.y - vy * unit * VEL_SCALE,
        ));
        ke_tip.push(Vec2::new(
            ke_x,
            e_base - (e.kinetic / e_max).clamp(0.0, 1.0) * BAR_MAX,
        ));
        pe_tip.push(Vec2::new(
            pe_x,
            e_base - (e.potential / e_max).clamp(0.0, 1.0) * BAR_MAX,
        ));
        energy_series.push((e.kinetic, e.potential));
    }

    let mut vel = Entity::new(
        format!("{id}.vel"),
        Shape::Arrow { to: vel_tip[0] },
        body_pts[0],
        style::GOLD,
    );
    vel.stroke.width = 3.0;
    vel.tags = overlay_tags.to_vec();
    s.add(vel);
    let mut ke = Entity::new(
        format!("{id}.ke"),
        Shape::Line { to: ke_tip[0] },
        Vec2::new(ke_x, e_base),
        style::CYAN,
    );
    ke.stroke.width = 12.0;
    ke.tags = overlay_tags.to_vec();
    s.add(ke);
    let mut pe = Entity::new(
        format!("{id}.pe"),
        Shape::Line { to: pe_tip[0] },
        Vec2::new(pe_x, e_base),
        style::MAGENTA,
    );
    pe.stroke.width = 12.0;
    pe.tags = overlay_tags.to_vec();
    s.add(pe);
    for (lid, lx, txt, col) in [
        (format!("{id}.kelbl"), ke_x, "KE", style::CYAN),
        (format!("{id}.pelbl"), pe_x, "PE", style::MAGENTA),
    ] {
        let mut lbl = Entity::new(
            lid,
            Shape::Text {
                content: txt.to_string(),
                size: 16.0,
            },
            Vec2::new(lx, e_base + 18.0),
            col,
        );
        lbl.tags = overlay_tags.to_vec();
        s.add(lbl);
    }

    let tracks = vec![
        PlaybackTrack {
            id: format!("{id}.vel"),
            prop: Prop::Pos,
            points: body_pts.to_vec(),
        },
        PlaybackTrack {
            id: format!("{id}.vel"),
            prop: Prop::To,
            points: vel_tip,
        },
        PlaybackTrack {
            id: format!("{id}.ke"),
            prop: Prop::To,
            points: ke_tip,
        },
        PlaybackTrack {
            id: format!("{id}.pe"),
            prop: Prop::To,
            points: pe_tip,
        },
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
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 200.0)
    };
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
        Shape::Polyline {
            pts: bob_pts.clone(),
        },
        Vec2::ZERO,
        style::DIM,
    );
    arc.stroke.width = 2.0;
    arc.opacity = 0.35;
    arc.tags = core_tags();
    s.add(arc);

    // rod: pivot → bob
    let mut rod = Entity::new(
        format!("{id}.rod"),
        Shape::Line { to: bob0 },
        center,
        style::FG,
    );
    rod.stroke.width = 3.0;
    rod.tags = core_tags();
    s.add(rod);

    // pivot dot
    let mut pivot = Entity::new(
        format!("{id}.pivot"),
        Shape::Circle { r: 6.0 },
        center,
        style::DIM,
    );
    pivot.stroke.fill = true;
    pivot.stroke.outline = false;
    pivot.tags = core_tags();
    s.add(pivot);

    // bob
    let mut bob = Entity::new(
        format!("{id}.bob"),
        Shape::Circle { r: 16.0 },
        bob0,
        style::CYAN,
    );
    bob.stroke.fill = true;
    bob.stroke.outline = false;
    bob.tags = core_tags();
    s.add(bob);

    // velocity arrow + KE/PE energy bars (shared across all sims)
    let (energy_series, overlay_tracks) = add_overlays(
        s,
        &id,
        center,
        unit,
        e_max,
        &bob_pts,
        &vel_world,
        &energies,
        &overlay_tags,
    );

    let mut playback = vec![
        PlaybackTrack {
            id: format!("{id}.bob"),
            prop: Prop::Pos,
            points: bob_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.rod"),
            prop: Prop::To,
            points: bob_pts,
        },
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

/// `forces(id, [dur])` — reveal a sim's **free-body force diagram**: the force
/// vectors on the body (for `ramp`: gravity `mg`, normal `N`, friction `f`, and
/// the acceleration `a`), which then ride the body during `run`. The arrows are
/// laid out hidden by the sim ctor; this fades them in over `dur` (default 0.6).
fn v_forces(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    let dur = a.opt_num(1)?.unwrap_or(0.6).max(0.05);
    let tag = format!("{id}.forces");
    let ids: Vec<String> = s
        .entities
        .iter()
        .filter(|e| e.tags.iter().any(|t| *t == tag))
        .map(|e| e.id.clone())
        .collect();
    if ids.is_empty() {
        return Err(Error::new(
            format!("sim `{id}` has no force diagram — only `ramp` provides one (call the sim ctor first)"),
            a.span_of(0),
        ));
    }
    let tracks = ids
        .into_iter()
        .map(|eid| TrackSpec {
            id: eid,
            prop: Prop::Opacity,
            target: TargetValue::Abs(Value::F(1.0)),
            start: 0.0,
            dur,
            easing: Easing::OutQuad,
        })
        .collect();
    Ok(Clip {
        tracks,
        events: vec![],
        dur,
    })
}

/// `run(id, [dur])` (alias `swing`) — replay a sim's pre-simulated motion over
/// `dur` seconds (default 6): every part + view marker animates along it. A
/// keyframed replay (one segment per frame) of every stored [`PlaybackTrack`].
fn v_play(s: &Scene, a: &Args) -> Result<Clip, Error> {
    let id = a.ident(0)?;
    // `run` is shared: the grid kit replays its pre-simulated CA/WFC frames
    if s.grids.contains_key(&id) {
        return crate::kits::grid::replay(s, a);
    }
    // `run` is shared: a creator-kit quiz emits its own ask→countdown→reveal beat
    if s.quizzes.contains_key(&id) {
        return crate::kits::creator::build_quiz_clip(s, &id, a.opt_num(1)?, a.span_of(1));
    }
    if s.timings.contains_key(&id) {
        return crate::kits::creator::build_generic_timing_clip(
            s,
            &id,
            a.opt_num(1)?,
            a.span_of(1),
        );
    }
    let sim = s.sims.get(&id).ok_or_else(|| {
        Error::new(
            format!("no sim `{id}` — call `pendulum(...)` (or another sim) first"),
            a.span_of(0),
        )
    })?;
    let frames = sim
        .playback
        .iter()
        .map(|p| p.points.len())
        .max()
        .unwrap_or(0);
    if frames < 2 {
        return Err(Error::new(
            format!("`{id}` has no motion to swing"),
            a.span_of(0),
        ));
    }
    let dur = a.opt_num(1)?.unwrap_or(6.0).max(0.1);
    let frame = dur / (frames - 1) as f32;
    let mut tracks = Vec::new();
    for pt in &sim.playback {
        for k in 1..pt.points.len() {
            // scalar props (a counter value, opacity, …) ride the point's x channel;
            // Vec2 props (position, endpoint) use the whole point
            let target = match pt.prop {
                Prop::Value | Prop::Opacity | Prop::Scale | Prop::Rot | Prop::Trace | Prop::Hue => {
                    TargetValue::Abs(Value::F(pt.points[k].x))
                }
                _ => TargetValue::Abs(Value::V(pt.points[k])),
            };
            tracks.push(TrackSpec {
                id: pt.id.clone(),
                prop: pt.prop,
                target,
                start: (k - 1) as f32 * frame,
                dur: frame,
                easing: Easing::Linear,
            });
        }
    }
    Ok(Clip {
        tracks,
        events: vec![],
        dur,
    })
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
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 320.0)
    };
    let k = a.opt_num(2)?.unwrap_or(10.0).max(0.1);
    let x0 = a.opt_num(3)?.unwrap_or(1.3);
    let unit = a.opt_num(4)?.unwrap_or(110.0);
    let damping = a.opt_num(5)?.unwrap_or(0.0).max(0.0);

    let sp = Spring {
        k,
        mass: 1.0,
        damping,
        x0,
    };
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
        Shape::Line {
            to: Vec2::new(wall_x, center.y + 34.0),
        },
        Vec2::new(wall_x, center.y - 34.0),
        style::DIM,
    );
    wall.stroke.width = 5.0;
    wall.tags = core_tags();
    s.add(wall);
    // spring: wall → mass, drawn as a real coil that stretches with the motion
    let mut spring = Entity::new(
        format!("{id}.spring"),
        Shape::Coil {
            to: mass0,
            turns: 12,
        },
        Vec2::new(wall_x, center.y),
        style::LIME,
    );
    spring.stroke.width = 3.0;
    spring.tags = core_tags();
    s.add(spring);
    // the mass block
    let mut mass = Entity::new(
        format!("{id}.mass"),
        Shape::Rect { w: 40.0, h: 40.0 },
        mass0,
        style::CYAN,
    );
    mass.stroke.fill = true;
    mass.stroke.outline = false;
    mass.tags = core_tags();
    s.add(mass);
    // faint range-of-motion path
    let mut path = Entity::new(
        format!("{id}.path"),
        Shape::Polyline {
            pts: mass_pts.clone(),
        },
        Vec2::ZERO,
        style::DIM,
    );
    path.stroke.width = 2.0;
    path.opacity = 0.3;
    path.tags = core_tags();
    s.add(path);

    let (energy_series, overlay_tracks) = add_overlays(
        s,
        &id,
        center,
        unit,
        e_max,
        &mass_pts,
        &vel_world,
        &energies,
        &overlay_tags,
    );

    let mut playback = vec![
        PlaybackTrack {
            id: format!("{id}.mass"),
            prop: Prop::Pos,
            points: mass_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.spring"),
            prop: Prop::To,
            points: mass_pts,
        },
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
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 240.0)
    };
    let a1 = a.opt_num(2)?.unwrap_or(90.0).to_radians();
    let a2 = a.opt_num(3)?.unwrap_or(90.0).to_radians();
    let unit = a.opt_num(4)?.unwrap_or(110.0);

    let dp = DoublePendulum {
        g: 9.8,
        l1: 1.0,
        l2: 1.0,
        m1: 2.0,
        m2: 2.0,
        a1,
        a2,
    };
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
    let mut trail = Entity::new(
        format!("{id}.path"),
        Shape::Polyline {
            pts: bob2_pts.clone(),
        },
        Vec2::ZERO,
        style::MAGENTA,
    );
    trail.stroke.width = 2.0;
    trail.opacity = 0.6;
    trail.tags = core_tags();
    s.add(trail);
    // pivot
    let mut pivot = Entity::new(
        format!("{id}.pivot"),
        Shape::Circle { r: 6.0 },
        center,
        style::DIM,
    );
    pivot.stroke.fill = true;
    pivot.stroke.outline = false;
    pivot.tags = core_tags();
    s.add(pivot);
    // arm 1: pivot → bob1
    let mut rod1 = Entity::new(
        format!("{id}.rod1"),
        Shape::Line { to: bob1_0 },
        center,
        style::FG,
    );
    rod1.stroke.width = 3.0;
    rod1.tags = core_tags();
    s.add(rod1);
    // arm 2: bob1 → bob2 (both ends move)
    let mut rod2 = Entity::new(
        format!("{id}.rod2"),
        Shape::Line { to: bob2_0 },
        bob1_0,
        style::FG,
    );
    rod2.stroke.width = 3.0;
    rod2.tags = core_tags();
    s.add(rod2);
    // bobs
    let mut bob1 = Entity::new(
        format!("{id}.bob1"),
        Shape::Circle { r: 12.0 },
        bob1_0,
        style::CYAN,
    );
    bob1.stroke.fill = true;
    bob1.stroke.outline = false;
    bob1.tags = core_tags();
    s.add(bob1);
    let mut bob2 = Entity::new(
        format!("{id}.bob2"),
        Shape::Circle { r: 14.0 },
        bob2_0,
        style::LIME,
    );
    bob2.stroke.fill = true;
    bob2.stroke.outline = false;
    bob2.tags = core_tags();
    s.add(bob2);

    let (energy_series, overlay_tracks) = add_overlays(
        s,
        &id,
        center,
        unit,
        e_max,
        &bob2_pts,
        &vel_world,
        &energies,
        &overlay_tags,
    );

    let mut playback = vec![
        PlaybackTrack {
            id: format!("{id}.bob1"),
            prop: Prop::Pos,
            points: bob1_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.bob2"),
            prop: Prop::Pos,
            points: bob2_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.rod1"),
            prop: Prop::To,
            points: bob1_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.rod2"),
            prop: Prop::Pos,
            points: bob1_pts,
        },
        PlaybackTrack {
            id: format!("{id}.rod2"),
            prop: Prop::To,
            points: bob2_pts,
        },
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
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 240.0)
    };
    let a0 = a.opt_num(2)?.unwrap_or(30.0).to_radians();
    let stretch0 = a.opt_num(3)?.unwrap_or(0.3);
    let unit = a.opt_num(4)?.unwrap_or(110.0);
    let damping = a.opt_num(5)?.unwrap_or(0.1).max(0.0);
    let sp = SpringPendulum {
        g: 9.81,
        k: 40.0,
        l0: 1.5,
        m: 1.0,
        damping,
        a0,
        stretch0,
    };
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
    let mut trail = Entity::new(
        format!("{id}.path"),
        Shape::Polyline {
            pts: bob_pts.clone(),
        },
        Vec2::ZERO,
        style::DIM,
    );
    trail.stroke.width = 2.0;
    trail.opacity = 0.35;
    trail.tags = core_tags();
    s.add(trail);
    let mut spring = Entity::new(
        format!("{id}.spring"),
        Shape::Coil {
            to: bob0,
            turns: 10,
        },
        center,
        style::LIME,
    );
    spring.stroke.width = 3.0;
    spring.tags = core_tags();
    s.add(spring);
    let mut pivot = Entity::new(
        format!("{id}.pivot"),
        Shape::Circle { r: 6.0 },
        center,
        style::DIM,
    );
    pivot.stroke.fill = true;
    pivot.stroke.outline = false;
    pivot.tags = core_tags();
    s.add(pivot);
    let mut bob = Entity::new(
        format!("{id}.bob"),
        Shape::Circle { r: 15.0 },
        bob0,
        style::CYAN,
    );
    bob.stroke.fill = true;
    bob.stroke.outline = false;
    bob.tags = core_tags();
    s.add(bob);
    let (energy_series, overlay_tracks) = add_overlays(
        s,
        &id,
        center,
        unit,
        e_max,
        &bob_pts,
        &vel_world,
        &energies,
        &overlay_tags,
    );
    let mut playback = vec![
        PlaybackTrack {
            id: format!("{id}.bob"),
            prop: Prop::Pos,
            points: bob_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.spring"),
            prop: Prop::To,
            points: bob_pts,
        },
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

/// `kapitza(id, [center], [angle0], [vibeamp], [unit])` — a pendulum whose pivot
/// vibrates fast enough that the INVERTED position becomes stable. `angle0` in
/// degrees (default 165, near inverted), `vibeamp` the drive strength (default 220).
fn c_kapitza(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 400.0)
    };
    let a0 = a.opt_num(2)?.unwrap_or(165.0).to_radians();
    let vibe_amp = a.opt_num(3)?.unwrap_or(220.0).max(0.0);
    let unit = a.opt_num(4)?.unwrap_or(150.0);
    let kp = Kapitza {
        g: 9.81,
        l: 1.0,
        m: 1.0,
        damping: 0.1,
        vibe_amp,
        vibe_freq: 30.0,
        a0,
    };
    let (sim_dt, substeps) = (0.002f32, 12usize); // fast vibration ⇒ fine dt
    let states = simulate(&kp, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let pivot_pts: Vec<Vec2> = states
        .iter()
        .map(|st| to_screen((0.0, kp.pivot_y(st[2]))))
        .collect();
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
    let mut trail = Entity::new(
        format!("{id}.path"),
        Shape::Polyline {
            pts: bob_pts.clone(),
        },
        Vec2::ZERO,
        style::DIM,
    );
    trail.stroke.width = 2.0;
    trail.opacity = 0.3;
    trail.tags = core_tags();
    s.add(trail);
    let mut rod = Entity::new(
        format!("{id}.rod"),
        Shape::Line { to: bob0 },
        pivot0,
        style::FG,
    );
    rod.stroke.width = 3.0;
    rod.tags = core_tags();
    s.add(rod);
    let mut pivot = Entity::new(
        format!("{id}.pivot"),
        Shape::Circle { r: 7.0 },
        pivot0,
        style::GOLD,
    );
    pivot.stroke.fill = true;
    pivot.stroke.outline = false;
    pivot.tags = core_tags();
    s.add(pivot);
    let mut bob = Entity::new(
        format!("{id}.bob"),
        Shape::Circle { r: 15.0 },
        bob0,
        style::CYAN,
    );
    bob.stroke.fill = true;
    bob.stroke.outline = false;
    bob.tags = core_tags();
    s.add(bob);
    let (energy_series, overlay_tracks) = add_overlays(
        s,
        &id,
        center,
        unit,
        e_max,
        &bob_pts,
        &vel_world,
        &energies,
        &overlay_tags,
    );
    let mut playback = vec![
        PlaybackTrack {
            id: format!("{id}.pivot"),
            prop: Prop::Pos,
            points: pivot_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.rod"),
            prop: Prop::Pos,
            points: pivot_pts,
        },
        PlaybackTrack {
            id: format!("{id}.rod"),
            prop: Prop::To,
            points: bob_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.bob"),
            prop: Prop::Pos,
            points: bob_pts,
        },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: kp.labels(),
            phase_xy: kp.phase_xy(),
            pos_var: kp.pos_var(),
            well: kp.well_curve(),
            energy: energy_series,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `cartpendulum(id, [center], [angle0], [unit])` — a pendulum on a spring-mounted
/// cart rolling on a track (the classic control system). `angle0` in degrees.
fn c_cartpendulum(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 340.0)
    };
    let a0 = a.opt_num(2)?.unwrap_or(45.0).to_radians();
    let unit = a.opt_num(3)?.unwrap_or(110.0);
    let cp = CartPendulum {
        g: 9.8,
        l: 1.0,
        mcart: 1.0,
        mbob: 1.0,
        k: 6.0,
        cart_damp: 0.0,
        bob_damp: 0.0,
        a0,
    };
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
    let mut track = Entity::new(
        format!("{id}.track"),
        Shape::Line {
            to: Vec2::new(center.x + 230.0, center.y + 22.0),
        },
        Vec2::new(wall_x, center.y + 22.0),
        style::DIM,
    );
    track.stroke.width = 2.0;
    track.opacity = 0.5;
    track.tags = core_tags();
    s.add(track);
    let mut wall = Entity::new(
        format!("{id}.wall"),
        Shape::Line {
            to: Vec2::new(wall_x, center.y + 22.0),
        },
        Vec2::new(wall_x, center.y - 34.0),
        style::DIM,
    );
    wall.stroke.width = 5.0;
    wall.tags = core_tags();
    s.add(wall);
    let mut spring = Entity::new(
        format!("{id}.spring"),
        Shape::Coil {
            to: cart0,
            turns: 10,
        },
        Vec2::new(wall_x, center.y),
        style::LIME,
    );
    spring.stroke.width = 3.0;
    spring.tags = core_tags();
    s.add(spring);
    let mut cart = Entity::new(
        format!("{id}.cart"),
        Shape::Rect { w: 52.0, h: 34.0 },
        cart0,
        style::PANEL,
    );
    cart.stroke.fill = true;
    cart.stroke.outline = true;
    cart.tags = core_tags();
    s.add(cart);
    let mut rod = Entity::new(
        format!("{id}.rod"),
        Shape::Line { to: bob0 },
        cart0,
        style::FG,
    );
    rod.stroke.width = 3.0;
    rod.tags = core_tags();
    s.add(rod);
    let mut bob = Entity::new(
        format!("{id}.bob"),
        Shape::Circle { r: 14.0 },
        bob0,
        style::CYAN,
    );
    bob.stroke.fill = true;
    bob.stroke.outline = false;
    bob.tags = core_tags();
    s.add(bob);
    let (energy_series, overlay_tracks) = add_overlays(
        s,
        &id,
        center,
        unit,
        e_max,
        &bob_pts,
        &vel_world,
        &energies,
        &overlay_tags,
    );
    let mut playback = vec![
        PlaybackTrack {
            id: format!("{id}.cart"),
            prop: Prop::Pos,
            points: cart_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.spring"),
            prop: Prop::To,
            points: cart_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.rod"),
            prop: Prop::Pos,
            points: cart_pts,
        },
        PlaybackTrack {
            id: format!("{id}.rod"),
            prop: Prop::To,
            points: bob_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.bob"),
            prop: Prop::Pos,
            points: bob_pts,
        },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: cp.labels(),
            phase_xy: cp.phase_xy(),
            pos_var: cp.pos_var(),
            well: cp.well_curve(),
            energy: energy_series,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `comparependulum(id, [center], [angle0], [unit])` — two driven-damped pendulums
/// started ≈0.001 rad apart: sensitive dependence, they diverge completely.
fn c_comparependulum(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 240.0)
    };
    let a0 = a.opt_num(2)?.unwrap_or(10.0).to_radians();
    let unit = a.opt_num(3)?.unwrap_or(130.0);
    let cmp = ComparePendulum {
        g: 9.81,
        l: 1.0,
        m: 1.0,
        damping: 0.5,
        drive_amp: 10.0,
        drive_freq: 0.667,
        a0,
        delta: 0.001,
    };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&cmp, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let a_pts: Vec<Vec2> = states.iter().map(|st| to_screen(cmp.bob_a(st))).collect();
    let b_pts: Vec<Vec2> = states.iter().map(|st| to_screen(cmp.bob_b(st))).collect();
    let energy_series: Vec<(f32, f32)> = states
        .iter()
        .map(|st| {
            let e = cmp.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();
    let (a0p, b0p) = (a_pts[0], b_pts[0]);
    let parts = format!("{id}.parts");
    let core_tags = || vec![id.clone(), parts.clone()];
    let mut ta = Entity::new(
        format!("{id}.pathA"),
        Shape::Polyline { pts: a_pts.clone() },
        Vec2::ZERO,
        style::CYAN,
    );
    ta.stroke.width = 2.0;
    ta.opacity = 0.25;
    ta.tags = core_tags();
    s.add(ta);
    let mut tb = Entity::new(
        format!("{id}.pathB"),
        Shape::Polyline { pts: b_pts.clone() },
        Vec2::ZERO,
        style::MAGENTA,
    );
    tb.stroke.width = 2.0;
    tb.opacity = 0.25;
    tb.tags = core_tags();
    s.add(tb);
    let mut pivot = Entity::new(
        format!("{id}.pivot"),
        Shape::Circle { r: 6.0 },
        center,
        style::DIM,
    );
    pivot.stroke.fill = true;
    pivot.stroke.outline = false;
    pivot.tags = core_tags();
    s.add(pivot);
    let mut rod_a = Entity::new(
        format!("{id}.rodA"),
        Shape::Line { to: a0p },
        center,
        style::CYAN,
    );
    rod_a.stroke.width = 3.0;
    rod_a.tags = core_tags();
    s.add(rod_a);
    let mut bob_a = Entity::new(
        format!("{id}.bobA"),
        Shape::Circle { r: 13.0 },
        a0p,
        style::CYAN,
    );
    bob_a.stroke.fill = true;
    bob_a.stroke.outline = false;
    bob_a.tags = core_tags();
    s.add(bob_a);
    let mut rod_b = Entity::new(
        format!("{id}.rodB"),
        Shape::Line { to: b0p },
        center,
        style::MAGENTA,
    );
    rod_b.stroke.width = 3.0;
    rod_b.tags = core_tags();
    s.add(rod_b);
    let mut bob_b = Entity::new(
        format!("{id}.bobB"),
        Shape::Circle { r: 13.0 },
        b0p,
        style::MAGENTA,
    );
    bob_b.stroke.fill = true;
    bob_b.stroke.outline = false;
    bob_b.tags = core_tags();
    s.add(bob_b);
    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.rodA"),
            prop: Prop::To,
            points: a_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.bobA"),
            prop: Prop::Pos,
            points: a_pts,
        },
        PlaybackTrack {
            id: format!("{id}.rodB"),
            prop: Prop::To,
            points: b_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.bobB"),
            prop: Prop::Pos,
            points: b_pts,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: cmp.labels(),
            phase_xy: cmp.phase_xy(),
            pos_var: cmp.pos_var(),
            well: cmp.well_curve(),
            energy: energy_series,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `verticalspring(id, [center], [stretch0], [unit], [damping])` — a mass bobbing
/// on a vertical spring under gravity (coil drawn from an anchor above).
fn c_verticalspring(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 170.0)
    };
    let stretch0 = a.opt_num(2)?.unwrap_or(0.6);
    let unit = a.opt_num(3)?.unwrap_or(120.0);
    let damping = a.opt_num(4)?.unwrap_or(0.2).max(0.0);
    let vs = VerticalSpring {
        g: 9.81,
        k: 20.0,
        l0: 1.0,
        m: 1.0,
        damping,
        stretch0,
    };
    sim_spring_like(s, &id, center, unit, &vs, |st| vs.body_velocity(st), true)
}

/// `springincline(id, [center], [angle], [unit], [damping])` — a mass on a spring
/// on an inclined plane (`angle` in degrees). Coil + bob run down the ramp.
fn c_springincline(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(430.0, 190.0)
    };
    let angle = a.opt_num(2)?.unwrap_or(30.0).to_radians();
    let unit = a.opt_num(3)?.unwrap_or(120.0);
    let damping = a.opt_num(4)?.unwrap_or(0.3).max(0.0);
    let si = SpringIncline {
        g: 9.81,
        k: 20.0,
        l0: 1.5,
        m: 1.0,
        damping,
        angle,
        stretch0: 0.6,
    };
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
    let mut ramp = Entity::new(
        format!("{id}.ramp"),
        Shape::Line { to: far },
        center,
        style::DIM,
    );
    ramp.stroke.width = 3.0;
    ramp.opacity = 0.6;
    ramp.tags = core_tags();
    s.add(ramp);
    let mut trail = Entity::new(
        format!("{id}.path"),
        Shape::Polyline {
            pts: bob_pts.clone(),
        },
        Vec2::ZERO,
        style::DIM,
    );
    trail.stroke.width = 2.0;
    trail.opacity = 0.3;
    trail.tags = core_tags();
    s.add(trail);
    let mut spring = Entity::new(
        format!("{id}.spring"),
        Shape::Coil {
            to: bob_pts[0],
            turns: 10,
        },
        center,
        style::LIME,
    );
    spring.stroke.width = 3.0;
    spring.tags = core_tags();
    s.add(spring);
    let mut anchor = Entity::new(
        format!("{id}.anchor"),
        Shape::Circle { r: 6.0 },
        center,
        style::GOLD,
    );
    anchor.stroke.fill = true;
    anchor.stroke.outline = false;
    anchor.tags = core_tags();
    s.add(anchor);
    let mut bob = Entity::new(
        format!("{id}.bob"),
        Shape::Circle { r: 15.0 },
        bob_pts[0],
        style::CYAN,
    );
    bob.stroke.fill = true;
    bob.stroke.outline = false;
    bob.tags = core_tags();
    s.add(bob);
    let (energy_series, overlay_tracks) = add_overlays(
        s,
        &id,
        center,
        unit,
        e_max,
        &bob_pts,
        &vel_world,
        &energies,
        &overlay_tags,
    );
    let mut playback = vec![
        PlaybackTrack {
            id: format!("{id}.bob"),
            prop: Prop::Pos,
            points: bob_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.spring"),
            prop: Prop::To,
            points: bob_pts,
        },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: si.labels(),
            phase_xy: si.phase_xy(),
            pos_var: si.pos_var(),
            well: si.well_curve(),
            energy: energy_series,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `bungee(id, [center], [unit], [damping])` — a bungee jump: free-fall then a
/// one-sided elastic cord catches and bounces the jumper.
fn c_bungee(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 120.0)
    };
    let unit = a.opt_num(2)?.unwrap_or(28.0);
    let damping = a.opt_num(3)?.unwrap_or(50.0).max(0.0);
    let bg = Bungee {
        g: 9.81,
        cord: 4.0,
        k: 800.0,
        m: 70.0,
        damping,
    };
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
    let mut plat = Entity::new(
        format!("{id}.platform"),
        Shape::Line {
            to: Vec2::new(center.x + 70.0, center.y),
        },
        Vec2::new(center.x - 70.0, center.y),
        style::DIM,
    );
    plat.stroke.width = 5.0;
    plat.tags = core_tags();
    s.add(plat);
    // elastic cord platform → jumper
    let mut cord = Entity::new(
        format!("{id}.cord"),
        Shape::Line { to: jumper_pts[0] },
        center,
        style::LIME,
    );
    cord.stroke.width = 2.5;
    cord.tags = core_tags();
    s.add(cord);
    let mut jumper = Entity::new(
        format!("{id}.jumper"),
        Shape::Circle { r: 14.0 },
        jumper_pts[0],
        style::CYAN,
    );
    jumper.stroke.fill = true;
    jumper.stroke.outline = false;
    jumper.tags = core_tags();
    s.add(jumper);
    let (energy_series, overlay_tracks) = add_overlays(
        s,
        &id,
        center,
        unit,
        e_max,
        &jumper_pts,
        &vel_world,
        &energies,
        &overlay_tags,
    );
    let mut playback = vec![
        PlaybackTrack {
            id: format!("{id}.jumper"),
            prop: Prop::Pos,
            points: jumper_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.cord"),
            prop: Prop::To,
            points: jumper_pts,
        },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: bg.labels(),
            phase_xy: bg.phase_xy(),
            pos_var: bg.pos_var(),
            well: bg.well_curve(),
            energy: energy_series,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `resonance(id, [center], [drivefreq], [unit], [damping])` — a driven spring; a
/// drive frequency near the natural √(k/m) pumps the amplitude up (resonance).
fn c_resonance(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 300.0)
    };
    let drive_freq = a.opt_num(2)?.unwrap_or(3.8);
    let unit = a.opt_num(3)?.unwrap_or(90.0);
    let damping = a.opt_num(4)?.unwrap_or(0.3).max(0.0);
    let rs = Resonance {
        k: 16.0,
        m: 1.0,
        damping,
        drive_amp: 2.0,
        drive_freq,
    };
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
    let mut trail = Entity::new(
        format!("{id}.path"),
        Shape::Polyline {
            pts: body_pts.clone(),
        },
        Vec2::ZERO,
        style::DIM,
    );
    trail.stroke.width = 2.0;
    trail.opacity = 0.3;
    trail.tags = core_tags();
    s.add(trail);
    let mut coil = Entity::new(
        format!("{id}.spring"),
        Shape::Coil {
            to: body0,
            turns: 11,
        },
        anchor,
        style::LIME,
    );
    coil.stroke.width = 3.0;
    coil.tags = core_tags();
    s.add(coil);
    // a wall (horizontal) or anchor dot (vertical)
    if vertical {
        let mut anc = Entity::new(
            format!("{id}.anchor"),
            Shape::Circle { r: 6.0 },
            anchor,
            style::GOLD,
        );
        anc.stroke.fill = true;
        anc.stroke.outline = false;
        anc.tags = core_tags();
        s.add(anc);
    } else {
        let mut wall = Entity::new(
            format!("{id}.wall"),
            Shape::Line {
                to: Vec2::new(anchor.x, center.y + 34.0),
            },
            Vec2::new(anchor.x, center.y - 34.0),
            style::DIM,
        );
        wall.stroke.width = 5.0;
        wall.tags = core_tags();
        s.add(wall);
    }
    let mut mass = Entity::new(
        format!("{id}.mass"),
        Shape::Circle { r: 15.0 },
        body0,
        style::CYAN,
    );
    mass.stroke.fill = true;
    mass.stroke.outline = false;
    mass.tags = core_tags();
    s.add(mass);
    let (energy_series, overlay_tracks) = add_overlays(
        s,
        id,
        center,
        unit,
        e_max,
        &body_pts,
        &vel_world,
        &energies,
        &overlay_tags,
    );
    let mut playback = vec![
        PlaybackTrack {
            id: format!("{id}.mass"),
            prop: Prop::Pos,
            points: body_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.spring"),
            prop: Prop::To,
            points: body_pts,
        },
    ];
    playback.extend(overlay_tracks);
    s.sims.insert(
        id.to_string(),
        SimData {
            playback,
            labels: sim.labels(),
            phase_xy: sim.phase_xy(),
            pos_var: sim.pos_var(),
            well: sim.well_curve(),
            energy: energy_series,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `doublespring(id, [center], [unit])` — two masses coupled by three springs
/// between walls; energy sloshes between them (beating / normal modes).
fn c_doublespring(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(430.0, 320.0)
    };
    let unit = a.opt_num(2)?.unwrap_or(85.0);
    let ds = DoubleSpring {
        m1: 1.0,
        m2: 1.0,
        k: 20.0,
        r: 1.8,
        w1: 0.0,
        w2: 6.0,
        damping: 0.0,
        x1_0: 2.4,
        x2_0: 4.0,
    };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&ds, sim_dt, substeps, SAMPLES);
    let sx = |x: f32| Vec2::new(center.x + x * unit, center.y);
    let b1: Vec<Vec2> = states.iter().map(|st| sx(st[0])).collect();
    let b2: Vec<Vec2> = states.iter().map(|st| sx(st[1])).collect();
    let energy: Vec<(f32, f32)> = states
        .iter()
        .map(|st| {
            let e = ds.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();
    let (w1s, w2s) = (sx(ds.w1), sx(ds.w2));
    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut mk_wall = |suffix: &str, at: Vec2| {
        let mut w = Entity::new(
            format!("{id}.{suffix}"),
            Shape::Line {
                to: Vec2::new(at.x, at.y + 40.0),
            },
            Vec2::new(at.x, at.y - 40.0),
            style::DIM,
        );
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
    for (sid, at, col) in [
        (format!("{id}.block1"), b1[0], style::CYAN),
        (format!("{id}.block2"), b2[0], style::MAGENTA),
    ] {
        let mut bl = Entity::new(sid, Shape::Rect { w: 34.0, h: 34.0 }, at, col);
        bl.stroke.fill = true;
        bl.stroke.outline = false;
        bl.tags = ct();
        s.add(bl);
    }
    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.block1"),
            prop: Prop::Pos,
            points: b1.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.block2"),
            prop: Prop::Pos,
            points: b2.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.spring1"),
            prop: Prop::To,
            points: b1.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.spring2"),
            prop: Prop::Pos,
            points: b1,
        },
        PlaybackTrack {
            id: format!("{id}.spring2"),
            prop: Prop::To,
            points: b2.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.spring3"),
            prop: Prop::Pos,
            points: b2,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: ds.labels(),
            phase_xy: ds.phase_xy(),
            pos_var: ds.pos_var(),
            well: ds.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `seriesparallel(id, [center], [unit])` — two identical masses, one on springs in
/// series (soft, slow) and one in parallel (stiff, fast), bobbing side by side.
fn c_seriesparallel(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 150.0)
    };
    let unit = a.opt_num(2)?.unwrap_or(70.0);
    let sp = SeriesParallel {
        g: 9.81,
        k1: 20.0,
        k2: 20.0,
        m: 1.0,
        damping: 0.0,
        l0: 0.8,
        stretch0: 0.5,
    };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&sp, sim_dt, substeps, SAMPLES);
    let (xs, xp) = (center.x - 140.0, center.x + 140.0);
    let sm: Vec<Vec2> = states
        .iter()
        .map(|st| Vec2::new(xs, center.y + st[0] * unit))
        .collect();
    let jn: Vec<Vec2> = states
        .iter()
        .map(|st| Vec2::new(xs, center.y + st[0] * 0.5 * unit))
        .collect();
    let pm: Vec<Vec2> = states
        .iter()
        .map(|st| Vec2::new(xp, center.y + st[2] * unit))
        .collect();
    let energy: Vec<(f32, f32)> = states
        .iter()
        .map(|st| {
            let e = sp.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();
    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    // anchors + labels
    for (sid, at, txt) in [
        (format!("{id}.anchorS"), Vec2::new(xs, center.y), "series"),
        (format!("{id}.anchorP"), Vec2::new(xp, center.y), "parallel"),
    ] {
        let mut anc = Entity::new(
            format!("{sid}.dot"),
            Shape::Circle { r: 5.0 },
            at,
            style::GOLD,
        );
        anc.stroke.fill = true;
        anc.stroke.outline = false;
        anc.tags = ct();
        s.add(anc);
        let mut lbl = Entity::new(
            sid,
            Shape::Text {
                content: txt.into(),
                size: 16.0,
            },
            Vec2::new(at.x, at.y - 20.0),
            style::DIM,
        );
        lbl.tags = ct();
        s.add(lbl);
    }
    // series: two stacked coils via the junction
    let mut c1 = Entity::new(
        format!("{id}.sCoil1"),
        Shape::Coil {
            to: jn[0],
            turns: 6,
        },
        Vec2::new(xs, center.y),
        style::LIME,
    );
    c1.stroke.width = 3.0;
    c1.tags = ct();
    s.add(c1);
    let mut c2 = Entity::new(
        format!("{id}.sCoil2"),
        Shape::Coil {
            to: sm[0],
            turns: 6,
        },
        jn[0],
        style::LIME,
    );
    c2.stroke.width = 3.0;
    c2.tags = ct();
    s.add(c2);
    // parallel: two side-by-side coils to the one mass
    for (sid, ax) in [
        (format!("{id}.pCoilL"), xp - 15.0),
        (format!("{id}.pCoilR"), xp + 15.0),
    ] {
        let mut c = Entity::new(
            sid,
            Shape::Coil {
                to: pm[0],
                turns: 6,
            },
            Vec2::new(ax, center.y),
            style::LIME,
        );
        c.stroke.width = 3.0;
        c.tags = ct();
        s.add(c);
    }
    for (sid, at, col) in [
        (format!("{id}.massS"), sm[0], style::CYAN),
        (format!("{id}.massP"), pm[0], style::MAGENTA),
    ] {
        let mut m = Entity::new(sid, Shape::Rect { w: 40.0, h: 32.0 }, at, col);
        m.stroke.fill = true;
        m.stroke.outline = false;
        m.tags = ct();
        s.add(m);
    }
    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.massS"),
            prop: Prop::Pos,
            points: sm.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.massP"),
            prop: Prop::Pos,
            points: pm.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.sCoil1"),
            prop: Prop::To,
            points: jn.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.sCoil2"),
            prop: Prop::Pos,
            points: jn,
        },
        PlaybackTrack {
            id: format!("{id}.sCoil2"),
            prop: Prop::To,
            points: sm,
        },
        PlaybackTrack {
            id: format!("{id}.pCoilL"),
            prop: Prop::To,
            points: pm.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.pCoilR"),
            prop: Prop::To,
            points: pm,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: sp.labels(),
            phase_xy: sp.phase_xy(),
            pos_var: sp.pos_var(),
            well: sp.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `carsuspension(id, [center], [unit])` — a quarter-car riding a scrolling road
/// (speed bump, washboard, pothole); the body bobs on its spring+damper.
fn c_carsuspension(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 430.0)
    };
    let _unit = a.opt_num(2)?;
    let cs = CarSuspension {
        m: 500.0,
        k: 20000.0,
        damping: 4000.0,
        speed: 8.0,
        bump: 0.08,
    };
    let (sim_dt, substeps) = (0.002f32, 12usize);
    let states = simulate(&cs, sim_dt, substeps, SAMPLES);
    let (uy, ux, rest) = (500.0f32, 12.0f32, 100.0f32); // vertical amp, road scale, rest suspension px
    let gy = center.y;
    // road polyline (world rx 0..130), scrolled by animating Pos.x
    let road_pts: Vec<Vec2> = (0..=260)
        .map(|i| {
            let rx = i as f32 * 0.5;
            Vec2::new(center.x + rx * ux, gy - road_height(rx, cs.bump) * uy)
        })
        .collect();
    let road_pos: Vec<Vec2> = states
        .iter()
        .map(|st| Vec2::new(-st[2] * ux, 0.0))
        .collect();
    let wheel: Vec<Vec2> = states
        .iter()
        .map(|st| Vec2::new(center.x, gy - road_height(st[2], cs.bump) * uy))
        .collect();
    let body: Vec<Vec2> = states
        .iter()
        .map(|st| Vec2::new(center.x, gy - rest - st[0] * uy))
        .collect();
    let energy: Vec<(f32, f32)> = states
        .iter()
        .map(|st| {
            let e = cs.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();
    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut road = Entity::new(
        format!("{id}.road"),
        Shape::Polyline { pts: road_pts },
        Vec2::ZERO,
        style::DIM,
    );
    road.stroke.width = 3.0;
    road.tags = ct();
    s.add(road);
    let mut susp = Entity::new(
        format!("{id}.suspension"),
        Shape::Coil {
            to: body[0],
            turns: 6,
        },
        wheel[0],
        style::LIME,
    );
    susp.stroke.width = 3.0;
    susp.tags = ct();
    s.add(susp);
    let mut wh = Entity::new(
        format!("{id}.wheel"),
        Shape::Circle { r: 16.0 },
        wheel[0],
        style::GOLD,
    );
    wh.stroke.fill = false;
    wh.stroke.outline = true;
    wh.stroke.width = 4.0;
    wh.tags = ct();
    s.add(wh);
    let mut car = Entity::new(
        format!("{id}.body"),
        Shape::Rect { w: 90.0, h: 44.0 },
        body[0],
        style::CYAN,
    );
    car.stroke.fill = true;
    car.stroke.outline = false;
    car.tags = ct();
    s.add(car);
    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.road"),
            prop: Prop::Pos,
            points: road_pos,
        },
        PlaybackTrack {
            id: format!("{id}.wheel"),
            prop: Prop::Pos,
            points: wheel.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.body"),
            prop: Prop::Pos,
            points: body.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.suspension"),
            prop: Prop::Pos,
            points: wheel,
        },
        PlaybackTrack {
            id: format!("{id}.suspension"),
            prop: Prop::To,
            points: body,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: cs.labels(),
            phase_xy: cs.phase_xy(),
            pos_var: cs.pos_var(),
            well: cs.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `piston(id, [center], [rpm], [unit])` — an engine piston: a spinning crank
/// drives a connecting rod that pushes a piston up and down in a cylinder.
fn c_piston(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 470.0)
    };
    let rpm = a.opt_num(2)?.unwrap_or(60.0).max(1.0);
    let u = a.opt_num(3)?.unwrap_or(1.4);
    let p = Piston {
        a: 50.0,
        l: 150.0,
        rpm,
    };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&p, sim_dt, substeps, SAMPLES);
    let o = center;
    let pin: Vec<Vec2> = states
        .iter()
        .map(|st| {
            let (px, py) = p.pin(st[0]);
            Vec2::new(o.x + px * u, o.y - py * u)
        })
        .collect();
    let piston_top: Vec<Vec2> = states
        .iter()
        .map(|st| Vec2::new(o.x, o.y - p.height(st[0]) * u))
        .collect();
    let (pin0, top0) = (pin[0], piston_top[0]);
    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let half_bore = 43.0 * u;
    // cylinder walls (fixed) spanning the piston's travel
    for (sid, x) in [
        (format!("{id}.cylL"), o.x - half_bore),
        (format!("{id}.cylR"), o.x + half_bore),
    ] {
        let mut w = Entity::new(
            sid,
            Shape::Line {
                to: Vec2::new(x, o.y - 300.0 * u),
            },
            Vec2::new(x, o.y - 80.0 * u),
            style::DIM,
        );
        w.stroke.width = 3.0;
        w.opacity = 0.6;
        w.tags = ct();
        s.add(w);
    }
    // crank circle (the crank's swept path)
    let mut crank = Entity::new(
        format!("{id}.crank"),
        Shape::Circle { r: p.a * u },
        o,
        style::DIM,
    );
    crank.stroke.fill = false;
    crank.stroke.outline = true;
    crank.stroke.width = 2.0;
    crank.opacity = 0.5;
    crank.tags = ct();
    s.add(crank);
    let mut arm = Entity::new(
        format!("{id}.arm"),
        Shape::Line { to: pin0 },
        o,
        style::GOLD,
    );
    arm.stroke.width = 4.0;
    arm.tags = ct();
    s.add(arm);
    let mut rod = Entity::new(
        format!("{id}.rod"),
        Shape::Line { to: top0 },
        pin0,
        style::FG,
    );
    rod.stroke.width = 3.0;
    rod.tags = ct();
    s.add(rod);
    let mut pistn = Entity::new(
        format!("{id}.piston"),
        Shape::Rect {
            w: 2.0 * half_bore - 8.0,
            h: 40.0,
        },
        top0,
        style::CYAN,
    );
    pistn.stroke.fill = true;
    pistn.stroke.outline = false;
    pistn.tags = ct();
    s.add(pistn);
    let mut hub = Entity::new(
        format!("{id}.hub"),
        Shape::Circle { r: 6.0 },
        o,
        style::GOLD,
    );
    hub.stroke.fill = true;
    hub.stroke.outline = false;
    hub.tags = ct();
    s.add(hub);
    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.arm"),
            prop: Prop::To,
            points: pin.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.rod"),
            prop: Prop::Pos,
            points: pin,
        },
        PlaybackTrack {
            id: format!("{id}.rod"),
            prop: Prop::To,
            points: piston_top.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.piston"),
            prop: Prop::Pos,
            points: piston_top,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: p.labels(),
            phase_xy: p.phase_xy(),
            pos_var: p.pos_var(),
            well: p.well_curve(),
            energy: states.iter().map(|_| (0.0, 0.0)).collect(),
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `molecule(id, [center], [atoms], [unit])` — N atoms bonded by springs,
/// vibrating about their equilibrium shape.
fn c_molecule(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(560.0, 300.0)
    };
    let n = (a.opt_num(2)?.unwrap_or(3.0) as usize).clamp(2, 6);
    let u = a.opt_num(3)?.unwrap_or(70.0);
    let mol = Molecule {
        n,
        k: 12.0,
        rest: 2.0,
        mass: 0.5,
        damping: 0.0,
    };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&mol, sim_dt, substeps, SAMPLES);
    let to_screen = |wx: f32, wy: f32| Vec2::new(center.x + wx * u, center.y - wy * u);
    let atom_pts: Vec<Vec<Vec2>> = (0..n)
        .map(|i| {
            states
                .iter()
                .map(|st| to_screen(st[i * 4], st[i * 4 + 1]))
                .collect()
        })
        .collect();
    let energy: Vec<(f32, f32)> = states
        .iter()
        .map(|st| {
            let e = mol.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();
    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut playback = Vec::new();
    // bonds (every pair) — drawn first, behind the atoms; real spring coils so
    // the bonds visibly stretch and compress as the atoms vibrate
    for i in 0..n {
        for j in (i + 1)..n {
            let bid = format!("{id}.bond{i}{j}");
            let mut b = Entity::new(
                bid.clone(),
                Shape::Coil {
                    to: atom_pts[j][0],
                    turns: 7,
                },
                atom_pts[i][0],
                style::LIME,
            );
            b.stroke.width = 2.5;
            b.tags = ct();
            s.add(b);
            playback.push(PlaybackTrack {
                id: bid.clone(),
                prop: Prop::Pos,
                points: atom_pts[i].clone(),
            });
            playback.push(PlaybackTrack {
                id: bid,
                prop: Prop::To,
                points: atom_pts[j].clone(),
            });
        }
    }
    for i in 0..n {
        let aid = format!("{id}.atom{i}");
        let col = if i == 0 { style::MAGENTA } else { style::CYAN };
        let mut at = Entity::new(aid.clone(), Shape::Circle { r: 16.0 }, atom_pts[i][0], col);
        at.stroke.fill = true;
        at.stroke.outline = false;
        at.tags = ct();
        s.add(at);
        playback.push(PlaybackTrack {
            id: aid,
            prop: Prop::Pos,
            points: atom_pts[i].clone(),
        });
    }
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: mol.labels(),
            phase_xy: mol.phase_xy(),
            pos_var: mol.pos_var(),
            well: mol.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// The shared **1-D collision resolver**: the post-collision velocities of two
/// masses, with restitution `e` (1 = perfectly elastic → the relative velocity
/// reverses; 0 = perfectly inelastic → they move together). Momentum is always
/// conserved. For equal masses and `e = 1` this reduces to a velocity swap.
pub fn collide_1d(m1: f32, v1: f32, m2: f32, v2: f32, e: f32) -> (f32, f32) {
    let p = m1 * v1 + m2 * v2; // conserved momentum
    let msum = m1 + m2;
    (
        (p - m2 * e * (v1 - v2)) / msum,
        (p + m1 * e * (v1 - v2)) / msum,
    )
}

/// Once `state[pi]` first leaves `[lo, hi]`, hold every later frame clamped to
/// that bound with zero velocity — a body coming to rest against a floor/stop.
fn freeze_range(states: &mut [Vec<f32>], pi: usize, vi: usize, lo: f32, hi: f32) {
    let mut hit: Option<Vec<f32>> = None;
    for st in states.iter_mut() {
        if let Some(h) = &hit {
            st.clone_from(h);
        } else if st[pi] <= lo || st[pi] >= hi {
            st[pi] = st[pi].clamp(lo, hi);
            st[vi] = 0.0;
            hit = Some(st.clone());
        }
    }
}

/// `robotarm(id, [center], [mode], [unit])` — a two-link arm that reaches for a
/// target by inverse-kinematics velocity control. `center` is the base (default
/// `(500, 440)`); `mode` selects the target: **1 = trace a circle** (default),
/// 2 = trace a figure-8, 0 = reach a fixed point and settle; `unit` px-per-metre
/// (default 150). Lays out `{id}.base`, `{id}.link1`, `{id}.elbow`, `{id}.link2`,
/// `{id}.ee`, the (moving) target `{id}.target`, and the end-effector trail
/// `{id}.path`. Animate with `run(id)` — in mode 1/2 the gripper tracks the
/// moving target continuously, tracing the shape.
fn c_robotarm(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(500.0, 440.0)
    };
    let mode = a.opt_num(2)?.unwrap_or(1.0).clamp(0.0, 2.0) as u8;
    let unit = a.opt_num(3)?.unwrap_or(150.0);
    let arm = RobotArm {
        l1: 1.0,
        l2: 0.5,
        gain: 5.0,
        mode,
        tx: 1.0,
        ty: 0.8,
    };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&arm, sim_dt, substeps, SAMPLES);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let elbow_pts: Vec<Vec2> = states.iter().map(|st| to_screen(arm.elbow(st))).collect();
    let ee_pts: Vec<Vec2> = states.iter().map(|st| to_screen(arm.body(st))).collect();
    let target_pts: Vec<Vec2> = states
        .iter()
        .map(|st| to_screen(arm.target(st[2])))
        .collect();
    let (elb0, ee0) = (elbow_pts[0], ee_pts[0]);
    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];

    let mut trail = Entity::new(
        format!("{id}.path"),
        Shape::Polyline {
            pts: ee_pts.clone(),
        },
        Vec2::ZERO,
        style::DIM,
    );
    trail.stroke.width = 2.0;
    trail.opacity = 0.4;
    trail.tags = ct();
    s.add(trail);
    let mut tgt = Entity::new(
        format!("{id}.target"),
        Shape::Circle { r: 11.0 },
        target_pts[0],
        style::LIME,
    );
    tgt.stroke.fill = false;
    tgt.stroke.outline = true;
    tgt.stroke.width = 2.5;
    tgt.tags = ct();
    s.add(tgt);
    let mut link1 = Entity::new(
        format!("{id}.link1"),
        Shape::Line { to: elb0 },
        center,
        style::GOLD,
    );
    link1.stroke.width = 6.0;
    link1.tags = ct();
    s.add(link1);
    let mut link2 = Entity::new(
        format!("{id}.link2"),
        Shape::Line { to: ee0 },
        elb0,
        style::CYAN,
    );
    link2.stroke.width = 5.0;
    link2.tags = ct();
    s.add(link2);
    let mut base = Entity::new(
        format!("{id}.base"),
        Shape::Circle { r: 9.0 },
        center,
        style::DIM,
    );
    base.stroke.fill = true;
    base.stroke.outline = false;
    base.tags = ct();
    s.add(base);
    let mut elbow = Entity::new(
        format!("{id}.elbow"),
        Shape::Circle { r: 6.0 },
        elb0,
        style::GOLD,
    );
    elbow.stroke.fill = true;
    elbow.stroke.outline = false;
    elbow.tags = ct();
    s.add(elbow);
    let mut ee = Entity::new(
        format!("{id}.ee"),
        Shape::Circle { r: 9.0 },
        ee0,
        style::CYAN,
    );
    ee.stroke.fill = true;
    ee.stroke.outline = false;
    ee.tags = ct();
    s.add(ee);

    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.link1"),
            prop: Prop::To,
            points: elbow_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.elbow"),
            prop: Prop::Pos,
            points: elbow_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.link2"),
            prop: Prop::Pos,
            points: elbow_pts,
        },
        PlaybackTrack {
            id: format!("{id}.link2"),
            prop: Prop::To,
            points: ee_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.ee"),
            prop: Prop::Pos,
            points: ee_pts,
        },
        PlaybackTrack {
            id: format!("{id}.target"),
            prop: Prop::Pos,
            points: target_pts,
        },
    ];
    let energy = states.iter().map(|_| (0.0, 0.0)).collect();
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: arm.labels(),
            phase_xy: arm.phase_xy(),
            pos_var: arm.pos_var(),
            well: arm.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// Lay out two hanging masses + their vertical ropes for a pulley machine, given
/// the two anchor points at the top of each rope. Returns the per-frame screen
/// positions so the caller can add pulley wheels / a scale between them.
fn pulley_masses(
    s: &mut Scene,
    id: &str,
    ct: &dyn Fn() -> Vec<String>,
    states: &[Vec<f32>],
    left_top: Vec2,
    right_top: Vec2,
    hang0: f32,
    unit: f32,
    m1: f32,
    m2: f32,
) -> (Vec<Vec2>, Vec<Vec2>) {
    let left_pts: Vec<Vec2> = states
        .iter()
        .map(|st| Vec2::new(left_top.x, left_top.y + hang0 + st[0] * unit))
        .collect();
    let right_pts: Vec<Vec2> = states
        .iter()
        .map(|st| Vec2::new(right_top.x, right_top.y + hang0 - st[0] * unit))
        .collect();
    let (w1, w2) = (24.0 + 5.0 * m1.sqrt(), 24.0 + 5.0 * m2.sqrt());
    let mut lr = Entity::new(
        format!("{id}.ropeL"),
        Shape::Line { to: left_pts[0] },
        left_top,
        style::DIM,
    );
    lr.stroke.width = 2.0;
    lr.tags = ct();
    s.add(lr);
    let mut rr = Entity::new(
        format!("{id}.ropeR"),
        Shape::Line { to: right_pts[0] },
        right_top,
        style::DIM,
    );
    rr.stroke.width = 2.0;
    rr.tags = ct();
    s.add(rr);
    let mut lm = Entity::new(
        format!("{id}.mass1"),
        Shape::Rect { w: w1, h: w1 },
        left_pts[0],
        style::CYAN,
    );
    lm.stroke.fill = true;
    lm.stroke.outline = false;
    lm.tags = ct();
    s.add(lm);
    let mut rm = Entity::new(
        format!("{id}.mass2"),
        Shape::Rect { w: w2, h: w2 },
        right_pts[0],
        style::MAGENTA,
    );
    rm.stroke.fill = true;
    rm.stroke.outline = false;
    rm.tags = ct();
    s.add(rm);
    (left_pts, right_pts)
}

fn pulley_playback(id: &str, left_pts: Vec<Vec2>, right_pts: Vec<Vec2>) -> Vec<PlaybackTrack> {
    vec![
        PlaybackTrack {
            id: format!("{id}.ropeL"),
            prop: Prop::To,
            points: left_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.ropeR"),
            prop: Prop::To,
            points: right_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.mass1"),
            prop: Prop::Pos,
            points: left_pts,
        },
        PlaybackTrack {
            id: format!("{id}.mass2"),
            prop: Prop::Pos,
            points: right_pts,
        },
    ]
}

/// `pulley(id, [center], [m1], [m2], [unit])` — a vertical **Atwood machine**:
/// two masses over one pulley. `center` is the pulley (default `(640, 170)`);
/// `m1`/`m2` the left/right masses in kg (default 3, 2); `unit` px-per-metre
/// (default 80). The heavier mass accelerates down at ((m₁−m₂)g)/(m₁+m₂). Lays
/// out `{id}.wheel`, `{id}.hub`, `{id}.ropeL/.ropeR`, `{id}.mass1/.mass2`.
/// Animate with `run(id)`; `energygraph` shows the KE↔PE trade.
fn c_pulley(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 170.0)
    };
    let m1 = a.opt_num(2)?.unwrap_or(3.0).max(0.1);
    let m2 = a.opt_num(3)?.unwrap_or(2.0).max(0.1);
    let unit = a.opt_num(4)?.unwrap_or(80.0);
    let p = Pulley {
        m1,
        m2,
        g: 9.81,
        damping: 0.0,
    };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let mut states = simulate(&p, sim_dt, substeps, SAMPLES);
    freeze_range(&mut states, 0, 1, -1.7, 1.7); // masses reach the floor / rise to the wheel
    let energy = states
        .iter()
        .map(|st| {
            let e = p.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let r_wheel = 34.0;
    let mut wheel = Entity::new(
        format!("{id}.wheel"),
        Shape::Circle { r: r_wheel },
        center,
        style::DIM,
    );
    wheel.stroke.fill = false;
    wheel.stroke.outline = true;
    wheel.stroke.width = 3.0;
    wheel.tags = ct();
    s.add(wheel);
    let (left_top, right_top) = (
        Vec2::new(center.x - r_wheel, center.y),
        Vec2::new(center.x + r_wheel, center.y),
    );
    let (left_pts, right_pts) = pulley_masses(
        s, &id, &ct, &states, left_top, right_top, 130.0, unit, m1, m2,
    );
    let mut hub = Entity::new(
        format!("{id}.hub"),
        Shape::Circle { r: 6.0 },
        center,
        style::GOLD,
    );
    hub.stroke.fill = true;
    hub.stroke.outline = false;
    hub.tags = ct();
    s.add(hub);

    let playback = pulley_playback(&id, left_pts, right_pts);
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: p.labels(),
            phase_xy: p.phase_xy(),
            pos_var: p.pos_var(),
            well: p.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `pulleyscale(id, [center], [m1], [m2], [unit])` — an Atwood machine over **two**
/// pulleys with an in-line spring scale on the rope between them. The scale reads
/// the rope tension 2·m₁·m₂·g/(m₁+m₂) — *not* the sum of the two weights, the
/// classic surprise. `center` is the mid-top (default `(640, 170)`). Lays out
/// `{id}.pulleyL/.pulleyR`, `{id}.ropeL/.ropeR`, `{id}.mass1/.mass2`, the top rope,
/// `{id}.scale`, and the reading `{id}.reading`. Animate with `run(id)`.
fn c_pulleyscale(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 170.0)
    };
    let m1 = a.opt_num(2)?.unwrap_or(4.0).max(0.1);
    let m2 = a.opt_num(3)?.unwrap_or(3.0).max(0.1);
    let unit = a.opt_num(4)?.unwrap_or(80.0);
    let p = Pulley {
        m1,
        m2,
        g: 9.81,
        damping: 0.0,
    };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let mut states = simulate(&p, sim_dt, substeps, SAMPLES);
    freeze_range(&mut states, 0, 1, -1.6, 1.6);
    let energy = states
        .iter()
        .map(|st| {
            let e = p.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let w = 150.0;
    let (left_top, right_top) = (
        Vec2::new(center.x - w, center.y),
        Vec2::new(center.x + w, center.y),
    );
    let r_wheel = 22.0;
    // top rope run through the scale + the two pulley wheels
    let mut top_rope = Entity::new(
        format!("{id}.toprope"),
        Shape::Polyline {
            pts: vec![
                left_top,
                Vec2::new(center.x - 34.0, center.y),
                Vec2::new(center.x + 34.0, center.y),
                right_top,
            ],
        },
        Vec2::ZERO,
        style::DIM,
    );
    top_rope.stroke.width = 2.0;
    top_rope.tags = ct();
    s.add(top_rope);
    for (sid, c) in [
        (format!("{id}.pulleyL"), left_top),
        (format!("{id}.pulleyR"), right_top),
    ] {
        let mut wheel = Entity::new(sid, Shape::Circle { r: r_wheel }, c, style::DIM);
        wheel.stroke.fill = false;
        wheel.stroke.outline = true;
        wheel.stroke.width = 3.0;
        wheel.tags = ct();
        s.add(wheel);
    }
    let (left_pts, right_pts) = pulley_masses(
        s, &id, &ct, &states, left_top, right_top, 120.0, unit, m1, m2,
    );
    // the spring scale sitting in the horizontal rope
    let mut scale = Entity::new(
        format!("{id}.scale"),
        Shape::Rect { w: 64.0, h: 30.0 },
        center,
        style::GOLD,
    );
    scale.stroke.fill = false;
    scale.stroke.outline = true;
    scale.stroke.width = 2.5;
    scale.tags = ct();
    s.add(scale);
    let mut reading = Entity::new(
        format!("{id}.reading"),
        Shape::Text {
            content: format!("T = {:.0} N", p.tension()),
            size: 15.0,
        },
        Vec2::new(center.x, center.y - 26.0),
        style::GOLD,
    );
    reading.tags = ct();
    s.add(reading);

    let playback = pulley_playback(&id, left_pts, right_pts);
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: p.labels(),
            phase_xy: p.phase_xy(),
            pos_var: p.pos_var(),
            well: p.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `blocktackle(id, [center], [load], [effort], [strands], [unit])` — a compound
/// pulley (block & tackle). `center` is the top support (default `(cx, 130)`);
/// `load` kg on the movable block (default 8); `effort` kg on the pulled end
/// (default 3); `strands` N = the mechanical advantage, clamped 1–4 (default 3);
/// `unit` px-per-metre (default 70). With N strands an effort of only load/N
/// balances the load. Lays out `{id}.beam`, `{id}.fixed`, `{id}.movable`,
/// `{id}.load`, `{id}.strand0…`, `{id}.effortrope`, `{id}.effort`, `{id}.malabel`.
/// Animate with `run(id)`; `energygraph` shows the KE↔PE trade.
fn c_blocktackle(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 130.0)
    };
    let load = a.opt_num(2)?.unwrap_or(8.0).max(0.1);
    let effort = a.opt_num(3)?.unwrap_or(3.0).max(0.1);
    let n = (a.opt_num(4)?.unwrap_or(3.0) as usize).clamp(1, 4);
    let unit = a.opt_num(5)?.unwrap_or(70.0);
    let bt = BlockTackle {
        load,
        effort,
        strands: n as f32,
        g: 9.81,
    };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let mut states = simulate(&bt, sim_dt, substeps, SAMPLES);
    freeze_range(&mut states, 0, 1, -1.2, 1.2); // block reaches the top / bottom stop
    let energy = states
        .iter()
        .map(|st| {
            let e = bt.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();

    // fixed geometry (px)
    let fixed_bottom = center.y + 34.0;
    let base_gap = 200.0;
    let movable_top0 = fixed_bottom + base_gap;
    let movable_c0 = movable_top0 + 16.0;
    let load_c0 = movable_c0 + 46.0;
    let (effort_x, effort_y0) = (center.x + 210.0, center.y + 150.0);
    let sy = |k: usize| states[k][0] * unit; // load rise, px (screen y decreases)

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];

    // ceiling beam + fixed block (static)
    let mut beam = Entity::new(
        format!("{id}.beam"),
        Shape::Line {
            to: Vec2::new(center.x + 150.0, center.y),
        },
        Vec2::new(center.x - 150.0, center.y),
        style::DIM,
    );
    beam.stroke.width = 5.0;
    beam.tags = ct();
    s.add(beam);
    let mut fixed = Entity::new(
        format!("{id}.fixed"),
        Shape::Rect { w: 96.0, h: 20.0 },
        Vec2::new(center.x, center.y + 22.0),
        style::DIM,
    );
    fixed.stroke.outline = true;
    fixed.stroke.fill = false;
    fixed.stroke.width = 2.5;
    fixed.tags = ct();
    s.add(fixed);

    // N supporting strands (fixed top anchor → movable block top; shorten as it rises)
    let strand_x = |i: usize| {
        if n == 1 {
            center.x
        } else {
            center.x - 30.0 + 60.0 * i as f32 / (n - 1) as f32
        }
    };
    let mut playback = Vec::new();
    for i in 0..n {
        let sx = strand_x(i);
        let sid = format!("{id}.strand{i}");
        let mut st = Entity::new(
            sid.clone(),
            Shape::Line {
                to: Vec2::new(sx, movable_top0),
            },
            Vec2::new(sx, fixed_bottom),
            style::LIME,
        );
        st.stroke.width = 2.0;
        st.tags = ct();
        s.add(st);
        let pts: Vec<Vec2> = (0..states.len())
            .map(|k| Vec2::new(sx, movable_top0 - sy(k)))
            .collect();
        playback.push(PlaybackTrack {
            id: sid,
            prop: Prop::To,
            points: pts,
        });
    }

    // movable block + the load hanging from it (both rise by x)
    let movable_pts: Vec<Vec2> = (0..states.len())
        .map(|k| Vec2::new(center.x, movable_c0 - sy(k)))
        .collect();
    let load_w = 34.0 + 4.0 * load.sqrt();
    let load_pts: Vec<Vec2> = (0..states.len())
        .map(|k| Vec2::new(center.x, load_c0 + load_w * 0.5 - sy(k)))
        .collect();
    let mut movable = Entity::new(
        format!("{id}.movable"),
        Shape::Rect { w: 82.0, h: 22.0 },
        movable_pts[0],
        style::CYAN,
    );
    movable.stroke.fill = true;
    movable.stroke.outline = false;
    movable.tags = ct();
    s.add(movable);
    let mut load_e = Entity::new(
        format!("{id}.load"),
        Shape::Rect {
            w: load_w,
            h: load_w,
        },
        load_pts[0],
        style::GOLD,
    );
    load_e.stroke.fill = true;
    load_e.stroke.outline = false;
    load_e.tags = ct();
    s.add(load_e);

    // effort end: a rope from the fixed block over to a hanging effort mass that
    // descends N× as far as the load rises
    let effort_w = 28.0 + 4.0 * effort.sqrt();
    let effort_pts: Vec<Vec2> = (0..states.len())
        .map(|k| Vec2::new(effort_x, effort_y0 + n as f32 * sy(k)))
        .collect();
    let effort_top: Vec<Vec2> = effort_pts
        .iter()
        .map(|p| Vec2::new(p.x, p.y - effort_w * 0.5))
        .collect();
    let mut erope = Entity::new(
        format!("{id}.effortrope"),
        Shape::Line { to: effort_top[0] },
        Vec2::new(center.x + 48.0, center.y + 22.0),
        style::MAGENTA,
    );
    erope.stroke.width = 2.0;
    erope.tags = ct();
    s.add(erope);
    let mut effort_e = Entity::new(
        format!("{id}.effort"),
        Shape::Rect {
            w: effort_w,
            h: effort_w,
        },
        effort_pts[0],
        style::MAGENTA,
    );
    effort_e.stroke.fill = true;
    effort_e.stroke.outline = false;
    effort_e.tags = ct();
    s.add(effort_e);

    let mut malabel = Entity::new(
        format!("{id}.malabel"),
        Shape::Text {
            content: format!("MA = {n}   (effort {effort:.0} kg ⇄ load {load:.0} kg)"),
            size: 17.0,
        },
        Vec2::new(center.x, center.y - 24.0),
        style::GOLD,
    );
    malabel.tags = ct();
    s.add(malabel);

    playback.push(PlaybackTrack {
        id: format!("{id}.movable"),
        prop: Prop::Pos,
        points: movable_pts,
    });
    playback.push(PlaybackTrack {
        id: format!("{id}.load"),
        prop: Prop::Pos,
        points: load_pts,
    });
    playback.push(PlaybackTrack {
        id: format!("{id}.effort"),
        prop: Prop::Pos,
        points: effort_pts,
    });
    playback.push(PlaybackTrack {
        id: format!("{id}.effortrope"),
        prop: Prop::To,
        points: effort_top,
    });
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: bt.labels(),
            phase_xy: bt.phase_xy(),
            pos_var: bt.pos_var(),
            well: bt.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `compoundpulley(id, [center], [mA], [mB], [mC], [unit])` — a compound pulley:
/// a fixed top pulley (mass A on the left, a movable lower pulley on the right)
/// whose movable pulley carries masses B and C. `center` is the ceiling attach
/// point (default `(520, 120)`); `mA`/`mB`/`mC` kg (default 5, 2, 2); `unit`
/// px-per-metre (default 70). Static when mA = mB + mC. Lays out `{id}.top`,
/// `{id}.mov` (movable pulley), `{id}.massA/.massB/.massC`, ropes `{id}.rope*`.
/// Animate with `run(id)`; `energygraph` shows the KE↔PE trade.
fn c_compoundpulley(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(520.0, 120.0)
    };
    let ma = a.opt_num(2)?.unwrap_or(5.0).max(0.1);
    let mb = a.opt_num(3)?.unwrap_or(2.0).max(0.1);
    let mc = a.opt_num(4)?.unwrap_or(2.0).max(0.1);
    let unit = a.opt_num(5)?.unwrap_or(70.0);
    let cp = CompoundPulley {
        ma,
        mb,
        mc,
        g: 9.81,
    };
    let (sim_dt, substeps) = (0.0016f32, 4usize);
    let mut states = simulate(&cp, sim_dt, substeps, SAMPLES);
    freeze_range(&mut states, 0, 1, -1.4, 1.4); // stop before things run off-stage
    let energy = states
        .iter()
        .map(|st| {
            let e = cp.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();

    // geometry (px)
    let r = 52.0f32;
    let tp = Vec2::new(center.x, center.y + 90.0); // fixed top pulley centre
    let (a_x, p_x) = (center.x - r, center.x + r); // A at TP's left tangent; P hangs at the right tangent
    let (b_x, c_x) = (p_x - r, p_x + r);
    let (a0, p0, b0, c0) = (tp.y + 130.0, tp.y + 180.0, tp.y + 320.0, tp.y + 420.0);
    let half = 26.0f32;
    let (xa, xb, xc) = (
        |k: usize| states[k][0] * unit,
        |k: usize| states[k][2] * unit,
        |k: usize| states[k][4] * unit,
    );
    // movable pulley rises as A descends (xP = −xA)
    let p_pts: Vec<Vec2> = (0..states.len())
        .map(|k| Vec2::new(p_x, p0 - xa(k)))
        .collect();
    let a_pts: Vec<Vec2> = (0..states.len())
        .map(|k| Vec2::new(a_x, a0 + xa(k)))
        .collect();
    let b_pts: Vec<Vec2> = (0..states.len())
        .map(|k| Vec2::new(b_x, b0 + xb(k)))
        .collect();
    let c_pts: Vec<Vec2> = (0..states.len())
        .map(|k| Vec2::new(c_x, c0 + xc(k)))
        .collect();

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];

    // ceiling rope → top-pulley hub; the fixed top pulley
    let mut rope0 = Entity::new(
        format!("{id}.rope0"),
        Shape::Line { to: tp },
        Vec2::new(center.x, center.y),
        style::DIM,
    );
    rope0.stroke.width = 2.0;
    rope0.tags = ct();
    s.add(rope0);
    for (sid, cpos, col, fill) in [
        (format!("{id}.top"), tp, style::DIM, false),
        (format!("{id}.tophub"), tp, style::GOLD, true),
        (format!("{id}.mov"), p_pts[0], style::LIME, false),
        (format!("{id}.movhub"), p_pts[0], style::GOLD, true),
    ] {
        let rr = if fill { 5.0 } else { r };
        let mut w = Entity::new(sid, Shape::Circle { r: rr }, cpos, col);
        w.stroke.fill = fill;
        w.stroke.outline = !fill;
        w.stroke.width = 3.0;
        w.tags = ct();
        s.add(w);
    }

    // ropes: TP-left→A, TP-right→movable hub, P-left→B, P-right→C
    let mut ra = Entity::new(
        format!("{id}.ropeA"),
        Shape::Line {
            to: Vec2::new(a_x, a_pts[0].y - half),
        },
        Vec2::new(a_x, tp.y),
        style::DIM,
    );
    ra.stroke.width = 2.0;
    ra.tags = ct();
    s.add(ra);
    let mut rp = Entity::new(
        format!("{id}.ropeP"),
        Shape::Line { to: p_pts[0] },
        Vec2::new(p_x, tp.y),
        style::DIM,
    );
    rp.stroke.width = 2.0;
    rp.tags = ct();
    s.add(rp);
    let mut rb = Entity::new(
        format!("{id}.ropeB"),
        Shape::Line {
            to: Vec2::new(b_x, b_pts[0].y - half),
        },
        Vec2::new(b_x, p_pts[0].y),
        style::DIM,
    );
    rb.stroke.width = 2.0;
    rb.tags = ct();
    s.add(rb);
    let mut rc = Entity::new(
        format!("{id}.ropeC"),
        Shape::Line {
            to: Vec2::new(c_x, c_pts[0].y - half),
        },
        Vec2::new(c_x, p_pts[0].y),
        style::DIM,
    );
    rc.stroke.width = 2.0;
    rc.tags = ct();
    s.add(rc);

    // the three masses + labels
    for (mid, lid, p0v, col, txt, m) in [
        (
            format!("{id}.massA"),
            format!("{id}.lblA"),
            a_pts[0],
            style::CYAN,
            "A",
            ma,
        ),
        (
            format!("{id}.massB"),
            format!("{id}.lblB"),
            b_pts[0],
            style::MAGENTA,
            "B",
            mb,
        ),
        (
            format!("{id}.massC"),
            format!("{id}.lblC"),
            c_pts[0],
            style::GOLD,
            "C",
            mc,
        ),
    ] {
        let w = 40.0 + 4.0 * m.sqrt();
        let mut e = Entity::new(mid, Shape::Rect { w, h: 2.0 * half }, p0v, col);
        e.stroke.fill = true;
        e.stroke.outline = false;
        e.tags = ct();
        s.add(e);
        let mut l = Entity::new(
            lid,
            Shape::Text {
                content: txt.into(),
                size: 22.0,
            },
            p0v,
            style::VOID,
        );
        l.font = crate::primitives::FontKind::MonoBold;
        l.tags = ct();
        s.add(l);
    }

    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.ropeA"),
            prop: Prop::To,
            points: a_pts.iter().map(|p| Vec2::new(a_x, p.y - half)).collect(),
        },
        PlaybackTrack {
            id: format!("{id}.ropeP"),
            prop: Prop::To,
            points: p_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.ropeB"),
            prop: Prop::Pos,
            points: (0..states.len())
                .map(|k| Vec2::new(b_x, p0 - xa(k)))
                .collect(),
        },
        PlaybackTrack {
            id: format!("{id}.ropeB"),
            prop: Prop::To,
            points: b_pts.iter().map(|p| Vec2::new(b_x, p.y - half)).collect(),
        },
        PlaybackTrack {
            id: format!("{id}.ropeC"),
            prop: Prop::Pos,
            points: (0..states.len())
                .map(|k| Vec2::new(c_x, p0 - xa(k)))
                .collect(),
        },
        PlaybackTrack {
            id: format!("{id}.ropeC"),
            prop: Prop::To,
            points: c_pts.iter().map(|p| Vec2::new(c_x, p.y - half)).collect(),
        },
        PlaybackTrack {
            id: format!("{id}.mov"),
            prop: Prop::Pos,
            points: p_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.movhub"),
            prop: Prop::Pos,
            points: p_pts,
        },
        PlaybackTrack {
            id: format!("{id}.massA"),
            prop: Prop::Pos,
            points: a_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.lblA"),
            prop: Prop::Pos,
            points: a_pts,
        },
        PlaybackTrack {
            id: format!("{id}.massB"),
            prop: Prop::Pos,
            points: b_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.lblB"),
            prop: Prop::Pos,
            points: b_pts,
        },
        PlaybackTrack {
            id: format!("{id}.massC"),
            prop: Prop::Pos,
            points: c_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.lblC"),
            prop: Prop::Pos,
            points: c_pts,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: cp.labels(),
            phase_xy: cp.phase_xy(),
            pos_var: cp.pos_var(),
            well: cp.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `ramp(id, [center], [angle], [mass], [applied], [unit])` — a block sliding on
/// an inclined plane with friction. `center` is the incline foot (default
/// `(360, 480)`); `angle` in DEGREES (default 30); `mass` kg (default 5);
/// `applied` a horizontal push in N (default 0); `unit` px-per-metre (default 70).
/// Lays out `{id}.incline`, `{id}.surface`, `{id}.block`, `{id}.anglelabel`.
/// Friction bleeds mechanical energy, so `energygraph`'s total decays. Animate
/// with `run(id)`.
fn c_ramp(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(360.0, 480.0)
    };
    let angle_deg = a.opt_num(2)?.unwrap_or(30.0).clamp(1.0, 80.0);
    let mass = a.opt_num(3)?.unwrap_or(5.0).max(0.1);
    let applied = a.opt_num(4)?.unwrap_or(0.0);
    let unit = a.opt_num(5)?.unwrap_or(70.0);
    let ang = angle_deg.to_radians();
    let ramp_len = 5.0f32;
    let block_w = 0.55f32; // metres
    let r = Ramp {
        g: 9.81,
        angle: ang,
        mass,
        mu_s: 0.5,
        mu_k: 0.3,
        applied,
        s0: ramp_len * 0.82,
    };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let mut states = simulate(&r, sim_dt, substeps, SAMPLES);
    freeze_range(&mut states, 0, 1, block_w * 0.5, ramp_len); // stops at the foot / top wall
    let energy = states
        .iter()
        .map(|st| {
            let e = r.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();

    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let foot = center;
    let top = to_screen((ramp_len * ang.cos(), ramp_len * ang.sin()));
    let corner = Vec2::new(top.x, foot.y);

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    // the incline triangle (filled, faint)
    let mut tri = Entity::new(
        format!("{id}.incline"),
        Shape::Polygon {
            pts: vec![foot, top, corner],
        },
        Vec2::ZERO,
        style::DIM,
    );
    tri.stroke.fill = true;
    tri.stroke.outline = true;
    tri.stroke.width = 2.0;
    tri.opacity = 0.35;
    tri.tags = ct();
    s.add(tri);
    // the sliding surface, drawn brighter
    let mut surf = Entity::new(
        format!("{id}.surface"),
        Shape::Line { to: top },
        foot,
        style::FG,
    );
    surf.stroke.width = 3.0;
    surf.tags = ct();
    s.add(surf);
    let mut alabel = Entity::new(
        format!("{id}.anglelabel"),
        Shape::Text {
            content: format!("{angle_deg:.0}°"),
            size: 16.0,
        },
        Vec2::new(foot.x + 46.0, foot.y - 14.0),
        style::DIM,
    );
    alabel.tags = ct();
    s.add(alabel);

    // block: a square aligned to the slope, riding the surface
    let u = (top - foot).normalize_or_zero();
    let nrm = Vec2::new(-u.y, u.x); // screen perpendicular, pointing off the surface
    let hl = block_w * 0.5 * unit;
    let local = vec![
        -u * hl - nrm * hl,
        u * hl - nrm * hl,
        u * hl + nrm * hl,
        -u * hl + nrm * hl,
    ];
    let block_center = |sarc: f32| {
        let (c, sn) = (ang.cos(), ang.sin());
        // sit block_w/2 off the surface along the world normal (−sinθ, cosθ)
        to_screen((sarc * c - block_w * 0.5 * sn, sarc * sn + block_w * 0.5 * c))
    };
    let block_pts: Vec<Vec2> = states.iter().map(|st| block_center(st[0])).collect();
    let mut block = Entity::new(
        format!("{id}.block"),
        Shape::Polygon { pts: local },
        block_pts[0],
        style::CYAN,
    );
    block.stroke.fill = true;
    block.stroke.outline = false;
    block.tags = ct();
    s.add(block);

    // ---- opt-in free-body force diagram: {id}.forces, hidden until forces(id) ----
    // gravity (mg, cyan), normal (N, lime), friction (f, magenta), acceleration (a, gold)
    let fscale = 100.0 / (mass * r.g); // px per newton (mg ≈ 100 px)
    let ascale = 34.0; // px per m/s² for the acceleration vector
    let svec = |fx: f32, fy: f32| Vec2::new(fx * fscale, -fy * fscale); // world (y-up N) → screen delta
    let (ct2, st2) = (ang.cos(), ang.sin());
    let (mut mg, mut nn, mut ff, mut aa) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (k, stt) in states.iter().enumerate() {
        let fv = r.force_vectors(stt[1]);
        mg.push(block_pts[k] + svec(fv[0].1, fv[0].2));
        nn.push(block_pts[k] + svec(fv[1].1, fv[1].2));
        ff.push(block_pts[k] + svec(fv[2].1, fv[2].2));
        let al = r.accel(stt[1]);
        aa.push(block_pts[k] + Vec2::new(al * ct2 * ascale, -al * st2 * ascale));
    }
    let mut force_tracks = Vec::new();
    for (suffix, label, col, tips) in [
        ("fmg", "mg", style::BLUE, mg), // gravity — blue, the textbook convention
        ("fN", "N", style::LIME, nn),   // normal — green
        ("ff", "f", style::MAGENTA, ff), // friction — magenta
        ("fa", "a", style::RED, aa),    // acceleration — red
    ] {
        // tagged ONLY `{id}.forces` (not the bare id / `.parts`) so the diagram is
        // revealed solely by `forces(id)` — a plain `show(id)` leaves it hidden
        let ftags = vec![format!("{id}.forces")];
        let fid = format!("{id}.{suffix}");
        let mut e = Entity::new(fid.clone(), Shape::Arrow { to: tips[0] }, block_pts[0], col);
        e.stroke.width = 3.0;
        e.opacity = 0.0; // revealed by forces(id)
        e.tags = ftags.clone();
        s.add(e);
        let lid = format!("{fid}L");
        let mut l = Entity::new(
            lid.clone(),
            Shape::Text {
                content: label.to_string(),
                size: 17.0,
            },
            tips[0],
            col,
        );
        l.opacity = 0.0;
        l.tags = ftags;
        s.add(l);
        let lab_pts: Vec<Vec2> = tips.iter().map(|p| *p + Vec2::new(12.0, -6.0)).collect();
        force_tracks.push(PlaybackTrack {
            id: fid.clone(),
            prop: Prop::Pos,
            points: block_pts.clone(),
        });
        force_tracks.push(PlaybackTrack {
            id: fid,
            prop: Prop::To,
            points: tips,
        });
        force_tracks.push(PlaybackTrack {
            id: lid,
            prop: Prop::Pos,
            points: lab_pts,
        });
    }

    let mut playback = vec![PlaybackTrack {
        id: format!("{id}.block"),
        prop: Prop::Pos,
        points: block_pts,
    }];
    playback.extend(force_tracks);
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: r.labels(),
            phase_xy: r.phase_xy(),
            pos_var: r.pos_var(),
            well: r.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `inclinepulley(id, [center], [angle], [m1], [m2], [unit])` — a block on an
/// incline tied over a pulley at the top to a hanging mass (the incline-Atwood).
/// `center` is the incline foot (default `(300, 500)`); `angle` degrees (default
/// 30); `m1` the incline block, `m2` the hanging mass (default 3, 2); smooth.
/// Lays out `{id}.incline/.surface/.pulley/.block/.rope1/.rope2/.mass2`.
/// Animate with `run(id)`; `energygraph` shows the KE↔PE trade.
fn c_inclinepulley(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(300.0, 500.0)
    };
    let angle_deg = a.opt_num(2)?.unwrap_or(30.0).clamp(5.0, 75.0);
    let m1 = a.opt_num(3)?.unwrap_or(3.0).max(0.1);
    let m2 = a.opt_num(4)?.unwrap_or(2.0).max(0.1);
    let unit = a.opt_num(5)?.unwrap_or(70.0);
    let ang = angle_deg.to_radians();
    let ip = InclinePulley {
        g: 9.81,
        angle: ang,
        m1,
        m2,
        mu_k: 0.0,
        mu_s: 0.0,
    };
    let (ramp_len, s_block0, bw) = (5.5f32, 1.4f32, 0.5f32);
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let mut states = simulate(&ip, sim_dt, substeps, SAMPLES);
    freeze_range(
        &mut states,
        0,
        1,
        -(s_block0 - 0.3),
        ramp_len - s_block0 - 0.6,
    );
    let energy = states
        .iter()
        .map(|st| {
            let e = ip.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();

    let (ct2, st2) = (ang.cos(), ang.sin());
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let foot = center;
    let top = to_screen((ramp_len * ct2, ramp_len * st2));
    let corner = Vec2::new(top.x, foot.y);
    let block_center = |sarc: f32| {
        to_screen((
            (s_block0 + sarc) * ct2 - bw * 0.5 * st2,
            (s_block0 + sarc) * st2 + bw * 0.5 * ct2,
        ))
    };
    let block_pts: Vec<Vec2> = states.iter().map(|st| block_center(st[0])).collect();
    let (hang0, m2_x) = (90.0f32, top.x + 44.0);
    let m2_pts: Vec<Vec2> = states
        .iter()
        .map(|st| Vec2::new(m2_x, top.y + hang0 + st[0] * unit))
        .collect();

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut tri = Entity::new(
        format!("{id}.incline"),
        Shape::Polygon {
            pts: vec![foot, top, corner],
        },
        Vec2::ZERO,
        style::DIM,
    );
    tri.stroke.fill = true;
    tri.stroke.outline = true;
    tri.stroke.width = 2.0;
    tri.opacity = 0.35;
    tri.tags = ct();
    s.add(tri);
    let mut surf = Entity::new(
        format!("{id}.surface"),
        Shape::Line { to: top },
        foot,
        style::FG,
    );
    surf.stroke.width = 3.0;
    surf.tags = ct();
    s.add(surf);
    let mut wheel = Entity::new(
        format!("{id}.pulley"),
        Shape::Circle { r: 15.0 },
        top,
        style::DIM,
    );
    wheel.stroke.fill = false;
    wheel.stroke.outline = true;
    wheel.stroke.width = 3.0;
    wheel.tags = ct();
    s.add(wheel);
    let u = (top - foot).normalize_or_zero();
    let nrm = Vec2::new(-u.y, u.x);
    let hl = bw * 0.5 * unit;
    let local = vec![
        -u * hl - nrm * hl,
        u * hl - nrm * hl,
        u * hl + nrm * hl,
        -u * hl + nrm * hl,
    ];
    let mut block = Entity::new(
        format!("{id}.block"),
        Shape::Polygon { pts: local },
        block_pts[0],
        style::CYAN,
    );
    block.stroke.fill = true;
    block.stroke.outline = false;
    block.tags = ct();
    s.add(block);
    let mut rope1 = Entity::new(
        format!("{id}.rope1"),
        Shape::Line { to: top },
        block_pts[0],
        style::DIM,
    );
    rope1.stroke.width = 2.0;
    rope1.tags = ct();
    s.add(rope1);
    let mut rope2 = Entity::new(
        format!("{id}.rope2"),
        Shape::Line { to: m2_pts[0] },
        top,
        style::DIM,
    );
    rope2.stroke.width = 2.0;
    rope2.tags = ct();
    s.add(rope2);
    let m2w = 30.0 + 5.0 * m2.sqrt();
    let mut mass2 = Entity::new(
        format!("{id}.mass2"),
        Shape::Rect { w: m2w, h: m2w },
        m2_pts[0],
        style::MAGENTA,
    );
    mass2.stroke.fill = true;
    mass2.stroke.outline = false;
    mass2.tags = ct();
    s.add(mass2);
    let mut al = Entity::new(
        format!("{id}.anglelabel"),
        Shape::Text {
            content: format!("{angle_deg:.0}°"),
            size: 16.0,
        },
        Vec2::new(foot.x + 46.0, foot.y - 14.0),
        style::DIM,
    );
    al.tags = ct();
    s.add(al);

    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.block"),
            prop: Prop::Pos,
            points: block_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.rope1"),
            prop: Prop::Pos,
            points: block_pts,
        },
        PlaybackTrack {
            id: format!("{id}.rope2"),
            prop: Prop::To,
            points: m2_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.mass2"),
            prop: Prop::Pos,
            points: m2_pts,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: ip.labels(),
            phase_xy: ip.phase_xy(),
            pos_var: ip.pos_var(),
            well: ip.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `doubleincline(id, [center], [angle1], [angle2], [m1], [m2], [unit])` — two
/// blocks on a wedge's two slopes, tied over a pulley at the apex. `center` is the
/// base centre (default `(cx, 500)`); `angle1`/`angle2` the left/right slopes in
/// degrees (default 50, 30); `m1`/`m2` the left/right blocks (default 12, 70); the
/// right slope is rough. Lays out `{id}.wedge/.pulley/.mass1/.mass2/.rope1/.rope2`.
/// Animate with `run(id)`.
fn c_doubleincline(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 500.0)
    };
    let a1_deg = a.opt_num(2)?.unwrap_or(50.0).clamp(10.0, 80.0);
    let a2_deg = a.opt_num(3)?.unwrap_or(30.0).clamp(10.0, 80.0);
    let m1 = a.opt_num(4)?.unwrap_or(12.0).max(0.1);
    let m2 = a.opt_num(5)?.unwrap_or(70.0).max(0.1);
    let unit = a.opt_num(6)?.unwrap_or(46.0);
    let (ang1, ang2) = (a1_deg.to_radians(), a2_deg.to_radians());
    let di = DoubleIncline {
        g: 9.81,
        a1: ang1,
        a2: ang2,
        m1,
        m2,
        mu_k: 0.25,
        mu_s: 0.3,
    };
    let h_apex = 3.0f32;
    let (l1, l2) = (h_apex / ang1.sin(), h_apex / ang2.sin());
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let mut states = simulate(&di, sim_dt, substeps, SAMPLES);
    let (dl0, dr0) = (l1 * 0.5, l2 * 0.4);
    freeze_range(
        &mut states,
        0,
        1,
        -(dr0 - 0.3),
        (dl0 - 0.3).min(l2 - dr0 - 0.3),
    );
    let energy = states
        .iter()
        .map(|st| {
            let e = di.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();

    let apex = Vec2::new(center.x, center.y - h_apex * unit);
    let foot_l = Vec2::new(apex.x - l1 * ang1.cos() * unit, center.y);
    let foot_r = Vec2::new(apex.x + l2 * ang2.cos() * unit, center.y);
    let dir_l = (foot_l - apex).normalize_or_zero();
    let dir_r = (foot_r - apex).normalize_or_zero();
    // outward surface normal = the perpendicular that points UP off the slope
    let up_perp = |d: Vec2| {
        let p = Vec2::new(-d.y, d.x);
        if p.y <= 0.0 {
            p
        } else {
            Vec2::new(d.y, -d.x)
        }
    };
    let (out_l, out_r) = (up_perp(dir_l), up_perp(dir_r));
    // block half-sizes (px); seat each block by its half-height so its bottom
    // face rests flush ON the slope surface
    let (h1, h2) = (
        (34.0 + 5.0 * m1.sqrt()).min(74.0) * 0.5,
        (34.0 + 5.0 * m2.sqrt()).min(74.0) * 0.5,
    );
    let m1_pts: Vec<Vec2> = states
        .iter()
        .map(|st| apex + dir_l * ((dl0 - st[0]) * unit) + out_l * h1)
        .collect();
    let m2_pts: Vec<Vec2> = states
        .iter()
        .map(|st| apex + dir_r * ((dr0 + st[0]) * unit) + out_r * h2)
        .collect();
    // block outlines rotated to sit square on each slope (along-slope × normal basis)
    let block_poly = |along: Vec2, out: Vec2, hw: f32| {
        vec![
            -along * hw - out * hw,
            along * hw - out * hw,
            along * hw + out * hw,
            -along * hw + out * hw,
        ]
    };
    let (local1, local2) = (block_poly(dir_l, out_l, h1), block_poly(dir_r, out_r, h2));

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut wedge = Entity::new(
        format!("{id}.wedge"),
        Shape::Polygon {
            pts: vec![foot_l, apex, foot_r],
        },
        Vec2::ZERO,
        style::DIM,
    );
    wedge.stroke.fill = true;
    wedge.stroke.outline = true;
    wedge.stroke.width = 2.0;
    wedge.opacity = 0.32;
    wedge.tags = ct();
    s.add(wedge);
    let mut wheel = Entity::new(
        format!("{id}.pulley"),
        Shape::Circle { r: 13.0 },
        apex,
        style::DIM,
    );
    wheel.stroke.fill = false;
    wheel.stroke.outline = true;
    wheel.stroke.width = 3.0;
    wheel.tags = ct();
    s.add(wheel);
    for (mid, pts0, col, local) in [
        (format!("{id}.mass1"), m1_pts[0], style::CYAN, local1),
        (format!("{id}.mass2"), m2_pts[0], style::MAGENTA, local2),
    ] {
        let mut e = Entity::new(mid, Shape::Polygon { pts: local }, pts0, col);
        e.stroke.fill = true;
        e.stroke.outline = false;
        e.tags = ct();
        s.add(e);
    }
    let mut rope1 = Entity::new(
        format!("{id}.rope1"),
        Shape::Line { to: apex },
        m1_pts[0],
        style::DIM,
    );
    rope1.stroke.width = 2.0;
    rope1.tags = ct();
    s.add(rope1);
    let mut rope2 = Entity::new(
        format!("{id}.rope2"),
        Shape::Line { to: m2_pts[0] },
        apex,
        style::DIM,
    );
    rope2.stroke.width = 2.0;
    rope2.tags = ct();
    s.add(rope2);
    for (lx, txt, deg) in [
        (foot_l.x + 40.0, "θ₁", a1_deg),
        (foot_r.x - 60.0, "θ₂", a2_deg),
    ] {
        let mut l = Entity::new(
            format!("{id}.ang{deg:.0}"),
            Shape::Text {
                content: format!("{txt} {deg:.0}°"),
                size: 15.0,
            },
            Vec2::new(lx, center.y - 12.0),
            style::DIM,
        );
        l.tags = ct();
        s.add(l);
    }

    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.mass1"),
            prop: Prop::Pos,
            points: m1_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.mass2"),
            prop: Prop::Pos,
            points: m2_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.rope1"),
            prop: Prop::Pos,
            points: m1_pts,
        },
        PlaybackTrack {
            id: format!("{id}.rope2"),
            prop: Prop::To,
            points: m2_pts,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: di.labels(),
            phase_xy: di.phase_xy(),
            pos_var: di.pos_var(),
            well: di.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `inclinebumper(id, [center], [angle], [mass], [stiffness], [unit])` — a block
/// slides down an incline and compresses a **spring bumper** at the base, then
/// launches back up (one-sided contact). `center` is the foot (default `(300,
/// 500)`); `angle` degrees (default 40); `mass` kg (default 2); `stiffness` k
/// (default 500). Lays out `{id}.incline/.surface/.spring/.plate/.block`. Animate
/// with `run(id)`; `energygraph` shows KE ↔ gravity+spring PE (conserved).
fn c_inclinebumper(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(300.0, 500.0)
    };
    let angle_deg = a.opt_num(2)?.unwrap_or(40.0).clamp(10.0, 80.0);
    let mass = a.opt_num(3)?.unwrap_or(2.0).max(0.1);
    let k = a.opt_num(4)?.unwrap_or(500.0).max(1.0);
    let unit = a.opt_num(5)?.unwrap_or(70.0);
    let ang = angle_deg.to_radians();
    let (s_base, s_contact, s0, bw) = (0.25f32, 1.25f32, 4.0f32, 0.5f32);
    let ib = InclineBumper {
        g: 9.81,
        angle: ang,
        m: mass,
        k,
        mu_k: 0.0,
        s_contact,
        s0,
    };
    let ramp_len = s0 + 1.0;
    let (sim_dt, substeps) = (0.003f32, 6usize);
    let states = simulate(&ib, sim_dt, substeps, SAMPLES);
    let energy = states
        .iter()
        .map(|st| {
            let e = ib.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();

    let (ct2, st2) = (ang.cos(), ang.sin());
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let foot = center;
    let top = to_screen((ramp_len * ct2, ramp_len * st2));
    let corner = Vec2::new(top.x, foot.y);
    let on_slope = |sarc: f32| to_screen((sarc * ct2, sarc * st2));
    let block_center =
        |sarc: f32| to_screen((sarc * ct2 - bw * 0.5 * st2, sarc * st2 + bw * 0.5 * ct2));
    let block_pts: Vec<Vec2> = states.iter().map(|st| block_center(st[0])).collect();
    // the coil runs from the fixed base plate up to min(block, contact) along the slope
    let coil_base = on_slope(s_base);
    let coil_top: Vec<Vec2> = states
        .iter()
        .map(|st| on_slope(st[0].min(s_contact).max(s_base + 0.05)))
        .collect();

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut tri = Entity::new(
        format!("{id}.incline"),
        Shape::Polygon {
            pts: vec![foot, top, corner],
        },
        Vec2::ZERO,
        style::DIM,
    );
    tri.stroke.fill = true;
    tri.stroke.outline = true;
    tri.stroke.width = 2.0;
    tri.opacity = 0.35;
    tri.tags = ct();
    s.add(tri);
    let mut surf = Entity::new(
        format!("{id}.surface"),
        Shape::Line { to: top },
        foot,
        style::FG,
    );
    surf.stroke.width = 3.0;
    surf.tags = ct();
    s.add(surf);
    // base plate (a short bar perpendicular to the slope at the spring's foot)
    let u = (top - foot).normalize_or_zero();
    let perp = Vec2::new(-u.y, u.x);
    let mut plate = Entity::new(
        format!("{id}.plate"),
        Shape::Line {
            to: coil_base + perp * 18.0,
        },
        coil_base - perp * 18.0,
        style::FG,
    );
    plate.stroke.width = 4.0;
    plate.tags = ct();
    s.add(plate);
    let mut spring = Entity::new(
        format!("{id}.spring"),
        Shape::Coil {
            to: coil_top[0],
            turns: 9,
        },
        coil_base,
        style::LIME,
    );
    spring.stroke.width = 3.0;
    spring.tags = ct();
    s.add(spring);
    let hl = bw * 0.5 * unit;
    let local = vec![
        -u * hl - perp * hl,
        u * hl - perp * hl,
        u * hl + perp * hl,
        -u * hl + perp * hl,
    ];
    let mut block = Entity::new(
        format!("{id}.block"),
        Shape::Polygon { pts: local },
        block_pts[0],
        style::CYAN,
    );
    block.stroke.fill = true;
    block.stroke.outline = false;
    block.tags = ct();
    s.add(block);
    let mut al = Entity::new(
        format!("{id}.anglelabel"),
        Shape::Text {
            content: format!("{angle_deg:.0}°"),
            size: 16.0,
        },
        Vec2::new(foot.x + 46.0, foot.y - 14.0),
        style::DIM,
    );
    al.tags = ct();
    s.add(al);

    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.block"),
            prop: Prop::Pos,
            points: block_pts,
        },
        PlaybackTrack {
            id: format!("{id}.spring"),
            prop: Prop::To,
            points: coil_top,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: ib.labels(),
            phase_xy: ib.phase_xy(),
            pos_var: ib.pos_var(),
            well: ib.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `springchain(id, [center], [angle], [unit])` — three blocks joined by two
/// springs on an incline (coupled oscillators / normal modes). `center` is the
/// middle block's rest point (default `(cx, 340)`); `angle` degrees (default 25).
/// Lays out `{id}.surface/.block1/.block2/.block3/.spring1/.spring2`. Animate with
/// `run(id)`; `energygraph` shows the energy sloshing between the modes.
fn c_springchain(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 340.0)
    };
    let angle_deg = a.opt_num(2)?.unwrap_or(25.0).clamp(0.0, 60.0);
    let unit = a.opt_num(3)?.unwrap_or(80.0);
    let ang = angle_deg.to_radians();
    let sc = SpringChain {
        m: 1.0,
        k: 18.0,
        rest: 1.4,
    };
    let (sim_dt, substeps) = (0.004f32, 6usize);
    let states = simulate(&sc, sim_dt, substeps, SAMPLES);
    let energy = states
        .iter()
        .map(|st| {
            let e = sc.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();

    // slope basis: up-slope (to the right) and the outward (up-off-surface) normal
    let along = Vec2::new(ang.cos(), -ang.sin());
    let nrm = Vec2::new(-ang.sin(), -ang.cos());
    let on_slope = |x: f32| center + along * (x * unit); // x = along-slope coordinate
    let bh = 22.0f32; // block half-size (px)
    let block_pts = |i: usize| -> Vec<Vec2> {
        states
            .iter()
            .map(|st| on_slope(st[i * 2]) + nrm * bh)
            .collect()
    };
    let (b1, b2, b3) = (block_pts(0), block_pts(1), block_pts(2));

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    // a slope surface segment under the chain
    let surf_a = on_slope(-sc.rest - 2.0);
    let surf_b = on_slope(sc.rest + 2.0);
    let mut surf = Entity::new(
        format!("{id}.surface"),
        Shape::Line { to: surf_b },
        surf_a,
        style::FG,
    );
    surf.stroke.width = 3.0;
    surf.tags = ct();
    s.add(surf);
    let mut al = Entity::new(
        format!("{id}.anglelabel"),
        Shape::Text {
            content: format!("{angle_deg:.0}°"),
            size: 15.0,
        },
        surf_a + Vec2::new(30.0, 26.0),
        style::DIM,
    );
    al.tags = ct();
    s.add(al);
    // two coils between consecutive blocks
    for (sid, tail0, head0) in [
        (format!("{id}.spring1"), b1[0], b2[0]),
        (format!("{id}.spring2"), b2[0], b3[0]),
    ] {
        let mut e = Entity::new(
            sid,
            Shape::Coil {
                to: head0,
                turns: 8,
            },
            tail0,
            style::LIME,
        );
        e.stroke.width = 2.5;
        e.tags = ct();
        s.add(e);
    }
    // three blocks (rotated square, sitting on the slope)
    let hl = bh;
    let local = vec![
        -along * hl - nrm * hl,
        along * hl - nrm * hl,
        along * hl + nrm * hl,
        -along * hl + nrm * hl,
    ];
    for (bid, p0, col) in [
        (format!("{id}.block1"), b1[0], style::CYAN),
        (format!("{id}.block2"), b2[0], style::MAGENTA),
        (format!("{id}.block3"), b3[0], style::GOLD),
    ] {
        let mut e = Entity::new(bid, Shape::Polygon { pts: local.clone() }, p0, col);
        e.stroke.fill = true;
        e.stroke.outline = false;
        e.tags = ct();
        s.add(e);
    }

    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.spring1"),
            prop: Prop::Pos,
            points: b1.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.spring1"),
            prop: Prop::To,
            points: b2.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.spring2"),
            prop: Prop::Pos,
            points: b2.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.spring2"),
            prop: Prop::To,
            points: b3.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.block1"),
            prop: Prop::Pos,
            points: b1,
        },
        PlaybackTrack {
            id: format!("{id}.block2"),
            prop: Prop::Pos,
            points: b2,
        },
        PlaybackTrack {
            id: format!("{id}.block3"),
            prop: Prop::Pos,
            points: b3,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: sc.labels(),
            phase_xy: sc.phase_xy(),
            pos_var: sc.pos_var(),
            well: sc.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `looptrack(id, [center], [radius], [height], [unit])` — a ball rolls down a
/// ramp and around a vertical **loop-the-loop** (a curved track). `center` is the
/// loop's bottom point on the ground (default `(520, 560)`); `radius` metres
/// (default 1); `height` the release height above the bottom (default 3 — it must
/// exceed 2·radius to clear the top); `unit` px-per-metre (default 90). The bead
/// is a curved-track energy solver: v = √(2g·(H − y)) along arc length. Lays out
/// `{id}.ramp/.loop/.ball/.start`. Animate with `run(id)`; `energygraph` shows KE↔PE.
fn c_looptrack(s: &mut Scene, a: &Args) -> Result<(), Error> {
    use std::f32::consts::TAU;
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(520.0, 560.0)
    };
    let r = a.opt_num(2)?.unwrap_or(1.0).max(0.2);
    let big_h = a.opt_num(3)?.unwrap_or(3.0).max(2.05 * r);
    let unit = a.opt_num(4)?.unwrap_or(90.0);
    let g = 9.81f32;
    // track: ramp from start=(−H, H) down to the loop bottom (0,0), then the loop.
    let ramp_len = big_h * std::f32::consts::SQRT_2; // 45° ramp
    let s_end = ramp_len + TAU * r;
    // world position + height along arc-length s
    let track = |arc: f32| -> (f32, f32) {
        if arc <= ramp_len {
            let f = arc / ramp_len;
            (-big_h + f * big_h, big_h - f * big_h) // (−H,H) → (0,0)
        } else {
            let phi = (arc - ramp_len) / r;
            (r * phi.sin(), r * (1.0 - phi.cos())) // loop, bottom → around
        }
    };
    let height = |arc: f32| track(arc).1;
    // pass 1: total time (fine Euler on ds/dt = √(2g(H−y)))
    let fdt = 0.0006f32;
    let (mut arc, mut t) = (0.0f32, 0.0f32);
    while arc < s_end && t < 20.0 {
        let v = (2.0 * g * (big_h - height(arc))).max(1e-4).sqrt();
        arc += v * fdt;
        t += fdt;
    }
    let t_total = t;
    // pass 2: sample SAMPLES frames evenly in time
    let frame_dt = t_total / SAMPLES as f32;
    let steps = (frame_dt / fdt).max(1.0) as usize;
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let mut ball_pts = Vec::with_capacity(SAMPLES + 1);
    let mut energy = Vec::with_capacity(SAMPLES + 1);
    arc = 0.0;
    for _ in 0..=SAMPLES {
        let (wx, wy) = track(arc);
        ball_pts.push(to_screen((wx, wy)));
        let v = (2.0 * g * (big_h - wy)).max(0.0).sqrt();
        energy.push((0.5 * v * v, g * wy)); // unit mass
        for _ in 0..steps {
            let vv = (2.0 * g * (big_h - height(arc))).max(1e-4).sqrt();
            arc += vv * fdt;
            if arc >= s_end {
                arc = s_end;
                break;
            }
        }
    }

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    // ramp line + loop circle (static track)
    let mut ramp = Entity::new(
        format!("{id}.ramp"),
        Shape::Line {
            to: to_screen((0.0, 0.0)),
        },
        to_screen((-big_h, big_h)),
        style::FG,
    );
    ramp.stroke.width = 3.0;
    ramp.tags = ct();
    s.add(ramp);
    let mut loop_e = Entity::new(
        format!("{id}.loop"),
        Shape::Circle { r: r * unit },
        to_screen((0.0, r)),
        style::FG,
    );
    loop_e.stroke.fill = false;
    loop_e.stroke.outline = true;
    loop_e.stroke.width = 3.0;
    loop_e.tags = ct();
    s.add(loop_e);
    // ground line
    let mut ground = Entity::new(
        format!("{id}.ground"),
        Shape::Line {
            to: to_screen((3.0 * r, 0.0)),
        },
        to_screen((-big_h - 0.5, 0.0)),
        style::DIM,
    );
    ground.stroke.width = 2.0;
    ground.tags = ct();
    s.add(ground);
    let mut start = Entity::new(
        format!("{id}.start"),
        Shape::Circle { r: 5.0 },
        to_screen((-big_h, big_h)),
        style::DIM,
    );
    start.stroke.fill = true;
    start.stroke.outline = false;
    start.tags = ct();
    s.add(start);
    let mut ball = Entity::new(
        format!("{id}.ball"),
        Shape::Circle { r: 13.0 },
        ball_pts[0],
        style::CYAN,
    );
    ball.stroke.fill = true;
    ball.stroke.outline = false;
    ball.tags = ct();
    s.add(ball);

    let playback = vec![PlaybackTrack {
        id: format!("{id}.ball"),
        prop: Prop::Pos,
        points: ball_pts,
    }];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["arc".into(), "t".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy,
            dt: frame_dt,
            states: Vec::new(),
        },
    );
    Ok(())
}

/// `stringwave(id, [center], [width], [amp], [pluck])` — a wave on a plucked
/// string: N masses on springs, fixed at both ends. `center` is the string's
/// midpoint (default `(cx, 360)`); `width` px (default 760); `amp` the vertical
/// scale px (default 90); `pluck` where the initial peak sits, 0..1 (default 0.3).
/// The string is drawn as a rainbow chain of segments `{id}.seg{i}` that wiggle
/// as the pulse travels and reflects. Animate with `run(id)`; `energygraph` too.
fn c_stringwave(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 360.0)
    };
    let width = a.opt_num(2)?.unwrap_or(760.0).max(100.0);
    let amp = a.opt_num(3)?.unwrap_or(90.0);
    let pluck = a.opt_num(4)?.unwrap_or(0.3).clamp(0.1, 0.9);
    let n = 36usize;
    let sw = StringWave {
        n,
        k: 220.0,
        m: 0.3,
        damping: 0.03,
        pluck,
    };
    let (sim_dt, substeps) = (0.002f32, 8usize);
    let states = simulate(&sw, sim_dt, substeps, SAMPLES);
    let energy = states
        .iter()
        .map(|st| {
            let e = sw.energy(st);
            (e.kinetic, e.potential)
        })
        .collect();

    // point j (0 = left fixed end … n+1 = right fixed end) per frame
    let x_of = |j: usize| center.x - width / 2.0 + width * j as f32 / (n as f32 + 1.0);
    let pts: Vec<Vec<Vec2>> = (0..=n + 1)
        .map(|j| {
            states
                .iter()
                .map(|st| {
                    let y = if j == 0 || j == n + 1 {
                        0.0
                    } else {
                        st[(j - 1) * 2]
                    };
                    Vec2::new(x_of(j), center.y - y * amp)
                })
                .collect()
        })
        .collect();

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut playback = Vec::new();
    for j in 0..=n {
        let sid = format!("{id}.seg{j}");
        let mut e = Entity::new(
            sid.clone(),
            Shape::Line { to: pts[j + 1][0] },
            pts[j][0],
            style::CYAN,
        );
        e.stroke.width = 3.5;
        e.hue = Some(200.0 + 200.0 * j as f32 / n as f32); // a cyan→magenta gradient along the string
        e.tags = ct();
        s.add(e);
        playback.push(PlaybackTrack {
            id: sid.clone(),
            prop: Prop::Pos,
            points: pts[j].clone(),
        });
        playback.push(PlaybackTrack {
            id: sid,
            prop: Prop::To,
            points: pts[j + 1].clone(),
        });
    }
    // fixed-end posts
    for (sid, x) in [
        (format!("{id}.postL"), x_of(0)),
        (format!("{id}.postR"), x_of(n + 1)),
    ] {
        let mut p = Entity::new(
            sid,
            Shape::Line {
                to: Vec2::new(x, center.y + 22.0),
            },
            Vec2::new(x, center.y - 22.0),
            style::DIM,
        );
        p.stroke.width = 4.0;
        p.tags = ct();
        s.add(p);
    }
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: sw.labels(),
            phase_xy: sw.phase_xy(),
            pos_var: sw.pos_var(),
            well: sw.well_curve(),
            energy,
            dt: sim_dt * substeps as f32,
            states,
        },
    );
    Ok(())
}

/// `newtonscradle(id, [center], [balls], [pulled])` — Newton's cradle: a row of
/// equal pendulum balls, touching at rest. Pull `pulled` balls back on the left
/// and release — the momentum passes through the chain and the same number swing
/// out the far side. `center` is the top bar's midpoint (default `(cx, 150)`);
/// `balls` default 5; `pulled` default 1. An EVENT-DRIVEN sim: free-flight
/// pendulums between elastic collisions resolved by `collide_1d`. Lays out
/// `{id}.bar/.string{i}/.ball{i}`. Animate with `run(id)`; `energygraph` too.
fn c_newtonscradle(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 150.0)
    };
    let n = (a.opt_num(2)?.unwrap_or(5.0) as usize).clamp(2, 8);
    let pulled = (a.opt_num(3)?.unwrap_or(1.0) as usize).clamp(1, n - 1);
    let (g, l_m) = (9.81f32, 1.5f32); // pendulum physics (angle is scale-free)
    let (l_px, r_px) = (230.0f32, 22.0f32); // display: string length, ball radius
    let amp = 0.55f32; // pull-back angle (rad)
    let g_over_l = g / l_m;

    // event-driven integration: free flight + elastic (e=1) collisions on contact
    let mut th = vec![0.0f32; n];
    let mut om = vec![0.0f32; n];
    for i in 0..pulled {
        th[i] = -amp; // the leftmost `pulled` balls start pulled to the left
    }
    let fdt = 0.0004f32;
    let t_total = 6.0f32;
    let frame_dt = t_total / SAMPLES as f32;
    let sub = (frame_dt / fdt).max(1.0) as usize;
    let mut th_frames: Vec<Vec<f32>> = Vec::with_capacity(SAMPLES + 1);
    let mut energy = Vec::with_capacity(SAMPLES + 1);
    let e_of = |th: &[f32], om: &[f32]| -> (f32, f32) {
        let (mut ke, mut pe) = (0.0, 0.0);
        for i in 0..th.len() {
            ke += 0.5 * (l_m * om[i]).powi(2); // unit mass
            pe += g * l_m * (1.0 - th[i].cos());
        }
        (ke, pe)
    };
    for _ in 0..=SAMPLES {
        th_frames.push(th.clone());
        energy.push(e_of(&th, &om));
        for _ in 0..sub {
            // free flight (semi-implicit Euler — stable)
            for i in 0..n {
                om[i] += -g_over_l * th[i].sin() * fdt;
                th[i] += om[i] * fdt;
            }
            // resolve contacts: adjacent balls touch when θ_i ≥ θ_{i+1}, closing
            // when ω_i > ω_{i+1}; equal mass + e=1 via the shared resolver
            loop {
                let mut hit = false;
                for i in 0..n - 1 {
                    if th[i] >= th[i + 1] - 1e-4 && om[i] > om[i + 1] + 1e-4 {
                        let (a2, b2) = collide_1d(1.0, om[i], 1.0, om[i + 1], 1.0);
                        om[i] = a2;
                        om[i + 1] = b2;
                        let mid = 0.5 * (th[i] + th[i + 1]);
                        th[i] = mid;
                        th[i + 1] = mid; // separate to avoid re-triggering
                        hit = true;
                    }
                }
                if !hit {
                    break;
                }
            }
        }
    }

    // geometry: pivots spaced 2·r on the bar; ball hangs at angle θ
    let x0 = |i: usize| center.x + (i as f32 - (n as f32 - 1.0) / 2.0) * 2.0 * r_px;
    let ball_pts = |i: usize| -> Vec<Vec2> {
        th_frames
            .iter()
            .map(|th| Vec2::new(x0(i) + l_px * th[i].sin(), center.y + l_px * th[i].cos()))
            .collect()
    };

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut bar = Entity::new(
        format!("{id}.bar"),
        Shape::Line {
            to: Vec2::new(x0(n - 1) + 30.0, center.y),
        },
        Vec2::new(x0(0) - 30.0, center.y),
        style::DIM,
    );
    bar.stroke.width = 5.0;
    bar.tags = ct();
    s.add(bar);
    let mut playback = Vec::new();
    for i in 0..n {
        let bp = ball_pts(i);
        let (sid, bid) = (format!("{id}.string{i}"), format!("{id}.ball{i}"));
        let mut st = Entity::new(
            sid.clone(),
            Shape::Line { to: bp[0] },
            Vec2::new(x0(i), center.y),
            style::DIM,
        );
        st.stroke.width = 1.5;
        st.tags = ct();
        s.add(st);
        let mut ball = Entity::new(bid.clone(), Shape::Circle { r: r_px }, bp[0], style::GOLD);
        ball.stroke.fill = true;
        ball.stroke.outline = false;
        ball.tags = ct();
        s.add(ball);
        playback.push(PlaybackTrack {
            id: sid,
            prop: Prop::To,
            points: bp.clone(),
        });
        playback.push(PlaybackTrack {
            id: bid,
            prop: Prop::Pos,
            points: bp,
        });
    }
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["θ".into(), "t".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy,
            dt: frame_dt,
            states: Vec::new(),
        },
    );
    Ok(())
}

/// Block 1's spring rest position (metres from the left wall).
const CB_X1_EQ: f32 = 1.7;

/// Event-driven trajectory of the colliding-blocks demo: `[x₁,v₁,x₂,v₂]` per frame.
/// Block 1 (left) is attached to the wall by a **spring** (stiffness `k`) — it
/// oscillates; block 2 (right) slides in freely. Block–block contacts use
/// `collide_1d(e)`; block 2 bounces off the right wall. Returned so the physics
/// (total mechanical energy, momentum) can be checked directly.
fn sim_collideblocks(
    m1: f32,
    m2: f32,
    e: f32,
    track: f32,
    w1: f32,
    w2: f32,
    k: f32,
) -> Vec<[f32; 4]> {
    let (xl, xr) = (0.0f32, track);
    let (mut x1, mut v1) = (CB_X1_EQ, 0.0f32);
    let (mut x2, mut v2) = (xr - w2 * 0.5 - 0.5, -2.6f32); // block 2 slides in from the right
    let fdt = 0.0005f32;
    let frame_dt = 7.0 / SAMPLES as f32;
    let sub = (frame_dt / fdt).max(1.0) as usize;
    let mut frames = Vec::with_capacity(SAMPLES + 1);
    for _ in 0..=SAMPLES {
        frames.push([x1, v1, x2, v2]);
        for _ in 0..sub {
            v1 += -(k / m1) * (x1 - CB_X1_EQ) * fdt; // the spring pulls block 1 to rest
            x1 += v1 * fdt;
            x2 += v2 * fdt;
            if x1 + w1 * 0.5 >= x2 - w2 * 0.5 && v1 > v2 {
                let (a, b) = collide_1d(m1, v1, m2, v2, e);
                v1 = a;
                v2 = b;
                let ov = (x1 + w1 * 0.5) - (x2 - w2 * 0.5);
                x1 -= ov * 0.5;
                x2 += ov * 0.5;
            }
            if x2 + w2 * 0.5 >= xr && v2 > 0.0 {
                v2 = -v2;
                x2 = xr - w2 * 0.5;
            }
            if x1 - w1 * 0.5 <= xl && v1 < 0.0 {
                v1 = -v1;
                x1 = xl + w1 * 0.5;
            }
        }
    }
    frames
}

/// Trajectory of a bullet fired into a block (perfectly inelastic — it embeds):
/// `[x_b,v_b,x_block,v_block]` per frame. Both phases are constant-velocity, so it
/// is exact in closed form; on contact `collide_1d(e=0)` gives the common velocity
/// m_b·v_b/(m_b+M). The frames are **time-warped** — 55% of them slow-mo the (very
/// short) flight so the bullet's journey is watchable, the rest cover the crawl.
fn sim_bulletblock(mb: f32, vb: f32, mbig: f32, track: f32, w_block: f32) -> Vec<[f32; 4]> {
    let xr = track;
    let x_start = 0.35f32; // muzzle
    let xk0 = track * 0.60; // block sits right-of-centre — plenty of runway
    let v_after = mb * vb / (mb + mbig); // collide_1d(e=0) common velocity
    let t_embed = (xk0 - w_block * 0.5 - x_start) / vb;
    let t_stop = t_embed + (xr - w_block * 0.5 - xk0) / v_after.max(0.05);
    let nf = (SAMPLES as f32 * 0.55) as usize; // frames spent on the flight (slow-mo)
    let nc = (SAMPLES - nf).max(1);
    let mut frames = Vec::with_capacity(SAMPLES + 1);
    for i in 0..=SAMPLES {
        let t = if i <= nf {
            t_embed * i as f32 / nf as f32
        } else {
            t_embed + (t_stop - t_embed) * (i - nf) as f32 / nc as f32
        };
        if t < t_embed {
            frames.push([x_start + vb * t, vb, xk0, 0.0]); // bullet flies, block at rest
        } else {
            let xk = (xk0 + v_after * (t - t_embed)).min(xr - w_block * 0.5);
            let vk = if xk >= xr - w_block * 0.5 - 1e-3 {
                0.0
            } else {
                v_after
            };
            frames.push([xk - w_block * 0.5 + 0.04, vk, xk, vk]); // embedded, crawling
        }
    }
    frames
}

/// `collideblocks(id, [center], [m1], [m2], [restitution], [unit])` — the classic
/// momentum demo. Block 1 (left) is **attached to the wall by a spring**; block 2
/// (right) slides in freely and they collide with restitution `e` (default 1 =
/// elastic → total mechanical energy conserved; <1 → energy lost), block 2 also
/// bouncing off the right wall. A live **Σp readout** (`{id}.mom`) shows momentum
/// is conserved at each collision. `center` is the track midpoint (default
/// `(cx, 430)`). Lays out `{id}.floor/.wallL/.wallR/.spring/.block1/.block2/.mom`.
/// Animate with `run(id)`; `energygraph` shows KE ↔ spring PE.
fn c_collideblocks(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 430.0)
    };
    let m1 = a.opt_num(2)?.unwrap_or(3.0).max(0.1);
    let m2 = a.opt_num(3)?.unwrap_or(1.0).max(0.1);
    let e = a.opt_num(4)?.unwrap_or(1.0).clamp(0.0, 1.0);
    let unit = a.opt_num(5)?.unwrap_or(100.0);
    let (track, k) = (6.0f32, 60.0f32);
    let (w1, w2) = (0.4 + 0.16 * m1.sqrt(), 0.4 + 0.16 * m2.sqrt());
    let frames = sim_collideblocks(m1, m2, e, track, w1, w2, k);
    let sx = |wx: f32| center.x - track * 0.5 * unit + wx * unit;
    let floor_y = center.y;
    let (hw1, hw2) = (w1 * 0.5 * unit, w2 * 0.5 * unit);
    let b1_pts: Vec<Vec2> = frames
        .iter()
        .map(|f| Vec2::new(sx(f[0]), floor_y - hw1))
        .collect();
    let b2_pts: Vec<Vec2> = frames
        .iter()
        .map(|f| Vec2::new(sx(f[2]), floor_y - hw2))
        .collect();
    let coil_end: Vec<Vec2> = frames
        .iter()
        .map(|f| Vec2::new(sx(f[0]) - hw1, floor_y - hw1))
        .collect();
    // KE of both blocks + the spring's potential energy — conserved when elastic
    let energy = frames
        .iter()
        .map(|f| {
            (
                0.5 * m1 * f[1] * f[1] + 0.5 * m2 * f[3] * f[3],
                0.5 * k * (f[0] - CB_X1_EQ).powi(2),
            )
        })
        .collect();
    let momentum: Vec<Vec2> = frames
        .iter()
        .map(|f| Vec2::new(m1 * f[1] + m2 * f[3], 0.0))
        .collect();

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut floor = Entity::new(
        format!("{id}.floor"),
        Shape::Line {
            to: Vec2::new(sx(track), floor_y),
        },
        Vec2::new(sx(0.0), floor_y),
        style::FG,
    );
    floor.stroke.width = 3.0;
    floor.tags = ct();
    s.add(floor);
    for (wid, x) in [
        (format!("{id}.wallL"), sx(0.0)),
        (format!("{id}.wallR"), sx(track)),
    ] {
        let mut w = Entity::new(
            wid,
            Shape::Line {
                to: Vec2::new(x, floor_y - 150.0),
            },
            Vec2::new(x, floor_y),
            style::DIM,
        );
        w.stroke.width = 5.0;
        w.tags = ct();
        s.add(w);
    }
    // the spring: left wall → block 1's left edge (stretches/compresses as it moves)
    let mut spring = Entity::new(
        format!("{id}.spring"),
        Shape::Coil {
            to: coil_end[0],
            turns: 8,
        },
        Vec2::new(sx(0.0), floor_y - hw1),
        style::LIME,
    );
    spring.stroke.width = 3.0;
    spring.tags = ct();
    s.add(spring);
    for (bid, p0, wpx, col) in [
        (format!("{id}.block1"), b1_pts[0], w1 * unit, style::CYAN),
        (format!("{id}.block2"), b2_pts[0], w2 * unit, style::MAGENTA),
    ] {
        let mut e2 = Entity::new(bid, Shape::Rect { w: wpx, h: wpx }, p0, col);
        e2.stroke.fill = true;
        e2.stroke.outline = false;
        e2.tags = ct();
        s.add(e2);
    }
    let mut lbl = Entity::new(
        format!("{id}.elabel"),
        Shape::Text {
            content: format!("e = {e:.1}"),
            size: 16.0,
        },
        Vec2::new(center.x, floor_y - 168.0),
        style::DIM,
    );
    lbl.tags = ct();
    s.add(lbl);
    // the live momentum readout — conserved at each collision
    let mcounter = crate::primitives::Counter {
        value: momentum[0].x,
        decimals: 1,
        prefix: "Σp = ".into(),
        suffix: " kg·m/s".into(),
    };
    let mut mom = Entity::new(
        format!("{id}.mom"),
        Shape::Text {
            content: mcounter.render(),
            size: 22.0,
        },
        Vec2::new(center.x, 46.0),
        style::GOLD,
    );
    mom.counter = Some(mcounter);
    mom.tags = ct();
    s.add(mom);

    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.block1"),
            prop: Prop::Pos,
            points: b1_pts,
        },
        PlaybackTrack {
            id: format!("{id}.block2"),
            prop: Prop::Pos,
            points: b2_pts,
        },
        PlaybackTrack {
            id: format!("{id}.spring"),
            prop: Prop::To,
            points: coil_end,
        },
        PlaybackTrack {
            id: format!("{id}.mom"),
            prop: Prop::Value,
            points: momentum,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["x".into(), "v".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy,
            dt: 7.0 / SAMPLES as f32,
            states: Vec::new(),
        },
    );
    Ok(())
}

/// `bulletblock(id, [center], [bulletmass], [speed], [blockmass], [unit])` — a
/// bullet fired into a block **embeds** (perfectly inelastic). The combined mass
/// crawls off at m_b·v_b/(m_b+M) — a dramatic speed drop, most of the kinetic
/// energy lost to the collision (`energygraph`'s total STEPS DOWN at impact).
/// `center` is the track midpoint (default `(cx, 430)`). Lays out `{id}.floor/
/// .block/.bullet`. Animate with `run(id)`.
fn c_bulletblock(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 430.0)
    };
    let mb = a.opt_num(2)?.unwrap_or(0.05).max(0.001);
    let vb = a.opt_num(3)?.unwrap_or(40.0).max(1.0);
    let mbig = a.opt_num(4)?.unwrap_or(1.95).max(0.1);
    let unit = a.opt_num(5)?.unwrap_or(150.0);
    let track = 6.0f32;
    let w_block = 0.5 + 0.12 * mbig.sqrt();
    let frames = sim_bulletblock(mb, vb, mbig, track, w_block);
    let sx = |wx: f32| center.x - track * 0.5 * unit + wx * unit;
    let floor_y = center.y;
    let bullet_y = floor_y - w_block * 0.5 * unit;
    let block_pts: Vec<Vec2> = frames
        .iter()
        .map(|f| Vec2::new(sx(f[2]), bullet_y))
        .collect();
    let bullet_pts: Vec<Vec2> = frames
        .iter()
        .map(|f| Vec2::new(sx(f[0]), bullet_y))
        .collect();
    // KE of the whole system (bullet + block); steps down at the inelastic impact
    let energy = frames
        .iter()
        .map(|f| (0.5 * mb * f[1] * f[1] + 0.5 * mbig * f[3] * f[3], 0.0))
        .collect();
    let speed: Vec<Vec2> = frames.iter().map(|f| Vec2::new(f[1], 0.0)).collect(); // the projectile's speed

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut floor = Entity::new(
        format!("{id}.floor"),
        Shape::Line {
            to: Vec2::new(sx(track), floor_y),
        },
        Vec2::new(sx(0.0), floor_y),
        style::FG,
    );
    floor.stroke.width = 3.0;
    floor.tags = ct();
    s.add(floor);
    let bw = w_block * unit;
    let mut block = Entity::new(
        format!("{id}.block"),
        Shape::Rect { w: bw, h: bw },
        block_pts[0],
        style::CYAN,
    );
    block.stroke.fill = true;
    block.stroke.outline = false;
    block.tags = ct();
    s.add(block);
    let mut bullet = Entity::new(
        format!("{id}.bullet"),
        Shape::Circle { r: 11.0 },
        bullet_pts[0],
        style::RED,
    );
    bullet.stroke.fill = true;
    bullet.stroke.outline = false;
    bullet.glow = 1.6;
    bullet.tags = ct();
    s.add(bullet);
    // live speed readout: the projectile's speed crashes at impact
    let vcounter = crate::primitives::Counter {
        value: speed[0].x,
        decimals: 0,
        prefix: "v = ".into(),
        suffix: " m/s".into(),
    };
    let mut vel = Entity::new(
        format!("{id}.vel"),
        Shape::Text {
            content: vcounter.render(),
            size: 24.0,
        },
        Vec2::new(center.x, floor_y - 150.0),
        style::RED,
    );
    vel.counter = Some(vcounter);
    vel.tags = ct();
    s.add(vel);

    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.block"),
            prop: Prop::Pos,
            points: block_pts,
        },
        PlaybackTrack {
            id: format!("{id}.bullet"),
            prop: Prop::Pos,
            points: bullet_pts,
        },
        PlaybackTrack {
            id: format!("{id}.vel"),
            prop: Prop::Value,
            points: speed,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["x".into(), "v".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy,
            dt: 4.0 / SAMPLES as f32,
            states: Vec::new(),
        },
    );
    Ok(())
}

/// `dropmass(id, [center], [dropheight], [unit])` — a mass dropped onto a block
/// resting on a spring: it free-falls, sticks in a perfectly **inelastic
/// collision** (energy is lost — the energy total steps down), then the heavier
/// combined mass oscillates about a lower equilibrium. `center` is the ceiling
/// anchor (default `(640, 150)`); `dropheight` metres above the block (default
/// 1.2); `unit` px-per-metre (default 80). Lays out `{id}.ceiling`, `{id}.spring`,
/// `{id}.block`, `{id}.drop`, and the two equilibrium markers `{id}.eq1/.eq2`.
/// Animate with `run(id)`; `energygraph` shows the collision energy loss.
fn c_dropmass(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 150.0)
    };
    let h = a.opt_num(2)?.unwrap_or(1.2).max(0.2);
    let unit = a.opt_num(3)?.unwrap_or(80.0);
    let (m1, m2, k, l0, g) = (1.0f32, 0.5f32, 20.0f32, 1.0f32, 9.81f32);
    let big_m = m1 + m2;
    let eq1 = l0 + m1 * g / k; // block equilibrium depth as m1 alone
    let eq2 = l0 + big_m * g / k; // deeper equilibrium once m2 sticks
    let shelf = eq1 - h; // drop starts h above the block
    let t_fall = (2.0 * h / g).sqrt();
    let v_imp = (2.0 * g * h).sqrt();
    let v_after = m2 * v_imp / big_m;
    let omega = (k / big_m).sqrt();
    let u0 = eq1 - eq2; // block starts above the new equilibrium (< 0)
    let spring_c = 0.5 * k * (eq1 - l0).powi(2); // constant spring PE while block sits at eq1
    let t_total = t_fall + 3.5;
    let dt = t_total / SAMPLES as f32;

    let mut block_d = Vec::with_capacity(SAMPLES + 1);
    let mut drop_d = Vec::with_capacity(SAMPLES + 1);
    let mut energy = Vec::with_capacity(SAMPLES + 1);
    let mut states = Vec::with_capacity(SAMPLES + 1);
    for kf in 0..=SAMPLES {
        let t = kf as f32 * dt;
        if t < t_fall {
            let dd = shelf + 0.5 * g * t * t;
            let vdrop = g * t;
            block_d.push(eq1);
            drop_d.push(dd);
            // datum at eq1: block PE ≡ spring_c; falling-mass energy is conserved
            energy.push((0.5 * m2 * vdrop * vdrop, spring_c - m2 * g * (dd - eq1)));
            states.push(vec![eq1, 0.0, t]);
        } else {
            let tp = t - t_fall;
            let bd = eq2 + u0 * (omega * tp).cos() + (v_after / omega) * (omega * tp).sin();
            let bv = -u0 * omega * (omega * tp).sin() + v_after * (omega * tp).cos();
            block_d.push(bd);
            drop_d.push(bd);
            energy.push((
                0.5 * big_m * bv * bv,
                0.5 * k * (bd - l0).powi(2) - big_m * g * (bd - eq1),
            ));
            states.push(vec![bd, bv, t]);
        }
    }
    let sy = |d: f32| center.y + d * unit;
    let block_pts: Vec<Vec2> = block_d
        .iter()
        .map(|&d| Vec2::new(center.x, sy(d)))
        .collect();
    let drop_pts: Vec<Vec2> = drop_d
        .iter()
        .enumerate()
        .map(|(i, &d)| {
            let y = if states[i][2] < t_fall {
                sy(d)
            } else {
                sy(d) - 38.0
            };
            Vec2::new(center.x, y)
        })
        .collect();

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let mut ceil = Entity::new(
        format!("{id}.ceiling"),
        Shape::Line {
            to: Vec2::new(center.x + 110.0, center.y),
        },
        Vec2::new(center.x - 110.0, center.y),
        style::DIM,
    );
    ceil.stroke.width = 5.0;
    ceil.tags = ct();
    s.add(ceil);
    for (base, d, col, txt) in [
        (format!("{id}.eq1"), eq1, style::LIME, "eq (m₁)"),
        (format!("{id}.eq2"), eq2, style::GOLD, "eq (m₁+m₂)"),
    ] {
        let mut m = Entity::new(
            base.clone(),
            Shape::Line {
                to: Vec2::new(center.x + 90.0, sy(d)),
            },
            Vec2::new(center.x - 90.0, sy(d)),
            col,
        );
        m.stroke.width = 1.5;
        m.opacity = 0.5;
        m.tags = ct();
        s.add(m);
        let mut lbl = Entity::new(
            format!("{base}.label"),
            Shape::Text {
                content: txt.into(),
                size: 13.0,
            },
            Vec2::new(center.x + 150.0, sy(d)),
            col,
        );
        lbl.opacity = 0.7;
        lbl.tags = ct();
        s.add(lbl);
    }
    let mut spring = Entity::new(
        format!("{id}.spring"),
        Shape::Coil {
            to: block_pts[0],
            turns: 10,
        },
        center,
        style::LIME,
    );
    spring.stroke.width = 3.0;
    spring.tags = ct();
    s.add(spring);
    let mut block = Entity::new(
        format!("{id}.block"),
        Shape::Rect { w: 46.0, h: 30.0 },
        block_pts[0],
        style::CYAN,
    );
    block.stroke.fill = true;
    block.stroke.outline = false;
    block.tags = ct();
    s.add(block);
    let mut drop = Entity::new(
        format!("{id}.drop"),
        Shape::Rect { w: 32.0, h: 26.0 },
        drop_pts[0],
        style::MAGENTA,
    );
    drop.stroke.fill = true;
    drop.stroke.outline = false;
    drop.tags = ct();
    s.add(drop);

    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.spring"),
            prop: Prop::To,
            points: block_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.block"),
            prop: Prop::Pos,
            points: block_pts,
        },
        PlaybackTrack {
            id: format!("{id}.drop"),
            prop: Prop::Pos,
            points: drop_pts,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["d".into(), "v".into(), "t".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy,
            dt,
            states,
        },
    );
    Ok(())
}

/// `raft(id, [center], [personmass], [raftmass], [unit])` — a person walking back
/// and forth on a floating raft. Momentum is conserved, so the centre of mass
/// stays fixed: the raft slides the opposite way, by −m_person/(m_person+m_raft)
/// of the person's step. `center` is the rest centre on the waterline (default
/// `(640, 380)`); masses in kg (default 70, 200); `unit` px-per-metre (default
/// 46). Lays out `{id}.water`, `{id}.cm`, `{id}.raft`, `{id}.body`, `{id}.head`.
/// Animate with `run(id)`. A kinematic constraint demo — no energy/phase views.
fn c_raft(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(640.0, 380.0)
    };
    let mp = a.opt_num(2)?.unwrap_or(70.0).max(1.0);
    let mr = a.opt_num(3)?.unwrap_or(200.0).max(1.0);
    let unit = a.opt_num(4)?.unwrap_or(46.0);
    let raft_len = 6.0f32;
    let walk = 1.0f32;
    let edge = raft_len / 2.0 - 0.3;
    let period = 4.0 * edge / walk;
    let dt = period / SAMPLES as f32;

    let mut states = Vec::with_capacity(SAMPLES + 1);
    let mut raftx = Vec::with_capacity(SAMPLES + 1);
    let mut personx = Vec::with_capacity(SAMPLES + 1);
    for kf in 0..=SAMPLES {
        let t = kf as f32 * dt;
        let ph = (t / period).fract() * 4.0; // 0..4 quarters
        let d = if ph < 1.0 {
            edge * ph
        } else if ph < 3.0 {
            edge * (2.0 - ph)
        } else {
            edge * (ph - 4.0)
        };
        let rx = -mp * d / (mp + mr);
        raftx.push(rx);
        personx.push(rx + d);
        states.push(vec![d, t]);
    }

    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];
    let raft_h = 16.0;
    let person_h = 50.0;
    let mut water = Entity::new(
        format!("{id}.water"),
        Shape::Line {
            to: Vec2::new(center.x + 360.0, center.y),
        },
        Vec2::new(center.x - 360.0, center.y),
        style::CYAN,
    );
    water.stroke.width = 2.0;
    water.opacity = 0.5;
    water.tags = ct();
    s.add(water);
    let mut cm = Entity::new(
        format!("{id}.cm"),
        Shape::Line {
            to: Vec2::new(center.x, center.y + 24.0),
        },
        Vec2::new(center.x, center.y - 150.0),
        style::DIM,
    );
    cm.stroke.width = 1.5;
    cm.opacity = 0.6;
    cm.tags = ct();
    s.add(cm);
    let mut cmlbl = Entity::new(
        format!("{id}.cmlabel"),
        Shape::Text {
            content: "CM (fixed)".into(),
            size: 14.0,
        },
        Vec2::new(center.x, center.y - 160.0),
        style::DIM,
    );
    cmlbl.tags = ct();
    s.add(cmlbl);

    let raft_pts: Vec<Vec2> = raftx
        .iter()
        .map(|&x| Vec2::new(center.x + x * unit, center.y))
        .collect();
    let feet_pts: Vec<Vec2> = personx
        .iter()
        .map(|&x| Vec2::new(center.x + x * unit, center.y - raft_h * 0.5))
        .collect();
    let head_pts: Vec<Vec2> = personx
        .iter()
        .map(|&x| Vec2::new(center.x + x * unit, center.y - raft_h * 0.5 - person_h))
        .collect();
    let mut raft = Entity::new(
        format!("{id}.raft"),
        Shape::Rect {
            w: raft_len * unit,
            h: raft_h,
        },
        raft_pts[0],
        style::GOLD,
    );
    raft.stroke.fill = true;
    raft.stroke.outline = false;
    raft.tags = ct();
    s.add(raft);
    let mut body = Entity::new(
        format!("{id}.body"),
        Shape::Line { to: head_pts[0] },
        feet_pts[0],
        style::MAGENTA,
    );
    body.stroke.width = 5.0;
    body.tags = ct();
    s.add(body);
    let mut head = Entity::new(
        format!("{id}.head"),
        Shape::Circle { r: 11.0 },
        head_pts[0],
        style::MAGENTA,
    );
    head.stroke.fill = true;
    head.stroke.outline = false;
    head.tags = ct();
    s.add(head);

    let playback = vec![
        PlaybackTrack {
            id: format!("{id}.raft"),
            prop: Prop::Pos,
            points: raft_pts,
        },
        PlaybackTrack {
            id: format!("{id}.body"),
            prop: Prop::Pos,
            points: feet_pts,
        },
        PlaybackTrack {
            id: format!("{id}.body"),
            prop: Prop::To,
            points: head_pts.clone(),
        },
        PlaybackTrack {
            id: format!("{id}.head"),
            prop: Prop::Pos,
            points: head_pts,
        },
    ];
    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["d".into(), "t".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy: states.iter().map(|_| (0.0, 0.0)).collect(),
            dt,
            states,
        },
    );
    Ok(())
}

/// `brachistochrone(id, [center], [unit])` — four beads race under gravity from
/// A=(0,0) down to B=(3,2) along four curves (straight, circular arc, parabola,
/// **cycloid**); the cycloid — the curve of fastest descent — wins. `center` is
/// the start point A (default `(360, 130)`); `unit` px-per-metre (default 130).
/// Lays out each curve `{id}.straight/.circle/.parabola/.cycloid`, a bead
/// `{id}.bead_*` on each, and the `{id}.markA/.markB` endpoints. Every bead is a
/// full RK4 bead-on-wire integration. Animate with `run(id)`.
fn c_brachistochrone(s: &mut Scene, a: &Args) -> Result<(), Error> {
    let id = a.ident(0)?;
    let center = if a.len() >= 2 {
        a.pair(1)?
    } else {
        Vec2::new(360.0, 130.0)
    };
    let unit = a.opt_num(2)?.unwrap_or(130.0);
    let (dd, hh, g) = (3.0f32, 2.0f32, 9.81f32);
    let curves = [
        ("straight", Curve::Straight { m: hh / dd }, style::DIM),
        (
            "circle",
            Curve::Circle {
                r: (dd * dd + hh * hh) / (2.0 * hh),
            },
            style::CYAN,
        ),
        ("parabola", Curve::Parabola { h: hh, d: dd }, style::GOLD),
        ("cycloid", build_cycloid(dd, hh), style::MAGENTA),
    ];
    let (sim_dt, substeps) = (0.002f32, 4usize);
    let to_screen = |(wx, wy): (f32, f32)| Vec2::new(center.x + wx * unit, center.y - wy * unit);
    let parts = format!("{id}.parts");
    let ct = || vec![id.clone(), parts.clone()];

    let mut playback = Vec::new();
    let mut cyc_states: Vec<Vec<f32>> = Vec::new();
    let mut cyc_energy: Vec<(f32, f32)> = Vec::new();
    for (name, curve, col) in curves.into_iter() {
        let bead = Bead {
            g,
            b: 0.0,
            d: dd,
            h: hh,
            curve: curve.clone(),
        };
        let states = simulate(&bead, sim_dt, substeps, SAMPLES);
        let npc = 90;
        let cpts: Vec<Vec2> = (0..=npc)
            .map(|i| {
                let x = dd * i as f32 / npc as f32;
                to_screen((x, -curve.y(x)))
            })
            .collect();
        let mut cl = Entity::new(
            format!("{id}.{name}"),
            Shape::Polyline { pts: cpts },
            Vec2::ZERO,
            col,
        );
        cl.stroke.width = if name == "cycloid" { 3.0 } else { 2.0 };
        cl.opacity = 0.7;
        cl.tags = ct();
        s.add(cl);
        let bpts: Vec<Vec2> = states.iter().map(|st| to_screen(bead.body(st))).collect();
        let mut be = Entity::new(
            format!("{id}.bead_{name}"),
            Shape::Circle { r: 8.0 },
            bpts[0],
            col,
        );
        be.stroke.fill = true;
        be.stroke.outline = false;
        be.tags = ct();
        s.add(be);
        playback.push(PlaybackTrack {
            id: format!("{id}.bead_{name}"),
            prop: Prop::Pos,
            points: bpts,
        });
        if name == "cycloid" {
            cyc_energy = states
                .iter()
                .map(|st| {
                    let e = bead.energy(st);
                    (e.kinetic, e.potential)
                })
                .collect();
            cyc_states = states;
        }
    }
    for (sid, pt, txt) in [
        (format!("{id}.markA"), to_screen((0.0, 0.0)), "A"),
        (format!("{id}.markB"), to_screen((dd, -hh)), "B"),
    ] {
        let mut m = Entity::new(sid, Shape::Circle { r: 6.0 }, pt, style::FG);
        m.stroke.fill = true;
        m.stroke.outline = false;
        m.tags = ct();
        s.add(m);
        let mut lbl = Entity::new(
            format!("{id}.label{txt}"),
            Shape::Text {
                content: txt.into(),
                size: 18.0,
            },
            Vec2::new(pt.x - 16.0, pt.y),
            style::FG,
        );
        lbl.tags = ct();
        s.add(lbl);
    }

    s.sims.insert(
        id,
        SimData {
            playback,
            labels: vec!["x".into(), "v".into(), "t".into()],
            phase_xy: None,
            pos_var: None,
            well: Vec::new(),
            energy: cyc_energy,
            dt: sim_dt * substeps as f32,
            states: cyc_states,
        },
    );
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
        Shape::Rect {
            w: 2.0 * half,
            h: 2.0 * half,
        },
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
        Shape::Text {
            content: title.to_string(),
            size: 15.0,
        },
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
            Error::new(
                format!("no sim `{id}` — call a sim ctor (e.g. `pendulum`) before `phase`"),
                a.span_of(0),
            )
        })?;
        let (xi, yi) = sim
            .phase_xy
            .ok_or_else(|| Error::new(format!("sim `{id}` has no phase portrait"), a.span_of(0)))?;
        if sim.states.len() < 2 {
            return Err(Error::new(
                format!("sim `{id}` has no trajectory"),
                a.span_of(0),
            ));
        }
        let l = &sim.labels;
        (
            sim.states
                .iter()
                .map(|st| (st[xi], st[yi]))
                .collect::<Vec<_>>(),
            l.get(xi).cloned().unwrap_or_default(),
            l.get(yi).cloned().unwrap_or_default(),
        )
    };
    let fit = panel_fit(&pts, center, half);
    let screen: Vec<Vec2> = pts.iter().map(|&(x, y)| fit(x, y)).collect();
    let base = format!("{id}.phase");
    let tags = vec![id.clone(), format!("{id}.parts"), base.clone()];
    add_panel(
        s,
        &base,
        center,
        half,
        &format!("phase: {yl} vs {xl}"),
        &tags,
    );

    let mut curve = Entity::new(
        format!("{base}.curve"),
        Shape::Polyline {
            pts: screen.clone(),
        },
        Vec2::ZERO,
        style::LIME,
    );
    curve.stroke.width = 2.0;
    curve.opacity = 0.75;
    curve.tags = tags.clone();
    s.add(curve);

    let mut dot = Entity::new(
        format!("{base}.dot"),
        Shape::Circle { r: 6.0 },
        screen[0],
        style::GOLD,
    );
    dot.stroke.fill = true;
    dot.stroke.outline = false;
    dot.tags = tags.clone();
    s.add(dot);

    if let Some(sim) = s.sims.get_mut(&id) {
        sim.playback.push(PlaybackTrack {
            id: format!("{base}.dot"),
            prop: Prop::Pos,
            points: screen,
        });
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
            Error::new(
                format!("no sim `{id}` — call a sim ctor (e.g. `pendulum`) before `well`"),
                a.span_of(0),
            )
        })?;
        if sim.well.is_empty() {
            return Err(Error::new(
                format!("sim `{id}` has no potential well"),
                a.span_of(0),
            ));
        }
        let posi = sim.pos_var.ok_or_else(|| {
            Error::new(format!("sim `{id}` has no position variable"), a.span_of(0))
        })?;
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

    let mut ball = Entity::new(
        format!("{base}.ball"),
        Shape::Circle { r: 8.0 },
        ball_screen[0],
        style::CYAN,
    );
    ball.stroke.fill = true;
    ball.stroke.outline = false;
    ball.tags = tags.clone();
    s.add(ball);

    if let Some(sim) = s.sims.get_mut(&id) {
        sim.playback.push(PlaybackTrack {
            id: format!("{base}.ball"),
            prop: Prop::Pos,
            points: ball_screen,
        });
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
        let pts: Vec<Vec2> = vals
            .iter()
            .enumerate()
            .map(|(k, &v)| fit(k as f32 * dt, v))
            .collect();
        let mut e = Entity::new(
            format!("{base}.c{i}"),
            Shape::Polyline { pts },
            Vec2::ZERO,
            *col,
        );
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
        Shape::Line {
            to: Vec2::new(xs[0], bot),
        },
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
            Error::new(
                format!("no sim `{id}` — call a sim ctor before `timegraph`"),
                a.span_of(0),
            )
        })?;
        let (xi, yi) = sim.phase_xy.ok_or_else(|| {
            Error::new(
                format!("sim `{id}` has no time-series variables"),
                a.span_of(0),
            )
        })?;
        if sim.states.len() < 2 {
            return Err(Error::new(
                format!("sim `{id}` has no trajectory"),
                a.span_of(0),
            ));
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
    add_time_view(
        s,
        &id,
        &format!("{id}.time"),
        center,
        half,
        &title,
        dt,
        &series,
        &tags,
    );
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
            Error::new(
                format!("no sim `{id}` — call a sim ctor before `energygraph`"),
                a.span_of(0),
            )
        })?;
        if sim.energy.len() < 2 {
            return Err(Error::new(
                format!("sim `{id}` has no energy series"),
                a.span_of(0),
            ));
        }
        let ke: Vec<f32> = sim.energy.iter().map(|e| e.0).collect();
        let pe: Vec<f32> = sim.energy.iter().map(|e| e.1).collect();
        let total: Vec<f32> = sim.energy.iter().map(|e| e.0 + e.1).collect();
        (
            vec![
                (style::CYAN, ke),
                (style::MAGENTA, pe),
                (style::GOLD, total),
            ],
            sim.dt,
        )
    };
    let tags = vec![id.clone(), format!("{id}.parts"), format!("{id}.energy")];
    add_time_view(
        s,
        &id,
        &format!("{id}.energy"),
        center,
        half,
        "energy: KE PE total",
        dt,
        &series,
        &tags,
    );
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
    r.ctor("piston", c_piston);
    r.ctor("molecule", c_molecule);
    r.ctor("robotarm", c_robotarm);
    r.ctor("pulley", c_pulley);
    r.ctor("pulleyscale", c_pulleyscale);
    r.ctor("blocktackle", c_blocktackle);
    r.ctor("compoundpulley", c_compoundpulley);
    r.ctor("ramp", c_ramp);
    r.ctor("inclinepulley", c_inclinepulley);
    r.ctor("doubleincline", c_doubleincline);
    r.ctor("inclinebumper", c_inclinebumper);
    r.ctor("springchain", c_springchain);
    r.ctor("looptrack", c_looptrack);
    r.ctor("stringwave", c_stringwave);
    r.ctor("newtonscradle", c_newtonscradle);
    r.ctor("collideblocks", c_collideblocks);
    r.ctor("bulletblock", c_bulletblock);
    r.ctor("dropmass", c_dropmass);
    r.ctor("raft", c_raft);
    r.ctor("brachistochrone", c_brachistochrone);
    // playback (`run` is the generic name; `swing` is a pendulum-friendly alias)
    r.verb("run", v_play);
    r.verb("swing", v_play);
    r.verb("forces", v_forces);
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
        let p = Pendulum {
            theta0: 0.05,
            ..Default::default()
        };
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
        for sub in [
            "p.pivot", "p.rod", "p.bob", "p.path", "p.vel", "p.ke", "p.pe",
        ] {
            assert!(base.contains(sub), "missing entity `{sub}`");
        }
        // `swing(p)` must resolve against those — proven by a clean parse+validate
        let m2 = crate::parse("canvas(\"16:9\");\npendulum(p);\nswing(p, 6);\n").unwrap();
        assert!(
            m2.validate().is_ok(),
            "pendulum+swing should validate: {:?}",
            m2.validate().err()
        );
    }

    /// The generic views (`phase`/`well`) lay out their panels + curve + marker
    /// off a sim's stored data, and the marker joins the `swing` playback.
    #[test]
    fn phase_and_well_views_build_and_animate() {
        let src = "canvas(\"16:9\");\npendulum(p, (300, 200), 1.5, 55);\n\
                   phase(p, (900, 200), 120);\nwell(p, (900, 480), 120);\nswing(p, 8);\n";
        let m = crate::parse(src).unwrap();
        let base = m.base();
        for sub in [
            "p.phase.curve",
            "p.phase.dot",
            "p.well.curve",
            "p.well.ball",
        ] {
            assert!(base.contains(sub), "missing view entity `{sub}`");
        }
        assert!(
            m.validate().is_ok(),
            "views + swing should validate: {:?}",
            m.validate().err()
        );
    }

    /// The time-graph views (`timegraph`/`energygraph`) build curves + a sweep
    /// line off the sim's series, and the sweep joins the `swing` playback.
    #[test]
    fn time_and_energy_graphs_build() {
        let src = "canvas(\"16:9\");\npendulum(p, (280, 210), 1.3, 55);\n\
                   timegraph(p, (900, 200), 100);\nenergygraph(p, (900, 470), 100);\nswing(p, 8);\n";
        let m = crate::parse(src).unwrap();
        let base = m.base();
        for sub in [
            "p.time.c0",
            "p.time.c1",
            "p.time.sweep",
            "p.energy.c0",
            "p.energy.c2",
            "p.energy.sweep",
        ] {
            assert!(base.contains(sub), "missing graph entity `{sub}`");
        }
        assert!(
            m.validate().is_ok(),
            "graphs + swing should validate: {:?}",
            m.validate().err()
        );
    }

    /// The spring reproduces the SHM period 2π√(m/k) from its own zero-crossings.
    #[test]
    fn spring_reproduces_shm_period() {
        let sp = Spring {
            k: 12.0,
            mass: 1.0,
            damping: 0.0,
            x0: 0.1,
        };
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
        let sp = Spring {
            k: 10.0,
            mass: 1.0,
            damping: 0.0,
            x0: 1.0,
        };
        let traj = ode::integrate(&sp.state0(), 0.0005, 8_000, |s, d| sp.deriv(s, d));
        let e0 = sp.energy(&traj[0]).total();
        for s in &traj {
            assert!(
                (sp.energy(s).total() - e0).abs() / e0 < 0.01,
                "energy drifted"
            );
        }
        let src = "canvas(\"16:9\");\nspring(sp, (360,300), 10, 1.2);\n\
                   phase(sp,(900,200),110);\nwell(sp,(900,470),110);\nrun(sp, 8);\n";
        let m = crate::parse(src).unwrap();
        let base = m.base();
        for sub in [
            "sp.wall",
            "sp.spring",
            "sp.mass",
            "sp.phase.curve",
            "sp.well.curve",
        ] {
            assert!(base.contains(sub), "missing `{sub}`");
        }
        assert!(
            m.validate().is_ok(),
            "spring + views + run should validate: {:?}",
            m.validate().err()
        );
    }

    /// The undamped double pendulum conserves total energy (a good check on the
    /// coupled equations of motion), and its ctor builds the two arms + inherits
    /// phase/energy views — but `well` is refused (4-D system).
    #[test]
    fn double_pendulum_conserves_energy_and_has_no_well() {
        let dp = DoublePendulum {
            g: 9.8,
            l1: 1.0,
            l2: 1.0,
            m1: 2.0,
            m2: 2.0,
            a1: 1.2,
            a2: 0.7,
        };
        let traj = ode::integrate(&dp.state0(), 0.002, 6_000, |s, d| dp.deriv(s, d));
        let e0 = dp.energy(&traj[0]).total();
        for s in &traj {
            assert!(
                (dp.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02,
                "energy drifted"
            );
        }
        assert!(
            dp.well_curve().is_empty(),
            "double pendulum should have no well curve"
        );

        let ok = "canvas(\"16:9\");\ndoublependulum(dp, (400,240), 120, 100);\n\
                  phase(dp,(940,200),110);\nenergygraph(dp,(940,470),110);\nrun(dp, 8);\n";
        let m = crate::parse(ok).unwrap();
        let base = m.base();
        for sub in [
            "dp.rod1",
            "dp.bob1",
            "dp.rod2",
            "dp.bob2",
            "dp.path",
            "dp.phase.curve",
        ] {
            assert!(base.contains(sub), "missing `{sub}`");
        }
        assert!(
            m.validate().is_ok(),
            "double pendulum + views should validate"
        );
        // `well` on a 4-D sim is refused at parse time
        assert!(
            crate::parse("canvas(\"16:9\");\ndoublependulum(d);\nwell(d,(0,0),100);\n").is_err()
        );
    }

    /// Elastic pendulum: undamped conserves energy; ctor builds its coil + views.
    #[test]
    fn spring_pendulum_conserves_energy() {
        let sp = SpringPendulum {
            g: 9.81,
            k: 40.0,
            l0: 1.5,
            m: 1.0,
            damping: 0.0,
            a0: 0.5,
            stretch0: 0.4,
        };
        let traj = ode::integrate(&sp.state0(), 0.001, 8_000, |s, d| sp.deriv(s, d));
        let e0 = sp.energy(&traj[0]).total();
        for s in &traj {
            assert!(
                (sp.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02,
                "energy drifted"
            );
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
        let cp = CartPendulum {
            g: 9.8,
            l: 1.0,
            mcart: 1.0,
            mbob: 1.0,
            k: 6.0,
            cart_damp: 0.0,
            bob_damp: 0.0,
            a0: 0.7,
        };
        let traj = ode::integrate(&cp.state0(), 0.001, 8_000, |s, d| cp.deriv(s, d));
        let e0 = cp.energy(&traj[0]).total();
        for s in &traj {
            assert!(
                (cp.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02,
                "energy drifted"
            );
        }
        let m = crate::parse("canvas(\"16:9\");\ncartpendulum(cp,(500,340),45);\nrun(cp,8);\n")
            .unwrap();
        for sub in ["cp.cart", "cp.spring", "cp.rod", "cp.bob"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Compare-pendulum: two starts 0.001 rad apart diverge (sensitive dependence).
    #[test]
    fn compare_pendulum_diverges() {
        let cmp = ComparePendulum {
            g: 9.81,
            l: 1.0,
            m: 1.0,
            damping: 0.5,
            drive_amp: 10.0,
            drive_freq: 0.667,
            a0: 0.1,
            delta: 0.001,
        };
        let traj = ode::integrate(&cmp.state0(), 0.002, 8_000, |s, d| cmp.deriv(s, d));
        let d0 = (traj[0][2] - traj[0][0]).abs();
        let dn = (traj.last().unwrap()[2] - traj.last().unwrap()[0]).abs();
        assert!(d0 < 0.01, "should start close: {d0}");
        // exponential separation (positive Lyapunov) — the signature of chaos,
        // even when the absolute gap is still modest over the window
        assert!(
            dn > 20.0 * d0,
            "should diverge exponentially: start {d0}, end {dn}"
        );
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
        let vs = VerticalSpring {
            g: 9.81,
            k: 20.0,
            l0: 1.0,
            m: 1.0,
            damping: 0.0,
            stretch0: 0.6,
        };
        let t1 = ode::integrate(&vs.state0(), 0.001, 6_000, |s, d| vs.deriv(s, d));
        let e0 = vs.energy(&t1[0]).total();
        for s in &t1 {
            assert!((vs.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02);
        }
        for (src, subs) in [
            (
                "verticalspring(vs);well(vs,(900,300),100);run(vs,8);",
                vec!["vs.spring", "vs.mass", "vs.well.curve"],
            ),
            (
                "springincline(si,(400,190),30);run(si,8);",
                vec!["si.ramp", "si.spring", "si.bob"],
            ),
            (
                "bungee(bg);run(bg,8);",
                vec!["bg.platform", "bg.cord", "bg.jumper"],
            ),
            (
                "resonance(rs);run(rs,8);",
                vec!["rs.wall", "rs.spring", "rs.mass"],
            ),
        ] {
            let m = crate::parse(&format!("canvas(\"16:9\");\n{src}")).unwrap();
            for sub in subs {
                assert!(m.base().contains(sub), "missing `{sub}`");
            }
            assert!(m.validate().is_ok(), "{src} should validate");
        }
    }

    /// Multi-body springs: double-spring conserves energy; series/parallel and the
    /// car all build their parts.
    #[test]
    fn multi_body_spring_sims() {
        let ds = DoubleSpring {
            m1: 1.0,
            m2: 1.0,
            k: 20.0,
            r: 1.8,
            w1: 0.0,
            w2: 6.0,
            damping: 0.0,
            x1_0: 2.4,
            x2_0: 4.0,
        };
        let t = ode::integrate(&ds.state0(), 0.001, 6_000, |s, d| ds.deriv(s, d));
        let e0 = ds.energy(&t[0]).total();
        for s in &t {
            assert!(
                (ds.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02,
                "double-spring energy drift"
            );
        }
        for (src, subs) in [
            (
                "doublespring(dd);run(dd,8);",
                vec!["dd.block1", "dd.block2", "dd.spring2"],
            ),
            (
                "seriesparallel(sp);run(sp,8);",
                vec!["sp.massS", "sp.massP", "sp.sCoil1"],
            ),
            (
                "carsuspension(car);run(car,8);",
                vec!["car.road", "car.wheel", "car.body", "car.suspension"],
            ),
        ] {
            let m = crate::parse(&format!("canvas(\"16:9\");\n{src}")).unwrap();
            for sub in subs {
                assert!(m.base().contains(sub), "missing `{sub}`");
            }
            assert!(m.validate().is_ok(), "{src} should validate");
        }
    }

    /// Piston: height stays within the slider-crank bounds [L−a, L+a]; the
    /// mechanism builds; and it correctly has no phase view (kinematic).
    #[test]
    fn piston_bounds_and_builds() {
        let p = Piston {
            a: 50.0,
            l: 150.0,
            rpm: 60.0,
        };
        for i in 0..200 {
            let h = p.height(i as f32 * 0.05);
            assert!(
                h >= p.l - p.a - 1.0 && h <= p.l + p.a + 1.0,
                "height {h} out of range"
            );
        }
        let m =
            crate::parse("canvas(\"16:9\");\npiston(eng,(640,470),60);\nrun(eng,8);\n").unwrap();
        for sub in ["eng.crank", "eng.arm", "eng.rod", "eng.piston"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
        // kinematic mechanism → no phase view
        assert!(crate::parse("canvas(\"16:9\");\npiston(e);\nphase(e,(0,0),100);\n").is_err());
    }

    /// Molecule: the undamped bond network conserves energy; atoms + bonds build.
    #[test]
    fn molecule_conserves_energy_and_builds() {
        let mol = Molecule {
            n: 3,
            k: 12.0,
            rest: 2.0,
            mass: 0.5,
            damping: 0.0,
        };
        let traj = ode::integrate(&mol.state0(), 0.001, 6_000, |s, d| mol.deriv(s, d));
        let e0 = mol.energy(&traj[0]).total();
        for s in &traj {
            assert!(
                (mol.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02,
                "molecule energy drift"
            );
        }
        let m = crate::parse("canvas(\"16:9\");\nmolecule(mo,(560,300),3);\nenergygraph(mo,(1000,300),110);\nrun(mo,8);\n").unwrap();
        for sub in ["mo.atom0", "mo.atom1", "mo.bond01", "mo.energy.c0"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Robot arm: mode 0 (fixed target) drives the end-effector onto the target
    /// and settles; mode 1 (circle) TRACKS a moving target, so the gripper keeps
    /// moving across the whole window (not a one-off snap). The ctor builds the
    /// links + joints.
    #[test]
    fn robotarm_reaches_and_tracks() {
        // mode 0 — reach a fixed point
        let fixed = RobotArm {
            l1: 1.0,
            l2: 0.5,
            gain: 5.0,
            mode: 0,
            tx: 1.0,
            ty: 0.8,
        };
        let traj = ode::integrate(&fixed.state0(), 0.002, 4000, |s, d| fixed.deriv(s, d));
        let end = fixed.body(traj.last().unwrap());
        let err = ((end.0 - 1.0).powi(2) + (end.1 - 0.8).powi(2)).sqrt();
        assert!(err < 0.05, "end-effector should reach target, err {err}");

        // mode 1 — the gripper tracks the moving circle, so it keeps moving in
        // the SECOND half of the run (this is what fixes "no animation")
        let arm = RobotArm {
            l1: 1.0,
            l2: 0.5,
            gain: 5.0,
            mode: 1,
            tx: 1.0,
            ty: 0.8,
        };
        let states = simulate(&arm, 0.004, 6, SAMPLES);
        let ee: Vec<(f32, f32)> = states.iter().map(|st| arm.body(st)).collect();
        let mid = ee.len() / 2;
        let late_travel: f32 = ee[mid..]
            .windows(2)
            .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
            .sum();
        assert!(
            late_travel > 0.5,
            "arm should keep tracking (late travel {late_travel})"
        );

        let m = crate::parse("canvas(\"16:9\");\nrobotarm(rb,(500,440),1);\nrun(rb,8);\n").unwrap();
        for sub in [
            "rb.base",
            "rb.link1",
            "rb.elbow",
            "rb.link2",
            "rb.ee",
            "rb.target",
        ] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Atwood machine: undamped, the KE gained equals the PE lost (energy is
    /// conserved); the scale variant reads the rope tension 2·m₁·m₂·g/(m₁+m₂).
    #[test]
    fn pulley_conserves_energy_and_reads_tension() {
        let p = Pulley {
            m1: 3.0,
            m2: 2.0,
            g: 9.81,
            damping: 0.0,
        };
        let traj = ode::integrate(&p.state0(), 0.001, 3000, |s, d| p.deriv(s, d));
        for s in &traj {
            assert!(
                p.energy(s).total().abs() < 0.5,
                "Atwood energy should stay ≈0 (KE cancels PE)"
            );
        }
        let equal = Pulley {
            m1: 10.0,
            m2: 10.0,
            g: 9.81,
            damping: 0.0,
        };
        assert!(
            (equal.tension() - 98.1).abs() < 0.1,
            "equal masses ⇒ scale reads m·g"
        );
        let m = crate::parse(
            "canvas(\"16:9\");\npulley(pl);\nenergygraph(pl,(1000,300),110);\nrun(pl,4);\n",
        )
        .unwrap();
        for sub in ["pl.wheel", "pl.mass1", "pl.mass2", "pl.energy.c0"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
        let m2 = crate::parse("canvas(\"16:9\");\npulleyscale(ps);\nrun(ps,4);\n").unwrap();
        for sub in ["ps.pulleyL", "ps.scale", "ps.reading", "ps.mass1"] {
            assert!(m2.base().contains(sub), "missing `{sub}`");
        }
        assert!(m2.validate().is_ok());
    }

    /// Block & tackle: N strands give a mechanical advantage of N (an effort of
    /// load/N balances the load), the dynamics reduce to the Atwood at N=1, the
    /// undamped system conserves energy, and the ctor builds its parts.
    #[test]
    fn blocktackle_mechanical_advantage() {
        // N strands ⇒ effort = load/N is the balance point (zero acceleration)
        for n in 1..=4 {
            let balanced = BlockTackle {
                load: 12.0,
                effort: 12.0 / n as f32,
                strands: n as f32,
                g: 9.81,
            };
            assert!(
                balanced.accel().abs() < 1e-4,
                "effort load/N should balance at N={n}"
            );
        }
        // extra effort lifts the load (positive rise); N=1 matches the Atwood formula
        let bt = BlockTackle {
            load: 8.0,
            effort: 3.0,
            strands: 3.0,
            g: 9.81,
        };
        assert!(bt.accel() > 0.0, "9 > 8 ⇒ load rises");
        let atwood = BlockTackle {
            load: 2.0,
            effort: 3.0,
            strands: 1.0,
            g: 9.81,
        };
        let expected = (3.0 - 2.0) * 9.81 / (2.0 + 3.0); // (m−M)g/(M+m)
        assert!(
            (atwood.accel() - expected).abs() < 1e-4,
            "N=1 must equal the Atwood"
        );
        // undamped ⇒ energy conserved (KE cancels PE, like the Atwood)
        let traj = ode::integrate(&bt.state0(), 0.001, 2500, |s, d| bt.deriv(s, d));
        for s in &traj {
            assert!(
                bt.energy(s).total().abs() < 0.5,
                "block-tackle energy should stay ≈0"
            );
        }
        let m = crate::parse("canvas(\"16:9\");\nblocktackle(bt,(cx,140),8,3,3);\nenergygraph(bt,(1010,320),120);\nrun(bt,5);\n").unwrap();
        for sub in [
            "bt.fixed",
            "bt.movable",
            "bt.load",
            "bt.effort",
            "bt.strand0",
            "bt.strand2",
            "bt.energy.c0",
        ] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Compound pulley (fixed + movable, masses A/B/C): the string constraints
    /// hold (a_A = −a_P, a_B + a_C = 2·a_P), it's static exactly when mA = mB+mC,
    /// energy is conserved, and the ctor builds all the parts.
    #[test]
    fn compoundpulley_constraints_and_balance() {
        // static balance when mA = mB + mC
        let bal = CompoundPulley {
            ma: 4.0,
            mb: 2.0,
            mc: 2.0,
            g: 9.81,
        };
        let (aa, ab, ac) = bal.accels();
        assert!(
            aa.abs() < 1e-4 && ab.abs() < 1e-4 && ac.abs() < 1e-4,
            "mA=mB+mC ⇒ static, got {aa},{ab},{ac}"
        );
        // heavier A ⇒ A descends, B & C hauled up; the constraint a_B+a_C = 2·a_P = −2·a_A holds
        let cp = CompoundPulley {
            ma: 5.0,
            mb: 2.0,
            mc: 2.0,
            g: 9.81,
        };
        let (aa, ab, ac) = cp.accels();
        assert!(aa > 0.0 && ab < 0.0 && ac < 0.0, "A sinks, B/C rise");
        assert!(
            (ab + ac - (-2.0 * aa)).abs() < 1e-3,
            "a_B + a_C must equal 2·a_P = −2·a_A"
        );
        // undamped, conservative ⇒ total mechanical energy stays put (starts at 0)
        let traj = ode::integrate(&cp.state0(), 0.001, 2000, |s, d| cp.deriv(s, d));
        for s in &traj {
            assert!(
                cp.energy(s).total().abs() < 0.5,
                "compound-pulley energy should stay ≈0"
            );
        }
        let m = crate::parse("canvas(\"16:9\");\ncompoundpulley(cp,(520,120),5,2,2);\nenergygraph(cp,(1010,320),120);\nrun(cp,4);\n").unwrap();
        for sub in [
            "cp.top",
            "cp.mov",
            "cp.massA",
            "cp.massB",
            "cp.massC",
            "cp.ropeP",
            "cp.energy.c0",
        ] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Ramp: kinetic friction dissipates mechanical energy as the block slides
    /// down, so the total falls; the ctor builds the incline + block.
    #[test]
    fn ramp_dissipates_energy() {
        let r = Ramp {
            g: 9.81,
            angle: 30f32.to_radians(),
            mass: 5.0,
            mu_s: 0.5,
            mu_k: 0.3,
            applied: 0.0,
            s0: 4.0,
        };
        let traj = ode::integrate(&r.state0(), 0.001, 2000, |s, d| r.deriv(s, d));
        let e0 = r.energy(&traj[0]).total();
        let e_end = r.energy(traj.last().unwrap()).total();
        assert!(
            e_end < e0 - 5.0,
            "friction should bleed energy: {e0} → {e_end}"
        );
        let m = crate::parse("canvas(\"16:9\");\nramp(rp,(360,480),30);\nenergygraph(rp,(1000,300),110);\nrun(rp,6);\n").unwrap();
        for sub in ["rp.incline", "rp.surface", "rp.block", "rp.energy.c2"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Ramp force diagram: the force vectors are physically right (gravity down,
    /// normal = mg·cosθ ⟂ to the slope, friction opposes the slide), the ctor lays
    /// out the `{id}.forces` group, `forces(id)` reveals it, and a sim without one
    /// refuses it.
    #[test]
    fn ramp_force_diagram() {
        let r = Ramp {
            g: 9.81,
            angle: 30f32.to_radians(),
            mass: 5.0,
            mu_s: 0.5,
            mu_k: 0.3,
            applied: 0.0,
            s0: 4.0,
        };
        let fv = r.force_vectors(-1.0); // sliding down the slope (v < 0)
        assert!(
            fv[0].1.abs() < 1e-4 && (fv[0].2 + 5.0 * 9.81).abs() < 1e-2,
            "gravity is mg straight down"
        );
        let nmag = (fv[1].1.powi(2) + fv[1].2.powi(2)).sqrt();
        assert!(
            (nmag - 5.0 * 9.81 * 30f32.to_radians().cos()).abs() < 1e-2,
            "normal = mg·cosθ"
        );
        assert!(
            fv[2].1 > 0.0,
            "friction points up-slope (+x) when the block slides down"
        );
        let m =
            crate::parse("canvas(\"16:9\");\nramp(rp,(360,470),30);\nforces(rp);\nrun(rp,4);\n")
                .unwrap();
        for sub in ["rp.fmg", "rp.fN", "rp.ff", "rp.fa", "rp.fNL"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
        // a sim with no force diagram refuses forces()
        assert!(crate::parse("canvas(\"16:9\");\npendulum(p);\nforces(p);\n").is_err());
    }

    /// Incline variants: the incline-Atwood climbs when m₂ outpulls m₁·sinθ (and
    /// balances at m₂ = m₁·sinθ); the wedge slides toward the heavier/steeper side;
    /// the spring bumper conserves energy and stays bounded (it bounces); all build.
    #[test]
    fn incline_variants() {
        let th = 30f32.to_radians();
        let ip = InclinePulley {
            g: 9.81,
            angle: th,
            m1: 3.0,
            m2: 2.0,
            mu_k: 0.0,
            mu_s: 0.0,
        };
        assert!(ip.accel(0.0) > 0.0, "m₂ outpulls m₁·sinθ ⇒ block climbs");
        let bal = InclinePulley {
            g: 9.81,
            angle: th,
            m1: 3.0,
            m2: 3.0 * th.sin(),
            mu_k: 0.0,
            mu_s: 0.0,
        };
        assert!(bal.accel(0.0).abs() < 1e-4, "m₂ = m₁·sinθ balances");
        let traj = ode::integrate(&ip.state0(), 0.001, 2000, |s, d| ip.deriv(s, d));
        for s in &traj {
            assert!(
                ip.energy(s).total().abs() < 0.5,
                "incline-Atwood energy ≈0 (frictionless)"
            );
        }

        let di = DoubleIncline {
            g: 9.81,
            a1: 50f32.to_radians(),
            a2: 30f32.to_radians(),
            m1: 12.0,
            m2: 70.0,
            mu_k: 0.25,
            mu_s: 0.3,
        };
        assert!(di.accel(0.0) > 0.0, "70 kg on 30° beats 12 kg on 50°");

        let ib = InclineBumper {
            g: 9.81,
            angle: 40f32.to_radians(),
            m: 2.0,
            k: 500.0,
            mu_k: 0.0,
            s_contact: 1.25,
            s0: 4.0,
        };
        let t = ode::integrate(&ib.state0(), 0.0005, 4000, |s, d| ib.deriv(s, d));
        let e0 = ib.energy(&t[0]).total();
        for s in &t {
            assert!(
                (ib.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02,
                "bumper energy drift"
            );
            assert!(
                s[0] > -0.1 && s[0] < 5.0,
                "bumper stays bounded (it bounces)"
            );
        }

        let m = crate::parse(
            "canvas(\"16:9\");\ninclinepulley(ip,(280,500),30,3,2);\ndoubleincline(dw,(640,500),50,30,12,70);\ninclinebumper(ib,(300,500),40,2,500);\nrun(ip,4);\n",
        )
        .unwrap();
        for sub in [
            "ip.pulley",
            "ip.block",
            "ip.mass2",
            "dw.wedge",
            "dw.mass1",
            "dw.mass2",
            "ib.spring",
            "ib.block",
        ] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Spring chain conserves energy and holds its centre of mass (total momentum
    /// stays 0); the loop-the-loop builds its ramp+circle track. Both ctors build.
    #[test]
    fn spring_chain_and_loop() {
        let sc = SpringChain {
            m: 1.0,
            k: 18.0,
            rest: 1.4,
        };
        let traj = ode::integrate(&sc.state0(), 0.001, 4000, |s, d| sc.deriv(s, d));
        let e0 = sc.energy(&traj[0]).total();
        for s in &traj {
            assert!(
                (sc.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.02,
                "spring-chain energy drift"
            );
            assert!(
                (s[1] + s[3] + s[5]).abs() < 1e-2,
                "total momentum stays ~0 (CM held)"
            );
        }
        let m = crate::parse(
            "canvas(\"16:9\");\nspringchain(sc,(640,340),25);\nlooptrack(lt,(520,560),1,3);\nenergygraph(lt,(1000,300),110);\nrun(sc,8);\n",
        )
        .unwrap();
        for sub in [
            "sc.block1",
            "sc.block2",
            "sc.block3",
            "sc.spring1",
            "sc.spring2",
            "lt.ramp",
            "lt.loop",
            "lt.ball",
            "lt.energy.c0",
        ] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// String wave: the undamped discretised wave equation conserves energy, the
    /// pulse redistributes off its starting shape (it travels), and the ctor builds
    /// its rainbow chain of segments.
    #[test]
    fn string_wave_propagates() {
        let sw = StringWave {
            n: 24,
            k: 220.0,
            m: 0.3,
            damping: 0.0,
            pluck: 0.3,
        };
        let traj = ode::integrate(&sw.state0(), 0.001, 3000, |s, d| sw.deriv(s, d));
        let e0 = sw.energy(&traj[0]).total();
        for s in &traj {
            assert!(
                (sw.energy(s).total() - e0).abs() / e0.abs().max(1e-3) < 0.03,
                "string energy drift"
            );
        }
        // the shape changes: the far-right mass's displacement swings away from its
        // small initial value as the pulse reaches it (a travelling wave)
        let far = (0.85 * 25.0) as usize; // a mass near the right end
        let y_start = traj[0][(far - 1) * 2];
        let moved = traj
            .iter()
            .any(|s| (s[(far - 1) * 2] - y_start).abs() > 0.15);
        assert!(moved, "the pulse should reach and move the far end");
        let m = crate::parse("canvas(\"16:9\");\nstringwave(sw,(640,360));\nenergygraph(sw,(1050,180),90);\nrun(sw,8);\n").unwrap();
        for sub in ["sw.seg0", "sw.seg18", "sw.postL", "sw.energy.c0"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// The 1-D collision resolver: elastic swaps equal masses & conserves KE,
    /// inelastic merges to the common velocity, momentum is always conserved; and
    /// Newton's cradle (built on it) conserves its energy and lays out its balls.
    #[test]
    fn collisions_resolver_and_cradle() {
        // equal-mass elastic ⇒ velocity swap
        assert_eq!(collide_1d(1.0, 5.0, 1.0, 0.0, 1.0), (0.0, 5.0));
        // perfectly inelastic ⇒ common velocity (momentum / total mass)
        let (a, b) = collide_1d(1.0, 5.0, 1.0, 0.0, 0.0);
        assert!((a - 2.5).abs() < 1e-5 && (b - 2.5).abs() < 1e-5);
        // unequal elastic: momentum AND kinetic energy conserved
        let (v1, v2) = collide_1d(2.0, 3.0, 1.0, 0.0, 1.0);
        assert!(
            (2.0 * v1 + 1.0 * v2 - 6.0).abs() < 1e-4,
            "momentum conserved"
        );
        assert!(
            (v1 * v1 + 0.5 * v2 * v2 - 9.0).abs() < 1e-3,
            "elastic ⇒ KE conserved"
        );

        let m = crate::parse("canvas(\"16:9\");\nnewtonscradle(nc,(640,150),5,1);\nenergygraph(nc,(1000,300),100);\nrun(nc,6);\n").unwrap();
        for sub in [
            "nc.bar",
            "nc.ball0",
            "nc.ball4",
            "nc.string0",
            "nc.energy.c0",
        ] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Verify the collision sims' physics: elastic blocks conserve total kinetic
    /// energy over the whole run (collisions + elastic walls); inelastic blocks
    /// lose it; and the embedded bullet leaves at exactly m_b·v_b/(m_b+M) with
    /// most of the kinetic energy gone. Both ctors build.
    #[test]
    fn collide_blocks_and_bullet_physics() {
        let (m1, m2, k) = (3.0f32, 1.0f32, 60.0f32);
        // total mechanical energy = both blocks' KE + block 1's spring PE
        let total = |f: &[f32; 4]| {
            0.5 * m1 * f[1] * f[1] + 0.5 * m2 * f[3] * f[3] + 0.5 * k * (f[0] - CB_X1_EQ).powi(2)
        };
        // elastic (e = 1): total mechanical energy conserved across the whole run
        let fr = sim_collideblocks(m1, m2, 1.0, 6.0, 0.68, 0.56, k);
        let e0 = total(&fr[0]);
        for f in &fr {
            assert!(
                (total(f) - e0).abs() < 0.03 * e0,
                "elastic ⇒ total mechanical energy conserved"
            );
        }
        assert!(
            fr.iter().any(|f| f[1].abs() > 0.5),
            "a collision actually happens (block 1 gets moving)"
        );
        // inelastic (e = 0.6): energy is lost — the run ends below where it started
        let fi = sim_collideblocks(m1, m2, 0.6, 6.0, 0.68, 0.56, k);
        assert!(
            total(fi.last().unwrap()) < 0.9 * e0,
            "inelastic ⇒ energy lost"
        );

        // bullet embeds: combined velocity = m_b·v_b/(m_b+M); most KE lost
        let (mb, vb, mbig) = (0.05f32, 40.0f32, 1.95f32);
        let fb = sim_bulletblock(mb, vb, mbig, 6.0, 0.67);
        let v_expected = mb * vb / (mb + mbig);
        let v_after = fb.iter().map(|f| f[3]).fold(0.0f32, f32::max);
        assert!(
            (v_after - v_expected).abs() < 0.05,
            "combined v = m_b·v_b/(m_b+M): {v_after} vs {v_expected}"
        );
        let (ke_before, ke_after) = (0.5 * mb * vb * vb, 0.5 * (mb + mbig) * v_after * v_after);
        assert!(
            ke_after < 0.1 * ke_before,
            "the inelastic embed loses most of the KE ({ke_after} vs {ke_before})"
        );

        let m = crate::parse("canvas(\"16:9\");\ncollideblocks(cb,(640,430),3,1);\nbulletblock(bb,(640,430));\nenergygraph(cb,(1000,250),90);\nrun(cb,6);\n").unwrap();
        for sub in [
            "cb.block1",
            "cb.block2",
            "cb.wallL",
            "cb.spring",
            "cb.mom",
            "cb.energy.c0",
            "bb.bullet",
            "bb.block",
        ] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Drop-mass: a perfectly inelastic collision loses kinetic energy
    /// (½·M·v_after² < ½·m₂·v_impact²); the ctor builds the block + drop + springs.
    #[test]
    fn dropmass_collision_loses_energy() {
        let (m1, m2, g, h) = (1.0f32, 0.5f32, 9.81f32, 1.2f32);
        let big_m = m1 + m2;
        let v_imp = (2.0 * g * h).sqrt();
        let v_after = m2 * v_imp / big_m;
        let ke_before = 0.5 * m2 * v_imp * v_imp;
        let ke_after = 0.5 * big_m * v_after * v_after;
        assert!(
            ke_after < ke_before,
            "inelastic collision must lose KE: {ke_before} → {ke_after}"
        );
        let m = crate::parse(
            "canvas(\"16:9\");\ndropmass(dm);\nenergygraph(dm,(1000,300),110);\nrun(dm,6);\n",
        )
        .unwrap();
        for sub in ["dm.spring", "dm.block", "dm.drop", "dm.eq1", "dm.energy.c0"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
        // a two-phase event system → no phase portrait
        assert!(crate::parse("canvas(\"16:9\");\ndropmass(d);\nphase(d,(0,0),100);\n").is_err());
    }

    /// Raft: momentum conservation keeps the centre of mass exactly fixed as the
    /// person walks; the ctor builds the raft + person + waterline.
    #[test]
    fn raft_keeps_center_of_mass_fixed() {
        let (mp, mr) = (70.0f32, 200.0f32);
        for &d in &[0.5f32, 1.5, -2.0, 2.7] {
            let raftx = -mp * d / (mp + mr);
            let cm = (mp * (raftx + d) + mr * raftx) / (mp + mr);
            assert!(cm.abs() < 1e-4, "CM should stay at 0, got {cm} at d={d}");
        }
        let m = crate::parse("canvas(\"16:9\");\nraft(rf);\nrun(rf,8);\n").unwrap();
        for sub in ["rf.water", "rf.cm", "rf.raft", "rf.body", "rf.head"] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Brachistochrone: the cycloid bead reaches B before the straight-line bead
    /// (the curve of fastest descent); the cycloid conserves energy; ctor builds.
    #[test]
    fn brachistochrone_cycloid_wins() {
        let (dd, hh, g) = (3.0f32, 2.0f32, 9.81f32);
        let finish = |curve: Curve| -> f32 {
            let bead = Bead {
                g,
                b: 0.0,
                d: dd,
                h: hh,
                curve,
            };
            let traj = ode::integrate(&bead.state0(), 0.001, 4000, |s, d| bead.deriv(s, d));
            for (i, st) in traj.iter().enumerate() {
                if st[0] >= dd - 0.02 {
                    return i as f32 * 0.001;
                }
            }
            f32::INFINITY
        };
        let t_cyc = finish(build_cycloid(dd, hh));
        let t_line = finish(Curve::Straight { m: hh / dd });
        assert!(
            t_cyc.is_finite() && t_line.is_finite(),
            "both beads should finish"
        );
        assert!(
            t_cyc < t_line,
            "cycloid should win: cyc {t_cyc} vs line {t_line}"
        );
        // the bead-on-wire EOM conserves energy — checked on the circular arc,
        // whose derivatives are closed-form (the cycloid's come from a lookup
        // table, so it carries interpolation noise unsuitable for a tight check)
        let bead = Bead {
            g,
            b: 0.0,
            d: dd,
            h: hh,
            curve: Curve::Circle {
                r: (dd * dd + hh * hh) / (2.0 * hh),
            },
        };
        let traj = ode::integrate(&bead.state0(), 0.0005, 1600, |s, d| bead.deriv(s, d));
        let e0 = bead.energy(&traj[0]).total();
        for st in traj.iter().take_while(|st| st[0] < dd - 0.05) {
            assert!(
                (bead.energy(st).total() - e0).abs() / e0.abs().max(1e-3) < 0.03,
                "bead energy drift"
            );
        }
        let m = crate::parse("canvas(\"16:9\");\nbrachistochrone(br,(360,130));\nrun(br,5);\n")
            .unwrap();
        for sub in [
            "br.straight",
            "br.cycloid",
            "br.bead_cycloid",
            "br.markA",
            "br.markB",
        ] {
            assert!(m.base().contains(sub), "missing `{sub}`");
        }
        assert!(m.validate().is_ok());
    }

    /// Damping bleeds energy: the swing loses mechanical energy over time.
    #[test]
    fn damping_bleeds_energy() {
        let p = Pendulum {
            damping: 0.5,
            ..Default::default()
        };
        let traj = ode::integrate(&p.state0(), 0.001, 5_000, |s, d| p.deriv(s, d));
        let e0 = p.energy(&traj[0]).total();
        let e_end = p.energy(traj.last().unwrap()).total();
        assert!(
            e_end < 0.5 * e0,
            "damped energy {e_end} should be well below {e0}"
        );
    }
}
