pub mod config;
pub mod constraints;
pub mod encoder;
pub mod metaheuristics;
pub mod objectives;
pub mod problem;
pub mod representation;

use config::{Config, ObjectiveConfig, OptimizationConfig, SolverConfig};
use constraints::Constraints;
use encoder::Encoder;
use metaheuristics::Metaheuristic;
use objectives::{metric::Metric, Objective};
use problem::Problem;
use representation::{AssembleList, Assets, Representation};
use representation::{Entry, Frequency, KeyDistribution, PairEquivalence};

use chrono::Local;
use clap::{Parser, Subcommand};
use console_error_panic_hook::set_once;
use csv::ReaderBuilder;
use js_sys::Function;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use serde_with::skip_serializing_none;
use std::fs::{read_to_string, write, create_dir_all};
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
use wasm_bindgen::{prelude::*, JsError};

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
        metric: String,
    },
    BetterSolution {
        metric: String,
        config: String,
        save: bool,
    },
    Elapsed(u128),
    PrepareOutput,
}

// 输出接口的抽象层
//
// 定义了一个特征，指定了所有在退火计算的过程中需要向用户反馈的数据。命令行界面、Web 界面只需要各自实现这些方法，就可向用户报告各种用户数据，实现方式可以很不一样。
pub trait Interface {
    fn post(&self, message: Message);
}

#[wasm_bindgen]
pub struct Web {
    post_message: Function,
    config: Config,
    info: AssembleList,
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
        js_info: JsValue,
        js_assets: JsValue,
    ) -> Result<Web, JsError> {
        set_once();
        let config: Config = from_value(js_config)?;
        let info: AssembleList = from_value(js_info)?;
        let assets: Assets = from_value(js_assets)?;
        Ok(Self {
            post_message,
            config,
            info,
            assets,
        })
    }

    pub fn update_config(&mut self, js_config: JsValue) -> Result<(), JsError> {
        self.config = from_value(js_config)?;
        Ok(())
    }

    pub fn update_info(&mut self, js_info: JsValue) -> Result<(), JsError> {
        self.info = from_value(js_info)?;
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
        let mut encoder = Encoder::new(&representation, self.info.clone(), &self.assets)?;
        let codes = encoder.encode(&representation.initial, &representation);
        let mut objective = Objective::new(&representation, encoder, self.assets.clone())?;
        let (metric, _) = objective.evaluate(&representation.initial, &None);
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
        let constraints = Constraints::new(&representation)?;
        let encoder = Encoder::new(&representation, self.info.clone(), &self.assets)?;
        let objective = Objective::new(&representation, encoder, self.assets.clone())?;
        let mut problem = Problem::new(representation, constraints, objective)?;
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

/// 封装了全部命令行参数，并采用 `derive(Parser)` 来生成解析代码。
#[derive(Parser, Clone)]
#[command(name = "汉字自动拆分系统")]
#[command(author, version, about, long_about)]
#[command(propagate_version = true)]
pub struct CommandLine {
    #[command(subcommand)]
    pub command: Command,
    /// 方案文件，默认为 config.yaml
    pub config: Option<PathBuf>,
    /// 拆分表，默认为 elements.txt
    #[arg(short, long, value_name = "FILE")]
    pub elements: Option<PathBuf>,
    /// 词频表，默认为 assets 目录下的 frequency.txt
    #[arg(short, long, value_name = "FILE")]
    pub frequency: Option<PathBuf>,
    /// 单键用指分布表，默认为 assets 目录下的 key_distribution.txt
    #[arg(short, long, value_name = "FILE")]
    pub key_distribution: Option<PathBuf>,
    /// 双键速度当量表，默认为 assets 目录下的 pair_equivalence.txt
    #[arg(short, long, value_name = "FILE")]
    pub pair_equivalence: Option<PathBuf>,
}

/// 命令行中所有可用的子命令
#[derive(Subcommand, Clone)]
pub enum Command {
    /// 使用方案文件和拆分表计算出字词编码并统计各类评测指标
    Encode,
    /// 评测当前方案的各项指标
    Evaluate,
    /// 基于拆分表和方案文件中的配置优化元素布局
    Optimize,
}

impl CommandLine {
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

    pub fn prepare_file(&self) -> (Config, AssembleList, Assets) {
        let Self {
            config,
            elements,
            frequency,
            key_distribution,
            pair_equivalence,
            ..
        } = self.clone();
        let config_path = config.unwrap_or(PathBuf::from("config.yaml"));
        let config_content = read_to_string(&config_path)
            .unwrap_or_else(|_| panic!("文件 {} 不存在", config_path.display()));
        let config: Config = serde_yaml::from_str(&config_content).unwrap();
        let elements_path = elements.unwrap_or(PathBuf::from("elements.txt"));
        let elements: AssembleList = Self::read(elements_path);

        let assets_dir = Path::new("assets");
        let f_path = frequency.unwrap_or(assets_dir.join("frequency.txt"));
        let frequency: Frequency = Self::read(f_path);
        let keq_path = key_distribution.unwrap_or(assets_dir.join("key_distribution.txt"));
        let key_distribution: KeyDistribution = Self::read(keq_path);
        let peq_path = pair_equivalence.unwrap_or(assets_dir.join("pair_equivalence.txt"));
        let pair_equivalence: PairEquivalence = Self::read(peq_path);
        let assets = Assets {
            frequency,
            key_distribution,
            pair_equivalence,
        };
        (config, elements, assets)
    }

    pub fn write_encode_results(entries: Vec<Entry>) {
        let path = PathBuf::from("code.txt");
        let mut writer = csv::WriterBuilder::new()
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

    pub fn report_metric(metric: Metric) {
        println!("当前方案评测：");
        print!("{}", metric);
    }
}

impl Interface for CommandLine {
    fn post(&self, message: crate::Message) {
        match message {
            Message::PrepareOutput => {
                create_dir_all("output").expect("should be able to create an output directory")
            }
            Message::TrialMax {
                temperature,
                accept_rate,
            } => println!(
                "若温度为 {:.2e}，接受率为 {:.2}%",
                temperature,
                accept_rate * 100.0
            ),
            Message::TrialMin {
                temperature,
                improve_rate,
            } => println!(
                "若温度为 {:.2e}，改进率为 {:.2}%",
                temperature,
                improve_rate * 100.0
            ),
            Message::Parameters { t_max, t_min } => {
                println!("参数寻找完成，从最高温 {} 降到最低温 {}……", t_max, t_min)
            }
            Message::Elapsed(time) => println!("计算一次评测用时：{} μs", time),
            Message::Progress {
                steps,
                temperature,
                metric,
            } => println!(
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
                let config_path = format!("output/{}.yaml", prefix);
                let metric_path = format!("output/{}.txt", prefix);
                println!(
                    "{} 系统搜索到了一个更好的方案，评测指标如下：",
                    time.format("%H:%M:%S")
                );
                print!("{}", metric);
                if save {
                    write(metric_path, metric).unwrap();
                    write(config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
                    println!(
                        "方案文件保存于 {}.yaml 中，评测指标保存于 {}.txt 中",
                        prefix, prefix
                    );
                }
            }
        }
    }
}
