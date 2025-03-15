use super::cache::Cache;
use super::metric::Metric;
use super::目标函数;
use crate::config::PartialWeights;
use crate::data::{数据, 用指标记, 编码信息, 键位分布损失函数};
use crate::错误;

#[derive(Clone)]
pub struct 默认目标函数 {
    parameters: Parameters,
    buckets: Vec<[Option<Cache>; 2]>,
}

#[derive(Clone)]
pub struct Parameters {
    pub ideal_distribution: Vec<键位分布损失函数>,
    pub pair_equivalence: Vec<f64>,
    pub fingering_types: Vec<用指标记>,
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
impl 默认目标函数 {
    /// 通过传入配置表示、编码器和共用资源来构造一个目标函数
    pub fn 新建(数据: &数据) -> Result<Self, 错误> {
        let ideal_distribution = 数据.键位分布信息.clone();
        let pair_equivalence = 数据.当量信息.clone();
        let fingering_types = 数据.预处理指法标记();
        let config = 数据
            .配置
            .optimization
            .as_ref()
            .ok_or("优化配置不存在")?
            .objective
            .clone();
        let radix = 数据.进制;
        let max_index = pair_equivalence.len() as u64;
        let make_cache = |x: &PartialWeights| Cache::new(x, radix, 数据.词列表.len(), max_index);
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

impl 目标函数 for 默认目标函数 {
    /// 计算各个部分编码的指标，然后将它们合并成一个指标输出
    fn 计算(&mut self, 编码结果: &mut [编码信息]) -> (Metric, f64) {
        let parameters = &self.parameters;

        // 开始计算指标
        for (index, code_info) in 编码结果.iter_mut().enumerate() {
            let frequency = code_info.频率;
            let bucket = if code_info.词长 == 1 {
                &mut self.buckets[0]
            } else {
                &mut self.buckets[1]
            };
            if let Some(cache) = &mut bucket[0] {
                cache.process(index, frequency, &mut code_info.全码, parameters);
            }
            if let Some(cache) = &mut bucket[1] {
                cache.process(index, frequency, &mut code_info.简码, parameters);
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
