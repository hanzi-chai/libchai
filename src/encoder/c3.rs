use super::{Driver, EncoderConfig};
use crate::representation::{Codes, Element, KeyMap, Representation};

/// 编码是否已被占据
/// 用一个数组和一个哈希集合来表示，数组用来表示四码以内的编码，哈希集合用来表示四码以上的编码
pub struct C3 {
    pub full_space: Vec<bool>,
    pub involved_message: Vec<Vec<usize>>,
}

impl C3 {
    pub fn new(length: usize) -> Self {
        Self {
            full_space: vec![false; length],
            involved_message: vec![],
        }
    }

    pub fn reset(&mut self) {
        self.full_space.iter_mut().for_each(|x| {
            *x = false;
        });
    }
}

impl Driver for C3 {
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

        // 部分编码，只计算变化的部分
        for element in moved_elements {
            for index in &self.involved_message[*element] {
                let pointer = &mut buffer[*index];
                let encodable = &config.encodables[*index];
                let sequence = &encodable.sequence;
                let full = &mut pointer.full;
                let code = if sequence.len() == 3 {
                    keymap[sequence[0]] as u64 * config.radix * config.radix
                        + keymap[sequence[1]] as u64 * config.radix
                        + keymap[sequence[2]] as u64
                } else {
                    keymap[sequence[0]] as u64 * config.radix + keymap[sequence[1]] as u64
                };
                full.check_actual(code);
            }
        }

        for pointer in buffer.iter_mut() {
            let full = &mut pointer.full;
            let duplicate = self.full_space[full.actual as usize];
            full.check_duplicate(duplicate);
            self.full_space[full.actual as usize] = true;
        }
    }
}
