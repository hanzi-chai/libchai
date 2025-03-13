use chai::config::SolverConfig;
use chai::encoders::default::DefaultEncoder;
use chai::encoders::Encoder;
use chai::metaheuristics::Metaheuristic;
use chai::objectives::{default::DefaultObjective, Objective};
use chai::problems::default::DefaultProblem;
use chai::representation::{Assets, Representation};
use chai::{Args, Command, CommandLine, Error};
use clap::Parser;
use std::thread::spawn;

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let command_line = CommandLine::new(args, None);
    let (config, assets) = command_line.prepare_file();
    let Assets {
        key_distribution,
        pair_equivalence,
        encodables,
    } = assets;
    let _config = config.clone();
    let length = encodables.len();
    let representation = Representation::new(config)?;
    match command_line.args.command {
        Command::Encode => {
            let mut encoder = DefaultEncoder::new(&representation, encodables)?;
            let mut objective =
                DefaultObjective::new(&representation, key_distribution, pair_equivalence, length)?;
            let buffer = encoder.encode(&representation.initial, &None).clone();
            let codes = representation.export_code(&buffer, &encoder.encodables);
            let (metric, _) = objective.evaluate(&mut encoder, &representation.initial, &None);
            command_line.write_encode_results(codes);
            command_line.report_metric(metric);
        }
        Command::Optimize => {
            let threads = command_line.args.threads.unwrap_or(1);
            let SolverConfig::SimulatedAnnealing(sa) =
                _config.optimization.unwrap().metaheuristic.unwrap();
            let mut handles = vec![];
            for index in 0..threads {
                let encoder = DefaultEncoder::new(&representation, encodables.clone())?;
                let objective = DefaultObjective::new(
                    &representation,
                    key_distribution.clone(),
                    pair_equivalence.clone(),
                    length,
                )?;
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
