use time::Duration;
mod assets;
use assets::Assets;
mod config;
use config::Config;
mod objective;
use objective::Objective;
mod problem;
use problem::{generic_solve, ElementPlacementProblem};
mod cli;
use clap::Parser;
use cli::Args;
mod encoder;
use encoder::{read_elements, Encoder};
mod constraints;
use constraints::Constraints;
mod metaheuristics;

fn main() {
    let args = Args::parse();
    let config = Config::new(&args.config);
    let assets = Assets::new();
    let elements = read_elements(&args.elements);
    config.validate_elements(&elements);
    let encoder = Encoder::new(&config, elements, &assets);
    let objective = Objective::new(&config, assets);
    let constraints = Constraints::new(&config);
    let mut problem = ElementPlacementProblem::new(&config, constraints, objective, encoder);
    let runtime = Duration::new(1, 0);
    let _solution = generic_solve(&config, &mut problem, runtime);
}
