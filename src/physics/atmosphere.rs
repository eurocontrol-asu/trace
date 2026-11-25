use crate::model::WindProfilePoint;

pub const G0: f64 = 9.80665;
pub const R_AIR: f64 = 287.058;
pub const P0: f64 = 101325.0;
pub const RHO0: f64 = 1.225;
pub const NM_TO_M: f64 = 1852.0;
pub const FT_TO_M: f64 = 0.3048;
pub const KT_TO_MS: f64 = 0.514444;
pub const FPM_TO_MPS: f64 = 0.00508;

pub struct AtmosOutput {
    pub rho: f64,
    pub t_c: f64,
}

pub fn calc_atmos(h_m: f64, t0_c: f64, lapse_kpm: f64) -> AtmosOutput {
    let t0_k = t0_c + 273.15;
    let t_amb_k = (t0_k - lapse_kpm * h_m).max(200.0);
    let exponent = G0 / (R_AIR * lapse_kpm);
    let pressure_pa = P0 * (t_amb_k / t0_k).powf(exponent);
    let rho = pressure_pa / (R_AIR * t_amb_k);
    AtmosOutput {
        rho,
        t_c: t_amb_k - 273.15,
    }
}

pub fn calc_wind_ms(
    h_m: f64,
    wind_mode: &str,
    tailwind_knots: f64,
    wind_profile: &Option<Vec<WindProfilePoint>>,
) -> f64 {
    if wind_mode == "constant" || wind_profile.is_none() {
        return tailwind_knots * KT_TO_MS;
    }

    let profile = wind_profile.as_ref().unwrap();
    if profile.is_empty() {
        return tailwind_knots * KT_TO_MS;
    }

    if profile.len() == 1 {
        return profile[0].tailwind_knots * KT_TO_MS;
    }

    // Linear interpolation with rule=2 (clamping at edges)
    if h_m <= profile[0].alt_m {
        return profile[0].tailwind_knots * KT_TO_MS;
    }
    if h_m >= profile.last().unwrap().alt_m {
        return profile.last().unwrap().tailwind_knots * KT_TO_MS;
    }

    for i in 0..profile.len() - 1 {
        let p1 = &profile[i];
        let p2 = &profile[i + 1];
        if h_m >= p1.alt_m && h_m <= p2.alt_m {
            let frac = (h_m - p1.alt_m) / (p2.alt_m - p1.alt_m);
            let tw = p1.tailwind_knots + frac * (p2.tailwind_knots - p1.tailwind_knots);
            return tw * KT_TO_MS;
        }
    }

    tailwind_knots * KT_TO_MS
}
