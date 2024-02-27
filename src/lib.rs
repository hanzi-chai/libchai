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
pub mod cli;

use crate::constraints::Constraints;
use crate::problem::ElementPlacementProblem;
use crate::{
    config::Config,
    encoder::Encoder,
    objectives::Objective,
    representation::{Assets, Representation},
};
use console_error_panic_hook::set_once;
use interface::Interface;
use js_sys::Function;
use representation::{Buffer, RawSequenceMap, WordList};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use serde_with::skip_serializing_none;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
struct Input {
    config: Config,
    characters: RawSequenceMap,
    words: WordList,
    assets: Assets,
}

#[wasm_bindgen]
pub struct WebInterface {
    post_message: Function,
}

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

impl WebInterface {
    pub fn new(post_message: Function) -> Self {
        Self { post_message }
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

fn prepare(js_input: JsValue) -> Result<(Representation, Encoder, Assets), JsError> {
    let Input {
        config,
        characters,
        words,
        assets,
    } = from_value(js_input)?;
    let representation = Representation::new(config)?;
    let encoder = Encoder::new(&representation, characters, words, &assets)?;
    Ok((representation, encoder, assets))
}

#[wasm_bindgen]
pub fn validate(js_config: JsValue) -> Result<(), JsError> {
    set_once();
    let _: Config = from_value(js_config)?;
    Ok(())
}

#[wasm_bindgen]
pub fn encode(js_input: JsValue) -> Result<JsValue, JsError> {
    set_once();
    let (representation, encoder, _) = prepare(js_input)?;
    let codes = encoder.encode(&representation.initial, &representation);
    Ok(to_value(&codes)?)
}

#[wasm_bindgen]
pub fn evaluate(js_input: JsValue) -> Result<JsValue, JsError> {
    set_once();
    let (representation, encoder, assets) = prepare(js_input)?;
    let mut buffer = Buffer::new(&encoder);
    let objective = Objective::new(&representation, encoder, assets)?;
    let (metric, _) = objective.evaluate(&representation.initial, &mut buffer)?;
    let metric = format!("{}", metric);
    Ok(to_value(&metric)?)
}

#[wasm_bindgen]
pub fn optimize(js_input: JsValue, post_message: Function) -> Result<(), JsError> {
    set_once();
    let (representation, encoder, assets) = prepare(js_input)?;
    let mut buffer = Buffer::new(&encoder);
    let objective = Objective::new(&representation, encoder, assets)?;
    let constraints = Constraints::new(&representation)?;
    let _ = objective.evaluate(&representation.initial, &mut buffer)?;
    let mut problem = ElementPlacementProblem::new(representation, constraints, objective, buffer)?;
    let web_interface = WebInterface::new(post_message);
    problem.solve(&web_interface);
    Ok(())
}
