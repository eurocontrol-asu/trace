use crate::model::SimulationInput;
use crate::physics::atmosphere::{calc_atmos, FPM_TO_MPS, FT_TO_M, KT_TO_MS, NM_TO_M};

pub struct PhaseLimits {
    pub taxi_out_min: f64,
    pub _takeoff_min: f64,
    pub roc_threshold_fpm: f64,
    pub rod_target_fpm: f64,
    pub rod_app_fpm: f64,
    pub vias_desc_kn: f64,
    pub top_h_m: f64,
    pub stage_m_target: f64,
    pub cruise_hold_frac: f64,
    pub hold_enabled: bool,
    pub hold_alt_m: f64,
    pub hold_time_min: f64,
    pub flare_alt_m: f64,
}

impl PhaseLimits {
    pub fn from_input(input: &SimulationInput) -> Self {
        Self {
            taxi_out_min: input.taxi_out_min,
            _takeoff_min: input.takeoff_min,
            roc_threshold_fpm: input.roc_threshold_fpm,
            rod_target_fpm: input.rod_target_fpm,
            rod_app_fpm: input.rod_app_fpm,
            vias_desc_kn: input.vias_desc_kn,
            top_h_m: input.top_fl * 100.0 * FT_TO_M,
            stage_m_target: input.stage_nm * NM_TO_M,
            cruise_hold_frac: input.cruise_hold_frac,
            hold_enabled: input.enable_hold && input.hold_time_min > 0.0,
            hold_alt_m: input.hold_alt_ft * FT_TO_M,
            hold_time_min: input.hold_time_min,
            flare_alt_m: input.flare_alt_m,
        }
    }
}

pub struct StateSnapshot {
    pub on_ground: bool,
    pub hold_done: bool,
    pub h_m: f64,
    pub s_m: f64,
    pub roc_fpm: f64,
}

pub fn check_phase_transition(
    current_phase: &str,
    t_phase_s: f64,
    state: &StateSnapshot,
    limits: &PhaseLimits,
    t0_c: f64,
    lapse_kpm: f64,
) -> &'static str {
    match current_phase {
        "taxi_out" => {
            if t_phase_s >= limits.taxi_out_min * 60.0 {
                "takeoff"
            } else {
                "taxi_out"
            }
        }
        "takeoff" => {
            if !state.on_ground && state.h_m >= 15.0 {
                "climb"
            } else {
                "takeoff"
            }
        }
        "climb" | "cruise" => {
            let descent_time_s = state.h_m / (limits.rod_target_fpm * FPM_TO_MPS);

            // Estimate average descent TAS
            let rho_h = calc_atmos(state.h_m, t0_c, lapse_kpm).rho;
            let tas_top = (limits.vias_desc_kn * KT_TO_MS) * (1.225 / rho_h).sqrt();
            let tas_bot = limits.vias_desc_kn * KT_TO_MS;
            let avg_descent_speed_mps = (tas_top + tas_bot) / 2.0;

            let descent_dist_m = descent_time_s * avg_descent_speed_mps;

            // Add approach distance estimate
            let approach_time_s = 200.0 / (limits.rod_app_fpm * FPM_TO_MPS);
            let approach_dist_m = approach_time_s * (limits.vias_desc_kn * KT_TO_MS);

            let tod_m_max = limits.stage_m_target - descent_dist_m - approach_dist_m - 3000.0;
            let user_tod_m = limits.stage_m_target * limits.cruise_hold_frac;
            let actual_tod_m = user_tod_m.min(tod_m_max);

            if state.s_m >= actual_tod_m {
                "descent"
            } else if current_phase == "climb"
                && (state.h_m >= limits.top_h_m
                    || (state.roc_fpm < limits.roc_threshold_fpm && t_phase_s > 30.0))
            {
                "cruise"
            } else {
                if current_phase == "climb" {
                    "climb"
                } else {
                    "cruise"
                }
            }
        }
        "descent" => {
            if limits.hold_enabled && !state.hold_done && state.h_m <= limits.hold_alt_m {
                "hold"
            } else if state.h_m <= 200.0 {
                "approach"
            } else {
                "descent"
            }
        }
        "hold" => {
            if t_phase_s >= limits.hold_time_min * 60.0 {
                "descent"
            } else {
                "hold"
            }
        }
        "approach" => {
            if state.h_m <= limits.flare_alt_m {
                "taxi_in"
            } else {
                "approach"
            }
        }
        "taxi_in" => "taxi_in",
        _ => "taxi_in",
    }
}
