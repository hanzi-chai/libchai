//! 目标函数接口，以及默认目标函数的实现
//!
//!

use crate::optimizers::决策;
use serde::Serialize;
use std::fmt::Display;
pub mod cache;
pub mod default;
pub mod metric;

pub trait 目标函数 {
    type 目标值: Display + Clone + Serialize;
    type 决策: 决策;
    fn 计算(
        &mut self,
        决策: &Self::决策,
        决策变化: &Option<<Self::决策 as 决策>::变化>,
    ) -> (Self::目标值, f64);
}
