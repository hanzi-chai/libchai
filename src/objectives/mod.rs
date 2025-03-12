//! 优化问题的目标函数。
//!
//!

use crate::encoders::Encoder;
use crate::representation::{DistributionLoss, Element, KeyMap, Label};
use metric::Metric;

pub mod cache;
pub mod metric;
pub mod default;

pub trait Objective: Clone {
    fn evaluate<E: Encoder>(
        &mut self,
        encoder: &mut E,
        candidate: &KeyMap,
        moved_elements: &Option<Vec<Element>>,
    ) -> (Metric, f64);
}

#[derive(Clone)]
pub struct Parameters {
    ideal_distribution: Vec<DistributionLoss>,
    pair_equivalence: Vec<f64>,
    fingering_types: Vec<Label>,
}
