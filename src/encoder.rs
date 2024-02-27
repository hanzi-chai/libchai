//! 编码引擎

use crate::config::{EncoderConfig, WordRule};
use crate::error::Error;
use crate::representation::{
    Assets, AutoSelect, Buffer, CodeInfo, Codes, EncodeExport, Entry, Key, KeyMap, Occupation,
    RawSequenceMap, Representation, Sequence, SequenceMap, MAX_COMBINATION_LENGTH,
};
use std::{cmp::Reverse, collections::HashMap, fmt::Debug, iter::zip};

// 支持二字词直到十字词
const MAX_WORD_LENGTH: usize = 10;

type Lookup = [Vec<(isize, isize)>; MAX_WORD_LENGTH - 1];

/// 一个可编码对象
#[derive(Debug, Clone)]
pub struct Encodable {
    pub name: String,
    pub sequence: Sequence,
    pub frequency: u64,
}

pub struct Encoder {
    pub characters_info: Vec<Encodable>,
    pub words_info: Option<Vec<Encodable>>,
    pub all_info: Option<Vec<Encodable>>,
    config: EncoderConfig,
    auto_select: AutoSelect,
    pub radix: usize,
    pub alphabet_radix: usize,
    select_keys: Vec<Key>,
    pub short_code_schemes: Option<Vec<CompiledShortCodeConfig>>,
    pub word_short_code_schemes: Option<Vec<CompiledShortCodeConfig>>,
}

#[derive(Debug)]
pub struct CompiledShortCodeConfig {
    pub prefix: usize,
    pub select_keys: Vec<usize>,
}

impl Encoder {
    /// 将 Rime 格式的［AaAbBaBb］这样的字符串转换成一个数对的列表
    /// 每个数对表示要取哪个字的哪个码
    fn parse_formula(s: &String, max_length: usize) -> Result<Vec<(isize, isize)>, Error> {
        let message = Error::from(format!("构词规则 {s} 不合法"));
        let mut ret: Vec<(isize, isize)> = Vec::new();
        let chars: Vec<char> = s.chars().collect();
        if chars.len() % 2 != 0 {
            return Err(message.clone());
        }
        let pairs = chars.len() / 2;
        let normalize = |x: isize| {
            // 有效的值是 0 到 25 之间
            if x < 0 || x > 25 {
                return Err(message.clone());
            }
            Ok(if x > 13 { x - 26 } else { x })
        };
        for i in 0..pairs {
            let character_symbol = chars[2 * i] as isize;
            let code_symbol = chars[2 * i + 1] as isize;
            let character_index = normalize(character_symbol - ('A' as isize))?;
            let code_index = normalize(code_symbol - ('a' as isize))?;
            ret.push((character_index, code_index));
        }
        if ret.len() > max_length {
            Err(format!("构词规则 {s} 的长度超过了最大长度 {max_length}").into())
        } else {
            Ok(ret)
        }
    }

    /// 将规则列表中的每一个字符串形式的规则都转换成数对的列表
    fn build_lookup(rules: &Vec<WordRule>, max_length: usize) -> Result<Lookup, Error> {
        let mut quick_lookup = Lookup::default();
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
                    quick_lookup[(i - 2) as usize] = Self::parse_formula(formula, max_length)?;
                    break;
                }
            }
            if !one_matched {
                return Err(format!("没有找到造 {i} 字词的规则").into());
            }
        }
        Ok(quick_lookup)
    }

    /// 提供配置表示、拆分表、词表和共用资源来创建一个编码引擎
    /// 字需要提供拆分表
    /// 词只需要提供词表，它对应的拆分序列从字推出
    pub fn new(
        representation: &Representation,
        sequence_map: RawSequenceMap,
        words: Vec<String>,
        assets: &Assets,
    ) -> Result<Encoder, Error> {
        let encoder = &representation.config.encoder;
        // 预处理单字拆分表
        let (weighted_sequences, sequence_map) =
            representation.transform_elements(&sequence_map)?;

        // 将拆分序列映射降序排列
        let mut characters_info: Vec<_> = weighted_sequences
            .into_iter()
            .map(|(char, sequence, importance)| {
                let char_frequency = *assets.character_frequency.get(&char).unwrap_or(&0);
                let frequency = char_frequency * importance / 100;
                Encodable {
                    name: char.to_string(),
                    sequence,
                    frequency,
                }
            })
            .collect();
        characters_info.sort_by_key(|x| Reverse(x.frequency));

        // 对词也是一样的操作
        let mut words_info = None;
        let mut all_info = None;
        if let Some(rule) = &encoder.rules {
            let mut words_raw = Self::build_word_sequence(
                rule,
                sequence_map,
                words,
                encoder.max_length,
                &assets.word_frequency,
            )?;
            words_raw.sort_by_key(|x| Reverse(x.frequency));
            let mut all_raw: Vec<_> = characters_info.iter().chain(words_raw.iter()).cloned().collect();
            // 用字词混频中的频率覆盖原来的频率
            for item in all_raw.iter_mut() {
                item.frequency = *assets.frequency.get(&item.name).unwrap_or(&0);
            }
            all_raw.sort_by_key(|x| Reverse(x.frequency));
            words_info = Some(words_raw);
            all_info = Some(all_raw);
        }

        // 处理自动上屏
        let auto_select = representation.transform_auto_select()?;

        // 处理简码规则
        let mut short_code_schemes = None;
        if let Some(schemes) = &encoder.short_code_schemes {
            short_code_schemes = Some(representation.transform_schemes(schemes)?);
        }
        let mut word_short_code_schemes = None;
        if let Some(schemes) = &encoder.word_short_code_schemes {
            word_short_code_schemes = Some(representation.transform_schemes(schemes)?);
        };
        let encoder = Encoder {
            characters_info,
            words_info,
            all_info,
            auto_select,
            config: encoder.clone(),
            radix: representation.radix,
            alphabet_radix: representation.alphabet_radix,
            select_keys: representation.select_keys.clone(),
            short_code_schemes,
            word_short_code_schemes,
        };
        Ok(encoder)
    }

    fn build_word_sequence(
        rules: &Vec<WordRule>,
        sequence_map: SequenceMap,
        words: Vec<String>,
        max_length: usize,
        word_frequency: &HashMap<String, u64>,
    ) -> Result<Vec<Encodable>, Error> {
        // 从词表生成词的拆分序列，滤掉因缺少字的拆分而无法构词的情况
        let mut words_all: Vec<Encodable> = Vec::new();
        let lookup = Self::build_lookup(rules, max_length)?;
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
            if !has_invalid_char {
                let frequency = *word_frequency.get(&word).unwrap_or(&0);
                let encodable = Encodable {
                    name: word,
                    sequence: word_elements,
                    frequency,
                };
                words_all.push(encodable);
            }
        }
        Ok(words_all)
    }

    pub fn encode_full(
        &self,
        keymap: &KeyMap,
        data: &Vec<Encodable>,
        output: &mut Codes,
        occupation: &mut Occupation,
    ) {
        for (encodable, pointer) in zip(data, output) {
            let sequence = &encodable.sequence;
            let mut code = 0_usize;
            let mut weight = 1_usize;
            for element in sequence {
                code += keymap[*element] * weight;
                weight *= self.radix;
            }
            // 全码时，忽略次选及之后的选择键，给所有不能自动上屏的码统一添加首选键
            // 这是为了便于计算重码，否则还要判断
            if !self.auto_select.get(code).unwrap_or(&true) {
                code += self.select_keys[0] * weight;
            }
            *pointer = CodeInfo {
                code,
                duplication: occupation.contains(code),
            };
            occupation.insert(code);
        }
    }

    pub fn encode_short(
        &self,
        full_codes: &Codes,
        short_codes: &mut Codes,
        full_occupation: &Occupation,
        schemes: &Vec<CompiledShortCodeConfig>,
    ) {
        let mut short_occupation = Occupation::new(self.get_space());
        for (code, pointer) in zip(full_codes, short_codes) {
            let full = &code.code;
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
                    let short = if index == 0 && *self.auto_select.get(prefix).unwrap_or(&true) {
                        prefix
                    } else {
                        prefix + key * modulo // 补选择键
                    };
                    // 决定出这个简码
                    if !full_occupation.contains(short) && !short_occupation.contains(short) {
                        short_occupation.insert(short);
                        *pointer = CodeInfo {
                            code: short,
                            duplication: false,
                        };
                        has_reduced = true;
                        break;
                    }
                }
                if has_reduced {
                    break;
                }
            }
            if has_reduced == false {
                *pointer = CodeInfo {
                    code: *full,
                    duplication: short_occupation.contains(*full),
                };
                short_occupation.insert(*full);
            }
        }
    }

    fn signed_index<T: Debug>(vector: &Vec<T>, index: isize) -> &T {
        return if index >= 0 {
            &vector[index as usize]
        } else {
            &vector[vector.len() - (-index as usize)]
        };
    }

    pub fn encode(&self, keymap: &KeyMap, representation: &Representation) -> EncodeExport {
        let mut buffer = Buffer::new(&self);
        let mut occupation = Occupation::new(representation.get_space());
        self.encode_full(keymap, &self.characters_info, &mut buffer.characters_full, &mut occupation);
        if self.short_code_schemes.is_some() {
            self.encode_short(
                &mut buffer.characters_full,
                buffer.characters_short.as_mut().unwrap(),
                &mut occupation,
                self.short_code_schemes.as_ref().unwrap(),
            );
        }
        let mut character_entries: Vec<Entry> = Vec::new();
        for (index, Encodable { name, .. }) in self.characters_info.iter().enumerate() {
            let full = representation.repr_code(buffer.characters_full[index].code);
            let short = buffer
                .characters_short
                .as_ref()
                .map(|x| representation.repr_code(x[index].code));
            character_entries.push(Entry {
                item: name.to_string(),
                full: full.iter().collect(),
                short: short.map(|x| x.iter().collect()),
            });
        }
        let mut word_entries: Option<Vec<Entry>> = None;
        if let Some(words) = self.words_info.as_ref() {
            self.encode_full(keymap, words, buffer.words_full.as_mut().unwrap(), &mut occupation);
            if self.word_short_code_schemes.is_some() {
                self.encode_short(
                    &mut buffer.words_full.as_mut().unwrap(),
                    buffer.words_short.as_mut().unwrap(),
                    &mut occupation,
                    self.word_short_code_schemes.as_ref().unwrap(),
                );
            }
            let entries = words
                .iter()
                .enumerate()
                .map(|(index, Encodable { name, .. })| {
                    let full =
                        representation.repr_code(buffer.words_full.as_ref().unwrap()[index].code);
                    let short = buffer
                        .words_short
                        .as_ref()
                        .map(|x| representation.repr_code(x[index].code));
                    Entry {
                        item: name.to_string(),
                        full: full.iter().collect(),
                        short: short.map(|x| x.iter().collect()),
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

    pub fn get_space(&self) -> usize {
        let max_length = self.config.max_length.min(MAX_COMBINATION_LENGTH);
        self.radix.pow(max_length as u32)
    }
}
