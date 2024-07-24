use super::{CompiledScheme, Driver, EncoderConfig};
use crate::representation::{Codes, KeyMap};
use std::iter::zip;

/// 编码是否已被占据
/// 用一个数组和一个哈希集合来表示，数组用来表示四码以内的编码，哈希集合用来表示四码以上的编码
pub struct SimpleOccupation {
    pub vector: Vec<u8>,
}

impl SimpleOccupation {
    pub fn new(length: usize) -> Self {
        Self {
            vector: vec![0; length],
        }
    }

    pub fn reset(&mut self) {
        self.vector.iter_mut().for_each(|x| {
            *x = 0;
        });
    }
}

impl Driver for SimpleOccupation {
    fn encode_full(&mut self, keymap: &KeyMap, config: &EncoderConfig, full: &mut Codes) {
        self.reset();
        let weights: Vec<_> = (0..=config.max_length)
            .map(|x| config.radix.pow(x as u32))
            .collect();
        for (encodable, pointer) in zip(&config.encodables, full.iter_mut()) {
            let sequence = &encodable.sequence;
            let mut code = 0_u64;
            for (element, weight) in zip(sequence, &weights) {
                code += keymap[*element] as u64 * weight;
            }
            pointer.full.duplicate = self.vector[code as usize] > 0;
            pointer.full.actual = config.wrap_actual(code, 0, weights[sequence.len()]);
            self.vector[code as usize] = self.vector[code as usize].saturating_add(1);
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
            let short = pointer.full.code % weights[encodable.level as usize];
            let rank = self.vector[short as usize];
            pointer.short.duplicate = rank > 0;
            pointer.short.actual = config.wrap_actual(short, rank, encodable.level);
            self.vector[short as usize] = rank.saturating_add(1);
        }
        // 常规简码
        for (pointer, encodable) in zip(buffer.iter_mut(), &config.encodables) {
            let schemes = &short_code[encodable.length - 1];
            if schemes.is_empty() || encodable.level != u64::MAX {
                continue;
            }
            let mut has_short = false;
            let full = pointer.full;
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
                // 将全码截取一部分出来
                let short = pointer.full.code % weight;
                let rank = self.vector[short as usize];
                if rank >= select_keys.len() as u8 {
                    continue;
                }
                pointer.short.actual = config.wrap_actual(short, rank, weight);
                pointer.short.duplicate = false;
                self.vector[short as usize] = self.vector[short as usize].saturating_add(1);
                self.vector[full.code as usize] = self.vector[full.code as usize].saturating_sub(1);
                has_short = true;
                break;
            }
            if !has_short {
                pointer.short.actual = full.actual;
                pointer.short.duplicate = self.vector[full.code as usize] > 0;
            }
        }
    }
}
