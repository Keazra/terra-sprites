# Terra Sprites

An ASCII artificial-life game for the terminal, inspired by *Creatures*.

Sprites have a genome, a simulated biochemistry and a brain that learns from experience. They live on a grid world of plants, water, toys and thorns. You watch, inspect, and intervene with a "hand of god": reward a sprite with a pet, correct it with a shock, or move things around, and watch individual sprites learn.

**Status:** milestone 1 is being built, slice by slice ([milestone](https://github.com/Keazra/terra-sprites/milestone/1)). Today it generates a terrarium of terrain from a seed, which you can scroll around and point at, and its clock can be paused, stepped and sped up.

## Running it

You need [Rust](https://rustup.rs) (stable). Then, from the repository root:

```bash
cargo run --release -- --seed 7
```

| Key | Action |
|---|---|
| `W` `A` `S` `D` or arrows | Scroll the map (hold Shift to scroll 5 tiles) |
| Mouse wheel | Scroll the map 3 tiles a notch (sideways too, on a wheel that tilts) |
| Mouse | Point at a tile: the status line says what's there |
| `space` | Pause / resume |
| `.` | Step one tick while paused (hold to keep stepping) |
| `+` / `-` | Faster / slower: each step doubles or halves the speed, from ⅛× up to 16×, then Max. The game starts at 1× (10 ticks per second). Holding either key stops at 1×; press again to go past it. |
| `Esc` | Quit, after "Quit? (y/n)": press `y` or `Esc` again |
| `Ctrl+C` | Quit at once |

| Flag | Effect |
|---|---|
| `--seed <n>` | Make the world from seed `n` (the top bar shows the seed of every world) |
| `--preset <file>` | Use a world config from a RON file, such as a copy of [`data/presets/default.ron`](data/presets/default.ron) |
| `--ascii` | Draw the map in plain ASCII instead of CP437 |

To run the checks CI runs: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.

## Design

- [M1 "A Sprite Lives" design](docs/design/m1-a-sprite-lives-v5.md) (final, v5). Earlier iterations and the external evaluations that shaped them are alongside it in [`docs/design/`](docs/design/).

## Roadmap

| Milestone | Contents |
|---|---|
| **M1 — A Sprite Lives** | World, ecology, biochemistry, learning brain, terminal UI, the hand, saves and replays |
| **M2 — Generations** | Reproduction, genetics, lineage, headless fast-forward |
| **M3 — Wild Terra** | Critters, more hazards and toys, possibly seasons and weather |
| **M4 — Words** | Teaching sprites words |
| **Tiles** | A tile-window front end with sprite-sheet support |

Built in Rust with [ratatui](https://ratatui.rs).
