use super::{Driver, EncoderConfig};
use crate::representation::{Codes, Element, KeyMap, Representation};

/// 编码是否已被占据
/// 用一个数组和一个哈希集合来表示，数组用来表示四码以内的编码，哈希集合用来表示四码以上的编码
pub struct Snow2 {
    pub full_space: Vec<bool>,
    pub short_space: Vec<bool>,
    pub involved_message: Vec<Vec<usize>>,
}

impl Snow2 {
    pub fn new(length: usize) -> Self {
        Self {
            full_space: vec![false; length],
            short_space: vec![false; length],
            involved_message: vec![],
        }
    }

    pub fn reset(&mut self) {
        self.full_space.iter_mut().for_each(|x| {
            *x = false;
        });
        self.short_space.iter_mut().for_each(|x| {
            *x = false;
        });
    }
}

impl Driver for Snow2 {
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
        let weights: [u64; 4] = [
            1,
            config.radix,
            config.radix * config.radix,
            config.radix * config.radix * config.radix,
        ];

        if moved_elements.is_empty() {
            for (encodable, pointer) in config.encodables.iter().zip(buffer.iter_mut()) {
                let sequence = &encodable.sequence;
                let full = &mut pointer.full;
                let mut code = keymap[sequence[0]] as u64
                    + keymap[sequence[1]] as u64 * weights[1]
                    + keymap[sequence[2]] as u64 * weights[2];
                if sequence.len() == 4 {
                    code += keymap[sequence[3]] as u64 * weights[3];
                };
                full.actual = code;
                full.has_changed = true;
            }
        } else {
            // 部分编码，只计算变化的部分
            for element in moved_elements {
                for index in &self.involved_message[*element] {
                    let pointer = &mut buffer[*index];
                    let encodable = &config.encodables[*index];
                    let sequence = &encodable.sequence;
                    let full = &mut pointer.full;
                    let mut code = keymap[sequence[0]] as u64
                        + keymap[sequence[1]] as u64 * weights[1]
                        + keymap[sequence[2]] as u64 * weights[2];
                    if sequence.len() == 4 {
                        code += keymap[sequence[3]] as u64 * weights[3];
                    };
                    full.check_actual(code);
                }
            }
        }

        for pointer in buffer.iter_mut() {
            let full = &mut pointer.full;
            let duplicate = self.full_space[full.actual as usize];
            full.check_duplicate(duplicate);
            self.full_space[full.actual as usize] = true;
        }

        // 出简码
        for (index, p) in buffer.iter_mut().enumerate() {
            let full = &mut p.full;
            let short = &mut p.short;
            let has_short = index <= 10000;
            let first = full.actual % weights[1];
            let second = full.actual % weights[2];
            if has_short && !self.short_space[first as usize] {
                short.check(first, false);
                self.short_space[first as usize] = true;
            } else if has_short && !self.short_space[second as usize] {
                short.check(second, false);
                self.short_space[second as usize] = true;
            } else {
                let duplicate = self.short_space[full.actual as usize];
                short.check(full.actual, duplicate);
                self.short_space[full.actual as usize] = true;
            }
        }
    }
}
