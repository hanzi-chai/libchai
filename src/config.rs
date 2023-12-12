use std::{fs, vec, collections::HashMap};
use yaml_rust::{Yaml, YamlLoader};
use crate::encoder::Elements;

pub type KeyMap = HashMap<String, char>;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct FormConfig {
    pub alphabet: Vec<char>,
    // pub maxcodelen: usize,
    pub grouping: HashMap<String, String>,
    pub mapping: HashMap<String, char>
}

#[derive(Debug, Clone)]
pub struct EncoderConfig {
    pub auto_select_length: usize,
    pub rules: Vec<WordRule>,
}

#[derive(Debug, Clone)]
pub struct TieredMetricWeights {
    pub top: Option<usize>,
    pub duplication: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct PartialMetricWeights {
    pub tiered: Vec<TieredMetricWeights>,
    pub duplication: Option<f64>,
    pub equivalence: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ObjectiveConfig {
    pub characters: Option<PartialMetricWeights>,
    pub words: Option<PartialMetricWeights>,
}

#[derive(Debug, Clone)]
pub struct AtomicConstraint {
    pub element: Option<String>,
    pub index: Option<usize>,
    pub keys: Option<Vec<char>>
}

#[derive(Debug, Clone)]
pub struct ConstraintsConfig {
    pub values: Vec<AtomicConstraint>
}

#[derive(Debug, Clone)]
pub enum MetaheuristicConfig {
    HillClimbing,
    SimulatedAnnealing
}

#[derive(Debug, Clone)]
pub struct OptimizationConfig {
    pub objective: ObjectiveConfig,
    pub constraints: ConstraintsConfig,
    pub metaheuristic: MetaheuristicConfig
}

#[derive(Debug, Clone)]
pub struct Config {
    pub yaml: Yaml,
    pub form: FormConfig,
    pub encoder: EncoderConfig,
    pub optimization: OptimizationConfig,
}

impl Config {
    pub fn new(name: &String) -> Self {
        let content = fs::read_to_string(name).expect("Should have been able to read the file");
        let mut multi = YamlLoader::load_from_str(&content).unwrap();
        let yaml = multi.pop().unwrap();
        let encoder = Self::build_config_encoder(&yaml["encoder"]);
        let form = Self::build_config_form(&yaml["form"]);
        let optimization = Self::build_config_optimization(&yaml["optimization"]);
        Config { yaml, form, encoder, optimization }
    }

    fn build_config_form(yaml: &Yaml) -> FormConfig {
        let mut grouping: HashMap<String, String> = HashMap::new();
        let mut mapping: KeyMap = HashMap::new();
        let alphabet = yaml["alphabet"].as_str().unwrap().to_string().chars().collect();
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
        FormConfig { alphabet, grouping, mapping }
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
    
    fn build_tiered_metric_weights(yaml: &Yaml) -> TieredMetricWeights {
        let top = yaml["top"].as_i64().and_then(|x| Some(x as usize));
        let duplication = yaml["duplication"].as_f64();
        TieredMetricWeights { top, duplication }
    }

    fn build_partial_metric_weights(yaml: &Yaml) -> Option<PartialMetricWeights> {
        if yaml.is_badvalue() {
            None
        } else {
            let duplication = yaml["duplication"].as_f64();
            let equivalence = yaml["equivalence"].as_f64();
            let tiered: Vec<TieredMetricWeights> = if let Some(raw) = yaml["tiered"].as_vec() {
                raw.iter().map(|x| Self::build_tiered_metric_weights(x)).collect()
            } else {
                vec![]
            };
            Some(PartialMetricWeights { tiered, duplication, equivalence })
        }
    }

    fn build_objective(yaml: &Yaml) -> ObjectiveConfig {
        let characters = Self::build_partial_metric_weights(&yaml["characters"]);
        let words = Self::build_partial_metric_weights(&yaml["words"]);
        ObjectiveConfig { characters, words }
    }

    fn build_constraint(yaml: &Yaml) -> AtomicConstraint {
        let element = yaml["element"].as_str().and_then(|x| Some(x.to_string()));
        let index = yaml["index"].as_i64().and_then(|x| Some(x as usize));
        let keys: Option<Vec<char>> = yaml["keys"].as_vec().and_then(|v| Some(v.iter().map(|x| x.as_str().unwrap().chars().next().unwrap()).collect()));
        AtomicConstraint { element, index, keys }
    }

    fn build_constraints(yaml: &Yaml) -> ConstraintsConfig {
        let elements = yaml["elements"].as_vec().unwrap().clone();
        let mut indices = yaml["indices"].as_vec().unwrap().clone();
        let mut element_indices = yaml["element_indices"].as_vec().unwrap().clone();
        let mut all = elements;
        all.append(&mut indices);
        all.append(&mut element_indices);
        let values = all.iter().map(Self::build_constraint).collect();
        ConstraintsConfig { values }
    }

    fn build_metaheuristic(yaml: &Yaml) -> MetaheuristicConfig {
        let algorithm = yaml["algorithm"].as_str().unwrap();
        match algorithm {
            "simulated_annealing" => MetaheuristicConfig::SimulatedAnnealing,
            "hill_climbing" => MetaheuristicConfig::HillClimbing,
            _ => panic!("Unknown algorithm")
        }
    }

    fn build_config_optimization(yaml: &Yaml) -> OptimizationConfig {
        let objective = Self::build_objective(&yaml["objective"]);
        let constraints = Self::build_constraints(&yaml["constraints"]);
        let metaheuristic = Self::build_metaheuristic(&yaml["metaheuristic"]);
        OptimizationConfig { objective, constraints, metaheuristic }
    }

    pub fn validate_elements(&self, elements: &Elements) {
        let mapping = &self.form.mapping;
        for (_, elems) in elements {
            for element in elems {
                if let None = mapping.get(element) {
                    panic!("Invalid element: {}", element);
                }
            }
        }
    }
}
