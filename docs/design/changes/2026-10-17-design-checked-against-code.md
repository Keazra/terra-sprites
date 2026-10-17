# The M2 design checked against the code

Slice 30a ([#148](https://github.com/Keazra/terra-sprites/issues/148), settled in [#226](https://github.com/Keazra/terra-sprites/issues/226) round 1, Q9): every rule in the M2 design was traced to the code that carries it out and a test that holds it, on 2026-10-17. Where the code was wrong it was fixed, test first, and where the design's words were wrong they're corrected here. A rule with no test got one. Nothing in the design was missing from the code. Where the design left a choice open, the row says it was Claude's call and which way it went, for the owner to overrule in review.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Remembered places are fixtures:** the design said an object type's entry says whether it "stands on its own tile"; what it says is whether it's a fixture, its `Fixture` tag. | #148 | §7.1 |
| 2 | **A committed path is M1's** (M1 design §3.7); M2 has no §3.7. | #148 | §7.4 |
| 3 | **A trip with no way there leaves the sprite's eye where it was,** on what it's doing, so the place it can't reach doesn't take its eye for the next tick either, and its latest step 5 is that tick's whether it's doing something or nothing. The code moved the eye, and kept an idle sprite's step 5 from an earlier tick, so it learned nothing by watching the next tick; both are fixed. | #148 | §7.4, §18.1 |
| 4 | **The title screen works as a world does:** under 100×30 it takes no clicks, though its keys still work, the pointer leaves "Quit? (y/n)" waiting, Load takes the wheel and clicks and says where it looked when empty, and a save's name outside CP437 shows `?`. Each was a gap the trace found; the code is fixed. | #148, Claude's call on the pointer at Quit | §8.2 |
| 5 | **A dormant gene sets nothing,** so the next gene that sets the same value works in its place, as the starter's two `watch_rate` genes rely on. The code always did this; the design didn't say. | #148 | §9.2 |
| 6 | **A sprite whose stage changes lets its last decision go,** since the concepts it was made with may be gone, so on the tick it grows up or grows old it learns nothing by watching, and one watching it is shown nothing. The code always did this; the design didn't say. | #148, Claude's call, keeping the code | §9.2, §18.1 |
| 7 | **A genome with no `grows_up` gene grows up at 7,500 ticks,** the middle of its range, as any trait no gene sets does (M1 design §4.8). | #148 | §9.3 |
| 8 | **The `age` sensor is a share of the age the sprite grows old at,** not of `grows_old`, so it reaches 1 sooner for a worn sprite or one with broken genes, as it grows old. | #148 | §9.3 |
| 9 | **Even the unworn starter takes more than one hit a tick** once old, about 1.7, so whole hits land each tick; the design said that was a worn sprite's case. | #148 | §9.4 |
| 10 | **The genes' average health is over the genes that can weaken,** settings left out, and a genome with none gives out as soon as it's old. | #148 | §9.4 |
| 11 | **Genes weakening leave an old sprite's mind as it was,** its last decision included, so it still learns by watching and is still watched. The code let that decision go whenever one of its instincts took a hit, which once old is often, so on those ticks it learned nothing by watching, and a young one watching it was shown nothing. The code is fixed. | #148 | §9.4, §18.1 |
| 12 | **An egg may be laid in shallow water:** on a tile a sprite could stand on, where the design said "on land". | #148 | §10.3 |
| 13 | **Longing never reaches 0.5 with the starter's genes:** it's about 0.28 after an hour with no male seen or heard, and levels off a little above 0.4. No rule waits on 0.5 since Slice 32 (§15.7). | #148 | §10.3 |
| 14 | **Space is blank, in the terminal's own background,** which is black on most. | #148 | §11.3 |
| 15 | **A theme without the glass and star keys draws the default theme's,** not "the built-in theme's": there are two. | #148 | §11.5 |
| 16 | **The starter has 137 values a change can move,** not 83, so about three hatchlings in four have a copying error, not a little over half. | #148 | §12.1, §12.6 |
| 17 | **A copy keeps at least one gene this build can read:** the last is kept only when every one before it was lost, and the gene kept may do nothing. The design read as if the final gene were never lost. | #148 | §13.1 |
| 18 | **A genome's longest is 236 genes since Slice 28** (the starter's 118, twice). | #148 | §13.1 |
| 19 | **A graph record that began off the beat** keeps its first count and those on the new beat as it squeezes. | #148 | §14.1 |
| 20 | **A call's `listen` ticks count the tick it calls,** 10 in all. | #148 | §15.1 |
| 21 | **With learning off, sprites grow no more familiar with each other:** knowing an individual is learned, though familiarity with object types still grows (M1 design §5.6). The code always did this; the design didn't say. | #148, Claude's call, keeping the code | §15.2 |
| 22 | **A call counts as heard for as long as it's remembered** (`heard_for`, 150 ticks), for when partners last heard each other and for a female's time with no male seen or heard. | #148 | §15.4, §15.7 |
| 23 | **A sprite is held back from hitting its family as a lean, not a rule:** any two of being cornered, crowded (adults only) and just hit, each at full strength, outweigh it, and one trying something new can still hit. The design said it never does. | #148 | §15.6 |
| 24 | **A family member found is one that was missed:** `found_family` comes only for an egg or young one out of sight at the last look and back in sight now. With one egg away and another in sight, the away one hatching counted as found, and so did a young one dying or growing up out of sight while another was in sight. The code is fixed, and which eggs and young were missed is world state. A save from before it loads with none missed, so one away when it was saved isn't found when it comes back. | #148 | §15.6 |
| 25 | **A fast-forwarded world is the game's with `--lab`:** fast-forward always makes a Lab world, and the mode is world state, so the Story world from the same seed differs in its state hash. | #148 | §16.3, §16.5 |
| 26 | **`--fast-forward` refuses more hours than a run can count,** 409,927,646,082,434, past which its tick counts overflowed. | #148 | §16.3 |
| 27 | **The family tree's pointer leaves the Cursor where it is** on the map beneath, as the graph's does (§14.2). It moved it, which the design didn't say; it's fixed. | #148, Claude's call, matching the graph | §17.3 |
| 28 | **The World tab sets a single generation's number to the right,** `generation        1`, as its other numbers are. | #148 | §17.5 |
| 29 | **The arrows and the pointer pass a hidden card by** in the family tree, where they could choose it, though choosing it did nothing. | #148 | §17.6 |
| 30 | **A parent shows its young something only when the young sees it,** as the table at the top of §18 says; one heard calling out of sight counted. The code is fixed. | #148, Claude's call, following §18's table | §18.1 |
