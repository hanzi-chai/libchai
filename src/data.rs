//! 递归定义汉字自动拆分所需要的基本数据格式。
//! 
//! 这部分内容太多，就不一一注释了。在开发文档中有详细解释。
//! 

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Draw {
    pub command: String,
    pub parameterList: Vec<i8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[serde(untagged)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(non_snake_case)]
pub enum Glyph {
    BasicComponent {
        tags: Option<Vec<String>>,
        strokes: Vec<Stroke>,
    },
    DerivedComponent {
        source: Option<String>,
        tags: Option<Vec<String>>,
        strokes: Vec<Stroke>,
    },
    Compound {
        operator: String,
        operandList: Vec<String>,
        tags: Option<Vec<String>>,
        order: Option<Vec<Block>>,
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub unicode: usize,
    pub tygf: u8,
    pub gb2312: bool,
    #[serialize_always] // JavaScript null
    pub name: Option<String>,
    #[serialize_always] // JavaScript null
    pub gf0014_id: Option<usize>,
    pub readings: Vec<String>,
    pub glyphs: Vec<Glyph>,
    pub ambiguous: Option<bool>,
}
