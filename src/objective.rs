use crate::assets::Assets;
use crate::assets::Frequency;
use crate::encoder::Code;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;

pub struct RankedData {
    code: String,
    frequency: usize,
}

pub fn merge_codes_and_frequency<T: Eq + Hash>(
    codes: &HashMap<T, String>,
    frequency: &Frequency<T>,
) -> Vec<RankedData> {
    let mut rank: Vec<RankedData> = Vec::new();
    for (key, code) in codes {
        let this_frequency = frequency.get(key).unwrap_or(&0);
        let data = RankedData {
            code: code.to_string(),
            frequency: *this_frequency,
        };
        rank.push(data);
    }
    rank.sort_by(|a, b| a.frequency.cmp(&b.frequency));
    return rank;
}

#[derive(Debug, Clone)]
pub struct TieredMetric {
    duplication: usize,
    capacity: [usize; 4],
}

pub struct TieredMetricWeights {
    duplication: f64,
    capacity: [f64; 4],
}

#[derive(Debug, Clone)]
pub struct PartialMetric {
    tiered: Vec<TieredMetric>,
    length: f64,
    duplication_rate: f64,
    equivalence: f64,
}

pub struct PartialMetricWeights {
    tiers: Vec<usize>,
    tiered: Vec<TieredMetricWeights>,
    length: f64,
    duplication_rate: f64,
    equivalence: f64,
}

pub struct Metric {
    pub characters: PartialMetric,
    pub characters_reduced: PartialMetric,
    pub words: PartialMetric,
    pub words_reduced: PartialMetric,
}

pub struct Objective {
    characters: PartialMetricWeights,
    words: PartialMetricWeights,
    assets: Assets,
}

impl Objective {
    pub fn new(assets: Assets) -> Objective {
        Objective {
            characters: PartialMetricWeights {
                tiers: vec![1500],
                tiered: vec![
                    TieredMetricWeights {
                        duplication: 10.0,
                        capacity: [0.0, 5.0, 3.0, 0.0],
                    },
                    TieredMetricWeights {
                        duplication: 10.0,
                        capacity: [0.0, 5.0, 3.0, 0.0],
                    },
                ],
                length: 1.0,
                duplication_rate: 10.0,
                equivalence: 1.0,
            },
            words: PartialMetricWeights {
                tiers: vec![2000, 10000],
                tiered: vec![
                    TieredMetricWeights {
                        duplication: 0.3,
                        capacity: [0.0, 5.0, 3.0, 0.0],
                    },
                    TieredMetricWeights {
                        duplication: 0.2,
                        capacity: [0.0, 5.0, 3.0, 0.0],
                    },
                    TieredMetricWeights {
                        duplication: 0.1,
                        capacity: [0.0, 5.0, 3.0, 0.0],
                    },
                ],
                length: 1.0,
                duplication_rate: 5.0,
                equivalence: 1.0,
            },
            assets,
        }
    }

    fn calculate_total_equivalence(&self, code: &String) -> f64 {
        let mut total = 0.0;
        let mut it = code.chars();
        let mut this = it.next().unwrap();
        while let Some(next) = it.next() {
            total += self.assets.equivalence.get(&(this, next)).unwrap_or(&0.0);
            this = next;
        }
        total
    }

    pub fn evaluate_partial_metric<T: Eq + Hash>(
        &self,
        codes: &HashMap<T, String>,
        frequency: &Frequency<T>,
        weights: &PartialMetricWeights,
    ) -> PartialMetric {
        let ranked = merge_codes_and_frequency(codes, frequency);
        let ntier = weights.tiers.len();
        let initial_tiered = TieredMetric {
            duplication: 0,
            capacity: [0; 4],
        };
        let mut tiered: Vec<TieredMetric> = vec![initial_tiered; ntier + 1];
        let mut occupied_codes: HashSet<String> = HashSet::new();
        let mut total_frequency = 0;
        let mut total_length = 0;
        let mut total_duplication = 0;
        let mut total_equivalence = 0.0;
        for (index, data) in ranked.iter().enumerate() {
            total_frequency += data.frequency;
            total_length += data.code.len() * data.frequency;
            total_equivalence +=
                self.calculate_total_equivalence(&data.code) * data.frequency as f64;
            if let Some(_) = occupied_codes.get(&data.code) {
                total_duplication += data.frequency;
                for (itier, tier) in weights.tiers.iter().enumerate() {
                    if index <= *tier {
                        tiered[itier].duplication += 1;
                    }
                }
                tiered[ntier].duplication += 1;
            }
            occupied_codes.insert(data.code.clone());
        }
        let total_frequency = total_frequency as f64;
        PartialMetric {
            duplication_rate: total_duplication as f64 / total_frequency,
            equivalence: total_equivalence / total_frequency,
            length: total_length as f64 / total_frequency,
            tiered,
        }
    }

    pub fn evaluate_metric(
        &self,
        character_codes: &Code<char>,
        word_codes: &Code<String>,
    ) -> Metric {
        let characters_result =
            self.evaluate_partial_metric(&character_codes, &self.assets.characters, &self.characters);
        let words_result =
            self.evaluate_partial_metric(&word_codes, &self.assets.words, &self.words);
        Metric {
            characters: characters_result.clone(),
            characters_reduced: characters_result.clone(),
            words: words_result.clone(),
            words_reduced: words_result.clone(),
        }
    }

    fn scalarize_tiered_metric(this: &TieredMetric, adjoint: &TieredMetricWeights) -> f64 {
        let mut real = this.duplication as f64 * adjoint.duplication;
        for (c, cw) in this.capacity.iter().zip(adjoint.capacity.iter()) {
            real += *c as f64 * cw;
        }
        return real;
    }

    fn scalarize_common_metric(this: &PartialMetric, adjoint: &PartialMetricWeights) -> f64 {
        let mut real = this.equivalence * adjoint.equivalence
            + this.duplication_rate * adjoint.duplication_rate
            + this.length * adjoint.length;
        for (tm, tmw) in this.tiered.iter().zip(adjoint.tiered.iter()) {
            real += Self::scalarize_tiered_metric(tm, tmw);
        }
        return real;
    }

    pub fn evaluate(&self, character_codes: &Code<char>, word_codes: &Code<String>) -> f64 {
        let metric = self.evaluate_metric(character_codes, word_codes);
        let Metric {
            characters, words, ..
        } = metric;
        return Self::scalarize_common_metric(&characters, &self.characters)
            + Self::scalarize_common_metric(&words, &self.words);
    }
}
