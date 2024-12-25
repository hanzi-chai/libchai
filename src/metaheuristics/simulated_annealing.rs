//! 退火算法

use super::Metaheuristic;
use crate::{
    constraints::Constraints,
    problem::{Problem, Solution},
    representation::Element,
    Interface, Message,
};
use rand::random;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use web_time::Instant;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub random_move: f64,
    pub random_swap: f64,
    pub random_full_key_swap: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// 退火算法的参数，包括最高温、最低温、步数
pub struct Schedule {
    pub t_max: f64,
    pub t_min: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedAnnealing {
    parameters: Option<Schedule>,
    steps: Option<usize>,
    report_after: Option<f64>,
    search_method: Option<SearchConfig>,
    update_interval: Option<usize>,
}

impl Metaheuristic for SimulatedAnnealing {
    fn solve(&self, problem: &mut Problem, interface: &dyn Interface) -> Solution {
        interface.post(Message::PrepareOutput);
        let schedule = self
            .parameters
            .unwrap_or_else(|| self.autosolve(problem, interface));
        self.solve_with(problem, schedule, interface)
    }
}

impl SimulatedAnnealing {
    /// 基于现有的一个解通过随机扰动创建一个新的解
    pub fn tweak_candidate(
        &self,
        candidate: &mut Solution,
        constraints: &Constraints,
    ) -> Vec<Element> {
        let method = self.search_method.as_ref().unwrap_or(&SearchConfig {
            random_move: 0.9,
            random_swap: 0.09,
            random_full_key_swap: 0.01,
        });
        let sum = method.random_move + method.random_swap + method.random_full_key_swap;
        let ratio1 = method.random_move / sum;
        let ratio2 = (method.random_move + method.random_swap) / sum;
        let number: f64 = random();
        if number < ratio1 {
            constraints.constrained_random_move(candidate)
        } else if number < ratio2 {
            constraints.constrained_random_swap(candidate)
        } else {
            constraints.constrained_full_key_swap(candidate)
        }
    }

    /// 退火算法求解的主函数
    fn solve_with(
        &self,
        problem: &mut Problem,
        parameters: Schedule,
        interface: &dyn Interface,
    ) -> Solution {
        let mut best_candidate = problem.initial_candidate();
        let mut best_rank = problem.rank_candidate(&best_candidate, &None);
        let mut annealing_candidate = best_candidate.clone();
        let mut annealing_rank = best_rank.clone();
        let mut last_diff = vec![];
        let Schedule { t_max, t_min } = parameters;
        let steps = self.steps.unwrap_or(1000);
        let start = Instant::now();
        let update_interval = self.update_interval.unwrap_or(1000);

        for step in 0..steps {
            // 等比级数降温：每一步的温度都是上一步的温度乘以一个固定倍数
            let progress = step as f64 / steps as f64;
            let temperature = t_max * (t_min / t_max).powf(progress);
            // 每过一定的步数，报告当前状态和计算速度
            if step % update_interval == 0 {
                let metric = format!("{}", annealing_rank.0);
                interface.post(Message::Progress {
                    steps: step,
                    temperature,
                    metric,
                });
                if step == update_interval {
                    let elapsed = start.elapsed().as_micros() / update_interval as u128;
                    interface.post(Message::Elapsed(elapsed));
                }
            }
            // 生成一个新解
            let mut next_candidate = annealing_candidate.clone();
            let diff = self.tweak_candidate(&mut next_candidate, &problem.constraints);
            let mut total_diff = diff.clone();
            total_diff.extend(&last_diff);
            let next_rank = problem.rank_candidate(&next_candidate, &Some(total_diff));
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
                problem.save_candidate(
                    &best_candidate,
                    &best_rank,
                    progress > self.report_after.unwrap_or(0.9),
                    interface,
                );
            }
        }
        interface.post(Message::Progress {
            steps,
            temperature: t_min,
            metric: format!("{}", best_rank.0),
        });
        problem.save_candidate(&best_candidate, &best_rank, true, interface);
        best_candidate
    }

    fn trial_run(
        &self,
        problem: &mut Problem,
        from: Solution,
        temperature: f64,
        steps: usize,
    ) -> (Solution, f64, f64) {
        let mut candidate = from.clone();
        let (_, mut energy) = problem.rank_candidate(&candidate, &None);
        let mut accepts = 0;
        let mut improves = 0;
        for _ in 0..steps {
            let mut next_candidate = candidate.clone();
            let moved_elements = self.tweak_candidate(&mut next_candidate, &problem.constraints);
            let (_, next_energy) = problem.rank_candidate(&next_candidate, &Some(moved_elements));
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
    pub fn autosolve(&self, problem: &mut Problem, interface: &dyn Interface) -> Schedule {
        // 最高温时，接受概率应该至少有这么多
        const HIGH_ACCEPTANCE: f64 = 0.98;
        // 最低温时，改进概率应该至多有这么多
        const LOW_IMPROVEMENT: f64 = 0.02;
        // 搜索温度时用的步进大小
        const MULTIPLIER: f64 = 2.0;

        let batch = 1000;
        let mut candidate = problem.initial_candidate();
        let (_, energy) = problem.rank_candidate(&candidate, &None);
        let mut sum_delta = 0.0;
        for _ in 0..batch {
            let mut next_candidate = candidate.clone();
            let moved_elements = self.tweak_candidate(&mut next_candidate, &problem.constraints);
            let (_, next_energy) = problem.rank_candidate(&next_candidate, &Some(moved_elements));
            sum_delta += (next_energy - energy).abs();
        }
        let initial_guess = sum_delta / batch as f64;
        let mut temperature = initial_guess;
        let mut accept_rate;
        let mut improve_rate;
        (candidate, accept_rate, improve_rate) =
            self.trial_run(problem, candidate, temperature, batch);
        while accept_rate > HIGH_ACCEPTANCE {
            temperature /= MULTIPLIER;
            (candidate, accept_rate, improve_rate) =
                self.trial_run(problem, candidate, temperature, batch);
            interface.post(Message::TrialMax {
                temperature,
                accept_rate,
            });
        }
        while accept_rate < HIGH_ACCEPTANCE {
            temperature *= MULTIPLIER;
            (candidate, accept_rate, improve_rate) =
                self.trial_run(problem, candidate, temperature, batch);
            interface.post(Message::TrialMax {
                temperature,
                accept_rate,
            });
        }
        let t_max = temperature;
        candidate = problem.initial_candidate();
        temperature = initial_guess;
        while improve_rate > LOW_IMPROVEMENT {
            temperature /= MULTIPLIER;
            (candidate, _, improve_rate) = self.trial_run(problem, candidate, temperature, batch);
            interface.post(Message::TrialMin {
                temperature,
                improve_rate,
            });
        }
        let t_min = temperature;
        interface.post(Message::Parameters { t_max, t_min });
        Schedule { t_max, t_min }
    }
}
