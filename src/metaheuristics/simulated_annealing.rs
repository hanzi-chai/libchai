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

use super::Metaheuristics;
use rand::random;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameters {
    pub tmax: f64,
    pub tmin: f64,
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
pub fn solve<T: Clone, M: Clone>(problem: &mut dyn Metaheuristics<T, M>, parameters: Parameters) -> T {
    let mut best_candidate = problem.generate_candidate();
    let mut best_rank = problem.rank_candidate(&best_candidate);
    let mut annealing_candidate = problem.clone_candidate(&best_candidate);
    let mut annealing_rank = best_rank.clone();
    let tfactor = (parameters.tmin / parameters.tmax).ln();

    for step in 0..parameters.steps {
        let progress = step as f64 / parameters.steps as f64;
        let t = parameters.tmax * (tfactor * progress).exp();
        let next_candidate = problem.tweak_candidate(&annealing_candidate);
        let next_rank = problem.rank_candidate(&next_candidate);
        let improvement = next_rank.1 - annealing_rank.1;
        if improvement < 0.0 || (random::<f64>() < (-improvement / t).exp()) {
            annealing_candidate = next_candidate;
            annealing_rank = next_rank;
        }

        if annealing_rank.1 < best_rank.1 {
            best_rank = annealing_rank.clone();
            best_candidate = problem.clone_candidate(&annealing_candidate);
            problem.save_candidate(&best_candidate, &best_rank);
        }
    }
    best_candidate
}
