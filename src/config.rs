//! 配置文件的定义
//!
//! 这部分内容太多，就不一一注释了。后期会写一个「`config.yaml` 详解」来统一解释各种配置文件的字段。
//!

use crate::optimizers::simulated_annealing::退火方法;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;

// config.info begin
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
}
// config.info end

// config.data begin
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    pub character_set: Option<String>,
    pub repertoire: Option<PrimitiveRepertoire>,
    pub glyph_customization: Option<HashMap<String, Glyph>>,
    pub reading_customization: Option<HashMap<String, Vec<Reading>>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
#[allow(non_snake_case)]
pub enum Draw {
    H { parameterList: [i8; 1] },
    V { parameterList: [i8; 1] },
    C { parameterList: [i8; 6] },
    Z { parameterList: [i8; 6] },
    A { parameterList: [i8; 1] },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(non_snake_case)]
pub enum Stroke {
    SVGStroke {
        feature: String,
        start: (i8, i8),
        curveList: Vec<Draw>,
    },
    ReferenceStroke {
        feature: String,
        index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: usize,
    pub strokes: usize,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(non_snake_case)]
pub enum Glyph {
    BasicComponent {
        tags: Option<Vec<String>>,
        strokes: Vec<Stroke>,
    },
    DerivedComponent {
        tags: Option<Vec<String>>,
        source: String,
        strokes: Vec<Stroke>,
    },
    SplicedComponent {
        tags: Option<Vec<String>>,
        operator: String,
        operandList: Vec<String>,
        order: Option<Vec<Block>>,
    },
    Compound {
        tags: Option<Vec<String>>,
        operator: String,
        operandList: Vec<String>,
        order: Option<Vec<Block>>,
    },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reading {
    pub pinyin: String,
    pub importance: f64,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimitiveCharacter {
    pub unicode: usize,
    pub tygf: u8,
    pub gb2312: u8,
    #[serialize_always] // JavaScript null
    pub name: Option<String>,
    #[serialize_always] // JavaScript null
    pub gf0014_id: Option<usize>,
    #[serialize_always] // JavaScript null
    pub gf3001_id: Option<usize>,
    pub readings: Vec<Reading>,
    pub glyphs: Vec<Glyph>,
    pub ambiguous: bool,
}

pub type PrimitiveRepertoire = HashMap<String, PrimitiveCharacter>;
// config.data end

// config.analysis begin
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Analysis {
    pub classifier: Option<HashMap<String, usize>>,
    pub degenerator: Option<Degenerator>,
    pub selector: Option<Vec<String>>,
    pub customize: Option<HashMap<String, Vec<String>>>,
    pub strong: Option<Vec<String>>,
    pub weak: Option<Vec<String>>,
    pub serializer: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Degenerator {
    pub feature: Option<HashMap<String, String>>,
    pub no_cross: Option<bool>,
}
// config.analysis end

// config.algebra begin
type Algebra = HashMap<String, Vec<Rule>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Rule {
    Xform { from: String, to: String },
    Xlit { from: String, to: String },
}
// config.algebra end

// config.form begin
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormConfig {
    pub alphabet: String,
    pub mapping_type: Option<usize>,
    pub mapping: HashMap<String, Mapped>,
    pub grouping: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MappedKey {
    Ascii(char),
    Reference { element: String, index: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Mapped {
    Basic(String),
    Advanced(Vec<MappedKey>),
}
// config.form end

// config.encoder begin
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderConfig {
    // 全局
    pub max_length: usize,
    pub select_keys: Option<Vec<char>>,
    pub auto_select_length: Option<usize>,
    pub auto_select_pattern: Option<String>,
    // 一字词全码
    pub sources: Option<HashMap<String, NodeConfig>>,
    pub conditions: Option<HashMap<String, EdgeConfig>>,
    // 多字词全码
    pub rules: Option<Vec<WordRule>>,
    // 简码
    pub short_code: Option<Vec<ShortCodeConfig>>,
    pub priority_short_codes: Option<Vec<(String, String, usize)>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ShortCodeConfig {
    Equal {
        length_equal: usize,
        schemes: Vec<Scheme>,
    },
    Range {
        length_in_range: (usize, usize),
        schemes: Vec<Scheme>,
    },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scheme {
    pub prefix: usize,
    pub count: Option<usize>,
    pub select_keys: Option<Vec<char>>,
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
// config.encoder end

// config.optimization begin
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
    pub fingering: Option<FingeringWeights>,
}

// let types = ["同手", "大跨", "小跨", "干扰", "错手", "三连", "备用", "备用"];
pub type FingeringWeights = [Option<f64>; 8];

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialWeights {
    pub tiers: Option<Vec<TierWeights>>,
    pub duplication: Option<f64>,
    pub key_distribution: Option<f64>,
    pub pair_equivalence: Option<f64>,
    pub extended_pair_equivalence: Option<f64>,
    pub fingering: Option<FingeringWeights>,
    pub levels: Option<Vec<LevelWeights>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementWithIndex {
    pub element: String,
    pub index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementAffinityTarget {
    pub element: ElementWithIndex,
    pub affinity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyAffinityTarget {
    pub key: char,
    pub affinity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffinityList<T> {
    pub from: ElementWithIndex,
    pub to: Vec<T>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regularization {
    pub strength: Option<f64>,
    pub element_affinities: Option<Vec<AffinityList<ElementAffinityTarget>>>,
    pub key_affinities: Option<Vec<AffinityList<KeyAffinityTarget>>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveConfig {
    pub characters_full: Option<PartialWeights>,
    pub words_full: Option<PartialWeights>,
    pub characters_short: Option<PartialWeights>,
    pub words_short: Option<PartialWeights>,
    pub regularization: Option<Regularization>,
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
#[serde(tag = "algorithm")]
pub enum SolverConfig {
    SimulatedAnnealing(退火方法),
    // TODO: Add more algorithms
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    pub objective: ObjectiveConfig,
    pub constraints: Option<ConstraintsConfig>,
    pub metaheuristic: Option<SolverConfig>,
}
// config.optimization end

// config.diagram begin

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutRow {
    pub keys: Vec<char>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BoxConfig {
    Key { style: Option<String> },
    Uppercase { style: Option<String> },
    Element { r#match: Option<String>, style: Option<String> },
    Custom { mapping: Option<String>, style: Option<String> },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramConfig {
    pub layout: Vec<LayoutRow>,
    pub contents: Vec<BoxConfig>,
    pub row_style: Option<String>,
    pub cell_style: Option<String>,
}

// config.diagram end

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 配置 {
    pub version: Option<String>,
    #[serialize_always] // JavaScript null
    pub source: Option<String>,
    pub info: Option<Info>,
    pub data: Option<Data>,
    pub analysis: Option<Analysis>,
    pub algebra: Option<Algebra>,
    pub form: FormConfig,
    pub encoder: EncoderConfig,
    pub optimization: Option<OptimizationConfig>,
    pub diagram: Option<DiagramConfig>,
}

impl Default for 配置 {
    fn default() -> Self {
        配置 {
            version: None,
            source: None,
            info: None,
            data: None,
            analysis: None,
            algebra: None,
            form: FormConfig {
                alphabet: "abcdefghijklmnopqrstuvwxyz".to_string(),
                mapping_type: None,
                mapping: HashMap::new(),
                grouping: None,
            },
            encoder: EncoderConfig {
                max_length: 1,
                select_keys: None,
                auto_select_length: None,
                auto_select_pattern: None,
                sources: None,
                conditions: None,
                rules: None,
                short_code: None,
                priority_short_codes: None,
            },
            optimization: None,
            diagram: None,
        }
    }
}
