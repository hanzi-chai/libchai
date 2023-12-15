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
            f.write_str(&format!("{}选重：{}；", specifier, duplication))
                .unwrap();
        }
        if let Some(levels) = &self.levels {
            for LevelMetric1 { length, frequency } in levels {
                f.write_str(&format!(
                    "{}{}键：{}；",
                    specifier,
                    hanzi_numbers[length - 1],
                    frequency
                ))
                .unwrap();
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PartialMetric {
    pub tiers: Option<Vec<TierMetric>>,
    pub duplication: Option<f64>,
    pub key_equivalence: Option<f64>,
    pub pair_equivalence: Option<f64>,
    pub levels: Option<Vec<LevelMetric2>>,
}

impl Display for PartialMetric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hanzi_numbers: Vec<char> = "一二三四五六七八九十".chars().collect();
        if let Some(duplication) = self.duplication {
            f.write_str(&format!("选重率：{:.2}%；", duplication * 100.0))
                .unwrap();
        }
        if let Some(equivalence) = self.key_equivalence {
            f.write_str(&format!("用指：{:.2}；", equivalence)).unwrap();
        }
        if let Some(equivalence) = self.pair_equivalence {
            f.write_str(&format!("当量：{:.2}；", equivalence)).unwrap();
        }
        if let Some(levels) = &self.levels {
            for LevelMetric2 { length, frequency } in levels {
                f.write_str(&format!(
                    "{}键：{:.2}%；",
                    hanzi_numbers[length - 1],
                    frequency * 100.0
                ))
                .unwrap();
            }
        }
        if let Some(tiers) = &self.tiers {
            for tier in tiers {
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
