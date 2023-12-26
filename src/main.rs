//! chai: 汉字自动拆分系统［命令行版］
//! 
//! `chai` 是一个使用 Rust 编写的命令行程序。用户提供拆分表以及方案配置文件，本程序能够生成单字、词组的编码并评测一系列指标，以及基于退火算法优化元素的布局。
//! 
//! 具体用法详见 README.md 和 config.md。

mod config;
mod data;
use clap::Parser;
use encoder::Encoder;
mod problem;
use problem::ElementPlacementProblem;
mod constraints;
mod encoder;
use constraints::Constraints;
mod cli;
mod metaheuristics;
use cli::{Cli, Command};
use representation::Representation;
mod objectives;
use objectives::Objective;
mod representation;
mod interface;

fn main() {
    let cli = Cli::parse();
    let (config, characters, words, assets) = cli.prepare_file();
    let representation = Representation::new(config);
    let encoder = Encoder::new(&representation, characters, words, &assets);
    let objective = Objective::new(&representation, encoder, assets);
    let mut buffer = objective.init_buffer();
    match cli.command {
        Command::Encode => {
            let (metric, _) = objective.evaluate(&representation.initial, &mut buffer);
            let codes = objective.export_codes(&mut buffer);
            let human_codes = representation.recover_codes(codes);
            Cli::write_encode_results(metric, human_codes);
        }
        Command::Optimize => {
            let constraints = Constraints::new(&representation);
            let mut problem =
                ElementPlacementProblem::new(representation, constraints, objective, buffer);
            problem.solve(&cli);
        }
    }
}
