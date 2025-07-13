use crate::config::配置;
use crate::contexts::default::默认上下文;
use crate::interfaces::{消息, 界面};
use crate::{
    原始可编码对象, 原始当量信息, 原始键位分布信息, 码表项
};
use chrono::Local;
use clap::{Parser, Subcommand};
use csv::{ReaderBuilder, WriterBuilder};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs::{create_dir_all, read_to_string, write, OpenOptions};
use std::io::Write;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};

pub trait 命令行参数: Clone {
    fn 是否为多线程(&self) -> bool;
}

/// 命令行参数的定义
#[derive(Parser, Clone)]
#[command(name = "汉字自动拆分系统")]
#[command(author, version, about, long_about)]
#[command(propagate_version = true)]
pub struct 默认命令行参数 {
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

impl 命令行参数 for 默认命令行参数 {
    fn 是否为多线程(&self) -> bool {
        self.threads.is_some()
    }
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
pub struct 命令行<P: 命令行参数> {
    pub 参数: P,
    pub 输出目录: PathBuf,
}

impl<P: 命令行参数> 命令行<P> {
    pub fn 新建(args: P, maybe_output_dir: Option<PathBuf>) -> Self {
        let output_dir = maybe_output_dir.unwrap_or_else(|| {
            let time = Local::now().format("%m-%d+%H_%M_%S").to_string();
            PathBuf::from(format!("output-{time}"))
        });
        create_dir_all(output_dir.clone()).unwrap();
        Self {
            参数: args,
            输出目录: output_dir,
        }
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

    pub fn 输出评测指标<M: Display + Serialize>(&self, metric: M) {
        let path = self.输出目录.join("评测指标.yaml");
        print!("{metric}");
        let metric_str = serde_yaml::to_string(&metric).unwrap();
        write(&path, metric_str).unwrap();
    }

    pub fn 生成子命令行(&self, index: usize) -> 命令行<P> {
        let child_dir = self.输出目录.join(format!("{index}"));
        命令行::新建(self.参数.clone(), Some(child_dir))
    }
}

pub fn 从命令行参数创建(参数: &默认命令行参数) -> 默认上下文 {
    let 默认命令行参数 {
        config,
        encodables: elements,
        key_distribution,
        pair_equivalence,
        ..
    } = 参数.clone();
    let config_path = config.unwrap_or(PathBuf::from("config.yaml"));
    let config_content = read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("文件 {} 不存在", config_path.display()));
    let config: 配置 = serde_yaml::from_str(&config_content).unwrap();
    let elements_path = elements.unwrap_or(PathBuf::from("elements.txt"));
    let encodables: Vec<原始可编码对象> = 命令行::<默认命令行参数>::read(elements_path);
    let assets_dir = Path::new("assets");
    let keq_path = key_distribution.unwrap_or(assets_dir.join("key_distribution.txt"));
    let key_distribution: 原始键位分布信息 = 命令行::<默认命令行参数>::read(keq_path);
    let peq_path = pair_equivalence.unwrap_or(assets_dir.join("pair_equivalence.txt"));
    let pair_equivalence: 原始当量信息 = 命令行::<默认命令行参数>::read(peq_path);
    默认上下文::新建(config, encodables, key_distribution, pair_equivalence).unwrap()
}

impl<P: 命令行参数> 界面 for 命令行<P> {
    fn 发送(&self, message: 消息) {
        let mut writer: Box<dyn Write> = if self.参数.是否为多线程() {
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
                "若温度为 {temperature:.2e}，接受率为 {:.2}%",
                accept_rate * 100.0
            ),
            消息::TrialMin {
                temperature,
                improve_rate,
            } => writeln!(
                &mut writer,
                "若温度为 {temperature:.2e}，改进率为 {:.2}%",
                improve_rate * 100.0
            ),
            消息::Parameters { t_max, t_min } => writeln!(
                &mut writer,
                "参数寻找完成，从最高温 {t_max} 降到最低温 {t_min}……"
            ),
            消息::Elapsed { time } => writeln!(&mut writer, "计算一次评测用时：{time} μs"),
            消息::Progress {
                steps,
                temperature,
                metric,
            } => writeln!(
                &mut writer,
                "已执行 {steps} 步，当前温度为 {temperature:.2e}，当前评测指标如下：\n{metric}",
            ),
            消息::BetterSolution {
                metric,
                config,
                save,
            } => {
                let 时刻 = Local::now();
                let 时间戳 = 时刻.format("%m-%d+%H_%M_%S_%3f").to_string();
                let 配置路径 = self.输出目录.join(format!("{时间戳}.yaml"));
                let 指标路径 = self.输出目录.join(format!("{时间戳}.txt"));
                if save {
                    write(指标路径, metric.clone()).unwrap();
                    write(配置路径, config).unwrap();
                    writeln!(
                        &mut writer,
                        "方案文件保存于 {时间戳}.yaml 中，评测指标保存于 {时间戳}.metric.yaml 中",
                    )
                    .unwrap();
                }
                writeln!(
                    &mut writer,
                    "{} 系统搜索到了一个更好的方案，评测指标如下：\n{}",
                    时刻.format("%H:%M:%S"),
                    metric
                )
            }
        };
        result.unwrap()
    }
}
