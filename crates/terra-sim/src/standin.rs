//! The stand-in chooser (design §5.5): until the brain arrives in slice 6,
//! it decides what a sprite with no action does next. It's called in one
//! place, where the brain's decision will take its place.

use rand_chacha::ChaCha8Rng;

use crate::random::uniform;
use crate::registry::Verb;

/// Wander or Rest, at even odds, from the world's RNG.
pub(crate) fn choose(rng: &mut ChaCha8Rng) -> Verb {
    if uniform(rng, 2) == 0 {
        Verb::Wander
    } else {
        Verb::Rest
    }
}

#[cfg(test)]
mod tests {
    use rand_chacha::rand_core::SeedableRng;

    use super::*;

    #[test]
    fn the_stand_in_wanders_or_rests_at_even_odds() {
        let mut rng = ChaCha8Rng::seed_from_u64(3);
        let choices: Vec<Verb> = (0..10_000).map(|_| choose(&mut rng)).collect();
        let wanders = choices.iter().filter(|&&v| v == Verb::Wander).count();
        let rests = choices.iter().filter(|&&v| v == Verb::Rest).count();
        assert_eq!(wanders + rests, 10_000, "only Wander or Rest");
        assert!((4_800..=5_200).contains(&wanders), "{wanders} wanders");
    }
}
