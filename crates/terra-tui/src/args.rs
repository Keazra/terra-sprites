//! Command-line flags (design §6.7).

use std::path::PathBuf;

/// What the player asked for on the command line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Args {
    /// `--seed <n>`: the world seed. Without it, the seed comes from the clock.
    pub seed: Option<u64>,
    /// `--preset <file>`: a world config to use instead of the built-in one.
    pub preset: Option<PathBuf>,
    /// `--ascii`: use the ascii theme.
    pub ascii: bool,
    /// `--theme <file>`: a theme to use instead of the built-in ones (design
    /// v34 §6.7).
    pub theme: Option<PathBuf>,
    /// Hidden developer flag: panic after the first frame, to check the terminal is restored.
    pub force_panic: bool,
}

/// How to run the game, shown with any flag error.
pub const USAGE: &str =
    "usage: terra-sprites [--seed <n>] [--preset <file>] [--ascii | --theme <file>]";

impl Args {
    /// Reads the flags, not including the program name.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Args, String> {
        let mut parsed = Args::default();
        let mut args = args.into_iter();
        while let Some(flag) = args.next() {
            match flag.as_str() {
                "--seed" => {
                    let value = value_of(&flag, args.next())?;
                    let seed = value.parse().map_err(|_| {
                        format!(
                            "`{value}` is not a seed: use a whole number from 0 to {}",
                            u64::MAX
                        )
                    })?;
                    parsed.seed = Some(seed);
                }
                "--preset" => parsed.preset = Some(value_of(&flag, args.next())?.into()),
                "--ascii" => parsed.ascii = true,
                "--theme" => parsed.theme = Some(value_of(&flag, args.next())?.into()),
                "--force-panic" => parsed.force_panic = true,
                _ => return Err(format!("unknown flag `{flag}`")),
            }
        }
        if parsed.ascii && parsed.theme.is_some() {
            return Err("`--ascii` and `--theme` both pick a theme: use one".into());
        }
        Ok(parsed)
    }
}

/// The value after `flag`, which must be present.
fn value_of(flag: &str, value: Option<String>) -> Result<String, String> {
    value.ok_or_else(|| format!("`{flag}` needs a value"))
}
