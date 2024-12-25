//! 编码引擎

use crate::Error;
use crate::representation::{
    Assemble, AssembleList, Assets, AutoSelect, Code, CodeInfo, CodeSubInfo, Codes, Element, Entry,
    Frequency, Key, KeyMap, Representation, Sequence, MAX_WORD_LENGTH,
};
use c3::C3;
use occupation::Occupation;
use rustc_hash::{FxHashMap, FxHashSet};
use simple_occupation::SimpleOccupation;
use std::cmp::Reverse;

pub mod c3;
pub mod occupation;
pub mod simple_occupation;

/// 一个可编码对象
#[derive(Debug, Clone)]
pub struct Encodable {
    pub name: String,
    pub length: usize,
    pub sequence: Sequence,
    pub frequency: u64,
    pub level: u64,
    pub hash: u16,
    pub index: usize,
}

#[derive(Debug)]
pub struct CompiledScheme {
    pub prefix: usize,
    pub select_keys: Vec<usize>,
}

pub fn adapt(
    frequency: &Frequency,
    words: &FxHashSet<String>,
) -> (Frequency, Vec<(String, String, u64)>) {
    let mut new_frequency = Frequency::new();
    let mut transition_pairs = Vec::new();
    for (word, value) in frequency {
        if words.contains(word) {
            new_frequency.insert(word.clone(), new_frequency.get(word).unwrap_or(&0) + *value);
        } else {
            // 使用逆向最大匹配算法来分词
            let chars: Vec<_> = word.chars().collect();
            let mut end = chars.len();
            let mut last_match: Option<String> = None;
            while end > 0 {
                let mut start = end - 1;
                // 如果最后一个字不在词表里，就不要了
                if !words.contains(&chars[start].to_string()) {
                    end -= 1;
                    continue;
                }
                // 继续向前匹配，看看是否能匹配到更长的词
                while start > 0
                    && words.contains(&chars[(start - 1)..end].iter().collect::<String>())
                {
                    start -= 1;
                }
                // 确定最大匹配
                let sub_word: String = chars[start..end].iter().collect();
                *new_frequency.entry(sub_word.clone()).or_default() += *value;
                if let Some(last) = last_match {
                    transition_pairs.push((sub_word.clone(), last, *value));
                }
                last_match = Some(sub_word);
                end = start;
            }
        }
    }
    (new_frequency, transition_pairs)
}

pub trait Driver {
    fn init(&mut self, _config: &EncoderConfig, _representation: &Representation) {}

    fn run(
        &mut self,
        keymap: &KeyMap,
        config: &EncoderConfig,
        buffer: &mut Codes,
        moved_elements: &[Element],
    );
}

pub struct EncoderConfig {
    pub radix: u64,
    pub max_length: usize,
    pub auto_select: AutoSelect,
    pub select_keys: Vec<Key>,
    pub first_key: Key,
    pub short_code: Option<[Vec<CompiledScheme>; MAX_WORD_LENGTH]>,
    pub encodables: Vec<Encodable>,
    pub elements_length: usize,
}

impl EncoderConfig {
    #[inline(always)]
    pub fn wrap_actual(&self, code: u64, rank: u8, weight: u64) -> u64 {
        if rank == 0 {
            if *self.auto_select.get(code as usize).unwrap_or(&true) {
                return code;
            } else {
                return code + (self.first_key as u64) * weight;
            }
        }
        let select = *self
            .select_keys
            .get(rank as usize)
            .unwrap_or(&self.select_keys[0]) as u64;
        code + select * weight
    }
}

pub struct Encoder {
    pub buffer: Codes,
    pub transition_matrix: Vec<Vec<(usize, u64)>>,
    pub config: EncoderConfig,
    driver: Box<dyn Driver>,
    default_driver: Occupation,
}

impl Encoder {
    /// 提供配置表示、拆分表、词表和共用资源来创建一个编码引擎
    /// 字需要提供拆分表
    /// 词只需要提供词表，它对应的拆分序列从字推出
    pub fn new(
        representation: &Representation,
        resource: AssembleList,
        assets: &Assets,
    ) -> Result<Self, Error> {
        let encoder = &representation.config.encoder;
        let max_length = encoder.max_length;
        if max_length >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }

        // 预处理词频
        let all_words: FxHashSet<_> = resource.iter().map(|x| x.name.clone()).collect();
        let (frequency, transition_pairs) = adapt(&assets.frequency, &all_words);

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
        let buffer = encodables
            .iter()
            .map(|x| CodeInfo {
                frequency: x.frequency,
                length: x.name.chars().count(),
                full: CodeSubInfo::default(),
                short: CodeSubInfo::default(),
            })
            .collect();
        let config = EncoderConfig {
            encodables,
            auto_select,
            max_length,
            radix: representation.radix,
            select_keys: representation.select_keys.clone(),
            first_key: representation.select_keys[0],
            elements_length: representation.element_repr.len(),
            short_code,
        };
        let mut driver: Box<dyn Driver> = if representation.config.info.name == "c3" {
            Box::new(C3::new(representation.get_space()))
        } else if representation.config.encoder.max_length <= 4 {
            Box::new(SimpleOccupation::new(representation.get_space()))
        } else {
            Box::new(Occupation::new(representation.get_space()))
        };
        driver.init(&config, representation);
        let encoder = Self {
            transition_matrix,
            buffer,
            config,
            driver,
            default_driver: Occupation::new(representation.get_space()),
        };
        Ok(encoder)
    }

    pub fn init(&mut self, keymap: &KeyMap) {
        self.default_driver
            .run(keymap, &self.config, &mut self.buffer, &[]);
    }

    pub fn prepare(&mut self, keymap: &KeyMap, moved_elements: &[Element]) {
        self.driver
            .run(keymap, &self.config, &mut self.buffer, moved_elements);
    }

    pub fn encode(&mut self, keymap: &KeyMap, representation: &Representation) -> Vec<Entry> {
        self.init(keymap);
        let mut entries: Vec<(usize, Entry)> = Vec::new();
        let recover = |code: Code| representation.repr_code(code).iter().collect();
        let buffer = &self.buffer;
        for (index, encodable) in self.config.encodables.iter().enumerate() {
            let entry = Entry {
                name: encodable.name.clone(),
                full: recover(buffer[index].full.code),
                full_rank: buffer[index].full.rank,
                short: recover(buffer[index].short.code),
                short_rank: buffer[index].short.rank,
            };
            entries.push((encodable.index, entry));
        }
        entries.sort_by_key(|x| x.0);
        entries.into_iter().map(|x| x.1).collect()
    }

    pub fn generate_first_order_duplication(&self, candidate: Vec<Key>) -> Vec<Vec<(u64, u64)>> {
        type Hasher = [Element; 4];
        let mut res_map: FxHashMap<Hasher, FxHashMap<Element, u64>> = FxHashMap::default();
        for encodable in &self.config.encodables {
            let sequence = encodable.sequence.clone();
            for index in 0..sequence.len() {
                let mut tuple: [Element; 4] = [0; 4];
                for (i, ptr) in tuple.iter_mut().enumerate() {
                    *ptr = sequence[i];
                }
                tuple[index] = 0;
                res_map
                    .entry(tuple)
                    .and_modify(|m| {
                        if !m.contains_key(&sequence[index]) {
                            m.insert(sequence[index], encodable.frequency);
                        }
                    })
                    .or_insert_with(|| {
                        let mut map = FxHashMap::default();
                        map.insert(sequence[index], encodable.frequency);
                        map
                    });
            }
        }
        let mut result = vec![vec![(0, 0); candidate.len()]; candidate.len()];
        for (_key, value) in res_map.iter() {
            for (i, (ei, fi)) in value.iter().enumerate() {
                for (j, (ej, fj)) in value.iter().enumerate() {
                    if i >= j {
                        continue;
                    }
                    result[*ei][*ej].0 += 1;
                    result[*ei][*ej].1 += fi.min(fj);
                }
            }
        }
        result
    }
}
