use chai::contexts::default::默认上下文;
use chai::encoders::default::默认编码器;
use chai::interfaces::command_line::{从命令行参数创建, 命令, 默认命令行参数};
use chai::objectives::default::默认目标函数;
use chai::objectives::目标函数;
use chai::operators::default::默认操作;
use chai::错误;
use criterion::{criterion_group, criterion_main, Criterion};
use std::path::PathBuf;

pub fn 读取(name: &str) -> 默认上下文 {
    let config = format!("examples/{}.yaml", name);
    let elements = format!("examples/{}.txt", name);
    let 参数 = 默认命令行参数 {
        command: 命令::Optimize,
        config: Some(PathBuf::from(config)),
        encodables: Some(PathBuf::from(elements)),
        key_distribution: None,
        pair_equivalence: None,
        threads: None,
    };
    let 输入 = 从命令行参数创建(&参数);
    默认上下文::新建(输入).expect("Failed to create context")
}

fn 计时(上下文: 默认上下文, 名称: &str, b: &mut Criterion) -> Result<(), 错误> {
    let 编码器 = 默认编码器::新建(&上下文)?;
    let mut 目标函数 = 默认目标函数::新建(&上下文, 编码器)?;
    目标函数.计算(&上下文.初始映射, &None);
    let 操作 = 默认操作::新建(&上下文)?;
    b.bench_function(名称, |b| {
        b.iter(|| {
            let mut 解 = 上下文.初始映射.clone();
            let 解变化 = 操作.有约束的随机移动(&mut 解);
            目标函数.计算(&解, &Some(解变化));
        })
    });
    Ok(())
}

fn 四码定长字词(b: &mut Criterion) {
    let 上下文 = 读取("米十五笔");
    计时(上下文, "四码定长字词", b).unwrap();
}

fn 四码定长单字(b: &mut Criterion) {
    let mut 上下文 = 读取("米十五笔");
    上下文.词列表 = 上下文
        .词列表
        .into_iter()
        .filter(|x| x.名称.chars().count() == 1)
        .collect();
    上下文
        .配置
        .optimization
        .as_mut()
        .unwrap()
        .objective
        .words_short = None;
    计时(上下文, "四码定长单字", b).unwrap();
}

fn 六码顶功(b: &mut Criterion) {
    let 上下文 = 读取("冰雪四拼");
    计时(上下文, "六码顶功", b).unwrap();
}

criterion_group!(benches, 四码定长字词, 四码定长单字, 六码顶功);
criterion_main!(benches);
