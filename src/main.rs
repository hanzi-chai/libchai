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
mod cli;
mod metaheuristics;
use cli::{prepare_file, Command, write_encode_results};

fn main() {
    let (config, elements, assets, command) = prepare_file();
    let cache = Cache::new(&config);
    let objective = Objective::new(&config, &cache, elements, assets);
    match command {
        Command::Encode => {
            let (metric, _, results) = objective.evaluate(&cache.initial, true);
            let results = results.unwrap();
            write_encode_results(metric, results);
        }
        Command::Optimize => {
            let constraints = Constraints::new(&config, &cache);
            let mut problem = ElementPlacementProblem::new(config, cache, constraints, objective);
            problem.solve();
        }
    }
}
