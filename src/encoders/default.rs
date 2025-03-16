use super::{简码配置, 编码器, 编码空间, 编码配置};
use crate::data::{元素, 元素映射, 可编码对象, 数据, 编码信息};
use crate::错误;
use rustc_hash::FxHashMap;
use std::iter::zip;

pub struct 默认编码器 {
    编码结果: Vec<编码信息>,
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
    pub fn 新建(数据: &数据) -> Result<Self, 错误> {
        let encoder = &数据.配置.encoder;
        let max_length = encoder.max_length;
        if max_length >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }
        let 词信息 = 数据.词列表.clone();
        let 编码结果 = 词信息.iter().map(编码信息::new).collect();
        let vector_length = 数据.进制.pow(max_length as u32) as usize;
        let vector = vec![u8::default(); vector_length];
        let 全码空间 = 编码空间 {
            vector,
            vector_length,
            hashmap: FxHashMap::default(),
        };
        let 简码空间 = 全码空间.clone();
        let mut 包含元素的词 = vec![];
        for _ in 0..=数据.元素转数字.len() {
            包含元素的词.push(vec![]);
        }
        for (index, encodable) in 词信息.iter().enumerate() {
            for element in &encodable.元素序列 {
                包含元素的词[*element].push(index);
            }
        }
        let 编码配置 = 编码配置::new(数据)?;
        let encoder = Self {
            编码结果,
            编码配置,
            词信息,
            全码空间,
            简码空间,
            包含元素的词,
        };
        Ok(encoder)
    }

    fn 重置(&mut self) {
        self.全码空间.vector.iter_mut().for_each(|x| {
            *x = 0;
        });
        self.全码空间.hashmap.clear();
        self.简码空间.vector.iter_mut().for_each(|x| {
            *x = 0;
        });
        self.简码空间.hashmap.clear();
    }

    fn 输出全码(&mut self, 映射: &元素映射, 移动的元素: &Option<Vec<元素>>) {
        let 编码配置 = &self.编码配置;
        let 编码结果 = &mut self.编码结果;
        let 乘数: Vec<_> = (0..=编码配置.最大码长)
            .map(|x| 编码配置.进制.pow(x as u32))
            .collect();
        if let Some(移动的元素) = 移动的元素 {
            for 元素 in 移动的元素 {
                for 索引 in &self.包含元素的词[*元素] {
                    let 编码信息 = &mut 编码结果[*索引];
                    let 词 = &self.词信息[*索引];
                    let 全码 = &mut 编码信息.全码;
                    let mut 原始编码 = 0_u64;
                    for (element, weight) in zip(&词.元素序列, &乘数) {
                        原始编码 += 映射[*element] * weight;
                    }
                    全码.原始编码 = 原始编码;
                    let 编码 = 编码配置.生成编码(原始编码, 0, 乘数[词.元素序列.len()]);
                    全码.写入编码(编码);
                }
            }
        } else {
            for (词, 编码结果) in zip(&self.词信息, 编码结果.iter_mut()) {
                let 全码 = &mut 编码结果.全码;
                let mut 原始编码 = 0_u64;
                for (element, weight) in zip(&词.元素序列, &乘数) {
                    原始编码 += 映射[*element] * weight;
                }
                // 对于全码，计算实际编码时不考虑第二及以后的选重键
                全码.原始编码 = 原始编码;
                let 编码 = 编码配置.生成编码(原始编码, 0, 乘数[词.元素序列.len()]);
                全码.写入编码(编码);
            }
        }

        for 编码信息 in 编码结果.iter_mut() {
            let 全码 = &mut 编码信息.全码;
            let 是否重码 = self.全码空间.查找数量(全码.原始编码) > 0;
            全码.写入选重(是否重码);
            self.全码空间.添加(全码.原始编码);
        }
    }

    fn 输出简码(&mut self) {
        let 编码配置 = &self.编码配置;
        let 编码结果 = &mut self.编码结果;
        let 乘数: Vec<_> = (0..=编码配置.最大码长)
            .map(|x| 编码配置.进制.pow(x as u32))
            .collect();
        let 简码配置列表 = 编码配置.简码配置列表.as_ref().unwrap();
        // 优先简码
        for (词, 编码结果) in zip(&self.词信息, 编码结果.iter_mut()) {
            if 词.简码等级 == u64::MAX {
                continue;
            }
            let 原始编码 = 编码结果.全码.原始编码 % 乘数[词.简码等级 as usize];
            编码结果.简码.原始编码 = 原始编码;
            let 序号 = self.简码空间.查找数量(原始编码);
            let 编码 = 编码配置.生成编码(原始编码, 序号, 乘数[词.简码等级 as usize]);
            编码结果.简码.写入(编码, 序号 > 0);
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
                let 权重 = 乘数[*prefix];
                // 如果根本没有这么多码，就放弃
                if 全码信息.原始编码 < 权重 {
                    continue;
                }
                // 首先将全码截取一部分出来
                let 原始编码 = 全码信息.原始编码 % 权重;
                let 序号 = self.全码空间.查找数量(原始编码) + self.简码空间.查找数量(原始编码);
                if 序号 >= select_keys.len() as u8 {
                    continue;
                }
                let 编码 = 编码配置.生成编码(原始编码, 序号, 权重);
                简码信息.原始编码 = 原始编码;
                简码信息.写入(编码, false);
                self.简码空间.添加(原始编码);
                有简码 = true;
                break;
            }
            if !有简码 {
                let 序号 = self.简码空间.查找数量(全码信息.原始编码);
                简码信息.原始编码 = 全码信息.原始编码;
                简码信息.写入(全码信息.实际编码, 序号 > 0);
                self.简码空间.添加(全码信息.原始编码);
            }
        }
    }
}

impl 编码器 for 默认编码器 {
    fn 编码(
        &mut self,
        keymap: &元素映射,
        moved_elements: &Option<Vec<元素>>,
    ) -> &mut Vec<编码信息> {
        self.重置();
        self.输出全码(keymap, moved_elements);
        if self.编码配置.简码配置列表.is_none()
            || self.编码配置.简码配置列表.as_ref().unwrap().is_empty()
        {
            return &mut self.编码结果;
        }
        self.输出简码();
        &mut self.编码结果
    }
}
