//! 遗传算法

use super::{Metaheuristic, Solution};
use crate::{problems::Problem, Interface};

pub struct Genetic {
    pub population_size: usize,
    pub generations: usize,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
}

impl Metaheuristic for Genetic {
    fn solve<P: Problem, I: Interface>(&self, _problem: &mut P, _interface: &I) -> Solution {
        todo!()
    }
}
