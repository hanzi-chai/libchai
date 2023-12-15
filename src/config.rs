use crate::{
    data::{Character, Glyph},
    encoder::{Elements, RawElements},
    metaheuristics::simulated_annealing,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::{BTreeMap, HashMap};

pub type KeyMap = Vec<char>;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    pub form: Option<BTreeMap<String, Glyph>>,
    pub repertoire: Option<BTreeMap<String, Character>>,
    pub classifier: Option<BTreeMap<String, usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WordRule {
    EqualRule {
        length_equal: usize,
        formula: String,
    },
    RangeRule {
        length_in_range: (usize, usize),
        formula: String,
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegeneratorConfig {
    pub feature: Option<BTreeMap<String, String>>,
    pub no_cross: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub degenerator: Option<DegeneratorConfig>,
    pub selector: Option<Vec<String>>,
    pub customize: Option<BTreeMap<String, Vec<String>>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormConfig {
    pub alphabet: String,
    pub mapping_type: Option<usize>,
    pub mapping: HashMap<String, String>,
    pub grouping: HashMap<String, String>,
    pub analysis: Option<AnalysisConfig>
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct CodableObjectConfig {
    pub r#type: String,
    pub subtype: Option<String>,
    pub key: Option<String>,
    pub rootIndex: Option<i64>,
    pub strokeIndex: Option<i64>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    #[serialize_always] // JavaScript null
    pub object: Option<CodableObjectConfig>,
    pub index: Option<usize>,
    #[serialize_always] // JavaScript null
    pub next: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfig {
    pub object: CodableObjectConfig,
    pub operator: String,
    pub value: Option<String>,
    #[serialize_always] // JavaScript null
    pub positive: Option<String>,
    #[serialize_always] // JavaScript null
    pub negative: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderConfig {
    pub max_length: Option<usize>,
    pub auto_select_length: Option<usize>,
    pub sources: Option<BTreeMap<String, NodeConfig>>,
    pub conditions: Option<BTreeMap<String, EdgeConfig>>,
    pub rules: Option<Vec<WordRule>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelMetricWeights {
    pub length: usize,
    pub frequency: f64
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredMetricWeights {
    pub top: Option<usize>,
    pub duplication: Option<f64>,
    pub levels: Option<Vec<LevelMetricWeights>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialMetricWeights {
    pub tiered: Option<Vec<TieredMetricWeights>>,
    pub duplication: Option<f64>,
    pub equivalence: Option<f64>,
    pub levels: Option<Vec<LevelMetricWeights>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveConfig {
    pub characters: Option<PartialMetricWeights>,
    pub words: Option<PartialMetricWeights>,
    pub characters_reduced: Option<PartialMetricWeights>,
    pub words_reduced: Option<PartialMetricWeights>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicConstraint {
    pub element: Option<String>,
    pub index: Option<usize>,
    pub keys: Option<Vec<char>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintsConfig {
    pub elements: Option<Vec<AtomicConstraint>>,
    pub indices: Option<Vec<AtomicConstraint>>,
    pub element_indices: Option<Vec<AtomicConstraint>>,
    pub grouping: Option<Vec<Vec<AtomicConstraint>>>
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "algorithm")]
pub enum MetaheuristicConfig {
    HillClimbing {
        runtime: u64,
    },
    SimulatedAnnealing {
        runtime: Option<u64>,
        parameters: Option<simulated_annealing::Parameters>,
    },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub objective: ObjectiveConfig,
    pub constraints: Option<ConstraintsConfig>,
    pub metaheuristic: MetaheuristicConfig,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: Option<String>,
    #[serialize_always] // JavaScript null
    pub source: Option<String>,
    pub info: Option<BTreeMap<String, String>>,
    pub data: Option<DataConfig>,
    pub form: FormConfig,
    pub encoder: EncoderConfig,
    pub optimization: OptimizationConfig,
}

pub struct Cache {
    pub initial: KeyMap,
    pub forward_converter: HashMap<String, usize>,
    pub reverse_converter: Vec<String>,
}

impl Cache {
    pub fn new(config: &Config) -> Self {
        let (initial, forward_converter, reverse_converter) = Self::transform_keymap(&config);
        Self {
            initial,
            forward_converter,
            reverse_converter,
        }
    }

    pub fn transform_keymap(config: &Config) -> (KeyMap, HashMap<String, usize>, Vec<String>) {
        let mut keymap: KeyMap = Vec::new();
        let mut forward_converter: HashMap<String, usize> = HashMap::new();
        let mut reverse_converter: Vec<String> = Vec::new();
        for (element, mapped) in &config.form.mapping {
            let chars: Vec<char> = mapped.chars().collect();
            if chars.len() == 1 {
                forward_converter.insert(element.clone(), keymap.len());
                reverse_converter.push(element.clone());
                keymap.push(chars[0]);
            } else {
                for (index, key) in chars.iter().enumerate() {
                    let name = format!("{}.{}", element.to_string(), index);
                    forward_converter.insert(name.clone(), keymap.len());
                    reverse_converter.push(name.clone());
                    keymap.push(*key);
                }
            }
        }
        (keymap, forward_converter, reverse_converter)
    }

    pub fn transform_elements(&self, elements: &RawElements) -> Elements {
        let mut new_elements: Elements = HashMap::new();
        for (char, elems) in elements {
            let mut converted_elems: Vec<usize> = Vec::new();
            for element in elems {
                if let Some(number) = self.forward_converter.get(element) {
                    converted_elems.push(*number);
                } else {
                    panic!("不合法的码元：{}", element);
                }
            }
            new_elements.insert(*char, converted_elems);
        }
        new_elements
    }

    pub fn update_config(&self, config: &Config, candidate: &KeyMap) -> Config {
        let mut new_config = config.clone();
        for (element, mapped) in &config.form.mapping {
            if mapped.len() == 1 {
                let number = *self.forward_converter.get(element).unwrap();
                let current_mapped = candidate[number];
                new_config
                    .form
                    .mapping
                    .insert(element.to_string(), current_mapped.to_string());
            } else {
                let mut all_codes = String::new();
                for index in 0..mapped.len() {
                    let name = format!("{}.{}", element.to_string(), index);
                    let number = *self.forward_converter.get(&name).unwrap();
                    let current_mapped = &candidate[number];
                    all_codes.push(*current_mapped);
                }
                new_config
                    .form
                    .mapping
                    .insert(element.to_string(), all_codes);
            }
        }
        new_config
    }
}
