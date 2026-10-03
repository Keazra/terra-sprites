# Data-format reference

Much of Terra Sprites is data rather than code: what the ground is like, what plants and toys do, what a sprite's body and instincts are, how big the world is. This reference covers every file you can edit, with a working example of each. The design doc (`docs/design/`, the highest `-vN`) is the full spec; each page here cites the sections it summarises.

## The files

| File | What it says | Page |
|---|---|---|
| `data/pack.ron` | The pack's name and version | [pack](pack.md) |
| `data/terrain.ron` | What each kind of ground is like | [terrain](terrain.md) |
| `data/categories.ron` | What sprites perceive things as | [categories](categories.md) |
| `data/tags.ron` | What touching a thing with a tag does, such as thorns | [tags](tags.md) |
| `data/objects.ron` | Every kind of plant, item and toy, and what it does | [objects](objects.md) |
| `data/chemicals.ron` | The chemicals in a sprite's body | [chemicals](chemicals.md) |
| `data/loci.ron` | The other values genes and objects read and write | [loci](loci.md) |
| `data/brain_io.ron` | What a sprite's brain can feel | [brain inputs](brain-io.md) |
| `data/physiology.ron` | The body's fixed rules, and the Cursor's touch | [physiology](physiology.md) |
| `data/names.ron` | The syllables random names are made of | [names](names.md) |
| `data/genomes/starter.ron`, and genome files | A sprite's genes | [genomes](genomes.md) |
| `data/presets/default.ron`, and preset files | A world's size and what's in it | [world config and presets](presets.md) |
| `scenarios/*.ron` | Headless test worlds for the lab runner | [lab scenarios](lab-scenarios.md) |

Themes (`themes/*.ron`), the glyphs and colours the screen draws, aren't on the list for now: the game can't load a theme of your own yet, only pick between its two built-in ones with `--ascii`. Loading one is [#109](https://github.com/Keazra/terra-sprites/issues/109), and themes get their page here with it.

## How the game uses them

- **The data pack** is everything in `data/` except the presets: the first eleven rows above. It's built into the game, so a change to `data/` takes effect the next time the game is built (`cargo run --release`). A file that's wrong stops the game at start with a message naming the file and the problem. A world keeps the pack it was made with, so editing the pack never changes a world already saved (design §2.8).
- **A folder of pack files** loads at start with `--data <dir>`, without rebuilding: each file of the pack the folder has, at the same path as in `data/` (`objects.ron`, `genomes/starter.ron`), replaces the built-in one, and any it lacks stays built in. A `presets/default.ron` in it is the default preset. It's for new worlds only: a save keeps the pack it was made with (design §2.8).
- **Presets** load at start with `--preset <file>`, without rebuilding.
- **Genome files** are what `g` exports and what the Place menu reads back, in the `genomes` folder of the game's folder (the help screen, `?`, shows where).
- **Lab scenarios** run headless with `cargo run --release -p terra-sim --example lab -- scenarios/<name>.ron --seeds 10`.

To try a change to the pack without starting the game, run the tests: `cargo test -p terra-sim --test data_pack` loads the built-in pack and fails with the same message the game would give.

## Writing RON

Every file is [RON](https://github.com/ron-rs/ron), Rust's object notation:

- `( ... )` is a record of named fields: `(name: "core", version: "1")`.
- `[ ... ]` is a list, `{ ... }` a map from keys to values: `{"fruit": 6}`.
- `Name(...)` is one of a fixed set of choices, with what it carries: `Stage("mature")`, `Inject(Actor, "food", 0.3)`. A choice that carries nothing is just its name: `Expire`, `Large`.
- Strings are in double quotes and characters in single quotes (`'S'`). `//` starts a comment.
- A trailing comma is fine. A field the format doesn't know is an error, so a misspelt field name is caught.

## Rules every file shares

- **IDs are permanent.** Where entries have an `id`, it's how saves, genomes and brains refer to them. Add new entries with new IDs, never renumber one, and never reuse the ID of one you removed (design §2.8). An ID or name used twice is an error.
- **Names are how files refer to each other.** An object type names its category, a verb injects a chemical by name, a genome names chemicals and loci. A name that doesn't exist is an error naming the file and the entry.
- **The engine enforces physics only.** What things do, and what they're called, belongs in the data. The one exception is content safety, which no data pack or genome can change.
- **Numbers that are fractions of full** (chemical levels, chances, fertility) run from 0 to 1. Times are in ticks; at 1× the game runs 1.25 ticks a second.
