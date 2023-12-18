use crate::cli::Assets;
use crate::config::ObjectiveConfig;
use crate::config::PartialMetricWeights;
use crate::encoder::Encoder;
use crate::metric::LevelMetric1;
use crate::metric::LevelMetric2;
use crate::metric::Metric;
use crate::metric::PartialMetric;
use crate::metric::TierMetric;
use crate::representation::Buffer;
use crate::representation::Representation;
use crate::representation::Codes;
use crate::representation::KeyMap;
use std::iter::zip;

#[derive(Debug)]
pub struct EncodeExport {
    pub character_list: Vec<char>,
    pub characters: Option<Codes>,
    pub characters_reduced: Option<Codes>,
    pub word_list: Vec<String>,
    pub words: Option<Codes>,
    pub words_reduced: Option<Codes>,
}

pub struct Objective {
    config: ObjectiveConfig,
    encoder: Encoder,
    character_frequencies: Frequencies,
    word_frequencies: Frequencies,
    key_equivalence: Vec<f64>,
    pair_equivalence: Vec<f64>
}

pub type Frequencies = Vec<f64>;

impl Objective {
    pub fn new(representation: &Representation, encoder: Encoder, assets: Assets) -> Self {
        let character_frequencies: Vec<_> = encoder.characters.iter().map(|x| *assets.character_frequency.get(x).unwrap_or(&0)).collect();
        let word_frequencies: Vec<_> = encoder.words.iter().map(|x| *assets.word_frequency.get(x).unwrap_or(&0)).collect();
        let key_equivalence = representation.transform_key_equivalence(&assets.key_equivalence);
        let pair_equivalence = representation.transform_pair_equivalence(&assets.pair_equivalence);
        Self {
            encoder,
            config: representation.config.optimization.objective.clone(),
            character_frequencies: Self::normalize_frequencies(&character_frequencies),
            word_frequencies: Self::normalize_frequencies(&word_frequencies),
            key_equivalence,
            pair_equivalence,
        }
    }

    fn normalize_frequencies(occurrences: &Vec<u64>) -> Frequencies {
        let total_occurrences: u64 = occurrences.iter().sum();
        occurrences
            .iter()
            .map(|x| *x as f64 / total_occurrences as f64)
            .collect()
    }

    pub fn evaluate_partial(
        &self,
        codes: &Codes,
        frequencies: &Frequencies,
        weights: &PartialMetricWeights
    ) -> (PartialMetric, f64) {
        // 处理总数据
        let mut total_duplication = 0.0;
        let mut total_keys = 0.0;
        let mut total_pairs = 0.0;
        let mut total_keys_equivalence = 0.0;
        let mut total_pair_equivalence = 0.0;
        let mut total_levels = vec![0.0; weights.levels.as_ref().unwrap_or(&vec![]).len()];
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
        // 清空
        let mut occupation = vec![false; self.key_equivalence.len()];
        for (index, (code, frequency)) in zip(codes, frequencies).enumerate() {
            let length = code.ilog(self.encoder.radix) as usize + 1;
            // 当量相关
            if let Some(_) = weights.key_equivalence {
                total_keys_equivalence +=
                    self.key_equivalence[*code] * *frequency;
                total_keys += length as f64 * frequency;
            }
            if let Some(_) = weights.pair_equivalence {
                total_pair_equivalence +=
                    self.pair_equivalence[*code] * *frequency;
                total_pairs += (length - 1) as f64 * frequency;
            }
            // 重码相关
            if occupation[*code] {
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
                    if level.length == length {
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
                                if level.length == length {
                                    tiers_levels[itier][ilevel] += 1;
                                }
                            }
                        }
                    }
                }
            }
            occupation[*code] = true;
        }
        let mut partial_metric = PartialMetric {
            tiers: None,
            key_equivalence: None,
            pair_equivalence: None,
            fingering: None,
            duplication: None,
            levels: None,
        };

        let mut loss = 0.0;
        if let Some(equivalence_weight) = weights.key_equivalence {
            let equivalence = total_keys_equivalence / total_keys;
            partial_metric.key_equivalence = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        if let Some(equivalence_weight) = weights.pair_equivalence {
            let equivalence = total_pair_equivalence / total_pairs;
            partial_metric.pair_equivalence = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        if let Some(duplication_weight) = weights.duplication {
            partial_metric.duplication = Some(total_duplication);
            loss += total_duplication * duplication_weight;
        }
        if let Some(levels_weight) = &weights.levels {
            let mut levels: Vec<LevelMetric2> = Vec::new();
            for (ilevel, level) in levels_weight.iter().enumerate() {
                let value = total_levels[ilevel];
                loss += value * level.frequency;
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
                    loss += duplication as f64 / total as f64 * duplication_weight;
                    tiers[itier].duplication = Some(duplication);
                }
                if let Some(level_weight) = &twights.levels {
                    for (ilevel, level) in level_weight.iter().enumerate() {
                        loss += tiers_levels[itier][ilevel] as f64 / total as f64 * level.frequency;
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
        return (partial_metric, loss);
    }

    pub fn evaluate(&self, candidate: &KeyMap, buffer: &mut Buffer) -> (Metric, f64) {
        let mut loss = 0.0;
        let mut metric = Metric {
            characters: None,
            words: None,
            characters_reduced: None,
            words_reduced: None,
        };
        if let Some(characters) = &self.config.characters {
            self.encoder
                .encode_character_full(&candidate, &mut buffer.characters);
            let (partial, accum) = self.evaluate_partial(
                &buffer.characters,
                &self.character_frequencies,
                characters,
            );
            loss += accum;
            metric.characters = Some(partial);
            if let Some(character_reduced) = &self.config.characters_reduced {
                self.encoder.encode_reduced(
                    &buffer.characters,
                    &mut buffer.characters_reduced,
                );
                let (partial, accum) = self.evaluate_partial(
                    &buffer.characters_reduced,
                    &self.character_frequencies,
                    character_reduced,
                );
                loss += accum;
                metric.characters_reduced = Some(partial);
            }
        }
        if let Some(words) = &self.config.words {
            self.encoder
                .encode_words_full(&candidate, &mut buffer.words);
            let (partial, accum) = self.evaluate_partial(
                &buffer.words,
                &self.word_frequencies,
                words,
            );
            loss += accum;
            metric.words = Some(partial);
            if let Some(words_reduced) = &self.config.words_reduced {
                self.encoder.encode_reduced(
                    &buffer.words,
                    &mut buffer.words_reduced,
                );
                let (partial, accum) = self.evaluate_partial(
                    &buffer.words_reduced,
                    &self.word_frequencies,
                    words_reduced,
                );
                loss += accum;
                metric.words_reduced = Some(partial);
            }
        }
        (metric, loss)
    }

    pub fn export_codes(&self, buffer: &Buffer) -> EncodeExport {
        EncodeExport {
            character_list: self.encoder.characters.clone(),
            characters: self.config.characters.as_ref().map(|_| buffer.characters.clone()),
            characters_reduced: self.config.characters_reduced.as_ref().map(|_| buffer.characters_reduced.clone()),
            word_list: self.encoder.words.clone(),
            words: self.config.words.as_ref().map(|_| buffer.words.clone()),
            words_reduced: self.config.words_reduced.as_ref().map(|_| buffer.words_reduced.clone()),
        }
    }
}
