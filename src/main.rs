use time::Duration;
mod assets;
use assets::Assets;
mod config;
use config::Config;
mod objective;
use objective::Objective;
mod problem;
use problem::ElementPlacementProblem;
mod cli;
use cli::Args;
use clap::Parser;
mod encoder;
use encoder::{Encoder, read_elements};
mod constraints;
use constraints::Constraints;
mod metaheuristics;

fn main() {
    let args = Args::parse();
    let config = Config::new(&args.config);
    let assets = Assets::new();
    let elements = read_elements(&args.elements);
    let encoder = Encoder::new(&config, elements, &assets);
    let objective = Objective::new(assets);
    let constraints = Constraints::new(vec![]);
    let mut problem = ElementPlacementProblem::new(
        config.form.mapping,
        constraints,
        objective,
        encoder,
    );
    let runtime = Duration::new(1, 0);
    let solution = metaheuristics::hill_climbing::solve(&mut problem, runtime);
    ElementPlacementProblem::write_solution("solution.txt", &solution);
}
