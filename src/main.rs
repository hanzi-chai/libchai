mod io;
use io::read_hashmap_from_file;
use mutators::random_mutate;
use std::{collections::HashMap, convert::identity};

mod metrics;
use metrics::{
    evaluate, make_dummy_result, merge_elements_and_frequency, RankedData,
};

use crate::{metrics::is_better, io::dump_hashmap_to_file};

mod mutators;

fn parse_element_list(raw: String) -> Vec<String> {
    raw.chars().map(|x| x.to_string()).collect()
}

pub struct Assets {
    characters: Vec<RankedData>,
    words: Vec<RankedData>,
    equivalence: HashMap<String, f64>,
}

pub fn tanxin_optimize(
    initial: &HashMap<String, String>,
    mutatable_keys: &Vec<String>,
    assets: &Assets,
) {
    let mut best_combined_result = (make_dummy_result(1), make_dummy_result(2));
    let mut best_map = initial.clone();
    let mut j = 0;
    while j < 100 {
        let next = random_mutate(&best_map, mutatable_keys);
        let characters_result = evaluate(
            &assets.characters,
            &next,
            &&assets.equivalence,
            &vec![1500_usize],
        );
        let words_result = evaluate(
            &&assets.words,
            &next,
            &assets.equivalence,
            &vec![2000_usize, 10000_usize],
        );
        let combined_result = (characters_result, words_result);
        if is_better(&combined_result, &best_combined_result) {
            best_combined_result = combined_result;
            best_map = next;
            println!("{:?}", &best_combined_result);
            let name = format!("output/{}.txt", j);
            dump_hashmap_to_file(&name, &best_map);
        }
        j += 1;
    }
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
    let map = read_hashmap_from_file("assets/map.txt", identity);
    let fixed_map = read_hashmap_from_file("assets/fixed_map.txt", identity);
    let assets = preprocess();
    let mutatable_keys: Vec<String> = map
        .iter()
        .map(|(k, _)| k.to_string())
        .filter(|k| fixed_map.get(k).is_some())
        .collect();
    tanxin_optimize(&map, &mutatable_keys, &assets);
}
