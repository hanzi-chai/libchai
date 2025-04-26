// 递归定义各种度量的数据结构以及它们输出到命令行的方式

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;

type 指法集 = HashSet<(char, char)>;
type 键盘布局 = [Vec<char>; 4];

// 指法分析
//
// 参考法月的《科学形码测评系统》，基于定义来推导出各种差指法组合都有哪些，然后封装成一个结构体便于主程序使用。
#[derive(Debug)]
pub struct 指法标记 {
    pub 同手: 指法集,
    pub 同指大跨排: 指法集,
    pub 同指小跨排: 指法集,
    pub 小指干扰: 指法集,
    pub 错手: 指法集,
}

#[derive(PartialEq, PartialOrd, Copy, Clone)]
enum 手指 {
    _大拇指,
    食指,
    中指,
    无名指,
    小指,
}

impl 指法标记 {
    pub fn new() -> 指法标记 {
        let 左手: 键盘布局 = [
            vec!['5', '4', '3', '2', '1'],
            vec!['t', 'r', 'e', 'w', 'q'],
            vec!['g', 'f', 'd', 's', 'a'],
            vec!['b', 'v', 'c', 'x', 'z'],
        ];
        let 右手: 键盘布局 = [
            vec!['6', '7', '8', '9', '0', '-', '='],
            vec!['y', 'u', 'i', 'o', 'p', '[', ']'],
            vec!['h', 'j', 'k', 'l', ';', '\''],
            vec!['n', 'm', ',', '.', '/'],
        ];
        let mut 左手标记 = Self::生成单手指法标记(&左手);
        let 右手标记 = Self::生成单手指法标记(&右手);
        左手标记.同手.extend(右手标记.同手);
        左手标记.同指大跨排.extend(右手标记.同指大跨排);
        左手标记.同指小跨排.extend(右手标记.同指小跨排);
        左手标记.小指干扰.extend(右手标记.小指干扰);
        左手标记.错手.extend(右手标记.错手);
        左手标记
    }

    fn 生成单手指法标记(单手布局: &键盘布局) -> 指法标记 {
        use 手指::*;
        let 列对应手指: [手指; 7] = [食指, 食指, 中指, 无名指, 小指, 小指, 小指];
        let 是长手指 = |x: 手指| x == 中指 || x == 无名指;
        let mut 同手 = 指法集::new();
        let mut 同指大跨排 = 指法集::new();
        let mut 同指小跨排 = 指法集::new();
        let mut 小指干扰 = 指法集::new();
        let mut 错手 = 指法集::new();
        for (行序号一, 行一) in 单手布局.iter().enumerate() {
            for (行序号二, 行二) in 单手布局.iter().enumerate() {
                for (列序号一, 列一) in 行一.iter().enumerate() {
                    for (列序号二, 列二) in 行二.iter().enumerate() {
                        let 组合 = (*列一, *列二);
                        同手.insert(组合);
                        let 手指一 = 列对应手指[列序号一];
                        let 手指二 = 列对应手指[列序号二];
                        let 行差值 = 行序号一.abs_diff(行序号二);
                        let 列差值 = 列序号一.abs_diff(列序号二);
                        if 手指一 == 手指二 {
                            if 行差值 >= 2 {
                                同指大跨排.insert(组合);
                            } else if 行差值 == 1 || 列差值 == 1 {
                                同指小跨排.insert(组合);
                            }
                        }
                        if (手指一 == 小指 && 手指二 >= 中指)
                            || (手指二 == 小指 && 手指一 >= 中指)
                        {
                            小指干扰.insert(组合);
                        }
                        let 错手一 = 是长手指(手指一) && 行序号一 > 行序号二 + 1;
                        let 错手二 = 是长手指(手指二) && 行序号二 > 行序号一 + 1;
                        if 错手一 || 错手二 {
                            错手.insert(组合);
                        }
                    }
                }
            }
        }
        指法标记 {
            同手,
            同指大跨排,
            同指小跨排,
            小指干扰,
            错手,
        }
    }
}

const 指法标记名称: [&str; 8] = [
    "同手", "大跨", "小跨", "干扰", "错手", "三连", "备用", "备用",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 键长指标 {
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
pub struct 层级指标 {
    pub top: Option<usize>,
    pub duplication: Option<u64>,
    pub levels: Option<Vec<LevelMetricUniform>>,
    pub fingering: Option<FingeringMetricUniform>,
}

impl Display for 层级指标 {
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
                    f.write_str(&format!("{}：{}；", 指法标记名称[index], frequency))?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 分组指标 {
    pub tiers: Option<Vec<层级指标>>,
    pub duplication: Option<f64>,
    pub key_distribution: Option<HashMap<char, f64>>,
    pub key_distribution_loss: Option<f64>,
    pub pair_equivalence: Option<f64>,
    pub extended_pair_equivalence: Option<f64>,
    pub fingering: Option<FingeringMetric>,
    pub levels: Option<Vec<键长指标>>,
}

const 键盘布局: [[char; 10]; 5] = [
    ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'],
    ['q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p'],
    ['a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';'],
    ['z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/'],
    ['_', '\'', '-', '=', '[', ']', '\\', '`', ' ', ' '],
];

impl Display for 分组指标 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hanzi_numbers: Vec<char> = "一二三四五六七八九十".chars().collect();
        if let Some(duplication) = self.duplication {
            f.write_str(&format!("选重率：{:.4}%；", duplication * 100.0))?;
        }
        if let Some(key_distribution_loss) = self.key_distribution_loss {
            f.write_str(&format!(
                "用指分布偏差：{:.2}%；",
                key_distribution_loss * 100.0
            ))?;
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
                        指法标记名称[index],
                        percent * 100.0
                    ))?;
                }
            }
        }
        if let Some(levels) = &self.levels {
            for 键长指标 { length, frequency } in levels {
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
        if let Some(key_distribution) = &self.key_distribution {
            for 行 in 键盘布局.iter() {
                if 行.iter().any(|x| key_distribution.contains_key(x)) {
                    f.write_str("\n")?;
                    let mut buffer = vec![];
                    for 键 in 行 {
                        if let Some(频率) = key_distribution.get(键) {
                            buffer.push(format!("{} {:5.2}%", 键, 频率 * 100.0));
                        }
                    }
                    f.write_str(&buffer.join(" | "))?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct 默认指标 {
    pub characters_full: Option<分组指标>,
    pub characters_short: Option<分组指标>,
    pub words_full: Option<分组指标>,
    pub words_short: Option<分组指标>,
    pub memory: Option<f64>,
}

impl Display for 默认指标 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(记忆量) = &self.memory {
            f.write_str(&format!("记忆量：{:.2}；\n", 记忆量))?;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    fn test_subset(all: HashSet<(char, char)>, target: &str) {
        for pair in target.split(' ') {
            let chars: Vec<char> = pair.chars().collect();
            assert!(all.contains(&(chars[0], chars[1])), "集合不包含：{}", pair);
        }
    }

    #[test]
    fn test_fingering_types() {
        let 测评系统同指大跨排 =
            "br bt ce ec mu my nu ny p/ qz rb rv tb tv um un vr vt wx xw ym yn zq ,i /p";
        let 测评系统同指小跨排 = "qa za fb gb vb dc cd ed de bf gf rf tf vf bg fg rg tg vg jh mh nh uh yh ki hj mj nj uj yj ik ol hm jm nm hn jn mn lo ;p aq fr gr tr ws xs ft gt rt hu ju yu bv fv gv sw sx hy jy uy az k, ;/ p; /;";
        let 测评系统小指干扰 = "aa ac ad ae aq as aw ax az ca cq cz da dq dz ea eq ez ip i/ i; kp k/ k; lp l/ l; op o/ o; pi pk pl po pp p; qa qc qd qe qq qs qw qx sa sq sz wa wq wz xa xq xz za zc zd ze zs zw zx zz ,p ,/ ,; /i /k /l /o // /; ;i ;k ;l ;o ;p ;/ ;;";
        let 测评系统错手 = "ct ,y tc y, cr ,u rc u, cw ,o wc o, qc ,p cq p, qx p. xq .p xe .i ex i. xr .u rx u. xt .y tx y.";
        let 指法标记 {
            同指大跨排,
            同指小跨排,
            小指干扰,
            错手,
            ..
        } = 指法标记::new();
        test_subset(同指大跨排, 测评系统同指大跨排);
        test_subset(同指小跨排, 测评系统同指小跨排);
        test_subset(小指干扰, 测评系统小指干扰);
        test_subset(错手, 测评系统错手);
    }
}
