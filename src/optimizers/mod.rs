//! 优化方法接口，以及若干优化方法的实现
//!

use crate::{
    data::{元素, 元素映射, 数据},
    encoders::编码器,
    objectives::{metric::Metric, 目标函数},
    界面,
};
pub mod genetic;
pub mod simulated_annealing;

#[derive(Debug)]
pub struct 计时器 {
    pub encode_reset: u128,
    pub encode_init: u128,
    pub encode_assembly: u128,
    pub encode_short: u128,
    pub encode_duplicate: u128,
    pub objective_accumulate: u128,
    pub objective_accept: u128,
}

pub static mut 全局计时器: 计时器 = 计时器 {
    encode_reset: 0,
    encode_init: 0,
    encode_assembly: 0,
    encode_short: 0,
    encode_duplicate: 0,
    objective_accumulate: 0,
    objective_accept: 0,
};

pub struct 优化结果 {
    pub 映射: 元素映射,
    pub 指标: Metric,
    pub 分数: f64,
}

pub struct 优化问题<E: 编码器, O: 目标函数, F> {
    pub 数据: 数据,
    pub 目标函数: O,
    pub 编码器: E,
    pub 操作: F,
}

impl<E: 编码器, O: 目标函数, F> 优化问题<E, O, F> {
    pub fn 新建(数据: 数据, 编码器: E, 目标函数: O, 操作: F) -> Self {
        Self {
            数据,
            目标函数,
            编码器,
            操作,
        }
    }

    pub fn 计算(&mut self, 映射: &元素映射, 变化: &Option<Vec<元素>>) -> (Metric, f64) {
        let 编码结果 = self.编码器.编码(映射, 变化);
        self.目标函数.计算(编码结果)
    }
}

pub trait 优化方法<F> {
    fn 优化<E: 编码器, O: 目标函数>(
        &self,
        问题: &mut 优化问题<E, O, F>,
        界面: &dyn 界面,
    ) -> 优化结果;
}
