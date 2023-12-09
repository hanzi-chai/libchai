use std::collections::HashMap;
use std::collections::HashSet;

pub struct RankedData {
    elements: Vec<String>,
    frequency: i32,
}

fn calculate_total_equivalence(code: &String, equivalence_map: &HashMap<String, f64>) -> f64 {
    let mut total = 0.0;
    for index in 0..(code.len() - 1) {
        total += equivalence_map.get(&code[index..index + 2]).unwrap_or(&0.0);
    }
    total
}

pub fn merge_elements_and_frequency(
    elements: &HashMap<String, Vec<String>>,
    frequency: &HashMap<String, i32>,
) -> Vec<RankedData> {
    let mut rank: Vec<RankedData> = Vec::new();
    for (key, this_elements) in elements {
        let this_frequency = frequency.get(key).unwrap_or(&0);
        let data = RankedData {
            elements: this_elements.to_vec(),
            frequency: *this_frequency,
        };
        rank.push(data);
    }
    rank.sort_by(|a, b| a.frequency.cmp(&b.frequency));
    return rank;
}

#[derive(Debug)]
pub struct PartialMetric {
    tiered_duplication: Vec<i32>,
    duplication_rate: f64,
    equivalence: f64,
}

pub struct PartialMetricWeights {
    tiered_duplication: Vec<f64>,
    duplication_rate: f64,
    equivalence: f64,
}

impl PartialMetric {
    pub fn real_value(&self, adjoint: &PartialMetricWeights) -> f64 {
        let mut real = self.equivalence * adjoint.equivalence + self.duplication_rate * adjoint.duplication_rate;
        for (index, value) in self.tiered_duplication.iter().enumerate() {
            real += (*value as f64) * adjoint.tiered_duplication[index];
        }
        return real;
    }
}

pub struct Metric {
    pub characters: PartialMetric,
    pub words: PartialMetric,
}

pub struct MetricWeights {
    characters: PartialMetricWeights,
    words: PartialMetricWeights,
}

impl Metric {
    pub fn real_value(&self, adjoint: &MetricWeights) -> f64 {
        let Metric { characters, words } = self;
        return characters.real_value(&adjoint.characters) + words.real_value(&adjoint.words);
    }
}

impl MetricWeights {
    pub fn new() -> MetricWeights {
        MetricWeights {
            characters: PartialMetricWeights {
                tiered_duplication: vec![10.0, 1.0],
                duplication_rate: 10.0,
                equivalence: 1.0,
            },
            words: PartialMetricWeights {
                tiered_duplication: vec![0.3, 0.2, 0.1],
                duplication_rate: 5.0,
                equivalence: 1.0,
            },
        }
    }
}

pub fn evaluate(
    ranked: &Vec<RankedData>,
    keymap: &HashMap<String, String>,
    equivalence_map: &HashMap<String, f64>,
    tiers: &Vec<usize>,
) -> PartialMetric {
    let mut codes: Vec<String> = Vec::new();
    let ntier = tiers.len();
    let mut tiered_duplication: Vec<i32> = tiers.iter().map(|_| 0).collect();
    tiered_duplication.push(0);
    let mut total_equivalence = 0.0;
    let mut occupied_codes: HashSet<String> = HashSet::new();
    let mut total_frequency = 0;
    let mut total_duplication = 0;
    for (index, data) in ranked.iter().enumerate() {
        total_frequency += data.frequency;
        let mut code = String::new();
        for element in &data.elements {
            if let Some(zone) = keymap.get(element) {
                code.push_str(zone);
            }
        }
        total_equivalence += calculate_total_equivalence(&code, equivalence_map) * data.frequency as f64;
        if let Some(_) = occupied_codes.get(&code) {
            for (itier, tier) in tiers.iter().enumerate() {
                if index <= *tier {
                    tiered_duplication[itier] += 1;
                }
            }
            tiered_duplication[ntier] += 1;
            total_duplication += data.frequency;
        }
        occupied_codes.insert(code.clone());
        codes.push(code);
    }
    let equivalence = total_equivalence / total_frequency as f64;
    let duplication_rate = total_duplication as f64 / total_frequency as f64;
    PartialMetric {
        duplication_rate,
        tiered_duplication,
        equivalence,
    }
}
