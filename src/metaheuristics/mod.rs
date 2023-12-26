//! 优化问题的求解方法。
//!
//! 本模块中的代码复制自一个[已有的项目](https://www.alfie.wtf/rustdoc/metaheuristics/metaheuristics/)，但是考虑到其余算法对输入方案优化参考意义不大，所以只复制了退火算法部分。
//!
//! 但是，为了保证可扩展性，仍然保留了这个库中对于不同类型算法和不同类型问题的特征抽象，即只要一个问题定义了 Metaheuristic 这个 trait，就能用所有不同的算法求解；而任何一个算法都可以只依赖于 Metaheuristic 这个 trait 里提供的方法来求解一个问题。相当于建立了一个多对多的模块化设计，这样也许以后使用遗传算法等其他方法也不需要大改结构。
//!

use crate::interface::Interface;
pub mod simulated_annealing;

/// 任何问题只要实现了这个 trait，就能用所有算法来求解
pub trait Metaheuristics<T, M> {
    /// 生成一个初始解
    ///
    ///```ignore
    /// let candidate = problem.generate_candidate();
    ///```
    fn generate_candidate(&mut self) -> T;

    /// 拷贝一份当前的解
    ///
    ///```ignore
    /// let new_candidate = problem.clone_candidate(&old_candidate);
    ///```
    fn clone_candidate(&mut self, candidate: &T) -> T;

    /// 对一个解来打分
    /// M 可以是任意复杂的一个结构体，存放了各种指标；而后面的 f64 是对这个结构体的各项指标的加权平均得到的一个标量值。
    fn rank_candidate(&mut self, candidate: &T) -> (M, f64);

    /// 基于现有的一个解通过随机扰动创建一个新的解
    ///
    ///```ignore
    /// let new_candidate = problem.tweak_candidate(&old_candidate);
    ///```
    fn tweak_candidate(&mut self, candidate: &T) -> T;

    /// 保存当前的一个解
    fn save_candidate(
        &self,
        candidate: &T,
        rank: &(M, f64),
        write_to_file: bool,
        interface: &dyn Interface,
    );
}
