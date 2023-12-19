use crate::metric::Metric;
use crate::representation::EncodeOutput;
use crate::config::Config;
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

    /// 词表，默认为 words.txt
    #[arg(long, value_name = "FILE")]
    pub words: Option<PathBuf>,

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

#[derive(Subcommand, Clone)]
pub enum Command {
    /// 使用方案文件和拆分表计算出字词编码并统计各类评测指标
    Encode,
    /// 基于拆分表和方案文件中的配置优化元素布局
    Optimize,
}

pub type RawSequenceMap = HashMap<char, Vec<String>>;
pub type WordList = Vec<String>;
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

impl Cli {
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
        path: PathBuf,
        kparser: fn(String) -> T,
        vparser: fn(String) -> S,
    ) -> HashMap<T, S> {
        let content = Self::get_file(path);
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

    pub fn prepare_file(&self) -> (Config, RawSequenceMap, WordList, Assets) {
        let config_path = self.config.clone().unwrap_or(PathBuf::from("config.yaml"));
        let config: Config = serde_yaml::from_str(&Self::get_file(config_path)).unwrap();

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

        let elemets_path = self.elements.clone().unwrap_or(PathBuf::from("elements.txt"));
        let elements: RawSequenceMap = Self::parse_hashmap(elemets_path, to_char, to_string_list);

        // prepare assets
        let assets_dir = Path::new("assets");
        let cf_path = self
            .character_frequency.clone()
            .unwrap_or(assets_dir.join("character_frequency.txt"));
        let character_frequency = Self::parse_hashmap(cf_path, to_char, to_u64);
        let wf_path = self
            .word_frequency.clone()
            .unwrap_or(assets_dir.join("word_frequency.txt"));
        let word_frequency = Self::parse_hashmap(wf_path, identity, to_u64);
        let keq_path = self
            .key_equivalence.clone()
            .unwrap_or(assets_dir.join("key_equivalence.txt"));
        let key_equivalence = Self::parse_hashmap(keq_path, to_char, to_f64);
        let peq_path = self
            .pair_equivalence.clone()
            .unwrap_or(assets_dir.join("pair_equivalence.txt"));
        let pair_equivalence = Self::parse_hashmap(peq_path, to_char_pair, to_f64);
        let words = if let Some(_) = self.words {
            vec![]
        } else {
            word_frequency.clone().into_keys().collect()
        };
        let assets = Assets {
            character_frequency,
            word_frequency,
            key_equivalence,
            pair_equivalence,
        };
        return (config, elements, words, assets);
    }

    pub fn export_code<T: Serialize>(path: &PathBuf, original: Vec<T>, code: Option<Vec<String>>, code_reduced: Option<Vec<String>>) {
        if code.is_none() && code_reduced.is_none() {
            return;
        }
        let mut writer = csv::WriterBuilder::new().delimiter(b'\t').has_headers(false).from_path(path).unwrap();
        if let (Some(code), Some(code_reduced)) = (code.as_ref(), code_reduced.as_ref()) {
            for (orig, (c, cr)) in zip(original, zip(code, code_reduced)) {
                writer.serialize((&orig, &c, &cr)).unwrap();
            }
        } else {
            for (orig, c) in zip(original, code.or(code_reduced).unwrap()) {
                writer.serialize((&orig, &c)).unwrap();
            }
        }
        writer.flush().unwrap();
    }

    pub fn write_encode_results(metric: Metric, results: EncodeOutput) {
        let c_path = PathBuf::from("characters.txt");
        let w_path = PathBuf::from("words.txt");
        Self::export_code(&c_path, results.character_list, results.characters, results.characters_reduced);
        Self::export_code(&w_path, results.word_list, results.words, results.words_reduced);
        println!("当前方案评测：");
        print!("{}", metric);
        println!("已完成编码，结果保存在 {} 和 {} 中", c_path.display(), w_path.display());
    }
}
