use crate::optimizers::解特征;
pub mod default;

pub trait 上下文 {
    type 解类型: 解特征;

    fn 序列化(&self, 解: &Self::解类型) -> String;
}
