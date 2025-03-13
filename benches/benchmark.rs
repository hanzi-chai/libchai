use chai::config::Config;
use chai::encoders::default::DefaultEncoder;
use chai::objectives::default::DefaultObjective;
use chai::problems::default::DefaultProblem;
use chai::problems::Problem;
use chai::representation::Assets;
use chai::{representation::Representation, Error};
use chai::{Args, CommandLine};
use criterion::{criterion_group, criterion_main, Criterion};

fn simulate_cli_input(name: &str) -> (Config, Assets) {
    let args = Args::生成(name);
    CommandLine::new(args, None).prepare_file()
}

fn process_cli_input(config: Config, assets: Assets, b: &mut Criterion) -> Result<(), Error> {
    let representation = Representation::new(config)?;
    let Assets {
        encodables,
        key_distribution,
        pair_equivalence,
    } = assets;
    let length = encodables.len();
    let encoder = DefaultEncoder::new(&representation, encodables)?;
    let objective =
        DefaultObjective::new(&representation, key_distribution, pair_equivalence, length)?;
    let mut problem = DefaultProblem::new(representation, objective, encoder)?;
    let candidate = problem.initialize();
    b.bench_function("Evaluation", |b| {
        b.iter(|| {
            problem.rank(&candidate, &None);
        })
    });
    Ok(())
}

fn length_3(b: &mut Criterion) {
    let (config, assets) = simulate_cli_input("easy");
    process_cli_input(config, assets, b).unwrap();
}

fn length_4(b: &mut Criterion) {
    let (config, assets) = simulate_cli_input("mswb");
    process_cli_input(config, assets, b).unwrap();
}

fn length_4_char_only(b: &mut Criterion) {
    let (mut config, mut assets) = simulate_cli_input("mswb");
    assets.encodables = assets
        .encodables
        .into_iter()
        .filter(|x| x.name.chars().count() == 1)
        .collect();
    config.optimization.as_mut().unwrap().objective.words_short = None;
    process_cli_input(config, assets, b).unwrap();
}

fn length_6(b: &mut Criterion) {
    let (config, assets) = simulate_cli_input("snow");
    process_cli_input(config, assets, b).unwrap();
}

criterion_group!(benches, length_4, length_4_char_only, length_6);
criterion_main!(benches);
