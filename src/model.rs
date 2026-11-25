use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindProfilePoint {
    pub alt_m: f64,
    pub tailwind_knots: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SimulationInput {
    // ---- Aircraft Configuration ----
    pub n_seats: f64,                    // N (Number of seats, e.g. 20)
    pub payload_per_seat: f64,          // kg (default 100)
    pub empty_offset: f64,              // kg (default 600)
    pub empty_linear: f64,              // kg/seat (default 95)
    pub empty_power: f64,               // coeff (default 40)
    pub empty_exp: f64,                 // exponent (default 1.25)
    pub pt_per_seat: f64,               // kg/seat powertrain (default 25)
    pub cabin_vol_per_seat: f64,        // L/seat (default 550)
    pub max_batt_vol_frac: f64,         // fraction (default 0.50)

    // ---- Battery & Energy Storage ----
    pub batt_frac_6: f64,               // fraction (default 0.55)
    pub batt_frac_slope: f64,           // slope (default 0.0025)
    pub dens_g_whkg: f64,               // Wh/kg (default 350)
    pub dens_v_whl: f64,                // Wh/L (default 550)
    pub soc_min: f64,                   // min state of charge (default 0.10)
    pub soc_max: f64,                   // max state of charge (default 0.90)
    pub system_eff: f64,                // efficiency (default 0.92)
    pub aging: f64,                     // aging factor (default 0.90)
    pub c_rate_cont: f64,               // cont C-rate (default 1.5)
    pub c_rate_peak: f64,               // peak C-rate (default 3.0)
    pub battery_sizing_mode: String,    // "legacy_fraction", "manual_energy", or "mission_sized"
    pub battery_usable_energy_kwh: f64, // manual usable energy; 0 retains fraction sizing
    pub battery_reserve_fraction: f64,
    pub battery_sizing_margin_fraction: f64,
    pub electric_sizing_mode: String,   // "battery_c_rate" or "rated_power"
    pub electric_rated_power_kw: f64,   // installed electric shaft rating; 0 retains C-rate sizing

    // ---- Aerodynamics ----
    pub s_m2: f64,                      // wing area m^2 (default 32)
    pub b_m: f64,                       // wing span m (default 18)
    pub cd0: f64,                       // zero-lift drag coeff (default 0.025)
    pub e_oswald: f64,                  // Oswald efficiency (default 0.85)

    // ---- Propulsion/Energy Conversion ----
    pub eta_prop0: f64,                 // prop efficiency (default 0.85)
    pub eta_elec_to_shaft: f64,         // motor efficiency (default 0.95)

    // ---- Environmental & Thermal ----
    pub t0_c: f64,                      // sea level temp deg C (default 15)
    pub lapse_kpm: f64,                 // lapse rate K/m (default 0.0065)
    pub t_batt0_c: f64,                 // initial batt temp deg C (default 25)
    pub c_th_kj: f64,                   // battery thermal cap kJ/K (default 500)
    pub ua_wk: f64,                     // battery heat transfer coeff W/K (default 500)
    pub p_heat_kw: f64,                 // heating power kW (default 10)
    pub p_cool_kw: f64,                 // cooling power kW (default 50)
    pub t_low_c: f64,                   // lower comfort temp deg C (default 20)
    pub t_high_c: f64,                  // upper comfort temp deg C (default 35)
    pub kp_temp: f64,                   // temp derate coeff linear (default 0.004)
    pub ke_temp: f64,                   // energy derate coeff linear (default 0.0015)

    // ---- Mission Profile & Flight Phases ----
    pub stage_nm: f64,                  // stage length NM (default 300)
    pub top_fl: f64,                    // top of climb FL (default 200)
    pub dt_s: f64,                      // time step s (default 10)
    pub max_sim_min: f64,               // max sim minutes (default 240)
    pub max_block_time_factor: f64,     // route-derived time guard multiplier (default 2.0)
    pub max_block_time_min: f64,        // explicit time guard, 0 selects automatic
    pub max_takeoff_time_min: f64,      // failed-takeoff guard (default 5)
    pub max_takeoff_distance_m: f64,    // failed-takeoff guard (default 2000)

    // ---- Flight Segment Durations ----
    pub taxi_out_min: f64,              // minutes (default 2.5)
    pub takeoff_min: f64,               // minutes (default 0.5)
    pub approach_min: f64,              // minutes (default 5)
    pub taxi_in_min: f64,               // minutes (default 2.5)

    // ---- Performance Limits & Logic ----
    pub roc_threshold_fpm: f64,         // climb->cruise threshold ft/min (default 100)
    pub roc_target_fpm: f64,            // target climb rate ft/min (default 1500)
    pub vias_taxi_kn: f64,              // knots (default 20)
    pub vias_to_kn: f64,                // knots (default 85)
    pub vias_climb_kn: f64,             // knots (default 140)
    pub vias_cruise_kn: f64,            // knots (default 180)
    pub vias_desc_kn: f64,              // knots (default 160)
    pub hold_ias_kn: f64,               // knots (default 160)
    pub rod_target_fpm: f64,            // target descent rate ft/min (default 700)
    pub cruise_hold_frac: f64,          // fraction (default 0.9)

    // ---- Hold & Reserve ----
    pub enable_hold: bool,              // toggle holding pattern
    pub hold_time_min: f64,             // minutes (default 0)
    pub hold_alt_ft: f64,               // ft (default 5000)

    // ---- Ground & Parasite Power ----
    pub mu_rr_taxi_out: f64,            // rolling resistance (default 0.02)
    pub mu_rr_taxi_in: f64,             // rolling resistance (default 0.02)
    pub p_hotel_ground_kw: f64,         // hotel power ground kW (default 6)
    pub p_hotel_flight_kw: f64,         // hotel power flight kW (default 3)
    pub flare_alt_m: f64,               // flare height m (default 10)
    pub rod_app_fpm: f64,               // approach descent rate ft/min (default 500)

    // ---- Wind & Weather ----
    pub wind_mode: String,              // "constant" or "profile" (default "constant")
    pub tailwind_knots: f64,            // knots (default 0)
    pub wind_profile: Option<Vec<WindProfilePoint>>,

    // ---- Aerodynamic/Performance Parameters ----
    pub cl_max: f64,                    // max lift coeff (default 1.8)
    pub k_liftoff: f64,                 // stall factor liftoff (default 1.20)

    // ---- Autothrottle & Control Law ----
    pub tau_throttle_s: f64,            // throttle lag s (default 8.0)
    pub kp_accel: f64,                  // speed PD P gain (default 0.2)
    pub kd_accel: f64,                  // speed PD D gain (default 0.8)

    // ---- Hybridization ----
    pub alpha_climb: f64,               // Electric fraction in climb
    pub alpha_cruise: f64,              // Electric fraction in cruise
    pub alpha_descent: f64,             // Electric fraction in descent
    pub alpha_taxi: f64,
    pub alpha_takeoff: f64,
    pub alpha_reserve: f64,

    pub custom_wind_ms: f64,
    pub routing_factor_pct: f64,

    pub eta_thermal: f64,               // Turboprop thermal eff (default 0.32)
    pub idle_fuel_kg_h: f64,            // Fuel flow at idle (default 55.0)

    pub enable_regen: bool,
    pub eta_regen: f64,
    pub max_regen_kw: f64,


    pub battery_tech: String,

    // ---- Hydrogen Propulsion ----
    // Two independent axes so any combination is possible:
    //   electric_source:  "Battery" | "FuelCell"   -- what powers the alpha-electric share
    //   combustion_fuel:  "Kerosene" | "Hydrogen"  -- what the (1-alpha) thermal engine burns
    pub electric_source: String,
    pub combustion_fuel: String,

    pub h2_tank_type: String,           // key into hydrogen::get_h2_tank_db()
    pub fuel_cell_tech: String,         // key into hydrogen::get_fuel_cell_db()
    pub h2_source: String,              // key into hydrogen::get_h2_source_db()

    // H2 tank (fuel + tank structure) mass sizing, mirrors batt_frac_6/slope pattern
    pub h2_tank_frac_6: f64,            // target MTOW fraction for tank+fuel at 6 seats (default 0.30)
    pub h2_tank_frac_slope: f64,        // slope vs seat count (default 0.0025)
    pub h2_vol_per_seat_l: f64,         // dedicated tank-bay volume allowance, L/seat (default 400)
    pub max_h2_vol_frac: f64,           // fraction of that allowance usable (default 0.85)
    pub h2_sizing_mode: String,         // "manual_capacity" or "mission_sized"
    pub h2_usable_capacity_kg: f64,     // manual usable H2 capacity; 0 retains legacy fraction sizing
    pub h2_reserve_fraction: f64,       // mission-sizing reserve fraction
    pub h2_sizing_margin_fraction: f64, // mission-sizing mass margin
    pub h2_tank_bay_volume_l: f64,      // explicit tank-bay volume; 0 derives from seats
    pub max_mtow_kg: f64,               // 0 disables MTOW constraint
    pub max_h2_system_mass_fraction: f64, // 0 disables tank-system mass constraint

    pub h2_lhv_kwh_kg: f64,             // default 33.3
    pub h2_boiloff_pct_per_day: f64,    // default 0.30 (LH2), 0 for compressed gas
    pub h2_ground_dwell_h: f64,         // pre-flight ground dwell for boil-off accounting (default 2.0)

    // Fuel cell stack sizing & performance
    pub fc_stack_frac_6: f64,           // stack mass fraction of MTOW at 6 seats (default 0.05)
    pub fc_stack_frac_slope: f64,       // default 0.001
    pub fc_sizing_mode: String,         // "mass_fraction" or "rated_power"
    pub fc_rated_power_kw: f64,         // explicit installed net stack rating; 0 uses fraction sizing
    pub eta_fuel_cell: f64,             // net LHV-basis stack efficiency (default 0.50)
    pub fuel_cell_bop_frac: f64,        // balance-of-plant parasitic fraction (default 0.15)
    pub fuel_cell_power_density_kw_kg: f64, // kW/kg (default 2.0)
    pub fuel_cell_stack_life_h: f64,    // hours before replacement (default 10000)

    // Optional small battery buffer for fast transients when electric_source == "FuelCell"
    pub enable_fc_battery_buffer: bool,
    pub fc_buffer_frac_6: f64,          // buffer battery mass fraction of MTOW (default 0.03)
    pub fc_buffer_frac_slope: f64,      // default 0.001

    // Fuel cell stack thermal management (mirrors battery thermal block, but stacks need
    // continuous active cooling even at cruise, unlike a mostly-passive battery)
    pub t_stack0_c: f64,                // initial stack temp (default 20)
    pub c_th_stack_kj: f64,             // stack thermal capacitance kJ/K (default 300)
    pub ua_stack_wk: f64,               // passive heat transfer coeff W/K (default 800)
    pub p_cool_stack_kw: f64,           // mandatory continuous active cooling draw kW (default 20)
    pub t_stack_op_c: f64,              // nominal operating temp degC (default 75)

    // H2 combustion (turboshaft burning H2 instead of kerosene)
    pub eta_thermal_h2: f64,            // thermal efficiency burning H2 (default 0.33)
    pub idle_fuel_kg_h_h2: f64,         // idle H2 mass flow kg/h (default 20.0)
    pub thermal_rated_power_kw: f64,    // installed thermal shaft rating; 0 retains legacy battery-derived limit

    pub co2_h2_kg_per_kg: f64,          // kgCO2/kg H2 well-to-tank (default from h2_source db)
}

impl Default for SimulationInput {
    fn default() -> Self {
        Self {
            n_seats: 20.0,
            payload_per_seat: 100.0,
            empty_offset: 600.0,
            empty_linear: 95.0,
            // Preserve the previous structural-regression baseline now that
            // these exposed inputs are applied by mass sizing.
            empty_power: 16.0,
            empty_exp: 1.18,
            pt_per_seat: 25.0,
            cabin_vol_per_seat: 550.0,
            max_batt_vol_frac: 0.50,
            battery_tech: "Lithium-Sulfur (Li-S) (500 Wh/kg assumed pack)".to_string(),

            enable_regen: false,
            eta_regen: 0.6,
            max_regen_kw: 500.0,

            batt_frac_6: 0.55,
            batt_frac_slope: 0.0025,
            dens_g_whkg: 500.0,
            dens_v_whl: 550.0,
            soc_min: 0.10,
            soc_max: 0.90,
            system_eff: 0.92,
            aging: 0.90,
            c_rate_cont: 1.5,
            c_rate_peak: 3.0,
            battery_sizing_mode: "legacy_fraction".to_string(),
            battery_usable_energy_kwh: 0.0,
            battery_reserve_fraction: 0.15,
            battery_sizing_margin_fraction: 0.05,
            electric_sizing_mode: "battery_c_rate".to_string(),
            electric_rated_power_kw: 0.0,

            s_m2: 32.0,
            b_m: 18.0,
            cd0: 0.025,
            e_oswald: 0.85,

            eta_prop0: 0.85,
            eta_elec_to_shaft: 0.95,

            t0_c: 15.0,
            lapse_kpm: 0.0065,
            t_batt0_c: 25.0,
            c_th_kj: 500.0,
            ua_wk: 500.0,
            p_heat_kw: 10.0,
            p_cool_kw: 50.0,
            t_low_c: 20.0,
            t_high_c: 35.0,
            kp_temp: 0.004,
            ke_temp: 0.0015,

            stage_nm: 300.0,
            top_fl: 200.0,
            dt_s: 10.0,
            max_sim_min: 240.0,
            max_block_time_factor: 2.0,
            max_block_time_min: 0.0,
            max_takeoff_time_min: 0.0,
            max_takeoff_distance_m: 0.0,

            taxi_out_min: 2.5,
            takeoff_min: 0.5,
            approach_min: 5.0,
            taxi_in_min: 2.5,

            roc_threshold_fpm: 100.0,
            roc_target_fpm: 1500.0,
            vias_taxi_kn: 20.0,
            vias_to_kn: 85.0,
            vias_climb_kn: 140.0,
            vias_cruise_kn: 180.0,
            vias_desc_kn: 160.0,
            hold_ias_kn: 160.0,
            rod_target_fpm: 700.0,
            cruise_hold_frac: 0.9,

            enable_hold: false,
            hold_time_min: 0.0,
            hold_alt_ft: 5000.0,

            mu_rr_taxi_out: 0.02,
            mu_rr_taxi_in: 0.02,
            p_hotel_ground_kw: 6.0,
            p_hotel_flight_kw: 3.0,
            flare_alt_m: 10.0,
            rod_app_fpm: 500.0,

            wind_mode: "constant".to_string(),
            tailwind_knots: 0.0,
            wind_profile: None,

            cl_max: 1.8,
            k_liftoff: 1.20,

            tau_throttle_s: 8.0,
            kp_accel: 0.2,
            kd_accel: 0.8,

            alpha_climb: 1.0,
            alpha_cruise: 1.0,
            alpha_descent: 1.0,
            alpha_taxi: 1.0,
            alpha_takeoff: 1.0,
            alpha_reserve: 1.0,

            custom_wind_ms: 0.0,
            routing_factor_pct: 0.0,

            eta_thermal: 0.32,
            idle_fuel_kg_h: 55.0,

            electric_source: "Battery".to_string(),
            combustion_fuel: "Kerosene".to_string(),

            h2_tank_type: "LH2 (Cryogenic Liquid)".to_string(),
            fuel_cell_tech: "PEM (Current)".to_string(),
            h2_source: "Green (Electrolysis)".to_string(),

            h2_tank_frac_6: 0.30,
            h2_tank_frac_slope: 0.0025,
            h2_vol_per_seat_l: 400.0,
            max_h2_vol_frac: 0.85,
            h2_sizing_mode: "manual_capacity".to_string(),
            h2_usable_capacity_kg: 0.0,
            h2_reserve_fraction: 0.15,
            h2_sizing_margin_fraction: 0.05,
            h2_tank_bay_volume_l: 0.0,
            max_mtow_kg: 0.0,
            max_h2_system_mass_fraction: 0.0,

            h2_lhv_kwh_kg: 33.3,
            h2_boiloff_pct_per_day: 0.30,
            h2_ground_dwell_h: 2.0,

            fc_stack_frac_6: 0.05,
            fc_stack_frac_slope: 0.001,
            fc_sizing_mode: "mass_fraction".to_string(),
            fc_rated_power_kw: 0.0,
            eta_fuel_cell: 0.50,
            fuel_cell_bop_frac: 0.15,
            fuel_cell_power_density_kw_kg: 2.0,
            fuel_cell_stack_life_h: 10_000.0,

            enable_fc_battery_buffer: false,
            fc_buffer_frac_6: 0.03,
            fc_buffer_frac_slope: 0.001,

            t_stack0_c: 20.0,
            c_th_stack_kj: 300.0,
            ua_stack_wk: 800.0,
            p_cool_stack_kw: 20.0,
            t_stack_op_c: 75.0,

            eta_thermal_h2: 0.33,
            idle_fuel_kg_h_h2: 20.0,
            thermal_rated_power_kw: 0.0,

            co2_h2_kg_per_kg: 0.9,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlightProfileRow {
    pub t_s: f64,
    pub t_min: f64,
    pub phase: String,
    pub alt_m: f64,
    pub alt_ft: f64,
    pub s_m: f64,
    pub s_km: f64,
    pub s_nm: f64,
    pub v_tas: f64,
    pub v_tas_kn: f64,
    pub v_cmd_tas: f64,
    pub v_gs: f64,
    pub v_gs_kn: f64,
    pub rho: f64,
    pub p_req_kw: f64,
    pub p_avail_shaft_kw: f64,
    pub roc_fpm: f64,
    pub t_batt_c: f64,
    pub fp: f64,
    pub fe: f64,
    pub e_step_kwh: f64,
    pub e_elec_kwh_cum: f64,
    pub n_seats: f64,
    pub dens_g_whkg: f64,
    #[serde(default)]
    pub t_stack_c: f64,
    #[serde(default)]
    pub h2_step_kg: f64,
    #[serde(default)]
    pub h2_kg_cum: f64,
    #[serde(default)]
    pub h2_step_m3: f64,
    #[serde(default)]
    pub h2_m3_cum: f64,
    #[serde(default)]
    pub fuel_step_kg: f64,
    #[serde(default)]
    pub fuel_kg_cum: f64,
    #[serde(default)]
    pub accel_ms2: f64,
    #[serde(default)]
    pub accel_g: f64,
    #[serde(default)]
    pub cl: f64,
    #[serde(default)]
    pub cd: f64,
    #[serde(default)]
    pub lift_to_drag: f64,
    #[serde(default)]
    pub thrust_n: f64,
    #[serde(default)]
    pub drag_n: f64,
    #[serde(default)]
    pub q_dyn: f64,
    #[serde(default)]
    pub roc_mps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseSummary {
    pub phase: String,
    pub time_min: f64,
    pub dist_nm: f64,
    pub energy_kwh: f64,
    pub max_tas_kn: f64,
    pub max_roc_fpm: f64,
    pub end_alt_ft: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MassMetrics {
    pub mtow_kg: f64,
    pub batt_kg: f64,
    pub payload_kg: f64,
    pub empty_kg: f64,
    // Hydrogen-specific (0 unless electric_source == "FuelCell" or combustion_fuel == "Hydrogen")
    pub h2_fuel_kg: f64,           // usable H2 fuel mass carried
    pub h2_tank_structure_kg: f64, // tank structure + insulation mass (not usable fuel)
    pub fc_stack_kg: f64,          // fuel cell stack mass (0 unless electric_source == "FuelCell")
    pub fc_buffer_kg: f64,         // optional small buffer battery mass alongside a fuel cell
    #[serde(default)]
    pub h2_system_kg: f64,
    #[serde(default)]
    pub h2_system_volume_l: f64,
    #[serde(default)]
    pub h2_volume_utilization: f64,
    #[serde(default)]
    pub h2_capacity_requested_kg: f64,
    #[serde(default)]
    pub h2_capacity_shortfall_kg: f64,
    #[serde(default)]
    pub fc_stack_kw_rated: f64,
    #[serde(default)]
    pub batt_capacity_requested_kwh: f64,
    #[serde(default)]
    pub batt_capacity_shortfall_kwh: f64,
    #[serde(default)]
    pub electric_rated_power_kw: f64,
}

impl Default for MassMetrics {
    fn default() -> Self {
        Self {
            mtow_kg: 0.0,
            batt_kg: 0.0,
            payload_kg: 0.0,
            empty_kg: 0.0,
            h2_fuel_kg: 0.0,
            h2_tank_structure_kg: 0.0,
            fc_stack_kg: 0.0,
            fc_buffer_kg: 0.0,
            h2_system_kg: 0.0,
            h2_system_volume_l: 0.0,
            h2_volume_utilization: 0.0,
            h2_capacity_requested_kg: 0.0,
            h2_capacity_shortfall_kg: 0.0,
            fc_stack_kw_rated: 0.0,
            batt_capacity_requested_kwh: 0.0,
            batt_capacity_shortfall_kwh: 0.0,
            electric_rated_power_kw: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub profile: Vec<FlightProfileRow>,
    pub phase_summaries: Vec<PhaseSummary>,
    pub fl_phys: f64,
    pub e_usable_kwh: f64,
    pub total_energy_kwh: f64,
    pub total_fuel_burn_kg: f64,
    pub energy_buffer_kwh: f64,
    pub t_flight_min: f64,
    pub block_time_min: f64,
    pub distance_nm: f64,
    pub end_reason: String,
    #[serde(default)]
    pub end_reason_detail: Option<String>,
    pub mass: MassMetrics,
    #[serde(default)]
    pub total_h2_combustion_kg: f64, // H2 burned by the thermal engine (combustion_fuel == "Hydrogen")
    #[serde(default)]
    pub total_h2_fc_kg: f64,         // H2 consumed by the fuel cell stack
    #[serde(default)]
    pub total_h2_kg: f64,            // sum of the two above
    #[serde(default)]
    pub h2_usable_kg: f64,           // usable H2 budget after boil-off (0 if not used)
    #[serde(default)]
    pub final_phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSimulationRequest {
    pub base_input: SimulationInput,
    pub seat_configs: Option<Vec<f64>>,
    pub energy_densities: Option<Vec<f64>>,
    #[serde(default, alias = "batch_sweeps")]
    pub parameter_sweeps: std::collections::HashMap<String, Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSimulationResult {
    pub runs: Vec<SimulationResult>,
}


impl SimulationResult {
    /// Net battery energy plus the lower heating value of fuel consumed.
    pub fn mission_energy_kwh_equivalent(&self) -> f64 {
        self.total_energy_kwh
            + self.total_h2_kg * crate::hydrogen::H2_LHV_KWH_KG
            + self.total_fuel_burn_kg * 11.92
    }
}
