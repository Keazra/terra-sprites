# The title's Cursor keeps its sprite

Fauxpaw, on 2026-10-17, once the title's world ran from launch ([#281](https://github.com/Keazra/terra-sprites/pull/281)): the sprite beneath the Cursor should stay still, always shown as the happy face. It slept, woke pleased, walked out of the Cursor's frame at 2.5 s, and the menu then covered the place it had been. Done in [#284](https://github.com/Keazra/terra-sprites/pull/284).

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The sprite in the Cursor's frame stays still, always drawn as a sprite is** (`☺`), for as long as the title screen is open. It no longer sleeps (`z`), wakes pleased (`♥`) or walks off, and the frame stays with it once the menu shows. | Fauxpaw, 2026-10-17 | §8.1 |
| 2 | **The light arrives above the middle of the screen,** on the tile nearest the middle of the rows above the title's boxes that a sprite could stand on and nothing is on, or at that middle when there's no such tile, so no box but the help screen covers the frame, however many choices, presets or saves it holds. It moves only when a resize would hide it. It arrived on the sprite nearest the middle, where the menu then covered it. | Fauxpaw, 2026-10-17; where, Claude's call | §8.1 |
| 3 | **That sprite is the scene's own,** drawn over the world, and the Cursor leads none of the world's sprites. A led sprite only follows the Cursor, so one of the world's led for the whole title screen would die of thirst in about ten minutes: on seeds 3, 7 and 11 of a 100×30 title world, the sprite nearest the middle, led from launch, died of thirst at ticks 3,048, 3,401 and 3,022. | Claude's call | §8.1 |
