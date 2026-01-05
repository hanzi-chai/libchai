use super::编码器;
use crate::contexts::default::{默认上下文, 默认决策, 默认安排};
use crate::{元素, 可编码对象, 最大词长, 编码, 编码信息, 自动上屏, 键};
use crate::{最大元素编码长度, 棱镜, 错误};
use rustc_hash::FxHashMap;
use std::iter::zip;

pub type 线性化决策 = Vec<[键; 最大元素编码长度]>;

#[derive(Clone)]
pub struct 编码空间 {
    pub 线性表: Vec<u8>,
    pub 线性表长度: usize,
    pub 哈希表: FxHashMap<编码, u8>,
}

impl 编码空间 {
    #[inline(always)]
    pub fn 添加(&mut self, 编码: u64) {
        if 编码 < self.线性表长度 as u64 {
            let 编码 = 编码 as usize;
            self.线性表[编码] = self.线性表[编码].saturating_add(1);
        } else {
            self.哈希表
                .entry(编码)
                .and_modify(|x| *x = x.saturating_add(1))
                .or_insert(1);
        }
    }

    #[inline(always)]
    pub fn 查找数量(&self, 编码: u64) -> u8 {
        if 编码 < self.线性表长度 as u64 {
            self.线性表[编码 as usize]
        } else {
            *self.哈希表.get(&编码).unwrap_or(&0)
        }
    }
}

#[derive(Debug)]
pub struct 简码配置 {
    pub prefix: usize,
    pub select_keys: Vec<键>,
}

pub struct 编码配置 {
    pub 进制: u64,
    pub 乘数列表: Vec<u64>,
    pub 最大码长: usize,
    pub 自动上屏查找表: 自动上屏,
    pub 选择键: Vec<键>,
    pub 首选键: 键,
    pub 简码配置列表: Option<[Vec<简码配置>; 最大词长]>,
}

impl 编码配置 {
    pub fn new(上下文: &默认上下文) -> Result<Self, 错误> {
        let 编码器配置 = &上下文.配置.encoder;
        let 最大码长 = 编码器配置.max_length;
        if 最大码长 >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }
        let 自动上屏查找表 = 上下文.预处理自动上屏()?;
        let mut 简码配置列表 = None;
        if let Some(configs) = &编码器配置.short_code {
            简码配置列表 = Some(上下文.预处理简码配置(configs.clone())?);
        }
        Ok(Self {
            自动上屏查找表,
            最大码长,
            进制: 上下文.棱镜.进制,
            乘数列表: (0..=最大码长)
                .map(|x| 上下文.棱镜.进制.pow(x as u32))
                .collect(),
            选择键: 上下文.选择键.clone(),
            首选键: 上下文.选择键[0],
            简码配置列表,
        })
    }

    #[inline(always)]
    pub fn 生成编码(
        &self, 原始编码: u64, 原始编码候选位置: u8, 选择键乘数: u64
    ) -> u64 {
        // 如果位于首选，检查是否能自动上屏，不能则加上首选键
        if 原始编码候选位置 == 0 {
            if *self.自动上屏查找表.get(原始编码 as usize).unwrap_or(&true) {
                return 原始编码;
            } else {
                return 原始编码 + self.首选键 * 选择键乘数;
            }
        }
        // 如果不是首选，无论如何都加上选择键
        let 选择键 = *self
            .选择键
            .get(原始编码候选位置 as usize)
            .unwrap_or(&self.选择键[0]);
        原始编码 + 选择键 * 选择键乘数
    }
}

pub struct 默认编码器 {
    棱镜: 棱镜,
    编码配置: 编码配置,
    词信息: Vec<可编码对象>,
    全码空间: 编码空间,
    简码空间: 编码空间,
    包含元素的词: Vec<Vec<usize>>,
}

impl 默认编码器 {
    /// 提供配置表示、拆分表、词表和共用资源来创建一个编码引擎
    /// 字需要提供拆分表
    /// 词只需要提供词表，它对应的拆分序列从字推出
    pub fn 新建(上下文: &默认上下文) -> Result<Self, 错误> {
        let 编码器配置 = &上下文.配置.encoder;
        let 最大码长 = 编码器配置.max_length;
        if 最大码长 >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }
        let 词信息 = 上下文.词列表.clone();
        let 线性表长度 = 上下文.棱镜.进制.pow(最大码长 as u32) as usize;
        let 全码空间 = 编码空间 {
            线性表: vec![u8::default(); 线性表长度],
            线性表长度,
            哈希表: FxHashMap::default(),
        };
        let 简码空间 = 全码空间.clone();
        let mut 包含元素的词 = vec![];
        for _ in 0..=上下文.棱镜.元素转数字.len() {
            包含元素的词.push(vec![]);
        }
        for (词序号, 词) in 词信息.iter().enumerate() {
            for (元素, _) in &词.元素序列 {
                包含元素的词[*元素].push(词序号);
            }
        }
        let 编码配置 = 编码配置::new(上下文)?;
        Ok(Self {
            编码配置,
            词信息,
            全码空间,
            简码空间,
            包含元素的词,
            棱镜: 上下文.棱镜.clone(),
        })
    }

    fn 重置(&mut self) {
        self.全码空间.线性表.iter_mut().for_each(|x| {
            *x = 0;
        });
        self.全码空间.哈希表.clear();
        self.简码空间.线性表.iter_mut().for_each(|x| {
            *x = 0;
        });
        self.简码空间.哈希表.clear();
    }

    fn 输出全码(
        &mut self,
        映射: &线性化决策,
        移动的元素: &Option<Vec<元素>>,
        编码结果: &mut [编码信息],
    ) {
        let 编码配置 = &self.编码配置;
        // 根据移动的元素更新编码结果，如果没有移动的元素则直接全部生成
        if let Some(移动的元素) = 移动的元素 {
            for 元素 in 移动的元素 {
                for 索引 in &self.包含元素的词[*元素] {
                    let 词 = &self.词信息[*索引];
                    let 全码信息 = &mut 编码结果[*索引].全码;
                    let mut 原始编码 = 0;
                    for ((元素, 位置), 乘数) in zip(&词.元素序列, &编码配置.乘数列表)
                    {
                        原始编码 += 映射[*元素][*位置] * 乘数;
                    }
                    全码信息.原始编码 = 原始编码;
                }
            }
        } else {
            for (词, 编码信息) in zip(&self.词信息, 编码结果.iter_mut()) {
                let 全码信息 = &mut 编码信息.全码;
                let mut 原始编码 = 0;
                for ((元素, 位置), 乘数) in zip(&词.元素序列, &编码配置.乘数列表)
                {
                    原始编码 += 映射[*元素][*位置] * 乘数;
                }
                全码信息.原始编码 = 原始编码;
            }
        }

        for (编码信息, 词) in 编码结果.iter_mut().zip(&self.词信息) {
            let 全码信息 = &mut 编码信息.全码;
            // 首先查找编码空间中已经有了多少个同样原始编码的词，确定候选位置
            let 原始编码候选位置 = self.全码空间.查找数量(全码信息.原始编码);
            全码信息.原始编码候选位置 = 原始编码候选位置;
            self.全码空间.添加(全码信息.原始编码);
            // 然后生成实际编码，并向全码信息中写入实际编码和实际编码是否重码的信息，用于测评
            // 注意：对于全码来说，暂且忽略次选及之后的选择键的影响，统一视为首选进行编码。这可以避免在四码类方案中大量出现五码的编码，影响性能
            let 乘数 = 编码配置.乘数列表[词.元素序列.len()];
            let 编码 = 编码配置.生成编码(全码信息.原始编码, 0, 乘数);
            let 是否重码 = 原始编码候选位置 > 0;
            全码信息.更新(编码, 是否重码);
        }
    }

    fn 输出简码(&mut self, 编码结果: &mut [编码信息]) {
        let 编码配置 = &self.编码配置;
        let 简码配置列表 = 编码配置.简码配置列表.as_ref().unwrap();
        // 优先简码
        for (词, 编码结果) in zip(&self.词信息, 编码结果.iter_mut()) {
            if 词.简码等级 == u64::MAX {
                continue;
            }
            let 原始编码 = 编码结果.全码.原始编码 % 编码配置.乘数列表[词.简码等级 as usize];
            编码结果.简码.原始编码 = 原始编码;
            let 序号 = self.简码空间.查找数量(原始编码);
            let 乘数 = 编码配置.乘数列表[词.简码等级 as usize];
            let 编码 = 编码配置.生成编码(原始编码, 序号, 乘数);
            编码结果.简码.更新(编码, 序号 > 0);
            self.简码空间.添加(原始编码);
        }
        // 常规简码
        for (词, 编码结果) in zip(&self.词信息, 编码结果.iter_mut()) {
            if 词.简码等级 != u64::MAX {
                continue;
            }
            let 简码配置 = &简码配置列表[词.词长 - 1];
            let mut 有简码 = false;
            let 全码信息 = &编码结果.全码;
            let 简码信息 = &mut 编码结果.简码;
            for 出简方式 in 简码配置 {
                let 简码配置 {
                    prefix,
                    select_keys,
                } = 出简方式;
                let 乘数 = 编码配置.乘数列表[*prefix];
                // 如果根本没有这么多码，就放弃
                if 全码信息.原始编码 < 乘数 {
                    continue;
                }
                // 将全码截取一部分出来，检查当前简码位置上的候选数量是否达到上限
                let 原始编码 = 全码信息.原始编码 % 乘数;
                let 序号 = self.全码空间.查找数量(原始编码) + self.简码空间.查找数量(原始编码);
                if 序号 >= select_keys.len() as u8 {
                    continue;
                }
                // 如果没有达到上限，就可以出这个简码
                let 编码 = 编码配置.生成编码(原始编码, 序号, 乘数);
                简码信息.原始编码 = 原始编码;
                简码信息.原始编码候选位置 = 序号;
                简码信息.更新(编码, false);
                self.简码空间.添加(原始编码);
                有简码 = true;
                break;
            }
            if !有简码 {
                let 序号 = self.简码空间.查找数量(全码信息.原始编码);
                简码信息.原始编码 = 全码信息.原始编码;
                简码信息.原始编码候选位置 = 序号;
                简码信息.更新(全码信息.实际编码, 序号 > 0);
                self.简码空间.添加(全码信息.原始编码);
            }
        }
    }

    pub fn 线性化(&self, 决策: &默认决策, 棱镜: &棱镜) -> 线性化决策 {
        let mut result: 线性化决策 = vec![Default::default(); 决策.元素.len()];
        for (序号, 安排) in 决策.元素.iter().enumerate() {
            if 序号 < 棱镜.进制 as usize {
                result[序号] = [序号 as 键, 0, 0, 0];
                continue;
            }
            match 安排 {
                默认安排::归并(元素) => {
                    result[序号] = result[*元素];
                    continue;
                }
                默认安排::键位(列表) => {
                    for (i, (元素, 位置)) in 列表.iter().enumerate() {
                        result[序号][i] = result[*元素][*位置];
                    }
                }
                _ => {}
            }
        }
        result
    }
}

impl 编码器 for 默认编码器 {
    type 决策 = 默认决策;
    fn 编码(
        &mut self, 映射: &Self::决策, 移动的元素: &Option<Vec<元素>>, 输出: &mut [编码信息]
    ) {
        self.重置();
        let 线性化决策 = self.线性化(映射, &self.棱镜);
        self.输出全码(&线性化决策, 移动的元素, 输出);
        if self.编码配置.简码配置列表.is_none()
            || self.编码配置.简码配置列表.as_ref().unwrap().is_empty()
        {
            return;
        }
        self.输出简码(输出);
    }
}
