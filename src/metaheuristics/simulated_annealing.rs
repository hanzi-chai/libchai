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
use rand::{thread_rng, Rng};
use time::{Duration, Instant};

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
pub fn solve<T>(problem: &mut dyn Metaheuristics<T>, runtime: Duration) -> T {
    let mut best_candidate = problem.generate_candidate();
    let mut annealing_candidate = problem.tweak_candidate(&best_candidate);
    let start_time = Instant::now();
    let runtime_in_milliseconds = runtime.whole_milliseconds() as f64;

    loop {
        let portion_elapsed =
            (start_time.elapsed().whole_milliseconds() as f64) / runtime_in_milliseconds;

        if portion_elapsed >= 1.0 {
            break;
        }

        let next_candidate = problem.tweak_candidate(&annealing_candidate);
        let next_is_better =
            problem.rank_candidate(&next_candidate) > problem.rank_candidate(&annealing_candidate);
        let replacement_threshold = 1.0f64.exp().powf(-10.0 * portion_elapsed.powf(3.0));

        if next_is_better || (thread_rng().gen_range(0.0..1.0) < replacement_threshold) {
            annealing_candidate = next_candidate;
        }

        if problem.rank_candidate(&annealing_candidate) > problem.rank_candidate(&best_candidate) {
            best_candidate = problem.clone_candidate(&annealing_candidate);
        }
    }

    best_candidate
}
