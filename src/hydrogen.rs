use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Storage tank technology for gaseous/liquid hydrogen.
/// `gravimetric_index` and `volumetric_density_kg_m3` describe the *whole tank system*
/// (structure + insulation + valves), not the raw H2 alone — this is what makes H2 storage
/// so different from batteries: the tank itself can weigh more than the fuel it carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H2TankTech {
    pub name: &'static str,
    pub gravimetric_index: f64,        // kg H2 / kg total tank system (fuel + structure)
    pub volumetric_density_kg_m3: f64, // kg H2 / m^3 of total tank system volume
    pub boiloff_pct_per_day: f64,      // passive venting loss, %/day (0 for compressed gas tanks)
    pub co2_manufacturing_kg_per_kg: f64, // kgCO2 per kg H2 capacity (tank manufacturing)
}

pub fn get_h2_tank_db() -> HashMap<&'static str, H2TankTech> {
    let mut m = HashMap::new();
    m.insert("LH2 (Cryogenic Liquid)", H2TankTech {
        name: "LH2 (Cryogenic Liquid)",
        gravimetric_index: 0.22,
        volumetric_density_kg_m3: 42.0,
        boiloff_pct_per_day: 0.30,
        co2_manufacturing_kg_per_kg: 25.0,
    });
    m.insert("CGH2 350 bar (Compressed Gas)", H2TankTech {
        name: "CGH2 350 bar (Compressed Gas)",
        gravimetric_index: 0.075,
        volumetric_density_kg_m3: 20.0,
        boiloff_pct_per_day: 0.0,
        co2_manufacturing_kg_per_kg: 35.0,
    });
    m.insert("CGH2 700 bar (Compressed Gas)", H2TankTech {
        name: "CGH2 700 bar (Compressed Gas)",
        gravimetric_index: 0.085,
        volumetric_density_kg_m3: 36.0,
        boiloff_pct_per_day: 0.0,
        co2_manufacturing_kg_per_kg: 42.0,
    });
    m
}

/// Fuel cell stack technology (converts H2 -> electricity, feeding the same
/// electric motor path that a battery would otherwise power).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuelCellTech {
    pub name: &'static str,
    pub eta_stack: f64,          // net LHV-basis electrical efficiency at the bus
    pub bop_parasitic_frac: f64, // fraction of gross stack output consumed by balance-of-plant
    pub power_density_kw_kg: f64, // kW (net) per kg of stack, used to size stack mass
    pub stack_life_h: f64,       // operating hours before EOL/replacement
    pub t_stack_op_c: f64,       // nominal operating temperature, degC
}

pub fn get_fuel_cell_db() -> HashMap<&'static str, FuelCellTech> {
    let mut m = HashMap::new();
    m.insert("PEM (Current)", FuelCellTech {
        name: "PEM (Current)",
        eta_stack: 0.50,
        bop_parasitic_frac: 0.15,
        power_density_kw_kg: 1.2,
        stack_life_h: 10_000.0,
        t_stack_op_c: 80.0,
    });
    m.insert("PEM (Advanced/2030)", FuelCellTech {
        name: "PEM (Advanced/2030)",
        eta_stack: 0.58,
        bop_parasitic_frac: 0.10,
        power_density_kw_kg: 2.5,
        stack_life_h: 20_000.0,
        t_stack_op_c: 80.0,
    });
    m.insert("SOFC (Solid Oxide, future)", FuelCellTech {
        name: "SOFC (Solid Oxide, future)",
        eta_stack: 0.65,
        bop_parasitic_frac: 0.12,
        power_density_kw_kg: 1.0,
        stack_life_h: 30_000.0,
        t_stack_op_c: 650.0,
    });
    m
}

/// Hydrogen production/supply pathway, provides a well-to-tank CO2 factor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H2SourceTech {
    pub name: &'static str,
    pub co2_kg_per_kg: f64,   // kgCO2 per kg H2 (well-to-tank)
}

pub fn get_h2_source_db() -> HashMap<&'static str, H2SourceTech> {
    let mut m = HashMap::new();
    m.insert("Grey (SMR)", H2SourceTech {
        name: "Grey (SMR)",
        co2_kg_per_kg: 10.5,
    });
    m.insert("Blue (SMR + CCS)", H2SourceTech {
        name: "Blue (SMR + CCS)",
        co2_kg_per_kg: 3.5,
    });
    m.insert("Green (Electrolysis)", H2SourceTech {
        name: "Green (Electrolysis)",
        co2_kg_per_kg: 0.9,
    });
    m
}

/// LHV of hydrogen in kWh/kg (~120 MJ/kg / 3.6).
pub const H2_LHV_KWH_KG: f64 = 33.3;
