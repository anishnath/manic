//! Optics — dispersion: how a material's refractive index varies with colour.
//! This is what makes a prism split white light and a lens show chromatic
//! aberration — the effects are *earned* by real physics, not painted.
//!
//! Faithful port of the goldmine's material system
//! (`optical-designer-model.js`): the 3-term **Sellmeier equation** with the
//! real Schott/literature coefficients, wrapped in a tiny **named glass catalog**
//! so a non-programmer picks a glass by word (`"bk7"`, `"sf11"`, `"diamond"`, …)
//! and never touches a coefficient. Plus a wavelength→RGB map so each traced
//! ray is drawn in its true colour.

use macroquad::prelude::Color;

/// The 3-term Sellmeier equation: `n²(λ) = 1 + Σ Bᵢ·λ²/(λ² − Cᵢ)`, `λ` in
/// micrometres, `coeffs = [B1, C1, B2, C2, B3, C3]` (2, 4, or 6 entries).
pub fn sellmeier_n(coeffs: &[f32], lambda_um: f32) -> f32 {
    let l2 = lambda_um * lambda_um;
    let mut nsq = 1.0f32;
    let mut i = 0;
    while i + 1 < coeffs.len() {
        nsq += coeffs[i] * l2 / (l2 - coeffs[i + 1]);
        i += 2;
    }
    if nsq > 0.0 {
        nsq.sqrt()
    } else {
        1.0
    }
}

/// A named glass → its refractive index at wavelength `lambda_um` (micrometres).
/// The coefficients are the real Schott/literature values. Unknown (or empty)
/// names fall back to borosilicate crown (BK7).
pub fn glass_n(name: &str, lambda_um: f32) -> f32 {
    let c: &[f32] = match name.to_ascii_lowercase().as_str() {
        "sf11" | "flint" | "denseflint" => &[
            1.73759695,
            0.013188707,
            0.313747346,
            0.0623068142,
            1.89878101,
            155.23629,
        ],
        "f2" => &[
            1.34533359,
            0.00997743871,
            0.209073176,
            0.0470450767,
            0.937357162,
            111.886764,
        ],
        "silica" | "quartz" | "fusedsilica" => &[
            0.6961663, 0.0046791, 0.4079426, 0.01351206, 0.8974794, 97.934003,
        ],
        "sapphire" => &[
            1.4313493,
            0.0052799261,
            0.65054713,
            0.0142382647,
            5.3414021,
            325.01783,
        ],
        "diamond" => &[4.3356, 0.0106, 0.3306, 0.0],
        "water" => &[0.75831, 0.01007, 0.08495, 8.91377],
        // "bk7" | "crown" | "glass" | _ → borosilicate crown
        _ => &[
            1.03961212,
            0.00600069867,
            0.231792344,
            0.0200179144,
            1.01046945,
            103.560653,
        ],
    };
    sellmeier_n(c, lambda_um)
}

/// Approximate visible-spectrum colour of a wavelength in **nanometres**
/// (~380–780 nm) — Bruton's piecewise map, with an intensity roll-off at the
/// deep-red / deep-violet ends and a floor so the extremes still read on a dark
/// background.
pub fn wavelength_rgb(nm: f32) -> Color {
    let (r, g, b) = if nm < 440.0 {
        (-(nm - 440.0) / (440.0 - 380.0), 0.0, 1.0)
    } else if nm < 490.0 {
        (0.0, (nm - 440.0) / (490.0 - 440.0), 1.0)
    } else if nm < 510.0 {
        (0.0, 1.0, -(nm - 510.0) / (510.0 - 490.0))
    } else if nm < 580.0 {
        ((nm - 510.0) / (580.0 - 510.0), 1.0, 0.0)
    } else if nm < 645.0 {
        (1.0, -(nm - 645.0) / (645.0 - 580.0), 0.0)
    } else {
        (1.0, 0.0, 0.0)
    };
    let factor = if nm < 420.0 {
        0.35 + 0.65 * (nm - 380.0) / (420.0 - 380.0)
    } else if nm > 700.0 {
        0.35 + 0.65 * (780.0 - nm) / (780.0 - 700.0)
    } else {
        1.0
    };
    // lift toward white a touch so deep blue/violet stay visible on dark bg
    let lift = 0.12;
    Color::new(
        ((r * factor) * (1.0 - lift) + lift).clamp(0.0, 1.0),
        ((g * factor) * (1.0 - lift) + lift).clamp(0.0, 1.0),
        ((b * factor) * (1.0 - lift) + lift).clamp(0.0, 1.0),
        1.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Normal dispersion: glass bends blue MORE than red (n falls with λ), and
    /// dense flint (SF11) bends more than crown (BK7) at every colour.
    #[test]
    fn index_falls_with_wavelength() {
        let n_blue = glass_n("bk7", 0.450);
        let n_red = glass_n("bk7", 0.650);
        assert!(
            n_blue > n_red,
            "blue index {n_blue} should exceed red {n_red}"
        );
        // BK7 at the sodium d-line (~0.589 µm) is ≈1.5168
        assert!((glass_n("bk7", 0.5893) - 1.5168).abs() < 0.002);
        assert!(
            glass_n("sf11", 0.550) > glass_n("bk7", 0.550),
            "flint is denser than crown"
        );
    }
}
