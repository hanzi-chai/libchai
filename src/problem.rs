//! 优化问题的整体定义。
//!
//! 目前只定义了最基础的元素布局问题，以后可能会定义更复杂的问题，如元素布局 + 元素选取等等。
//!

use crate::constraints::Constraints;
use crate::error::Error;
use crate::interface::Interface;
use crate::objectives::metric::Metric;
use crate::objectives::Objective;
use crate::representation::{KeyMap, Representation};

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

    /// 生成一个初始解
    ///
    ///```ignore
    /// let candidate = problem.generate_candidate();
    ///```
    pub fn clone_candidate(&mut self, candidate: &Solution) -> Solution {
        candidate.clone()
    }

    /// 拷贝一份当前的解
    ///
    ///```ignore
    /// let new_candidate = problem.clone_candidate(&old_candidate);
    ///```
    pub fn generate_candidate(&mut self) -> Solution {
        self.representation.initial.clone()
    }

    /// 对一个解来打分
    /// M 可以是任意复杂的一个结构体，存放了各种指标；而后面的 f64 是对这个结构体的各项指标的加权平均得到的一个标量值。
    pub fn rank_candidate(&mut self, candidate: &Solution) -> (Metric, f64) {
        let (metric, loss) = self.objective.evaluate(candidate).unwrap();
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
        let new_config = self.representation.update_config(candidate);
        let metric = format!("{}", rank.0);
        interface.report_solution(new_config, metric, write_to_file);
    }
}
