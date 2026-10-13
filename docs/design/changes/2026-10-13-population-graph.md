# The population graph

Slice 26 ([#144](https://github.com/Keazra/terra-sprites/issues/144)): a graph of the world's population over its whole run, stacked by life stage, full screen on `p` and small at the top of the World tab, with a moment picked and read under it. Settled in a round of four questions on 2026-10-13, whose recommendations the owner kept ("No disagreements").

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The world keeps a population record:** its sprites by life stage at the first tick it runs, then every 100 ticks, with the eggs laid and hatched, the sprites spawned and the deaths since the count before. It's world state, saved, in the state hash and checked every tick; nothing in the world reads it. At 512 counts it keeps every other one and counts half as often, so it covers the whole run and a save stays small. A save from before it begins its record when loaded. | #144 ("done when": the whole run) | §14.1 |
| 2 | **The graph stacks the life stages,** young, adults, old and eggs from the bottom, in the theme's colours, with a dotted dark gray line at the world's limit (the round said gray, but old sprites are gray). A column is 100 ticks until the run fills the graph, then the whole run is squeezed in. | Q1 (B) | §14.2 |
| 3 | **`p` opens it over the panels, and a small copy heads the World tab.** Time runs on behind the full one. | Q2 (C) | §14.2, §14.4 |
| 4 | **The arrows or the pointer pick a moment,** marked `▲`; two lines under the graph read its count in each stage and what was laid, hatched, spawned and died since the column before, with the causes. With nothing picked, they read now. A pick stays on its moment as time runs; the pointer leaves the Cursor where it is. Laid eggs are counted so a rise in eggs reads too. | Q3 (B); the slice's review | §14.3 |
| 5 | **Themes gain a `population` section:** each stage's colour, the whole and half blocks and the limit's dot. A theme without it, or part of it, draws what it leaves out as the default theme does. | Needed by 2 | §14.5 |
| 6 | **The graph is a panel of its own,** `Graph`, about the world: denied, the overlay is empty and the World tab leaves it out. | M1 design §6.4 | §14.6 |
| 7 | **How we know it worked:** UI tests of both graphs, a picked moment and a loaded world; the `generations` run byte for byte the same as `main`'s; A5 at least 200 ticks a second and within 5% of `main`. Measured: all met. | Q4 | §14.7 |
| 8 | **G3 is the graph.** | #144 | §6 |

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **`p` opens the population graph:** the overlays, the key table, the help screen, the key hints, the World tab and the snapshots name it, pointing to M2 design §14. | #144 | §6.1, §6.5, §7.2 |
| 2 | **A `Graph` panel** joins the information policy's. | #144 | §6.4 |
