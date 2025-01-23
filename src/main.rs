//! chai: 汉字自动拆分系统［命令行版］
//!
//! `chai` 是一个使用 Rust 编写的命令行程序。用户提供拆分表以及方案配置文件，本程序能够生成编码并评测一系列指标，以及基于退火算法优化元素的布局。
//!
//! 具体用法详见 README.md 和 config.md。

use chai::config::SolverConfig;
use chai::encoder::Encoder;
use chai::metaheuristics::Metaheuristic;
use chai::objectives::Objective;
use chai::problems::default::DefaultProblem;
use chai::problems::snow2::Snow2;
use chai::problems::snow4layout::Snow4Layout;
use chai::representation::Representation;
use chai::{Command, CommandLine, Error};
use clap::Parser;

fn main() -> Result<(), Error> {
    let cli = CommandLine::parse();
    let (config, resource, assets) = cli.prepare_file();
    let representation = Representation::new(config)?;
    match cli.command {
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
            let config = representation.config.clone();
            let solver = config.optimization.unwrap().metaheuristic.unwrap();
            let encoder = Encoder::new(&representation, resource, &assets)?;
            let objective = Objective::new(&representation, encoder, assets)?;
            match solver {
                SolverConfig::SimulatedAnnealing(sa) => {
                    if config.info.name == "冰雪双拼" || config.info.name == "冰雪双拼声介" {
                        let mut problem = Snow2::new(representation, objective);
                        sa.solve(&mut problem, &cli);
                    } else if config.info.name == "冰雪四拼手机布局" {
                        let mut problem = Snow4Layout::new(representation, objective);
                        sa.solve(&mut problem, &cli);
                    } else {
                        let mut problem = DefaultProblem::new(representation, objective)?;
                        sa.solve(&mut problem, &cli);
                    }
                }
            }
        }
    }
    Ok(())
}
