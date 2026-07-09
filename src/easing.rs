//! Easing curves. To add one: new variant + match arm in [`Easing::apply`].

/// An easing curve. Applied to normalized time before interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Easing {
    Linear,
    InQuad,
    OutQuad,
    InOutQuad,
    InCubic,
    OutCubic,
    #[default]
    InOutCubic,
    /// Overshoots past the target, then settles.
    OutBack,
    /// Springy oscillation into place.
    OutElastic,
    OutBounce,
}

impl Easing {
    /// Map normalized time `u ∈ [0,1]` through this curve.
    pub fn apply(self, u: f32) -> f32 {
        let u = u.clamp(0.0, 1.0);
        match self {
            Easing::Linear => u,
            Easing::InQuad => u * u,
            Easing::OutQuad => u * (2.0 - u),
            Easing::InOutQuad => {
                if u < 0.5 {
                    2.0 * u * u
                } else {
                    -1.0 + (4.0 - 2.0 * u) * u
                }
            }
            Easing::InCubic => u * u * u,
            Easing::OutCubic => {
                let v = u - 1.0;
                v * v * v + 1.0
            }
            Easing::InOutCubic => {
                if u < 0.5 {
                    4.0 * u * u * u
                } else {
                    let v = 2.0 * u - 2.0;
                    0.5 * v * v * v + 1.0
                }
            }
            Easing::OutBack => {
                const C1: f32 = 1.70158;
                const C3: f32 = C1 + 1.0;
                let v = u - 1.0;
                1.0 + C3 * v * v * v + C1 * v * v
            }
            Easing::OutElastic => {
                const C4: f32 = std::f32::consts::TAU / 3.0;
                if u == 0.0 || u == 1.0 {
                    u
                } else {
                    (2.0f32).powf(-10.0 * u) * ((u * 10.0 - 0.75) * C4).sin() + 1.0
                }
            }
            Easing::OutBounce => {
                const N1: f32 = 7.5625;
                const D1: f32 = 2.75;
                let mut u = u;
                if u < 1.0 / D1 {
                    N1 * u * u
                } else if u < 2.0 / D1 {
                    u -= 1.5 / D1;
                    N1 * u * u + 0.75
                } else if u < 2.5 / D1 {
                    u -= 2.25 / D1;
                    N1 * u * u + 0.9375
                } else {
                    u -= 2.625 / D1;
                    N1 * u * u + 0.984375
                }
            }
        }
    }
}
