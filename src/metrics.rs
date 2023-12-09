use std::collections::HashMap;
use std::collections::HashSet;
use std::f64::INFINITY;

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
pub struct EvaluationResult {
    tiered_duplication: Vec<i32>,
    equivalence: f64,
}

type CombinedResult = (EvaluationResult, EvaluationResult);

pub fn is_better(r1: &CombinedResult, r2: &CombinedResult) -> bool {
    let (r1c, r1w) = r1;
    let (r2c, r2w) = r2;
    for (itier, dup) in r1c.tiered_duplication.iter().enumerate() {
        if *dup > r2c.tiered_duplication[itier] {
            return false;
        }
    }
    for (itier, dup) in r1w.tiered_duplication.iter().enumerate() {
        if *dup > r2w.tiered_duplication[itier] {
            return false;
        }
    }
    if r1c.equivalence > r2c.equivalence {
        return false;
    }
    if r1w.equivalence > r2w.equivalence {
        return false;
    }
    return true;
}

pub fn make_dummy_result(ntier: usize) -> EvaluationResult {
    EvaluationResult { tiered_duplication: vec![0; ntier], equivalence: INFINITY }
}

pub fn evaluate(
    ranked: &Vec<RankedData>,
    keymap: &HashMap<String, String>,
    equivalence_map: &HashMap<String, f64>,
    tiers: &Vec<usize>,
) -> EvaluationResult {
    let mut codes: Vec<String> = Vec::new();
    let ntier = tiers.len();
    let mut tiered_duplication: Vec<i32> = tiers.iter().map(|_| 0).collect();
    tiered_duplication.push(0);
    let mut equivalence = 0.0;
    let mut occupied_codes: HashSet<String> = HashSet::new();
    for (index, data) in ranked.iter().enumerate() {
        let mut code = String::new();
        for element in &data.elements {
            if let Some(zone) = keymap.get(element) {
                code.push_str(zone);
            }
        }
        equivalence += calculate_total_equivalence(&code, equivalence_map);
        if let Some(_) = occupied_codes.get(&code) {
            for (itier, tier) in tiers.iter().enumerate() {
                if index <= *tier {
                    tiered_duplication[itier] += 1;
                }
            }
            tiered_duplication[ntier] += 1;
        }
        occupied_codes.insert(code.clone());
        codes.push(code);
    }
    EvaluationResult {
        tiered_duplication,
        equivalence,
    }
}
