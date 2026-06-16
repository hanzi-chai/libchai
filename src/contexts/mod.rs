use crate::{
    config::{安排, 安排描述, 广义码位},
    optimizers::决策,
    元素, 错误,
};
use indexmap::IndexMap;
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
