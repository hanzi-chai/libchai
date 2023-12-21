use crate::config::{SolverConfig, SearchConfig};
use crate::constraints::Constraints;
use crate::metaheuristics::{simulated_annealing, Metaheuristics};
use crate::metric::Metric;
use crate::objective::Objective;
use crate::representation::{Buffer, KeyMap, Representation};
use chrono::Local;
use rand::random;
use std::fs;
use std::time::Duration;

// 未来可能会有更加通用的解定义
type Solution = KeyMap;

pub struct ElementPlacementProblem {
    representation: Representation,
    constraints: Constraints,
    objective: Objective,
    buffer: Buffer,
}

impl ElementPlacementProblem {
    pub fn new(
        representation: Representation,
        constraints: Constraints,
        objective: Objective,
        buffer: Buffer,
    ) -> Self {
        Self {
            representation,
            constraints,
            objective,
            buffer,
        }
    }
}

impl Metaheuristics<Solution, Metric> for ElementPlacementProblem {
    fn clone_candidate(&mut self, candidate: &Solution) -> Solution {
        return candidate.clone();
    }

    fn generate_candidate(&mut self) -> Solution {
        return self.representation.initial.clone();
    }

    fn rank_candidate(&mut self, candidate: &Solution) -> (Metric, f64) {
        let (metric, loss) = self.objective.evaluate(candidate, &mut self.buffer);
        return (metric, loss);
    }

    fn tweak_candidate(&mut self, candidate: &Solution) -> Solution {
        let method = self.representation.config.optimization.metaheuristic.search_method.as_ref().unwrap_or(&SearchConfig { random_move: 0.9, random_swap: 0.1 });
        let ratio = method.random_move / (method.random_move + method.random_swap);
        if random::<f64>() < ratio {
            self.constraints.constrained_random_move(candidate)
        } else {
            self.constraints.constrained_random_swap(candidate)
        }
    }

    fn save_candidate(&self, candidate: &Solution, rank: &(Metric, f64), write_to_file: bool) {
        let time = Local::now();
        let prefix = format!("{}", time.format("%m-%d+%H_%M_%S_%3f"));
        let config_path = format!("output/{}.yaml", prefix);
        let metric_path = format!("output/{}.txt", prefix);
        println!(
            "{} 系统搜索到了一个更好的方案，评测指标如下：",
            time.format("%H:%M:%S")
        );
        print!("{}", rank.0);
        let new_config = self.representation.update_config(&candidate);
        let content = serde_yaml::to_string(&new_config).unwrap();
        if write_to_file {
            fs::write(metric_path, format!("{}", rank.0)).unwrap();
            fs::write(config_path, content).unwrap();
            println!(
                "方案文件保存于 {}.yaml 中，评测指标保存于 {}.txt 中",
                prefix, prefix
            );
        }
        println!("");
    }
}

impl ElementPlacementProblem {
    pub fn solve(&mut self) -> Solution {
        let _ = fs::create_dir_all("output").expect("should be able to create an output directory");
        let SolverConfig { parameters, runtime, report_after, .. } = self
            .representation
            .config
            .optimization
            .metaheuristic
            .clone();
        if let Some(parameters) = parameters {
            simulated_annealing::solve(self, parameters.clone(), report_after)
        } else {
            let runtime = runtime.unwrap_or(10);
            let duration = Duration::new(runtime * 60, 0);
            simulated_annealing::autosolve(self, duration, report_after)
        }
    }
}
