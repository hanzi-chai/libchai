//! 退火算法

use super::优化结果;
use crate::contexts::上下文;
use crate::interfaces::{消息, 界面};
use crate::objectives::目标函数;
use crate::operators::{default::变异配置, 变异};
use crate::optimizers::解特征;
use rand::random;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use web_time::Instant;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// 退火算法的参数，包括最高温、最低温、步数
pub struct 降温时间表 {
    pub t_max: f64,
    pub t_min: f64,
    pub steps: usize,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 退火方法 {
    pub parameters: Option<降温时间表>,
    pub report_after: Option<f64>,
    pub search_method: Option<变异配置>,
    pub update_interval: Option<usize>,
}

impl 退火方法 {
    pub fn 优化<O: 目标函数, F: 变异<解类型 = O::解类型>, C: 上下文<解类型 = O::解类型>>(
        &self,
        初始解: &O::解类型,
        目标函数: &mut O,
        操作: &mut F,
        上下文: &C,
        界面: &dyn 界面,
    ) -> 优化结果<O> {
        let 降温时间表 = self
            .parameters
            .unwrap_or_else(|| self.调参(初始解, 目标函数, 操作, 界面));
        self.solve_with(初始解, 目标函数, 操作, 上下文, 降温时间表, 界面)
    }

    /// 退火算法求解的主函数
    fn solve_with<O: 目标函数, F: 变异<解类型 = O::解类型>, C: 上下文<解类型 = O::解类型>>(
        &self,
        初始解: &O::解类型,
        目标函数: &mut O,
        操作: &mut F,
        上下文: &C,
        降温时间表: 降温时间表,
        界面: &dyn 界面,
    ) -> 优化结果<O> {
        let mut 最优解 = 初始解.clone();
        let mut 最优指标 = 目标函数.计算(&最优解, &None);
        let mut 当前解 = 最优解.clone();
        let mut 当前指标 = 最优指标.clone();
        let 降温时间表 {
            t_max: 最高温,
            t_min: 最低温,
            steps: 总步数,
        } = 降温时间表;
        let 开始时间 = Instant::now();
        let 更新频率 = self.update_interval.unwrap_or(1000);
        let mut 上一个变化 = None;

        for 步骤 in 0..总步数 {
            // 等比级数降温：每一步的温度都是上一步的温度乘以一个固定倍数
            let 进度 = 步骤 as f64 / 总步数 as f64;
            let 温度 = 最高温 * (最低温 / 最高温).powf(进度);
            // 每过一定的步数，报告当前状态和计算速度
            if 步骤 % 更新频率 == 0 || 步骤 == 总步数 - 1 {
                界面.发送(消息::Progress {
                    steps: 步骤,
                    temperature: 温度,
                    metric: format!("{}", 当前指标.0),
                });
                if 步骤 == 更新频率 {
                    let elapsed = 开始时间.elapsed().as_micros() as u64 / 更新频率 as u64;
                    界面.发送(消息::Elapsed { time: elapsed });
                }
            }
            // 生成一个新解
            let mut 尝试解 = 当前解.clone();
            let 尝试解变化 = 操作.变异(&mut 尝试解);
            let 变化 = if let Some(上一个变化) = 上一个变化 {
                F::解类型::除法(&上一个变化, &尝试解变化)
            } else {
                尝试解变化.clone()
            };
            let 尝试指标 = 目标函数.计算(&尝试解, &Some(变化));
            // 如果满足退火条件，接受新解
            let 改进 = 尝试指标.1 - 当前指标.1;
            if 改进 < 0.0 || (random::<f64>() < (-改进 / 温度).exp()) {
                当前解.clone_from(&尝试解);
                当前指标 = 尝试指标;
                上一个变化 = None;
            } else {
                上一个变化 = Some(尝试解变化);
            }
            // 如果当前解优于目前的最优解，更新最优解
            if 当前指标.1 < 最优指标.1 {
                最优指标 = 当前指标.clone();
                最优解.clone_from(&当前解);
                let 是否保存 = 进度 > self.report_after.unwrap_or(0.9);
                界面.发送(消息::BetterSolution {
                    metric: format!("{}", 最优指标.0),
                    config: 上下文.序列化(&最优解),
                    save: 是否保存,
                })
            }
        }
        界面.发送(消息::BetterSolution {
            metric: format!("{}", 最优指标.0),
            config: 上下文.序列化(&最优解),
            save: true,
        });
        优化结果 {
            映射: 最优解,
            指标: 最优指标.0.clone(),
            分数: 最优指标.1,
        }
    }

    fn trial_run<O: 目标函数, F: 变异<解类型 = O::解类型>>(
        &self,
        目标函数: &mut O,
        操作: &mut F,
        from: O::解类型,
        temperature: f64,
        steps: usize,
    ) -> (O::解类型, f64, f64) {
        let mut candidate = from.clone();
        let (_, mut energy) = 目标函数.计算(&candidate, &None);
        let mut accepts = 0;
        let mut improves = 0;

        for _ in 0..steps {
            let mut next_candidate = candidate.clone();
            let moved_elements = 操作.变异(&mut next_candidate);
            let (_, next_energy) = 目标函数.计算(&next_candidate, &Some(moved_elements));
            let energy_delta = next_energy - energy;
            if energy_delta < 0.0 || (-energy_delta / temperature).exp() > random::<f64>() {
                accepts += 1;
                if energy_delta < 0.0 {
                    improves += 1;
                }
                candidate = next_candidate;
                energy = next_energy;
            }
        }
        let accept_rate = accepts as f64 / steps as f64;
        let improve_rate = improves as f64 / steps as f64;
        (candidate, accept_rate, improve_rate)
    }

    // 不提供参数，通过试验来获得一组参数的办法
    pub fn 调参<O: 目标函数, F: 变异<解类型 = O::解类型>>(
        &self,
        初始解: &O::解类型,
        目标函数: &mut O,
        操作: &mut F,
        界面: &dyn 界面,
    ) -> 降温时间表 {
        // 最高温时，接受概率应该至少有这么多
        const HIGH_ACCEPTANCE: f64 = 0.98;
        // 最低温时，改进概率应该至多有这么多
        const LOW_IMPROVEMENT: f64 = 0.02;
        // 搜索温度时用的步进大小
        const MULTIPLIER: f64 = 2.0;

        let batch = 1000;
        let mut candidate = 初始解.clone();
        let (_, energy) = 目标函数.计算(&candidate, &None);
        let mut sum_delta = 0.0;
        for _ in 0..batch {
            let mut next_candidate = candidate.clone();
            let moved_elements = 操作.变异(&mut next_candidate);
            let (_, next_energy) = 目标函数.计算(&next_candidate, &Some(moved_elements));
            sum_delta += (next_energy - energy).abs();
        }
        let initial_guess = sum_delta / batch as f64;
        let mut temperature = initial_guess;
        let mut accept_rate;
        let mut improve_rate;
        (candidate, accept_rate, improve_rate) =
            self.trial_run(目标函数, 操作, candidate, temperature, batch);
        while accept_rate > HIGH_ACCEPTANCE {
            temperature /= MULTIPLIER;
            (candidate, accept_rate, improve_rate) =
                self.trial_run(目标函数, 操作, candidate, temperature, batch);
            界面.发送(消息::TrialMax {
                temperature,
                accept_rate,
            });
        }
        while accept_rate < HIGH_ACCEPTANCE {
            temperature *= MULTIPLIER;
            (candidate, accept_rate, improve_rate) =
                self.trial_run(目标函数, 操作, candidate, temperature, batch);
            界面.发送(消息::TrialMax {
                temperature,
                accept_rate,
            });
        }
        let t_max = temperature;
        candidate = 初始解.clone();
        temperature = initial_guess;
        while improve_rate > LOW_IMPROVEMENT {
            temperature /= MULTIPLIER;
            (candidate, _, improve_rate) =
                self.trial_run(目标函数, 操作, candidate, temperature, batch);
            界面.发送(消息::TrialMin {
                temperature,
                improve_rate,
            });
        }
        let t_min = temperature;
        界面.发送(消息::Parameters { t_max, t_min });
        降温时间表 {
            t_max,
            t_min,
            steps: 1000,
        }
    }
}
