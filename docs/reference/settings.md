# Settings

The **settings screen** is where you choose how the game looks and feels: its glyph set, its palette, motion and square tiles (design §6.7). What you change there is saved in `settings.ron` in the game's folder and read each time the game starts, so you choose once. The settings are yours, not the world's: a save or a replay never holds them, so a world looks the way you like on any computer, and plays out the same however it's drawn.

## The settings screen

`Ctrl+T` opens it, a box over the map, a row each:

| Row | What it changes | What it can be |
|---|---|---|
| Glyphs | The glyph set: the characters the map, the Cursor and the emotes are drawn with (design §6.2) | `CP437`, the default; `ASCII`; then each theme file in your themes folder, by name |
| Colours | The palette: the colours, bold and reverse video, and the panels' shades | `default`; `high contrast`; `colour-blind`; `mono`; then each theme file in your themes folder, by name |
| Motion | The screen's small movements ([manual](../manual.md#motion), design §6.9) | `on`, the default, or `off` |
| Square tiles | Each tile drawn two cells wide, so the world looks its true shape; `o` is the same switch (M2 design §11.7) | `off`, the default, or `on` |

`↑` and `↓`, or a row's number, choose a row. `←` and `→`, `Enter` or a click change it, at once, and the status line says what changed, such as "Colours: mono". A theme file that doesn't load is refused: the status line says why, such as "Couldn't use theme night: it has no glyph for terrain(rock)", and the row keeps what it had. Time runs on behind the box, `Esc` closes it and `?` opens the help. [Themes](themes.md) says how to make a theme file of your own and what the built-in ones are.

## settings.ron

The game writes `settings.ron` in its folder each time you change a row or press `o`. The help screen (`?`) shows the game's folder; it's

| System | The game's folder |
|---|---|
| Windows | `%APPDATA%\terra-sprites` |
| macOS | `~/Library/Application Support/terra-sprites` |
| Linux and others | `$XDG_DATA_HOME/terra-sprites`, or `~/.local/share/terra-sprites` |

It holds only the rows you've changed. A row it leaves out takes the game's own default, so a later version's new default still reaches a row you never touched. Having picked ASCII glyphs, a theme file of your own for the colours and motion off, it reads:

<!-- example: settings -->
```ron
// Terra Sprites' settings: what you've changed on the settings screen (Ctrl+T).
(
    glyphs: "ASCII",
    colours: "night.ron",
    motion: false,
)
```

| Field | What it holds |
|---|---|
| `glyphs` | The glyph set: a built-in one by the name the screen shows it by, `"CP437"` or `"ASCII"`, or a theme file in your themes folder by its file's name, with its `.ron`, such as `"night.ron"`. |
| `colours` | The palette, the same way: `"default"`, `"high contrast"`, `"colour-blind"` or `"mono"`, or a theme file's name. |
| `motion` | `true` or `false`. |
| `square_tiles` | `true` or `false`. |

- **It's never half written.** The game writes the new file beside the old one, then puts it in its place, as it does a save, so one cut short leaves the old one as it was. If it can't be written at all, the status line says "Couldn't save the settings: …", and the change still holds until the game is closed.
- **A field the game doesn't know is passed over,** unlike in the data pack's files, so a settings file from a later version, with rows this one hasn't got, still works. A built-in name is matched in any case.
- **You can edit it** while the game is closed. While it's open, the game writes the file afresh from what it read at start and what you've changed since, so an edit made then is lost at the next change.
- **Without a game's folder,** when the game can't tell where your home folder is, nothing is saved, and the settings last until the game is closed.

## At start

Each row starts as the first of these that gives it:

1. **A flag,** for that run only: `--ascii` gives the glyphs, `--theme <file>` the glyphs and the colours (with `--ascii`, the colours alone), and `--no-motion` turns motion off. A flag is never saved: changing a row writes that row alone, so what a flag picked is saved only if you pick it on the screen. A theme given with `--theme` that doesn't load stops the game, saying why.
2. **`settings.ron`.**
3. **`NO_COLOR`,** for the colours: set in the environment to anything but nothing, it starts the game in mono, as [no-color.org](https://no-color.org) asks.
4. **The game's own default:** CP437 glyphs, the default colours, motion on and square tiles off.

When `settings.ron` is wrong, the game still starts:

- **A file it can't read,** or one that isn't settings, is left as it is, and every row starts on its default. The title screen says "Couldn't read the settings: …", with what's wrong, or the status line does if a flag opened a world straight away. The next row you change writes a new file in its place, holding that row.
- **A theme file it names that no longer loads,** or a name the game doesn't know, starts that row on its default, saying "Couldn't use theme night: …" or "Couldn't read the settings: there are no glyphs called …". The file keeps the name, so once the theme loads again, the row starts on it again, unless you've changed the row since.

## Carried over

The settings stay as you left them over a load, back to the title screen, which is drawn in them too, and into the next world.
