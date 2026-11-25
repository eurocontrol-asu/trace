pub mod atmosphere;
pub mod mass;
pub mod transitions;

use std::f64::consts::PI;
use crate::model::{FlightProfileRow, MassMetrics, PhaseSummary, SimulationInput, SimulationResult};
use crate::physics::atmosphere::{
    calc_atmos, calc_wind_ms, FPM_TO_MPS, FT_TO_M, G0, KT_TO_MS, NM_TO_M, RHO0,
};
use crate::physics::mass::init_aircraft_mass;
use crate::physics::transitions::{check_phase_transition, PhaseLimits, StateSnapshot};

pub fn build_profile_dynamic(input: &SimulationInput) -> SimulationResult {
    let uses_h2 = input.electric_source == "FuelCell" || input.combustion_fuel == "Hydrogen";
    let uses_battery = input.electric_source == "Battery";
    let size_battery = input.battery_sizing_mode == "mission_sized" && uses_battery;
    let size_h2 = input.h2_sizing_mode == "mission_sized" && uses_h2;

    if size_battery || size_h2 {
        // Size every active mission-sized store together. A larger battery or tank
        // changes MTOW and therefore the demand seen by the other store.
        let seed = build_profile_once(input, None, None, size_battery, size_h2);
        let mut battery_capacity_kwh = if size_battery {
            Some(seed.total_energy_kwh
            * (1.0 + input.battery_reserve_fraction).max(0.0)
            * (1.0 + input.battery_sizing_margin_fraction).max(0.0))
        } else {
            None
        };
        let mut h2_capacity_kg = if size_h2 {
            Some(seed.total_h2_kg
            * (1.0 + input.h2_reserve_fraction).max(0.0)
            * (1.0 + input.h2_sizing_margin_fraction).max(0.0))
        } else {
            None
        };

        for _ in 0..3 {
            let candidate = build_profile_once(
                input,
                h2_capacity_kg,
                battery_capacity_kwh,
                false,
                false,
            );
            if candidate.distance_nm <= 0.0
                || !matches!(candidate.end_reason.as_str(), "energy" | "h2_exhausted")
            {
                return candidate;
            }
            let scale = (input.stage_nm / candidate.distance_nm).max(1.05);
            if candidate.end_reason == "energy" {
                if let Some(capacity) = battery_capacity_kwh.as_mut() {
                    *capacity *= scale;
                } else {
                    return candidate;
                }
            } else if let Some(capacity) = h2_capacity_kg.as_mut() {
                *capacity *= scale;
            } else {
                return candidate;
            }
        }
        return build_profile_once(input, h2_capacity_kg, battery_capacity_kwh, false, false);
    }
    build_profile_once(input, None, None, false, false)
}

fn build_profile_once(
    input_raw: &SimulationInput,
    h2_capacity_override_kg: Option<f64>,
    battery_capacity_override_kwh: Option<f64>,
    ignore_battery_limit: bool,
    ignore_h2_limit: bool,
) -> SimulationResult {
    let mut input = input_raw.clone();

    // Override with DB if known
    let db = crate::battery::get_battery_db();
    if let Some(tech) = db.get(input.battery_tech.as_str()) {
        input.dens_g_whkg = tech.specific_energy * 1000.0;
        input.soc_min = 1.0 - tech.dod;
        input.system_eff = tech.eta;
    }

    // Hydrogen tech overrides (tank, fuel cell stack, H2 supply pathway)
    let h2_tank_db = crate::hydrogen::get_h2_tank_db();
    if let Some(tank) = h2_tank_db.get(input.h2_tank_type.as_str()) {
        input.h2_boiloff_pct_per_day = tank.boiloff_pct_per_day;
    }
    let fc_db = crate::hydrogen::get_fuel_cell_db();
    if let Some(fc) = fc_db.get(input.fuel_cell_tech.as_str()) {
        input.eta_fuel_cell = fc.eta_stack;
        input.fuel_cell_bop_frac = fc.bop_parasitic_frac;
        input.fuel_cell_power_density_kw_kg = fc.power_density_kw_kg;
        input.fuel_cell_stack_life_h = fc.stack_life_h;
    }
    let uses_battery = input.electric_source == "Battery";
    let uses_fuel_cell = input.electric_source == "FuelCell";
    let uses_h2_combustion = input.combustion_fuel == "Hydrogen";
    let uses_h2 = uses_fuel_cell || uses_h2_combustion;
    let uses_buffer = uses_fuel_cell && input.enable_fc_battery_buffer;

    let mass_res: MassMetrics = init_aircraft_mass(
        &input,
        h2_capacity_override_kg,
        battery_capacity_override_kwh,
    );
    let mtow_kg = mass_res.mtow_kg;
    // `batt_kg` plays the role of the *electric-source* battery: the main propulsion pack
    // when electric_source == "Battery", or the small transient buffer pack alongside a
    // fuel cell (0 if no buffer / not using a fuel cell).
    let batt_kg = if uses_fuel_cell { mass_res.fc_buffer_kg } else { mass_res.batt_kg };
    let fc_stack_kg = mass_res.fc_stack_kg;
    let h2_fuel_kg = mass_res.h2_fuel_kg;

    let e_usable_kwh_nom = if uses_fuel_cell && !uses_buffer {
        0.0
    } else {
        batt_kg
            * input.dens_g_whkg
            * (input.soc_max - input.soc_min)
            * input.system_eff
            * input.aging
            / 1000.0
    };


    let fc_stack_kw_rated = mass_res.fc_stack_kw_rated;

    let (mut p_cont_kw_nom, mut p_peak_kw_nom) = if uses_fuel_cell {
        let buffer_peak_kw = if uses_buffer {
            batt_kg * input.dens_g_whkg * input.c_rate_peak / 1000.0
        } else {
            0.0
        };
        (fc_stack_kw_rated, fc_stack_kw_rated + buffer_peak_kw)
    } else {
        (
            batt_kg * input.dens_g_whkg * input.c_rate_cont / 1000.0,
            batt_kg * input.dens_g_whkg * input.c_rate_peak / 1000.0,
        )
    };
    if !uses_fuel_cell && input.electric_sizing_mode == "rated_power" && input.electric_rated_power_kw > 0.0 {
        p_cont_kw_nom = input.electric_rated_power_kw;
        p_peak_kw_nom = input.electric_rated_power_kw;
    }
    let thermal_rated_power_kw = if uses_h2_combustion {
        if input.thermal_rated_power_kw > 0.0 {
            input.thermal_rated_power_kw
        } else {
            // A pure H2 turbine has no traction battery from which to derive
            // installed power. Size its default rating from the aircraft's
            // installed power allowance, with takeoff margin.
            input.pt_per_seat * input.n_seats * 2.0
        }
    } else {
        0.0
    };

    // H2 fuel budget, net of a simple flat boil-off loss accrued over the pre-flight
    // ground dwell time (LH2 only; boiloff_pct_per_day is 0 for compressed-gas tanks).
    let boiloff_frac = if uses_h2 {
        ((input.h2_boiloff_pct_per_day / 100.0) * (input.h2_ground_dwell_h / 24.0)).clamp(0.0, 0.95)
    } else {
        0.0
    };
    let h2_usable_kg_nom = h2_fuel_kg * (1.0 - boiloff_frac);
    let h2_volume_per_kg_m3 = h2_tank_db
        .get(input.h2_tank_type.as_str())
        .map(|tank| 1.0 / tank.volumetric_density_kg_m3)
        .unwrap_or(0.0);

    let tau_pitch_s = 2.0 + (mtow_kg / 10000.0) * 1.5;

    let mut limits = PhaseLimits::from_input(&input);
    limits.stage_m_target *= 1.0 + input.routing_factor_pct / 100.0;

    let mut profile: Vec<FlightProfileRow> = Vec::with_capacity(
        ((input.max_sim_min * 60.0) / input.dt_s).ceil() as usize + 100,
    );

    // Initial state
    let mut current_phase = "taxi_out";
    let mut t_s: f64 = 0.0;
    let mut t_phase_s: f64 = 0.0;
    let mut s_m: f64 = 0.0;
    let mut h_m: f64 = 0.0;
    let mut v_ms: f64 = 0.5;
    let mut on_ground = true;
    let mut hold_done = false;
    let mut t_batt: f64 = input.t_batt0_c;
    let mut t_stack: f64 = input.t_stack0_c;
    let mut e_elec_kwh: f64 = 0.0;
    let mut p_shaft_kw_cmd: f64 = 0.0;
    let mut roc_fpm: f64 = 0.0;
    let mut speed_error_prev: f64 = 0.0;
    let mut gamma_rad: f64 = 0.0;

    // 1. Dynamic Wing Sizing (Compute effective wing area and span based on MTOW scaling)
    let baseline_mtow = 8500.0; // Approx baseline MTOW for 20 seats
    let scale_factor = (mtow_kg / baseline_mtow).max(0.1);

    let effective_s_m2 = input.s_m2 * scale_factor;
    let effective_b_m = input.b_m * scale_factor.sqrt(); // Preserve aspect ratio

    let ar = effective_b_m.powi(2) / effective_s_m2;
    let k_ind = 1.0 / (PI * input.e_oswald * ar);
    let mut end_reason = "max_time".to_string();
    let min_ground_speed_kn = (input.vias_cruise_kn * 0.5).max(90.0);
    let automatic_block_time_min = (45.0 + (input.stage_nm / min_ground_speed_kn) * 60.0)
        * input.max_block_time_factor.max(1.0);
    let max_block_time_min = if input.max_block_time_min > 0.0 {
        input.max_block_time_min
    } else {
        automatic_block_time_min
    };
    let max_sim_time_s = max_block_time_min * 60.0;
    let design_constraint = if input.max_mtow_kg > 0.0 && mtow_kg > input.max_mtow_kg {
        Some("mtow_limit")
    } else if mass_res.h2_capacity_shortfall_kg > 1e-6 {
        Some("h2_tank_volume_limit")
    } else {
        None
    };

    let mut total_fuel_burn_kg: f64 = 0.0;
    let mut total_h2_combustion_kg: f64 = 0.0;
    let mut total_h2_fc_kg: f64 = 0.0;

    while t_s <= max_sim_time_s {
        // 1. Atmospheric Conditions
        let atmos = calc_atmos(h_m, input.t0_c, input.lapse_kpm);
        let rho_h = atmos.rho;
        let sigma = rho_h / RHO0;

        // 2. Temperature Effects
        // For a fuel cell, power/energy derate tracks stack temperature against its
        // operating setpoint; otherwise it tracks the (main or buffer) battery vs. 25degC.
        let dt_rel = if uses_fuel_cell {
            t_stack - input.t_stack_op_c
        } else {
            t_batt - 25.0
        };
        let fp = (1.0 - input.kp_temp * dt_rel.abs()).max(0.0);
        let fe = (1.0 - input.ke_temp * dt_rel.abs()).max(0.0);

        // 3. Wind
        let w_ms = calc_wind_ms(h_m, &input.wind_mode, input.tailwind_knots, &input.wind_profile);
        let v_t = v_ms.max(0.1);

        // 4. Reference Speed (IAS -> TAS)
        let v_ref_ias_kn = match current_phase {
            "taxi_out" | "taxi_in" => input.vias_taxi_kn,
            "takeoff" => input.vias_to_kn,
            "climb" => input.vias_climb_kn,
            "cruise" => input.vias_cruise_kn,
            "descent" => input.vias_desc_kn,
            "hold" => input.hold_ias_kn,
            "approach" => input.vias_desc_kn,
            _ => input.vias_cruise_kn,
        };
        let v_ref_ias_ms = v_ref_ias_kn * KT_TO_MS;
        let mut v_cmd_tas_ms = v_ref_ias_ms / sigma.max(0.1).sqrt();

        // 5. Stall Speed & Liftoff
        let v_stall = if rho_h > 1e-6 {
            ((2.0 * mtow_kg * G0) / (rho_h * effective_s_m2 * input.cl_max)).sqrt()
        } else {
            999.0
        };
        let v_lof = input.k_liftoff * v_stall;

        if current_phase == "takeoff" {
            v_cmd_tas_ms = v_cmd_tas_ms.max(v_lof * 1.05);
        }

        // 6. Vertical Speed Target
        let mut roc_target_mps: f64 = 0.0;
        if current_phase == "climb" {
            let alt_diff_m = (limits.top_h_m - h_m).max(0.0);
            let capture_factor = 1.0_f64.min(0.05_f64.max(alt_diff_m / 600.0));
            let density_factor = 0.35 + 0.65 * sigma;
            roc_target_mps = (input.roc_target_fpm * capture_factor * density_factor) * FPM_TO_MPS;
        } else if current_phase == "descent" {
            let target_bot_m = 200.0;
            let alt_diff_m = (h_m - target_bot_m).max(0.0);
            let capture_factor = 1.0_f64.min(0.15_f64.max(alt_diff_m / 400.0));
            roc_target_mps = -(limits.rod_target_fpm * capture_factor) * FPM_TO_MPS;
        } else if current_phase == "approach" {
            roc_target_mps = -limits.rod_app_fpm * FPM_TO_MPS;
        } else if current_phase == "takeoff" && !on_ground {
            roc_target_mps = 1200.0 * FPM_TO_MPS;
        }

        let gamma_rad_target = (roc_target_mps / v_t.max(10.0)).clamp(-0.2, 0.2).asin();

        // Apply pitch inertia (first-order lag)
        gamma_rad = gamma_rad
            + (gamma_rad_target - gamma_rad) * (1.0 - (-input.dt_s / tau_pitch_s).exp());

        // 7. Aerodynamic Forces
        let q_dyn = 0.5 * rho_h * v_t.powi(2);
        let cl_trim = if q_dyn > 1.0 && !on_ground {
            (mtow_kg * G0 * gamma_rad.cos()) / (q_dyn * effective_s_m2)
        } else {
            0.4
        };

        if !on_ground && cl_trim > input.cl_max * 1.05 {
            end_reason = "stall".to_string();
            break;
        }

        let cl = cl_trim.clamp(0.05, input.cl_max);
        let cd_fc_cooling = if uses_fuel_cell {
            let p_fc_kw = fc_stack_kw_rated;
            let eta_fc = input.eta_fuel_cell.clamp(0.40, 0.70);
            (0.0050 * (p_fc_kw / (effective_s_m2 * 10.0)) * ((1.0 - eta_fc) / eta_fc)).clamp(0.0, 0.045)
        } else {
            0.0
        };
        let cd = input.cd0 + cd_fc_cooling + k_ind * cl.powi(2);
        let d_n = q_dyn * effective_s_m2 * cd;

        // 8. Speed Controller (PD)
        let speed_error = v_cmd_tas_ms - v_t;
        let error_deriv = (speed_error - speed_error_prev) / input.dt_s;
        let accel_cmd_ms2 = (input.kp_accel * speed_error + input.kd_accel * error_deriv).clamp(-2.0, 2.0);

        // 9. Kinematic Constraints
        let min_sin_gamma = -d_n / (mtow_kg * G0);
        let eta_prop = (input.eta_prop0 * (0.9 + 0.1 * sigma)).max(0.70);
        let source_power_kw = (if current_phase == "takeoff" {
            p_peak_kw_nom
        } else {
            p_cont_kw_nom
        }) * fp;
        let p_avail_shaft_kw = source_power_kw + thermal_rated_power_kw * fp;
        let mut t_req_n = d_n + mtow_kg * G0 * gamma_rad.sin() + mtow_kg * accel_cmd_ms2;

        if on_ground {
            let mu = if current_phase == "taxi_out" {
                input.mu_rr_taxi_out
            } else {
                input.mu_rr_taxi_in
            };
            t_req_n += mu * (mtow_kg * G0 - q_dyn * effective_s_m2 * cl).max(0.0);
            gamma_rad = 0.0;
        }

        // 11. Shaft Power Required and Available
        let p_shaft_req_raw_kw = (t_req_n * v_t.max(5.0)) / (eta_prop * 1000.0);
        let mut p_regen_kw = 0.0;
        let mut p_shaft_req_kw = p_shaft_req_raw_kw;

        if p_shaft_req_raw_kw < 0.0 {
            if input.enable_regen {
                p_regen_kw = (p_shaft_req_raw_kw.abs() * input.eta_regen).min(input.max_regen_kw);
            }
            p_shaft_req_kw = 0.0;
        }

        let t_req_n = t_req_n.max(0.0);
        let p_shaft_cmd_lim = p_shaft_req_kw.clamp(0.0, p_avail_shaft_kw);

        // 12. Throttle Lag
        let p_shaft_target_kw = if current_phase == "takeoff" {
            // Takeoff uses maximum available shaft power; the speed controller is
            // unsuitable here because it can under-command thrust before liftoff.
            p_avail_shaft_kw
        } else {
            p_shaft_cmd_lim
        };
        p_shaft_kw_cmd = p_shaft_target_kw
            + (p_shaft_kw_cmd - p_shaft_target_kw) * (-input.dt_s / input.tau_throttle_s).exp();
        let p_shaft_kw = p_shaft_kw_cmd;

        // 13. Actual Thrust & Net Acceleration
        let t_n = (p_shaft_kw * 1000.0 * eta_prop) / v_t.max(5.0);
        let mut f_net = t_n - d_n - mtow_kg * G0 * gamma_rad.sin();

        if on_ground {
            let mu = if current_phase == "taxi_out" {
                input.mu_rr_taxi_out
            } else {
                input.mu_rr_taxi_in
            };
            f_net -= mu * (mtow_kg * G0 - q_dyn * effective_s_m2 * cl).max(0.0);
        }

        let accel_ms2 = f_net / mtow_kg;

        // 14. Update Position & Speed
        let v_next = (v_t + accel_ms2 * input.dt_s).max(0.0);
        let mut h_next = (h_m + (v_t + v_next) / 2.0 * gamma_rad.sin() * input.dt_s).max(0.0);

        // 15. Liftoff Check
        if on_ground && v_next >= v_lof && current_phase == "takeoff" {
            on_ground = false;
        }
        if on_ground {
            h_next = 0.0;
        }

        let effective_wind = w_ms + input.custom_wind_ms;
        let s_next = s_m + (v_next + effective_wind) * input.dt_s;

        // 16. Calculate ROC BEFORE updating altitude state
        let roc_mps = (h_next - h_m) / input.dt_s;
        roc_fpm = roc_mps / FPM_TO_MPS;

        // 17. Power, Energy & Fuel Consumption
        let mut p_hotel = if on_ground {
            input.p_hotel_ground_kw.max(0.0)
        } else {
            input.p_hotel_flight_kw.max(0.0)
        };

        // Stack cooling is both a thermal sink and an electrical auxiliary load.
        if uses_fuel_cell {
            p_hotel += input.p_cool_stack_kw.max(0.0);
        }
        if batt_kg > 0.0 {
            if t_batt < input.t_low_c {
                p_hotel += input.p_heat_kw.max(0.0);
            } else if t_batt > input.t_high_c {
                p_hotel += input.p_cool_kw.max(0.0);
            }
        }

        let alpha = match current_phase {
            "taxi_out" | "taxi_in" => input.alpha_taxi,
            "takeoff" => input.alpha_takeoff,
            "climb" => input.alpha_climb,
            "cruise" => input.alpha_cruise,
            "descent" => input.alpha_descent,
            "hold" | "approach" => input.alpha_reserve,
            _ => 1.0,
        };

        let (p_elec_in_kw, p_thermal_in_kw) = if batt_kg <= 0.0 && !uses_fuel_cell {
            // Pure thermal engine: alternator powers hotel loads
            let p_th = (p_shaft_kw + p_hotel) / if uses_h2_combustion {
                input.eta_thermal_h2
            } else {
                input.eta_thermal
            };
            (0.0, p_th)
        } else {
            let p_el = (p_shaft_kw * alpha) / input.eta_elec_to_shaft;
            let p_th = (p_shaft_kw * (1.0 - alpha)) / if uses_h2_combustion {
                input.eta_thermal_h2
            } else {
                input.eta_thermal
            };
            (p_el, p_th)
        };

        let (idle_flow_active, lhv_active) = if uses_h2_combustion {
            let p_ref_kw = input.thermal_rated_power_kw.max(input.pt_per_seat * input.n_seats * 1.5);
            let scale = (p_ref_kw / 1500.0).clamp(0.12, 4.0);
            (input.idle_fuel_kg_h_h2 * scale, input.h2_lhv_kwh_kg)
        } else {
            let p_ref_kw = (input.pt_per_seat * input.n_seats * 1.5).max(300.0);
            let scale = (p_ref_kw / 1500.0).clamp(0.12, 4.0);
            (input.idle_fuel_kg_h * scale, 11.92) // 11.92 kWh/kg kerosene LHV
        };
        let fuel_flow_kg_h = if p_thermal_in_kw > 0.0 {
            idle_flow_active + p_thermal_in_kw / lhv_active
        } else {
            0.0
        };
        let fuel_step_kg = fuel_flow_kg_h * (input.dt_s / 3600.0);

        // 18. Energy Accounting — split the electric bus load across whichever source
        // is active (battery, or fuel cell stack + optional buffer), and convert any
        // fuel-cell draw into H2 mass consumption.
        let p_total_elec = if batt_kg <= 0.0 && !uses_fuel_cell {
            0.0
        } else {
            p_elec_in_kw + p_hotel - p_regen_kw
        };

        let mut p_stack_kw: f64 = 0.0;
        let mut p_buffer_kw: f64 = 0.0;
        let mut h2_fc_step_kg = 0.0;

        if uses_fuel_cell {
            let p_stack_avail_kw = fc_stack_kw_rated * fp;
            if uses_buffer {
                if p_total_elec >= 0.0 {
                    p_stack_kw = p_total_elec.min(p_stack_avail_kw);
                    p_buffer_kw = p_total_elec - p_stack_kw; // excess above stack rating -> buffer
                } else {
                    // Net regenerative load: only the buffer can absorb it, the stack can't run in reverse.
                    p_buffer_kw = p_total_elec;
                }
            } else {
                // No buffer: the stack must directly cover the whole bus (already clamped
                // upstream via p_avail_shaft_kw, so this is rarely power-limited here).
                p_stack_kw = p_total_elec.max(0.0);
            }

            if p_stack_kw > 0.0 {
                let h2_power_in_kw =
                    p_stack_kw / (input.eta_fuel_cell * (1.0 - input.fuel_cell_bop_frac).max(0.05));
                h2_fc_step_kg = (h2_power_in_kw / input.h2_lhv_kwh_kg) * (input.dt_s / 3600.0);
            }
        }

        let e_step_kwh = if uses_fuel_cell {
            (p_buffer_kw * input.dt_s) / 3600.0 // buffer's own net throughput (charge/discharge)
        } else {
            (p_total_elec * input.dt_s) / 3600.0 // Battery-Electric: unchanged, full draw on main pack
        };
        e_elec_kwh += e_step_kwh;

        if uses_h2_combustion {
            total_h2_combustion_kg += fuel_step_kg;
        } else {
            total_fuel_burn_kg += fuel_step_kg;
        }
        total_h2_fc_kg += h2_fc_step_kg;
        let h2_step_kg = if uses_h2_combustion { fuel_step_kg } else { 0.0 } + h2_fc_step_kg;

        // 19. Thermal Management (battery/buffer pack, and separately the fuel cell stack)
        let mut t_batt_next = t_batt;
        if batt_kg > 0.0 {
            let q_passive = input.ua_wk * (atmos.t_c - t_batt);
            let p_tms_w = if t_batt < input.t_low_c {
                input.p_heat_kw * 1000.0
            } else if t_batt > input.t_high_c {
                -input.p_cool_kw * 1000.0
            } else {
                0.0
            };
            let elec_loss_source_kw = if uses_fuel_cell { p_buffer_kw.abs() } else { p_elec_in_kw };
            let q_internal_w = (elec_loss_source_kw * 1000.0) * (1.0 - input.system_eff);
            t_batt_next = t_batt
                + ((q_passive + p_tms_w + q_internal_w) / (batt_kg * input.c_th_kj * 1000.0)) * input.dt_s;
        }

        let mut t_stack_next = t_stack;
        if uses_fuel_cell {
            let q_passive_stack = input.ua_stack_wk * (atmos.t_c - t_stack);
            let p_cool_active_w = if p_stack_kw > 0.0 { -input.p_cool_stack_kw * 1000.0 } else { 0.0 };
            let q_internal_stack_w = (p_stack_kw * 1000.0) * (1.0 - input.eta_fuel_cell);
            let stack_thermal_mass = (fc_stack_kg.max(0.1)) * input.c_th_stack_kj * 1000.0;
            t_stack_next = t_stack
                + ((q_passive_stack + p_cool_active_w + q_internal_stack_w) / stack_thermal_mass) * input.dt_s;
        }

        let cur_t_s = t_s + input.dt_s;
        let cur_e_cum = e_elec_kwh;
        let cur_h2_cum = total_h2_combustion_kg + total_h2_fc_kg;
        let cur_fuel_cum = total_fuel_burn_kg;
        let h2_step_m3 = h2_step_kg * h2_volume_per_kg_m3;

        // 19. Log Row
        profile.push(FlightProfileRow {
            t_s,
            t_min: t_s / 60.0,
            phase: current_phase.to_string(),
            alt_m: h_m,
            alt_ft: h_m / FT_TO_M,
            s_m,
            s_km: s_m / 1000.0,
            s_nm: s_m / NM_TO_M,
            v_tas: v_t,
            v_tas_kn: v_t / KT_TO_MS,
            v_cmd_tas: v_cmd_tas_ms,
            v_gs: v_t + w_ms,
            v_gs_kn: (v_t + w_ms) / KT_TO_MS,
            rho: rho_h,
            p_req_kw: p_shaft_req_kw,
            p_avail_shaft_kw,
            roc_fpm,
            t_batt_c: t_batt,
            fp,
            fe,
            e_step_kwh,
            e_elec_kwh_cum: e_elec_kwh,
            n_seats: input.n_seats,
            dens_g_whkg: input.dens_g_whkg,
            t_stack_c: t_stack,
            h2_step_kg,
            h2_kg_cum: cur_h2_cum,
            h2_step_m3,
            h2_m3_cum: cur_h2_cum * h2_volume_per_kg_m3,
            fuel_step_kg: if uses_h2_combustion { 0.0 } else { fuel_step_kg },
            fuel_kg_cum: cur_fuel_cum,
            accel_ms2,
            accel_g: accel_ms2 / G0,
            cl,
            cd,
            lift_to_drag: if cd > 0.0 { cl / cd } else { 0.0 },
            thrust_n: t_n,
            drag_n: d_n,
            q_dyn,
            roc_mps,
        });

        // 20. Update State Variables
        v_ms = v_next;
        s_m = s_next;
        h_m = h_next;
        t_batt = t_batt_next;
        t_stack = t_stack_next;
        e_elec_kwh = cur_e_cum;
        t_s = cur_t_s;
        t_phase_s += input.dt_s;
        speed_error_prev = speed_error;

        // 21. Check Termination Conditions
        if let Some(reason) = design_constraint {
            end_reason = reason.to_string();
            break;
        }

        if current_phase == "takeoff" && on_ground
            && ((input.max_takeoff_time_min > 0.0 && t_phase_s >= input.max_takeoff_time_min * 60.0)
                || (input.max_takeoff_distance_m > 0.0 && s_m >= input.max_takeoff_distance_m))
        {
            end_reason = "takeoff_performance".to_string();
            break;
        }

        let e_usable_inst = e_usable_kwh_nom * fe;
        if !ignore_battery_limit
            && (uses_battery || uses_buffer)
            && batt_kg > 0.0
            && e_elec_kwh > e_usable_inst
        {
            end_reason = "energy".to_string();
            break;
        }

        if uses_h2 && !ignore_h2_limit && cur_h2_cum > h2_usable_kg_nom {
            end_reason = "h2_exhausted".to_string();
            break;
        }

        if current_phase == "taxi_in" && t_phase_s >= input.taxi_in_min * 60.0 {
            end_reason = "success".to_string();
            break;
        }

        // 22. Phase Transitions
        let snap = StateSnapshot {
            on_ground,
            hold_done,
            h_m,
            s_m,
            roc_fpm,
        };
        let next_phase = check_phase_transition(
            current_phase,
            t_phase_s,
            &snap,
            &limits,
            input.t0_c,
            input.lapse_kpm,
        );

        if next_phase != current_phase {
            let prev_phase = current_phase;
            current_phase = next_phase;
            t_phase_s = 0.0;

            if ["climb", "cruise", "descent"].contains(&next_phase) {
                speed_error_prev = 0.0;
            }

            if next_phase == "descent" && prev_phase == "hold" {
                hold_done = true;
            }

            if next_phase == "taxi_in" {
                h_m = 0.0;
                on_ground = true;
            }
        }
    }

    // Post-process summaries
    let max_climb_alt = profile
        .iter()
        .filter(|r| r.phase == "climb")
        .map(|r| r.alt_m)
        .fold(0.0_f64, f64::max);

    let fl_phys = ((max_climb_alt / FT_TO_M) / 100.0 * 10.0).round() / 10.0;
    let total_energy_kwh: f64 = profile.iter().map(|r| r.e_step_kwh).sum();
    let energy_buffer_kwh = e_usable_kwh_nom - total_energy_kwh;

    // Calculate phase summaries
    let mut phase_summaries: Vec<PhaseSummary> = Vec::new();
    let phases_order = [
        "taxi_out", "takeoff", "climb", "cruise", "descent", "hold", "approach", "taxi_in",
    ];

    for ph in phases_order {
        let phase_rows: Vec<&FlightProfileRow> = profile.iter().filter(|r| r.phase == ph).collect();
        if !phase_rows.is_empty() {
            let time_min = phase_rows.len() as f64 * input.dt_s / 60.0;
            let dist_nm = phase_rows.last().unwrap().s_nm - phase_rows.first().unwrap().s_nm;
            let energy_kwh: f64 = phase_rows.iter().map(|r| r.e_step_kwh).sum();
            let max_tas_kn = phase_rows
                .iter()
                .map(|r| r.v_tas_kn)
                .fold(0.0_f64, f64::max);
            let max_roc_fpm = phase_rows
                .iter()
                .map(|r| r.roc_fpm)
                .fold(f64::NEG_INFINITY, f64::max);
            let end_alt_ft = phase_rows.last().unwrap().alt_ft;

            phase_summaries.push(PhaseSummary {
                phase: ph.to_string(),
                time_min: (time_min * 10.0).round() / 10.0,
                dist_nm: (dist_nm * 10.0).round() / 10.0,
                energy_kwh: (energy_kwh * 10.0).round() / 10.0,
                max_tas_kn: max_tas_kn.round(),
                max_roc_fpm: max_roc_fpm.round(),
                end_alt_ft: end_alt_ft.round(),
            });
        }
    }

    let total_h2_kg = total_h2_combustion_kg + total_h2_fc_kg;
    let t_flight_min = profile.last().map(|r| r.t_min).unwrap_or(0.0);
    let block_time_min = profile.last().map(|r| r.t_min).unwrap_or(0.0);
    let distance_nm = profile.last().map(|r| r.s_nm).unwrap_or(0.0);

    let end_reason_detail = match end_reason.as_str() {
        "energy" => Some(format!("The usable battery energy ({:.1} kWh) was completely depleted.", e_usable_kwh_nom)),
        "h2_exhausted" => Some(format!("Hydrogen tank was fully depleted. Required H2 was higher than the usable capacity of {:.1} kg. Tank volume limit might be restricting growth.", h2_usable_kg_nom)),
        "mtow_limit" => Some("The aircraft MTOW exceeded the absolute limit configured in the aircraft properties.".to_string()),
        "h2_tank_volume_limit" => Some("The required hydrogen tank volume exceeds the available tank bay volume. Cannot fit the fuel onboard.".to_string()),
        "stall" => Some("The aircraft stalled because the required lift coefficient exceeded the maximum CL. This usually happens if the thrust available is insufficient to maintain speed, or empty mass is too high.".to_string()),
        "takeoff_performance" => Some("Failed to takeoff within the configured maximum time or distance limits.".to_string()),
        "max_time" => Some("The flight exceeded the maximum allowed block time.".to_string()),
        "success" => None,
        _ => None,
    };

    SimulationResult {
        profile,
        phase_summaries,
        end_reason_detail,
        fl_phys,
        e_usable_kwh: (e_usable_kwh_nom * 100.0).round() / 100.0,
        total_energy_kwh: (total_energy_kwh * 100.0).round() / 100.0,
        total_fuel_burn_kg: (total_fuel_burn_kg * 100.0).round() / 100.0,
        energy_buffer_kwh: (energy_buffer_kwh * 100.0).round() / 100.0,
        t_flight_min,
        block_time_min,
        distance_nm,
        end_reason,
        mass: mass_res,
        total_h2_combustion_kg: (total_h2_combustion_kg * 1000.0).round() / 1000.0,
        total_h2_fc_kg: (total_h2_fc_kg * 1000.0).round() / 1000.0,
        total_h2_kg: (total_h2_kg * 1000.0).round() / 1000.0,
        h2_usable_kg: (h2_usable_kg_nom * 1000.0).round() / 1000.0,
        final_phase: current_phase.to_string(),
    }
}
