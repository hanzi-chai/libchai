//! 默认优化问题。

use crate::config::{AtomicConstraint, MappedKey};
use crate::objectives::metric::Metric;
use crate::objectives::Objective;
use crate::representation::{assemble, Element, Key, KeyMap, Representation};
use crate::{Error, Interface, Message};
use rand::random;
use rand::{seq::SliceRandom, thread_rng, Rng};
use std::collections::{HashMap, HashSet};

use super::{MutateConfig, Problem, Solution};

pub struct DefaultProblem {
    representation: Representation,
    objective: Objective,
    fixed: HashSet<Element>,
    narrowed: HashMap<Element, Vec<Key>>,
    alphabet: Vec<Key>,
}

impl Problem for DefaultProblem {
    fn initialize(&mut self) -> Solution {
        self.representation.initial.clone()
    }

    fn rank(
        &mut self,
        candidate: &Solution,
        moved_elements: &Option<Vec<Element>>,
    ) -> (Metric, f64) {
        let (metric, loss) = self.objective.evaluate(candidate, moved_elements);
        (metric, loss)
    }

    fn update(
        &self,
        candidate: &Solution,
        rank: &(Metric, f64),
        save: bool,
        interface: &dyn Interface,
    ) {
        let config = self.representation.update_config(candidate);
        let metric = format!("{}", rank.0);
        let config = serde_yaml::to_string(&config).unwrap();
        interface.post(Message::BetterSolution {
            metric,
            config,
            save,
        })
    }

    fn mutate(&self, candidate: &mut Solution, config: &MutateConfig) -> Vec<Element> {
        let sum = config.random_move + config.random_swap + config.random_full_key_swap;
        let ratio1 = config.random_move / sum;
        let ratio2 = (config.random_move + config.random_swap) / sum;
        let number: f64 = random();
        if number < ratio1 {
            self.constrained_random_move(candidate)
        } else if number < ratio2 {
            self.constrained_random_swap(candidate)
        } else {
            self.constrained_full_key_swap(candidate)
        }
    }
}

// 默认的问题实现，使用配置文件中的约束来定义各种算子
impl DefaultProblem {
    pub fn new(representation: Representation, objective: Objective) -> Result<Self, Error> {
        let (fixed, narrowed) = Self::make_constraints(&representation)?;
        let alphabet: Vec<_> = representation
            .config
            .form
            .alphabet
            .chars()
            .map(|x| *representation.key_repr.get(&x).unwrap()) // 在生成表示的时候已经确保了这里一定有对应的键
            .collect();
        Ok(Self {
            representation,
            objective,
            fixed,
            narrowed,
            alphabet,
        })
    }

    /// 传入配置表示来构造约束，把用户在配置文件中编写的约束「编译」成便于快速计算的数据结构
    pub fn make_constraints(
        representation: &Representation,
    ) -> Result<(HashSet<Element>, HashMap<Element, Vec<Key>>), Error> {
        let mut fixed: HashSet<Element> = HashSet::new();
        let mut narrowed: HashMap<Element, Vec<Key>> = HashMap::new();
        let mut values: Vec<AtomicConstraint> = Vec::new();
        let lookup = |x: String| {
            let element_number = representation.element_repr.get(&x);
            element_number.ok_or(format!("{x} 不存在于键盘映射中"))
        };
        let optimization = representation
            .config
            .optimization
            .as_ref()
            .ok_or("优化配置不存在")?;
        if let Some(constraints) = &optimization.constraints {
            values.append(&mut constraints.elements.clone().unwrap_or_default());
            values.append(&mut constraints.indices.clone().unwrap_or_default());
            values.append(&mut constraints.element_indices.clone().unwrap_or_default());
        }
        let mapping = &representation.config.form.mapping;
        for atomic_constraint in &values {
            let AtomicConstraint {
                element,
                index,
                keys,
            } = atomic_constraint;
            let elements: Vec<usize> = match (element, index) {
                // 如果指定了元素和码位
                (Some(element), Some(index)) => {
                    let element = *lookup(assemble(element, *index))?;
                    vec![element]
                }
                // 如果指定了码位
                (None, Some(index)) => {
                    let mut elements = Vec::new();
                    for (key, value) in mapping {
                        let normalized = value.normalize();
                        if let Some(MappedKey::Ascii(_)) = normalized.get(*index) {
                            let element = *lookup(assemble(key, *index))?;
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
                            elements.push(*lookup(assemble(element, i))?);
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
                                .key_repr
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
        let radix = self.representation.radix as usize;
        let elements = self.representation.initial.len();
        let mut rng = thread_rng();
        loop {
            let key = rng.gen_range(radix..elements);
            if !self.fixed.contains(&key) {
                return key;
            }
        }
    }

    fn get_swappable_element(&self) -> usize {
        let radix = self.representation.radix as usize;
        let elements = self.representation.initial.len();
        let mut rng = thread_rng();
        loop {
            let key = rng.gen_range(radix..elements);
            if !self.fixed.contains(&key) {
                return key;
            }
        }
    }

    pub fn constrained_random_swap(&self, keymap: &mut KeyMap) -> Vec<Element> {
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

    pub fn constrained_full_key_swap(&self, keymap: &mut KeyMap) -> Vec<Element> {
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

    pub fn constrained_random_move(&self, keymap: &mut KeyMap) -> Vec<Element> {
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
