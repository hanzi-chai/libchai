//! chai: 汉字自动拆分系统［命令行版］
//!
//! `chai` 是一个使用 Rust 编写的命令行程序。用户提供拆分表以及方案配置文件，本程序能够生成编码并评测一系列指标，以及基于退火算法优化元素的布局。
//!
//! 具体用法详见 README.md 和 config.md。

use chai::cli::{Cli, Command};
use chai::constraints::Constraints;
use chai::encoder::occupation::Occupation;
use chai::encoder::simple_occupation::SimpleOccupation;
use chai::encoder::{Driver, Encoder};
use chai::objectives::Objective;
use chai::problem::Problem;
use chai::{error::Error, representation::Representation};
use clap::Parser;

fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    let (config, resource, assets) = cli.prepare_file();
    let representation = Representation::new(config)?;
    let space = representation.get_space();
    let driver = Box::new(Occupation::new(space));
    let constraints = Constraints::new(&representation)?;
    match cli.command {
        Command::Encode => {
            let mut encoder = Encoder::new(&representation, resource, &assets, driver)?;
            let codes = encoder.encode(&representation.initial, &representation);
            Cli::write_encode_results(codes);
        }
        Command::Evaluate => {
            let encoder = Encoder::new(&representation, resource, &assets, driver)?;
            let mut objective = Objective::new(&representation, encoder, assets)?;
            let (metric, _) = objective.evaluate(&representation.initial)?;
            Cli::report_metric(metric);
        }
        Command::Optimize => {
            let config = representation.config.clone();
            let solver = config.optimization.unwrap().metaheuristic.unwrap();
            let driver: Box<dyn Driver> = if representation.config.encoder.max_length <= 4 {
                Box::new(SimpleOccupation::new(representation.get_space()))
            } else {
                Box::new(Occupation::new(representation.get_space()))
            };
            let encoder = Encoder::new(&representation, resource, &assets, driver)?;
            let objective = Objective::new(&representation, encoder, assets)?;
            let mut problem = Problem::new(representation, constraints, objective)?;
            solver.solve(&mut problem, &cli);
        }
    }
    Ok(())
}
