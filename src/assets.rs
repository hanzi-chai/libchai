use std::collections::HashMap;
use std::convert::identity;
use std::fs::File;
use std::io::{BufRead, BufReader};

pub type Equivalence = HashMap<(char, char), f64>;
pub type Frequency<T> = HashMap<T, usize>;

#[derive(Debug)]
pub struct Assets {
    pub characters: Frequency<char>,
    pub words: Frequency<String>,
    pub equivalence: Equivalence,
}

impl Assets {
    pub fn new() -> Assets {
        let frequency_parser = |x: String| x.parse::<usize>().unwrap();
        let equivalence_parser = |x: String| x.parse::<f64>().unwrap();
        let char_parser = |x: String| x.chars().next().unwrap();
        let char_pair_parser = |x: String| {
            let mut it = x.chars();
            let first = it.next().unwrap();
            let second = it.next().unwrap();
            (first, second)
        };
        let character_frequency = Self::read_hashmap_from_file(&String::from("assets/character_frequency.txt"), char_parser, frequency_parser);
        let word_frequency = Self::read_hashmap_from_file(&String::from("assets/word_frequency.txt"), identity, frequency_parser);
        let equivalence =
            Self::read_hashmap_from_file(&String::from("assets/equivalence.txt"), char_pair_parser, equivalence_parser);
        Assets {
            characters: character_frequency,
            words: word_frequency,
            equivalence,
        }
    }

    pub fn read_hashmap_from_file<T: Eq + std::hash::Hash, S>(
        name: &String,
        kparser: fn(String) -> T,
        vparser: fn(String) -> S,
    ) -> HashMap<T, S> {
        let mut hashmap: HashMap<T, S> = HashMap::new();
        let file = File::open(name).expect("Failed to open file");
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.expect("cannot read line");
            let fields: Vec<&str> = line.trim().split('\t').collect();
            hashmap.insert(kparser(fields[0].to_string()), vparser(fields[1].to_string()));
        }
        hashmap
    }
}
