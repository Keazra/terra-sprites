# Terra Sprites — M1 "A Sprite Lives" design (v29)

- **Status:** Final
- **Date:** 2026-10-02
- **Supersedes:** [v28](archive/m1-a-sprite-lives-v28.md) (earlier: [v27](archive/m1-a-sprite-lives-v27.md), [v26](archive/m1-a-sprite-lives-v26.md), [v25](archive/m1-a-sprite-lives-v25.md), [v24](archive/m1-a-sprite-lives-v24.md), [v23](archive/m1-a-sprite-lives-v23.md), [v22](archive/m1-a-sprite-lives-v22.md), [v21](archive/m1-a-sprite-lives-v21.md), [v20](archive/m1-a-sprite-lives-v20.md), [v19](archive/m1-a-sprite-lives-v19.md), [v18](archive/m1-a-sprite-lives-v18.md), [v17](archive/m1-a-sprite-lives-v17.md), [v16](archive/m1-a-sprite-lives-v16.md), [v15](archive/m1-a-sprite-lives-v15.md), [v14](archive/m1-a-sprite-lives-v14.md), [v13](archive/m1-a-sprite-lives-v13.md), [v12](archive/m1-a-sprite-lives-v12.md), [v11](archive/m1-a-sprite-lives-v11.md), [v10](archive/m1-a-sprite-lives-v10.md), [v9](archive/m1-a-sprite-lives-v9.md), [v8](archive/m1-a-sprite-lives-v8.md), [v7](archive/m1-a-sprite-lives-v7.md), [v6](archive/m1-a-sprite-lives-v6.md), [v5](archive/m1-a-sprite-lives-v5.md), [v4](archive/m1-a-sprite-lives-v4.md), [v3](archive/m1-a-sprite-lives-v3.md), [v2](archive/m1-a-sprite-lives-v2.md), [v1](archive/m1-a-sprite-lives.md))
- **Covers:** Milestone 1 in full detail, plus the architecture decisions that every later milestone depends on

---

## Changes from v28

Slice 14 ([#15](https://github.com/Keazra/terra-sprites/issues/15)), body language and the finished screen, settled with the owner before building: seven questions, each with a recommendation, all accepted. New terms in [`CONTEXT.md`](../../CONTEXT.md): Track, event filter, colour mode, help screen, sprite list, information policy.

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The information policy names its panels and subjects.** A display asks `can_view(panel, subject)`, where the panel is one of map colours, emotes, the selected sprite's marks, an event log line, the top bar's counts, a sprite list row, the status line's tile info, or an inspector tab, and the subject is the world, a sprite or a tile. A display denied shows nothing in its place; a denied event log line is left out. M1's policy, `Omniscient`, always says yes; a test with one that says no shows every display blank. | Follows from the slice | §6.4 |
| 2 | **Drive colours live in the themes,** beside the glyphs, by the drive's name (`hunger: yellow`). A drive a data pack adds that no theme colours draws in the sprite's own colour. The help screen's legend lists the pack's drives in its own order (`DataPack::drives`), so a new drive appears there with no new code. | Owner decision (recommendation accepted), over putting terminal colours in the data pack | §6.2, §6.3 |
| 3 | **The colour modes** are "strongest drive" and "plain"; the game starts on strongest drive. `b` goes to the next and the status line says which for 3 seconds: "Colours: plain". A drive must be above .5 to colour its sprite; of two equally strong, the first in the data pack's order wins. | Recommendation | §6.3 |
| 4 | **Failed and Resting emotes.** Failed is a light yellow `?` (white `?` is an object the theme doesn't know), on an action ending blocked, timed out or failed. Resting is a light blue `z` for as long as the rest lasts **and at least a second** of real time, so it's seen at every speed. Of two emotes at once the newest wins, except Resting, which gives way to any other and comes back after it. | Owner decision (recommendation accepted) | §6.3 |
| 5 | **Pleased from a good feeling** (`♥`): a tick in which a sprite's `last_r` reaches **0.3** shows it, as a pet or a hug does. Measured over 1.79 million sprite-ticks of the default world: about 6,000 felt any reward, and 927 reached 0.3, about one in seven, such as a meal while hungry; small comforts don't. The screen collects these sprites as each tick runs (`Ticks`), since `last_r` is overwritten by the next. | Measured, as the owner asked ("I'll set how good by measuring") | §6.3 |
| 6 | **Track (`T`)** keeps the selected sprite in the middle of the map view, as far as the wall allows. It follows whichever sprite is selected; scrolling by hand turns it off. With no sprite selected it's refused: "Select a sprite to track it". The status line says "Tracking Mira #12" and "Stopped tracking". | v21's Track, built at last; the issue's "follow mode (`f`)" is this, renamed in v21 | §6.1, §6.5 |
| 7 | **A terminal smaller than 100×30** shows only "Terminal too small" and "needs 100x30, this is 80x24", in the middle. The game carries on as it was set: shrinking the window doesn't pause it, and `space` and `Esc` still work; "Quit? (y/n)" shows under the message. At 100×30 and up, every panel always shows. | Owner decision (recommendation accepted) | §6.1 |
| 8 | **The event filter** is `m`, all → selected → major, and a click on the filters on the event log's border does the same. They read ` [all] selected major `, the shown one in brackets. "Selected" is the events the selected sprite did or had done to it; "major" is deaths, lessons learned and refused commands. | Owner decision (recommendation accepted): v1 shows the filters but gave them no key | §6.1, §6.5 |
| 9 | **The help screen** (`?`) fills the screen between the top bar and the status line: every key in three columns of groups, then the colour legend (a sprite glyph in each drive's colour, and "none"), the emote legend, and the game's folder for its files. It fits at 100×30, so it doesn't scroll. `?` or `Esc` closes it; other keys do nothing while it's open. The top bar ends with `? help`. | Owner decision (recommendation accepted); scrolling dropped since it fits | §6.1 |
| 10 | **The sprite list** (`l`) fills the same space: a row per sprite, with its label, age, strongest drive above .5 (in the map's colour, or `-`) and what it's doing as the Body tab says it. `Tab` sorts by number, name (named ones first, by name, then the rest by number), age (oldest first) or drive (in the data pack's order, strongest first, none last); `Shift+Tab` goes back. The arrows or the wheel move a highlight that stays on its sprite when re-sorted; `Enter` or a click on a row selects that sprite, centres the view on it and closes the list; `l` or `Esc` closes it. Its title says the order, " Sprites ── sorted by drive ", and the status line's hints read `↑↓ choose  enter go to it  tab sort  esc close`. The column is "Drive", not "Need", since pain is a drive but not a need. | Owner decision (recommendation accepted) | §6.1 |
| 11 | **Hesitating before a choice,** the note carried on #15, moved to its own issue, [#103](https://github.com/Keazra/terra-sprites/issues/103): it's brain work needing a design session of its own. | Owner decision (recommendation accepted) | — |
| 12 | **Snapshot tests** of each inspector tab, the help screen and the sprite list, in both themes, are text files in `crates/terra-tui/tests/snapshots/`, written afresh with `UPDATE_SNAPSHOTS=1`. A second test checks every cell of every screen, prompt and overlay is within CP437. | Follows from §7.2 | §7.2 |

**Earlier changes** are in the archived revisions, in [`archive/`](archive/). Each opens with its own table: v28's changes (from v27) head [v28](archive/m1-a-sprite-lives-v28.md), and so on back to v2. So "v16 change 16" is row 16 of the table at the top of [v16](archive/m1-a-sprite-lives-v16.md). The current revision carries only its own table, so the spec doesn't open with its whole history.

---

## 0. Vision and key decisions

Terra Sprites is a terminal artificial-life game inspired by *Creatures*. Sprites are small creatures with a genome, a simulated biochemistry and a learning brain. They live on a top-down grid world with plants, water, toys and hazards.

**The simulation is the star.** Behaviour must emerge from drives, chemistry and learning, not from scripts. The player watches, inspects and intervenes through **the Cursor** (training, moving things) and sees individual sprites learn from experience.

**The Cursor** (v21): to the sprites, the player is an advanced creature, and the Cursor is the hard-light projection they cast into the terrarium, a 3×3 grid of light. In M1 sprites can't see it, so its touch is a feeling from nowhere. Making it visible, so sprites can come to know it, is a slice of its own ([#60](https://github.com/Keazra/terra-sprites/issues/60)). Later milestones add breeding and evolution, a wilder world, language, and a tile-rendered front end.

| Decision | Choice | Why |
|---|---|---|
| Core appeal | Emergent artificial life | The simulation must be real, not scripted to look alive |
| Platform | Rust terminal app (ratatui + crossterm) | Fast, deterministic simulation with no GC; solid on Windows Terminal |
| Where behaviour comes from | Learning within a lifetime **and** evolution across generations | Instincts evolve; personalities are learned |
| Brain model | Two pathways that compete, as in the fruit-fly brain (v16): instinct (attention and concepts → decision, from genes, fixed) and learning (what each object type is worth, per need, with separate good and bad channels, and habits), with reinforcement carried by chemicals | Learning and evolution are both visible at 20–100 sprites, reward and correct mean something, and a species with very basic instincts can learn its world |
| World | Top-down grid | Simple movement and perception; room for many sprites |
| Player role | Through the Cursor: observe, reward, correct, grab, place | Player feedback feeds directly into learning |
| Reproduction (M2) | Sexual and asexual; the genome decides which | Mode of reproduction can itself evolve |
| Population | 20–100 sprites | Individuals still matter, and evolution is visible within hours |
| Language | M4 | Brain I/O is designed now so words can plug in later |
| Information UI | Omniscient, plus body language on the map | M1 is about observing and tuning learning; a policy seam leaves room for a diegetic mode later |
| Rendering | Real terminal in M1; tile window with sprite sheets later | M1 risk stays on the sim; semantic tiles and CP437-only text keep the door open |

---

## 1. Scope

### 1.1 Roadmap

| Milestone | Contents |
|---|---|
| **M1 — A Sprite Lives** *(this document)* | World, ecology, biochemistry, learning brain, terminal UI, the Cursor, save/load, replay |
| **M2 — Generations** | Life stages; sexual and asexual reproduction decided by the genome; crossover and mutation; lineage and family-tree view; population graphs; headless fast-forward mode |
| **M3 — Wild Terra** | Critters (prey and predators); more hazards and toys; possibly seasons, weather, day/night and temperature |
| **M4 — Words** | The player (and later, sprites) name objects and verbs; word inputs and a Speak output |
| **Tiles** *(UI milestone, can be scheduled any time after M1)* | A tile-window front end that draws bitmap tilesets and sprite sheets through the semantic-tile seam (§6.2) |
| **Beyond Terra Sprites** *(vision, [#47](https://github.com/Keazra/terra-sprites/issues/47))* | A larger game: the player is one adventurer in a world of unique NPCs that run on a version of the Sprite system, with skills a player can lock, and a character the player can hand over to live as an NPC and take back. Nothing is built for it; it's why `terra-sim` stays a general engine |

### 1.2 In scope for M1

- **World:** a grid world with terrain, plus data-defined objects that have lifecycles: berry bushes, berries, thornbushes, balls, and water.
- **Sprites:** each has a genome, a biochemistry and a lobe brain that learns. Sprites age, and die from injury, which covers starvation, dehydration, old age and harm.
- **Starter population:** sprites are created from a starter genome with small random variation. They can also be spawned from a genome file.
- **Terminal UI:**
  - map with body language (drive colours and emotes)
  - sprite inspector (Body / Brain / Chem / Genome / World tabs)
  - event log
  - the Cursor, invisible to sprites
  - time controls
  - save, load, autosave
  - replay recording and playback
- **Lab runner:** a scenario runner with no UI, used by tests and for tuning.

### 1.3 Out of scope for M1

- Reproduction of any kind; the population can only be topped up by the player spawning sprites
- Day/night, seasons, temperature
- More than one type of nutrient
- Sprites carrying objects (only the Cursor holds things)
- Line of sight, swimming
- Organs and organ failure ([#31](https://github.com/Keazra/terra-sprites/issues/31))
- Critters, language
- A diegetic information mode
- The tile-window front end
- A general headless mode; only the lab runner exists in M1
- GitHub CI, until a remote exists
- Mods: the one-file format, a mods folder, and installing into or removing from a running world (v19, [#73](https://github.com/Keazra/terra-sprites/issues/73))

### 1.4 Obligations M1 carries for later milestones

- **Genome format:** gene type IDs are stable, payloads are versioned, and unknown genes are preserved (§2.8).
- **Registries:** every ID is stable and append-only. This covers chemicals, loci, brain inputs and outputs, categories and object types. `Mate` (M2) and `Speak` (M4) have IDs reserved now.
- **Objects:** defined as data, with a closed rule vocabulary (§3.5).
- **Easy modding** (v19): a mod will be able to add object types, genomes and tags of its own (v23), and nothing else (§3.5.5). Instincts and genomes name categories, never object types, and nothing outside a world's own pack refers to an object type by number. A mod's own tags are known by an ID the mod is given at random once, when it's made, and keeps in its file, so two mods' tags of the same name never clash ([#73](https://github.com/Keazra/terra-sprites/issues/73)).
- **Rendering:** the map is drawn from semantic tiles, and all UI text uses only CP437 characters (§6.2).
- **Information:** everything the UI displays passes through the `InfoPolicy` seam (§6.4).

---

## 2. Architecture

### 2.1 Repository layout

```
terra-sprites/
├─ Cargo.toml                 workspace
├─ crates/
│  ├─ terra-sim/              library: the whole simulation. No terminal, no file or network I/O.
│  │  ├─ examples/lab.rs      headless scenario runner (tests + tuning)
│  │  └─ examples/baseline/   baseline runs on `main` (§7.6)
│  └─ terra-tui/              binary `terra-sprites`: ratatui + crossterm front end
├─ data/                      default data pack (RON), embedded in the binary
│  ├─ pack.ron                pack name + version
│  ├─ terrain.ron  categories.ron  tags.ron  objects.ron  chemicals.ron  loci.ron  brain_io.ron  physiology.ron
│  ├─ genomes/starter.ron
│  └─ presets/default.ron     default WorldConfig
├─ themes/                    UI themes (cp437.ron, ascii.ron). UI assets, NOT part of the sim data pack.
└─ scenarios/                 lab scenarios (RON) for tests and tuning
```

**Dependency boundary:**
- `terra-sim` takes a data pack as parsed values and never touches the filesystem itself.
- `terra-tui` loads files, owns the terminal, and talks to the sim only through its public API.

**Main crates:**

| Crate | Used for |
|---|---|
| `ratatui`, `crossterm` | Terminal UI |
| `serde`, `ron` | Data files and genome text |
| `rmp-serde` | Saves |
| `rand_chacha` (with `serde1`) | Seeded RNG whose state can be saved |
| `libm` | Portable transcendental math |
| `xxhash-rust` (xxh3) | State hash |
| `directories` | Platform data folder |
| `proptest`, `criterion` | Property tests, benchmarks |

### 2.2 Simulation API (shape)

```rust
let config = WorldConfig::from_ron(text, &data_pack)?;   // a preset names object types, so it's checked against the pack
let mut world = World::new(config, data_pack, seed);    // can't fail: config and pack were validated when parsed
world.submit(Command::Reward { sprite, amplified: false, reach: 3 }); // stamped for tick now+1, applied at step 1
let events: Vec<Event> = world.step();                   // advances exactly one tick
world.state_hash();                                      // u64, stable across runs
world.check_invariants();                                // debug/test builds
```

- The UI only **reads** world state.
- Every change goes through a `Command`.
- Everything that happened is reported as an `Event`.

### 2.3 Entities and ordering

- Sprites and objects get `EntityId(u64)` values from a world counter that only goes up. **IDs are never reused.**
- Every per-entity pass processes entities in **ascending ID order**. How they're stored is an implementation detail.
- Game logic never iterates a `HashMap` or `HashSet`.
- There is exactly one RNG, a ChaCha8 owned by the world. Every random draw in the sim comes from it.

### 2.4 Canonical tick order

**This table is the single source of truth for timing.** Every other section refers to it by step number and does not restate it.

| Step | Name | What happens |
|---|---|---|
| 1 | **Commands** | Apply the commands stamped for this tick, in the order they were submitted. The Cursor's direct chemical injections happen here, and its pulses go into the target's `incoming` buffer. Invalid commands are rejected with a `CommandRejected` event. |
| 2 | **Environment** | Object rules run (stages, counters, spawning, spreading, expiry). This covers the objects that exist **when step 2 begins**, in ascending ID order, with rules in the order listed (§3.5). Objects created during step 2 first run next tick. Then rolling items move one tile each, in ascending ID order (§3.5.4), and then sliding sprites, likewise (v25). |
| 3 | **Biochemistry** | For every sprite, **led or not**: (a) pulse latch: `live ← incoming` and `incoming` is emptied; (b) physiology; (c) reactions; (d) half-life decay; (e1) Level emitters; (e2) Rise/Fall emitters; (f) receptors. No level leaves [0, 1] at any point (§4.4). Then **death check #1**: if `injury ≥ 1.0` the sprite is marked **dying**. So is a sprite whose injury a crash took to 1 at step 2, whatever (b)'s healing does (v25). |
| 4 | **Learning** | For every sprite not marked dying: read each need's relief, `reward` and `punishment`, then **reset reward and punishment to 0** (learning consumes them). Credit worth to the thing touched and habits along the trace ring buffer (the freshest entry is from the previous tick), then let learned values fade (§5.6). |
| 5 | **Sense and decide** | For every sprite not marked dying: refresh the perception flood if needed (§3.6); then, unless the sprite is led (§6.5) or sliding (§3.5.4), **5.0** check the current action, **5a** attention, **5b** decision (§5.5). The activations are snapshotted. |
| 6 | **Resolve actions** | In an order shuffled each tick by the world RNG: movement steps (including head-on swaps, §3.7) and verb effects. A led sprite's steps take it towards the Cursor (§6.5); a sliding one takes none (v25). Effects inject physical chemicals and write `incoming` pulses. Then each deciding sprite **commits one trace entry**: its snapshot (§5.6). This happens every tick, whether the action just started or is continuing. A led sprite decides nothing, so it commits none. |
| 7 | **Deaths and events** | **Death check #2** catches injury from step 6. Sprites marked dying are removed; a led sprite that dies empties the Cursor. Events are emitted, and the tick counter goes up. |

**Timing consequences** (these hold everywhere):

- **An action's effects reach the sprite's next decision, never its current one.** An action at tick *t* is processed chemically at step 3 of *t+1*, learned from at step 4 of *t+1*, and affects the decision at step 5 of *t+1*.
- **The Cursor's feedback credits what the player just saw.** The player sees the world as it stands at the end of tick *t*, and their command is stamped *t+1*. It's applied at step 1 of *t+1*, and at step 4 the freshest trace entry is decision *t*. A player reacts in about a second, which is many ticks at high speed, so a Reward carries its reach back, how far back the feeling looks for the sprite's latest attempt (§5.6). A Correct looks back only the touch window, so a late shock can't land on the wrong thing.
- **Pulses stay live long enough to be seen.** A pulse written at step 6 of *t*, or at step 1 of *t+1*, is live for steps 3–5 of *t+1*: emitters see it at step 3, and attention and concepts see it at step 5.
- **Reward only credits the past.** Reward and punishment produced at step 3 are consumed at step 4 of the same tick, so they credit entries up to the previous tick and never linger onto later decisions.
- **A sprite whose injury crosses 1.0 at step 3 makes no decision that tick.**
- **Led sprites** (v23) run steps 3 and 4 and refresh their flood at step 5, but make no decisions and commit no trace entries. At step 6 they walk towards the Cursor.
- **Sliding sprites** (v25) are the same, except that they take no steps at step 6: they slide at step 2. A slide that ends at step 2, stopped or past its last tile, frees the sprite to choose at that tick's step 5, or, if the Cursor took hold of it meanwhile, to walk after the Cursor at step 6. A crash at step 2 reaches the sprite's chemistry at step 3 of the same tick, and its learning at step 4.

Before step 1, the world keeps every sprite's chemical levels as they stand, so the Chem tab can show each one's change over the tick (§6.1). It's bookkeeping, not a step: nothing in the sim reads those levels.

### 2.5 Commands and events

| Command | Effect | Rejected when |
|---|---|---|
| `Reward { sprite, amplified, reach_back }` | A pet, or amplified a hug: injects `reward` directly (§4.6); pulses `petted`. Its feeling looks `reach_back` ticks back for the sprite's latest attempt (§5.6) | The sprite is gone (missing, or dead: a dead sprite has left the world). A led sprite isn't refused (v23) |
| `Correct { sprite, amplified }` | A zap, or amplified a shock: injects `punishment` and `pain` directly (§4.6); pulses `shocked`. Its feeling looks back only `touch_window` ticks, whatever the speed (v21 change 6) | As Reward |
| `TakeHold { sprite }` (v23) | Takes hold of the sprite, which is then **led** (§6.5): its action ends `pulled_away`, and the Cursor's tile, as the world knows it, becomes the sprite's. A sliding sprite's slide carries on to its end first (v25) | The Cursor already holds or leads something, or the sprite is gone |
| `PickUp { item }` (v23) | Picks up the item, which is then **held**: it leaves the map, and a roll ends (§3.5.4) | The Cursor already holds or leads something; the item is gone; it's a fixture |
| `LetGo` (v23) | Lets go of the led sprite, which chooses afresh at its next step 5 | The Cursor leads no sprite |
| `PutDown { tile }` (v23) | Puts the held item on the tile, at rest | The Cursor holds no item, or the item can't go there (§3.4): a sprite's under it is fine, but not another object, or terrain that isn't walkable |
| `MoveCursor { tile }` (v23) | Moves the Cursor, as the world knows it, to the tile. The UI sends it at each move onto a new tile while the Cursor leads a sprite, or will once a queued `TakeHold` applies (§6.5); the world keeps the tile it was last told | The tile is off the map |
| `Throw { from, toward, tiles }` (v25) | Puts the held item down on `from` and sets it rolling `tiles` tiles in the direction `toward`, one of the 8, as a kicked ball rolls (§3.5.4), from step 2 of this tick. `tiles` is taken as at least 1, and at most the Cursor's furthest for the item's size (Appendix B) | As `PutDown` |
| `Shove { toward, tiles }` (v25) | Lets go of the led sprite, which slides `tiles` tiles in the direction `toward` (§3.5.4), from step 2 of this tick. `tiles` is bounded as for `Throw`, by the sprite's size | The Cursor leads no sprite |
| `Place { tile, object_type }` (v28) | Creates an object of the type with that stable ID, at the beginning of its first stage, if its data offers it in the Place menu (§3.5.1). Placing leaves what the Cursor leads or holds alone | The type can't be placed (or isn't in the pack); the tile is off the map; something's in the way as for `PutDown`, or for a solid object, a sprite there or terrain that doesn't allow fixtures; or the tile fails one of the type's placement conditions, named |
| `SpawnSprite { tile, genome: Option<Genome> }` (v28) | Creates a newborn sprite. `None` means the starter genome with spawn variation (§4.9) | The tile is off the map; there's no room: a sprite, a solid object, or terrain that isn't walkable (an item is fine, §3.4); or the genome doesn't fit the pack |
| `Rename { sprite, name }` (v28) | Sets the sprite's name, trimmed of spaces at either end. The screen makes up a random one from the pack's syllables (§6.5), so naming never draws from the world's RNG | The sprite is gone, or the name is empty, longer than 16 characters, or not CP437 |

**Commands carry values, never references.** A genome loaded from a file is parsed by the UI and put inside the command in full, and a Reward carries its reach back as a number of ticks, worked out by the UI from the speed (§6.5). This keeps the command log self-contained. A reach back below `touch_window` or above `max_reach_back` (Appendix B) is taken as that bound.

**Each command is applied on its own.** Several in one tick all apply, in order, but a level can't pass 1, so ten pets in a tick give one full dose of reward.

**Events** each carry the tick and the entity IDs involved. The M1 set:
- `ActionStarted`, `ActionEnded { outcome, action }`: `action` is the ended action's view, with its target and whether it made its attempt, so the screen can describe it afterwards (§6.1)
- `Ate`, `Drank`, `Played`, `Hit`, `Pricked`
- `Rewarded { sprite, amplified }`, `Corrected { sprite, amplified }`
- `Placed { id, object_type, pos }`, `Spawned { id, pos }`, `Renamed { id, name }` (v28): the Cursor placed an object or spawned a sprite, or the player named one
- `Died { name, cause, age }`: the sprite has left the world by the time the event log prints this, so the event carries its name, or none for an unnamed sprite (§6.5). The causes are in §4.10.
- `LearnedMilestone { learned, good }`: a lesson (§5.6), once per learned value; `learned` names it (an object type's, a category's summary's (v19), a remembered sprite's, or sprites in general's worth for a need, general good, bad or fear (v18); a habit, an object type and a verb; or new things), and `good` whether it rose
- `ObjectSpawned { object_type, pos }`: an object was created during the tick (by a lifecycle rule; later also by the Cursor)
- `ObjectRemoved { object_type, reason }`: an object left the world. The reason is `Expired` (its last stage ended), `Destroyed` (`DestroySelf`) or `Replaced` (`ReplaceWith`). A berry that sprouts emits `ObjectRemoved { reason: Replaced }` for the berry and `ObjectSpawned` for the bush.
- `TookHold { sprite }`, `LetGo { sprite }` (v23): the Cursor began or stopped leading a sprite
- `PickedUp { id, object_type }`, `PutDown { id, object_type, pos }` (v23): the Cursor picked up or put down an item
- `Threw { id, object_type }`, `Shoved { sprite }` (v25): the Cursor threw an item or shoved a sprite
- `Crashed { sprite, into, hurt }` (v25): a sliding sprite crashed into a thing (§3.5.4), a sprite or an object, and whether the crash hurt it
- `CursorEmptied { reason }`: what the Cursor had left it by itself: the held item expired, or the led sprite died
- `CommandRejected { command, reason }`. The reasons (v23): the sprite or item is gone; the Cursor already holds or leads something; the thing is rooted (a fixture); the Cursor has nothing to let go of (or shove), or no item to put down (or throw); something's in the way, named: a sprite, an object, or the terrain. From v28 also: no room for a new sprite, named likewise; the type can't be placed; a placement condition fails, named; the name is empty, too long or not CP437; the genome doesn't fit

### 2.6 Determinism and floating point

- Simulation maths uses `f32`, and only IEEE-754 basic operations (+ − × ÷ and sqrt), which are exactly specified.
- Transcendental functions (exp, pow, tanh…) go through the pure-Rust **`libm`** crate, never `std`.
- No fast-math. No parallelism in sim logic in M1.
- **No FMA fusion.** rustc never marks floating-point operations as contractible, so LLVM doesn't fuse `a * b + c` into FMA instructions, and no compiler flags are needed. (This differs from C and C++, where Clang contracts by default.) The one real FMA risk is an explicit `mul_add`, which is fused by definition and falls back to the platform's `fmaf` on hardware without FMA. **Sim code doesn't use `mul_add`.** The CI cross-platform hash check (§7.5) is the empirical proof.
- **Guaranteed:** bit-identical results with the same sim version, the same data pack, the same config, seed or snapshot, the same commands, and the same target triple.
- **Promised across platforms only for the targets in the CI determinism matrix (§7.5),** once its check passes: Linux x86_64, Windows x86_64, macOS aarch64, and Linux aarch64 if available. Any other 64-bit target is only **expected** to match. The hash check is the evidence; the reasoning above about the compiler is background, not proof.
- **Not supported:** replaying across sim versions.

### 2.7 Replays

A replay file contains:

| Part | Contents |
|---|---|
| Header | `sim_version`, `schema_version`, the **embedded data pack** and its identity (name, version, content hash), the `WorldConfig` |
| Start | `Fresh { seed }` or `Snapshot(save bytes)` |
| Commands | `[(tick, Command)]`, including commands that were rejected, so the rejections replay exactly too |
| Checkpoints | `[(tick, state_hash)]` every 1,000 ticks |

- **Version mismatch:** a replay whose `sim_version` or `schema_version` differs from the running build is refused, with a clear message.
- **Divergence:** during playback, checkpoints are verified, and the first mismatch reports the **first tick where the replay diverged**.
- **Session log:** every session writes `last_session.replay`. Its start point is the fresh world or the loaded save. It's flushed at each autosave and on exit, including from the panic hook.
- **Playback:** `--replay <file>` disables all input that changes the world. Time controls and the inspector still work.

### 2.8 Saves and compatibility

**The save file:**
- Encoded as MessagePack with named fields (`rmp-serde`).
- **Header:** magic `TSPR`, `schema_version`, `sim_version`.
- **Contents:**
  - the tick counter and the full **RNG state**
  - the `WorldConfig` and the embedded data pack
  - the entity ID counter and every entity
  - the Cursor: the item it holds or the sprite it leads, and the tile it was last told it's on (v23)
  - the commands submitted and waiting for the next tick (v21), such as clicks made while paused
  - per sprite:
    - chemicals, both pulse buffers, and `last_r` (§5.6). Not the levels from one tick ago that the Chem tab's changes come from: nothing in the sim reads them, so after a load every change reads blank for one tick
    - the brain (learned worth, habits and familiarity for each object type (v19), the sprites it remembers and what it learned about each (v18), the worth of new things, which lessons have fired, and the trace ring buffer)
    - the current action, including any Wander destination, bout progress and committed path (§3.7)
    - movement state (move points, blocked-tick counter, last step direction), and a slide's direction and tiles left (v25)
    - the cached perception flood and its refresh timer

**Schema evolution:**
- **Additive changes**, meaning new fields with serde defaults, need no version bump.
- **Breaking changes** bump `schema_version` and add a `migrate_vN_to_vN+1` that works on frozen copies of the old types. Loading runs the chain one step at a time.
- A save **newer** than the build is refused.
- Every released schema version keeps a **golden save file** in the test suite, and it must still load.

**Data pack ownership:**
- When a world is created, the data pack in use (embedded defaults or `--data` overrides) is **embedded in the world**.
- Loading a save uses the save's pack, never the files on disk.
- `--data` affects only new worlds. Swapping the pack of an existing world is not supported in M1.

**Genes:**
- Each gene has a stable `GeneTypeId` (u16) and a `payload_version` (u8).
- Older payloads are upgraded on load by a migration function for that gene type.
- A gene with an **unknown type**, or a payload version **newer** than this build knows, is kept as opaque bytes:
  - it is not expressed and not mutated
  - it is inherited unchanged (M2)
  - it's shown in the genome viewer as "unknown gene type N"
  - it round-trips through load and save byte for byte

**Genome files** (RON: the starter genome, and genome files the player loads or exports):

```ron
(
    format: 1,
    genes: [
        Trait(trait: "speed", value: 7),
        Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: 0.02, chem: "hunger"),
        Emitter(locus: Locus("ate"), mode: Level, gain: -0.6, chem: "hunger"),
        Gene(type: 42, version: 1, payload: "93a4c2"),
    ],
)
```

- Genes are written by **type name**, with chemicals, loci and traits **by name**, as in `objects.ron`. Loading resolves the names to stable IDs.
- Fields at their defaults may be left out (`invert: false`, `threshold: 0`).
- **Any gene may be written by number** as `Gene(type, version, payload)`, with the payload's bytes in hex. A build that knows the type decodes it, migrating an older version; one that doesn't keeps it as an unknown gene. Exporting writes known genes by name and unknown ones by number, so an unknown gene reads back unchanged.
- **`format`** numbers the file's layout, so a later build can upgrade an older file. A file whose format is newer than the build is refused.
- **Limit:** a file that names a gene type this build has never heard of can't be kept as an unknown gene, since its number isn't known. It is refused, with a message naming the type.

**Registries** (chemicals, loci, brain inputs and outputs, categories, object types, gene types, brain parameters):
- Append-only, and IDs are never reused.
- A saved brain that predates newly registered brain inputs or outputs is migrated by adding neurons with neutral (zero) weights.
- Concepts are identified by their input signature, so they are unaffected.

### 2.9 Error handling

- **Expected failures return `Result` errors and never crash the sim.** That covers invalid commands (rejected with an event), malformed or invalid data and genome files, and save/load failures.
- **Internal invariant violations** are caught by `debug_assert!` and by `World::check_invariants()` in debug and test builds.
- A **panic hook** restores the terminal and flushes `last_session.replay`.

---

## 3. World and ecology

### 3.1 Grid and terrain

- The world is a fixed-size grid, **256×160 tiles** by default (configurable: each side 32–1024).
- The map's edge is the terrarium's **wall**. No step crosses it.
- Movement is 8-directional.
- Terrain properties come from `data/terrain.ron`:

| Terrain | Walkable | Step cost | Fertility | Drinkable | Allows fixtures |
|---|---|---|---|---|---|
| Grass | ✓ | 10 | 1.0 | | ✓ |
| Dirt | ✓ | 10 | 0.5 | | ✓ |
| Sand | ✓ | 15 | 0.0 | | ✓ |
| Shallow water | ✓ | 25 | 0.0 | ✓ | |
| Deep water | ✗ | – | – | | – |
| Rock | ✗ | – | – | | – |

- Every walkable terrain states all four properties; an unwalkable one states only that it isn't walkable.
- **Walkable is not the opposite of solid.** Rock is **solid**: nothing can ever move through it. Deep water isn't walkable, but it isn't solid either: crossing it is a matter of ability, and swimming is out of scope for M1 (§1.3; entity tags, [#27](https://github.com/Keazra/terra-sprites/issues/27)). In M1 both simply can't be entered.
- An orthogonal step costs the destination tile's step cost. A diagonal step costs `cost × 14 / 10`, in integer maths.
- **No corner-cutting:** a diagonal step is allowed only if both tiles beside the diagonal are walkable and hold no solid object.

### 3.2 World generation and connectivity

1. **Terrain:** seeded value noise produces height and moisture fields. It is built in-house from + − × ÷ alone (so it needs no `libm`), and its random lattice values come from the world RNG. Terrain bands are set **by percentile**, so every seed gets about the same mix: the lowest 20% of heights are deep water, then 12% shallow water, 8% sand, 50% land, and the highest 10% rock. The driest 40% of the land is dirt; the rest is grass.
2. **Regions:** a flood fill using exactly the movement rules above (**shallow water counts as walkable**) finds the walkable regions. The largest one is the **mainland**.
3. **Joining regions:**
   - Every other region of **≥64 tiles** is joined to the mainland, largest first, along the route that **carves the fewest tiles, then takes the fewest steps**. The route uses orthogonal steps only, so it never depends on a diagonal, and the connection it carves is one tile wide. Deep water becomes shallow water; any other unwalkable terrain becomes dirt. A route that crosses another region joins that region too.
   - Regions **under 64 tiles** become rock.
4. **Placement:** sprites and objects are placed on the mainland, obeying the rules in §3.3–3.4.
   - **Objects:** each object type's count comes from the preset's densities (§3.9). Solid objects are placed first, then items, each type in ID order. Each object goes on a tile drawn uniformly from the type's remaining candidates: mainland tiles where it may go. A tile found unusable is set aside for the rest of that type (one that would cut a path might become usable once a neighbour fills in, but checking again would slow generation). If the map runs out of room, generation places what fits and carries on.
   - **Generation always keeps paths open:** a solid object goes only where `KeepsPathsOpen` (§3.3) holds, so no world starts out split.
   - **Objects start partway through their lives.** For an object with stages, generation draws every stage's duration, then an age uniformly within their total, and starts the object at that point: in the stage the age falls in, with that stage's remaining time. Its counters start at 0, and no `OnStageEnter` fires for the stage it starts in. Without this, every bush would start as a seedling (no fruit for 1,500 ticks or more) and they'd all expire in the same few thousand ticks.
   - **Sprites** are placed after the objects: the preset's count (§3.9), each on a tile drawn uniformly from the walkable mainland tiles that hold no object and no sprite. Each is made from the starter genome with spawn variation (§4.9).
   - **The first population doesn't all run dry at once.** Each sprite world generation places starts with its energy and its hydration each drawn uniformly between 60% and 100% of the newborn level (§4.7). Without this, sprites that all start full would all get hungry, get thirsty, and die of neglect on the same tick.

Property tests check full connectivity across many seeds.

### 3.3 Solid objects and keeping paths open

An object is **solid** if nothing can move through it, and a **fixture** if it's attached to the ground (§3.5.1). In M1 these always go together: every solid object is a fixture, and every other object is an **item**.

**Where a solid object may stand** is physics, and nothing more: a walkable tile whose terrain allows fixtures (§3.1), holding no sprite and no object. Solid objects may stand side by side, in clumps and hedges, beside rock, water or the wall.

**Keeping paths open is the data's choice.** A solid object can cut a path: one thornbush in a one-tile corridor cuts off everything beyond it, and a diagonal line of bushes is a wall, because sprites can't cut corners. The engine doesn't forbid this everywhere. Instead, the rule vocabulary has a location condition a rule can ask for (§3.5.2):

- **`KeepsPathsOpen`** holds at a tile if a solid object there would leave **the open tiles on its four sides (N, E, S, W) still joined to one another by stepping around it**, through the 8 tiles that surround it. An open tile is walkable and holds no solid object. The tile's own contents don't matter, so a berry can ask it of its own tile before becoming a bush.

**Why it's enough:** any path through the tile enters and leaves by two of its four side tiles, and the condition says those two are joined around it, so the path can go around instead. A diagonal step past the tile is only allowed when both tiles beside it are open, and those are two of its side tiles, so it goes around too. Every rule that makes a solid object only where `KeepsPathsOpen` holds therefore **never splits the map or traps a sprite**. A property test checks this under random sequences of placements and removals.

| Case | `KeepsPathsOpen` |
|---|---|
| Beside another bush in open ground, or in a 2×2 clump | holds |
| The end of a line of bushes, or beside rock or the wall | holds, while the other sides stay joined |
| Plugging a one-tile corridor | fails: its two open sides can't reach each other around it |
| The piece that would close a loop, or carry a line to the wall | fails |

- The built-in berry sprouting and thornbush spreading ask for it (§3.5.3), and world generation always does (§3.2). A data pack that leaves it out gets objects that can wall things off; that's its author's choice.
- **The Cursor's placing asks it only if the type's data does** (v28): a type's `place` may list conditions (§3.5.1). The built-in berry bush lists none, so the player may wall things off, or pen sprites in, on purpose.
- **It only looks at the 8 tiles around, so it errs on the safe side:** it can refuse a tile whose sides would still meet the long way round. That's the price of a check that costs nothing.
- **A removal can leave a pocket.** Solid objects may fill a dead end; if the one at the end later expires, its tile can be left closed off by the others. The pocket held nothing but that object, so no sprite is ever in one, and an item that later drops there just expires.
- It's judged when an object appears. A solid object that could be pushed could later be shoved into a corridor, so it would need a rule of its own; that's another reason M1 doesn't allow one (§3.5.1).

### 3.4 Space rules

- A tile holds **at most one sprite** and **at most one object**.
- A sprite can stand on an item. It can never share a tile with a solid object.
- An **item** may go on any walkable tile that holds no object, whether or not a sprite stands there, and whatever the terrain allows for fixtures.
- Sprites interact with their own tile or an adjacent one (the 8-neighbourhood).
- Only the Cursor holds things in M1, and only items: sprites are led, never lifted (v23, §6.5).

### 3.5 Objects defined as data (`data/objects.ron`)

#### 3.5.1 Schema

| Field | Meaning |
|---|---|
| `id` | Stable `ObjectTypeId` |
| `name` | Referenced by rules and themes |
| `plural` | How the screen says the object type in general, shown as written, less any spaces at either end: `"berry bushes"` (v17). Left out for a thing you don't count, such as water, which then reads with "is" ("water is good for thirst"). The sim never reads it. |
| `category` | The name of its category (§3.5.5), which is what brains perceive (v19) |
| `tags` | The object's **tags**, e.g. `[Solid, Fixture]`. Having a tag means yes; lacking it means no; leaving the field out means no tags. `Solid`: nothing can move through it (§3.3). `Fixture`: attached to the ground, so nothing can push, pull or carry it. From slice 11b, also tags defined in the data, such as `Thorny` (§3.5.6). |
| `pseudo` | `true` for Water and Sprite: a verb table only, no instances, tags or lifecycle |
| `counters` | Named integer counters with maximums, e.g. `{"fruit": 6}` |
| `stages` | `[(name, ticks: (min, max), next: Stage(name) \| Expire)]`. The duration is drawn from the world RNG when the stage is entered. An object with no stages is permanent. |
| `rules` | `[(trigger, if: [conditions], do: [effects])]` |
| `verbs` | `{Verb: [effects]}`: what happens when a sprite applies that verb to this object |
| `size` | `Small`, `Medium` or `Large` (from slice 7b). What a rolling item does to what it meets depends on it (§3.5.4). Sprites' size is the `sprite` pseudo type's. Required on every type with instances; a pseudo type gives it or not (`sprite` does, `water` doesn't). |
| `hardness` | 0 to 1 (from slice 7b). A rolling item crushes a smaller, softer one (§3.5.4). Given with `size`, and only with it. |
| `visual` | `[(if: [conditions], state: name)]`: the first match names the visual state the theme draws. If nothing matches, the state is `"default"`. `Chance` is not allowed here, so rendering never uses the RNG. |
| `place` | `(label: "berry bush seedling", if: [conditions])` (v28): the Cursor's Place menu offers the type under that label (§6.5). `if` is optional: the conditions the tile must meet, judged for the new object at the start of its first stage, such as `KeepsPathsOpen`. `Chance` is not allowed, so placing never uses the RNG. Left out, the type can't be placed; a pseudo type can't have one. The built-in pack places the berry bush, the berry and the ball, with no conditions. |

**Glyphs and colours are not in `objects.ron`.** Themes map `(object name, visual state)` to how it looks (§6.2).

**The built-in tags are a closed set.** M1 knows `Solid` and `Fixture`, and loads only two combinations: **both** (a solid fixture, like a bush) or **neither** (an item, like a berry). Anything else is a load error. A solid object without `Fixture` would be pushable, and a fixture without `Solid` could be walked over (a floor switch); both wait for the entity-tags design ([#27](https://github.com/Keazra/terra-sprites/issues/27)). Tags belong to the object type; tags gained or lost in play are part of that design too.

#### 3.5.2 Rule vocabulary (closed)

**Triggers:**
- `Every(n)` fires when `(tick + object_id) % n == 0`, which staggers objects so they don't all fire on the same tick
- `OnStageEnter(stage)`
- `OnExpire` fires when the last stage ends. After its rules run, the object is removed unless it has already been replaced.

**Conditions:**
- `InStage(name)`
- `Counter(name, cmp, value)`
- `Chance(p)`, which draws from the world RNG
- `Fertility(cmp, f)`, which is a **location** condition
- `DensityBelow(type, radius, max)`, which is true when fewer than `max` objects of `type` are within the Chebyshev `radius`. It is a **location** condition.
- `KeepsPathsOpen`, which is true when a solid object on the tile would leave the open tiles on its four sides joined around it (§3.3). It is a **location** condition.

**Effects:**

| Effect | Behaviour |
|---|---|
| `AddCounter(name, Δ)` | Clamps to [0, max] |
| `RequireCounter(name, n)` | **In a verb only:** if the counter is below *n*, the verb **fails**. Later effects don't run, and the outcome is `failed`. |
| `SpawnNearby(type, radius)` | Creates an object on a tile within the Chebyshev radius (on the map, this object's own tile included) where the new type may go (§3.3–3.4), chosen uniformly among those candidates (one RNG draw if there's at least one). A fixed scan order would make bushes drift in one direction. Does nothing, with no draw, if there are no candidates. |
| `SpreadTo(type, radius, [conditions])` | Draws **one** tile uniformly from the (2·radius+1)² square around this object, cut down to the map (one RNG draw). If the tile passes placement and the conditions, which are evaluated *at that tile*, the object is created there. Otherwise nothing happens. The object's own tile is in the square and always fails placement; that's harmless. |
| `ReplaceWith(type)` | Replaces this object with a new one (new ID, first stage) on the same tile. This object is taken off the tile first, then placement is checked for the new type, e.g. terrain that allows fixtures and no sprite on the tile. If that fails, nothing happens and this object stays. |
| `DestroySelf` | Removes the object |
| `Inject(Actor \| Target, chemical, amount)` | Adds to a chemical. **Only physical chemicals are allowed.** Anything else is a load error. Injury it adds is put down to this object type, for the cause of death (§4.10). |
| `Signal(Actor \| Target, locus)` | Writes a pulse to the `incoming` buffer. A pulse on the Target records the Actor as its source; if several land on one sprite in a tick, the one resolved last counts. |
| `Push(max_tiles)` | Sets this item **rolling** up to `max_tiles` away from the actor, in the direction from actor to item snapped to 8 directions (if both are on the same tile, the actor's last step direction, or N if it has none). It moves from step 2 of the next tick, one tile a tick, bouncing off what it meets (§3.5.4). A push on an item already rolling starts a fresh roll. |

**Verb-only effects:** `Inject`, `Signal`, `Push` and `RequireCounter` need an actor, so they may only appear in a `verbs` table. Using one in a lifecycle rule is a **load error**.

**Evaluation semantics:**
- Step 2 covers the objects that exist when it begins, in ascending ID order, with rules in the order listed. Objects created during step 2 (by `SpawnNearby`, `SpreadTo` or `ReplaceWith`) first run their rules next tick. Objects placed by the Cursor at step 1 already exist, so they run in the same tick.
- **An object's turn,** in this order:
  1. **The stage clock.** If the current stage has run its full duration, the object enters the next stage (drawing that stage's duration), or, if it was the last stage, it is **expiring**. At most one transition happens per turn.
  2. **Its rules, in the order listed.** A rule runs if its trigger fires this turn: `Every(n)` when `(tick + object_id) % n == 0`; `OnStageEnter(s)` if the object entered `s` this turn; `OnExpire` if it's expiring. **An expiring object runs only its `OnExpire` rules.** A new object enters its first stage on its first turn, so `OnStageEnter` fires for that stage then (not for an object that world generation starts partway through a stage, §3.2).
  3. **`DestroySelf`, or a `ReplaceWith` that succeeds, ends the turn.** The object is gone, so no later effect or rule runs.
  4. An expiring object that is still there after its rules is removed.
- **Conditions are evaluated left to right and stop at the first false one.** So a `Chance` after a false condition doesn't draw from the RNG.
- **Convention:** put location conditions before `Chance`.
- Effects run in order.
- **Draws:** `Chance(p)` takes one 32-bit draw and succeeds if its top 24 bits, as a fraction of 2²⁴, are below `p`. A uniform choice among *n* candidates takes one 64-bit draw *x* and picks index `(x × n) >> 64`. Its bias is below one in 2⁵⁰, and unlike rejection sampling it always takes exactly one draw.
- `DensityBelow` counts every object of the type on a tile within the radius, including the object evaluating it if it's of that type.
- **Held objects have no tile.** Location conditions evaluate as false, and effects that need a tile (`SpawnNearby`, `SpreadTo`, `ReplaceWith`) do nothing. Stage timers, counters and `Chance` keep running. An object that expires while held empties the Cursor and emits `CursorEmptied`.
- Names of chemicals, loci and types are resolved to stable IDs when the file is loaded. An unknown name is a load error.
- **Load errors** (every one names the object type and what's wrong):
  - an unknown object type, chemical, locus, stage or counter name, or a duplicate `id` or `name`
  - a verb-only effect in a lifecycle rule
  - `Inject` of a chemical that isn't physical; `Signal` of a locus that isn't a pulse
  - `Chance` in a visual rule
  - a stage whose `ticks` minimum is below 1 or above its maximum; a stage name used twice in one type; a counter maximum of 0; `Every(0)`; a `Chance` outside [0, 1]
  - a tag combination M1 doesn't support (§3.5.1)
  - a pseudo type with tags, counters, stages, rules or visual rules; a spawn, spread, replacement or `DensityBelow` that names a pseudo type
  - a `Push` in the verb table of a type that isn't an item: a fixture, or a pseudo type (from slice 7a)
  - a pseudo type's verb that does anything but `Inject` and `Signal`: water and sprites aren't objects, so nothing else applies (from slice 7a)
  - a verb table for a verb other than Eat, Drink, Hit and Play: the others move, rest or are reserved, and never act through a target's table (§5.2)

**Scope rule:** any later object type that fits this vocabulary needs **no new code**. A genuinely new behaviour means adding one case to the rule enum. (M3 critters are agents, not objects.)

#### 3.5.3 M1 object types

```ron
(id: 1, name: "berry_bush", category: "bush", tags: [Solid, Fixture],
 counters: {"fruit": 6},
 stages: [(name: "seedling", ticks: (1500, 2500),   next: Stage("mature")),
          (name: "mature",   ticks: (20000, 30000), next: Expire)],
 rules: [
   (trigger: Every(200), if: [InStage("mature")], do: [AddCounter("fruit", 1)]),
   (trigger: Every(50),  if: [Counter("fruit", Ge, 6), Chance(0.2)],
                         do: [AddCounter("fruit", -1), SpawnNearby("berry", 1)]),
 ],
 verbs: {
   Eat: [RequireCounter("fruit", 1), AddCounter("fruit", -1),
         Inject(Actor, "food", 0.3), Signal(Actor, "ate")],
   Hit: [Signal(Actor, "did_hit")],
 },
 visual: [(if: [InStage("seedling")], state: "seedling"),
          (if: [Counter("fruit", Ge, 1)], state: "fruiting")])

(id: 2, name: "berry", category: "fruit",
 stages: [(name: "fresh", ticks: (1500, 2500), next: Expire)],
 rules: [
   (trigger: OnExpire,
    if: [Fertility(Ge, 0.5), DensityBelow("berry_bush", 4, 3), KeepsPathsOpen, Chance(0.1)],
    do: [ReplaceWith("berry_bush")]),
 ],
 verbs: { Eat: [Inject(Actor, "food", 0.3), Signal(Actor, "ate"), DestroySelf] })

(id: 3, name: "thornbush", category: "bush", tags: [Solid, Fixture],
 stages: [(name: "grown", ticks: (40000, 60000), next: Expire)],
 rules: [
   (trigger: Every(2000), if: [Chance(0.1)],
    do: [SpreadTo("thornbush", 4, [DensityBelow("thornbush", 4, 2), KeepsPathsOpen])]),
 ],
 verbs: {
   Eat:  [Inject(Actor, "injury", 0.05), Signal(Actor, "pricked")],
   Hit:  [Inject(Actor, "injury", 0.03), Signal(Actor, "pricked"), Signal(Actor, "did_hit")],
   Play: [Inject(Actor, "injury", 0.03), Signal(Actor, "pricked")],
 })

(id: 4, name: "ball", category: "toy",
 verbs: {
   Play: [Push(4), Signal(Actor, "played")],
   Hit:  [Push(2), Signal(Actor, "did_hit")],
 })

(id: 100, name: "water", category: "water", pseudo: true,
 verbs: { Drink: [Inject(Actor, "water", 0.2), Signal(Actor, "drank")] })

(id: 101, name: "sprite", category: "sprite", pseudo: true,
 verbs: {
   Hit:  [Inject(Target, "injury", 0.03), Signal(Target, "was_hit"), Signal(Actor, "did_hit")],
   Play: [Signal(Actor, "played_social"), Signal(Target, "played_social")],
 })
```

**The resulting ecology:**
- **Food:** bushes carry fruit. Overripe fruit drops as berries, which are eaten or expire. Expiring berries sometimes sprout new bushes on fertile land that isn't crowded. Food therefore has a geography, spreads, and can be overgrazed.
- **Thornbushes** give no food and spread slowly. They exist so sprites have something to learn to avoid. **Every contact verb hurts, deliberately, including Play:** thorns hurt whatever you do to them, and only Approach and Retreat are safe. Scenario A1 counts only thornbush Eats. Harm from Play also teaches sprites to pay less attention to thornbushes, which is a legitimate part of the lesson.
- **Balls** are permanent toys. A kick sets one rolling (§3.5.4).

The rule constants above are starting values, tuned with the lab runner.

**Sizes and hardness (from slice 7b):**

| Type | Size | Hardness |
|---|---|---|
| berry | Small | 0.1 |
| ball | Medium | 0.5 |
| berry_bush, thornbush | Large | 1.0 |
| sprite | Large | 0.5 |

So a ball crushes a berry, knocks another ball on, and bounces off bushes and sprites. Water has no size: shallow water is walkable terrain that a ball rolls across, and deep water bounces it.

#### 3.5.4 Rolling items

A `Push` (§3.5.2) sets an item **rolling**: it keeps a direction (one of 8) and the tiles it has left, both world state, hashed and saved (§2.8). At step 2, after the object rules, each rolling item in ascending ID order tries to move one tile. Whatever happens uses up one tile, and at none left the roll ends.

**What it meets.** The tile ahead **stops** it if it isn't walkable, is off the map, holds a sprite or a solid object, or the move would cut a corner (§3.1). Sprites and objects are taken as they stand at that moment in step 2. From slice 7b, an item on the tile ahead also decides by size and hardness:

| The item ahead is… | Result |
|---|---|
| Bigger than the rolling item | It stops the rolling item (a bounce) |
| The same size | **Knock on:** the rolling item stays put, and the two **swap rolls**. The item ahead rolls on in the rolling item's direction with the tiles it had left less this tick's, and the rolling item takes whatever roll the item ahead had, stopping if it had none |
| Smaller and softer | **Crush:** the item ahead is removed (`ObjectRemoved { reason: Destroyed }`), and the rolling item moves onto its tile |
| Smaller, and at least as hard | **Knock on,** as for the same size |

Before 7b, any item ahead stopped it. A tile holding several things uses the strongest result: a bounce over a knock-on over a crush. Since a tile holds at most one object (§3.4), that only matters for an item under a sprite, which bounces it.

**A knock-on is an exchange of momentum.** Every item rolls one tile a tick, and items of the same size weigh the same, so swapping rolls is what equal balls do:
- A ball meeting one at rest stops, and sends it on. A kick of 4 that meets a ball after 1 tile sends it 2 more; a roll on its last tile only nudges, and neither moves.
- Two balls meeting head on both bounce back, each with the other's distance.
- A glancing knock swaps whole rolls too, which a grid of 8 directions makes close enough.
- A smaller, harder item knocked on swaps as well. Momentum by weight waits for the richer-play design ([#45](https://github.com/Keazra/terra-sprites/issues/45)); no M1 object meets that case.

**Once a tick.** An item moves at most once a tick. One knocked on, or handed a roll by a knock, starts moving at the next tick's step 2, whatever its ID.

**Bouncing.** A stopped roll turns instead of ending:
- **Head on** (an orthogonal direction, or a diagonal whose corner tile alone stops it, or whose two side tiles both do): it reverses.
- **Slantwise** (a diagonal where only one side stops it): it glances off, reversing only the part of its direction that ran into the obstacle. A ball heading NE into a wall on its east goes NW; into one on its north, SE. A side counts as stopping it only if it would bounce it; an item it would knock on or crush there doesn't.
- It then meets the tile in its new direction by the usual rules: it moves there, knocks on or crushes what's there, or, if that tile would bounce it too, the roll ends where it is. Either way the bounce uses up the tick's tile.

**No harm.** Nothing a rolling item does hurts a sprite, and a sprite it bounces off feels nothing (§3.8). Bounces aren't events.

**The Cursor** picking up a rolling item ends its roll (slice 11a).

**Thrown items and shoved sprites** (v23, slice 11b; settled in v25). The player aims both by pulling back (§6.5), and the command carries the direction and how far:
- **A thrown item** leaves the Cursor rolling, as a pushed one does, by these same rules. It's put down where the aiming began, and moves from step 2 of the same tick.
- **A shoved sprite slides** as far as the shove sends it, in the shove's direction, one tile a tick. It slides at step 2, after the rolling items, in ascending ID order, and chooses nothing as it goes (§2.4): no attention, decision or trace entries, and no steps at step 6. Taken hold of by the Cursor meanwhile, it slides on to the end, and then follows the Cursor (§6.5); shoved again before the end, it starts a fresh slide, as a push on a rolling item starts a fresh roll (§3.5.2).
- **What stops a slide:** whatever would stop a rolling item, taken as it stands at that moment in step 2, but without a bounce: the slide ends where the sprite is. Deep water stops it like a wall until swimming ([#27](https://github.com/Keazra/terra-sprites/issues/27)). An item doesn't stop it: a sprite can stand on one.
- **A crash.** Stopping against a solid object or a sprite is a **crash**, a contact (§3.5.6), which hurts only if the thing's tags say so. A slide stopped by what's on the tile ahead crashes into it; one stopped from cutting a corner (§3.1), into the solid object beside it, and of two, the one to its east or west. A diagonal slide passes the corner before it can reach the tile ahead, so the corner comes first: stopped there, it never reaches what's ahead. Stopped by terrain or the map's edge, it doesn't crash. A sprite crashed into feels nothing in M1, as a sprite a ball bounces off feels nothing; a crash hurting by its force waits for [#96](https://github.com/Keazra/terra-sprites/issues/96).
- **How far:** the Cursor sends a thing up to a distance set by its size (Appendix B): small 6 tiles, medium 6, large 5. Sprites are large, so a shove goes up to 5. Size stands in for weight until things have weights ([#96](https://github.com/Keazra/terra-sprites/issues/96)).

#### 3.5.5 Categories (`data/categories.ron`) (v19)

A **category** is what a sprite perceives a thing as. Attention chooses between categories (§5.3), instincts and genomes name them, and a category's summary judges the object types in it a sprite hasn't met (§5.6).

```ron
[
    (id: 1, name: "bush",   plural: "bushes"),
    (id: 2, name: "fruit"),
    (id: 4, name: "water"),
    (id: 5, name: "toy",    plural: "toys"),
    (id: 6, name: "sprite", plural: "sprites"),
]
```

- **Grouping.** Things share a category because they look alike to a sprite, never because of what they do to it, and not by a botanist's classes: a bush and a berry are different categories, though berries grow on bushes. A sprite still has to learn what each thing does, and can be fooled by a look-alike.
- **IDs are permanent and append-only;** a retired ID is never reused, and nor is its brain input's. Category 3 was `thornbush` until thornbushes joined bush in slice 9e (v20 change 1), so 3 and input 38 stay unused.
- **`plural`** is how the screen says the category in general, as an object type's is (§3.5.1): "bushes are bad". It's left out for one you don't count, which then reads with "is": "fruit is good for hunger".
- **Each object type names one category.** A name that isn't in the list is a load error, as are a duplicate ID or name. A category no object type names is allowed.
- **`water` and `sprite` must be in the list.** Water tiles and sprites aren't objects, so no object type can say what they are, and every world has them: the engine perceives them as the categories with these two names, and a list without either is a load error. Their pseudo types' verb tables are found through them, as before (found while building slice 9d: test worlds that replace the object types have no pseudo types, and their sprites must still be perceived). One with no pseudo type has no verb table, so every try on it is fruitless (§5.2, v20).
- **Each category has a Target input,** `attended_<name>` (§5.2). Its ID follows from the category's: `35 + id` for IDs 1–6, and `37 + id` from 7, past `target_distance` and `target_adjacent`. So the list holds IDs up to 26.
- **Mods choose from this list and never add to it** (§1.4). The core game adds categories in its own updates; a world keeps the copy of the pack it was made with (§2.8), so a new category never changes an existing world.

#### 3.5.6 Tags defined in the data (`data/tags.ron`) (v23, slice 11b)

A **contact** is a sprite touching a thing: eating it, hitting it, playing with it, or crashing into it (§3.5.4). Walking past never counts (§3.8). A tag defined in the data says what each contact with a thing that has it does to the sprite making it, and every thing with the tag behaves the same way.

```ron
[
    (name: "Thorny",
     contact: {
       Eat:   [Inject(Actor, "injury", 0.05), Signal(Actor, "pricked")],
       Hit:   [Inject(Actor, "injury", 0.03), Signal(Actor, "pricked")],
       Play:  [Inject(Actor, "injury", 0.03), Signal(Actor, "pricked")],
       Crash: [Inject(Actor, "injury", 0.03), Signal(Actor, "pricked")],
     }),
]
```

- **The thornbush becomes `tags: [Solid, Fixture, Thorny]`,** and its verb table keeps only what isn't thorns: `Hit: [Signal(Actor, "did_hit")]`. The amounts are today's, so this move alone must leave every lab report identical, byte for byte; crashes come after it.
- A contact runs the tags' effects, in the order the thing lists its tags, then the thing's own verb table. **A try that a tag answers isn't fruitless** (§5.2).
- Effects are `Inject` and `Signal` on the Actor, the sprite making contact; anything else is a load error, as is an unknown tag name or a tag defined twice.
- `Solid` and `Fixture` are built in (§3.5.1), not defined here. Brains never perceive tags: they perceive categories (§3.5.5).
- **Easy modding:** a new thorny thing is one word in its entry. A mod may bring tags of its own (§1.4).

### 3.6 Perception, reachability and goal tiles

**The flood:**
- Each sprite has one **bounded Dijkstra flood**. It covers walkable tiles within its `sense_radius` (Chebyshev distance), rounded to the nearest whole tile (9.87 reaches 10), using step costs.
- Tiles holding another sprite can be crossed at a penalty of 30, three grass steps (set in `physiology.ron`), since sprites move. So a sprite detours up to about 3 tiles to go around another, and beyond that walks up and waits (§3.7).
- The flood is recomputed when the sprite enters a new tile, or 8 ticks after its last one.
- It provides both distances and paths. **No separate A* is needed.**

**Goal tiles:**

| Target | Goal tiles |
|---|---|
| Solid object | Any walkable neighbour |
| Item | Its own tile or any neighbour. A rolling item's are re-planned whenever it moves, as a sprite's are. |
| Water | The shallow-water tile itself or any neighbour |
| Sprite | Any neighbour. The path is re-planned whenever the target moves. |

**Re-planning uses the cached flood.** When a target moves, the new path is traced back through the existing flood's predecessor grid, which costs no new Dijkstra run. A full re-flood happens only on the refresh rules above (the sprite changes tile, or 8 ticks pass). If the target leaves the flood's area, it's no longer reachable, and the action ends (§5.5).

**Candidates:**
- For each category, the candidate is the reachable thing that **draws the eye most** (v19, §5.3): the one whose nearness, worth and newness, and for a sprite how frightening it is while near, add up highest, ties to the lower entity ID. Among things of one object type, that's the **nearest**: the one with the lowest `(path cost, entity ID)`. For Water, the tile index `y × width + x` stands in for the entity ID.
- An instance is reachable if the flood reached one of its goal tiles. **A closer instance that can't be reached is never a candidate.**
- The Sprite candidate (v18) is the reachable other sprite that **draws the eye most** (§5.3): nearness, what it's worth to the sprite, and how frightening it is while near, ties to the lower entity ID. **While a `was_hit` pulse is live, it's the attacker instead** (the pulse's source), provided the attacker is reachable.

**Cost:** the flood covers at most (2r+1)² tiles (841 when r = 14). It's the main cost per sprite, and one of the first things benchmarked.

### 3.7 Movement timing and conflicts

- **Move points:** a moving sprite gains `speed` points per tick (the Trait gene, 4–12). When its points reach the cost of its next step, it tries the step.
  - **Counted in tenths,** so speed keeps its decimals: a sprite gains `round(speed × 10)` tenths a tick, and a step costs its §3.1 cost × 10 (an orthogonal grass step, 100). A sprite of speed 7.4 gains 74 a tick, so it walks a little faster than one of 7.0, and it pays energy for 7.4 (§4.8).
- **Carry-over:** after a step, leftover points carry over, capped at one step's cost. A blocked sprite banks points only up to the cost of its next step.
  - Example: speed 5 on grass is one orthogonal step every 2 ticks.
- **Conflicts:**
  - Steps are attempted in step 6's per-tick shuffled order, and **the first sprite to claim a tile gets it**.
  - A sprite can't enter a tile whose occupant hasn't moved yet, **except in a head-on swap:**
    - Say A, when its turn comes, tries to step into B's tile. B's next planned step is into A's tile, and B has enough move points for it.
    - Both move and both spend their points. B counts as having moved this tick.
    - Diagonal swaps must obey the no-corner-cutting rule for both sprites.
  - Swaps clear the common case of two sprites meeting in a one-tile corridor. Such corridors exist: the connections carved in §3.2 are one tile wide.
- **Blocked re-planning:**
  - **Trigger:** 3 consecutive blocked ticks.
  - **The search:** a separate, one-off bounded Dijkstra from the sprite, with the same radius and step costs as the perception flood (§3.6), except that **tiles holding sprites can't be entered**. It looks for a path to the action's goal tiles, or to the Wander destination. It doesn't replace the perception flood, isn't used for perception, and isn't cached.
  - **Path found:** the path becomes the action's **committed path**, stored with the action and saved (§2.8). The sprite follows it step by step, and regular flood refreshes **don't override it**. Without this, the next refresh (which allows occupied tiles at a penalty) could route the sprite straight back into the blocked tile.
  - **Committed path dropped:** if the sprite is blocked again, the 3-tick count and the search start over. If the action's target moves (a sprite, or a rolling item, §3.5.4), the path is dropped and normal flood-based planning resumes. The path also ends with the action.
  - **A blocked tick** is one where the sprite had the points for its next step but couldn't take it: a sprite stood there, or the step has become impossible (a bush grew since the flood).
- **Several steps a tick:** a sprite takes as many steps in a tick as its points pay for, so speed 12 on grass is 1.2 steps a tick. A swap ends both sprites' walking for the tick.
  - **No path found** (for example, a sprite resting in the only corridor, or on the Wander destination itself): **the action ends with outcome `blocked`**, freeing the brain to choose again.
  - **Retreat** doesn't use the search. It moves by straight-line distance (§5.5), so for Retreat "no path" means no free neighbour increases the Chebyshev distance from the target. If walls, water or solid objects are all that's in the way, the sprite is **cornered** at once. If sprites stand on every neighbour that would increase it, that's a blocked tick, and after `replan_after` of them in a row it's cornered too. A cornered retreat ends `blocked` and fires the `cornered` pulse (§4.2).
  - **Cost:** the search only runs after 3 blocked ticks, which is rare, so it doesn't threaten the §3.9 performance target.
- **Why a seeded shuffle and not ID order:** both are deterministic. But ID order would favour older sprites in every contest, and M2 evolution would pick up that bias.
- **Energy:** each step costs energy under physiology rules (§4.8).
- **Stamina doesn't stop walking.** Each step drains stamina and rest restores it (§4.4), but a sprite with none left still walks. Low stamina only raises tiredness (§4.5), which the brain can answer by resting.

### 3.8 No incidental harm

Harm only ever comes from **explicit verbs** aimed at an object. There is no damage from bumping into things or passing near them, and none from a rolling item: a ball that bounces off a sprite doesn't hurt it (§3.5.4).

This keeps credit assignment clean. If thorns scratched sprites walking past, the pain would be credited to whatever the sprite was doing at the time, usually "approach food". Sprites would learn the wrong lesson.

**A crash** (v23, slice 11b) is the one harm that doesn't follow a sprite's own verb: it follows the player's shove (§3.5.4). It's a contact, so it's credited cleanly, to the thing crashed into (§5.6). It harms only the sprite that slid, and only if the thing's tags say so (v25).

### 3.9 Defaults and performance

- **Defaults:** 256×160 map; 30 starter sprites (configurable, 20–100).
- **The sprite count is a plain number** in the preset, `sprites: 30`, from 20 to 100. It isn't a density: the population is a design target, not ecology, so it stays the same on any map size. A preset that leaves it out gets no sprites, as a type it doesn't name gets no objects.
- **Plants and toys are set by density**, so a bigger map gets proportionally more and food stays as easy to find: about 150 berry bushes, 40 thornbushes and 6 balls per 15,360 tiles (a 160×96 area). On the default map that is about 400 berry bushes, 107 thornbushes and 16 balls. The sprite count doesn't scale; it stays in the 20–100 range.
- **In the preset**, densities are counts per area:

  ```ron
  objects: {"berry_bush": 150, "thornbush": 40, "ball": 6},
  per_tiles: 15360,
  ```

  A map of *t* tiles gets `(n × t + per_tiles / 2) / per_tiles` objects of a type with count *n*, in integer maths (rounding halves up): exactly 400, 107 and 16 on the default map. Each name must be an object type in the data pack that isn't a pseudo type, so the preset is parsed against the pack (§2.2). A type the preset doesn't name gets none.
- **Performance target:** **≥200 ticks per second with 100 sprites** in a release build.

---

## 4. Biochemistry

### 4.1 The central split

> **The world decides what happens to the body. The genome decides how that feels.**

| Class | Chemicals | Changed by | Can evolve? |
|---|---|---|---|
| **Physical** | `energy`, `hydration`, `stamina`, `food` (gut), `water` (gut), `injury` | Physiology (`physiology.ron`) and object verbs **only** | No |
| **Signal** | Drives: `hunger`, `thirst`, `pain`, `tiredness`, `boredom`, `loneliness`, `crowdedness`. Learning signals: `reward`, `punishment` | Genome, and the Cursor | Yes |
| **Hormone** | `h0`–`h15`, unnamed | Genome | Yes. These are spare channels evolution can put to use. |

- Chemicals are listed in `data/chemicals.ron` with stable IDs (Appendix A), one entry each, `(id: 1, name: "energy", class: Physical)`. The hormones are listed one by one, `h0` to `h15`. IDs and names are unique.
- Concentrations are `f32` values in [0, 1], one array per sprite.
- If physiology were evolvable, M2 evolution would simply remove the costs, so this split is fixed now in M1.

### 4.2 Loci

Loci are listed in `data/loci.ron` with stable IDs. Each has a kind: `(id: 32, name: "ate", kind: Pulse)`. Which loci the brain feels is listed in `brain_io.ron` (§5.2), not here. The kinds are `BodySensor`, `Pulse` and `ReceptorTarget`. Chemical levels aren't listed: they're referenced as `Chem(id)`. IDs and names are unique.

| Kind | Loci | Brain-visible |
|---|---|---|
| **Chemical level** | Any chemical, referenced as `Chem(id)`. Genes can read physical chemicals too. | Drives and hormones only (§5.2) |
| **Body sensors** (physiology fills these in every tick) | `always` (= 1) | ✓ |
| | `age` (normalized by lifespan) | ✓ |
| | `nearby_sprites` (sprites within 3 tiles ÷ 4, capped at 1) | ✓ |
| | `moving` (1 if the sprite stepped during the previous tick), `resting` (1 while a Rest action is running) | ✗ physiology only: they reflect the current action every tick, so as brain inputs they would loop output back into input |
| **Event pulses** (one tick, latched, §2.4) | `ate`, `drank`, `played`, `played_social`, `pricked`, `was_hit`, `did_hit`, `petted`, `shocked`, `cornered`, `fruitless` (v16: a try that did nothing, §5.2) | ✓ |
| **Receptor targets** (written by receptors) | `learning_rate_mod` (0.5–2×), `exploration_mod`, `curiosity_mod` (v16, 0–2×, §5.3) | ✗ |

- **The pulse latch:**
  - Writes go to `incoming`: at step 1 (the Cursor) and at step 6 (verb effects, and `cornered` when a retreat finds no step away, §3.7).
  - At step 3a, `live` is replaced by `incoming`, and `incoming` is emptied.
  - Reads use `live`: emitters at step 3, and the brain at step 5.
- **Pulse sources:** a pulse a verb writes on its target carries the actor as its source, so `was_hit` records the **attacker**. It's latched with the pulse and lasts as long; if two land in a tick, the one resolved last counts, and a pulse a sprite's own verb writes on it has no source, so it clears one. Sources are world state, hashed and saved.
- **Modulator neutral points:** `learning_rate_mod`, `exploration_mod` and `curiosity_mod` rest at 1.0 when no receptor writes them.

### 4.3 Gene types for biochemistry

| ID | Gene | Semantics |
|---|---|---|
| 1 | `HalfLife(chem, ticks)` | Exponential decay. The per-tick factor `0.5^(1/ticks)` is computed once at decode time with `libm`. |
| 2 | `Reaction(reactants ≤2 → products ≤2, rate)` | Extent = rate × min over reactants of (concentration ÷ coefficient). Each reactant loses coefficient × extent and each product gains coefficient × extent. An empty slot means "nothing". |
| 3 | `Emitter(locus, mode, invert, threshold, gain, chem)` | Mode **Level**: signal = the locus value, or `1 − value` if `invert` is set. **Rise** / **Fall**: signal = the locus's positive / negative change since the previous tick. Emits `gain × max(0, signal − threshold)`. Gain may be negative, which removes chemical. |
| 4 | `Receptor(chem, threshold, gain, target_locus)` | Each tick, `target = clamp(1.0 + Σ over receptors aimed at it of gain × max(0, level − threshold), target range)`. 1.0 is the neutral point. |
| 5 | `InitialConcentration(chem, value)` | The starting level of a chemical at birth |
| 6 | `Trait(trait_id, value)` | An evolvable body trait. Clamped to a range set by physiology, with a cost set by physiology (§4.8). |

**Restrictions**, which close every route by which genes could change physical chemistry:

| Gene | Physical chemicals | Signal chemicals and hormones |
|---|---|---|
| `HalfLife` | ✗ (decay rates are fixed physiology) | ✓ |
| `Emitter`: chemical written | ✗ | ✓ |
| `Emitter`: locus read | ✓ (reading only) | ✓ |
| `InitialConcentration` | ✗ (newborn physical levels are fixed physiology) | ✓ |
| `Reaction` | **Catalyst only:** identical coefficients on both sides, e.g. `hunger + food → food` | ✓ |
| `Receptor` | Reads any chemical; writes only receptor-target loci | — |

A gene that breaks these restrictions is **flagged when decoded, not expressed**, and marked in the genome viewer.

**Duplicate genes.** Some genes set one value; the rest add up.
- **Genes that set one value:** `HalfLife` and `InitialConcentration` (per chemical), `Trait` (per trait), `BrainParam` (per parameter), `Instinct` (per concept signature and verb) and `AttentionInstinct` (per input and category). The first such gene in genome order is expressed. A later one that sets the same value is **unexpressed**, and marked as such in the genome viewer.
- **Genes that add up:** `Reaction`, `Emitter` and `Receptor`. Every one of them applies.

So a gene can be marked in four ways: **flagged** (it breaks the restrictions), **unexpressed** (an earlier gene sets the same value), **unknown** (this build can't read it, §2.8) or **unmatched** (v19: it names a category this world doesn't have, §5.7).

**The starter genome must be clean.** A flagged, unknown or unmatched (v19, §5.7) gene in the data pack's starter genome is a **load error**, because it would silently do nothing. Any other genome just has the gene marked.

`reward` and `punishment` are consumed by learning every tick (§5.6), so a `HalfLife` gene on them is valid but has no effect.

**Value fields.** Every gene type declares which fields are **values**. Values are the only fields spawn variation (§4.9) changes, and the only ones M2's point mutation will change. Everything else is **structural** and never varies: IDs, reaction coefficients, modes, flags, signatures. Varying a structural field could turn a valid catalyst into one that breaks the physical-chemistry rule, or point a gene at a different chemical.

| Gene type | Value fields |
|---|---|
| 1 `HalfLife` | `ticks` (integer, ≥ 1) |
| 2 `Reaction` | `rate` |
| 3 `Emitter` | `threshold`, `gain` |
| 4 `Receptor` | `threshold`, `gain` |
| 5 `InitialConcentration` | `value` |
| 6 `Trait` | `value` |
| 7 `BrainParam` | `value` |
| 8 `Instinct` | `weight` |
| 9 `AttentionInstinct` | `weight` |

If M2 adds structural mutation, every mutated gene must be validated again against the restrictions above.

**Value ranges,** checked when a genome is read (a gene outside them is an error in the file, not a flagged gene):
- `HalfLife` ticks: at least 1.
- `Reaction` rate: 0 to 1, so a reaction never uses more than its reactants hold. A reaction needs one or two reactants, since they set its extent, and at most two products. Each coefficient is at least 1.
- Thresholds: at least 0. They can be above 1, because some loci are: receptor targets reach 4.
- `InitialConcentration` value: 0 to 1.
- Gains and trait values: any number. A trait is clamped to its range when expressed.

Spawn variation keeps each value within its range after varying it.

### 4.4 Inside step 3

For each sprite:
1. **(a) Pulse latch** (§4.2).
2. **(b) Physiology** (`physiology.ron`, fixed, not evolvable):
   - basal metabolism, plus the cost of `sense_radius`
   - energy spent on steps taken last tick, which depends on `speed`
   - digestion (`food → energy`, `water → hydration`)
   - hydration loss
   - stamina drains per step and recovers while idle (faster while `resting`)
   - injury heals slowly
   - starvation, dehydration and old age add injury (§4.10)
3. **(c) Reactions**, in genome order.
4. **(d) Half-life decay.**
5. **(e) Emitters, in two passes.** Both run **after** reactions.
   - **(e1) Level emitters** run in genome order.
   - **(e2) Rise/Fall emitters** run in genome order. Their "change" is the locus value now (after reactions, decay, e1, and any earlier e2 emitter) minus its value at the end of the previous tick's step 3.
   - Each emitter sees the effects of every emitter before it. This guarantees that a drive knocked down by a pulse-keyed Level emitter produces its Fall reward **in the same tick**, ready for step 4.
   - A chain of Rise/Fall emitters, such as injury → pain → punishment, works within one tick if its genes are in chain order. The starter genome orders them that way; evolution can reorder them.
6. **(f) Receptors** update the modulators.

**No level ever leaves [0, 1], even partway through.** Physiology clamps its chemicals once it's done: it works on them below 0 only to tell that energy or hydration has run out. Each reaction and each emitter then clamps what it writes, and decay can't leave the range. So no gene ever reads a level outside [0, 1], and a Rise or Fall emitter only sees a change that really happened: a drive at 0 that a relief emitter pushes down stays at 0, and releases no reward.

Then **death check #1** runs (§2.4).

### 4.5 Starter-genome pathways

**Principle: sharp relief for one-shot actions.**
- A drive relieved by a one-shot action (eat, drink, play) falls **in a single tick**, triggered by that action's event pulse.
- **Reward comes from the drive's fall.** It therefore arrives as one burst on the tick after the action, and it depends on need: a full sprite's hunger can't fall far, so eating when full earns little.
- Relief is gradual only where the relieving action is itself **ongoing** (Rest, staying near others). The reward then lands on the trace entries of that same ongoing action, which is where it belongs.
- v1 relieved hunger gradually during digestion, which dripped reward onto whatever the sprite did next. (Rewarding the `ate` pulse directly would fix the drip, but would reward eating even when full.)

**How a sprite learns that berries are good:**
1. Low `energy` → an inverted Level emitter (threshold 0.5) raises `hunger`.
2. Hunger is a brain input. The sprite decides *Eat berry bush* at tick *t*.
3. The verb injects `food` 0.3 and pulses `ate` (step 6 of *t*).
4. At tick *t+1*:
   - the pulse is latched, and physiology starts digesting food into energy over the following ticks
   - **(e1)** a negative-gain Level emitter on the `ate` pulse drops `hunger` sharply, in this one tick
   - step 4 reads hunger's fall as **hunger relief** (v16) and credits it to the thing touched, the berry bush eaten from: the bush grows "good for hunger"
5. If energy is still below 0.5 once digestion has run, hunger creeps back up, and the sprite is likely to eat again.

**The other drives:**

| Drive | Rises from | Falls from (relief) |
|---|---|---|
| `thirst` | Low `hydration` (inverted Level) | `drank` pulse: sharp, one tick |
| `tiredness` | Low `stamina` (inverted Level) | A negative-gain Level emitter on `stamina`: gradual, as Rest restores stamina |
| `boredom` | `always` (slowly) | `played` and `played_social` pulses: sharp, one tick. **Social play relieves boredom as well as loneliness, but much less than kicking a ball** (0.15 to a kick's 0.5). |
| `loneliness` | Low `nearby_sprites` (inverted Level) | `played_social` pulse (sharp); high `nearby_sprites` (gradual, while staying near others) |
| `crowdedness` | High `nearby_sprites` (Level, with threshold) | Low `nearby_sprites` (inverted Level, strong negative gain): falls within a few ticks of leaving the crowd |
| `pain` | A **Rise** emitter on `injury`, and the `pricked` / `was_hit` pulses | Short half-life |

**Learning signals** (v16):
- **Relief is read from the needs themselves** (§5.6): each need's fall of at least `relief_deadband` (0.02 per tick) is its relief, so slow drifts never teach. Gradual relief pathways are set fast enough to clear the deadband. The starter genome no longer has v15's Fall emitters from each need into `reward`, which would count relief twice.
- `reward` is the general good channel: the Cursor's pets and hugs, and anything a genome adds.
- A Rise emitter on `pain` releases `punishment`. So a zap or a shock, which injects pain as well as punishment (§4.6), feels a little worse than its punishment alone.
- Emitter genes are ordered so that each chain completes within one tick (§4.4).
- Learning **consumes** `reward` and `punishment` every tick (§5.6), so they never linger.

**Moods and modulators** (v16):
- **Need makes a sprite restless.** Receptors on hunger, thirst and boredom raise `exploration_mod`, so a pressing need makes attention and choice less sure: a thirsty sprite that doesn't know where water is tries more things. Tiredness doesn't, since a tired sprite rests, and loneliness and crowdedness already draw its eye to sprites.
- **Wariness.** A Level emitter on `pain` feeds hormone `h0`, which decays over a few hundred ticks, and a negative-gain receptor on `h0` lowers `curiosity_mod`. A run of hurts makes a sprite wary of the unfamiliar for a while. The hormone is unnamed in the data; the starter genome's comment names it.
- Both are genes, so they vary and can evolve. Their values are tuned in slice 9.

v1's catalytic reactions (`hunger + food → food`, `thirst + water → water`) are no longer in the starter genome. They're still valid genes, and evolution may rediscover them.

Only hunger, thirst, tiredness and pain are tied to physical need in M1. Whether evolution keeps boredom and loneliness as useful drives is an M2 experiment.

### 4.6 The Cursor's touch

- **Reward** injects `reward` directly: a **pet** 0.5, or amplified, a **hug** 1.0 (v21).
- **Correct** injects `punishment` and `pain` directly: a **zap** 0.5 and 0.3, or amplified, a **shock** 1.0 and 0.6 (v21).
- **Neither injures.** However many times a sprite is shocked, it can't die of it: the Cursor teaches and never harms.
- All four skip the genome, with amounts set in `physiology.ron`, so the player's teaching tools work on every sprite. The amounts are a starting point for the lab's strength sweep (§7.1).
- Each also pulses `petted` or `shocked`, so a genome can add its own reactions on top. The pulses name what the sprite *senses*; the `reward` chemical rises from other causes too (eating, play). A pet and a hug pulse the same `petted`, and a zap and a shock the same `shocked`.
- **A sprite can't tell where the touch came from** while the Cursor is invisible, which in M1 it always is (§0, §5.6).

### 4.7 Newborn state

- Physical chemicals start at fixed levels from `physiology.ron`. The one exception is the first population, which world generation places with its energy and hydration drawn between 60% and 100% of those levels (§3.2).
- Signal chemicals and hormones start at their `InitialConcentration` genes, or 0.

### 4.8 Traits and their costs

| Trait | Range (`physiology.ron`) | Physical cost |
|---|---|---|
| `speed` | 4–12 move points per tick, counted in tenths (§3.7) | Energy per step ∝ (speed / 8)² |
| `sense_radius` | 6–14 tiles | Basal metabolism grows linearly with the radius |
| `lifespan` | 20,000–200,000 ticks | Once exceeded, old age adds `injury` every tick |

The starter genome's traits are speed 7, sense_radius 10 and lifespan 60,000 ticks (about 13 hours at 1×, 50 minutes at 16×). A genome with no `Trait` gene for a trait gets the middle of its range.

### 4.9 Spawn variation

A sprite spawned from the starter genome (`SpawnSprite { genome: None }`, or the initial population) has **each value field (§4.3) multiplied by a uniform factor in [0.9, 1.1]**, drawn from the world RNG, then clamped to its range. Fields are processed in genome order, with one draw per value field. Integer values, such as `HalfLife` ticks, pool size and arity, are rounded afterwards. Structural fields are never touched.

### 4.10 Death

- **There is one route to death: `injury ≥ 1.0`.** Starvation (energy at 0), dehydration (hydration at 0) and old age (age past lifespan) all add injury every tick.
- **The sources of injury:**
  - **Physiology's three,** built in: starvation, dehydration and old age.
  - **Hurt by *object type*:** injury an object verb injects (§3.5.2) is put down to the object type whose verb table did it. A thornbush's `Eat` hurts the eater, so that is "hurt by thornbush"; a sprite's `Hit` hurts the target, so that is "hurt by sprite". A new harmful object type needs no code.
- **Cause of death: a fading tally.** Each source keeps a running total of the injury it has added, and every total halves about every 350 ticks (the fade time is in `physiology.ron`), so recent injury counts most. `Died.cause` is the source with the biggest total. This needs one number per source per sprite, where an exact window of the last 500 ticks would need 500.
- **Healing** lowers `injury` but not the tallies. They only fade.
- **A hurt** (v21) is injury *or pain* that something did to a sprite. A zap or a shock hurts but adds no injury, so it never counts towards a cause of death.

---

## 5. Brain

### 5.1 Overview

The brain works the way a fruit fly's does, not the way its parts are laid out (v16). Two pathways compete for every choice:

- **Instinct**, which the genome builds and which never changes in a sprite's life. The starter genome's instincts are very basic: they connect needs to actions and know the sprite's own kind, but know nothing about food, water or toys (§5.8).
- **Learning**, which starts blank. It learns what each object type is **worth**, and a smaller set of **habits** (an object type × a verb). Learning can outvote an instinct but never erase it.

```
 STATE inputs (stable InputIds)       ATTENTION                              DECISION
 drives, hormones, body sensors ──►  instinct links (State × category)  ──►  instinct: concepts × verbs
 (nearby_sprites, age, always),      + salience for nearness                 + habit (attended thing × verb)
 event pulses                        + worth of each thing                   + worth of the attended thing
                                     + curiosity about unfamiliar things     ──► verb + target
                                           │ picks target
                                           └──► TARGET inputs: attended category (one-hot),
                                                target distance, target adjacent ──► concepts only

 LEARNING (step 4): each need's relief, `reward` and `punishment` ──► worth of the thing touched;
                    good less bad, and disappointment ──► habits, along the trace
```

- **Worth is remembered per need.** Relieving hunger teaches "good for hunger", and that worth counts in proportion to how hungry the sprite is. Pets, shocks and hurts are general channels that always count.
- **Good and bad are separate channels,** as the fly's reward and punishment neurons are. Bad lessons are learned faster and fade more slowly by default, and genes can change both.
- **Worth goes to the thing touched** when the feeling arrives; habits are credited along the trace (§5.6).
- **Individuals** (v18): sprites are remembered one by one, and sprites in general are a summary of those a sprite knows. **Fear**, of whoever hurt it, is kept apart from badness: a bad thing is avoided, a frightening one watched and backed away from while near.
- **Categories and object types** (v19): a sprite perceives **categories**, and learns about **object types**. Attention chooses a category, and within it the thing that draws the eye most; instincts and genomes name only categories. What a sprite learns is about the object type it touched, and a category's **summary** of the types it knows judges one it hasn't met, as sprites in general judges a stranger. So a new fruit is judged by the fruit a sprite already knows, once it knows a few, and still becomes its own thing through experience.

### 5.2 I/O registry (`data/brain_io.ron`)

Every input and output has a stable, append-only ID (Appendix A). Each input belongs to a **group**:

| Group | Inputs | Attention | Concepts |
|---|---|---|---|
| **State** (37) | 7 drives, 16 hormones, `nearby_sprites`, `age`, `always`, 11 event pulses | ✓ | ✓ |
| **Target** (one per category, plus 2) | `attended_<category>` for each category (v19, §3.5.5): `attended_bush`, `attended_fruit`, `attended_water`, `attended_toy`, `attended_sprite`; `TargetDistance` (normalized path cost: the cost over 10 × the flood's reach, capped at 1), `TargetAdjacent` | ✗ | ✓ |

- **Target inputs are outputs of attention.** They never feed back into attention.
- **What the file holds:** the State inputs, each with its stable input ID and the chemical or locus it reads, e.g. `(id: 1, name: "hunger", reads: Chem("hunger"))`, `(id: 27, name: "ate", reads: Locus("ate"))`. A name that isn't a chemical or locus of the right kind (a drive or hormone; a body sensor or pulse) is a load error, as is a duplicate ID or name. The verbs, `target_distance` and `target_adjacent` are fixed in code, which gives them their meaning; the attended inputs follow the category list (v19, §3.5.5). Target inputs take IDs 36 to 63 (36, 37 and 39–43 so far: 38 was `attended_thornbush`, retired with its category in v20), and State inputs any other ID from 1: 1–35, then from 64 (from slice 7c). A State input numbered 36 to 63 is a load error. Their names, used in genome files, are `attended_<category>` (`attended_bush` … `attended_sprite`), `target_distance` and `target_adjacent`, and the verb names.
- **The needs** (v16): the file also lists which drives are **needs**, the ones whose relief teaches worth: `needs: ["hunger", "thirst", "tiredness", "boredom", "loneliness", "crowdedness"]`. Each must be a drive that is a State input; anything else is a load error. Pain is a drive but not a need: its fall is a hurt fading, not relief. The data names the channels, so a pack with a new drive adds a channel with no code.
- **The first-order needs** (v21): `first_order: ["hunger", "thirst"]`, the needs that come before a sprite's likes. While one presses, what the sprite merely likes pulls less (§5.6). Each must be a need; the others are second-order. A pecking order between the second-order needs themselves waits for [#50](https://github.com/Keazra/terra-sprites/issues/50).
- **Not brain inputs:**
  - physical chemicals, because the brain feels the body only through drives
  - `reward` and `punishment`, because they're learning signals
  - `moving` and `resting`, because they're physiology only

**Outputs (verbs):**

| Kind | Verbs | Available when |
|---|---|---|
| **Movement** | Approach, Retreat | Any attended target, whatever its verb table says; Approach only while the sprite isn't on one of the target's goal tiles, where it would do nothing (v15) |
| **Interaction** | Eat, Drink, Hit, Play | **Any attended target** (v16): a sprite can try anything on anything it can reach |
| **Targetless** | Rest, Wander | Always |

`Mate` (M2) and `Speak` (M4) have IDs reserved.

**Trying anything** (v16). A verb table says what an attempt *does*, not what is *allowed*. A try whose target has no verb table (water or a sprite in a pack with no pseudo type for it, v20), whose verb table has no rule for the verb, or whose rule's requirement isn't met (a bush with no fruit), is a **fruitless try**: nothing happens, the action ends `failed`, and the sprite feels a `fruitless` pulse (§4.2). Eating a ball, drinking from a bush or eating another sprite are fruitless tries; for now a sprite that is tried on doesn't notice (being nibbled belongs with social play, [#46](https://github.com/Keazra/terra-sprites/issues/46)). An object's data may also give a verb an unintended effect, such as a device that sparks when hit; effects that reach others nearby need an area effect of their own ([#61](https://github.com/Keazra/terra-sprites/issues/61)).

**Why interaction verbs walk there themselves, and what Approach is for.**

Interaction verbs include the walk, so one decision leads straight to one outcome.

If Eat required being adjacent first, eating would take two decisions, *Approach* then *Eat*, and a habit would only follow the second. The approach step would get credit only through the fading trace. Learning would be slower and more fragile, and the A1–A3 criteria depend on credit reaching the action directly.

Approach still has its own jobs:
- **Closeness.** Loneliness falls as `nearby_sprites` rises, without having to touch anyone.
- **Following** another sprite.
- **Going towards what is worth it** (v16): a thing's worth adds to Approach as well as to the interactions.
- **A counterpart to Retreat,** which M3 (hunting, herding) and M4 ("come here") will build on.

**Stored per brain:** each brain stores its index → ID tables. Concepts are identified by **input signature**, never by index; worth and habits by object type ID (v19) and verb ID, and remembered sprites by entity ID.

### 5.3 Attention (step 5a)

- **Candidates** are each category's reachable thing that draws the eye most (§3.6). With no candidates, attention is empty, all Target inputs are 0, and only targetless verbs are available.
- **Score per category** (v16):

  `score_c = Σᵢ xᵢ·A[i][c] + salience_gain × (1 − distance_c) + value_gain × value_c + curiosity × novelty_c × boldness + vigilance × fear_c`

  - *i* ranges over **State** inputs only, and `A` holds the attention instincts, which never learn.
  - `distance_c` is that candidate's own normalized path cost.
  - `value_c` is what the candidate is worth to the sprite now (§5.6): its object type's worth, or, for a type the sprite doesn't know yet, its category's summary (v19).
  - `novelty_c = 1 − familiarity_c` (§5.6), the candidate's object type's (v19), and `boldness = max(0, 1 + new_things) × curiosity_mod`, where `new_things` is the learned worth of the unfamiliar and `curiosity_mod` a receptor target that wariness lowers (§4.2, §4.5).
  - `fear_c` (v18) is how frightening the candidate is while near: `−F × max(0, 1 − distance_c / fear_reach)`, where `F` (−1 to 0) is what the sprite learned about that sprite, or the summary for one it doesn't remember (§5.6). It is 0 for anything but sprites in M1. It counts even while a hit is felt or the sprite is cornered, so the sprite keeps watching whoever did it; only fear's pull on the decision is quiet then (§5.5). Measured, quieting it here too let a cornered sprite's eye drift to a stranger, and it backed away from the stranger nearly as often as from its bully. A bad thing draws the eye *less* (its worth is negative); a frightening one draws it *more*.
  - **The candidate's own terms** (v19; for sprites since v18): `distance_c`, `value_c`, `novelty_c` and `fear_c` are the candidate's (§3.6), the reachable thing in the category whose nearness, worth, newness and fear terms add up highest. In a category with one object type, sprites aside, that's its nearest instance, as before v19.
- **With no current action:** attention **samples** a category with softmax(score / τ_att), where **τ_att = τ_att_base × `exploration_mod`**.
- **With any action still running**, targetless ones included: attention is **noise-free**. It changes target only if a rival's score beats the current target's by `attention_margin`. Ties go to the lower `CategoryId`. That switch ends a target-bound action.
- **An action keeps its target.** A target-bound verb is aimed at its category's candidate as it was when the verb was chosen, and stays aimed at that instance for the whole action. A closer instance of the same category appearing later doesn't retarget it; the Target inputs describe the instance the action is aimed at.

### 5.4 Concept lobe

- **Activation:** the **product** of a concept's inputs. Each input may be negated, turning `x` into `1 − x`. This gives a fuzzy AND with NOT.
- **Concepts feed the instinct pathway only** (v16). Learning is about object types and verbs, not concepts (§5.6).
- **Two kinds of concept:**
  1. **Singletons.** There's always exactly one for every input, and the genome can't remove them.
  2. **Innate conjunctions** of 2–3 inputs, created by `Instinct` genes.
- **The recruitable pool is on hold** (v16). Slice 9 measured it, recruited on reward (v15's design) and as a fly-style random pool, and neither changed behaviour: nothing in the world yet rewards a combination differently from its parts ([#10](https://github.com/Keazra/terra-sprites/issues/10)). It returns, measured again, once the world has such situations. Its brain parameters keep their IDs and do nothing.

### 5.5 Decision and action lifecycle

**Step 5 runs in this order:**
1. **5.0 Check the current action.** It ends as `failed` if its target no longer exists or has become unreachable (a retreat's, only if it no longer exists), and as `timed_out` if the action has hit the timeout. A Wander whose destination the flood no longer reaches ends as `failed`, unless the sprite is keeping to a committed path (§3.7).
2. **5a Attention** (§5.3).
3. **5b Decision:**
   - **Score** each *available* verb (v16):

     `s_v = Σₖ aₖ·W[k][v] + H[c][v] + side_v × value_gain × value_c − shy_v × value_gain × fear_c + back_v × flight × fear_c`

     where `W` holds the instinct links (concepts × verbs, which never learn), `c` is the attended thing (the candidate of the attended category, or the target aimed at), `H[c][v]` the habit for doing `v` to its object type (v19; for sprites, to sprites), `value_c` its worth now (§5.6) and `fear_c` how frightening it is while near (§5.3). `side_v` is +1 for Approach and the interactions, and 0 for Retreat, Rest and Wander (v18: badness no longer backs away); `shy_v` is 1 for Approach, Eat, Drink and Play; `back_v` is 1 for Retreat. With nothing attended, the habit, worth and fear terms are 0. So a thing worth having draws the sprite to it, a bad one is left alone, and a frightening one nearby is backed away from and not gone near. **Fear never adds to Hit** (v18, measured: it started feuds); hitting back is the instinct's. While a `was_hit` or `cornered` pulse is live, `fear_c` is 0 here: what the sprite does in those moments is instinct's, though fear still catches its eye (§5.3).
   - **With no current action:** sample a verb from softmax(s / τ), where **τ = τ_base × `exploration_mod`**, drawing on the world RNG.
     - If the chosen verb is **Wander**, its destination is sampled now: uniformly from the reachable tiles in the flood whose Chebyshev distance from the sprite is more than `sense_radius / 2`, the trait as it is, not rounded as the flood's reach is. If there are none, it samples from all reachable tiles other than the sprite's own. If there are none at all, Wander ends straight away with outcome `failed`.
   - **With a current action, verb c:** it continues unless some available verb v has `s_v > s_c + switch_margin`. If one does, the sprite switches **deterministically** to the highest-scoring such verb, with ties going to the lower `OutputId`.
4. **Snapshot** the inputs, concept activations, chosen verb, attended category and object type (v19), and the verb's **motive** (below), ready for the trace entry committed at step 6.

**The motive** of a chosen verb is the need whose instinct did most to choose it: of the needs' singletons, the one with the largest positive `a_k × W[k][v]`, ties to the lower input ID. A verb that no need's instinct pushed has no motive. Disappointment uses it (§5.6).

**Rule:** all randomness in the brain (attention, verb, Wander destination) happens **only at action boundaries**. A running action can only be ended by a deterministic change larger than the margin, by losing its target, by timing out, or by completing. It's never ended by noise.

World randomness is separate: plant rules and the order actions resolve in. It can change how an action turns out, but it ends an action only through those same deterministic rules.

**Action lifecycle:**

| Verb | Behaviour | Ends |
|---|---|---|
| Approach | Walks to a goal tile | On arrival |
| Eat / Drink / Hit / Play | Walks to a goal tile, then makes **one attempt** on the tick it arrives, in the same step 6 (at once, if it starts on a goal tile) | After the attempt, whether `applied` or `failed`; a fruitless try (§5.2) is `failed` and pulses `fruitless` |
| Retreat | Each step goes to the walkable, free neighbour that increases the **Chebyshev distance** from the target the most. Among those, it takes the one pointing **most directly away**: the smallest angle between the step and the line from the target through the sprite. Exact ties follow the direction order N, NE, E, SE, S, SW, W, NW. Path distance isn't used, because it would need a second flood run from the target. With no step further away, the sprite is cornered (§3.7). | After 6 steps (`retreat_bout`), `applied`; cornered, `blocked` |
| Rest | Stays put and sets `resting` | After 10 ticks, `applied` |
| Wander | Follows the flood path to the destination chosen at 5b | On arrival, `applied` |

- Any action that moves ends with outcome **`blocked`** when blocked re-planning finds no way forward (§3.7). A retreat doesn't re-plan: it ends `blocked` when it's cornered (§3.7).
- **Verbs by slice.** Slice 6 made Approach, Eat, Drink, Rest and Wander available, slice 7a Hit and Play, slice 7c Retreat. Slice 9 (v16) offers every interaction on any target.
- **A free neighbour,** for Retreat, is a walkable tile with no sprite and no solid object that the step reaches without cutting a corner (§3.1). An item doesn't stop it. A target on the sprite's own tile (an item it stands on) is at distance 0, so any free neighbour gains distance, and the direction order alone chooses.
- **Every** action that hasn't ended otherwise ends at the **action timeout** of 60 ticks, with outcome `timed_out`.
- **A led or sliding sprite has no action** (v23, v25). Taking hold of it ends its action `pulled_away`; a shove lets go of it, and it slides with none. Let go and not sliding, it chooses afresh at its next step 5.
- Bout lengths and the timeout are set in `physiology.ron`.
- **Outcomes:** `applied` / `walking` / `blocked` / `failed` / `interrupted` / `timed_out` / `pulled_away`. `walking` and `blocked` can also be per-tick outcomes of an action that's still running. `interrupted` ends an action the sprite changed its mind about: attention switched away from its target (§5.3), or another verb beat it by more than `switch_margin` (5b). `pulled_away` (v23) ends the action of a sprite the Cursor takes hold of (§6.5).

### 5.6 Learning (step 4)

**What a brain learns** (v16), all starting at 0 and all within [−1, 1]:

| What | Per | Learns from | Credited to |
|---|---|---|---|
| **Worth for a need**, `G_n[c]` (0 to 1) | need × object type (v19), or need × remembered sprite (v18) | that need's relief | the thing touched |
| **General good**, `G[c]` (0 to 1) | object type, or remembered sprite | `reward` (pets; anything the genome adds) | the thing touched |
| **Bad**, `B[c]` (−1 to 0) | object type, or remembered sprite | `punishment` from its own touch (hurts, shocks) | the thing touched |
| **Fear**, `F[s]` (−1 to 0) (v18) | remembered sprite | `punishment` from a hurt done to it (a `was_hit` pulse live) | the one who did it |
| **Habit**, `H[c][v]` | object type × verb; sprites as a whole × verb | good less bad, and disappointment | the trace |
| **The worth of new things**, `new_things` | brain | good less bad, times how new the thing touched was | the thing touched |
| **Familiarity**, `familiarity_c` (0 to 1) | object type; sprites as a whole | attending to it | — |

In this section, *c* is an object type (v19). Sprites are learned about one by one for worth and fear, and as a whole for habits and familiarity.

**A thing's worth now:** `value_c = Σ_n level_n × G_n[c] + quiet × G[c] + B[c]`, over the needs *n*. A berry learned good for hunger counts for a lot when the sprite is starving and nothing when it's full; a thornbush learned bad counts always. For an object type the sprite doesn't know yet, the values are its category's summary (below).

**First-order needs quiet what's merely liked** (v21). `quiet = 1 − quieting × urgency²`, where urgency is the highest level of the sprite's first-order needs (hunger and thirst, §5.2) and `quieting` a brain parameter (0.8 by default). Squared, a little hunger or thirst hardly quiets anything, and a desperate one quiets most. It scales a thing's general good in its worth, and every good habit in the decision (§5.5); worth for a need, bad, fear and bad habits are never quieted. So a starving sprite still likes its ball a little, but its eye turns to what might feed it. It never blocks: at full urgency a fifth of a like is left.

**The learning signals,** read at step 4:
- **Relief** for each need: its fall since the previous tick's step 4, when that fall is at least `relief_deadband` (physiology, 0.02), else 0. Relief is read straight from the needs (§5.2), so a drive knocked down by eating is relief whatever emitter did it. A slow drift never counts.
- **Good:** `reward`. **Bad:** `punishment`. Both are then **reset to 0**: learning consumes them, so they never linger (the reasoning of v12 still holds: a lingering reward credits what the sprite did after it).
- The sprite's **`last_r`** ("felt", saved state, §6.1) is `Σ relief + reward − punishment`.
- Every learning rate below (worth, bad, fear and habits) is scaled by `learning_rate_mod`; fading and familiarity aren't.

**The thing touched** is the target of the sprite's latest attempt (an interaction's one try, applied or failed) if it was within `touch_window` ticks (physiology, 3). It is what the feeling is about. With nothing touched, worth learns nothing that tick; habits still do.
- **A crash** (v23, slice 11b) counts as touching the thing crashed into, so a sprite shoved into a thornbush learns thornbushes are bad. It has no verb and the sprite chose nothing, so it teaches no habit. **While a crash is the thing touched, habits learn nothing** (v25), not even along the trace: the trace holds what the sprite chose before it was taken hold of, which the crash's pain isn't about. So a crash counts as the sprite's latest attempt, with no verb: a pet or a shock that reaches back to it (below) teaches worth or badness for the thing, and no habit.
- **Not the attacker** (v16, measured): being hit teaches no worth or badness. With all sprites one kind, it made a sprite shy of every sprite, and fight or flight collapsed to flight (v16 change 16). Since v18 it teaches fear of that one sprite instead (below), and habits along the trace.
- **Why not the trace** (v16, measured): crediting worth back along the trace blamed each thornbush prick partly on the berry bush or water the sprite had looked at a moment before. Bad lessons fade slowly, so in the A1 arena berries, bushes and water all ended near −0.5, and meals fell by two-thirds. Crediting only the thing touched made thornbushes alone bad (−0.93) and meals recovered. It suits a species that knows the world by touch, and it is how blame should work ([#57](https://github.com/Keazra/terra-sprites/issues/57)).

**The Cursor's reach back** (v21). A pet or hug carries its reach back (§2.5): about two seconds of the player's time at their speed, at least `touch_window` and at most `max_reach_back`. A zap or shock looks back only `touch_window`, whatever the speed: measured, a shock reaching further nearly always found something to blame, often a recent drink, and sprites shocked at random moments learned to avoid water and died of thirst (v21 change 6). In a tick when the Cursor rewarded the sprite (a `petted` pulse is live at step 4), that tick's reward goes to the sprite's **latest attempt, if it was within the Reward's reach back**; in a tick when it corrected the sprite (a `shocked` pulse is live), that tick's punishment goes to its latest attempt within `touch_window`, even when a pet lands in the same tick. Each goes:
- **to worth:** good, or bad, for the thing that attempt touched, by the rules below;
- **to the habit** for that thing and that attempt's verb, at full weight, `H[c][v] += habit_rate × reward` (or `− habit_rate × punishment`), in place of the trace.

With no attempt within reach, the tick teaches as any other: no worth, and habits along the trace. Relief, disappointment and fear are unchanged: relief keeps the touch window, and while a `was_hit` pulse is live punishment teaches fear of the hitter (below), even a shock's. **Why the latest attempt:** the player rewards what they saw the sprite do, and at 8× a player reacts after the sprite has moved on. The trace would spread the pet over the walk to whatever came next, and a fixed 20-tick window would span 16 seconds at 1×, long enough to credit a meal eaten well before a pet for resting.

**Worth and bad,** for the thing touched, of object type *c*. A hit's punishment (a `was_hit` pulse live) teaches fear instead, below, even when the sprite had just touched a sprite itself:
- `G_n[c] += worth_rate_good × relief_n` for each need; `G[c] += worth_rate_good × reward`; `B[c] −= worth_rate_bad × punishment`.
- **A sprite touched** (v18) is learned about as that one sprite, at the faster `individual_rate_good` and `individual_rate_bad`: `G_n[s]`, `G[s]` and `B[s]`, remembering it if it didn't yet. Nothing is learned about sprites in general directly.
- `new_things += (worth_rate_good × (Σ relief + reward) − worth_rate_bad × punishment) × novelty`, where `novelty` is how new the thing was when the sprite attended to it at that tick's step 5. A sprite hurt whenever it investigates grows shy of the unfamiliar; one rewarded for it grows bold.

**Fear** (v18). While a `was_hit` pulse is live, `F[a] −= fear_rate × punishment` for the attacker *a* (the pulse's source), remembering it if it didn't yet. That punishment teaches no worth and no badness, and still teaches habits along the trace.

**Sprites in general** (v18) are a summary of the *n* sprites a sprite remembers: each value (each `G_n`, `G`, `B` and `F`) is their mean × `clamp((n − 1) / (generalise − 1), 0, 1)`. So a sprite that knows only its bully fears no stranger, and one hurt by three different sprites is wary of every stranger. A sprite it doesn't remember is judged by the summary, and the Brain tab shows the summary as "sprites".

**Category summaries** (v19) work the same way for object types. A sprite **knows** an object type once it has touched one. A category's summary is, for each value (each `G_n`, `G` and `B`), the mean over the *n* object types in the category the sprite knows × `clamp((n − 1) / (generalise_types − 1), 0, 1)`. An object type the sprite doesn't know is judged by its category's summary, which counts for nothing until the sprite knows two types in the category. So one thornbush doesn't make every bush bad: slice 9 measured that failure when pricks leaked onto berry bushes (above). Nothing is learned about a category directly; habits and familiarity have no summary, so an unknown type's start at 0. Object types are never forgotten. Sprites in general is the sprite category's summary, over the sprites it remembers.

**Habits,** along the trace:
- **The trace:** every tick, each deciding sprite commits one entry at the end of step 6: that tick's snapshot (its verb, the object type it attended (v19), the verb's motive, and how new that type was). Entries are kept while `λ^age ≥ 0.01`, up to a cap of 512. For each entry *j*, `weight_j = λ^(t − t_j)`; at tick *t* the freshest entry is from *t−1*.
- For each entry with an aimed verb and an attended object type: `H[c_j][v_j] += habit_rate × (reward − punishment − disappointment_j) × weight_j`.
- **Disappointment:** a fruitless try disappoints the need that chose it. If the latest attempt was fruitless and its entry has a motive *n*, that entry's `disappointment_j = disappointment × level_n`. Playing with a ball out of plain curiosity costs nothing; a hungry sprite that tries to eat a ball learns that eating balls doesn't work.
- **Relief doesn't teach habits:** it teaches worth, which draws the sprite back. Measured, habits from good less bad and disappointment were enough.
- **Praise credits both** (owner decision): a pet just after a sprite tries to eat a ball raises the ball's general good *and* the habit of eating balls. Players can teach tricks and quirks, on purpose or by accident, and without more praise the habit fades.

**Fading.** Every tick, after learning: each `G_n` and `G` and `new_things` is multiplied by `1 − worth_fade_good`, each `B` by `1 − worth_fade_bad`, each `F` by `1 − fear_fade`, each good habit by `1 − habit_fade` and each bad habit by `1 − habit_fade_bad` (v21). The starter genome's defaults learn bad faster than good and fade it about four times more slowly (Appendix B), habits as well as worth since v21.

**Forgetting** (v18). A remembered sprite all of whose values are under `forget_below` (physiology, 0.01) is forgotten, and so is one that has died, at step 4. There's no limit on how many a sprite remembers.

**Familiarity** rises by `familiarity_rate` for the attended object type (v19) on each tick the sprite commits a trace entry, and never falls in M1. (Whether it should wear off again belongs with habituation, below.)

**Lessons:** a `LearnedMilestone` event fires the first time a learned value is `lesson_threshold` (0.5) or more from 0: a worth for a need, a general good, a bad, a fear, a habit, or the worth of new things, about an object type, a category's summary (v19), a remembered sprite (by its ID) or sprites in general (v18). Each fires at most once in a sprite's life; which have fired is saved state.

**When nothing is learned.** A lab scenario's control run (§7.1) switches learning off world-wide: step 4 still consumes `reward` and `punishment` and records `last_r`, but changes no worth, habit or `new_things`. Familiarity still grows.

**Later, not in slice 9:**
- **Habituation:** repeated senses, pets and shocks included, wear off gently and recover with time. It is its own slice after the core brain. Slice 9 measured that it isn't what stops a bored sprite fiddling: that is mostly an arena with no toys (about 45% of ticks there, 1–2% in the default world). Lowering a thing's worth for the need behind a fruitless try stopped the fiddling but killed 8 of 10 sprites: with boredom stuck at 1, "useless when bored" outweighed "good when thirsty".
- **Decisions that build up over time,** with body language ([#15](https://github.com/Keazra/terra-sprites/issues/15)).
- **Places** ([#40](https://github.com/Keazra/terra-sprites/issues/40)), and **the visible Cursor** that sprites see and learn about ([#60](https://github.com/Keazra/terra-sprites/issues/60), slice 11d). Individual sprites came in v18. Three readings of what a visible Cursor's touch teaches wait for that slice: sprites also learn to like or fear the Cursor; they learn about the Cursor instead of the thing; or the lesson holds only while the Cursor is in sight.
- **Genes that switch on at life stages** (M2, [#49](https://github.com/Keazra/terra-sprites/issues/49)): a new instinct joins the instinct pathway mid-life without touching what was learned.

### 5.7 Genes for the brain

| ID | Gene | Semantics |
|---|---|---|
| 7 | `BrainParam(param_id, value)` | One gene per parameter, clamped to a range set in `physiology.ron` (Appendix B). A parameter with no gene takes its default from the same place. |
| 8 | `Instinct(inputs: [(InputId, negated)] ≤3, verb, weight)` | Creates or merges the concept with this signature and sets its instinct link to `verb`. A single input refers to that input's singleton. |
| 9 | `AttentionInstinct(InputId, CategoryId, weight)` | An attention instinct. **Only State inputs are allowed.** Anything else is flagged and not expressed. A category the world's pack doesn't have (v19) makes it an **unmatched gene**: kept but with no effect, marked in the genome viewer "names a category this world doesn't have", and passed on unchanged, as unknown genes are (§2.8). So does an `Instinct` (8) whose inputs include `attended_<category>` for such a category. Genome files name categories by name, and an unmatched name is kept as written. |

All references are stable IDs. The value fields of each gene type, which are the only fields spawn variation and mutation change, are listed in §4.3.

**Evolution can grow smarter instincts** (v16). The gene types can express instincts about categories ("hunger draws the eye to fruit", "attended bush → retreat"); only the starter genome leaves them out. Instincts name categories, never object types (v19), so a genome works with any mix of object types, and what's particular to one type is learned. Generations that happen to carry such genes survive better, so what the species learns can become instinct over time (M2).

### 5.8 Starter genome: very basic

The species is dropped into a world it doesn't know and has to learn about it. Its instincts connect needs to actions and know its own kind; they know nothing about food, water, toys or danger (v16).

| Need or event | Attention instinct | Decision instinct |
|---|---|---|
| Hunger | — | hunger → Eat (whatever is attended) |
| Thirst | — | thirst → Drink |
| Tiredness | — | tiredness → Rest |
| Boredom | — | boredom → Play |
| Loneliness | → Sprite | loneliness → Approach, Play |
| Pain | — | pain → Retreat |
| Being hit | `was_hit` → Sprite (the attacker, §3.6) | `was_hit & attended sprite` → Hit (0.56) |
| Crowdedness | → Sprite | crowdedness → Retreat; `crowdedness & attended sprite → Hit` (0.6) |
| Cornered | — | `cornered & attended sprite` → Hit (1.0) |
| Nothing pressing | — | `always` → Wander (mild) |

- **What goes, from v15:** the attention instincts that drew hunger to berry bushes and berries (and weakly to thornbushes), thirst to water and boredom to balls. A newborn looks at whatever is nearest or least familiar, and learns which things answer which need. v15's deliberate mistakes are no longer needed: nearness and curiosity lead a newborn into thornbushes on their own.
- **Its own kind** is the one thing it knows: loneliness and crowding draw its eye to sprites, and it turns towards whatever hit it. Ducklings imprint; a sociable species knows its kind.
- **Fight or flight** stays as in v15 (§5.8 of v15): pain leads to Retreat, `was_hit` with the attacker attended to Hit a little more weakly, a crowded sprite sometimes hits a neighbour (the mistake Correct training cures, A3), and a cornered one fights. **Fear** (v18) comes after: it keeps a sprite away from a sprite that hurt it while that one is near, and never pulls towards hitting. So courage is the strength of the hit-back instinct, and timidity is `flight`; the rest of temperament is its own design ([#53](https://github.com/Keazra/terra-sprites/issues/53)).
- **A content sprite wanders:** `always → Wander` is a mild habit any grown need outweighs.
- **How the brain learns,** in `BrainParam` genes: the trace stays short (`trace_decay` .5, v15 change 10), and the worth, habit, curiosity, familiarity, individual and fear parameters take physiology's defaults (Appendix B) until tuning says otherwise.
- **The body's side** (§4.5): relief comes from the needs themselves, so the genome no longer turns a drive's fall into `reward`; need makes a sprite restless; and a run of hurts makes it wary.
- **Measured** (v16 prototype, default world, 30 sprites, 5 seeds): with these instincts, 1 sprite died in 20,000 ticks, against 2 with v15's; thornbush contacts fell from 31 to about 6 per 5,000 ticks, against 100 to 43 with v15's; and sprites learned water is good for thirst (+.56), other sprites good for boredom (+.24) and loneliness (+.20), balls good for boredom (+.11), and thornbushes bad (−.43). Food is plentiful, so sprites snack before hunger passes the relief deadband and learn berries only weakly (+.03 to +.06); viability tuning (slice 17) revisits it.

### 5.8.1 Budget

- **Per sprite:** instinct links (≤ ~105 concepts × 8 verbs, plus 37 × one per category attention), and learned values for each object type (v19; 6 in M1, counting water and sprites): 6 needs, 2 general, 8 habits and a familiarity; and 1 new-things. Category summaries are worked out when needed, not stored. Under 1.2k numbers, about 5 KB. Plus 9 numbers for each sprite it remembers (v18): measured, about 5 on average and 15 at most, with no measurable cost in run time.
- **Whole world:** 100 sprites is about 0.5 MB.
- **Per tick:** a few thousand multiply-adds per sprite for the instincts; learning touches one thing and at most the trace's habit cells.

### 5.9 Legibility

`Brain::explain()` returns, and the front end reads it as `SpriteView::explain`:
- the attention scores, highest first, from the snapshot step 5 keeps
- what adds to or takes from the current verb's score, largest first whatever the sign, from the same snapshot: each instinct concept, the attended thing's habit for the verb, its worth, and how frightening it is (v18)
- which thing each category's score is for: its candidate's object type (v19), or for sprites which sprite (v18); and the thing a verb is aimed at

`SpriteView::memory` returns the sprite's **memory** (v16): its learned values furthest from 0, largest first, of every kind (a worth for a need, a general good, a bad, a fear, a habit, the worth of new things), about object types, category summaries (v19), remembered sprites and sprites in general (v18), up to five; ties keep that order. The Brain tab leaves out one that rounds to `.00`. It is read on its own, not through `explain`, since a sprite has learned things before it first decides. Step 4 runs before step 5, so the values the tab shows are the ones step 5 scored with.

---

## 6. UI and the Cursor

### 6.1 Layout

- The minimum terminal size is **100×30**; 140×40 is recommended.
- A smaller terminal shows only "Terminal too small" and "needs 100x30, this is 80x24", in the middle, and the game carries on as it was set: shrinking the window doesn't pause it, and `Esc` still asks to quit, with "Quit? (y/n)" under the message (v29). At 100×30 and up, every panel always shows.
- All text uses only CP437 characters (§6.2).

```
 Terra Sprites │ tick 48,210 │ ► 4x │ seed 7 │ sprites 27 │ saved 2m ago                     ? help
┌─ Map ────────────────────────────────────────────┐┌─ Mira #12 ── [Body] Brain Chem Genome World ┐
│..,,,..~~~~≈≈≈≈~~..........♣....#########........ ││ Going to eat the berry bush · 3 tiles to go │
│.,,,...~~~≈≈≈≈≈~~....♣.........########....♠..... ││ age 3,410 · speed 7 · sense 10              │
│..,,...~~~~≈≈≈~~..........☺.......#####.......♣.. ││ hunger     ███████░░░ .71 ▲                 │
│...,....~~~~~~~.....•.........☺.....##....☺...... ││ thirst     ██░░░░░░░░ .18                   │
│.....,...................♣.............♠......... ││ pain       ░░░░░░░░░░ .00                   │
│....☺....○....................................... ││ tiredness  ████░░░░░░ .39                   │
│..........................♣.......☺.............♠ ││ energy .42 · hydration .77 · injury .05     │
└──────────────────────────────────────────────────┘└─────────────────────────────────────────────┘
┌─ Events ─────────────────────────────────────────────────────────────── [all] selected major ─┐
│ 48,207  Mira was pricked by a thornbush                                                        │
│ 48,190  Mira learned: thornbushes are bad                                                      │
│ 48,102  Kel died (starvation, age 6,020)                                                       │
└────────────────────────────────────────────────────────────────────────────────────────────────┘
 (61,40) grass · Mira #12 │ SELECT │ following Mira #12 │ holding: berry   Z select  X train  C grab
```

**Brain tab** (v16; the layout is agreed in slice 9, and this revision is amended to match):

```
┌─ #12 ── Body [Brain] Chem Genome World ────┐
│ ATTENTION                                  │
│ ► Sprite #7                           1.40 │
│   berry bush                           .52 │
│   water                                .34 │
│                                            │
│ DECISION: RETREAT                     1.10 │
│   fear: Sprite #7                     +.62 │
│   pain                                +.40 │
│                                            │
│ MEMORY                                     │
│   Sprite #7 is frightening            -.80 │
│   thornbushes are bad                 -.62 │
│   Mira #12 is good for loneliness     +.41 │
└────────────────────────────────────────────┘
```

- **Attention** lists each category in reach with its score, highest first, and marks the attended one `►`. With nothing in reach it reads "nothing in sight". Each row names the thing it scored (v19): the object type, "berry bush", or for sprites the sprite (v18), as the event log names sprites, "Sprite #7", "Mira #12". That's the one a running action is aimed at, or else the one that draws the eye most (§3.6, §5.3).
- **The decision** names the verb chosen, or kept, at the latest step 5, with its score, then the five parts adding most to it, largest first whatever the sign: instinct concepts, named by their inputs as the Genome tab names instincts; the attended thing's worth; its habit for the verb; and how frightening it is, "fear: Sprite #7" (v18). Worth and fear name the object type or the sprite, not the category. One whose part rounds to `.00` is left out. A name too long for its row wraps between words.
- **Memory** lists the sprite's learned values furthest from 0 (§5.9), each with its value, worded as the event log words a lesson: "Sprite #7 is frightening", "Mira #12 is good for loneliness", and sprites in general as "sprites are frightening" (v18). The part is left out while nothing has been learned.
- **An object type, in general,** is worded by its `plural` (§3.5.1): "thornbushes are bad". One without a plural, such as water, reads with "is". **A category's summary** (v19) is worded by the category's own `plural` (§3.5.5), "bushes are bad", or by its name with "is", and shows once it counts, as "sprites" does. This replaces v17's wording of a kind by the plural of the first object type perceived as it.
- **While the Cursor leads it** (v23), attention and the decision read "Being led: it decides nothing", since the decision from before it was taken hold of would mislead; its memory still shows below.
- Before a sprite's first decision, attention and the decision read "Nothing decided yet", as they do while a sprite in a hand-made world works through its scripted actions. Its memory still shows below (v17), since such a sprite can learn before it decides:

  ```
  │ Nothing decided yet                        │
  │                                            │
  │ MEMORY                                     │
  │   thornbushes are bad                 -.93 │
  ```

**Panels:**
- **Top bar:** tick, speed, seed, population, save status, and at the right end `? help` (v29). Object counts, food included, are the World tab's job.
- **Map view:**
  - Its viewport scrolls with `W` `A` `S` `D` or the arrow keys, or follows the selected sprite (`T`, "track", v21): Track keeps it in the middle of the map view, as far as the wall allows, and follows whichever sprite is selected. Scrolling by hand turns it off (v29).
  - A map smaller than the space gets a map view shrunk to fit it, at the top-left.
  - Its border is **double-lined** (`═ ║`) on any side where the terrarium's wall is in view, and single-lined where the map carries on.
  - The Cursor is 3×3 tiles and follows the pointer, or the sprite it follows (§6.5).
- **Inspector tabs** (`[` and `]`, wrapping around). The game starts on World. The inspector's title is the selected sprite and the tabs, with the open one in brackets; with no sprite selected, it's the tabs alone. A sprite's label too long to fit is shortened, never the tab names.

| Tab | Shows |
|---|---|
| **Body** | What the sprite is doing, in plain words (below); age and lifespan; speed and sense radius as expressed; a bar for each drive, with its level and an arrow for its change over the last tick (`▲` rising, `▼` falling, none when steady); the physical levels; the observed list (below) |
| **Brain** | Output of `explain()` (below) |
| **Chem** | One list: each chemical on its own line with its level and its change per tick (`-.0002`, blank when it rounds to 0), the physical chemicals, then a blank line and the signal chemicals. From slice 8 the reward line also shows `last_r` as "felt": `reward  .00  felt -.42`. The 16 hormones sit in a 4×4 grid of levels below, so all of it fits at 100×30. Reward and punishment are consumed every tick, so their levels always read 0 between ticks. |
| **Genome** | Genes grouped under headings: traits first, on one line (`speed 7.21 · sense 9.87 · lifespan 61,204`), then half-lives, reactions, emitters, receptors, starting levels, brain settings (`tau base .2`), instincts (`thirst & not target adjacent → drink -.5`) and attention instincts (`hunger → attends to bush +.8`, naming the category as the gene does, v19), and unknown genes last, each group in genome order. Each gene is a plain line with 3 significant figures, so spawn variation shows: `low energy → hunger +.00428 past .515`, `hunger falls → reward +1.02 past .0198`, `hunger halves every 2,041 ticks`. A line too long for the tab wraps, indented. A flagged, unexpressed, unknown or unmatched (v19) gene is dimmed, with its reason on the line below (§4.3); an unmatched gene written by number shows as its number, as an unknown gene does. `g` exports to RON. |
| **World** | The data pack's identity; population and deaths by cause (counted by the sim, so they're saved); each object type in ID order with its count, the count in each stage (left out for a type with only one stage), and the total of each counter (such as the fruit on all bushes). It's drawn from the data, so a pack's new object types appear with no new code. |

- **The Body tab's action line** describes what the sprite is doing, in the present tense, naming its target, with no coordinates. It never speaks as the sprite: the Brain tab shows *why*, and a line in the sprite's own voice could contradict it. When an action ends, its ending stays until the next one starts.

  | Moment | Line | Detail view (`v`) |
  |---|---|---|
  | Wandering | `Wandering off · 5 tiles to go` | `WANDER → (61,40) · walking (5 tiles)` |
  | Held up behind another sprite | `Wandering off · waiting to get past` | `WANDER → (61,40) · blocked (2 ticks)` |
  | Arrived | `Arrived` | `WANDER → (61,40) · applied` |
  | Gave up, no way through | `Gave up: the way was blocked` | `WANDER → (61,40) · blocked` |
  | Gave up at the timeout | `Gave up: it took too long` | `WANDER → (61,40) · timed_out` |
  | Gave up, destination out of reach | `Gave up: it couldn't get there` | `WANDER → (61,40) · failed` |
  | Changed its mind | `Changed its mind` | `WANDER → (61,40) · interrupted` |
  | Resting | `Resting · 6 ticks left` | `REST · 4 of 10 ticks` |
  | Rested | `Rested` | `REST · applied` |

  | Going to eat | `Going to eat the berry bush · 3 tiles to go` | `EAT → berry_bush #812 · walking (3 tiles)` |
  | Ate | `Ate from the berry bush` | `EAT → berry_bush #812 · applied` |
  | Ate a berry whole | `Ate the berry` | `EAT → berry #9 · applied` |
  | Couldn't eat | `Couldn't eat from the berry bush` | `EAT → berry_bush #812 · failed` |
  | Going to drink | `Going to drink · 2 tiles to go` | `DRINK → water (40,12) · walking (2 tiles)` |
  | Drank | `Drank` | `DRINK → water (40,12) · applied` |
  | Going over | `Going over to Sprite #530 · 4 tiles to go` | `APPROACH → sprite #530 · walking (4 tiles)` |
  | Got there | `Got to Sprite #530` | `APPROACH → sprite #530 · applied` |

  Gave-up lines keep the Wander wording. A target that vanished reads `Gave up: the berry was gone`. Hit, Play and Retreat follow the same pattern:

  | Moment | Line | Detail view (`v`) |
  |---|---|---|
  | Going to play | `Going to play with the ball · 3 tiles to go` | `PLAY → ball #40 · walking (3 tiles)` |
  | Kicked | `Kicked the ball` | `PLAY → ball #40 · applied` |
  | Played | `Played with Sprite #7` | `PLAY → sprite #7 · applied` |
  | Going to hit | `Going to hit Sprite #7 · 1 tile to go` | `HIT → sprite #7 · walking (1 tile)` |
  | Hit | `Hit Sprite #7` | `HIT → sprite #7 · applied` |
  | Hurt doing it | `Hit the thornbush, and got hurt` | `HIT → thornbush #77 · applied` |
  | Backing away | `Backing away from Sprite #7 · 4 steps to go` | `RETREAT → sprite #7 · walking (4 steps)` |
  | Held up by a sprite | `Backing away from Sprite #7 · waiting for room` | `RETREAT → sprite #7 · blocked (2 ticks)` |
  | Backed away | `Backed away from Sprite #7` | `RETREAT → sprite #7 · applied` |
  | Cornered | `Backed into a corner` | `RETREAT → sprite #7 · blocked` |

  - **"Kicked"** is a Play whose effects include a `Push`, worked out from the verb table, not the object's name; a Hit that pushes stays "Hit".
  - **", and got hurt"** follows any action in which its own sprite was hurt, whatever hurt it. The data names things: the screen keeps no list of words like "pricked". An Eat that hurts reads `Tried to eat the thornbush, and got hurt`.
  - **Led and shoved** (v23). While the Cursor leads the sprite, the line says so in place of an action, with how far it is from the Cursor; a shove (slice 11b) reads the same way:

    | Moment | Line | Detail view (`v`) |
    |---|---|---|
    | Being led | `Being led · 4 tiles behind` | `LED → (61,40) · walking (4 tiles)` |
    | Caught up with the Cursor | `Being led` | `LED → (61,40)` |
    | Sliding (11b) | `Shoved · 2 tiles to go` | `SHOVED → NE · 2 tiles left` |

    A sprite taken hold of while it slides reads as sliding until the slide ends (v25). While it slides, the Brain tab reads "Shoved: it decides nothing", as a led sprite's reads "Being led: it decides nothing".
- **The observed list** ends the Body tab: what the player has watched the selected sprite finish since selecting it, newest first, at most 500 lines.

  ```
   Observed
          just now · Rested
       8 ticks ago · Wandered off, but gave
                     up: the way was blocked
       9 ticks ago · Wandered off ×2
   1,296 ticks ago · Rested
  ```

  - One line per finished action, in the past tense: what it did ("Wandered off", "Rested", "Ate from the berry bush", "Ate the berry", "Drank", "Went over to Sprite #530"), or, if it went badly, what it set out to do and how that went ("Went to eat the berry bush, but it was empty", "…, but changed its mind", "Wandered off, but gave up: the way was blocked"). A retreat reads "Backed away from Sprite #7", or "Backed away from Sprite #7, but was cornered".
  - **What others did to it** goes in the same list, in time order: "Was hit by Sprite #7", "Sprite #9 played with it".
  - **The Cursor's touch** goes in it too (v21), told as the sprite felt it. While the Cursor is invisible the sprite can't know where it came from: "Felt a gentle touch out of nowhere" (a pet), "Felt a warm embrace out of nowhere" (a hug), "Felt a zap out of nowhere", "Felt a jolt out of nowhere" (a shock).
  - **Being led** (v23) goes in it the same way, once it's let go: "Was pulled along out of nowhere". An action it was taken hold of in the middle of reads "Went to eat the berry bush, but was pulled away". From slice 11b, a shove: a shove lets go, so first "Was pulled along out of nowhere", then, once the slide ends, "Was shoved out of nowhere"; with a crash, "Was shoved out of nowhere, into Sprite #7", or, when it hurt, "Was shoved out of nowhere, into a thornbush, and got hurt" (v25).
  - Lines that read the same in a row merge into one with a count (`×3`); its time is when the latest of them finished. The times count up live and are right-aligned; a long line wraps under its own text.
  - It starts afresh when another sprite is selected (reselecting the same one keeps it), and reads "nothing yet" until something finishes.
- **The detail view** (`v`, in every build) shows the exact workings behind what the screen describes in words: it switches the action line to the exact verb, destination and outcome. It stays on until `v` is pressed again, whatever is selected.
- **Map marks for the selected sprite,** always shown:
  - **The Decision marker,** where it's heading, flashes like a text cursor, in real time: half a second a plain `X`, half a second the tile as it is, while the action is under way: a Wander's destination, or the goal tile an Eat, Drink or Approach is walking to. A retreat has no destination, so no mark. A sprite standing on the destination is drawn over the mark.
  - **The Attention marker,** what it attends to, is a steady dark grey background: the one thing attention is on (§5.3), never every tile of its category. While an aimed action runs, that's its target; otherwise the attended category's candidate, which can move from tile to tile as the sprite walks.
- **The inspector and the selection:**
  - The Body, Brain, Chem and Genome tabs show the selected sprite. With none selected, they read "No sprite selected: click one, or press Tab".
  - Selecting a sprite while the World tab is open switches to Body. From any other tab, the tab stays.
  - When the selected sprite dies, its tabs read "Mira #12 died of dehydration at age 4,012" until another sprite is selected.
  - A tab too long to fit scrolls a page with `PgUp` / `PgDn`, or 3 lines per notch of the mouse wheel over the inspector. It stops at the top and at the end, and goes back to the top when the tab or the selection changes.
- **Event log:** each event shows its sprite by name and ID, as "Mira #12", or "Sprite #530" for a sprite with no name (§6.5). Newest first, filtered to all / the selected sprite / major events only (deaths, learning milestones, rejected commands). The filters sit on the log's top border, ` [all] selected major `, the shown one in brackets; `m`, or a click on them, goes to the next (v29). "Selected" is the events the selected sprite did or had done to it. It never shows object events (`ObjectSpawned`, `ObjectRemoved`): they happen dozens of times a minute and would bury everything else, and the World tab counts objects instead. Nor does it show most action events (`ActionStarted`, `ActionEnded`): with 30 sprites they come about 3 for every tick. The Body tab shows the selected sprite's action instead, and emotes show resting and giving up on the map (§6.3). **The exceptions** (from slice 7a) are every Play and Hit that applied, and any action in which a sprite was hurt (not Retreat, which does nothing to anyone), worded as the Body tab words them: "Sprite #4 kicked a ball", "Sprite #9 played with Sprite #2", "Sprite #7 hit Sprite #12", "Sprite #3 tried to eat a thornbush and got hurt". They're rare enough not to bury deaths; if they do, the major-events filter is the answer.
  - **The Cursor's touch** (v21) is logged, spoken to the player: "You petted Sprite #12", "You hugged Sprite #12", "You zapped Sprite #12", "You shocked Sprite #12". So is a refusal: "Couldn't pet Sprite #12: it's gone".
  - **Grabbing** (v23) is logged the same way: "You took hold of Mira #12", "You let go of Mira #12", "You picked up a ball", "You put the ball down", and from slice 11b "You threw the ball", "You shoved Mira #12", and "Mira #12 was shoved into a thornbush and got hurt" (`Threw`, `Shoved`, and a `Crashed` that hurt, v25; a crash that doesn't hurt isn't logged). A held item that expires: "The berry you were holding expired". A led sprite that dies has only its death line. Refusals say what stood in the way: "Couldn't take hold of Sprite #7: it's gone", "Couldn't take hold of Sprite #7: you're already leading Mira #12" (or "…holding something"), "Couldn't put the ball down: a berry is there" (or "…: it can't go on rock", "…: it can't go in deep water"). Letting go of a sprite can't fail: it's already standing somewhere. The screen's clicks follow what the Cursor has hold of, so the "already" refusals come only from clicks queued in a race; the one players meet is putting something down where it can't go.
  - **Lines that read the same in a row merge** into one with a count, "You petted Sprite #12 ×10", as the observed list's do; its tick is the latest one's. Spam-clicking would otherwise fill the log.
- **Status line:** the tile under the Cursor, the cursor mode, the sprite the Cursor follows (`following Mira #12`, v26), what the Cursor holds or leads (`holding: berry`, `leading: Mira #12`, v23, in every mode), and hints for the active keys. A prompt such as "Quit? (y/n)" takes its place while open.
  - **The key hints** (v22) sit at the right and lead with the mode keys, `Z select  X train  C grab`, then `WASD scroll  space pause  . step  +/- speed  esc quit`. Short of room, whole hints drop from the end, so the mode keys are the last to go, together.
  - **While aiming** (v25), the key hints read `let go to throw  esc cancel`, or `let go to shove  esc cancel`. **While the sprite list is open** (v29), they read `↑↓ choose  enter go to it  tab sort  esc close`.
  - **A refused click's reason** (v22) takes the key hints' place for about 3 seconds of real time, worded as the event log words it: "Couldn't pet Sprite #12: it's gone". A Train click with nothing to act on says so the same way: "No sprite here to pet" (or hug, zap, shock). So do an `F` or a middle click with nothing to follow (v26), "No sprite here to follow", and a right click in Select mode while there's nothing to activate, "Nothing here to activate". So does a Grab click (v23): "Nothing here to grab", or on a fixture, "Can't grab the berry bush: it's rooted to the ground"; and a Grab right click with nothing held, led or under the Cursor (v25): "Nothing here to throw or shove", or on a fixture, "Can't throw the thornbush: it's rooted to the ground". On a line too full for it, such as when the Cursor follows a sprite standing on a berry, the reason still shows, and the tile's part of the line is cut short to make room.
  - The tile names its terrain and any object on it, with the object's stage if it has stages: `(61,40) grass · berry bush (mature)`.
  - **Display names** are the data's names with `_` shown as a space (`berry_bush` → "berry bush"), so `objects.ron` needs no separate display name. Its `plural` is the one other form the screen needs (§3.5.1).
- **Overlays** (v29) fill the screen between the top bar and the status line, inside a single-lined border with their name at the left of its top edge and ` esc close ` at the right. The status line stays.
  - **The help screen** (`?`): every key in three columns of groups (time and view; the Cursor; sprites and the inspector), then the colour legend, a sprite glyph in each of the data pack's drives' colours and "none", the emote legend, and the game's folder for its files (§6.7). It fits at 100×30, so it doesn't scroll. `?` or `Esc` closes it; other keys do nothing while it's open.

    ```
    ┌─ Help ─────────────────────────────────────────────────────────────────────────────── esc close ─┐
    │ TIME                            CURSOR                          SPRITES                          │
    │ space      pause / resume       Z X C      select/train/grab    Tab        next sprite           │
    │ .          step                 Q E        left / right click   Shift+Tab  previous sprite       │
    │ + -        faster / slower      Shift+Q E  hug / shock          l          sprite list           │
    │ VIEW                            Ctrl+click hug / shock          r          name it               │
    │ WASD       scroll (Shift: 5)    F          follow a sprite      g          save its genome       │
    │ T          track the selected   middle     follow a sprite      INSPECTOR                        │
    │ b          sprite colours       wheel      change mode          [ ]        switch tabs           │
    │ m          event filter         C again    the Place menu       PgUp PgDn  scroll a tab          │
    │ v          exact detail         hold E     aim, let go to send                                   │
    │ ?          this help                                                                             │
    │ Esc        back, then quit                                                                       │
    │ Ctrl+C     quit at once                                                                          │
    │                                                                                                  │
    │ COLOURS: a sprite's strongest drive, once it's above half                                        │
    │ ☺ hunger  ☺ thirst  ☺ pain  ☺ tiredness  ☺ boredom  ☺ loneliness  ☺ crowdedness  ☺ none          │
    │ EMOTES   ! hurt  ‼ zapped  ? gave up  z resting  ♥ pleased                                       │
    │                                                                                                  │
    │ FILES    C:\Users\Player\AppData\Roaming\terra-sprites                                           │
    └──────────────────────────────────────────────────────────────────────────────────────────────────┘
    ```

  - **The sprite list** (`l`): a row per sprite the policy lets it show (§6.4), with its label, age, strongest drive above .5 in the map's colour (or `-`), and what it's doing as the Body tab's first line says it, in the detail view too.

    ```
    ┌─ Sprites ── sorted by drive ───────────────────────────────────────────────────────── esc close ─┐
    │ Sprite       Age  Drive        Doing                                                             │
    │ Mira #12   3,410  hunger       Going to eat the berry bush · 3 tiles to go                       │
    │ Sprite #7  1,022  thirst       Going to drink · 2 tiles to go                                    │
    │ Kel #4     5,903  -            Resting · 6 ticks left                                            │
    └──────────────────────────────────────────────────────────────────────────────────────────────────┘
    ```

    - It opens sorted as it was last, starting by number, with the selected sprite highlighted, or else the first. `Tab` sorts by number, name (named sprites first, by name, then the rest by number), age (oldest first) or drive (in the data pack's order, strongest first within each, those with none last); `Shift+Tab` goes back. The title says the order.
    - The arrows or the wheel move the highlight, a row in reverse video, which stays on its sprite when the list is re-sorted. A list too long for the screen scrolls to keep it in view.
    - `Enter`, or a click on a row, selects that sprite, centres the view on it and closes the list. `l` or `Esc` closes it without choosing; other keys do nothing while it's open. With no sprites, it reads "No sprites".

### 6.2 Semantic tiles and themes

- **Drawn from meaning:** the map renders **semantic tiles**, e.g. `Terrain(Grass)`, `Object("berry_bush", "fruiting")`, `Sprite { colour_mode_state }`, `Emote(Hurt)`, never characters directly.
- **Themes** (`themes/*.ron`) map each semantic tile to a character, colours and modifiers.
  - Themes are **UI assets**. They aren't part of the sim data pack, and don't affect saves or replays.
  - The future Tiles milestone adds themes that map to a sheet index and tint instead.
- **Objects are keyed by name and visual state**, e.g. `object("berry_bush", "fruiting")`. An object matching none of its visual rules is in the state `"default"`: the bare berry bush is `object("berry_bush", "default")`.
  - **Fallback:** a theme with no entry for an object's state uses that object's `"default"` entry, and failing that a `?` glyph. So a data pack's new object types still draw in any theme.
  - A test checks that each built-in theme covers every visual state of every object type in the built-in pack.
- **All UI text is limited to CP437 characters**, so any CP437 bitmap font or tileset can render every panel.

| Thing | `cp437` (default) | `ascii` (`--ascii`) | Colour |
|---|---|---|---|
| Grass / dirt / sand | `.` `,` `:` | same | green / brown / yellow |
| Shallow / deep water | `~` `≈` | `~` `=` | cyan / blue |
| Rock | `#` | `#` | grey |
| Berry bush: seedling / bare / fruiting | `'` `♣` `♣` | `'` `&` `&` | green / green / **bold red** |
| Thornbush | `♠` | `*` | magenta |
| Berry | `•` | `%` | red |
| Ball | `○` | `o` | white |
| Sprite | `☺` | `@` | by colour mode (§6.3) |
| Selected sprite | `☻` | `@` in reverse video (`&` is the berry bush) | by colour mode (§6.3) |
| Cursor arrows | `↓ ↑ → ←` | `v ^ > <` | the cursor mode's colour |
| Cursor arrows, following (v21; v26 name) | `▼ ▲ ► ◄` | `v ^ > <` (the status line says it's following) | the cursor mode's colour |
| Mode marks: Select / Train / Grab (v21) | `♦` `±` `∩` | `S` `T` `G` | the mode's colour |
| Status marks: idle / grab / release / empty / sent / applied / rejected | `·` `↑` `↓` `░` `+` `☼` `?` | `-` `^` `v` `_` `+` `*` `?` | the mode's colour |
| Decision marker (flashes) | `X` | `X` | white |
| The leash, while the Cursor leads a sprite (flashes, v23) | `·` | `;` | Grab's yellow |
| The aim line, while aiming a throw or a shove (steady, v25): its path / where it ends | `·` `°` | `;` `O` | Grab's yellow |
| Attention marker | the tile's own glyph | the tile's own glyph | dark grey background |

- Every **terrain and object type** on the map, and sprites, has a unique glyph. Colour is used only for **state**, and the status line always names what's under the Cursor. The Cursor's arrows and marks are UI drawn over the map, and may share glyphs with each other (grab and release are arrows).
- **`--ascii` swaps only what themes cover:** the map's glyphs, the Cursor and the emotes. Frames and text stay CP437 in every theme.

### 6.3 Body language (UI only)

**Sprite colour modes** (`b` cycles them; the status line says which for 3 seconds, "Colours: plain", v29):
- **Strongest drive** (v29 name; "dominant drive" before), where the game starts: the highest drive above 0.5, or the sprite's own colour if none is. Of two equally strong, the first in the data pack's order. **Each drive's colour is the theme's** (v29), by the drive's name, `drives: {hunger: yellow, …}`; a drive no theme colours draws in the sprite's own colour. The built-in themes:

| Drive | Colour |
|---|---|
| hunger | yellow |
| thirst | cyan |
| pain | red |
| tiredness | blue |
| boredom | dark grey |
| loneliness | magenta |
| crowdedness | light red |

- **Plain:** the sprite's own colour.
- M2 adds **lineage**.

**Emotes** swap with the sprite glyph at about 2 Hz, for about 1 second of **real** time:

| Emote | `cp437` | `ascii` | Triggered by |
|---|---|---|---|
| Hurt (red) | `!` | `!` | Being hurt by something: a thornbush, a sprite's hit. Not starvation, dehydration or old age, which injure every tick. Built in slice 7a, ahead of the others. |
| Shocked (yellow, v21) | `‼` | `/` | `Corrected`: a zap or a shock, instead of Hurt. Built in slice 10. The owner asked for a lightning bolt (`🗲`), which isn't CP437; the tile front end can draw one |
| Failed (light yellow, v29) | `?` | `?` | `ActionEnded` with outcome `failed`, `blocked` or `timed_out`. Light yellow, since a white `?` is an object the theme doesn't know |
| Resting (light blue, v29) | `z` | `z` | While Rest lasts, and at least a second of real time (v29), so it's seen at every speed |
| Pleased | `♥` | `+` | `Rewarded`: a pet or a hug (built in slice 10); or a tick in which `last_r` reaches **0.3** (v29). Measured over 1.79 million sprite-ticks of the default world, about one good feeling in seven reaches it, such as a meal while hungry |

- Emote timers live in `App` and are driven by **sim events**, so emotes don't vanish at 16× or Max speed. The frame loop runs each tick through `Ticks`, which keeps, tick by tick, the events and the sprites whose `last_r` reached 0.3 then, since the next tick overwrites it; the screen takes them in in tick order, so of two emotes in one frame the later tick's wins (v29).
- **Two at once** (v29): the newest wins, except Resting, which gives way to any other and comes back after it while the rest lasts.
- Body language never changes the sim.

### 6.4 `InfoPolicy`

- **Every** information display asks `InfoPolicy::can_view(panel, subject)` before showing anything. That includes map colours, emotes, the event log, counts in the top bar, the sprite list, the tile-info line and every inspector tab.
- **Panels** (v29): `MapColours`, `Emotes`, `Marks` (the selected sprite's Decision and Attention markers), `EventLog` (a line), `Counts` (the top bar's population), `SpriteList` (a row), `TileInfo` (the status line's part about the tile under the Cursor) and `Tab(tab)`. **Subjects:** `World`, `Sprite(id)` or `Tile(pos)`. An event log line asks about its first sprite, or the world. A display denied shows nothing in its place: a sprite in its own colour, no emote, a blank tab, and the inspector's title without the sprite's name. A denied event log line is left out, so the lines after it close up rather than leave a gap that would show something happened.
- In M1 the answer is always yes (`Omniscient`).
- A future diegetic Play mode is a new policy. The policy only controls what the UI **reveals**. A physical in-world scanner would be a separate sim feature with its own state and commands.

### 6.5 The Cursor

**What it is** (v21): the Cursor is the player's hard-light projection into the terrarium (§0), and the 3×3 grid on the map is its form. In M1 it's invisible to sprites.
- **Visible or invisible** (v21): each cursor mode will have its own switch, and every mode starts invisible. The switch, and what a visible Cursor means to sprites, come with the visible Cursor's slice, 11d ([#60](https://github.com/Keazra/terra-sprites/issues/60)); until then the Cursor is always invisible, and a touch from it is a feeling from nowhere (§4.6, §5.6).

- **Scrolling:** `W` `A` `S` `D` or the arrow keys scroll the viewport one tile, and Shift makes it 5. Holding a key keeps scrolling. The viewport stops at the wall. There is **no keyboard cursor**.
- **The Cursor follows the pointer,** unless it follows a sprite (below). It sits on the tile the pointer **points at** (v27): one up and one left of the tile under the pointer, so the pointer's arrow rests on the Cursor's lower-right corner rather than hiding the tile. On the map view's top row the tile pointed at stays in that row, and on its left column in that column; and on the map view's border just right of or below the tiles, the pointer points at the last column or row, so every tile in view can be pointed at. The Cursor changes as the map scrolls beneath a still pointer. When the pointer leaves the map view, the Cursor stays on its last tile. A click acts on the tile pointed at, as the cursor mode says.
- **`Q` and `E` are the left and right click** (v21), in every mode, acting where the Cursor is: on the tile the pointer points at, or on the sprite it follows. So a following Cursor keeps the keys on its sprite: in Select mode `Q` can't pick up a sprite the pointer happens to rest on. Each acts once per press: holding one doesn't repeat, so a held key can't pour out pets. In Grab mode `E` also has a release, as the right button does: holding it aims a throw or a shove, and letting it go sends it (v25, below). Keyboard players can train without the mouse: `Tab` to a sprite, `F` to follow it, `X` for Train, `Q` to pet.
- **Reach:** the Cursor reaches anywhere on the map.
- **Ownership:** **the Cursor lives in the sim** as far as it touches the world: `World` owns the item it holds or the sprite it leads, one thing at a time, and while it leads, its tile (v23).
  - A held item stays in its World storage with no tile. It's left out of the occupancy index and perception, and its lifecycle keeps running (§3.5.2).
  - A led sprite stays on the map (below).
  - `App` owns where the Cursor is, the cursor mode and Follow, and reads `world.cursor()` to draw what it holds or leads. While the Cursor is invisible to sprites, where it is touches the world only while it leads a sprite: then each move onto a new tile sends `MoveCursor` (§2.5). The visible Cursor (11d) will send it always.

**Leading a sprite** (v23). Sprites are led, never lifted:
- **A Grab-mode click on a sprite takes hold of it,** and the next Grab-mode click lets go. A sprite on the tile comes before an item.
- **The led sprite stays on the map** and walks after the Cursor at its own pace: its speed, its step costs, its energy. It heads for the Cursor's tile, or, if it can't stand there or its flood doesn't reach it, for the tile its flood reaches nearest the Cursor. Held up by another sprite, it waits or goes round, as any walking sprite does.
- **The leash** (v23 change 14): while the Cursor leads a sprite, it goes no further from it than 5 tiles in a square, in every mode. Pointed further, or following a sprite further off, it stops at the leash's end nearest where it would be, and moves on as the led sprite catches up. A click lands where the Cursor is, not past the leash. The leash shows as a dotted line from the Cursor to the led sprite, flashing as the Decision marker does, in Grab's yellow, over empty ground only, so sprites and objects stay visible. Its reach is a UI setting.
- **It chooses nothing while led:** no attention, decision or trace entries (§2.4). So it can't eat, drink, flee or fight back, even when hurt: the Cursor's grip is hard light, stronger than a sprite. Other sprites perceive it as usual, and pets and shocks reach it.
- **It follows the Cursor in every cursor mode.** Leading the followed sprite, Follow steps aside (below), so the Cursor follows the pointer and the sprite follows the Cursor.
- **Let go,** it chooses afresh at its next step 5. Taken hold of mid-action, its action ends `pulled_away` (§5.5).
- **Being led isn't a sense:** a sprite learns nothing from it as such. What the Cursor's handling means to sprites is the visible Cursor's question (11d).

**Throwing and shoving** (v25). In Grab mode the right click sends what the Cursor has hold of: it **throws** a held item, which rolls (§3.5.4), or **shoves** a led sprite, which is let go and slides (§3.5.4). Sprites are never thrown (v23 change 8). The player aims by **pulling back**, like a pool cue:
- **Pressing the right button** (or `E`) **starts aiming** what the Cursor has hold of, whatever the pointer points at. With the Cursor empty, the press first grabs what's under it, as a left click would: the followed sprite, or else the sprite there, or else the item there. With nothing there, it sends nothing, flashes `?`, and says "Nothing here to throw or shove"; on a fixture, "Can't throw the thornbush: it's rooted to the ground".
- **While aiming, the Cursor sits on the thing and stays still,** still gripping it, and the pointer does the pulling. Leading, the Cursor goes onto the led sprite as the press lands, and since a led sprite walks towards the Cursor, it stands still until the aim ends; its body goes on, and pets and shocks reach it. A sprite sliding when the press lands slides on, and the Cursor rides along on it. Holding, the Cursor stays where it was pressed, and the held item is drawn there, under the Cursor, until it's thrown.
- **The player pulls the pointer back** from the Cursor, away from where the thing should go. The thing will go the opposite way to the pull, from where it is, snapped to the nearest of the 8 directions; the pull's length, from the Cursor to the tile the pointer points at, in the game's own measure (a diagonal step is one tile), sets how far, up to the Cursor's furthest for the thing's size (§3.5.4). So pressed with the pointer off a led sprite, the aim already pulls as far as the pointer is from it.
- **The aim line** shows it: steady dots in Grab's yellow from the thing, along its path, as far as the pull sends it, with a small circle where it would stop if nothing's in the way (§6.2). It's drawn over empty ground only, as the leash is, so a bush in the way shows through. It's straight, and doesn't predict bounces.
- **Letting go sends it:** `Throw { from, toward, tiles }`, from where aiming began, or `Shove { toward, tiles }` (§2.5). Letting go with the pointer on the Cursor sends nothing, and `Esc` cancels: either way the thing stays held or led, whether the press grabbed it or it was held already.
- **From where:** a throw starts where aiming began, and is refused, as putting the item down would be, where the item can't go. Following, the Cursor sits on its sprite, so a throw starts at the sprite's feet. A shove starts from wherever the led sprite stands when it applies.
- **With the keyboard,** `E` is the right button: hold it, pull with the mouse, and let it go. In a terminal that doesn't report a key being let go, a second press of `E` sends it. Windows Terminal reports it.
- **A sliding sprite** can be taken hold of: its slide carries on to its end, and then it follows the Cursor. Taking hold of a sprite doesn't lift it, so unlike picking up a rolling item, it can't end the slide.

**The Cursor's form** is 3×3 tiles, centred on its target:

```
M ↓ Y      M: the mode mark (top-left and bottom-right)
→ ☺ ←      Y, N: the status marks (top-right, bottom-left)
N ↑ M      centre: the target tile, in reverse video, glyph still visible
```

- The arrows and marks take the mode's colour. Parts that fall outside the map view aren't drawn.
- The Cursor covers the 8 tiles around its target while it sits there; the status line still names the target.
- The arrows and marks are theme glyphs (§6.2). **Following a sprite,** the arrows are solid, clamping the sprite (`▼ ► ◄ ▲`).

**Follow** (v21 as locking on; v26). The Cursor can follow a sprite, so a pet or a shock needs no aim:
- **Following,** the Cursor sits on its sprite and moves with it, in every mode, whatever the pointer does. If the sprite walks out of view, the Cursor goes with it, and a click still reaches it.
- **`F` turns Follow on or off, in every mode,** and a **middle click** does the same where it points (v26). Following, it stops, wherever the Cursor is. Otherwise it follows the sprite on the Cursor's tile, or, with none there, the selected sprite, so `Tab` then `F` works without the mouse; with neither, it sends nothing, flashes `?`, and says "No sprite here to follow". A middle click off the map view points nowhere and does nothing, as a click there does. Held, `F` acts once, so it can't flicker Follow on and off. A middle click is the mouse, so it leaves the quit prompt open; `F` is a key, and cancels it.
- **Follow is the Cursor's, and the selection the inspector's** (v26). Following a sprite doesn't select it, and selecting another sprite, by a left click on it, `Tab` or `Shift+Tab`, leaves Follow where it is. So the player can read one sprite's Brain tab while petting its neighbour; the status line says which sprite the Cursor follows.
- **A left click on empty ground clears the selection,** and leaves Follow as it is.
- **The followed sprite's death ends Follow,** whichever sprite is selected. The Cursor stays where the sprite died, and follows the pointer again.
- **While the Cursor leads the followed sprite** (v23 change 6), Follow steps aside, in every mode: the Cursor follows the pointer, within the leash. Let go, Follow comes back and the Cursor sits on the sprite again, so the player can lead a sprite to water and go straight back to training it. `F` meanwhile ends Follow for good. Stopping Follow while aiming leaves the aim as it was: the Cursor stays where aiming began until the aim ends.
- **Otherwise Follow holds, whatever the Cursor has hold of.** Holding a berry picked up with Follow off, then following a hungry sprite, a Grab click puts the berry down at its feet. Leading one sprite while following another, the led sprite is led towards the followed one, the Cursor waiting at the leash's end until it catches up.
- **Only sprites are selected and followed** (v23; followed, v26). The selection is sprites only, and a rolling ball stops within a few tiles, so it can be grabbed where it ends up.
- The status line names the sprite the Cursor follows (`following Mira #12`).
- **Follow isn't Track.** `T` (track) has the view follow the selected sprite; Follow has the Cursor follow a sprite.

**Activate** (v26). In Select mode the right click (or `E`) activates the thing clicked, such as a device: a hatchery, a food dispenser ([#26](https://github.com/Keazra/terra-sprites/issues/26)). Activating needs something selectable to activate ([#90](https://github.com/Keazra/terra-sprites/issues/90)); until then there's nothing, so it sends nothing, flashes `?`, and says "Nothing here to activate". It touches neither the selection nor Follow.

**Cursor modes:**

| Key | Mode | Mark | Colour | A left click (`Q`)… | A right click (`E`)… | `Y` / `N` |
|---|---|---|---|---|---|---|
| `Z` | **Select** (the default) | `♦` | white | selects the sprite pointed at; on empty ground, clears the selection | activates the thing clicked; nothing can be activated yet (v26, above) | `·` / `·` |
| `X` | **Train** (v21) | `±` | light magenta (v22) | rewards the target: a **pet**; amplified, a **hug** | corrects the target: a **zap**; amplified, a **shock** | feedback (below) |
| `C` | **Grab** (slice 11a) | `∩` | yellow | with the Cursor empty, grabs what's under it: takes hold of a sprite (`TakeHold`), or else picks up an item (`PickUp`); leading, lets go (`LetGo`); holding, puts the item down (`PutDown`). Pressing `C` again opens the Place menu (slice 11c) | throws a held item or shoves a led sprite, aimed by pulling back; with the Cursor empty, grabs what's under it first (v25, above) | empty: `↑` / `░`; holding or leading: `↓` / the thing's glyph |

- **Train's target** is the sprite the Cursor follows, or else the sprite on the Cursor's tile. A pet or hug sends `Reward` (§2.5) with its reach back; a zap or shock sends `Correct`, which looks back only the touch window. So at high speed a pet still finds what the player meant, and an exact shock needs a pause.
- **Amplifying:** Shift with `Q` or `E`, or Ctrl with a click. Shift with a click can't be used: Windows Terminal keeps Shift+click for selecting text while a program has the mouse.
- **The reach back** a Reward carries is about **two seconds of the player's time** at the current speed, in ticks, never below `touch_window` or above `max_reach_back` (Appendix B): 3 at 1× and slower, 5 at 2×, 10 at 4×, 20 at 8×, 40 at 16× and Max. While paused it's the reach back of the speed time will resume at. The two seconds are a UI setting, tuned in play.
- **The mouse wheel cycles the modes:** a notch down picks the next (Select → Train → Grab, then back to Select), a notch up the previous. A wheel event also points, like every mouse event. The wheel doesn't scroll the map. It cycles the modes over the map view; over the inspector, it scrolls the open tab instead (§6.1); elsewhere it does nothing (v22). Built in slice 10b; Grab joins in slice 11a.
- **Place menu** (v28): `C` pressed again in Grab mode opens it over the map's top-left corner, inside the map view; a list too long for it scrolls to keep the highlighted item in view. It lists the object types whose data offers them (§3.5.1), by label, in ID order (berry bush seedling, berry, ball), then **new sprite** (the starter genome with spawn variation) and **sprite from a genome file**, which lists the `.ron` files in the genomes folder (§6.7) by name. A number key (`1`–`9`), the arrow keys and `Enter`, or a click picks an item; a click off the menu, or `Esc`, closes it. A file that doesn't read is refused with why: "Couldn't read mira-12: …".
  - **The chosen item waits on the Cursor,** in reserve through other modes, until a Grab-mode left click places it: `Place { tile, object_type }`, or `SpawnSprite { tile, genome }` carrying the file's genome in full. One click places one. While it waits, Grab's marks show `↓` and the item's glyph, the status line says `placing: berry` in every mode, and in Grab mode the hints read `click to place  right-click put away`.
  - **A right click puts it away** (v28); it doesn't grab or aim.
  - **It isn't held** (v28): it waits on top of what the Cursor leads or holds, and placing leaves that as it was, so a led sprite can be fed where it stands.
- **Feedback:** in Train mode, each click flashes `+` in both status marks at once ("sent"). When the sim reports back, they flash `☼` (applied) or `?` (rejected). Flashes last about 0.3 s of **real** time and are driven by events, like emotes (§6.3). In Grab mode the marks show the Cursor's state instead, and a rejected `TakeHold`, `PickUp`, `LetGo`, `PutDown`, `Throw`, `Shove` or `Place` flashes `?`.
  - **Both flashes show, at every speed** (v22). At speed a command can apply in the same frame it's sent, so `☼` or `?` waits until `+` has shown for its 0.3 s. `?` wins over `☼`: a command applied in the same tick as a refusal, or while its `?` is still to show or showing, doesn't replace it. The marks follow the latest click: a click that sends puts any earlier click's result not yet shown behind it.
  - **While paused,** `+` flashes at each click, and `☼` or `?` when time moves and the commands apply.
  - A flash under way carries on through a change of mode, in the new mode's colour.
- **Nothing to act on:** a click in Train or Grab mode, a right click in Select mode, or `F`, with nothing to act on sends no command and flashes `?` at once. The status line says so (§6.1): "No sprite here to pet".
- **Switching modes keeps each mode's state.** What the Cursor holds or leads, or a Place item not yet put down, waits in reserve while other modes are in use, and a led sprite keeps following the Cursor (v23); the selection and Follow stay in every mode. While the Cursor holds or leads something, the status line shows it in every mode (`leading: Mira #12`, `holding: berry`), because a led sprite can't eat or drink (§2.4) and a held berry can expire.
- **Leaving a mode:** `Esc` returns to Select, keeping Follow. `Esc` first closes any open menu or overlay. From Select, `Esc` asks to quit (§6.6). A right click no longer returns to Select (v21).
- **Pausing queues actions.** Commands are stamped for the next tick (§2.5), so clicks made while paused apply, in click order, when time next moves (`space` or `.`). Grab mode's marks follow the queue: after a queued `TakeHold` or `PickUp`, `Y` shows `↓` and `N` the thing being grabbed; after a queued `Throw` or `Shove` (v25), they show the Cursor empty. If the sim rejects it, `?` flashes and the marks return to the Cursor's real state.

**Other keys:**

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Select the next / previous sprite by ID, wrapping around, and centre the viewport on it if it's out of view. With none selected, start from the lowest / highest ID; after the selected sprite dies, carry on from its ID. Follow stays where it is |
| `PgUp` / `PgDn` | Scroll the open inspector tab by a page |
| `T` | Track: the view follows the selected sprite (v21; `f` before), or stops. With none selected, "Select a sprite to track it" (v29) |
| `F` | **Follow** (v26): the Cursor follows the sprite under it, or else the selected sprite, or stops (above). The middle mouse button does the same where it points. Not a cursor mode |
| `b` | Cycle the colour mode |
| `m` | Cycle the event log's filter: all, selected, major (v29) |
| `[` / `]` | Previous / next inspector tab, wrapping around |
| `g` | Save the selected sprite's genome to the genomes folder (§6.7), as `mira-530-tick-5310.ron`; the status line says where (v28) |
| `r` | Name the selected sprite: a name you type, or one generated at random |
| `l` | Open or close the sprite list |
| `v` | Switch the detail view on or off: the exact action line (§6.1) |
| `?` | Open or close help |
| `Esc` | Close a menu or overlay; otherwise back to Select; from Select, ask to quit |
| `Ctrl+C` | Quit at once |

- A rejected command shows its reason on the status line, for about 3 seconds in the key hints' place (§6.1, v22), and in the event log.
- **Names:** a sprite has no name until the player gives it one with `r`, either a name they make up or one generated at random. The player has to care first. Until then the screen shows a sprite by its ID, as "Sprite #530", and once named, as "Mira #530".
  - **Naming** (v28): `r` with a sprite selected opens a prompt on the status line, `Name Sprite #530: Tobek_   enter ok  tab another  esc cancel`, offering a random name. Typing replaces it, `Backspace` rubs out, `Tab` offers another, `Enter` sends `Rename` and `Esc` gives up. While it's open, keys type rather than act. Only letters CP437 can show, and spaces, are typed, up to 16. With no sprite selected, `r` says "Select a sprite to name it".
  - **A random name** (v28) is made up by the screen from the data pack's syllables (`names.ron`): a first syllable, sometimes a middle one, and a last. It's a pure function of a seed, so it never draws from the world's RNG, and the syllables are checked at load to make only valid names.

### 6.6 Time and the frame loop

- **Keys:** `space` pauses and resumes; `.` steps while paused (below); `+` and `-` step through ⅛×, ¼×, ½×, 1× (1.25 ticks per second), 2×, 4×, 8×, 16× (20 ticks per second) and **Max**. Each press halves or doubles the rate. The game starts at 1×; `-` stops at ⅛× (5/32 of a tick a second) and `+` at Max. The top bar labels the slow speeds `1/2x`, `1/4x` and `1/8x`, and shows the speed while paused too, as `|| paused 4x` (v27).
- **A step** (v27) runs one real second's worth of ticks at the speed, rounded down, at least 1 and at most 20: 1 at 1× and slower, 2 at 2×, 5 at 4×, 10 at 8×, 20 at 16× and Max. It runs within the frame's budget like any other ticks, and what the budget cuts short runs on the next frames. A press before a step has finished starts a whole step afresh, at the speed then, rather than adding another. Resuming drops what's left of it.
- **Quitting:** `Esc` (from Select, with no menu open) asks "Quit? (y/n)". `y` or a second `Esc` quits; any other key cancels. `Ctrl+C` quits at once.
- **Exact pacing:** the UI clock counts owed ticks in integer maths, with rates in 32nds of a tick per second, so every speed (including ⅛×) runs at exactly its nominal rate with no drift.
- **Held keys:**
  - **1× is a stop for held keys.** A held `+` or `-` stops at 1×; a fresh press is needed to go past it, in either direction.
  - **16× is a stop for a held `+`.** Max runs as fast as the computer allows, so reaching it takes a fresh press. A held `-` coming down from Max passes 16× and stops only at 1×.
  - **`space` toggles pause only on a fresh press,** so holding it doesn't flicker.
  - **Holding `.` keeps stepping,** a step per repeat, which don't pile up (above).
  - **Telling a hold from a press:** a key counts as held when the terminal reports it as repeating, or when it's pressed again with no release in between. The second rule is only trusted where the terminal reports releases: Windows always does, as does a terminal with the kitty keyboard protocol on (below), and any other terminal is trusted once a release arrives. Keys are tracked by physical key, so a `+` whose release is reported as `=` (Shift released first) still counts as released.
  - **The kitty keyboard protocol** (v27): where the terminal supports it (kitty, WezTerm, Ghostty, recent foot), the game turns it on at start, asking for every key, `+` and the letters included, to be reported with its repeats and releases. It's turned off again on exit and in the panic hook, before the alternate screen is left, since the terminal keeps it per screen. Shift and the lock keys then arrive as keys of their own, and do nothing. Such a terminal reports a shifted key as the key with Shift held, so `>`, `_`, `{` and `}` act as `.`, `-`, `[` and `]` there, where other terminals ignore them; and Caps Lock doesn't count as Shift, so it neither amplifies `Q` and `E` nor makes scrolling jump. Windows reports releases without it.
  - **Limitation:** terminals that report neither repeats nor releases, and don't support the kitty protocol, can't tell a hold from taps, so every press there counts as fresh.
- **Single-threaded in M1.** Each frame:
  1. drain input into `Action`s and `Command`s
  2. run as many ticks as the speed allows, within a **~25 ms** budget
  3. render, at about 30 fps
- At Max, the sim uses whatever time rendering leaves.

### 6.7 Files

- **Data:** the default data pack is embedded in the binary. `--data <dir>` overrides it for **new** worlds (§2.8).
- **Saving and loading:** `F5` quicksaves, `F9` quickloads, and `Ctrl+S` / `Ctrl+O` save or load by name.
- **Autosave:** every 10 real-time minutes, keeping the last 3.
- **Location:** saves, autosaves, exported genomes and `last_session.replay` go in the platform data folder: `%APPDATA%\terra-sprites` on Windows, `~/Library/Application Support/terra-sprites` on macOS, and elsewhere `$XDG_DATA_HOME/terra-sprites` or `~/.local/share/terra-sprites`. The help overlay shows the path.
  - **Genomes** (v28) are exported to, and the Place menu reads them from, its `genomes` folder.
- **Command-line flags:** `--seed <n>`, `--preset <file>`, `--data <dir>`, `--ascii`, `--replay <file>`.

### 6.8 UI architecture

- **`App`** holds the UI state: where the Cursor is, cursor mode (and each mode's reserve), Follow (v21 as the lock), viewport, selection, screen state (Normal / Menu / Prompt / Help / Sprite list), colour mode, emote and feedback timers, and the active `InfoPolicy`.
- **Input:** a keybinding table maps keys to UI `Action`s. Each `Action` either changes `App` or produces a `Command`.
- **Rendering** is a pure function, `render(frame, &App, &World)`.
- **Panic hook:** it restores the terminal and flushes the replay.

---

## 7. Testing and acceptance

Implementation is **test-first, one vertical slice at a time.** Everything in `terra-sim` is deterministic for a given seed, so no test is flaky: results change only when the code or the tuning does.

### 7.1 Tools

- **`World::state_hash()`:** xxh3 with a fixed seed over a canonical serialization of the world.
- **Replay checkpoints:** a hash every 1,000 ticks, and playback reports the first divergence (§2.7).
- **`World::check_invariants()`:** runs at the end of every tick, after step 7, in debug builds and tests, and panics naming the tick and the broken rule, so a violation fails on the tick that caused it. On the default map it costs about 1.2 ms a tick in a debug build (a tick itself took 0.8 ms at slice 5), which adds about 30 s to the debug test run; release builds skip it. It checks:
  - the occupancy index matches entity positions
  - concentrations are within [0, 1]
  - no tile holds more than one object, and every solid object stands on terrain that allows fixtures
  - held items appear in neither the index nor perception, and a led sprite is on the map like any other
  - IDs only go up
- **Scripted actions:** a hand-made world (`Scenario`) can start sprites on scripted actions, a Wander to a tile or a Rest, in order, before the stand-in or the brain chooses for them. Tests and lab scenarios use them to set up exact situations, such as two sprites meeting in a corridor.
- **Lab runner:** `cargo run -p terra-sim --example lab -- scenarios/<name>.ron --seeds 10` prints metrics. The scenarios double as tests and as the main tuning tool.
  - **A lab scenario** is RON text: the world, either a hand-drawn map (rows of the ascii theme's terrain glyphs, as test maps are drawn, and a key naming an object type or a sprite for any other character, which stands on grass) or a world generated from the built-in default preset (a named preset waits for a scenario that needs one); how many ticks to run; and the windows to count in, as tick ranges.
  - **Metrics,** per seed and window: applied actions by verb and target type (`eat thornbush 23`), and deaths by cause. The runner prints each seed's, then the median across seeds.
  - **A control run** (v16): a scenario may ask for each seed to run twice, the second time with learning switched off world-wide (§5.6). Nothing else differs: learning draws no random numbers, so the two runs part only where what was learned changes a choice. The runner prints both.
  - `terra-sim` parses a scenario from text and runs it for a seed; only the example reads files, as the crate has no filesystem access. Tests `include_str!` the same files.
- **Trainer hook:** a scenario can include a trainer. It reads tick *t*'s events and submits commands through the same queue as the player, stamped *t+1*. What it does is set in the scenario file (v21): the verb and target type it answers (an applied Play on a ball), what it gives the actor (a pet or a zap; amplified, a hug or a shock), how many ticks it waits first (0 by default, so the command is stamped *t+1*; a player at 8× might take 20), the reach back its pets carry (`touch_window` by default; a trainer that zaps or shocks takes none, v21 change 6), and the tick training stops (9,900, §7.3). A scenario with a trainer asks for a control run **without the trainer**, learning still on.
- **Lessons** (v21): the report also counts each run's lessons (`LearnedMilestone`s) by what was learned, so a trained run can be checked for lessons its control didn't learn.

### 7.2 Test layers

1. **Unit tests (`terra-sim`):**
   - **Biochemistry:**
     - half-life factors, reaction extents, emitter Level/Rise/Fall, receptors
     - the two-pass emitter order: a pulse-keyed drive drop produces its Fall reward in the same tick
     - the gene restriction table (genes that break it are flagged, not expressed)
     - spawn variation changes only value fields, and a varied starter genome has no flagged genes
     - the pulse latch
   - **Brain:**
     - concept products and negation, input groups, verb kinds, and every interaction offered on any target
     - sampling only at boundaries, deterministic switching and tie-breaks
     - Wander destination sampling, Retreat's Chebyshev step choice and direction order, action bounds and outcomes
     - one trace entry per tick, including while an action continues
     - learning consumes reward and punishment, and `last_r` records relief and reward less punishment
     - relief: a need's fall at or over the deadband, and not a slower one; pain's fall is never relief
     - worth goes to the thing touched, within the touch window; nothing touched, or only an attacker, no worth
     - each need's worth counts in proportion to that need; bad counts always
     - habits along the trace with ring-buffer weighting; disappointment only for a fruitless try with a motive
     - the motive: the need whose instinct pushed the verb most, ties to the lower input
     - fading at each channel's rate; familiarity and the worth of new things
     - lessons once per learned value; a control run learns nothing
   - **World:**
     - flood reachability, candidate tie-breaks, goal tiles
     - integer move points, conflict order, head-on swaps (including diagonal corner rules)
     - blocked re-planning: the one-off search treats occupied tiles as impassable; a found path is committed and survives flood refreshes; it's dropped when blocked again or when a sprite target moves; no path ends in `blocked`
     - re-planning toward a moving target without re-flooding
     - every rule condition and effect, short-circuiting, held-object semantics
     - verb-only effects in lifecycle rules are load errors; objects created during step 2 first run next tick
   - **Commands:** stamping for the next tick, applying in submission order, the amounts each injects and the pulse it sends, levels capped at 1 however many land in a tick, every rejection path, and the reach back's bounds.
   - **Throws, shoves and crashes** (v25): a throw's bounds and where it starts; a slide's timing, what stops it, and what a crash is with; a crash runs the thing's tags, counts as the thing touched, and teaches no habit, not even along the trace; a sliding sprite taken hold of slides on to the end.
   - **The Cursor's reach back** (v21): its feeling goes to the latest attempt within the reach back, at full weight, and not to an older one or along the trace; with no attempt within the reach back, the tick teaches as any other; relief keeps the touch window; a `was_hit` pulse still makes punishment teach fear.
2. **Property tests (`proptest`):**
   - The map is connected for any seed.
   - Among random placements and removals, placing a solid object where `KeepsPathsOpen` holds never splits the open ground into more pieces.
   - Concentrations always stay in bounds.
   - **Genes can't change physical chemistry directly.** This runs on the pure step-3 function with identical scripted physical inputs for 1,000 ticks: step flags, `resting`, verb injections, pulses and `nearby_sprites`. A random genome runs beside a control genome, and **their physical chemical arrays must be bit-identical on every tick**. Both genomes share identical `Trait` genes, since traits change physical costs legitimately; every other gene type varies randomly.
3. **Determinism tests:**
   - The same inputs give the same hash on every tick.
   - Save at tick *k*, load, and continue: the result must match an uninterrupted run hash for hash.
   - Replays round-trip. Mismatched versions or data packs are refused.
   - A deliberately introduced divergence is detected at the right checkpoint.
4. **Compatibility tests:**
   - Golden files: the v1 save, the starter genome in RON, and a genome containing unknown gene types. All must load.
   - Unknown genes survive load → save byte for byte.
5. **Behaviour scenarios** (§7.3).
6. **UI tests:**
   - Keybindings map to the right `Action`s and `Command`s.
   - ratatui `TestBackend` snapshots cover each inspector tab, the help screen and the sprite list in both themes (v29): text files in `crates/terra-tui/tests/snapshots/`, written afresh with `UPDATE_SNAPSHOTS=1` and reviewed like code.
   - A deny-all `InfoPolicy` blanks **every** display.
   - Emote timers work from events.
   - The too-small-terminal message appears.
   - All UI strings are within CP437: a test plays the default world and checks every cell of every screen, prompt and overlay (v29).
7. **Benchmarks (`criterion`, run locally):**
   - the whole tick with 100 sprites
   - one sprite's flood
   - one sprite's brain step

### 7.3 Behaviour scenario protocol

**Protocol** (A2 and A3; A1 has no trainer, and its control is a run without learning):
- **Training:** the trainer reacts only to events from ticks **0–9,899**.
- **Washout:** ticks 9,900–9,999 have no trainer. Any command the trainer queued lands before measurement begins, and the last training reward has been consumed.
- **Measurement:** ticks 10,000–20,000, with **no trainer in either run**, so we measure what was learned, not ongoing reward.
- **Counts:** only actions with outcome `applied` are counted.
- **Control:** each trained run is compared with a matched control (same seed, no trainer, learning on), taking the **median across 10 seeds**. As in A1, the trained runs' median is compared with the controls' median (v21).
- **The trainer** pets for A2, the light touch, and shocks for A3 (v21 change 15), each with no delay and the default reach back (v21).
- **Minimum baseline:** a scenario only counts if the control has **≥20 counted actions** in the measurement window. Below that, the test **fails** as badly calibrated.

| # | Scenario | Trainer during training | Pass condition |
|---|---|---|---|
| A1 | 1 sprite; berry bushes, thornbushes, water. The world may be shaped to meet the baseline: a small walled arena where thornbushes outnumber berry bushes | none | Applied thornbush contacts (Eat, Play and Hit on a thornbush) over ticks 0–20,000 ≤ 50% of the control's, where the control is the same seed with learning switched off (v16, §7.1). The control needs ≥20 such contacts; below that the test fails as badly calibrated. The learning sprite must live through the run. |
| A2 | 1 sprite; balls, food, water | Each applied Play on a Ball at tick *t* < 9,900 → a pet, `Reward(actor)`, stamped *t+1* | Applied Play-on-Ball count ≥ 1.5× control |
| A3 | 4 sprites in a small arena; food, water | Each applied Hit on a Sprite at tick *t* < 9,900 → a shock, `Correct(actor, amplified)`, meaning **the hitter**, stamped *t+1* | Applied Hit-on-Sprite count, all sprites combined, ≤ 0.5× control |

**Measured with slice 10's PR** (v21), not CI tests:
- **The strength sweep:** A2's and A3's trained runs with the pet or zap at a few strengths (¼, ½ and full), to see how many a lesson takes and what they do to the result.
- **A late trainer:** A2 with the trainer waiting 20 ticks before each pet and a reach back of 20, as a player at 8× might (`scenarios/a2-late-trainer.ron`). **The bar,** agreed before measuring: A2 still passes (≥ 1.5× control), and the median count of odd lessons, those the trained sprite learns and its control doesn't, about anything but balls, is 0. **Measured:** 1.8×, and a median of 2.5 odd lessons, so the second part missed; the numbers went to the owner, who kept the reach back for pets and set shocks to the touch window (v21 change 6).

**The thorn trap** (slice 8, v12 change 11) is a lab scenario, not a CI test: the default world for 30,000 ticks over 10 seeds, counting deaths by thornbush. It passes if the median is a third of `main`'s before learning, or less. Ten runs of 30 sprites are too slow for CI, so the PR reports both sets of numbers, and each baseline run records it for `main` (§7.6, v24).

**Slice 11b's proof** (v25): moving the thornbush's pricks into the `Thorny` tag (§3.5.6) must leave every lab scenario's report identical to `main`'s, byte for byte, and so must throwing, shoving and crashing, since no lab scenario throws or shoves.

### 7.4 M1 acceptance criteria

M1 is **done** when all of the following hold:

| # | Criterion | Measured by |
|---|---|---|
| A1 | The thornbush lesson (§7.3) | Scenario test |
| A2 | Reward training (§7.3) | Scenario test |
| A3 | Correct training (§7.3) | Scenario test |
| A4 | **Viability.** In the default world, ≥80% of sprites survive the first 10,000 ticks, and starvation plus dehydration cause <25% of deaths over the first 50,000 ticks (median of 10 seeds) | Baseline run's viability run (§7.6) |
| A5 | **Performance.** 100 sprites at ≥200 ticks per second in a release build on the development machine | Benchmark |
| A6 | **Determinism and persistence.** Test layers 3 and 4 pass | Tests |
| A7 | **Robustness.** A headless soak of 1,000,000 ticks at Max speed with a random script of the Cursor's commands produces no panics and no invariant violations | Baseline run's soak (§7.6) |
| A8 | **Playability.** Every feature in §3–§6 works in Windows Terminal, and a forced panic leaves the terminal usable | Manual checklist |
| A9 | **Docs.** A README covering controls; **every user-editable format** (`pack`, `terrain`, `objects`, `chemicals`, `loci`, `brain_io`, `physiology`, `themes/*.ron`, genome RON, world config and presets, lab scenarios), including the left-to-right condition semantics and the location-before-`Chance` convention; and the replay workflow. Saves and replays are documented as opaque. | Review |

The thresholds in A1–A4 are **starting calibrations**. If tuning shows one is badly calibrated, it changes through an amendment to this document, not by deleting the test.

### 7.5 CI (once a GitHub remote exists)

- **Platforms:** Windows and Linux for the full test suite.
- **Determinism matrix,** covering both architectures:

| Runner | Target |
|---|---|
| Linux (`ubuntu-latest`) | x86_64 |
| Windows (`windows-latest`) | x86_64 |
| macOS (`macos-latest`, Apple silicon) | aarch64 |
| Linux (`ubuntu-24.04-arm`), if the repo's plan includes arm64 runners | aarch64 |

- **Checks:**
  - `cargo fmt --check`, `clippy -D warnings`, and all tests
  - a **cross-platform determinism check**: one fixed scenario runs on every runner in the matrix, and all the state hashes must match. Once it passes, §2.6's cross-platform promise applies to **exactly the targets in the matrix**. Adding a target to the promise means adding it to the matrix.

### 7.6 Baseline runs

What's too slow for CI is measured on `main` instead, each time it moves, by a **baseline run** (v24). It runs on the development machine in a clean checkout of `main`, never in a working folder. It's started daily, and skipped when `main` hasn't moved since the last one.

- **What it measures:**
  - **The viability run:** the default world on seeds 1–10, 50,000 ticks each, with no player. It gives:
    - A4: survival at tick 10,000, and starvation and dehydration's share of deaths over 50,000 ticks, as the median of 10 seeds
    - the thorn trap's count: deaths by thornbush in ticks 0–30,000, the same worlds as its lab scenario (§7.3)
    - per seed, who survived, what killed the rest, and how often each verb was applied.
  - **A1–A3's numbers:** each scenario's medians, for its runs and their controls, on CI's seeds. CI stays the judge; the report records them so that a change shows.
  - **The soak (A7),** once slice 17 ([#18](https://github.com/Keazra/terra-sprites/issues/18)) builds it: a million ticks with a random script of the Cursor's commands, on a new seed each run.
- **The baseline report** names the commit it measured and the one it's compared with, and opens with every number that moved.
  - Each criterion is **met**, **not met yet** or **no data**, against §7.3 and §7.4's pass marks exactly, so a share of no deaths is no data. A criterion that a tuning slice hasn't yet reached names that slice.
  - **Broken** means only a panic or a broken invariant (a crash is a shoved sprite's, §3.5.4). It's given with the seed, what it said, which names the tick for a broken invariant, and the command that replays it. The seeds that finished are still reported, and a median over fewer seeds than ran says so.
- **The observer,** an AI agent, reads the report, the code and the commits since the last one, and writes the **briefing**. The briefing gives, in this order: in plain words, what was tested and what changed; the numbers that moved and what might explain them; anything that looks wrong, and how sure it is. The observer changes nothing itself. The launcher does everything that has an effect, and files an issue, or comments on an open one, only for a broken run; the issue carries the observer's guess at the cause, labelled as a guess, which the owner is happy to see on GitHub.
- **Open:** A4's share of deaths means little when few sprites die. At v23, 5 of 300 sprites died in 50,000 ticks, and five of the ten seeds had no deaths, so a median of their shares has no value. Its wording is for slice 17 to settle, alongside the tuning.

---

## Appendix A — Initial registries

IDs are stable and append-only. Gaps are left for growth, and the ranges are a convention, not a rule.

**Gene types:**

| ID | Gene |
|---|---|
| 1 | HalfLife |
| 2 | Reaction |
| 3 | Emitter |
| 4 | Receptor |
| 5 | InitialConcentration |
| 6 | Trait |
| 7 | BrainParam |
| 8 | Instinct |
| 9 | AttentionInstinct |

**Chemicals:**

| ID | Chemical | Class |
|---|---|---|
| 1 | energy | Physical |
| 2 | hydration | Physical |
| 3 | stamina | Physical |
| 4 | food | Physical |
| 5 | water | Physical |
| 6 | injury | Physical |
| 16 | hunger | Signal (drive) |
| 17 | thirst | Signal (drive) |
| 18 | pain | Signal (drive) |
| 19 | tiredness | Signal (drive) |
| 20 | boredom | Signal (drive) |
| 21 | loneliness | Signal (drive) |
| 22 | crowdedness | Signal (drive) |
| 23 | reward | Signal |
| 24 | punishment | Signal |
| 32–47 | h0–h15 | Hormone |

**Loci** (chemical levels are referenced as `Chem(id)`):

| ID | Locus | Kind |
|---|---|---|
| 1 | always | Body sensor |
| 2 | age | Body sensor |
| 3 | nearby_sprites | Body sensor |
| 4 | moving | Body sensor |
| 5 | resting | Body sensor |
| 32 | ate | Pulse |
| 33 | drank | Pulse |
| 34 | played | Pulse |
| 35 | played_social | Pulse |
| 36 | pricked | Pulse |
| 37 | was_hit | Pulse |
| 38 | did_hit | Pulse |
| 39 | petted | Pulse |
| 40 | shocked | Pulse |
| 41 | cornered | Pulse |
| 42 | fruitless | Pulse (v16) |
| 64 | learning_rate_mod | Receptor target |
| 65 | exploration_mod | Receptor target |
| 66 | curiosity_mod | Receptor target (v16) |

**Categories** (in `categories.ron` from v19, §3.5.5):

| ID | Category | Object types |
|---|---|---|
| 1 | bush (was BerryBush) | berry_bush, thornbush (from v20) |
| 2 | fruit (was Berry) | berry |
| 3 | *retired* | was thornbush until v20; never reused |
| 4 | water | water |
| 5 | toy (was Ball) | ball |
| 6 | sprite | sprite |

**Object types:**

| ID | Object type |
|---|---|
| 1 | berry_bush |
| 2 | berry |
| 3 | thornbush |
| 4 | ball |
| 100 | water (pseudo) |
| 101 | sprite (pseudo) |

**Brain inputs:**

| IDs | Inputs | Group |
|---|---|---|
| 1–7 | drives (hunger … crowdedness) | State |
| 8–23 | h0–h15 | State |
| 24 | nearby_sprites | State |
| 25 | age | State |
| 26 | always | State |
| 27–35 | ate, drank, played, played_social, pricked, was_hit, did_hit, petted, shocked | State |
| 36–41 | `attended_<category>` for categories 1–6 (v19: `35 + id`; from category 7, `37 + id`, §3.5.5). 38 was `attended_thornbush`, retired with category 3 (v20) | Target |
| 42 | TargetDistance | Target |
| 43 | TargetAdjacent | Target |
| 44–63 | kept for Target inputs | Target |
| 64 | cornered | State |
| 65 | fruitless | State (v16) |

**Verbs (outputs):**

| ID | Verb | Status |
|---|---|---|
| 1 | Approach | |
| 2 | Eat | |
| 3 | Drink | |
| 4 | Hit | |
| 5 | Play | |
| 6 | Retreat | |
| 7 | Rest | |
| 8 | Wander | |
| 9 | Mate | reserved for M2 |
| 10 | Speak | reserved for M4 |

**Brain parameters** (in `BrainParam` genes, and in `physiology.ron` by name):

| ID | Parameter |
|---|---|
| 1 | learning_rate (η) |
| 2 | trace_decay (λ) |
| 3 | relax_rate |
| 4 | consolidate_rate |
| 5 | tau_base |
| 6 | tau_att_base |
| 7 | switch_margin |
| 8 | attention_margin |
| 9 | salience_gain |
| 10 | pool_size |
| 11 | max_arity |
| 12 | recruit_threshold |
| 13 | novelty_threshold |
| 14 | forget_ticks |
| 15 | worth_rate_good |
| 16 | worth_rate_bad |
| 17 | worth_fade_good |
| 18 | worth_fade_bad |
| 19 | habit_rate |
| 20 | habit_fade |
| 21 | value_gain |
| 22 | curiosity |
| 23 | familiarity_rate |
| 24 | disappointment |
| 25 | individual_rate_good (v18) |
| 26 | individual_rate_bad (v18) |
| 27 | fear_rate (v18) |
| 28 | fear_fade (v18) |
| 29 | generalise (v18) |
| 30 | vigilance (v18) |
| 31 | flight (v18) |
| 32 | fear_reach (v18) |
| 33 | generalise_types (v19) |
| 34 | quieting (v21) |
| 35 | habit_fade_bad (v21) |

Since v16, `learning_rate`, `relax_rate` and `consolidate_rate` do nothing (learning is worth and habits, §5.6), nor do 10–14 while the recruitable pool is on hold (§5.4). Their IDs stay taken.

**Traits:**

| ID | Trait |
|---|---|
| 1 | speed |
| 2 | sense_radius |
| 3 | lifespan |

## Appendix B — `physiology.ron` contents

This file fixes the **mechanisms and ranges**. The starting values are tuned with the lab runner.

- **Newborn physical levels:** energy, hydration, stamina, injury (0), gut contents (0). The first population starts with energy and hydration between 60% and 100% of these (§3.2).
- **Metabolism:**
  - basal energy per tick
  - cost per tile of sense radius
  - base step energy cost × (speed / 8)²
- **Digestion:** `food → energy` and `water → hydration` rates.
- **Hydration, stamina and healing:** hydration loss per tick; stamina drain per step, recovery while idle, recovery while resting; healing per tick.
- **Injury from starvation, dehydration and old age:** amounts per tick, and the fade time of the cause-of-death tallies (they halve about every 350 ticks, §4.10).
- **First-cut timescales,** for a sprite at rest, which the starting values aim for: full hydration lasts about 3,000 ticks and full energy about 6,000; once either runs out, injury kills in about 1,000 more; a sprite past its lifespan dies within about 2,000.
- **Trait ranges:** speed 4–12, sense_radius 6–14, lifespan 20,000–200,000.
- **BrainParam ranges (initial),** each with a default for a genome that has no gene for it:

| Parameter | Range | Default |
|---|---|---|
| η (`learning_rate`) | 0.001–0.5 | 0.05 |
| λ (`trace_decay`) | 0.5–0.99 | 0.9 |
| `relax_rate` | 0–0.01 | 0.001 |
| `consolidate_rate` | 0–0.001 | 0.0001 |
| `tau_base` | 0.05–2.0 | 0.2 |
| `tau_att_base` | 0.05–2.0 | 0.2 |
| `switch_margin` | 0–1 | 0.2 |
| `attention_margin` | 0–1 | 0.2 |
| `salience_gain` | 0–2 | 0.5 |
| `pool_size` | 0–64 | 32 |
| `max_arity` | 1–3 | 3 |
| `recruit_threshold` | 0–1 | 0.3 |
| `novelty_threshold` | 0–1 | 0.5 |
| `forget_ticks` | 100–100,000 | 5,000 |
| `worth_rate_good` | 0–1 | 0.5 |
| `worth_rate_bad` | 0–1 | 0.8 |
| `worth_fade_good` | 0–0.01 | 0.0002 |
| `worth_fade_bad` | 0–0.01 | 0.00005 |
| `habit_rate` | 0–1 | 0.3 |
| `habit_fade` | 0–0.01 | 0.0002 |
| `value_gain` | 0–2 | 1.0 |
| `curiosity` | 0–1 | 0.3 |
| `familiarity_rate` | 0–0.05 | 0.002 |
| `disappointment` | 0–1 | 0.3 |
| `individual_rate_good` (v18) | 0–1 | 0.8 |
| `individual_rate_bad` (v18) | 0–1 | 0.9 |
| `fear_rate` (v18) | 0–1 | 1.0 |
| `fear_fade` (v18) | 0–0.01 | 0.00005 |
| `generalise` (v18) | 2–10 | 3 |
| `vigilance` (v18) | 0–2 | 0.8 |
| `flight` (v18) | 0–2 | 0.8 |
| `fear_reach` (v18) | 0.1–1 | 0.5 |
| `generalise_types` (v19) | 2–10 | 3 |
| `quieting` (v21) | 0–1 | 0.8 |
| `habit_fade_bad` (v21) | 0–0.01 | 0.00005 |

The v16 defaults are the prototype's (#10), a starting point for slice 9's tuning, and the v18 ones slice 9c's prototype's (#62). `generalise_types` starts at `generalise`'s value. Rows for parameters that do nothing since v16 stay, since a genome may still carry their genes.

- **Receptor target ranges:** `learning_rate_mod` 0.5–2.0, `exploration_mod` 0.25–4.0, `curiosity_mod` 0–2.0 (v16), all neutral at 1.0.
- **The Cursor's touch** (v21): the `reward` of a pet (0.5) and a hug (1.0); the `punishment` and `pain` of a zap (0.5, 0.3) and a shock (1.0, 0.6); and `max_reach_back`, the longest reach back a Reward can carry (40 ticks, two seconds at 16×). The furthest the Cursor throws or shoves a thing of each size (v25, §3.5.4): small 6 tiles, medium 6, large 5.
- **Behaviour constants:**
  - Retreat bout: 6 steps
  - Rest bout: 10 ticks
  - action timeout: 60 ticks
  - flood refresh: 8 ticks after the sprite's last flood
  - occupied-tile penalty: 30 (three grass steps)
  - re-plan after 3 blocked ticks
  - `nearby_sprites` radius (3) and normalization (4)
- **Spawn variation:** ±10%.
- **Lesson threshold** (`lesson_threshold`): 0.5.
- **Learning** (v16): `relief_deadband` 0.02 per tick; `touch_window` 3 ticks; `forget_below` 0.01 (v18).

## Appendix C — Risks

| Risk | Mitigation |
|---|---|
| Learning is too slow, too fast or unstable | Lab runner, and A1–A4 as guardrails; every constant is data |
| Concept recruitment adds tuning burden | On hold since v16: measured, it changed no behaviour (§5.4) |
| Very basic instincts starve or kill newborns | Measured with the v16 prototype: in the default world 1 sprite died in 20,000 ticks, against 2 with v15's instincts. Need makes a sprite restless, and A4 guards it |
| Reward lands on the wrong decision | Learning consumes reward every tick (§5.6); one-shot actions give sharp, pulse-keyed relief with a deadband (§4.5); worth goes only to the thing touched, not back along the trace (v16). Habits still use the trace, so a habit can take blame for a moment near the one that earned it ([#57](https://github.com/Keazra/terra-sprites/issues/57)). |
| Fear corners sprites more often | Measured (v18): 270 corners per run against 110, from sprites backing away from those they fear; cornered sprites fight back more often (23% against 15%). Fear pulls only while the feared sprite is near (`fear_reach`); habituation ([#72](https://github.com/Keazra/terra-sprites/issues/72)) may ease it further |
| Corridor congestion | Head-on swaps; blocked re-planning ends hopeless actions early (§3.7) |
| The perception flood is too expensive | Benchmarked early; the refresh interval and radius range are data |
| Cross-platform determinism is unverified | Promised only for targets in the CI determinism matrix (both x86_64 and aarch64) once its check passes; "expected" elsewhere (§2.6, §7.5) |
| A behaviour-test threshold is badly calibrated | The minimum-baseline rule fails loudly; amend thresholds, never delete tests |
