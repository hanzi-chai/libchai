//! 输出接口的抽象层
//! 
//! 定义了一个特征，指定了所有在退火计算的过程中需要向用户反馈的数据。命令行界面、Web 界面只需要各自实现这些方法，就可向用户报告各种用户数据，实现方式可以很不一样。

use crate::config::Config;

pub trait Interface {
    fn prepare_output(&self);

    fn init_autosolve(&self);

    fn report_trial_t_max(&self, temperature: f64, accept_rate: f64);

    fn report_t_max(&self, temperature: f64);

    fn report_trial_t_min(&self, temperature: f64, improve_rate: f64);

    fn report_t_min(&self, temperature: f64);

    fn report_parameters(&self, t_max: f64, t_min: f64, steps: usize);

    fn report_elapsed(&self, time: u128);

    fn report_schedule(&self, step: usize, temperature: f64, metric: String);

    fn report_solution(&self, config: Config, metric: String, save: bool);
}
