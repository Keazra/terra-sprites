//! Shared argument parsing for the terra-sim examples.
//!
//! Each example keeps its own flags and defaults. This module holds the
//! repeated pieces: reading a number after a flag, and the start of `main`
//! that collects args, prints usage on error, and returns failure.
//!
//! Examples include this module via `#[path]`, so each example is its own
//! crate and may not call every helper. Unused ones are not dead code of the
//! shared API.
#![allow(dead_code)]

use std::ops::RangeInclusive;
use std::process::ExitCode;
use std::slice::Iter;

/// Collects the command-line arguments, runs `parse`, and on success calls
/// `body`. On a parse error, prints the message and `usage`, and returns
/// failure — the pattern every example shared at the top of `main`.
pub fn run<T>(
    usage: &str,
    parse: impl FnOnce(&[String]) -> Result<T, String>,
    body: impl FnOnce(T) -> ExitCode,
) -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
        Ok(parsed) => body(parsed),
        Err(message) => {
            eprintln!("{message}\nusage: {usage}");
            ExitCode::FAILURE
        }
    }
}

/// The number after `--ticks`.
pub fn ticks(args: &mut Iter<'_, String>) -> Result<u64, String> {
    number("--ticks", args)
}

/// The number after `--seed`.
pub fn seed(args: &mut Iter<'_, String>) -> Result<u64, String> {
    number("--seed", args)
}

/// Lab-style `--seeds N`: seeds 1 through N (N must be above 0).
pub fn seeds(args: &mut Iter<'_, String>) -> Result<RangeInclusive<u64>, String> {
    let n = positive("--seeds", args)?;
    Ok(1..=n)
}

/// Rest-style `--seeds A-B`: the inclusive range from A to B.
pub fn seeds_range(args: &mut Iter<'_, String>) -> Result<(u64, u64), String> {
    let value = args.next().ok_or("--seeds needs a range, such as 1-10")?;
    let bad = || format!("--seeds needs a range, such as 1-10, not {value}");
    let (a, b) = value.split_once('-').ok_or_else(bad)?;
    let (a, b) = (a.parse().map_err(|_| bad())?, b.parse().map_err(|_| bad())?);
    if a > b {
        return Err(bad());
    }
    Ok((a, b))
}

/// The next argument as a whole number. Messages match speed and rest:
/// "`{flag} needs a number`" / "`{flag} needs a number, not {value}`".
pub fn number(flag: &str, args: &mut Iter<'_, String>) -> Result<u64, String> {
    let value = args.next().ok_or(format!("{flag} needs a number"))?;
    value
        .parse()
        .map_err(|_| format!("{flag} needs a number, not {value}"))
}

/// The next argument as a whole number above 0. Messages match lab and
/// baseline: "`{flag} needs a number`" / "`{flag} takes a whole number above
/// 0, not {value}`".
pub fn positive(flag: &str, args: &mut Iter<'_, String>) -> Result<u64, String> {
    let value = args.next().ok_or(format!("{flag} needs a number"))?;
    value
        .parse()
        .ok()
        .filter(|&n| n > 0)
        .ok_or(format!("{flag} takes a whole number above 0, not {value}"))
}
