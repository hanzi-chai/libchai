//! 冰雪双拼的优化问题。
//!

use std::collections::HashMap;

use rand::{random, seq::SliceRandom};

use crate::{
    objectives::{metric::Metric, Objective},
    representation::{Element, Representation},
    Interface, Message,
};

use super::{Problem, Solution};

pub struct Snow2 {
    representation: Representation,
    objective: Objective,
    initials: Vec<Element>,
    finals: Vec<Element>,
    // 《中华通韵》中的韵部
    final_groups: Vec<[Element; 4]>,
    radicals: Vec<Element>,
}

impl Problem for Snow2 {
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

    fn mutate(
        &self,
        candidate: &mut Solution,
        _config: &super::MutateConfig,
    ) -> Vec<crate::representation::Element> {
        let number: f64 = random();
        // 一共有三种情况：
        if number < 0.8 {
            // 1. 随机移动一个字根
            self.randomly_move_radical(candidate)
        } else if number < 0.9 {
            // 2. 随机移动一个声母
            self.randomly_move_initial(candidate)
        } else {
            // 3. 随机交换两个《中华通韵》中的韵部
            self.randomly_swap_final(candidate)
        }
    }
}

impl Snow2 {
    pub fn new(representation: Representation, objective: Objective) -> Self {
        let mut initials = vec![];
        let mut finals_map = HashMap::new();
        let mut radicals = vec![];
        let mut finals = vec![];
        for element in (representation.radix as usize)..representation.initial.len() {
            let repr = &representation.repr_element[&element];
            if repr.starts_with("落声") {
                initials.push(element);
            } else if repr.starts_with("落韵") {
                finals.push(element);
                let chars: Vec<char> = repr.chars().collect();
                let tone = chars[chars.len() - 1].to_digit(10).unwrap() - 1;
                let toneless: String = chars[..(chars.len() - 1)].iter().collect();
                finals_map
                    .entry(toneless)
                    .or_insert([Element::default(); 4])[tone as usize] = element;
            } else {
                radicals.push(element);
            }
        }
        let final_groups: Vec<[Element; 4]> = finals_map.into_iter().map(|(_, v)| v).collect();
        Self {
            representation,
            objective,
            initials,
            finals,
            final_groups,
            radicals,
        }
    }

    pub fn randomly_move_radical(&self, candidate: &mut Solution) -> Vec<Element> {
        let mut rng = rand::thread_rng();
        let element: Element = *self.radicals.choose(&mut rng).unwrap();
        let destinations: Vec<char> = "zawevmio;/".chars().collect();
        let key = destinations.choose(&mut rng).unwrap();
        candidate[element] = self.representation.key_repr[key];
        vec![element]
    }

    pub fn randomly_move_initial(&self, candidate: &mut Solution) -> Vec<Element> {
        let mut rng = rand::thread_rng();
        let element: Element = *self.initials.choose(&mut rng).unwrap();
        let destinations: Vec<char> = "qsxdcrftgbyhnujklp".chars().collect();
        let key = destinations.choose(&mut rng).unwrap();
        candidate[element] = self.representation.key_repr[key];
        vec![element]
    }

    pub fn randomly_move_final(&self, candidate: &mut Solution) -> Vec<Element> {
        let mut rng = rand::thread_rng();
        let element: Element = *self.finals.choose(&mut rng).unwrap();
        let destinations: Vec<char> = "qazwsxedcrfvtgbyhnujmik,ol.p;/".chars().collect();
        let key = destinations.choose(&mut rng).unwrap();
        candidate[element] = self.representation.key_repr[key];
        vec![element]
    }

    pub fn randomly_swap_final(&self, candidate: &mut Solution) -> Vec<Element> {
        let mut rng = rand::thread_rng();
        let group1 = *self.final_groups.choose(&mut rng).unwrap();
        let group2 = *self.final_groups.choose(&mut rng).unwrap();
        for (element1, element2) in group1.iter().zip(group2.iter()) {
            let (key1, key2) = (candidate[*element1], candidate[*element2]);
            candidate[*element1] = key2;
            candidate[*element2] = key1;
        }
        vec![
            group1[0], group1[1], group1[2], group1[3], group2[0], group2[1], group2[2], group2[3],
        ]
    }
}
