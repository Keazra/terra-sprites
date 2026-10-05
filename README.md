# Terra Sprites

An ASCII artificial-life game for the terminal, inspired by *Creatures*.

Sprites have a genome, a simulated biochemistry and a brain that learns from experience. They live on a grid world of plants, water, toys and thorns. You watch, inspect, and intervene with a "hand of god": reward a sprite with a pet, correct it with a shock, or move things around, and watch individual sprites learn.

**Status:** milestone 1 is built, slice by slice ([milestone](https://github.com/Keazra/terra-sprites/milestone/1)), and is in its final play-through before sign-off ([M1 acceptance](docs/acceptance/m1.md)). Today it generates a terrarium from a seed, with plants that grow, fruit, spread and expire, and a first population of sprites. The sprites start out on instinct: a hungry sprite walks to a berry bush and eats, a thirsty one finds water and drinks, a bored one kicks a ball (which rolls, bounces, knocks other balls on and crushes berries in its way), a lonely one goes over to others and plays with them, and a content one wanders or rests. A hurt sprite backs away from what hurt it; one that's hit usually backs away from its attacker, though some sprites hit back, and a cornered one fights. A crowded sprite backs away from the crowd, and now and then hits a neighbour. They find their way around each other and pass head-on in narrow passages. Play, hits, hurts and deaths show in the event log, and a red `!` flashes over a hurt sprite.

Sprites learn from what happens to them. Each one learns, by touch, what things are worth: berries are good when hungry, water when thirsty, and a thornbush it bit is bad, so it stops going near thornbushes. It learns habits, such as "eating balls doesn't work", and whether new things are worth investigating. It remembers particular sprites: one that played with it is good company, and one that hit it becomes frightening, so it backs away from that one without becoming shy of every sprite. What it learns about one thing doesn't spread to the rest of its kind until it knows a few of them: one thornbush doesn't make every bush bad. Lessons fade unless they're renewed, bad ones more slowly than good, and instinct is never erased, only outvoted. Each time a sprite learns something well enough, the event log says so ("Sprite #12 learned: thornbushes are bad").

You can teach them too. In Train mode a click pets the sprite under the Cursor and a right click zaps it (hold Ctrl with a click, or Shift with `Q` or `E`, for a hug or a shock). A pet rewards what the sprite just tried, so it learns to like that thing and to do it again, and a zap does the opposite. The Cursor can also follow a sprite so it moves with it, and in Grab mode pick up items, lead a sprite, throw items and shove sprites, or place new berries, balls, bushes and sprites. You can name a sprite, and save its genome to place a copy later.

You can select a sprite and look inside it: its Brain tab shows what it's paying attention to and why it does what it does, and below that its memory, everything it has learned and how strongly. The map flashes an `X` where it's heading and shades what it's paying attention to in grey. You can scroll around and point at things, and the clock can be paused, stepped and sped up. A world can be saved and loaded, and saves itself every 10 minutes; and the map can be drawn with a theme of your own.

## Getting started

You need [Rust](https://rustup.rs) (stable) and a terminal of at least 100×30 characters; 140×40 is better. On Windows, use Windows Terminal. Then, from the repository root:

```bash
cargo run --release -- --seed 7
```

The first build takes a minute or two. The game opens on a new terrarium, made from seed 7, with time running at 1×. Some things to try first:

1. **Watch one sprite.** Click a sprite (`☺`) to select it. The panel beside the map shows its body; `]` moves to its Brain tab, which says what it's paying attention to and why. `T` makes the view follow it.
2. **Control time.** `space` pauses, `.` steps a little while paused, and `+` speeds up. A sprite's life is long at 1×, so try 8× or 16×.
3. **Teach it something.** Press `X` for Train mode. When the sprite does something you like, click it to pet it; right-click to zap it for something you don't. It learns from both.
4. **Move things.** Press `C` for Grab mode. Click a berry (`•`) to pick it up, and click again beside a hungry sprite to put it down. Press `C` once more for the Place menu, to add berries, balls, bushes and new sprites.
5. **Keep the world.** `F5` saves and `F9` loads it back; the game also saves itself every 10 minutes. `?` shows every key, the colour legend, and where your saves are.

The status line at the bottom always says what's under the Cursor, and the event log under the map tells you what the sprites are doing.

## Controls

The same key in capitals or lower case does the same thing.

| Key | Action |
|---|---|
| `space` | Pause / resume |
| `.` | Step while paused: a second's worth of ticks at the current speed, one at 1× and slower, up to 20 at 16× and Max (hold to keep stepping) |
| `+` / `-` | Faster / slower: each step doubles or halves the speed, from ⅛× up to 16×, then Max. The game starts at 1× (1.25 ticks per second, slow enough to watch sprites walk). Holding `+` stops at 1× and at 16×, and holding `-` at 1×; press again to go past |
| `W` `A` `S` `D` or arrows | Scroll the map (hold Shift to scroll 5 tiles) |
| Mouse | Point at a tile: the status line says what's there. Click a sprite to select it |
| `T` | Track: the view follows the selected sprite; again to stop |
| `b` | Sprite colours: what a sprite's colour shows, such as its strongest drive or nothing. The status line says which |
| `m` | Event log filter: all events, the selected sprite's, or only the major ones (deaths, lessons learned, refusals) |
| `v` | Detail view: exactly what the selected sprite is doing |
| `?` | Help: every key, the colour legend and where the game keeps its files |
| `Z` / `X` / `C` | Cursor mode: Select / Train / Grab. The mouse wheel over the map changes mode too. `C` again in Grab mode opens the Place menu |
| `Q` / `E` | Left / right click where the Cursor is. In Train mode the left pets and the right zaps (with Ctrl on a click or Shift on a key, a hug or a shock); in Grab mode the left picks up, leads, lets go or puts down, and holding the right aims a throw or a shove, which goes when you let go |
| `F` or the middle button | Follow the sprite under the Cursor (or else the selected one) so the Cursor moves with it; again to stop |
| `H` | Make the Cursor visible to sprites in the current cursor mode, or hide it again. Each mode starts hidden; while visible the Cursor is drawn as a frame of light (`═` and `║`) and the status line says "seen"; sprites can go to it or back away from it, and they learn to like or fear it from how you treat them |
| Place menu | `1`–`9`, or the arrows and `Enter`, or a click picks: a berry bush seedling, a berry, a ball, a new sprite, or a sprite from a genome file. The next Grab-mode click places it; a right click puts it away |
| `Tab` / `Shift+Tab` | Select the next / previous sprite |
| `l` | Sprite list: every sprite, with its age, strongest drive and what it's doing. The arrows choose, `Tab` changes the order, `Enter` goes to it |
| `r` | Name the selected sprite: type a name, or `Tab` for another random one, then `Enter` |
| `g` | Save the selected sprite's genome to the genomes folder (the status line says where); the Place menu reads it back |
| `[` / `]` | Previous / next inspector tab |
| `PgUp` / `PgDn`, or the mouse wheel over the inspector | Scroll a long tab |
| `F5` / `F9` | Quicksave / quickload. Loading over a world that has run since it was last saved asks first (`y` to load), and a loaded world starts paused |
| `Ctrl+S` / `Ctrl+O` | Save under a name you type / pick a save to load, newest first. The world also saves itself every 10 minutes of running time and when you quit, keeping the last 3 autosaves. See [saves](docs/reference/saves.md) |
| `Ctrl+T` | Pick a theme: how the map is drawn. It lists the two built-in themes, then your own from the `themes` folder of the game's folder; see [themes](docs/reference/themes.md) |
| `Ctrl+R` | In a replay, take it over: the world is yours from the tick it has reached, and the rest of the recording is dropped. Its next autosave writes a new `last_session.replay`, so copy a replay you want to keep first |
| `Esc` | Each press does the first that applies: close a menu or overlay, cancel an aim, let go of what the Cursor holds or leads (a held item is put down where the Cursor is), go back to Select, and from Select ask "Quit? (y/n)". Only `y` quits; `Esc` again keeps playing |
| `Ctrl+C` | Quit at once |

| Flag | Effect |
|---|---|
| `--seed <n>` | Make the world from seed `n` (the top bar shows the seed of every world) |
| `--preset <file>` | Use a world config from a RON file, such as a copy of [`data/presets/default.ron`](data/presets/default.ron) |
| `--data <dir>` | Use the data pack files in a folder, such as a changed copy of [`data/objects.ron`](data/objects.ron), in place of the built-in ones; any file the folder lacks stays built in. A `presets/default.ron` in the folder is the default preset |
| `--replay <file>` | Play a replay back, such as `last_session.replay`, which every session writes to the game's folder (the help screen shows where). Time, the view and the inspector work, but nothing can change the world until you take it over with `Ctrl+R`. It pauses at the end, and if the replay ever drifts from what was recorded, it pauses and says between which ticks. See [replays](docs/reference/saves.md#replays) |
| `--ascii` | Draw the map in plain ASCII instead of CP437 |
| `--theme <file>` | Start with a theme of your own, such as an edited copy of [`themes/cp437.ron`](themes/cp437.ron), rather than picking it with `Ctrl+T` each time |

To run the checks CI runs: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`.

## Same seed, same world

A world made from the same seed and world config, given the same clicks and keys, plays out exactly the same every time, in the same version of the game. That holds across computers too, for these targets:

| Computer | Target |
|---|---|
| Linux PC | `x86_64-unknown-linux-gnu` |
| Windows PC | `x86_64-pc-windows-msvc` |
| Mac with Apple silicon | `aarch64-apple-darwin` |
| Linux on ARM | `aarch64-unknown-linux-gnu` |

CI checks this on every pull request: it runs the default world from seed 7 for 20,000 ticks on each of them with nobody clicking, then for 10,000 ticks with a random script of the Cursor's commands, and fails if any of them ends up in a different state. Other 64-bit computers are expected to match too, but aren't checked. To run the check yourself, `cargo run --release -p terra-sim --example determinism` prints the hash of the world's state every 1,000 ticks; CI's run summary shows what each computer printed.

## Changing the game

The ground, the plants and toys, a sprite's body and its instincts, the size of the world and how the map is drawn are files anyone can edit. The [data-format reference](docs/reference/README.md) explains each one, with a working example.

## Design

- [M1 "A Sprite Lives" design](docs/design/m1-a-sprite-lives-v38.md) (v38, the current revision). Earlier revisions and the external evaluations that shaped them are in [`docs/design/archive/`](docs/design/archive/).

## Roadmap

| Milestone | Contents |
|---|---|
| **M1 — A Sprite Lives** | World, ecology, biochemistry, learning brain, terminal UI, the hand, saves and replays |
| **M2 — Generations** | Reproduction, genetics, lineage, headless fast-forward |
| **M3 — Wild Terra** | Critters, more hazards and toys, possibly seasons and weather |
| **M4 — Words** | Teaching sprites words |
| **Tiles** | A tile-window front end with sprite-sheet support |

Built in Rust with [ratatui](https://ratatui.rs).
