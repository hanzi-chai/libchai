//! 目标函数接口，以及默认目标函数的实现
//!
//!

use crate::data::编码信息;
use metric::Metric;

pub mod cache;
pub mod default;
pub mod metric;

pub trait 目标函数 {
    fn 计算(&mut self, 编码结果: &mut [编码信息]) -> (Metric, f64);
}
