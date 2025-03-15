use super::default::Parameters;
use super::metric::FingeringMetric;
use super::metric::FingeringMetricUniform;
use super::metric::LevelMetric;
use super::metric::LevelMetricUniform;
use super::metric::PartialMetric;
use super::metric::TierMetric;
use crate::config::PartialWeights;
use crate::data::最大按键组合长度;
use crate::data::编码;
use crate::data::部分编码信息;
use crate::data::键位分布损失函数;
use std::iter::zip;

#[derive(Debug, Clone)]
pub struct Cache {
    partial_weights: PartialWeights,
    total_count: usize,
    total_frequency: i64,
    total_pairs: i64,
    total_extended_pairs: i64,
    distribution: Vec<i64>,
    total_pair_equivalence: f64,
    total_extended_pair_equivalence: f64,
    total_duplication: i64,
    total_fingering: [i64; 8],
    total_levels: Vec<i64>,
    tiers_duplication: Vec<i64>,
    tiers_levels: Vec<Vec<i64>>,
    tiers_fingering: Vec<[i64; 8]>,
    max_index: u64,
    segment: u64,
    length_breakpoints: Vec<u64>,
    radix: u64,
}

impl Cache {
    #[inline(always)]
    pub fn process(
        &mut self,
        index: usize,
        frequency: u64,
        c: &mut 部分编码信息,
        parameters: &Parameters,
    ) {
        if !c.有变化 {
            return;
        }
        c.有变化 = false;
        self.accumulate(index, frequency, c.实际编码, c.选重标记, parameters, 1);
        if c.上一个实际编码 == 0 {
            return;
        }
        self.accumulate(
            index,
            frequency,
            c.上一个实际编码,
            c.上一个选重标记,
            parameters,
            -1,
        );
    }

    pub fn finalize(&self, parameters: &Parameters) -> (PartialMetric, f64) {
        let partial_weights = &self.partial_weights;
        let ideal_distribution = &parameters.ideal_distribution;
        // 初始化返回值和标量化的损失函数
        let mut partial_metric = PartialMetric {
            tiers: None,
            key_distribution: None,
            pair_equivalence: None,
            extended_pair_equivalence: None,
            fingering: None,
            duplication: None,
            levels: None,
        };
        let mut loss = 0.0;
        // 一、全局指标
        // 1. 按键分布
        if let Some(key_distribution_weight) = partial_weights.key_distribution {
            // 首先归一化
            let total: i64 = self.distribution.iter().sum();
            let distribution = self
                .distribution
                .iter()
                .map(|x| *x as f64 / total as f64)
                .collect();
            let distance = Cache::get_distribution_distance(&distribution, ideal_distribution);
            partial_metric.key_distribution = Some(distance);
            loss += distance * key_distribution_weight;
        }
        // 2. 组合当量
        if let Some(equivalence_weight) = partial_weights.pair_equivalence {
            let equivalence = self.total_pair_equivalence / self.total_pairs as f64;
            partial_metric.pair_equivalence = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        // 3. 词间当量
        if let Some(equivalence_weight) = partial_weights.extended_pair_equivalence {
            let equivalence =
                self.total_extended_pair_equivalence / self.total_extended_pairs as f64;
            partial_metric.extended_pair_equivalence = Some(equivalence);
            loss += equivalence * equivalence_weight;
        }
        // 4. 差指法
        if let Some(fingering_weight) = &partial_weights.fingering {
            let mut fingering = FingeringMetric::default();
            for (i, weight) in fingering_weight.iter().enumerate() {
                if let Some(weight) = weight {
                    fingering[i] = Some(self.total_fingering[i] as f64 / self.total_pairs as f64);
                    loss += self.total_fingering[i] as f64 * weight;
                }
            }
            partial_metric.fingering = Some(fingering);
        }
        // 5. 重码
        if let Some(duplication_weight) = partial_weights.duplication {
            let duplication = self.total_duplication as f64 / self.total_frequency as f64;
            partial_metric.duplication = Some(duplication);
            loss += duplication * duplication_weight;
        }
        // 6. 简码
        if let Some(levels_weight) = &partial_weights.levels {
            let mut levels: Vec<LevelMetric> = Vec::new();
            for (ilevel, level) in levels_weight.iter().enumerate() {
                let value = self.total_levels[ilevel] as f64 / self.total_frequency as f64;
                loss += value * level.frequency;
                levels.push(LevelMetric {
                    length: level.length,
                    frequency: value,
                });
            }
            partial_metric.levels = Some(levels);
        }
        // 二、分级指标
        if let Some(tiers_weight) = &partial_weights.tiers {
            let mut tiers: Vec<TierMetric> = tiers_weight
                .iter()
                .map(|x| TierMetric {
                    top: x.top,
                    duplication: None,
                    levels: None,
                    fingering: None,
                })
                .collect();
            for (itier, tier_weights) in tiers_weight.iter().enumerate() {
                let count = tier_weights.top.unwrap_or(self.total_count) as f64;
                // 1. 重码
                if let Some(duplication_weight) = tier_weights.duplication {
                    let duplication = self.tiers_duplication[itier];
                    loss += duplication as f64 / count * duplication_weight;
                    tiers[itier].duplication = Some(duplication as u64);
                }
                // 2. 简码
                if let Some(level_weight) = &tier_weights.levels {
                    for (ilevel, level) in level_weight.iter().enumerate() {
                        loss += self.tiers_levels[itier][ilevel] as f64 / count * level.frequency;
                    }
                    tiers[itier].levels = Some(
                        level_weight
                            .iter()
                            .enumerate()
                            .map(|(i, v)| LevelMetricUniform {
                                length: v.length,
                                frequency: self.tiers_levels[itier][i] as u64,
                            })
                            .collect(),
                    );
                }
                // 3. 差指法
                if let Some(fingering_weight) = &tier_weights.fingering {
                    let mut fingering = FingeringMetricUniform::default();
                    for (i, weight) in fingering_weight.iter().enumerate() {
                        if let Some(weight) = weight {
                            let value = self.tiers_fingering[itier][i];
                            fingering[i] = Some(value as u64);
                            loss += value as f64 / count * weight;
                        }
                    }
                    tiers[itier].fingering = Some(fingering);
                }
            }
            partial_metric.tiers = Some(tiers);
        }
        (partial_metric, loss)
    }
}

impl Cache {
    pub fn new(
        partial_weights: &PartialWeights,
        radix: u64,
        total_count: usize,
        max_index: u64,
    ) -> Self {
        let total_frequency = 0;
        let total_pairs = 0;
        let total_extended_pairs = 0;
        // 初始化全局指标的变量
        // 1. 只有加权指标，没有计数指标
        let distribution = vec![0; radix as usize];
        let total_pair_equivalence = 0.0;
        let total_extended_pair_equivalence = 0.0;
        // 2. 有加权指标，也有计数指标
        let total_duplication = 0;
        let total_fingering = [0; 8];
        let nlevel = partial_weights.levels.as_ref().map_or(0, |v| v.len());
        let total_levels = vec![0; nlevel];
        // 初始化分级指标的变量
        let ntier = partial_weights.tiers.as_ref().map_or(0, |v| v.len());
        let tiers_duplication = vec![0; ntier];
        let mut tiers_levels = vec![];
        if let Some(tiers) = &partial_weights.tiers {
            for tier in tiers {
                let vec = vec![0; tier.levels.as_ref().map_or(0, |v| v.len())];
                tiers_levels.push(vec);
            }
        }
        let tiers_fingering = vec![[0; 8]; ntier];
        let segment = radix.pow((最大按键组合长度 - 1) as u32);
        let length_breakpoints: Vec<u64> = (0..=8).map(|x| radix.pow(x)).collect();

        Self {
            partial_weights: partial_weights.clone(),
            total_count,
            total_frequency,
            total_pairs,
            total_extended_pairs,
            distribution,
            total_pair_equivalence,
            total_extended_pair_equivalence,
            total_duplication,
            total_fingering,
            total_levels,
            tiers_duplication,
            tiers_levels,
            tiers_fingering,
            max_index,
            segment,
            length_breakpoints,
            radix,
        }
    }

    /// 用指分布偏差
    /// 计算按键使用率与理想使用率之间的偏差。对于每个按键，偏差是实际频率与理想频率之间的差值乘以一个惩罚系数。用户可以根据自己的喜好自定义理想频率和惩罚系数。
    fn get_distribution_distance(
        distribution: &Vec<f64>,
        ideal_distribution: &Vec<键位分布损失函数>,
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

    #[inline(always)]
    pub fn accumulate(
        &mut self,
        index: usize,
        frequency: u64,
        code: 编码,
        duplicate: bool,
        parameters: &Parameters,
        sign: i64,
    ) {
        let frequency = frequency as i64 * sign;
        let radix = self.radix;
        let length = self
            .length_breakpoints
            .iter()
            .position(|&x| code < x)
            .unwrap() as u64;
        self.total_frequency += frequency;
        self.total_pairs += (length - 1) as i64 * frequency;
        let partial_weights = &self.partial_weights;
        // 一、全局指标
        // 1. 按键分布
        if partial_weights.key_distribution.is_some() {
            let mut current = code;
            while current > 0 {
                let key = current % self.radix;
                if let Some(x) = self.distribution.get_mut(key as usize) {
                    *x += frequency;
                }
                current /= self.radix;
            }
        }
        // 2. 组合当量
        if partial_weights.pair_equivalence.is_some() {
            let mut code = code;
            while code > self.radix {
                let partial_code = (code % self.max_index) as usize;
                self.total_pair_equivalence +=
                    parameters.pair_equivalence[partial_code] * frequency as f64;
                code /= self.segment;
            }
        }
        // 4. 差指法
        if let Some(fingering) = &partial_weights.fingering {
            let mut code = code;
            while code > radix {
                let label = parameters.fingering_types[(code % self.max_index) as usize];
                for (i, weight) in fingering.iter().enumerate() {
                    if weight.is_some() {
                        self.total_fingering[i] += frequency * label[i] as i64;
                    }
                }
                code /= self.segment;
            }
        }
        // 5. 重码
        if duplicate {
            self.total_duplication += frequency;
        }
        // 6. 简码
        if let Some(levels) = &partial_weights.levels {
            for (ilevel, level) in levels.iter().enumerate() {
                if level.length == length as usize {
                    self.total_levels[ilevel] += frequency;
                }
            }
        }
        // 二、分级指标
        if let Some(tiers) = &partial_weights.tiers {
            for (itier, tier) in tiers.iter().enumerate() {
                if index >= tier.top.unwrap_or(self.total_count) {
                    continue;
                }
                // 1. 重码
                if duplicate {
                    self.tiers_duplication[itier] += sign;
                }
                // 2. 简码
                if let Some(levels) = &tier.levels {
                    for (ilevel, level) in levels.iter().enumerate() {
                        if level.length == length as usize {
                            self.tiers_levels[itier][ilevel] += sign;
                        }
                    }
                }
                // 3. 差指法
                if let Some(fingering) = &tier.fingering {
                    let mut code = code;
                    while code > radix {
                        let label = parameters.fingering_types[(code % self.max_index) as usize];
                        for (i, weight) in fingering.iter().enumerate() {
                            if weight.is_some() {
                                self.tiers_fingering[itier][i] += sign * label[i] as i64;
                            }
                        }
                        code /= self.segment;
                    }
                }
            }
        }
    }
}
