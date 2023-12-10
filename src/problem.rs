use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::config::KeyMap;
use crate::encoder::Encoder;
use crate::io::{Assets, Elements};
use crate::mutators::random_mutate;
use crate::metrics::{evaluate, Metric, MetricWeights};
use metaheuristics::Metaheuristics;

// 未来可能会有更加通用的解定义
type Solution = KeyMap;

pub struct ElementPlacementProblem {
    initial: Solution,
    mutable_keys: Vec<String>,
    elements: Elements,
    assets: Assets,
    weights: MetricWeights,
    encoder: Encoder
}

impl ElementPlacementProblem {
    pub fn new(initial: Solution, mutable_keys: Vec<String>, assets: Assets, elements: Elements, weights: MetricWeights, encoder: Encoder) -> Self {
        Self { initial, mutable_keys, elements, assets, weights, encoder }
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
        let character_codes = self.encoder.encode_characters(&self.elements, candidate);
        let word_list: Vec<String> = self.assets.words.keys().map(|x| x.to_string()).collect();
        let word_codes: HashMap<String, String> = self.encoder.encode_words(&character_codes, &word_list);
        let characters_result = evaluate(
            &character_codes,
            &self.assets.characters,
            &self.assets.equivalence,
            &vec![1500_usize],
        );
        let words_result = evaluate(
            &word_codes,
            &self.assets.words,
            &self.assets.equivalence,
            &vec![2000_usize, 10000_usize],
        );
        let metric = Metric { characters: characters_result, words: words_result};
        return metric.real_value(&self.weights);
    }

    fn tweak_candidate(&mut self, candidate: &Solution) -> Solution {
        let next = random_mutate(candidate, &self.mutable_keys);
        return next;
    }
}
