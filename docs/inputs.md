# Mission inputs

Start with `SimulationInput::default()` and override the fields you know. The defaults describe an illustrative aircraft. For a comparison between real aircraft, replace the geometry, mass, propulsion, and mission assumptions with documented values.

```rust
use trace_engine::{model::SimulationInput, physics::build_profile_dynamic};

let input = SimulationInput {
    n_seats: 20.0,
    stage_nm: 80.0,
    top_fl: 100.0,
    ..SimulationInput::default()
};
let mission = build_profile_dynamic(&input);
println!("{}: {:.1} NM flown", mission.end_reason, mission.distance_nm);
```

Check `end_reason` before interpreting energy or fuel totals. A failed mission can return a partial flight profile.

## Choose inputs by subsystem

| Subsystem | Fields to start with | Meaning |
| --- | --- | --- |
| Mission | `stage_nm`, `top_fl`, `dt_s` | Route length in nautical miles, top of climb in hundreds of feet (FL100 is 10,000 ft), and positive integration step in seconds. |
| Aircraft | `n_seats`, `payload_per_seat`, `s_m2`, `b_m` | Seat count, modeled payload in kg per seat, wing area in m², and span in m. The payload model multiplies seat count by payload per seat. |
| Drag and lift | `cd0`, `e_oswald`, `cl_max` | Dimensionless zero-lift drag, Oswald efficiency, and maximum lift coefficient. |
| Speeds | `vias_*_kn`, `hold_ias_kn` | Phase targets in knots of indicated airspeed. |
| Wind | `wind_mode`, `tailwind_knots`, `wind_profile` | Use `constant` for a single wind value or `profile` with altitude points in metres. Positive wind is tailwind, negative is headwind. |
| Holding | `enable_hold`, `hold_time_min`, `hold_alt_ft` | Enable the hold and set its duration in minutes and altitude in feet. |

## Choose propulsion and storage

- `electric_source` selects `Battery` or `FuelCell`. `combustion_fuel` selects `Kerosene` or `Hydrogen` for the combustion share.
- The `alpha_*` fields represent the electric share of propulsion by phase. Enter fractions from 0 to 1, so 0.4 means 40% electric. To request an entirely electric propulsion split, set each phase share to 1.
- For batteries, use `legacy_fraction` for mass fraction sizing, `manual_energy` with positive `battery_usable_energy_kwh` for a specified usable capacity, or `mission_sized` to estimate capacity from a simulation. The selected battery technology can override specific energy, minimum state of charge, and system efficiency. Check the catalog in [battery.rs](../src/battery.rs) when setting these values.
- For hydrogen, use `manual_capacity` with positive `h2_usable_capacity_kg` to specify usable capacity or `mission_sized` to estimate it from the mission. Zero manual capacity retains fraction sizing. Tank and stack selection can override physical assumptions. See [hydrogen.rs](../src/hydrogen.rs) for technology keys.
- Use `rated_power` with a positive rating when specifying an installed electric shaft or fuel cell power limit. The battery C-rate and fuel cell mass fraction modes derive ratings from the modeled hardware.
- Reserve and sizing margin fields ending in `_fraction` use decimal fractions. `h2_boiloff_pct_per_day` uses percent per day, so 0.30 means 0.30% per day.

The field definitions and exact executable defaults are in [model.rs](../src/model.rs). Use finite values, positive aircraft dimensions, a positive timestep, and ordered battery state-of-charge limits between 0 and 1. For reproducible comparisons, record the complete inputs and the TRACE commit used.
