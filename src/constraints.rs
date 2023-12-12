use crate::config::{AtomicConstraint, Config, KeyMap};
use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet};

pub struct Constraints {
    pub alphabet: Vec<char>,
    pub elements: Vec<String>,
    pub fixed: HashSet<String>,
    pub narrowed: HashMap<String, Vec<char>>,
}

impl Constraints {
    pub fn new(config: &Config) -> Constraints {
        let elements: Vec<String> = config.form.mapping.keys().map(|x| x.to_string()).collect();
        let alphabet = config.form.alphabet.clone();
        let mut fixed: HashSet<String> = HashSet::new();
        let mut narrowed: HashMap<String, Vec<char>> = HashMap::new();
        for atomic_constraint in &config.optimization.constraints.values {
            let AtomicConstraint {
                element,
                index,
                keys,
            } = atomic_constraint;
            for one_element in &elements {
                let excluded = if one_element.contains(".") {
                    let vec: Vec<&str> = one_element.split('.').collect();
                    let p1 = vec[0];
                    let p2 = vec[1].parse::<usize>().unwrap();
                    index.is_some_and(|x| x != p2) || element.clone().is_some_and(|x| x != p1)
                } else {
                    index.is_some_and(|x| x == 1) || element.clone().is_some_and(|x| x != *one_element)
                };
                if !excluded {
                    if let Some(keys) = keys {
                        narrowed.insert(one_element.to_string(), keys.clone());
                    } else {
                        fixed.insert(one_element.to_string());
                    }
                }
            }
        }
        Constraints {
            alphabet,
            elements,
            fixed,
            narrowed,
        }
    }

    fn get_movable_element(&self) -> &String {
        let mut rng = thread_rng();
        loop {
            let index = rng.gen_range(0..self.elements.len());
            let key = &self.elements[index];
            if !self.fixed.contains(key) {
                return key;
            }
        }
    }

    fn get_swappable_element(&self) -> &String {
        let mut rng = thread_rng();
        loop {
            let index = rng.gen_range(0..self.elements.len());
            let key = &self.elements[index];
            if !self.fixed.contains(key) && !self.narrowed.contains_key(key) {
                return key;
            }
        }
    }

    pub fn constrained_random_swap(&self, map: &KeyMap) -> KeyMap {
        let mut next = map.clone();
        let element1 = self.get_swappable_element().to_string();
        let element2 = self.get_swappable_element().to_string();
        if let (Some(char1), Some(char2)) = (map.get(&element1), map.get(&element2)) {
            next.insert(element1.to_string(), *char2);
            next.insert(element2.to_string(), *char1);
        }
        next
    }

    pub fn constrained_random_move(&self, map: &KeyMap) -> KeyMap {
        let mut rng = thread_rng();
        let mut next = map.clone();
        let movable_element = self.get_movable_element().to_string();
        let destinations = self.narrowed.get(&movable_element).unwrap_or(&self.alphabet);
        let char_index = rng.gen_range(0..destinations.len());
        let char = destinations[char_index];
        next.insert(movable_element, char);
        next
    }
}
