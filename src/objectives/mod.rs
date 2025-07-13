//! 目标函数接口，以及默认目标函数的实现
//!
//!

use crate::optimizers::解特征;
use serde::Serialize;
use std::fmt::Display;
pub mod cache;
pub mod default;
pub mod metric;

pub trait 目标函数 {
    type 目标值: Display + Clone + Serialize;
    type 解类型: 解特征;
    fn 计算(
        &mut self,
        解: &Self::解类型,
        解变化: &Option<<Self::解类型 as 解特征>::变化>,
    ) -> (Self::目标值, f64);

    fn 接受新解(&mut self);
}
