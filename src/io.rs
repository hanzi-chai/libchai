use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use crate::config::{KeyMap, Config};

pub fn read_keymap(name: &str) -> (KeyMap, Vec<String>) {
    let mut keymap: KeyMap = HashMap::new();
    let mut mutable_keys: Vec<String> = Vec::new();
    let file = File::open(name).expect("Failed to open file");
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.expect("cannot read line");
        let fields: Vec<&str> = line.trim().split('\t').collect();
        let key: Vec<char>= fields[1].chars().collect();
        keymap.insert(fields[0].to_string(), key[0]);
        if fields.len() > 2 {
            mutable_keys.push(fields[0].to_string());
        }
    }
    (keymap, mutable_keys)
}

pub fn read_hashmap_from_file<T>(name: &str, parser: fn(String) -> T) -> HashMap<String, T> {
    let mut keymap: HashMap<String, T> = HashMap::new();
    let file = File::open(name).expect("Failed to open file");
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.expect("cannot read line");
        let fields: Vec<&str> = line.trim().split('\t').collect();
        keymap.insert(fields[0].to_string(), parser(fields[1].to_string()));
    }
    keymap
}

pub struct Assets {
    pub characters: HashMap<String, i32>,
    pub words: HashMap<String, i32>,
    pub equivalence: HashMap<String, f64>,
}

pub fn preprocess() -> Assets {
    let character_frequency = read_hashmap_from_file("assets/character_frequency.txt", |x| {
        x.parse::<i32>().unwrap()
    });
    let word_frequency =
        read_hashmap_from_file("assets/word_frequency.txt", |x| x.parse::<i32>().unwrap());
    let equivalence =
        read_hashmap_from_file("assets/equivalence.txt", |x| x.parse::<f64>().unwrap());
    Assets {
        characters: character_frequency,
        words: word_frequency,
        equivalence,
    }
}

pub type Elements = HashMap<char, Vec<String>>;

pub fn read_and_simplify_elements(name: &PathBuf, config: &Config) -> Elements {
    let mut elements_map: Elements = HashMap::new();
    let file = File::open(name).expect("Failed to open file");
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.expect("cannot read line");
        let fields: Vec<&str> = line.trim().split('\t').collect();
        let chars: Vec<char> = fields[0].to_string().chars().collect();
        let elements: Vec<String> = fields[1].to_string().split(' ').map(|x| config.form.grouping.get(x).unwrap_or(&x.to_string()).to_string()).collect();
        elements_map.insert(chars[0], elements);
    }
    return elements_map;
}
