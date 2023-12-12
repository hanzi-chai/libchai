use crate::{
    assets::Assets,
    config::{Config, KeyMap, WordRule},
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    fs::File,
    io::BufRead,
    io::BufReader
};

const MAX_WORD_LENGTH: usize = 20;

pub type Entry = (char, Vec<String>);
pub type Elements = Vec<Entry>;
#[derive(Debug)]
pub struct Encoder {
    characters: Vec<Entry>,
    words: Vec<String>,
    // 支持二字词直到十字词
    auto_select_length: usize,
    quick_lookup: [Vec<(isize, isize)>; MAX_WORD_LENGTH - 1],
}

pub type Code<T> = HashMap<T, String>;

impl Encoder {
    fn parse_formula(s: &String) -> Vec<(isize, isize)> {
        let mut ret: Vec<(isize, isize)> = Vec::new();
        let chars: Vec<char> = s.chars().collect();
        assert!(chars.len() % 2 == 0);
        let pairs = chars.len() / 2;
        let normalize = |x: isize| if x > 13 { x - 26 } else { x };
        for i in 0..pairs {
            let character_symbol = chars[2 * i];
            let code_symbol = chars[2 * i + 1];
            let character_index = normalize((character_symbol as isize) - ('A' as isize));
            let code_index = normalize((code_symbol as isize) - ('a' as isize));
            ret.push((character_index, code_index));
        }
        ret
    }

    pub fn new(config: &Config, mut characters: Elements, assets: &Assets) -> Encoder {
        let words: Vec<String> = assets.words.keys().map(|x| x.to_string()).collect();
        let cf = &assets.characters;
        characters.sort_by(|a, b| cf.get(&a.0).unwrap_or(&0).cmp(&cf.get(&b.0).unwrap_or(&0)));
        let mut quick_lookup: [Vec<(isize, isize)>; MAX_WORD_LENGTH - 1] = Default::default();
        for i in 2..=MAX_WORD_LENGTH {
            // 尝试从规则列表中找到一个能符合当前长度的规则
            let mut one_matched = false;
            for rule in &config.encoder.rules {
                let (is_matched, formula) = match rule {
                    WordRule::EqualRule {
                        length_equal,
                        formula,
                    } => (*length_equal == i, formula),
                    WordRule::RangeRule {
                        length_in_range,
                        formula,
                    } => (length_in_range[0] <= i && length_in_range[1] >= i, formula),
                };
                if is_matched {
                    one_matched = true;
                    quick_lookup[(i - 2) as usize] = Self::parse_formula(formula);
                    break;
                }
            }
            if !one_matched {
                panic!("没有找到造 {} 字词的规则", i);
            }
        }
        Encoder {
            characters,
            words,
            auto_select_length: config.encoder.auto_select_length,
            quick_lookup,
        }
    }

    pub fn encode_characters(&self, keymap: &KeyMap) -> Code<char> {
        let mut codes: Code<char> = HashMap::new();
        for (key, elements) in &self.characters {
            let mut code = String::new();
            for element in elements {
                if let Some(key) = keymap.get(element) {
                    code.push(*key);
                }
            }
            if code.len() < self.auto_select_length {
                code.push('_');
            }
            codes.insert(*key, code);
        }
        return codes;
    }

    pub fn encode_characters_reduced(&self, character_codes: &Code<char>) -> Code<char> {
        let mut reduced_codes: Code<char> = HashMap::new();
        let mut known_reduced_codes: HashSet<String> = HashSet::new();
        for (character, code) in character_codes {
            for i in 1..code.len() {
                let mut reduced_code = code[0..i].to_string();
                if reduced_code.len() < self.auto_select_length {
                    reduced_code.push('_');
                }
                if let None = known_reduced_codes.get(&reduced_code) {
                    reduced_codes.insert(*character, reduced_code.clone());
                    known_reduced_codes.insert(reduced_code);
                    break;
                }
                if i + 1 == code.len() {
                    reduced_codes.insert(*character, code.to_string());
                }
            }
        }
        reduced_codes
    }

    fn signed_index<T: Debug>(vector: &Vec<T>, index: isize) -> &T {
        return if index >= 0 {
            &vector[index as usize]
        } else {
            &vector[vector.len() - (-index as usize)]
        };
    }

    pub fn encode_words(&self, character_codes: &Code<char>) -> Code<String> {
        let mut codes: Code<String> = HashMap::new();
        for word in &self.words {
            let characters: Vec<char> = word.chars().collect();
            let length = characters.len();
            let rule = &self.quick_lookup[length - 2];
            let mut code = String::new();
            for (chi, coi) in rule {
                let character = Self::signed_index(&characters, *chi);
                if let Some(character_code) = character_codes.get(character) {
                    let keys = character_code.clone().chars().collect();
                    let key = Self::signed_index(&keys, *coi);
                    code.push(*key);
                }
            }
            codes.insert(word.to_string(), code);
        }
        return codes;
    }

    pub fn encode(&self, keymap: &KeyMap) -> (Code<char>, Code<String>) {
        let character_codes = self.encode_characters(keymap);
        let word_codes = self.encode_words(&character_codes);
        (character_codes, word_codes)
    }
}

pub fn read_elements(name: &String) -> Vec<Entry> {
    let mut elements: Vec<Entry> = Vec::new();
    let file = File::open(name).expect("Failed to open file");
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.expect("cannot read line");
        let fields: Vec<&str> = line.trim().split('\t').collect();
        let char = fields[0].chars().next().unwrap();
        let elems = fields[1].split(' ').map(|x| x.to_string()).collect();
        elements.push((char, elems));
    }
    elements
}
