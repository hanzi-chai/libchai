//! 算子接口，以及默认操作的实现（包含变异算子）
//!

use crate::optimizers::决策;

pub mod default;

pub trait 变异 {
    type 决策: 决策;
    /// 基于现有的一个决策通过随机扰动创建一个新的决策，返回变异的元素
    fn 变异(&mut self, 映射: &mut Self::决策) -> <Self::决策 as 决策>::变化;
}

pub trait 杂交 {
    type 决策: 决策;
    /// 基于现有的一个决策通过随机扰动创建一个新的决策
    fn 杂交(
        &mut self, 映射一: &Self::决策, 映射二: &Self::决策
    ) -> Self::决策;
}
