# Themes

A **theme** says how the map is drawn: the character and colour of each kind of ground, each plant and toy, sprites, their emotes, the Cursor, the glass round the map and the stars in space past it (design §6.2, M2 design §11), the colours and blocks of the population graph (M2 design §14), and the lineage colours (§17). It also colours the screen's text by its job, shades the panels beside and under the map, or leaves them to lines (design §6.1, §6.2), and gives motion its colours and the look of a drive's bar's waves (design §6.9). A theme is two parts, chosen apart: a **glyph set**, its characters, and a **palette**, the rest (below). The game's own are theme files in [`themes/`](../../themes), [`cp437.ron`](../../themes/cp437.ron) the default. To draw it your own way, copy one, edit it, and put it in the `themes` folder of the game's folder (design §6.7). The help screen (`?`) shows where the game's folder is:

| System | The themes folder |
|---|---|
| Windows | `%APPDATA%\terra-sprites\themes` |
| macOS | `~/Library/Application Support/terra-sprites/themes` |
| Linux and others | `$XDG_DATA_HOME/terra-sprites/themes`, or `~/.local/share/terra-sprites/themes` |

Then press `Ctrl+T` in the game. The [settings screen](settings.md) offers each `.ron` file in the folder by name, after the built-in ones, on two of its rows: **Glyphs**, the glyph set, and **Colours**, the palette (below). Choose a row with `↑` and `↓`, change it with `←` and `→`, `Enter` or a click, and the map is drawn in it at once. The status line says what changed, such as "Colours: my-theme". To draw your theme whole, pick it on both rows. No rebuilding is needed, and you can edit the file and pick it again to see the change.

The game remembers what you picked, in [`settings.ron`](settings.md), and starts in it next time. To start in a theme for one run only, give its file when you start the game, and it's drawn whole:

```bash
cargo run --release -- --theme my-theme.ron --seed 7
```

`--ascii` picks only the glyph set, so it goes with `--theme`: `--ascii --theme my-theme.ron` draws your theme's palette in plain ASCII. Neither is saved: the next run starts in what you picked on the settings screen. A theme only changes how things look: it isn't part of the world, so it never changes what happens, and a save or replay plays out the same in any theme. The characters of frames and text are the same in every theme: a theme sets only their colours.

## Glyph sets and palettes

A theme is two parts, chosen apart (design §6.2): a **glyph set**, every character it draws with, and a **palette**, everything else. So the map can be drawn in ASCII with high contrast, or with your own glyphs in mono, without a theme file for each pair.

| Part | What it gives |
|---|---|
| Glyph set | Each `glyph:` character in `tiles`, `objects` and `cursor`; the Cursor's `arrows`, `followed_arrows`, `visible_frame` and `status_marks`; `space`'s glass and stars; and `population`'s `full`, `half` and `bands` characters and its `limit`'s. |
| Palette | Every glyph's `fg`, `bold` and `reversed`; `drives`; `attention_marker`; `population`'s `stages` and its `limit`'s colour; `lineage`; `text`; `panels`; `motion`; `colour`; `foreground`; and `colour_modes`. |

Every theme file is whole, and picked as both, on both of the settings screen's rows or with `--theme`, it draws as written. Taken as a glyph set, only its characters are used; taken as a palette, everything but them. An object type or state that one part leaves out falls back in that part, as under [`objects`](#objects) below. The game has these built in:

| Built in | File | What it is |
|---|---|---|
| CP437 glyph set | [`cp437.ron`](../../themes/cp437.ron) | The default glyphs: `☺ ♣ ♠ • ≈` and the rest of CP437 (design §6.2). It's the default palette's file too. |
| ASCII glyph set | [`ascii.ron`](../../themes/ascii.ron) | Plain ASCII, for a terminal or font that can't draw CP437, picked with `--ascii`. Frames and text stay CP437. Its colours are the default palette's, shades too, so the file read whole draws as `--ascii` does. |
| Default palette | [`cp437.ron`](../../themes/cp437.ron) | Named colours on the map and the text, which follow the terminal's own scheme, and exact ones only where no name fits. The panels sit on shades. |
| High contrast | [`high_contrast.ron`](../../themes/high_contrast.ron) | Bright exact colours, the same in every terminal, each at 7:1 or more against Windows Terminal's black: minor text light grey rather than dark, headings and keys bold, a brighter attention marker, and lines between the panels in place of shades. |
| Colour-blind | [`colour_blind.ron`](../../themes/colour_blind.ron) | Exact colours from Okabe and Ito's set for colour-blind readers, so the drives, the lineage, a gain against a loss and the graph's stages are told apart without red against green. |
| Mono | [`mono.ron`](../../themes/mono.ron) | No colour at all ([`colour: false`](#colour), below), only bold and reverse video, and lines between the panels. What colour tells in the others is read from the signs, arrows and capitals, which stay, and the population graph's bands from a character each ([`bands`](#population)). What it can't show is a sprite's colour (`b`): its strongest drive is read in the sprite list and the Body tab, and its lineage on the family tree, as `b` and the help say. `NO_COLOR` set to anything but nothing starts the game in it, as [no-color.org](https://no-color.org) asks, unless you've picked a palette on the settings screen or with `--theme`. |
| Game Kid | [`game_kid.ron`](../../themes/game_kid.ron) | Greens on the terminal's own black, after the four of an old handheld's screen, `#0f380f`, `#306230`, `#8bac0f` and `#9bbc0f`, with greens on the same ramp, lighter or darker, where text must be read on black or things kept apart. Its [`foreground`](#foreground) draws the text the game leaves in the terminal's own colour in green, and every grey the game draws itself, such as the title screen's, in its shades. Greens can't tell the drives or the lineage apart, so it says [`colour_modes: false`](#colour_modes): as in mono, every sprite keeps its own colour, and `b` and the help say where its drive and lineage are read. The population graph's stages are greens of clearly different lightness, and motion plays in greens. The panels sit on dark green shades. In a terminal that doesn't show exact colours, such as macOS's Terminal, each is drawn as the nearest of the 256 (below): the lighter greens stay greens, two pairs of them as one and `#c5dc58` as a yellow, but the darkest two of the four, `#0f380f` and `#306230`, and the panels' shade are nearest a grey, so there the ground, the deep water and the panels are grey on black. |

Each palette's file draws the CP437 glyphs, so it loads as a whole theme too: `--theme themes/high_contrast.ron` starts the game in high contrast, and `--ascii --theme themes/mono.ron` in mono and ASCII. Copied into your themes folder, they're on the settings screen's rows as well, after the built-in ones.

## The rules

- **Every character must be in CP437,** the character set of the old PC screen, so any CP437 font or tileset can draw the game (design §6.2). That includes `☺ ♣ ♠ • ○ ≈ ▲ ║ ░` and plain ASCII; it doesn't include, say, `λ`, `€` or emoji. A character outside it is refused, naming the character and what it was for.
- **Colours** are the 16 terminal colours, by name: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `dark_gray`, `light_red`, `light_green`, `light_yellow`, `light_blue`, `light_magenta`, `light_cyan`, `white`. How each looks is up to the terminal's own colours. Terminals have no brown, so the default palette draws dirt in `yellow`.
- **A colour can also be exact,** written `"#rrggbb"` in double quotes, such as `"#181a22"`. A named colour follows the player's terminal colours and an exact one doesn't, so the default palette uses exact colours only where no name fits: the panels' shades, since even `dark_gray` is far too bright for a background (`#767676` in Windows Terminal), and motion's colours. High contrast and colour-blind use exact colours throughout, to look the same in every terminal. On a terminal that doesn't say it shows exact colours, such as macOS's Terminal, the game draws each as the nearest of the 256 colours nearly every terminal shows (design §6.2). A colour that's neither a name nor `"#rrggbb"` is refused, naming it.
- **Give each kind of thing its own character,** and use colour for its state, as the built-in glyph sets do: then a player who can't tell colours apart can still tell a bush from a thornbush. The game doesn't enforce this.
- **A theme that's wrong is refused,** with a message saying what's wrong. On the settings screen, it's passed over, for the next one along that loads, and the status line says why, such as "Couldn't use theme my-theme: it has no glyph for terrain(rock)". Given with `--theme`, the game doesn't start, and says the same.

## Fields

A theme is one record with fourteen fields. The first five are needed; `colour`, `foreground`, `colour_modes`, `space`, `population`, `lineage`, `text`, `panels` and `motion` may be left out.

| Field | Meaning |
|---|---|
| `tiles` | A map from each of the fixed things below to its glyph. All of them must be there. |
| `drives` | A map from a drive's name, from [chemicals](chemicals.md), to the colour of a sprite whose strongest drive it is, once it's above half (design §6.3). A drive left out draws the sprite in its own colour. It may be empty, `{}`. |
| `objects` | A map from an object type's name, from [objects](objects.md), to a map from each of its visual states to its glyph. It may leave things out (below). |
| `attention_marker` | A colour: the background of the tile the selected sprite is paying attention to. |
| `cursor` | How the Cursor is drawn (below). |
| `space` | The glass round the map and the stars past it (below). Optional: whatever a theme leaves out of it, it draws as the default theme does. |
| `population` | How the population graph (`p`) is drawn (below). Optional, as `space` is. |
| `lineage` | The lineage colours (`b`) (below). Optional, as `space` is. |
| `text` | The screen's text, styled by its job (below). Optional, as `space` is. |
| `panels` | The shades the inspector and the event log sit on (below). Optional: a theme without them draws lines between the panels instead. |
| `motion` | Motion's colours, and how a drive's bar shows its drive's pace (below). Optional, as `space` is. |
| `colour` | `false` for a palette with no colour at all, as mono's (below). Optional: `true` if it's left out. |
| `foreground` | A colour: what the game leaves in the terminal's own text colour is drawn in it, and every grey on the screen in its shades, as Game Kid's are (below). Optional: a theme without it leaves them to the terminal. |
| `colour_modes` | `false` for a palette whose colours can't tell the drives or the lineage apart, as Game Kid's can't (below). Optional: `true` if it's left out. |

A **glyph** is `(glyph: '♣', fg: green)`, with two optional flags: `bold: true`, and `reversed: true`, which swaps the colour and the background. Characters go in single quotes; a quote itself is `'\''`.

### `tiles`

| Key | What it is |
|---|---|
| `terrain(grass)`, `terrain(dirt)`, `terrain(sand)`, `terrain(shallow_water)`, `terrain(deep_water)`, `terrain(rock)` | The ground ([terrain](terrain.md)). |
| `sprite` | A sprite. Its colour shows when no drive colours it, and always with `b`'s plain colours. |
| `selected_sprite` | The selected sprite. |
| `young_sprite`, `selected_young_sprite` | A young sprite, and one selected (M2 design §9.6). Optional: a theme without them draws the young as grown. |
| `egg`, `selected_egg` | Retired: an egg is an item until it hatches, drawn from `objects` (M2 design §10.7). A theme that still has them loads, and draws the egg with its `egg` glyph if its `objects` have no `"egg"`. |
| `decision_marker` | Where the selected sprite is heading. It flashes. |
| `emote(hurt)`, `emote(pleased)`, `emote(shocked)`, `emote(failed)`, `emote(resting)` | What takes turns with a sprite's glyph when it's hurt, petted, zapped, gives up, or rests (design §6.3). |
| `emote(dancing)`, `emote(calling)` | What takes turns with a sprite's glyph while it dances for another, or calls and listens (M2 design §15.8). Optional: a theme without them draws both as `emote(pleased)`. |

### `objects`

Each object type's visual states come from its `visual` rules in `objects.ron`, plus `"default"`, the state of an object none of its rules match. In the built-in pack, `berry_bush` has `"seedling"`, `"fruiting"` and `"default"` (bare); the others have only `"default"`.

- A state left out draws the object's `"default"` look, and an object type left out, or with no `"default"`, draws as a white `?`. So a new object type from a data pack still shows in every theme.
- A name the world's data pack doesn't have, of an object type, a visual state or a drive, is refused, naming it. That catches a misspelling, which would otherwise draw as `?` without saying why.

### `cursor`

| Field | Meaning |
|---|---|
| `arrows` | The Cursor's four arrows round its 3×3 grid, as `(up: '↑', down: '↓', left: '←', right: '→')`, named by the way each points. They take the cursor mode's colour. |
| `followed_arrows` | The arrows while the Cursor follows a sprite. |
| `visible_frame` | The four sides while sprites can see the Cursor, in place of the arrows, named as the arrows they replace. |
| `status_marks` | The two marks beside the Cursor's middle: `idle` (nothing to report), `sent` (a click just sent a command), `applied` and `rejected` (the world just did it, or refused it), and Grab mode's `grab`, `empty` and `release`. |
| `mode_marks` | A glyph for each cursor mode, `select`, `train` and `grab`. Its colour is the mode's colour, which the arrows and marks take too. All three must be there. |
| `leash` | A dot of the line from the Cursor to a sprite it leads. |
| `aim` | The aim line while throwing or shoving: `(path: <glyph>, end: <glyph>)`, a dot of its path and where it would stop. |

### `space`

The map view can scroll past the map's wall, into space (M2 design §11). `space` is a record of two maps, `glass` and `stars`, each from the keys below to a glyph. Either map, and any key in it, may be left out, and is then drawn as the default theme draws it, so a theme made before there was space still works. A key the game doesn't know is refused, naming it.

| `glass` key | What it is |
|---|---|
| `horizontal`, `vertical` | The glass along the top and bottom of the map, and down its sides, one tile outside it. |
| `top_left`, `top_right`, `bottom_left`, `bottom_right` | The glass's four corners. |

| `stars` key | What it is |
|---|---|
| `far`, `near`, `nearest` | The stars at each of three depths, drifting right to left past the glass, the nearer the quicker. A nearer star is drawn over a farther one. |

### `population`

The population graph (`p`, and small at the top of the World tab) stacks the eggs and the sprites by life stage, youngest to oldest from the bottom, eggs, young, adults and old, each in its colour, or without colour in its own character: full screen as shares of each column, and small as how many, with a dotted line at the most sprites the world holds (M2 design §14). `population` is a record of five fields, each of which may be left out and is then drawn as the default theme draws it. A field the game doesn't know is refused, naming it.

| Field | Meaning |
|---|---|
| `stages` | A colour for each life stage and for eggs: `(young: light_green, adult: cyan, old: gray, egg: light_yellow)`, any of them left out as you like. |
| `full` | A character wholly of one stage: `'█'` in the default theme, `'#'` in the ASCII one. |
| `half` | A character's lower half, `'▄'` or `'_'`: it's drawn in the stage below, on the stage above as its background, or on nothing at the top of the stack. |
| `bands` | A character for each life stage and for eggs, which a palette without [colour](#colour) draws it in, so the stages are told apart without colour: `(young: '▓', adult: '█', old: '▒', egg: '░')` in the default theme, `(young: '+', adult: '#', old: '=', egg: ':')` in the ASCII one, any of them left out as you like. A character of two stages is then the lower's, and the legend shows each stage's own. |
| `limit` | A glyph, a dot of the small graph's line at the most sprites the world holds, drawn where nothing else is. |

### `lineage`

`b`'s lineage colours draw each sprite on the map by where it stands in the selected one's lineage, as the family tree (`k`) shows it; the selected sprite keeps its own colour, and with none selected, every sprite does (M2 design §17). `lineage` is a record of four colours, each of which may be left out and is then drawn as the default theme draws it. A field the game doesn't know is refused, naming it.

| Field | Meaning |
|---|---|
| `elders` | Its parents and grandparents: `light_blue` in the default theme. |
| `siblings` | Its brothers and sisters, by either parent: `light_red`. |
| `young` | Its young, and theirs: `light_magenta`. |
| `others` | Every other sprite: `dark_gray`. |

### `text`

The screen's text is coloured by what it is, so a tab reads at a glance (design §6.2). `text` is a record of seven roles, each a style: `(fg: light_cyan, bold: true)`, with `fg`, `bold` and `reversed` each optional. A role left out is styled as the default theme styles it, and a role the game doesn't know is refused, naming it. Colour never carries a meaning alone: the signs, arrows and capitals stay, so the screen reads the same in any theme.

| Role | What it styles | Default theme |
|---|---|---|
| `heading` | Headings: `ATTENTION`, `WATER`, `Observed`, `HORMONES`, help's headings, the sprite list's heading, the World tab's graph's heading | `light_cyan`, bold |
| `muted` | Minor text: a log line's tick, `and 3 more`, a `·` between parts, a bar's empty part, what a hint's key does, the dead on the family tree, the population graph's "as shares of each column" | `dark_gray` |
| `good` | A gain, `+.99` on the Brain tab, and a reward felt, on the Chem tab | `light_green` |
| `bad` | A loss, `-.19`, and a punishment felt | `light_red` |
| `key` | The key in a hint, `Z` of `Z select`, the top bar's `?`, the keys in a question such as `(y/n)` and in an overlay's corner, and help's keys | bold |
| `selected` | What's chosen: the open tab, a menu's highlighted item, the sprite list's highlighted row | reversed |
| `spot` | What a tooltip can explain when the pointer is on it: a drive's name on the Body tab, a chemical's on the Chem tab, a gene's name on the Genome tab and the event log's filters; and a sprite's name in a line of the log while it's in the world, which lights the sprite up on the map instead; but a gene with no effect, which keeps `muted`. The one the pointer is on is drawn in it in reverse video, with its tooltip, if it has one, beside the pointer | `"#87afd7"`, a soft blue |

A zero, `+.00`, is neither good nor bad, and a chemical's change on the Chem tab stays plain, since a rise is good for food and bad for hunger. A drive's bar on the Body tab is drawn in the colour `drives` gives the drive, a block filled in part too.

### `panels`

`panels` is a record of two colours: `shade`, what the inspector and the event log sit on, and `title`, what their title rows sit on (the inspector's tabs, the log's "Events" and its filters). Both are needed. The default theme's are `"#181a22"` and `"#282c3a"`, a shade and two lighter than black. While motion is on, a tab opened sweeps in out of `shade` (design §6.9). The shades are the palette's, so ASCII glyphs in the default palette sit on them. A theme without `panels`, such as high contrast and mono, draws a line instead: `│` between the map and the inspector, and a `─` rule over the log with its title and filters in it (design §6.1). So does one with [`colour: false`](#colour), even if it has `panels`, since its shades couldn't be seen.

### `motion`

Motion is the small effects drawn over the screen as things happen, such as a death's flash and a hatching's sparks (design §6.9); `--no-motion` turns it off. `motion` is a record of six fields, five colours and how a drive's bar shows its drive's pace, each of which may be left out and is then drawn as the default theme draws it. A field the game doesn't know is refused, naming it. A fade blends exact colours, so a named colour here blends from how Windows Terminal's default scheme, Campbell, draws it, whatever the player's terminal shows.

| Field | Meaning | Default theme |
|---|---|---|
| `flash` | What a dying sprite's tile flashes before it fades back. | `"#d0d0d0"` |
| `ash` | The grey a dead sprite's ghost fades to. | `"#5c5c5c"` |
| `spark` | The sparks round an egg that hatches, and what the tile of one about to hatch flashes towards. | `"#ffe9a0"` |
| `trail` | The tint on the tiles the selected sprite has just left. | `"#2b3047"` |
| `dark` | What a pop-up fades in from, and out to as it closes, and what was under it fades back from; what a tab opened sweeps in from in a theme without `panels`; and the background a fade takes on a tile that has none of its own. | `"#0c0c0c"` |
| `bar` | How a drive's bar on the Body tab shows how fast its drive moves against how fast it usually rises (below): `crashing` or `ripple`, not in quotes. | `crashing` |

`bar` chooses the waves on a drive's bar, which play while motion is on, below 4× and in a palette with colour, and are only colour: no block's character changes. A drive's usual rise is the average of its last hundred rises a tick since you selected the sprite, and its first rise is as usual for it. Either way, a falling drive shows a faint dark dip running slowly back down its bar, and a steady one nothing.

- **`crashing`**, crashing waves: while the drive rises, a crest runs up the filled part to the bar's end, breaks there, the end block flaring, and its spray runs on into one to three empty blocks past the level and dies away, changing their colour but not their `░`, so the level never looks higher than it is. The faster the rise against its usual rise, the taller the crest, the more often it comes, the brighter the flare and the further the spray.
- **`ripple`**, a running ripple: while the drive rises, a crest runs up the filled part and off the bar's end, taller and more often the faster, with no flare or spray. No built-in palette chooses it, and the settings screen doesn't offer it, so it's chosen only by a theme file of your own.

### `colour`

`colour: false` draws the whole screen in the terminal's own colours, with nothing but bold and reverse video, as the mono palette does: every colour the theme names, and every one the game draws itself, such as the title screen's greys, gives way to the terminal's. The theme still names its colours, since the fields above need them, but they're never seen: mono's are all `white`. The tile the selected sprite attends to, which `attention_marker` would give a background, is underlined instead, which no glyph is, so it never reads as a fruiting bush's bold; and motion's colours don't play, a drive's bar's waves among them, though the view still pans and a drive's bar still eases (design §6.9).

### `foreground`

Most of the screen's text, and its frames, are drawn in the terminal's own text colour, whatever the palette. `foreground` gives that a colour of its own: `foreground: "#9bbc0f"` draws them in Game Kid's lightest classic green. Every grey on the screen is then drawn in a shade of it, as the drawing of each frame ends: the terminal's own text colour, as Windows Terminal's Campbell scheme draws it, `#cccccc`, becomes the foreground itself; Campbell's black, `#0c0c0c`, stays as it is, and anything darker becomes it; and a grey between or past them becomes the colour as far along from that black to the foreground. So `dark_gray`, `#767676` in Campbell, is drawn a darker shade of the foreground, and `white` a lighter one, and the game's own greys, such as the title screen's, take its hue. That includes the greys a theme names itself, `gray` or `"#5c5c5c"` say, so a palette with a foreground names none it wants to stay grey. A colour that isn't grey stays as it is, and so does the terminal's own background. A fade blends the terminal's own text colour from the foreground, so it passes through no grey either (design §6.9). It does nothing in a palette with [`colour: false`](#colour), which has none.

### `colour_modes`

`b` colours each sprite on the map by its strongest drive, or by where it stands in the selected sprite's lineage (design §6.3). `colour_modes: false` says the palette's colours can't tell them apart, as Game Kid's greens can't tell nine drives apart: then every sprite is drawn in its own colour, whatever `b` says, and `b` says where they're read instead, "Colours: lineage, which this palette can't show: see the family tree (k)", and the help's colour legend says the same, as in mono. Its `lineage` may still be given, and isn't drawn. Its `drives` may be too, and aren't drawn on the map, but still colour a drive's bar on the Body tab and the drive in the sprite list's Drive column. A palette with [`colour: false`](#colour) can't show them either, whatever this says.

## Working example

A bright theme, for a terminal whose dark colours are hard to see: light colours everywhere, and the CP437 glyphs.

<!-- example: theme -->
```ron
(
    tiles: {
        terrain(grass):         (glyph: '.', fg: light_green),
        terrain(dirt):          (glyph: ',', fg: yellow),
        terrain(sand):          (glyph: ':', fg: light_yellow),
        terrain(shallow_water): (glyph: '~', fg: light_cyan),
        terrain(deep_water):    (glyph: '≈', fg: light_blue),
        terrain(rock):          (glyph: '#', fg: white),
        sprite:                 (glyph: '☺', fg: white, bold: true),
        selected_sprite:        (glyph: '☻', fg: white, bold: true, reversed: true),
        decision_marker:        (glyph: 'X', fg: white, bold: true),
        emote(hurt):            (glyph: '!', fg: light_red, bold: true),
        emote(pleased):         (glyph: '♥', fg: light_magenta),
        emote(shocked):         (glyph: '‼', fg: light_yellow),
        emote(failed):          (glyph: '?', fg: light_yellow),
        emote(resting):         (glyph: 'z', fg: light_blue),
    },
    drives: {
        "hunger":      light_yellow,
        "thirst":      light_cyan,
        "pain":        light_red,
        "tiredness":   light_blue,
        "loneliness":  light_magenta,
    },
    objects: {
        "berry_bush": {
            "seedling": (glyph: '\'', fg: light_green),
            "default":  (glyph: '♣', fg: light_green),
            "fruiting": (glyph: '♣', fg: light_red, bold: true),
        },
        "berry":     { "default": (glyph: '•', fg: light_red) },
        "thornbush": { "default": (glyph: '♠', fg: light_magenta) },
        "ball":      { "default": (glyph: '○', fg: white, bold: true) },
        "egg":       { "default": (glyph: 'o', fg: light_yellow, bold: true) },
    },
    attention_marker: gray,
    cursor: (
        arrows: (up: '↑', down: '↓', left: '←', right: '→'),
        followed_arrows: (up: '▲', down: '▼', left: '◄', right: '►'),
        visible_frame: (up: '═', down: '═', left: '║', right: '║'),
        status_marks: (
            idle: '·', sent: '+', applied: '☼', rejected: '?',
            grab: '↑', empty: '░', release: '↓',
        ),
        mode_marks: {
            select: (glyph: '♦', fg: white),
            train:  (glyph: '±', fg: light_magenta),
            grab:   (glyph: '∩', fg: light_yellow),
        },
        leash: (glyph: '·', fg: light_yellow),
        aim: (path: (glyph: '·', fg: light_yellow), end: (glyph: '°', fg: light_yellow)),
    ),
    space: (
        stars: {
            far: (glyph: '·', fg: gray),
        },
    ),
    population: (
        stages: (old: white),
        limit: (glyph: '·', fg: gray),
    ),
    lineage: (others: gray),
    text: (
        heading: (fg: light_yellow, bold: true),
        muted:   (fg: gray),
    ),
    panels: (shade: "#1c1c1c", title: "#303030"),
    motion: (flash: white, bar: ripple),
)
```

Save it as `bright.ron` in the themes folder and pick it as the Glyphs and the Colours on the settings screen (`Ctrl+T`). Boredom, crowdedness, arousal and anxiety are left out of `drives`, so a sprite bored, crowded, aroused or anxious above all else draws in white. `space` gives only the far stars, in gray rather than dark gray, so the glass and the nearer stars are drawn as the default theme draws them. `population` draws old sprites in white and the line of the world's limit in gray; the rest of the graph is drawn as in the default theme. `lineage` draws the sprites outside the lineage in gray, a little brighter than the default's dark gray. `text` makes headings light yellow and minor text gray, so they stand out more; gains, losses, keys and the open tab are styled as in the default theme. `panels` sets the shades in neutral greys. `motion` makes a death's flash white and shows a drive's pace on its bar in a running ripple; its other colours are the default theme's.
