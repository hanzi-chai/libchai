//! 配置文件的定义
//!
//! 这部分内容太多，就不一一注释了。后期会写一个「`config.yaml` 详解」来统一解释各种配置文件的字段。
//!

use crate::optimizers::simulated_annealing::退火方法;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

// config.info begin
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 基本信息 {
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
}
// config.info end

// config.data begin
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 数据配置 {
    pub repertoire: Option<原始字库>,
    pub glyph_customization: Option<IndexMap<String, 字形>>,
    pub tags: Option<Vec<String>>,
    pub transformers: Option<Vec<变换器>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 变换器 {
    pub from: 模式,
    pub to: 模式,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct 模式 {
    operator: char,
    operandList: Vec<节点>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum 节点 {
    Pattern(模式),
    Variable { id: usize },
    Character(char),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
#[allow(non_snake_case)]
pub enum 绘制 {
    H { parameterList: [i16; 1] },
    V { parameterList: [i16; 1] },
    C { parameterList: [i16; 6] },
    Z { parameterList: [i16; 6] },
    A { parameterList: [i16; 1] },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(non_snake_case)]
pub enum 笔画 {
    矢量笔画 {
        feature: String,
        start: (i16, i16),
        curveList: Vec<绘制>,
    },
    引用笔画 {
        feature: String,
        index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 笔画块 {
    pub index: usize,
    pub strokes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 复合体参数 {
    pub gap2: Option<f64>,
    pub scale2: Option<f64>,
    pub gap3: Option<f64>,
    pub scale3: Option<f64>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(non_snake_case)]
pub enum 字形 {
    BasicComponent {
        tags: Option<Vec<String>>,
        strokes: Vec<笔画>,
    },
    DerivedComponent {
        tags: Option<Vec<String>>,
        source: String,
        strokes: Vec<笔画>,
    },
    SplicedComponent {
        tags: Option<Vec<String>>,
        operator: String,
        operandList: Vec<String>,
        order: Option<Vec<笔画块>>,
        parameters: Option<复合体参数>,
    },
    Compound {
        tags: Option<Vec<String>>,
        operator: String,
        operandList: Vec<String>,
        order: Option<Vec<笔画块>>,
        parameters: Option<复合体参数>,
    },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 原始汉字 {
    pub unicode: usize,
    pub tygf: u8,
    pub gb2312: u8,
    #[serialize_always] // JavaScript null
    pub name: Option<String>,
    #[serialize_always] // JavaScript null
    pub gf0014_id: Option<usize>,
    #[serialize_always] // JavaScript null
    pub gf3001_id: Option<usize>,
    pub glyphs: Vec<字形>,
    pub ambiguous: bool,
}

pub type 原始字库 = IndexMap<String, 原始汉字>;
// config.data end

// config.analysis begin
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 分析配置 {
    pub classifier: Option<IndexMap<String, usize>>,
    pub degenerator: Option<退化配置>,
    pub selector: Option<Vec<String>>,
    pub customize: Option<IndexMap<String, Vec<String>>>,
    pub dynamic_customize: Option<IndexMap<String, Vec<Vec<String>>>>,
    pub strong: Option<Vec<String>>,
    pub weak: Option<Vec<String>>,
    pub component_analyzer: Option<String>,
    pub compound_analyzer: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 退化配置 {
    pub feature: Option<IndexMap<String, String>>,
    pub no_cross: Option<bool>,
}
// config.analysis end

// config.algebra begin
type 拼写运算自定义 = IndexMap<String, Vec<运算规则>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum 运算规则 {
    Xform { from: String, to: String },
    Xlit { from: String, to: String },
}
// config.algebra end

// config.form begin
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 键盘配置 {
    pub alphabet: String,
    pub mapping_type: Option<usize>,
    pub mapping: IndexMap<String, 安排>,
    pub mapping_space: Option<IndexMap<String, Vec<安排描述>>>,
    pub mapping_variables: Option<IndexMap<String, 变量规则>>,
    pub mapping_generators: Option<Vec<决策生成器规则>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 变量规则 {
    pub keys: Vec<char>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 决策生成器规则 {
    pub regex: String,
    pub value: 安排描述,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 安排描述 {
    pub value: 安排,
    pub score: f64,
    pub condition: Option<Vec<条件>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct 条件 {
    pub element: String,
    pub op: String,
    pub value: 安排,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum 广义码位 {
    Ascii(char),
    Reference { element: String, index: usize },
    Variable { variable: String },
    Placeholder(()),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum 安排 {
    Basic(String),
    Advanced(Vec<广义码位>),
    Grouped { element: String },
    Unused(()),
}
// config.form end

// config.encoder begin
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 编码配置 {
    // 全局
    pub max_length: usize,
    pub select_keys: Option<Vec<char>>,
    pub auto_select_length: Option<usize>,
    pub auto_select_pattern: Option<String>,
    // 一字词全码
    pub sources: Option<IndexMap<String, 源节点配置>>,
    pub conditions: Option<IndexMap<String, 条件节点配置>>,
    // 多字词全码
    pub rules: Option<Vec<构词规则>>,
    // 简码
    pub short_code: Option<Vec<简码规则>>,
    pub short_code_list: Option<Vec<优先简码>>,
    // 组装器
    pub assembler: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 优先简码 {
    pub word: String,
    pub sources: Vec<Vec<String>>,
    pub level: usize,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct 取码对象 {
    pub r#type: String,
    pub subtype: Option<String>,
    pub key: Option<String>,
    pub rootIndex: Option<i64>,
    pub strokeIndex: Option<i64>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 源节点配置 {
    #[serialize_always] // JavaScript null
    pub object: Option<取码对象>,
    pub index: Option<usize>,
    #[serialize_always] // JavaScript null
    pub next: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 条件节点配置 {
    pub object: 取码对象,
    pub operator: String,
    pub value: Option<String>,
    #[serialize_always] // JavaScript null
    pub positive: Option<String>,
    #[serialize_always] // JavaScript null
    pub negative: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum 简码规则 {
    Equal {
        length_equal: usize,
        schemes: Vec<简码模式>,
    },
    Range {
        length_in_range: (usize, usize),
        schemes: Vec<简码模式>,
    },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 简码模式 {
    pub prefix: usize,
    pub count: Option<usize>,
    pub select_keys: Option<Vec<char>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum 构词规则 {
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
pub struct 码长权重 {
    pub length: usize,
    pub frequency: f64,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 层级权重 {
    pub top: Option<usize>,
    pub duplication: Option<f64>,
    pub levels: Option<Vec<码长权重>>,
    pub fingering: Option<指法权重>,
}

// let types = ["同手", "大跨", "小跨", "干扰", "错手", "三连", "备用", "备用"];
pub type 指法权重 = [Option<f64>; 8];

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 部分权重 {
    pub tiers: Option<Vec<层级权重>>,
    pub duplication: Option<f64>,
    pub key_distribution: Option<f64>,
    pub pair_equivalence: Option<f64>,
    pub extended_pair_equivalence: Option<f64>,
    pub fingering: Option<指法权重>,
    pub levels: Option<Vec<码长权重>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 目标配置 {
    pub characters_full: Option<部分权重>,
    pub words_full: Option<部分权重>,
    pub characters_short: Option<部分权重>,
    pub words_short: Option<部分权重>,
    pub regularization_strength: Option<f64>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "algorithm")]
pub enum 求解器配置 {
    SimulatedAnnealing(退火方法),
    // TODO: Add more algorithms
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 优化配置 {
    pub objective: 目标配置,
    pub metaheuristic: Option<求解器配置>,
}
// config.optimization end

// config.diagram begin

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutRow {
    pub keys: Vec<char>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum 区块配置 {
    Key {
        style: Option<String>,
    },
    Uppercase {
        style: Option<String>,
    },
    Element {
        r#match: Option<String>,
        style: Option<String>,
    },
    Custom {
        mapping: Option<String>,
        style: Option<String>,
    },
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 图示配置 {
    pub layout: Vec<LayoutRow>,
    pub contents: Vec<区块配置>,
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
    pub info: Option<基本信息>,
    pub data: Option<数据配置>,
    pub analysis: Option<分析配置>,
    pub algebra: Option<拼写运算自定义>,
    pub form: 键盘配置,
    pub encoder: 编码配置,
    pub optimization: Option<优化配置>,
    pub diagram: Option<图示配置>,
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
            form: 键盘配置 {
                alphabet: "abcdefghijklmnopqrstuvwxyz".to_string(),
                mapping_type: None,
                mapping: IndexMap::new(),
                mapping_space: None,
                mapping_variables: None,
                mapping_generators: None,
            },
            encoder: 编码配置 {
                max_length: 1,
                select_keys: None,
                auto_select_length: None,
                auto_select_pattern: None,
                sources: None,
                conditions: None,
                rules: None,
                short_code: None,
                short_code_list: None,
                assembler: None,
            },
            optimization: None,
            diagram: None,
        }
    }
}
