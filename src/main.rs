use time::Duration;
mod io;
use io::{preprocess, read_and_simplify_elements};
mod config;
use config::Config;
mod metrics;
use metrics::MetricWeights;
mod mutators;
mod problem;
use problem::ElementPlacementProblem;
mod cli;
use cli::Args;
use clap::Parser;

use crate::encoder::Encoder;
mod encoder;

fn main() {
    let args = Args::parse();
    let config = Config::new(&args.config);
    let assets = preprocess();
    let elements = read_and_simplify_elements(&args.elements, &config);
    let encoder = Encoder::new(&config);
    // let (initial, mutable_keys) = read_keymap("assets/map.txt");
    // 这里要测试使用 YAML 配置文件提供输入，暂时把原来的屏蔽了，所以不能定义 mutable_keys。后面会修正。
    let mutable_keys = vec![];
    let weights = MetricWeights::new();
    let mut problem = ElementPlacementProblem::new(config.form.mapping, mutable_keys, assets, elements, weights, encoder);
    let runtime = Duration::new(1, 0);
    let solution = metaheuristics::hill_climbing::solve(&mut problem, runtime);
    ElementPlacementProblem::write_solution("solution.txt", &solution);
}
