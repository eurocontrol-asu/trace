use trace_engine::{model::SimulationInput, physics::build_profile_dynamic};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let input = match args.next() {
        Some(path) if args.next().is_none() => {
            serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(path)?))?
        }
        None => SimulationInput {
            stage_nm: 80.0,
            ..SimulationInput::default()
        },
        Some(_) => return Err("Usage: simulate [mission.json]".into()),
    };
    let result = build_profile_dynamic(&input);
    println!("Mission status: {}", result.end_reason);
    println!("Distance: {:.1} NM", result.distance_nm);
    if let Some(detail) = result.end_reason_detail {
        println!("{detail}");
    }
    Ok(())
}
