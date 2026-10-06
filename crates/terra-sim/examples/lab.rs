//! The lab runner (design §7.1): runs a lab scenario over several seeds and
//! prints what each counted, with the median.
//!
//! `cargo run --release -p terra-sim --example lab -- scenarios/a1-thornbush.ron --seeds 10`
//!
//! `--seed N` runs only seed N, as a baseline report's replay command does
//! (design §7.6).

#[path = "common/mod.rs"]
mod common;

use std::ops::RangeInclusive;
use std::process::ExitCode;

use common::args::{self, positive, seeds};
use terra_sim::{DataPack, LabScenario, report};

fn main() -> ExitCode {
    args::run(
        "lab <scenario.ron> [--seeds N | --seed N]",
        parse,
        |(path, seeds)| {
            #[expect(
                clippy::disallowed_methods,
                reason = "the lab runner reads the scenario file; the sim itself never does"
            )]
            let text = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(error) => {
                    eprintln!("can't read {path}: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let data = DataPack::builtin().expect("the built-in data pack is valid");
            let lab = match LabScenario::from_ron(&text, &data) {
                Ok(lab) => lab,
                Err(error) => {
                    eprintln!("{path}: {error}");
                    return ExitCode::FAILURE;
                }
            };
            let runs: Vec<_> = seeds
                .map(|seed| (seed, lab.run(data.clone(), seed)))
                .collect();
            print!("{}", report(&runs, &data));
            ExitCode::SUCCESS
        },
    )
}

/// The scenario's path and which seeds to run: 1 to N, 10 unless `--seeds`
/// says, or only the one `--seed` names.
fn parse(args: &[String]) -> Result<(String, RangeInclusive<u64>), String> {
    let mut path = None;
    let mut seed_range = 1..=10;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--seeds" {
            seed_range = seeds(&mut args)?;
        } else if arg == "--seed" {
            let n = positive("--seed", &mut args)?;
            seed_range = n..=n;
        } else if path.is_none() {
            path = Some(arg.clone());
        } else {
            return Err(format!("unexpected argument {arg}"));
        }
    }
    Ok((path.ok_or("which scenario?")?, seed_range))
}
