//! 目标函数接口，以及默认目标函数的实现
//!
//!

use std::fmt::Display;

use serde::Serialize;

use crate::data::{元素映射, 编码信息};

pub mod cache;
pub mod default;
pub mod metric;

pub trait 目标函数 {
    type 目标值: Display + Clone + Serialize;
    fn 计算(
        &mut self, 编码结果: &mut [编码信息], 映射: &元素映射
    ) -> (Self::目标值, f64);
}
