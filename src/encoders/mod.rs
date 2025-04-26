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
    pub 线性表: Vec<u8>,
    pub 线性表长度: usize,
    pub 哈希表: FxHashMap<编码, u8>,
}

impl 编码空间 {
    #[inline(always)]
    pub fn 添加(&mut self, 编码: u64) {
        if 编码 < self.线性表长度 as u64 {
            let 编码 = 编码 as usize;
            self.线性表[编码] = self.线性表[编码].saturating_add(1);
        } else {
            self.哈希表
                .entry(编码)
                .and_modify(|x| *x = x.saturating_add(1))
                .or_insert(1);
        }
    }

    #[inline(always)]
    pub fn 查找数量(&self, 编码: u64) -> u8 {
        if 编码 < self.线性表长度 as u64 {
            self.线性表[编码 as usize]
        } else {
            *self.哈希表.get(&编码).unwrap_or(&0)
        }
    }
}

#[derive(Debug)]
pub struct 简码配置 {
    pub prefix: usize,
    pub select_keys: Vec<键>,
}

pub struct 编码配置 {
    pub 进制: u64,
    pub 乘数列表: Vec<u64>,
    pub 最大码长: usize,
    pub 自动上屏查找表: 自动上屏,
    pub 选择键: Vec<键>,
    pub 首选键: 键,
    pub 简码配置列表: Option<[Vec<简码配置>; 最大词长]>,
}

impl 编码配置 {
    pub fn new(数据: &数据) -> Result<Self, 错误> {
        let 编码器配置 = &数据.配置.encoder;
        let 最大码长 = 编码器配置.max_length;
        if 最大码长 >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }
        let 自动上屏查找表 = 数据.预处理自动上屏()?;
        let mut 简码配置列表 = None;
        if let Some(configs) = &编码器配置.short_code {
            简码配置列表 = Some(数据.预处理简码配置(configs.clone())?);
        }
        Ok(Self {
            自动上屏查找表,
            最大码长,
            进制: 数据.进制,
            乘数列表: (0..=最大码长).map(|x| 数据.进制.pow(x as u32)).collect(),
            选择键: 数据.选择键.clone(),
            首选键: 数据.选择键[0],
            简码配置列表,
        })
    }

    #[inline(always)]
    pub fn 生成编码(
        &self, 原始编码: u64, 原始编码候选位置: u8, 选择键乘数: u64
    ) -> u64 {
        // 如果位于首选，检查是否能自动上屏，不能则加上首选键
        if 原始编码候选位置 == 0 {
            if *self.自动上屏查找表.get(原始编码 as usize).unwrap_or(&true) {
                return 原始编码;
            } else {
                return 原始编码 + self.首选键 * 选择键乘数;
            }
        }
        // 如果不是首选，无论如何都加上选择键
        let 选择键 = *self
            .选择键
            .get(原始编码候选位置 as usize)
            .unwrap_or(&self.选择键[0]);
        原始编码 + 选择键 * 选择键乘数
    }
}
