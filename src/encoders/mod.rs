//! 编码引擎

use crate::{
    representation::{
        AutoSelect, Code, Codes, Element, Key, KeyMap, Representation, Sequence, MAX_WORD_LENGTH,
    },
    Error,
};
use rustc_hash::FxHashMap;

pub mod default;

pub trait Encoder {
    fn encode(&mut self, keymap: &KeyMap, moved_elements: &Option<Vec<Element>>) -> &mut Codes;
}

#[derive(Clone)]
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
    pub select_keys: Vec<Key>,
}

pub struct EncoderConfig {
    pub radix: u64,
    pub max_length: usize,
    pub auto_select: AutoSelect,
    pub select_keys: Vec<Key>,
    pub first_key: Key,
    pub short_code: Option<[Vec<CompiledScheme>; MAX_WORD_LENGTH]>,
}

impl EncoderConfig {
    pub fn new(representation: &Representation) -> Result<Self, Error> {
        let encoder = &representation.config.encoder;
        let max_length = encoder.max_length;
        if max_length >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }
        let auto_select = representation.transform_auto_select()?;
        let mut short_code = None;
        if let Some(configs) = &encoder.short_code {
            short_code = Some(representation.transform_short_code(configs.clone())?);
        }
        let result = Self {
            auto_select,
            max_length,
            radix: representation.radix,
            select_keys: representation.select_keys.clone(),
            first_key: representation.select_keys[0],
            short_code,
        };
        Ok(result)
    }

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
