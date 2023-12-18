mod config;
mod data;
mod metric;
mod objective;
use clap::Parser;
use encoder::Encoder;
use objective::Objective;
mod problem;
use problem::ElementPlacementProblem;
mod constraints;
mod encoder;
use constraints::Constraints;
mod cli;
mod metaheuristics;
use cli::{Command, Cli};
use representation::Representation;
mod objectives;
mod representation;

fn main() {
    let cli = Cli::parse();
    let (config, characters, words, assets) = cli.prepare_file();
    let representation = Representation::new(config);
    let mut buffer = representation.init_buffer(characters.len(), words.len());
    let encoder = Encoder::new(&representation, characters, words, &assets);
    let objective = Objective::new(&representation, encoder, assets);
    match cli.command.unwrap_or(Command::Encode) {
        Command::Encode => {
            let (metric, _) = objective.evaluate(&representation.initial, &mut buffer);
            let codes = objective.export_codes(&mut buffer);
            let human_codes = representation.recover_codes(codes);
            Cli::write_encode_results(metric, human_codes);
        }
        Command::Optimize => {
            let constraints = Constraints::new(&representation);
            let mut problem = ElementPlacementProblem::new(representation, constraints, objective, buffer);
            problem.solve();
        }
    }
}
