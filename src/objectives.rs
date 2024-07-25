//! 优化问题的目标函数。
//!
//!

pub mod cache;
pub mod fingering;
pub mod metric;

use crate::config::ObjectiveConfig;
use crate::encoder::Encoder;
use crate::error::Error;
use crate::representation::Assets;
use crate::representation::DistributionLoss;
use crate::representation::KeyMap;
use crate::representation::Label;
use crate::representation::Representation;
use cache::Cache;
use metric::Metric;

pub struct Objective {
    config: ObjectiveConfig,
    encoder: Encoder,
    ideal_distribution: Vec<DistributionLoss>,
    pair_equivalence: Vec<f64>,
    fingering_types: Vec<Label>,
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
            .ok_or("优化配置不存在")?;
        let objective = Self {
            encoder,
            config: config.objective.clone(),
            ideal_distribution,
            pair_equivalence,
            fingering_types,
        };
        Ok(objective)
    }

    /// 计算各个部分编码的指标，然后将它们合并成一个指标输出
    pub fn evaluate(&mut self, candidate: &KeyMap) -> Result<(Metric, f64), Error> {
        let mut loss = 0.0;
        let mut metric = Metric {
            characters_full: None,
            words_full: None,
            characters_short: None,
            words_short: None,
        };
        self.encoder.prepare(candidate);
        let total_count = self.encoder.buffer.len();
        let radix = self.encoder.config.radix;
        let max_index = self.pair_equivalence.len() as u64;
        let mut cf_cache = self
            .config
            .characters_full
            .as_ref()
            .map(|x| Cache::new(&x, radix, total_count, max_index));
        let mut cs_cache = self
            .config
            .characters_short
            .as_ref()
            .map(|x| Cache::new(&x, radix, total_count, max_index));
        let mut wf_cache = self
            .config
            .words_full
            .as_ref()
            .map(|x| Cache::new(&x, radix, total_count, max_index));
        let mut ws_cache = self
            .config
            .words_short
            .as_ref()
            .map(|x| Cache::new(&x, radix, total_count, max_index));

        // 开始计算指标
        for (index, code_info) in self.encoder.buffer.iter().enumerate() {
            if code_info.length == 1 {
                cf_cache.as_mut().map(|x| x.accumulate(index, code_info.frequency, code_info.full, &self, &self.config.characters_full.as_ref().unwrap()));
                cs_cache.as_mut().map(|x| x.accumulate(index, code_info.frequency, code_info.short, &self, &self.config.characters_short.as_ref().unwrap()));
            } else {
                wf_cache.as_mut().map(|x| x.accumulate(index, code_info.frequency, code_info.full, &self, &self.config.words_full.as_ref().unwrap()));
                ws_cache.as_mut().map(|x| x.accumulate(index, code_info.frequency, code_info.short, &self, &self.config.words_short.as_ref().unwrap()));
            }
        }

        // 一字全码
        if let Some(characters_weight) = &self.config.characters_full {
            let (partial, accum) = cf_cache
                .unwrap()
                .finalize(characters_weight, &self.ideal_distribution);
            loss += accum;
            metric.characters_full = Some(partial);
        }
        // 一字简码
        if let Some(characters_short) = &self.config.characters_short {
            let (partial, accum) = cs_cache
                .unwrap()
                .finalize(characters_short, &self.ideal_distribution);
            loss += accum;
            metric.characters_short = Some(partial);
        }
        // 多字全码
        if let Some(words_weight) = &self.config.words_full {
            let (partial, accum) = wf_cache
                .unwrap()
                .finalize(words_weight, &self.ideal_distribution);
            loss += accum;
            metric.words_full = Some(partial);
        }
        // 多字简码
        if let Some(words_short) = &self.config.words_short {
            let (partial, accum) = ws_cache
                .unwrap()
                .finalize(words_short, &self.ideal_distribution);
            loss += accum;
            metric.words_short = Some(partial);
        }
        Ok((metric, loss))
    }
}
