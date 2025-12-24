use chai::config::SolverConfig;
use chai::contexts::default::默认上下文;
use chai::encoders::default::默认编码器;
use chai::objectives::{default::默认目标函数, 目标函数};
use chai::operators::default::默认操作;
use chai::optimizers::{优化方法, 优化问题};
use chai::server;
use chai::{命令, 命令行, 命令行参数, 错误};
use clap::Parser;
use std::thread::spawn;

#[tokio::main]
async fn main() -> Result<(), 错误> {
    let 参数 = 命令行参数::parse();

    match 参数.command {
        命令::Server { port } => {
            server::start_server(port).await.unwrap();
        }
        命令::Encode { data } => {
            // 重构参数结构，以便复用现有的数据加载逻辑
            let 重构参数 = 命令行参数 {
                command: 命令::Encode { data: data.clone() },
            };
            let 命令行 = 命令行::新建(重构参数, None);
            let 数据 = 命令行.准备数据();

            let mut 编码器 = 默认编码器::新建(&数据)?;
            let mut 目标函数 = 默认目标函数::新建(&数据)?;
            let mut 编码结果 = 编码器.编码(&数据.初始映射, &None).clone();
            let 码表 = 数据.生成码表(&编码结果);
            let (指标, _) = 目标函数.计算(&mut 编码结果, &数据.初始映射);
            命令行.输出编码结果(码表);
            命令行.输出评测指标(指标);
        }
        命令::Optimize { data, threads } => {
            // 重构参数结构，以便复用现有的数据加载逻辑
            let 重构参数 = 命令行参数 {
                command: 命令::Optimize {
                    data: data.clone(),
                    threads,
                },
            };
            let 命令行 = 命令行::新建(重构参数, None);
            let 数据 = 命令行.准备数据();
            let _config = 数据.配置.clone();

            let 退火 = match _config.optimization {
                Some(opt) => match opt.metaheuristic {
                    Some(SolverConfig::SimulatedAnnealing(sa)) => sa,
                    _ => return Err("配置文件中缺少模拟退火算法配置".into()),
                },
                None => return Err("配置文件中缺少优化配置".into()),
            };

            let mut 线程池 = vec![];
            for 线程序号 in 0..threads {
                let 编码器 = 默认编码器::新建(&数据)?;
                let 目标函数 = 默认目标函数::新建(&数据)?;
                let 操作 = 默认操作::新建(&数据)?;
                let mut 问题 = 优化问题::新建(数据.clone(), 编码器, 目标函数, 操作);
                let 优化方法 = 退火.clone();
                let 子命令行 = 命令行.生成子命令行(线程序号);
                let _上下文 = 上下文.clone();
                let 线程 = spawn(move || {
                    优化方法.优化(&_上下文.初始决策, &mut 目标函数, &mut 操作, &_上下文, &子命令行)
                });
                线程池.push((线程序号, 线程));
            }

            let mut 优化结果列表 = vec![];
            for (线程序号, 线程) in 线程池 {
                优化结果列表.push((线程序号, 线程.join().unwrap()));
            }
            优化结果列表.sort_by(|a, b| a.1.分数.partial_cmp(&b.1.分数).unwrap());
            for (线程序号, 优化结果) in 优化结果列表 {
                print!("线程{}：{}", 线程序号, 优化结果.指标);
            }
        }
    }
    Ok(())
}
