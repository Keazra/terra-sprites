# Panels without boxes, and colours with jobs

The screen's panels lose their boxes, and their text gets colours by what it is. Fauxpaw settled it in the UI lessons round of 2026-10-19 (questions 1, 2 and part of 8), as Look 1 ([#302](https://github.com/Keazra/terra-sprites/issues/302)), the first of the Look slices ([#301](https://github.com/Keazra/terra-sprites/issues/301)), and it was built in [#383](https://github.com/Keazra/terra-sprites/pull/383).

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The map view has no frame,** and runs to the screen's edges: at 100×30 it shows 54×24 tiles, where it showed 52×21. The pointer on the inspector's left column or the event log's title row points at the last column or row, as it did on the map view's border. | #302 | §6.1, §6.5 |
| 2 | **In a theme with shades, there are no lines between the panels:** the inspector and the event log sit on a shade a little lighter than the terminal's black, their title rows on one lighter again. In a theme without, such as ASCII, a `│` down the inspector's left and a `─` rule over the log, meeting at `┴`. The inspector has 23 rows of 45 columns under its title row, and the event log its title row and three events, in 4 rows where it took 5. | #302 | §6.1 |
| 3 | **The open tab is in reverse video,** in the theme's style for what's chosen, as wide as brackets made it, in place of `[Body]`. A sprite's full label now fits beside the tabs more often: "Sprite #530" does. | #302 | §6.1 |
| 4 | **The event filters sit at the right of the log's title row,** where they sat on its top border. | #302 | §6.1 |
| 5 | **Pop-ups keep their boxes:** menus, the overlays and the debug overlay, which now sits in the map view's top-right corner rather than inside its border. | #302 | §6.1 |
| 6 | **A theme's colour can be exact,** `"#rrggbb"`, as well as one of the 16 names. The default theme keeps names for the map, so it follows the player's terminal colours, and uses exact colours only for the panels' shades. A terminal that doesn't say in `COLORTERM` that it shows exact colours, such as macOS's Terminal, gets the nearest of 256 colours instead; Windows' terminals all show them. | #302 | §6.2 |
| 7 | **Themes style the screen's text by its job,** in a `text` section of six roles: heading, muted, good, bad, key and selected. Every built-in theme sets them; a role a theme leaves out is styled as the default theme styles it. Headings are light cyan and bold, minor text dark grey, gains and a reward felt light green, losses and a punishment felt light red, a hint's key bold and what's chosen in reverse video. A drive's bar is in its drive's colour. Colour never carries a meaning alone. | #302 | §6.1, §6.2 |
| 8 | **Themes give the panels' shades** in a `panels` section, `shade` and `title`; a theme without one draws lines. | #302 | §6.1, §6.2 |

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The map view has no border at all,** where it was single-lined all round: the glass and space show where the world ends. | #302 | §11.2, §11.6 |
