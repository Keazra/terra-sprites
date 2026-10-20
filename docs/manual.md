# Terra Sprites manual

How to play Terra Sprites and how to work on it: everything past what the [README](../README.md) covers.

- [Where it's at](#where-its-at)
- [What sprites do](#what-sprites-do)
- [Getting started](#getting-started)
- [Controls](#controls)
- [Fast-forward](#fast-forward)
- [Same seed, same world](#same-seed-same-world)
- [Changing the game](#changing-the-game)
- [Clips: GIFs, videos and SVGs](#clips-gifs-videos-and-svgs)
- [Working on the game](#working-on-the-game)
- [Roadmap](#roadmap)

## Where it's at

Milestone 1 is done: every slice is built ([milestone](https://github.com/Keazra/terra-sprites/milestone/1)), and the owner signed it off after playing it through ([M1 acceptance](acceptance/m1.md)). Milestone 2, *Generations*, is nearly done ([milestone](https://github.com/Keazra/terra-sprites/milestone/2)): sprites now hatch from eggs, grow up, court, breed and grow old, and the young are born with a mix of their parents' genes. Two slices are left: a harsher world, where most young die unless another sprite or the player guides them, and the sign-off. Milestone 3 is being planned.

## What sprites do

Sprites have a genome, a simulated biochemistry and a brain that learns from experience. They live on a grid world of plants, water, toys and thorns. You watch, inspect, and intervene with a "hand of god": reward a sprite with a pet, correct it with a shock, or move things around, and watch individual sprites learn.

The game generates a terrarium from a seed, with plants that grow, fruit, spread and expire, and a first population of sprites. The sprites start out on instinct: a hungry sprite walks to a berry bush and eats, a thirsty one finds water and drinks, a bored one kicks a ball (which rolls, bounces, knocks other balls on and crushes berries in its way), a lonely one goes over to others and plays with them, and a content one wanders or rests. A hurt sprite backs away from what hurt it; one that's hit usually backs away from its attacker, though some sprites hit back, and a cornered one fights. A crowded sprite backs away from the crowd, and now and then hits a neighbour. They find their way around each other and pass head-on in narrow passages. Play, hits, hurts and deaths show in the event log, and a red `!` flashes over a hurt sprite.

Sprites learn from what happens to them. Each one learns, by touch, what things are worth: berries are good when hungry, water when thirsty, and a thornbush it bit is bad, so it stops going near thornbushes. It learns habits, such as "eating balls doesn't work", and whether new things are worth investigating. It remembers particular sprites: one that played with it is good company, and one that hit it becomes frightening, so it backs away from that one without becoming shy of every sprite. What it learns about one thing doesn't spread to the rest of its kind until it knows a few of them: one thornbush doesn't make every bush bad. Lessons fade unless they're renewed, bad ones more slowly than good, and instinct is never erased, only outvoted. Each time a sprite learns something well enough, the event log says so ("Sprite #12 learned: thornbushes are bad").

Sprites live whole lives: the world's first live 4 to 8 hours at 1×, and their descendants can evolve longer lives. Each one hatches from an egg as a young sprite, which walks slowly, learns fast and watches the grown-ups, its parents most: a young sprite that sees its mother drink learns that water is good for thirst. After about an hour it grows up. Every sprite hatches male, and a grown male who sees no females turns female. A grown sprite looks for a partner, calling for one it can't see and dancing for one it knows, and the pair lay an egg and stay near it, and near their young until those grow up. The young are born with a mix of their parents' genes: now and then a number is copied wrong, or a gene is doubled, lost or changed, and hidden flaws show only when both parents pass the same one on. After about five hours a sprite grows old. It slows, its genes weaken, and about an hour later it dies. The family tree shows any sprite's parents, grandparents and young, and the population graph the world's rise and fall over the whole run.

You can teach them too. In Train mode a click pets the sprite under the Cursor and a right click zaps it (hold Ctrl with a click, or Shift with `Q` or `E`, for a hug or a shock). A pet rewards what the sprite just tried, so it learns to like that thing and to do it again, and a zap does the opposite. The Cursor can also follow a sprite so it moves with it, and in Grab mode pick up items, lead a sprite, throw items and shove sprites, or place new berries, balls, bushes and sprites. You can name a sprite, and save its genome to place a copy later.

You can select a sprite and look inside it: its Brain tab shows what it's paying attention to and why it does what it does, and below that its memory, everything it has learned and how strongly, and the places it remembers finding water or food. The map flashes an `X` where it's heading and shades what it's paying attention to in grey. You can scroll around and point at things, and the clock can be paused, stepped and sped up. A world can be saved and loaded, and saves itself every 10 minutes; and the map can be drawn with a theme of your own.

## Getting started

You need [Rust](https://rustup.rs) 1.97.1 or newer (`rustup update stable`) and a terminal of at least 100×30 characters; 140×40 is better. On Windows, use Windows Terminal. To install the game as a command, without downloading the code yourself:

```bash
cargo install --git https://github.com/Keazra/terra-sprites terra-tui
terra-sprites
```

Run the same `cargo install` line again to update it. Or, from a copy of the repository:

```bash
cargo run --release
```

The first build takes a minute or two. The game opens on its title screen: a light finds a sprite in the dark, and a terrarium wakes around it (any key skips this). Pick **New world** and press `Enter` for a new terrarium with time running at 1×, or **Continue** to carry on from your last save. A new world is a Story world, the game as it's meant to be played, which runs at 4× at most; `→` in New world picks a Lab world instead, for experiments, with every speed up to Max. Hold `?` there to see what each mode means. To go straight into a world instead, give it a seed: `cargo run --release -- --seed 7`, or `terra-sprites --seed 7` if you installed it (every flag below works either way). Some things to try first:

1. **Watch one sprite.** Click a sprite (`☺`) to select it. The panel beside the map shows its body; `]` moves to its Brain tab, which says what it's paying attention to and why. `T` makes the view follow it.
2. **Control time.** `space` pauses, `.` steps a little while paused, and `+` speeds up. A sprite's life is long at 1×, so try 4×.
3. **Teach it something.** Press `X` for Train mode. When the sprite does something you like, click it to pet it; right-click to zap it for something you don't. It learns from both.
4. **Move things.** Press `C` for Grab mode. Click a berry (`•`) to pick it up, and click again beside a hungry sprite to put it down. Press `C` once more for the Place menu, to add berries, balls, bushes and new sprites.
5. **Keep the world.** `F5` saves and `F9` loads it back; the game also saves itself every 10 minutes. `?` shows every key, the colour legend, and where your saves are.

The status line at the bottom always says what's under the Cursor, and the event log under the map tells you what the sprites are doing.

### Uninstalling

If you installed it with `cargo install`, this removes the `terra-sprites` command (`terra-tui` is the name of the code it's built from):

```bash
cargo uninstall terra-tui
```

If you ran it from a copy of the repository instead, delete that folder.

Either way, your worlds stay. Everything the game writes goes in one folder, the game's folder: saves and autosaves, the last session's replay, genomes you saved and fast-forward tables, along with any themes or presets you put there yourself. The help screen (`?`) shows where it is on its last line, `FILES`. Delete it to remove them too, or keep it, and the game finds them again if you reinstall:

| System | Delete the game's folder |
|---|---|
| Windows (PowerShell) | `Remove-Item -Recurse "$env:APPDATA\terra-sprites"` |
| macOS | `rm -rf ~/Library/Application\ Support/terra-sprites` |
| Linux and others | `rm -rf ~/.local/share/terra-sprites`, or `$XDG_DATA_HOME/terra-sprites` if you've set that |

`cargo install` also leaves what it downloaded in Cargo's folder (`~/.cargo`, or `$env:USERPROFILE\.cargo` on Windows), so the next build needn't download it again: the game's code, in the folders whose names start with `terra-sprites-` in `git/checkouts` and `git/db` (about 15 MB), and the libraries it's built from, in `registry` (about 130 MB), which any other Rust program you build shares. Both are safe to delete, since Cargo downloads what it needs again. If you installed Rust only for this game, `rustup self uninstall` removes Rust and everything in Cargo's folder, the game's command included; the game's folder stays until you delete it.

## Controls

The same key in capitals or lower case does the same thing.

| Key | Action |
|---|---|
| `space` | Pause / resume |
| `.` | Step while paused: a second's worth of ticks at the current speed, one at 1× and slower, up to 20 at 16× and Max (hold to keep stepping) |
| `+` / `-` | Faster / slower: each step doubles or halves the speed, from ⅛× up to 4× in a Story world, or on to 16× and then Max in a Lab world (the top bar says `lab` after the speed). The game starts at 1× (1.25 ticks per second, slow enough to watch sprites walk). Holding `+` stops at 1× and at 16×, and holding `-` at 1×; press again to go past |
| `W` `A` `S` `D` or arrows | Scroll the map (hold Shift to scroll 5 tiles) |
| Mouse | Point at a tile: the status line says what's there. Click a sprite to select it |
| `T` | Track: the view follows the selected sprite; again to stop |
| `b` | Sprite colours: what a sprite's colour shows: its strongest drive, nothing, or its lineage, which marks the selected sprite's parents and grandparents, brothers and sisters, and young and theirs each in a colour, and every other sprite dark gray. The status line says which |
| `m` | Event log filter: all events, the selected sprite's, or only the major ones (deaths, lessons learned, refusals) |
| `v` | Detail view: exactly what the selected sprite is doing |
| `?` | Help: every key, the colour legend and where the game keeps its files |
| `F3` | Debug overlay, in the map's top-right corner, and again to hide it: how the game is running on your computer, for a screenshot when it feels slow or a held key misbehaves. Read over the last second: the ticks a second it ran against the speed asked for (or `paused`, or `at Max`), the simulation's time in a frame against its 25 ms budget, and the drawing's time in a frame, with the frames a second. Then whether your terminal reports key releases: `releases reported`, `no releases` (a held key is told from taps only by its repeats), or `kitty protocol`. It measures the game as you play it, drawing included, where the speed example (`cargo run --release -p terra-sim --example speed`) measures the simulation alone |
| `Z` / `X` / `C` | Cursor mode: Select / Train / Grab. The mouse wheel over the map changes mode too. `C` again in Grab mode opens the Place menu |
| `Q` / `E` | Left / right click where the Cursor is. In Train mode the left pets and the right zaps (with Ctrl on a click or Shift on a key, a hug or a shock); in Grab mode the left picks up, leads, lets go or puts down, and holding the right aims a throw or a shove, which goes when you let go |
| `F` or the middle button | Follow the sprite under the Cursor (or else the selected one) so the Cursor moves with it; again to stop |
| `H` | Make the Cursor visible to sprites in the current cursor mode, or hide it again. Each mode starts hidden; while visible the Cursor is drawn as a frame of light (`═` and `║`) and the status line says "seen"; sprites can go to it or back away from it, and they learn to like or fear it from how you treat them |
| Place menu | `1`–`9`, or the arrows and `Enter`, or a click picks: a berry bush seedling, a berry, a ball, a new sprite, or a sprite from a genome file. The next Grab-mode click places it; a right click puts it away |
| `Tab` / `Shift+Tab` | Select the next / previous sprite |
| `l` | Sprite list: every sprite, with its age, life stage, strongest drive, generation and what it's doing. The arrows choose, `Tab` changes the order, `Enter` goes to it. `/` filters it: type the start of a name, a life stage, `gen 3` or what a sprite is doing, and only the sprites with it in their row stay; `Esc` clears the filter |
| `k` | Family tree: the selected sprite's grandparents and parents above it, its young below, grouped by their other parent, its partner named as such, and its brothers and sisters, living or dead (the dead in dark gray). The arrows or the mouse choose one, which blinks, and `Enter` or a click walks the tree to it and selects it |
| `p` | Population graph: the world's population over the whole run, each column split by the share of eggs, young, adults and old, from the bottom. `←` `→` or the mouse pick a moment, and the lines under the graph say how many there were then, of which generations, and what was laid, hatched, spawned or died since the column before. A small copy at the top of the World tab shows how many, with the world's limit |
| `r` | Name the selected sprite: type a name, or `Tab` for another random one, then `Enter` |
| `g` | Save the selected sprite's genome to the genomes folder (the status line says where); the Place menu reads it back |
| `[` / `]` | Previous / next inspector tab |
| `PgUp` / `PgDn`, or the mouse wheel over the inspector | Scroll a long tab |
| `F5` / `F9` | Quicksave / quickload. Loading over a world that has run since it was last saved asks first (`y` to load), and a loaded world starts paused |
| `Ctrl+S` / `Ctrl+O` | Save under a name you type / pick a save to load, newest first. The world also saves itself every 10 minutes of running time and when you quit, keeping the last 3 autosaves. See [saves](reference/saves.md) |
| `Ctrl+T` | Pick a theme: how the map is drawn. It lists the two built-in themes, then your own from the `themes` folder of the game's folder; see [themes](reference/themes.md) |
| `Ctrl+R` | In a replay, take it over: the world is yours from the tick it has reached, and the rest of the recording is dropped. Its next autosave writes a new `last_session.replay`, so copy a replay you want to keep first |
| `Esc` | Each press does the first that applies: close a menu or overlay, cancel an aim, let go of what the Cursor holds or leads (a held item is put down where the Cursor is), go back to Select, and from Select ask to quit: `y` quits the game, `t` goes back to the title screen (both save first, as quitting does), and any other key keeps playing |
| `Ctrl+C` | Quit at once |

| Flag | Effect |
|---|---|
| `--seed <n>` | Make the world from seed `n` (the top bar shows the seed of every world) |
| `--lab` | Make a Lab world rather than a Story world: every speed, up to Max |
| `--load <save>` | Open a save straight away, as Load does: a save's name in your saves folder, such as `--load quicksave`, or a path to a `.tspr` file |
| `--preset <file>` | Use a world config from a RON file, such as a copy of [`data/presets/default.ron`](../data/presets/default.ron) |
| `--data <dir>` | Use the data pack files in a folder, such as a changed copy of [`data/objects.ron`](../data/objects.ron), in place of the built-in ones; any file the folder lacks stays built in. A `presets/default.ron` in the folder is the default preset |
| `--replay <file>` | Play a replay back, such as `last_session.replay`, which every session writes to the game's folder (the help screen shows where). Time, the view and the inspector work, but nothing can change the world until you take it over with `Ctrl+R`. It pauses at the end, and if the replay ever drifts from what was recorded, it pauses and says between which ticks. See [replays](reference/saves.md#replays) |
| `--ascii` | Draw the map in plain ASCII instead of CP437 |
| `--theme <file>` | Start with a theme of your own, such as an edited copy of [`themes/cp437.ron`](../themes/cp437.ron), rather than picking it with `Ctrl+T` each time |
| `--fast-forward <hours>` | Run a Lab world with no screen for that many hours of play at 1×, then save it; see [fast-forward](#fast-forward) |
| `--save <name>` | With `--fast-forward`, the name to save under |
| `--version` | Print the game's version and the commit it was built from, and stop |

## Fast-forward

A Lab world can run for many generations with no screen, as fast as your computer allows, and then be opened in the game:

```powershell
cargo run --release -- --fast-forward 200 --seed 7
```

That runs a new world from seed 7 for 200 hours of play at 1× (900,000 ticks, about 90 generations), printing a row every tenth of the way: the sprites at each life stage, the eggs, the deepest generation reached, and the deaths since the row before, by cause. A line under the rows says how far it has got and how long it has left. `--preset` and `--data` work as they do for a new world; `--load mira` carries on the Lab save `mira` instead (a Story save can't be fast-forwarded).

At the end it saves the world in your saves folder as `seed-7-200h` (or `mira-200h`; `--save <name>` picks another name), so the title screen's Continue opens it. `Ctrl+C` stops it early and saves what has run. Beside the save it writes two tables that open in Excel or Google Sheets: `seed-7-200h-population.csv`, the population over the whole run, and `seed-7-200h-genes.csv`, the middle value of each of the starter genome's gene values among the living, an hour apart. The report ends with the ones that moved most.

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

The ground, the plants and toys, a sprite's body and its instincts, the size of the world and how the map is drawn are files anyone can edit. The [data-format reference](reference/README.md) explains each one, with a working example.

## Clips: GIFs, videos and SVGs

The renderer draws the game off-screen, through the game's own screen code, into a GIF, an MP4 or an animated SVG that loops. It's what the game looks like in Windows Terminal: Cascadia Mono, the Campbell colours, the same characters, title lettering and emotes. The Cursor is out of the picture unless `--cursor` asks for it. The title screen's opening scene:

```powershell
cargo run --release -p terra-tui --example render -- title -o title.gif
```

A world from seed 7, from tick 200 to tick 300 at 4×:

```powershell
cargo run --release -p terra-tui --example render -- world --seed 7 --from 200 --to 300 -o seed-7.gif
```

The two clips at the top of the README are made the same way, the second at 1× with the Brain tab of the sprite it tracks:

```powershell
cargo run --release -p terra-tui --example render -- title -o docs/clips/title.gif
cargo run --release -p terra-tui --example render -- world --seed 7 --from 897 --to 926 --track 533 --tab brain --speed 1x -o docs/clips/thornbush.gif
```

The file's ending picks the format: `.gif`, `.svg` or `.mp4`. A GIF or an SVG needs nothing else; an MP4 needs [ffmpeg](https://ffmpeg.org/download.html) on the PATH, and the renderer says so if it isn't. The world runs to `--from` unseen, then is drawn frame by frame to `--to`, at `--speed` (`1x`, `4x`, or `max` for 100 ticks a second; past 4× needs `--lab`), each frame showing for `--frame` milliseconds (100 by default). An SVG for a README is best kept under 2 MB, and the renderer says when one isn't: a busy world at a high speed runs past it in a few seconds.

`--replay <file>` plays a replay back instead of a seed's world, from where its recording starts unless `--from` says. That's how to show the Cursor at work: play the scene in the game, then render its `last_session.replay`.

The picture is 100×30 screen cells by default, the game's smallest screen with its panels. `--size <WxH>` picks another, `--map-view` draws the map view alone, with no border or panels, at any size, `--hide` leaves out parts of the screen for the map view and the inspector to take their room (`top-bar`, `inspector`, `event-log` or `status-line`, a comma between several, as in `--hide event-log`), and `--cell <WxH>` sizes a screen cell in pixels (10×20 by default). The view tracks the sprite `Tab` would select first, or `--track <n>` names another. Upright 9:16, for Shorts, at 1080×1920:

```powershell
cargo run --release -p terra-tui --example render -- world --seed 7 --from 200 --to 300 --size 54x48 --map-view --cell 20x40 -o seed-7-upright.mp4
cargo run --release -p terra-tui --example render -- title --size 108x96 -o title-upright.mp4
```

`--font <file.ttf>` draws in another font. To compare fonts, `samples` draws a still of a world's screen in Cascadia Mono and in each `--font`, a file each, and says which of the screen's characters a font has no glyph for: `cargo run --release -p terra-tui --example render -- samples --font DejaVuSansMono.ttf --font Iosevka.ttf -o fonts.gif` writes `fonts-cascadia-mono.gif`, `fonts-DejaVuSansMono.gif` and `fonts-Iosevka.gif`. The same command makes the same file, byte for byte, on any computer (an MP4, with the same ffmpeg). `--help` lists the rest.

## Working on the game

To run the checks CI runs: `.\scripts\check.ps1` in PowerShell, or `scripts/check.sh` (formatting, lints and every test). To try a pull request's build: `.\scripts\try-pr.ps1 <number>`. To have Gemini review them, through the Antigravity CLI: `.\scripts\review-pr.ps1`, which lists the open PRs to tick (`-Sandbox` runs the reviews hands-off in a Linux sandbox; the script's opening comment says how).

The game's designs:

- [M2 "Generations" design](design/m2-generations.md) (the plan for milestone 2: its slices, the time scale of a sprite's life, and what "M2 is done" means; and what its slices have settled so far: memory of places, the title screen, and where places sit on the Brain tab, life stages, breeding, genes, the population graph, and fast-forward and the speed cap).
- [M1 "A Sprite Lives" design](design/m1-a-sprite-lives.md), the rulebook for everything M2 doesn't change.
- [M4 "Terra" design](design/m4-terra.md) (the plan for the ship, a later milestone: its sectors, devices and sealed doors, and what "M4 is done" means).
- [Design changes](design/changes/): each change to a design, one file per PR; its README says where the earlier numbered revisions are. The evaluations of v1 are in [`docs/design/reviews/`](design/reviews/).

The `docs/terra-sprites-docs` folder is empty in your copy. It's where the owner keeps their private working notes, and nothing in the game needs it.

## Roadmap

| Milestone | Contents |
|---|---|
| **M1 — A Sprite Lives** *(done)* | World, ecology, biochemistry, learning brain, terminal UI, the Cursor, saves and replays |
| **M2 — Generations** | Memory of places, a title screen, life stages and ageing, breeding, genetics, a family tree, population graphs, fast-forward and a speed cap, learning by watching, a harsher world |
| **M3 — Body & verbs** | Limbs that give sprites their verbs: touching, grabbing, pushing, pulling, throwing and dropping things |
| **M4 — Terra** | The terrarium is a ship, Terra: its sectors open one by one as the sprites learn |
| **M5 — Wild Terra** | Critters, more hazards and toys, possibly seasons and weather |
| **M6 — Words** | Teaching sprites words |
| **Tiles** | A tile-window front end with sprite-sheet support |
