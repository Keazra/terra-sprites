# Generations on the population graph

The population graph's reading says which generations were alive at the picked moment, as the World tab says it for now. Settled in Slice 25's design round (Q6, A) and filed as [#208](https://github.com/Keazra/terra-sprites/issues/208), built in PR [#215](https://github.com/Keazra/terra-sprites/pull/215); the owner chose three reading lines on 2026-10-14, when the one line #208 described didn't fit the game's smallest screen.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Each count in the population record keeps the oldest and newest generations among its sprites,** not its eggs, and keeps them when the record squeezes. The world checks every tick that they run from the oldest to the newest, with none in a count of no sprites. A save from before has counts without them, which read no generations; the save format stays 1, with a default. A replay recorded before parts from its recording, as one does between builds that share a version number. Nothing in the world reads them. | #208 | §14.1, CONTEXT "Population record" |
| 2 | **The lines that read a moment say the generations alive then,** `generations 3 to 6` or `generation 1`, after the count of sprites, as the World tab says them. | #208; Slice 25 round, Q6 | §14.3, §17.5 |
| 3 | **The reading is three lines, not two:** the tick, how many sprites and their generations; the eggs and the stages with their shares; and what happened since. On one line it ran to about 110 characters, past the box at 100 columns, and cut off the old; the graph gives up a row for the third line. | The owner, 2026-10-14 (a card in the #208 thread) | §14.2's mock-up, §14.3 |
| 4 | **The full graph's middle label always says `50%`,** as §14.2 has it. With an odd number of rows, as the 19 rows at 100×30 now are, it would say `52%`, the top of the middle row, where half way is that row's middle. | Found in the build, with row 3 | §14.2 |
| 5 | **Generations on the graph are built,** so they leave the lists of what's later. | #208 | §14.8, §17.8 |
