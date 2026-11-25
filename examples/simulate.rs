use trace_engine::{model::SimulationInput, physics::build_profile_dynamic};

fn main() {
    let input = SimulationInput {
        stage_nm: 80.0,
        ..SimulationInput::default()
    };
    let result = build_profile_dynamic(&input);
    println!("Mission status: {}", result.end_reason);
    println!("Distance: {:.1} NM", result.distance_nm);
    if let Some(detail) = result.end_reason_detail {
        println!("{detail}");
    }
}
