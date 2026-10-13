# Copy and change

Slice 24's second part, 24b ([#142](https://github.com/Keazra/terra-sprites/issues/142), [PR #199](https://github.com/Keazra/terra-sprites/pull/199)): now and then a whole gene is doubled, lost, or changed in what it does as an egg's genes are copied. What a change can change was settled in the slice's first design round (Q4); how often, which genes can be lost, how the player finds out and the bars in a second round of four questions on 2026-10-13, where the owner chose one hatchling in 10 for how often and left the other recommendations standing.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Doubled, lost or changed:** as an egg's genes are copied, one roll a gene decides whether it's doubled, lost or changed in what it does, each with `copy_and_change`'s chance (0.00052), so about one hatchling of the starter's in ten has one. A double sits after its original, with its switch and flaws. A change is to what an instinct reacts to or its action, what an attention instinct reacts to or what it attends to, or the life stages a gene works in (not a trait's or a starting level's), each a choice the pack has; three in four (`on_copy`) are on a fresh copy, so the original still works. Unknown and unmatched genes are kept exactly, taking no roll. A pack without `copy_and_change` changes nothing. | Round 1, Q4; round 2, Q1 (owner: "one in 10") | §13.1 |
| 2 | **Any gene can be lost,** vital ones too; the last gene this build can read never is. | Round 2, Q2 | §13.1 |
| 3 | **Every changed gene is checked again;** none that worked comes to change the body directly; mating stays between adults whatever a gene says. | Round 1, Q4 | §5, §13.1 |
| 4 | **A genome never grows past twice the starter's length** (`longest` 2): a copy is made only while it fits, and a pair's child drops genes only one parent has, the last first, to fit. | Round 1, Q4 | §12.2, §13.1 |
| 5 | **The Genome tab lists genes new, doubled and lost,** and the log names a hatchling's changed genes on its hatching line. | Round 2, Q3 (B) | §13.2 |
| 6 | **How we know it worked:** 200,000 ticks, seeds 1–10: a gene the starter lacks carried by a tenth of the living in at least 5 seeds; tests with random genomes for safety and length; sprites alive in at least 8, never over the limit; A1–A5 still met. The report also says in how many of a new gene's carriers it works. Measured at `d8a33c8`: all met; new genes took hold in 8 seeds, but worked in a tenth of the living in only 3. Hunger and thirst deaths rose from 0.6% to 5.1% of the long run's, where losing "always → wander" and "male_near → longing" spread. | Round 2, Q4 | §13.3 |
| 7 | **G4's numbers line up with the starter's by name,** so a sprite with a gene doubled or lost still counts when §12.7's measure is run again. | Needed by 1 | §12.7, §13.3 |
| 8 | **Later:** the error rate as a gene (#194); Slice 32's courtship and families (#186) to check that pairs still pay, since breeding alone spread in two seeds. | Round 1, Q2; this part's measurements | §13.4 |
