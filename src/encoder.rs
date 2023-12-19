use crate::{
    cli::{Assets, RawSequenceMap},
    config::{Config, EncoderConfig, WordRule},
    representation::{Codes, KeyMap, Representation, Sequence, SequenceMap},
};
use std::{cmp::Reverse, fmt::Debug, iter::zip};

// 支持二字词直到十字词
const MAX_WORD_LENGTH: usize = 10;

#[derive(Debug)]
pub struct Encoder {
    pub characters: Vec<char>,
    characters_sequence: Vec<Sequence>,
    pub words: Vec<String>,
    words_sequence: Vec<Sequence>,
    config: EncoderConfig,
    pub radix: usize,
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
                length_in_range: (4, MAX_WORD_LENGTH),
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

    // 字需要提供拆分表，但是词只需要提供词表
    pub fn new(
        representation: &Representation,
        sequence_map: RawSequenceMap,
        words: Vec<String>,
        assets: &Assets,
    ) -> Encoder {
        // 预处理单字拆分表
        let sequence_map = representation.transform_elements(&sequence_map);

        // 将拆分序列映射降序排列，然后拆分成两个数组，一个只放字，一个只放序列
        let mut characters_all: Vec<(char, Sequence)> = sequence_map.clone().into_iter().collect();
        characters_all
            .sort_by_key(|x| Reverse(*assets.character_frequency.get(&x.0).unwrap_or(&0)));
        let (characters, characters_sequence): (Vec<_>, Vec<_>) =
            characters_all.into_iter().unzip();

        // 对词也是一样的操作
        let mut words_all = Self::build_word_sequence(representation, sequence_map, words);
        words_all.sort_by_key(|x| Reverse(*assets.word_frequency.get(&x.0).unwrap_or(&0)));
        let (words, words_sequence): (Vec<_>, Vec<_>) = words_all.into_iter().unzip();
        Encoder {
            characters,
            characters_sequence,
            words,
            words_sequence,
            config: representation.config.encoder.clone(),
            radix: representation.config.form.alphabet.len() + 2,
        }
    }

    fn build_word_sequence(
        representation: &Representation,
        sequence_map: SequenceMap,
        words: Vec<String>,
    ) -> Vec<(String, Sequence)> {
        let max_length = representation.config.encoder.max_length;
        // 从词表生成词的拆分序列，滤掉因缺少字的拆分而无法构词的情况
        let lookup = Self::build_lookup(&representation.config);
        let mut words_all: Vec<(String, Sequence)> = Vec::new();
        // 如果根本没想优化词，就不考虑这个拆分
        if let None = representation.config.optimization.objective.words {
            return words_all;
        }
        for word in words {
            let chars: Vec<char> = word.chars().collect();
            // 过滤掉太长的词
            if chars.len() > MAX_WORD_LENGTH {
                continue;
            }
            let rule = &lookup[chars.len() - 2]; // 二字词的下标是 0，所以要减二
            let mut word_elements: Vec<usize> = Vec::new();
            let mut has_invalid_char = false;
            for (char_index, code_index) in rule {
                let char = Self::signed_index(&chars, *char_index);
                if let Some(sequence) = sequence_map.get(char) {
                    let value = Self::signed_index(sequence, *code_index);
                    word_elements.push(*value);
                } else {
                    has_invalid_char = true;
                    break;
                }
            }
            if word_elements.len() > max_length {
                panic!(
                    "按当前的构词规则，词语「{}」包含的元素数量为 {}，超过了最大码长 {}",
                    word,
                    word_elements.len(),
                    max_length
                );
            }
            if !has_invalid_char {
                words_all.push((word.clone(), word_elements));
            }
        }
        words_all
    }

    fn get_auto_select_value(&self) -> usize {
        let auto_select_length = self.config.auto_select_length.unwrap_or(0);
        self.radix.pow(auto_select_length as u32 - 1)
    }

    fn get_space(&self) -> usize {
        let max_length = self.config.max_length;
        self.radix.pow(max_length as u32)
    }

    pub fn encode_full(&self, keymap: &KeyMap, data: &Vec<Sequence>, output: &mut Codes) {
        let auto_select = self.get_auto_select_value();
        for (sequence, pointer) in zip(data, output) {
            let mut code = 0_usize;
            let mut weight = 1_usize;
            for element in sequence {
                code += keymap[*element] * weight;
                weight *= self.radix;
            }
            if code <= auto_select {
                code += (self.radix - 1) * weight;
            }
            *pointer = code;
        }
    }

    pub fn encode_reduced(&self, full_code: &Codes, output: &mut Codes) {
        let auto_select = self.get_auto_select_value();
        let mut occupation = vec![false; self.get_space()];
        for (code, pointer) in zip(full_code, output) {
            let mut has_reduced = false;
            let mut reduced_code = 0;
            let mut modulo = self.radix;
            while reduced_code < *code {
                reduced_code = code % modulo;
                if reduced_code < auto_select {
                    reduced_code += (self.radix - 1) * modulo;
                }
                modulo *= self.radix;
                if !occupation[reduced_code] {
                    occupation[reduced_code] = true;
                    *pointer = reduced_code;
                    has_reduced = true;
                    break;
                }
            }
            if has_reduced == false {
                *pointer = *code;
            }
        }
    }

    pub fn encode_character_full(&self, keymap: &KeyMap, output: &mut Codes) {
        self.encode_full(keymap, &self.characters_sequence, output)
    }

    pub fn encode_words_full(&self, keymap: &KeyMap, output: &mut Codes) {
        self.encode_full(keymap, &self.words_sequence, output)
    }

    fn signed_index<T: Debug>(vector: &Vec<T>, index: isize) -> &T {
        return if index >= 0 {
            &vector[index as usize]
        } else {
            &vector[vector.len() - (-index as usize)]
        };
    }
}
