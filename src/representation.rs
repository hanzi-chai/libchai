use std::collections::HashMap;

use crate::{cli::RawSequenceMap, config::Config, objective::EncodeExport};

// 元素用一个无符号整数表示
pub type Element = usize;

// 字或词的拆分序列
pub type Sequence = Vec<Element>;

// 字到拆分序列的映射
pub type SequenceMap = HashMap<char, Sequence>;

// 编码用无符号整数表示
pub type Code = usize;

pub type Codes = Vec<Code>;

// 按键用无符号整数表示
pub type Key = usize;

// 元素映射用一个数组表示，下标是元素
pub type KeyMap = Vec<Key>;

#[derive(Debug)]
pub struct EncodeOutput {
    pub character_list: Vec<char>,
    pub characters: Option<Vec<String>>,
    pub characters_reduced: Option<Vec<String>>,
    pub word_list: Vec<String>,
    pub words: Option<Vec<String>>,
    pub words_reduced: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct Buffer {
    pub characters: Codes,
    pub characters_reduced: Codes,
    pub words: Codes,
    pub words_reduced: Codes,
}

pub struct Representation {
    pub config: Config,
    pub initial: KeyMap,
    pub element_repr: HashMap<String, Element>,
    pub repr_element: HashMap<Element, String>,
    pub key_repr: HashMap<char, Key>,
    pub repr_key: HashMap<Key, char>,
    pub radix: usize,
}

impl Representation {
    pub fn new(config: Config) -> Self {
        let (radix, key_repr, repr_key) = Self::transform_alphabet(&config);
        let (initial, element_repr, repr_element) = Self::transform_keymap(&config, &key_repr);
        Self {
            config,
            initial,
            element_repr,
            repr_element,
            key_repr,
            repr_key,
            radix,
        }
    }

    pub fn transform_alphabet(config: &Config) -> (usize, HashMap<char, Key>, HashMap<Key, char>) {
        // 0 = no code
        // 1, ... 26 = a, ..., z
        // 27 = _
        let mut key_repr: HashMap<char, Key> = HashMap::new();
        let mut repr_key: HashMap<Key, char> = HashMap::new();
        let mut index = 1_usize;
        for key in config.form.alphabet.chars() {
            key_repr.insert(key, index);
            repr_key.insert(index, key);
            index += 1;
        }
        // 这个以后会支持自定义
        let select_keys = String::from("_");
        for key in select_keys.chars() {
            key_repr.insert(key, index);
            repr_key.insert(index, key);
            index += 1;
        }
        let radix = index;
        (radix, key_repr, repr_key)
    }

    pub fn transform_keymap(
        config: &Config,
        key_repr: &HashMap<char, Key>,
    ) -> (KeyMap, HashMap<String, Element>, HashMap<Element, String>) {
        let mut keymap: KeyMap = Vec::new();
        let mut forward_converter: HashMap<String, usize> = HashMap::new();
        let mut reverse_converter: HashMap<usize, String> = HashMap::new();
        for (element, mapped) in &config.form.mapping {
            let chars: Vec<Key> = mapped.chars().map(|x| *key_repr.get(&x).unwrap()).collect();
            if chars.len() == 1 {
                forward_converter.insert(element.clone(), keymap.len());
                reverse_converter.insert(keymap.len(), element.clone());
                keymap.push(chars[0]);
            } else {
                for (index, key) in chars.iter().enumerate() {
                    let name = format!("{}.{}", element.to_string(), index);
                    forward_converter.insert(name.clone(), keymap.len());
                    reverse_converter.insert(keymap.len(), name.clone());
                    keymap.push(*key);
                }
            }
        }
        (keymap, forward_converter, reverse_converter)
    }

    pub fn transform_elements(&self, raw_sequence_map: &RawSequenceMap) -> SequenceMap {
        let mut sequence_map = SequenceMap::new();
        for (char, sequence) in raw_sequence_map {
            let mut converted_elems: Vec<usize> = Vec::new();
            if sequence.len() > self.config.encoder.max_length {
                panic!(
                    "汉字「{}」包含的元素数量为 {}，超过了最大码长 {}",
                    char,
                    sequence.len(),
                    self.config.encoder.max_length
                );
            }
            for element in sequence {
                if let Some(number) = self.element_repr.get(element) {
                    converted_elems.push(*number);
                } else {
                    panic!("不合法的码元：{}", element);
                }
            }
            sequence_map.insert(*char, converted_elems);
        }
        sequence_map
    }

    pub fn update_config(&self, candidate: &KeyMap) -> Config {
        let mut new_config = self.config.clone();
        for (element, mapped) in &self.config.form.mapping {
            if mapped.len() == 1 {
                let number = *self.element_repr.get(element).unwrap();
                let current_mapped = candidate[number];
                new_config
                    .form
                    .mapping
                    .insert(element.to_string(), current_mapped.to_string());
            } else {
                let mut all_codes = String::new();
                for index in 0..mapped.len() {
                    let name = format!("{}.{}", element.to_string(), index);
                    let number = *self.element_repr.get(&name).unwrap();
                    let current_mapped = &candidate[number];
                    let key = *self.repr_key.get(current_mapped).unwrap();
                    all_codes.push(key);
                }
                new_config
                    .form
                    .mapping
                    .insert(element.to_string(), all_codes);
            }
        }
        new_config
    }

    pub fn repr_code(&self, code: Code) -> Vec<char> {
        let mut chars: Vec<char> = Vec::new();
        let mut remainder = code;
        while remainder > 0 {
            let k = remainder % self.radix as usize;
            remainder /= self.radix as usize;
            if k == 0 {
                continue;
            }
            let char = self.repr_key.get(&k).unwrap();
            chars.push(*char);
        }
        chars
    }

    pub fn repr_code_list(&self, codes: Codes) -> Vec<String> {
        codes
            .iter()
            .map(|x| {
                let chars = self.repr_code(*x);
                let string = chars.iter().collect();
                string
            })
            .collect()
    }

    pub fn transform_key_equivalence(&self, key_equivalence: &HashMap<char, f64>) -> Vec<f64> {
        let mut result: Vec<f64> = vec![];
        for code in 0..self.get_space() {
            let chars = self.repr_code(code);
            let mut total = 0.0;
            for char in chars {
                total += key_equivalence.get(&char).unwrap();
            }
            result.push(total);
        }
        result
    }

    pub fn transform_pair_equivalence(
        &self,
        pair_equivalence: &HashMap<(char, char), f64>,
    ) -> Vec<f64> {
        let mut result: Vec<f64> = vec![];
        for code in 0..self.get_space() {
            let chars = self.repr_code(code);
            if chars.len() < 2 {
                result.push(0.0);
                continue;
            }
            let mut total = 0.0;
            for i in 0..(chars.len() - 1) {
                let pair = (chars[i], chars[i + 1]);
                total += pair_equivalence.get(&pair).unwrap();
            }
            result.push(total);
        }
        result
    }

    fn get_space(&self) -> usize {
        let max_length = self.config.encoder.max_length;
        self.radix.pow(max_length as u32)
    }

    pub fn init_buffer(&self, nchar: usize, nword: usize) -> Buffer {
        Buffer {
            characters: vec![0; nchar],
            characters_reduced: vec![0; nchar],
            words: vec![0; nword],
            words_reduced: vec![0; nword],
        }
    }

    pub fn recover_codes(&self, codes: EncodeExport) -> EncodeOutput {
        let EncodeExport {
            character_list,
            characters,
            characters_reduced,
            word_list,
            words,
            words_reduced,
        } = codes;
        EncodeOutput {
            character_list,
            characters: characters.map(|x| self.repr_code_list(x)),
            characters_reduced: characters_reduced.map(|x| self.repr_code_list(x)),
            word_list,
            words: words.map(|x| self.repr_code_list(x)),
            words_reduced: words_reduced.map(|x| self.repr_code_list(x)),
        }
    }
}
