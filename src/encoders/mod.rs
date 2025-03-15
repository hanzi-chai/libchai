//! 编码器接口，以及默认编码器的实现

use crate::{
    data::{元素, 元素映射, 数据, 最大词长, 编码, 编码信息, 自动上屏, 键},
    错误,
};
use rustc_hash::FxHashMap;

pub mod default;

pub trait 编码器 {
    fn 编码(
        &mut self,
        keymap: &元素映射,
        moved_elements: &Option<Vec<元素>>,
    ) -> &mut Vec<编码信息>;
}

#[derive(Clone)]
pub struct 编码空间 {
    pub vector: Vec<u8>,
    pub vector_length: usize,
    pub hashmap: FxHashMap<编码, u8>,
}

impl 编码空间 {
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

#[derive(Debug)]
pub struct 简码配置 {
    pub prefix: usize,
    pub select_keys: Vec<键>,
}

pub struct 编码配置 {
    pub radix: u64,
    pub max_length: usize,
    pub auto_select: 自动上屏,
    pub select_keys: Vec<键>,
    pub first_key: 键,
    pub short_code: Option<[Vec<简码配置>; 最大词长]>,
}

impl 编码配置 {
    pub fn new(representation: &数据) -> Result<Self, 错误> {
        let encoder = &representation.配置.encoder;
        let max_length = encoder.max_length;
        if max_length >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }
        let auto_select = representation.预处理自动上屏()?;
        let mut short_code = None;
        if let Some(configs) = &encoder.short_code {
            short_code = Some(representation.预处理简码配置(configs.clone())?);
        }
        let result = Self {
            auto_select,
            max_length,
            radix: representation.进制,
            select_keys: representation.选择键.clone(),
            first_key: representation.选择键[0],
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
                return code + self.first_key * weight;
            }
        }
        let select = *self
            .select_keys
            .get(rank as usize)
            .unwrap_or(&self.select_keys[0]);
        code + select * weight
    }
}
