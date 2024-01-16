//! 编码引擎

use crate::{
    config::{EncoderConfig, ShortCodeConfig, WordRule},
    representation::{
        Assets, Buffer, Codes, EncodeExport, Entry, Key, KeyMap, Occupation, RawSequenceMap,
        Representation, Sequence, SequenceMap,
    },
};
use std::{cmp::Reverse, fmt::Debug, iter::zip};

// 支持二字词直到十字词
const MAX_WORD_LENGTH: usize = 10;

#[derive(Debug)]
pub struct Encoder {
    pub characters: Vec<char>,
    characters_sequence: Vec<Sequence>,
    pub words: Option<Vec<String>>,
    words_sequence: Option<Vec<Sequence>>,
    config: EncoderConfig,
    pub radix: usize,
    pub alphabet_radix: usize,
    auto_select: Vec<bool>,
    select_keys: Vec<Key>,
    short_code_schemes: Option<Vec<CompiledShortCodeConfig>>,
}

#[derive(Debug)]
struct CompiledShortCodeConfig {
    pub prefix: usize,
    pub select_keys: Vec<usize>,
}

impl Encoder {
    /// 将 Rime 格式的［AaAbBaBb］这样的字符串转换成一个数对的列表
    /// 每个数对表示要取哪个字的哪个码
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

    /// 将规则列表中的每一个字符串形式的规则都转换成数对的列表
    fn build_lookup(rules: &Vec<WordRule>) -> [Vec<(isize, isize)>; MAX_WORD_LENGTH - 1] {
        let mut quick_lookup: [Vec<(isize, isize)>; MAX_WORD_LENGTH - 1] = Default::default();
        for i in 2..=MAX_WORD_LENGTH {
            // 尝试从规则列表中找到一个能符合当前长度的规则
            let mut one_matched = false;
            for rule in rules {
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

    /// 提供配置表示、拆分表、词表和共用资源来创建一个编码引擎
    /// 字需要提供拆分表
    /// 词只需要提供词表，它对应的拆分序列从字推出
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
        let short_code_schemes = representation
            .config
            .encoder
            .short_code_schemes
            .as_ref()
            .map(|x| Self::build_short_code_schemes(x, representation));

        // 对词也是一样的操作
        let mut words_all = Self::build_word_sequence(representation, sequence_map, words);
        words_all
            .as_mut()
            .map(|x| x.sort_by_key(|x| Reverse(*assets.word_frequency.get(&x.0).unwrap_or(&0))));
        let (words, words_sequence): (Option<Vec<_>>, Option<Vec<_>>) =
            words_all.map_or((None, None), |x| {
                let (a, b) = x.into_iter().unzip();
                (Some(a), Some(b))
            });
        Encoder {
            characters,
            characters_sequence,
            words,
            words_sequence,
            config: representation.config.encoder.clone(),
            radix: representation.radix,
            alphabet_radix: representation.alphabet_radix,
            auto_select: representation.transform_auto_select(),
            select_keys: representation.select_keys.clone(),
            short_code_schemes,
        }
    }

    fn build_short_code_schemes(
        schemes: &Vec<ShortCodeConfig>,
        representation: &Representation,
    ) -> Vec<CompiledShortCodeConfig> {
        let parse_keys = |s: &Vec<char>| -> Vec<usize> {
            s.iter()
                .map(|c| {
                    *representation
                        .key_repr
                        .get(&c)
                        .expect("简码的选择键没有包含在全局选择键中")
                })
                .collect()
        };
        schemes
            .iter()
            .map(|s| {
                let count = s.count.unwrap_or(1);
                let select_keys = s
                    .select_keys
                    .as_ref()
                    .map_or(representation.select_keys.clone(), parse_keys);
                assert!(
                    count as usize <= select_keys.len(),
                    "选重数量不能高于选择键数量"
                );
                CompiledShortCodeConfig {
                    prefix: s.prefix,
                    select_keys: select_keys[..count].to_vec(),
                }
            })
            .collect()
    }

    fn build_word_sequence(
        representation: &Representation,
        sequence_map: SequenceMap,
        words: Vec<String>,
    ) -> Option<Vec<(String, Sequence)>> {
        let rules = &representation.config.encoder.rules;
        // 如果没有组词规则，就不生成词拆表
        if rules.is_none() {
            return None;
        }
        // 从词表生成词的拆分序列，滤掉因缺少字的拆分而无法构词的情况
        let mut words_all: Vec<(String, Sequence)> = Vec::new();
        let lookup = Self::build_lookup(rules.as_ref().unwrap());
        let max_length = representation.config.encoder.max_length;
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
        Some(words_all)
    }

    pub fn get_space(&self) -> usize {
        let max_length = self.config.max_length;
        self.radix.pow(max_length as u32)
    }

    pub fn encode_full(
        &self,
        keymap: &KeyMap,
        data: &Vec<Sequence>,
        output: &mut Codes,
        occupation: &mut Occupation,
    ) {
        for (sequence, pointer) in zip(data, output) {
            let mut code = 0_usize;
            let mut weight = 1_usize;
            for element in sequence {
                code += keymap[*element] * weight;
                weight *= self.radix;
            }
            // 全码时，忽略次选及之后的选择键，给所有不能自动上屏的码统一添加首选键
            // 这是为了便于计算重码，否则还要判断
            if !self.auto_select[code] {
                code += self.select_keys[0] * weight;
            }
            *pointer = (code, occupation[code]);
            occupation[code] = true;
        }
    }

    pub fn encode_short(
        &self,
        full_codes: &Codes,
        short_codes: &mut Codes,
        full_occupation: &Occupation,
    ) {
        let schemes = self.short_code_schemes.as_ref().unwrap();
        let mut short_occupation = vec![false; self.get_space()];
        for ((full, _), pointer) in zip(full_codes, short_codes) {
            let mut has_reduced = false;
            for scheme in schemes {
                let CompiledShortCodeConfig {
                    prefix,
                    select_keys,
                } = scheme;
                // 如果根本没有这么多码，就放弃
                if *full < self.radix.pow((*prefix - 1) as u32) {
                    continue;
                }
                // 首先将全码截取一部分出来
                let modulo = self.radix.pow(*prefix as u32);
                let prefix = full % modulo;
                for (index, key) in select_keys.iter().enumerate() {
                    // 如果是首选且不能自动上屏，就要加选择键
                    let short = if index == 0 && self.auto_select[prefix] {
                        prefix
                    } else {
                        prefix + key * modulo // 补选择键
                    };
                    // 决定出这个简码
                    if !full_occupation[short] && !short_occupation[short] {
                        short_occupation[short] = true;
                        *pointer = (short, false);
                        has_reduced = true;
                        break;
                    }
                }
                if has_reduced {
                    break;
                }
            }
            if has_reduced == false {
                *pointer = (*full, short_occupation[*full]);
                short_occupation[*full] = true;
            }
        }
    }

    pub fn encode_character_full(
        &self,
        keymap: &KeyMap,
        output: &mut Codes,
        occupation: &mut Occupation,
    ) {
        self.encode_full(keymap, &self.characters_sequence, output, occupation)
    }

    pub fn encode_words_full(
        &self,
        keymap: &KeyMap,
        output: &mut Codes,
        occupation: &mut Occupation,
    ) {
        self.encode_full(
            keymap,
            self.words_sequence.as_ref().expect("没有词组数据"),
            output,
            occupation,
        )
    }

    fn signed_index<T: Debug>(vector: &Vec<T>, index: isize) -> &T {
        return if index >= 0 {
            &vector[index as usize]
        } else {
            &vector[vector.len() - (-index as usize)]
        };
    }

    pub fn init_buffer(&self) -> Buffer {
        Buffer {
            characters_full: vec![(0, false); self.characters.len()],
            characters_short: self
                .short_code_schemes
                .as_ref()
                .map(|_| vec![(0, false); self.characters.len()]),
            words_full: self.words.as_ref().map(|x| vec![(0, false); x.len()]),
        }
    }

    pub fn encode(&self, keymap: &KeyMap, representation: &Representation) -> EncodeExport {
        let mut buffer = self.init_buffer();
        let mut occupation: Occupation = vec![false; self.get_space()];
        self.encode_character_full(keymap, &mut buffer.characters_full, &mut occupation);
        if self.short_code_schemes.is_some() {
            self.encode_short(
                &mut buffer.characters_full,
                buffer.characters_short.as_mut().unwrap(),
                &mut occupation,
            );
        }
        let mut character_entries: Vec<Entry> = Vec::new();
        for (index, character) in self.characters.iter().enumerate() {
            let full = representation.repr_code(buffer.characters_full[index].0);
            let short = buffer
                .characters_short
                .as_ref()
                .map(|x| representation.repr_code(x[index].0));
            character_entries.push(Entry {
                item: character.to_string(),
                full: full.iter().collect(),
                short: short.map(|x| x.iter().collect()),
            });
        }
        let mut word_entries: Option<Vec<Entry>> = None;
        if let Some(words) = self.words.as_ref() {
            self.encode_words_full(keymap, buffer.words_full.as_mut().unwrap(), &mut occupation);
            let entries = words
                .iter()
                .enumerate()
                .map(|(index, word)| {
                    let full =
                        representation.repr_code(buffer.words_full.as_ref().unwrap()[index].0);
                    Entry {
                        item: word.to_string(),
                        full: full.iter().collect(),
                        short: None,
                    }
                })
                .collect();
            word_entries = Some(entries);
        }
        EncodeExport {
            characters: character_entries,
            words: word_entries,
        }
    }
}
