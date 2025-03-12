use crate::representation::{
    Assemble, AssembleList, Code, CodeInfo, CodeSubInfo, Codes, Element, Entry, KeyMap,
    Representation,
};
use crate::Error;
use rustc_hash::FxHashMap;
use std::cmp::Reverse;
use std::iter::zip;

use super::{CompiledScheme, Encodable, Encoder, EncoderConfig, Space};

pub struct DefaultEncoder {
    pub buffer: Codes,
    pub config: EncoderConfig,
    pub full_space: Space,
    pub short_space: Space,
    pub involved_message: Vec<Vec<usize>>,
}

impl DefaultEncoder {
    /// 提供配置表示、拆分表、词表和共用资源来创建一个编码引擎
    /// 字需要提供拆分表
    /// 词只需要提供词表，它对应的拆分序列从字推出
    pub fn new(representation: &Representation, resource: AssembleList) -> Result<Self, Error> {
        let encoder = &representation.config.encoder;
        let max_length = encoder.max_length;
        if max_length >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }

        // 将拆分序列映射降序排列
        let mut encodables = Vec::new();
        for (index, assemble) in resource.into_iter().enumerate() {
            let Assemble {
                name,
                frequency,
                level,
                ..
            } = assemble.clone();
            let sequence = representation.transform_elements(assemble)?;
            encodables.push(Encodable {
                name: name.clone(),
                length: name.chars().count(),
                sequence,
                frequency,
                level,
                index,
            });
        }

        encodables.sort_by_key(|x| Reverse(x.frequency));

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
        let length = config.radix.pow(max_length as u32) as usize;
        let vector = vec![u8::default(); length];
        let hashset = FxHashMap::default();
        let full_space = Space {
            vector: vector.clone(),
            vector_length: length,
            hashmap: hashset.clone(),
        };
        let short_space = Space {
            vector,
            vector_length: length,
            hashmap: hashset,
        };
        let mut involved_message = vec![];
        for _ in 0..=config.elements_length {
            involved_message.push(vec![]);
        }
        for (index, encodable) in config.encodables.iter().enumerate() {
            for element in &encodable.sequence {
                involved_message[*element].push(index);
            }
        }
        let encoder = Self {
            buffer,
            config,
            full_space,
            short_space,
            involved_message,
        };
        Ok(encoder)
    }

    pub fn reset(&mut self) {
        self.full_space.vector.iter_mut().for_each(|x| {
            *x = 0;
        });
        self.full_space.hashmap.clear();
        self.short_space.vector.iter_mut().for_each(|x| {
            *x = 0;
        });
        self.short_space.hashmap.clear();
    }

    fn encode_full(&mut self, keymap: &KeyMap, moved_elements: &Option<Vec<Element>>) {
        let config = &self.config;
        let buffer = &mut self.buffer;
        let weights: Vec<_> = (0..=config.max_length)
            .map(|x| config.radix.pow(x as u32))
            .collect();
        if let Some(moved_elements) = moved_elements {
            for element in moved_elements {
                for index in &self.involved_message[*element] {
                    let pointer = &mut buffer[*index];
                    let encodable = &config.encodables[*index];
                    let sequence = &encodable.sequence;
                    let full = &mut pointer.full;
                    let mut code = 0_u64;
                    for (element, weight) in zip(sequence, &weights) {
                        code += keymap[*element] as u64 * weight;
                    }
                    full.code = code;
                    let actual = config.wrap_actual(code, 0, weights[sequence.len()]);
                    full.check_actual(actual);
                }
            }
        } else {
            for (encodable, pointer) in zip(&config.encodables, buffer.iter_mut()) {
                let sequence = &encodable.sequence;
                let full = &mut pointer.full;
                let mut code = 0_u64;
                for (element, weight) in zip(sequence, &weights) {
                    code += keymap[*element] as u64 * weight;
                }
                // 对于全码，计算实际编码时不考虑第二及以后的选重键
                full.code = code;
                let actual = config.wrap_actual(code, 0, weights[sequence.len()]);
                full.check_actual(actual);
            }
        }

        for pointer in buffer.iter_mut() {
            let full = &mut pointer.full;
            let duplicate = self.full_space.rank(full.code) > 0;
            full.check_duplicate(duplicate);
            self.full_space.insert(full.code);
        }
    }

    fn encode_short(&mut self) {
        let config = &self.config;
        let buffer = &mut self.buffer;
        let weights: Vec<_> = (0..=config.max_length)
            .map(|x| config.radix.pow(x as u32))
            .collect();
        let short_code = config.short_code.as_ref().unwrap();
        // 优先简码
        for (encodable, pointer) in zip(&config.encodables, buffer.iter_mut()) {
            if encodable.level == u64::MAX {
                continue;
            }
            let code = pointer.full.code % weights[encodable.level as usize];
            let rank = self.short_space.rank(code);
            let actual = config.wrap_actual(code, rank, weights[encodable.level as usize]);
            pointer.short.check(actual, rank > 0);
            self.short_space.insert(code);
        }
        // 常规简码
        for (pointer, encodable) in zip(buffer.iter_mut(), &config.encodables) {
            if encodable.level != u64::MAX {
                continue;
            }
            let schemes = &short_code[encodable.length - 1];
            let mut has_short = false;
            let full = &pointer.full;
            let short = &mut pointer.short;
            for scheme in schemes {
                let CompiledScheme {
                    prefix,
                    select_keys,
                } = scheme;
                let weight = weights[*prefix];
                // 如果根本没有这么多码，就放弃
                if full.code < weight {
                    continue;
                }
                // 首先将全码截取一部分出来
                let code = full.code % weight;
                let rank = self.full_space.rank(code) + self.short_space.rank(code);
                if rank >= select_keys.len() as u8 {
                    continue;
                }
                let actual = config.wrap_actual(code, rank, weight);
                short.check(actual, false);
                self.short_space.insert(code);
                has_short = true;
                break;
            }
            if !has_short {
                let code = full.code;
                let rank = self.short_space.rank(full.code);
                short.check(full.actual, rank > 0);
                self.short_space.insert(code);
            }
        }
    }

    pub fn encode(&mut self, representation: &Representation) -> Vec<Entry> {
        let keymap = &representation.initial;
        self.run(keymap, &None);
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
}

impl Encoder for DefaultEncoder {
    fn run(&mut self, keymap: &KeyMap, moved_elements: &Option<Vec<Element>>) {
        self.reset();
        self.encode_full(keymap, moved_elements);
        if self.config.short_code.is_none() || self.config.short_code.as_ref().unwrap().is_empty() {
            return;
        }
        self.encode_short();
    }

    fn get_buffer(&mut self) -> &mut Codes {
        &mut self.buffer
    }
}
