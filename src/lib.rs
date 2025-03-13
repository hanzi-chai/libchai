pub mod config;
pub mod encoders;
pub mod metaheuristics;
pub mod objectives;
pub mod problems;
pub mod representation;

use config::{Config, ObjectiveConfig, OptimizationConfig, SolverConfig};
use encoders::default::DefaultEncoder;
use encoders::Encoder;
use metaheuristics::Metaheuristic;
use objectives::default::DefaultObjective;
use objectives::{metric::Metric, Objective};
use problems::default::DefaultProblem;
use representation::{Assets, RawEncodable, Representation};
use representation::{Entry, KeyDistribution, PairEquivalence};

use chrono::Local;
use clap::{Parser, Subcommand};
use console_error_panic_hook::set_once;
use csv::{ReaderBuilder, WriterBuilder};
use js_sys::Function;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use serde_with::skip_serializing_none;
use std::fs::{create_dir_all, read_to_string, write, OpenOptions};
use std::io::Write;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use wasm_bindgen::{prelude::*, JsError};

/// 错误类型
#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self { message: value }
    }
}

impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

impl From<Error> for JsError {
    fn from(value: Error) -> Self {
        JsError::new(&value.message)
    }
}

/// 向用户反馈的消息类型
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[skip_serializing_none]
pub enum Message {
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
        config: Config,
        save: bool,
    },
    Elapsed(u128),
}

// 输出接口的抽象层
//
// 命令行界面、Web 界面只需要各自实现 post 方法，就可向用户报告各种用户数据
pub trait Interface {
    fn post(&self, message: Message);
}

#[wasm_bindgen]
pub struct Web {
    post_message: Function,
    config: Config,
    assets: Assets,
}

#[wasm_bindgen]
pub fn validate(js_config: JsValue) -> Result<JsValue, JsError> {
    set_once();
    let config: Config = from_value(js_config)?;
    let config_str = serde_yaml::to_string(&config).unwrap();
    Ok(to_value(&config_str)?)
}

#[wasm_bindgen]
impl Web {
    pub fn new(
        post_message: Function,
        js_config: JsValue,
        js_assets: JsValue,
    ) -> Result<Web, JsError> {
        set_once();
        let config: Config = from_value(js_config)?;
        let assets: Assets = from_value(js_assets)?;
        Ok(Self {
            post_message,
            config,
            assets,
        })
    }

    pub fn update_config(&mut self, js_config: JsValue) -> Result<(), JsError> {
        self.config = from_value(js_config)?;
        Ok(())
    }

    pub fn update_assets(&mut self, js_assets: JsValue) -> Result<(), JsError> {
        self.assets = from_value(js_assets)?;
        Ok(())
    }

    pub fn encode_evaluate(&self, js_objective: JsValue) -> Result<JsValue, JsError> {
        let objective: ObjectiveConfig = from_value(js_objective)?;
        let mut config = self.config.clone();
        config.optimization = Some(OptimizationConfig {
            objective,
            constraints: None,
            metaheuristic: None,
        });
        let representation = Representation::new(config)?;
        let Assets {
            key_distribution,
            pair_equivalence,
            encodables,
        } = self.assets.clone();
        let mut encoder = DefaultEncoder::new(&representation, encodables)?;
        let buffer = encoder.encode(&representation.initial, &None).clone();
        let codes = representation.export_code(&buffer, &encoder.encodables);
        let mut objective = DefaultObjective::new(
            &representation,
            key_distribution,
            pair_equivalence,
            codes.len(),
        )?;
        let (metric, _) = objective.evaluate(&mut encoder, &representation.initial, &None);
        Ok(to_value(&(codes, metric))?)
    }

    pub fn optimize(&self) -> Result<(), JsError> {
        let solver = self
            .config
            .optimization
            .as_ref()
            .unwrap()
            .metaheuristic
            .as_ref()
            .unwrap();
        let representation = Representation::new(self.config.clone())?;
        let Assets {
            key_distribution,
            pair_equivalence,
            encodables,
        } = self.assets.clone();
        let encoder = DefaultEncoder::new(&representation, encodables)?;
        let objective = DefaultObjective::new(
            &representation,
            key_distribution,
            pair_equivalence,
            self.assets.encodables.len(),
        )?;
        let mut problem = DefaultProblem::new(representation, objective, encoder)?;
        match solver {
            SolverConfig::SimulatedAnnealing(config) => {
                config.solve(&mut problem, self);
            }
        }
        Ok(())
    }
}

impl Interface for Web {
    fn post(&self, message: Message) {
        let js_message = to_value(&message).unwrap();
        self.post_message
            .call1(&JsValue::null(), &js_message)
            .unwrap();
    }
}

/// chai 是一个使用 Rust 编写的命令行程序。用户提供拆分表以及方案配置文件，本程序能够生成编码并评测一系列指标，以及基于退火算法优化元素的布局。
#[derive(Parser, Clone)]
#[command(name = "汉字自动拆分系统")]
#[command(author, version, about, long_about)]
#[command(propagate_version = true)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
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

impl Args {
    pub fn 生成(name: &str) -> Self {
        let config = format!("examples/{}.yaml", name);
        let elements = format!("examples/{}.txt", name);
        Args {
            command: Command::Optimize,
            config: Some(PathBuf::from(config)),
            encodables: Some(PathBuf::from(elements)),
            key_distribution: None,
            pair_equivalence: None,
            threads: None,
        }
    }
}

/// 命令行中所有可用的子命令
#[derive(Subcommand, Clone)]
pub enum Command {
    /// 使用方案文件和拆分表计算出字词编码并统计各类评测指标
    Encode,
    /// 基于拆分表和方案文件中的配置优化元素布局
    Optimize,
}

pub struct CommandLine {
    pub args: Args,
    pub output_dir: PathBuf,
}

impl CommandLine {
    pub fn new(args: Args, maybe_output_dir: Option<PathBuf>) -> Self {
        let output_dir = maybe_output_dir.unwrap_or_else(|| {
            let time = Local::now().format("%m-%d+%H_%M_%S").to_string();
            PathBuf::from(format!("output-{}", time))
        });
        create_dir_all(output_dir.clone()).unwrap();
        Self { args, output_dir }
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

    pub fn prepare_file(&self) -> (Config, Assets) {
        let Args {
            config,
            encodables: elements,
            key_distribution,
            pair_equivalence,
            ..
        } = self.args.clone();
        let config_path = config.unwrap_or(PathBuf::from("config.yaml"));
        let config_content = read_to_string(&config_path)
            .unwrap_or_else(|_| panic!("文件 {} 不存在", config_path.display()));
        let config: Config = serde_yaml::from_str(&config_content).unwrap();
        let elements_path = elements.unwrap_or(PathBuf::from("elements.txt"));
        let elements: Vec<RawEncodable> = Self::read(elements_path);

        let assets_dir = Path::new("assets");
        let keq_path = key_distribution.unwrap_or(assets_dir.join("key_distribution.txt"));
        let key_distribution: KeyDistribution = Self::read(keq_path);
        let peq_path = pair_equivalence.unwrap_or(assets_dir.join("pair_equivalence.txt"));
        let pair_equivalence: PairEquivalence = Self::read(peq_path);
        let assets = Assets {
            encodables: elements,
            key_distribution,
            pair_equivalence,
        };
        (config, assets)
    }

    pub fn write_encode_results(&self, entries: Vec<Entry>) {
        let path = self.output_dir.join("编码.txt");
        let mut writer = WriterBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_path(&path)
            .unwrap();
        for Entry {
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

    pub fn report_metric(&self, metric: Metric) {
        let path = self.output_dir.join("评测指标.yaml");
        print!("{}", metric);
        let metric_str = serde_yaml::to_string(&metric).unwrap();
        write(&path, metric_str).unwrap();
    }

    pub fn make_child(&self, index: usize) -> CommandLine {
        let child_dir = self.output_dir.join(format!("{}", index));
        CommandLine::new(self.args.clone(), Some(child_dir))
    }
}

impl Interface for CommandLine {
    fn post(&self, message: Message) {
        let mut writer: Box<dyn Write> = if let Some(_) = &self.args.threads {
            let log_path = self.output_dir.join("log.txt");
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
            Message::TrialMax {
                temperature,
                accept_rate,
            } => writeln!(
                &mut writer,
                "若温度为 {:.2e}，接受率为 {:.2}%",
                temperature,
                accept_rate * 100.0
            ),
            Message::TrialMin {
                temperature,
                improve_rate,
            } => writeln!(
                &mut writer,
                "若温度为 {:.2e}，改进率为 {:.2}%",
                temperature,
                improve_rate * 100.0
            ),
            Message::Parameters { t_max, t_min } => writeln!(
                &mut writer,
                "参数寻找完成，从最高温 {} 降到最低温 {}……",
                t_max, t_min
            ),
            Message::Elapsed(time) => writeln!(&mut writer, "计算一次评测用时：{} μs", time),
            Message::Progress {
                steps,
                temperature,
                metric,
            } => writeln!(
                &mut writer,
                "已执行 {} 步，当前温度为 {:.2e}，当前评测指标如下：\n{}",
                steps, temperature, metric
            ),
            Message::BetterSolution {
                metric,
                config,
                save,
            } => {
                let time = Local::now();
                let prefix = time.format("%m-%d+%H_%M_%S_%3f").to_string();
                let config_path = self.output_dir.join(format!("{}.yaml", prefix));
                let metric_path = self.output_dir.join(format!("{}.metric.yaml", prefix));
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
