use super::{Driver, EncoderConfig};
use crate::representation::{Codes, KeyMap};
use std::iter::zip;

/// 编码是否已被占据
/// 用一个数组和一个哈希集合来表示，数组用来表示四码以内的编码，哈希集合用来表示四码以上的编码
pub struct C3 {
    pub full_space: Vec<u8>,
}

impl C3 {
    pub fn new(length: usize) -> Self {
        Self {
            full_space: vec![0; length],
        }
    }

    pub fn reset(&mut self) {
        self.full_space.iter_mut().for_each(|x| {
            *x = 0;
        });
    }
}

impl Driver for C3 {
    fn run(&mut self, keymap: &KeyMap, config: &EncoderConfig, buffer: &mut Codes) {
        self.reset();
        // 1. 全码
        for (encodable, pointer) in zip(&config.encodables, buffer.iter_mut()) {
            let sequence = &encodable.sequence;
            assert!(sequence.len() >= 3);
            let code = keymap[sequence[0]] as u64 * config.radix * config.radix
                + keymap[sequence[1]] as u64 * config.radix
                + keymap[sequence[2]] as u64;
            pointer.full.actual = code;
        }

        for pointer in buffer.iter_mut() {
            let rank = self.full_space[pointer.full.actual as usize];
            pointer.full.duplicate = rank > 0;
            self.full_space[pointer.full.actual as usize] = rank.saturating_add(1);
        }
    }
}
