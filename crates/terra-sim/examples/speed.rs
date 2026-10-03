//! The performance check (design §3.9, §7.4 A5): runs the default world with
//! 100 sprites in a release build and reports how many ticks a second it
//! manages. A5 asks for at least 200 on the development machine.
//!
//! `cargo run --release -p terra-sim --example speed`
//!
//! `--ticks N` measures N ticks (5,000 by default), `--seed N` makes the world
//! from seed N (7 by default). Close other busy programs first: anything else
//! the computer is doing slows it.

use std::process::ExitCode;
use std::time::Instant;

use terra_sim::{DataPack, World, WorldConfig};

/// The target, in ticks a second.
const TARGET: f64 = 200.0;

/// Ticks run before measuring, so sprites are spread out and busy, as in
/// play, rather than standing where they were placed.
const SETTLE: u64 = 300;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (ticks, seed) = match parse(&args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}\nusage: speed [--ticks N] [--seed N]");
            return ExitCode::FAILURE;
        }
    };
    if cfg!(debug_assertions) {
        eprintln!("warning: this is a debug build; add --release for a fair measure");
    }
    let data = DataPack::builtin().expect("the built-in data pack is valid");
    let default = include_str!("../../../data/presets/default.ron");
    let preset = default.replace("sprites: 30,", "sprites: 100,");
    assert_ne!(preset, default, "the default preset names its sprite count");
    let config = WorldConfig::from_ron(&preset, &data).expect("a valid preset");
    let mut world = World::new(config, data, seed);
    println!("default world, 100 sprites, seed {seed}");
    while world.tick() < SETTLE {
        world.step();
    }
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
        let target = match flag.as_str() {
            "--ticks" => &mut ticks,
            "--seed" => &mut seed,
            _ => return Err(format!("unknown argument {flag}")),
        };
        let value = args.next().ok_or(format!("{flag} needs a number"))?;
        *target = value
            .parse()
            .map_err(|_| format!("{flag} needs a number, not {value}"))?;
    }
    if ticks == 0 {
        return Err("--ticks needs a number above 0".to_string());
    }
    Ok((ticks, seed))
}
