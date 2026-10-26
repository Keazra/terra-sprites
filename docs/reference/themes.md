# Themes

A **theme** says how the map is drawn: the character and colour of each kind of ground, each plant and toy, sprites, their emotes, the Cursor, the glass round the map and the stars in space past it (design §6.2, M2 design §11), the colours and blocks of the population graph (M2 design §14), and the lineage colours (§17). It also colours the screen's text by its job, and shades the panels beside and under the map, or leaves them to lines (design §6.1, §6.2). The game has two built in, `themes/cp437.ron` (the default) and `themes/ascii.ron` (`--ascii`). To draw it your own way, copy one, edit it, and put it in the `themes` folder of the game's folder (design §6.7). The help screen (`?`) shows where the game's folder is:

| System | The themes folder |
|---|---|
| Windows | `%APPDATA%\terra-sprites\themes` |
| macOS | `~/Library/Application Support/terra-sprites/themes` |
| Linux and others | `$XDG_DATA_HOME/terra-sprites/themes`, or `~/.local/share/terra-sprites/themes` |

Then press `Ctrl+T` in the game. The **Themes** menu lists the two built-in themes, then each `.ron` file in the folder by name; pick one with its number, the arrows and `Enter`, or a click, and the map is drawn with it at once. The status line says "Theme: my-theme". No rebuilding is needed, and you can edit the file and pick it again to see the change.

The game doesn't remember the theme you picked once it's closed. To start in one, give its file when you start the game:

```bash
cargo run --release -- --theme my-theme.ron --seed 7
```

`--theme` and `--ascii` can't be used together, since both pick a theme. A theme only changes how things look: it isn't part of the world, so it never changes what happens, and a save or replay plays out the same in any theme. The characters of frames and text are the same in every theme: a theme sets only their colours.

## The rules

- **Every character must be in CP437,** the character set of the old PC screen, so any CP437 font or tileset can draw the game (design §6.2). That includes `☺ ♣ ♠ • ○ ≈ ▲ ║ ░` and plain ASCII; it doesn't include, say, `λ`, `€` or emoji. A character outside it is refused, naming the character and what it was for.
- **Colours** are the 16 terminal colours, by name: `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `dark_gray`, `light_red`, `light_green`, `light_yellow`, `light_blue`, `light_magenta`, `light_cyan`, `white`. How each looks is up to the terminal's own palette. Terminals have no brown, so the built-in themes draw dirt in `yellow`.
- **A colour can also be exact,** written `"#rrggbb"` in double quotes, such as `"#181a22"`. A named colour follows the player's terminal colours and an exact one doesn't, so the default theme uses exact colours only where no name fits: the panels' shades, since even `dark_gray` is far too bright for a background (`#767676` in Windows Terminal). On a terminal that doesn't say it shows exact colours, such as macOS's Terminal, the game draws each as the nearest of the 256 colours nearly every terminal shows (design §6.2). A colour that's neither a name nor `"#rrggbb"` is refused, naming it.
- **Give each kind of thing its own character,** and use colour for its state, as the built-in themes do: then a player who can't tell colours apart can still tell a bush from a thornbush. The game doesn't enforce this.
- **A theme that's wrong is refused,** with a message saying what's wrong. Picked from the menu, the status line says so, such as "Couldn't use theme my-theme: it has no glyph for terrain(rock)", and the map keeps the theme it had. Given with `--theme`, the game doesn't start, and says the same.

## Fields

A theme is one record with ten fields. The first five are needed; `space`, `population`, `lineage`, `text` and `panels` may be left out.

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

The population graph (`p`, and small at the top of the World tab) stacks the eggs and the sprites by life stage, youngest to oldest from the bottom, eggs, young, adults and old, each in its colour: full screen as shares of each column, and small as how many, with a dotted line at the most sprites the world holds (M2 design §14). `population` is a record of four fields, each of which may be left out and is then drawn as the default theme draws it. A field the game doesn't know is refused, naming it.

| Field | Meaning |
|---|---|
| `stages` | A colour for each life stage and for eggs: `(young: light_green, adult: cyan, old: gray, egg: light_yellow)`, any of them left out as you like. |
| `full` | A character wholly of one stage: `'█'` in the default theme, `'#'` in the ASCII one. |
| `half` | A character's lower half, `'▄'` or `'_'`: it's drawn in the stage below, on the stage above as its background, or on nothing at the top of the stack. |
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

The screen's text is coloured by what it is, so a tab reads at a glance (design §6.2). `text` is a record of six roles, each a style: `(fg: light_cyan, bold: true)`, with `fg`, `bold` and `reversed` each optional. A role left out is styled as the default theme styles it, and a role the game doesn't know is refused, naming it. Colour never carries a meaning alone: the signs, arrows and capitals stay, so the screen reads the same in any theme.

| Role | What it styles | Default theme |
|---|---|---|
| `heading` | Headings: `ATTENTION`, `WATER`, `Observed`, `HORMONES`, help's headings, the sprite list's heading, the World tab's graph's heading | `light_cyan`, bold |
| `muted` | Minor text: a log line's tick, `and 3 more`, a `·` between parts, a bar's empty part, what a hint's key does, the dead on the family tree | `dark_gray` |
| `good` | A gain or a rise: `+.99` on the Brain tab, `+.0017` on the Chem tab | `light_green` |
| `bad` | A loss or a fall: `-.19`, `-.0020` | `light_red` |
| `key` | The key in a hint, `Z` of `Z select`, and help's keys | bold |
| `selected` | What's chosen: the open tab, a menu's highlighted item, the sprite list's highlighted row | reversed |

A drive's bar on the Body tab is drawn in the colour `drives` gives the drive.

### `panels`

`panels` is a record of two colours: `shade`, what the inspector and the event log sit on, and `title`, what their title rows sit on (the inspector's tabs, the log's "Events" and its filters). Both are needed. The default theme's are `"#181a22"` and `"#282c3a"`, a shade and two lighter than black. A theme without `panels`, such as the ASCII one, draws a line instead: `│` between the map and the inspector, and a `─` rule over the log with its title and filters in it (design §6.1).

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
)
```

Save it as `bright.ron` in the themes folder and pick it with `Ctrl+T`. Boredom, crowdedness, arousal and anxiety are left out of `drives`, so a sprite bored, crowded, aroused or anxious above all else draws in white. `space` gives only the far stars, in gray rather than dark gray, so the glass and the nearer stars are drawn as the default theme draws them. `population` draws old sprites in white and the line of the world's limit in gray; the rest of the graph is drawn as in the default theme. `lineage` draws the sprites outside the lineage in gray, a little brighter than the default's dark gray. `text` makes headings light yellow and minor text gray, so they stand out more; gains, losses, keys and the open tab are styled as in the default theme. `panels` sets the shades in neutral greys.
