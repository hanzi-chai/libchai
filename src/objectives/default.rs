
use crate::config::PartialWeights;
use crate::encoders::Encoder;
use crate::representation::{Element, KeyDistribution, KeyMap, PairEquivalence, Representation};
use crate::Error;
use super::cache::Cache;
use super::metric::Metric;
use super::{Objective, Parameters};

#[derive(Clone)]
pub struct DefaultObjective {
    parameters: Parameters,
    buckets: Vec<[Option<Cache>; 2]>,
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

/// 目标函数
impl DefaultObjective {
    /// 通过传入配置表示、编码器和共用资源来构造一个目标函数
    pub fn new(
        representation: &Representation,
        key_distribution: KeyDistribution,
        pair_equivalence: PairEquivalence,
        total_count: usize,
    ) -> Result<Self, Error> {
        let ideal_distribution =
            representation.generate_ideal_distribution(&key_distribution);
        let pair_equivalence = representation.transform_pair_equivalence(&pair_equivalence);
        let fingering_types = representation.transform_fingering_types();
        let config = representation
            .config
            .optimization
            .as_ref()
            .ok_or("优化配置不存在")?
            .objective
            .clone();
        let radix = representation.radix;
        let max_index = pair_equivalence.len() as u64;
        let make_cache = |x: &PartialWeights| Cache::new(x, radix, total_count, max_index);
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
            parameters,
            buckets,
        };
        Ok(objective)
    }
}

impl Objective for DefaultObjective {
    /// 计算各个部分编码的指标，然后将它们合并成一个指标输出
    fn evaluate<E: Encoder>(
        &mut self,
        encoder: &mut E,
        candidate: &KeyMap,
        moved_elements: &Option<Vec<Element>>,
    ) -> (Metric, f64) {
        let buffer = encoder.encode(candidate, moved_elements);
        let parameters = &self.parameters;

        // 开始计算指标
        for (index, code_info) in buffer.iter_mut().enumerate() {
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
