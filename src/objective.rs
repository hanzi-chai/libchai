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
use crate::metric::TierMetric;
use std::collections::HashSet;

pub struct Objective {
    config: ObjectiveConfig,
    assets: Assets,
    encoder: Encoder,
}

impl Objective {
    pub fn new(
        config: &Config,
        cache: &Cache,
        raw_elements: RawElements,
        assets: Assets,
    ) -> Objective {
        let elements = cache.transform_elements(&raw_elements);
        let encoder = Encoder::new(&config, elements, &assets);
        Objective {
            assets,
            encoder,
            config: config.optimization.objective.clone(),
        }
    }

    fn calculate_total_key_equivalence(&self, code: &String) -> f64 {
        let mut total = 0.0;
        for char in code.chars() {
            total += self.assets.key_equivalence.get(&char).unwrap_or(&0.0);
        }
        total
    }

    fn calculate_total_pair_equivalence(&self, code: &String) -> f64 {
        let mut total = 0.0;
        let mut it = code.chars();
        let mut this = it.next().unwrap();
        while let Some(next) = it.next() {
            total += self
                .assets
                .pair_equivalence
                .get(&(this, next))
                .unwrap_or(&0.0);
            this = next;
        }
        total
    }

    pub fn evaluate_partial(
        &self,
        codes: &Code,
        weights: &PartialMetricWeights,
    ) -> (PartialMetric, f64) {
        // 处理总数据
        let mut total_frequency = 0;
        let mut total_keys = 0;
        let mut total_pairs = 0;
        let mut total_duplication = 0;
        let mut total_keys_equivalence = 0.0;
        let mut total_pair_equivalence = 0.0;
        let mut total_levels = vec![0_usize; weights.levels.as_ref().unwrap_or(&vec![]).len()];
        // 处理分级的数据
        let ntier = weights.tiers.as_ref().map_or(0, |v| v.len());
        let mut tiers_duplication = vec![0; ntier];
        let mut tiers_levels: Vec<Vec<usize>> = vec![];
        if let Some(tiers) = &weights.tiers {
            for tier in tiers {
                let vec = vec![0_usize, tier.levels.as_ref().map_or(0, |v| v.len())];
                tiers_levels.push(vec);
            }
        }
        let mut occupied_codes: HashSet<String> = HashSet::new();
        for (index, (code, frequency)) in codes.iter().enumerate() {
            total_frequency += frequency;
            // 当量相关
            if let Some(_) = weights.key_equivalence {
                total_keys_equivalence +=
                    self.calculate_total_key_equivalence(code) * *frequency as f64;
                total_keys += code.len() * frequency;
            }
            if let Some(_) = weights.pair_equivalence {
                total_pair_equivalence +=
                    self.calculate_total_pair_equivalence(code) * *frequency as f64;
                total_pairs += (code.len() - 1) * frequency;
            }
            // 重码相关
            if let Some(_) = occupied_codes.get(code) {
                total_duplication += frequency;
                if let Some(tiers) = &weights.tiers {
                    for (itier, tier) in tiers.iter().enumerate() {
                        let top = tier.top.unwrap_or(std::usize::MAX);
                        if index <= top {
                            tiers_duplication[itier] += 1;
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
            if let Some(tiers) = &weights.tiers {
                for (itier, tier) in tiers.iter().enumerate() {
                    let top = tier.top.unwrap_or(std::usize::MAX);
                    if index <= top {
                        if let Some(levels) = &tier.levels {
                            for (ilevel, level) in levels.iter().enumerate() {
                                if level.length == code.len() {
                                    tiers_levels[itier][ilevel] += 1;
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
        let total_keys = total_keys as f64;
        let mut partial_metric = PartialMetric {
            tiers: None,
            key_equivalence: None,
            pair_equivalence: None,
            duplication: None,
            levels: None,
        };

        let mut real = 0.0;
        if let Some(equivalence_weight) = weights.key_equivalence {
            let equivalence = total_keys_equivalence / total_keys;
            partial_metric.key_equivalence = Some(equivalence);
            real += equivalence * equivalence_weight;
        }
        if let Some(equivalence_weight) = weights.pair_equivalence {
            let equivalence = total_pair_equivalence / total_pairs;
            partial_metric.pair_equivalence = Some(equivalence);
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
                levels.push(LevelMetric2 {
                    length: level.length,
                    frequency: value,
                });
            }
            partial_metric.levels = Some(levels);
        }
        if let Some(tiers_weight) = &weights.tiers {
            let mut tiers: Vec<TierMetric> = tiers_weight
                .iter()
                .map(|x| TierMetric {
                    top: x.top,
                    duplication: None,
                    levels: None,
                })
                .collect();
            for (itier, twights) in tiers_weight.iter().enumerate() {
                let total = twights.top.unwrap_or(codes.len());
                if let Some(duplication_weight) = twights.duplication {
                    let duplication = tiers_duplication[itier];
                    real += duplication as f64 / total as f64 * duplication_weight;
                    tiers[itier].duplication = Some(duplication);
                }
                if let Some(level_weight) = &twights.levels {
                    for (ilevel, level) in level_weight.iter().enumerate() {
                        real += tiers_levels[itier][ilevel] as f64 / total as f64 * level.frequency;
                    }
                    tiers[itier].levels = Some(
                        level_weight
                            .iter()
                            .enumerate()
                            .map(|(i, v)| LevelMetric1 {
                                length: v.length,
                                frequency: tiers_levels[itier][i],
                            })
                            .collect(),
                    );
                }
            }
            partial_metric.tiers = Some(tiers);
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
