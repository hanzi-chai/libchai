//! 优化问题的求解方法。
//!

use crate::{
    config::SolverConfig,
    interface::Interface,
    problem::{Problem, Solution},
};
pub mod simulated_annealing;

pub trait Metaheuristic {
    fn solve(&self, problem: &mut Problem, interface: &dyn Interface) -> Solution;
}

impl SolverConfig {
    pub fn solve<I: Interface>(&self, problem: &mut Problem, interface: &I) {
        match self {
            SolverConfig::SimulatedAnnealing(sa) => {
                sa.solve(problem, interface);
            }
        }
    }
}
