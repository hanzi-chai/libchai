// 递归定义各种度量的数据结构以及它们输出到命令行的方式

use std::fmt::Display;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

type FingeringSet = HashSet<(char, char)>;
type Layout = [Vec<char>; 4];

// 指法分析
//
// 参考法月的《科学形码测评系统》，基于定义来推导出各种差指法组合都有哪些，然后封装成一个结构体便于主程序使用。
#[derive(Debug)]
pub struct FingeringTypes {
    pub same_hand: FingeringSet,
    pub same_finger_large_jump: FingeringSet,
    pub same_finger_small_jump: FingeringSet,
    pub little_finger_interference: FingeringSet,
    pub awkward_upside_down: FingeringSet,
}

pub fn get_partial_fingering_types(layout: &Layout) -> FingeringTypes {
    let map_char_index_to_finger: [usize; 7] = [2, 2, 3, 4, 5, 5, 5];
    let is_long_finger = |x: usize| x == 3 || x == 4;
    let is_short_finger = |x: usize| x == 2 || x == 5;
    let mut same_hand = FingeringSet::new();
    let mut same_finger_large_jump = FingeringSet::new();
    let mut same_finger_small_jump = FingeringSet::new();
    let mut little_finger_interference = FingeringSet::new();
    let mut awkward_upside_down = FingeringSet::new();
    for (row1, content1) in layout.iter().enumerate() {
        for (row2, content2) in layout.iter().enumerate() {
            for (column1, char1) in content1.iter().enumerate() {
                for (column2, char2) in content2.iter().enumerate() {
                    let pair = (*char1, *char2);
                    same_hand.insert(pair);
                    let finger1 = map_char_index_to_finger[column1];
                    let finger2 = map_char_index_to_finger[column2];
                    let row_diff = row1.abs_diff(row2);
                    if finger1 == finger2 {
                        if row_diff >= 2 {
                            same_finger_large_jump.insert(pair);
                        } else if row_diff == 1 {
                            same_finger_small_jump.insert(pair);
                        }
                    }
                    if (finger1 == 5 && finger2 >= 3) || (finger2 == 5 && finger1 >= 3) {
                        little_finger_interference.insert(pair);
                    }
                    // 短指击上排，长指击下排
                    let awkward1 =
                        row1 < row2 && is_short_finger(finger1) && is_long_finger(finger2);
                    // 长指击下排，短指击上排
                    let awkward2 =
                        row1 > row2 && is_long_finger(finger1) && is_short_finger(finger2);
                    if (awkward1 || awkward2) && row_diff >= 2 {
                        awkward_upside_down.insert(pair);
                    }
                }
            }
        }
    }
    FingeringTypes {
        same_hand,
        same_finger_large_jump,
        same_finger_small_jump,
        little_finger_interference,
        awkward_upside_down,
    }
}

pub fn get_fingering_types() -> FingeringTypes {
    let left_layout: Layout = [
        vec!['5', '4', '3', '2', '1'],
        vec!['t', 'r', 'e', 'w', 'q'],
        vec!['g', 'f', 'd', 's', 'a'],
        vec!['b', 'v', 'c', 'x', 'z'],
    ];
    let right_layout: Layout = [
        vec!['6', '7', '8', '9', '0', '-', '='],
        vec!['y', 'u', 'i', 'o', 'p', '[', ']'],
        vec!['h', 'j', 'k', 'l', ';', '\''],
        vec!['n', 'm', ',', '.', '/'],
    ];
    let mut left_types = get_partial_fingering_types(&left_layout);
    let right_types = get_partial_fingering_types(&right_layout);
    left_types.same_hand.extend(right_types.same_hand);
    left_types
        .same_finger_large_jump
        .extend(right_types.same_finger_large_jump);
    left_types
        .same_finger_small_jump
        .extend(right_types.same_finger_small_jump);
    left_types
        .little_finger_interference
        .extend(right_types.little_finger_interference);
    left_types
        .awkward_upside_down
        .extend(right_types.awkward_upside_down);
    left_types
}

const FINGERING_LABELS: [&str; 8] = [
    "同手", "大跨", "小跨", "干扰", "错手", "三连", "备用", "备用",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelMetric {
    pub length: usize,
    pub frequency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelMetricUniform {
    pub length: usize,
    pub frequency: u64,
}

pub type FingeringMetric = [Option<f64>; 8];
pub type FingeringMetricUniform = [Option<u64>; 8];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierMetric {
    pub top: Option<usize>,
    pub duplication: Option<u64>,
    pub levels: Option<Vec<LevelMetricUniform>>,
    pub fingering: Option<FingeringMetricUniform>,
}

impl Display for TierMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hanzi_numbers: Vec<char> = "一二三四五六七八九十".chars().collect();
        let specifier = if let Some(top) = self.top {
            format!("{} ", top)
        } else {
            String::from("全部")
        };
        if let Some(duplication) = self.duplication {
            f.write_str(&format!("{}选重：{}；", specifier, duplication))?;
        }
        if let Some(levels) = &self.levels {
            for LevelMetricUniform { length, frequency } in levels {
                f.write_str(&format!(
                    "{}{}键：{}；",
                    specifier,
                    hanzi_numbers[length - 1],
                    frequency
                ))?;
            }
        }
        if let Some(fingering) = &self.fingering {
            for (index, frequency) in fingering.iter().enumerate() {
                if let Some(frequency) = frequency {
                    f.write_str(&format!("{}：{}；", FINGERING_LABELS[index], frequency))?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialMetric {
    pub tiers: Option<Vec<TierMetric>>,
    pub duplication: Option<f64>,
    pub key_distribution: Option<f64>,
    pub pair_equivalence: Option<f64>,
    pub extended_pair_equivalence: Option<f64>,
    pub fingering: Option<FingeringMetric>,
    pub levels: Option<Vec<LevelMetric>>,
}

impl Display for PartialMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hanzi_numbers: Vec<char> = "一二三四五六七八九十".chars().collect();
        // 宇浩提到过，当量是一个敏感数字。增加它的有效数字
        if let Some(duplication) = self.duplication {
            f.write_str(&format!("选重率：{:.4}%；", duplication * 100.0))?;
        }
        if let Some(key_distribution) = self.key_distribution {
            f.write_str(&format!("用指分布偏差：{:.2}%；", key_distribution * 100.0))?;
        }
        if let Some(equivalence) = self.pair_equivalence {
            f.write_str(&format!("组合当量：{:.4}；", equivalence))?;
        }
        if let Some(equivalence) = self.extended_pair_equivalence {
            f.write_str(&format!("词间当量：{:.4}；", equivalence))?;
        }
        if let Some(fingering) = &self.fingering {
            for (index, percent) in fingering.iter().enumerate() {
                if let Some(percent) = percent {
                    f.write_str(&format!(
                        "{}：{:.2}%；",
                        FINGERING_LABELS[index],
                        percent * 100.0
                    ))?;
                }
            }
        }
        if let Some(levels) = &self.levels {
            for LevelMetric { length, frequency } in levels {
                f.write_str(&format!(
                    "{}键：{:.2}%；",
                    hanzi_numbers[length - 1],
                    frequency * 100.0
                ))?;
            }
        }
        if let Some(tiers) = &self.tiers {
            for tier in tiers {
                f.write_str(&format!("{}", tier))?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub characters_full: Option<PartialMetric>,
    pub characters_short: Option<PartialMetric>,
    pub words_full: Option<PartialMetric>,
    pub words_short: Option<PartialMetric>,
}

impl Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(characters) = &self.characters_full {
            f.write_str(&format!("一字全码［{}］\n", characters))?;
        }
        if let Some(words) = &self.words_full {
            f.write_str(&format!("多字全码［{}］\n", words))?;
        }
        if let Some(characters_reduced) = &self.characters_short {
            f.write_str(&format!("一字简码［{}］\n", characters_reduced))?;
        }
        if let Some(words_reduced) = &self.words_short {
            f.write_str(&format!("多字简码［{}］\n", words_reduced))?;
        }
        Ok(())
    }
}
