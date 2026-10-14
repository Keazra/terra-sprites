# The family tree

Slice 25 ([#143](https://github.com/Keazra/terra-sprites/issues/143)): each sprite has a generation, the world keeps a record of its dead, and `k` opens a family tree of the selected sprite, from grandparents to young, walked with the arrows; `b` gains the lineage colours. Settled in a round of nine questions on 2026-10-14. The owner kept every recommendation, made the chosen sprite blink ("Probably a blinking cursor", Q5), and pinned grieving for and remembering the dead for later (Q8, [#206](https://github.com/Keazra/terra-sprites/issues/206)).

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Each sprite and egg has a generation:** 1 for the world's first sprites and the Cursor's, else one more than its parents', or the newer parent's. It also knows where it came from: the world's first, placed by the Cursor, or hatched. | Q6 (A) | §16.1 |
| 2 | **The world keeps a record of its dead,** each with its name, parents, generation, origin, birth, death, cause, life stage and sex. It's world state, saved, in the state hash and checked every tick there's been a death. It keeps the newest 10,000, forgetting the longest dead first, but never a sprite the player named. | Q4 (B) | §16.1 |
| 3 | **Nothing in the world reads the record,** so the sprites live exactly as before. Grieving for and remembering the dead is for later. | Q8 (A); the owner, #206 | §16.1, §16.8 |
| 4 | **A save from before the slice** loads with no record: its dead are forgotten, named from their young and said to have **lived long ago**, and its sprites and eggs are given generations in ID order. The save format stays 1. | M1 design §2.8 | §16.1 |
| 5 | **`k` opens the family tree over the panels,** on the selected sprite or egg, living or dead. Time runs on behind it; `Tab` and `Shift+Tab` redraw it around the next sprite. | Q1 (A) | §16.2 |
| 6 | **It reaches from grandparents to young,** with the young grouped by their other parent, and brothers and sisters at the foot with the parent they share when it's one. A row that doesn't fit scrolls, with `◄` and `►`. A sprite in an inbred line can be on it twice. | Q2 (A) | §16.2 |
| 7 | **Each card says** the life stage and how many young, an egg's hatching, a death and its cause, or `lived long ago`, the dead in dark gray; the sprite in the middle also its sex, age and generation. A sprite with no parents says where it came from. | Q3 (A) | §16.2 |
| 8 | **The arrows or the pointer choose a sprite on it,** which blinks; `Enter` or a click walks the tree to it and selects it, dead or alive, but not one that lived long ago. | Q5 (A), with the blink the owner's | §16.3 |
| 9 | **A sprite named before it died keeps its name** in the record, so a load names the dead as before. | Needed by 2 | §16.3 |
| 10 | **`b` gains a third colour mode, lineage:** elders light blue, brothers and sisters light red, young and theirs light magenta, the rest dark gray. The round called it Family; it's lineage, since a family is Slice 32's household. Themes gain a `lineage` section, and the help screen a line. | Q7 (A); CONTEXT.md | §16.4 |
| 11 | **The Body tab and an egg's card say the generation** after the parents or origin, and the World tab the generations alive. | Q6 (A) | §16.5 |
| 12 | **The tree is a panel of its own,** `FamilyTree`, about each sprite on it; a hidden one is a blank card in its place, unnamed as another's parent, that leads nowhere, and the lineage colours ask it too. | M1 design §6.4; the slice's review | §16.6 |
| 13 | **How we know it worked:** UI tests and snapshots of the tree, the world's tests of the record, the `generations` run byte for byte the same as `main`'s, and A5 at least 200 ticks a second and within 5% of `main`. Measured: the same run, and the same speed within the noise. | Q9 | §16.7 |
| 14 | **G2 is the tree;** generations on the population graph, and the partner on the tree once Slice 32 is in, are later. | #143 | §6, §10.7, §14.8, §15.8, §16.8 |

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **`k` opens the family tree:** the overlays, the key table, the help screen, the key hints and the snapshots name it, pointing to M2 design §16. The Body and World tabs say the generation. | #143 | §6.1, §6.5, §7.2 |
| 2 | **Lineage, the colour mode M1 promised M2 would add,** is described, and the help screen gains its legend. | #143 | §6.1, §6.3 |
| 3 | **A `FamilyTree` panel** joins the information policy's. | #143 | §6.4 |
