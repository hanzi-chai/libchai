//! 数据结构的定义

use crate::config::{Mapped, MappedKey, Scheme, ShortCodeConfig, ValueDescription, 配置};
use crate::contexts::{
    上下文, 合并初始决策, 展开变量, 应用生成器, 拓扑排序, 条件, 条件安排
};
use crate::encoders::default::简码配置;
use crate::interfaces::默认输入;
use crate::optimizers::决策;
use crate::{
    元素, 元素图, 可编码对象, 当量信息, 最大按键组合长度, 最大词长, 棱镜, 码表项, 编码, 编码信息,
    键, 键位分布信息,
};
use crate::{最大元素编码长度, 错误};
use indexmap::IndexMap;
use itertools::Itertools;
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
    pub 初始决策: 默认决策,
    pub 决策空间: 默认决策空间,
    pub 棱镜: 棱镜,
    pub 选择键: Vec<键>,
    pub 元素图: 元素图,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum 默认安排 {
    键位([(元素, usize); 最大元素编码长度]),
    归并(元素),
}

impl 默认安排 {
    pub fn from(原始安排: &Mapped, 棱镜: &棱镜, 元素: &String) -> Result<Self, 错误> {
        if matches!(原始安排, Mapped::Basic(_) | Mapped::Advanced(_)) {
            let 归一化映射值 = 原始安排.normalize();
            let mut 安排 = [(0, 0); 最大元素编码长度];
            for (序号, 映射键) in 归一化映射值.iter().enumerate() {
                if let MappedKey::Ascii(x) = 映射键 {
                    if let Some(键) = 棱镜.键转数字.get(x) {
                        安排[序号] = (*键 as usize, 0);
                    } else {
                        return Err(format!("元素 {元素} 的编码中的字符 {x} 并不在字母表中").into());
                    }
                } else if let MappedKey::Reference { element, index } = 映射键 {
                    if let Some(元素编号) = 棱镜.元素转数字.get(element) {
                        安排[序号] = (*元素编号, *index);
                    } else {
                        return Err(
                            format!("元素 {元素} 的编码中的引用元素 {element} 并不存在").into()
                        );
                    }
                } else {
                    return Err(format!("元素 {元素} 的编码格式不正确").into());
                }
            }
            Ok(默认安排::键位(安排))
        } else {
            let Mapped::Grouped { element } = 原始安排 else {
                return Err(format!("元素 {元素} 的编码格式不正确").into());
            };
            if let Some(元素编号) = 棱镜.元素转数字.get(element) {
                Ok(默认安排::归并(*元素编号))
            } else {
                Err(format!("元素 {元素} 的编码中的引用元素 {element} 并不存在").into())
            }
        }
    }

    pub fn to(&self, 棱镜: &棱镜) -> Mapped {
        match self {
            默认安排::归并(引用元素) => Mapped::Grouped {
                element: 棱镜.数字转元素[引用元素].clone(),
            },
            默认安排::键位(取值) => {
                let mut 列表 = vec![];
                for (元素, 位置) in 取值 {
                    if *元素 == 0 {
                        break;
                    } else if 棱镜.数字转键.contains_key(&(*元素 as u64)) {
                        let 键 = 棱镜.数字转键[&(*元素 as u64)];
                        列表.push(MappedKey::Ascii(键));
                    } else {
                        let 元素名称 = 棱镜.数字转元素[元素].clone();
                        列表.push(MappedKey::Reference {
                            element: 元素名称,
                            index: *位置,
                        });
                    }
                }
                if 列表.iter().all(|x| matches!(x, MappedKey::Ascii(_))) {
                    Mapped::Basic(
                        列表
                            .iter()
                            .map(|x| match x {
                                MappedKey::Ascii(c) => *c,
                                _ => unreachable!(),
                            })
                            .collect(),
                    )
                } else {
                    Mapped::Advanced(列表)
                }
            }
        }
    }
}

type 默认条件安排 = 条件安排<默认安排>;

#[derive(Debug, Clone)]
pub struct 默认决策 {
    pub 元素: Vec<默认安排>,
}

impl 默认决策 {
    pub fn 允许(&self, 条件安排: &默认条件安排) -> bool {
        for 条件 in &条件安排.条件 {
            if 条件.谓词 != (self.元素[条件.元素] == 条件.值) {
                return false;
            }
        }
        return true;
    }
}

impl 决策 for 默认决策 {
    type 变化 = Vec<元素>;

    fn 除法(旧变化: &Self::变化, 新变化: &Self::变化) -> Self::变化 {
        旧变化
            .iter()
            .chain(新变化.iter())
            .unique()
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct 默认决策空间 {
    pub 元素: Vec<Vec<默认条件安排>>,
}

impl 上下文 for 默认上下文 {
    type 决策 = 默认决策;
    fn 序列化(&self, 决策: &Self::决策) -> String {
        let mut 新配置 = self.配置.clone();
        for (元素名称, 安排) in 新配置.form.mapping.iter_mut() {
            let 元素 = *self.棱镜.元素转数字.get(元素名称).unwrap();
            let 新安排 = 决策.元素[元素].to(&self.棱镜);
            *安排 = 新安排;
        }
        to_string(&新配置).unwrap()
    }
}

impl 默认上下文 {
    pub fn 新建(输入: 默认输入) -> Result<Self, 错误> {
        let (初始决策, 决策空间, 元素图, 选择键, 棱镜) =
            Self::构建棱镜和初始决策(&输入.配置)?;
        let 最大码长 = 输入.配置.encoder.max_length;
        let 词列表 = 棱镜.预处理词列表(输入.词列表, 最大码长)?;
        let 组合长度 = 最大码长.min(最大按键组合长度);
        let 编码空间大小 = 棱镜.进制.pow(组合长度 as u32) as usize;
        let 键位分布信息 = 棱镜.预处理键位分布信息(&输入.原始键位分布信息);
        let 当量信息 = 棱镜.预处理当量信息(&输入.原始当量信息, 编码空间大小);
        Ok(Self {
            配置: 输入.配置,
            词列表,
            键位分布信息,
            当量信息,
            初始决策,
            棱镜,
            选择键,
            决策空间,
            元素图,
        })
    }

    pub fn 构建棱镜和初始决策(
        配置: &配置,
    ) -> Result<(默认决策, 默认决策空间, 元素图, Vec<键>, 棱镜), 错误> {
        // 1. 构建初始决策和决策空间
        let 原始决策 = 配置.form.mapping.clone();
        let mut 原始决策空间 = 配置.form.mapping_space.clone().unwrap_or_default();
        let 原始变量映射 = 配置.form.mapping_variables.clone().unwrap_or_default();
        let 原始生成器列表 = 配置.form.mapping_generators.clone().unwrap_or_default();
        // 合并初始决策
        合并初始决策(&mut 原始决策空间, &原始决策);
        // 应用生成器
        应用生成器(&mut 原始决策空间, &原始生成器列表);
        // 展开变量
        展开变量(&mut 原始决策空间, &原始变量映射);
        // 拓扑排序
        let (排序后元素名称, 原始元素图) = 拓扑排序(&原始决策空间)?;

        // 2. 构建棱镜
        let mut 键转数字: FxHashMap<char, 键> = FxHashMap::default();
        let mut 数字转键: FxHashMap<键, char> = FxHashMap::default();
        let mut 元素转数字: FxHashMap<String, 元素> = FxHashMap::default();
        let mut 数字转元素: FxHashMap<元素, String> = FxHashMap::default();
        let 原始选择键 = 配置.encoder.select_keys.clone().unwrap_or(vec!['_']);
        if 原始选择键.is_empty() {
            return Err("选择键不能为空！".into());
        }
        for 键 in 配置.form.alphabet.chars().chain(原始选择键.iter().cloned()) {
            if 键转数字.contains_key(&键) {
                return Err("编码键有重复！".into());
            };
            let 键编号 = 键转数字.len() + 1;
            键转数字.insert(键, 键编号 as 键);
            数字转键.insert(键编号 as 键, 键);
            元素转数字.insert(键.to_string(), 键编号);
            数字转元素.insert(键编号, 键.to_string());
        }
        let 进制 = 键转数字.len() as 键 + 1;
        let 选择键 = 原始选择键
            .iter()
            .map(|k| *键转数字.get(k).unwrap())
            .collect();
        for 元素名称 in &排序后元素名称 {
            let 元素编号 = 元素转数字.len() + 1;
            元素转数字.insert(元素名称.clone(), 元素编号);
            数字转元素.insert(元素编号, 元素名称.clone());
        }
        let 棱镜 = 棱镜 {
            键转数字,
            数字转键,
            元素转数字,
            数字转元素,
            进制,
        };

        // 3. 使用棱镜构建初始决策和决策空间
        let (初始决策, 决策空间, 元素图) = Self::构建初始决策和决策空间(
            &棱镜,
            &排序后元素名称,
            &原始决策,
            &原始决策空间,
            &原始元素图,
        )?;
        Ok((初始决策, 决策空间, 元素图, 选择键, 棱镜))
    }

    pub fn 构建初始决策和决策空间(
        棱镜: &棱镜,
        排序后元素名称: &Vec<String>,
        原始决策: &IndexMap<String, Mapped>,
        原始决策空间: &IndexMap<String, Vec<ValueDescription>>,
        原始元素图: &FxHashMap<String, Vec<String>>,
    ) -> Result<(默认决策, 默认决策空间, 元素图), 错误> {
        // 3. 使用棱镜构建初始决策和决策空间
        let mut 初始决策 = 默认决策 { 元素: vec![] };
        let mut 决策空间 = 默认决策空间 { 元素: vec![] };
        let mut 元素图: FxHashMap<元素, Vec<_>> = FxHashMap::default();
        for k in 0..棱镜.进制 {
            let 安排 = 默认安排::键位([(k as usize, 0), (0, 0), (0, 0), (0, 0)]);
            let 条件安排 = 默认条件安排 {
                安排: 安排.clone(),
                条件: vec![],
                分数: 0.0,
            };
            初始决策.元素.push(安排);
            决策空间.元素.push(vec![条件安排]);
        }
        for 元素名称 in 排序后元素名称 {
            let 原始安排 = &原始决策[元素名称];
            let mut 安排列表 = vec![];
            let 原始安排列表 = 原始决策空间[元素名称].clone();
            let 编号 = 棱镜.元素转数字[元素名称];
            let 安排 = 默认安排::from(原始安排, &棱镜, 元素名称)?;
            for 其余原始安排 in &原始安排列表 {
                let mut 条件列表 = vec![];
                for c in 其余原始安排.condition.clone().unwrap_or_default() {
                    条件列表.push(条件 {
                        元素: 棱镜.元素转数字[&c.element],
                        谓词: c.op == "是",
                        值: 默认安排::from(&c.value, &棱镜, &c.element)?,
                    });
                }
                let 条件字根安排 = 默认条件安排 {
                    安排: 默认安排::from(&其余原始安排.value, &棱镜, 元素名称)?,
                    条件: 条件列表,
                    分数: 其余原始安排.score,
                };
                安排列表.push(条件字根安排);
            }
            初始决策.元素.push(安排);
            决策空间.元素.push(安排列表);
            let 下游 = 原始元素图.get(元素名称).unwrap();
            let 下游编号: Vec<_> = 下游.iter().map(|x| 棱镜.元素转数字[x]).collect();
            元素图.insert(编号, 下游编号);
        }
        Ok((初始决策, 决策空间, 元素图))
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
