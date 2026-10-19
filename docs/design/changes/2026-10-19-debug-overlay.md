# A debug overlay

`F3` shows how the game is running on the player's computer, so a slow or sticky feeling can be shown in a screenshot instead of described. An idea from the UI lessons round of 2026-10-19 ([#311](https://github.com/Keazra/terra-sprites/issues/311)), which Fauxpaw said needs nothing from them; its open questions took the defaults written there: `F3`, in every build, beside the speed example rather than in its place.

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **`F3` shows or hides the debug overlay,** in every build: a small box inside the map view's border, at its top right, over whatever's open. It says the ticks a second run over the last second against what the speed asks for (`paused`, or `at Max`), the simulation's time in a frame against the 25 ms budget, the drawing's time in a frame with the frames a second, and whether the terminal reports key releases. It answers no prompt and never touches the world. | #311 | §6.1, §6.5 |
| 2 | **The frame loop times each frame** and the screen reads the times once a second's worth is in. The overlay measures the game as it's played, drawing included; the speed example (A5) stays the measure of the simulation alone. | #311 | §6.1 |
| 3 | **The help screen lists `F3`** under a heading of its own, `DEBUG`, in its third column. | #311 | §6.1 |
