# Gene warnings

[#274](https://github.com/Keazra/terra-sprites/issues/274): the baseline report and the end of a fast-forward say when the species' genes have gone somewhere worth a look, the "genetic canary" of the owner's notes. Designed on 2026-10-19 by an agent alone, from #274's design brief, the owner's principles and the decisions on [#237](https://github.com/Keazra/terra-sprites/issues/237) and [#221](https://github.com/Keazra/terra-sprites/issues/221), as triage's autonomous design allows (`.claude/skills/triage/AUTONOMOUS-DESIGN.md`). The owner agrees the warning levels before anything is measured (`docs/agents/how-we-work.md`, "When a decision waits on a measurement"). Nothing is built yet: the build follows Slice 30 ([#148](https://github.com/Keazra/terra-sprites/issues/148)).

## The levels to agree

| Warning | Level | From |
|---|---|---|
| A needs gene is being lost | more than half the hatched sprites alive lack it | #237's decision 4, its line for careful copying |
| A gene value has run to the end of its range | its middle among the living has gone nine tenths of the way from the starter's value to that end | #274's settled list; Q3 below says why not "within a tenth of the end" |
| Too few to judge | no warning with fewer than 10 hatched sprites alive | A4's "fewer than 10 deaths" (M1 design §7.4) |
| Flaws and broken genes | reported, no warning yet | #221; Q3 |

## The questions, answered

The slice loop's grilling step, run by the designing thread on itself: each question, its answer, and the principle or decision it rests on.

1. **What does the warning look at?** At the end of the run, among the hatched sprites alive: the hidden flaws they carry and their broken genes, each on average; the share of them lacking each of the starter's genes; and each gene value held within a range, by its middle among the living.
   *Rests on:* #274's settled list (flaws building up, genes breaking, values running to the edge of their range); #237's decision 4, whose 220-hour report lists the share of the living lacking each starter gene.
2. **Which sprites count?** For flaws and lacking genes, the hatched sprites alive, eggs aside: the world's first sprites have the starter's genes, so they could only water the numbers down. For gene values, the middle among all the living, as the genes table and the values that moved most count it, so a warning's number is the table's; by the end of a run long enough to matter they're the same sprites.
   *Rests on:* how the generations run (§12.7, §13.3) and #237's checks counted flaws and lacking genes; §16.4's middle among the living.
3. **What counts as a warning?**
   - **More than half lacking a needs gene.** #237 already drew this line for careful copying, so the warning says, on every run, when half of that trigger holds.
   - **A gene value's middle nine tenths of the way to the end of its range,** measured from the starter's value. The design brief said "within a tenth of the range's end", but three of the starter's brain parameters start that close to an end (`trace_decay` at the bottom of its range, 0.5; `tau_base` at 0.1 and `tau_att_base` at 0.2, above a bottom of 0.05), so they'd warn from the first tick. Measured from the starter's value, a warning means the species has used up nearly all the room its range gave it, and physiology's limit, not selection, now holds the value. Whether that limit is right is then the owner's call.
   - **No warning on fewer than 10 hatched sprites,** as A4 judges nothing on fewer than 10 deaths: a world nearly gone says nothing about genes.
   - **No warning for flaws and broken genes yet.** #221 found that die-outs came from gene changes, not flaws, and no run has shown where flaws level off over a hundred generations. Fast-forward's genes table gains them each hour, so the owner's 220-hour runs will show it, and a warning can follow (§16.7).
   - **No warning for values with no range,** such as an instinct's weight or a half-life. They have no end to run to, and moving is what G4 asks of them, so a warning on a big move would sound just when evolution works. An instinct's weight works as 1 past 1 (M1 design §4.3), but many of the starter's weights are 1 already, so that's no end to warn at either.
   - **No warning for the starter's other genes** that the species loses: a gene the species does better without is evolution (#237's decision 1). They're listed, from a tenth of the living, as the generations run lists them.
   *Rests on:* #237's decisions 1 and 4; #221; G4 (§12.7); A4 (M1 design §7.4).
4. **Which genes are the needs genes, and where is that written?** The eleven #237 named, found from the data rather than listed in the code: the needs (`brain_io.ron`) that a starter gene raises from a physical chemical are the body's needs, hunger, thirst and tiredness; a needs gene is a starter gene that adds to one, is its half-life, or is an instinct with it as an input. On the starter at `ce9cfc7` that finds exactly #237's eleven (checked with a scratch build, below), and a test pins them for the built-in pack, so a starter that changes them fails the test and the change goes to the owner. A data pack with another species gets its own needs genes, with no new code.
   *Rests on:* the owner's principle that each thing is self-contained in its data and that modding should stay easy (`docs/agents/how-we-work.md`); #237's list.
5. **How long a run?** The baseline's viability run as it is, 50,000 ticks on seeds 1–10, about 11 hours of play: no extra run, and it catches a change that makes the species throw something away fast, on every commit of `main`. About a hundred generations is the end of a 220-hour fast-forward on the owner's PC, and Slice 30's 220-hour report.
   *Rests on:* design §7.6, which sends what's too slow for CI to the baseline; GitHub's six-hour job limit; #274's settled "the same check at the end of a fast-forward".
6. **Where is it shown?** In the baseline report, near the top, after what moved and anything broken, and in a Genes section with a row for each seed; at the end of a fast-forward, after the values that moved most; and in two new columns of fast-forward's genes table, flaws and broken genes each hour. What moved covers each seed's warnings, flaws and broken genes, so a warning that comes or goes is among the first lines. A warning is no verdict: no criterion's verdict changes, and the launcher files an issue only for a broken run, as before.
   *Rests on:* design §7.6 (the report opens with what moved; issues only for a broken run); §16.4.
7. **Where does the logic live?** In the simulation's library, beside the gene values' code: given a world, one measure counts it all and gives the warnings, reading the ranges and the needs genes from the world's own data pack, which only the library can read. The baseline, fast-forward and the generations run use it. Slice 30's 220-hour report comes first and counts lacking genes as the generations run does (§13.3); the build takes that count into the library unchanged, so that report, careful copying's trigger and the warnings count alike, and the generations run's report stays byte for byte as it was, as §16.4's move did. The viability run's result carries its living's genes at its end, so the baseline runs nothing extra.
   *Rests on:* #274's note that the gene-value code is already shared; one thing in one place; §16.4's precedent.
8. **What's it called?** "Gene warnings", in plain words. The owner's notes called the idea a genetic canary, and the name is theirs to change.
   *Rests on:* "Plain words, from the start" (`docs/agents/how-we-work.md`).
9. **Does it change the game?** No. It only reads the world at the end of a run, so worlds, saves and state hashes are as they were, and nothing a slice or check is measuring now moves. The build ranks minor: no playtest.
   *Rests on:* triage's "Contained" test; the playtest ranks (`docs/agents/how-we-work.md`).

## What it would have said, for information

These were looked at after the levels above were set, and changed none of them.

**`main` at `ce9cfc7`, the viability run** (the default world left alone, 50,000 ticks, seeds 1–10, in the cloud, a scratch build of the measure that isn't in the game repo):

No warnings, on any seed. Nine seeds ended with 82 to 99 sprites alive and seed 8 with 46, all of them hatched. No needs gene was lacked by more than 2% of them, and no ranged value went more than a fifth of the way to an end of its range. The measure found the eleven needs genes #237 named, and 14 of the starter's 163 gene values held within a range.

| Seed | Hatched alive | Flaws carried, each | Broken genes, each | Needs gene lacked most | Ranged value furthest towards an end |
|---|---|---|---|---|---|
| 1 | 91 | 5.0 | 0.19 | `hunger → Eat`, 1% | `SenseRadius` 10 → 10.4, 11% of the way to 14 |
| 2 | 96 | 6.0 | 0.12 | `hunger`'s half-life, 1% | `SenseRadius` 10 → 10.3, 8% of the way to 14 |
| 3 | 98 | 5.2 | 0.08 | `stamina Level → tiredness`, 1% | `GrowsOld` 22,500 → 28,676, 16% of the way to 60,000 |
| 4 | 96 | 6.4 | 0.16 | none | `GrowsOld` 22,500 → 30,466, 21% of the way to 60,000 |
| 5 | 96 | 5.2 | 0.07 | none | `SenseRadius` 10 → 10.6, 14% of the way to 14 |
| 6 | 92 | 5.5 | 0.18 | `hunger → Eat`, 2% | `SenseRadius` 10 → 9.7, 9% of the way to 6 |
| 7 | 95 | 4.8 | 0.13 | `ate Level → hunger`, 1% | `SenseRadius` 10 → 10.5, 13% of the way to 14 |
| 8 | 46 | 4.8 | 0.30 | none | `SenseRadius` 10 → 10.5, 13% of the way to 14 |
| 9 | 99 | 4.4 | 0.14 | `stamina Level → tiredness`, 1% | `SenseRadius` 10 → 9.4, 14% of the way to 6 |
| 10 | 82 | 5.8 | 0.10 | none | `watch_rate` (young) 0.5 → 0.53, 6% of the way to 1 |

The starter's genes a tenth or more of them lacked were two, neither a needs gene: `bred Level → arousal`, by 14% in seed 5, and `h0`'s half-life, by 13% in seed 7.

**#225's 220-hour check** (a scratch build of Slice 29's settings on the owner's PC, seeds 1–10, 2026-10-18): no needs gene would have warned; the most any lacked was `thirst → Drink`, by 15%. The range warning would have caught the short childhoods that check found: the middle `GrowsUp` reached 1,500 ticks, the bottom of its range, in seed 3 and 1,791 in seed 8 by hour 200, past the warning at 1,800. Those childhoods outlasted thirst, the fault that check went on to fix by pacing a young sprite's dying (#225).

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The viability run reports the genes of its living at its end, with gene warnings,** and the baseline report gives them after what moved and anything broken. A warning is no verdict, and the launcher files no issue for one. | #274 | §7.6 |

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Gene warnings:** at the end of a fast-forward and in the baseline report, when more than half the hatched sprites alive lack a needs gene, or a gene value's middle has gone nine tenths of the way to the end of its range, with at least 10 hatched sprites alive. Flaws, broken genes, the other starter genes lacked and values with no range are reported with no warning. | #274; #237's decisions 1 and 4; #221 | §16.3, §16.4, §16.6 |
| 2 | **The needs genes are found from the data:** the starter's genes that add to a need it raises from a physical chemical, are its half-life, or are an instinct with it as an input; eleven in the built-in pack, pinned by a test. | #237; the owner's modding principle | §16.6 |
| 3 | **Fast-forward's genes table gains flaws and broken genes** each hour, among the hatched sprites alive. | #274 | §16.4 |
| 4 | **One measure in the simulation's library** gives the warnings to the baseline, fast-forward and the generations run, and takes over the count of lacking genes that Slice 30's 220-hour report makes first. | #274 | §16.6 |
| 5 | **Later:** a warning for flaws, once the 220-hour runs show where they level off. "Later" becomes §16.7. | #274 | §16.7 |

## CONTEXT.md

**Needs gene** and **Gene warning** are new; **Gene value** says which values are held within a range.
