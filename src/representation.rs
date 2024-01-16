//! 内部数据结构的表示和定义

use crate::config::{Config, Mapped, MappedKey};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type RawSequenceMap = HashMap<char, String>;
pub type WordList = Vec<String>;
pub type KeyDistribution = HashMap<char, f64>;
pub type PairEquivalence = HashMap<String, f64>;
pub type Frequency<T> = HashMap<T, u64>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Assets {
    pub character_frequency: Frequency<char>,
    pub word_frequency: Frequency<String>,
    pub key_distribution: KeyDistribution,
    pub pair_equivalence: PairEquivalence,
}

/// 元素用一个无符号整数表示
pub type Element = usize;

/// 字或词的拆分序列
pub type Sequence = Vec<Element>;

/// 字到拆分序列的映射
pub type SequenceMap = HashMap<char, Sequence>;

/// 编码用无符号整数表示
pub type Code = usize;

/// 一组编码
pub type Codes = Vec<(Code, bool)>;

/// 按键用无符号整数表示
pub type Key = usize;

/// 元素映射用一个数组表示，下标是元素
pub type KeyMap = Vec<Key>;

/// 每个编码上占据了几个候选
pub type Occupation = Vec<bool>;

#[derive(Debug, Serialize)]
pub struct Entry {
    pub item: String,
    pub full: String,
    pub short: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EncodeExport {
    pub characters: Vec<Entry>,
    pub words: Option<Vec<Entry>>,
}

#[derive(Debug)]
pub struct Buffer {
    pub characters_full: Codes,
    pub characters_short: Option<Codes>,
    pub words_full: Option<Codes>,
}

/// 配置表示是对配置文件的进一步封装，除了保存一份配置文件本身之外，还根据配置文件的内容推导出用于各种转换的映射
pub struct Representation {
    pub config: Config,
    pub initial: KeyMap,
    pub element_repr: HashMap<String, Element>,
    pub repr_element: HashMap<Element, String>,
    pub key_repr: HashMap<char, Key>,
    pub repr_key: HashMap<Key, char>,
    pub radix: usize,
    pub alphabet_radix: usize,
    pub select_keys: Vec<Key>,
}

impl Mapped {
    pub fn len(&self) -> usize {
        match self {
            Mapped::Basic(s) => s.len(),
            Mapped::Advanced(v) => v.len(),
        }
    }

    pub fn normalize(&self) -> Vec<MappedKey> {
        match self {
            Mapped::Advanced(vector) => vector.clone(),
            Mapped::Basic(string) => string.chars().map(|x| MappedKey::Ascii(x)).collect(),
        }
    }
}

pub fn assemble(element: &String, index: usize) -> String {
    if index == 0 {
        element.to_string()
    } else {
        format!("{}.{}", element.to_string(), index)
    }
}

impl Representation {
    pub fn new(config: Config) -> Self {
        let (radix, alphabet_radix, select_keys, key_repr, repr_key) = Self::transform_alphabet(&config);
        let (initial, element_repr, repr_element) = Self::transform_keymap(&config, &key_repr);
        Self {
            config,
            initial,
            element_repr,
            repr_element,
            key_repr,
            repr_key,
            radix,
            alphabet_radix,
            select_keys,
        }
    }

    /// 读取字母表和选择键列表，然后分别对它们的每一个按键转换成无符号整数
    /// 1, ... n = 所有常规编码键
    /// n + 1, ..., m = 所有选择键
    pub fn transform_alphabet(
        config: &Config,
    ) -> (usize, usize, Vec<Key>, HashMap<char, Key>, HashMap<Key, char>) {
        let mut key_repr: HashMap<char, Key> = HashMap::new();
        let mut repr_key: HashMap<Key, char> = HashMap::new();
        let mut index = 1_usize;
        for key in config.form.alphabet.chars() {
            assert!(!key_repr.contains_key(&key), "编码键有重复！");
            key_repr.insert(key, index);
            repr_key.insert(index, key);
            index += 1;
        }
        let alphabet_radix = index;
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
        (radix, alphabet_radix, parsed_select_keys, key_repr, repr_key)
    }

    /// 读取元素映射，然后把每一个元素转换成无符号整数，从而可以用向量来表示一个元素布局，向量的下标就是元素对应的数
    pub fn transform_keymap(
        config: &Config,
        key_repr: &HashMap<char, Key>,
    ) -> (KeyMap, HashMap<String, Element>, HashMap<Element, String>) {
        let mut keymap: KeyMap = Vec::new();
        let mut forward_converter: HashMap<String, usize> = HashMap::new();
        let mut reverse_converter: HashMap<usize, String> = HashMap::new();
        for (element, mapped) in &config.form.mapping {
            let normalized = mapped.normalize();
            for (index, mapped_key) in normalized.iter().enumerate() {
                if let MappedKey::Ascii(x) = mapped_key {
                    let key = key_repr.get(&x).expect(&format!(
                        "元素 {} 的编码中的字符 {} 并不在字母表中",
                        element, x
                    ));
                    let name = assemble(element, index);
                    forward_converter.insert(name.clone(), keymap.len());
                    reverse_converter.insert(keymap.len(), name.clone());
                    keymap.push(*key);
                }
            }
        }
        (keymap, forward_converter, reverse_converter)
    }

    /// 读取拆分表，将拆分序列中的每一个元素按照先前确定的元素 -> 整数映射来转换为整数向量
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

    /// 根据一个计算中得到的元素布局来生成一份新的配置文件，其余内容不变直接复制过来
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
            let new_mapped = match mapped {
                Mapped::Basic(string) => {
                    let mut all_codes = String::new();
                    for index in 0..string.len() {
                        let name = assemble(element, index);
                        all_codes.push(lookup(&name));
                    }
                    Mapped::Basic(all_codes)
                }
                Mapped::Advanced(vector) => {
                    let all_codes: Vec<MappedKey> = vector
                        .iter()
                        .enumerate()
                        .map(|(index, mapped_key)| match mapped_key {
                            MappedKey::Ascii(_) => {
                                MappedKey::Ascii(lookup(&assemble(element, index)))
                            }
                            other => other.clone(),
                        })
                        .collect();
                    Mapped::Advanced(all_codes)
                }
            };
            new_config.form.mapping.insert(new_element, new_mapped);
        }
        new_config
    }

    /// 如前所述，建立了一个按键到整数的映射之后，可以将字符串看成具有某个进制的数。所以，给定一个数，也可以把它转化为字符串
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

    /// 根据编码字符和未归一化的键位分布，生成一个理想的键位分布
    pub fn generate_ideal_distribution(&self, key_distribution: &HashMap<char, f64>) -> Vec<f64> {
        let mut result: Vec<f64> = (0..self.alphabet_radix)
            .map(|x| {
                self.repr_key
                    .get(&x)
                    .map_or(0.0, |c| *key_distribution.get(c).unwrap())
            })
            .collect();
        // 归一化
        let sum: f64 = result.iter().sum();
        for i in result.iter_mut() {
            *i /= sum;
        }
        result
    }

    /// 将编码空间内所有的编码组合预先计算好速度当量
    /// 按照这个字符串所对应的整数为下标，存储到一个大数组中
    pub fn transform_pair_equivalence(&self, pair_equivalence: &HashMap<String, f64>) -> Vec<f64> {
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

    /// 将编码空间内所有的编码组合预先计算好新速度当量（杏码算法）
    /// 按照这个字符串所对应的整数为下标，存储到一个大数组中
    pub fn transform_new_pair_equivalence(
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
            //遍历所有组合
            let mut combinations: Vec<String> = vec!["".to_string()];
            for i in 1..chars.len() - 1 {
                for j in 0..combinations.len() {
                    combinations.push(format!("{}{}", combinations[j], chars[i]));
                }
            }
            let mut total = 0.0;
            for s in combinations.iter() {
                let mut thistime = 0.0;
                let s_chars: Vec<char> = format!("{}{}{}", chars[0], s, chars[chars.len() - 1])
                    .chars()
                    .collect();
                for i in 0..(s_chars.len() - 1) {
                    let pair: String = [s_chars[i], s_chars[i + 1]].iter().collect();
                    thistime += pair_equivalence
                        .get(&pair)
                        .expect(&format!("键位组合 {:?} 的速度当量数据未知", pair));
                }
                if thistime > total {
                    total = thistime
                }
            }
            result.push(total);
        }
        result
    }

    /// 将编码空间内所有的编码组合预先计算好是否能自动上屏
    /// 按照这个字符串所对应的整数为下标，存储到一个大数组中
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
}
