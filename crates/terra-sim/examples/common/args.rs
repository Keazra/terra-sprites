//! Shared argument parsing for the terra-sim examples.
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
