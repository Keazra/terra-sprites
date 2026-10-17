# A selected sprite is in reverse video at any age

In the default theme a selected young sprite was drawn in reverse video, since CP437 has no filled `ö`, while a selected grown one only swapped `☺` for `☻`. The owner chose on 2026-10-17 to highlight every selected sprite, so a grown one is now `☻` in reverse video, the look the young, eggs and the ASCII theme already had ([#282](https://github.com/Keazra/terra-sprites/issues/282)).

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A selected sprite is `☻` in reverse video in CP437,** where it was `☻` alone, as it's `@` in reverse video in ASCII. It keeps `☻`, so a selected sprite still differs from the Cursor's centre over one that isn't. | #282 | §6.2 |

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A selected young sprite is in reverse video, as a grown one is,** where the design had said only which theme keys draw it. | #282 | §9.6 |
