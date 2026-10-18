# Pictures that leave parts of the screen out

The renderer's clips ([#291](https://github.com/Keazra/terra-sprites/issues/291)) can leave out parts of the screen, such as the event log, which Fauxpaw found often looks a bit garish in a clip. Built in [#292](https://github.com/Keazra/terra-sprites/pull/292), 2026-10-18.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A picture can leave out any of the top bar, the inspector, the event log and the status line,** and the map view takes the room they leave, with the inspector beside it while that's drawn: the event log's rows, the inspector's columns, the top bar's and the status line's row. The game itself always draws them all. | #291 | §11.4 |
