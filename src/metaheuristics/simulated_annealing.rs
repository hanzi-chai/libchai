//! 退火算法

use super::Metaheuristic;
use crate::{
    constraints::Constraints,
    interface::Interface,
    problem::{Problem, Solution},
    representation::Element,
};
use rand::random;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use web_time::{Duration, Instant};

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
    pub steps: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedAnnealing {
    parameters: Option<Schedule>,
    runtime: Option<u64>,
    report_after: Option<f64>,
    search_method: Option<SearchConfig>,
}

impl Metaheuristic for SimulatedAnnealing {
    fn solve(&self, problem: &mut Problem, interface: &dyn Interface) -> Solution {
        interface.prepare_output();
        if let Some(schedule) = self.parameters {
            self.solve(problem, schedule, interface)
        } else {
            let runtime = self.runtime.unwrap_or(10);
            let schedule = self.autosolve(problem, runtime, interface);
            self.solve(problem, schedule, interface)
        }
    }
}

impl SimulatedAnnealing {
    /// 基于现有的一个解通过随机扰动创建一个新的解
    ///
    ///```ignore
    /// let new_candidate = problem.tweak_candidate(&old_candidate);
    ///```
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
    fn solve(
        &self,
        problem: &mut Problem,
        parameters: Schedule,
        interface: &dyn Interface,
    ) -> Solution {
        let mut best_candidate = problem.generate_candidate();
        let mut best_rank = problem.rank_candidate(&best_candidate, &None);
        let mut annealing_candidate = problem.clone_candidate(&best_candidate);
        let mut annealing_rank = best_rank.clone();
        let mut last_moved_elements = vec![];
        let Schedule {
            t_max,
            t_min,
            steps,
        } = parameters;
        let log_space = t_max.ln() - t_min.ln();
        let start = Instant::now();

        for step in 0..steps {
            let progress = step as f64 / steps as f64;
            let temperature = t_max / (log_space * progress).exp();
            if step % 1000 == 0 {
                interface.report_schedule(step, temperature, format!("{}", annealing_rank.0));
            }
            let mut next_candidate = annealing_candidate.clone();
            let current_moved_elements =
                self.tweak_candidate(&mut next_candidate, &problem.constraints);
            let mut moved_elements = current_moved_elements.clone();
            moved_elements.extend(&last_moved_elements);
            let next_rank = problem.rank_candidate(&next_candidate, &Some(moved_elements));
            if step == 1000 {
                let elapsed = start.elapsed().as_micros() / 1000;
                interface.report_elapsed(elapsed);
            }
            let improvement = next_rank.1 - annealing_rank.1;
            if improvement < 0.0 || (random::<f64>() < (-improvement / temperature).exp()) {
                annealing_candidate.clone_from(&next_candidate);
                annealing_rank = next_rank;
                last_moved_elements.clear();
            } else {
                last_moved_elements = current_moved_elements;
            }
            if annealing_rank.1 < best_rank.1 {
                best_rank = annealing_rank.clone();
                best_candidate = problem.clone_candidate(&annealing_candidate);
                problem.save_candidate(
                    &best_candidate,
                    &best_rank,
                    progress > self.report_after.unwrap_or(0.9),
                    interface,
                );
            }
        }
        interface.report_schedule(steps, t_min, format!("{}", annealing_rank.0));
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
        let mut candidate = problem.clone_candidate(&from);
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

    // 不提供参数，而是提供预期运行时间，通过试验来获得一组参数的办法
    pub fn autosolve(
        &self,
        problem: &mut Problem,
        runtime: u64,
        interface: &dyn Interface,
    ) -> Schedule {
        let batch = 1000;
        interface.init_autosolve();
        let mut candidate = problem.generate_candidate();
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
        let mut total_steps = 0_usize;
        let start = Instant::now();
        let mut accept_rate;
        let mut improve_rate;
        (candidate, accept_rate, improve_rate) =
            self.trial_run(problem, candidate, temperature, batch);
        total_steps += batch;
        while accept_rate > 0.98 {
            temperature /= 2.0;
            (candidate, accept_rate, improve_rate) =
                self.trial_run(problem, candidate, temperature, batch);
            total_steps += batch;
            interface.report_trial_t_max(temperature, accept_rate);
        }
        while accept_rate < 0.98 {
            temperature *= 2.0;
            (candidate, accept_rate, improve_rate) =
                self.trial_run(problem, candidate, temperature, batch);
            total_steps += batch;
            interface.report_trial_t_max(temperature, accept_rate);
        }
        interface.report_t_max(temperature);
        let t_max = temperature;
        candidate = problem.generate_candidate();
        temperature = initial_guess;
        while improve_rate > 0.01 {
            temperature /= 4.0;
            (candidate, _, improve_rate) = self.trial_run(problem, candidate, temperature, batch);
            total_steps += batch;
            interface.report_trial_t_min(temperature, improve_rate);
        }
        interface.report_t_min(temperature);
        let t_min = temperature;
        let elapsed = start.elapsed();
        let duration = Duration::new(runtime * 60, 0);
        let steps = total_steps * duration.as_millis() as usize / elapsed.as_millis() as usize;
        interface.report_parameters(t_max, t_min, steps);
        Schedule {
            t_max,
            t_min,
            steps,
        }
    }
}
