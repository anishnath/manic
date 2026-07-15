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
