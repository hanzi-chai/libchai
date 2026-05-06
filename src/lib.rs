//! libchai 是使用 Rust 实现的汉字编码输入方案的优化算法。它同时发布为一个 Rust crate 和一个 NPM 模块，前者可以在 Rust 项目中安装为依赖来使用，后者可以通过汉字自动拆分系统的图形界面来使用。
//!
//! chai 是使用 libchai 实现的命令行程序，用户提供方案配置文件、拆分表和评测信息，本程序能够生成编码并评测一系列指标，以及基于退火算法优化元素的布局。

pub mod config;
pub mod contexts;
pub mod encoders;
pub mod interfaces;
pub mod objectives;
pub mod operators;
pub mod optimizers;
#[cfg(feature = "server")]
pub mod server;

use config::{安排, 广义码位};
use objectives::metric::指法标记;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::io;
use wasm_bindgen::JsError;

use crate::config::条件;

/// 只考虑长度为 1 到 10 的词
pub const 最大词长: usize = 10;

/// 只对低于最大按键组合长度的编码预先计算当量
pub const 最大按键组合长度: usize = 4;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct 原始元素序列及条件列表 {
    pub 元素序列: Vec<广义码位>,
    pub 条件列表: Vec<条件>,
}

/// 从配置文件中读取的原始可编码对象
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct 原始可编码对象 {
    pub 词: String,
    /// 简单格式：单个元素序列，无条件
    #[serde(default)]
    pub 元素序列: Option<Vec<广义码位>>,
    /// 复杂格式：多个带条件的元素序列
    #[serde(default)]
    pub 全部元素序列: Option<Vec<原始元素序列及条件列表>>,
    pub 频率: u64,
    #[serde(default = "原始可编码对象::默认级别")]
    pub 简码长度: u64,
}

impl 原始可编码对象 {
    const fn 默认级别() -> u64 {
        u64::MAX
    }
}

pub type 原始键位分布信息 = FxHashMap<char, 键位分布损失函数>;
pub type 键位分布信息 = Vec<键位分布损失函数>;
pub type 原始当量信息 = FxHashMap<String, f64>;
pub type 当量信息 = Vec<f64>;

/// 键位分布的理想值和惩罚值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 键位分布损失函数 {
    pub 理想值: f64,
    pub 低于惩罚: f64,
    pub 高于惩罚: f64,
}

/// 元素用一个无符号整数表示
pub type 元素 = usize;

/// 元素位 = 元素 + 位置 * 元素总数，位置从 0 开始，元素总数由棱镜决定。这样设计的好处是可以把元素和位置的信息都编码在一个整数中，方便处理和比较。
pub type 元素位 = usize;

/// 最大元素序列长度，超过这个长度的编码会被拒绝
pub const 最大元素序列长度: usize = 8;

/// 可选元素（安排可能为 未选取 的元素）的最大数量，超过这个数量会被拒绝
/// 这个限制来自位图的大小：16 * 64 = 1024 位
pub const 最大元素数量: usize = 1024;

/// 可编码对象的序列
pub type 元素序列 = [元素位; 最大元素序列长度];

/// 元素关系图
pub type 元素图 = FxHashMap<元素, Vec<元素>>;

/// 最大元素编码长度
pub const 最大元素编码长度: usize = 4;

/// 编码用无符号整数表示
pub type 编码 = u64;

/// 包含词、词长、元素序列、频率等信息
#[derive(Debug, Clone)]
pub struct 可编码对象 {
    pub 词: String,
    pub 词长: usize,
    pub 元素序列: 元素序列,
    pub 全部元素序列: Vec<(元素序列, 位图)>,
    pub 频率: u64,
    pub 简码长度: u64,
    pub 原始顺序: usize,
}

// 位图，用于表示元素是否存在，最多支持 1024 个元素
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct 位图 {
    pub 位图: [u64; 16],
}

impl 位图 {
    pub fn new() -> Self {
        Self { 位图: [0; 16] }
    }

    pub fn 从条件列表创建(条件列表: &[条件], 棱镜: &棱镜) -> Self {
        let mut bitmap = Self::new();
        for 条件 in 条件列表 {
            if let Some(&元素) = 棱镜.元素转数字.get(&条件.element) {
                if let Some(&位图索引) = 棱镜.可选元素位图索引.get(&元素) {
                    bitmap.insert(位图索引);
                } else {
                    // 如果条件中的元素不在棱镜中，说明配置文件有问题，直接忽略这个条件
                    // 也可以选择抛出错误，但考虑到健壮性，这里选择忽略
                    eprintln!(
                        "警告：条件中的元素「{}」在棱镜中未找到，已忽略这个条件",
                        条件.element
                    );
                }
            }
        }
        bitmap
    }

    pub fn insert(&mut self, i: usize) {
        let (block, bit) = (i / 64, i % 64);
        self.位图[block] |= 1 << bit;
    }

    pub fn remove(&mut self, i: usize) {
        let (block, bit) = (i / 64, i % 64);
        self.位图[block] &= !(1 << bit);
    }

    pub fn subset(&self, other: &Self) -> bool {
        for i in 0..16 {
            if self.位图[i] & !other.位图[i] != 0 {
                return false;
            }
        }
        true
    }

    pub fn union(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for i in 0..16 {
            result.位图[i] = self.位图[i] | other.位图[i];
        }
        result
    }

    pub fn intersection(&self, other: &Self) -> Self {
        let mut result = Self::new();
        for i in 0..16 {
            result.位图[i] = self.位图[i] & other.位图[i];
        }
        result
    }
}

/// 全码或简码的编码信息
#[derive(Clone, Debug, Copy, Default)]
pub struct 部分编码信息 {
    pub 原始编码: 编码,       // 原始编码
    pub 原始编码候选位置: u8, // 原始编码上的选重位置
    pub 实际编码: 编码,       // 实际编码
    pub 选重标记: u8,         // 实际编码的候选位置（0 表示首选，>0 表示选重）
    pub 上一个实际编码: 编码, // 前一个实际编码
    pub 上一个选重标记: u8,   // 前一个实际编码的候选位置
    pub 有变化: bool,         // 编码是否发生了变化
}

impl 部分编码信息 {
    #[inline(always)]
    pub fn 更新(&mut self, 编码: 编码, 选重标记: u8) {
        if self.实际编码 == 编码 && self.选重标记 == 选重标记 {
            return;
        }
        self.有变化 = true;
        self.上一个实际编码 = self.实际编码;
        self.上一个选重标记 = self.选重标记;
        self.实际编码 = 编码;
        self.选重标记 = 选重标记;
    }
}

/// 包含长度、频率、全码和简码，用于传给目标函数来统计
#[derive(Clone, Debug)]
pub struct 编码信息 {
    pub 词长: usize,
    pub 频率: u64,
    pub 全码: 部分编码信息,
    pub 简码: 部分编码信息,
}

impl 编码信息 {
    pub fn new(词: &可编码对象) -> Self {
        Self {
            词长: 词.词长,
            频率: 词.频率,
            全码: 部分编码信息::default(),
            简码: 部分编码信息::default(),
        }
    }
}

/// 按键用无符号整数表示
pub type 键 = u64;

/// 用指标记
pub type 指法向量 = [u8; 8];

/// 自动上屏判断数组
pub type 自动上屏 = Vec<bool>;

/// 用于输出为文本码表，包含了名称、全码、简码、全码排名和简码排名
#[derive(Debug, Clone, Serialize, Default)]
pub struct 码表项 {
    pub 词: String,
    pub 全码: String,
    pub 全码排名: u8,
    pub 简码: String,
    pub 简码排名: u8,
}

impl 安排 {
    pub fn normalize(&self) -> Vec<广义码位> {
        match self {
            安排::Advanced(vector) => vector.clone(),
            安排::Basic(string) => string.chars().map(广义码位::Ascii).collect(),
            _ => panic!("无法把归并或禁用表示成列表形式"),
        }
    }
}

pub fn 元素标准名称(element: &String, index: usize) -> String {
    if index == 0 {
        element.to_string()
    } else {
        format!("{element}.{index}")
    }
}

#[derive(Debug, Clone)]
pub struct 棱镜 {
    pub 键转数字: FxHashMap<char, 键>,
    pub 数字转键: FxHashMap<键, char>,
    pub 元素转数字: FxHashMap<String, 元素>,
    pub 数字转元素: FxHashMap<元素, String>,
    pub 进制: u64,
    /// 安排可能为 未选取 的元素的紧凑位图索引，仅包含这些元素
    /// 键为棱镜中的元素编号，值为位图中的紧凑索引（0, 1, 2, ...）
    pub 可选元素位图索引: FxHashMap<元素, usize>,
}

impl 棱镜 {
    pub fn 元素总数(&self) -> usize {
        self.元素转数字.len() + 1 // 加上 0 这个特殊元素
    }

    /// 如前所述，建立了一个按键到整数的映射之后，可以将字符串看成具有某个进制的数。所以，给定一个数，也可以把它转化为字符串
    pub fn 数字转编码(&self, code: 编码) -> Vec<char> {
        let mut chars = Vec::new();
        let mut remainder = code;
        while remainder > 0 {
            let k = remainder % self.进制;
            remainder /= self.进制;
            if k == 0 {
                continue;
            }
            let char = self.数字转键.get(&k).unwrap(); // 从内部表示转换为字符，不需要检查
            chars.push(*char);
        }
        chars
    }

    pub fn 预处理元素序列(
        &self,
        词: &String,
        原始元素序列: &[广义码位],
        最大码长: usize,
    ) -> Result<元素序列, 错误> {
        let mut 元素序列 = 元素序列::default();
        let 原始元素序列长度 = 原始元素序列.len();
        if 原始元素序列长度 > 最大码长 {
            return Err(format!(
                "编码对象「{词}」包含的元素数量为 {原始元素序列长度}，超过了最大码长 {最大码长}"
            )
            .into());
        }
        for (i, 码位) in 原始元素序列.iter().enumerate() {
            let 元素位 = match 码位 {
                广义码位::Reference { element, index } => {
                    if let Some(&元素) = self.元素转数字.get(element) {
                        元素 + index * self.元素总数()
                    } else {
                        return Err(format!(
                            "编码对象「{词}」包含的元素「{element}」无法在键盘映射中找到"
                        )
                        .into());
                    }
                }
                _ => 0,
            };
            元素序列[i] = 元素位;
        }
        return Ok(元素序列);
    }

    pub fn 预处理词列表(
        &self,
        原始词列表: Vec<原始可编码对象>,
        最大码长: usize,
    ) -> Result<Vec<可编码对象>, 错误> {
        let mut 词列表 = vec![];
        for (原始顺序, 原始可编码对象) in 原始词列表.into_iter().enumerate() {
            let 原始可编码对象 {
                词,
                频率,
                元素序列: 原始单一元素序列,
                全部元素序列: 原始全部元素序列,
                简码长度,
            } = 原始可编码对象;
            let 原始全部元素序列: Vec<原始元素序列及条件列表> = match (原始单一元素序列, 原始全部元素序列) {
                (Some(seq), None) => vec![原始元素序列及条件列表 { 元素序列: seq, 条件列表: vec![] }],
                (None, Some(list)) => list,
                _ => panic!("编码对象「{词}」必须恰好提供「元素序列」或「全部元素序列」之一", 词 = 词),
            };
            let mut 全部元素序列 = vec![];
            assert!(!原始全部元素序列.is_empty(), "编码对象「{词}」至少需要一个元素序列", 词 = 词);
            assert!(原始全部元素序列.last().unwrap().条件列表.is_empty(), "编码对象「{词}」的最后一个元素序列必须没有任何条件", 词 = 词);
            for 原始元素序列及条件列表 { 元素序列: 原始元素序列, 条件列表 } in 原始全部元素序列 {
                let 元素序列 = self.预处理元素序列(&词, &原始元素序列, 最大码长)?;
                let 位图 = 位图::从条件列表创建(&条件列表, self);
                全部元素序列.push((元素序列, 位图));
            }
            let c = 可编码对象 {
                词: 词.clone(),
                词长: 词.chars().count(),
                频率,
                简码长度,
                元素序列: 全部元素序列[0].0,
                全部元素序列,
                原始顺序,
            };
            词列表.push(c);
        }
        词列表.sort_by_key(|x| Reverse(x.频率));
        Ok(词列表)
    }

    /// 根据编码字符和未归一化的键位分布，生成一个理想的键位分布
    pub fn 预处理键位分布信息(
        &self,
        原始键位分布信息: &原始键位分布信息,
    ) -> Vec<键位分布损失函数> {
        let default_loss = 键位分布损失函数 {
            理想值: 0.0,
            低于惩罚: 0.0,
            高于惩罚: 0.0,
        };
        let mut 键位分布信息: Vec<键位分布损失函数> = (0..self.进制)
            .map(|键| {
                // 0 只是为了占位，不需要统计
                if 键 == 0 {
                    default_loss.clone()
                } else {
                    let 键名称 = self.数字转键[&键];
                    原始键位分布信息
                        .get(&键名称)
                        .unwrap_or(&default_loss)
                        .clone()
                }
            })
            .collect();
        键位分布信息.iter_mut().for_each(|x| {
            x.理想值 /= 100.0;
        });
        键位分布信息
    }

    /// 将编码空间内所有的编码组合预先计算好速度当量
    /// 按照这个字符串所对应的整数为下标，存储到一个大数组中
    pub fn 预处理当量信息(
        &self, 原始当量信息: &原始当量信息, space: usize
    ) -> Vec<f64> {
        let mut result: Vec<f64> = vec![0.0; space];
        for (index, equivalence) in result.iter_mut().enumerate() {
            let chars = self.数字转编码(index as u64);
            for correlation_length in [2, 3, 4] {
                if chars.len() < correlation_length {
                    break;
                }
                // N 键当量
                for i in 0..=(chars.len() - correlation_length) {
                    let substr: String = chars[i..(i + correlation_length)].iter().collect();
                    *equivalence += 原始当量信息.get(&substr).unwrap_or(&0.0);
                }
            }
        }
        result
    }

    /// 将编码空间内所有的编码组合预先计算好差指法标记
    /// 标记压缩到一个 64 位整数中，每四位表示一个字符的差指法标记
    /// 从低位到高位，依次是：同手、同指大跨排、同指小跨排、小指干扰、错手、三连击
    /// 按照这个字符串所对应的整数为下标，存储到一个大数组中
    pub fn 预处理指法标记(&self, 空间: usize) -> Vec<指法向量> {
        let 指法标记 = 指法标记::new();
        let mut result: Vec<指法向量> = Vec::with_capacity(空间);
        for code in 0..空间 {
            let chars = self.数字转编码(code as u64);
            if chars.len() < 2 {
                result.push(指法向量::default());
                continue;
            }
            let mut 指法向量 = 指法向量::default();
            for i in 0..(chars.len() - 1) {
                let pair = (chars[i], chars[i + 1]);
                if 指法标记.同手.contains(&pair) {
                    指法向量[0] += 1;
                }
                if 指法标记.同指大跨排.contains(&pair) {
                    指法向量[1] += 1;
                }
                if 指法标记.同指小跨排.contains(&pair) {
                    指法向量[2] += 1;
                }
                if 指法标记.小指干扰.contains(&pair) {
                    指法向量[3] += 1;
                }
                if 指法标记.错手.contains(&pair) {
                    指法向量[4] += 1;
                }
            }
            for i in 0..(chars.len() - 2) {
                let triple = (chars[i], chars[i + 1], chars[i + 2]);
                if triple.0 == triple.1 && triple.1 == triple.2 {
                    指法向量[5] += 1;
                }
            }
            result.push(指法向量);
        }
        result
    }
}

/// 错误类型
#[derive(Debug, Clone)]
pub struct 错误 {
    pub message: String,
}

impl From<String> for 错误 {
    fn from(value: String) -> Self {
        Self { message: value }
    }
}

impl From<&str> for 错误 {
    fn from(value: &str) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl From<io::Error> for 错误 {
    fn from(value: io::Error) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl From<serde_json::Error> for 错误 {
    fn from(value: serde_json::Error) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl From<错误> for JsError {
    fn from(value: 错误) -> Self {
        JsError::new(&value.message)
    }
}

#[cfg(target_arch = "wasm32")]
pub fn formatted_local_now() -> String {
    use js_sys::Date;
    let date = Date::new_0();
    let month = date.get_month() as u32 + 1; // JS 月份从 0 开始
    let day = date.get_date() as u32;
    let hour = date.get_hours() as u32;
    let minute = date.get_minutes() as u32;
    let second = date.get_seconds() as u32;
    format!(
        "{:02}-{:02}+{:02}_{:02}_{:02}",
        month, day, hour, minute, second
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn formatted_local_now() -> String {
    use chrono::Local;
    Local::now().format("%m-%d+%H_%M_%S").to_string()
}
