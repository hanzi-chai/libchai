// 递归定义各种度量的数据结构以及它们输出到命令行的方式

use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct LevelMetric1 {
    pub length: usize,
    pub frequency: usize,
}

#[derive(Debug, Clone)]
pub struct LevelMetric2 {
    pub length: usize,
    pub frequency: f64,
}

#[derive(Debug, Clone)]
pub struct TierMetric {
    pub top: Option<usize>,
    pub duplication: Option<usize>,
    pub levels: Option<Vec<LevelMetric1>>,
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
            for LevelMetric1 { length, frequency } in levels {
                f.write_str(&format!(
                    "{}{}键：{}；",
                    specifier,
                    hanzi_numbers[length - 1],
                    frequency
                ))?;
            }
        }
        Ok(())
    }
}

pub type FingeringMetric = [Option<f64>; 8];

#[derive(Debug, Clone)]
pub struct PartialMetric {
    pub tiers: Option<Vec<TierMetric>>,
    pub duplication: Option<f64>,
    pub key_distribution: Option<f64>,
    pub new_key_equivalence: Option<f64>,
    pub new_key_equivalence_modified: Option<f64>,
    pub pair_equivalence: Option<f64>,
    pub new_pair_equivalence: Option<f64>,
    pub extended_pair_equivalence: Option<f64>,
    pub fingering: Option<FingeringMetric>,
    pub levels: Option<Vec<LevelMetric2>>,
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
        if let Some(equivalence) = self.new_key_equivalence {
            f.write_str(&format!("杏码式用指当量：{:.4}；", equivalence))?;
        }
        if let Some(equivalence) = self.new_key_equivalence_modified {
            f.write_str(&format!("杏码式用指当量改：{:.4}；", equivalence))?;
        }
        if let Some(equivalence) = self.pair_equivalence {
            f.write_str(&format!("组合当量：{:.4}；", equivalence))?;
        }
        if let Some(equivalence) = self.new_pair_equivalence {
            f.write_str(&format!("杏码式组合当量：{:.4}；", equivalence))?;
        }
        if let Some(equivalence) = self.extended_pair_equivalence {
            f.write_str(&format!("词间当量：{:.4}；", equivalence))?;
        }
        let types = ["同手", "大跨", "小跨", "干扰", "错手", "三连", "备用", "备用"];
        if let Some(fingering) = &self.fingering {
            for (index, percent) in fingering.iter().enumerate() {
                if let Some(percent) = percent {
                    f.write_str(&format!("{}：{:.2}%；", types[index], percent * 100.0))?;
                }
            }
        }
        if let Some(levels) = &self.levels {
            for LevelMetric2 { length, frequency } in levels {
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

#[derive(Debug, Clone)]
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
