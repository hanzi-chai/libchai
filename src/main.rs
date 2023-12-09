mod io;
use io::{read_hashmap_from_file, dump_hashmap_to_file};
use problem::ElementPlacementProblem;
use std::{collections::HashMap, convert::identity};
use time::Duration;

mod metrics;
use metrics::{merge_elements_and_frequency, RankedData, MetricWeights,
};

mod mutators;
mod problem;

fn parse_element_list(raw: String) -> Vec<String> {
    raw.chars().map(|x| x.to_string()).collect()
}

pub struct Assets {
    characters: Vec<RankedData>,
    words: Vec<RankedData>,
    equivalence: HashMap<String, f64>,
}

fn preprocess() -> Assets {
    let character_elements =
        read_hashmap_from_file("assets/character_elements.txt", parse_element_list);
    let character_frequency = read_hashmap_from_file("assets/character_frequency.txt", |x| {
        x.parse::<i32>().unwrap()
    });
    let characters = merge_elements_and_frequency(&character_elements, &character_frequency);
    let word_elements = read_hashmap_from_file("assets/word_elements.txt", parse_element_list);
    let word_frequency =
        read_hashmap_from_file("assets/word_frequency.txt", |x| x.parse::<i32>().unwrap());
    let words = merge_elements_and_frequency(&word_elements, &word_frequency);
    let equivalence =
        read_hashmap_from_file("assets/equivalence.txt", |x| x.parse::<f64>().unwrap());
    Assets {
        characters,
        words,
        equivalence,
    }
}

fn main() {
    let initial = read_hashmap_from_file("assets/map.txt", identity);
    let fixed_map = read_hashmap_from_file("assets/fixed_map.txt", identity);
    let assets = preprocess();
    let mutable_keys: Vec<String> = initial
        .iter()
        .map(|(k, _)| k.to_string())
        .filter(|k| fixed_map.get(k).is_some())
        .collect();
    let adjoint_metric = MetricWeights::new();
    let mut problem = ElementPlacementProblem::new(initial, mutable_keys, assets, adjoint_metric);
    let runtime = Duration::new(10, 0);
    let solution = metaheuristics::hill_climbing::solve(&mut problem, runtime);
    dump_hashmap_to_file("solution.txt", &solution);
}
