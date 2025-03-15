//! libchai 是使用 Rust 实现的汉字编码输入方案的优化算法。它同时发布为一个 Rust crate 和一个 NPM 模块，前者可以在 Rust 项目中安装为依赖来使用，后者可以通过汉字自动拆分系统的图形界面来使用。
//!
//! chai 是使用 libchai 实现的命令行程序，用户提供方案配置文件、拆分表和评测信息，本程序能够生成编码并评测一系列指标，以及基于退火算法优化元素的布局。

pub mod config;
pub mod data;
pub mod encoders;
pub mod objectives;
pub mod operators;
pub mod optimizers;

use chrono::Local;
use clap::{Parser, Subcommand};
use config::{ObjectiveConfig, OptimizationConfig, SolverConfig, 配置};
use console_error_panic_hook::set_once;
use csv::{ReaderBuilder, WriterBuilder};
use data::{原始可编码对象, 数据};
use data::{原始当量信息, 原始键位分布信息, 码表项};
use encoders::default::默认编码器;
use encoders::编码器;
use js_sys::Function;
use objectives::default::默认目标函数;
use objectives::{metric::Metric, 目标函数};
use operators::default::默认操作;
use optimizers::{优化方法, 优化问题};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use std::fs::{create_dir_all, read_to_string, write, OpenOptions};
use std::io::Write;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use wasm_bindgen::{prelude::*, JsError};

/// 错误类型
#[derive(Debug, Clone)]
pub struct 错误 {
    pub message: String,
}

impl From<String> for 错误 {
    fn from(value: String) -> Self {
        Self { message: value }
    }
}

impl From<&str> for 错误 {
    fn from(value: &str) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl From<错误> for JsError {
    fn from(value: 错误) -> Self {
        JsError::new(&value.message)
    }
}

/// 图形界面参数的定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 图形界面参数 {
    pub 配置: 配置,
    pub 词列表: Vec<原始可编码对象>,
    pub 原始键位分布信息: 原始键位分布信息,
    pub 原始当量信息: 原始当量信息,
}

impl Default for 图形界面参数 {
    fn default() -> Self {
        Self {
            配置: 配置::default(),
            词列表: vec![],
            原始键位分布信息: HashMap::new(),
            原始当量信息: HashMap::new(),
        }
    }
}

/// 向用户反馈的消息类型
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[skip_serializing_none]
pub enum 消息 {
    TrialMax {
        temperature: f64,
        accept_rate: f64,
    },
    TrialMin {
        temperature: f64,
        improve_rate: f64,
    },
    Parameters {
        t_max: f64,
        t_min: f64,
    },
    Progress {
        steps: usize,
        temperature: f64,
        metric: Metric,
    },
    BetterSolution {
        metric: Metric,
        config: 配置,
        save: bool,
    },
    Elapsed(u128),
}

/// 定义了向用户报告消息的接口，用于统一命令行和图形界面的输出方式
///
/// 命令行界面、图形界面只需要各自实现 post 方法，就可向用户报告各种用户数据
pub trait 界面 {
    fn 发送(&self, 消息: 消息);
}

/// 通过图形界面来使用 libchai 的入口，实现了界面特征
#[wasm_bindgen]
pub struct Web {
    回调: Function,
    参数: 图形界面参数,
}

/// 用于在图形界面验证输入的配置是否正确
#[wasm_bindgen]
pub fn validate(js_config: JsValue) -> Result<JsValue, JsError> {
    set_once();
    let config: 配置 = from_value(js_config)?;
    let config_str = serde_yaml::to_string(&config).unwrap();
    Ok(to_value(&config_str)?)
}

#[wasm_bindgen]
impl Web {
    pub fn new(回调: Function) -> Web {
        set_once();
        let 参数 = 图形界面参数::default();
        Self { 回调, 参数 }
    }

    pub fn sync(&mut self, 前端参数: JsValue) -> Result<(), JsError> {
        self.参数 = from_value(前端参数)?;
        Ok(())
    }

    pub fn encode_evaluate(&self, 前端目标函数配置: JsValue) -> Result<JsValue, JsError> {
        let 目标函数配置: ObjectiveConfig = from_value(前端目标函数配置)?;
        let 图形界面参数 {
            mut 配置,
            原始键位分布信息,
            原始当量信息,
            词列表,
        } = self.参数.clone();
        配置.optimization = Some(OptimizationConfig {
            objective: 目标函数配置,
            constraints: None,
            metaheuristic: None,
        });
        let 数据 = 数据::新建(配置, 词列表, 原始键位分布信息, 原始当量信息)?;
        let mut 编码器 = 默认编码器::新建(&数据)?;
        let mut 编码结果 = 编码器.编码(&数据.初始映射, &None).clone();
        let 码表 = 数据.生成码表(&编码结果);
        let mut 目标函数 = 默认目标函数::新建(&数据)?;
        let (指标, _) = 目标函数.计算(&mut 编码结果);
        Ok(to_value(&(码表, 指标))?)
    }

    pub fn optimize(&self) -> Result<(), JsError> {
        let 图形界面参数 {
            配置,
            原始键位分布信息,
            原始当量信息,
            词列表,
        } = self.参数.clone();
        let 优化方法配置 = 配置.clone().optimization.unwrap().metaheuristic.unwrap();
        let 数据 = 数据::新建(配置, 词列表, 原始键位分布信息, 原始当量信息)?;
        let 编码器 = 默认编码器::新建(&数据)?;
        let 目标函数 = 默认目标函数::新建(&数据)?;
        let 操作 = 默认操作::新建(&数据)?;
        let mut 问题 = 优化问题::新建(数据, 编码器, 目标函数, 操作);
        let SolverConfig::SimulatedAnnealing(退火) = 优化方法配置;
        退火.优化(&mut 问题, self);
        Ok(())
    }
}

impl 界面 for Web {
    fn 发送(&self, message: 消息) {
        let js_message = to_value(&message).unwrap();
        self.回调.call1(&JsValue::null(), &js_message).unwrap();
    }
}

/// 命令行参数的定义
#[derive(Parser, Clone)]
#[command(name = "汉字自动拆分系统")]
#[command(author, version, about, long_about)]
#[command(propagate_version = true)]
pub struct 命令行参数 {
    #[command(subcommand)]
    pub command: 命令,
    /// 方案文件，默认为 config.yaml
    pub config: Option<PathBuf>,
    /// 频率序列表，默认为 elements.txt
    #[arg(short, long, value_name = "FILE")]
    pub encodables: Option<PathBuf>,
    /// 单键用指分布表，默认为 assets 目录下的 key_distribution.txt
    #[arg(short, long, value_name = "FILE")]
    pub key_distribution: Option<PathBuf>,
    /// 双键速度当量表，默认为 assets 目录下的 pair_equivalence.txt
    #[arg(short, long, value_name = "FILE")]
    pub pair_equivalence: Option<PathBuf>,
    /// 线程数，默认为 1
    #[arg(short, long)]
    pub threads: Option<usize>,
}

/// 命令行中所有可用的子命令
#[derive(Subcommand, Clone)]
pub enum 命令 {
    /// 使用方案文件和拆分表计算出字词编码并统计各类评测指标
    Encode,
    /// 基于拆分表和方案文件中的配置优化元素布局
    Optimize,
}

/// 通过命令行来使用 libchai 的入口，实现了界面特征
pub struct 命令行 {
    pub 参数: 命令行参数,
    pub 输出目录: PathBuf,
}

impl 命令行 {
    pub fn 新建(args: 命令行参数, maybe_output_dir: Option<PathBuf>) -> Self {
        let output_dir = maybe_output_dir.unwrap_or_else(|| {
            let time = Local::now().format("%m-%d+%H_%M_%S").to_string();
            PathBuf::from(format!("output-{}", time))
        });
        create_dir_all(output_dir.clone()).unwrap();
        Self {
            参数: args,
            输出目录: output_dir,
        }
    }

    pub fn 读取(name: &str) -> 数据 {
        let config = format!("examples/{}.yaml", name);
        let elements = format!("examples/{}.txt", name);
        let 参数 = 命令行参数 {
            command: 命令::Optimize,
            config: Some(PathBuf::from(config)),
            encodables: Some(PathBuf::from(elements)),
            key_distribution: None,
            pair_equivalence: None,
            threads: None,
        };
        let cli = 命令行::新建(参数, None);
        cli.准备数据()
    }

    fn read<I, T>(path: PathBuf) -> T
    where
        I: for<'de> Deserialize<'de>,
        T: FromIterator<I>,
    {
        let mut reader = ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .flexible(true)
            .from_path(path)
            .unwrap();
        reader.deserialize().map(|x| x.unwrap()).collect()
    }

    pub fn 准备数据(&self) -> 数据 {
        let 命令行参数 {
            config,
            encodables: elements,
            key_distribution,
            pair_equivalence,
            ..
        } = self.参数.clone();
        let config_path = config.unwrap_or(PathBuf::from("config.yaml"));
        let config_content = read_to_string(&config_path)
            .unwrap_or_else(|_| panic!("文件 {} 不存在", config_path.display()));
        let config: 配置 = serde_yaml::from_str(&config_content).unwrap();
        let elements_path = elements.unwrap_or(PathBuf::from("elements.txt"));
        let encodables: Vec<原始可编码对象> = Self::read(elements_path);

        let assets_dir = Path::new("assets");
        let keq_path = key_distribution.unwrap_or(assets_dir.join("key_distribution.txt"));
        let key_distribution: 原始键位分布信息 = Self::read(keq_path);
        let peq_path = pair_equivalence.unwrap_or(assets_dir.join("pair_equivalence.txt"));
        let pair_equivalence: 原始当量信息 = Self::read(peq_path);
        数据::新建(config, encodables, key_distribution, pair_equivalence).unwrap()
    }

    pub fn 输出编码结果(&self, entries: Vec<码表项>) {
        let path = self.输出目录.join("编码.txt");
        let mut writer = WriterBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_path(&path)
            .unwrap();
        for 码表项 {
            name,
            full,
            full_rank,
            short,
            short_rank,
        } in entries
        {
            writer
                .serialize((&name, &full, &full_rank, &short, &short_rank))
                .unwrap();
        }
        writer.flush().unwrap();
        println!("已完成编码，结果保存在 {} 中", path.clone().display());
    }

    pub fn 输出评测指标(&self, metric: Metric) {
        let path = self.输出目录.join("评测指标.yaml");
        print!("{}", metric);
        let metric_str = serde_yaml::to_string(&metric).unwrap();
        write(&path, metric_str).unwrap();
    }

    pub fn 生成子命令行(&self, index: usize) -> 命令行 {
        let child_dir = self.输出目录.join(format!("{}", index));
        命令行::新建(self.参数.clone(), Some(child_dir))
    }
}

impl 界面 for 命令行 {
    fn 发送(&self, message: 消息) {
        let mut writer: Box<dyn Write> = if self.参数.threads.is_some() {
            let log_path = self.输出目录.join("log.txt");
            let file = OpenOptions::new()
                .create(true) // 如果文件不存在，则创建
                .append(true) // 追加写入，不覆盖原有内容
                .open(log_path)
                .expect("Failed to open file");
            Box::new(file)
        } else {
            Box::new(std::io::stdout())
        };
        let result = match message {
            消息::TrialMax {
                temperature,
                accept_rate,
            } => writeln!(
                &mut writer,
                "若温度为 {:.2e}，接受率为 {:.2}%",
                temperature,
                accept_rate * 100.0
            ),
            消息::TrialMin {
                temperature,
                improve_rate,
            } => writeln!(
                &mut writer,
                "若温度为 {:.2e}，改进率为 {:.2}%",
                temperature,
                improve_rate * 100.0
            ),
            消息::Parameters { t_max, t_min } => writeln!(
                &mut writer,
                "参数寻找完成，从最高温 {} 降到最低温 {}……",
                t_max, t_min
            ),
            消息::Elapsed(time) => writeln!(&mut writer, "计算一次评测用时：{} μs", time),
            消息::Progress {
                steps,
                temperature,
                metric,
            } => writeln!(
                &mut writer,
                "已执行 {} 步，当前温度为 {:.2e}，当前评测指标如下：\n{}",
                steps, temperature, metric
            ),
            消息::BetterSolution {
                metric,
                config,
                save,
            } => {
                let time = Local::now();
                let prefix = time.format("%m-%d+%H_%M_%S_%3f").to_string();
                let config_path = self.输出目录.join(format!("{}.yaml", prefix));
                let metric_path = self.输出目录.join(format!("{}.metric.yaml", prefix));
                let mut res1 = writeln!(
                    &mut writer,
                    "{} 系统搜索到了一个更好的方案，评测指标如下：\n{}",
                    time.format("%H:%M:%S"),
                    metric
                );
                let config = serde_yaml::to_string(&config).unwrap();
                let metric = serde_yaml::to_string(&metric).unwrap();
                if save {
                    write(metric_path, metric).unwrap();
                    write(config_path, config).unwrap();
                    let res2 = writeln!(
                        &mut writer,
                        "方案文件保存于 {}.yaml 中，评测指标保存于 {}.metric.yaml 中",
                        prefix, prefix
                    );
                    res1 = res1.and(res2);
                }
                res1
            }
        };
        result.unwrap();
    }
}
