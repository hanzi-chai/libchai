#![feature(test)]
extern crate test;
use chai::config::Config;
use test::Bencher;

use std::path::PathBuf;
use chai::metaheuristics::Metaheuristics;
use chai::representation::{Assets, Buffer, RawSequenceMap};
use chai::{representation::Representation, error::Error};
use chai::encoder::Encoder;
use chai::objectives::Objective;
use chai::constraints::Constraints;
use chai::problem::ElementPlacementProblem;
use chai::cli::{Cli, Command};

fn simulate_cli_input(config: &str, elements: &str) -> Cli {
    Cli {
        command: Command::Optimize,
        config: Some(PathBuf::from(config)),
        elements: Some(PathBuf::from(elements)),
        words: None,
        character_frequency: None,
        word_frequency: None,
        frequency: None,
        key_distribution: None,
        pair_equivalence: None,
    }
}

fn process_cli_input(config: Config, characters: RawSequenceMap, words: Vec<String>, assets: Assets, b: &mut Bencher) -> Result<(), Error> {
    let representation = Representation::new(config)?;
    let encoder = Encoder::new(&representation, characters, words, &assets)?;
    let buffer = Buffer::new(&encoder);
    let objective = Objective::new(&representation, encoder, assets)?;
    let constraints = Constraints::new(&representation)?;
    let mut problem =
        ElementPlacementProblem::new(representation, constraints, objective, buffer)?;
    let mut candidate = problem.generate_candidate();
    b.iter(|| {
        candidate = problem.tweak_candidate(&candidate);
        problem.rank_candidate(&candidate);
    });
    Ok(())
}

#[bench]
fn length_4(b: &mut Bencher) -> Result<(), Error> {
    let cli = simulate_cli_input("config.yaml", "elements.txt");
    let (config, characters, words, assets) = cli.prepare_file();
    process_cli_input(config, characters, words, assets, b)
}

#[bench]
fn length_4_char_only(b: &mut Bencher) -> Result<(), Error> {
    let cli = simulate_cli_input("config.yaml", "elements.txt");
    let (mut config, characters, words, assets) = cli.prepare_file();
    config.optimization.as_mut().unwrap().objective.words_full = None;
    process_cli_input(config, characters, words, assets, b)
}

#[bench]
fn length_6(b: &mut Bencher) -> Result<(), Error> {
    let cli = simulate_cli_input("sbpy.yaml", "sbpy.txt");
    let (config, characters, words, assets) = cli.prepare_file();
    process_cli_input(config, characters, words, assets, b)
}
