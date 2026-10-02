//! Sprites' names (design v26 §6.5). A sprite has none until the
//! player gives it one, typed or made at random from the data's syllables
//! (`names.ron`). The screen makes a random name up and sends it in a
//! `Rename`, so naming never draws from the world's RNG (design v26 change
//! 3): a name is made with an RNG of its own, seeded by the screen.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use serde::Deserialize;

use crate::cp437::is_cp437;
use crate::random::{chance, uniform};

/// The names file, relative to the pack root.
pub(crate) const NAMES: &str = "names.ron";

/// The most characters a name may have (design v26 §2.5).
pub const MAX_NAME_CHARS: usize = 16;

/// Why a name was refused (design v26 §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProblem {
    /// Nothing but spaces.
    Empty,
    /// More than `MAX_NAME_CHARS` characters.
    TooLong,
    /// A character CP437 can't show.
    NotCp437,
}

/// `name` as a sprite would carry it, trimmed of spaces at either end, or
/// why it can't be one.
pub(crate) fn checked(name: &str) -> Result<&str, NameProblem> {
    let name = name.trim_matches(' ');
    if name.is_empty() {
        Err(NameProblem::Empty)
    } else if name.chars().count() > MAX_NAME_CHARS {
        Err(NameProblem::TooLong)
    } else if !name.chars().all(is_cp437) {
        Err(NameProblem::NotCp437)
    } else {
        Ok(name)
    }
}

/// `names.ron`: the syllables random names are made of.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Syllables {
    first: Vec<String>,
    middle: Vec<String>,
    last: Vec<String>,
    middle_chance: f32,
}

impl Syllables {
    /// The syllables, or what's wrong with them: every list has some, each
    /// a run of letters, and the longest name they make is a valid one.
    pub(crate) fn validate(self) -> Result<Syllables, String> {
        let lists = [
            ("first", &self.first),
            ("middle", &self.middle),
            ("last", &self.last),
        ];
        for (which, list) in lists {
            if list.is_empty() {
                return Err(format!("the {which} syllables are empty"));
            }
            if let Some(bad) = list
                .iter()
                .find(|s| s.is_empty() || !s.chars().all(|c| c.is_alphabetic() && is_cp437(c)))
            {
                return Err(format!(
                    "the {which} syllable {bad:?} isn't a run of CP437 letters"
                ));
            }
        }
        if !(0.0..=1.0).contains(&self.middle_chance) {
            return Err(format!(
                "middle_chance is {}, outside 0 to 1",
                self.middle_chance
            ));
        }
        let longest = |list: &[String]| list.iter().map(|s| s.chars().count()).max().unwrap_or(0);
        let most = longest(&self.first) + longest(&self.middle) + longest(&self.last);
        if most > MAX_NAME_CHARS {
            return Err(format!(
                "the longest syllables make a name of {most} letters, over the {MAX_NAME_CHARS} a name may have"
            ));
        }
        Ok(self)
    }

    /// The random name `seed` makes: a first syllable, sometimes a middle
    /// one, and a last, starting with a capital. The same seed always makes
    /// the same name.
    pub(crate) fn name(&self, seed: u64) -> String {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut pick =
            |list: &[String]| list[uniform(&mut rng, list.len() as u64) as usize].clone();
        let first = pick(&self.first);
        let middle = pick(&self.middle);
        let last = pick(&self.last);
        let middle = if chance(&mut rng, self.middle_chance) {
            middle
        } else {
            String::new()
        };
        let joined = format!("{first}{middle}{last}").to_lowercase();
        let mut chars = joined.chars();
        let capital = chars.next().into_iter().flat_map(char::to_uppercase);
        capital.chain(chars).collect()
    }
}
