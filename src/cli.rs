use crate::encoder::{Code, EncodeResults};
use crate::metric::Metric;
use crate::{config::Config, encoder::RawElements};
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::convert::identity;
use std::iter::zip;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "汉字自动拆分系统")]
#[command(author, version, about, long_about)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// 方案文件，默认为 config.yaml
    pub config: Option<PathBuf>,

    /// 拆分表，默认为 elements.txt
    #[arg(short, long, value_name = "FILE")]
    pub elements: Option<PathBuf>,

    /// 字频表，默认为 assets 目录下的 character_frequency.txt
    #[arg(short, long, value_name = "FILE")]
    pub character_frequency: Option<PathBuf>,

    /// 词频表，默认为 assets 目录下的 word_frequency.txt
    #[arg(short, long, value_name = "FILE")]
    pub word_frequency: Option<PathBuf>,

    /// 单键用指当量表，默认为 assets 目录下的 key_equivalence.txt
    #[arg(short, long, value_name = "FILE")]
    pub key_equivalence: Option<PathBuf>,

    /// 双键速度当量表，默认为 assets 目录下的 pair_equivalence.txt
    #[arg(short, long, value_name = "FILE")]
    pub pair_equivalence: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Command {
    /// 使用方案文件和拆分表计算出字词编码并统计各类评测指标
    Encode,
    /// 基于拆分表和方案文件中的配置优化元素布局
    Optimize,
}

fn get_file(path: PathBuf) -> String {
    if path.exists() {
        fs::read_to_string(path).unwrap()
    } else {
        let mut dir = env::current_exe().unwrap();
        dir.pop();
        let abspath = dir.join(&path);
        if abspath.exists() {
            fs::read_to_string(abspath).unwrap().trim().to_string()
        } else {
            panic!("无法找到文件：{}", path.display())
        }
    }
}

fn parse_hashmap<T: Eq + std::hash::Hash, S>(
    content: &String,
    kparser: fn(String) -> T,
    vparser: fn(String) -> S,
) -> HashMap<T, S> {
    let mut hashmap: HashMap<T, S> = HashMap::new();
    for line in content.split('\n') {
        let fields: Vec<&str> = line.trim().split('\t').collect();
        hashmap.insert(
            kparser(fields[0].to_string()),
            vparser(fields[1].to_string()),
        );
    }
    hashmap
}

pub type KeyEquivalence = HashMap<char, f64>;
pub type PairEquivalence = HashMap<(char, char), f64>;
pub type Frequency<T> = HashMap<T, u64>;

#[derive(Debug)]
pub struct Assets {
    pub character_frequency: Frequency<char>,
    pub word_frequency: Frequency<String>,
    pub key_equivalence: KeyEquivalence,
    pub pair_equivalence: PairEquivalence,
}

pub fn prepare_file() -> (Config, RawElements, Assets, Command) {
    let args = Cli::parse();
    let config_path = args.config.unwrap_or(PathBuf::from("config.yaml"));
    let config: Config = serde_yaml::from_str(&get_file(config_path)).unwrap();

    // small parsers for TSV file
    let to_u64 = |x: String| x.parse::<u64>().unwrap();
    let to_f64 = |x: String| x.parse::<f64>().unwrap();
    let to_char = |x: String| x.chars().next().unwrap();
    let to_char_pair = |x: String| {
        let mut it = x.chars();
        let first = it.next().unwrap();
        let second = it.next().unwrap();
        (first, second)
    };
    let to_string_list = |x: String| x.split(' ').map(|x| x.to_string()).collect();

    let elemets_path = args.elements.unwrap_or(PathBuf::from("elements.txt"));
    let elements: RawElements = parse_hashmap(&get_file(elemets_path), to_char, to_string_list);

    // prepare assets
    let assets_dir = Path::new("assets");
    let cf_path = args
        .character_frequency
        .unwrap_or(assets_dir.join("character_frequency.txt"));
    let character_frequency = parse_hashmap(&get_file(cf_path), to_char, to_u64);
    let wf_path = args
        .word_frequency
        .unwrap_or(assets_dir.join("word_frequency.txt"));
    let word_frequency = parse_hashmap(&get_file(wf_path), identity, to_u64);
    let keq_path = args
        .key_equivalence
        .unwrap_or(assets_dir.join("key_equivalence.txt"));
    let key_equivalence = parse_hashmap(&get_file(keq_path), to_char, to_f64);
    let peq_path = args
        .pair_equivalence
        .unwrap_or(assets_dir.join("pair_equivalence.txt"));
    let pair_equivalence = parse_hashmap(&get_file(peq_path), to_char_pair, to_f64);
    let assets = Assets {
        character_frequency,
        word_frequency,
        key_equivalence,
        pair_equivalence,
    };
    return (config, elements, assets, args.command.unwrap_or(Command::Encode));
}

pub fn export_code<T: Serialize>(path: &PathBuf, code: Option<Code<T>>, code_reduced: Option<Code<T>>) {
    if code.is_none() && code_reduced.is_none() {
        return;
    }
    let mut writer = csv::WriterBuilder::new().delimiter(b'\t').has_headers(false).from_path(path).unwrap();
    if let (Some(code), Some(code_reduced)) = (code.as_ref(), code_reduced.as_ref()) {
        for (c, cr) in zip(code, code_reduced) {
            writer.serialize((&c.original, &c.code, &cr.code)).unwrap();
        }
    } else {
        for c in code.or(code_reduced).unwrap() {
            writer.serialize((&c.original, &c.code)).unwrap();
        }
    }
    writer.flush().unwrap();
}

pub fn write_encode_results(metric: Metric, results: EncodeResults) {
    let c_path = PathBuf::from("characters.txt");
    let w_path = PathBuf::from("words.txt");
    export_code(&c_path, results.characters, results.characters_reduced);
    export_code(&w_path, results.words, results.words_reduced);
    println!("当前方案评测：");
    print!("{}", metric);
    println!("已完成编码，结果保存在 {} 和 {} 中", c_path.display(), w_path.display());
}
