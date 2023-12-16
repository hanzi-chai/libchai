use crate::{
    cli::Assets,
    config::{Config, KeyMap, WordRule},
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};
use serde::{Serialize, Deserialize};

// 支持二字词直到十字词
const MAX_WORD_LENGTH: usize = 10;

pub type RawElements = HashMap<char, Vec<String>>;
pub type Elements = HashMap<char, Vec<usize>>;
pub type RankedElements<T> = (T, Vec<usize>, u64);

#[derive(Debug)]
pub struct Encoder {
    characters: Vec<RankedElements<char>>,
    words: Vec<RankedElements<String>>,
    auto_select_length: usize,
    pub max_length: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Encoded<T> {
    pub original: Option<T>,
    pub code: String,
    pub frequency: u64,
}

pub type Code<T> = Vec<Encoded<T>>;

#[derive(Debug)]
pub struct EncodeResults {
    pub characters: Option<Code<char>>,
    pub characters_reduced: Option<Code<char>>,
    pub words: Option<Code<String>>,
    pub words_reduced: Option<Code<String>>,
}

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
        let default_rules: Vec<WordRule> = vec![
            WordRule::EqualRule {
                length_equal: 2,
                formula: String::from("AaAbBaBb"),
            },
            WordRule::EqualRule {
                length_equal: 3,
                formula: String::from("AaBaCaCb"),
            },
            WordRule::RangeRule {
                length_in_range: (4, 20),
                formula: String::from("AaBaCaZa"),
            },
        ];
        for i in 2..=MAX_WORD_LENGTH {
            // 尝试从规则列表中找到一个能符合当前长度的规则
            let mut one_matched = false;
            for rule in config.encoder.rules.as_ref().unwrap_or(&default_rules) {
                let (is_matched, formula) = match rule {
                    WordRule::EqualRule {
                        formula,
                        length_equal,
                    } => (*length_equal == i, formula),
                    WordRule::RangeRule {
                        formula,
                        length_in_range,
                    } => (length_in_range.0 <= i && length_in_range.1 >= i, formula),
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
        let mut characters: Vec<RankedElements<char>> = elements
            .iter()
            .map(|(k, v)| (*k, v.clone(), *assets.character_frequency.get(k).unwrap_or(&0)))
            .collect();
        characters.sort_by(|a, b| b.2.cmp(&a.2));
        // 词库中的字可能并没有在拆分表中
        let lookup = Self::build_lookup(config);
        let mut words: Vec<RankedElements<String>> = Vec::new();
        for (word, freq) in &assets.word_frequency {
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
                words.push((word.clone(), word_elements, *freq));
            }
        }
        words.sort_by(|a, b| b.2.cmp(&a.2));
        Encoder {
            characters,
            words,
            auto_select_length: config.encoder.auto_select_length.unwrap_or(0),
            max_length: config.encoder.max_length.unwrap_or(std::usize::MAX),
        }
    }

    pub fn encode_full<T: Clone>(&self, keymap: &KeyMap, data: &Vec<RankedElements<T>>, with_original: bool) -> Code<T> {
        let mut codes: Code<T> = Vec::new();
        for (original, elements, frequency) in data {
            let mut code = String::new();
            for element in elements {
                code.push(keymap[*element]);
            }
            if code.len() < self.auto_select_length {
                code.push('_');
            }
            let wrapped = if with_original { Some(original.clone()) } else { None };
            codes.push(Encoded { original: wrapped, code, frequency: *frequency });
        }
        return codes;
    }

    pub fn encode_reduced<T: Clone>(&self, full_code: &Code<T>) -> Code<T> {
        let mut reduced_codes: Code<T> = Vec::new();
        let mut known_reduced_codes: HashSet<String> = HashSet::new();
        for Encoded { original, code, frequency } in full_code {
            let mut has_reduced = false;
            for i in 1..code.len() {
                let mut reduced_code = code[0..i].to_string();
                if reduced_code.len() < self.auto_select_length {
                    reduced_code.push('_');
                }
                if !known_reduced_codes.contains(&reduced_code) {
                    known_reduced_codes.insert(reduced_code.clone());
                    reduced_codes.push(Encoded { original: original.clone(), code: reduced_code, frequency: *frequency });
                    has_reduced = true;
                    break;
                }
            }
            if has_reduced == false {
                reduced_codes.push(Encoded { original: original.clone(), code: code.clone(), frequency: *frequency });
            }
        }
        reduced_codes
    }

    pub fn encode_character_full(&self, keymap: &KeyMap, with_original: bool) -> Code<char> {
        self.encode_full(keymap, &self.characters, with_original)
    }

    pub fn encode_words_full(&self, keymap: &KeyMap, with_original: bool) -> Code<String> {
        self.encode_full(keymap, &self.words, with_original)
    }

    fn signed_index<T: Debug>(vector: &Vec<T>, index: isize) -> &T {
        return if index >= 0 {
            &vector[index as usize]
        } else {
            &vector[vector.len() - (-index as usize)]
        };
    }
}
