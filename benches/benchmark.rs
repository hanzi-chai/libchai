use chai::config::Config;
use chai::encoder::occupation::Occupation;
use chai::encoder::simple_occupation::SimpleOccupation;
use criterion::{criterion_group, criterion_main, Criterion};

use chai::cli::{Cli, Command};
use chai::constraints::Constraints;
use chai::encoder::{Driver, Encoder};
use chai::objectives::Objective;
use chai::problem::Problem;
use chai::representation::{AssembleList, Assets};
use chai::{error::Error, representation::Representation};
use std::path::PathBuf;

fn simulate_cli_input(name: &str) -> (Config, AssembleList, Assets) {
    let config = format!("examples/{}.yaml", name);
    let elements = format!("examples/{}.txt", name);
    let cli = Cli {
        command: Command::Optimize,
        config: Some(PathBuf::from(config)),
        elements: Some(PathBuf::from(elements)),
        frequency: None,
        key_distribution: None,
        pair_equivalence: None,
    };
    cli.prepare_file()
}

fn do_benchmark(
    representation: Representation,
    elements: AssembleList,
    assets: Assets,
    driver: Box<dyn Driver>,
    b: &mut Criterion,
) -> Result<(), Error> {
    let constraints = Constraints::new(&representation)?;
    let encoder = Encoder::new(&representation, elements, &assets, driver)?;
    let objective = Objective::new(&representation, encoder, assets)?;
    let mut problem = Problem::new(representation, constraints, objective)?;
    let candidate = problem.generate_candidate();
    b.bench_function("Evaluation", |b| {
        b.iter(|| {
            problem.rank_candidate(&candidate);
        })
    });
    Ok(())
}

fn process_cli_input(
    config: Config,
    elements: AssembleList,
    assets: Assets,
    b: &mut Criterion,
) -> Result<(), Error> {
    let representation = Representation::new(config)?;
    let driver: Box<dyn Driver> = if representation.config.encoder.max_length <= 4 {
        Box::new(SimpleOccupation::new(representation.get_space()))
    } else {
        Box::new(Occupation::new(representation.get_space()))
    };
    do_benchmark(representation, elements, assets, driver, b)
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
