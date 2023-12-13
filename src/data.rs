use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Draw {
    pub command: String,
    pub parameterList: Vec<i8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct SVGStroke {
    pub feature: String,
    pub start: (i8, i8),
    pub curveList: Vec<Draw>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub source: Option<String>,
    pub strokes: Vec<SVGStroke>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: usize,
    pub strokes: usize
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Partition {
    pub operator: String,
    pub operandList: Vec<String>,
    pub tags: Option<Vec<String>>,
    pub order: Option<Vec<Block>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Glyph {
    pub unicode: usize,
    pub name: Option<String>,
    pub gf0014_id: Option<usize>,
    pub component: Option<Component>,
    pub compound: Option<Vec<Partition>>,
    pub ambiguous: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub unicode: usize,
    pub pinyin: Vec<String>,
    pub tygf: u8,
    pub gb2312: bool
}
