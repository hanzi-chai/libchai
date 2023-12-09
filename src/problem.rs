use std::collections::HashMap;

use crate::{mutators::random_mutate, Assets, metrics::{evaluate, Metric, MetricWeights}};
use metaheuristics::Metaheuristics;

type Solution = HashMap<String, String>;

pub struct ElementPlacementProblem {
    initial: Solution,
    mutable_keys: Vec<String>,
    assets: Assets,
    adjoint_metric: MetricWeights,
}

impl ElementPlacementProblem {
    pub fn new(initial: Solution, mutable_keys: Vec<String>, assets: Assets, adjoint_metric: MetricWeights) -> Self {
        Self { initial, mutable_keys, assets, adjoint_metric }
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
        let characters_result = evaluate(
            &self.assets.characters,
            &candidate,
            &self.assets.equivalence,
            &vec![1500_usize],
        );
        let words_result = evaluate(
            &self.assets.words,
            &candidate,
            &self.assets.equivalence,
            &vec![2000_usize, 10000_usize],
        );
        let metric = Metric { characters: characters_result, words: words_result};
        return metric.real_value(&self.adjoint_metric);
    }

    fn tweak_candidate(&mut self, candidate: &Solution) -> Solution {
        let next = random_mutate(candidate, &self.mutable_keys);
        return next;
    }
}
