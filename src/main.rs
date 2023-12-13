mod assets;
mod config;
use config::{Cache, Config};
mod data;
mod objective;
use objective::Objective;
mod problem;
use problem::ElementPlacementProblem;
mod cli;
use cli::Args;
use clap::Parser;
mod encoder;
mod constraints;
use constraints::Constraints;
mod metaheuristics;

fn main() {
    let args = Args::parse();
    let config = Config::new(&args.config);
    let cache = Cache::new(&config);
    let objective = Objective::new(&config, &cache, &args.elements);
    let constraints = Constraints::new(&config, &cache);
    let mut problem = ElementPlacementProblem::new(config, cache, constraints, objective);
    problem.solve();
}
