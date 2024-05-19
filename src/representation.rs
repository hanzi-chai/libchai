//! 内部数据结构的表示和定义

use crate::{
    config::{Config, Mapped, MappedKey, Scheme, ShortCodeConfig},
    encoder::{CompiledScheme, Encodable, Encoder},
    error::Error,
    objectives::fingering::get_fingering_types,
};
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MAX_WORD_LENGTH: usize = 10;

#[derive(Deserialize)]
pub struct Input {
    pub config: Config,
    pub info: AssembleList,
    pub assets: Assets,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Assemble {
    pub name: String,
    pub sequence: String,
    pub importance: u64,
    #[serde(default = "Assemble::suggested_level_default")]
    pub level: i64,
}

impl Assemble {
    const fn suggested_level_default() -> i64 {
        -1
    }
}

pub type AssembleList = Vec<Assemble>;
pub type WordList = Vec<String>;
pub type KeyDistribution = HashMap<char, DistributionLoss>;
pub type PairEquivalence = HashMap<String, f64>;
pub type Frequency = HashMap<String, u64>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionLoss {
    pub ideal: f64,
    pub lt_penalty: f64,
    pub gt_penalty: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Assets {
    pub frequency: Frequency,
    pub key_distribution: KeyDistribution,
    pub pair_equivalence: PairEquivalence,
}

/// 元素用一个无符号整数表示
pub type Element = usize;

/// 字或词的拆分序列
pub type Sequence = Vec<Element>;

/// 编码用无符号整数表示
pub type Code = usize;

///
#[derive(Clone, Debug, Copy)]
pub struct CodeInfo {
    pub code: Code,
    pub frequency: u64,
    pub rank: i8,
    pub single: bool,
}

/// 一组编码
pub type Codes = Vec<CodeInfo>;

/// 按键用无符号整数表示
pub type Key = usize;

/// 元素映射用一个数组表示，下标是元素
pub type KeyMap = Vec<Key>;

/// 用指标记
pub type Label = [u8; 8];

/// 编码是否已被占据
/// 用一个数组和一个哈希集合来表示，数组用来表示四码以内的编码，哈希集合用来表示四码以上的编码
pub struct Occupation {
    pub vector: Vec<Slot>,
    pub hashmap: FxHashMap<usize, u8>,
}

#[derive(Default, Clone)]
pub struct Slot {
    pub hash: u16,
    pub count: u8,
}

impl Occupation {
    pub fn new(length: usize) -> Self {
        let vector = vec![Slot::default(); length];
        let hashset = FxHashMap::default();
        Self {
            vector,
            hashmap: hashset,
        }
    }

    pub fn insert(&mut self, index: usize, hash: u16) {
        if index < self.vector.len() {
            self.vector[index].hash = hash;
            self.vector[index].count += 1;
        } else {
            self.hashmap
                .insert(index, self.hashmap.get(&index).unwrap_or(&0) + 1);
        }
    }

    pub fn rank(&self, index: usize) -> u8 {
        if index < self.vector.len() {
            self.vector[index].count
        } else {
            *self.hashmap.get(&index).unwrap_or(&0)
        }
    }

    pub fn rank_hash(&self, index: usize, hash: u16) -> u8 {
        if index < self.vector.len() {
            self.vector[index].count - (self.vector[index].hash == hash) as u8
        } else {
            *self.hashmap.get(&index).unwrap_or(&0)
        }
    }
}

pub type AutoSelect = Vec<bool>;

pub const MAX_COMBINATION_LENGTH: usize = 4;

#[derive(Debug, Serialize)]
pub struct Entry {
    pub name: String,
    pub full: String,
    pub full_rank: i8,
    pub short: String,
    pub short_rank: i8,
}

#[derive(Debug)]
pub struct Buffer {
    pub full: Codes,
    pub short: Codes,
}

impl Buffer {
    pub fn new(encoder: &Encoder) -> Self {
        let make_placeholder = |x: &Encodable| CodeInfo {
            code: 0,
            frequency: x.frequency,
            rank: 0,
            single: x.name.chars().count() == 1,
        };
        let it = encoder.encodables.iter();
        Self {
            full: it.clone().map(make_placeholder).collect(),
            short: it.clone().map(make_placeholder).collect(),
        }
    }
}

/// 配置表示是对配置文件的进一步封装，除了保存一份配置文件本身之外，还根据配置文件的内容推导出用于各种转换的映射
#[derive(Debug, Clone)]
pub struct Representation {
    pub config: Config,
    pub initial: KeyMap,
    pub element_repr: FxHashMap<String, Element>,
    pub repr_element: FxHashMap<Element, String>,
    pub key_repr: FxHashMap<char, Key>,
    pub repr_key: FxHashMap<Key, char>,
    pub radix: usize,
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
    pub fn new(config: Config) -> Result<Self, Error> {
        let (radix, select_keys, key_repr, repr_key) = Self::transform_alphabet(&config)?;
        let (initial, element_repr, repr_element) = Self::transform_keymap(&config, &key_repr)?;
        let repr = Self {
            config,
            initial,
            element_repr,
            repr_element,
            key_repr,
            repr_key,
            radix,
            select_keys,
        };
        Ok(repr)
    }

    /// 读取字母表和选择键列表，然后分别对它们的每一个按键转换成无符号整数
    /// 1, ... n = 所有常规编码键
    /// n + 1, ..., m = 所有选择键
    pub fn transform_alphabet(
        config: &Config,
    ) -> Result<(usize, Vec<Key>, FxHashMap<char, Key>, FxHashMap<Key, char>), Error> {
        let mut key_repr: FxHashMap<char, Key> = FxHashMap::default();
        let mut repr_key: FxHashMap<Key, char> = FxHashMap::default();
        let mut index = 1_usize;
        for key in config.form.alphabet.chars() {
            if key_repr.contains_key(&key) {
                return Err("编码键有重复！".into());
            };
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
        if select_keys.len() < 1 {
            return Err("选择键不能为空！".into());
        }
        let mut parsed_select_keys: Vec<Key> = vec![];
        for key in select_keys {
            if key_repr.contains_key(&key) {
                return Err("编码键有重复！".into());
            };
            key_repr.insert(*key, index);
            repr_key.insert(index, *key);
            parsed_select_keys.push(index);
            index += 1;
        }
        let radix = index;
        Ok((radix, parsed_select_keys, key_repr, repr_key))
    }

    /// 读取元素映射，然后把每一个元素转换成无符号整数，从而可以用向量来表示一个元素布局，向量的下标就是元素对应的数
    pub fn transform_keymap(
        config: &Config,
        key_repr: &FxHashMap<char, Key>,
    ) -> Result<
        (
            KeyMap,
            FxHashMap<String, Element>,
            FxHashMap<Element, String>,
        ),
        Error,
    > {
        let mut keymap: KeyMap = Vec::new();
        let mut forward_converter: FxHashMap<String, usize> = FxHashMap::default();
        let mut reverse_converter: FxHashMap<usize, String> = FxHashMap::default();
        for (element, mapped) in &config.form.mapping {
            let normalized = mapped.normalize();
            for (index, mapped_key) in normalized.iter().enumerate() {
                if let MappedKey::Ascii(x) = mapped_key {
                    if let Some(key) = key_repr.get(&x) {
                        let name = assemble(element, index);
                        forward_converter.insert(name.clone(), keymap.len());
                        reverse_converter.insert(keymap.len(), name.clone());
                        keymap.push(*key);
                    } else {
                        return Err(
                            format!("元素 {element} 的编码中的字符 {x} 并不在字母表中").into()
                        );
                    }
                }
            }
        }
        Ok((keymap, forward_converter, reverse_converter))
    }

    /// 读取拆分表，将拆分序列中的每一个元素按照先前确定的元素 -> 整数映射来转换为整数向量
    pub fn transform_elements(&self, assemble: Assemble) -> Result<Sequence, Error> {
        let max_length = self.config.encoder.max_length;
        let name = assemble.name;
        let raw_sequence: Vec<_> = assemble.sequence.split(' ').collect();
        let mut sequence = Sequence::new();
        let length = raw_sequence.len();
        if length > max_length {
            return Err(format!(
                "编码对象「{name}」包含的元素数量为 {length}，超过了最大码长 {max_length}"
            )
            .into());
        }
        for element in raw_sequence {
            if let Some(number) = self.element_repr.get(element) {
                sequence.push(*number);
            } else {
                return Err(format!(
                    "编码对象「{name}」包含的元素「{element}」无法在键盘映射中找到"
                )
                .into());
            }
        }
        Ok(sequence)
    }

    /// 根据一个计算中得到的元素布局来生成一份新的配置文件，其余内容不变直接复制过来
    pub fn update_config(&self, candidate: &KeyMap) -> Config {
        let mut new_config = self.config.clone();
        let lookup = |element: &String| {
            let number = *self.element_repr.get(element).unwrap(); // 输入的时候已经检查过一遍，不需要再次检查
            let current_mapped = &candidate[number];
            *self.repr_key.get(current_mapped).unwrap() // 同上
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
        let mut chars: Vec<char> = Vec::with_capacity(self.config.encoder.max_length);
        let mut remainder = code;
        while remainder > 0 {
            let k = remainder % self.radix as usize;
            remainder /= self.radix as usize;
            if k == 0 {
                continue;
            }
            let char = self.repr_key.get(&k).unwrap(); // 从内部表示转换为字符，不需要检查
            chars.push(*char);
        }
        chars
    }

    /// 根据编码字符和未归一化的键位分布，生成一个理想的键位分布
    pub fn generate_ideal_distribution(
        &self,
        key_distribution: &KeyDistribution,
    ) -> Vec<DistributionLoss> {
        let default_loss = DistributionLoss {
            ideal: 0.1,
            lt_penalty: 0.0,
            gt_penalty: 1.0,
        };
        let mut result: Vec<DistributionLoss> = (0..self.radix)
            .map(|x| {
                // 0 只是为了占位，不需要统计
                if x == 0 {
                    return default_loss.clone();
                }
                let key = self.repr_key.get(&x).unwrap();
                key_distribution.get(key).unwrap_or(&default_loss).clone()
            })
            .collect();
        // 归一化
        let sum: f64 = result.iter().map(|x| x.ideal).sum();
        for i in result.iter_mut() {
            i.ideal /= sum;
        }
        result
    }

    /// 将编码空间内所有的编码组合预先计算好速度当量
    /// 按照这个字符串所对应的整数为下标，存储到一个大数组中
    pub fn transform_pair_equivalence(&self, pair_equivalence: &HashMap<String, f64>) -> Vec<f64> {
        let mut result: Vec<f64> = Vec::with_capacity(self.get_space());
        for code in 0..self.get_space() {
            let chars = self.repr_code(code);
            if chars.len() < 2 {
                result.push(0.0);
                continue;
            }
            let mut total = 0.0;
            for i in 0..(chars.len() - 1) {
                let pair: String = [chars[i], chars[i + 1]].iter().collect();
                total += pair_equivalence.get(&pair).unwrap_or(&0.0);
            }
            result.push(total);
        }
        result
    }

    /// 将编码空间内所有的编码组合预先计算好差指法标记
    /// 标记压缩到一个 64 位整数中，每四位表示一个字符的差指法标记
    /// 从低位到高位，依次是：同手、同指大跨排、同指小跨排、小指干扰、错手、三连击
    /// 按照这个字符串所对应的整数为下标，存储到一个大数组中
    pub fn transform_fingering_types(&self) -> Vec<Label> {
        let fingering_types = get_fingering_types();
        let mut result: Vec<Label> = Vec::with_capacity(self.get_space());
        for code in 0..self.get_space() {
            let chars = self.repr_code(code);
            if chars.len() < 2 {
                result.push(Label::default());
                continue;
            }
            let mut total = Label::default();
            for i in 0..(chars.len() - 1) {
                let pair = (chars[i], chars[i + 1]);
                if fingering_types.same_hand.contains(&pair) {
                    total[0] += 1;
                }
                if fingering_types.same_finger_large_jump.contains(&pair) {
                    total[1] += 1;
                }
                if fingering_types.same_finger_small_jump.contains(&pair) {
                    total[2] += 1;
                }
                if fingering_types.little_finger_interference.contains(&pair) {
                    total[3] += 1;
                }
                if fingering_types.awkward_upside_down.contains(&pair) {
                    total[4] += 1;
                }
            }
            for i in 0..(chars.len() - 2) {
                let triple = (chars[i], chars[i + 1], chars[i + 2]);
                if triple.0 == triple.1 && triple.1 == triple.2 {
                    total[5] += 1;
                }
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
        let mut result: Vec<f64> = Vec::with_capacity(self.get_space());
        for code in 0..self.get_space() {
            let chars = self.repr_code(code);
            if chars.len() < 2 {
                result.push(0.0);
                continue;
            }
            //遍历所有组合
            let combinations = 2_usize.pow(chars.len() as u32);
            let start = 2_usize.pow(chars.len() as u32 - 1) + 1;
            let mut total = 0.0;
            for s in (start..combinations).step_by(2) {
                let mut thistime = 0.0;
                let s_chars: Vec<char> = chars
                    .iter()
                    .enumerate()
                    .filter_map(|(index, char)| {
                        if s & (1 << index) != 0 {
                            Some(*char)
                        } else {
                            None
                        }
                    })
                    .collect();
                for i in 0..(s_chars.len() - 1) {
                    let pair: String = [s_chars[i], s_chars[i + 1]].iter().collect();
                    thistime += pair_equivalence.get(&pair).unwrap_or(&0.0);
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
    pub fn transform_auto_select(&self) -> Result<Vec<bool>, Error> {
        let mut result: Vec<bool> = vec![];
        let encoder = &self.config.encoder;
        let mut re: Option<Regex> = None;
        if let Some(pattern) = &encoder.auto_select_pattern {
            let re_or_error = Regex::new(pattern);
            if let Ok(regex) = re_or_error {
                re = Some(regex);
            } else {
                return Err(format!("正则表达式 {pattern} 无法解析").into());
            }
        }
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
        Ok(result)
    }

    pub fn transform_schemes(&self, schemes: &Vec<Scheme>) -> Result<Vec<CompiledScheme>, Error> {
        let mut compiled_schemes = Vec::new();
        for scheme in schemes {
            let prefix = scheme.prefix;
            let count = scheme.count.unwrap_or(1);
            let select_keys = if let Some(keys) = &scheme.select_keys {
                let mut transformed_keys = Vec::new();
                for key in keys {
                    let transformed_key = self
                        .key_repr
                        .get(&key)
                        .ok_or(format!("简码的选择键 {key} 不在全局选择键中"))?;
                    transformed_keys.push(*transformed_key);
                }
                transformed_keys
            } else {
                self.select_keys.clone()
            };
            if count as usize > select_keys.len() {
                return Err("选重数量不能高于选择键数量".into());
            }
            compiled_schemes.push(CompiledScheme {
                prefix,
                select_keys: select_keys[..count].to_vec(),
            });
        }
        Ok(compiled_schemes)
    }

    pub fn transform_short_code(
        &self,
        configs: Vec<ShortCodeConfig>,
    ) -> Result<[Vec<CompiledScheme>; MAX_WORD_LENGTH], Error> {
        let mut short_code: [Vec<CompiledScheme>; MAX_WORD_LENGTH] = Default::default();
        for config in configs {
            match config {
                ShortCodeConfig::Equal {
                    length_equal,
                    schemes,
                } => {
                    short_code[length_equal - 1].extend(self.transform_schemes(&schemes)?);
                }
                ShortCodeConfig::Range {
                    length_in_range: (from, to),
                    schemes,
                } => {
                    for length in from..=to {
                        short_code[length - 1].extend(self.transform_schemes(&schemes)?);
                    }
                }
            }
        }
        Ok(short_code)
    }

    pub fn get_space(&self) -> usize {
        let max_length = self.config.encoder.max_length.min(MAX_COMBINATION_LENGTH);
        self.radix.pow(max_length as u32)
    }
}
