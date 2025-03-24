use super::cache::Cache;
use super::metric::默认指标;
use super::目标函数;
use crate::config::PartialWeights;
use crate::data::{
    元素映射, 数据, 正则化, 用指标记, 编码信息, 键位分布损失函数
};
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
    pub 正则化: 正则化,
    pub 正则化强度: f64,
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
        let 正则化 = 数据.正则化.clone();
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
            正则化,
            正则化强度: config.regularization.and_then(|x| x.strength).unwrap_or(1.0)
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
    fn 计算(
        &mut self, 编码结果: &mut [编码信息], 映射: &元素映射
    ) -> (默认指标, f64) {
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
        let mut metric = 默认指标 {
            characters_full: None,
            words_full: None,
            characters_short: None,
            words_short: None,
            memory: None,
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

        if !parameters.正则化.is_empty() {
            let mut 记忆量 = 映射.len() as f64;
            for (元素, 键) in 映射.iter().enumerate() {
                if 元素 as u64 == *键 {
                    记忆量 -= 1.0;
                    continue;
                }
                if let Some(归并列表) = parameters.正则化.get(&元素) {
                    let mut 最大亲和度 = 0.0;
                    for (目标元素, 亲和度) in 归并列表.iter() {
                        if 映射[*目标元素] == *键 {
                            最大亲和度 = 亲和度.max(最大亲和度);
                        }
                    }
                    记忆量 -= 最大亲和度;
                }
            }
            metric.memory = Some(记忆量);
            let 归一化记忆量 = 记忆量 / 映射.len() as f64;
            loss += 归一化记忆量 * parameters.正则化强度;
        }
        (metric, loss)
    }
}
