use super::metric::PartialMetric;
use super::metric::TierMetric;
use super::Parameters;
use crate::config::PartialWeights;
use crate::representation::Code;
use crate::representation::CodeSubInfo;

#[derive(Debug)]
pub struct Cache {
    duplication: i64,
}

impl super::Cache for Cache {
    #[inline(always)]
    fn process(
        &mut self,
        index: usize,
        frequency: u64,
        c: &mut CodeSubInfo,
        parameters: &Parameters,
    ) {
        if !c.has_changed {
            return;
        }
        c.has_changed = false;
        self.accumulate(index, frequency, c.actual, c.duplicate, parameters, 1);
        if c.p_actual == 0 {
            return;
        }
        self.accumulate(index, frequency, c.p_actual, c.p_duplicate, parameters, -1);
    }

    fn finalize(&self, _: &Parameters) -> (PartialMetric, f64) {
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
        let loss = self.duplication as f64;
        let tiers: Vec<TierMetric> = vec![TierMetric {
            top: None,
            duplication: Some(self.duplication as u64),
            levels: None,
            fingering: None,
        }];
        partial_metric.tiers = Some(tiers);
        (partial_metric, loss)
    }
}

impl Cache {
    pub fn new(
        _partial_weights: &PartialWeights,
        _radix: u64,
        _total_count: usize,
        _max_index: u64,
    ) -> Self {
        Self { duplication: 0 }
    }

    #[inline(always)]
    pub fn accumulate(
        &mut self,
        _index: usize,
        _frequency: u64,
        _code: Code,
        duplicate: bool,
        _parameters: &Parameters,
        sign: i64,
    ) {
        if duplicate {
            self.duplication += sign;
        }
    }
}
