# The Cursor no longer leads an egg

The owner turned down [PR #192](https://github.com/Keazra/terra-sprites/pull/192) after trying it on 2026-10-13: "Eggs are acting like they're Sprites - they're being led around as opposed to being able to be picked up and laid down. Eggs should be objects, until they're hatched, and become an actual Sprite." This change undoes [`2026-10-12-leading-eggs.md`](2026-10-12-leading-eggs.md), so the design is as it was before #192. Making eggs objects is [#196](https://github.com/Keazra/terra-sprites/issues/196).

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The Cursor can't take hold of an egg again,** as before #192: an egg can't be touched, led or shoved. Both rows of `2026-10-12-leading-eggs.md` are undone, and the Cursor's mark no longer shows an egg. A replay recorded with #192 parts from its recording if the player led an egg. | The owner, 2026-10-13; #196 | §10.4, §10.7 |
