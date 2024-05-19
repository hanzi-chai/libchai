//! 命令行界面
//!
//! 此模块基于 `clap` 包实现了命令行的参数设置，标准输出以及文件读写。
//!

use crate::config::Config;
use crate::interface::Interface;
use crate::objectives::metric::Metric;
use crate::representation::{
    AssembleList, Assets, Entry, Frequency, KeyDistribution, PairEquivalence,
};
use chrono::Local;
use clap::{Parser, Subcommand};
use csv::ReaderBuilder;
use serde::Deserialize;
use std::iter::FromIterator;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// 封装了全部命令行参数，并采用 `derive(Parser)` 来生成解析代码。
#[derive(Parser, Clone)]
#[command(name = "汉字自动拆分系统")]
#[command(author, version, about, long_about)]
#[command(propagate_version = true)]
pub struct Cli {
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

impl Cli {
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
        let config_content = fs::read_to_string(&config_path)
            .expect(&format!("文件 {} 不存在", config_path.display()));
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
        return (config, elements, assets);
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
                .serialize((&name, &full, &full_rank.abs(), &short, &short_rank.abs()))
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

impl Interface for Cli {
    fn prepare_output(&self) {
        let _ = fs::create_dir_all("output").expect("should be able to create an output directory");
    }

    fn init_autosolve(&self) {
        println!("开始寻找参数……");
    }

    fn report_trial_t_max(&self, temperature: f64, accept_rate: f64) {
        println!(
            "若温度为 {:.2e}，接受率为 {:.2}%",
            temperature,
            accept_rate * 100.0
        );
    }

    fn report_t_max(&self, temperature: f64) {
        println!(
            "接受率已符合标准，体系最高温度估计为：t_max = {:.2e}",
            temperature
        );
    }

    fn report_trial_t_min(&self, temperature: f64, improve_rate: f64) {
        println!(
            "若温度为 {:.2e}，改进率为 {:.2}%",
            temperature,
            improve_rate * 100.0
        );
    }

    fn report_t_min(&self, temperature: f64) {
        println!(
            "改进率已符合标准，体系最低温度估计为：t_min = {:.2e}",
            temperature
        );
    }

    fn report_parameters(&self, t_max: f64, t_min: f64, steps: usize) {
        println!(
            "参数寻找完成，将在 {} 步内从最高温 {} 降到最低温 {}……",
            steps, t_max, t_min
        );
    }

    fn report_elapsed(&self, time: u128) {
        println!("计算一次评测用时：{} μs", time);
    }

    fn report_schedule(&self, step: usize, temperature: f64, metric: String) {
        println!(
            "优化已执行 {} 步，当前温度为 {:.2e}，当前评测指标如下：",
            step, temperature
        );
        println!("{}", metric);
    }

    fn report_solution(&self, config: Config, metric: String, save: bool) {
        let time = Local::now();
        let prefix = format!("{}", time.format("%m-%d+%H_%M_%S_%3f"));
        let config_path = format!("output/{}.yaml", prefix);
        let metric_path = format!("output/{}.txt", prefix);
        println!(
            "{} 系统搜索到了一个更好的方案，评测指标如下：",
            time.format("%H:%M:%S")
        );
        print!("{}", metric);
        if save {
            fs::write(metric_path, metric).unwrap();
            fs::write(config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
            println!(
                "方案文件保存于 {}.yaml 中，评测指标保存于 {}.txt 中",
                prefix, prefix
            );
        }
    }
}
