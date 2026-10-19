# The sprite list filters what's typed after `/`

In the sprite list, `/` starts a filter that narrows it as the player types, and the list gains a Generation column so a generation can be found. The owner said on 2026-10-19 that the filter needs nothing from them; the counts in the inspector's tab names that [#313](https://github.com/Keazra/terra-sprites/issues/313) also suggests wait for the Look slices, which change its tab row.

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **`/` filters the sprite list,** as kdash's lists do, since `l` closes it and a name can hold an `l`. Typed characters go to the filter, `Backspace` rubs one out, and the arrows, `Tab`, `Shift+Tab`, `Enter` and clicks work as before. `Esc` clears the filter and a second closes the list. | #313 | §6.1 |
| 2 | **A sprite matches when the words typed start words of one column its row shows,** in order and side by side, case aside, with its generation read as `generation 3`. A word ending in a digit is a whole number. So `old`, `Tobek`, `gen 3` or `drink` all work, and `gen 3` isn't generation 31. The highlight stays on its sprite while it matches, or else goes to the first that does. | #313 | §6.1 |
| 3 | **While it's filtered, the title shows the filter and how many match,** `filter: old_ ── 1 of 3`, the corner says `esc clear`, and the key hints end `tab sort  esc clear`. Unfiltered, they start `/ filter`, which goes first when the status line is short of room. | #313 | §6.1 |
| 4 | **The list has a Generation column** after the drive, and the design's picture of the list has the life stage, which Slice 22 added. | #313 | §6.1 |
| 5 | **The help screen lists `l then /`, "filter the list",** and the key table's `l` row says `/` filters it. | #313 | §6.1, §6.5 |

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The sprite list shows each sprite's generation** in a Generation column, which its filter finds with `gen 3` or `generation 3`. | #313 | §17.1 |
