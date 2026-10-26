# Small screens step down, and square tiles

A screen smaller than 100×30 steps down a part at a time rather than saying "Terminal too small", the inspector can fill the screen, and tiles can be drawn two cells wide, so they're square. Fauxpaw settled it in the UI lessons round of 2026-10-19 (questions 5 and 6), as Look 4 ([#305](https://github.com/Keazra/terra-sprites/issues/305)) of the Look slices ([#301](https://github.com/Keazra/terra-sprites/issues/301)).

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Under 100 columns, the inspector pops over the map view's right side,** shut until `i` opens it, and the map view takes the whole width. `Esc` closes it once there's no aim to cancel. While it's open, the pointer over it is on it: a click or the wheel there reaches the inspector, not the map beneath, which the row below it can't point at either. While it's shut, the hints add `i inspector` after the mode keys, where there's room. Opening it leaves the view where it was; while it's open, Track, `Tab` and the sprite list centre a sprite beside it, the view scrolls a quarter of the part beside it past the wall, and a Cursor left under it isn't drawn, nor reached by `Q`, `E` or `F`. | #305 | §6.1, §6.5 |
| 2 | **With fewer than 8 rows for the inspector, or fewer than its 46 columns, the map view is drawn alone** with the top bar and the status line, and `i` is refused: "No room for the inspector". Grown again, the inspector over the map is shut. Under 30 rows the event log goes, as it did. | #305 | §6.1 |
| 3 | **The smallest screen is 20×5,** where it was 100×30. Under it, "Terminal too small" says what it needs on two lines where one is too wide. The title screen needs 60×20 of its own. | #305 | §6.1 |
| 4 | **The top bar, short of room for `? help`,** drops the seed, then the game's name, when it was saved and how many sprites there are. **The help** puts its columns of keys one under another where they don't fit side by side. | #305 | §6.1 |
| 5 | **`i` fills the screen with the inspector,** between the top bar and the status line, its tab's lines in as many columns of 45 as fit, read down each and on to the next: two at 100×30, three at 140×40. `PgUp` and `PgDn` turn a page of all its columns. The map view and the event log beneath it take no clicks. The inspector's keys and the pointer over it work, with `Tab`, `v`, `?` and time's keys; the map's do nothing. `i` again, or `Esc`, takes it back. Where the inspector pops over the map and is shut, `i` opens it there first: one key makes it a size bigger each press, where the issue named `i` for the pop-over and left the full screen's key open. | #305 | §6.1, §6.5 |
| 6 | **The help lists both keys:** `o square tiles` under VIEW and `i open, or full screen` under INSPECTOR. | #305 | §6.1, §6.5 |

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **`o` draws each tile two cells wide,** so tiles are square, and again goes back. It's off at the start of every run and isn't part of the world; remembering it between runs waits for Look 6's settings file ([#307](https://github.com/Keazra/terra-sprites/issues/307)). At 140×40 the map view shows 47 by 34 tiles, and at 100×30, 27 by 24. | #305 | §11.7 |
| 2 | **A tile's second cell repeats its terrain, and is blank beside a sprite, an object or a mark,** its use otherwise left for M3 to settle with how holding and gripping show on the map. | #305, and its comment from M3's slices | §11.7 |
| 3 | **Everything on the map follows it,** in tiles: the pointer, clicks, scrolling, the margin past the wall, Track and `Tab`'s centring, the Cursor's frame, aim lines, the leash, the decision marker and the stars. The glass down the sides keeps to the cell beside the map. | #305 | §11.6, §11.7 |
| 4 | **The title screen says "Terminal too small" under 60×20,** where it did under 100×30. | #305 | §8.2 |
