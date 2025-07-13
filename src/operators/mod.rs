//! 算子接口，以及默认操作的实现（包含变异算子）
//!

use crate::optimizers::解特征;

pub mod default;

pub trait 变异 {
    type 解类型: 解特征;
    /// 基于现有的一个解通过随机扰动创建一个新的解，返回变异的元素
    fn 变异(&mut self, 映射: &mut Self::解类型) -> <Self::解类型 as 解特征>::变化;
}

pub trait 杂交 {
    type 解类型: 解特征;
    /// 基于现有的一个解通过随机扰动创建一个新的解
    fn 杂交(
        &mut self, 映射一: &Self::解类型, 映射二: &Self::解类型
    ) -> Self::解类型;
}
