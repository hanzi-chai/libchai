use std::{fs, path::PathBuf, vec, collections::HashMap};
use yaml_rust::{Yaml, YamlLoader};

pub type KeyMap = HashMap<String, char>;

#[derive(Debug)]
pub enum WordRule {
    EqualRule {
        length_equal: usize,
        formula: String,
    },
    RangeRule {
        length_in_range: Vec<usize>,
        formula: String,
    },
}

fn get_default_rules() -> Vec<WordRule> {
    vec![
        WordRule::EqualRule {
            length_equal: 2,
            formula: String::from("AaAbBaBb"),
        },
        WordRule::EqualRule {
            length_equal: 3,
            formula: String::from("AaBaCaCb"),
        },
        WordRule::RangeRule {
            length_in_range: vec![4],
            formula: String::from("AaBaCaZa"),
        },
    ]
}

#[derive(Debug)]
pub struct FormConfig {
    // pub alphabet: String,
    // pub maxcodelen: usize,
    pub grouping: HashMap<String, String>,
    pub mapping: HashMap<String, char>
}

#[derive(Debug)]
pub struct EncoderConfig {
    pub auto_select_length: usize,
    pub rules: Vec<WordRule>,
}

#[derive(Debug)]
pub struct Config {
    pub form: FormConfig,
    pub encoder: EncoderConfig,
}

impl Config {
    pub fn new(name: &PathBuf) -> Config {
        let content = fs::read_to_string(name).expect("Should have been able to read the file");
        let raw = YamlLoader::load_from_str(&content).unwrap();
        let yaml = raw[0].clone();
        Self::build_config(&yaml)
    }

    fn build_config_form(yaml: &Yaml) -> FormConfig {
        let mut grouping: HashMap<String, String> = HashMap::new();
        let mut mapping: KeyMap = HashMap::new();
        let _mapping = yaml["mapping"].as_hash().unwrap();
        let _grouping = yaml["grouping"].as_hash().unwrap();
        for (_element, _mapped) in _mapping {
            let element = _element.as_str().unwrap();
            let mapped: Vec<char> = _mapped.as_str().unwrap().chars().collect();
            if mapped.len() == 1 {
                mapping.insert(element.to_string(), mapped[0]);
            } else {
                for (index, key) in mapped.iter().enumerate() {
                    mapping.insert(format!("{}.{}", element.to_string(), index), *key);
                }
            }
        }
        for (_element, _mapped) in _grouping {
            let element = _element.as_str().unwrap();
            let mapped = _mapped.as_str().unwrap();
            grouping.insert(element.to_string(), mapped.to_string());
        }
        FormConfig { grouping, mapping }
    }
    
    fn build_config_encoder(yaml: &Yaml) -> EncoderConfig {
        let auto_select_length = yaml["auto_select_length"].as_i64().unwrap() as usize;
        let rules = if let Some(vec) = yaml["rules"].as_vec() {
            let mut parsed_rules: Vec<WordRule> = vec![];
            for content in vec {
                let formula = content["formula"].as_str().unwrap().to_string();
                let rule = if let Some(length_equal) = content["length_equal"].as_i64() {
                    WordRule::EqualRule { length_equal: length_equal as usize, formula }
                } else {
                    let v = content["length_in_range"].as_vec().unwrap();
                    let length_in_range = v.iter().map(|yaml| yaml.as_i64().unwrap() as usize).collect();
                    WordRule::RangeRule { length_in_range, formula }
                };
                parsed_rules.push(rule);
            }
            parsed_rules
        } else {
            get_default_rules()
        };
        return EncoderConfig {
            auto_select_length,
            rules,
        };
    }
    
    pub fn build_config(yaml: &Yaml) -> Config {
        let encoder = Self::build_config_encoder(&yaml["encoder"]);
        let form = Self::build_config_form(&yaml["form"]);
        Config { form, encoder }
    }
    
}
