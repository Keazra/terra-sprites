# The M2 design checked against the code

Slice 30a ([#148](https://github.com/Keazra/terra-sprites/issues/148), settled in [#226](https://github.com/Keazra/terra-sprites/issues/226) round 1, Q9): every rule in the M2 design was traced to the code that carries it out and a test that holds it, on 2026-10-17. Where the code was wrong it was fixed, test first, and where the design's words were wrong they're corrected here. A rule with no test got one. Nothing in the design was missing from the code.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Remembered places are fixtures:** the design said an object type's entry says whether it "stands on its own tile"; what it says is whether it's a fixture, its `Fixture` tag. | Trace | §7.1 |
| 2 | **A committed path is M1's** (M1 design §3.7); M2 has no §3.7. | Trace | §7.4 |
| 3 | **The `age` sensor is a share of the age the sprite grows old at,** not of `grows_old`, so it reaches 1 sooner for a worn sprite or one with broken genes, as it grows old. | Trace | §9.3 |
| 4 | **Even the unworn starter takes more than one hit a tick** once old, about 1.7, so whole hits land each tick; the design said that was a worn sprite's case. | Trace | §9.4 |
| 5 | **The genes' average health is over the genes that can weaken,** settings left out, and a genome with none gives out as soon as it's old. | Trace | §9.4 |
| 6 | **An egg may be laid in shallow water:** on a tile a sprite could stand on, where the design said "on land". | Trace | §10.3 |
| 7 | **Longing never reaches 0.5 with the starter's genes:** it's about 0.28 after an hour with no male seen or heard, and levels off a little above 0.4. No rule waits on 0.5 since Slice 32 (§15.7). | Trace | §10.3 |
| 8 | **Space is blank, in the terminal's own background,** which is black on most. | Trace | §11.3 |
| 9 | **A theme without the glass and star keys draws the default theme's,** not "the built-in theme's": there are two. | Trace | §11.5 |
| 10 | **The starter has 137 values a change can move,** not 83, so about three hatchlings in four have a copying error, not a little over half. | Trace | §12.1, §12.6 |
| 11 | **A copy keeps at least one gene this build can read:** the last is kept only when every one before it was lost, and the gene kept may do nothing. The design read as if the final gene were never lost. | Trace | §13.1 |
| 12 | **A genome's longest is 236 genes since Slice 28** (the starter's 118, twice). | Trace | §13.1 |
| 13 | **A graph record that began off the beat** keeps its first count and those on the new beat as it squeezes. | Trace | §14.1 |
| 14 | **The graph's and the tree's pointers leave the Cursor where it is** on the map beneath. The tree's moved it, which the design didn't say; it now works as the graph's does. | Trace; lead's call, matching §14.3 | §14.3, §17.3 |
| 15 | **With learning off, sprites grow no more familiar with each other:** knowing an individual is learned, though familiarity with object types still grows (M1 design §5.6). The code always did this; the design didn't say. | Trace; lead's call, keeping the code | §15.2 |
| 16 | **A call counts as heard for as long as it's remembered** (`heard_for`, 150 ticks), for when partners last heard each other and for a female's time with no male seen or heard. | Trace | §15.4, §15.7 |
| 17 | **A sprite is held back from hitting its family, as a lean:** cornered, crowded and hit together, or trying something new, it still can. The design said it never does. | Trace | §15.6 |
| 18 | **A fast-forwarded world is the game's with `--lab`:** fast-forward always makes a Lab world, and the mode is world state, so the Story world from the same seed differs in its state hash. | Trace | §16.3, §16.5 |
| 19 | **The World tab sets a single generation's number to the right,** `generation        1`, as its other numbers are. | Trace | §17.5 |
| 20 | **The arrows and the pointer pass a hidden card by** in the family tree, where they could choose it, though choosing it did nothing. | Trace | §17.6 |
