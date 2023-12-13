use crate::assets::Assets;
use crate::config::Cache;
use crate::config::Config;
use crate::config::KeyMap;
use crate::config::ObjectiveConfig;
use crate::config::PartialMetricWeights;
use crate::encoder::Code;
use crate::encoder::Encoder;
use crate::encoder::RawElements;
use std::collections::HashSet;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct TieredMetric {
    pub top: Option<usize>,
    pub duplication: Option<usize>,
    pub levels: Option<Vec<usize>>,
}

impl Display for TieredMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hanzi_numbers: Vec<char> = "一二三四五六七八九十".chars().collect();
        let specifier = if let Some(top) = self.top {
            format!("{} ", top)
        } else {
            String::from("全部")
        };
        if let Some(duplication) = self.duplication {
            f.write_str(&format!("{}选重：{}；", specifier, duplication)).unwrap();
        }
        if let Some(levels) = &self.levels {
            for (level_index, value) in levels.iter().enumerate() {
                f.write_str(&format!("{}{}键：{}；", specifier, hanzi_numbers[level_index], value)).unwrap();
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PartialMetric {
    pub tiered: Option<Vec<TieredMetric>>,
    pub duplication: Option<f64>,
    pub equivalence: Option<f64>,
    pub levels: Option<Vec<f64>>,
}

impl Display for PartialMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hanzi_numbers: Vec<char> = "一二三四五六七八九十".chars().collect();
        if let Some(duplication) = self.duplication {
            f.write_str(&format!("选重率：{:.2}%；", duplication * 100.0))
                .unwrap();
        }
        if let Some(equivalence) = self.equivalence {
            f.write_str(&format!("当量：{:.2}；", equivalence)).unwrap();
        }
        if let Some(levels) = &self.levels {
            for (level_index, value) in levels.iter().enumerate() {
                f.write_str(&format!("{}键：{:.2}%；", hanzi_numbers[level_index], value * 100.0)).unwrap();
            }
        }
        if let Some(tiered) = &self.tiered {
            for tier in tiered {
                f.write_str(&format!("{}", tier)).unwrap();
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Metric {
    pub characters: Option<PartialMetric>,
    pub words: Option<PartialMetric>,
    pub characters_reduced: Option<PartialMetric>,
    pub words_reduced: Option<PartialMetric>,
}

impl Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(characters) = &self.characters {
            f.write_str(&format!("单字全码［{}］\n", characters))
                .unwrap();
        }
        if let Some(words) = &self.words {
            f.write_str(&format!("词语全码［{}］\n", words)).unwrap();
        }
        if let Some(characters_reduced) = &self.characters_reduced {
            f.write_str(&format!("单字简码［{}］\n", characters_reduced))
                .unwrap();
        }
        if let Some(words_reduced) = &self.words_reduced {
            f.write_str(&format!("词语简码［{}］\n", words_reduced))
                .unwrap();
        }
        Ok(())
    }
}

pub struct Objective {
    config: ObjectiveConfig,
    assets: Assets,
    encoder: Encoder,
}

impl Objective {
    pub fn new(config: &Config, cache: &Cache, name: &String) -> Objective {
        let assets = Assets::new();
        let raw_elements: RawElements = Assets::read_hashmap_from_file(
            name,
            |x| x.chars().next().unwrap(),
            |x| x.split(' ').map(|x| x.to_string()).collect(),
        );
        let elements = cache.transform_elements(&raw_elements);
        let encoder = Encoder::new(&config, elements, &assets);
        Objective {
            assets,
            encoder,
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

    pub fn evaluate_partial(
        &self,
        codes: &Code,
        weights: &PartialMetricWeights,
    ) -> (PartialMetric, f64) {
        let mut occupied_codes: HashSet<String> = HashSet::new();
        let mut total_frequency = 0;
        let mut total_duplication = 0;
        let ntier = weights
            .tiered
            .as_ref()
            .and_then(|x| Some(x.len()))
            .unwrap_or(0);
        let mut tiered_duplication = vec![0; ntier];
        let mut total_equivalence = 0.0;
        let mut total_levels = vec![0_usize; self.encoder.max_length];
        let mut tiered_levels = vec![total_levels.clone(); ntier];
        for (index, (code, frequency)) in codes.iter().enumerate() {
            total_frequency += frequency;
            total_equivalence += self.calculate_total_equivalence(code) * *frequency as f64;
            // 重码相关
            if let Some(_) = occupied_codes.get(code) {
                total_duplication += frequency;
                if let Some(tiered) = &weights.tiered {
                    for (itier, tier) in tiered.iter().enumerate() {
                        let top = tier.top.unwrap_or(std::usize::MAX);
                        if index <= top {
                            tiered_duplication[itier] += 1;
                        }
                    }
                }
            }
            // 简码相关
            let level_index = code.len() - 1;
            total_levels[level_index] += frequency;
            if let Some(tiered) = &weights.tiered {
                for (itier, tier) in tiered.iter().enumerate() {
                    let top = tier.top.unwrap_or(std::usize::MAX);
                    if index <= top {
                        tiered_levels[itier][level_index] += 1;
                    }
                }
            }
            occupied_codes.insert(code.clone());
        }
        let total_frequency = total_frequency as f64;
        let mut partial_metric = PartialMetric {
            tiered: None,
            equivalence: None,
            duplication: None,
            levels: None,
        };

        let mut real = 0.0;
        if let Some(equivalence_weight) = weights.equivalence {
            let equivalence = total_equivalence / total_frequency;
            partial_metric.equivalence = Some(equivalence);
            real += equivalence * equivalence_weight;
        }
        if let Some(duplication_weight) = weights.duplication {
            let duplication = total_duplication as f64 / total_frequency;
            partial_metric.duplication = Some(duplication);
            real += duplication * duplication_weight;
        }
        if let Some(levels_weight) = &weights.levels {
            let mut levels: Vec<f64> = Vec::new();
            for (level_index, weight) in levels_weight.iter().enumerate() {
                let value = total_levels[level_index] as f64 / total_frequency;
                real += value * weight;
                levels.push(value)
            }
            partial_metric.levels = Some(levels);
        }
        if let Some(tiered_weight) = &weights.tiered {
            let mut tiered: Vec<TieredMetric> = tiered_weight
                .iter()
                .map(|x| TieredMetric {
                    top: x.top,
                    duplication: None,
                    levels: None,
                })
                .collect();
            for (itier, twights) in tiered_weight.iter().enumerate() {
                let total = twights.top.unwrap_or(codes.len());
                if let Some(duplication_weight) = twights.duplication {
                    let duplication = tiered_duplication[itier];
                    real += duplication as f64 / total as f64 * duplication_weight;
                    tiered[itier].duplication = Some(duplication);
                }
                if let Some(level_weight) = &twights.levels {
                    let levels = &tiered_levels[itier];
                    for (level_index, weight) in level_weight.iter().enumerate() {
                        real += levels[level_index] as f64 / total as f64 * weight;
                    }
                    tiered[itier].levels = Some(levels[0..level_weight.len()].to_vec());
                }
            }
            partial_metric.tiered = Some(tiered);
        }
        return (partial_metric, real);
    }

    pub fn evaluate(&self, candidate: &KeyMap) -> (Metric, f64) {
        let mut loss = 0.0;
        let mut metric = Metric {
            characters: None,
            words: None,
            characters_reduced: None,
            words_reduced: None,
        };
        if let Some(characters) = &self.config.characters {
            let character_codes = self.encoder.encode_character_full(&candidate);
            let (partial, accum) = self.evaluate_partial(&character_codes, characters);
            loss += accum;
            metric.characters = Some(partial);
            if let Some(character_reduced) = &self.config.characters_reduced {
                let character_codes_reduced = self.encoder.encode_reduced(&character_codes);
                // println!("{:?}", character_codes_reduced);
                let (partial, accum) =
                    self.evaluate_partial(&character_codes_reduced, character_reduced);
                loss += accum;
                metric.characters_reduced = Some(partial);
            }
        }
        if let Some(words) = &self.config.words {
            let word_codes = self.encoder.encode_words_full(&candidate);
            let (partial, accum) = self.evaluate_partial(&word_codes, words);
            loss += accum;
            metric.words = Some(partial);
            if let Some(words_reduced) = &self.config.words_reduced {
                let word_codes_reduced = self.encoder.encode_reduced(&word_codes);
                let (partial, accum) = self.evaluate_partial(&word_codes_reduced, words_reduced);
                loss += accum;
                metric.words_reduced = Some(partial);
            }
        }
        (metric, loss)
    }
}
