//! 优化问题的目标函数。
//!
//!

pub mod c3;
pub mod fingering;
pub mod general;
pub mod metric;

use crate::config::PartialWeights;
use crate::encoder::Encoder;
use crate::representation::{
    Assets, CodeSubInfo, DistributionLoss, Element, KeyMap, Label, Representation,
};
use crate::Error;
use metric::{Metric, PartialMetric};

pub struct Parameters {
    ideal_distribution: Vec<DistributionLoss>,
    pair_equivalence: Vec<f64>,
    fingering_types: Vec<Label>,
}

pub struct Objective {
    encoder: Encoder,
    parameters: Parameters,
    buckets: Vec<[Option<general::Cache>; 2]>,
}

pub type Frequencies = Vec<f64>;

pub enum PartialType {
    CharactersFull,
    CharactersShort,
    WordsFull,
    WordsShort,
}

impl PartialType {
    pub fn is_characters(&self) -> bool {
        matches!(self, Self::CharactersFull | Self::CharactersShort)
    }
}

pub trait Cache {
    fn process(
        &mut self,
        index: usize,
        frequency: u64,
        c: &mut CodeSubInfo,
        parameters: &Parameters,
    );
    fn finalize(&self, parameters: &Parameters) -> (PartialMetric, f64);
}

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
        let fingering_types = representation.transform_fingering_types();
        let config = representation
            .config
            .optimization
            .as_ref()
            .ok_or("优化配置不存在")?
            .objective
            .clone();
        let total_count = encoder.buffer.len();
        let radix = encoder.config.radix;
        let max_index = pair_equivalence.len() as u64;
        let make_cache = |x: &PartialWeights| general::Cache::new(x, radix, total_count, max_index);
        let cf = config.characters_full.as_ref().map(make_cache);
        let cs = config.characters_short.as_ref().map(make_cache);
        let wf = config.words_full.as_ref().map(make_cache);
        let ws = config.words_short.as_ref().map(make_cache);
        let buckets = vec![[cf, cs], [wf, ws]];
        let parameters = Parameters {
            ideal_distribution,
            pair_equivalence,
            fingering_types,
        };
        let objective = Self {
            encoder,
            parameters,
            buckets,
        };
        Ok(objective)
    }

    /// 计算各个部分编码的指标，然后将它们合并成一个指标输出
    pub fn evaluate(
        &mut self,
        candidate: &KeyMap,
        moved_elements: &Option<Vec<Element>>,
    ) -> (Metric, f64) {
        if let Some(moved_elements) = moved_elements {
            self.encoder.prepare(candidate, moved_elements);
        } else {
            self.encoder.init(candidate);
        }
        let parameters = &self.parameters;

        // 开始计算指标
        for (index, code_info) in self.encoder.buffer.iter_mut().enumerate() {
            let frequency = code_info.frequency;
            let bucket = if code_info.length == 1 {
                &mut self.buckets[0]
            } else {
                &mut self.buckets[1]
            };
            if let Some(cache) = &mut bucket[0] {
                cache.process(index, frequency, &mut code_info.full, parameters);
            }
            if let Some(cache) = &mut bucket[1] {
                cache.process(index, frequency, &mut code_info.short, parameters);
            }
        }

        let mut loss = 0.0;
        let mut metric = Metric {
            characters_full: None,
            words_full: None,
            characters_short: None,
            words_short: None,
        };
        for (index, bucket) in self.buckets.iter().enumerate() {
            let _ = &bucket[0].as_ref().map(|x| {
                let (partial, accum) = x.finalize(parameters);
                loss += accum;
                if index == 0 {
                    metric.characters_full = Some(partial);
                } else {
                    metric.words_full = Some(partial);
                }
            });
            let _ = &bucket[1].as_ref().map(|x| {
                let (partial, accum) = x.finalize(parameters);
                loss += accum;
                if index == 0 {
                    metric.characters_short = Some(partial);
                } else {
                    metric.words_short = Some(partial);
                }
            });
        }

        (metric, loss)
    }
}
