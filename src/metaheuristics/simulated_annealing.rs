//! Find an approximate solution to your optimisation problem using Simulated Annealing
//!
//! When metal is heated to melting point, its atoms are let loose to move freely, and do so in a
//! random fashion. If cooled too quickly (a process known as quenching), the random positioning of
//! atoms gets frozen in time, creating a hard and brittle metal. However if allowed to cool at a
//! slower rate (a process known as annealing), the atoms arrange in a more uniform fashion,
//! creating a soft and malleable metal. Simulated annealing borrows from this process.
//!
//! Here we duplicate the functionality of the `metaheuristics::hill_climbing` module but with
//! slight modification - at each iteration, we introduce a probability of going downhill! This
//! probability mimicks the cooling temperature from its physical counterpart. At first, the
//! probability of going downhill is 1. But as time moves on, we lower that probability until time
//! has run out.
//!
//! The probability of going downhill, or cooling temperature, is given by the following function:
//!ignore
//!    P(t) = e^(-10*(t^3))
//!
//! and can be seen by the following gnuplot:
//!
//!```bash
//!cat <<EOF | gnuplot -p
//!  set xrange [0:1];
//!  set yrange [0:1];
//!  set xlabel "Runtime";
//!  set ylabel "Probability of going downhill";
//!  set style line 12 lc rgb '#9bffff' lt 0 lw 1;
//!  set grid back ls 12;
//!  set style line 11 lc rgb '#5980d4' lt 1;
//!  set border 3 back ls 11;
//!  set tics nomirror;
//!  set key off;
//!  f(x) = exp(-10*(x**3));
//!  plot f(x) with lines lc rgb '#d52339';
//!EOF
//!```
//!
//! For more info on Simulated Annealing, please see the [Simulated
//! Annealing](https://wikipedia.org/wiki/Simulated_annealing) Wikipedia article.
//!
//!# Examples
//!
//!```ignore
//!let solution = metaheuristics::simulated_annealing::solve(&mut problem, runtime);
//!```

use std::{
    fmt::Display,
    time::{Duration, Instant},
};

use super::Metaheuristics;
use rand::random;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    pub t_max: f64,
    pub t_min: f64,
    pub steps: usize,
}

/// Returns an approximate solution to your optimisation problem using Simulated Annealing
///
///# Parameters
///
/// `problem` is the type that implements the `Metaheuristics` trait.
///
/// `runtime` is a `time::Duration` specifying how long to spend searching for a solution.
///
///# Examples
///
///```ignore
///let solution = metaheuristics::simulated_annealing::solve(&mut problem, runtime);
///```
pub fn solve<T: Clone, M: Clone + Display>(
    problem: &mut dyn Metaheuristics<T, M>,
    parameters: Parameters,
    report_after: Option<f64>,
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

    for step in 0..steps {
        let progress = step as f64 / steps as f64;
        let temperature = t_max / (log_space * progress).exp();
        let next_candidate = problem.tweak_candidate(&annealing_candidate);
        let start = Instant::now();
        let next_rank = problem.rank_candidate(&next_candidate);
        let elapsed = start.elapsed();
        if step == 0 {
            println!("计算一次评测用时：{} μs", elapsed.as_micros());
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
            );
        }
        if step % 10000 == 0 {
            println!(
                "优化已执行 {} 步，当前温度为 {:.2e}，当前评测指标如下：",
                step, temperature
            );
            println!("{}", annealing_rank.0);
        }
    }
    problem.save_candidate(&best_candidate, &best_rank, true);
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

pub fn autosolve<T: Clone, M: Clone + Display>(
    problem: &mut dyn Metaheuristics<T, M>,
    duration: Duration,
    report_after: Option<f64>,
) -> T {
    let batch = 1000;
    println!("开始寻找参数……");
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
    println!("体系最高温度的初始猜测：t_max = {:.2e}", temperature);
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
        println!(
            "若温度为 {:.2e}，接受率为 {:.2}%",
            temperature,
            accept_rate * 100.0
        );
    }
    while accept_rate < 0.98 {
        temperature *= 2.0;
        (candidate, accept_rate, improve_rate) = trial_run(problem, candidate, temperature, batch);
        total_steps += batch;
        println!(
            "若温度为 {:.2e}，接受率为 {:.2}%",
            temperature,
            accept_rate * 100.0
        );
    }
    let t_max = temperature;
    println!(
        "接受率已符合标准，体系最高温度估计为：t_max = {:.2e}",
        t_max
    );
    candidate = problem.generate_candidate();
    temperature = initial_guess;
    while improve_rate > 0.005 {
        temperature /= if improve_rate > 0.01 { 16.0 } else { 4.0 };
        (candidate, _, improve_rate) = trial_run(problem, candidate, temperature, batch);
        total_steps += batch;
        println!(
            "若温度为 {:.2e}，改进率为 {:.2}%",
            temperature,
            improve_rate * 100.0
        );
    }
    let t_min = temperature;
    println!(
        "改进率已符合标准，体系最低温度估计为：t_min = {:.2e}",
        t_min
    );
    let elapsed = start.elapsed();
    let steps = total_steps * duration.as_millis() as usize / elapsed.as_millis() as usize;
    println!(
        "参数寻找完成，将在 {} 分钟内用 {} 步为您优化……",
        duration.as_secs() / 60,
        steps
    );
    let parameters = Parameters {
        t_max,
        t_min,
        steps,
    };
    solve(problem, parameters, report_after)
}
