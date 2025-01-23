//! 冰雪四拼手机键位布局的优化问题。
//!

use crate::{
    objectives::{metric::Metric, Objective},
    representation::{Element, Representation},
    Interface, Message,
};
use itertools::Itertools;

use super::{MutateConfig, Problem, Solution};

pub struct Snow4Layout {
    representation: Representation,
    objective: Objective,
    group1: Vec<Vec<[char; 3]>>,
    group2: Vec<Vec<char>>,
    group3: Vec<Vec<char>>,
    index: usize,
}

impl Problem for Snow4Layout {
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
        interface.post(Message::BetterSolution {
            metric: rank.0.clone(),
            config,
            save,
        })
    }

    fn mutate(&mut self, candidate: &mut Solution, _config: &MutateConfig) -> Vec<Element> {
        let index1 = self.index % self.group1.len();
        let index2 = (self.index / self.group1.len()) % self.group2.len();
        let index3 = (self.index / self.group1.len() / self.group2.len()) % self.group3.len();
        let info1 = self.group1[index1].clone();
        let info2 = self.group2[index2].clone();
        let info3 = self.group3[index3].clone();
        // b, p, m, d, t, n 不变
        // g, k, h, j, q, x, z, c, s, w, y, v
        for (i, elements) in vec![
            ["g", "k", "h"],
            ["j", "q", "x"],
            ["z", "c", "s"],
            ["w", "y", "v"],
        ]
        .into_iter()
        .enumerate()
        {
            for (j, element) in elements.iter().enumerate() {
                let repr = self.representation.element_repr[&element.to_string()];
                candidate[repr] = self.representation.key_repr[&info1[i][j]];
            }
        }
        // r, l, f
        for (i, element) in vec!["r", "l", "f"].into_iter().enumerate() {
            let repr = self.representation.element_repr[element];
            candidate[repr] = self.representation.key_repr[&info2[i]];
        }
        // a, e, i, o, u
        for (i, element) in vec!["a", "e", "i", "o", "u"].into_iter().enumerate() {
            let repr = self.representation.element_repr[element];
            candidate[repr] = self.representation.key_repr[&info3[i]];
        }
        self.index += 1;
        vec![]
    }
}

fn make_permutation<T: Clone>(elements: &Vec<T>) -> Vec<Vec<T>> {
    let length = elements.len();
    elements
        .iter()
        .permutations(length)
        .map(|p| p.into_iter().cloned().collect())
        .collect()
}

impl Snow4Layout {
    pub fn new(representation: Representation, objective: Objective) -> Self {
        let group1 = vec![
            ['G', 'K', 'H'],
            ['J', 'Q', 'X'],
            ['Z', 'C', 'S'],
            ['W', 'Y', 'V'],
        ];
        let group2 = vec!['R', 'L', 'F'];
        let group3 = vec!['A', 'E', 'I', 'O', 'U'];
        Self {
            representation,
            objective,
            group1: make_permutation(&group1),
            group2: make_permutation(&group2),
            group3: make_permutation(&group3),
            index: 0,
        }
    }
}
