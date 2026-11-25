use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryTech {
    pub name: &'static str,
    pub specific_energy: f64, // kWh/kg
    pub dod: f64,             // Depth of Discharge limit
    pub eta: f64,             // Round-trip efficiency
    pub cycle_life: f64,      // Max cycles before EOL
    pub co2_manufacturing: f64, // kgCO2/kWh
}

pub fn get_battery_db() -> HashMap<&'static str, BatteryTech> {
    let mut m = HashMap::new();
    m.insert("Lithium-Ion (NMC) (promising)", BatteryTech {
        name: "Lithium-Ion (NMC) (promising)",
        specific_energy: 0.195,
        dod: 0.80,
        eta: 0.93,
        cycle_life: 1500.0,
        co2_manufacturing: 80.0,
    });
    m.insert("Lithium-Ion (LFP) (promising)", BatteryTech {
        name: "Lithium-Ion (LFP) (promising)",
        specific_energy: 0.160,
        dod: 0.85,
        eta: 0.95,
        cycle_life: 3000.0,
        co2_manufacturing: 60.0,
    });
    m.insert("Solid State (Li-Metal) (future)", BatteryTech {
        name: "Solid State (Li-Metal) (future)",
        specific_energy: 0.350,
        dod: 0.90,
        eta: 0.96,
        cycle_life: 1000.0,
        co2_manufacturing: 120.0,
    });
    // Exploratory pack-level breakthrough case. This is deliberately named as
    // an assumption rather than a market forecast; its 500 Wh/kg value is the
    // explicit sensitivity target used by the scenario generator.
    m.insert("Lithium-Sulfur (Li-S) (500 Wh/kg assumed pack)", BatteryTech {
        name: "Lithium-Sulfur (Li-S) (500 Wh/kg assumed pack)",
        specific_energy: 0.500,
        dod: 0.80,
        eta: 0.90,
        cycle_life: 500.0,
        co2_manufacturing: 100.0,
    });
    m
}
