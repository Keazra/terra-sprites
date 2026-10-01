//! The lab runner (design §7.1): runs a lab scenario over several seeds and
//! prints what each counted, with the median.
//!
//! `cargo run --release -p terra-sim --example lab -- scenarios/a1-thornbush.ron --seeds 10`
//!
//! `--seed N` runs only seed N, as a baseline report's replay command does
//! (design §7.6).

use std::process::ExitCode;

use terra_sim::{DataPack, LabScenario, report};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (path, seeds) = match parse(&args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}\nusage: lab <scenario.ron> [--seeds N | --seed N]");
            return ExitCode::FAILURE;
        }
    };
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
}

/// The scenario's path and which seeds to run: 1 to N, 10 unless `--seeds`
/// says, or only the one `--seed` names.
fn parse(args: &[String]) -> Result<(String, std::ops::RangeInclusive<u64>), String> {
    let mut path = None;
    let mut seeds = 1..=10;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--seeds" || arg == "--seed" {
            let n = args.next().ok_or(format!("{arg} needs a number"))?;
            let n = n
                .parse()
                .ok()
                .filter(|&n| n > 0)
                .ok_or(format!("{arg} takes a whole number above 0, not {n}"))?;
            seeds = if arg == "--seeds" { 1..=n } else { n..=n };
        } else if path.is_none() {
            path = Some(arg.clone());
        } else {
            return Err(format!("unexpected argument {arg}"));
        }
    }
    Ok((path.ok_or("which scenario?")?, seeds))
}
