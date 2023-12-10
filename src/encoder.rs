use crate::{config::{Config, WordRule, KeyMap}, io::Elements};
use std::{collections::HashMap, fmt::Debug};

const MAX_WORD_LENGTH: usize = 20;

#[derive(Debug)]
pub struct Encoder {
    // 支持二字词直到十字词
    auto_select_length: usize,
    quick_lookup: [Vec<(isize, isize)>; MAX_WORD_LENGTH - 1],
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

    pub fn new(config: &Config) -> Encoder {
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
        Encoder { auto_select_length: config.encoder.auto_select_length, quick_lookup }
    }

    pub fn encode_characters(
        &self,
        character_elements: &Elements,
        keymap: &KeyMap,
    ) -> HashMap<String, String> {
        let mut codes: HashMap<String, String> = HashMap::new();
        for (key, elements) in character_elements {
            let mut code = String::new();
            for element in elements {
                if let Some(key) = keymap.get(element) {
                    code.push(*key);
                }
            }
            if code.len() < self.auto_select_length {
                code.push('_');
            }
            codes.insert(key.to_string(), code);
        }
        return codes;
    }

    fn negative_index<T: Debug>(vector: &Vec<T>, index: isize) -> &T {
        return if index >= 0 {
            &vector[index as usize]
        } else {
            &vector[vector.len() - (-index as usize)]
        };
    }

    pub fn encode_words(
        &self,
        character_codes: &HashMap<String, String>,
        word_list: &Vec<String>,
    ) -> HashMap<String, String> {
        let mut codes: HashMap<String, String> = HashMap::new();
        for word in word_list {
            let characters: Vec<char> = word.chars().collect();
            let length = characters.len();
            let rule = &self.quick_lookup[length - 2];
            let mut code = String::new();
            for (chi, coi) in rule {
                let character = Self::negative_index(&characters, *chi);
                if let Some(character_code) = character_codes.get(&character.to_string()) {
                    let keys = character_code.clone().chars().collect();
                    let key = Self::negative_index(&keys, *coi);
                    code.push(*key);
                }
            }
            codes.insert(word.to_string(), code);
        }
        return codes;
    }
}
