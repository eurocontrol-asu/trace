//! Targeted pure-battery payload--range frontier study.
//!
//! This is intentionally separate from `generate_scenarios`: the broad
//! technology comparison uses 10-seat / 100-NM increments and cannot locate a
//! commuter boundary such as 150 NM.

use trace_engine::model::SimulationInput;
use trace_engine::physics::build_profile_dynamic;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufWriter, Write};

#[derive(Clone, Copy)]
struct PackCase {
    label: &'static str,
    wh_per_kg: f64,
}

#[derive(Clone, Copy)]
struct SensitivityCase {
    flight_level: f64,
    wind_kn: f64,
    ambient_c: f64,
    cd0: f64,
    e_oswald: f64,
    reserve_fraction: f64,
    sizing_margin_fraction: f64,
    label: &'static str,
}

fn main() -> std::io::Result<()> {
    // Pack-level values are explicit assumptions.  Do not set `battery_tech`:
    // named database entries override the raw Wh/kg input inside the simulator.
    let pack_cases = [
        PackCase {
            label: "Current Li-ion pack",
            wh_per_kg: 195.0,
        },
        PackCase {
            label: "Improved Li-ion pack",
            wh_per_kg: 225.0,
        },
        PackCase {
            label: "Near-term Li-ion pack",
            wh_per_kg: 250.0,
        },
        PackCase {
            label: "Advanced Li-metal pack",
            wh_per_kg: 300.0,
        },
        PackCase {
            label: "Future solid-state pack",
            wh_per_kg: 350.0,
        },
        PackCase {
            label: "Advanced solid-state pack",
            wh_per_kg: 400.0,
        },
        PackCase {
            label: "Aspirational pack",
            wh_per_kg: 450.0,
        },
    ];
    let seats: Vec<usize> = (6..=30).collect();
    let ranges_nm: Vec<usize> = (50..=400).step_by(10).collect();
    let flight_levels = [150.0, 200.0, 250.0];
    let winds_kn = [-20.0, 0.0, 20.0];
    let temperatures_c = [5.0, 15.0, 30.0];
    // These three technical cases mirror the fourth dimension of the original
    // 162k study.  Together with FL, wind, and ambient temperature they yield
    // 3 × 3 × 3 × 3 = 81 operational/technical variations per design point.
    let technical_cases = [
        ("Base", 0.025, 0.85, 0.15, 0.05),
        ("Optimistic aero / reserve", 0.021, 0.90, 0.10, 0.03),
        ("Conservative aero / reserve", 0.030, 0.80, 0.20, 0.08),
    ];
    let mut sensitivities = Vec::with_capacity(81);
    for &flight_level in &flight_levels {
        for &wind_kn in &winds_kn {
            for &ambient_c in &temperatures_c {
                for &(label, cd0, e_oswald, reserve_fraction, sizing_margin_fraction) in
                    &technical_cases
                {
                    sensitivities.push(SensitivityCase {
                        flight_level,
                        wind_kn,
                        ambient_c,
                        cd0,
                        e_oswald,
                        reserve_fraction,
                        sizing_margin_fraction,
                        label,
                    });
                }
            }
        }
    }
    let total = pack_cases.len() * seats.len() * ranges_nm.len() * sensitivities.len();

    println!("Battery payload-range frontier: {total} central scenarios");
    println!("  seats: 6--30 (1-seat steps)");
    println!("  range: 50--400 NM (10-NM steps)");
    println!("  pack energy: 195, 225, 250, 300, 350, 400, 450 Wh/kg");
    println!(
        "  sensitivity variations: {} per design point",
        sensitivities.len()
    );

    let mut jobs: Vec<(PackCase, usize, usize, SensitivityCase)> = Vec::with_capacity(total);
    for pack in pack_cases {
        for &seat in &seats {
            for &range_nm in &ranges_nm {
                for sensitivity in &sensitivities {
                    jobs.push((pack, seat, range_nm, *sensitivity));
                }
            }
        }
    }

    let rows: Vec<String> = jobs
        .par_iter()
        .copied()
        .map(|(pack, seat, range_nm, sensitivity)| {
            let mut input = SimulationInput::default();
            input.n_seats = seat as f64;
            input.stage_nm = range_nm as f64;
            input.top_fl = sensitivity.flight_level;
            input.tailwind_knots = sensitivity.wind_kn;
            input.custom_wind_ms = sensitivity.wind_kn * 0.514_444;
            input.t0_c = sensitivity.ambient_c;
            input.cd0 = sensitivity.cd0;
            input.e_oswald = sensitivity.e_oswald;
            input.electric_source = "Battery".to_string();
            input.combustion_fuel = "Kerosene".to_string();
            input.alpha_taxi = 1.0;
            input.alpha_takeoff = 1.0;
            input.alpha_climb = 1.0;
            input.alpha_cruise = 1.0;
            input.alpha_descent = 1.0;
            input.alpha_reserve = 1.0;
            input.battery_tech.clear();
            input.dens_g_whkg = pack.wh_per_kg;
            input.dens_v_whl = 550.0;
            input.soc_min = 0.20;
            input.soc_max = 0.90;
            input.system_eff = 0.93;
            input.aging = 0.90;
            input.battery_sizing_mode = "mission_sized".to_string();
            input.battery_reserve_fraction = sensitivity.reserve_fraction;
            input.battery_sizing_margin_fraction = sensitivity.sizing_margin_fraction;
            input.electric_sizing_mode = "rated_power".to_string();
            input.electric_rated_power_kw = (seat as f64 * 70.0).max(350.0);
            input.max_takeoff_time_min = 5.0;
            input.max_takeoff_distance_m = 1_200.0;
            // A transparent study assumption, retained in the CSV for audit.
            input.max_mtow_kg = (seat as f64 * 450.0).max(5_000.0);

            let result = build_profile_dynamic(&input);
            let mass = &result.mass;
            let batt_mass_fraction = mass.batt_kg / mass.mtow_kg.max(1.0);
            let batt_volume_l = mass.batt_kg * input.dens_g_whkg / input.dens_v_whl;
            let batt_volume_limit_l = input.max_batt_vol_frac * input.n_seats * input.cabin_vol_per_seat;
            let volume_utilization = batt_volume_l / batt_volume_limit_l.max(1.0);
            let binding_limit = if result.end_reason == "mtow_limit" {
                "mtow"
            } else if mass.batt_capacity_shortfall_kwh > 1e-6 && volume_utilization >= 0.999 {
                "battery_volume"
            } else if mass.batt_capacity_shortfall_kwh > 1e-6 {
                "battery_mass_fraction"
            } else if result.end_reason == "energy" {
                "energy"
            } else if result.end_reason == "takeoff_performance" {
                "takeoff_performance"
            } else {
                "none_or_other"
            };

            format!(
                "\"{}\",{},{},\"{}\",{:.0},{:.0},{:.0},{:.3},{:.2},{:.0},{:.0},{:.0},{:.0},{:.2},{:.2},{:.2},{:.3},{:.3},{:.2},{:.2},{:.2},{:.2},\"{}\",\"{}\"",
                pack.label,
                seat,
                range_nm,
                sensitivity.label,
                sensitivity.flight_level,
                sensitivity.wind_kn,
                sensitivity.ambient_c,
                sensitivity.cd0,
                sensitivity.e_oswald,
                input.max_mtow_kg,
                input.max_takeoff_distance_m,
                input.battery_reserve_fraction * 100.0,
                input.battery_sizing_margin_fraction * 100.0,
                result.distance_nm,
                mass.mtow_kg,
                mass.batt_kg,
                batt_mass_fraction,
                volume_utilization,
                mass.batt_capacity_requested_kwh,
                mass.batt_capacity_shortfall_kwh,
                result.e_usable_kwh,
                result.total_energy_kwh,
                result.end_reason,
                binding_limit,
            )
        })
        .collect();

    std::fs::create_dir_all("analysis")?;
    let file = File::create("analysis/battery_frontier_510k.csv")?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "Pack case,Seats,Target NM,Sensitivity case,Flight level,Wind kn,Ambient C,CD0,Oswald efficiency,Max MTOW kg,Takeoff distance m,Reserve pct,Sizing margin pct,Achieved NM,MTOW kg,Battery kg,Battery mass fraction,Battery volume utilization,Requested battery kWh,Battery shortfall kWh,Usable battery kWh,Energy used kWh,End reason,Binding limit")?;
    for row in rows {
        writeln!(writer, "{row}")?;
    }
    writer.flush()?;
    println!("Wrote analysis/battery_frontier_510k.csv");
    Ok(())
}
