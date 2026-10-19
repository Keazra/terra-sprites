# The sprite list filters as you type

Typing in the sprite list narrows it, from the ideas the UI lessons round turned up ([#313](https://github.com/Keazra/terra-sprites/issues/313)). The owner said on 2026-10-19 that the filter needs nothing from them, so it was built with the defaults the autonomous-work briefs picked. The tab counts #313 also suggests wait for the Look slices, which change the inspector's tab row.

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **`/` filters the sprite list,** as kdash's lists do, since `l` closes it and a name can hold an `l`. Typed characters go to the filter, `Backspace` rubs one out, and the arrows, `Tab`, `Enter` and clicks work as before. `Esc` clears the filter and a second closes the list. | #313 | §6.1 |
| 2 | **A sprite matches when the filter is in any column its row shows,** case aside, with its generation read as `gen 3`, so `old`, `Tobek`, `gen 3` or `drink` all work. The highlight stays on its sprite while it matches, or else goes to the first that does. | #313 | §6.1 |
| 3 | **While it's filtered, the title shows the filter and how many match,** `filter: old_ ── 1 of 3`, the corner says `esc clear`, and the key hints end `tab sort  esc clear`. Unfiltered, they start `/ filter`. | #313 | §6.1 |
| 4 | **The list has a Gen column** after the drive, so a generation can be found, and the life stage in the design's picture of it, which Slice 22 added. | #313 | §6.1 |
| 5 | **The help screen lists `l then /`, "filter the list",** and the key table's `l` row says `/` filters it. | #313 | §6.1, §6.5 |

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The sprite list shows each sprite's generation** in a Gen column, which its filter reads as `gen 3`. | #313 | §17.5 |
