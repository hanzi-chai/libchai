use crate::config::{ObjectiveConfig, OptimizationConfig, SolverConfig, 配置};
use crate::contexts::default::默认上下文;
use crate::encoders::default::默认编码器;
use crate::interfaces::{默认输入, 消息, 界面};
use crate::objectives::default::默认目标函数;
use crate::objectives::目标函数;
use crate::operators::default::默认操作;
use console_error_panic_hook::set_once;
use js_sys::Function;
use serde::Serialize;
use serde_wasm_bindgen::{from_value, to_value, Serializer};
use wasm_bindgen::{prelude::*, JsError};

/// 通过图形界面来使用 libchai 的入口，实现了界面特征
#[wasm_bindgen]
pub struct Web {
    回调: Function,
    参数: 默认输入,
}

/// 用于在图形界面验证输入的配置是否正确
#[wasm_bindgen]
pub fn validate(js_config: JsValue) -> Result<JsValue, JsError> {
    set_once();
    let 配置: 配置 = from_value(js_config)?;
    let 序列化 = Serializer::json_compatible();
    Ok(配置.serialize(&序列化)?)
}

#[wasm_bindgen]
impl Web {
    pub fn new(回调: Function) -> Web {
        set_once();
        let 参数 = 默认输入::default();
        Self { 回调, 参数 }
    }

    pub fn sync(&mut self, 前端参数: JsValue) -> Result<(), JsError> {
        self.参数 = from_value(前端参数)?;
        Ok(())
    }

    pub fn encode_evaluate(&self, 前端目标函数配置: JsValue) -> Result<JsValue, JsError> {
        let 目标函数配置: ObjectiveConfig = from_value(前端目标函数配置)?;
        let mut 输入 = self.参数.clone();
        输入.配置.optimization = Some(OptimizationConfig {
            objective: 目标函数配置,
            metaheuristic: None,
        });
        let 上下文 = 默认上下文::新建(输入)?;
        let 编码器 = 默认编码器::新建(&上下文)?;
        let mut 目标函数 = 默认目标函数::新建(&上下文, 编码器)?;
        let (指标, _) = 目标函数.计算(&上下文.初始映射, &None);
        let 码表 = 上下文.生成码表(&目标函数.编码结果);
        Ok(to_value(&(码表, 指标))?)
    }

    pub fn optimize(&self) -> Result<(), JsError> {
        let 优化方法配置 = self.参数.配置.clone().optimization.unwrap().metaheuristic.unwrap();
        let 上下文 = 默认上下文::新建(self.参数.clone())?;
        let 编码器 = 默认编码器::新建(&上下文)?;
        let mut 目标函数 = 默认目标函数::新建(&上下文, 编码器)?;
        let mut 操作 = 默认操作::新建(&上下文)?;
        let SolverConfig::SimulatedAnnealing(退火) = 优化方法配置;
        退火.优化(&上下文.初始映射, &mut 目标函数, &mut 操作, &上下文, self);
        Ok(())
    }
}

impl 界面 for Web {
    fn 发送(&self, 消息: 消息) {
        let 序列化 = Serializer::json_compatible();
        let 前端消息 = 消息.serialize(&序列化).unwrap();
        self.回调.call1(&JsValue::null(), &前端消息).unwrap();
    }
}
