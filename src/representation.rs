use crate::{config::Config, objective::EncodeExport};
use serde::{Serialize, Deserialize};
use regex::Regex;
use std::collections::HashMap;

pub type RawSequenceMap = HashMap<char, String>;
pub type WordList = Vec<String>;
pub type KeyEquivalence = HashMap<char, f64>;
pub type PairEquivalence = HashMap<String, f64>;
pub type Frequency<T> = HashMap<T, u64>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Assets {
    pub character_frequency: Frequency<char>,
    pub word_frequency: Frequency<String>,
    pub key_equivalence: KeyEquivalence,
    pub pair_equivalence: PairEquivalence,
}

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
    pub select_keys: Vec<Key>,
}

impl Representation {
    pub fn new(config: Config) -> Self {
        let (radix, select_keys, key_repr, repr_key) = Self::transform_alphabet(&config);
        let (initial, element_repr, repr_element) = Self::transform_keymap(&config, &key_repr);
        Self {
            config,
            initial,
            element_repr,
            repr_element,
            key_repr,
            repr_key,
            radix,
            select_keys,
        }
    }

    pub fn transform_alphabet(
        config: &Config,
    ) -> (usize, Vec<Key>, HashMap<char, Key>, HashMap<Key, char>) {
        // 0 = 空字符串
        // 1, ... n = 所有常规编码键
        // n + 1, ..., m = 所有选择键
        let mut key_repr: HashMap<char, Key> = HashMap::new();
        let mut repr_key: HashMap<Key, char> = HashMap::new();
        let mut index = 1_usize;
        for key in config.form.alphabet.chars() {
            assert!(!key_repr.contains_key(&key), "编码键有重复！");
            key_repr.insert(key, index);
            repr_key.insert(index, key);
            index += 1;
        }
        let default_select_keys = vec!['_'];
        let select_keys = config
            .encoder
            .select_keys
            .as_ref()
            .unwrap_or(&default_select_keys);
        assert!(select_keys.len() >= 1, "选择键不能为空！");
        let mut parsed_select_keys: Vec<Key> = vec![];
        for key in select_keys {
            assert!(!key_repr.contains_key(&key), "编码键或选择键有重复！");
            key_repr.insert(*key, index);
            repr_key.insert(index, *key);
            parsed_select_keys.push(index);
            index += 1;
        }
        let radix = index;
        (radix, parsed_select_keys, key_repr, repr_key)
    }

    pub fn transform_keymap(
        config: &Config,
        key_repr: &HashMap<char, Key>,
    ) -> (KeyMap, HashMap<String, Element>, HashMap<Element, String>) {
        let mut keymap: KeyMap = Vec::new();
        let mut forward_converter: HashMap<String, usize> = HashMap::new();
        let mut reverse_converter: HashMap<usize, String> = HashMap::new();
        for (element, mapped) in &config.form.mapping {
            let chars: Vec<Key> = mapped
                .chars()
                .map(|x| {
                    *key_repr.get(&x).expect(&format!(
                        "元素 {} 的编码 {} 中的字符 {} 并不在字母表中",
                        element, mapped, x
                    ))
                })
                .collect();
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
        let max_length = self.config.encoder.max_length;
        if max_length >= 6 {
            panic!("目前暂不支持最大码长大于等于 6 的方案计算！")
        }
        for (char, sequence) in raw_sequence_map {
            let mut converted_elems: Vec<usize> = Vec::new();
            let sequence: Vec<_> = sequence.split(' ').map(|x| x.to_string()).collect();
            if sequence.len() > max_length {
                panic!(
                    "汉字「{}」包含的元素数量为 {}，超过了最大码长 {}",
                    char,
                    sequence.len(),
                    max_length
                );
            }
            for element in &sequence {
                if let Some(number) = self.element_repr.get(element) {
                    converted_elems.push(*number);
                } else {
                    panic!(
                        "汉字「{}」包含的元素「{}」无法在键盘映射中找到",
                        char, element
                    );
                }
            }
            sequence_map.insert(*char, converted_elems);
        }
        sequence_map
    }

    pub fn update_config(&self, candidate: &KeyMap) -> Config {
        let mut new_config = self.config.clone();
        let lookup = |element: &String| {
            let number = *self
                .element_repr
                .get(element)
                .expect(&format!("元素「{}」未知", element));
            let current_mapped = &candidate[number];
            *self
                .repr_key
                .get(current_mapped)
                .expect(&format!("按键代号「{}」未知", current_mapped))
        };
        for (element, mapped) in &self.config.form.mapping {
            let new_element = element.clone();
            let new_mapped = if mapped.len() == 1 {
                lookup(element).to_string()
            } else {
                let mut all_codes = String::new();
                for index in 0..mapped.len() {
                    let name = format!("{}.{}", element, index);
                    all_codes.push(lookup(&name));
                }
                all_codes
            };
            new_config.form.mapping.insert(new_element, new_mapped);
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
                total += key_equivalence
                    .get(&char)
                    .expect(&format!("键位 {} 的用指当量数据未知", char));
            }
            result.push(total);
        }
        result
    }

    pub fn transform_pair_equivalence(
        &self,
        pair_equivalence: &HashMap<String, f64>,
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
                let pair: String = [chars[i], chars[i + 1]].iter().collect();
                total += pair_equivalence
                    .get(&pair)
                    .expect(&format!("键位组合 {:?} 的速度当量数据未知", pair));
            }
            result.push(total);
        }
        result
    }

    pub fn transform_auto_select(&self) -> Vec<bool> {
        let mut result: Vec<bool> = vec![];
        let encoder = &self.config.encoder;
        let re = encoder
            .auto_select_pattern
            .as_ref()
            .map(|x| Regex::new(x).expect("正则表达式不合法"));
        for code in 0..self.get_space() {
            let chars = self.repr_code(code);
            let string: String = chars.iter().collect();
            let is_matched = if let Some(re) = &re {
                re.is_match(&string)
            } else if let Some(length) = encoder.auto_select_length {
                chars.len() >= length
            } else {
                true
            };
            let is_max_length = chars.len() == encoder.max_length;
            result.push(is_matched || is_max_length);
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
