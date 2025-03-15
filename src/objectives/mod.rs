//! 目标函数接口，以及默认目标函数的实现
//!
//!

use crate::data::{元素, 元素映射};
use crate::encoders::编码器;
use metric::Metric;

pub mod cache;
pub mod default;
pub mod metric;

pub trait 目标函数 {
    fn 计算<E: 编码器>(
        &mut self,
        encoder: &mut E,
        candidate: &元素映射,
        moved_elements: &Option<Vec<元素>>,
    ) -> (Metric, f64);
}
