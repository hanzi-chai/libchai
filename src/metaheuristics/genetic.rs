//! 遗传算法

use crate::{objectives::metric::Metric, problems::{Problem, Solution}, Interface};

use super::Metaheuristic;

pub struct Genetic {
    pub population_size: usize,
    pub generations: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
}

impl Metaheuristic for Genetic {
    fn solve(&self, _problem: &mut dyn Problem, _interface: &dyn Interface) -> (Solution, Metric, f64) {
        todo!()
    }
}
