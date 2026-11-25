# Contributing to TRACE Engine

Build the project with `cargo build --locked` and run the existing checks with `cargo test --locked`. The README shows how to run a mission locally.

For a bug report, include the source revision, Rust version, mission input, observed result, and expected behavior. Share a minimal scenario with data you have permission to redistribute.

For a model change, document the equation or assumption, units, references, and the range in which you expect it to apply. Include a reproducible comparison and explain changes in feasibility, phase transitions, energy use, or sizing. For API changes, describe the effect on callers and update the engine guide when needed.
