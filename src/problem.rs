//! 优化问题的整体定义。
//! 
//! 目前只定义了最基础的元素布局问题，以后可能会定义更复杂的问题，如元素布局 + 元素选取等等。
//! 

use crate::config::{SolverConfig, SearchConfig};
use crate::constraints::Constraints;
use crate::interface::Interface;
use crate::metaheuristics::{simulated_annealing, Metaheuristics};
use crate::objectives::Objective;
use crate::objectives::metric::Metric;
use crate::representation::{Buffer, KeyMap, Representation};
use rand::random;

// 未来可能会有更加通用的解定义
type Solution = KeyMap;

pub struct ElementPlacementProblem {
    representation: Representation,
    constraints: Constraints,
    objective: Objective,
    buffer: Buffer,
}

impl ElementPlacementProblem {
    pub fn new(
        representation: Representation,
        constraints: Constraints,
        objective: Objective,
        buffer: Buffer,
    ) -> Self {
        Self {
            representation,
            constraints,
            objective,
            buffer,
        }
    }
}

impl Metaheuristics<Solution, Metric> for ElementPlacementProblem {
    fn clone_candidate(&mut self, candidate: &Solution) -> Solution {
        return candidate.clone();
    }

    fn generate_candidate(&mut self) -> Solution {
        return self.representation.initial.clone();
    }

    fn rank_candidate(&mut self, candidate: &Solution) -> (Metric, f64) {
        let (metric, loss) = self.objective.evaluate(candidate, &mut self.buffer).unwrap();
        return (metric, loss);
    }

    fn tweak_candidate(&mut self, candidate: &Solution) -> Solution {
        let method = self.representation.config.optimization.metaheuristic.search_method.as_ref().unwrap_or(&SearchConfig { random_move: 0.9, random_swap: 0.09, random_full_key_swap: 0.01 });
        let ratio1 = method.random_move / (method.random_move + method.random_swap + method.random_full_key_swap);
        let ratio2 = (method.random_move + method.random_swap) / (method.random_move + method.random_swap + method.random_full_key_swap);
        let randomnumber = random::<f64>();
        if randomnumber < ratio1 {
            self.constraints.constrained_random_move(candidate)
        } else if randomnumber < ratio2 {
            self.constraints.constrained_random_swap(candidate)
        } else {
            self.constraints.constrained_full_key_swap(candidate)
        }
    }

    fn save_candidate(&self, candidate: &Solution, rank: &(Metric, f64), write_to_file: bool, interface: &dyn Interface) {
        let new_config = self.representation.update_config(&candidate);
        let metric = format!("{}", rank.0);
        interface.report_solution(new_config, metric, write_to_file);
    }
}

impl ElementPlacementProblem {
    pub fn solve(&mut self, interface: &dyn Interface) -> Solution {
        interface.prepare_output();
        let SolverConfig { parameters, runtime, report_after, .. } = self
            .representation
            .config
            .optimization
            .metaheuristic
            .clone();
        if let Some(parameters) = parameters {
            simulated_annealing::solve(self, parameters.clone(), report_after, interface)
        } else {
            let runtime = runtime.unwrap_or(10);
            simulated_annealing::autosolve(self, runtime, report_after, interface)
        }
    }
}
