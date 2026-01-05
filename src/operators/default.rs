use super::变异;
use crate::contexts::default::{默认上下文, 默认决策, 默认决策空间};
use crate::optimizers::决策;
use crate::错误;
use crate::{元素, 元素图};
use rand::seq::{IndexedRandom, IteratorRandom};
use rand::{random_range, rng};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::VecDeque;

pub struct 默认操作 {
    决策空间: 默认决策空间,
    元素图: 元素图,
}

#[skip_serializing_none]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct 变异配置 {
    pub random_move: f64,
    pub random_swap: f64,
    pub random_full_key_swap: f64,
}

pub const DEFAULT_MUTATE: 变异配置 = 变异配置 {
    random_move: 0.9,
    random_swap: 0.09,
    random_full_key_swap: 0.01,
};

impl 变异 for 默认操作 {
    type 决策 = 默认决策;
    fn 变异(&mut self, 决策: &mut Self::决策) -> <默认决策 as 决策>::变化 {
        let mut 变化 = self.随机移动(决策);
        self.传播(&mut 变化, 决策);
        变化
    }
}

// 默认的问题实现，使用配置文件中的约束来定义各种算子
impl 默认操作 {
    pub fn 新建(上下文: &默认上下文) -> Result<Self, 错误> {
        Ok(Self {
            决策空间: 上下文.决策空间.clone(),
            元素图: 上下文.元素图.clone(),
        })
    }

    fn 传播(&self, 变化: &mut <默认决策 as 决策>::变化, 决策: &mut 默认决策) {
        // 初始化队列
        let mut 队列 = VecDeque::new();
        for 元素 in 变化.iter() {
            for 下游元素 in self.元素图.get(元素).unwrap_or(&vec![]) {
                if !队列.contains(下游元素) {
                    队列.push_back(下游元素.clone());
                }
            }
        }
        // 传播直到队列为空
        let mut iters = 0;
        while !队列.is_empty() {
            iters += 1;
            if iters > 100 {
                panic!("传播超过 100 次仍未结束，可能出现死循环");
            }
            let 元素 = 队列.pop_front().unwrap();
            let mut 合法 = false;
            let mut 新安排列表 = vec![];
            for 条件安排 in &self.决策空间.元素[元素] {
                if 决策.允许(条件安排) {
                    if 条件安排.安排 == 决策.元素[元素] {
                        合法 = true;
                        break;
                    }
                    新安排列表.push(条件安排.安排.clone());
                }
            }
            if !合法 {
                if 新安排列表.is_empty() {
                    panic!("没有合法的安排，传播失败");
                } else {
                    let 新安排 = 新安排列表.choose(&mut rng()).unwrap();
                    变化.push(元素);
                    决策.元素[元素] = 新安排.clone();
                }
            }
            for 下游元素 in self.元素图.get(&元素).unwrap_or(&vec![]) {
                if !队列.contains(下游元素) {
                    队列.push_back(下游元素.clone());
                }
            }
        }
    }

    pub fn 随机移动(&self, 决策: &mut 默认决策) -> Vec<元素> {
        let mut rng = rng();
        const MAX_TRIES: usize = 100;
        for _ in 0..MAX_TRIES {
            let 元素 = (0..决策.元素.len()).choose(&mut rng).unwrap();
            // 蓄水池抽样
            let mut 下一个安排 = None;
            let mut count = 0;
            for 条件安排 in &self.决策空间.元素[元素] {
                if &条件安排.安排 != &决策.元素[元素] && 决策.允许(条件安排) {
                    count += 1;
                    if random_range(0..count) == 0 {
                        下一个安排 = Some(&条件安排.安排);
                    }
                }
            }
            if let Some(下一个安排) = 下一个安排 {
                决策.元素[元素] = 下一个安排.clone();
                return vec![元素];
            }
        }
        vec![]
    }
}
