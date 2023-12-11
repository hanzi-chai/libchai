//! Find approximate solutions to your optimisation problem using metaheuristics algorithms
//!
//! The aim of this crate is to host various Metaheuristics algorithms. Patches
//! implementing useful algorithms most welcome.
//!
//! The documentation for this crate can be [found
//! here](https://www.alfie.wtf/rustdoc/metaheuristics/metaheuristics/).
//!
//!## What are Metaheuristics
//!
//! Metaheuristics are a class of stochastic optimisation algorithms. These type of algorithms rely
//! on randomness to jump around the search space, then sample where they land for possible
//! solutions. In simple terms, **metaheuristics are structured trial and error**.
//!
//! If you've got a trial and error problem, and individual trials can be compared and ranked
//! against each other, Metaheuristics may be your most viable option at getting good results.
//!
//! For more information, please see the
//! [Metaheuristics](https://en.wikipedia.org/wiki/Metaheuristic) Wikipedia article, and
//! [Essentials of
//! Metaheuristics](https://www.amazon.com/Essentials-Metaheuristics-Second-Sean-Luke/dp/1300549629).
//!
//!## How can I use this crate
//!
//! By implementing the `Metaheuristics` trait, the algorithms within the following modules will be
//! available to you. To see an example implementation, check out the [Travelling Salesman
//! Problem](https://www.alfie.wtf/rustdoc/travelling_salesman/travelling_salesman/) crate.
//!
//!# Examples
//!
//!```ignore
//! let solution = metaheuristics::hill_climbing::solve(&mut problem, runtime);
//!```
//!
//!# Support
//!
//! Please report any bugs or feature requests at:
//!
//! * [https://gitlab.com/alfiedotwtf/metaheuristics/issues](https://gitlab.com/alfiedotwtf/metaheuristics/issues)
//!
//! Feel free to fork the repository and submit pull requests :)
//!
//!# Author
//!
//! [Alfie John](https://www.alfie.wtf) &lt;[alfie@alfie.wtf](mailto:alfie@alfie.wtf)&gt;
//!
//!# Warranty
//!
//! IT COMES WITHOUT WARRANTY OF ANY KIND.
//!
//!# Copyright and License
//!
//! This program is free software: you can redistribute it and/or modify it
//! under the terms of the GNU General Public License as published by the Free
//! Software Foundation, either version 3 of  the License, or (at your option)
//! any later version.
//!
//! This program is distributed in the hope that it will be useful, but
//! WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY
//! or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
//! more details.
//!
//! You should have received a copy of the GNU General Public License along
//! with this program. If not, see
//! [http://www.gnu.org/licenses/](http://www.gnu.org/licenses/).
extern crate rand;
extern crate time;

pub mod hill_climbing;
pub mod random_search;
pub mod simulated_annealing;

/// Implement this simple trait to apply metaheuristics to your optimisation problems
pub trait Metaheuristics<T> {
    /// Clone the supplied candidate solution
    ///
    ///```ignore
    /// let new_candidate = problem.clone_candidate(&old_candidate);
    ///```
    fn clone_candidate(&mut self, candidate: &T) -> T;

    /// Randomly generate a new candidate solution
    ///
    ///```ignore
    /// let candidate = problem.generate_candidate();
    ///```
    fn generate_candidate(&mut self) -> T;

    /// Rank a candidate solution so that it can be compared with another (higher is better)
    ///
    ///```ignore
    /// if problem.rank_candidate(&new_candidate) > problem.rank_candidate(&old_candidate) {
    ///     ...
    /// }
    ///```
    fn rank_candidate(&mut self, candidate: &T) -> f64;

    /// Clone the supplied candidate solution, then make a small (but random) modification
    ///
    ///```ignore
    /// let new_candidate = problem.tweak_candidate(&old_candidate);
    ///```
    fn tweak_candidate(&mut self, candidate: &T) -> T;
}
