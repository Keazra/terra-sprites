# Life stages and ageing

Slice 22 ([#140](https://github.com/Keazra/terra-sprites/issues/140), PR #177): sprites grow up and grow old at ages their genes set, genes switch with life stage, and old age is the genome weakening. Settled with the owner in round 1 on 2026-10-11; the owner changed the first answer from the body wearing out to genes weakening the same day, after reading how real ageing works.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Three life stages,** young, adult and old. An Egg stage may come later. | Owner, Q2 | §9.1 |
| 2 | **A world's first sprites start as adults,** at ages drawn through adulthood; a placed sprite is a newborn. | Round 1 | §9.1 |
| 3 | **A gene can switch with life stage** (`from`, `until`), and is dormant outside them. Traits and starting levels can't switch. Changing stage leaves what was learned untouched. | #49, round 1 | §9.2 |
| 4 | **Two genes time a life,** `grows_up` and `grows_old`, so each evolves on its own. Sprites grow up and grow old at their 1- and 3-hour times from this slice on. | Owner, Q3, Q4 | §2, §9.3 |
| 5 | **`grows_old` replaces lifespan.** Old genomes and saves read `lifespan` as it. | Owner, Q6 | §2, §9.3 |
| 6 | **Growing up costs food,** the same energy however long it takes. A later old age's energy cost is built but held at 0 until genes evolve in Slice 24, since it undid A3's training. | Owner, Q5; owner, 2026-10-11 | §3, §9.3 |
| 7 | **Old age is the genome weakening:** every gene has a health that scales its effect; once old, damage lands on random genes (instincts at half), until its body gives out at about half and old age kills it. Healing slows as genes weaken. Not inherited. | Owner, Q1 (revised), 2026-10-11 | §3, §9.4 |
| 8 | **Wear comes from the body** (starving, parched, injured), never heals, brings old age sooner and speeds decay up to 3×, a tunable number. A bad injury can scar a gene at any age. Stress chemicals damaging genes wait for Slice 24 (#142). | Owner, Q1, Q9, Q10 | §9.4 |
| 9 | **The young walk slower, are more curious and learn faster** (`plasticity`, a new brain parameter), and only adults fight for room. | Owner, Q5, Q7, Q8 | §9.5 |
| 10 | **The young are drawn `ö` (CP437) and `a` (ASCII);** the log says when a sprite grows up and grows old; the Body tab shows its stage, wear and genes' health; the Genome tab shows each weakened gene's health. | Owner, Q11, Q12 | §9.6 |
