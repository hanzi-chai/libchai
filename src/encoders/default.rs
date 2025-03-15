use super::{简码配置, 编码器, 编码空间, 编码配置};
use crate::data::{元素, 元素映射, 可编码对象, 数据, 编码信息};
use crate::错误;
use rustc_hash::FxHashMap;
use std::iter::zip;

pub struct 默认编码器 {
    buffer: Vec<编码信息>,
    config: 编码配置,
    encodables: Vec<可编码对象>,
    full_space: 编码空间,
    short_space: 编码空间,
    involved_message: Vec<Vec<usize>>,
}

impl 默认编码器 {
    /// 提供配置表示、拆分表、词表和共用资源来创建一个编码引擎
    /// 字需要提供拆分表
    /// 词只需要提供词表，它对应的拆分序列从字推出
    pub fn 新建(representation: &数据) -> Result<Self, 错误> {
        let encoder = &representation.配置.encoder;
        let max_length = encoder.max_length;
        if max_length >= 8 {
            return Err("目前暂不支持最大码长大于等于 8 的方案计算！".into());
        }
        let encodables = representation.词列表.clone();
        let buffer = encodables.iter().map(编码信息::new).collect();
        let vector_length = representation.进制.pow(max_length as u32) as usize;
        let vector = vec![u8::default(); vector_length];
        let full_space = 编码空间 {
            vector,
            vector_length,
            hashmap: FxHashMap::default(),
        };
        let short_space = full_space.clone();
        let mut involved_message = vec![];
        for _ in 0..=representation.元素转数字.len() {
            involved_message.push(vec![]);
        }
        for (index, encodable) in encodables.iter().enumerate() {
            for element in &encodable.元素序列 {
                involved_message[*element].push(index);
            }
        }
        let config = 编码配置::new(representation)?;
        let encoder = Self {
            buffer,
            config,
            encodables,
            full_space,
            short_space,
            involved_message,
        };
        Ok(encoder)
    }

    fn 重置(&mut self) {
        self.full_space.vector.iter_mut().for_each(|x| {
            *x = 0;
        });
        self.full_space.hashmap.clear();
        self.short_space.vector.iter_mut().for_each(|x| {
            *x = 0;
        });
        self.short_space.hashmap.clear();
    }

    fn 输出全码(&mut self, keymap: &元素映射, moved_elements: &Option<Vec<元素>>) {
        let config = &self.config;
        let buffer = &mut self.buffer;
        let weights: Vec<_> = (0..=config.max_length)
            .map(|x| config.radix.pow(x as u32))
            .collect();
        if let Some(moved_elements) = moved_elements {
            for element in moved_elements {
                for index in &self.involved_message[*element] {
                    let pointer = &mut buffer[*index];
                    let encodable = &self.encodables[*index];
                    let sequence = &encodable.元素序列;
                    let full = &mut pointer.全码;
                    let mut code = 0_u64;
                    for (element, weight) in zip(sequence, &weights) {
                        code += keymap[*element] * weight;
                    }
                    full.原始编码 = code;
                    let actual = config.wrap_actual(code, 0, weights[sequence.len()]);
                    full.写入编码(actual);
                }
            }
        } else {
            for (encodable, pointer) in zip(&self.encodables, buffer.iter_mut()) {
                let sequence = &encodable.元素序列;
                let full = &mut pointer.全码;
                let mut code = 0_u64;
                for (element, weight) in zip(sequence, &weights) {
                    code += keymap[*element] * weight;
                }
                // 对于全码，计算实际编码时不考虑第二及以后的选重键
                full.原始编码 = code;
                let actual = config.wrap_actual(code, 0, weights[sequence.len()]);
                full.写入编码(actual);
            }
        }

        for pointer in buffer.iter_mut() {
            let full = &mut pointer.全码;
            let duplicate = self.full_space.rank(full.原始编码) > 0;
            full.写入选重(duplicate);
            self.full_space.insert(full.原始编码);
        }
    }

    fn 输出简码(&mut self) {
        let config = &self.config;
        let buffer = &mut self.buffer;
        let weights: Vec<_> = (0..=config.max_length)
            .map(|x| config.radix.pow(x as u32))
            .collect();
        let short_code = config.short_code.as_ref().unwrap();
        // 优先简码
        for (encodable, pointer) in zip(&self.encodables, buffer.iter_mut()) {
            if encodable.简码等级 == u64::MAX {
                continue;
            }
            let code = pointer.全码.原始编码 % weights[encodable.简码等级 as usize];
            let rank = self.short_space.rank(code);
            let actual = config.wrap_actual(code, rank, weights[encodable.简码等级 as usize]);
            pointer.简码.写入(actual, rank > 0);
            self.short_space.insert(code);
        }
        // 常规简码
        for (pointer, encodable) in zip(buffer.iter_mut(), &self.encodables) {
            if encodable.简码等级 != u64::MAX {
                continue;
            }
            let schemes = &short_code[encodable.词长 - 1];
            let mut has_short = false;
            let full = &pointer.全码;
            let short = &mut pointer.简码;
            for scheme in schemes {
                let 简码配置 {
                    prefix,
                    select_keys,
                } = scheme;
                let weight = weights[*prefix];
                // 如果根本没有这么多码，就放弃
                if full.原始编码 < weight {
                    continue;
                }
                // 首先将全码截取一部分出来
                let code = full.原始编码 % weight;
                let rank = self.full_space.rank(code) + self.short_space.rank(code);
                if rank >= select_keys.len() as u8 {
                    continue;
                }
                let actual = config.wrap_actual(code, rank, weight);
                short.写入(actual, false);
                self.short_space.insert(code);
                has_short = true;
                break;
            }
            if !has_short {
                let code = full.原始编码;
                let rank = self.short_space.rank(full.原始编码);
                short.写入(full.实际编码, rank > 0);
                self.short_space.insert(code);
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
        if self.config.short_code.is_none() || self.config.short_code.as_ref().unwrap().is_empty() {
            return &mut self.buffer;
        }
        self.输出简码();
        &mut self.buffer
    }
}
