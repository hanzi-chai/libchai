//! chai: 汉字自动拆分系统［命令行版］
//! 
//! `chai` 是一个使用 Rust 编写的命令行程序。用户提供拆分表以及方案配置文件，本程序能够生成编码并评测一系列指标，以及基于退火算法优化元素的布局。
//! 
//! 具体用法详见 README.md 和 config.md。

use chai::representation::Buffer;
use chai::{representation::Representation, error::Error};
use chai::encoder::Encoder;
use chai::objectives::Objective;
use chai::constraints::Constraints;
use chai::problem::ElementPlacementProblem;
use chai::cli::{Cli, Command};
use clap::Parser;

fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    let (config, resource, assets) = cli.prepare_file();
    let representation = Representation::new(config)?;
    let encoder = Encoder::new(&representation, resource, &assets)?;
    match cli.command {
        Command::Encode => {
            let codes = encoder.encode(&representation.initial, &representation);
            Cli::write_encode_results(codes);
        }
        Command::Evaluate => {
            let mut buffer = Buffer::new(&encoder);
            let objective = Objective::new(&representation, encoder, assets)?;
            let (metric, _) = objective.evaluate(&representation.initial, &mut buffer)?;
            Cli::report_metric(metric);
        }
        Command::Optimize => {
            let buffer = Buffer::new(&encoder);
            let objective = Objective::new(&representation, encoder, assets)?;
            let constraints = Constraints::new(&representation)?;
            let mut problem =
                ElementPlacementProblem::new(representation, constraints, objective, buffer)?;
            problem.solve(&cli);
        }
    }
    Ok(())
}
