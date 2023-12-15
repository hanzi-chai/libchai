use serde::{Serialize, Deserialize};
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
        curveList: Vec<Draw>
    },
    IndexedStroke(i8)
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub source: Option<String>,
    pub strokes: Vec<Stroke>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: usize,
    pub strokes: usize
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Partition {
    pub operator: String,
    pub operandList: Vec<String>,
    pub tags: Option<Vec<String>>,
    pub order: Option<Vec<Block>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Glyph {
    pub unicode: usize,
    #[serialize_always] // JavaScript null
    pub name: Option<String>,
    #[serialize_always] // JavaScript null
    pub gf0014_id: Option<usize>,
    pub default_type: String,
    pub component: Option<Component>,
    pub compound: Option<Vec<Partition>>,
    pub ambiguous: Option<bool>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub unicode: usize,
    pub pinyin: Vec<String>,
    pub tygf: u8,
    pub gb2312: bool
}
