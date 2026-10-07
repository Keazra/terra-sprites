# Terra Sprites — M2 "Generations" design (v1)

- **Status:** Draft: the plan. Each slice adds its details in a new revision.
- **Date:** 2026-10-05
- **Builds on:** [M1 design v39](https://github.com/Keazra/terra-sprites/blob/refs/tags/design/m1-v39/docs/design/m1-a-sprite-lives-v39.md), which stays the rulebook for everything M2 doesn't change
- **Covers:** what milestone 2 contains, its slices and their order, the time scale of a sprite's life, the direction for ageing, and what "M2 is done" means

---

## Decisions from the planning session

The owner planned M2 in a `/grill-with-docs` session on 2026-10-05, in three rounds of questions. The rounds are notes in the Claude project's shared files, not in this repository; everything they settled is in the table below. Each recommendation the owner didn't object to stands, as usual.

| # | Decision | Source | Sections |
|---|---|---|---|
| 1 | **M2 is the roadmap's six items** (life stages; breeding, alone or with a partner as the genome decides; mixing and mutating genes; a family tree; population graphs; a fast-forward mode with no screen), **plus** mating only between adults (#69), memory of places (#40), learning by watching (#66), a harsher world (#122), the speed cap (#121) **and a title screen**. Parenting (#67) and dead bodies (#85) wait. | Round 1 Q1; the title screen is the owner's addition | §1 |
| 2 | **A sprite lives about 4 hours at 1×:** grown up at about 1 hour, old at about 3. "That's enough time to get attached to a Sprite." | The owner, round 1 Q3; ticks in round 2 Q6 | §2 |
| 3 | **Lives get shorter with breeding,** not before, so the world never ships with everyone dying off. | Round 2 Q7 | §2 |
| 4 | **The genome brings ageing about,** not a fixed timer: genes that switch on with age, wear from a hard life, and later the quality of a sprite's genes. A long life costs something. | The owner, round 2 Q6; readings in round 3 Q11 | §3 |
| 5 | **One M2 design document,** sketched in planning; each slice adds a revision. | Round 1 Q4 | — |
| 6 | **Eleven slices, in order, numbered on from M1's:** Slices 20–30. | Round 2 Q8–Q9 | §4 |
| 7 | **Memory of places is built first.** | Round 1's decision card (recommended; the owner raised no objection) | §4 |
| 8 | **The title screen is its own slice, opened by a design pass:** mock-ups, then a prototype in Windows Terminal. | The owner, round 2 Q10; round 3 Q12 | §4 |
| 9 | **Seven criteria say M2 is done,** with numbers set in each slice before measuring. | Round 1 Q2 | §6 |

---

## 0. Vision

M1 made a sprite live: it feels, learns and dies. M2 makes **generations**. Sprites grow up, grow old and have young; children take after their parents but not exactly; and over an evening the player watches a family line, and a species, change.

The owner's principles carry over unchanged (`docs/agents/how-we-work.md`). Three matter most here:
- **The player has to care first.** A life long enough to get attached to is why a sprite lives about 4 hours at 1× (§2).
- **Grown sprites fend for themselves; the young survive by luck or guidance.** M2 is when this can be met (Slice 29).
- **Content safety is the engine's one exception** to "the engine enforces physics only" (§5).

---

## 1. Scope

### 1.1 In M2

| Item | Slice | Issue |
|---|---|---|
| Memory of places | 20 | [#138](https://github.com/Keazra/terra-sprites/issues/138) (from [#40](https://github.com/Keazra/terra-sprites/issues/40)) |
| Title screen | 21 | [#139](https://github.com/Keazra/terra-sprites/issues/139) |
| Life stages and ageing | 22 | [#140](https://github.com/Keazra/terra-sprites/issues/140) (from [#49](https://github.com/Keazra/terra-sprites/issues/49)) |
| Breeding, adults only | 23 | [#141](https://github.com/Keazra/terra-sprites/issues/141) (with [#69](https://github.com/Keazra/terra-sprites/issues/69)) |
| Mixing and mutating genes | 24 | [#142](https://github.com/Keazra/terra-sprites/issues/142) |
| Family tree (lineage) | 25 | [#143](https://github.com/Keazra/terra-sprites/issues/143) |
| Population graph | 26 | [#144](https://github.com/Keazra/terra-sprites/issues/144) |
| Fast-forward with no screen, and the speed cap | 27 | [#145](https://github.com/Keazra/terra-sprites/issues/145) (with [#121](https://github.com/Keazra/terra-sprites/issues/121)) |
| Learning by watching | 28 | [#146](https://github.com/Keazra/terra-sprites/issues/146) (from [#66](https://github.com/Keazra/terra-sprites/issues/66)) |
| A harsher world | 29 | [#147](https://github.com/Keazra/terra-sprites/issues/147) (from [#122](https://github.com/Keazra/terra-sprites/issues/122)) |
| M2 acceptance | 30 | [#148](https://github.com/Keazra/terra-sprites/issues/148) |

### 1.2 Not in M2

- Parenting: caring for, correcting and learning about the young ([#67](https://github.com/Keazra/terra-sprites/issues/67)). Learning by watching already gives the young guidance.
- Dead bodies that stay in the world ([#85](https://github.com/Keazra/terra-sprites/issues/85)).
- Everything M1 left for M3 and later (M1 design §1.1, §1.3): critters, weather, language, the tile front end, mods.
- The other parked design ideas stay `hitl` issues until a milestone takes them.

### 1.3 Alongside M2

A clean-up thread, separate from the slices, checks the outside review's claims ([#137](https://github.com/Keazra/terra-sprites/issues/137)) against the code and fixes what holds up, with the small notes from the owner's M1 play-through that an agent can do alone (#124, #126, #128, #129, #130).

---

## 2. Time scale

The owner's times, as playtime at 1× (1.25 ticks a second, M1 design §6.6):

| Moment | Playtime at 1× | Ticks | At 4× |
|---|---|---|---|
| Grows up | about 1 hour | about 4,500 | about 15 minutes |
| Grows old | about 3 hours | about 13,500 | about 45 minutes |
| Dies of old age | about 4 hours | about 18,000 | about 1 hour |

- **These are the starter genome's times.** The genome brings them about (§3), so they vary from sprite to sprite and can evolve.
- **Growing old and lifespan are two moments.** Growing old, at about 13,500 ticks, is the start of the old stage, when decline and wear's speeding of it begin (§3). **Lifespan** comes later: it's the age at which old age starts to injure a sprite (M1 design §4.8, §4.10). Old age kills within about 2,000 ticks after it, so the starter lifespan becomes about **16,000 ticks**, down from 60,000. The range lifespan may evolve within comes down in proportion, from 20,000–200,000 to about **6,000–60,000**. Slice 22 may recast lifespan as part of ageing (§3); the times above stay the aim.
- **A generation takes about 15–20 minutes at 4×,** the full game's top speed (Slice 27), so an evening sees several. Long runs of evolution belong to the fast-forward mode.
- **Lives get shorter in Slice 23, with breeding.** Until sprites have young, nothing replaces the dead: with 4-hour lives, the default world would be empty after about 4 hours of play. So lives stay at 60,000 ticks through Slice 22, and Slice 23 brings both changes at once.
- **Childhood ends at growing up.** M1's viability run counts ticks 0–10,000 as childhood because sprites had no stages (M1 design §7.4, A4). From Slice 22, childhood is a stage, and the harsher world's goal (§6, G5) is counted at growing up.

---

## 3. Ageing: the direction

The owner (round 2): the body stops growing at around 1 hour, and at around 3 hours "the genome begins to decay, amplified or damaged by stress throughout their lives, and the 'quality' of their genes". Rather than a timer, ageing comes from the genome and the life a sprite has had. Four readings of that were laid out (round 3 Q11):

| Reading | What it is | When |
|---|---|---|
| **A. Genes that switch on and off with age** | A growth gene works until about 1 hour; ageing genes switch on at about 3. The timing is in the genome, so it varies and can evolve, as in *Creatures*. A gene that switches on mid-life adds to the instinct pathway without touching what was learned (#49). | Slice 22 |
| **B. Wear** | Hunger, thirst, injury and fear leave lasting wear that healing doesn't undo. Once old, a sprite declines faster the more wear it carries. Real animals age faster under long-term stress. Wear is a fixed rule of the body (M1 design §4.1), so evolution can't breed it away. | Slice 22 |
| **C. Gene quality** | A sprite carrying more broken or harmful mutations, from copying errors or from close kin as parents, declines sooner, as inbred animals do. | Slice 24, once genes mutate |
| **D. The genome wearing out** | From old age, genes fail one at a time: an instinct fades, the senses dull, digestion slows. Closest to the owner's words, and hardest to tune: a sprite that loses the gene that makes it feel thirst dies of thirst without seeming to mind. | Slice 22's first question |

**The direction:** A sets when a sprite grows up and grows old; B sets how much a hard life speeds decline; C joins in Slice 24. Whether decline is the body wearing out (slower, duller senses, slower healing, then old-age harm) or genes failing one by one (D) is the first question of Slice 22's design round, tried in a prototype.

**A long life costs something.** Anything the genome controls about ageing needs a cost, or evolution breeds sprites that never grow old (M1 design §4.1). In M1, lifespan costs nothing, so with inheritance it would creep up to the most allowed. Real animals pay for long life: repair takes energy, so the long-lived spend more on upkeep and breed more slowly. **A later old age costs a little energy every tick,** as a bigger sense radius does (M1 design §4.8); Slice 22 sets how much.

---

## 4. The slices

Each slice is a GitHub issue and its own PR, worked as in the `handover` skill: its open questions are settled first, in plain words, and recorded in a new revision of this document and in `CONTEXT.md`. Each issue lists its slice's questions. Numbering carries on from M1's last, slice 19, so every slice number stays unique.

| Slice | What the player sees | Needs first |
|---|---|---|
| **20. Memory of places** | A thirsty sprite heads back to water it found earlier, even out of sight | — |
| **21. Title screen** | A screen at launch, before any world; flags that make or load a world skip it. Opens with a design pass: mock-ups to pick from, then a rough prototype played in Windows Terminal | — |
| **22. Life stages and ageing** | The young look and act young; sprites grow old and decline (§3) | — |
| **23. Breeding** | Adults have young, alone or with a partner as their genes decide; only between adults (§5); the 4-hour life (§2) | 22 |
| **24. Mixing and mutating genes** | Children differ from their parents; gene quality joins ageing | 23 |
| **25. Family tree** | A sprite's parents, grandparents and children, living or dead | 23 |
| **26. Population graph** | The population over time | 23 |
| **27. Fast-forward and the speed cap** | A world run for many generations with no screen; 4× in the full game, faster in a Lab mode | 23 (and 21, if Lab mode is chosen there) |
| **28. Learning by watching** | The young copy what adults do, their parents most | 22, 25 |
| **29. A harsher world** | About 20% of unattended newborns grow up | 20, 28 |
| **30. M2 acceptance** | The criteria of §6, measured and played | all |

- **20, 21 and 22 can start at once.** 21 touches only the front end. 20 and 22 both touch the brain and the body, so running them side by side means merging carefully.
- **24, 25, 26 and 27 can run side by side** once 23 is in.

---

## 5. Content safety: mating only between adults

The owner's one content-safety rule ([#69](https://github.com/Keazra/terra-sprites/issues/69), `docs/agents/how-we-work.md`): **attraction and mating attempts towards a sprite that isn't an adult are impossible,** for every genome and every data pack. Data can be edited and genes evolve, so the engine enforces it, the one exception to "the engine enforces physics only".

Slice 23 builds it:
- `Mate` is never offered unless both sprites are adults. It's a hard check in verb availability, not a learned or genetic lean.
- Attraction, whatever form it takes, never targets a non-adult.
- `check_invariants` checks it every tick, and property tests with random genomes pin it.
- Genes that switch on at adulthood (§3, reading A) are the natural home for mating instincts; the engine's check holds regardless.

---

## 6. Acceptance

M2 is done when these hold, as well as M1's A1–A9. Each slice sets its criterion's numbers before measuring, as M1 did.

| # | Criterion | Slice |
|---|---|---|
| G1 | **Generations.** Left alone, the sprites keep their population going for several generations in most seeds: it doesn't die out, and it doesn't grow past the world's limit. | 23 |
| G2 | **Family.** Any sprite's parents, grandparents and children can be seen, living or dead. | 25 |
| G3 | **Population.** A graph shows the population rising and falling over time. | 26 |
| G4 | **Evolution.** After enough generations, the species has measurably changed from the starter genome, beyond what noise explains. | 24 |
| G5 | **A harsh childhood.** About 20% of unattended newborns grow up, and grown sprites rarely die of hunger or thirst. | 29 |
| G6 | **Fast-forward.** A world can run for many generations with no screen, and be opened afterwards. | 27 |
| G7 | **Content safety.** A child is never a mating target, for any genome or data pack (§5). | 23 |

Slice 30 gathers the evidence in `docs/acceptance/m2.md`, as `docs/acceptance/m1.md` did for M1, with a play-through checklist for the owner. Slices without a row of their own are checked there: the title screen (Slice 21) and how each slice feels in play, as M1's A8 checked the game in Windows Terminal. Memory of places (Slice 20) shows in G5, which can't be met without it.
