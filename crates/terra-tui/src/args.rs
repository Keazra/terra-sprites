//! Command-line flags (design §6.7).

use std::path::PathBuf;

use terra_sim::Mode;

use crate::fast_forward::MOST_HOURS;
use crate::text::thousands;

/// What the player asked for on the command line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Args {
    /// `--seed <n>`: the world seed. Without it, the seed comes from the clock.
    pub seed: Option<u64>,
    /// `--preset <file>`: a world config to use instead of the built-in one.
    pub preset: Option<PathBuf>,
    /// `--data <dir>`: a folder of data pack files, each replacing the
    /// built-in one, for a new world (design §2.8, §6.7).
    pub data: Option<PathBuf>,
    /// `--replay <file>`: play a replay back instead of a new world (design
    /// §2.7).
    pub replay: Option<PathBuf>,
    /// The new world's mode: Story, or Lab for `--lab` (M2 design §16).
    pub mode: Mode,
    /// `--load <save>`: open a saved world, by its name in the saves folder
    /// or its file, as Load does on the title screen (M2 design §16).
    pub load: Option<String>,
    /// `--fast-forward <hours>`: run a Lab world for that many hours of
    /// play at 1× with no screen, then save it (M2 design §16).
    pub fast_forward: Option<u64>,
    /// `--save <name>`: the name a fast-forward saves its world under.
    pub save: Option<String>,
    /// `--ascii`: use the ascii theme.
    pub ascii: bool,
    /// `--theme <file>`: a theme to use instead of the built-in ones (design
    /// v34 §6.7).
    pub theme: Option<PathBuf>,
    /// `--version`: print the version and the commit the game was built
    /// from, and stop (design §6.7).
    pub version: bool,
    /// Hidden developer flag: panic after the first frame, to check the terminal is restored.
    pub force_panic: bool,
}

/// The version and the commit the game was built from, as "0.1.0 (cc96347)"
/// (design §6.7). The commit is "unknown" when it wasn't built from git.
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("TERRA_COMMIT"), ")");

/// How to run the game, shown with any flag error.
pub const USAGE: &str = "usage: terra-sprites [--seed <n>] [--preset <file>] [--data <dir>] [--lab] [--ascii | --theme <file>]\n\
                         \x20      terra-sprites --load <save> [--ascii | --theme <file>]\n\
                         \x20      terra-sprites --replay <file> [--ascii | --theme <file>]\n\
                         \x20      terra-sprites --fast-forward <hours> [--seed <n>] [--preset <file>] [--data <dir> | --load <save>] [--save <name>]\n\
                         \x20      terra-sprites --version";

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
                "--data" => parsed.data = Some(value_of(&flag, args.next())?.into()),
                "--replay" => parsed.replay = Some(value_of(&flag, args.next())?.into()),
                "--lab" => parsed.mode = Mode::Lab,
                "--load" => parsed.load = Some(value_of(&flag, args.next())?),
                "--fast-forward" => {
                    let value = value_of(&flag, args.next())?;
                    // A run of more hours couldn't count its ticks (M2
                    // design §16.3).
                    let hours = value
                        .parse()
                        .ok()
                        .filter(|hours: &u64| (1..=MOST_HOURS).contains(hours));
                    parsed.fast_forward = Some(hours.ok_or_else(|| {
                        format!(
                            "`{value}` is not a number of hours to fast-forward: use a whole number from 1 to {}",
                            thousands(MOST_HOURS)
                        )
                    })?);
                }
                "--save" => parsed.save = Some(value_of(&flag, args.next())?),
                "--ascii" => parsed.ascii = true,
                "--theme" => parsed.theme = Some(value_of(&flag, args.next())?.into()),
                "--version" => parsed.version = true,
                "--force-panic" => parsed.force_panic = true,
                _ => return Err(format!("unknown flag `{flag}`")),
            }
        }
        if parsed.ascii && parsed.theme.is_some() {
            return Err("`--ascii` and `--theme` both pick a theme: use one".into());
        }
        let new_world = [
            ("--seed", parsed.seed.is_some()),
            ("--preset", parsed.preset.is_some()),
            ("--data", parsed.data.is_some()),
            ("--lab", parsed.mode == Mode::Lab),
        ];
        let makes_one = new_world
            .iter()
            .find(|(_, given)| *given)
            .map(|(flag, _)| flag);
        // A replay brings its own world, so what makes or opens another
        // doesn't apply (design §2.7).
        if parsed.replay.is_some() {
            if let Some(flag) = makes_one {
                return Err(format!(
                    "`{flag}` makes a new world, and `--replay` plays a recorded one: use one or the other"
                ));
            }
            for (flag, given) in [
                ("--load", parsed.load.is_some()),
                ("--fast-forward", parsed.fast_forward.is_some()),
            ] {
                if given {
                    return Err(format!(
                        "`--replay` plays a recorded world, and `{flag}` takes another: use one or the other"
                    ));
                }
            }
        }
        // A save brings its own world, mode and all (M2 design §16).
        if parsed.load.is_some()
            && let Some(flag) = makes_one
        {
            return Err(format!(
                "`{flag}` makes a new world, and `--load` opens a saved one: use one or the other"
            ));
        }
        if parsed.save.is_some() && parsed.fast_forward.is_none() {
            return Err(
                "`--save` names the save a fast-forward writes: use it with `--fast-forward`"
                    .into(),
            );
        }
        Ok(parsed)
    }
}

/// The value after `flag`, which must be present.
fn value_of(flag: &str, value: Option<String>) -> Result<String, String> {
    value.ok_or_else(|| format!("`{flag}` needs a value"))
}
