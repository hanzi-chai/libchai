use chai::config::SolverConfig;
use chai::encoders::default::默认编码器;
use chai::encoders::编码器;
use chai::objectives::{default::默认目标函数, 目标函数};
use chai::operators::default::默认操作;
use chai::optimizers::{优化方法, 优化问题};
use chai::{命令, 命令行, 命令行参数, 错误};
use clap::Parser;
use std::thread::spawn;

fn main() -> Result<(), 错误> {
    let 参数 = 命令行参数::parse();
    let 命令行 = 命令行::新建(参数, None);
    let 数据 = 命令行.准备数据();
    let _config = 数据.配置.clone();
    match 命令行.参数.command {
        命令::Encode => {
            let mut encoder = 默认编码器::新建(&数据)?;
            let mut objective = 默认目标函数::新建(&数据)?;
            let buffer = encoder.编码(&数据.初始映射, &None).clone();
            let codes = 数据.生成码表(&buffer);
            let (metric, _) = objective.计算(&mut encoder, &数据.初始映射, &None);
            命令行.输出编码结果(codes);
            命令行.输出评测指标(metric);
        }
        命令::Optimize => {
            let threads = 命令行.参数.threads.unwrap_or(1);
            let SolverConfig::SimulatedAnnealing(sa) =
                _config.optimization.unwrap().metaheuristic.unwrap();
            let mut handles = vec![];
            for index in 0..threads {
                let 编码器 = 默认编码器::新建(&数据)?;
                let 目标函数 = 默认目标函数::新建(&数据)?;
                let 操作 = 默认操作::新建(&数据)?;
                let mut 问题 = 优化问题::新建(数据.clone(), 编码器, 目标函数, 操作);
                let solver = sa.clone();
                let child = 命令行.生成子命令行(index);
                let handle = spawn(move || solver.优化(&mut 问题, &child));
                handles.push(handle);
            }
            let mut results = vec![];
            for handle in handles {
                results.push(handle.join().unwrap());
            }
            results.sort_by(|a, b| a.分数.partial_cmp(&b.分数).unwrap());
            for solution in results {
                print!("{}", solution.指标);
            }
        }
    }
    Ok(())
}
