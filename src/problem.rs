use std::fs;

use crate::config::{KeyMap, Config, MetaheuristicConfig};
use crate::constraints::Constraints;
use crate::encoder::Encoder;
use crate::metaheuristics::{Metaheuristics, simulated_annealing, hill_climbing};
use crate::objective::{Objective, Metric};
use rand::random;
use time::Duration;
use chrono::Local;
use linked_hash_map::LinkedHashMap;
use yaml_rust::{YamlEmitter, Yaml};

// 未来可能会有更加通用的解定义
type Solution = KeyMap;

pub struct ElementPlacementProblem {
    initial: Solution,
    constraints: Constraints,
    objective: Objective,
    encoder: Encoder,
}

impl ElementPlacementProblem {
    pub fn new(
        config: &Config,
        constraints: Constraints,
        objective: Objective,
        encoder: Encoder,
    ) -> Self {
        Self {
            initial: config.form.mapping.clone(),
            constraints,
            objective,
            encoder,
        }
    }
}

impl Metaheuristics<Solution, Metric> for ElementPlacementProblem {
    fn clone_candidate(&mut self, candidate: &Solution) -> Solution {
        return candidate.clone();
    }

    fn generate_candidate(&mut self) -> Solution {
        return self.initial.clone();
    }

    fn rank_candidate(&mut self, candidate: &Solution) -> (Metric, f64) {
        let (character_codes, word_codes) = self.encoder.encode(candidate);
        let (metric, loss) = self.objective.evaluate(&character_codes, &word_codes);
        return (metric, loss);
    }

    fn tweak_candidate(&mut self, candidate: &Solution) -> Solution {
        if random::<f64>() < 0.9 {
            self.constraints.constrained_random_move(candidate)
        } else {
            self.constraints.constrained_random_swap(candidate)
        }
    }

    fn save_candidate(&self, candidate: &Solution, rank: &(Metric, f64)) {
        let _ = fs::create_dir_all("output").expect("should be able to create an output directory");
        let time = Local::now();
        let prefix = format!("{}", time.format("%Y-%m-%d+%H_%M_%S"));
        let config_path = format!("output/{}.patch.yaml", prefix);
        let metric_path = format!("output/{}.txt", prefix);
        fs::write(metric_path, format!("{}", rank.0)).unwrap();
        let mut map: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        for (key, value) in candidate {
            map.insert(Yaml::String(key.to_string()), Yaml::String(value.to_string()));
        }
        let map = Yaml::Hash(map);
        let mut patch: LinkedHashMap<Yaml, Yaml> = LinkedHashMap::new();
        patch.insert(Yaml::String("form/mapping".to_string()), map);
        let yaml = Yaml::Hash(patch);
        let mut dump = String::new();
        let mut emitter = YamlEmitter::new(&mut dump);
        emitter.dump(&yaml).unwrap();
        fs::write(config_path, dump).unwrap();
    }
}

pub fn generic_solve(config: &Config, problem: &mut ElementPlacementProblem, runtime: Duration) -> Solution {
    match config.optimization.metaheuristic {
        MetaheuristicConfig::SimulatedAnnealing => {
            simulated_annealing::solve(problem, runtime)
        }
        MetaheuristicConfig::HillClimbing => {
            hill_climbing::solve(problem, runtime)
        }
    }
}
