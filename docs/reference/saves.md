# Saves

A **save** is a whole world, frozen at one tick, that the game can load back exactly as it was (design §2.8, §6.7). Unlike the other files in this reference, a save isn't for editing: it's written by the game and read by the game, and its insides are opaque. This page says what a save keeps, where saves go, and what happens when one can't be loaded.

To carry a single sprite from one world to another, export its genome with `g` instead ([genomes](genomes.md)): a genome file is text you can read and edit, and the Place menu reads it back.

## Making and loading saves

| Key | What it does |
|---|---|
| `F5` | Saves the world as the **quicksave**, over the last one. |
| `F9` | Loads the quicksave. |
| `Ctrl+S` | Saves under a name you type, up to 40 characters. It offers the seed and tick, such as `seed 7 tick 48210`; typing replaces it. The same name saves over the old one. |
| `Ctrl+O` | Lists every save, newest first, the quicksave and autosaves included, to pick one to load. |

- **Autosaves.** The game saves itself every 10 minutes of running time, and when you quit. Paused time doesn't count, and quitting doesn't save again if the world hasn't run since it was last saved. It keeps the last 3: `autosave-1` is the newest.
- **Loading asks first** if the world on screen has run since it was last saved, since loading would lose that: "Load quicksave? The world has run 4m since it was last saved (y/n)". `y` loads; so does `F9` again, when it's the quicksave asked about. Any other key keeps the world.
- **A loaded world starts paused,** with the view and the Cursor where they were when it was saved. Your settings, such as the speed, the theme and the open inspector tab, stay as they were.
- The top bar says how long ago the world was last saved, autosaved or loaded: `not saved`, `saved just now`, `saved 3m ago`.

## Where saves go

Saves are files in the `saves` folder of the game's folder. The help screen (`?`) shows the game's folder; it's

| System | The game's folder |
|---|---|
| Windows | `%APPDATA%\terra-sprites` |
| macOS | `~/Library/Application Support/terra-sprites` |
| Linux and others | `$XDG_DATA_HOME/terra-sprites`, or `~/.local/share/terra-sprites` |

Each save is one file ending in `.tspr`: `quicksave.tspr`, `autosave-1.tspr` to `autosave-3.tspr`, and each named save as its name, such as `seed 7 tick 48210.tspr`.

- **Names are made safe for any system.** A character some system can't have in a file name (`/ \ : * ? " < > |`) becomes `-`, dots and spaces at the end are left off, and a name Windows keeps for a device, such as `con` or `lpt1`, gets a `-` after it. The game always says a save's name as it's stored, which is the name the load list shows.
- **A save is never half written.** The game writes the new file beside the old one, then puts it in its place, so a save cut short, by a crash or a full disk, leaves the old save as it was.
- **You can copy, rename and delete save files** while the game isn't using them. A file renamed in the folder is listed under its new name. Copying the folder is a backup.

## What a save keeps

Everything that decides what happens next, so a loaded world carries on exactly as the saved one would have:

- every sprite: its body chemistry, its brain and everything it has learned, what it's doing and where it's going;
- every plant, item and toy, with its stage and timers;
- the map, the tick, and the world's random numbers, so the same clicks give the same future;
- what the Cursor holds or leads, and clicks made while paused that haven't happened yet;
- **the data pack the world was made with**, as the text of each of its files.

That last point means a world keeps its own rules. Editing `data/` and rebuilding changes new worlds, never one already saved: loading a save always uses the pack inside it (design §2.8). It also keeps the seed and preset the world was made from, which the top bar shows, and where the view was.

What's yours rather than the world's isn't in a save: the speed, the theme, the colours, the inspector tab, the event log's filter, and the event log itself.

## When a save won't load

A save that can't be loaded is refused, with the reason on the status line, and the world on screen carries on as it was. The game never crashes on a bad save.

| Message | What it means |
|---|---|
| "Couldn't load *name*: it isn't a Terra Sprites save" | The file doesn't start the way every save does. |
| "Couldn't load *name*: a newer Terra Sprites (0.2.0) saved it, in save format 2; this one reads formats up to 1" | A later version of the game wrote it. Load it in that version or newer. |
| "Couldn't load *name*: it's damaged: …" | The file has changed since it was written, or doesn't fit its own world. Every save carries a checksum of its contents, so a file damaged on disk, or edited by hand, is caught before it's read. Try an autosave. |
| "Couldn't load *name*: its data pack's *file* doesn't load: …" | The pack inside the save uses something this version of the game can't read. |

**Old saves keep loading.** A save made by an earlier version of the game loads in a later one: when the save format changes, the new version upgrades older saves as it reads them (design §2.8). The format's number only goes up when it has to.

## The format, for the curious

You don't need this to play or to mod; it's here so nobody has to read the code to find out. A save is a 4-byte magic, `TSPR`, then a short header that never changes shape (the save format's number, the version of the game that wrote it, and an xxh3 checksum of the rest), then the world, encoded as MessagePack with named fields. The world's part is internal and changes between versions, so tools shouldn't rely on it. Design §2.8 lists it in full.

Replays, which record a session so it can be played back tick for tick, will get their own section with slice 13 ([#14](https://github.com/Keazra/terra-sprites/issues/14)).
