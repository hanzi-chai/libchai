//! 优化方法接口，以及若干优化方法的实现
//!

use crate::objectives::目标函数;
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

pub trait 决策: Clone {
    type 变化: Clone;

    // 返回 ba^{-1}
    fn 除法(旧变化: &Self::变化, 新变化: &Self::变化) -> Self::变化;
}

pub struct 优化结果<O: 目标函数> {
    pub 映射: O::决策,
    pub 指标: O::目标值,
    pub 分数: f64,
}
