# Engine module structure

| Module | Responsibility |
| --- | --- |
| `model` | Mission inputs, flight profile rows, mass breakdown, and simulation results. Field suffixes identify units such as nautical miles, kilograms, kilowatts, and kilowatt-hours. |
| `battery` | Battery technology assumptions, including specific energy, depth of discharge, and efficiency. |
| `hydrogen` | Hydrogen tank and fuel cell technology assumptions and hydrogen fuel properties. |
| `physics` | Mission orchestration and time integration. The main loop updates aircraft motion, propulsion demand, energy stores, and thermal state. |
| `physics::atmosphere` | Atmospheric properties, wind profiles, and unit conversion constants. |
| `physics::mass` | Aircraft mass build-up, energy storage allocation, and mass and volume constraints. |
| `physics::transitions` | Flight phase transition rules based on the current state and phase limits. |
| `sizing_lp` | Constructs candidate aircraft configurations and evaluates them through the mission simulator. Hybrid candidates start from fixed phase splits. The module name is historical. |
| `optimizer` | Searches climb, cruise, and descent power splits for a given scenario. |
| `generate_battery_frontier` | Runs battery payload and range studies using the simulation library and writes a local CSV. |

## How the modules work together

1. A mission begins with a `SimulationInput`. The simulator applies the selected technology assumptions and uses the mass model to initialize the aircraft.
2. During each timestep, atmospheric and wind calculations inform the aircraft state update. Phase transition rules determine when the mission advances. The simulator records the profile and returns energy and fuel totals, mass metrics, and a termination reason.
3. When mission sizing is enabled for a battery or hydrogen store, the simulator estimates capacity from an initial run, then reruns the mission with the estimated capacity. Changing storage capacity can change aircraft mass and mission demand.
4. Aircraft sizing and hybrid optimization both call the same simulator. They compare successful candidates using net battery energy plus the lower heating value of consumed fuel and hydrogen. Hybrid optimization returns the original input when no candidate succeeds.

When changing a model assumption, record its source and units and compare the resulting mission against a reproducible baseline. See [CONTRIBUTING.md](../CONTRIBUTING.md).
