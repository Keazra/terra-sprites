# Eggs are items until they hatch

The owner, on [PR #192](https://github.com/Keazra/terra-sprites/pull/192) on 2026-10-13: "Eggs should be objects, until they're hatched, and become an actual Sprite." Their answer to [#190](https://github.com/Keazra/terra-sprites/issues/190) stands: the Cursor can pick an egg up, but not throw it. Built in [#196](https://github.com/Keazra/terra-sprites/issues/196) by [PR #200](https://github.com/Keazra/terra-sprites/pull/200), with its four defaults: an egg held when it's due waits, sprites don't push it, sprites can't eat or hurt it, and an old save's eggs load as items. This replaces the egg as a sprite at a first life stage, from [`2026-10-12-breeding.md`](2026-10-12-breeding.md).

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **An egg is an item, the object type `egg`, until it hatches,** small like a berry, holding the sprite it hatches into: its genes, parents, name and when it was laid. A sprite's life stages are young, adult and old; `Egg` in a save or genome file reads as young. | The owner, 2026-10-13; #196 | §9.1, §10.4 |
| 2 | **It hatches as its type's stage ends, `next: Hatch`,** 750 ticks after it was laid, where it lies, into a young male with a newborn's body and the egg's ID. Only one type may hatch, and only a sprite lays one. Physiology's `breeding.hatches` is gone; a pack that still has it lays egg items that hatch after it. | #196 | §10.4 |
| 3 | **An egg due to hatch waits** while the Cursor holds it, hatching on the tick it's put down, and while a sprite stands on it. | #196, default 1 | §10.4 |
| 4 | **Sprites can't move, eat or hurt an egg:** it has no verbs. A rolling item knocks it on, and never crushes it. | #196, defaults 2 and 3 | §10.4 |
| 5 | **The Cursor picks an egg up and puts it down, and never throws one,** as the type's new `throwable: false` says: refused at once on screen, "Can't throw an egg", and by the world. Pets and zaps don't reach it. It can still be named. | The owner on #190; #196 | §10.4 |
| 6 | **A mother lays on a free tile with no item on it** as well as no sprite. Eggs count toward `most_sprites` on the map or in the Cursor. | #196 | §10.3, §10.4, §10.6 |
| 7 | **An old save's eggs load as items,** on their tile or the nearest free one, hatching when they would have; one that can't be an item hatches as it loads. A replay recorded before this change, with the same version number, parts from its recording once an egg is laid. | #196, default 4 | §10.4 |
| 8 | **On screen, an egg is drawn from the theme's `objects`,** reversed while selected; an old theme's `egg` tile still draws it. Its card, on every sprite tab, says when it hatches, or what it waits for, and its parents, and the selection goes on to the hatchling. The sprite list and Tab leave eggs out; the World tab counts them; the Cursor's mark shows a led young sprite small. | #196 | §10.7 |
