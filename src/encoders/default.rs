use super::{CompiledScheme, Encodable, Encoder, EncoderConfig, Space};
use crate::representation::{CodeInfo, Codes, Element, KeyMap, RawEncodable, Representation};
use crate::Error;
use rustc_hash::FxHashMap;
use std::iter::zip;

pub struct DefaultEncoder {
    pub buffer: Codes,
    pub config: EncoderConfig,
    pub encodables: Vec<Encodable>,
    pub full_space: Space,
    pub short_space: Space,
    pub involved_message: Vec<Vec<usize>>,
}

impl DefaultEncoder {
    /// 提供配置表示、拆分表、词表和共用资源来创建一个编码引擎
    /// 字需要提供拆分表
    /// 词只需要提供词表，它对应的拆分序列从字推出
    pub fn new(
        representation: &Representation,
        raw_encodables: Vec<RawEncodable>,
    ) -> Result<Self, Error> {
        let encoder = &representation.config.encoder;
        let max_length = encoder.max_length;
        if max_length >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }
        let encodables = representation.transform_encodables(raw_encodables)?;
        let buffer = encodables.iter().map(CodeInfo::new).collect();
        let vector_length = representation.radix.pow(max_length as u32) as usize;
        let vector = vec![u8::default(); vector_length];
        let full_space = Space {
            vector,
            vector_length,
            hashmap: FxHashMap::default(),
        };
        let short_space = full_space.clone();
        let mut involved_message = vec![];
        for _ in 0..=representation.element_repr.len() {
            involved_message.push(vec![]);
        }
        for (index, encodable) in encodables.iter().enumerate() {
            for element in &encodable.sequence {
                involved_message[*element].push(index);
            }
        }
        let config = EncoderConfig::new(&representation)?;
        let encoder = Self {
            buffer,
            config,
            encodables,
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
                    let encodable = &self.encodables[*index];
                    let sequence = &encodable.sequence;
                    let full = &mut pointer.full;
                    let mut code = 0_u64;
                    for (element, weight) in zip(sequence, &weights) {
                        code += keymap[*element] as u64 * weight;
                    }
                    full.primitive = code;
                    let actual = config.wrap_actual(code, 0, weights[sequence.len()]);
                    full.set_code(actual);
                }
            }
        } else {
            for (encodable, pointer) in zip(&self.encodables, buffer.iter_mut()) {
                let sequence = &encodable.sequence;
                let full = &mut pointer.full;
                let mut code = 0_u64;
                for (element, weight) in zip(sequence, &weights) {
                    code += keymap[*element] as u64 * weight;
                }
                // 对于全码，计算实际编码时不考虑第二及以后的选重键
                full.primitive = code;
                let actual = config.wrap_actual(code, 0, weights[sequence.len()]);
                full.set_code(actual);
            }
        }

        for pointer in buffer.iter_mut() {
            let full = &mut pointer.full;
            let duplicate = self.full_space.rank(full.primitive) > 0;
            full.set_duplicate(duplicate);
            self.full_space.insert(full.primitive);
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
        for (encodable, pointer) in zip(&self.encodables, buffer.iter_mut()) {
            if encodable.level == u64::MAX {
                continue;
            }
            let code = pointer.full.primitive % weights[encodable.level as usize];
            let rank = self.short_space.rank(code);
            let actual = config.wrap_actual(code, rank, weights[encodable.level as usize]);
            pointer.short.set(actual, rank > 0);
            self.short_space.insert(code);
        }
        // 常规简码
        for (pointer, encodable) in zip(buffer.iter_mut(), &self.encodables) {
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
                if full.primitive < weight {
                    continue;
                }
                // 首先将全码截取一部分出来
                let code = full.primitive % weight;
                let rank = self.full_space.rank(code) + self.short_space.rank(code);
                if rank >= select_keys.len() as u8 {
                    continue;
                }
                let actual = config.wrap_actual(code, rank, weight);
                short.set(actual, false);
                self.short_space.insert(code);
                has_short = true;
                break;
            }
            if !has_short {
                let code = full.primitive;
                let rank = self.short_space.rank(full.primitive);
                short.set(full.code, rank > 0);
                self.short_space.insert(code);
            }
        }
    }
}

impl Encoder for DefaultEncoder {
    fn encode(&mut self, keymap: &KeyMap, moved_elements: &Option<Vec<Element>>) -> &mut Codes {
        self.reset();
        self.encode_full(keymap, moved_elements);
        if self.config.short_code.is_none() || self.config.short_code.as_ref().unwrap().is_empty() {
            return &mut self.buffer;
        }
        self.encode_short();
        &mut self.buffer
    }
}
