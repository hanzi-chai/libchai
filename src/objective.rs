use crate::cli::Assets;
use crate::config::Cache;
use crate::config::Config;
use crate::config::KeyMap;
use crate::config::ObjectiveConfig;
use crate::config::PartialMetricWeights;
use crate::encoder::Code;
use crate::encoder::Encoder;
use crate::encoder::RawElements;
use crate::metric::LevelMetric1;
use crate::metric::LevelMetric2;
use crate::metric::Metric;
use crate::metric::PartialMetric;
use crate::metric::TieredMetric;
use std::collections::HashSet;

pub struct Objective {
    config: ObjectiveConfig,
    assets: Assets,
    encoder: Encoder,
}

impl Objective {
    pub fn new(config: &Config, cache: &Cache, raw_elements: RawElements, assets: Assets) -> Objective {
        let elements = cache.transform_elements(&raw_elements);
        let encoder = Encoder::new(&config, elements, &assets);
        Objective {
            assets,
            encoder,
            config: config.optimization.objective.clone(),
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

    pub fn evaluate_partial(
        &self,
        codes: &Code,
        weights: &PartialMetricWeights,
    ) -> (PartialMetric, f64) {
        let mut occupied_codes: HashSet<String> = HashSet::new();
        let mut total_frequency = 0;
        let mut total_pairs = 0;
        let mut total_duplication = 0;
        let ntier = weights.tiered.as_ref().unwrap_or(&vec![]).len();
        let mut tiered_duplication = vec![0; ntier];
        let mut total_equivalence = 0.0;
        let mut total_levels = vec![0_usize; weights.levels.as_ref().unwrap_or(&vec![]).len()];
        let mut tiered_levels: Vec<Vec<usize>> = vec![];
        if let Some(tiered) = &weights.tiered {
            for tier in tiered {
                let vec = vec![0_usize, tier.levels.as_ref().unwrap_or(&vec![]).len()];
                tiered_levels.push(vec);
            }
        }
        for (index, (code, frequency)) in codes.iter().enumerate() {
            total_frequency += frequency;
            if let Some(_) = weights.equivalence {
                total_equivalence += self.calculate_total_equivalence(code) * *frequency as f64;
                total_pairs += (code.len() - 1) * frequency;
            }
            // 重码相关
            if let Some(_) = occupied_codes.get(code) {
                total_duplication += frequency;
                if let Some(tiered) = &weights.tiered {
                    for (itier, tier) in tiered.iter().enumerate() {
                        let top = tier.top.unwrap_or(std::usize::MAX);
                        if index <= top {
                            tiered_duplication[itier] += 1;
                        }
                    }
                }
            }
            // 简码相关
            if let Some(levels) = &weights.levels {
                for (ilevel, level) in levels.iter().enumerate() {
                    if level.length == code.len() {
                        total_levels[ilevel] += frequency;
                    }
                }
            }
            if let Some(tiered) = &weights.tiered {
                for (itier, tier) in tiered.iter().enumerate() {
                    let top = tier.top.unwrap_or(std::usize::MAX);
                    if index <= top {
                        if let Some(levels) = &tier.levels {
                            for (ilevel, level) in levels.iter().enumerate() {
                                if level.length == code.len() {
                                    tiered_levels[itier][ilevel] += 1;
                                }
                            }
                        }
                    }
                }
            }
            occupied_codes.insert(code.clone());
        }
        let total_frequency = total_frequency as f64;
        let total_pairs = total_pairs as f64;
        let mut partial_metric = PartialMetric {
            tiered: None,
            equivalence: None,
            duplication: None,
            levels: None,
        };

        let mut real = 0.0;
        if let Some(equivalence_weight) = weights.equivalence {
            let equivalence = total_equivalence / total_pairs;
            partial_metric.equivalence = Some(equivalence);
            real += equivalence * equivalence_weight;
        }
        if let Some(duplication_weight) = weights.duplication {
            let duplication = total_duplication as f64 / total_frequency;
            partial_metric.duplication = Some(duplication);
            real += duplication * duplication_weight;
        }
        if let Some(levels_weight) = &weights.levels {
            let mut levels: Vec<LevelMetric2> = Vec::new();
            for (ilevel, level) in levels_weight.iter().enumerate() {
                let value = total_levels[ilevel] as f64 / total_frequency;
                real += value * level.frequency;
                levels.push(LevelMetric2 { length: level.length, frequency: value })
            }
            partial_metric.levels = Some(levels);
        }
        if let Some(tiered_weight) = &weights.tiered {
            let mut tiered: Vec<TieredMetric> = tiered_weight
                .iter()
                .map(|x| TieredMetric {
                    top: x.top,
                    duplication: None,
                    levels: None,
                })
                .collect();
            for (itier, twights) in tiered_weight.iter().enumerate() {
                let total = twights.top.unwrap_or(codes.len());
                if let Some(duplication_weight) = twights.duplication {
                    let duplication = tiered_duplication[itier];
                    real += duplication as f64 / total as f64 * duplication_weight;
                    tiered[itier].duplication = Some(duplication);
                }
                if let Some(level_weight) = &twights.levels {
                    for (ilevel, level) in level_weight.iter().enumerate() {
                        real += tiered_levels[itier][ilevel] as f64 / total as f64 * level.frequency;
                    }
                    tiered[itier].levels = Some(level_weight.iter().enumerate().map(|(i, v)| LevelMetric1 { length: v.length, frequency: tiered_levels[itier][i] }).collect());
                }
            }
            partial_metric.tiered = Some(tiered);
        }
        return (partial_metric, real);
    }

    pub fn evaluate(&self, candidate: &KeyMap) -> (Metric, f64) {
        let mut loss = 0.0;
        let mut metric = Metric {
            characters: None,
            words: None,
            characters_reduced: None,
            words_reduced: None,
        };
        if let Some(characters) = &self.config.characters {
            let character_codes = self.encoder.encode_character_full(&candidate);
            let (partial, accum) = self.evaluate_partial(&character_codes, characters);
            loss += accum;
            metric.characters = Some(partial);
            if let Some(character_reduced) = &self.config.characters_reduced {
                let character_codes_reduced = self.encoder.encode_reduced(&character_codes);
                let (partial, accum) =
                    self.evaluate_partial(&character_codes_reduced, character_reduced);
                loss += accum;
                metric.characters_reduced = Some(partial);
            }
        }
        if let Some(words) = &self.config.words {
            let word_codes = self.encoder.encode_words_full(&candidate);
            let (partial, accum) = self.evaluate_partial(&word_codes, words);
            loss += accum;
            metric.words = Some(partial);
            if let Some(words_reduced) = &self.config.words_reduced {
                let word_codes_reduced = self.encoder.encode_reduced(&word_codes);
                let (partial, accum) = self.evaluate_partial(&word_codes_reduced, words_reduced);
                loss += accum;
                metric.words_reduced = Some(partial);
            }
        }
        (metric, loss)
    }
}
