# Terra Sprites

An ASCII artificial-life game for the terminal, inspired by *Creatures*.

Sprites have a genome, a simulated biochemistry and a brain that learns from experience. They live on a grid world of plants, water, toys and thorns. You watch, inspect, and intervene with a "hand of god": reward a sprite with a pet, correct it with a shock, or move things around, and watch individual sprites learn.

**Status:** milestone 1 is being built, slice by slice ([milestone](https://github.com/Keazra/terra-sprites/milestone/1)). Today it generates a terrarium from a seed, with plants that grow, fruit, spread and expire, and a first population of sprites. The sprites start out on instinct: a hungry sprite walks to a berry bush and eats, a thirsty one finds water and drinks, a bored one kicks a ball (which rolls, bounces, knocks other balls on and crushes berries in its way), a lonely one goes over to others and plays with them, and a content one wanders or rests. A hurt sprite backs away from what hurt it; one that's hit usually backs away from its attacker, though some sprites hit back, and a cornered one fights. A crowded sprite backs away from the crowd, and now and then hits a neighbour. They find their way around each other and pass head-on in narrow passages. Play, hits, hurts and deaths show in the event log, and a red `!` flashes over a hurt sprite.

Sprites learn from what happens to them. Each one learns, by touch, what things are worth: berries are good when hungry, water when thirsty, and a thornbush it bit is bad, so it stops going near thornbushes. It learns habits, such as "eating balls doesn't work", and whether new things are worth investigating. It remembers particular sprites: one that played with it is good company, and one that hit it becomes frightening, so it backs away from that one without becoming shy of every sprite. What it learns about one thing doesn't spread to the rest of its kind until it knows a few of them: one thornbush doesn't make every bush bad. Lessons fade unless they're renewed, bad ones more slowly than good, and instinct is never erased, only outvoted. Each time a sprite learns something well enough, the event log says so ("Sprite #12 learned: thornbushes are bad").

You can teach them too. In Train mode a click pets the sprite under the Cursor and a right click zaps it (hold Ctrl with a click, or Shift with `Q` or `E`, for a hug or a shock). A pet rewards what the sprite just tried, so it learns to like that thing and to do it again, and a zap does the opposite. The Cursor can also lock on to a sprite so the Cursor moves with it, and in Grab mode pick up items, lead a sprite, throw items and shove sprites.

You can select a sprite and look inside it: its Brain tab shows what it's paying attention to and why it does what it does, and below that its memory, everything it has learned and how strongly. The map flashes an `X` where it's heading and shades what it's paying attention to in grey. You can scroll around and point at things, and the clock can be paused, stepped and sped up.

## Running it

You need [Rust](https://rustup.rs) (stable). Then, from the repository root:

```bash
cargo run --release -- --seed 7
```

| Key | Action |
|---|---|
| `W` `A` `S` `D` or arrows | Scroll the map (hold Shift to scroll 5 tiles) |
| Mouse | Point at a tile: the status line says what's there. Click a sprite to select it |
| `Z` / `X` / `C` | Cursor mode: Select / Train / Grab |
| `Q` / `E` | Left / right click where the Cursor is. In Select mode the right click locks on to the selected sprite; in Train mode the left pets and the right zaps (with Ctrl on a click or Shift on a key, a hug or a shock); in Grab mode the left picks up, leads, lets go or puts down, and holding the right aims a throw or a shove |
| `Tab` / `Shift+Tab` | Select the next / previous sprite |
| `[` / `]` | Previous / next inspector tab |
| `PgUp` / `PgDn`, or the mouse wheel over the inspector | Scroll a long tab |
| `v` | Detail view: exactly what the selected sprite is doing |
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

- [M1 "A Sprite Lives" design](docs/design/m1-a-sprite-lives-v26.md) (v26, the current revision). Earlier revisions and the external evaluations that shaped them are in [`docs/design/archive/`](docs/design/archive/).

## Roadmap

| Milestone | Contents |
|---|---|
| **M1 — A Sprite Lives** | World, ecology, biochemistry, learning brain, terminal UI, the hand, saves and replays |
| **M2 — Generations** | Reproduction, genetics, lineage, headless fast-forward |
| **M3 — Wild Terra** | Critters, more hazards and toys, possibly seasons and weather |
| **M4 — Words** | Teaching sprites words |
| **Tiles** | A tile-window front end with sprite-sheet support |

Built in Rust with [ratatui](https://ratatui.rs).
