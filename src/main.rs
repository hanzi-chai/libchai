use chai::config::SolverConfig;
use chai::encoders::default::默认编码器;
use chai::objectives::{default::默认目标函数, 目标函数};
use chai::operators::default::默认操作;
use chai::interfaces::command_line::{从命令行参数创建, 命令, 命令行, 默认命令行参数};
use chai::错误;
use clap::Parser;
use std::thread::spawn;

fn main() -> Result<(), 错误> {
    let 参数 = 默认命令行参数::parse();
    let 命令行 = 命令行::新建(参数, None);
    let 上下文 = 从命令行参数创建(&命令行.参数);
    let _config = 上下文.配置.clone();
    match 命令行.参数.command {
        命令::Encode => {
            let 编码器 = 默认编码器::新建(&上下文)?;
            let mut 目标函数 = 默认目标函数::新建(&上下文, 编码器)?;
            let (指标, _) = 目标函数.计算(&上下文.初始映射, &None);
            let 码表 = 上下文.生成码表(&目标函数.编码结果);
            命令行.输出编码结果(码表);
            命令行.输出评测指标(指标);
        }
        命令::Optimize => {
            let 线程数 = 命令行.参数.threads.unwrap_or(1);
            let SolverConfig::SimulatedAnnealing(退火) =
                _config.optimization.unwrap().metaheuristic.unwrap();
            let mut 线程池 = vec![];
            for 线程序号 in 0..线程数 {
                let 编码器 = 默认编码器::新建(&上下文)?;
                let mut 目标函数 = 默认目标函数::新建(&上下文, 编码器)?;
                let mut 操作 = 默认操作::新建(&上下文)?;
                let 优化方法 = 退火.clone();
                let 子命令行 = 命令行.生成子命令行(线程序号);
                let _上下文 = 上下文.clone();
                let 线程 = spawn(move || {
                    优化方法.优化(&_上下文.初始映射, &mut 目标函数, &mut 操作, &_上下文, &子命令行)
                });
                线程池.push(线程);
            }
            let mut 优化结果列表 = vec![];
            for 线程 in 线程池 {
                优化结果列表.push(线程.join().unwrap());
            }
            优化结果列表.sort_by(|a, b| a.分数.partial_cmp(&b.分数).unwrap());
            for 优化结果 in 优化结果列表 {
                print!("{}", 优化结果.指标);
            }
        }
    }
    Ok(())
}
