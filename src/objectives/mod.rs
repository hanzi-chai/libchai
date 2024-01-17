//! 优化问题的目标函数。
//!
//!

pub mod fingering;
pub mod metric;

use crate::config::ObjectiveConfig;
use crate::config::PartialWeights;
use crate::encoder::Encoder;
use crate::error::Error;
use crate::representation::Assets;
use crate::representation::Buffer;
use crate::representation::Codes;
use crate::representation::KeyMap;
use crate::representation::Occupation;
use crate::representation::Representation;
use metric::LevelMetric1;
use metric::LevelMetric2;
use metric::Metric;
use metric::PartialMetric;
use metric::TierMetric;
use std::iter::zip;

pub struct Objective {
    config: ObjectiveConfig,
    encoder: Encoder,
    character_frequencies: Frequencies,
    word_frequencies: Option<Frequencies>,
    ideal_distribution: Vec<f64>,
    pair_equivalence: Vec<f64>,
    new_pair_equivalence: Vec<f64>,
}

pub type Frequencies = Vec<f64>;

/// 目标函数
impl Objective {
    /// 通过传入配置表示、编码器和共用资源来构造一个目标函数
    pub fn new(representation: &Representation, encoder: Encoder, assets: Assets) -> Self {
        let character_frequencies: Vec<_> = encoder
            .characters
            .iter()
            .map(|x| *assets.character_frequency.get(x).unwrap_or(&0))
            .collect();
        let word_frequencies: Option<Vec<_>> = encoder.words.as_ref().map(|x| {
            x.iter()
                .map(|word| *assets.word_frequency.get(word).unwrap_or(&0))
                .collect()
        });
        let ideal_distribution =
            representation.generate_ideal_distribution(&assets.key_distribution);
        let pair_equivalence = representation.transform_pair_equivalence(&assets.pair_equivalence);
        let new_pair_equivalence =
            representation.transform_new_pair_equivalence(&assets.pair_equivalence);
        Self {
            encoder,
            config: representation.config.optimization.objective.clone(),
            character_frequencies: Self::normalize_frequencies(&character_frequencies),
            word_frequencies: word_frequencies.as_ref().map(Self::normalize_frequencies),
            ideal_distribution,
            pair_equivalence,
            new_pair_equivalence,
        }
    }

    fn normalize_frequencies(occurrences: &Vec<u64>) -> Frequencies {
        let total_occurrences: u64 = occurrences.iter().sum();
        occurrences
            .iter()
            .map(|x| *x as f64 / total_occurrences as f64)
            .collect()
    }

    fn get_distribution_distance(
        &self,
        distribution: &Vec<f64>,
        ideal_distribution: &Vec<f64>,
    ) -> f64 {
        let mut distance = 0.0;
        for (frequency, ideal_frequency) in zip(distribution, ideal_distribution) {
            if frequency > ideal_frequency {
                distance += frequency - ideal_frequency;
            }
        }
        distance
    }

    /// 计算一部分编码的指标，这里的部分可以是单字全码、单字简码、词语全码或词语简码
    pub fn evaluate_partial(
        &self,
        codes: &Codes,
        frequencies: &Frequencies,
        weights: &PartialWeights,
    ) -> (PartialMetric, f64) {
        // 初始化整体指标的变量
        let mut total_duplication = 0.0;
        let mut total_pairs = 0.0;
        // 新当量以键数为单位
        let mut total_new_keys = 0.0;
        let mut total_new_keys_equivalence = 0.0;
        let mut total_new_keys_equivalence_modified = 0.0;
        let mut total_pair_equivalence = 0.0;
        let mut total_new_pair_equivalence = 0.0;
        let mut total_levels = vec![0.0; weights.levels.as_ref().unwrap_or(&vec![]).len()];
        // 初始化分级指标的变量
        let ntier = weights.tiers.as_ref().map_or(0, |v| v.len());
        let mut tiers_duplication = vec![0; ntier];
        let mut tiers_levels: Vec<Vec<usize>> = vec![];
        if let Some(tiers) = &weights.tiers {
            for tier in tiers {
                let vec = vec![0_usize; tier.levels.as_ref().map_or(0, |v| v.len())];
                tiers_levels.push(vec);
            }
        }
        let mut distribution = vec![0.0; self.encoder.alphabet_radix];
        // 标记初始字符、结束字符的频率
        let mut chuma = vec![0 as f64; self.encoder.radix];
        let mut moma = vec![0.0 as f64; self.encoder.radix];
        for (index, ((code, duplicated), frequency)) in zip(codes, frequencies).enumerate() {
            let length = code.ilog(self.encoder.radix) as usize + 1;
            // 按键分布
            if weights.key_distribution.is_some() {
                let mut current = *code;
                while current > 0 {
                    let key = current % self.encoder.radix;
                    if key < distribution.len() {
                        distribution[key] += *frequency;
                    }
                    current /= self.encoder.radix;
                }
            }
            // 杏码式用指当量，只统计最初的1码
            if let Some(_) = weights.new_key_equivalence {
                total_new_keys_equivalence +=
                    *frequency / self.ideal_distribution[*code % self.encoder.radix];
            }
            // 杏码式用指当量改
            if let Some(_) = weights.new_key_equivalence_modified {
                //取得首末码
                let codefirst = *code % self.encoder.radix;
                let mut codelast = *code;
                while codelast > self.encoder.radix {
                    codelast /= self.encoder.radix;
                }
                chuma[codefirst] = chuma[codefirst] + *frequency;
                moma[codelast] = moma[codelast] + *frequency;
            }
            if let Some(_) = weights.pair_equivalence {
                total_pair_equivalence += self.pair_equivalence[*code] * *frequency;
                total_pairs += (length - 1) as f64 * frequency;
            }
            if let Some(_) = weights.new_pair_equivalence {
                total_new_pair_equivalence += self.new_pair_equivalence[*code] * *frequency;
                total_new_keys += length as f64 * frequency;
            }
            // 重码
            if *duplicated {
                total_duplication += frequency;
                if let Some(tiers) = &weights.tiers {
                    for (itier, tier) in tiers.iter().enumerate() {
                        let top = tier.top.unwrap_or(std::usize::MAX);
                        if index < top {
                            tiers_duplication[itier] += 1;
                        }
                    }
                }
            }
            // 简码
            if let Some(levels) = &weights.levels {
                for (ilevel, level) in levels.iter().enumerate() {
                    if level.length == length {
                        total_levels[ilevel] += frequency;
                    }
                }
            }
            // 分级指标
            if let Some(tiers) = &weights.tiers {
                for (itier, tier) in tiers.iter().enumerate() {
                    let top = tier.top.unwrap_or(std::usize::MAX);
                    if index < top {
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
        }
        if let Some(_) = weights.new_key_equivalence_modified {
            //将首末码与全局的首末码频率拼起来
            for i in 0..self.encoder.radix {
                for j in 0..self.encoder.radix {
                    total_new_keys_equivalence_modified +=
                        self.pair_equivalence[j + i * self.encoder.radix] * chuma[i] * moma[j];
                }
            }
        }
        let mut partial_metric = PartialMetric {
            tiers: None,
            key_distribution: None,
            new_key_equivalence: None,
            new_key_equivalence_modified: None,
            pair_equivalence: None,
            new_pair_equivalence: None,
            fingering: None,
            duplication: None,
            levels: None,
        };

        let mut loss = 0.0;
        if let Some(key_distribution_weight) = weights.key_distribution {
            // 首先归一化
            let total: f64 = distribution.iter().sum();
            for i in distribution.iter_mut() {
                *i /= total;
            }
            let distance = self.get_distribution_distance(&distribution, &self.ideal_distribution);
            partial_metric.key_distribution = Some(distance);
            loss += distance * key_distribution_weight;
        }
        if let Some(equivalence_weight) = weights.new_key_equivalence {
            let equivalence = total_new_keys_equivalence / total_new_keys;
            partial_metric.new_key_equivalence = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        if let Some(equivalence_weight) = weights.new_key_equivalence_modified {
            let equivalence = total_new_keys_equivalence_modified / total_new_keys;
            partial_metric.new_key_equivalence_modified = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        if let Some(equivalence_weight) = weights.pair_equivalence {
            let equivalence = total_pair_equivalence / total_pairs;
            partial_metric.pair_equivalence = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        if let Some(equivalence_weight) = weights.new_pair_equivalence {
            let equivalence = total_new_pair_equivalence / total_new_keys;
            partial_metric.new_pair_equivalence = Some(equivalence);
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

    /// 计算各个部分编码的指标，然后将它们合并成一个指标输出
    pub fn evaluate(
        &self,
        candidate: &KeyMap,
        buffer: &mut Buffer,
    ) -> Result<(Metric, f64), Error> {
        let mut loss = 0.0;
        let mut metric = Metric {
            characters: None,
            words: None,
            characters_reduced: None,
            words_reduced: None,
        };
        if let Some(characters) = &self.config.characters_full {
            let mut occupation: Occupation = vec![false; self.pair_equivalence.len()];
            self.encoder.encode_character_full(
                &candidate,
                &mut buffer.characters_full,
                &mut occupation,
            );
            let (partial, accum) = self.evaluate_partial(
                &buffer.characters_full,
                &self.character_frequencies,
                characters,
            );
            loss += accum;
            metric.characters = Some(partial);
            if let Some(characters_short) = &self.config.characters_short {
                let characters_short_buffer =
                    buffer.characters_short.as_mut().ok_or("简码模式未定义")?;
                self.encoder.encode_short(
                    &buffer.characters_full,
                    characters_short_buffer,
                    &mut occupation,
                );
                let (partial, accum) = self.evaluate_partial(
                    characters_short_buffer,
                    &self.character_frequencies,
                    characters_short,
                );
                loss += accum;
                metric.characters_reduced = Some(partial);
            }
        }
        if let Some(words) = &self.config.words_full {
            let mut occupation: Occupation = vec![false; self.encoder.get_space()];
            let words_buffer = buffer.words_full.as_mut().ok_or("组词规则未定义")?;
            self.encoder
                .encode_words_full(&candidate, words_buffer, &mut occupation);
            let (partial, accum) =
                self.evaluate_partial(&words_buffer, self.word_frequencies.as_ref().unwrap(), words);
            loss += accum;
            metric.words = Some(partial);
        }
        Ok((metric, loss))
    }
}
