use crate::assets::Assets;
use crate::assets::Frequency;
use crate::config::Config;
use crate::config::ObjectiveConfig;
use crate::config::PartialMetricWeights;
use crate::encoder::Code;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;

pub struct RankedData {
    code: String,
    frequency: usize,
}

pub fn merge_codes_and_frequency<T: Eq + Hash>(
    codes: &HashMap<T, String>,
    frequency: &Frequency<T>,
) -> Vec<RankedData> {
    let mut rank: Vec<RankedData> = Vec::new();
    for (key, code) in codes {
        let this_frequency = frequency.get(key).unwrap_or(&0);
        let data = RankedData {
            code: code.to_string(),
            frequency: *this_frequency,
        };
        rank.push(data);
    }
    rank.sort_by(|a, b| b.frequency.cmp(&a.frequency));
    return rank;
}

#[derive(Debug, Clone)]
pub struct TieredMetric {
    pub top: Option<usize>,
    pub duplication: usize,
}

impl Display for TieredMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let leading = if let Some(top) = self.top {
            format!("前 {}：", top)
        } else {
            String::from("全部：")
        };
        f.write_str(&format!("{} 选重数 {}\n", leading, self.duplication))
            .unwrap();
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PartialMetric {
    pub tiered: Vec<TieredMetric>,
    pub duplication: Option<f64>,
    pub equivalence: Option<f64>,
}

impl Display for PartialMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(duplication) = self.duplication {
            f.write_str(&format!("\t动态选重率：{:.2}%\n", duplication * 100.0))
                .unwrap();
        }
        if let Some(equivalence) = self.equivalence {
            f.write_str(&format!("\t当量：{:.2}\n", equivalence))
                .unwrap();
        }
        for tier in &self.tiered {
            f.write_str(&format!("\t{}", tier)).unwrap();
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Metric {
    pub characters: Option<PartialMetric>,
    pub words: Option<PartialMetric>,
}

impl Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(characters) = &self.characters {
            f.write_str(&format!("单字：\n{}", characters)).unwrap();
        }
        if let Some(words) = &self.words {
            f.write_str(&format!("词组：\n{}", words)).unwrap();
        }
        Ok(())
    }
}

pub struct Objective {
    config: ObjectiveConfig,
    assets: Assets,
}

impl Objective {
    pub fn new(config: &Config, assets: Assets) -> Objective {
        Objective {
            assets,
            config: config.optimization.objective.clone(),
        }
    }

    fn calculate_total_equivalence(&self, code: &String) -> f64 {
        let mut total = 0.0;
        let mut it = code.chars();
        let mut this = it.next().unwrap();
        while let Some(next) = it.next() {
            total += self.assets.equivalence.get(&(this, next)).unwrap_or(&0.0);
            this = next;
        }
        total
    }

    pub fn evaluate_partial<T: Eq + Hash>(
        &self,
        codes: &HashMap<T, String>,
        frequency: &Frequency<T>,
        weights: &PartialMetricWeights,
    ) -> (PartialMetric, f64) {
        let ranked = merge_codes_and_frequency(codes, frequency);
        let mut occupied_codes: HashSet<String> = HashSet::new();
        let mut total_frequency = 0;
        let mut total_duplication = 0;
        let mut tiered_duplication = vec![0; weights.tiered.len()];
        let mut total_equivalence = 0.0;
        for (index, data) in ranked.iter().enumerate() {
            total_frequency += data.frequency;
            total_equivalence +=
                self.calculate_total_equivalence(&data.code) * data.frequency as f64;
            // 重码相关
            if let Some(_) = occupied_codes.get(&data.code) {
                total_duplication += data.frequency;
                for (itier, tier) in weights.tiered.iter().enumerate() {
                    let top = tier.top.unwrap_or(std::usize::MAX);
                    if index <= top {
                        tiered_duplication[itier] += 1;
                    }
                }
            }
            occupied_codes.insert(data.code.clone());
        }
        let total_frequency = total_frequency as f64;
        let mut equivalence: Option<f64> = None;
        let mut duplication: Option<f64> = None;

        let mut real = 0.0;
        if let Some(equivalence_weight) = weights.equivalence {
            equivalence = Some(total_equivalence / total_frequency);
            real += equivalence.unwrap() * equivalence_weight;
        }
        if let Some(duplication_weight) = weights.duplication {
            duplication = Some(total_duplication as f64 / total_frequency);
            real += duplication.unwrap() * duplication_weight;
        }
        for (itier, twights) in weights.tiered.iter().enumerate() {
            let total = twights.top.unwrap_or(ranked.len());
            if let Some(duplication_weight) = twights.duplication {
                real += tiered_duplication[itier] as f64 * duplication_weight / total as f64;
            }
        }
        let tiered = weights
            .tiered
            .iter()
            .enumerate()
            .map(|(itier, tier)| TieredMetric {
                top: tier.top,
                duplication: tiered_duplication[itier],
            })
            .collect();
        return (
            PartialMetric {
                tiered,
                duplication,
                equivalence,
            },
            real,
        );
    }

    pub fn evaluate(
        &self,
        character_codes: &Code<char>,
        word_codes: &Code<String>,
    ) -> (Metric, f64) {
        let mut loss = 0.0;
        let mut metric = Metric {
            characters: None,
            words: None,
        };
        if let Some(characters) = &self.config.characters {
            let (partial, accum) =
                self.evaluate_partial(character_codes, &self.assets.characters, characters);
            loss += accum;
            metric.characters = Some(partial);
        }
        if let Some(words) = &self.config.words {
            let (partial, accum) = self.evaluate_partial(word_codes, &self.assets.words, words);
            loss += accum;
            metric.words = Some(partial);
        }
        (metric, loss)
    }
}
