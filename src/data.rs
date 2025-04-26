//! 数据结构的定义

use crate::{
    config::{Mapped, MappedKey, Regularization, Scheme, ShortCodeConfig, 配置},
    encoders::简码配置,
    objectives::metric::指法标记,
    错误,
};
use regex::Regex;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::HashMap};

/// 只考虑长度为 1 到 10 的词
pub const 最大词长: usize = 10;

/// 只对低于最大按键组合长度的编码预先计算当量
pub const 最大按键组合长度: usize = 4;

/// 从配置文件中读取的原始可编码对象
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct 原始可编码对象 {
    pub name: String,
    pub sequence: String,
    pub frequency: u64,
    #[serde(default = "原始可编码对象::默认级别")]
    pub level: u64,
}

impl 原始可编码对象 {
    const fn 默认级别() -> u64 {
        u64::MAX
    }
}

pub type 原始键位分布信息 = HashMap<char, 键位分布损失函数>;
pub type 键位分布信息 = Vec<键位分布损失函数>;
pub type 原始当量信息 = HashMap<String, f64>;
pub type 当量信息 = Vec<f64>;

/// 键位分布的理想值和惩罚值
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 键位分布损失函数 {
    pub ideal: f64,
    pub lt_penalty: f64,
    pub gt_penalty: f64,
}

/// 元素用一个无符号整数表示
pub type 元素 = usize;

/// 可编码对象的序列
pub type 元素序列 = Vec<元素>;

/// 编码用无符号整数表示
pub type 编码 = u64;

/// 包含词、词长、元素序列、频率等信息
#[derive(Debug, Clone)]
pub struct 可编码对象 {
    pub 名称: String,
    pub 词长: usize,
    pub 元素序列: 元素序列,
    pub 频率: u64,
    pub 简码等级: u64,
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

/// 元素映射用一个数组表示，下标是元素
pub type 元素映射 = Vec<键>;

/// 用指标记
pub type 指法向量 = [u8; 8];

/// 自动上屏判断数组
pub type 自动上屏 = Vec<bool>;

/// 用于输出为文本码表，包含了名称、全码、简码、全码排名和简码排名
#[derive(Debug, Serialize)]
pub struct 码表项 {
    pub name: String,
    pub full: String,
    pub full_rank: u8,
    pub short: String,
    pub short_rank: u8,
}

pub type 正则化 = FxHashMap<元素, Vec<(元素, f64)>>;

/// 将用户提供的输入转换为内部数据结构，并提供了一些实用的方法
#[derive(Debug, Clone)]
pub struct 数据 {
    pub 配置: 配置,
    pub 词列表: Vec<可编码对象>,
    pub 键位分布信息: 键位分布信息,
    pub 当量信息: 当量信息,
    pub 初始映射: 元素映射,
    pub 正则化: 正则化,
    pub 进制: u64,
    pub 选择键: Vec<键>,
    pub 键转数字: FxHashMap<char, 键>,
    pub 数字转键: FxHashMap<键, char>,
    pub 元素转数字: FxHashMap<String, 元素>,
    pub 数字转元素: FxHashMap<元素, String>,
}

impl Mapped {
    pub fn length(&self) -> usize {
        match self {
            Mapped::Basic(s) => s.len(),
            Mapped::Advanced(v) => v.len(),
        }
    }

    pub fn normalize(&self) -> Vec<MappedKey> {
        match self {
            Mapped::Advanced(vector) => vector.clone(),
            Mapped::Basic(string) => string.chars().map(MappedKey::Ascii).collect(),
        }
    }
}

type 字母表信息 = (u64, Vec<键>, FxHashMap<char, 键>, FxHashMap<键, char>);
type 映射信息 = (元素映射, FxHashMap<String, 元素>, FxHashMap<元素, String>);

impl 数据 {
    pub fn 新建(
        配置: 配置,
        原始词列表: Vec<原始可编码对象>,
        原始键位分布信息: 原始键位分布信息,
        原始当量信息: 原始当量信息,
    ) -> Result<Self, 错误> {
        let (进制, 选择键, 键转数字, 数字转键) = Self::预处理字母表(&配置)?;
        let (初始映射, 元素转数字, 数字转元素) = Self::预处理映射(&配置, &键转数字, 进制)?;
        let 最大码长 = 配置.encoder.max_length;
        let 词列表 = Self::预处理词列表(原始词列表, 最大码长, &元素转数字)?;
        let 组合长度 = 最大码长.min(最大按键组合长度);
        let 编码空间大小 = 进制.pow(组合长度 as u32) as usize;
        let 键位分布信息 = Self::预处理键位分布信息(&原始键位分布信息, 进制, &数字转键);
        let 当量信息 = Self::预处理当量信息(&原始当量信息, 编码空间大小, 进制, &数字转键);
        let 正则化 = if let Some(正则化配置) = 配置
            .optimization
            .clone()
            .and_then(|x| Some(x.objective))
            .and_then(|x| Some(x.regularization))
            .flatten()
        {
            Self::预处理正则化(&正则化配置, &元素转数字)?
        } else {
            FxHashMap::default()
        };
        let repr = Self {
            配置,
            词列表,
            键位分布信息,
            当量信息,
            初始映射,
            元素转数字,
            数字转元素,
            键转数字,
            数字转键,
            进制,
            选择键,
            正则化,
        };
        Ok(repr)
    }

    /// 读取字母表和选择键列表，然后分别对它们的每一个按键转换成无符号整数
    /// 1, ... n = 所有常规编码键
    /// n + 1, ..., m = 所有选择键
    pub fn 预处理字母表(config: &配置) -> Result<字母表信息, 错误> {
        let mut 键转数字: FxHashMap<char, 键> = FxHashMap::default();
        let mut 数字转键: FxHashMap<键, char> = FxHashMap::default();
        let mut index = 1;
        for key in config.form.alphabet.chars() {
            if 键转数字.contains_key(&key) {
                return Err("编码键有重复！".into());
            };
            键转数字.insert(key, index);
            数字转键.insert(index, key);
            index += 1;
        }
        let default_select_keys = vec!['_'];
        let select_keys = config
            .encoder
            .select_keys
            .as_ref()
            .unwrap_or(&default_select_keys);
        if select_keys.is_empty() {
            return Err("选择键不能为空！".into());
        }
        let mut parsed_select_keys: Vec<键> = vec![];
        for key in select_keys {
            if 键转数字.contains_key(key) {
                return Err("编码键有重复！".into());
            };
            键转数字.insert(*key, index);
            数字转键.insert(index, *key);
            parsed_select_keys.push(index);
            index += 1;
        }
        let radix = index;
        Ok((radix, parsed_select_keys, 键转数字, 数字转键))
    }

    /// 读取元素映射，然后把每一个元素转换成无符号整数，从而可以用向量来表示一个元素布局，向量的下标就是元素对应的数
    pub fn 预处理映射(
        配置: &配置,
        键转数字: &FxHashMap<char, 键>,
        进制: u64,
    ) -> Result<映射信息, 错误> {
        let mut 元素映射: 元素映射 = (0..进制).collect();
        let mut 元素转数字: FxHashMap<String, 元素> = FxHashMap::default();
        let mut 数字转元素: FxHashMap<元素, String> = FxHashMap::default();
        for (键字符, 键) in 键转数字 {
            元素转数字.insert(键字符.to_string(), *键 as usize);
            数字转元素.insert(*键 as usize, 键字符.to_string());
        }
        for (元素, 映射值) in &配置.form.mapping {
            let 映射值 = 映射值.normalize();
            for (序号, 映射键) in 映射值.iter().enumerate() {
                if let MappedKey::Ascii(x) = 映射键 {
                    if let Some(键) = 键转数字.get(x) {
                        let 元素名 = Self::序列化(元素, 序号);
                        元素转数字.insert(元素名.clone(), 元素映射.len());
                        数字转元素.insert(元素映射.len(), 元素名.clone());
                        元素映射.push(*键);
                    } else {
                        return Err(format!("元素 {元素} 的编码中的字符 {x} 并不在字母表中").into());
                    }
                }
            }
        }
        Ok((元素映射, 元素转数字, 数字转元素))
    }

    pub fn 预处理正则化(
        正则化: &Regularization,
        元素转数字: &FxHashMap<String, 元素>,
    ) -> Result<FxHashMap<元素, Vec<(元素, f64)>>, 错误> {
        let mut result = FxHashMap::default();
        if let Some(列表) = &正则化.element_affinities {
            for 规则 in 列表 {
                let 元素名称 = Self::序列化(&规则.from.element, 规则.from.index);
                let 元素 = 元素转数字
                    .get(&元素名称)
                    .ok_or(format!("元素 {元素名称} 不存在"))?;
                let mut 亲和度列表 = Vec::new();
                for 目标 in 规则.to.iter() {
                    let 目标元素名称 = Self::序列化(&目标.element.element, 目标.element.index);
                    let 目标元素 = 元素转数字
                        .get(&目标元素名称)
                        .ok_or(format!("目标元素 {目标元素名称} 不存在"))?;
                    亲和度列表.push((*目标元素, 目标.affinity));
                }
                result.insert(*元素, 亲和度列表);
            }
        }
        if let Some(列表) = &正则化.key_affinities {
            for 规则 in 列表 {
                let 元素名称 = Self::序列化(&规则.from.element, 规则.from.index);
                let 元素 = 元素转数字
                    .get(&元素名称)
                    .ok_or(format!("元素 {元素名称} 不存在"))?;
                let mut 亲和度列表 = Vec::new();
                for 目标 in 规则.to.iter() {
                    let 目标键位 = 元素转数字
                        .get(&目标.key.to_string())
                        .ok_or(format!("目标键位不存在"))?;
                    亲和度列表.push((*目标键位, 目标.affinity));
                }
                result.insert(*元素, 亲和度列表);
            }
        }
        Ok(result)
    }
    pub fn 序列化(element: &String, index: usize) -> String {
        if index == 0 {
            element.to_string()
        } else {
            format!("{}.{}", element, index)
        }
    }

    /// 读取拆分表，将拆分序列中的每一个元素按照先前确定的元素 -> 整数映射来转换为整数向量
    pub fn 预处理词列表(
        raw_encodables: Vec<原始可编码对象>,
        max_length: usize,
        element_repr: &FxHashMap<String, 元素>,
    ) -> Result<Vec<可编码对象>, 错误> {
        let mut encodables = Vec::new();
        for (index, assemble) in raw_encodables.into_iter().enumerate() {
            let 原始可编码对象 {
                name,
                frequency,
                level,
                sequence,
            } = assemble;
            let raw_sequence: Vec<_> = sequence.split(' ').collect();
            let mut sequence = 元素序列::new();
            let length = raw_sequence.len();
            if length > max_length {
                return Err(format!(
                    "编码对象「{name}」包含的元素数量为 {length}，超过了最大码长 {max_length}"
                )
                .into());
            }
            for element in raw_sequence {
                if let Some(number) = element_repr.get(element) {
                    sequence.push(*number);
                } else {
                    return Err(format!(
                        "编码对象「{name}」包含的元素「{element}」无法在键盘映射中找到"
                    )
                    .into());
                }
            }
            encodables.push(可编码对象 {
                名称: name.clone(),
                词长: name.chars().count(),
                元素序列: sequence,
                频率: frequency,
                简码等级: level,
                原始顺序: index,
            });
        }

        encodables.sort_by_key(|x| Reverse(x.频率));
        Ok(encodables)
    }

    pub fn 生成码表(&self, buffer: &[编码信息]) -> Vec<码表项> {
        let mut entries: Vec<(usize, 码表项)> = Vec::new();
        let encodables = &self.词列表;
        let recover = |code: 编码| {
            Self::数字转编码(code, self.进制, &self.数字转键)
                .iter()
                .collect()
        };
        for (index, encodable) in encodables.iter().enumerate() {
            let entry = 码表项 {
                name: encodable.名称.clone(),
                full: recover(buffer[index].全码.原始编码),
                full_rank: buffer[index].全码.原始编码候选位置,
                short: recover(buffer[index].简码.原始编码),
                short_rank: buffer[index].简码.原始编码候选位置,
            };
            entries.push((encodable.原始顺序, entry));
        }
        entries.sort_by_key(|x| x.0);
        entries.into_iter().map(|x| x.1).collect()
    }

    /// 根据一个计算中得到的元素布局来生成一份新的配置文件，其余内容不变直接复制过来
    pub fn 更新配置(&self, candidate: &元素映射) -> 配置 {
        let mut new_config = self.配置.clone();
        let lookup = |element: &String| {
            let number = *self.元素转数字.get(element).unwrap(); // 输入的时候已经检查过一遍，不需要再次检查
            let current_mapped = &candidate[number];
            *self.数字转键.get(current_mapped).unwrap() // 同上
        };
        for (element, mapped) in &self.配置.form.mapping {
            let new_element = element.clone();
            let new_mapped = match mapped {
                Mapped::Basic(string) => {
                    let mut all_codes = String::new();
                    for index in 0..string.len() {
                        let name = Self::序列化(element, index);
                        all_codes.push(lookup(&name));
                    }
                    Mapped::Basic(all_codes)
                }
                Mapped::Advanced(vector) => {
                    let all_codes: Vec<MappedKey> = vector
                        .iter()
                        .enumerate()
                        .map(|(index, mapped_key)| match mapped_key {
                            MappedKey::Ascii(_) => {
                                MappedKey::Ascii(lookup(&Self::序列化(element, index)))
                            }
                            other => other.clone(),
                        })
                        .collect();
                    Mapped::Advanced(all_codes)
                }
            };
            new_config.form.mapping.insert(new_element, new_mapped);
        }
        new_config
    }

    /// 如前所述，建立了一个按键到整数的映射之后，可以将字符串看成具有某个进制的数。所以，给定一个数，也可以把它转化为字符串
    pub fn 数字转编码(
        code: 编码, 进制: u64, repr_key: &FxHashMap<键, char>
    ) -> Vec<char> {
        let mut chars = Vec::new();
        let mut remainder = code;
        while remainder > 0 {
            let k = remainder % 进制;
            remainder /= 进制;
            if k == 0 {
                continue;
            }
            let char = repr_key.get(&k).unwrap(); // 从内部表示转换为字符，不需要检查
            chars.push(*char);
        }
        chars
    }

    /// 根据编码字符和未归一化的键位分布，生成一个理想的键位分布
    pub fn 预处理键位分布信息(
        原始键位分布信息: &原始键位分布信息,
        进制: u64,
        数字转键: &FxHashMap<键, char>,
    ) -> Vec<键位分布损失函数> {
        let default_loss = 键位分布损失函数 {
            ideal: 0.0,
            lt_penalty: 0.0,
            gt_penalty: 1.0,
        };
        let mut 键位分布信息: Vec<键位分布损失函数> = (0..进制)
            .map(|键| {
                // 0 只是为了占位，不需要统计
                if 键 == 0 {
                    default_loss.clone()
                } else {
                    let 键名称 = 数字转键[&键];
                    原始键位分布信息
                        .get(&键名称)
                        .unwrap_or(&default_loss)
                        .clone()
                }
            })
            .collect();
        键位分布信息.iter_mut().for_each(|x| {
            x.ideal /= 100.0;
        });
        键位分布信息
    }

    /// 将编码空间内所有的编码组合预先计算好速度当量
    /// 按照这个字符串所对应的整数为下标，存储到一个大数组中
    pub fn 预处理当量信息(
        原始当量信息: &原始当量信息,
        space: usize,
        进制: u64,
        数字转键: &FxHashMap<键, char>,
    ) -> Vec<f64> {
        let mut result: Vec<f64> = vec![0.0; space];
        for (index, equivalence) in result.iter_mut().enumerate() {
            let chars = Self::数字转编码(index as u64, 进制, 数字转键);
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
    pub fn 预处理指法标记(&self) -> Vec<指法向量> {
        let 指法标记 = 指法标记::new();
        let mut result: Vec<指法向量> = Vec::with_capacity(self.get_space());
        for code in 0..self.get_space() {
            let chars = Self::数字转编码(code as u64, self.进制, &self.数字转键);
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

    /// 将编码空间内所有的编码组合预先计算好是否能自动上屏
    /// 按照这个字符串所对应的整数为下标，存储到一个大数组中
    pub fn 预处理自动上屏(&self) -> Result<Vec<bool>, 错误> {
        let mut result: Vec<bool> = vec![];
        let encoder = &self.配置.encoder;
        let mut re: Option<Regex> = None;
        if let Some(pattern) = &encoder.auto_select_pattern {
            let re_or_error = Regex::new(pattern);
            if let Ok(regex) = re_or_error {
                re = Some(regex);
            } else {
                return Err(format!("正则表达式 {pattern} 无法解析").into());
            }
        }
        for code in 0..self.get_space() {
            let chars = Self::数字转编码(code as u64, self.进制, &self.数字转键);
            let string: String = chars.iter().collect();
            let is_matched = if let Some(re) = &re {
                re.is_match(&string)
            } else if let Some(length) = encoder.auto_select_length {
                chars.len() >= length
            } else {
                true
            };
            let is_max_length = chars.len() == encoder.max_length;
            result.push(is_matched || is_max_length);
        }
        Ok(result)
    }

    pub fn 预处理简码规则(
        &self,
        schemes: &Vec<Scheme>,
    ) -> Result<Vec<简码配置>, 错误> {
        let mut compiled_schemes = Vec::new();
        for scheme in schemes {
            let prefix = scheme.prefix;
            let count = scheme.count.unwrap_or(1);
            let select_keys = if let Some(keys) = &scheme.select_keys {
                let mut transformed_keys = Vec::new();
                for key in keys {
                    let transformed_key = self
                        .键转数字
                        .get(key)
                        .ok_or(format!("简码的选择键 {key} 不在全局选择键中"))?;
                    transformed_keys.push(*transformed_key);
                }
                transformed_keys
            } else {
                self.选择键.clone()
            };
            if count > select_keys.len() {
                return Err("选重数量不能高于选择键数量".into());
            }
            compiled_schemes.push(简码配置 {
                prefix,
                select_keys: select_keys[..count].to_vec(),
            });
        }
        Ok(compiled_schemes)
    }

    pub fn 预处理简码配置(
        &self,
        configs: Vec<ShortCodeConfig>,
    ) -> Result<[Vec<简码配置>; 最大词长], 错误> {
        let mut short_code: [Vec<简码配置>; 最大词长] = Default::default();
        for config in configs {
            match config {
                ShortCodeConfig::Equal {
                    length_equal,
                    schemes,
                } => {
                    short_code[length_equal - 1].extend(self.预处理简码规则(&schemes)?);
                }
                ShortCodeConfig::Range {
                    length_in_range: (from, to),
                    schemes,
                } => {
                    for length in from..=to {
                        short_code[length - 1].extend(self.预处理简码规则(&schemes)?);
                    }
                }
            }
        }
        Ok(short_code)
    }

    pub fn get_space(&self) -> usize {
        let max_length = self.配置.encoder.max_length.min(最大按键组合长度);
        self.进制.pow(max_length as u32) as usize
    }
}
