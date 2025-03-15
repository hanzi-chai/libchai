use super::变异;
use crate::config::{AtomicConstraint, MappedKey, SolverConfig};
use crate::data::{键, 数据};
use crate::data::{元素, 元素映射};
use crate::错误;
use rand::seq::{IteratorRandom, SliceRandom};
use rand::{random, thread_rng};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::{HashMap, HashSet};

pub struct 默认操作 {
    fixed: HashSet<元素>,
    narrowed: HashMap<元素, Vec<键>>,
    alphabet: Vec<键>,
    radix: usize,    // 码表的基数
    elements: usize, // 键盘映射的元素个数
    变异配置: 变异配置,
}

#[skip_serializing_none]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct 变异配置 {
    pub random_move: f64,
    pub random_swap: f64,
    pub random_full_key_swap: f64,
}

pub const DEFAULT_MUTATE: 变异配置 = 变异配置 {
    random_move: 0.9,
    random_swap: 0.09,
    random_full_key_swap: 0.01,
};

impl 变异 for 默认操作 {
    fn 变异(&mut self, candidate: &mut 元素映射) -> Vec<元素> {
        let 变异配置 {
            random_move,
            random_swap,
            random_full_key_swap,
        } = self.变异配置;
        let sum = random_move + random_swap + random_full_key_swap;
        let ratio1 = random_move / sum;
        let ratio2 = (random_move + random_swap) / sum;
        let number: f64 = random();
        if number < ratio1 {
            self.有约束的随机移动(candidate)
        } else if number < ratio2 {
            self.有约束的随机交换(candidate)
        } else {
            self.有约束的整键随机交换(candidate)
        }
    }
}

// 默认的问题实现，使用配置文件中的约束来定义各种算子
impl 默认操作 {
    pub fn 新建(数据: &数据) -> Result<Self, 错误> {
        let (fixed, narrowed) = Self::make_constraints(数据)?;
        let config = 数据.配置.optimization.clone();
        let SolverConfig::SimulatedAnnealing(退火方法) = config.unwrap().metaheuristic.unwrap();
        let 变异配置 = 退火方法.search_method.unwrap_or(DEFAULT_MUTATE);
        let alphabet: Vec<_> = 数据
            .配置
            .form
            .alphabet
            .chars()
            .map(|x| *数据.键转数字.get(&x).unwrap()) // 在生成表示的时候已经确保了这里一定有对应的键
            .collect();
        Ok(Self {
            fixed,
            narrowed,
            alphabet,
            radix: 数据.进制 as usize,
            elements: 数据.初始映射.len(),
            变异配置,
        })
    }

    /// 传入配置表示来构造约束，把用户在配置文件中编写的约束「编译」成便于快速计算的数据结构
    fn make_constraints(
        representation: &数据,
    ) -> Result<(HashSet<元素>, HashMap<元素, Vec<键>>), 错误> {
        let mut fixed: HashSet<元素> = HashSet::new();
        let mut narrowed: HashMap<元素, Vec<键>> = HashMap::new();
        let mut values: Vec<AtomicConstraint> = Vec::new();
        let lookup = |x: String| {
            let element_number = representation.元素转数字.get(&x);
            element_number.ok_or(format!("{x} 不存在于键盘映射中"))
        };
        let optimization = representation
            .配置
            .optimization
            .as_ref()
            .ok_or("优化配置不存在")?;
        if let Some(constraints) = &optimization.constraints {
            values.append(&mut constraints.elements.clone().unwrap_or_default());
            values.append(&mut constraints.indices.clone().unwrap_or_default());
            values.append(&mut constraints.element_indices.clone().unwrap_or_default());
        }
        let mapping = &representation.配置.form.mapping;
        for atomic_constraint in &values {
            let AtomicConstraint {
                element,
                index,
                keys,
            } = atomic_constraint;
            let elements: Vec<usize> = match (element, index) {
                // 如果指定了元素和码位
                (Some(element), Some(index)) => {
                    let element = *lookup(数据::assemble(element, *index))?;
                    vec![element]
                }
                // 如果指定了码位
                (None, Some(index)) => {
                    let mut elements = Vec::new();
                    for (key, value) in mapping {
                        let normalized = value.normalize();
                        if let Some(MappedKey::Ascii(_)) = normalized.get(*index) {
                            let element = *lookup(数据::assemble(key, *index))?;
                            elements.push(element);
                        }
                    }
                    elements
                }
                // 如果指定了元素
                (Some(element), None) => {
                    let mapped = mapping
                        .get(element)
                        .ok_or(format!("约束中的元素 {element} 不在键盘映射中"))?;
                    let mut elements = Vec::new();
                    for (i, x) in mapped.normalize().iter().enumerate() {
                        if let MappedKey::Ascii(_) = x {
                            elements.push(*lookup(数据::assemble(element, i))?);
                        }
                    }
                    elements
                }
                _ => return Err("约束必须至少提供 element 或 index 之一".into()),
            };
            for element in elements {
                if let Some(keys) = keys {
                    let mut transformed = Vec::new();
                    for key in keys {
                        transformed.push(
                            *representation
                                .键转数字
                                .get(key)
                                .ok_or(format!("约束中的键 {key} 不在键盘映射中"))?,
                        );
                    }
                    if transformed.is_empty() {
                        return Err("约束中的键列表不能为空".into());
                    }
                    narrowed.insert(element, transformed);
                } else {
                    fixed.insert(element);
                }
            }
        }
        Ok((fixed, narrowed))
    }

    fn get_movable_element(&self) -> usize {
        let mut rng = thread_rng();
        loop {
            let key = (self.radix..self.elements).choose(&mut rng).unwrap();
            if !self.fixed.contains(&key) {
                return key;
            }
        }
    }

    fn get_swappable_element(&self) -> usize {
        let mut rng = thread_rng();
        loop {
            let key = (self.radix..self.elements).choose(&mut rng).unwrap();
            if !self.fixed.contains(&key) {
                return key;
            }
        }
    }

    pub fn 有约束的随机交换(&self, keymap: &mut 元素映射) -> Vec<元素> {
        let element1 = self.get_swappable_element();
        let key1 = keymap[element1];
        let mut element2 = self.get_swappable_element();
        while keymap[element2] == key1 {
            element2 = self.get_swappable_element();
        }
        let key2 = keymap[element2];
        let destinations1 = self.narrowed.get(&element1).unwrap_or(&self.alphabet);
        let destinations2 = self.narrowed.get(&element2).unwrap_or(&self.alphabet);
        //分开判断可行性。这样如果无法交换，至少移动一下。
        if destinations1.contains(&key2) {
            keymap[element1] = key2;
        }
        if destinations2.contains(&key1) {
            keymap[element2] = key1;
        }
        vec![element1, element2]
    }

    pub fn 有约束的整键随机交换(&self, keymap: &mut 元素映射) -> Vec<元素> {
        let mut rng = thread_rng();
        // 寻找一个可移动元素和一个它的可行移动位置，然后把这两个键上的所有元素交换
        // 这样交换不成也至少能移动一次
        let movable_element = self.get_movable_element();
        let key1 = keymap[movable_element];
        let mut destinations = self
            .narrowed
            .get(&movable_element)
            .unwrap_or(&self.alphabet)
            .clone();
        destinations.retain(|x| *x != key1);
        let key2 = destinations.choose(&mut rng).unwrap(); // 在编译约束时已经确保了这里一定有可行的移动位置
        let mut moved_elements = vec![];
        for (element, key) in keymap.iter_mut().enumerate() {
            if *key != key1 && *key != *key2 || self.fixed.contains(&element) {
                continue;
            }
            let destination = if *key == *key2 { key1 } else { *key2 };
            // 将元素移动到目标
            let destinations2 = self.narrowed.get(&element).unwrap_or(&self.alphabet);
            if destinations2.contains(&destination) {
                *key = destination;
            }
            moved_elements.push(element);
        }
        moved_elements
    }

    pub fn 有约束的随机移动(&self, keymap: &mut 元素映射) -> Vec<元素> {
        let mut rng = thread_rng();
        let movable_element = self.get_movable_element();
        let current = keymap[movable_element];
        let destinations = self
            .narrowed
            .get(&movable_element)
            .unwrap_or(&self.alphabet);
        let mut key = destinations.choose(&mut rng).unwrap(); // 在编译约束时已经确保了这里一定有可行的移动位置
        while *key == current {
            key = destinations.choose(&mut rng).unwrap();
        }
        keymap[movable_element] = *key;
        vec![movable_element]
    }
}
