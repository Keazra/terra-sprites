# Terra Sprites — M1 "A Sprite Lives" design (v5)

- **Status:** Final
- **Date:** 2026-09-23
- **Supersedes:** [v4](m1-a-sprite-lives-v4.md) (earlier: [v3](m1-a-sprite-lives-v3.md), [v2](m1-a-sprite-lives-v2.md), [v1](m1-a-sprite-lives.md))
- **Covers:** Milestone 1 in full detail, plus the architecture decisions that every later milestone depends on

---

## Changes from v4

Decided with the owner while building slice 2 ([#3](https://github.com/Keazra/terra-sprites/issues/3)): the controls and the hand are reworked around a mouse-driven cursor, and the slice's refinements to terrain and the map view are recorded. The domain terms are in [`CONTEXT.md`](../../CONTEXT.md).

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The keyboard scrolls the map; the mouse points.** `W` `A` `S` `D` or the arrow keys scroll the viewport (Shift: 5 tiles). The cursor follows the mouse pointer, and there is no keyboard cursor. | Owner decision | §6.1, §6.5 |
| 2 | **A 3×3 cursor** centred on its target: arrows on its sides point in, **mode marks** sit at two corners and **status marks** `Y` and `N` at the other two. | Owner decision | §6.2, §6.5 |
| 3 | **Cursor modes:** `E` Select, `Q` Hand (grab, drop, and the Place menu), `Z` Reward, `X` Correct, and the mouse wheel cycles through them. A right-click or `Esc` returns to Select. Switching modes keeps each mode's state, so what the hand holds waits in reserve, and the status line always shows it. Pausing lets you queue actions for the next tick. | Owner decision | §6.5, §6.8 |
| 4 | **Click feedback:** in Reward and Correct mode each click flashes the status marks `+` at once, then `☼` (applied) or `?` (rejected) when the sim reports back. A click with nothing on the tile to act on sends no command and flashes `?`. | Owner decision | §6.5 |
| 5 | **Tickle becomes Reward** (a pet or hug) and **Slap becomes Correct** (an electric shock). The chemistry is unchanged: Reward injects `reward`; Correct injects `punishment` and `pain`. The sprite senses `petted` or `shocked` (renamed from `tickled`/`slapped`; same IDs), kept apart from the `reward` chemical that other things also raise. | Owner decision | §0, §2.2, §2.5, §4.6, §5.2, §5.8, §6.3, §7.3, §7.4, App. A, App. B |
| 6 | **Quitting is `Esc`, twice:** `Esc` asks "Quit? (y/n)"; `y` or a second `Esc` quits. `Ctrl+C` quits at once. `q` no longer quits (this replaces v4 change 2: `q` sits beside `W` and `A`). | Owner decision | §6.5, §6.6 |
| 7 | **Keys retired or moved:** `t` (Tickle) and `s` (Slap) give way to modes, and `s` now scrolls; `g` no longer grabs (`Q` does) and now exports the genome (`e` is Select); `p` (Place) becomes `Q` pressed again in Hand mode; `Enter` is dropped (a click selects); `hjkl` and `q` (quit) are gone. Dropping `hjkl` also removes their clash with `l` (sprite list). | Follows from 1, 3 and 6 | §6.1, §6.5 |
| 8 | **Slice 2's terrain and map-view decisions:** the map's edge is the terrarium's **wall**, and the map view's border is double-lined where the wall is in view; a small map gets a shrunk map view; the top bar shows the seed; `--ascii` swaps only what themes cover. Generation uses terrain bands by percentile and carves the route with the fewest carved tiles, then the fewest steps, by orthogonal steps; value noise needs no `libm`. Map sides are 32–1024 tiles. `World::new` can't fail, because the config and data pack are validated when parsed. | Slice 2 design session with the owner | §2.2, §3.1, §3.2, §6.1, §6.2 |
| 9 | **A bigger default map, with plants and toys by density.** The default map grows from 160×96 to 256×160 (about 2.7× the area): at the recommended terminal size, with the inspector open, 160×96 was only a couple of screens across. Berry bushes, thornbushes and balls are now set per area, so the bigger map gets proportionally more of them and food stays as easy to find; the sprite count doesn't scale. | Owner decision | §3.1, §3.9 |

---

## Changes in v4 (from v3)

Three small amendments, all made while building slice 1 ([#2](https://github.com/Keazra/terra-sprites/issues/2)).

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Slower speeds.** `-` keeps halving below 1×, to ½× (5 ticks/s), ¼× (2.5 ticks/s) and ⅛× (1.25 ticks/s), then stops; `+` climbs back the same steps. The game still starts at 1×. At 1× a sprite's decisions go by too fast to follow. The UI clock counts rates in eighths of a tick per second, so ⅛× stays exact in integer maths. The top bar labels them `1/2x`, `1/4x`, `1/8x`: ASCII, because CP437 has no ⅛. | Owner feedback after trying slice 1 | §6.6 |
| 2 | **Quit key recorded:** `q` or `Ctrl+C`. v3 never listed a way to quit; slice 1 added one. `q` doesn't clash with any planned key. | Slice 1 implementation | §6.5, §6.6 |
| 3 | **Held keys stop at 1×.** Holding `+` or `-` steps the speed until it reaches 1×, then stops; a fresh press is needed to go past it. 1× is where most observation happens, and with nine speeds a held key would otherwise shoot straight past it. Also: holding `space` toggles pause only once (no flicker), and holding `.` keeps stepping. | Owner feedback after trying slice 1 | §6.6 |

---

## Changes in v3 (from v2)

v2 was reviewed once more and graded PASS, with two clarifications requested. Both are adopted, plus one related gap found while working on the first.

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Blocked re-planning is fully specified.** It's a separate one-off search that treats occupied tiles as impassable. A path it finds becomes the action's **committed path**, which regular flood refreshes can't override, so the sprite can't loop back into the blocked tile. Finding no path ends the action as `blocked`. | Review of v2, point 1 | §2.8, §3.7, §7.2 |
| 2 | **Retreat measures straight-line (Chebyshev) distance from the target**, not path distance. Path distance would need a second flood run from the target. | Self (found while working on point 1) | §3.7, §5.5, §7.2 |
| 3 | **The determinism promise is tied to the CI matrix,** and the matrix covers both x86_64 and aarch64. v2 claimed x86_64 and aarch64 coverage, but its CI only ran Windows and Linux, both x86_64. | Review of v2, point 2 | §2.6, §7.5, App. C |

---

## Changes in v2 (from v1)

The changes come from the GPT and Gemini evaluations of v1 ([m1-gpt-eval.md](m1-gpt-eval.md), [m1-gemini-eval.md](m1-gemini-eval.md)) and from self-review. Evaluation points were assessed on their merits, not adopted wholesale.

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Learning consumes reward and punishment.** Step 4 reads them, then resets them to 0, so each tick's reward is a one-off signal and credit only flows backwards. In v1, reward lingered with a ~3-tick half-life and learning read its level every tick, so decisions *after* a reward collected about 4× the credit of the decision that earned it. | Self (found while checking Gemini 3) | §2.4, §4.3, §4.5, §5.6, App. C |
| 2 | **Sharp drive relief in the starter genome.** One-shot actions relieve their drive in a single tick, keyed to the event pulse (`ate`, `drank`, `played`…). Reward still comes from the fall in the drive, so it stays need-gated. Fall emitters get a deadband. This replaces v1's slow, digestion-driven hunger relief. | Gemini 3, with a different fix from the one proposed | §4.5 |
| 3 | **Emitters run in two passes:** Level emitters, then Rise/Fall emitters, each in genome order. This guarantees that a drive falling from a pulse this tick produces reward in the same tick. | Self (required by change 2) | §2.4, §4.4 |
| 4 | **Head-on swaps are allowed.** The 3-blocked-tick re-plan treats occupied tiles as impassable, and ends the action as `blocked` when no path remains. | Gemini 4, with a different fix | §3.7, §5.5 |
| 5 | **Only value fields vary.** Spawn variation (and later M2 mutation) changes only each gene type's declared value fields, never IDs, coefficients, modes or signatures. | GPT 4 | §4.3, §4.9, §5.7 |
| 6 | **Verb-only effects** (`Inject`, `Signal`, `Push`, `RequireCounter`) outside a verb table are a load error. Objects created during step 2 first run next tick. | GPT 5 | §2.4, §3.5.2 |
| 7 | **Social play relieves boredom**, through the starter genome responding to `played_social`. | GPT 2 | §4.5 |
| 8 | **Thornbush Play is deliberate** and stated as such. | GPT 3 | §3.5.3 |
| 9 | **Training scenarios get a washout:** the trainer stops reacting at tick 9,900. | GPT 1 | §7.3 |
| 10 | **Clarifications:** trace entries are committed every tick; why interaction verbs walk and what Approach is for; re-planning toward a moving target uses the cached flood; why no compiler FMA flags are needed. | Gemini 1, 2, 5, 6 | §2.6, §3.6, §5.2, §5.5, §5.6 |
| 11 | **UI reads consumed reinforcement.** Reward is 0 after each tick, so the UI reads each sprite's `last_r` (the r consumed at step 4) instead of the reward level. | Self (required by change 1) | §5.6, §6.1, §6.3 |

---

## 0. Vision and key decisions

Terra Sprites is a terminal artificial-life game inspired by *Creatures*. Sprites are small creatures with a genome, a simulated biochemistry and a learning brain. They live on a top-down grid world with plants, water, toys and hazards.

**The simulation is the star.** Behaviour must emerge from drives, chemistry and learning, not from scripts. The player watches, inspects and intervenes with a "hand of god" (rewarding, correcting, moving things) and sees individual sprites learn from experience. Later milestones add breeding and evolution, a wilder world, language, and a tile-rendered front end.

| Decision | Choice | Why |
|---|---|---|
| Core appeal | Emergent artificial life | The simulation must be real, not scripted to look alive |
| Platform | Rust terminal app (ratatui + crossterm) | Fast, deterministic simulation with no GC; solid on Windows Terminal |
| Where behaviour comes from | Learning within a lifetime **and** evolution across generations | Instincts evolve; personalities are learned |
| Brain model | Creatures-style lobes (attention → concepts → decision) with reinforcement carried by chemicals | The only candidate where both learning and evolution are visible at 20–100 sprites, and reward and correct mean something |
| World | Top-down grid | Simple movement and perception; room for many sprites |
| Player role | Hand of god: observe, reward, correct, grab, place | Player feedback feeds directly into learning |
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
| **M1 — A Sprite Lives** *(this document)* | World, ecology, biochemistry, learning brain, terminal UI, the hand, save/load, replay |
| **M2 — Generations** | Life stages; sexual and asexual reproduction decided by the genome; crossover and mutation; lineage and family-tree view; population graphs; headless fast-forward mode |
| **M3 — Wild Terra** | Critters (prey and predators); more hazards and toys; possibly seasons, weather, day/night and temperature |
| **M4 — Words** | The player (and later, sprites) name objects and verbs; word inputs and a Speak output |
| **Tiles** *(UI milestone, can be scheduled any time after M1)* | A tile-window front end that draws bitmap tilesets and sprite sheets through the semantic-tile seam (§6.2) |

### 1.2 In scope for M1

- **World:** a grid world with terrain, plus data-defined objects that have lifecycles: berry bushes, berries, thornbushes, balls, and water.
- **Sprites:** each has a genome, a biochemistry and a lobe brain that learns. Sprites age, and die from injury, which covers starvation, dehydration, old age and harm.
- **Starter population:** sprites are created from a starter genome with small random variation. They can also be spawned from a genome file.
- **Terminal UI:**
  - map with body language (drive colours and emotes)
  - sprite inspector (Body / Brain / Chem / Genome / World tabs)
  - event log
  - the hand
  - time controls
  - save, load, autosave
  - replay recording and playback
- **Lab runner:** a scenario runner with no UI, used by tests and for tuning.

### 1.3 Out of scope for M1

- Reproduction of any kind; the population can only be topped up by the player spawning sprites
- Day/night, seasons, temperature
- More than one type of nutrient
- Sprites carrying objects (only the hand can carry things)
- Line of sight, swimming
- Critters, language
- A diegetic information mode
- The tile-window front end
- A general headless mode; only the lab runner exists in M1
- GitHub CI, until a remote exists

### 1.4 Obligations M1 carries for later milestones

- **Genome format:** gene type IDs are stable, payloads are versioned, and unknown genes are preserved (§2.8).
- **Registries:** every ID is stable and append-only. This covers chemicals, loci, brain inputs and outputs, categories and object types. `Mate` (M2) and `Speak` (M4) have IDs reserved now.
- **Objects:** defined as data, with a closed rule vocabulary (§3.5).
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
│  │  └─ examples/lab.rs      headless scenario runner (tests + tuning)
│  └─ terra-tui/              binary `terra-sprites`: ratatui + crossterm front end
├─ data/                      default data pack (RON), embedded in the binary
│  ├─ pack.ron                pack name + version
│  ├─ terrain.ron  objects.ron  chemicals.ron  loci.ron  brain_io.ron  physiology.ron
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
let mut world = World::new(config, data_pack, seed);    // config and pack were validated when parsed
world.submit(Command::Reward { sprite });                // stamped for tick now+1, applied at step 1
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
| 1 | **Commands** | Apply the commands stamped for this tick, in the order they were submitted. The hand's direct chemical injections happen here, and its pulses go into the target's `incoming` buffer. Invalid commands are rejected with a `CommandRejected` event. |
| 2 | **Environment** | Object rules run (stages, counters, spawning, spreading, expiry). This covers the objects that exist **when step 2 begins**, in ascending ID order, with rules in the order listed (§3.5). Objects created during step 2 first run next tick. |
| 3 | **Biochemistry** | For every sprite, **held or not**: (a) pulse latch: `live ← incoming` and `incoming` is emptied; (b) physics; (c) reactions; (d) half-life decay; (e1) Level emitters; (e2) Rise/Fall emitters; (f) clamp to [0, 1]; (g) receptors. Then **death check #1**: if `injury ≥ 1.0` the sprite is marked **dying**. |
| 4 | **Learning** | For every sprite not marked dying: read `r = reward − punishment`, then **reset both to 0** (learning consumes them). Reinforce using the trace ring buffer (the freshest entry is from the previous tick), relax the two-timescale weights, then recruitment (§5.6). |
| 5 | **Sense and decide** | For every sprite that is neither dying nor held: refresh the perception flood if needed (§3.6), then **5.0** check the current action, **5a** attention, **5b** decision (§5.5). The activations are snapshotted. |
| 6 | **Resolve actions** | In an order shuffled each tick by the world RNG: movement steps (including head-on swaps, §3.7) and verb effects. Effects inject physical chemicals and write `incoming` pulses. Then each deciding sprite **commits one trace entry**: its snapshot plus the outcome. This happens every tick, whether the action just started or is continuing. |
| 7 | **Deaths and events** | **Death check #2** catches injury from step 6. Sprites marked dying are removed; a held sprite that dies empties the hand. Events are emitted, and the tick counter goes up. |

**Timing consequences** (these hold everywhere):

- **An action's effects reach the sprite's next decision, never its current one.** An action at tick *t* is processed chemically at step 3 of *t+1*, learned from at step 4 of *t+1*, and affects the decision at step 5 of *t+1*.
- **Hand feedback credits what the player just saw.** The player sees the world as it stands at the end of tick *t*, and their command is stamped *t+1*. It's applied at step 1 of *t+1*, and at step 4 the freshest trace entry is decision *t*.
- **Pulses stay live long enough to be seen.** A pulse written at step 6 of *t*, or at step 1 of *t+1*, is live for steps 3–5 of *t+1*: emitters see it at step 3, and attention and concepts see it at step 5.
- **Reward only credits the past.** Reward and punishment produced at step 3 are consumed at step 4 of the same tick, so they credit entries up to the previous tick and never linger onto later decisions.
- **A sprite whose injury crosses 1.0 at step 3 makes no decision that tick.**
- **Held sprites** run steps 3 and 4 but skip steps 5 and 6. They make no decisions and commit no trace entries.

### 2.5 Commands and events

| Command | Effect | Rejected when |
|---|---|---|
| `Reward { sprite }` | A pet or hug: injects `reward` directly; pulses `petted` | The sprite is missing, dead or held |
| `Correct { sprite }` | An electric shock: injects `punishment` and `pain` directly; pulses `shocked` | The sprite is missing, dead or held |
| `Grab { tile }` | Picks up the sprite on the tile if there is one, otherwise the loose item | The hand is full, there's nothing grabbable, or the only thing there is a fixture |
| `Drop { tile }` | Puts the held entity on the tile | The hand is empty, or placement rules are violated (§3.3–3.4) |
| `Place { tile, object_type }` | Creates a berry, a ball or a berry-bush seedling | Placement rules are violated, or the type can't be placed |
| `SpawnSprite { tile, genome: Option<Genome> }` | Creates a sprite. `None` means the starter genome with spawn variation (§4.9) | The tile is occupied or not walkable, or the genome fails validation |
| `Rename { sprite, name }` | Sets the display name | The sprite is missing, or the name is empty, too long or not CP437 |

**Commands carry values, never references.** A genome loaded from a file is parsed by the UI and put inside the command in full. This keeps the command log self-contained.

**Events** each carry the tick and the entity IDs involved. The M1 set:
- `ActionStarted`, `ActionEnded { outcome }`
- `Ate`, `Drank`, `Played`, `Hit`, `Pricked`
- `Rewarded`, `Corrected`
- `Spawned`
- `Died { cause, age }`
- `LearnedMilestone { link, delta }`
- `ObjectSpawned` / `ObjectExpired`
- `HandEmptied { reason }`
- `CommandRejected { command, reason }`

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
  - the hand
  - per sprite:
    - chemicals, both pulse buffers, and `last_r` (§5.6)
    - the brain (links, concept pool, trace ring buffer)
    - the current action, including any Wander destination, bout progress and committed path (§3.7)
    - movement state (move points, blocked-tick counter, last step direction)
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

| Terrain | Walkable | Step cost | Fertility | Drinkable |
|---|---|---|---|---|
| Grass | ✓ | 10 | 1.0 | |
| Dirt | ✓ | 10 | 0.5 | |
| Sand | ✓ | 15 | 0.0 | |
| Shallow water | ✓ | 25 | 0.0 | ✓ |
| Deep water | ✗ | – | – | |
| Rock | ✗ | – | – | |

- An orthogonal step costs the destination tile's step cost. A diagonal step costs `cost × 14 / 10`, in integer maths.
- **No corner-cutting:** a diagonal step is allowed only if both tiles beside the diagonal are walkable and free of fixtures.

### 3.2 World generation and connectivity

1. **Terrain:** seeded value noise produces height and moisture fields. It is built in-house from + − × ÷ alone (so it needs no `libm`), and its random lattice values come from the world RNG. Terrain bands are set **by percentile**, so every seed gets about the same mix: the lowest 20% of heights are deep water, then 12% shallow water, 8% sand, 50% land, and the highest 10% rock. The driest 40% of the land is dirt; the rest is grass.
2. **Regions:** a flood fill using exactly the movement rules above (**shallow water counts as walkable**) finds the walkable regions. The largest one is the **mainland**.
3. **Joining regions:**
   - Every other region of **≥64 tiles** is joined to the mainland, largest first, along the route that **carves the fewest tiles, then takes the fewest steps**. The route uses orthogonal steps only, so it never depends on a diagonal, and the connection it carves is one tile wide. Deep water becomes shallow water; any other unwalkable terrain becomes dirt. A route that crosses another region joins that region too.
   - Regions **under 64 tiles** become rock.
4. **Placement:** sprites and objects are placed on the mainland, obeying the rules in §3.3–3.4.

Property tests check full connectivity across many seeds.

### 3.3 The fixture ring rule

A **fixture** is an object that blocks movement. A fixture may only be placed, spawned, spread or created by `ReplaceWith` onto a tile that meets all of these:
- the tile is walkable, holds no sprite, and holds no fixture
- **all 8 neighbouring tiles are walkable and free of fixtures**

**Why this is enough:** every fixture is surrounded by a ring of walkable tiles, so any path through the fixture's tile can go around the ring instead. Any diagonal step the fixture forbids under the no-corner-cutting rule connects two tiles of that ring, which are joined by orthogonal steps. So **no ecology outcome can ever disconnect the map or trap a sprite.** A property test checks this invariant under random sequences of placements and removals.

### 3.4 Space rules

- A tile holds **at most one sprite**, **at most one fixture** and **at most one loose item** (a berry or a ball). Items don't block movement, and a sprite can stand on an item.
- Sprites interact with their own tile or an adjacent one (the 8-neighbourhood).
- Only the hand carries things in M1.

### 3.5 Objects defined as data (`data/objects.ron`)

#### 3.5.1 Schema

| Field | Meaning |
|---|---|
| `id` | Stable `ObjectTypeId` |
| `name` | Referenced by rules and themes |
| `category` | Stable `CategoryId`, which is what brains perceive |
| `placement` | `Fixture` (blocks movement; ring rule applies) or `Item` (loose-item slot) |
| `pseudo` | `true` for Water and Sprite: a verb table only, no instances or lifecycle |
| `counters` | Named integer counters with maximums, e.g. `{"fruit": 6}` |
| `stages` | `[(name, ticks: (min, max), next: Stage(name) \| Expire)]`. The duration is drawn from the world RNG when the stage is entered. An object with no stages is permanent. |
| `rules` | `[(trigger, if: [conditions], do: [effects])]` |
| `verbs` | `{Verb: [effects]}`: what happens when a sprite applies that verb to this object |
| `visual` | `[(if: [conditions], state: name)]`: the first match names the visual state the theme draws. If nothing matches, the state is `"default"`. `Chance` is not allowed here, so rendering never uses the RNG. |

**Glyphs and colours are not in `objects.ron`.** Themes map `(object name, visual state)` to how it looks (§6.2).

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

**Effects:**

| Effect | Behaviour |
|---|---|
| `AddCounter(name, Δ)` | Clamps to [0, max] |
| `RequireCounter(name, n)` | **In a verb only:** if the counter is below *n*, the verb **fails**. Later effects don't run, and the outcome is `failed`. |
| `SpawnNearby(type, radius)` | Creates an object on a free, valid tile within the radius, chosen uniformly among the candidates (one RNG draw if there's at least one). A fixed scan order would make bushes drift in one direction. Does nothing, with no draw, if there are no candidates. |
| `SpreadTo(type, radius, [conditions])` | Draws **one** candidate tile uniformly from within the radius (one RNG draw). If the tile passes placement and the conditions, which are evaluated *at that tile*, the object is created there. Otherwise nothing happens. |
| `ReplaceWith(type)` | Replaces this object with a new one (new ID, first stage) on the same tile. Placement is checked for the new type, e.g. the fixture ring rule and no sprite on the tile. Does nothing if that fails. |
| `DestroySelf` | Removes the object |
| `Inject(Actor \| Target, chemical, amount)` | Adds to a chemical. **Only physical chemicals are allowed.** Anything else is a load error. |
| `Signal(Actor \| Target, locus)` | Writes a pulse to the `incoming` buffer. A pulse on the Target records the Actor as its source. |
| `Push(max_tiles)` | Moves this item up to `max_tiles` away from the actor, in the direction from actor to item snapped to 8 directions (if both are on the same tile, the actor's last step direction, or N if it has none). It stops before non-walkable tiles, fixtures, or tiles holding a loose item. Sprites don't stop it. |

**Verb-only effects:** `Inject`, `Signal`, `Push` and `RequireCounter` need an actor, so they may only appear in a `verbs` table. Using one in a lifecycle rule is a **load error**.

**Evaluation semantics:**
- Step 2 covers the objects that exist when it begins, in ascending ID order, with rules in the order listed. Objects created during step 2 (by `SpawnNearby`, `SpreadTo` or `ReplaceWith`) first run their rules next tick. Objects placed by the hand at step 1 already exist, so they run in the same tick.
- **Conditions are evaluated left to right and stop at the first false one.** So a `Chance` after a false condition doesn't draw from the RNG.
- **Convention:** put location conditions before `Chance`.
- Effects run in order.
- **Held objects have no tile.** Location conditions evaluate as false, and effects that need a tile (`SpawnNearby`, `SpreadTo`, `ReplaceWith`) do nothing. Stage timers, counters and `Chance` keep running. An object that expires while held empties the hand and emits `HandEmptied`.
- Names of chemicals, loci and types are resolved to stable IDs when the file is loaded. An unknown name is a load error.

**Scope rule:** any later object type that fits this vocabulary needs **no new code**. A genuinely new behaviour means adding one case to the rule enum. (M3 critters are agents, not objects.)

#### 3.5.3 M1 object types

```ron
(id: 1, name: "berry_bush", category: BerryBush, placement: Fixture,
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

(id: 2, name: "berry", category: Berry, placement: Item,
 stages: [(name: "fresh", ticks: (1500, 2500), next: Expire)],
 rules: [
   (trigger: OnExpire,
    if: [Fertility(Ge, 0.5), DensityBelow("berry_bush", 4, 3), Chance(0.1)],
    do: [ReplaceWith("berry_bush")]),
 ],
 verbs: { Eat: [Inject(Actor, "food", 0.3), Signal(Actor, "ate"), DestroySelf] })

(id: 3, name: "thornbush", category: Thornbush, placement: Fixture,
 stages: [(name: "grown", ticks: (40000, 60000), next: Expire)],
 rules: [
   (trigger: Every(2000), if: [Chance(0.1)],
    do: [SpreadTo("thornbush", 4, [DensityBelow("thornbush", 4, 2)])]),
 ],
 verbs: {
   Eat:  [Inject(Actor, "injury", 0.05), Signal(Actor, "pricked")],
   Hit:  [Inject(Actor, "injury", 0.03), Signal(Actor, "pricked"), Signal(Actor, "did_hit")],
   Play: [Inject(Actor, "injury", 0.03), Signal(Actor, "pricked")],
 })

(id: 4, name: "ball", category: Ball, placement: Item,
 verbs: {
   Play: [Push(4), Signal(Actor, "played")],
   Hit:  [Push(2), Signal(Actor, "did_hit")],
 })

(id: 100, name: "water", category: Water, pseudo: true,
 verbs: { Drink: [Inject(Actor, "water", 0.2), Signal(Actor, "drank")] })

(id: 101, name: "sprite", category: Sprite, pseudo: true,
 verbs: {
   Hit:  [Inject(Target, "injury", 0.03), Signal(Target, "was_hit"), Signal(Actor, "did_hit")],
   Play: [Signal(Actor, "played_social"), Signal(Target, "played_social")],
 })
```

**The resulting ecology:**
- **Food:** bushes carry fruit. Overripe fruit drops as berries, which are eaten or rot. Rotting berries sometimes sprout new bushes on fertile land that isn't crowded. Food therefore has a geography, spreads, and can be overgrazed.
- **Thornbushes** give no food and spread slowly. They exist so sprites have something to learn to avoid. **Every contact verb hurts, deliberately, including Play:** thorns hurt whatever you do to them, and only Approach and Retreat are safe. Scenario A1 counts only thornbush Eats. Harm from Play also teaches sprites to pay less attention to thornbushes, which is a legitimate part of the lesson.
- **Balls** are permanent toys.

The rule constants above are starting values, tuned with the lab runner.

### 3.6 Perception, reachability and goal tiles

**The flood:**
- Each sprite has one **bounded Dijkstra flood**. It covers walkable tiles within its `sense_radius` (Chebyshev distance), using step costs.
- Tiles holding another sprite can be crossed at a penalty (set in `physiology.ron`), since sprites move.
- The flood is recomputed when the sprite enters a new tile, or every 8 ticks.
- It provides both distances and paths. **No separate A* is needed.**

**Goal tiles:**

| Target | Goal tiles |
|---|---|
| Fixture | Any walkable neighbour |
| Item | Its own tile or any neighbour |
| Water | The shallow-water tile itself or any neighbour |
| Sprite | Any neighbour. The path is re-planned whenever the target moves. |

**Re-planning uses the cached flood.** When a target moves, the new path is traced back through the existing flood's predecessor grid, which costs no new Dijkstra run. A full re-flood happens only on the refresh rules above (the sprite changes tile, or 8 ticks pass). If the target leaves the flood's area, it's no longer reachable, and the action ends (§5.5).

**Candidates:**
- For each category, the candidate is the **nearest reachable** instance: the one with the lowest `(path cost, entity ID)`. For Water, the tile index `y × width + x` stands in for the entity ID.
- An instance is reachable if the flood reached one of its goal tiles. **A closer instance that can't be reached is never a candidate.**
- The Sprite candidate is the nearest reachable other sprite. **While a `was_hit` pulse is live, it's the attacker instead** (the pulse's source), provided the attacker is reachable. Otherwise it falls back to the nearest.

**Cost:** the flood covers at most (2r+1)² tiles (841 when r = 14). It's the main cost per sprite, and one of the first things benchmarked.

### 3.7 Movement timing and conflicts

- **Move points:** a moving sprite gains `speed` points per tick (the Trait gene, 4–12). When its points reach the cost of its next step, it tries the step.
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
  - **Committed path dropped:** if the sprite is blocked again, the 3-tick count and the search start over. If the action's target is a sprite and that target moves, the path is dropped and normal flood-based planning resumes. The path also ends with the action.
  - **No path found** (for example, a sprite resting in the only corridor): **the action ends with outcome `blocked`**, freeing the brain to choose again.
  - **Retreat** doesn't use the search. It moves by straight-line distance (§5.5), so for Retreat "no path" means no free neighbour increases the Chebyshev distance from the target.
  - **Cost:** the search only runs after 3 blocked ticks, which is rare, so it doesn't threaten the §3.9 performance target.
- **Why a seeded shuffle and not ID order:** both are deterministic. But ID order would favour older sprites in every contest, and M2 evolution would pick up that bias.
- **Energy:** each step costs energy under physics rules (§4.8).

### 3.8 No incidental harm

Harm only ever comes from **explicit verbs** aimed at an object. There is no damage from bumping into things or passing near them.

This keeps credit assignment clean. If thorns scratched sprites walking past, the pain would be credited to whatever the sprite was doing at the time, usually "approach food". Sprites would learn the wrong lesson.

### 3.9 Defaults and performance

- **Defaults:** 256×160 map; 30 starter sprites (configurable, 20–100).
- **Plants and toys are set by density**, so a bigger map gets proportionally more and food stays as easy to find: about 150 berry bushes, 40 thornbushes and 6 balls per 15,360 tiles (a 160×96 area). On the default map that is about 400 berry bushes, 107 thornbushes and 16 balls. The sprite count doesn't scale; it stays in the 20–100 range.
- **Performance target:** **≥200 ticks per second with 100 sprites** in a release build.

---

## 4. Biochemistry

### 4.1 The central split

> **The world decides what happens to the body. The genome decides how that feels.**

| Class | Chemicals | Changed by | Can evolve? |
|---|---|---|---|
| **Physical** | `energy`, `hydration`, `stamina`, `food` (gut), `water` (gut), `injury` | Physics (`physiology.ron`) and object verbs **only** | No |
| **Signal** | Drives: `hunger`, `thirst`, `pain`, `tiredness`, `boredom`, `loneliness`, `crowdedness`. Learning signals: `reward`, `punishment` | Genome, and the hand | Yes |
| **Hormone** | `h0`–`h15`, unnamed | Genome | Yes. These are spare channels evolution can put to use. |

- Chemicals are listed in `data/chemicals.ron` with stable IDs (Appendix A).
- Concentrations are `f32` values in [0, 1], one array per sprite.
- If physics were evolvable, M2 evolution would simply remove the costs, so this split is fixed now in M1.

### 4.2 Loci

Loci are listed in `data/loci.ron` with stable IDs. Each has a kind and a `brain_visible` flag.

| Kind | Loci | Brain-visible |
|---|---|---|
| **Chemical level** | Any chemical, referenced as `Chem(id)`. Genes can read physical chemicals too. | Drives and hormones only (§5.2) |
| **Body sensors** (physics fills these in every tick) | `always` (= 1) | ✓ |
| | `age` (normalized by lifespan) | ✓ |
| | `nearby_sprites` (sprites within 3 tiles ÷ 4, capped at 1) | ✓ |
| | `moving` (1 if the sprite stepped during the previous tick), `resting` (1 while a Rest action is running) | ✗ physiology only: they reflect the current action every tick, so as brain inputs they would loop output back into input |
| **Event pulses** (one tick, latched, §2.4) | `ate`, `drank`, `played`, `played_social`, `pricked`, `was_hit`, `did_hit`, `petted`, `shocked` | ✓ |
| **Receptor targets** (written by receptors) | `learning_rate_mod` (0.5–2×), `exploration_mod` | ✗ |

- **The pulse latch:**
  - Writes go to `incoming`: at step 1 (the hand) and at step 6 (verb effects).
  - At step 3a, `live` is replaced by `incoming`, and `incoming` is emptied.
  - Reads use `live`: emitters at step 3, and the brain at step 5.
- **Pulse sources:** a pulse can carry a source entity. `was_hit` records the attacker.
- **Modulator neutral points:** `learning_rate_mod` and `exploration_mod` rest at 1.0 when no receptor writes them.

### 4.3 Gene types for biochemistry

| ID | Gene | Semantics |
|---|---|---|
| 1 | `HalfLife(chem, ticks)` | Exponential decay. The per-tick factor `0.5^(1/ticks)` is computed once at decode time with `libm`. |
| 2 | `Reaction(reactants ≤2 → products ≤2, rate)` | Extent = rate × min over reactants of (concentration ÷ coefficient). Each reactant loses coefficient × extent and each product gains coefficient × extent. An empty slot means "nothing". |
| 3 | `Emitter(locus, mode, invert, threshold, gain, chem)` | Mode **Level**: signal = the locus value, or `1 − value` if `invert` is set. **Rise** / **Fall**: signal = the locus's positive / negative change since the previous tick. Emits `gain × max(0, signal − threshold)`. Gain may be negative, which removes chemical. |
| 4 | `Receptor(chem, threshold, gain, target_locus)` | Each tick, `target = clamp(1.0 + Σ over receptors aimed at it of gain × max(0, level − threshold), target range)`. 1.0 is the neutral point. |
| 5 | `InitialConcentration(chem, value)` | The starting level of a chemical at birth |
| 6 | `Trait(trait_id, value)` | An evolvable body trait. Clamped to a range set by physics, with a cost set by physics (§4.8). |

**Restrictions**, which close every route by which genes could change physical chemistry:

| Gene | Physical chemicals | Signal chemicals and hormones |
|---|---|---|
| `HalfLife` | ✗ (decay rates are fixed physics) | ✓ |
| `Emitter`: chemical written | ✗ | ✓ |
| `Emitter`: locus read | ✓ (reading only) | ✓ |
| `InitialConcentration` | ✗ (newborn physical levels are fixed physics) | ✓ |
| `Reaction` | **Catalyst only:** identical coefficients on both sides, e.g. `hunger + food → food` | ✓ |
| `Receptor` | Reads any chemical; writes only receptor-target loci | — |

A gene that breaks these restrictions is **flagged when decoded, not expressed**, and marked in the genome viewer.

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

### 4.4 Inside step 3

For each sprite:
1. **(a) Pulse latch** (§4.2).
2. **(b) Physics** (`physiology.ron`, fixed, not evolvable):
   - basal metabolism, plus the cost of `sense_radius`
   - energy spent on steps taken last tick, which depends on `speed`
   - digestion (`food → energy`, `water → hydration`)
   - hydration loss
   - stamina drains per step and recovers while idle (faster while `resting`)
   - injury heals slowly
   - starvation, dehydration and senescence add injury (§4.10)
3. **(c) Reactions**, in genome order.
4. **(d) Half-life decay.**
5. **(e) Emitters, in two passes.** Both run **after** reactions.
   - **(e1) Level emitters** run in genome order.
   - **(e2) Rise/Fall emitters** run in genome order. Their "change" is the locus value now (after reactions, decay, e1, and any earlier e2 emitter) minus its value at the end of the previous tick's step 3.
   - Each emitter sees the effects of every emitter before it. This guarantees that a drive knocked down by a pulse-keyed Level emitter produces its Fall reward **in the same tick**, ready for step 4.
   - A chain of Rise/Fall emitters, such as injury → pain → punishment, works within one tick if its genes are in chain order. The starter genome orders them that way; evolution can reorder them.
6. **(f) Clamp** every concentration to [0, 1].
7. **(g) Receptors** update the modulators.

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
   - the pulse is latched, and physics starts digesting food into energy over the following ticks
   - **(e1)** a negative-gain Level emitter on the `ate` pulse drops `hunger` sharply, in this one tick
   - **(e2)** a **Fall** emitter on `hunger`, with a deadband, releases `reward`
   - step 4 consumes the reward and credits the Eat entry from tick *t* most
5. If energy is still below 0.5 once digestion has run, hunger creeps back up, and the sprite is likely to eat again.

**The other drives:**

| Drive | Rises from | Falls from (relief) |
|---|---|---|
| `thirst` | Low `hydration` (inverted Level) | `drank` pulse: sharp, one tick |
| `tiredness` | Low `stamina` (inverted Level) | A negative-gain Level emitter on `stamina`: gradual, as Rest restores stamina |
| `boredom` | `always` (slowly) | `played` and `played_social` pulses: sharp, one tick. **Social play relieves boredom as well as loneliness.** |
| `loneliness` | Low `nearby_sprites` (inverted Level) | `played_social` pulse (sharp); high `nearby_sprites` (gradual, while staying near others) |
| `crowdedness` | High `nearby_sprites` (Level, with threshold) | Low `nearby_sprites` (inverted Level, strong negative gain): falls within a few ticks of leaving the crowd |
| `pain` | A **Rise** emitter on `injury`, and the `pricked` / `was_hit` pulses | Short half-life |

**Learning signals:**
- A Fall emitter on each need drive (hunger, thirst, tiredness, boredom, loneliness, crowdedness) releases `reward`. Each has a **deadband** of 0.02 per tick (its threshold), so slow drifts never drip reward. Gradual relief pathways are set fast enough to clear the deadband, so ongoing relief is still rewarded.
- A Rise emitter on `pain` releases `punishment`.
- Emitter genes are ordered so that each chain completes within one tick (§4.4).
- Learning **consumes** `reward` and `punishment` every tick (§5.6), so they never linger.

v1's catalytic reactions (`hunger + food → food`, `thirst + water → water`) are no longer in the starter genome. They're still valid genes, and evolution may rediscover them.

Only hunger, thirst, tiredness and pain are tied to physical need in M1. Whether evolution keeps boredom and loneliness as useful drives is an M2 experiment.

### 4.6 The hand

- **Reward** (a pet or hug) injects `reward` directly.
- **Correct** (an electric shock) injects `punishment` and `pain` directly.
- Both skip the genome, with amounts set in `physiology.ron`, so the player's teaching tools work on every sprite.
- Both also pulse `petted` or `shocked`, so a genome can add its own reactions on top. The pulses name what the sprite *senses*; the `reward` chemical rises from other causes too (eating, play).

### 4.7 Newborn state

- Physical chemicals start at fixed levels from `physiology.ron`.
- Signal chemicals and hormones start at their `InitialConcentration` genes, or 0.

### 4.8 Traits and their costs

| Trait | Range (`physiology.ron`) | Physical cost |
|---|---|---|
| `speed` | 4–12 move points per tick | Energy per step ∝ (speed / 8)² |
| `sense_radius` | 6–14 tiles | Basal metabolism grows linearly with the radius |
| `lifespan` | 20,000–200,000 ticks | Once exceeded, senescence adds `injury` every tick |

The starter genome's traits are speed 7, sense_radius 10 and lifespan 60,000 ticks (100 minutes at 1×).

### 4.9 Spawn variation

A sprite spawned from the starter genome (`SpawnSprite { genome: None }`, or the initial population) has **each value field (§4.3) multiplied by a uniform factor in [0.9, 1.1]**, drawn from the world RNG, then clamped to its range. Fields are processed in genome order, with one draw per value field. Integer values, such as `HalfLife` ticks, pool size and arity, are rounded afterwards. Structural fields are never touched.

### 4.10 Death

- **There is one route to death: `injury ≥ 1.0`.** Starvation (energy at 0), dehydration (hydration at 0) and senescence (age past lifespan) all add injury every tick.
- **Cause of death:** injury is tracked by source over the last 500 ticks, and `Died.cause` is the biggest contributor. The sources are starvation, dehydration, senescence, thorns and sprite hits.

---

## 5. Brain

### 5.1 Overview

```
 STATE inputs (stable InputIds)       ATTENTION               CONCEPT LOBE                   DECISION LOBE
 drives, hormones, body sensors ──►  score per category ──►  singletons (one per input) ──►  score per verb ──► verb + target
 (nearby_sprites, age, always),      (learned links A)       + innate conjunctions           (learned links W)
 event pulses                              │ picks target     + recruitable pool
                                           └──► TARGET inputs: attended category (one-hot),
                                                target distance, target adjacent ──► concepts only
```

### 5.2 I/O registry (`data/brain_io.ron`)

Every input and output has a stable, append-only ID (Appendix A). Each input belongs to a **group**:

| Group | Inputs | Attention | Concepts |
|---|---|---|---|
| **State** (35) | 7 drives, 16 hormones, `nearby_sprites`, `age`, `always`, 9 event pulses | ✓ | ✓ |
| **Target** (8) | 6 × `AttendedCategory` (BerryBush, Berry, Thornbush, Water, Ball, Sprite), `TargetDistance` (normalized path cost), `TargetAdjacent` | ✗ | ✓ |

- **Target inputs are outputs of attention.** They never feed back into attention.
- **Not brain inputs:**
  - physical chemicals, because the brain feels the body only through drives
  - `reward` and `punishment`, because they're learning signals
  - `moving` and `resting`, because they're physiology only

**Outputs (verbs):**

| Kind | Verbs | Available when |
|---|---|---|
| **Movement** | Approach, Retreat | Any attended target, whatever its verb table says |
| **Interaction** | Eat, Drink, Hit, Play | The attended category's verb table includes the verb |
| **Targetless** | Rest, Wander | Always |

`Mate` (M2) and `Speak` (M4) have IDs reserved.

**Why interaction verbs walk there themselves, and what Approach is for.**

Interaction verbs include the walk, so one decision leads straight to one outcome.

If Eat required being adjacent first, eating would take two decisions, *Approach* then *Eat*, and reward would only follow the second. The approach step would get credit only through the fading trace: after a 20-tick walk with λ = 0.9, about 12% remains. Learning would be slower and more fragile, and the A1–A3 criteria depend on credit reaching the action directly.

Approach still has its own jobs:
- **Closeness.** Loneliness falls as `nearby_sprites` rises, without having to touch anyone.
- **Following** another sprite.
- **A counterpart to Retreat,** which M3 (hunting, herding) and M4 ("come here") will build on.

It only wins decisions where learning finds it useful.

**Stored per brain:** each brain stores its index → ID tables. Concepts are identified by **input signature**, never by index.

### 5.3 Attention (step 5a)

- **Candidates** are the per-category nearest reachable instances (§3.6). With no candidates, attention is empty, all Target inputs are 0, and only targetless verbs are available.
- **Score per category:** `score_c = Σᵢ xᵢ·A[i][c] + salience_gain × (1 − candidate_distance_c)`. Here *i* ranges over **State** inputs only, and `candidate_distance_c` is that candidate's own normalized path cost.
- **With no current action:** attention **samples** a category with softmax(score / τ_att), where **τ_att = τ_att_base × `exploration_mod`**.
- **With any action still running**, targetless ones included: attention is **noise-free**. It changes target only if a rival's score beats the current target's by `attention_margin`. Ties go to the lower `CategoryId`. That switch ends a target-bound action.

### 5.4 Concept lobe

- **Activation:** the **product** of a concept's inputs. Each input may be negated, turning `x` into `1 − x`. This gives a fuzzy AND with NOT.
- **Three kinds of concept:**
  1. **Singletons.** There's always exactly one for every input, and the genome can't remove them. Every input can be learned about from birth. `always` works as a learnable per-verb bias; there is no separate bias term.
  2. **Innate conjunctions** of 2–3 inputs, created by `Instinct` genes.
  3. **A recruitable pool** of `pool_size` neurons (0–64; 0 turns it off).
- **Recruiting:** at step 4, for the most-credited entry (§5.6), a free pool neuron is committed to that entry's top-k most active inputs (k = `max_arity`). Candidate inputs exclude `always` (it's always 1) and inputs with activation 0; the signature uses no negations. This happens when three things are true:
  - |r| is at least `recruit_threshold`
  - the best matching conjunction's activation was below `novelty_threshold`
  - a free neuron exists
- **Forgetting:** a committed pool neuron whose outgoing weights all stay below 0.05 in magnitude for `forget_ticks` is freed again.

### 5.5 Decision and action lifecycle

**Step 5 runs in this order:**
1. **5.0 Check the current action.** It ends if its target no longer exists or has become unreachable, or if the action has hit the timeout.
2. **5a Attention** (§5.3).
3. **5b Decision:**
   - **Score** each *available* verb: `s_v = Σₖ aₖ·W[k][v]`.
   - **With no current action:** sample a verb from softmax(s / τ), where **τ = τ_base × `exploration_mod`**, drawing on the world RNG.
     - If the chosen verb is **Wander**, its destination is sampled now: uniformly from the reachable tiles in the flood whose Chebyshev distance from the sprite is more than `sense_radius / 2`. If there are none, it samples from all reachable tiles other than the sprite's own. If there are none at all, Wander ends straight away with outcome `failed`.
   - **With a current action, verb c:** it continues unless some available verb v has `s_v > s_c + switch_margin`. If one does, the sprite switches **deterministically** to the highest-scoring such verb, with ties going to the lower `OutputId`.
4. **Snapshot** the activations (inputs, concept activations, chosen verb, attended category), ready for the trace entry committed at step 6.

**Rule:** all randomness in the brain (attention, verb, Wander destination) happens **only at action boundaries**. A running action can only be ended by a deterministic change larger than the margin, by losing its target, by timing out, or by completing. It's never ended by noise.

World randomness is separate: plant rules and the order actions resolve in. It can change how an action turns out, but it ends an action only through those same deterministic rules.

**Action lifecycle:**

| Verb | Behaviour | Ends |
|---|---|---|
| Approach | Walks to a goal tile | On arrival |
| Eat / Drink / Hit / Play | Walks to a goal tile, then makes **one attempt** | After the attempt, whether `applied` or `failed` |
| Retreat | Each step goes to the walkable, free neighbour that increases the **Chebyshev distance** from the target the most. Ties follow fixed direction order N, NE, E, SE, S, SW, W, NW. Path distance isn't used, because it would need a second flood run from the target. | After 6 steps |
| Rest | Stays put and sets `resting` | After 10 ticks |
| Wander | Follows the flood path to the destination chosen at 5b | On arrival |

- Any action that moves ends with outcome **`blocked`** when blocked re-planning finds no way forward (§3.7).
- **Every** action that hasn't ended otherwise ends at the **action timeout** of 60 ticks, with outcome `timed_out`.
- Bout lengths and the timeout are set in `physiology.ron`.
- **Outcomes:** `applied` / `walking` / `blocked` / `failed` / `timed_out`. `walking` and `blocked` can also be per-tick outcomes of an action that's still running.

### 5.6 Learning (step 4)

**`LearnableLinks`** is one unit shared by attention links A (State inputs × categories) and decision links W (concepts × verbs):
- **Two weights per link, Creatures-style:**
  - `w` is the working weight, and `w_long` the consolidated one.
  - Each tick, `w += relax_rate × (w_long − w)` and `w_long += consolidate_rate × (w − w_long)`.
  - Both start at the instinct value (0 if there isn't one).
- **Trace ring buffer:**
  - **Every tick**, each deciding sprite commits one entry at the end of step 6: that tick's step-5 snapshot plus the outcome. This applies whether the action just started or is continuing, e.g. `walking`.
  - A multi-tick action therefore leaves one entry per tick, each with its own activations. When a bite follows a 10-tick walk, the walking ticks *and* the bite tick are credited, and concepts such as `hungry & TargetAdjacent` (active only on arrival) get their share. This is standard eligibility-trace behaviour, not dilution.
  - Entries are kept while `λ^age ≥ 0.01`, up to a cap of 512.
- **Reinforcement:**
  - `r = reward − punishment`, read at step 4 (after step 3 has produced this tick's reward). **Both chemicals are then reset to 0.**
  - Consuming them makes reinforcement a one-off per tick. The trace spreads credit **backwards** only, to the decisions that led to the reward. In v1, reward lingered (with a ~3-tick half-life) and was read every tick. With λ = 0.9, decisions made *after* a reward collected about 4× the credit of the decision that earned it, and even a 1-tick half-life left the ratio at 1:1.
  - The consumed value is kept as the sprite's **`last_r`** (saved state), for display (§6.3) and debugging.
  - For each entry *j*, `weight_j = λ^(t − t_j)`. At tick *t*, the freshest entry is from *t−1*.
  - Decision links: `W[k][v_j] += η × learning_rate_mod × r × weight_j × a_{k,j}`.
  - Attention links: `A[i][c_j] += η × learning_rate_mod × r × weight_j × x_{i,j}`, but **only for entries whose verb used the target**.
  - All weights are clamped to [−1, 1].
  - When r = 0, the reinforcement pass is skipped, so it costs nothing.
- **Recruitment** (§5.4) is evaluated against the entry with the largest `weight_j × |r|`.
- **Learning milestone:** a `LearnedMilestone` event fires the first time any W link moves **±0.5** away from its instinct value.

### 5.7 Genes for the brain

| ID | Gene | Semantics |
|---|---|---|
| 7 | `BrainParam(param_id, value)` | One gene per parameter, clamped to a range set in `physiology.ron` (Appendix B) |
| 8 | `Instinct(inputs: [(InputId, negated)] ≤3, verb, weight)` | Creates or merges the concept with this signature and sets its starting W link to `verb`. A single input refers to that input's singleton. |
| 9 | `AttentionInstinct(InputId, CategoryId, weight)` | The starting A link. **Only State inputs are allowed.** Anything else is flagged and not expressed. |

All references are stable IDs. The value fields of each gene type, which are the only fields spawn variation and mutation change, are listed in §4.3.

### 5.8 Starter genome: deliberately imperfect

The M1 demo is watching learning happen, so the starter instincts are good but not perfect:

| Need | Attention instinct | Decision instinct | Deliberate mistake |
|---|---|---|---|
| Hunger | → BerryBush (strong), Berry (strong), **Thornbush (weak)** | hunger → Eat (general, not berry-specific) | Newborns sometimes try to eat thornbushes. The Thornbush singleton's Eat weight should become strongly negative through experience. |
| Thirst | → Water | thirst → Drink | |
| Tiredness | — | tiredness → Rest | |
| Boredom | → Ball | boredom → Play | |
| Loneliness | → Sprite | loneliness → Approach, Play | |
| Pain | — | pain → Retreat | |
| Crowdedness | → Sprite | crowdedness → Retreat; **`crowdedness & Sprite → Hit` (0.2)** | Sprites sometimes hit their neighbours. Correct training should reduce this. |

### 5.8.1 Budget

- **Links per sprite:** 43 singletons + up to about 60 innate conjunctions + up to 64 pool concepts gives ≤ ~170 concepts. × 8 verbs, plus attention at 35 × 6, comes to about 1.6k links, around 20 KB.
- **Whole world:** 100 sprites is about 2 MB.
- **Per tick:** a few thousand multiply-adds per sprite.

### 5.9 Legibility

`Brain::explain()` returns:
- the attention scores
- the concepts contributing most to the current verb's score
- the **learned links ranked by `|w − instinct|`**, e.g. `hungry & thornbush → eat  -0.71  (instinct +0.10)`

---

## 6. UI and the hand

### 6.1 Layout

- The minimum terminal size is **100×30**; 140×40 is recommended.
- A smaller terminal shows a clear "terminal too small" message.
- All text uses only CP437 characters (§6.2).

```
 Terra Sprites │ tick 48,210 │ ► 4x │ seed 7 │ sprites 27 │ bushes 141 │ saved 2m ago       ? help
┌─ Map ────────────────────────────────────────────┐┌─ Mira #12 ── [Body] Brain Chem Genome World ┐
│..,,,..~~~~≈≈≈≈~~..........♣....#########........ ││ EAT → berry bush · walking (3 tiles)        │
│.,,,...~~~≈≈≈≈≈~~....♣.........########....♠..... ││ age 3,410 · speed 7 · sense 10              │
│..,,...~~~~≈≈≈~~..........☺.......#####.......♣.. ││ hunger     ███████░░░ .71 ▲                 │
│...,....~~~~~~~.....•.........☺.....##....☺...... ││ thirst     ██░░░░░░░░ .18                   │
│.....,...................♣.............♠......... ││ pain       ░░░░░░░░░░ .00                   │
│....☺....○....................................... ││ tiredness  ████░░░░░░ .39                   │
│..........................♣.......☺.............♠ ││ energy .42 · hydration .77 · injury .05     │
└──────────────────────────────────────────────────┘└─────────────────────────────────────────────┘
┌─ Events ─────────────────────────────────────────────────────────────── [all|selected|major] ─┐
│ 48,207  Mira was pricked by a thornbush                                                        │
│ 48,190  Mira learned: hungry & thornbush → eat  -0.52                                          │
│ 48,102  Kel died (starvation, age 6,020)                                                       │
└────────────────────────────────────────────────────────────────────────────────────────────────┘
 (61,40) grass · berry (fresh) │ SELECT │ hand: Mira    E select  Q hand  Z reward  X correct
```

**Brain tab:**

```
┌─ Mira #12 ── Body [Brain] Chem Genome World ┐
│ ATTENTION             DECISION: EAT (1.42)  │
│ ► berry bush  1.21    hungry         +0.91  │
│   water       0.34    hungry & bush  +0.40  │
│   ball        0.12    always         +0.11  │
│ LEARNED (w, instinct)                       │
│ hungry & thornbush → eat   -0.52  (+0.10)   │
│ petted → play              +0.31  ( 0.00)   │
└─────────────────────────────────────────────┘
```

**Panels:**
- **Top bar:** tick, speed, seed, population, food counts, save status.
- **Map view:**
  - Its viewport scrolls with `W` `A` `S` `D` or the arrow keys, or follows the selected sprite (`f`).
  - A map smaller than the space gets a map view shrunk to fit it, at the top-left.
  - Its border is **double-lined** (`═ ║`) on any side where the terrarium's wall is in view, and single-lined where the map carries on.
  - The cursor is 3×3 tiles and follows the mouse (§6.5).
- **Inspector tabs** (`[` and `]`):

| Tab | Shows |
|---|---|
| **Body** | Current action and outcome, drive bars with trend arrows, physical levels, traits |
| **Brain** | Output of `explain()` |
| **Chem** | Every chemical with its level and change per tick, plus `last_r`. Reward and punishment are consumed every tick, so their levels always read 0 between ticks. |
| **Genome** | Genes grouped by type; flagged, unexpressed and unknown genes are marked; `g` exports to RON |
| **World** | Population, food counts, deaths by cause, and the data pack's identity |

- **Events panel:** filtered to all / the selected sprite / major events only (deaths, learning milestones, rejected commands).
- **Status line:** the tile under the cursor, the cursor mode, what the hand holds (in every mode), and hints for the active keys. A prompt such as "Quit? (y/n)" takes its place while open.
- **Overlays:** `?` for help (keys, colour legend, save path), `l` for a sortable sprite list to jump to.

### 6.2 Semantic tiles and themes

- **Drawn from meaning:** the map renders **semantic tiles**, e.g. `Terrain(Grass)`, `Object("berry_bush", "fruiting")`, `Sprite { colour_mode_state }`, `Emote(Hurt)`, never characters directly.
- **Themes** (`themes/*.ron`) map each semantic tile to a character, colours and modifiers.
  - Themes are **UI assets**. They aren't part of the sim data pack, and don't affect saves or replays.
  - The future Tiles milestone adds themes that map to a sheet index and tint instead.
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
| Cursor arrows | `↓ ↑ → ←` | `v ^ > <` | the cursor mode's colour |
| Mode marks: Select / Hand / Reward / Correct | `♦` `∩` `♥` `‼` | `S` `H` `R` `C` | the mode's colour |
| Status marks: idle / lift / drop / empty hand / sent / applied / rejected | `·` `↑` `↓` `░` `+` `☼` `?` | `-` `^` `v` `_` `+` `*` `?` | the mode's colour |

- Every **kind** of thing on the map has a unique glyph. Colour is used only for **state**, and the status line always names what's under the cursor. The cursor's arrows and marks are UI drawn over the map, and may share glyphs with each other (lift and drop are arrows).
- **`--ascii` swaps only what themes cover:** the map's glyphs, the cursor and the emotes. Frames and text stay CP437 in every theme.

### 6.3 Body language (UI only)

**Sprite colour modes** (`b` cycles them):
- **Dominant drive:** the highest drive above 0.5, or white if none is.

| Drive | Colour |
|---|---|
| hunger | yellow |
| thirst | cyan |
| pain | red |
| tiredness | blue |
| boredom | dark grey |
| loneliness | magenta |
| crowdedness | light red |

- **Plain.**
- M2 adds **lineage**.

**Emotes** swap with the sprite glyph at about 2 Hz, for about 1 second of **real** time:

| Emote | `cp437` | `ascii` | Triggered by |
|---|---|---|---|
| Hurt | `!` | `!` | `Pricked`, a hit received, `Corrected` |
| Failed | `?` | `?` | `ActionEnded` with outcome `failed`, `blocked` or `timed_out` |
| Resting | `z` | `z` | While Rest lasts |
| Pleased | `♥` | `+` | `Rewarded`, or `last_r` above the UI's spike threshold |

- Emote timers live in `App` and are driven by **sim events**, so emotes don't vanish at 16× or Max speed.
- Body language never changes the sim.

### 6.4 `InfoPolicy`

- **Every** information display asks `InfoPolicy::can_view(panel, subject)` before showing anything. That includes map colours, emotes, the events panel, counts in the top bar, the sprite list, the tile-info line and every inspector tab.
- In M1 the answer is always yes.
- A future diegetic Play mode is a new policy. The policy only controls what the UI **reveals**. A physical in-world scanner would be a separate sim feature with its own state and commands.

### 6.5 The hand

- **Scrolling:** `W` `A` `S` `D` or the arrow keys scroll the viewport one tile, and Shift makes it 5. Holding a key keeps scrolling. The viewport stops at the wall. There is **no keyboard cursor**.
- **The cursor follows the mouse.** It sits on the tile under the pointer, so it changes as the map scrolls beneath a still pointer. When the pointer leaves the map view, the cursor stays on its last tile. A click acts on the cursor's tile, as the cursor mode says.
- **Reach:** the hand reaches anywhere on the map.
- **Ownership:** **the hand lives in the sim.** `World` owns `Hand { held: Option<HeldEntity> }`.
  - A held sprite or item stays in its World storage with location `Held`. It's left out of the occupancy index and perception.
  - Its biochemistry, learning and item lifecycle keep running (§2.4, §3.5.2).
  - `App` owns the cursor and the cursor mode, and reads `world.hand()` to draw them.

**The cursor** is 3×3 tiles, centred on its target:

```
M ↓ Y      M: the mode mark (top-left and bottom-right)
→ ☺ ←      Y, N: the status marks (top-right, bottom-left)
N ↑ M      centre: the target tile, in reverse video, glyph still visible
```

- The arrows and marks take the mode's colour. Parts that fall outside the map view aren't drawn.
- The cursor covers the 8 tiles around its target while it sits there; the status line still names the target.
- The arrows and marks are theme glyphs (§6.2).

**Cursor modes:**

| Key | Mode | Mark | Colour | A click on the cursor's tile… | `Y` / `N` |
|---|---|---|---|---|---|
| `E` | **Select** (the default) | `♦` | white | selects the sprite there; on an empty tile, clears the selection | `·` / `·` |
| `Q` | **Hand** | `∩` | yellow | with the hand empty, sends `Grab` (a sprite is taken before a loose item); holding something, sends `Drop`. Pressing `Q` again opens the Place menu | empty: `↑` / `░`; holding: `↓` / the held thing's glyph |
| `Z` | **Reward** | `♥` | green | sends `Reward` to the sprite there | feedback (below) |
| `X` | **Correct** | `‼` | light red | sends `Correct` to the sprite there | feedback (below) |

- **The mouse wheel cycles the modes:** a notch down picks the next (Select → Hand → Reward → Correct, then back to Select), a notch up the previous. A wheel event also points, like every mouse event. The wheel doesn't scroll the map.
- **Place menu:** berry, ball, bush seedling, new sprite (starter genome + variation), sprite from a genome file. The chosen item waits in the hand's marks, and the next click on a tile sends `Place { tile, object_type }` or `SpawnSprite`.
- **Feedback:** in Reward and Correct mode, each click flashes `+` in both status marks at once ("sent"). When the sim reports back, they flash `☼` (applied) or `?` (rejected). Flashes last about 0.3 s of **real** time and are driven by events, like emotes (§6.3). In Hand mode a rejected `Grab`, `Drop` or `Place` flashes `?`.
- **Nothing to act on:** a click in Hand, Reward or Correct mode with nothing on the tile to act on sends no command and flashes `?` at once.
- **Switching modes keeps each mode's state.** What the hand holds, or a Place item not yet put down, waits in reserve while other modes are in use; the selection stays in every mode. While the hand holds something, the status line shows it in every mode (`hand: Mira`), because a held sprite can't eat or drink (§2.4) and a held berry can rot.
- **Leaving a mode:** a right-click or `Esc` returns to Select. `Esc` first closes any open menu or overlay. From Select, `Esc` asks to quit (§6.6).
- **Pausing queues actions.** Commands are stamped for the next tick (§2.5), so clicks made while paused apply, in click order, when time next moves (`space` or `.`). Hand mode's marks follow the queue: after a queued `Grab`, `Y` shows `↓` and `N` the thing being grabbed. If the sim rejects it, `?` flashes and the marks return to the hand's real state.

**Other keys:**

| Key | Action |
|---|---|
| `Tab` / `Shift+Tab` | Cycle through sprites |
| `f` | Follow the selected sprite |
| `b` | Cycle the colour mode |
| `[` / `]` | Switch inspector tabs |
| `g` | Export the genome (Genome tab) |
| `r` | Rename the selected sprite |
| `l` | Open the sprite list |
| `?` | Open help |
| `Esc` | Close a menu or overlay; otherwise back to Select; from Select, ask to quit |
| `Ctrl+C` | Quit at once |

- A rejected command shows its reason on the status line and in the event log.
- **Names:** sprite names come from a fixed syllable generator applied to the sprite's ID. It uses no RNG draws, so names are deterministic.

### 6.6 Time and the frame loop

- **Keys:** `space` pauses and resumes; `.` steps one tick while paused; `+` and `-` step through ⅛×, ¼×, ½×, 1× (10 ticks per second), 2×, 4×, 8×, 16× and **Max**. Each step halves or doubles the rate. The game starts at 1×; `-` stops at ⅛× (1.25 ticks/s) and `+` at Max. The top bar labels the slow speeds `1/2x`, `1/4x` and `1/8x`.
- **Quitting:** `Esc` (from Select, with no menu open) asks "Quit? (y/n)". `y` or a second `Esc` quits; any other key cancels. `Ctrl+C` quits at once.
- **Exact pacing:** the UI clock counts owed ticks in integer maths, with rates in eighths of a tick per second, so every speed (including ⅛×) runs at exactly its nominal rate with no drift.
- **Held keys:**
  - **1× is a stop for held keys.** A held `+` or `-` stops at 1×; a fresh press is needed to go past it, in either direction.
  - **`space` toggles pause only on a fresh press,** so holding it doesn't flicker.
  - **Holding `.` keeps stepping,** one tick per repeat.
  - **Telling a hold from a press:** a key counts as held when the terminal reports it as repeating, or when it's pressed again with no release in between. The second rule is only trusted where the terminal reports releases: Windows always does, and any other terminal is trusted once a release arrives. Keys are tracked by physical key, so a `+` whose release is reported as `=` (Shift released first) still counts as released.
  - **Limitation:** terminals that report neither repeats nor releases can't tell a hold from taps, so every press there counts as fresh.
- **Single-threaded in M1.** Each frame:
  1. drain input into `Action`s and `Command`s
  2. run as many ticks as the speed allows, within a **~25 ms** budget
  3. render, at about 30 fps
- At Max, the sim uses whatever time rendering leaves.

### 6.7 Files

- **Data:** the default data pack is embedded in the binary. `--data <dir>` overrides it for **new** worlds (§2.8).
- **Saving and loading:** `F5` quicksaves, `F9` quickloads, and `Ctrl+S` / `Ctrl+O` save or load by name.
- **Autosave:** every 10 real-time minutes, keeping the last 3.
- **Location:** saves, autosaves, exported genomes and `last_session.replay` go in the platform data folder. The help overlay shows the path.
- **Command-line flags:** `--seed <n>`, `--preset <file>`, `--data <dir>`, `--ascii`, `--replay <file>`.

### 6.8 UI architecture

- **`App`** holds the UI state: cursor, cursor mode (and each mode's reserve), viewport, selection, screen state (Normal / Menu / Prompt / Help), colour mode, emote and feedback timers, and the active `InfoPolicy`.
- **Input:** a keybinding table maps keys to UI `Action`s. Each `Action` either changes `App` or produces a `Command`.
- **Rendering** is a pure function, `render(frame, &App, &World)`.
- **Panic hook:** it restores the terminal and flushes the replay.

---

## 7. Testing and acceptance

Implementation is **test-first, one vertical slice at a time.** Everything in `terra-sim` is deterministic for a given seed, so no test is flaky: results change only when the code or the tuning does.

### 7.1 Tools

- **`World::state_hash()`:** xxh3 with a fixed seed over a canonical serialization of the world.
- **Replay checkpoints:** a hash every 1,000 ticks, and playback reports the first divergence (§2.7).
- **`World::check_invariants()`:** runs every tick in debug builds and tests. It checks:
  - the occupancy index matches entity positions
  - concentrations are within [0, 1]
  - every fixture has its clear ring
  - held entities appear in neither the index nor perception
  - IDs only go up
- **Lab runner:** `cargo run -p terra-sim --example lab -- scenarios/<name>.ron --seeds 10` prints metrics. The scenarios double as tests and as the main tuning tool.
- **Trainer hook:** a scenario can include a trainer. It reads tick *t*'s events and submits commands stamped *t+1* through the same queue as the player.

### 7.2 Test layers

1. **Unit tests (`terra-sim`):**
   - **Biochemistry:**
     - half-life factors, reaction extents, emitter Level/Rise/Fall, receptors
     - the two-pass emitter order: a pulse-keyed drive drop produces its Fall reward in the same tick
     - the gene restriction table (genes that break it are flagged, not expressed)
     - spawn variation changes only value fields, and a varied starter genome has no flagged genes
     - the pulse latch
   - **Brain:**
     - concept products and negation, input groups, verb kinds and availability
     - sampling only at boundaries, deterministic switching and tie-breaks
     - Wander destination sampling, Retreat's Chebyshev step choice and direction order, action bounds and outcomes
     - one trace entry per tick, including while an action continues
     - learning consumes reward and punishment: a single reward burst credits only entries from before it, and `last_r` records it
     - ring-buffer weighting, two-timescale weights, recruitment and freeing
   - **World:**
     - flood reachability, candidate tie-breaks, goal tiles
     - integer move points, conflict order, head-on swaps (including diagonal corner rules)
     - blocked re-planning: the one-off search treats occupied tiles as impassable; a found path is committed and survives flood refreshes; it's dropped when blocked again or when a sprite target moves; no path ends in `blocked`
     - re-planning toward a moving target without re-flooding
     - every rule condition and effect, short-circuiting, held-object semantics
     - verb-only effects in lifecycle rules are load errors; objects created during step 2 first run next tick
   - **Commands:** every rejection path.
2. **Property tests (`proptest`):**
   - The map is connected for any seed.
   - Random sequences of fixture placements and removals never disconnect it.
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
   - ratatui `TestBackend` snapshots cover each inspector tab and the map in both themes.
   - A deny-all `InfoPolicy` blanks **every** display.
   - Emote timers work from events.
   - The too-small-terminal message appears.
   - All UI strings are within CP437.
7. **Benchmarks (`criterion`, run locally):**
   - the whole tick with 100 sprites
   - one sprite's flood
   - one sprite's brain step

### 7.3 Behaviour scenario protocol

**Protocol** (A2 and A3):
- **Training:** the trainer reacts only to events from ticks **0–9,899**.
- **Washout:** ticks 9,900–9,999 have no trainer. Any command the trainer queued lands before measurement begins, and the last training reward has been consumed.
- **Measurement:** ticks 10,000–20,000, with **no trainer in either run**, so we measure what was learned, not ongoing reward.
- **Counts:** only actions with outcome `applied` are counted.
- **Control:** each trained run is compared with a matched control (same seed, no trainer), taking the **median across 10 seeds**.
- **Minimum baseline:** a scenario only counts if the control has **≥20 counted actions** in the measurement window. Below that, the test **fails** as badly calibrated.

| # | Scenario | Trainer during training | Pass condition |
|---|---|---|---|
| A1 | 1 sprite; berry bushes, thornbushes, water | none | Applied thornbush Eats per 5,000 ticks: ticks 15k–20k ≤ 50% of ticks 0–5k. This compares the sprite with its own earlier self, so it has no control run. It needs ≥20 such Eats in ticks 0–5k; below that it fails as badly calibrated. |
| A2 | 1 sprite; balls, food, water | Each applied Play on a Ball at tick *t* < 9,900 → `Reward(actor)` stamped *t+1* | Applied Play-on-Ball count ≥ 1.5× control |
| A3 | 4 sprites in a small arena; food, water | Each applied Hit on a Sprite at tick *t* < 9,900 → `Correct(actor)`, meaning **the hitter**, stamped *t+1* | Applied Hit-on-Sprite count, all sprites combined, ≤ 0.5× control |

### 7.4 M1 acceptance criteria

M1 is **done** when all of the following hold:

| # | Criterion | Measured by |
|---|---|---|
| A1 | The thornbush lesson (§7.3) | Scenario test |
| A2 | Reward training (§7.3) | Scenario test |
| A3 | Correct training (§7.3) | Scenario test |
| A4 | **Viability.** In the default world, ≥80% of sprites survive the first 10,000 ticks, and starvation plus dehydration cause <25% of deaths over the first 50,000 ticks (median of 10 seeds) | Scenario test |
| A5 | **Performance.** 100 sprites at ≥200 ticks per second in a release build on the development machine | Benchmark |
| A6 | **Determinism and persistence.** Test layers 3 and 4 pass | Tests |
| A7 | **Robustness.** A headless soak of 1,000,000 ticks at Max speed with a random script of hand commands produces no panics and no invariant violations | Soak test |
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
| 64 | learning_rate_mod | Receptor target |
| 65 | exploration_mod | Receptor target |

**Categories:**

| ID | Category |
|---|---|
| 1 | BerryBush |
| 2 | Berry |
| 3 | Thornbush |
| 4 | Water |
| 5 | Ball |
| 6 | Sprite |

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
| 36–41 | AttendedCategory 1–6 | Target |
| 42 | TargetDistance | Target |
| 43 | TargetAdjacent | Target |

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

**Traits:**

| ID | Trait |
|---|---|
| 1 | speed |
| 2 | sense_radius |
| 3 | lifespan |

## Appendix B — `physiology.ron` contents

This file fixes the **mechanisms and ranges**. The starting values are tuned with the lab runner.

- **Newborn physical levels:** energy, hydration, stamina, injury (0), gut contents (0).
- **Metabolism:**
  - basal energy per tick
  - cost per tile of sense radius
  - base step energy cost × (speed / 8)²
- **Digestion:** `food → energy` and `water → hydration` rates.
- **Hydration, stamina and healing:** hydration loss per tick; stamina drain per step, recovery while idle, recovery while resting; healing per tick.
- **Injury from starvation, dehydration and senescence:** amounts per tick, and the window used to attribute cause of death (500 ticks).
- **Trait ranges:** speed 4–12, sense_radius 6–14, lifespan 20,000–200,000.
- **BrainParam ranges (initial):**

| Parameter | Range |
|---|---|
| η (`learning_rate`) | 0.001–0.5 |
| λ (`trace_decay`) | 0.5–0.99 |
| `relax_rate` | 0–0.01 |
| `consolidate_rate` | 0–0.001 |
| `tau_base` | 0.05–2.0 |
| `tau_att_base` | 0.05–2.0 |
| `switch_margin` | 0–1 |
| `attention_margin` | 0–1 |
| `salience_gain` | 0–2 |
| `pool_size` | 0–64 |
| `max_arity` | 1–3 |
| `recruit_threshold` | 0–1 |
| `novelty_threshold` | 0–1 |
| `forget_ticks` | 100–100,000 |

- **Receptor target ranges:** `learning_rate_mod` 0.5–2.0, `exploration_mod` 0.25–4.0, both neutral at 1.0.
- **Hand stimuli:** Reward's `reward` amount; Correct's `punishment` and `pain` amounts.
- **Behaviour constants:**
  - Retreat bout: 6 steps
  - Rest bout: 10 ticks
  - action timeout: 60 ticks
  - flood refresh: every 8 ticks
  - occupied-tile penalty
  - re-plan after 3 blocked ticks
  - `nearby_sprites` radius (3) and normalization (4)
- **Spawn variation:** ±10%.
- **Learning milestone threshold:** 0.5.

## Appendix C — Risks

| Risk | Mitigation |
|---|---|
| Learning is too slow, too fast or unstable | Lab runner, and A1–A4 as guardrails; every constant is data |
| Concept recruitment adds tuning burden | `pool_size = 0` turns it off; singletons still give a linear model that works |
| Reward lands on the wrong decision | Learning consumes reward every tick (§5.6); one-shot actions give sharp, pulse-keyed relief; Fall emitters have a deadband (§4.5). What's left is ongoing pain (e.g. starvation) punishing whatever the sprite is doing, which is inherent to reward from drives. |
| Corridor congestion | Head-on swaps; blocked re-planning ends hopeless actions early (§3.7) |
| The perception flood is too expensive | Benchmarked early; the refresh interval and radius range are data |
| Cross-platform determinism is unverified | Promised only for targets in the CI determinism matrix (both x86_64 and aarch64) once its check passes; "expected" elsewhere (§2.6, §7.5) |
| A behaviour-test threshold is badly calibrated | The minimum-baseline rule fails loudly; amend thresholds, never delete tests |
