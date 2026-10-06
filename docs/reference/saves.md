# Saves and replays

A **save** is a whole world, frozen at one tick, that the game can load back exactly as it was (design §2.8, §6.7). A **replay** is a whole session, which the game can play back tick for tick (design §2.7). Unlike the other files in this reference, neither is for editing: the game writes them and reads them, and their insides are opaque. This page says what a save keeps, where saves go and what happens when one can't be loaded, then how to record a replay, play it back, take it over, and read what it says if it ever parts from its recording.

To carry a single sprite from one world to another, export its genome with `g` instead ([genomes](genomes.md)): a genome file is text you can read and edit, and the Place menu reads it back.

## Making and loading saves

| Key | What it does |
|---|---|
| `F5` | Saves the world as the **quicksave**, over the last one. |
| `F9` | Loads the quicksave. |
| `Ctrl+S` | Saves under a name you type, up to 40 characters. It offers the seed and tick, such as `seed 7 tick 48210`; typing replaces it. The same name saves over the old one. |
| `Ctrl+O` | Lists every save, newest first, the quicksave and autosaves included, to pick one to load. |

The title screen loads them too: **Continue** loads the newest save, whichever kind it is, and **Load** lists them all as `Ctrl+O` does.

- **Autosaves.** The game saves itself every 10 minutes of running time, and when you quit or go back to the title screen. Paused time doesn't count, and quitting doesn't save again if the world hasn't run since it was last saved. It keeps the last 3: `autosave-1` is the newest.
- **Loading asks first** if the world on screen has run since it was last saved, since loading would lose that: "Load quicksave? The world has run 4m since it was last saved (y/n)". `y` loads; so does `F9` again, when it's the quicksave asked about. Any other key keeps the world.
- **A loaded world starts paused,** with the view and the Cursor where they were when it was saved. Your settings, such as the speed, the theme and the open inspector tab, stay as they were.
- The top bar says how long ago the world was last saved, autosaved or loaded: `not saved`, `saved just now` for the first 3 seconds, then `saved 4s ago`, `saved 20s ago`, `saved 3m ago`.

## Where saves go

Saves are files in the `saves` folder of the game's folder. The help screen (`?`) shows the game's folder; it's

| System | The game's folder |
|---|---|
| Windows | `%APPDATA%\terra-sprites`; in PowerShell, which Windows Terminal opens by default, that's `$env:APPDATA\terra-sprites` |
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
| "Couldn't load *name*: its data pack has no *file*" or "…its data pack's *file* doesn't load: …" | The pack inside the save uses something this version of the game can't read. |
| "Couldn't load *name*: …" with any other reason | The file couldn't be read at all, such as one deleted or locked by another program; the reason is the system's own words. |

Saving can fail too, as "Couldn't save: …" or "Couldn't autosave: …", most often from a full disk or a folder the game can't write to. The world on screen carries on either way, and the last good save is left as it was.

**Old saves keep loading.** A save made by an earlier version of the game loads in a later one: when the save format changes, the new version upgrades older saves as it reads them (design §2.8). The format's number only goes up when it has to.

## The format, for the curious

You don't need this to play or to mod; it's here so nobody has to read the code to find out. A save is a 4-byte magic, `TSPR`, then a short header that never changes shape (the save format's number, the version of the game that wrote it, and an xxh3 checksum of the rest), then the world, encoded as MessagePack with named fields. The world's part is internal and changes between versions, so tools shouldn't rely on it. Design §2.8 lists it in full.

## Replays

Every session records itself as it goes. The game keeps the **session log**, a replay of everything since the world was made or last loaded, in the game's folder (not its `saves` folder) as `last_session.replay`. Playing it back shows the same world doing the same things, sprite for sprite and tick for tick, because the same world given the same clicks always plays out the same way.

That makes a replay the way to show someone what happened, such as a sprite that did something odd, or the game crashing: the replay leads up to the moment.

### Recording

There's nothing to switch on. The session log is written:

- at each autosave, every 10 minutes of running time;
- when you quit, or go back to the title screen;
- when the game crashes, after it has put the terminal back, so a crash still leaves its replay;
- when the game stops because the terminal failed.

It holds where the world started, every command your clicks and keys sent the world (the ones the world refused too), and a fingerprint of the world, a **checkpoint**, where the recording starts and every 1,000 ticks. Where it starts:

- **A new world** starts the session log from its seed and world config.
- **Loading a save** (`F9`, `Ctrl+O`, or from the title screen) starts it afresh, from the save. A replay holds one stretch of play, so what came before the load is gone from it.
- **Taking over a replay** starts it afresh too (below).

**Each session writes over the last one's `last_session.replay`,** at its first autosave or when it's closed, whichever comes first. To keep a replay, copy it somewhere else, or give it another name, before you start the game again. Playing one back with `--replay` never writes the session log, so watching a replay doesn't replace it.

### Playing one back

Start the game with `--replay` and the replay's file. From the source, that's `cargo run --release -- --replay <file>`; with the built game, `terra-sprites --replay <file>`. The help screen (`?`) shows the game's folder, which is where `last_session.replay` is:

| System | Playing back the session log |
|---|---|
| Windows (PowerShell) | `cargo run --release -- --replay "$env:APPDATA\terra-sprites\last_session.replay"` |
| Windows (Command Prompt) | `cargo run --release -- --replay "%APPDATA%\terra-sprites\last_session.replay"` |
| macOS | `cargo run --release -- --replay ~/Library/Application\ Support/terra-sprites/last_session.replay` |
| Linux | `cargo run --release -- --replay ~/.local/share/terra-sprites/last_session.replay` |

On Windows, use the line for the shell you're in: PowerShell, which Windows Terminal opens by default, only understands `$env:APPDATA`, and `%APPDATA%` works only in the Command Prompt.

A replay brings its own world, so `--seed`, `--preset` and `--data` can't go with it; `--ascii` and `--theme` can.

- **It starts paused,** saying where it's from and how far it goes: "Replaying last_session.replay: from a new world to tick 48,210", or "from a save at tick 300". The top bar says "replay to tick 48,210" where it would say when the world was last saved.
- **You watch; the recording plays.** Time works (pause, step, speed), as do the view, selecting, following, the sprite list and the inspector. Whatever would change the world is refused, with "It's a replay: only time, the view and the inspector work, until Ctrl+R takes it over": the Train and Grab modes, the mouse wheel over the map, naming a sprite, showing the Cursor (`H`) and loading a save. Saving (`F5`, `Ctrl+S`) and exporting a genome (`g`) still work, since they only read the world.
- **The recorded player's Cursor isn't drawn,** though what it did happens: the pets land, the items move.
- **At the end it pauses once:** "The replay ends here, at tick 48,210: time can run on, with nothing more done". The top bar says "replay ended". Time can run on from there, with nobody clicking.
- **A replay never autosaves,** as the world isn't yours until you take it over.

### Taking over

`Ctrl+R` during playback makes the world yours from the tick the replay has reached: "You took over the replay at tick 4,210: it's your world now". Pause first to take over at an exact moment.

- **You take over the recorded Cursor:** where it is, what it holds or leads (in Grab mode if it has hold of something), and whether sprites can see it.
- **The rest of the recording is dropped.** The clicks it had still to play never happen.
- **The replay file doesn't change,** so you can play it back again and take over somewhere else.
- **A new session log starts** from the world as you took it over, and the top bar goes back to saying when the world was last saved. Its first write, at the next autosave or when you quit, replaces `last_session.replay`. If you're playing back `last_session.replay` itself, copy it first to keep it.

### When a replay parts from its recording

Playback checks each checkpoint as it reaches it, the start's at once. A replay plays the very same world in the very same version of the game, so they always match, unless something in the game isn't as deterministic as it should be, which is a bug. (The game tells versions apart by their number, so a game you've changed and rebuilt without changing its version plays an older replay, which can then part from its recording. That one isn't a bug.) If one doesn't match, playback pauses and says between which checkpoints the world parted:

| Message | What it means |
|---|---|
| "The replay parted from its recording between ticks 1,000 and 2,000" | The checkpoint at tick 1,000 matched and the one at 2,000 didn't, so the world went its own way somewhere in those ticks. A checkpoint every 1,000 ticks can't name the tick itself. |
| "The replay was different from its recording from tick 300" | Not even where it starts matched. |

The top bar then says "replay diverged by tick 2,000". Time can run on, but from there it shows what the game does now, not what happened. Please report it as an issue, with the replay file and the message: the ticks it names are where to look.

### When a replay won't play

A replay that can't be played stops the game before it starts, with a message such as "terra-sprites: can't play the replay last_session.replay: it isn't a Terra Sprites replay".

| The reason it gives | What it means |
|---|---|
| "it isn't a Terra Sprites replay" | The file doesn't start the way every replay does, such as a save given to `--replay`. |
| "it was recorded by Terra Sprites 0.2.0 (save format 2), and a replay plays only in the build that recorded it: this is 0.1.0 (save format 1)" | A different version of the game recorded it. Unlike a save, a replay isn't upgraded: play it in the version that recorded it. |
| "it's damaged: …" | The file has changed since it was written. Like a save, every replay carries a checksum of its contents. |
| "it names the data pack …, but its world was made with …" | The replay's world wasn't made with the data pack the replay says it was. |
| "its data pack has no *file*", "its data pack's *file* doesn't load: …" or "its start doesn't load: …" | The pack or the save inside the replay can't be read by this version of the game. |
| Any other reason | The file couldn't be read at all, in the system's own words. |

If the game can't write the session log, it says "Couldn't write the replay: …" on the status line, or, once the game has closed, in the terminal.

### The replay format, for the curious

A replay is a 4-byte magic, `TSRP`, then the same short header a save has (the save format's number, the version of the game, a checksum of the rest), then MessagePack with named fields: the data pack, as the text of each of its files with its name, version and a hash; where it starts, a seed and world config or a save; each command with the tick it's applied at; the checkpoints, as `(tick, state hash)`; and the tick the recording reached. Design §2.7 lists it in full. Like a save's world, it's internal and changes between versions.
