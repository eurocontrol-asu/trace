# TRACE

TRACE (Techno-Economic Regional Aircraft Concept Explorer) is a Rust engine for simulating flight missions and sizing regional aircraft concepts.

TRACE uses a **stateful mission simulation**. At each timestep, it uses the aircraft’s current speed, altitude, remaining energy, and temperatures to compute the next state. The effects of earlier flight phases carry forward. For example, energy used during climb reduces what is available for cruise and any subsequent hold.

This lets you assess whether a concept can complete a mission under the chosen assumptions, see where it reaches a limit, and explore how changes to the aircraft or mission affect the result. TRACE is intended for conceptual studies and comparisons. Results require independent validation for operational or certification decisions.

## Run a mission

Install Rust with Cargo, then run the example from this directory.

```sh
cargo run --locked --example simulate
```

The example uses `SimulationInput` to describe a mission and calls
`trace_engine::physics::build_profile_dynamic`. The returned
`SimulationResult` contains the flight profile, energy and fuel totals,
mass breakdown, and termination status. Inspect `end_reason` and
`end_reason_detail` to understand how the simulation ended.

Use the [mission input guide](docs/inputs.md) to configure aircraft, propulsion, and flight conditions.

See [the engine guide](docs/engine.md) for module boundaries and [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution workflow.

## License

TRACE is licensed under the [European Union Public Licence v1.2](LICENSE) (EUPL-1.2).
