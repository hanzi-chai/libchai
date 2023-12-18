use crate::{
    data::{Character, Glyph},
    metaheuristics::simulated_annealing,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::{BTreeMap, HashMap};

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
    },
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
    pub analysis: Option<AnalysisConfig>,
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
    pub max_length: usize,
    pub auto_select_length: Option<usize>,
    pub sources: Option<BTreeMap<String, NodeConfig>>,
    pub conditions: Option<BTreeMap<String, EdgeConfig>>,
    pub rules: Option<Vec<WordRule>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelMetricWeights {
    pub length: usize,
    pub frequency: f64,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierMetricWeights {
    pub top: Option<usize>,
    pub duplication: Option<f64>,
    pub levels: Option<Vec<LevelMetricWeights>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingeringWeights {
    pub same_hand: Option<f64>,
    pub same_finger_large_jump: Option<f64>,
    pub same_finger_small_jump: Option<f64>,
    pub little_finger_inteference: Option<f64>,
    pub awkward_upside_down: Option<f64>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialMetricWeights {
    pub tiers: Option<Vec<TierMetricWeights>>,
    pub duplication: Option<f64>,
    pub key_equivalence: Option<f64>,
    pub pair_equivalence: Option<f64>,
    pub fingering: Option<FingeringWeights>,
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
    pub grouping: Option<Vec<Vec<AtomicConstraint>>>,
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
        report_after: Option<f64>,
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
