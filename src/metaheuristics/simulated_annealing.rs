//! 退火算法

use super::Metaheuristics;
use crate::interface::Interface;
use rand::random;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use web_time::{Duration, Instant};

/// 退火算法的参数，包括最高温、最低温、步数
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Parameters {
    pub t_max: f64,
    pub t_min: f64,
    pub steps: usize,
}

/// 退火算法求解的主函数
pub fn solve<T: Clone, M: Clone + Display>(
    problem: &mut dyn Metaheuristics<T, M>,
    parameters: Parameters,
    report_after: Option<f64>,
    interface: &dyn Interface,
) -> T {
    let mut best_candidate = problem.generate_candidate();
    let mut best_rank = problem.rank_candidate(&best_candidate);
    let mut annealing_candidate = problem.clone_candidate(&best_candidate);
    let mut annealing_rank = best_rank.clone();
    let Parameters {
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
        let next_candidate = problem.tweak_candidate(&annealing_candidate);
        let next_rank = problem.rank_candidate(&next_candidate);
        if step == 1000 {
            let elapsed = start.elapsed().as_micros() / 1000;
            interface.report_elapsed(elapsed);
        }
        let improvement = next_rank.1 - annealing_rank.1;
        if improvement < 0.0 || (random::<f64>() < (-improvement / temperature).exp()) {
            annealing_candidate = next_candidate;
            annealing_rank = next_rank;
        }
        if annealing_rank.1 < best_rank.1 {
            best_rank = annealing_rank.clone();
            best_candidate = problem.clone_candidate(&annealing_candidate);
            problem.save_candidate(
                &best_candidate,
                &best_rank,
                progress > report_after.unwrap_or(0.9),
                interface,
            );
        }
    }
    interface.report_schedule(steps, t_min, format!("{}", annealing_rank.0));
    problem.save_candidate(&best_candidate, &best_rank, true, interface);
    best_candidate
}

fn trial_run<T: Clone, M: Clone>(
    problem: &mut dyn Metaheuristics<T, M>,
    from: T,
    temperature: f64,
    steps: usize,
) -> (T, f64, f64) {
    let mut candidate = problem.clone_candidate(&from);
    let (_, mut energy) = problem.rank_candidate(&candidate);
    let mut accepts = 0;
    let mut improves = 0;
    for _ in 0..steps {
        let next_candidate = problem.tweak_candidate(&candidate);
        let (_, next_energy) = problem.rank_candidate(&next_candidate);
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
pub fn autosolve<T: Clone, M: Clone + Display>(
    problem: &mut dyn Metaheuristics<T, M>,
    runtime: u64,
    report_after: Option<f64>,
    interface: &dyn Interface,
) -> T {
    let batch = 1000;
    interface.init_autosolve();
    let mut candidate = problem.generate_candidate();
    let (_, energy) = problem.rank_candidate(&candidate);
    let mut sum_delta = 0.0;
    for _ in 0..batch {
        let next_candidate = problem.tweak_candidate(&candidate);
        let (_, next_energy) = problem.rank_candidate(&next_candidate);
        sum_delta += (next_energy - energy).abs();
    }
    let initial_guess = sum_delta / batch as f64;
    let mut temperature = initial_guess;
    let mut total_steps = 0_usize;
    let start = Instant::now();
    let mut accept_rate;
    let mut improve_rate;
    (candidate, accept_rate, improve_rate) = trial_run(problem, candidate, temperature, batch);
    total_steps += batch;
    while accept_rate > 0.98 {
        temperature /= 2.0;
        (candidate, accept_rate, improve_rate) = trial_run(problem, candidate, temperature, batch);
        total_steps += batch;
        interface.report_trial_t_max(temperature, accept_rate);
    }
    while accept_rate < 0.98 {
        temperature *= 2.0;
        (candidate, accept_rate, improve_rate) = trial_run(problem, candidate, temperature, batch);
        total_steps += batch;
        interface.report_trial_t_max(temperature, accept_rate);
    }
    interface.report_t_max(temperature);
    let t_max = temperature;
    candidate = problem.generate_candidate();
    temperature = initial_guess;
    while improve_rate > 0.01 {
        temperature /= 4.0;
        (candidate, _, improve_rate) = trial_run(problem, candidate, temperature, batch);
        total_steps += batch;
        interface.report_trial_t_min(temperature, improve_rate);
    }
    interface.report_t_min(temperature);
    let t_min = temperature;
    let elapsed = start.elapsed();
    let duration = Duration::new(runtime * 60, 0);
    let steps = total_steps * duration.as_millis() as usize / elapsed.as_millis() as usize;
    interface.report_parameters(t_max, t_min, steps);
    let parameters = Parameters {
        t_max,
        t_min,
        steps,
    };
    solve(problem, parameters, report_after, interface)
}
