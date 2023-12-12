use std::fs;
use crate::config::{KeyMap, Config, MetaheuristicConfig};
use crate::constraints::Constraints;
use crate::encoder::Encoder;
use crate::metaheuristics::{Metaheuristics, simulated_annealing, hill_climbing};
use crate::objective::Objective;
use chrono::prelude::*;
use rand::random;
use time::Duration;

// 未来可能会有更加通用的解定义
type Solution = KeyMap;

pub struct ElementPlacementProblem {
    config: Config,
    constraints: Constraints,
    objective: Objective,
    encoder: Encoder,
}

impl ElementPlacementProblem {
    pub fn new(
        config: Config,
        constraints: Constraints,
        objective: Objective,
        encoder: Encoder,
    ) -> Self {
        Self {
            config,
            constraints,
            objective,
            encoder,
        }
    }
}

impl Metaheuristics<Solution> for ElementPlacementProblem {
    fn clone_candidate(&mut self, candidate: &Solution) -> Solution {
        return candidate.clone();
    }

    fn generate_candidate(&mut self) -> Solution {
        return self.config.form.mapping.clone();
    }

    fn rank_candidate(&mut self, candidate: &Solution) -> f64 {
        let (character_codes, word_codes) = self.encoder.encode(candidate);
        let (_, loss) = self.objective.evaluate(&character_codes, &word_codes);
        return loss;
    }

    fn tweak_candidate(&mut self, candidate: &Solution) -> Solution {
        if random::<f64>() < 0.9 {
            self.constraints.constrained_random_move(candidate)
        } else {
            self.constraints.constrained_random_swap(candidate)
        }
    }

    fn save_candidate(&self, candidate: &Solution) {
        let _ = fs::create_dir_all("output").expect("should be able to create an output directory");
        let time = Local::now();
        let prefix = format!("{}", time.format("%Y-%m-%d+%H_%M_%S"));
        let config_path = format!("output/{}.config.yaml", prefix);
        let metric_path = format!("output/{}.txt", prefix);
        let mut new_config = self.config.clone();
        for key in self.config.form.mapping.keys() {
            new_config.form.mapping.insert(key.clone(), *candidate.get(key).unwrap());
        }
        new_config.write_config(&config_path);
        let (character_codes, word_codes) = self.encoder.encode(candidate);
        let (metric, _) = self.objective.evaluate(&character_codes, &word_codes);
        fs::write(metric_path, format!("{}", metric)).unwrap();
    }
}

pub fn generic_solve(problem: &mut ElementPlacementProblem, runtime: Duration) -> Solution {
    match problem.config.optimization.metaheuristic {
        MetaheuristicConfig::SimulatedAnnealing => {
            simulated_annealing::solve(problem, runtime)
        }
        MetaheuristicConfig::HillClimbing => {
            hill_climbing::solve(problem, runtime)
        }
    }
}
