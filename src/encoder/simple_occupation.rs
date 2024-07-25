use super::{CompiledScheme, Driver, EncoderConfig};
use crate::representation::{Codes, KeyMap};
use std::iter::zip;

/// 编码是否已被占据
/// 用一个数组和一个哈希集合来表示，数组用来表示四码以内的编码，哈希集合用来表示四码以上的编码
pub struct SimpleOccupation {
    pub full_space: Vec<u8>,
    pub short_space: Vec<u8>,
}

impl SimpleOccupation {
    pub fn new(length: usize) -> Self {
        Self {
            full_space: vec![0; length],
            short_space: vec![0; length],
        }
    }

    pub fn reset(&mut self) {
        self.full_space.iter_mut().for_each(|x| {
            *x = 0;
        });
        self.short_space.iter_mut().for_each(|x| {
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
            let rank = self.full_space[code as usize];
            pointer.full.code = code;
            pointer.full.duplicate = rank > 0;
            pointer.full.actual = config.wrap_actual(code, 0, weights[sequence.len()]);
            self.full_space[code as usize] = rank.saturating_add(1);
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
            let rank = self.short_space[short as usize];
            pointer.short.duplicate = rank > 0;
            pointer.short.actual = config.wrap_actual(short, rank, encodable.level);
            self.short_space[short as usize] = rank.saturating_add(1);
        }
        // 常规简码
        for (pointer, encodable) in zip(buffer.iter_mut(), &config.encodables) {
            if encodable.level != u64::MAX {
                continue;
            }
            let schemes = &short_code[encodable.length - 1];
            let mut has_short = false;
            let full = pointer.full;
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
                // 将全码截取一部分出来
                let short = full.code % weight;
                let rank = self.full_space[short as usize] + self.short_space[short as usize];
                if rank >= select_keys.len() as u8 {
                    continue;
                }
                pointer.short.actual = config.wrap_actual(short, rank, weight);
                pointer.short.duplicate = false;
                self.short_space[short as usize] =
                    self.short_space[short as usize].saturating_add(1);
                has_short = true;
                break;
            }
            if !has_short {
                let rank = self.short_space[full.code as usize];
                pointer.short.actual = full.actual;
                pointer.short.duplicate = rank != 0;
                self.short_space[full.code as usize] = rank.saturating_add(1);
            }
        }
    }
}
