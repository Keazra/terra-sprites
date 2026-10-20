//! Shared argument parsing for the terra-sim examples, and their warning
//! in a debug build.
//!
//! Examples include this module via `#[path]`, so each example is its own
//! crate and may not call every helper. Unused ones are not dead code of the
//! shared API.
#![allow(dead_code)]

use std::slice::Iter;

/// Collects the command-line arguments and runs `parse`. On a parse error,
/// prints the message and `usage`, and returns `None`.
pub fn parse<T>(usage: &str, parse: impl FnOnce(&[String]) -> Result<T, String>) -> Option<T> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
        Ok(parsed) => Some(parsed),
        Err(message) => {
            eprintln!("{message}\nusage: {usage}");
            None
        }
    }
}

/// The next argument as a whole number. Its messages: "`{flag} needs a
/// number`" / "`{flag} needs a number, not {value}`".
pub fn number(flag: &str, args: &mut Iter<'_, String>) -> Result<u64, String> {
    let value = args.next().ok_or(format!("{flag} needs a number"))?;
    value
        .parse()
        .map_err(|_| format!("{flag} needs a number, not {value}"))
}

/// The next argument as a whole number above 0. Its messages: "`{flag}
/// needs a number`" / "`{flag} takes a whole number above 0, not {value}`".
pub fn positive(flag: &str, args: &mut Iter<'_, String>) -> Result<u64, String> {
    let value = args.next().ok_or(format!("{flag} needs a number"))?;
    value
        .parse()
        .ok()
        .filter(|&n| n > 0)
        .ok_or(format!("{flag} takes a whole number above 0, not {value}"))
}

/// The next argument as a range of seeds, `A-B` with A no more than B.
pub fn seeds(flag: &str, args: &mut Iter<'_, String>) -> Result<(u64, u64), String> {
    let value = args
        .next()
        .ok_or(format!("{flag} needs a range, such as 1-10"))?;
    range(value).ok_or(format!("{flag} needs a range, such as 1-10, not {value}"))
}

/// The next argument as seeds: a list, `5,9`, or a range, `1-10`, with A
/// no more than B.
pub fn seed_list(flag: &str, args: &mut Iter<'_, String>) -> Result<Vec<u64>, String> {
    let value = args
        .next()
        .ok_or(format!("{flag} needs seeds, such as 5,9 or 1-10"))?;
    let bad = || format!("{flag} needs seeds, such as 5,9 or 1-10, not {value}");
    if value.contains('-') {
        let (a, b) = range(value).ok_or_else(bad)?;
        return Ok((a..=b).collect());
    }
    value
        .split(',')
        .map(|seed| seed.parse().map_err(|_| bad()))
        .collect()
}

/// `A-B` as `(A, B)`, if A is no more than B.
fn range(value: &str) -> Option<(u64, u64)> {
    let (a, b) = value.split_once('-')?;
    let (a, b) = (a.parse().ok()?, b.parse().ok()?);
    (a <= b).then_some((a, b))
}

/// Warns, in a debug build, that the example wants `--release`: "add
/// --release" and then `why`, such as ", or it will take a long time".
pub fn warn_if_debug(why: &str) {
    if cfg!(debug_assertions) {
        eprintln!("warning: this is a debug build; add --release{why}");
    }
}
