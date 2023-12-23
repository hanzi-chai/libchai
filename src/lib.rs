mod config;
mod constraints;
mod data;
mod encoder;
mod metaheuristics;
mod metric;
mod objective;
mod problem;
mod representation;
use std::collections::HashMap;

use crate::{representation::{Representation, Assets}, objective::Objective, config::Config, encoder::Encoder};
use representation::Buffer;
use wasm_bindgen::prelude::*;


#[wasm_bindgen]
pub struct Interface {
    representation: Representation,
    objective: Objective,
    buffer: Buffer,
}

#[wasm_bindgen]
impl Interface {
    pub fn new(
        js_config: JsValue,
        js_characters: JsValue,
        js_words: JsValue,
        js_assets: JsValue,
    ) -> Interface {
        let config: Config = serde_wasm_bindgen::from_value(js_config).unwrap();
        let representation = Representation::new(config);
        let characters: HashMap<char, String> = serde_wasm_bindgen::from_value(js_characters).unwrap();
        let words: Vec<String> = serde_wasm_bindgen::from_value(js_words).unwrap();
        let buffer = representation.init_buffer(characters.len(), words.len());
        let assets: Assets = serde_wasm_bindgen::from_value(js_assets).unwrap();
        let encoder = Encoder::new(&representation, characters, words, &assets);
        let objective = Objective::new(&representation, encoder, assets);
        Interface { representation, objective, buffer }
    }
    
    pub fn evaluate(&mut self) -> String {
        let (metric, _) = self.objective.evaluate(&self.representation.initial, &mut self.buffer);
        format!("{}", metric)
    }
}
