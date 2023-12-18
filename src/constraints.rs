use crate::{config::AtomicConstraint, representation::{Representation, KeyMap, Element, Key}};
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
    pub fn new(representation: &Representation) -> Constraints {
        let elements = representation.initial.len();
        let alphabet = representation.config.form.alphabet.chars().map(|x| *representation.key_repr.get(&x).unwrap()).collect();
        let mut fixed: HashSet<Element> = HashSet::new();
        let mut narrowed: HashMap<Element, Vec<Key>> = HashMap::new();
        let mut grouped: HashMap<Element, Vec<Element>> = HashMap::new();
        let mut values: Vec<AtomicConstraint> = Vec::new();
        if let Some(constraints) = &representation.config.optimization.constraints {
            values.append(&mut constraints.elements.clone().unwrap_or_default());
            values.append(&mut constraints.indices.clone().unwrap_or_default());
            values.append(&mut constraints.element_indices.clone().unwrap_or_default());
            if let Some(grouping) = &constraints.grouping {
                for group in grouping {
                    let mut vec: Vec<usize> = Vec::new();
                    for AtomicConstraint {
                        element,
                        index,
                        keys: _,
                    } in group
                    {
                        let element = element.as_ref().unwrap();
                        let index = index.unwrap();
                        let name = format!("{}.{}", element.to_string(), index);
                        let number = representation
                            .element_repr
                            .get(&name)
                            .expect(&format!("{} 并不存在", name));
                        vec.push(*number);
                    }
                    for number in &vec {
                        grouped.insert(*number, vec.clone());
                    }
                }
            }
        }
        for atomic_constraint in &values {
            let AtomicConstraint {
                element,
                index,
                keys,
            } = atomic_constraint;
            for element_number in 0..elements {
                let one_element = representation.repr_element.get(&element_number).unwrap();
                let excluded = if one_element.contains(".") {
                    let vec: Vec<&str> = one_element.split('.').collect();
                    let p1 = vec[0];
                    let p2 = vec[1].parse::<usize>().unwrap();
                    index.is_some_and(|x| x != p2) || element.clone().is_some_and(|x| x != p1)
                } else {
                    index.is_some_and(|x| x == 1)
                        || element.clone().is_some_and(|x| x != *one_element)
                };
                if !excluded {
                    if let Some(keys) = keys {
                        narrowed.insert(element_number, keys.iter().map(|x| *representation.key_repr.get(x).unwrap()).collect());
                    } else {
                        fixed.insert(element_number);
                    }
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
                && !self.narrowed.contains_key(&key)
                && !self.grouped.contains_key(&key)
            {
                return key;
            }
        }
    }

    pub fn constrained_random_swap(&self, map: &KeyMap) -> KeyMap {
        let mut next = map.clone();
        let element1 = self.get_swappable_element();
        let element2 = self.get_swappable_element();
        next[element1] = map[element2];
        next[element2] = map[element1];
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
        let char = destinations.choose(&mut rng).unwrap();
        if let Some(group) = self.grouped.get(&movable_element) {
            for number in group {
                next[*number] = *char;
            }
        } else {
            next[movable_element] = *char;
        }
        next
    }
}
