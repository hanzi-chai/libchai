pub mod cli;
pub mod config;
pub mod constraints;
pub mod data;
pub mod encoder;
pub mod error;
pub mod interface;
pub mod metaheuristics;
pub mod objectives;
pub mod problem;
pub mod representation;

use crate::constraints::Constraints;
use crate::problem::ElementPlacementProblem;
use crate::{
    config::Config,
    encoder::Encoder,
    objectives::Objective,
    representation::{Assets, Representation},
};
use config::{ObjectiveConfig, OptimizationConfig};
use console_error_panic_hook::set_once;
use interface::Interface;
use js_sys::Function;
use problem::solve;
use representation::{AssembleList, Buffer};
use serde::Serialize;
use serde_wasm_bindgen::{from_value, to_value};
use serde_with::skip_serializing_none;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[skip_serializing_none]
enum Message {
    Parameters {
        t_max: Option<f64>,
        t_min: Option<f64>,
        steps: Option<usize>,
    },
    Progress {
        steps: usize,
        temperature: f64,
        metric: String,
    },
    BetterSolution {
        metric: String,
        config: String,
        save: bool,
    },
}

#[wasm_bindgen]
pub struct WebInterface {
    post_message: Function,
    config: Config,
    info: AssembleList,
    assets: Assets,
}

#[wasm_bindgen]
pub fn validate(js_config: JsValue) -> Result<JsValue, JsError> {
    set_once();
    let config: Config = from_value(js_config)?;
    let config_str = serde_yaml::to_string(&config).unwrap();
    Ok(to_value(&config_str)?)
}

#[wasm_bindgen]
impl WebInterface {
    pub fn new(
        post_message: Function,
        js_config: JsValue,
        js_info: JsValue,
        js_assets: JsValue,
    ) -> Result<WebInterface, JsError> {
        set_once();
        let config: Config = from_value(js_config)?;
        let info: AssembleList = from_value(js_info)?;
        let assets: Assets = from_value(js_assets)?;
        Ok(Self {
            post_message,
            config,
            info,
            assets,
        })
    }

    pub fn update_config(&mut self, js_config: JsValue) -> Result<(), JsError> {
        self.config = from_value(js_config)?;
        Ok(())
    }

    pub fn update_info(&mut self, js_info: JsValue) -> Result<(), JsError> {
        self.info = from_value(js_info)?;
        Ok(())
    }

    pub fn update_assets(&mut self, js_assets: JsValue) -> Result<(), JsError> {
        self.assets = from_value(js_assets)?;
        Ok(())
    }

    pub fn encode_evaluate(&self, js_objective: JsValue) -> Result<JsValue, JsError> {
        let objective: ObjectiveConfig = from_value(js_objective)?;
        let mut config = self.config.clone();
        config.optimization = Some(OptimizationConfig {
            objective,
            constraints: None,
            metaheuristic: None,
        });
        let representation = Representation::new(config)?;
        let encoder = Encoder::new(&representation, self.info.clone(), &self.assets)?;
        let codes = encoder.encode(&representation.initial, &representation);
        let mut buffer = Buffer::new(&encoder);
        let objective = Objective::new(&representation, encoder, self.assets.clone())?;
        let (metric, _) = objective.evaluate(&representation.initial, &mut buffer)?;
        Ok(to_value(&(codes, metric))?)
    }

    pub fn optimize(&self) -> Result<(), JsError> {
        let representation = Representation::new(self.config.clone())?;
        let encoder = Encoder::new(&representation, self.info.clone(), &self.assets)?;
        let mut buffer = Buffer::new(&encoder);
        let objective = Objective::new(&representation, encoder, self.assets.clone())?;
        let constraints = Constraints::new(&representation)?;
        let _ = objective.evaluate(&representation.initial, &mut buffer)?;
        let mut problem =
            ElementPlacementProblem::new(representation, constraints, objective, buffer)?;
        let solver = self.config.optimization.as_ref().unwrap().metaheuristic.as_ref().unwrap();
        solve(&mut problem, solver, self);
        Ok(())
    }

    fn post(&self, message: Message) -> Result<(), JsValue> {
        let js_message = to_value(&message)?;
        let _ = self.post_message.call1(&JsValue::null(), &js_message)?;
        Ok(())
    }
}

impl Interface for WebInterface {
    fn prepare_output(&self) {}

    fn init_autosolve(&self) {}

    fn report_elapsed(&self, _: u128) {}

    fn report_trial_t_max(&self, t_max: f64, _: f64) {
        let _ = self.post(Message::Parameters {
            t_max: Some(t_max),
            t_min: None,
            steps: None,
        });
    }

    fn report_t_max(&self, t_max: f64) {
        let _ = self.post(Message::Parameters {
            t_max: Some(t_max),
            t_min: None,
            steps: None,
        });
    }

    fn report_trial_t_min(&self, t_min: f64, _: f64) {
        let _ = self.post(Message::Parameters {
            t_max: None,
            t_min: Some(t_min),
            steps: None,
        });
    }

    fn report_t_min(&self, t_min: f64) {
        let _ = self.post(Message::Parameters {
            t_max: None,
            t_min: Some(t_min),
            steps: None,
        });
    }

    fn report_parameters(&self, t_max: f64, t_min: f64, steps: usize) {
        let message = Message::Parameters {
            t_max: Some(t_max),
            t_min: Some(t_min),
            steps: Some(steps),
        };
        let _ = self.post(message);
    }

    fn report_schedule(&self, steps: usize, temperature: f64, metric: String) {
        let message = Message::Progress {
            steps,
            temperature,
            metric,
        };
        let _ = self.post(message);
    }

    fn report_solution(&self, config: Config, metric: String, save: bool) {
        let _ = self.post(Message::BetterSolution {
            metric,
            config: serde_yaml::to_string(&config).unwrap(),
            save,
        });
    }
}
