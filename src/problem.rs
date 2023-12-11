use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};

use crate::config::KeyMap;
use crate::constraints::Constraints;
use crate::encoder::Encoder;
use crate::metaheuristics::Metaheuristics;
use crate::objective::Objective;

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
        initial: Solution,
        constraints: Constraints,
        objective: Objective,
        encoder: Encoder,
    ) -> Self {
        Self {
            initial,
            constraints,
            objective,
            encoder,
        }
    }

    pub fn write_solution(name: &str, solution: &Solution) {
        let _ = fs::create_dir_all("output").expect("should be able to create an output directory");
        let path = String::from("output") + name;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("Unable to create file");
        let mut writer = BufWriter::new(file);
        for (key, value) in solution {
            writeln!(&mut writer, "{}\t{}", key, value).expect("Unable to write to file");
        }
    }
}

impl Metaheuristics<Solution> for ElementPlacementProblem {
    fn clone_candidate(&mut self, candidate: &Solution) -> Solution {
        return candidate.clone();
    }

    fn generate_candidate(&mut self) -> Solution {
        return self.initial.clone();
    }

    fn rank_candidate(&mut self, candidate: &Solution) -> f64 {
        let (character_codes, word_codes) = self.encoder.encode(candidate);
        return self.objective.evaluate(&character_codes, &word_codes);
    }

    fn tweak_candidate(&mut self, candidate: &Solution) -> Solution {
        let next = self.constraints.constrained_random_move(candidate);
        return next;
    }
}
