# Terra Sprites

An ASCII artificial-life game for the terminal, inspired by *Creatures*.

Sprites have a genome, a simulated biochemistry and a brain that learns from experience. They live on a grid world of plants, water, toys and thorns. You watch, inspect, and intervene with a "hand of god": reward a sprite with a pet, correct it with a shock, or move things around, and watch individual sprites learn.

**Status:** milestone 1 is being built, slice by slice ([milestone](https://github.com/Keazra/terra-sprites/milestone/1)). Today it generates a terrarium from a seed, with plants that grow, fruit, spread and expire, and a first population of sprites. The sprites wander about and rest, find their way around each other, and pass head-on in narrow passages; until they have brains, a coin toss decides which. They can't eat or drink yet, so they grow hungry and thirsty and die, and each death shows in the event log. You can select a sprite and look inside it, scroll around and point at things, and the clock can be paused, stepped and sped up.

## Running it

You need [Rust](https://rustup.rs) (stable). Then, from the repository root:

```bash
cargo run --release -- --seed 7
```

| Key | Action |
|---|---|
| `W` `A` `S` `D` or arrows | Scroll the map (hold Shift to scroll 5 tiles) |
| Mouse | Point at a tile: the status line says what's there. Click a sprite to select it |
| `Tab` / `Shift+Tab` | Select the next / previous sprite |
| `[` / `]` | Previous / next inspector tab |
| `PgUp` / `PgDn`, or the mouse wheel over the inspector | Scroll a long tab |
| `v` | Detail view: exactly what the selected sprite is doing, and an `X` where it's heading |
| `space` | Pause / resume |
| `.` | Step one tick while paused (hold to keep stepping) |
| `+` / `-` | Faster / slower: each step doubles or halves the speed, from ⅛× up to 16×, then Max. The game starts at 1× (1.25 ticks per second, slow enough to watch sprites walk). Holding either key stops at 1×; press again to go past it. |
| `Esc` | Quit, after "Quit? (y/n)": press `y` or `Esc` again |
| `Ctrl+C` | Quit at once |

| Flag | Effect |
|---|---|
| `--seed <n>` | Make the world from seed `n` (the top bar shows the seed of every world) |
| `--preset <file>` | Use a world config from a RON file, such as a copy of [`data/presets/default.ron`](data/presets/default.ron) |
| `--ascii` | Draw the map in plain ASCII instead of CP437 |

To run the checks CI runs: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.

## Design

- [M1 "A Sprite Lives" design](docs/design/m1-a-sprite-lives-v9.md) (final, v9). Earlier iterations and the external evaluations that shaped them are alongside it in [`docs/design/`](docs/design/).

## Roadmap

| Milestone | Contents |
|---|---|
| **M1 — A Sprite Lives** | World, ecology, biochemistry, learning brain, terminal UI, the hand, saves and replays |
| **M2 — Generations** | Reproduction, genetics, lineage, headless fast-forward |
| **M3 — Wild Terra** | Critters, more hazards and toys, possibly seasons and weather |
| **M4 — Words** | Teaching sprites words |
| **Tiles** | A tile-window front end with sprite-sheet support |

Built in Rust with [ratatui](https://ratatui.rs).
