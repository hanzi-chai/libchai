use crate::{
    assets::Assets,
    config::{Config, KeyMap},
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

// 支持二字词直到十字词
const MAX_WORD_LENGTH: usize = 10;

pub type RawElements = HashMap<char, Vec<String>>;
pub type Elements = HashMap<char, Vec<usize>>;
pub type RankedElements = (Vec<usize>, usize);

#[derive(Debug)]
pub struct Encoder {
    characters: Vec<RankedElements>,
    words: Vec<RankedElements>,
    auto_select_length: usize,
    pub max_length: usize,
}

pub type Code = Vec<(String, usize)>;

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

    fn build_lookup(config: &Config) -> [Vec<(isize, isize)>; MAX_WORD_LENGTH - 1] {
        let mut quick_lookup: [Vec<(isize, isize)>; MAX_WORD_LENGTH - 1] = Default::default();
        for i in 2..=MAX_WORD_LENGTH {
            // 尝试从规则列表中找到一个能符合当前长度的规则
            let mut one_matched = false;
            for rule in &config.encoder.rules {
                let formula = &rule.formula;
                let is_matched = if let Some(length_equal) = rule.length_equal {
                    length_equal == i
                } else {
                    let length_in_range = &rule.length_in_range.clone().unwrap();
                    length_in_range[0] <= i && length_in_range[1] >= i
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
        quick_lookup
    }

    pub fn new(config: &Config, elements: Elements, assets: &Assets) -> Encoder {
        let mut characters: Vec<RankedElements> = elements.iter().map(
            |(k, v)| (v.clone(), *assets.characters.get(k).unwrap_or(&0))
        ).collect();
        characters.sort_by(|a, b| b.1.cmp(&a.1));
        // 词库中的字可能并没有在拆分表中
        let lookup = Self::build_lookup(config);
        let mut words: Vec<RankedElements> = Vec::new();
        for (word, freq) in &assets.words {
            let chars: Vec<char> = word.chars().collect();
            let rule_index = chars.len() - 2;
            if rule_index + 1 >= lookup.len() {
                continue;
            }
            let rule = &lookup[rule_index];
            let mut word_elements: Vec<usize> = Vec::new();
            let mut invalid_char = false;
            for (char_index, code_index) in rule {
                let c = Self::signed_index(&chars, *char_index);
                if let Some(ce) = elements.get(c) {
                    let value = Self::signed_index(ce, *code_index);
                    word_elements.push(value.clone());
                } else {
                    invalid_char = true;
                    break;
                }
            }
            if !invalid_char {
                words.push((word_elements, *freq));
            }
        }
        words.sort_by(|a, b| b.1.cmp(&a.1));
        Encoder {
            characters,
            words,
            auto_select_length: config.encoder.auto_select_length.unwrap_or(0),
            max_length: config.encoder.maxlength.unwrap_or(std::usize::MAX)
        }
    }

    pub fn encode_full(&self, keymap: &KeyMap, data: &Vec<RankedElements>) -> Code {
        let mut codes: Code = Vec::new();
        for (elements, freq) in data {
            let mut code = String::new();
            for element in elements {
                code.push(keymap[*element]);
            }
            if code.len() < self.auto_select_length {
                code.push('_');
            }
            codes.push((code, *freq));
        }
        return codes;
    }

    pub fn encode_reduced(&self, full_code: &Code) -> Code {
        let mut reduced_codes: Code = Vec::new();
        let mut known_reduced_codes: HashSet<String> = HashSet::new();
        for (code, freq) in full_code {
            let mut has_reduced = false;
            for i in 1..code.len() {
                let mut reduced_code = code[0..i].to_string();
                if reduced_code.len() < self.auto_select_length {
                    reduced_code.push('_');
                }
                if !known_reduced_codes.contains(&reduced_code) {
                    known_reduced_codes.insert(reduced_code.clone());
                    reduced_codes.push((reduced_code, *freq));
                    has_reduced = true;
                    break;
                }
            }
            if has_reduced == false {
                reduced_codes.push((code.clone(), *freq));
            }
        }
        reduced_codes
    }

    pub fn encode_character_full(&self, keymap: &KeyMap) -> Code {
        self.encode_full(keymap, &self.characters)
    }

    pub fn encode_words_full(&self, keymap: &KeyMap) -> Code {
        self.encode_full(keymap, &self.words)
    }

    fn signed_index<T: Debug>(vector: &Vec<T>, index: isize) -> &T {
        return if index >= 0 {
            &vector[index as usize]
        } else {
            &vector[vector.len() - (-index as usize)]
        };
    }
}
