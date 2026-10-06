//! The soak (design §7.4 A7, §7.6): the default world for a million ticks
//! with a random script of the Cursor's commands, from one seed. Baseline
//! runs soak on a new seed each time; this runs one again, as a baseline
//! report's replay command does.
//!
//! `cargo run --profile baseline -p terra-sim --example soak -- --seed N [--ticks T]`
//!
//! Build it with the `baseline` profile, which keeps the self-check after
//! every tick (§7.1), so a broken invariant panics naming the tick.

#[path = "common/args.rs"]
mod args;

use std::process::ExitCode;
use std::time::Instant;

use terra_sim::{DataPack, DeathCause, Soak};

fn main() -> ExitCode {
    let Some((seed, ticks)) = args::parse("soak --seed N [--ticks T]", parse) else {
        return ExitCode::FAILURE;
    };
    let data = DataPack::builtin().expect("the built-in data pack is valid");
    println!("Soaking the default world, seed {seed}, for {ticks} ticks.");
    let started = Instant::now();
    let run = Soak::run(data.clone(), seed, ticks);
    println!(
        "Finished unbroken in {} s: {} sprites alive at the end (at most {}), {} objects.",
        started.elapsed().as_secs(),
        run.sprites,
        run.most_sprites,
        run.objects
    );
    println!("\nCommand                  Given  Refused");
    for (name, given) in &run.given {
        let refused = run.refused.get(name).copied().unwrap_or(0);
        println!("{name:<24} {given:>5}  {refused:>7}");
    }
    println!("\nDeaths:");
    if run.deaths.is_empty() {
        println!("  none");
    }
    for (cause, n) in &run.deaths {
        let cause = match cause {
            DeathCause::Starvation => "starvation".to_string(),
            DeathCause::Dehydration => "dehydration".into(),
            DeathCause::OldAge => "old age".into(),
            DeathCause::HurtBy(id) => {
                format!("hurt by {}", data.object_type_name(*id).unwrap_or("?"))
            }
        };
        println!("  {cause}: {n}");
    }
    ExitCode::SUCCESS
}

/// The seed, and how many ticks: a million unless `--ticks` says.
fn parse(args: &[String]) -> Result<(u64, u64), String> {
    let (mut seed, mut ticks) = (None, 1_000_000);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => seed = Some(args::number("--seed", &mut args)?),
            "--ticks" => ticks = args::number("--ticks", &mut args)?,
            _ => return Err(format!("unexpected argument {arg}")),
        }
    }
    Ok((seed.ok_or("which seed? (--seed)")?, ticks))
}
