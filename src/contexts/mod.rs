use crate::{
    config::{安排, 广义码位, 决策生成器规则, 变量规则, 安排描述},
    optimizers::决策,
    元素, 错误,
};
use indexmap::IndexMap;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;
pub mod default;

pub trait 上下文 {
    type 决策: 决策;

    fn 序列化(&self, 决策: &Self::决策) -> String;
}

#[derive(Debug, Clone)]
pub struct 条件<T> {
    pub 元素: 元素,
    pub 谓词: bool,
    pub 值: T,
}

#[derive(Debug, Clone)]
pub struct 条件安排<T> {
    pub 安排: T,
    pub 分数: f64,
    pub 条件: Vec<条件<T>>,
}

pub fn 合并初始决策(
    原始决策空间: &mut IndexMap<String, Vec<安排描述>>,
    原始决策: &mut IndexMap<String, 安排>,
) {
    for 元素名称 in 原始决策.keys() {
        if !原始决策空间.contains_key(元素名称) {
            原始决策空间.insert(元素名称.clone(), vec![]);
        }
    }
    // 移除多余元素
    for 元素名称 in 原始决策空间.keys() {
        if !原始决策.contains_key(元素名称) {
            原始决策.insert(元素名称.clone(), 安排::Unused(()));
        }
    }
    // 确保每个元素的当前决策都在决策空间中
    for (元素名称, 原始安排列表) in 原始决策空间.iter_mut() {
        let 原始安排 = 原始决策[元素名称].clone();
        if !原始安排列表.iter().any(|x| &x.value == &原始安排) {
            原始安排列表.insert(
                0,
                安排描述 {
                    value: 原始安排,
                    score: 0.0,
                    condition: None,
                },
            );
        }
    }
}

pub fn 展开变量(
    原始决策空间: &mut IndexMap<String, Vec<安排描述>>,
    原始变量映射: &IndexMap<String, 变量规则>,
) {
    for (_, 原始安排列表) in 原始决策空间.iter_mut() {
        let mut 队列 = VecDeque::from(原始安排列表.clone());
        原始安排列表.clear();
        while let Some(原始安排) = 队列.pop_front() {
            let mut 是否展开 = false;
            if let 安排::Advanced(keys) = &原始安排.value {
                for (序号, 映射键) in keys.iter().enumerate() {
                    if let 广义码位::Variable {
                        variable: generator,
                    } = 映射键
                    {
                        let 变量取值列表 = 原始变量映射.get(generator).unwrap();
                        for 变量取值 in &变量取值列表.keys {
                            let mut 新映射键列表 = keys.clone();
                            新映射键列表[序号] = 广义码位::Ascii(*变量取值);
                            let 新原始安排 = 安排描述 {
                                value: 安排::Advanced(新映射键列表),
                                score: 原始安排.score,
                                condition: 原始安排.condition.clone(),
                            };
                            队列.push_back(新原始安排);
                        }
                        是否展开 = true;
                        break;
                    }
                }
            }
            if !是否展开 {
                原始安排列表.push(原始安排);
            }
        }
    }
}

pub fn 拓扑排序(
    原始决策空间: &IndexMap<String, Vec<安排描述>>,
) -> Result<(Vec<String>, FxHashMap<String, Vec<String>>), 错误> {
    // 构造入度表
    let mut 入度 = FxHashMap::default();
    let mut 元素图 = FxHashMap::default();
    for 元素名称 in 原始决策空间.keys() {
        入度.insert(元素名称.clone(), 0);
        元素图.insert(元素名称.clone(), vec![]);
    }
    for (元素名称, 原始安排列表) in 原始决策空间 {
        let mut 依赖 = FxHashSet::default();
        for 原始安排 in 原始安排列表 {
            if let 安排::Advanced(keys) = &原始安排.value {
                for k in keys {
                    if let 广义码位::Reference { element, .. } = k {
                        依赖.insert(element.clone());
                    }
                }
            } else if let 安排::Grouped { element } = &原始安排.value {
                依赖.insert(element.clone());
            }
            if let Some(条件列表) = &原始安排.condition {
                for 条件 in 条件列表 {
                    依赖.insert(条件.element.clone());
                }
            }
        }
        for 依赖元素 in &依赖 {
            元素图.get_mut(依赖元素).map(|v| {
                v.push(元素名称.clone());
                *入度.get_mut(元素名称).unwrap() += 1;
            });
        }
    }

    // 拓扑排序
    let mut 队列 = VecDeque::new();
    for (元素名称, d) in &入度 {
        if *d == 0 {
            队列.push_back(元素名称.clone());
        }
    }

    let mut 排序后元素名称 = Vec::new();
    while let Some(u) = 队列.pop_front() {
        排序后元素名称.push(u.clone());
        for v in &元素图[&u] {
            let deg = 入度.get_mut(v).unwrap();
            *deg -= 1;
            if *deg == 0 {
                队列.push_back(v.clone());
            }
        }
    }

    // 检测环
    if 排序后元素名称.len() != 原始决策空间.len() {
        let remaining: Vec<_> = 入度
            .into_iter()
            .filter(|(_, deg)| *deg > 0)
            .map(|(k, _)| k)
            .collect();
        return Err(format!("检测到依赖环，无法进行拓扑排序，剩余节点：{:?}", remaining).into());
    }

    Ok((排序后元素名称, 元素图))
}

pub fn 应用生成器(
    原始决策空间: &mut IndexMap<String, Vec<安排描述>>,
    原始生成器列表: &Vec<决策生成器规则>,
) {
    for 生成器 in 原始生成器列表 {
        let regex = Regex::new(&生成器.regex).unwrap();
        for (元素名称, 原始安排列表) in 原始决策空间.iter_mut() {
            if !regex.is_match(元素名称) {
                continue;
            }
            if let 安排::Advanced(keys) = &生成器.value.value {
                let mut values = FxHashSet::default();
                for 现有安排 in 原始安排列表.iter() {
                    if matches!(&现有安排.value, 安排::Basic(_) | 安排::Advanced(_)) {
                        let 现有键 = &现有安排.value.normalize();
                        let mut valid = true;
                        let 合成 = keys
                            .iter()
                            .enumerate()
                            .map(|(i, x)| {
                                let k = &现有键[i];
                                // 引用不能被替换为变量
                                if matches!(k, 广义码位::Reference { .. }) {
                                    if matches!(x, 广义码位::Variable { .. }) {
                                        valid = false;
                                    }
                                }
                                if let 广义码位::Placeholder(_) = x {
                                    k.clone()
                                } else {
                                    x.clone()
                                }
                            })
                            .collect();
                        if !valid {
                            continue;
                        }
                        values.insert(合成);
                    }
                }
                for value in values {
                    let 新原始安排 = 安排描述 {
                        value: 安排::Advanced(value),
                        score: 生成器.value.score,
                        condition: 生成器.value.condition.clone(),
                    };
                    原始安排列表.push(新原始安排);
                }
            } else {
                原始安排列表.push(生成器.value.clone());
            }
        }
    }
}
