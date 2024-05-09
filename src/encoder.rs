//! 编码引擎

use rustc_hash::FxHashMap;

use crate::config::EncoderConfig;
use crate::error::Error;
use crate::representation::{
    Assemble, AssembleList, Assets, AutoSelect, Buffer, EncodeExport, Entry, Frequency, Key,
    KeyMap, Occupation, Representation, Sequence, MAX_COMBINATION_LENGTH,
};
use std::collections::HashSet;
use std::{cmp::Reverse, fmt::Debug, iter::zip};

/// 一个可编码对象
#[derive(Debug, Clone)]
pub struct Encodable {
    pub name: String,
    pub length: usize,
    pub sequence: Sequence,
    pub frequency: u64,
    pub level: i64,
}

pub struct Encoder {
    pub encodables: Vec<Encodable>,
    pub transition_matrix: Vec<Vec<(usize, u64)>>,
    config: EncoderConfig,
    auto_select: AutoSelect,
    pub radix: usize,
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
    pub fn adapt(frequency: &Frequency, words: &HashSet<String>) -> (Frequency, Vec<(String, String, u64)>) {
        let mut new_frequency = Frequency::new();
        let mut transition_pairs = Vec::new();
        for (word, value) in frequency {
            if words.contains(word) {
                new_frequency.insert(word.clone(), new_frequency.get(word).unwrap_or(&0) + *value);
            } else {
                // 使用逆向最大匹配算法来分词
                let chars: Vec<_> = word.chars().collect();
                let mut end = chars.len();
                let mut start;
                let mut last_match: Option<String> = None;
                while end > 0 {
                    start = end - 1;
                    let partial_word: String = chars[start..end].iter().collect();
                    // 如果最后一个字不在词表里，就不要了
                    if !words.contains(&partial_word) {
                        end -= 1;
                        continue;
                    }
                    // 继续向前匹配，看看是否能匹配到更长的词
                    while start > 0 {
                        start -= 1;
                        let partial_word: String = chars[start..end].iter().collect();
                        if !words.contains(&partial_word) {
                            start += 1;
                            break;
                        }
                    }
                    // 确定最大匹配
                    let maximum_match: String = chars[start..end].iter().collect();
                    assert!(
                        words.contains(&maximum_match),
                        "词表里没有 {:} {:} {:?}",
                        start,
                        end,
                        maximum_match
                    );
                    new_frequency.insert(
                        maximum_match.clone(),
                        new_frequency.get(&maximum_match).unwrap_or(&0) + *value,
                    );
                    if let Some(last) = last_match {
                        transition_pairs.push((maximum_match.clone(), last, *value));
                    }
                    last_match = Some(maximum_match);
                    end = start;
                }
            }
        }
        (new_frequency, transition_pairs)
    }

    /// 提供配置表示、拆分表、词表和共用资源来创建一个编码引擎
    /// 字需要提供拆分表
    /// 词只需要提供词表，它对应的拆分序列从字推出
    pub fn new(
        representation: &Representation,
        info: AssembleList,
        assets: &Assets,
    ) -> Result<Encoder, Error> {
        let encoder = &representation.config.encoder;
        let max_length = encoder.max_length;
        if max_length >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }

        // 预处理词频
        let all_words: HashSet<_> = info.iter().map(|x| x.name.clone()).collect();
        let (frequency, transition_pairs) = Self::adapt(&assets.frequency, &all_words);

        // 将拆分序列映射降序排列
        let mut encodables = Vec::new();
        for assemble in info {
            let Assemble {
                name,
                importance,
                level,
                ..
            } = assemble.clone();
            let sequence = representation.transform_elements(assemble)?;
            let char_frequency = *frequency.get(&name).unwrap_or(&0);
            let frequency = char_frequency * importance / 100;
            encodables.push(Encodable {
                name: name.clone(),
                length: name.chars().count(),
                sequence,
                frequency,
                level,
            });
        }

        encodables.sort_by_key(|x| Reverse(x.frequency));
        
        let map_word_to_index: FxHashMap<String, usize> = encodables
            .iter()
            .enumerate()
            .map(|(index, x)| (x.name.clone(), index))
            .collect();
        let mut transition_matrix = vec![vec![]; encodables.len()];
        for (from, to, value) in transition_pairs {
            let from = *map_word_to_index.get(&from).unwrap();
            let to = *map_word_to_index.get(&to).unwrap();
            transition_matrix[from].push((to, value));
        }
        for row in transition_matrix.iter_mut() {
            row.sort_by_key(|x| x.0);
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
            encodables,
            transition_matrix,
            auto_select,
            config: encoder.clone(),
            radix: representation.radix,
            select_keys: representation.select_keys.clone(),
            short_code_schemes,
            word_short_code_schemes,
        };
        Ok(encoder)
    }

    pub fn encode_full(&self, keymap: &KeyMap, buffer: &mut Buffer, occupation: &mut Occupation) {
        for (encodable, pointer) in zip(&self.encodables, &mut buffer.full) {
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
        // 优先简码
        for ((code, pointer), encodable) in zip(zip(&buffer.full, &mut buffer.short), &self.encodables) {
            if encodable.level == -1 {
                continue;
            }
            let full = &code.code;
            let modulo = self.radix.pow(encodable.level as u32);
            let prefix = full % modulo;
            let key = self.select_keys[0];
            let short = if *self.auto_select.get(prefix).unwrap_or(&true) {
                prefix
            } else {
                prefix + key * modulo
            };
            pointer.code = short;
            pointer.duplication = false;
        }
        // 常规简码
        for ((code, pointer), encodable) in zip(zip(&buffer.full, &mut buffer.short), &self.encodables) {
            if encodable.level >= 0 {
                continue;
            }
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

    pub fn encode(&self, keymap: &KeyMap, representation: &Representation) -> EncodeExport {
        let mut buffer = Buffer::new(&self);
        let mut occupation = Occupation::new(representation.get_space());
        self.encode_full(keymap, &mut buffer, &mut occupation);
        self.encode_short(&mut buffer, &mut occupation);
        let characters_info: Vec<_> = self.encodables.iter().filter(|x| x.length == 1).collect();
        let mut character_entries = Entry {
            item: characters_info.iter().map(|x| x.name.to_string()).collect(),
            full: buffer
                .full
                .iter()
                .filter(|x| x.single)
                .map(|x| representation.repr_code(x.code).iter().collect())
                .collect(),
            short: None,
        };
        if self.short_code_schemes.is_some() {
            character_entries.short = Some(
                buffer
                    .short
                    .iter()
                    .filter(|x| x.single)
                    .map(|x| representation.repr_code(x.code).iter().collect())
                    .collect(),
            );
        }
        let words_info: Vec<_> = self.encodables.iter().filter(|x| x.length > 1).collect();
        let mut word_entries = Entry {
            item: words_info.iter().map(|x| x.name.to_string()).collect(),
            full: buffer
                .full
                .iter()
                .filter(|x| !x.single)
                .map(|x| representation.repr_code(x.code).iter().collect())
                .collect(),
            short: None,
        };
        if self.word_short_code_schemes.is_some() {
            word_entries.short = Some(
                buffer
                    .short
                    .iter()
                    .filter(|x| !x.single)
                    .map(|x| representation.repr_code(x.code).iter().collect())
                    .collect(),
            );
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
