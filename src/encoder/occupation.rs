use super::{CompiledScheme, Driver, EncoderConfig};
use crate::representation::{Code, Codes, Element, KeyMap};
use rustc_hash::FxHashMap;
use std::iter::zip;

#[derive(Default, Clone)]
pub struct Slot {
    pub hash: u16,
    pub count: u8,
}

pub struct Space {
    pub vector: Vec<Slot>,
    pub hashmap: FxHashMap<Code, u8>,
}

impl Space {
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

/// 编码是否已被占据
/// 用一个数组和一个哈希集合来表示，数组用来表示四码以内的编码，哈希集合用来表示四码以上的编码
pub struct Occupation {
    pub full_space: Space,
    pub short_space: Space,
}

impl Occupation {
    pub fn new(length: usize) -> Self {
        let vector = vec![Slot::default(); length];
        let hashset = FxHashMap::default();
        Self {
            full_space: Space {
                vector: vector.clone(),
                hashmap: hashset.clone(),
            },
            short_space: Space {
                vector,
                hashmap: hashset,
            },
        }
    }

    pub fn reset(&mut self) {
        self.full_space.vector.iter_mut().for_each(|x| {
            x.count = 0;
            x.hash = 0;
        });
        self.full_space.hashmap.clear();
        self.short_space.vector.iter_mut().for_each(|x| {
            x.count = 0;
            x.hash = 0;
        });
        self.short_space.hashmap.clear();
    }

    fn encode_full(&mut self, keymap: &KeyMap, config: &EncoderConfig, buffer: &mut Codes) {
        let weights: Vec<_> = (0..=config.max_length)
            .map(|x| config.radix.pow(x as u32))
            .collect();
        for (encodable, pointer) in zip(&config.encodables, buffer.iter_mut()) {
            let sequence = &encodable.sequence;
            let full = &mut pointer.full;
            let mut code = 0_u64;
            for (element, weight) in zip(sequence, &weights) {
                code += keymap[*element] as u64 * weight;
            }
            let rank = self.full_space.rank_hash(code, encodable.hash);
            // 对于全码，计算实际编码时不考虑第二及以后的选重键
            full.code = code;
            full.rank = rank;
            full.actual = config.wrap_actual(code, 0, weights[sequence.len()]);
            full.duplicate = rank > 0;
            self.full_space.insert(code, encodable.hash);
        }
    }

    fn encode_short(&mut self, config: &EncoderConfig, buffer: &mut Codes) {
        let weights: Vec<_> = (0..=config.max_length)
            .map(|x| config.radix.pow(x as u32))
            .collect();
        let short_code = config.short_code.as_ref().unwrap();
        // 优先简码
        for (encodable, pointer) in zip(&config.encodables, buffer.iter_mut()) {
            if encodable.level == u64::MAX {
                continue;
            }
            let short = &mut pointer.short;
            let modulo = config.radix.pow(encodable.level as u32);
            let code = pointer.full.code % modulo;
            let rank = self.short_space.rank_hash(code, encodable.hash);
            short.code = code;
            short.rank = rank;
            short.actual = config.wrap_actual(code, rank, modulo);
            short.duplicate = false;
            self.short_space.insert(code, encodable.hash);
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
            let hash = encodable.hash;
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
                let rank =
                    self.full_space.rank_hash(code, hash) + self.short_space.rank_hash(code, hash);
                if rank >= select_keys.len() as u8 {
                    continue;
                }
                short.code = code;
                short.rank = rank;
                short.actual = config.wrap_actual(code, rank, weight);
                short.duplicate = false;
                self.short_space.insert(code, hash);
                has_short = true;
                break;
            }
            if !has_short {
                let rank = self.short_space.rank_hash(full.code, hash);
                short.code = full.code;
                short.rank = rank;
                short.actual = full.actual;
                short.duplicate = rank != 0;
                self.short_space.insert(pointer.full.code, hash);
            }
        }
    }
}

impl Driver for Occupation {
    fn run(
        &mut self,
        keymap: &KeyMap,
        config: &EncoderConfig,
        buffer: &mut Codes,
        _: &[Element],
    ) {
        self.reset();

        // 不考虑差分，统一标注所有编码为已变更
        for pointer in buffer.iter_mut() {
            pointer.full.has_changed = true;
            pointer.short.has_changed = true;
        }

        self.encode_full(keymap, config, buffer);

        if config.short_code.is_none() || config.short_code.as_ref().unwrap().is_empty() {
            return;
        }

        self.encode_short(config, buffer);
    }
}
