use super::{CompiledScheme, Driver, EncoderConfig};
use crate::representation::{Codes, Element, KeyMap, Representation};
use std::iter::zip;

/// 编码是否已被占据
/// 用一个数组和一个哈希集合来表示，数组用来表示四码以内的编码，哈希集合用来表示四码以上的编码
pub struct SimpleOccupation {
    pub full_space: Vec<u8>,
    pub short_space: Vec<u8>,
    pub involved_message: Vec<Vec<usize>>,
}

impl SimpleOccupation {
    pub fn new(length: usize) -> Self {
        Self {
            full_space: vec![0; length],
            short_space: vec![0; length],
            involved_message: vec![],
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

    pub fn encode_full(
        &mut self,
        keymap: &KeyMap,
        config: &EncoderConfig,
        buffer: &mut Codes,
        moved_elements: &[Element],
    ) {
        let weights: Vec<_> = (0..=config.max_length)
            .map(|x| config.radix.pow(x as u32))
            .collect();
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

        for pointer in buffer.iter_mut() {
            let full = &mut pointer.full;
            let duplicate = self.full_space[full.code as usize] > 0;
            full.check_duplicate(duplicate);
            self.full_space[full.code as usize] =
                self.full_space[full.code as usize].saturating_add(1);
        }
    }

    pub fn encode_short(&mut self, config: &EncoderConfig, buffer: &mut Codes) {
        let weights: Vec<_> = (0..=config.max_length)
            .map(|x| config.radix.pow(x as u32))
            .collect();
        let short_code = config.short_code.as_ref().unwrap();
        // 2. 优先简码
        for (pointer, encodable) in zip(buffer.iter_mut(), &config.encodables) {
            if encodable.level == u64::MAX {
                continue;
            }
            let code = pointer.full.code % weights[encodable.level as usize];
            let rank = self.short_space[code as usize];
            let actual = config.wrap_actual(code, rank, weights[encodable.level as usize]);
            pointer.short.check(actual, rank > 0);
            self.short_space[code as usize] = rank.saturating_add(1);
        }
        // 3. 常规简码
        for (p, encodable) in zip(buffer.iter_mut(), &config.encodables) {
            if encodable.level != u64::MAX {
                continue;
            }
            let schemes = &short_code[encodable.length - 1];
            let mut has_short = false;
            let full = &p.full;
            let short = &mut p.short;
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
                let code = full.code % weight;
                let rank = self.full_space[code as usize] + self.short_space[code as usize];
                if rank >= select_keys.len() as u8 {
                    continue;
                }
                let actual = config.wrap_actual(code, rank, weight);
                short.check(actual, false);
                self.short_space[code as usize] = self.short_space[code as usize].saturating_add(1);
                has_short = true;
                break;
            }
            if !has_short {
                let code = full.code;
                let duplicate = self.short_space[code as usize] > 0;
                short.check(full.actual, duplicate);
                self.short_space[code as usize] = self.short_space[code as usize].saturating_add(1);
            }
        }
    }
}

impl Driver for SimpleOccupation {
    fn init(&mut self, config: &EncoderConfig, _: &Representation) {
        for _ in 0..=config.elements_length {
            self.involved_message.push(vec![]);
        }
        for (index, encodable) in config.encodables.iter().enumerate() {
            for element in &encodable.sequence {
                self.involved_message[*element].push(index);
            }
        }
    }

    fn run(
        &mut self,
        keymap: &KeyMap,
        config: &EncoderConfig,
        buffer: &mut Codes,
        moved_elements: &[Element],
    ) {
        self.reset();
        self.encode_full(keymap, config, buffer, moved_elements);
        if config.short_code.is_none() || config.short_code.as_ref().unwrap().is_empty() {
            return;
        }
        self.encode_short(config, buffer);
    }
}
