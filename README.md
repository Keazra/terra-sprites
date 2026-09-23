# Terra Sprites

An ASCII artificial-life game for the terminal, inspired by *Creatures*.

Sprites have a genome, a simulated biochemistry and a brain that learns from experience. They live on a grid world of plants, water, toys and thorns. You watch, inspect, and intervene with a "hand of god": tickle a sprite to reward it, slap it to punish it, or move things around, and watch individual sprites learn.

**Status:** milestone 1 is being built, slice by slice ([milestone](https://github.com/Keazra/terra-sprites/milestone/1)). Today the game shows an empty world whose clock you can pause, step and speed up.

## Running it

You need [Rust](https://rustup.rs) (stable). Then, from the repository root:

```bash
cargo run --release
```

| Key | Action |
|---|---|
| `space` | Pause / resume |
| `.` | Step one tick while paused (hold to keep stepping) |
| `+` / `-` | Faster / slower: each step doubles or halves the speed, from ⅛× up to 16×, then Max. The game starts at 1× (10 ticks per second). Holding either key stops at 1×; press again to go past it. |
| `q` or `Ctrl+C` | Quit |

To run the checks CI runs: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.

## Design

- [M1 "A Sprite Lives" design](docs/design/m1-a-sprite-lives-v4.md) (final, v4). Earlier iterations and the external evaluations that shaped them are alongside it in [`docs/design/`](docs/design/).

## Roadmap

| Milestone | Contents |
|---|---|
| **M1 — A Sprite Lives** | World, ecology, biochemistry, learning brain, terminal UI, the hand, saves and replays |
| **M2 — Generations** | Reproduction, genetics, lineage, headless fast-forward |
| **M3 — Wild Terra** | Critters, more hazards and toys, possibly seasons and weather |
| **M4 — Words** | Teaching sprites words |
| **Tiles** | A tile-window front end with sprite-sheet support |

Built in Rust with [ratatui](https://ratatui.rs).
