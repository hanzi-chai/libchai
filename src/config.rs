//! 递归定义 YAML 配置文件中的所有字段，以及它们和一个 Rust 结构体之间的序列化、反序列化操作应该如何执行。
//! 
//! 这部分内容太多，就不一一注释了。后期会写一个「`config.yaml` 详解」来统一解释各种配置文件的字段。
//! 

use crate::{
    data::Character,
    metaheuristics::simulated_annealing,
};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::{BTreeMap, HashMap};

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    pub repertoire: Option<BTreeMap<String, Character>>,
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
    pub classifier: Option<BTreeMap<String, usize>>,
    pub degenerator: Option<DegeneratorConfig>,
    pub selector: Option<Vec<String>>,
    pub customize: Option<BTreeMap<String, Vec<String>>>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MappedKey {
    Ascii(char),
    Reference { element: String, index: usize }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Mapped {
    Basic(String),
    Advanced(Vec<MappedKey>)
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormConfig {
    pub alphabet: String,
    pub mapping_type: Option<usize>,
    pub mapping: HashMap<String, Mapped>,
    pub grouping: Option<HashMap<String, String>>,
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
pub struct ShortCodeConfig {
    pub prefix: usize,
    pub count: Option<usize>,
    pub select_keys: Option<Vec<char>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderConfig {
    // 全局
    pub max_length: usize,
    pub select_keys: Option<Vec<char>>,
    pub auto_select_length: Option<usize>,
    pub auto_select_pattern: Option<String>,
    // 单字全码
    pub sources: Option<BTreeMap<String, NodeConfig>>,
    pub conditions: Option<BTreeMap<String, EdgeConfig>>,
    // 单字简码
    pub short_code_schemes: Option<Vec<ShortCodeConfig>>,
    // 词语全码
    pub rules: Option<Vec<WordRule>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelWeights {
    pub length: usize,
    pub frequency: f64,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierWeights {
    pub top: Option<usize>,
    pub duplication: Option<f64>,
    pub levels: Option<Vec<LevelWeights>>,
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
pub struct PartialWeights {
    pub tiers: Option<Vec<TierWeights>>,
    pub duplication: Option<f64>,
    pub key_distribution: Option<f64>,
    //杏码的「用指当量」。
    pub new_key_equivalence: Option<f64>,
    //杏码的「用指当量」（改），假定连续输入时预测上一键从而计算组合当量（慢）。
    pub new_key_equivalence_modified: Option<f64>,
    pub pair_equivalence: Option<f64>,
    //杏码的「速度（组合）当量」。
    pub new_pair_equivalence: Option<f64>,
    pub fingering: Option<FingeringWeights>,
    pub levels: Option<Vec<LevelWeights>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveConfig {
    pub characters_full: Option<PartialWeights>,
    pub words_full: Option<PartialWeights>,
    pub characters_short: Option<PartialWeights>,
    pub words_short: Option<PartialWeights>,
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
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub random_move: f64,
    pub random_swap: f64,
    pub random_full_key_swap: f64,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    pub algorithm: String,
    pub runtime: Option<u64>,
    pub parameters: Option<simulated_annealing::Parameters>,
    pub report_after: Option<f64>,
    pub search_method: Option<SearchConfig>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub objective: ObjectiveConfig,
    pub constraints: Option<ConstraintsConfig>,
    pub metaheuristic: SolverConfig,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Algebra {
    Xform { from: String, to: String },
    Xlit { from: String, to: String }
}

type AlgebraConfig = BTreeMap<String, Vec<Algebra>>;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: Option<String>,
    #[serialize_always] // JavaScript null
    pub source: Option<String>,
    pub info: Option<Info>,
    pub data: Option<DataConfig>,
    pub analysis: Option<AnalysisConfig>,
    pub algebra: Option<AlgebraConfig>,
    pub form: FormConfig,
    pub encoder: EncoderConfig,
    pub optimization: OptimizationConfig,
}
