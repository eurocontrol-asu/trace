# TRACE

TRACE (Techno-Economic Regional Aircraft Concept Explorer) is a Rust engine for simulating flight missions and sizing regional aircraft concepts.

TRACE uses a **stateful mission simulation**. At each timestep, it uses the aircraft’s current speed, altitude, remaining energy, and temperatures to compute the next state. The effects of earlier flight phases carry forward. For example, energy used during climb reduces what is available for cruise and any subsequent hold.

This lets you assess whether a concept can complete a mission under the chosen assumptions, see where it reaches a limit, and explore how changes to the aircraft or mission affect the result. TRACE is intended for conceptual studies and comparisons. Results require independent validation for operational or certification decisions.

## Quickstart

Install Git and Rust with Cargo. Then clone TRACE and run the included mission example.

```sh
git clone https://github.com/eurocontrol-asu/trace.git
cd trace
cargo run --locked --example simulate
```

The example sets `stage_nm` to 80 nautical miles and uses the default aircraft and flight settings for the other inputs. The run prints:

```text
Mission status: success
Distance: 90.2 NM
```

`success` means the simulated mission completed under those assumptions. `Distance` is the distance accumulated along the simulated flight profile and can differ from the input stage length. If the mission ends early, the example prints the reason when available.

### Change the mission inputs

The [20-seat liquid hydrogen fuel cell example](examples/fc-20seat-lh2-300nm.json) lists every mission input in the public Rust model. It captures the physical settings of the TRACE frontend’s 300 NM startup scenario. Copy it to your own file, edit the values, and pass its path after `--`.

```sh
cp examples/fc-20seat-lh2-300nm.json my-mission.json
cargo run --locked --example simulate -- my-mission.json
```

For example, edit `stage_nm` to change the route length in nautical miles. Keep the complete file to preserve the scenario settings. If you omit a field, the runner uses the Rust library default for that field.

In Rust code, you can set the same fields directly on `SimulationInput`. See the [mission input guide](docs/inputs.md) for field meanings, units, and propulsion settings.

See [the engine guide](docs/engine.md) for module boundaries and [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution workflow.

## License

TRACE is licensed under the [European Union Public Licence v1.2](LICENSE) (EUPL-1.2).
