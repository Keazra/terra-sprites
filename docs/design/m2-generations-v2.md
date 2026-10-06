# Terra Sprites — M2 "Generations" design (v2)

- **Status:** Draft: the plan, with what each slice settles added in a new revision.
- **Date:** 2026-10-06
- **Supersedes:** [v1](archive/m2-generations-v1.md), whose table holds the planning session's decisions
- **Builds on:** [M1 design v39](m1-a-sprite-lives-v39.md), which stays the rulebook for everything M2 doesn't change
- **Covers:** what milestone 2 contains, its slices and their order, the time scale of a sprite's life, the direction for ageing, what "M2 is done" means, and memory of places (Slice 20)

---

## Changes from v1

Slice 20 (memory of places, [#138](https://github.com/Keazra/terra-sprites/issues/138)). Its questions went to the owner in the project's shared files (`slice-20/questions.md`), each with a recommendation; silence accepts a recommendation, and the owner raised no objection.

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A sprite remembers places where something that stays put eased a need:** water, and objects fixed in place, such as bushes. Not loose things (berries, balls), sprites or the Cursor, and not places where it was hurt. | Slice 20 Q1 | §7.1 |
| 2 | **Up to 8 places, at most 3 of one kind, merged within 5 tiles, fading:** relief there remembers a place fully; it fades by the new `place_fade` brain gene, by half in about 3,500 ticks by default; the faintest makes room for a new one; one seen gone is forgotten. The limit of 3 a kind came from the owner's play-through, where sprites remembered only water. | Q2; the owner, 2026-10-06 | §7.2 |
| 3 | **A remembered place competes as a thing at the edge of sight,** pulling by its worth times how well it's remembered, and only while nothing of its kind is in sight. | Q3 | §7.3 |
| 4 | **The sprite knows the way:** setting off plans a path all the way there, and the trip has as long as the walk takes on top of the timeout. | Q4 | §7.4 |
| 5 | **The player sees places in the Brain tab and the doing line,** not on the map. | Q5 | §7.5 |
| 6 | **Learning by watching adds nothing to places yet.** | Q6 | §7.6 |
| 7 | **The bar:** the behaviour test, and no worse than `main` on A1–A4; the scarce-water runs are for information. | Q7 | §7.7 |

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

---

## 7. Memory of places (Slice 20)

Until Slice 20 a sprite only ever went for what it could see, about 10 tiles round it (M1 design §3.6). Water out of sight was forgotten, which is why scarce water killed grown sprites in Slice 17's measurements. A sprite now remembers where it found things, and a thirsty sprite heads back to the lake it drank from earlier. A remembered place is world state: it's saved, it's part of the state hash, and the self-check checks it every tick (M1 design §2.3, §7.1).

### 7.1 What a sprite remembers

- **A place where something it touched eased a need.** When learning (M1 design §5.6, step 4) credits relief of any need to the thing the sprite touched within `touch_window`, and that thing stays put, the sprite remembers the thing and its tile.
- **Things that stay put** are water tiles and objects fixed in place: each object type's entry already says whether an object stands on its own tile (M1 design §3.5.1), so a new fixed thing that eases a need, such as a spring or a fruit tree, is remembered with no code. Loose things (berries, balls) are not remembered: they get eaten or roll away. Nor are sprites or the Cursor, which walk off.
- **It's learned, not instinct.** A newborn remembers nothing.
- **Not places where it got hurt.** Harm only comes from touching something the sprite can see, so knowing where danger lies out of sight changes nothing yet; critters (M3) are when it would matter.

### 7.2 How many, and how they fade

- **Up to `remembered_places.held` places, 8 in the built-in physiology, and at most `per_kind` of one object type, 3.** Places of the same object type within `remembered_places.merge` tiles of each other (5, counted as tiles of Chebyshev distance) are one place, so a pond is one place, not fifty water tiles: relief at a new tile of it moves the place there.
- **Why a limit a kind.** A big lake's shores are many places 5 tiles apart, and a sprite drinks far more often than it eats. Without the limit, water filled about 7 of a sprite's 8 places by tick 20,000 in a default world (seed 3), and pushed out the bushes it had eaten from: the owner saw sprites remember only water. With it, 26 of 30 sprites remember a bush by tick 10,000, against 18 of 30 by tick 20,000 without it.
- **Relief there remembers a place fully,** strength 1, whether it's new or a visit tops it up.
- **It fades each tick** by the `place_fade` brain parameter: its strength is multiplied by 1 − `place_fade`. The default, 0.0002, halves it in about 3,500 ticks, about 45 minutes at 1×, as worth for a need fades (M1 design §5.6). Its range is 0 to 0.01, so evolution can tune it; a long memory has a natural cost in trips to places that have changed.
- **Forgotten** once it fades below `forget_below` (0.01), as a remembered sprite is; or when the sprite needs room, the faintest goes first, the oldest of equals: the faintest of its kind when it holds `per_kind` of that kind already, else the faintest of all; or **at once when the sprite sees it gone**: the place is within its sense radius and the thing there no longer exists, such as a bush that died.
- A saved world from before Slice 20 loads with no places, and its brains get `place_fade` from their genome as the current pack expresses it (saves carry their own pack, M1 design §2.8, so the parameter's range and default fill in for a pack that doesn't name it).

### 7.3 How a remembered place competes

- **A remembered place is weighed like a thing at the edge of sight:** its category offers it among what's in sight, at a distance of 1, the flood's edge, so it gets none of the pull that being close gives (M1 design §5.3). Its worth, in the draw and in attention, is multiplied by its strength, the place's **recall**: draw = value_gain × worth × recall, plus curiosity and fear as for any thing.
- **Hunger and thirst pull through that worth,** as they do for things in sight: water learned to be good for thirst is worth a lot to a thirsty sprite and nothing to one that isn't. So a thirsty sprite's eye turns to the lake it remembers, and a content one doesn't give it a thought.
- **Only while none is in sight.** A place isn't offered while anything of its object type is in the sprite's flood, so a sprite that can see water drinks there rather than walking to the lake it remembers. Nor is one whose thing is gone.
- Then it chooses what to do as with anything it attends to: the starter genome's thirst leads to drinking, so thirsty, no water in sight and a lake remembered, it goes to drink at the lake. Every verb that goes to its target is offered for a remembered place; **Retreat is not,** since backing away from something out of sight means nothing.

### 7.4 Trips: how it finds its way back

An action aimed at a remembered place out of sight is a **trip**.

- **It knows the way.** Setting off, it searches a flood out to the place's distance plus its sense radius, which reaches the place wherever the walk winds, weighing tiles with sprites on as its flood does (M1 design §3.6). It keeps to that path, as to a committed path (§3.7). With no way there, such as a bush grown across the only path, it forgets the place and doesn't set off: it carries on with what it was doing, and chooses again at its next step 5.
- **Time to get there:** a trip has the timeout (60 ticks) plus as long as the walk takes at its speed, so a long trip doesn't give up halfway. That's set when it sets off: finding the way again on the way adds no time, so a trip held up again and again still gives up.
- **On the way,** when a sprite blocks the path, it plans the way again from where it stands, round sprites, at once rather than after `replan_after` ticks as a blocked walk does (M1 design §3.7), since its whole way is planned already; with no way at all, the trip ends blocked and the place is kept, since a sprite in the way soon moves.
- **Once the place is in reach of its flood,** the trip becomes an ordinary action, as if it had seen the thing all along: it drinks or eats when it gets there, and the drink teaches what drinks always teach, topping the place up.
- What it attends to can still change its mind mid-trip, by the usual margin (M1 design §5.3); while out of sight, the trip is weighed by how well its place is remembered.

### 7.5 What the player sees

- **The Brain tab** lists the places under MEMORY, best remembered first, as PLACES: what, how far and which way, and how well remembered:

  ```
   MEMORY
     water is good for thirst            +.61
   PLACES
     water · 34 tiles NE                  .92
     berry bush · 12 tiles W              .40
  ```
- **The doing line** says a trip is from memory: `Going to drink at the water it remembers · 34 tiles to go`, `Going to eat the berry bush it remembers · 12 tiles to go`. The detail view adds `· from memory`: `DRINK → water (40,12) · from memory · walking (34 tiles)`.
- **The map** shows where it's heading with the flashing destination mark, as for any walk. A key that marks the selected sprite's places on the map was left as a possible small issue of its own.

### 7.6 Learning by watching

Nothing extra in Slice 20. A child that follows its parent to water and drinks there remembers the place itself, by §7.1. Slice 28 decides whether watching another sprite drink teaches the place too.

### 7.7 How we know it worked

Set before measuring:
- **The issue's test:** a sprite that found water and wandered away goes back to it when thirsty, from out of sight (`crates/terra-sim/tests/remembered_places.rs`).
- **No worse than `main`** on A1–A4, allowing 10% for noise.
- **For information, no bar:** Slice 17's scarce-water runs again, water on 8% and 4% of the map.

**Measured (2026-10-06),** in release builds of `main` and of this slice, on seeds 1–10 with the lab (`scenarios/*.ron --seeds 10`):

| | `main` | Slice 20 |
|---|---|---|
| A1–A3 lab runs | — | the same, line for line: in these small worlds the water and the bushes are always in sight, so no place is ever offered |
| A4: median alive at tick 10,000 | 100% | 100% |
| A4: hunger and thirst deaths, ticks 10,000–50,000, all seeds | 3 | 2 |
| A5: ticks a second, 100 sprites | 324 | 309, met |

Scarce water, for information, measured before the limit of 3 a kind: the default world with its water bands scaled down (deep 5/8 of the water, shallow 3/8, then 8% sand), with a scratch build, nothing committed. 300 sprites over the 10 seeds, 50,000 ticks:

| Water | Deaths by tick 10,000, `main` → Slice 20 | Grown-up deaths, ticks 10,000–50,000 | Of them, hunger or thirst |
|---|---|---|---|
| 8% of tiles | 30 → 23 | 89 → 3 | all |
| 4% of tiles | 92 → 55 | 139 → 1 | all |

**What it shows.** A sprite that survives childhood now fends for itself, as the owner wants of a grown sprite: with water scarce, grown-up deaths fall from 89 and 139 to 3 and 1. Childhood is still the hard part, since a newborn remembers nothing until it first finds water. That's what Slice 29, the harsher world, builds on (§6, G5).
