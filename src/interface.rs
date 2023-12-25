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

    fn report_solution(&self, config: String, metric: String, save: bool);
}
