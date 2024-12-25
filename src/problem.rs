//! 优化问题的整体定义。
//!
//! 目前只定义了最基础的元素布局问题，以后可能会定义更复杂的问题，如元素布局 + 元素选取等等。
//!

use crate::constraints::Constraints;
use crate::objectives::metric::Metric;
use crate::objectives::Objective;
use crate::representation::{Element, KeyMap, Representation};
use crate::{Error, Interface, Message};

// 未来可能会有更加通用的解定义
pub type Solution = KeyMap;

pub struct Problem {
    representation: Representation,
    pub constraints: Constraints,
    objective: Objective,
}

impl Problem {
    pub fn new(
        representation: Representation,
        constraints: Constraints,
        objective: Objective,
    ) -> Result<Self, Error> {
        Ok(Self {
            representation,
            constraints,
            objective,
        })
    }

    /// 拷贝一份初始解
    pub fn initial_candidate(&mut self) -> Solution {
        self.representation.initial.clone()
    }

    /// 对一个解来打分
    /// Metric 存放了各种指标；后面的 f64 是对各项指标加权求和得到的标量值
    pub fn rank_candidate(
        &mut self,
        candidate: &Solution,
        moved_elements: &Option<Vec<Element>>,
    ) -> (Metric, f64) {
        let (metric, loss) = self.objective.evaluate(candidate, moved_elements);
        (metric, loss)
    }

    /// 保存当前的一个解
    pub fn save_candidate(
        &self,
        candidate: &Solution,
        rank: &(Metric, f64),
        write_to_file: bool,
        interface: &dyn Interface,
    ) {
        let config = self.representation.update_config(candidate);
        let metric = format!("{}", rank.0);
        let config = serde_yaml::to_string(&config).unwrap();
        interface.post(Message::BetterSolution {
            metric,
            config,
            save: write_to_file,
        })
    }
}
