use chai::data::数据;
use chai::encoders::default::默认编码器;
use chai::encoders::编码器;
use chai::objectives::default::默认目标函数;
use chai::objectives::目标函数;
use chai::operators::default::默认操作;
use chai::{命令行, 错误};
use criterion::{criterion_group, criterion_main, Criterion};

fn 计时(数据: 数据, 名称: &str, b: &mut Criterion) -> Result<(), 错误> {
    let mut 编码器 = 默认编码器::新建(&数据)?;
    let mut 目标函数 = 默认目标函数::新建(&数据)?;
    let 操作 = 默认操作::新建(&数据)?;
    b.bench_function(名称, |b| {
        b.iter(|| {
            let mut 映射 = 数据.初始映射.clone();
            let 模拟移动的元素 = 操作.有约束的随机移动(&mut 映射);
            let mut 编码结果 = 编码器.编码(&映射, &Some(模拟移动的元素));
            目标函数.计算(&mut 编码结果);
        })
    });
    Ok(())
}

fn 四码定长字词(b: &mut Criterion) {
    let 数据 = 命令行::读取("米十五笔");
    计时(数据, "四码定长字词", b).unwrap();
}

fn 四码定长单字(b: &mut Criterion) {
    let mut 数据 = 命令行::读取("米十五笔");
    数据.词列表 = 数据
        .词列表
        .into_iter()
        .filter(|x| x.名称.chars().count() == 1)
        .collect();
    数据
        .配置
        .optimization
        .as_mut()
        .unwrap()
        .objective
        .words_short = None;
    计时(数据, "四码定长单字", b).unwrap();
}

fn 六码顶功(b: &mut Criterion) {
    let 数据 = 命令行::读取("冰雪四拼");
    计时(数据, "六码顶功", b).unwrap();
}

criterion_group!(benches, 四码定长字词, 四码定长单字, 六码顶功);
criterion_main!(benches);
