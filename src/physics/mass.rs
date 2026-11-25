use crate::model::{MassMetrics, SimulationInput};

/// Sizes MTOW and the energy-storage subsystem masses.
///
/// Two independent axes decide which subsystems get sized:
///   - `electric_source`: "Battery" (existing pack) or "FuelCell" (H2 tank + stack, optionally
///      with a small buffer battery for fast transients)
///   - `combustion_fuel`: "Kerosene" (not mass-tracked, existing simplification) or "Hydrogen"
///      (tank + fuel IS mass-tracked, since tank weight is the dominant H2 design constraint)
///
/// Every active subsystem is given a target mass fraction of MTOW (mirroring the original
/// `batt_frac_6`/`batt_frac_slope` pattern), the fractions are solved simultaneously against
/// MTOW, and then each volume-constrained subsystem is checked against its own dedicated
/// volume allowance (cabin fraction for batteries, tank-bay allowance for H2).
pub fn init_aircraft_mass(
    input: &SimulationInput,
    h2_capacity_override_kg: Option<f64>,
    battery_capacity_override_kwh: Option<f64>,
) -> MassMetrics {
    let payload_kg = input.n_seats * input.payload_per_seat;
    // Regional passenger cabin scaling (cabin amenities, galleys, pressurization structure).
    // The coefficient and exponent are exposed controls, so they must be part of
    // the structural regression rather than hidden constants.
    let empty_struct = input.empty_offset * (input.n_seats / 20.0).powf(1.12)
        + input.empty_linear * input.n_seats
        + input.empty_power.max(0.0) * input.n_seats.powf(input.empty_exp.max(0.0));
    let powertrain_kg = input.pt_per_seat * input.n_seats;
    let non_energy_kg = payload_kg + empty_struct + powertrain_kg;

    let uses_battery = input.electric_source == "Battery";
    let uses_fuel_cell = input.electric_source == "FuelCell";
    let uses_h2 = uses_fuel_cell || input.combustion_fuel == "Hydrogen";
    let uses_buffer = uses_fuel_cell && input.enable_fc_battery_buffer;

    // Calculate the overall energy fractions for scaling tank and battery masses
    let electric_energy_fraction = input.alpha_cruise.clamp(0.0, 1.0);
    let thermal_energy_fraction = 1.0 - electric_energy_fraction;

    let h2_energy_fraction = (if uses_fuel_cell { electric_energy_fraction } else { 0.0 })
        + (if input.combustion_fuel == "Hydrogen" { thermal_energy_fraction } else { 0.0 });

    let battery_energy_fraction = if uses_battery { electric_energy_fraction } else { 0.0 };

    // Target mass fractions of MTOW for each active subsystem, scaled by hybridization degree.
    let batt_frac = if uses_battery {
        let base = (input.batt_frac_6 - input.batt_frac_slope * (input.n_seats - 6.0)).max(0.25);
        base * battery_energy_fraction
    } else {
        0.0
    };

    let h2_frac = if uses_h2 {
        let base = (input.h2_tank_frac_6 - input.h2_tank_frac_slope * (input.n_seats - 6.0)).max(0.10);
        base * h2_energy_fraction
    } else {
        0.0
    };

    let fc_frac = if uses_fuel_cell {
        let base = (input.fc_stack_frac_6 - input.fc_stack_frac_slope * (input.n_seats - 6.0)).max(0.01);
        base * electric_energy_fraction
    } else {
        0.0
    };

    let buffer_frac = if uses_buffer {
        let base = (input.fc_buffer_frac_6 - input.fc_buffer_frac_slope * (input.n_seats - 6.0)).max(0.005);
        base * electric_energy_fraction
    } else {
        0.0
    };

    let total_frac = (batt_frac + h2_frac + fc_frac + buffer_frac).min(0.90);

    let mut mtow_kg = non_energy_kg / (1.0 - total_frac);
    let mut batt_kg = mtow_kg * batt_frac;
    let mut h2_system_kg = mtow_kg * h2_frac; // tank structure + usable fuel, combined
    let mut fc_stack_kg = mtow_kg * fc_frac;
    let mut fc_buffer_kg = mtow_kg * buffer_frac;

    if uses_fuel_cell && input.fc_sizing_mode == "rated_power" && input.fc_rated_power_kw > 0.0 {
        let p_mw = input.fc_rated_power_kw / 1000.0;
        let bop_tms_scale = 1.0 + 0.30 * p_mw.max(0.1).powf(0.35);
        fc_stack_kg = (input.fc_rated_power_kw / input.fuel_cell_power_density_kw_kg.max(0.01)) * bop_tms_scale;
        mtow_kg = non_energy_kg + batt_kg + h2_system_kg + fc_stack_kg + fc_buffer_kg;
    }

    // Volume constraint on the battery pack (existing behaviour, unchanged).
    if uses_battery {
        let requested_batt_kwh = battery_capacity_override_kwh.unwrap_or_else(|| {
            if input.battery_sizing_mode == "manual_energy" {
                input.battery_usable_energy_kwh
            } else {
                0.0
            }
        });
        if requested_batt_kwh > 0.0 {
            let usable_fraction = (input.soc_max - input.soc_min)
                * input.system_eff
                * input.aging;
            batt_kg = requested_batt_kwh * 1000.0
                / (input.dens_g_whkg * usable_fraction.max(0.01));
            mtow_kg = non_energy_kg + batt_kg + h2_system_kg + fc_stack_kg + fc_buffer_kg;
        }

        // Realistic gravimetric battery mass fraction ceiling (commuter vs regional airframes)
        let max_batt_frac = if input.electric_source == "Battery" && input.alpha_cruise >= 0.99 {
            (0.38 - 0.0035 * (input.n_seats - 10.0)).clamp(0.10, 0.38)
        } else if input.electric_source == "Battery" && input.alpha_cruise > 0.0 {
            (0.25 - 0.0018 * (input.n_seats - 10.0)).clamp(0.08, 0.25)
        } else {
            0.45
        };
        let max_allowed_batt_kg = max_batt_frac * (non_energy_kg + h2_system_kg + fc_stack_kg + fc_buffer_kg) / (1.0 - max_batt_frac);
        if batt_kg > max_allowed_batt_kg {
            batt_kg = max_allowed_batt_kg;
            mtow_kg = non_energy_kg + batt_kg + h2_system_kg + fc_stack_kg + fc_buffer_kg;
        }

        let cabin_vol_l = input.n_seats * input.cabin_vol_per_seat;
        let vol_cap_l = input.max_batt_vol_frac * cabin_vol_l;
        let batt_vol_l = (batt_kg * input.dens_g_whkg) / input.dens_v_whl;
        if batt_vol_l > vol_cap_l {
            batt_kg = vol_cap_l * input.dens_v_whl / input.dens_g_whkg;
            mtow_kg = non_energy_kg + batt_kg + h2_system_kg + fc_stack_kg + fc_buffer_kg;
        }
    }

    // Volume constraint on the H2 tank system (dedicated fuselage bay, not the cabin).
    let db = crate::hydrogen::get_h2_tank_db();
    let tank_tech = db.get(input.h2_tank_type.as_str());
    let vol_density = tank_tech.map(|t| t.volumetric_density_kg_m3).unwrap_or(40.0);
    let gravimetric_index = tank_tech.map(|t| t.gravimetric_index).unwrap_or(0.20);

    if uses_h2 {
        let requested_h2_kg = h2_capacity_override_kg.unwrap_or(input.h2_usable_capacity_kg);
        if requested_h2_kg > 0.0 {
            h2_system_kg = requested_h2_kg / gravimetric_index;
            mtow_kg = non_energy_kg + batt_kg + h2_system_kg + fc_stack_kg + fc_buffer_kg;
        }

        let vol_scale = (1.0 - 0.0075 * (input.n_seats - 10.0)).clamp(0.40, 1.0);
        let bay_vol_l = if input.h2_tank_bay_volume_l > 0.0 {
            input.h2_tank_bay_volume_l
        } else {
            input.n_seats * input.h2_vol_per_seat_l * input.max_h2_vol_frac * vol_scale
        };
        // Volumetric density is expressed as kg of usable H2 per m³ of the
        // complete tank system, not kg of tank-system hardware per m³.
        let h2_system_vol_l = (h2_system_kg * gravimetric_index / vol_density) * 1000.0;
        if h2_system_vol_l > bay_vol_l {
            h2_system_kg = bay_vol_l / 1000.0 * vol_density / gravimetric_index;
            mtow_kg = non_energy_kg + batt_kg + h2_system_kg + fc_stack_kg + fc_buffer_kg;
        }
        if input.max_h2_system_mass_fraction > 0.0 {
            let max_system_kg = input.max_h2_system_mass_fraction * mtow_kg;
            if h2_system_kg > max_system_kg {
                h2_system_kg = max_system_kg;
                mtow_kg = non_energy_kg + batt_kg + h2_system_kg + fc_stack_kg + fc_buffer_kg;
            }
        }
    }

    // Small battery buffer alongside a fuel cell is not volume-constrained (assumed small
    // enough to tuck into the powertrain bay) — mass fraction sizing is sufficient for v1.
    if !uses_buffer {
        fc_buffer_kg = 0.0;
    }

    let h2_fuel_kg = h2_system_kg * gravimetric_index;
    let h2_tank_structure_kg = h2_system_kg - h2_fuel_kg;
    let h2_system_volume_l = if uses_h2 {
        h2_system_kg * gravimetric_index / vol_density * 1000.0
    } else {
        0.0
    };
    let vol_scale = (1.0 - 0.0075 * (input.n_seats - 10.0)).clamp(0.40, 1.0);
    let h2_bay_volume_l = if input.h2_tank_bay_volume_l > 0.0 {
        input.h2_tank_bay_volume_l
    } else {
        input.n_seats * input.h2_vol_per_seat_l * input.max_h2_vol_frac * vol_scale
    };
    let h2_requested_kg = h2_capacity_override_kg.unwrap_or(input.h2_usable_capacity_kg);
    let batt_requested_kwh = battery_capacity_override_kwh.unwrap_or_else(|| {
        if input.battery_sizing_mode == "manual_energy" {
            input.battery_usable_energy_kwh
        } else {
            0.0
        }
    });
    let batt_usable_kwh = batt_kg * input.dens_g_whkg
        * (input.soc_max - input.soc_min) * input.system_eff * input.aging / 1000.0;

    MassMetrics {
        mtow_kg,
        batt_kg,
        payload_kg,
        empty_kg: empty_struct,
        h2_fuel_kg,
        h2_tank_structure_kg,
        fc_stack_kg,
        fc_buffer_kg,
        h2_system_kg,
        h2_system_volume_l,
        h2_volume_utilization: if uses_h2 && h2_bay_volume_l > 0.0 {
            h2_system_volume_l / h2_bay_volume_l
        } else {
            0.0
        },
        h2_capacity_requested_kg: h2_requested_kg,
        h2_capacity_shortfall_kg: (h2_requested_kg - h2_fuel_kg).max(0.0),
        fc_stack_kw_rated: if uses_fuel_cell && input.fc_sizing_mode == "rated_power" && input.fc_rated_power_kw > 0.0 {
            input.fc_rated_power_kw
        } else {
            fc_stack_kg * input.fuel_cell_power_density_kw_kg
        },
        batt_capacity_requested_kwh: batt_requested_kwh,
        batt_capacity_shortfall_kwh: (batt_requested_kwh - batt_usable_kwh).max(0.0),
        electric_rated_power_kw: if uses_battery && input.electric_sizing_mode == "rated_power" && input.electric_rated_power_kw > 0.0 {
            input.electric_rated_power_kw
        } else {
            batt_kg * input.dens_g_whkg * input.c_rate_cont / 1000.0
        },
    }
}
