use chai::config::Config;
use chai::problems::default::DefaultProblem;
use criterion::{criterion_group, criterion_main, Criterion};

use chai::{CommandLine, Command};
use chai::encoder::Encoder;
use chai::objectives::Objective;
use chai::problems::Problem;
use chai::representation::{AssembleList, Assets};
use chai::{Error, representation::Representation};
use std::path::PathBuf;

fn simulate_cli_input(name: &str) -> (Config, AssembleList, Assets) {
    let config = format!("examples/{}.yaml", name);
    let elements = format!("examples/{}.txt", name);
    let cli = CommandLine {
        command: Command::Optimize,
        config: Some(PathBuf::from(config)),
        elements: Some(PathBuf::from(elements)),
        frequency: None,
        key_distribution: None,
        pair_equivalence: None,
    };
    cli.prepare_file()
}

fn process_cli_input(
    config: Config,
    elements: AssembleList,
    assets: Assets,
    b: &mut Criterion,
) -> Result<(), Error> {
    let representation = Representation::new(config)?;
    let encoder = Encoder::new(&representation, elements, &assets)?;
    let objective = Objective::new(&representation, encoder, assets)?;
    let mut problem = DefaultProblem::new(representation, objective)?;
    let candidate = problem.initialize();
    b.bench_function("Evaluation", |b| {
        b.iter(|| {
            problem.rank(&candidate, &None);
        })
    });
    Ok(())
}

fn length_3(b: &mut Criterion) {
    let (config, resource, assets) = simulate_cli_input("easy");
    process_cli_input(config, resource, assets, b).unwrap();
}

fn length_4(b: &mut Criterion) {
    let (config, resource, assets) = simulate_cli_input("mswb");
    process_cli_input(config, resource, assets, b).unwrap();
}

fn length_4_char_only(b: &mut Criterion) {
    let (mut config, resource, assets) = simulate_cli_input("mswb");
    let resource = resource
        .into_iter()
        .filter(|x| x.name.chars().count() == 1)
        .collect();
    config.optimization.as_mut().unwrap().objective.words_short = None;
    process_cli_input(config, resource, assets, b).unwrap();
}

fn length_6(b: &mut Criterion) {
    let (config, resource, assets) = simulate_cli_input("snow");
    process_cli_input(config, resource, assets, b).unwrap();
}

criterion_group!(benches, length_3, length_4, length_4_char_only, length_6);
criterion_main!(benches);
