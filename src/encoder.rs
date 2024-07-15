//! 编码引擎

use rustc_hash::FxHashSet;

use crate::representation::{Buffer, Frequency, KeyMap, Sequence};

pub mod generic;

/// 一个可编码对象
#[derive(Debug, Clone)]
pub struct Encodable {
    pub name: String,
    pub length: usize,
    pub sequence: Sequence,
    pub frequency: u64,
    pub level: u64,
    pub hash: u16,
    pub index: usize,
}

#[derive(Debug)]
pub struct CompiledScheme {
    pub prefix: usize,
    pub select_keys: Vec<usize>,
}


pub fn adapt(
    frequency: &Frequency,
    words: &FxHashSet<String>,
) -> (Frequency, Vec<(String, String, u64)>) {
    let mut new_frequency = Frequency::new();
    let mut transition_pairs = Vec::new();
    for (word, value) in frequency {
        if words.contains(word) {
            new_frequency.insert(word.clone(), new_frequency.get(word).unwrap_or(&0) + *value);
        } else {
            // 使用逆向最大匹配算法来分词
            let chars: Vec<_> = word.chars().collect();
            let mut end = chars.len();
            let mut last_match: Option<String> = None;
            while end > 0 {
                let mut start = end - 1;
                // 如果最后一个字不在词表里，就不要了
                if !words.contains(&chars[start].to_string()) {
                    end -= 1;
                    continue;
                }
                // 继续向前匹配，看看是否能匹配到更长的词
                while start > 0
                    && words.contains(&chars[(start - 1)..end].iter().collect::<String>())
                {
                    start -= 1;
                }
                // 确定最大匹配
                let sub_word: String = chars[start..end].iter().collect();
                *new_frequency.entry(sub_word.clone()).or_default() += *value;
                if let Some(last) = last_match {
                    transition_pairs.push((sub_word.clone(), last, *value));
                }
                last_match = Some(sub_word);
                end = start;
            }
        }
    }
    (new_frequency, transition_pairs)
}

pub trait Encoder {
    fn encode_full(&self, keymap: &KeyMap, buffer: &mut Buffer);
    fn encode_short(&self, buffer: &mut Buffer);
    fn get_radix(&self) -> u64;
    fn get_space(&self) -> usize;
    fn get_actual_code(&self, code: u64, rank: i8, length: u32) -> (u64, u32);
    fn get_transitions(&self, index: usize) -> &[(usize, u64)];
}