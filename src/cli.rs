use crate::{config::Config, encoder::RawElements};
use clap::Parser;
use std::collections::HashMap;
use std::convert::identity;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(name = "汉字自动拆分系统")]
#[command(author, version, about, long_about = None)]
pub struct Args {
    pub config: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    pub elements: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    pub character_frequency: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    pub word_frequency: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    pub key_equivalence: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    pub pair_equivalence: Option<PathBuf>,
}

fn get_file(path: PathBuf) -> String {
    if path.exists() {
        fs::read_to_string(path).unwrap()
    } else {
        let mut dir = env::current_exe().unwrap();
        dir.pop();
        let abspath = dir.join(&path);
        if abspath.exists() {
            fs::read_to_string(abspath).unwrap()
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
pub type Frequency<T> = HashMap<T, usize>;

#[derive(Debug)]
pub struct Assets {
    pub character_frequency: Frequency<char>,
    pub word_frequency: Frequency<String>,
    pub key_equivalence: KeyEquivalence,
    pub pair_equivalence: PairEquivalence,
}

pub fn prepare_file() -> (Config, RawElements, Assets) {
    let args = Args::parse();
    let config_path = args.config.unwrap_or(PathBuf::from("config.yaml"));
    let config: Config = serde_yaml::from_str(&get_file(config_path)).unwrap();

    // small parsers for TSV file
    let to_usize = |x: String| x.parse::<usize>().unwrap();
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
    let character_frequency = parse_hashmap(&get_file(cf_path), to_char, to_usize);
    let wf_path = args
        .word_frequency
        .unwrap_or(assets_dir.join("word_frequency.txt"));
    let word_frequency = parse_hashmap(&get_file(wf_path), identity, to_usize);
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
    return (config, elements, assets);
}
