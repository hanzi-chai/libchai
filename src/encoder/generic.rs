
use rustc_hash::{FxHashMap, FxHashSet};
use super::{CompiledScheme, Encodable, Encoder};

use crate::config::EncoderConfig;
use crate::error::Error;
use crate::representation::{
    Assemble, AssembleList, Assets, AutoSelect, Buffer, Code, Entry, Key, KeyMap,
    Representation, MAX_COMBINATION_LENGTH, MAX_WORD_LENGTH,
};
use std::{cmp::Reverse, iter::zip};


pub struct GenericEncoder {
    pub encodables: Vec<Encodable>,
    pub transition_matrix: Vec<Vec<(usize, u64)>>,
    pub config: EncoderConfig,
    auto_select: AutoSelect,
    pub radix: u64,
    select_keys: Vec<Key>,
    pub short_code: Option<[Vec<CompiledScheme>; MAX_WORD_LENGTH]>,
}

impl Encoder for GenericEncoder {
    fn get_actual_code(&self, code: u64, rank: i8, length: u32) -> (u64, u32) {
        if rank == 0 && *self.auto_select.get(code as usize).unwrap_or(&true) {
            return (code, length);
        }
        let select = *self
            .select_keys
            .get(rank.unsigned_abs() as usize)
            .unwrap_or(&self.select_keys[0]) as u64
            * self.radix.pow(length);
        (code + select, length + 1)
    }

    fn encode_full(&self, keymap: &KeyMap, buffer: &mut Buffer) {
        let weights: Vec<_> = (0..=self.config.max_length)
            .map(|x| self.radix.pow(x as u32))
            .collect();
        for (encodable, pointer) in zip(&self.encodables, &mut buffer.full) {
            let sequence = &encodable.sequence;
            let mut code = 0_u64;
            for (element, weight) in zip(sequence, &weights) {
                code += keymap[*element] as u64 * weight;
            }
            pointer.code = code;
            pointer.rank = buffer.occupation.rank(code) as i8;
            buffer.occupation.insert(code, encodable.hash);
        }
    }

    fn encode_short(&self, buffer: &mut Buffer) {
        if self.short_code.is_none() {
            return;
        }
        let short_code = self.short_code.as_ref().unwrap();
        // 优先简码
        for ((code, pointer), encodable) in
            zip(zip(&buffer.full, &mut buffer.short), &self.encodables)
        {
            if encodable.level == u64::MAX {
                continue;
            }
            let modulo = self.radix.pow(encodable.level as u32);
            let short = code.code % modulo;
            pointer.code = short;
            pointer.rank = 0;
            buffer.occupation.insert(short, encodable.hash);
        }
        // 常规简码
        for ((code, pointer), encodable) in
            zip(zip(&buffer.full, &mut buffer.short), &self.encodables)
        {
            let schemes = &short_code[encodable.length - 1];
            if schemes.is_empty() || encodable.level != u64::MAX {
                continue;
            }
            let full = &code.code;
            let mut has_reduced = false;
            let hash = encodable.hash;
            for scheme in schemes {
                let CompiledScheme {
                    prefix,
                    select_keys,
                } = scheme;
                // 如果根本没有这么多码，就放弃
                if *full < self.radix.pow((*prefix - 1) as u32) {
                    continue;
                }
                // 首先将全码截取一部分出来
                let modulo = self.radix.pow(*prefix as u32);
                let short = full % modulo;
                let capacity = select_keys.len() as u8;
                if buffer.occupation.rank_hash(short, hash) >= capacity {
                    continue;
                }
                pointer.code = short;
                pointer.rank = buffer.occupation.rank_hash(short, hash) as i8;
                buffer.occupation.insert(short, hash);
                has_reduced = true;
                break;
            }
            if !has_reduced {
                pointer.code = *full;
                pointer.rank = buffer.occupation.rank_hash(*full, hash) as i8;
                buffer.occupation.insert(*full, hash);
            }
        }
    }

    fn get_space(&self) -> usize {
        let max_length = self.config.max_length.min(MAX_COMBINATION_LENGTH);
        self.radix.pow(max_length as u32) as usize
    }

    fn get_radix(&self) -> u64 {
        self.radix
    }

    fn get_transitions(&self, index: usize) -> &[(usize, u64)] {
        &self.transition_matrix[index]
    }
}

impl GenericEncoder {
    /// 提供配置表示、拆分表、词表和共用资源来创建一个编码引擎
    /// 字需要提供拆分表
    /// 词只需要提供词表，它对应的拆分序列从字推出
    pub fn new(
        representation: &Representation,
        resource: AssembleList,
        assets: &Assets,
    ) -> Result<GenericEncoder, Error> {
        let encoder = &representation.config.encoder;
        let max_length = encoder.max_length;
        if max_length >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }

        // 预处理词频
        let all_words: FxHashSet<_> = resource.iter().map(|x| x.name.clone()).collect();
        let (frequency, transition_pairs) = super::adapt(&assets.frequency, &all_words);

        // 将拆分序列映射降序排列
        let mut encodables = Vec::new();
        for (index, assemble) in resource.into_iter().enumerate() {
            let Assemble {
                name,
                importance,
                level,
                ..
            } = assemble.clone();
            let sequence = representation.transform_elements(assemble)?;
            let char_frequency = *frequency.get(&name).unwrap_or(&0);
            let frequency = char_frequency * importance / 100;
            let hash: u16 = (name.chars().map(|x| x as u32).sum::<u32>()) as u16;
            encodables.push(Encodable {
                name: name.clone(),
                length: name.chars().count(),
                sequence,
                frequency,
                level,
                hash,
                index,
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
        let mut short_code = None;
        if let Some(configs) = &encoder.short_code {
            short_code = Some(representation.transform_short_code(configs.clone())?);
        }
        let encoder = GenericEncoder {
            encodables,
            transition_matrix,
            auto_select,
            config: encoder.clone(),
            radix: representation.radix,
            select_keys: representation.select_keys.clone(),
            short_code,
        };
        Ok(encoder)
    }

    pub fn encode(&self, keymap: &KeyMap, representation: &Representation) -> Vec<Entry> {
        let mut buffer = Buffer::new(&self.encodables, self.get_space());
        self.encode_full(keymap, &mut buffer);
        self.encode_short(&mut buffer);
        let mut entries: Vec<(usize, Entry)> = Vec::new();
        let recover = |code: Code| representation.repr_code(code).iter().collect();
        for (index, encodable) in self.encodables.iter().enumerate() {
            let entry = Entry {
                name: encodable.name.clone(),
                full: recover(buffer.full[index].code),
                full_rank: buffer.full[index].rank,
                short: recover(buffer.short[index].code),
                short_rank: buffer.short[index].rank,
            };
            entries.push((encodable.index, entry));
        }
        entries.sort_by_key(|x| x.0);
        entries.into_iter().map(|x| x.1).collect()
    }
}