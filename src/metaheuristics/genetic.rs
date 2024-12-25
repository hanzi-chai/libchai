//! 遗传算法

use crate::{problem::{Problem, Solution}, Interface};

use super::Metaheuristic;

pub struct Genetic {
    pub population_size: usize,
    pub generations: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
}

impl Metaheuristic for Genetic {
    fn solve(&self, problem: &mut Problem, interface: &dyn Interface) -> Solution {
        todo!()
    }
}
