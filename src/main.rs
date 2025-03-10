//! chai: 汉字自动拆分系统［命令行版］
//!
//! `chai` 是一个使用 Rust 编写的命令行程序。用户提供拆分表以及方案配置文件，本程序能够生成编码并评测一系列指标，以及基于退火算法优化元素的布局。
//!
//! 具体用法详见 README.md 和 config.md。

use std::fs::create_dir_all;
use std::path::PathBuf;
use std::thread;

use chai::config::SolverConfig;
use chai::encoder::Encoder;
use chai::metaheuristics::Metaheuristic;
use chai::objectives::metric::Metric;
use chai::objectives::Objective;
use chai::problems::default::DefaultProblem;
use chai::problems::snow2::Snow2;
use chai::problems::snow4layout::Snow4Layout;
use chai::problems::Solution;
use chai::representation::{Assemble, Assets, Representation};
use chai::{Command, CommandLine, CommandLineArgs, Error};
use chrono::Local;
use clap::Parser;

fn run_optimization(
    representation: Representation,
    resource: Vec<Assemble>,
    assets: Assets,
    cli: &CommandLine,
) -> Result<(Solution, Metric, f64), Error> {
    let config = representation.config.clone();
    let solver = config.optimization.unwrap().metaheuristic.unwrap();
    let encoder = Encoder::new(&representation, resource, &assets)?;
    let objective = Objective::new(&representation, encoder, assets)?;
    let result = match solver {
        SolverConfig::SimulatedAnnealing(sa) => {
            if config.info.name == "冰雪双拼" || config.info.name == "冰雪双拼声介" {
                let mut problem = Snow2::new(representation, objective);
                sa.solve(&mut problem, cli)
            } else if config.info.name == "冰雪四拼手机布局" {
                let mut problem = Snow4Layout::new(representation, objective);
                sa.solve(&mut problem, cli)
            } else {
                let mut problem = DefaultProblem::new(representation, objective)?;
                sa.solve(&mut problem, cli)
            }
        }
    };
    Ok(result)
}

fn main() -> Result<(), Error> {
    let args = CommandLineArgs::parse();
    let (config, resource, assets) = args.prepare_file();
    let representation = Representation::new(config)?;
    match args.command {
        Command::Encode => {
            let mut encoder = Encoder::new(&representation, resource, &assets)?;
            let codes = encoder.encode(&representation.initial, &representation);
            CommandLine::write_encode_results(codes);
        }
        Command::Evaluate => {
            let encoder = Encoder::new(&representation, resource, &assets)?;
            let mut objective = Objective::new(&representation, encoder, assets)?;
            let (metric, _) = objective.evaluate(&representation.initial, &None);
            CommandLine::report_metric(metric);
        }
        Command::Optimize => {
            // 用当前时间戳生成输出路径
            let time = Local::now().format("%m-%d+%H_%M_%S").to_string();
            let parent_dir = PathBuf::from(format!("output-{}", time));
            create_dir_all(parent_dir.clone()).unwrap();
            let threads = args.threads.unwrap_or(1);
            if threads == 1 {
                let command_line = CommandLine::new(args.clone(), parent_dir);
                run_optimization(representation, resource, assets, &command_line)?;
            } else {
                let mut handles = vec![];
                for index in 0..threads {
                    let output_dir = parent_dir.join(format!("{}", index));
                    create_dir_all(output_dir.clone()).unwrap();
                    let representation = representation.clone();
                    let resource = resource.clone();
                    let assets = assets.clone();
                    let command_line = CommandLine::new(args.clone(), output_dir);
                    let handle = thread::spawn(move || {
                        run_optimization(representation, resource, assets, &command_line).unwrap()
                    });
                    handles.push(handle);
                }
                let mut metrics: Vec<_> = handles
                    .into_iter()
                    .map(|handle| handle.join().unwrap())
                    .collect();
                metrics.sort_by_key(|m| (m.2 * 1e10).round() as i64);
                metrics
                    .into_iter()
                    .for_each(|(_, metric, __)| CommandLine::report_metric(metric));
            }
        }
    }
    Ok(())
}
