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
use crate::representation::CodeInfo;
use crate::representation::Codes;
use crate::representation::DistributionLoss;
use crate::representation::KeyMap;
use crate::representation::Label;
use crate::representation::Occupation;
use crate::representation::Representation;
use crate::representation::MAX_COMBINATION_LENGTH;
use metric::LevelMetric1;
use metric::LevelMetric2;
use metric::Metric;
use metric::PartialMetric;
use metric::TierMetric;
use std::iter::zip;

use self::metric::FingeringMetric;

pub struct Objective {
    config: ObjectiveConfig,
    encoder: Encoder,
    ideal_distribution: Vec<DistributionLoss>,
    pair_equivalence: Vec<f64>,
    new_pair_equivalence: Vec<f64>,
    fingering_types: Vec<Label>,
}

pub type Frequencies = Vec<f64>;

/// 目标函数
impl Objective {
    /// 通过传入配置表示、编码器和共用资源来构造一个目标函数
    pub fn new(
        representation: &Representation,
        encoder: Encoder,
        assets: Assets,
    ) -> Result<Self, Error> {
        let ideal_distribution =
            representation.generate_ideal_distribution(&assets.key_distribution);
        let pair_equivalence = representation.transform_pair_equivalence(&assets.pair_equivalence);
        let new_pair_equivalence =
            representation.transform_new_pair_equivalence(&assets.pair_equivalence);
        let fingering_types = representation.transform_fingering_types();
        let config = representation
            .config
            .optimization
            .as_ref()
            .ok_or("优化配置不存在")?;
        let objective = Self {
            encoder,
            config: config.objective.clone(),
            ideal_distribution,
            pair_equivalence,
            new_pair_equivalence,
            fingering_types,
        };
        Ok(objective)
    }

    /// 用指分佈偏差
    // This is a metric indicating whether the distribution of the
    // keys is ergonomic. It calculates the deviation of the empirical
    // distribution of frequencies from the ideal one. In an ideal
    // situation, the frequency of keys should follows the following
    // rule of thumbs:

    // - The middle row should be used more often.
    // - The middle and index fingers should be used more often.
    // - The keys covered by the index fingers should not be used too
    // frequently to avoid tiredness of index fingers.
    // - The keys covered by right-hand fingers should be used more
    // than the corresponding keys covered by left-hand fingers.

    // Users can adjust the ideal frequencies by via an input mapping
    // table.
    fn get_distribution_distance(
        &self,
        distribution: &Vec<f64>,
        ideal_distribution: &Vec<DistributionLoss>,
    ) -> f64 {
        let mut distance = 0.0;
        for (frequency, loss) in zip(distribution, ideal_distribution) {
            let diff = frequency - loss.ideal;
            if diff > 0.0 {
                distance += loss.gt_penalty * diff;
            } else {
                distance -= loss.lt_penalty * diff;
            }
        }
        distance
    }

    /// 计算一部分编码的指标，这里的部分可以是一字全码、一字简码、多字全码或多字简码
    pub fn evaluate_partial(
        &self,
        codes: &Codes,
        weights: &PartialWeights,
        group: bool,
    ) -> (PartialMetric, f64) {
        let mut total_frequency = 0;
        // 初始化整体指标的变量
        let mut total_duplication = 0;
        let mut total_pairs = 0;
        // 新当量以键数为单位
        let mut total_new_keys = 0;
        let mut total_new_keys_equivalence = 0.0;
        let mut total_new_keys_equivalence_modified = 0.0;
        let mut total_pair_equivalence = 0.0;
        let mut total_new_pair_equivalence = 0.0;
        // 词间当量
        let mut total_extended_pairs = 0;
        let mut total_extended_pair_equivalence = 0.0;
        let mut total_labels = [0_u64; 8];
        let mut total_levels = vec![0; weights.levels.as_ref().unwrap_or(&vec![]).len()];
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
        let mut distribution = vec![0_u64; self.encoder.radix];
        // 标记初始字符、结束字符的频率
        let mut chuma = vec![0_u64; self.encoder.radix];
        let mut moma = vec![0_u64; self.encoder.radix];
        let max_index = self.pair_equivalence.len();
        let segment = self.encoder.radix.pow((MAX_COMBINATION_LENGTH - 1) as u32);
        for (index, code_info) in codes.iter().enumerate() {
            let CodeInfo {
                code,
                rank,
                frequency,
                single,
            } = code_info.clone();
            if group != single || code == 0 {
                continue;
            }
            total_frequency += frequency;
            let code = self.encoder.get_actual_code(code, rank);
            let length = code.ilog(self.encoder.radix) as u64 + 1;
            // 按键分布
            if weights.key_distribution.is_some() {
                let mut current = code;
                while current > 0 {
                    let key = current % self.encoder.radix;
                    if key < distribution.len() {
                        distribution[key] += frequency;
                    }
                    current /= self.encoder.radix;
                }
            }
            // 按键分布：杏码式用指当量，只统计最初的1码
            if let Some(_) = weights.new_key_equivalence {
                total_new_keys_equivalence +=
                    frequency as f64 / self.ideal_distribution[code % self.encoder.radix].ideal;
            }
            // 按键分布：杏码式用指当量改
            if let Some(_) = weights.new_key_equivalence_modified {
                //取得首末码
                let codefirst = code % self.encoder.radix;
                let mut codelast = code;
                while codelast > self.encoder.radix {
                    codelast /= self.encoder.radix;
                }
                chuma[codefirst] = chuma[codefirst] + frequency;
                moma[codelast] = moma[codelast] + frequency;
            }
            // 组合当量
            if let Some(_) = weights.pair_equivalence {
                let mut code = code;
                while code > self.encoder.radix {
                    total_pair_equivalence +=
                        self.pair_equivalence[code % max_index] * frequency as f64;
                    code /= segment;
                }
                total_pairs += (length - 1) * frequency;
            }
            if let Some(_) = weights.new_pair_equivalence {
                let mut code = code;
                while code > self.encoder.radix {
                    total_new_pair_equivalence +=
                        self.new_pair_equivalence[code % max_index] * frequency as f64;
                    code /= segment;
                }
                total_new_keys += length * frequency;
            }
            // 词间当量
            if let Some(_) = weights.extended_pair_equivalence {
                let transitions = &self.encoder.transition_matrix[index];
                let last_char = code / self.encoder.radix.pow(length as u32 - 1);
                for (i, weight) in transitions {
                    let next_char = codes[*i].code % self.encoder.radix;
                    let combination = last_char + next_char * self.encoder.radix;
                    let equivalence = self.pair_equivalence[combination];
                    total_extended_pair_equivalence += equivalence * *weight as f64;
                    total_extended_pairs += *weight;
                }
            }
            // 差指法统计
            if let Some(fingering) = &weights.fingering {
                let mut code = code;
                while code > self.encoder.radix {
                    let label = self.fingering_types[code % max_index];
                    for (i, weight) in fingering.iter().enumerate() {
                        if let Some(_) = weight {
                            total_labels[i] += frequency * label[i] as u64;
                        }
                    }
                    code /= segment;
                }
            }
            // 重码
            if rank > 0 {
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
                    if level.length == length as usize {
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
                                if level.length == length as usize {
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
                    total_new_keys_equivalence_modified += self.pair_equivalence
                        [j + i * self.encoder.radix]
                        * (chuma[i] * moma[j]) as f64;
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
            extended_pair_equivalence: None,
            fingering: None,
            duplication: None,
            levels: None,
        };

        let mut loss = 0.0;
        if let Some(key_distribution_weight) = weights.key_distribution {
            // 首先归一化
            let total: u64 = distribution.iter().sum();
            let distribution = distribution
                .iter()
                .map(|x| *x as f64 / total as f64)
                .collect();
            let distance = self.get_distribution_distance(&distribution, &self.ideal_distribution);
            partial_metric.key_distribution = Some(distance);
            loss += distance * key_distribution_weight;
        }
        if let Some(equivalence_weight) = weights.new_key_equivalence {
            let equivalence = total_new_keys_equivalence / total_new_keys as f64;
            partial_metric.new_key_equivalence = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        if let Some(equivalence_weight) = weights.new_key_equivalence_modified {
            let equivalence = total_new_keys_equivalence_modified / total_new_keys as f64;
            partial_metric.new_key_equivalence_modified = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        if let Some(equivalence_weight) = weights.pair_equivalence {
            let equivalence = total_pair_equivalence / total_pairs as f64;
            partial_metric.pair_equivalence = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        if let Some(equivalence_weight) = weights.new_pair_equivalence {
            let equivalence = total_new_pair_equivalence / total_new_keys as f64;
            partial_metric.new_pair_equivalence = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        if let Some(equivalence_weight) = weights.extended_pair_equivalence {
            let equivalence = total_extended_pair_equivalence / total_extended_pairs as f64;
            partial_metric.extended_pair_equivalence = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        if let Some(fingering_weight) = &weights.fingering {
            let mut fingering = FingeringMetric::default();
            for (i, weight) in fingering_weight.iter().enumerate() {
                if let Some(weight) = weight {
                    fingering[i] = Some(total_labels[i] as f64 / total_pairs as f64);
                    loss += total_labels[i] as f64 * weight;
                }
            }
            partial_metric.fingering = Some(fingering);
        }
        if let Some(duplication_weight) = weights.duplication {
            let duplication = total_duplication as f64 / total_frequency as f64;
            partial_metric.duplication = Some(duplication);
            loss += duplication * duplication_weight;
        }
        if let Some(levels_weight) = &weights.levels {
            let mut levels: Vec<LevelMetric2> = Vec::new();
            for (ilevel, level) in levels_weight.iter().enumerate() {
                let value = total_levels[ilevel] as f64 / total_frequency as f64;
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
            characters_full: None,
            words_full: None,
            characters_short: None,
            words_short: None,
        };
        let mut full_occupation = Occupation::new(self.pair_equivalence.len());
        let mut short_occupation = Occupation::new(self.pair_equivalence.len());
        self.encoder
            .encode_full(candidate, buffer, &mut full_occupation);
        self.encoder
            .encode_short(buffer, &mut full_occupation, &mut short_occupation);
        // 一字全码
        if let Some(characters_weight) = &self.config.characters_full {
            let (partial, accum) = self.evaluate_partial(&buffer.full, characters_weight, true);
            loss += accum;
            metric.characters_full = Some(partial);
        }
        // 一字简码
        if let Some(characters_short) = &self.config.characters_short {
            let (partial, accum) = self.evaluate_partial(&buffer.short, characters_short, true);
            loss += accum;
            metric.characters_short = Some(partial);
        }
        // 多字全码
        if let Some(words_weight) = &self.config.words_full {
            let (partial, accum) = self.evaluate_partial(&buffer.full, words_weight, false);
            loss += accum;
            metric.words_full = Some(partial);
        }
        // 多字简码
        if let Some(words_short) = &self.config.words_short {
            let (partial, accum) = self.evaluate_partial(&buffer.short, words_short, false);
            loss += accum;
            metric.words_short = Some(partial);
        }
        Ok((metric, loss))
    }
}
