//! 优化问题的约束。

use crate::{
    config::{AtomicConstraint, GroupConstraint},
    representation::{Element, Key, KeyMap, Representation},
};
use rand::{seq::SliceRandom, thread_rng, Rng};
use std::collections::{HashMap, HashSet};

pub struct Constraints {
    pub alphabet: Vec<Key>,
    pub elements: usize,
    pub fixed: HashSet<Element>,
    pub narrowed: HashMap<Element, Vec<Key>>,
    pub grouped: HashMap<Element, Vec<Element>>,
}

impl Constraints {
    /// 传入配置表示来构造约束，把用户在配置文件中编写的约束「编译」成便于快速计算的数据结构
    pub fn new(representation: &Representation) -> Constraints {
        let elements = representation.initial.len();
        let alphabet = representation
            .config
            .form
            .alphabet
            .chars()
            .map(|x| *representation.key_repr.get(&x).unwrap())
            .collect();
        let mut fixed: HashSet<Element> = HashSet::new();
        let mut narrowed: HashMap<Element, Vec<Key>> = HashMap::new();
        let mut grouped: HashMap<Element, Vec<Element>> = HashMap::new();
        let mut values: Vec<AtomicConstraint> = Vec::new();
        let lookup = |x: &String| {
            *representation
                .element_repr
                .get(x)
                .expect(&format!("{} 不存在于键盘映射中", x))
        };
        let assemble = |x: &String, i: &usize| format!("{}.{}", x.to_string(), i);
        if let Some(constraints) = &representation.config.optimization.constraints {
            values.append(&mut constraints.elements.clone().unwrap_or_default());
            values.append(&mut constraints.indices.clone().unwrap_or_default());
            values.append(&mut constraints.element_indices.clone().unwrap_or_default());
            if let Some(grouping) = &constraints.grouping {
                for group in grouping {
                    let mut vec: Vec<usize> = Vec::new();
                    for GroupConstraint { element, index } in group {
                        vec.push(lookup(&assemble(element, index)));
                    }
                    for number in &vec {
                        grouped.insert(*number, vec.clone());
                    }
                }
            }
        }
        let mapping = &representation.config.form.mapping;
        for atomic_constraint in &values {
            let AtomicConstraint {
                element,
                index,
                keys,
            } = atomic_constraint;
            let elements: Vec<String> = match (element, index) {
                (Some(element), Some(index)) => {
                    vec![assemble(element, index)]
                }
                (None, Some(index)) => mapping
                    .iter()
                    .filter_map(|(key, value)| {
                        if *index == 0 {
                            if value.len() == 1 {
                                Some(key.clone())
                            } else {
                                Some(assemble(key, index))
                            }
                        } else {
                            if value.len() > *index {
                                Some(assemble(key, index))
                            } else {
                                None
                            }
                        }
                    })
                    .collect(),
                (Some(element), None) => {
                    let mapped = mapping
                        .get(element)
                        .expect(&format!("约束中的元素 {} 不在键盘映射中", element));
                    let mapped_len: Vec<char> = mapped.chars().collect();
                    if mapped_len.len() == 1 {
                        vec![element.clone()]
                    } else {
                        (0..mapped_len.len())
                            .map(|index| format!("{}.{}", element.to_string(), index))
                            .collect()
                    }
                }
                _ => panic!("约束必须至少提供 element 或 index 之一"),
            };
            for element in elements {
                let element_number = lookup(&element);
                if let Some(keys) = keys {
                    narrowed.insert(
                        element_number,
                        keys.iter()
                            .map(|x| *representation.key_repr.get(x).unwrap())
                            .collect(),
                    );
                } else {
                    fixed.insert(element_number);
                }
            }
        }
        Constraints {
            alphabet,
            elements,
            fixed,
            narrowed,
            grouped,
        }
    }

    fn get_movable_element(&self) -> usize {
        let mut rng = thread_rng();
        loop {
            let key = rng.gen_range(0..self.elements);
            if !self.fixed.contains(&key) {
                return key;
            }
        }
    }

    fn get_swappable_element(&self) -> usize {
        let mut rng = thread_rng();
        loop {
            let key = rng.gen_range(0..self.elements);
            if !self.fixed.contains(&key)
                //&& !self.narrowed.contains_key(&key)
                && !self.grouped.contains_key(&key)
            {
                return key;
            }
        }
    }

    fn swap_narrowed_elements(&self, map: &KeyMap, element1: usize, element2: usize) -> KeyMap {
        let mut next = map.clone();
        let destinations1 = self
            .narrowed
            .get(&element1)
            .unwrap_or(&self.alphabet);
        let destinations2 = self
            .narrowed
            .get(&element2)
            .unwrap_or(&self.alphabet);
        //分开判断可行性。这样如果无法交换，至少移动一下。
        if destinations1.contains(&map[element2]) {
            next[element1] = map[element2];
        }
        if destinations2.contains(&map[element1]) {
            next[element2] = map[element1];
        }
        next
    }

    pub fn constrained_random_swap(&self, map: &KeyMap) -> KeyMap {
        let element1 = self.get_swappable_element();
        let element2 = self.get_swappable_element();
        self.swap_narrowed_elements(map, element1, element2)
    }

    pub fn constrained_full_key_swap(&self, map: &KeyMap) -> KeyMap {
        let mut rng = thread_rng();
        let mut next = map.clone();
        //寻找一个可移动元素。这样交换不成也至少能移动一次
        let movable_element = self.get_movable_element();
        let destinations = self
            .narrowed
            .get(&movable_element)
            .unwrap_or(&self.alphabet);
        let key = destinations
            .choose(&mut rng)
            .expect(&format!("元素 {} 无法移动", movable_element));
        let former_key = map[movable_element];
        for i in 0..map.len() {
            if map[i] == former_key || map[i] == *key {
                let mut destination = *key;
                if map[i] == destination {
                    destination = former_key;
                }
                //将元素移动到目标
                //考虑到组合中的元素必然在同样的键上，有同样的约束条件，也必然跟随移动，这里不再判断组合
                let destinations2 = self
                    .narrowed
                    .get(&i)
                    .unwrap_or(&self.alphabet);
                if destinations2.contains(&destination) {
                    next[i] = destination;
                }
            }
        }
        next
    }

    pub fn constrained_random_move(&self, map: &KeyMap) -> KeyMap {
        let mut rng = thread_rng();
        let mut next = map.clone();
        let movable_element = self.get_movable_element();
        let destinations = self
            .narrowed
            .get(&movable_element)
            .unwrap_or(&self.alphabet);
        let key = destinations
            .choose(&mut rng)
            .expect(&format!("元素 {} 无法移动", movable_element));
        if let Some(group) = self.grouped.get(&movable_element) {
            for number in group {
                next[*number] = *key;
            }
        } else {
            next[movable_element] = *key;
        }
        next
    }
}
