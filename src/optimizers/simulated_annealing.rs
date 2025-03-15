//! 退火算法

use std::time::Instant;

use super::{优化方法, 优化结果, 优化问题};
use crate::{
    data::元素映射,
    encoders::编码器,
    objectives::目标函数,
    operators::{default::变异配置, 变异},
    消息, 界面,
};
use rand::random;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// 退火算法的参数，包括最高温、最低温、步数
pub struct 降温时间表 {
    pub t_max: f64,
    pub t_min: f64,
    pub steps: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 退火方法 {
    pub parameters: Option<降温时间表>,
    pub report_after: Option<f64>,
    pub search_method: Option<变异配置>,
    pub update_interval: Option<usize>,
}

impl<F: 变异> 优化方法<F> for 退火方法 {
    fn 优化<E: 编码器, O: 目标函数>(
        &self,
        问题: &mut 优化问题<E, O, F>,
        界面: &dyn 界面,
    ) -> 优化结果 {
        let 降温时间表 = self.parameters.unwrap_or_else(|| self.调参(问题, 界面));
        self.solve_with(问题, 降温时间表, 界面)
    }
}

impl 退火方法 {
    /// 退火算法求解的主函数
    fn solve_with<E: 编码器, O: 目标函数, F: 变异>(
        &self,
        问题: &mut 优化问题<E, O, F>,
        parameters: 降温时间表,
        interface: &dyn 界面,
    ) -> 优化结果 {
        let mut best_candidate = 问题.数据.初始映射.clone();
        let mut best_rank = 问题.计算(&best_candidate, &None);
        let mut annealing_candidate = best_candidate.clone();
        let mut annealing_rank = best_rank.clone();
        let mut last_diff = vec![];
        let 降温时间表 {
            t_max,
            t_min,
            steps,
        } = parameters;
        let start = Instant::now();
        let update_interval = self.update_interval.unwrap_or(1000);

        for step in 0..steps {
            // 等比级数降温：每一步的温度都是上一步的温度乘以一个固定倍数
            let progress = step as f64 / steps as f64;
            let temperature = t_max * (t_min / t_max).powf(progress);
            // 每过一定的步数，报告当前状态和计算速度
            if step % update_interval == 0 {
                interface.发送(消息::Progress {
                    steps: step,
                    temperature,
                    metric: annealing_rank.0.clone(),
                });
                if step == update_interval {
                    let elapsed = start.elapsed().as_micros() / update_interval as u128;
                    interface.发送(消息::Elapsed(elapsed));
                }
            }
            // 生成一个新解
            let mut next_candidate = annealing_candidate.clone();
            let diff = 问题.操作.变异(&mut next_candidate);
            let mut total_diff = diff.clone();
            total_diff.extend(&last_diff);
            let next_rank = 问题.计算(&next_candidate, &Some(total_diff));
            // 如果满足退火条件，接受新解
            let improvement = next_rank.1 - annealing_rank.1;
            if improvement < 0.0 || (random::<f64>() < (-improvement / temperature).exp()) {
                annealing_candidate.clone_from(&next_candidate);
                annealing_rank = next_rank;
                last_diff.clear();
            } else {
                last_diff = diff;
            }
            // 如果当前解优于目前的最优解，更新最优解
            if annealing_rank.1 < best_rank.1 {
                best_rank = annealing_rank.clone();
                best_candidate.clone_from(&annealing_candidate);
                let save = progress > self.report_after.unwrap_or(0.9);
                let 配置 = 问题.数据.更新配置(&best_candidate);
                interface.发送(消息::BetterSolution {
                    metric: best_rank.0.clone(),
                    config: 配置,
                    save,
                })
            }
        }
        interface.发送(消息::Progress {
            steps,
            temperature: t_min,
            metric: best_rank.0.clone(),
        });
        let 配置 = 问题.数据.更新配置(&best_candidate);
        interface.发送(消息::BetterSolution {
            metric: best_rank.0.clone(),
            config: 配置,
            save: true,
        });
        优化结果 {
            映射: best_candidate,
            指标: best_rank.0.clone(),
            分数: start.elapsed().as_secs_f64(),
        }
    }

    fn trial_run<E: 编码器, O: 目标函数, F: 变异>(
        &self,
        问题: &mut 优化问题<E, O, F>,
        from: 元素映射,
        temperature: f64,
        steps: usize,
    ) -> (元素映射, f64, f64) {
        let mut candidate = from.clone();
        let (_, mut energy) = 问题.计算(&candidate, &None);
        let mut accepts = 0;
        let mut improves = 0;

        for _ in 0..steps {
            let mut next_candidate = candidate.clone();
            let moved_elements = 问题.操作.变异(&mut next_candidate);
            let (_, next_energy) = 问题.计算(&next_candidate, &Some(moved_elements));
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
    pub fn 调参<E: 编码器, O: 目标函数, F: 变异>(
        &self,
        问题: &mut 优化问题<E, O, F>,
        界面: &dyn 界面,
    ) -> 降温时间表 {
        // 最高温时，接受概率应该至少有这么多
        const HIGH_ACCEPTANCE: f64 = 0.98;
        // 最低温时，改进概率应该至多有这么多
        const LOW_IMPROVEMENT: f64 = 0.02;
        // 搜索温度时用的步进大小
        const MULTIPLIER: f64 = 2.0;

        let batch = 1000;
        let mut candidate = 问题.数据.初始映射.clone();
        let (_, energy) = 问题.计算(&candidate, &None);
        let mut sum_delta = 0.0;
        for _ in 0..batch {
            let mut next_candidate = candidate.clone();
            let moved_elements = 问题.操作.变异(&mut next_candidate);
            let (_, next_energy) = 问题.计算(&next_candidate, &Some(moved_elements));
            sum_delta += (next_energy - energy).abs();
        }
        let initial_guess = sum_delta / batch as f64;
        let mut temperature = initial_guess;
        let mut accept_rate;
        let mut improve_rate;
        (candidate, accept_rate, improve_rate) =
            self.trial_run(问题, candidate, temperature, batch);
        while accept_rate > HIGH_ACCEPTANCE {
            temperature /= MULTIPLIER;
            (candidate, accept_rate, improve_rate) =
                self.trial_run(问题, candidate, temperature, batch);
            界面.发送(消息::TrialMax {
                temperature,
                accept_rate,
            });
        }
        while accept_rate < HIGH_ACCEPTANCE {
            temperature *= MULTIPLIER;
            (candidate, accept_rate, improve_rate) =
                self.trial_run(问题, candidate, temperature, batch);
            界面.发送(消息::TrialMax {
                temperature,
                accept_rate,
            });
        }
        let t_max = temperature;
        candidate = 问题.数据.初始映射.clone();
        temperature = initial_guess;
        while improve_rate > LOW_IMPROVEMENT {
            temperature /= MULTIPLIER;
            (candidate, _, improve_rate) = self.trial_run(问题, candidate, temperature, batch);
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
