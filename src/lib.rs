mod config;
mod constraints;
mod data;
mod encoder;
mod interface;
mod metaheuristics;
mod objectives;
mod problem;
mod representation;
use crate::constraints::Constraints;
use crate::problem::ElementPlacementProblem;
use crate::{
    config::Config,
    encoder::Encoder,
    objectives::Objective,
    representation::{Assets, Representation},
};
use interface::Interface;
use js_sys::Function;
use representation::{WordList, RawSequenceMap};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use serde_with::skip_serializing_none;

#[derive(Deserialize)]
struct Input {
    config: Config,
    characters: RawSequenceMap,
    words: WordList,
    assets: Assets,
}

fn jsvalue<T: Serialize>(v: T) -> JsValue {
    serde_wasm_bindgen::to_value(&v).unwrap()
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
        save: bool
    },
}

impl WebInterface {
    pub fn new(post_message: Function) -> Self {
        Self { post_message }
    }

    fn post(&self, message: Message) {
        self.post_message
            .call1(&JsValue::null(), &jsvalue(message))
            .unwrap();
    }
}

impl Interface for WebInterface {
    fn prepare_output(&self) {}

    fn init_autosolve(&self) {}

    fn report_elapsed(&self, _: u128) {}

    fn report_trial_t_max(&self, t_max: f64, _: f64) {
        self.post(Message::Parameters {
            t_max: Some(t_max),
            t_min: None,
            steps: None,
        });
    }

    fn report_t_max(&self, t_max: f64) {
        self.post(Message::Parameters {
            t_max: Some(t_max),
            t_min: None,
            steps: None,
        });
    }

    fn report_trial_t_min(&self, t_min: f64, _: f64) {
        self.post(Message::Parameters {
            t_max: None,
            t_min: Some(t_min),
            steps: None,
        });
    }

    fn report_t_min(&self, t_min: f64) {
        self.post(Message::Parameters {
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
        self.post(message);
    }

    fn report_schedule(&self, steps: usize, temperature: f64, metric: String) {
        let message = Message::Progress {
            steps,
            temperature,
            metric,
        };
        self.post(message);
    }

    fn report_solution(&self, config: String, metric: String, save: bool) {
        self.post(Message::BetterSolution { metric, config, save });
    }
}

fn prepare(js_input: JsValue) -> (Representation, Objective) {
    let Input {
        config,
        characters,
        words,
        assets,
    } = serde_wasm_bindgen::from_value(js_input).unwrap();
    let representation = Representation::new(config);
    let encoder = Encoder::new(&representation, characters, words, &assets);
    let objective = Objective::new(&representation, encoder, assets);
    (representation, objective)
}

#[wasm_bindgen]
pub fn evaluate(js_input: JsValue) -> JsValue {
    console_error_panic_hook::set_once();
    let (representation, objective) = prepare(js_input);
    let mut buffer = objective.init_buffer();
    let (metric, _) = objective.evaluate(&representation.initial, &mut buffer);
    let metric = format!("{}", metric);
    let codes = objective.export_codes(&mut buffer);
    let human_codes = representation.recover_codes(codes);
    serde_wasm_bindgen::to_value(&(metric, human_codes)).unwrap()
}

#[wasm_bindgen]
pub fn optimize(js_input: JsValue, post_message: Function) {
    console_error_panic_hook::set_once();
    let (representation, objective) = prepare(js_input);
    let buffer = objective.init_buffer();
    let constraints = Constraints::new(&representation);
    let mut problem = ElementPlacementProblem::new(representation, constraints, objective, buffer);
    let web_interface = WebInterface::new(post_message);
    problem.solve(&web_interface);
}
