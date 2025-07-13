use rustc_hash::FxHashMap;

use super::cache::缓存;
use super::metric::默认指标;
use super::目标函数;
use crate::config::PartialWeights;
use crate::contexts::default::默认上下文;
use crate::encoders::编码器;
use crate::{
    元素, 元素映射, 指法向量, 正则化, 编码信息, 键位分布损失函数
};
use crate::错误;

#[derive(Clone)]
pub struct 默认目标函数<E: 编码器> {
    pub 参数: 默认目标函数参数,
    pub 编码器: E,
    pub 编码结果: Vec<编码信息>,
    pub 编码结果缓冲: Vec<编码信息>,
    pub 计数桶列表: Vec<[Option<缓存>; 2]>,
    pub 计数桶列表缓冲: Vec<[Option<缓存>; 2]>,
}

#[derive(Clone)]
pub struct 默认目标函数参数 {
    pub 键位分布信息: Vec<键位分布损失函数>,
    pub 当量信息: Vec<f64>,
    pub 指法计数: Vec<指法向量>,
    pub 数字转键: FxHashMap<u64, char>,
    pub 正则化: 正则化,
    pub 正则化强度: f64,
}

pub type Frequencies = Vec<f64>;

pub enum PartialType {
    CharactersFull,
    CharactersShort,
    WordsFull,
    WordsShort,
}

impl PartialType {
    pub fn is_characters(&self) -> bool {
        matches!(self, Self::CharactersFull | Self::CharactersShort)
    }
}

/// 目标函数
impl<E: 编码器<解类型 = 元素映射>> 默认目标函数<E> {
    /// 通过传入配置表示、编码器和共用资源来构造一个目标函数
    pub fn 新建(上下文: &默认上下文, 编码器: E) -> Result<Self, 错误> {
        let 键位分布信息 = 上下文.键位分布信息.clone();
        let 当量信息 = 上下文.当量信息.clone();
        let 正则化 = if let Some(正则化配置) = 上下文
            .配置
            .optimization
            .clone()
            .map(|x| x.objective)
            .and_then(|x| x.regularization)
        {
            默认上下文::预处理正则化(&正则化配置, &上下文.棱镜.元素转数字)?
        } else {
            FxHashMap::default()
        };
        let 指法计数 = 上下文.棱镜.预处理指法标记(上下文.get_space());
        let config = 上下文
            .配置
            .optimization
            .as_ref()
            .ok_or("优化配置不存在")?
            .objective
            .clone();
        let 最大编码 = 当量信息.len() as u64;
        let 构造缓存 =
            |x: &PartialWeights| 缓存::new(x, 上下文.棱镜.进制, 上下文.词列表.len(), 最大编码);
        let 一字全码 = config.characters_full.as_ref().map(构造缓存);
        let 一字简码 = config.characters_short.as_ref().map(构造缓存);
        let 多字全码 = config.words_full.as_ref().map(构造缓存);
        let 多字简码 = config.words_short.as_ref().map(构造缓存);
        let 计数桶列表 = vec![[一字全码, 一字简码], [多字全码, 多字简码]];
        let 参数 = 默认目标函数参数 {
            键位分布信息,
            当量信息,
            指法计数,
            数字转键: 上下文.棱镜.数字转键.clone(),
            正则化,
            正则化强度: config
                .regularization
                .and_then(|x| x.strength)
                .unwrap_or(1.0),
        };
        let 编码结果: Vec<_> = 上下文.词列表.iter().map(编码信息::new).collect();
        Ok(Self {
            参数,
            编码器,
            编码结果: 编码结果.clone(),
            计数桶列表: 计数桶列表.clone(),
            编码结果缓冲: 编码结果.clone(),
            计数桶列表缓冲: 计数桶列表.clone(),
        })
    }
}

impl<E: 编码器<解类型 = 元素映射>> 目标函数 for 默认目标函数<E> {
    type 目标值 = 默认指标;
    type 解类型 = 元素映射;

    /// 计算各个部分编码的指标，然后将它们合并成一个指标输出
    fn 计算(&mut self, 解: &元素映射, 变化: &Option<Vec<元素>>) -> (默认指标, f64) {
        let 参数 = &self.参数;
        self.编码结果缓冲.clone_from(&self.编码结果);
        self.编码器.编码(解, 变化, &mut self.编码结果缓冲);
        self.计数桶列表缓冲.clone_from(&self.计数桶列表);
        let mut 桶序号列表: Vec<_> = self.计数桶列表缓冲.iter().map(|_| 0).collect();
        // 开始计算指标
        for 编码信息 in self.编码结果缓冲.iter_mut() {
            let 频率 = 编码信息.频率;
            let 桶索引 = if 编码信息.词长 == 1 { 0 } else { 1 };
            let 桶 = &mut self.计数桶列表缓冲[桶索引];
            let 桶序号 = 桶序号列表[桶索引];
            if let Some(缓存) = &mut 桶[0] {
                缓存.处理(桶序号, 频率, &mut 编码信息.全码, 参数);
            }
            if let Some(缓存) = &mut 桶[1] {
                缓存.处理(桶序号, 频率, &mut 编码信息.简码, 参数);
            }
            桶序号列表[桶索引] += 1;
        }

        if 变化.is_none() {
            self.编码结果.clone_from(&self.编码结果缓冲);
            self.计数桶列表.clone_from(&self.计数桶列表缓冲);
        }

        let mut 目标函数 = 0.0;
        let mut 指标 = 默认指标 {
            characters_full: None,
            words_full: None,
            characters_short: None,
            words_short: None,
            memory: None,
        };
        for (桶索引, 桶) in self.计数桶列表缓冲.iter().enumerate() {
            let _ = &桶[0].as_ref().map(|x| {
                let (分组指标, 分组目标函数) = x.汇总(参数);
                目标函数 += 分组目标函数;
                if 桶索引 == 0 {
                    指标.characters_full = Some(分组指标);
                } else {
                    指标.words_full = Some(分组指标);
                }
            });
            let _ = &桶[1].as_ref().map(|x| {
                let (分组指标, 分组目标函数) = x.汇总(参数);
                目标函数 += 分组目标函数;
                if 桶索引 == 0 {
                    指标.characters_short = Some(分组指标);
                } else {
                    指标.words_short = Some(分组指标);
                }
            });
        }

        if !参数.正则化.is_empty() {
            let mut 记忆量 = 解.len() as f64;
            for (元素, 键) in 解.iter().enumerate() {
                if 元素 as u64 == *键 {
                    记忆量 -= 1.0;
                    continue;
                }
                if let Some(归并列表) = 参数.正则化.get(&元素) {
                    let mut 最大亲和度 = 0.0;
                    for (目标元素, 亲和度) in 归并列表.iter() {
                        if 解[*目标元素] == *键 {
                            最大亲和度 = 亲和度.max(最大亲和度);
                        }
                    }
                    记忆量 -= 最大亲和度;
                }
            }
            指标.memory = Some(记忆量);
            let 归一化记忆量 = 记忆量 / 解.len() as f64;
            目标函数 += 归一化记忆量 * 参数.正则化强度;
        }
        (指标, 目标函数)
    }

    fn 接受新解(&mut self) {
        self.编码结果.clone_from(&self.编码结果缓冲);
        self.计数桶列表.clone_from(&self.计数桶列表缓冲);
    }
}
