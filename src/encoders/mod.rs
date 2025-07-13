//! 编码器接口，以及默认编码器的实现

use crate::{optimizers::解特征, 编码信息};

pub mod default;

pub trait 编码器 {
    type 解类型: 解特征;
    fn 编码(
        &mut self,
        解: &Self::解类型,
        变化: &Option<<Self::解类型 as 解特征>::变化>,
        输出: &mut [编码信息],
    );
}
