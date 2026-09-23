# Terra Sprites

An ASCII artificial-life game for the terminal, inspired by *Creatures*.

Sprites have a genome, a simulated biochemistry and a brain that learns from experience. They live on a grid world of plants, water, toys and thorns. You watch, inspect, and intervene with a "hand of god": tickle a sprite to reward it, slap it to punish it, or move things around, and watch individual sprites learn.

**Status:** milestone 1 is designed and being planned. There is no playable build yet.

## Design

- [M1 "A Sprite Lives" design](docs/design/m1-a-sprite-lives-v3.md) (final, v3). Earlier iterations and the external evaluations that shaped them are alongside it in [`docs/design/`](docs/design/).

## Roadmap

| Milestone | Contents |
|---|---|
| **M1 — A Sprite Lives** | World, ecology, biochemistry, learning brain, terminal UI, the hand, saves and replays |
| **M2 — Generations** | Reproduction, genetics, lineage, headless fast-forward |
| **M3 — Wild Terra** | Critters, more hazards and toys, possibly seasons and weather |
| **M4 — Words** | Teaching sprites words |
| **Tiles** | A tile-window front end with sprite-sheet support |

Built in Rust with [ratatui](https://ratatui.rs).
