use super::*;
use crate::{
    config::{ObjectiveConfig, OptimizationConfig, SolverConfig}, contexts::default::默认上下文, encoders::default::默认编码器, interfaces::web::图形界面参数, objectives::{default::默认目标函数, metric::默认指标, 目标函数}, operators::default::默认操作, 码表项, 错误
};
use console_error_panic_hook::set_once;

/// 纯 Rust 的 Web API 接口，与 wasm_bindgen Web 结构一一对应
#[derive(Default)]
pub struct WebApi {
    参数: 图形界面参数,
    回调: Option<Box<dyn Fn(&消息) + Send + Sync>>,
}

impl WebApi {
    /// 创建新的 WebApi 实例，与 Web::new 对应
    pub fn new() -> Self {
        set_once();
        let 参数 = 图形界面参数::default();
        Self {
            参数, 回调: None
        }
    }

    /// 设置消息回调函数
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(&消息) + Send + Sync + 'static,
    {
        self.回调 = Some(Box::new(callback));
    }

    /// 同步前端参数，与 Web::sync 对应
    pub fn sync(&mut self, 前端参数: 图形界面参数) -> Result<(), 错误> {
        self.参数 = 前端参数;
        Ok(())
    }

    /// 编码评估，与 Web::encode_evaluate 对应
    pub fn encode_evaluate(
        &self,
        目标函数配置: ObjectiveConfig,
    ) -> Result<(Vec<码表项>, 默认指标), 错误> {
        let 图形界面参数 {
            mut 配置,
            原始键位分布信息,
            原始当量信息,
            词列表,
        } = self.参数.clone();

        配置.optimization = Some(OptimizationConfig {
            objective: 目标函数配置,
            constraints: None,
            metaheuristic: None,
        });

        let 上下文 = 默认上下文::新建(配置, 词列表, 原始键位分布信息, 原始当量信息)?;
        let 编码器 = 默认编码器::新建(&上下文)?;
        let mut 目标函数 = 默认目标函数::新建(&上下文, 编码器)?;
        let (指标, _) = 目标函数.计算(&上下文.初始映射, &None);
        let 码表 = 上下文.生成码表(&目标函数.编码结果);

        Ok((码表, 指标))
    }

    /// 优化，与 Web::optimize 对应  
    pub fn optimize(&self) -> Result<(), 错误> {
        let 图形界面参数 {
            配置,
            原始键位分布信息,
            原始当量信息,
            词列表,
        } = self.参数.clone();

        let 优化方法配置 = 配置.clone().optimization.unwrap().metaheuristic.unwrap();
        let 上下文 = 默认上下文::新建(配置, 词列表, 原始键位分布信息, 原始当量信息)?;
        let 编码器 = 默认编码器::新建(&上下文)?;
        let mut 目标函数 = 默认目标函数::新建(&上下文, 编码器)?;
        let mut 操作 = 默认操作::新建(&上下文)?;
        let SolverConfig::SimulatedAnnealing(退火) = 优化方法配置;
        退火.优化(&上下文.初始映射, &mut 目标函数, &mut 操作, &上下文, self);
        Ok(())
    }
}

impl 界面 for WebApi {
    fn 发送(&self, 消息: 消息) {
        if let Some(ref callback) = self.回调 {
            callback(&消息);
        }
    }
}
