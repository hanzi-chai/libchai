use super::{CompiledScheme, Driver, EncoderConfig};
use crate::representation::{Code, CodeSubInfo, Codes, KeyMap};
use rustc_hash::FxHashMap;
use std::iter::zip;

#[derive(Default, Clone)]
pub struct Slot {
    pub hash: u16,
    pub count: u8,
}

/// 编码是否已被占据
/// 用一个数组和一个哈希集合来表示，数组用来表示四码以内的编码，哈希集合用来表示四码以上的编码
pub struct Occupation {
    pub vector: Vec<Slot>,
    pub hashmap: FxHashMap<Code, u8>,
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

    pub fn reset(&mut self) {
        self.vector.iter_mut().for_each(|x| {
            x.count = 0;
            x.hash = 0;
        });
        self.hashmap.clear();
    }

    pub fn insert(&mut self, index: u64, hash: u16) {
        if index < self.vector.len() as u64 {
            let index = index as usize;
            self.vector[index].hash = hash;
            self.vector[index].count = self.vector[index].count.saturating_add(1);
        } else {
            self.hashmap
                .insert(index, self.hashmap.get(&index).unwrap_or(&0) + 1);
        }
    }

    pub fn remove(&mut self, index: u64) {
        if index < self.vector.len() as u64 {
            let index = index as usize;
            self.vector[index].count = self.vector[index].count.saturating_sub(1);
        } else {
            self.hashmap.insert(
                index,
                self.hashmap.get(&index).unwrap_or(&0).saturating_sub(1),
            );
        }
    }

    pub fn rank(&self, index: u64) -> u8 {
        if index < self.vector.len() as u64 {
            let index = index as usize;
            self.vector[index].count
        } else {
            *self.hashmap.get(&index).unwrap_or(&0)
        }
    }

    pub fn rank_hash(&self, index: u64, hash: u16) -> u8 {
        if index < self.vector.len() as u64 {
            let index = index as usize;
            if self.vector[index].hash == hash {
                0
            } else {
                self.vector[index].count
            }
        } else {
            *self.hashmap.get(&index).unwrap_or(&0)
        }
    }
}

impl Driver for Occupation {
    fn encode_full(&mut self, keymap: &KeyMap, config: &EncoderConfig, full: &mut Codes) {
        self.reset();
        let weights: Vec<_> = (0..=config.max_length)
            .map(|x| config.radix.pow(x as u32))
            .collect();
        for (encodable, pointer) in zip(&config.encodables, full) {
            let sequence = &encodable.sequence;
            let mut code = 0_u64;
            for (element, weight) in zip(sequence, &weights) {
                code += keymap[*element] as u64 * weight;
            }
            let rank = self.rank(code);
            pointer.full = CodeSubInfo {
                code,
                rank,
                actual: config.wrap_actual(code, 0, weights[sequence.len()]),
                duplicate: rank > 0,
            };
            self.insert(code, encodable.hash);
        }
    }

    fn encode_short(&mut self, config: &EncoderConfig, buffer: &mut Codes) {
        if config.short_code.is_none() || config.short_code.as_ref().unwrap().is_empty() {
            return;
        }
        let weights: Vec<_> = (0..=config.max_length)
            .map(|x| config.radix.pow(x as u32))
            .collect();
        let short_code = config.short_code.as_ref().unwrap();
        // 优先简码
        for (pointer, encodable) in zip(buffer.iter_mut(), &config.encodables) {
            if encodable.level == u64::MAX {
                continue;
            }
            let modulo = config.radix.pow(encodable.level as u32);
            let short = pointer.full.code % modulo;
            let rank = self.rank_hash(short, encodable.hash);
            pointer.short = CodeSubInfo {
                code: short,
                rank,
                actual: config.wrap_actual(short, rank, modulo),
                duplicate: false,
            };
            self.insert(short, encodable.hash);
        }
        // 常规简码
        for (pointer, encodable) in zip(buffer.iter_mut(), &config.encodables) {
            let schemes = &short_code[encodable.length - 1];
            if schemes.is_empty() || encodable.level != u64::MAX {
                continue;
            }
            let mut has_reduced = false;
            let hash = encodable.hash;
            for scheme in schemes {
                let CompiledScheme {
                    prefix,
                    select_keys,
                } = scheme;
                let weight = weights[*prefix];
                // 如果根本没有这么多码，就放弃
                if pointer.full.code < weight {
                    continue;
                }
                // 首先将全码截取一部分出来
                let short = pointer.full.code % weight;
                let rank = self.rank_hash(short, hash);
                if rank >= select_keys.len() as u8 {
                    continue;
                }
                pointer.short = CodeSubInfo {
                    code: short,
                    rank: self.rank_hash(short, hash),
                    actual: config.wrap_actual(short, rank, weight),
                    duplicate: rank > 0,
                };
                self.insert(short, hash);
                self.remove(pointer.full.code);
                has_reduced = true;
                break;
            }
            if !has_reduced {
                let rank = self.rank_hash(pointer.full.code, hash);
                pointer.short = CodeSubInfo {
                    code: pointer.full.code,
                    rank,
                    actual: pointer.full.actual,
                    duplicate: pointer.full.duplicate,
                };
            }
        }
    }
}
