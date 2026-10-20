# A debug overlay

`F3` shows how the game is running on the player's computer, so a slow or sticky feeling can be shown in a screenshot instead of described. An idea from the UI lessons round of 2026-10-19 ([#311](https://github.com/Keazra/terra-sprites/issues/311)), built in [#323](https://github.com/Keazra/terra-sprites/pull/323). Fauxpaw said it needs nothing from them, so its open questions took the defaults an agent picked: `F3`, which is free and the common key for it; in every build, since the point is a screenshot from a player's computer; and beside the speed example rather than in its place.

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **`F3` shows or hides the debug overlay,** in every build: a small box inside the map view's border, at its top right, over any menu or overlay in a world. It says the ticks a second run over the last second against what the speed asks for (`paused`, or `at Max`), the simulation's time in a frame against the 25 ms budget, the drawing's time in a frame with the frames a second, and whether the terminal reports key releases. `F3` works whatever's open, a name being typed included. | #311 | §6.1, §6.5 |
| 2 | **`F3` answers no prompt and never touches the world.** "Quit?" and the question before loading still wait, and the help screen and the sprite list, where other keys do nothing, take it too. | #311 | §6.1, §6.6, §6.7 |
| 3 | **The frame loop times each frame,** its ticks and their time as the budget counts it, and the drawing's, and the screen reads the times once a second's worth is in. The overlay measures the game as it's played, drawing included; the speed example (A5) stays the measure of the simulation alone. | #311 | §6.1, §6.6 |
| 4 | **The help screen lists `F3`** under a heading of its own, `DEBUG`, in its third column. | #311 | §6.1 |

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **`F3` doesn't answer "Quit?"**: it shows or hides the debug overlay and the question waits. | #311 | §8.4 |
