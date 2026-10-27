# Glyph sets and palettes, chosen apart, and a settings screen

A theme's characters and its colours are chosen apart, four palettes join the default (the fourth, Game Kid, from Fauxpaw's notes on playing Look 6), `NO_COLOR` starts the game in mono, and a settings screen, saved between runs, gathers how the game looks and feels. Fauxpaw settled it in the UI lessons round of 2026-10-19 (question 8), as Look 6 ([#307](https://github.com/Keazra/terra-sprites/issues/307)) of the Look slices ([#301](https://github.com/Keazra/terra-sprites/issues/301)), and it was built in [#394](https://github.com/Keazra/terra-sprites/pull/394).

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A theme is a glyph set and a palette, chosen apart.** The glyph set gives the characters, the palette the colours, bold and reverse video, the text roles, the panel shades and motion's colours. Two glyph sets are built in, CP437 and ASCII; a theme file of the player's own can be either, or both. | #307 | §6.2 |
| 2 | **Three new palettes:** high contrast, colour-blind (Okabe and Ito's colours, never red against green) and mono (bold and reverse video only, with the attended tile underlined, since bold is a fruiting bush's), beside the default. Shades go with the palette, so ASCII glyphs in the default palette sit on shades, and high contrast and mono draw lines. What mono can't show, a sprite's drive and lineage colours, `b` and the help say where to read instead, and motion's colours don't play in it, though the pan and the bars still move. | #307 | §6.1, §6.2, §6.3, §6.9 |
| 3 | **`NO_COLOR` starts the game in mono,** unless the player has picked a palette. | #307 | §6.2 |
| 4 | **`Ctrl+T` opens the settings screen,** where the Themes menu was: glyphs, colours, motion and square tiles, a row each. A theme file that doesn't load is passed over for the next that does, saying why. Look 8 adds the bell and focus. | #307 | §6.1, §6.5, §6.7, §6.9 |
| 5 | **The settings are saved in `settings.ron`** in the game's folder, never in a world save, only what the player changed, read at start. Flags last only for their run and win over it. A broken file, or a theme it names that won't load, falls back to the defaults and says so; a broken file is kept aside as `settings.ron.broken` before the first save, rather than written over. | #307 | §6.7 |
| 6 | **The look carries back to the title screen and into the next world,** as well as over a load. | #307 | §6.7 |
| 7 | **`--ascii` picks only the glyphs,** so it can go with `--theme`. | #307 | §6.2, §6.7 |
| 8 | **A fifth palette, Game Kid** (Fauxpaw, 2026-10-27, from playing Look 6): greens on the terminal's black, after the original Game Boy's four, with greens on their ramp where text must be read or things kept apart. A palette may give a `foreground`, which draws the terminal's own text colour, and every grey the game draws itself, in its shades, and may say it can't show the colour modes, as Game Kid's greens can't: then, as in mono, a sprite keeps its own colour and `b` and the help say where its drive and lineage are read. Motion plays in it, in greens. It's last on the Colours row. | #307 | §6.2, §6.3, §6.7, §6.9 |

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Square tiles are remembered between runs,** a row of the settings screen. | #307 | §11.7 |
| 2 | **Without colour, the population graph draws each band in a character of its own,** from the glyph set (`░ ▓ █ ▒` in CP437), a character of two bands showing the lower, and its legend the same. | #307 | §14.2 |

## Glossary (`CONTEXT.md`)

**Glyph set**, **Palette** and **Settings screen** are new; **Palette** lists Game Kid among the built-in ones. **Theme** says it's a glyph set and a palette. **Colour mode** and **World config** say what their avoided words mean instead, and **Panel shade** that a palette without shades draws lines.
