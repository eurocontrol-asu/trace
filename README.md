# TRACE

TRACE (Techno-Economic Regional Aircraft Concept Explorer) is a Rust engine for exploring regional aircraft concepts. This edition models flight missions, propulsion, energy use, aircraft sizing, and mission feasibility. Use it for conceptual studies and comparisons. Results require independent validation for operational or certification decisions.

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
