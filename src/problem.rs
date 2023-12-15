use std::fs;
use std::time::{Instant, Duration};
use crate::config::{KeyMap, Config, Cache, MetaheuristicConfig};
use crate::constraints::Constraints;
use crate::metaheuristics::{Metaheuristics, simulated_annealing, hill_climbing};
use crate::metric::Metric;
use crate::objective::Objective;
use rand::random;
use chrono::Local;

// 未来可能会有更加通用的解定义
type Solution = KeyMap;

pub struct ElementPlacementProblem {
    config: Config,
    cache: Cache,
    constraints: Constraints,
    objective: Objective,
}

impl ElementPlacementProblem {
    pub fn new(
        config: Config,
        cache: Cache,
        constraints: Constraints,
        objective: Objective,
    ) -> Self {
        Self {
            config,
            cache,
            constraints,
            objective,
        }
    }
}

impl Metaheuristics<Solution, Metric> for ElementPlacementProblem {
    fn clone_candidate(&mut self, candidate: &Solution) -> Solution {
        return candidate.clone();
    }

    fn generate_candidate(&mut self) -> Solution {
        return self.cache.initial.clone();
    }

    fn rank_candidate(&mut self, candidate: &Solution) -> (Metric, f64) {
        let start = Instant::now();
        let (metric, loss) = self.objective.evaluate(candidate);
        let _elapsed = start.elapsed();
        return (metric, loss);
    }

    fn tweak_candidate(&mut self, candidate: &Solution) -> Solution {
        if random::<f64>() < 0.9 {
            self.constraints.constrained_random_move(candidate)
        } else {
            self.constraints.constrained_random_swap(candidate)
        }
    }

    fn save_candidate(&self, candidate: &Solution, rank: &(Metric, f64)) {
        let time = Local::now();
        let prefix = format!("{}", time.format("%m-%d+%H_%M_%S_%3f"));
        let config_path = format!("output/{}.yaml", prefix);
        let metric_path = format!("output/{}.txt", prefix);
        println!("搜索到一个更好的方案，已为您保存在 {}.yaml 中", prefix);
        print!("{}", rank.0);
        fs::write(metric_path, format!("{}", rank.0)).unwrap();
        let new_config = self.cache.update_config(&self.config, &candidate);
        let content = serde_yaml::to_string(&new_config).unwrap();
        fs::write(config_path, content).unwrap();
    }
}

impl ElementPlacementProblem {
    pub fn solve(&mut self) -> Solution {
        let _ = fs::create_dir_all("output").expect("should be able to create an output directory");
        let metaheuristic = self.config.optimization.metaheuristic.clone();
        match metaheuristic {
            MetaheuristicConfig::SimulatedAnnealing { runtime, parameters } => {
                if let Some(parameters) = parameters {
                    simulated_annealing::solve(self, parameters.clone())
                } else if let Some(runtime) = runtime {
                    let duration = Duration::new(runtime * 60, 0);
                    simulated_annealing::autosolve(self, duration)
                } else {
                    panic!("退火算法无法执行，因为配置文件中既没有提供参数，也没有提供运行时间");
                }
            
            }
            MetaheuristicConfig::HillClimbing { runtime } => {
                let duration = Duration::new(runtime * 60, 0);
                hill_climbing::solve(self, duration)
            }
        }
    }

}
