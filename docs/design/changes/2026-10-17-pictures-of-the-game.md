# Pictures of the game

The renderer ([#241](https://github.com/Keazra/terra-sprites/issues/241)) draws the game's screens into GIFs, MP4s and animated SVGs through the game's own screen code. For clips with no player in them, the Cursor can be withdrawn, and the map view can be drawn alone. Built in the renderer's PR, 2026-10-17.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A picture with no player in it withdraws the Cursor:** out of reach, as over space, with no pointer anywhere. It isn't drawn, and the status line says nothing of what it's over, where a pointer over space has it say "space". The pointer coming back over the map brings it back. | #241 | §11.4, CONTEXT "Withdrawn Cursor", "Clip" |
| 2 | **The map view can be drawn alone,** its tiles and what's past the wall filling the picture, with no border, top bar, panels or status line, for clips upright for phones. | #241 | §11.4 |
