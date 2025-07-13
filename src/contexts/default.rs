//! 数据结构的定义

use crate::config::{Mapped, MappedKey, Regularization, Scheme, ShortCodeConfig, 配置};
use crate::contexts::上下文;
use crate::encoders::default::简码配置;
use crate::{
    元素, 元素映射, 元素标准名称, 原始可编码对象, 原始当量信息, 原始键位分布信息, 可编码对象,
    当量信息, 最大按键组合长度, 最大词长, 棱镜, 码表项, 编码, 编码信息, 键, 键位分布信息,
};
use crate::错误;
use regex::Regex;
use rustc_hash::FxHashMap;
use serde_yaml::to_string;

/// 将用户提供的输入转换为内部数据结构，并提供了一些实用的方法
#[derive(Debug, Clone)]
pub struct 默认上下文 {
    pub 配置: 配置,
    pub 词列表: Vec<可编码对象>,
    pub 键位分布信息: 键位分布信息,
    pub 当量信息: 当量信息,
    pub 初始映射: 元素映射,
    pub 棱镜: 棱镜,
    pub 选择键: Vec<键>,
}

impl 上下文 for 默认上下文 {
    type 解类型 = 元素映射;
    fn 序列化(&self, 解: &Self::解类型) -> String {
        let mut new_config = self.配置.clone();
        let lookup = |element: &String| {
            let number = *self.棱镜.元素转数字.get(element).unwrap(); // 输入的时候已经检查过一遍，不需要再次检查
            let current_mapped = &解[number];
            *self.棱镜.数字转键.get(current_mapped).unwrap() // 同上
        };
        for (element, mapped) in &self.配置.form.mapping {
            let new_element = element.clone();
            let new_mapped = match mapped {
                Mapped::Basic(string) => {
                    let mut all_codes = String::new();
                    for index in 0..string.len() {
                        let name = 元素标准名称(element, index);
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
                                MappedKey::Ascii(lookup(&元素标准名称(element, index)))
                            }
                            other => other.clone(),
                        })
                        .collect();
                    Mapped::Advanced(all_codes)
                }
            };
            new_config.form.mapping.insert(new_element, new_mapped);
        }
        to_string(&new_config).unwrap()
    }
}

impl 默认上下文 {
    pub fn 新建(
        配置: 配置,
        原始词列表: Vec<原始可编码对象>,
        原始键位分布信息: 原始键位分布信息,
        原始当量信息: 原始当量信息,
    ) -> Result<Self, 错误> {
        let (初始映射, 选择键, 棱镜) = Self::构建棱镜(&配置)?;
        let 最大码长 = 配置.encoder.max_length;
        let 词列表 = 棱镜.预处理词列表(原始词列表, 最大码长)?;
        let 组合长度 = 最大码长.min(最大按键组合长度);
        let 编码空间大小 = 棱镜.进制.pow(组合长度 as u32) as usize;
        let 键位分布信息 = 棱镜.预处理键位分布信息(&原始键位分布信息);
        let 当量信息 = 棱镜.预处理当量信息(&原始当量信息, 编码空间大小);
        Ok(Self {
            配置,
            词列表,
            键位分布信息,
            当量信息,
            初始映射,
            棱镜,
            选择键,
        })
    }

    /// 读取字母表和选择键列表，然后分别对它们的每一个按键转换成无符号整数
    /// 1, ... n = 所有常规编码键
    /// n + 1, ..., m = 所有选择键
    pub fn 构建棱镜(配置: &配置) -> Result<(元素映射, Vec<键>, 棱镜), 错误> {
        let mut 键转数字: FxHashMap<char, 键> = FxHashMap::default();
        let mut 数字转键: FxHashMap<键, char> = FxHashMap::default();
        let mut 数字 = 1;
        for 键 in 配置.form.alphabet.chars() {
            if 键转数字.contains_key(&键) {
                return Err("编码键有重复！".into());
            };
            键转数字.insert(键, 数字);
            数字转键.insert(数字, 键);
            数字 += 1;
        }
        let 默认选择键 = vec!['_'];
        let 原始选择键 = 配置.encoder.select_keys.as_ref().unwrap_or(&默认选择键);
        if 原始选择键.is_empty() {
            return Err("选择键不能为空！".into());
        }
        let mut 选择键: Vec<键> = vec![];
        for 键 in 原始选择键 {
            if 键转数字.contains_key(键) {
                return Err("编码键有重复！".into());
            };
            键转数字.insert(*键, 数字);
            数字转键.insert(数字, *键);
            选择键.push(数字);
            数字 += 1;
        }
        let 进制 = 数字;
        let mut 元素映射: 元素映射 = (0..进制).collect();
        let mut 元素转数字: FxHashMap<String, 元素> = FxHashMap::default();
        let mut 数字转元素: FxHashMap<元素, String> = FxHashMap::default();
        for (键字符, 键) in &键转数字 {
            元素转数字.insert(键字符.to_string(), *键 as usize);
            数字转元素.insert(*键 as usize, 键字符.to_string());
        }
        for (元素, 映射值) in &配置.form.mapping {
            let 映射值 = 映射值.normalize();
            for (序号, 映射键) in 映射值.iter().enumerate() {
                if let MappedKey::Ascii(x) = 映射键 {
                    if let Some(键) = 键转数字.get(x) {
                        let 元素名 = 元素标准名称(元素, 序号);
                        元素转数字.insert(元素名.clone(), 元素映射.len());
                        数字转元素.insert(元素映射.len(), 元素名.clone());
                        元素映射.push(*键);
                    } else {
                        return Err(format!("元素 {元素} 的编码中的字符 {x} 并不在字母表中").into());
                    }
                }
            }
        }
        let 棱镜 = 棱镜 {
            键转数字,
            数字转键,
            元素转数字,
            数字转元素,
            进制,
        };
        Ok((元素映射, 选择键, 棱镜))
    }

    pub fn 预处理正则化(
        正则化: &Regularization,
        元素转数字: &FxHashMap<String, 元素>,
    ) -> Result<FxHashMap<元素, Vec<(元素, f64)>>, 错误> {
        let mut result = FxHashMap::default();
        if let Some(列表) = &正则化.element_affinities {
            for 规则 in 列表 {
                let 元素名称 = 元素标准名称(&规则.from.element, 规则.from.index);
                let 元素 = 元素转数字
                    .get(&元素名称)
                    .ok_or(format!("元素 {元素名称} 不存在"))?;
                let mut 亲和度列表 = Vec::new();
                for 目标 in 规则.to.iter() {
                    let 目标元素名称 =
                        元素标准名称(&目标.element.element, 目标.element.index);
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
                let 元素名称 = 元素标准名称(&规则.from.element, 规则.from.index);
                let 元素 = 元素转数字
                    .get(&元素名称)
                    .ok_or(format!("元素 {元素名称} 不存在"))?;
                let mut 亲和度列表 = Vec::new();
                for 目标 in 规则.to.iter() {
                    let 目标键位 = 元素转数字
                        .get(&目标.key.to_string())
                        .ok_or("目标键位不存在")?;
                    亲和度列表.push((*目标键位, 目标.affinity));
                }
                result.insert(*元素, 亲和度列表);
            }
        }
        Ok(result)
    }

    pub fn 生成码表(&self, 编码结果: &[编码信息]) -> Vec<码表项> {
        let mut 码表: Vec<(usize, 码表项)> = Vec::new();
        let 转编码 = |code: 编码| self.棱镜.数字转编码(code).iter().collect();
        for (序号, 可编码对象) in self.词列表.iter().enumerate() {
            let 码表项 = 码表项 {
                name: 可编码对象.名称.clone(),
                full: 转编码(编码结果[序号].全码.原始编码),
                full_rank: 编码结果[序号].全码.原始编码候选位置,
                short: 转编码(编码结果[序号].简码.原始编码),
                short_rank: 编码结果[序号].简码.原始编码候选位置,
            };
            码表.push((可编码对象.原始顺序, 码表项));
        }
        码表.sort_by_key(|x| x.0);
        码表.into_iter().map(|x| x.1).collect()
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
            let chars = self.棱镜.数字转编码(code as u64);
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
                        .棱镜
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
        原始简码配置列表: Vec<ShortCodeConfig>,
    ) -> Result<[Vec<简码配置>; 最大词长], 错误> {
        let mut short_code: [Vec<简码配置>; 最大词长] = Default::default();
        for config in 原始简码配置列表 {
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
        self.棱镜.进制.pow(max_length as u32) as usize
    }
}
