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
pub mod server;

use config::{安排, 广义码位};
use objectives::metric::指法标记;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::io;
use wasm_bindgen::JsError;

/// 只考虑长度为 1 到 10 的词
pub const 最大词长: usize = 10;

/// 只对低于最大按键组合长度的编码预先计算当量
pub const 最大按键组合长度: usize = 4;

/// 从配置文件中读取的原始可编码对象
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct 原始可编码对象 {
    pub 词: String,
    pub 元素序列: String,
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

/// 可编码对象的序列
pub type 元素序列 = Vec<(元素, usize)>;

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
    pub 频率: u64,
    pub 简码长度: u64,
    pub 原始顺序: usize,
}

/// 全码或简码的编码信息
#[derive(Clone, Debug, Copy, Default)]
pub struct 部分编码信息 {
    pub 原始编码: 编码,       // 原始编码
    pub 原始编码候选位置: u8, // 原始编码上的选重位置
    pub 实际编码: 编码,       // 实际编码
    pub 选重标记: bool,       // 实际编码是否算作重码
    pub 上一个实际编码: 编码, // 前一个实际编码
    pub 上一个选重标记: bool, // 前一个实际编码是否算作重码
    pub 有变化: bool,         // 编码是否发生了变化
}

impl 部分编码信息 {
    #[inline(always)]
    pub fn 更新(&mut self, 编码: 编码, 选重标记: bool) {
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
}

impl 棱镜 {
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

    pub fn 预处理词列表(
        &self,
        原始词列表: Vec<原始可编码对象>,
        最大码长: usize,
    ) -> Result<Vec<可编码对象>, 错误> {
        let mut 词列表 = Vec::new();
        for (原始顺序, 原始可编码对象) in 原始词列表.into_iter().enumerate() {
            let 原始可编码对象 {
                词: name,
                频率: frequency,
                元素序列: sequence,
                简码长度: level,
            } = 原始可编码对象;
            let 原始元素序列: Vec<_> = sequence.split(' ').collect();
            let mut 元素序列 = 元素序列::new();
            let length = 原始元素序列.len();
            if length > 最大码长 {
                return Err(format!(
                    "编码对象「{name}」包含的元素数量为 {length}，超过了最大码长 {最大码长}"
                )
                .into());
            }
            for 原始元素 in 原始元素序列 {
                let (元素, 位置) = if 原始元素.contains(".") {
                    let parts: Vec<&str> = 原始元素.split('.').collect();
                    if parts.len() != 2 {
                        return Err(format!(
                            "编码对象「{name}」包含的元素「{原始元素}」格式不正确"
                        )
                        .into());
                    }
                    let 元素名称 = parts[0];
                    let index: usize = match parts[1].parse() {
                        Ok(v) => v,
                        Err(_) => {
                            return Err(format!(
                                "编码对象「{name}」包含的元素「{原始元素}」格式不正确"
                            )
                            .into());
                        }
                    };
                    if let Some(元素) = self.元素转数字.get(元素名称) {
                        元素序列.push((*元素, index));
                    } else {
                        return Err(format!(
                            "编码对象「{name}」包含的元素「{原始元素}」无法在键盘映射中找到"
                        )
                        .into());
                    }
                    continue;
                } else {
                    let 元素名称 = 原始元素;
                    if let Some(元素) = self.元素转数字.get(元素名称) {
                        (*元素, 0)
                    } else {
                        return Err(format!(
                            "编码对象「{name}」包含的元素「{原始元素}」无法在键盘映射中找到"
                        )
                        .into());
                    }
                };
                元素序列.push((元素, 位置));
            }
            词列表.push(可编码对象 {
                词: name.clone(),
                词长: name.chars().count(),
                频率: frequency,
                简码长度: level,
                元素序列,
                原始顺序,
            });
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
