//! 编码引擎

use crate::config::{EncoderConfig, WordRule};
use crate::error::Error;
use crate::representation::{
    Assets, AutoSelect, Buffer, EncodeExport, Entry, Key, KeyMap, Occupation, Representation,
    Resource, Sequence, SequenceMap, MAX_COMBINATION_LENGTH,
};
use std::{cmp::Reverse, collections::HashMap, fmt::Debug, iter::zip};

// 支持二字词直到十字词
const MAX_WORD_LENGTH: usize = 10;

type Lookup = [Vec<(isize, isize)>; MAX_WORD_LENGTH - 1];

/// 一个可编码对象
#[derive(Debug, Clone)]
pub struct Encodable {
    pub name: String,
    pub length: usize,
    pub sequence: Sequence,
    pub frequency: u64,
}

pub struct Encoder {
    pub info: Vec<Encodable>,
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
        resource: Resource,
        assets: &Assets,
    ) -> Result<Encoder, Error> {
        let encoder = &representation.config.encoder;
        // 预处理单字拆分表
        let (weighted_sequences, character_sequence_map) =
            representation.transform_elements(&resource.character_elements)?;

        // 将拆分序列映射降序排列
        let mut info: Vec<_> = weighted_sequences
            .into_iter()
            .map(|(char, sequence, importance)| {
                let char_frequency = *assets.character_frequency.get(&char).unwrap_or(&0);
                let frequency = char_frequency * importance / 100;
                Encodable {
                    name: char.to_string(),
                    length: 1,
                    sequence,
                    frequency,
                }
            })
            .collect();

        // 对词也是一样的操作
        if let Some(rule) = &encoder.rules {
            let words_raw: Vec<Encodable> = if let Some(word_elements) = &resource.word_elements {
                let (weighted_sequences, _) = representation.transform_elements(word_elements)?;
                weighted_sequences
                    .into_iter()
                    .map(|(word, sequence, importance)| {
                        let word_frequency = *assets.word_frequency.get(&word).unwrap_or(&0);
                        let frequency = word_frequency * importance / 100;
                        Encodable {
                            name: word.to_string(),
                            length: word.chars().count(),
                            sequence,
                            frequency,
                        }
                    })
                    .collect()
            } else {
                let result = Self::build_word_sequence(
                    rule,
                    character_sequence_map,
                    resource.words,
                    encoder.max_length,
                    &assets.word_frequency,
                )?;
                result
            };
            info.extend(words_raw.into_iter());
        }
        info.sort_by_key(|x| Reverse(x.frequency));

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
            info,
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
                if let Some(sequence) = sequence_map.get(&char.to_string()) {
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
                    length: chars.len(),
                    sequence: word_elements,
                    frequency,
                };
                words_all.push(encodable);
            }
        }
        Ok(words_all)
    }

    pub fn encode_full(&self, keymap: &KeyMap, buffer: &mut Buffer, occupation: &mut Occupation) {
        for (encodable, pointer) in zip(&self.info, &mut buffer.full) {
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
            pointer.code = code;
            pointer.duplication = occupation.contains(code);
            occupation.insert(code);
        }
    }

    pub fn encode_short(&self, buffer: &mut Buffer, full_occupation: &Occupation) {
        let mut short_occupation = Occupation::new(self.get_space());
        for ((code, pointer), encodable) in zip(zip(&buffer.full, &mut buffer.short), &self.info) {
            let full = &code.code;
            let mut has_reduced = false;
            let schemes = if encodable.length == 1 {
                &self.short_code_schemes
            } else {
                &self.word_short_code_schemes
            };
            if schemes.is_none() {
                continue;
            }
            let schemes = schemes.as_ref().unwrap();
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
                        pointer.code = short;
                        pointer.duplication = false;
                        has_reduced = true;
                        break;
                    }
                }
                if has_reduced {
                    break;
                }
            }
            if has_reduced == false {
                pointer.code = *full;
                pointer.duplication = short_occupation.contains(*full);
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

    pub fn split(&self, buffer: &mut Buffer) {
        let mut i_characters = 0;
        let mut i_words = 0;
        for (encodable, pointer) in zip(&self.info, &buffer.full) {
            if encodable.length == 1 {
                buffer.characters_full[i_characters] = *pointer;
                i_characters += 1;
            } else {
                buffer.words_full[i_words] = *pointer;
                i_words += 1;
            }
        }
        let mut i_characters = 0;
        let mut i_words = 0;
        for (encodable, pointer) in zip(&self.info, &buffer.short) {
            if encodable.length == 1 {
                buffer.characters_short[i_characters] = *pointer;
                i_characters += 1;
            } else {
                buffer.words_short[i_words] = *pointer;
                i_words += 1;
            }
        }
    }

    pub fn encode(&self, keymap: &KeyMap, representation: &Representation) -> EncodeExport {
        let mut buffer = Buffer::new(&self);
        let mut occupation = Occupation::new(representation.get_space());
        self.encode_full(keymap, &mut buffer, &mut occupation);
        self.encode_short(&mut buffer, &mut occupation);
        self.split(&mut buffer);
        let characters_info: Vec<_> = self.info.iter().filter(|x| x.length == 1).collect();
        let mut character_entries = Entry {
            item: characters_info.iter().map(|x| x.name.to_string()).collect(),
            full: buffer
                .characters_full
                .iter()
                .map(|x| representation.repr_code(x.code).iter().collect())
                .collect(),
            short: None,
        };
        if self.short_code_schemes.is_some() {
            character_entries.short = Some(buffer
                .characters_short
                .iter()
                .map(|x| representation.repr_code(x.code).iter().collect())
                .collect());
        }
        let words_info: Vec<_> = self.info.iter().filter(|x| x.length > 1).collect();
        let mut word_entries = Entry {
            item: words_info.iter().map(|x| x.name.to_string()).collect(),
            full: buffer
                .words_full
                .iter()
                .map(|x| representation.repr_code(x.code).iter().collect())
                .collect(),
            short: None
        };
        if self.word_short_code_schemes.is_some() {
            word_entries.short = Some(buffer
                .words_short
                .iter()
                .map(|x| representation.repr_code(x.code).iter().collect())
                .collect());
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
