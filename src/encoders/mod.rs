//! 编码引擎

use crate::representation::{
    AutoSelect, Code, Codes, Element, Key, KeyMap, Sequence, MAX_WORD_LENGTH,
};
use rustc_hash::FxHashMap;

pub mod default;

pub trait Encoder {
    fn run(&mut self, keymap: &KeyMap, moved_elements: &Option<Vec<Element>>);

    fn get_buffer(&mut self) -> &mut Codes;
}

pub struct Space {
    pub vector: Vec<u8>,
    pub vector_length: usize,
    pub hashmap: FxHashMap<Code, u8>,
}

impl Space {
    #[inline(always)]
    pub fn insert(&mut self, code: u64) {
        if code < self.vector_length as u64 {
            let index = code as usize;
            self.vector[index] = self.vector[index].saturating_add(1);
        } else {
            self.hashmap
                .entry(code)
                .and_modify(|x| *x = x.saturating_add(1))
                .or_insert(1);
        }
    }

    #[inline(always)]
    pub fn rank(&self, code: u64) -> u8 {
        if code < self.vector_length as u64 {
            let index = code as usize;
            self.vector[index]
        } else {
            *self.hashmap.get(&code).unwrap_or(&0)
        }
    }
}

/// 一个可编码对象
#[derive(Debug, Clone)]
pub struct Encodable {
    pub name: String,
    pub length: usize,
    pub sequence: Sequence,
    pub frequency: u64,
    pub level: u64,
    pub index: usize,
}

#[derive(Debug)]
pub struct CompiledScheme {
    pub prefix: usize,
    pub select_keys: Vec<usize>,
}

pub struct EncoderConfig {
    pub radix: u64,
    pub max_length: usize,
    pub auto_select: AutoSelect,
    pub select_keys: Vec<Key>,
    pub first_key: Key,
    pub short_code: Option<[Vec<CompiledScheme>; MAX_WORD_LENGTH]>,
    pub encodables: Vec<Encodable>,
    pub elements_length: usize,
}

impl EncoderConfig {
    #[inline(always)]
    pub fn wrap_actual(&self, code: u64, rank: u8, weight: u64) -> u64 {
        if rank == 0 {
            if *self.auto_select.get(code as usize).unwrap_or(&true) {
                return code;
            } else {
                return code + (self.first_key as u64) * weight;
            }
        }
        let select = *self
            .select_keys
            .get(rank as usize)
            .unwrap_or(&self.select_keys[0]) as u64;
        code + select * weight
    }
}
