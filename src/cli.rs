use crate::config::Config;
use crate::metric::Metric;
use crate::representation::{Assets, EncodeOutput, RawSequenceMap, WordList};
use clap::{Parser, Subcommand};
use csv::{ReaderBuilder, Reader};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::fs::File;
use std::iter::zip;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Parser)]
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

#[derive(Debug, Serialize, Deserialize)]
struct Test {
    key: char,
    value: f64,
}

impl Cli {
    fn get_reader(path: PathBuf) -> Reader<File> {
        return ReaderBuilder::new().delimiter(b'\t').has_headers(false).from_path(path).unwrap()
    }

    pub fn prepare_file(&self) -> (Config, RawSequenceMap, WordList, Assets) {
        let config_path = self.config.clone().unwrap_or(PathBuf::from("config.yaml"));
        let config_content = fs::read_to_string(&config_path).expect(&format!("文件 {} 不存在", config_path.display()));
        let config: Config = serde_yaml::from_str(&config_content).unwrap();

        let elemets_path = self
            .elements
            .clone()
            .unwrap_or(PathBuf::from("elements.txt"));
        let elements: HashMap<char, String> = Self::get_reader(elemets_path).deserialize().map(|x| x.unwrap()).collect();

        // prepare assets
        let assets_dir = Path::new("assets");
        let cf_path = self
            .character_frequency
            .clone()
            .unwrap_or(assets_dir.join("character_frequency.txt"));
        let character_frequency: HashMap<char, u64> = Self::get_reader(cf_path).deserialize().map(|x| x.unwrap()).collect();
        let wf_path = self
            .word_frequency
            .clone()
            .unwrap_or(assets_dir.join("word_frequency.txt"));
        let word_frequency: HashMap<String, u64> = Self::get_reader(wf_path).deserialize().map(|x| x.unwrap()).collect();
        let keq_path = self
            .key_equivalence
            .clone()
            .unwrap_or(assets_dir.join("key_equivalence.txt"));
        let key_equivalence: HashMap<char, f64> = Self::get_reader(keq_path).deserialize().map(|x| x.unwrap()).collect();
        let peq_path = self
            .pair_equivalence
            .clone()
            .unwrap_or(assets_dir.join("pair_equivalence.txt"));
        let pair_equivalence: HashMap<String, f64> = Self::get_reader(peq_path).deserialize().map(|x| x.unwrap()).collect();
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

    pub fn export_code<T: Serialize>(
        path: &PathBuf,
        original: Vec<T>,
        code: Option<Vec<String>>,
        code_reduced: Option<Vec<String>>,
    ) {
        if code.is_none() && code_reduced.is_none() {
            return;
        }
        let mut writer = csv::WriterBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_path(path)
            .unwrap();
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
        Self::export_code(
            &c_path,
            results.character_list,
            results.characters,
            results.characters_reduced,
        );
        Self::export_code(
            &w_path,
            results.word_list,
            results.words,
            results.words_reduced,
        );
        println!("当前方案评测：");
        print!("{}", metric);
        println!(
            "已完成编码，结果保存在 {} 和 {} 中",
            c_path.display(),
            w_path.display()
        );
    }
}
