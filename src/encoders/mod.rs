//! 编码器接口，以及默认编码器的实现

use crate::{optimizers::决策, 编码信息};

pub mod default;

pub trait 编码器 {
    type 决策: 决策;
    fn 编码(
        &mut self,
        决策: &Self::决策,
        变化: &Option<<Self::决策 as 决策>::变化>,
        输出: &mut [编码信息],
    );
}
