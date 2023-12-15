mod config;
use config::Cache;
mod data;
mod metric;
mod objective;
use objective::Objective;
mod problem;
use problem::ElementPlacementProblem;
mod constraints;
mod encoder;
use constraints::Constraints;
mod metaheuristics;
mod cli;
use cli::prepare_file;

fn main() {
    let (config, elements, assets) = prepare_file();
    let cache = Cache::new(&config);
    let objective = Objective::new(&config, &cache, elements, assets);
    let constraints = Constraints::new(&config, &cache);
    let mut problem = ElementPlacementProblem::new(config, cache, constraints, objective);
    problem.solve();
}
