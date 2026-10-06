//! The performance check (design §3.9, §7.4 A5): runs the default world with
//! 100 sprites in a release build and reports how many ticks a second it
//! manages. A5 asks for at least 200 on the development machine.
//!
//! `cargo run --release -p terra-sim --example speed`
//!
//! `--ticks N` measures N ticks (5,000 by default), `--seed N` makes the world
//! from seed N (7 by default). Close other busy programs first: anything else
//! the computer is doing slows it.

#[path = "common/args.rs"]
mod args;

use std::process::ExitCode;
use std::time::Instant;

use terra_sim::bench::crowded_world;

/// The target, in ticks a second.
const TARGET: f64 = 200.0;

fn main() -> ExitCode {
    let Some((ticks, seed)) = args::parse("speed [--ticks N] [--seed N]", parse) else {
        return ExitCode::FAILURE;
    };
    if cfg!(debug_assertions) {
        eprintln!("warning: this is a debug build; add --release for a fair measure");
    }
    println!("default world, 100 sprites, seed {seed}, settled for a few hundred ticks");
    let mut world = crowded_world(seed);
    let start = Instant::now();
    for _ in 0..ticks {
        world.step();
    }
    let seconds = start.elapsed().as_secs_f64();
    let rate = ticks as f64 / seconds;
    println!(
        "{ticks} ticks in {seconds:.2} s: {rate:.0} ticks a second, {:.2} ms a tick",
        1000.0 * seconds / ticks as f64
    );
    println!(
        "{} sprites alive at the end, state hash {:016x}",
        world.sprites().count(),
        world.state_hash()
    );
    let verdict = if rate >= TARGET { "met" } else { "not met" };
    println!("A5 (at least {TARGET:.0} ticks a second): {verdict}");
    ExitCode::SUCCESS
}

/// The ticks to measure and the seed, from the arguments.
fn parse(args: &[String]) -> Result<(u64, u64), String> {
    let (mut ticks, mut seed) = (5_000, 7);
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--ticks" => ticks = args::number("--ticks", &mut args)?,
            "--seed" => seed = args::number("--seed", &mut args)?,
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    if ticks == 0 {
        return Err("--ticks needs a number above 0".to_string());
    }
    Ok((ticks, seed))
}
