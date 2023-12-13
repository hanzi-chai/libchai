use std::{fs, collections::{HashMap, BTreeMap}};
use serde::{Serialize, Deserialize};
use crate::{encoder::{Elements, RawElements}, data::{Glyph, Character}, metaheuristics::simulated_annealing};

pub type KeyMap = Vec<char>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    pub form: BTreeMap<String, Glyph>,
    pub repertoire: BTreeMap<String, Character>,
    pub classifier: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordRule {
    pub formula: String,
    pub length_equal: Option<usize>,
    pub length_in_range: Option<Vec<usize>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegeneratorConfig {
    pub feature: BTreeMap<String, String>,
    pub nocross: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub degenerator: DegeneratorConfig,
    pub selector: Vec<String>,
    pub customize: BTreeMap<String, Vec<String>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormConfig {
    pub maxcodelen: usize,
    pub alphabet: String,
    pub analysis: AnalysisConfig,
    pub grouping: HashMap<String, String>,
    pub mapping: HashMap<String, String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct CodableObjectConfig {
    pub r#type: String,
    pub subtype: Option<String>,
    pub rootIndex: Option<i64>,
    pub strokeIndex: Option<i64>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfig {
    pub object: CodableObjectConfig,
    pub operator: String,
    pub value: Option<String>,
    pub positive: Option<String>,
    pub negative: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub object: Option<CodableObjectConfig>,
    pub next: Option<String>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderConfig {
    pub sources: BTreeMap<String, NodeConfig>,
    pub conditions: BTreeMap<String, EdgeConfig>,
    pub maxlength: usize,
    pub auto_select_length: usize,
    pub rules: Vec<WordRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredMetricWeights {
    pub top: Option<usize>,
    pub duplication: Option<f64>,
    pub levels: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialMetricWeights {
    pub tiered: Option<Vec<TieredMetricWeights>>,
    pub duplication: Option<f64>,
    pub equivalence: Option<f64>,
    pub levels: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveConfig {
    pub characters: Option<PartialMetricWeights>,
    pub words: Option<PartialMetricWeights>,
    pub characters_reduced: Option<PartialMetricWeights>,
    pub words_reduced: Option<PartialMetricWeights>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicConstraint {
    pub element: Option<String>,
    pub index: Option<usize>,
    pub keys: Option<Vec<char>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintsConfig {
    pub elements: Vec<AtomicConstraint>,
    pub indices: Vec<AtomicConstraint>,
    pub element_indices: Vec<AtomicConstraint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "algorithm")]
pub enum MetaheuristicConfig {
    HillClimbing {
        runtime: Option<i64>
    },
    SimulatedAnnealing {
        runtime: Option<i64>,
        parameters: Option<simulated_annealing::Parameters>
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub objective: ObjectiveConfig,
    pub constraints: ConstraintsConfig,
    pub metaheuristic: MetaheuristicConfig
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: String,
    pub source: Option<String>,
    pub info: BTreeMap<String, String>,
    pub data: DataConfig,
    pub form: FormConfig,
    pub encoder: EncoderConfig,
    pub optimization: OptimizationConfig,
}

impl Config {
    pub fn new(name: &String) -> Self {
        let content = fs::read_to_string(name).expect("Should have been able to read the file");
        let config: Config = serde_yaml::from_str(&content).unwrap();
        config
    }
}

pub struct Cache {
    pub initial: KeyMap,
    pub forward_converter: HashMap<String, usize>,
    pub reverse_converter: Vec<String>,
}

impl Cache {
    pub fn new(config: &Config) -> Self {
        let (initial, forward_converter, reverse_converter) = Self::transform_keymap(&config);
        Self { initial, forward_converter, reverse_converter }
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
                new_config.form.mapping.insert(element.to_string(), current_mapped.to_string());
            } else {
                let mut all_codes = String::new();
                for index in 0..mapped.len() {
                    let name = format!("{}.{}", element.to_string(), index);
                    let number = *self.forward_converter.get(&name).unwrap();
                    let current_mapped = &candidate[number];
                    all_codes.push(*current_mapped);
                }
                new_config.form.mapping.insert(element.to_string(), all_codes);
            }
        }
        new_config
    }
}
