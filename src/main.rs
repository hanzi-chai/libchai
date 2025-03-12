use chai::config::SolverConfig;
use chai::encoders::default::DefaultEncoder;
use chai::metaheuristics::Metaheuristic;
use chai::objectives::{default::DefaultObjective, Objective};
use chai::problems::default::DefaultProblem;
use chai::representation::Representation;
use chai::{Args, Command, CommandLine, Error};
use clap::Parser;
use std::thread::spawn;

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let command_line = CommandLine::new(args, None);
    let (config, resource, assets) = command_line.prepare_file();
    let _config = config.clone();
    let representation = Representation::new(config)?;
    match command_line.args.command {
        Command::Encode => {
            let length = resource.len();
            let mut encoder = DefaultEncoder::new(&representation, resource)?;
            let mut objective = DefaultObjective::new(&representation, assets, length)?;
            let codes = encoder.encode(&representation);
            let (metric, _) = objective.evaluate(&mut encoder, &representation.initial, &None);
            command_line.write_encode_results(codes);
            command_line.report_metric(metric);
        }
        Command::Optimize => {
            let length = resource.len();
            let threads = command_line.args.threads.unwrap_or(1);
            let SolverConfig::SimulatedAnnealing(sa) =
                _config.optimization.unwrap().metaheuristic.unwrap();
            let mut handles = vec![];
            for index in 0..threads {
                let encoder = DefaultEncoder::new(&representation, resource.clone())?;
                let objective = DefaultObjective::new(&representation, assets.clone(), length)?;
                let mut problem = DefaultProblem::new(representation.clone(), objective, encoder)?;
                let solver = sa.clone();
                let child = command_line.make_child(index);
                let handle = spawn(move || solver.solve(&mut problem, &child));
                handles.push(handle);
            }
            let mut results = vec![];
            for handle in handles {
                results.push(handle.join().unwrap());
            }
            results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
            for solution in results {
                print!("{}", solution.metric);
            }
        }
    }
    Ok(())
}
