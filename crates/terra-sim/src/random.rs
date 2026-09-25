//! Draws from the world's RNG (design §2.3, §3.5.2). Each takes exactly one draw.

use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::Rng;

/// One of `n` choices, uniformly: one 64-bit draw scaled to `n`. The bias is
/// below one in 2⁵⁰ for any `n` the sim uses, and unlike rejection sampling it
/// always takes exactly one draw.
pub(crate) fn uniform(rng: &mut ChaCha8Rng, n: u64) -> u64 {
    debug_assert!(n > 0, "choosing among no choices");
    ((u128::from(rng.next_u64()) * u128::from(n)) >> 64) as u64
}

/// Whether an event of probability `p` happens: one 32-bit draw, whose top 24
/// bits as a fraction of 2²⁴ are compared with `p`. Exact in `f32`.
pub(crate) fn chance(rng: &mut ChaCha8Rng, p: f32) -> bool {
    unit(rng) < p
}

/// A random value in [0, 1) from the top 24 bits of a draw, so it is exact in `f32`.
pub(crate) fn unit(rng: &mut ChaCha8Rng) -> f32 {
    (rng.next_u32() >> 8) as f32 / (1u32 << 24) as f32
}
