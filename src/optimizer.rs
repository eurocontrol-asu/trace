use crate::model::SimulationInput;
use crate::physics::build_profile_dynamic;

pub fn optimize_hybridization(base_input: &SimulationInput) -> SimulationInput {
    let mut best_input = base_input.clone();
    let mut best_score = f64::NEG_INFINITY;

    // Grid search for alphas (0.0 to 1.0 with step 0.25)
    let steps = [0.0, 0.25, 0.5, 0.75, 1.0];

    for &a_climb in &steps {
        for &a_cruise in &steps {
            for &a_descent in &steps {
                let mut test_input = base_input.clone();
                test_input.alpha_climb = a_climb;
                test_input.alpha_cruise = a_cruise;
                test_input.alpha_descent = a_descent;

                let result = build_profile_dynamic(&test_input);

                if result.end_reason != "success" {
                    continue;
                }

                let score = -result.mission_energy_kwh_equivalent();
                if score > best_score {
                    best_score = score;
                    best_input = test_input;
                }
            }
        }
    }

    best_input
}
