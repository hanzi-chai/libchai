//! 优化问题的整体定义。
//!
//! 目前只定义了最基础的元素布局问题，以后可能会定义更复杂的问题，如元素布局 + 元素选取等等。
//!
pub mod default;
pub mod snow2;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::objectives::metric::Metric;
use crate::representation::{Element, KeyMap};
use crate::Interface;

// 未来可能会有更加通用的解定义
pub type Solution = KeyMap;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutateConfig {
    pub random_move: f64,
    pub random_swap: f64,
    pub random_full_key_swap: f64,
}

pub trait Problem {
    /// 给出一个初始解
    fn initialize(&mut self) -> Solution;

    /// 对一个解来打分
    /// Metric 存放了各种指标；后面的 f64 是对各项指标加权求和得到的标量值
    fn rank(&mut self, candidate: &Solution, diff: &Option<Vec<Element>>) -> (Metric, f64);

    /// 报告一个比之前的解都要更好的解，可以选择
    fn update(&self, candidate: &Solution, rank: &(Metric, f64), save: bool, interface: &dyn Interface);

    /// 基于现有的一个解通过随机扰动创建一个新的解
    fn mutate(&self, candidate: &mut Solution, config: &MutateConfig) -> Vec<Element>;
}
