//! 优化问题的求解方法。
//!

use crate::{
    config::SolverConfig,
    interface::Interface,
    problem::{Problem, Solution},
};
pub mod simulated_annealing;

#[derive(Debug)]
pub struct Timer {
    pub encode_reset: u128,
    pub encode_init: u128,
    pub encode_assembly: u128,
    pub encode_short: u128,
    pub encode_duplicate: u128,
    pub objective_accumulate: u128,
    pub objective_accept: u128,
}

pub static mut TIMER: Timer = Timer {
    encode_reset: 0,
    encode_init: 0,
    encode_assembly: 0,
    encode_short: 0,
    encode_duplicate: 0,
    objective_accumulate: 0,
    objective_accept: 0,
};

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
