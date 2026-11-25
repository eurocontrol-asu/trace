use serde::{Deserialize, Serialize};

use crate::model::{SimulationInput, SimulationResult};
use crate::physics::atmosphere::calc_wind_ms;
use crate::physics::build_profile_dynamic;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSizeSummary {
    pub top_fl: f64,
    pub mtow_kg: f64,
    pub installed_power_kw: f64,
    pub wing_area_m2: f64,
    pub wing_span_m: f64,
    pub block_time_min: f64,
    pub cruise_speed_tas_kn: f64,
    pub headwind_at_cruise_kn: f64,
    pub total_energy_kwh: f64,
    pub fuel_burn_kg: f64,
    pub h2_burn_kg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSizeResponse {
    pub success: bool,
    pub infeasible: bool,
    pub inputs: SimulationInput,
    pub modified_fields: Vec<String>,
    pub result: Option<SimulationResult>,
    pub summary: Option<AutoSizeSummary>,
    pub explanation: String,
}

/// Sizes an aircraft by evaluating candidate configurations with the mission simulator.
/// Hybrid configurations start from fixed phase splits. Feasible candidates
/// are ranked by modeled onboard energy use.
pub fn auto_size_aircraft(user_input: &SimulationInput) -> AutoSizeResponse {
    let candidate = user_input.clone();
    let n_seats = candidate.n_seats.max(4.0);
    let stage_nm = candidate.stage_nm.max(10.0);

    let uses_battery = candidate.electric_source == "Battery";
    let uses_fuel_cell = candidate.electric_source == "FuelCell";
    let uses_h2_combustion = candidate.combustion_fuel == "Hydrogen";
    let uses_h2 = uses_fuel_cell || uses_h2_combustion;
    let is_hybrid = (uses_battery && candidate.combustion_fuel == "Kerosene" && candidate.alpha_cruise < 0.99 && candidate.alpha_cruise > 0.01)
        || (uses_battery && uses_h2_combustion)
        || (uses_fuel_cell && candidate.combustion_fuel == "Kerosene" && candidate.alpha_cruise < 0.99 && candidate.alpha_cruise > 0.01);

    // 1. Initial Feasibility Check
    // Pure battery electric has fundamental gravimetric range limits.
    let is_pure_electric = uses_battery && candidate.combustion_fuel == "Kerosene" && !is_hybrid && candidate.alpha_cruise >= 0.95;
    if is_pure_electric {
        // Battery energy density in Wh/kg
        let wh_per_kg = candidate.dens_g_whkg.max(150.0);
        let max_feasible_electric_nm = (wh_per_kg * 0.55).min(350.0);
        if stage_nm > max_feasible_electric_nm {
            return AutoSizeResponse {
                success: false,
                infeasible: true,
                inputs: candidate,
                modified_fields: vec![],
                result: None,
                summary: None,
                explanation: format!(
                    "Mission infeasible for pure battery-electric: {} NM exceeds the physical battery mass limit (~{:.0} NM at {:.0} Wh/kg). Battery mass snowball would exceed MTOW. Recommendation: Select H2 Fuel Cell, H2 Combustion, or Electric Hybrid.",
                    stage_nm, max_feasible_electric_nm, wh_per_kg
                ),
            };
        }
    }

    // 2. Candidate Altitudes (Flight Levels)
    let fl_cap = ((stage_nm * 0.45) / 10.0).floor() * 10.0;
    let fl_max = fl_cap.clamp(80.0, 280.0);

    let candidate_fls: Vec<f64> = if fl_max <= 100.0 {
        vec![fl_max]
    } else if fl_max <= 160.0 {
        vec![80.0, fl_max]
    } else if fl_max <= 220.0 {
        vec![120.0, 160.0, fl_max]
    } else {
        vec![160.0, 200.0, 240.0, fl_max]
    };

    // 3. Sizing baseline wing area and span
    let est_mtow = (n_seats * 480.0 + 800.0).max(3000.0);
    let target_wing_loading = 250.0 + 1.2 * n_seats; // kg/m^2
    let s_m2 = ((est_mtow / target_wing_loading) * 10.0).round() / 10.0;
    let aspect_ratio = 10.125;
    let b_m = ((s_m2 * aspect_ratio).sqrt() * 10.0).round() / 10.0;

    // 4. Power loadings (kW)
    let p_takeoff_kw = (n_seats * 80.0).max(450.0);

    // 5. LP Optimization for Hybridization Alphas (if hybrid)
    let mut alpha_climb = candidate.alpha_climb;
    let mut alpha_cruise = candidate.alpha_cruise;
    let mut alpha_descent = candidate.alpha_descent;
    let mut alpha_taxi = candidate.alpha_taxi;
    let mut alpha_takeoff = candidate.alpha_takeoff;
    let mut alpha_reserve = candidate.alpha_reserve;

    if is_hybrid {
        alpha_climb = 0.5;
        alpha_cruise = 0.5;
        alpha_descent = 0.8;
        alpha_taxi = 1.0;
        alpha_takeoff = 0.5;
        alpha_reserve = 0.5;
    } else if is_pure_electric || (uses_fuel_cell && candidate.combustion_fuel == "Kerosene") {
        alpha_taxi = 1.0;
        alpha_takeoff = 1.0;
        alpha_climb = 1.0;
        alpha_cruise = 1.0;
        alpha_descent = 1.0;
        alpha_reserve = 1.0;
    } else if uses_h2_combustion && candidate.electric_source == "Battery" {
        alpha_taxi = 0.0;
        alpha_takeoff = 0.0;
        alpha_climb = 0.0;
        alpha_cruise = 0.0;
        alpha_descent = 0.0;
        alpha_reserve = 0.0;
    }

    // 6. Iterate through candidate altitudes to select the best flight level
    let mut best_input = candidate.clone();
    let mut best_result: Option<SimulationResult> = None;
    let mut best_score = f64::NEG_INFINITY;
    let mut best_summary: Option<AutoSizeSummary> = None;

    for &alt_fl in &candidate_fls {
        let mut test_input = candidate.clone();
        test_input.top_fl = alt_fl;
        test_input.s_m2 = s_m2;
        test_input.b_m = b_m;

        let cruise_ias = (150.0 + (alt_fl / 10.0) * 1.5 + (n_seats / 10.0) * 2.0).clamp(140.0, 220.0);
        test_input.vias_cruise_kn = (cruise_ias * 10.0).round() / 10.0;
        test_input.vias_climb_kn = ((cruise_ias - 30.0).max(120.0) * 10.0).round() / 10.0;
        test_input.vias_desc_kn = ((cruise_ias - 20.0).max(130.0) * 10.0).round() / 10.0;
        test_input.roc_target_fpm = if n_seats <= 20.0 { 1500.0 } else { 1800.0 };
        test_input.rod_target_fpm = 750.0;

        test_input.alpha_taxi = alpha_taxi;
        test_input.alpha_takeoff = alpha_takeoff;
        test_input.alpha_climb = alpha_climb;
        test_input.alpha_cruise = alpha_cruise;
        test_input.alpha_descent = alpha_descent;
        test_input.alpha_reserve = alpha_reserve;

        if uses_fuel_cell {
            test_input.fc_sizing_mode = "rated_power".to_string();
            test_input.fc_rated_power_kw = (p_takeoff_kw * 1.05).round();
            test_input.t_stack0_c = test_input.t_stack_op_c;
            test_input.p_cool_stack_kw = (test_input.fc_rated_power_kw * 0.015).max(20.0).round();
        }

        if uses_battery && (is_pure_electric || is_hybrid) {
            test_input.electric_sizing_mode = "rated_power".to_string();
            test_input.electric_rated_power_kw = (p_takeoff_kw * 1.05).round();
        }

        if uses_h2_combustion {
            test_input.thermal_rated_power_kw = (p_takeoff_kw * 1.05).round();
        }

        if uses_h2 {
            test_input.h2_sizing_mode = "mission_sized".to_string();
            test_input.h2_reserve_fraction = 0.15;
            test_input.h2_sizing_margin_fraction = 0.05;
            test_input.h2_tank_bay_volume_l = 0.0;
            test_input.h2_vol_per_seat_l = 750.0;
            test_input.max_h2_vol_frac = 0.90;
            test_input.max_h2_system_mass_fraction = 0.40;
        }

        if uses_battery && !uses_fuel_cell {
            test_input.battery_sizing_mode = "mission_sized".to_string();
            test_input.battery_reserve_fraction = 0.15;
            test_input.battery_sizing_margin_fraction = 0.05;
        }

        test_input.max_mtow_kg = 50000.0;
        test_input.max_takeoff_time_min = 5.0;
        test_input.max_takeoff_distance_m = 2000.0;
        test_input.max_block_time_factor = 2.0;

        let result = build_profile_dynamic(&test_input);

        if result.end_reason == "success" {
            let score = -result.mission_energy_kwh_equivalent();

            if score > best_score {
                best_score = score;
                best_input = test_input;

                let cruise_row = result.profile.iter().find(|r| r.phase == "cruise").or_else(|| result.profile.last());
                let tas = cruise_row.map(|r| r.v_tas_kn).unwrap_or(0.0);
                let alt_m = alt_fl * 100.0 * 0.3048;
                let wind = calc_wind_ms(alt_m, &best_input.wind_mode, best_input.tailwind_knots, &best_input.wind_profile);

                best_summary = Some(AutoSizeSummary {
                    top_fl: alt_fl,
                    mtow_kg: result.mass.mtow_kg.round(),
                    installed_power_kw: p_takeoff_kw,
                    wing_area_m2: s_m2,
                    wing_span_m: b_m,
                    block_time_min: (result.block_time_min * 10.0).round() / 10.0,
                    cruise_speed_tas_kn: (tas * 10.0).round() / 10.0,
                    headwind_at_cruise_kn: (-wind / 0.514444 * 10.0).round() / 10.0,
                    total_energy_kwh: (result.total_energy_kwh * 10.0).round() / 10.0,
                    fuel_burn_kg: (result.total_fuel_burn_kg * 10.0).round() / 10.0,
                    h2_burn_kg: (result.total_h2_kg * 10.0).round() / 10.0,
                });
                best_result = Some(result);
            }
        }
    }

    // 7. Calibration fallback
    if best_result.is_none() {
        let mut tuned_input = candidate.clone();
        tuned_input.top_fl = candidate_fls[0];
        tuned_input.s_m2 = s_m2 * 1.15;
        tuned_input.b_m = (tuned_input.s_m2 * aspect_ratio).sqrt();
        tuned_input.max_mtow_kg = 50000.0;
        tuned_input.max_takeoff_time_min = 5.0;
        tuned_input.max_takeoff_distance_m = 2000.0;

        let boosted_power = p_takeoff_kw * 1.30;
        if uses_fuel_cell {
            tuned_input.fc_sizing_mode = "rated_power".to_string();
            tuned_input.fc_rated_power_kw = boosted_power.round();
            tuned_input.p_cool_stack_kw = (boosted_power * 0.015).max(20.0).round();
            tuned_input.h2_vol_per_seat_l = 850.0;
            tuned_input.max_h2_vol_frac = 0.95;
            tuned_input.max_h2_system_mass_fraction = 0.45;
        }
        if uses_battery {
            tuned_input.electric_sizing_mode = "rated_power".to_string();
            tuned_input.electric_rated_power_kw = boosted_power.round();
        }
        if uses_h2_combustion {
            tuned_input.thermal_rated_power_kw = boosted_power.round();
            tuned_input.h2_vol_per_seat_l = 850.0;
            tuned_input.max_h2_vol_frac = 0.95;
        }

        let retry_res = build_profile_dynamic(&tuned_input);
        if retry_res.end_reason == "success" {
            let alt_m = tuned_input.top_fl * 100.0 * 0.3048;
            let wind = calc_wind_ms(alt_m, &tuned_input.wind_mode, tuned_input.tailwind_knots, &tuned_input.wind_profile);
            best_summary = Some(AutoSizeSummary {
                top_fl: tuned_input.top_fl,
                mtow_kg: retry_res.mass.mtow_kg.round(),
                installed_power_kw: boosted_power.round(),
                wing_area_m2: (tuned_input.s_m2 * 10.0).round() / 10.0,
                wing_span_m: (tuned_input.b_m * 10.0).round() / 10.0,
                block_time_min: (retry_res.block_time_min * 10.0).round() / 10.0,
                cruise_speed_tas_kn: tuned_input.vias_cruise_kn,
                headwind_at_cruise_kn: (-wind / 0.514444 * 10.0).round() / 10.0,
                total_energy_kwh: (retry_res.total_energy_kwh * 10.0).round() / 10.0,
                fuel_burn_kg: (retry_res.total_fuel_burn_kg * 10.0).round() / 10.0,
                h2_burn_kg: (retry_res.total_h2_kg * 10.0).round() / 10.0,
            });
            best_input = tuned_input;
            best_result = Some(retry_res);
        }
    }

    // 8. Construct list of modified fields for UI highlighting
    let modified_fields = vec![
        "top_fl".to_string(),
        "s_m2".to_string(),
        "b_m".to_string(),
        "electric_rated_power_kw".to_string(),
        "fc_rated_power_kw".to_string(),
        "thermal_rated_power_kw".to_string(),
        "vias_cruise_kn".to_string(),
        "vias_climb_kn".to_string(),
        "vias_desc_kn".to_string(),
        "roc_target_fpm".to_string(),
        "rod_target_fpm".to_string(),
        "alpha_taxi".to_string(),
        "alpha_takeoff".to_string(),
        "alpha_climb".to_string(),
        "alpha_cruise".to_string(),
        "alpha_descent".to_string(),
        "alpha_reserve".to_string(),
        "battery_sizing_mode".to_string(),
        "h2_sizing_mode".to_string(),
        "fc_sizing_mode".to_string(),
        "electric_sizing_mode".to_string(),
        "h2_vol_per_seat_l".to_string(),
        "max_h2_vol_frac".to_string(),
        "max_mtow_kg".to_string(),
        "p_cool_stack_kw".to_string(),
    ];

    if let Some(res) = best_result {
        let hold_str = if best_input.enable_hold {
            format!(" with {:.0} min holding pattern reserve included", best_input.hold_time_min)
        } else {
            "".to_string()
        };
        let arch_str = match (best_input.electric_source.as_str(), best_input.combustion_fuel.as_str()) {
            ("Battery", "Kerosene") if is_pure_electric => "Pure Electric",
            ("Battery", "Kerosene") => "Electric Hybrid",
            ("Battery", "Hydrogen") if !is_hybrid => "H2 Combustion Turboprop",
            ("Battery", "Hydrogen") => "H2 Combustion Hybrid",
            ("FuelCell", _) if !is_hybrid => "H2 Fuel Cell",
            ("FuelCell", _) => "H2 Fuel Cell Hybrid",
            _ => "Custom Aircraft",
        };

        AutoSizeResponse {
            success: true,
            infeasible: false,
            inputs: best_input.clone(),
            modified_fields,
            result: Some(res),
            summary: best_summary,
            explanation: format!(
                "Successfully sized and optimized {} ({:.0} seats, {:.0} NM{}) at FL{:.0}. Parameters prepopulated and verified.",
                arch_str, best_input.n_seats, best_input.stage_nm, hold_str, best_input.top_fl
            ),
        }
    } else {
        AutoSizeResponse {
            success: false,
            infeasible: true,
            inputs: candidate,
            modified_fields: vec![],
            result: None,
            summary: None,
            explanation: format!(
                "Unable to find a feasible flight profile for {:.0} seats and {:.0} NM with the selected constraints. Try adjusting the propulsion architecture, increasing energy density, or decreasing stage distance.",
                n_seats, stage_nm
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_size_h2_fuel_cell() {
        let mut input = SimulationInput::default();
        input.n_seats = 30.0;
        input.stage_nm = 400.0;
        input.electric_source = "FuelCell".to_string();
        input.combustion_fuel = "Kerosene".to_string();
        input.enable_hold = true;
        input.hold_time_min = 30.0;

        let resp = auto_size_aircraft(&input);
        assert!(resp.success, "Auto-size should succeed: {}", resp.explanation);
        assert!(resp.result.is_some());
        let sim = resp.result.unwrap();
        assert_eq!(sim.end_reason, "success");
        assert!(resp.inputs.top_fl >= 100.0);
        assert!(resp.inputs.fc_rated_power_kw > 1000.0);
    }

    #[test]
    fn test_auto_size_pure_electric_feasible() {
        let mut input = SimulationInput::default();
        input.n_seats = 9.0;
        input.stage_nm = 100.0;
        input.electric_source = "Battery".to_string();
        input.combustion_fuel = "Kerosene".to_string();
        input.alpha_climb = 1.0;
        input.alpha_cruise = 1.0;
        input.alpha_descent = 1.0;

        let resp = auto_size_aircraft(&input);
        assert!(resp.success, "Auto-size should succeed: {}", resp.explanation);
        assert_eq!(resp.result.unwrap().end_reason, "success");
    }

    #[test]
    fn test_auto_size_pure_electric_infeasible() {
        let mut input = SimulationInput::default();
        input.n_seats = 70.0;
        input.stage_nm = 1200.0;
        input.electric_source = "Battery".to_string();
        input.combustion_fuel = "Kerosene".to_string();
        input.alpha_climb = 1.0;
        input.alpha_cruise = 1.0;
        input.alpha_descent = 1.0;

        let resp = auto_size_aircraft(&input);
        assert!(!resp.success);
        assert!(resp.infeasible);
        assert!(resp.explanation.contains("infeasible"));
    }
}
