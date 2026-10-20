# Terra Sprites — M2 "Generations" design

- **Status:** Draft: the plan, with what each slice settles added as it's settled.
- **Date:** 2026-10-12
- **History:** edited in place since v5; each change is a file in [`changes/`](changes/), whose README says where v1 to v5 are
- **Builds on:** [M1 design](m1-a-sprite-lives.md), which stays the rulebook for everything M2 doesn't change
- **Covers:** what milestone 2 contains, its slices and their order, the time scale of a sprite's life, the direction for ageing, what "M2 is done" means, memory of places (Slice 20), the title screen (Slice 21), where places sit on the Brain tab, life stages and ageing (Slice 22), breeding (Slice 23), the view past the wall (Slice 31), mixing and mutating genes (Slice 24), with copy and change (24b), the population graph (Slice 26), courtship and families (Slice 32), fast-forward and the speed cap (Slice 27), with gene warnings (#274), the family tree (Slice 25), learning by watching (Slice 28), and a harsher world (Slice 29)

---

## 0. Vision

M1 made a sprite live: it feels, learns and dies. M2 makes **generations**. Sprites grow up, grow old and have young; children take after their parents but not exactly; and over an evening the player watches a family line, and a species, change.

The owner's principles carry over unchanged (`docs/agents/how-we-work.md`). Three matter most here:
- **The player has to care first.** A life long enough to get attached to is why a sprite lives 4 to 8 hours at 1×, about 6 on average (§2).
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
| Courtship and families | 32 | [#186](https://github.com/Keazra/terra-sprites/issues/186) |

### 1.2 Not in M2

- Parenting: caring for, correcting and learning about the young ([#67](https://github.com/Keazra/terra-sprites/issues/67)). Learning by watching already gives the young guidance.
- Dead bodies that stay in the world ([#85](https://github.com/Keazra/terra-sprites/issues/85)).
- Everything M1 left for M3 and later (M1 design §1.1, §1.3): critters, weather, language, the tile front end, mods.
- The other parked design ideas stay `hitl` issues until a milestone takes them.

### 1.3 Alongside M2

A clean-up thread, separate from the slices, checks the outside review's claims ([#137](https://github.com/Keazra/terra-sprites/issues/137)) against the code and fixes what holds up, with the small notes from the owner's M1 play-through that an agent can do alone (#124, #126, #128, #129, #130).

**The view past the wall** (Slice 31, [#184](https://github.com/Keazra/terra-sprites/issues/184), from [#180](https://github.com/Keazra/terra-sprites/issues/180)) lets the view go past the terrarium's wall into space, with stars going by (§11). It touches only the front end, so it can be built beside any slice. It was designed with the ship ([#181](https://github.com/Keazra/terra-sprites/issues/181)), which is a milestone of its own after Body & verbs: M4, Terra ([its design](m4-terra.md)).

---

## 2. Time scale

The owner's times, as playtime at 1× (1.25 ticks a second, M1 design §6.6):

| Moment | Playtime at 1× | Ticks | At 4× |
|---|---|---|---|
| Grows up | about 1 hour | about 4,500 | about 15 minutes |
| Grows old | about 5 hours (3 to 7) | about 22,500 | about 1¼ hours |
| Dies of old age | about 6 hours (4 to 8) | about 27,000 | about 1½ hours |

- **These are the starter genome's times.** The genome brings them about (§3), so they vary from sprite to sprite and can evolve: a later old age costs nothing, so lives may grow longer over the generations, up to old age at about 13 hours (§12.5).
- **Lives differ by up to 2 hours either way.** The owner (2026-10-12): "4 hours should be the shortest, and eight at the longest, for an average of 6 hours." A new sprite's `grows_old` varies by up to 40% at spawn where its other genes vary by 10% (§9.3), and a hatchling takes one parent's, now and then changed in copying (§12). Old age still lasts about an hour, and a hard life can still bring it sooner (§9.4).
- **Growing old is where M1's lifespan went** (#140). The `grows_old` gene sets when a sprite grows old, and its genes then weaken until its body gives out and old age kills it (§9.4). How long that takes depends on the life it has had, not on a second gene. A save or genome that says `lifespan` reads it as `grows_old`, whose range is about **6,000–60,000**.
- **A generation takes about 15–20 minutes at 4×,** Story mode's top speed (§16.2), so an evening sees several. Long runs of evolution belong to fast-forward (§16.3).
- **Lives got shorter in Slice 23, with breeding.** Until sprites had young, nothing replaced the dead: with 4-hour lives, the default world would have been empty after about 4 hours of play. So in Slice 22 sprites grew up and grew old at 1 and 3 hours, but old age lasted about 10 hours, and Slice 23 shortened it to about 1 (§9.4), for lives of about 4 hours. Later in Slice 23 the owner set them at 4 to 8 (2026-10-12), by growing old later.
- **Childhood ends at growing up.** M1's viability run counts ticks 0–10,000 as childhood because sprites had no stages (M1 design §7.4, A4). From Slice 22, childhood is a stage, and the harsher world's goal (§6, G5) is counted at growing up: from Slice 29, A4 counts who grows up and the deaths by life stage, and "alive at tick 10,000" is gone (§19.6).

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

**Settled in Slice 22 (#140): A, B and D.** Genes switch with life stage (§9.2), and old age is the genome wearing out: each gene has a health that weakens its effect, damage lands on random genes once a sprite is old, and a hard life brings that sooner and speeds it up (§9.4). Rather than failing outright, genes weaken a little at a time, so a sprite slows and dulls as it ages.

**A long life costs something,** this section said: anything the genome controls about ageing needs a cost, or evolution breeds sprites that never grow old (M1 design §4.1), and real animals pay for long life in upkeep. Slice 22 built the cost, a little energy every tick for a later old age (§9.3), and held it at 0.

**Settled in Slice 24 (#142): no cost.** The owner (2026-10-13) prefers sprites to die as the body and brain fail, not by an energy charge, and lives may grow long (§12.5). C joins as hidden flaws: broken genes bring old age sooner and speed its decay, as wear does (§12.4).

---

## 4. The slices

Each slice is a GitHub issue and its own PR, worked as in `docs/agents/how-we-work.md`: its open questions are settled first, in plain words, and recorded in this document, in a file in [`changes/`](changes/) and in `CONTEXT.md`. Each issue lists its slice's questions. Numbering carries on from M1's last, slice 19, so every slice number stays unique.

| Slice | What the player sees | Needs first |
|---|---|---|
| **20. Memory of places** | A thirsty sprite heads back to water it found earlier, even out of sight | — |
| **21. Title screen** | A screen at launch, before any world; flags that make or load a world skip it. Opens with a design pass: mock-ups to pick from, then a rough prototype played in Windows Terminal | — |
| **22. Life stages and ageing** | The young look and act young; sprites grow old and decline (§3) | — |
| **23. Breeding** | Adults have young, alone or with a partner as their genes decide; only between adults (§5); lives of 4 to 8 hours (§2) | 22 |
| **24. Mixing and mutating genes** | Children differ from their parents; gene quality joins ageing; now and then a gene is doubled, lost or changed in what it does | 23 |
| **25. Family tree** | A sprite's parents, grandparents and children, living or dead | 23 |
| **26. Population graph** | The population over time | 23 |
| **27. Fast-forward and the speed cap** | A world run for many generations with no screen; 4× in Story mode, faster in Lab mode (§16) | 21, 23 |
| **28. Learning by watching** | The young copy what adults do, their parents most | 22, 25 |
| **29. A harsher world** | 10% to 30% of unattended newborns grow up, and most young raised among sprites do (§19) | 20, 28 |
| **30. M2 acceptance** | The criteria of §6, measured and played | all |
| **31. The view past the wall** | The view scrolls past the glass into space, with stars going by (§11) | — |
| **32. Courtship and families** | Sprites call for a mate, court before mating, and stay together as partners; parents keep near their eggs and young; lone eggs are very rare (§15) | 23, 24 |

- **20, 21 and 22 can start at once.** 21 touches only the front end. 20 and 22 both touch the brain and the body, so running them side by side means merging carefully.
- **24, 25, 26 and 27 can run side by side** once 23 is in.
- **32 comes before 28 and 29,** which lean on parents being near their young: the young copy their parents most, and guidance is how newborns survive a harsher world.

---

## 5. Content safety: mating only between adults

The owner's one content-safety rule ([#69](https://github.com/Keazra/terra-sprites/issues/69), `docs/agents/how-we-work.md`): **attraction and mating attempts towards a sprite that isn't an adult are impossible,** for every genome and every data pack. Data can be edited and genes evolve, so the engine enforces it, the one exception to "the engine enforces physics only".

Slice 23 builds it:
- `Mate` is never offered unless both sprites are adults (an old sprite counts as one, §9.1). It's a hard check in verb availability, not a learned or genetic lean. Nor is `BreedAlone`, unless the sprite is grown (§10.3). Nor is `Court`, a dance before mating, unless both are adults (Slice 32, §15.3).
- Attraction, whatever form it takes, never targets a non-adult. **The breeding drive works only between adults** (Slice 23): `physiology.ron` names it (`breeding.drive`, the starter's `arousal`), and whatever the genes, it reads 0 in a sprite not grown up, and in an adult's choices about one: it draws no eye to it, moves the sprite to do nothing to it, and its body language shows none, to other sprites or on screen: the engine's views read it as 0 in a young sprite. Its fall teaches worth only after a Mate, and only about that partner, never sprites in general or new things (§10.2).
- **Courtship and calls are attraction too** (Slice 32): a heard call from a sprite not grown up counts as a sprite not grown up, so the breeding drive draws no eye to it and moves the sprite to nothing about it. Caring for the young is a drive of its own, anxiety (§15.6), and an attention instinct about whom a sprite is to it that reads the breeding drive draws no eye to one not grown up, its own young included (Slice 32). A genome can still make anxiety from arousal through its chemistry, as it can any chemical, but courting and mating stay between adults whatever moves a sprite.
- `check_invariants` checks it every tick, and property tests with random genomes pin it.
- Genes that switch on at adulthood (§3, reading A) are the natural home for mating instincts; the engine's check holds regardless.

---

## 6. Acceptance

M2 is done when these hold, as well as M1's A1–A9. Each slice sets its criterion's numbers before measuring, as M1 did.

| # | Criterion | Slice |
|---|---|---|
| G1 | **Generations.** Left alone, the sprites keep their population going for several generations: it doesn't die out, and it doesn't grow past the world's limit. Measured by the long run (§19.6), the default world left alone for 220 hours (990,000 ticks) on seeds 1–10 (#222): a seed lasts when at the end it still has sprites or eggs, it never had fewer than 10 sprites (eggs not counted) at any count of its population record, and its sprites over its last 50 hours averaged at least half of what they averaged over hours 50–100. At least 9 of 10 seeds must last. Slice 23's 22-hour check (§10.8) stays a quick check while a slice is built. | 23; its bar #222's, measured from 29 |
| G2 | **Family.** Any sprite's parents, grandparents and children can be seen, living or dead, on the family tree (§17). | 25 |
| G3 | **Population.** A graph shows the population rising and falling over time (§14). | 26 |
| G4 | **Evolution.** After enough generations, the species has measurably changed from the starter genome, beyond what noise explains: at least 3 gene numbers moved a tenth the same way in every seed that lasts the long run (§19.6, #226), where Slice 24 asked for 10 seeds of 200,000 ticks (§12.7). | 24; not met there, so measured again with 27 and 29 |
| G5 | **A harsh childhood.** 10% to 30% of unattended newborns grow up, in the middle seed of ten; at least 60% of the young hatched among sprites grow up; and adults dying of hunger or thirst are under 1 in 20 of grown-up deaths (§19.6). | 29 |
| G6 | **Fast-forward.** A world can run for many generations with no screen, and be opened afterwards (§16). | 27 |
| G7 | **Content safety.** A child is never a mating target, for any genome or data pack (§5). | 23 |

Slice 30 gathers the evidence in `docs/acceptance/m2.md`, as `docs/acceptance/m1.md` did for M1, with a play-through checklist for the owner. Slices without a row of their own are checked there: the title screen (Slice 21) and how each slice feels in play, as M1's A8 checked the game in Windows Terminal. Memory of places (Slice 20) shows in G5, which can't be met without it.

---

## 7. Memory of places (Slice 20)

Until Slice 20 a sprite only ever went for what it could see, about 10 tiles round it (M1 design §3.6). Water out of sight was forgotten, which is why scarce water killed grown sprites in Slice 17's measurements. A sprite now remembers where it found things, and a thirsty sprite heads back to the lake it drank from earlier. A remembered place is world state: it's saved, it's part of the state hash, and the self-check checks it every tick (M1 design §2.3, §7.1).

### 7.1 What a sprite remembers

- **A place where something it touched eased a need.** When learning (M1 design §5.6, step 4) credits relief of any need to the thing the sprite touched within `touch_window`, and that thing stays put, the sprite remembers the thing and its tile.
- **Things that stay put** are water tiles and objects fixed in place: each object type's entry already says whether an object is a fixture, attached to the ground (its `Fixture` tag, M1 design §3.5.1), so a new fixed thing that eases a need, such as a spring or a fruit tree, is remembered with no code. Loose things (berries, balls) are not remembered: they get eaten or roll away. Nor are sprites or the Cursor, which walk off.
- **It's learned, not instinct.** A newborn remembers nothing.
- **Not places where it got hurt.** Harm only comes from touching something the sprite can see, so knowing where danger lies out of sight changes nothing yet; critters (M5) are when it would matter.

### 7.2 How many, and how they fade

- **Up to `remembered_places.held` places, 8 in the built-in physiology, and at most `per_kind` of one object type, 3.** Places of the same object type within `remembered_places.merge` tiles of each other (5, counted as tiles of Chebyshev distance) are one place, so a pond is one place, not fifty water tiles: relief at a tile of it moves the place there, and takes in every other place of its kind within 5 tiles of that tile, so no two places of a kind are ever within 5 tiles of each other.
- **Why a limit a kind.** A big lake's shores are many places 5 tiles apart, and a sprite drinks far more often than it eats. Without the limit, water filled about 7 of a sprite's 8 places by tick 20,000 in a default world (seed 3), and pushed out the bushes it had eaten from: the owner saw sprites remember only water. With it, 26 of 30 sprites remember a bush by tick 10,000, against 18 of 30 by tick 20,000 without it.
- **Relief there remembers a place fully,** its **recall**, how well it's remembered, at 1, whether it's new or a visit tops it up.
- **It fades each tick** by the `place_fade` brain parameter: its recall is multiplied by 1 − `place_fade`. The default, 0.0002, halves it in about 3,500 ticks, about 45 minutes at 1×, as worth for a need did until lessons were made to fade slowly (M1 design §5.6, 2026-10-12). A place can change under the sprite, as a bush dies, so it still fades this fast. Its range is 0 to 0.01, so evolution can tune it; a long memory has a natural cost in trips to places that have changed.
- **Forgotten** once it fades below `forget_below` (0.01), as a remembered sprite is; or when the sprite needs room, the faintest goes first, the oldest of equals: the faintest of its kind when it holds `per_kind` of that kind already, else the faintest of all; or **at once when the sprite sees it gone**: the place is within its sense radius and the thing there no longer exists, such as a bush that died.
- A saved world from before Slice 20 loads with no places, and its brains get `place_fade` from their genome as the current pack expresses it (saves carry their own pack, M1 design §2.8, so the parameter's range and default fill in for a pack that doesn't name it). One saved while this slice was being built, before the limit of 3 a kind and the merge rule as above, loads with its places brought within them: of two of a kind too near, the older goes, then the faintest of a kind past 3, then the faintest past 8.

### 7.3 How a remembered place competes

- **A remembered place is weighed like a thing at the edge of sight:** its category offers it among what's in sight, at a distance of 1, the flood's edge, so it gets none of the pull that being close gives (M1 design §5.3). Its worth, in the draw and in attention, is multiplied by the place's recall: draw = value_gain × worth × recall, plus curiosity and fear as for any thing.
- **Hunger and thirst pull through that worth,** as they do for things in sight: water learned to be good for thirst is worth a lot to a thirsty sprite and nothing to one that isn't. So a thirsty sprite's eye turns to the lake it remembers, and a content one doesn't give it a thought.
- **Only while none is in sight.** A place isn't offered while anything of its object type is in the sprite's flood, so a sprite that can see water drinks there rather than walking to the lake it remembers. Nor is one whose thing is gone.
- Then it chooses what to do as with anything it attends to: the starter genome's thirst leads to drinking, so thirsty, no water in sight and a lake remembered, it goes to drink at the lake. Every verb that goes to its target is offered for a remembered place; **Retreat is not,** since backing away from something out of sight means nothing.

### 7.4 Trips: how it finds its way back

An action aimed at a remembered place out of sight is a **trip**.

- **It knows the way.** Setting off, it searches a flood out to the place's distance plus its sense radius, which reaches the place wherever the walk winds, weighing tiles with sprites on as its flood does (M1 design §3.6). It keeps to that path, as to a committed path (M1 design §3.7). With no way there, such as a bush grown across the only path, it forgets the place and doesn't set off: it carries on with what it was doing, its eye where it was, decides nothing that tick, and chooses again at its next step 5. A running action it changed its mind about ends only once the new one is sure to start, so a place it can't get to never interrupts anything.
- **Time to get there:** a trip has the timeout (60 ticks) plus as long as the walk takes at its speed, so a long trip doesn't give up halfway. That's set when it sets off: finding the way again on the way adds no time, so a trip held up again and again still gives up.
- **On the way,** when a sprite blocks the path, it plans the way again from where it stands, round sprites, at once rather than after `replan_after` ticks as a blocked walk does (M1 design §3.7), since its whole way is planned already; with no way at all, the trip ends blocked and the place is kept, since a sprite in the way soon moves.
- **Once the place is in reach of its flood,** the trip becomes an ordinary action, as if it had seen the thing all along: it drinks or eats when it gets there, and the drink teaches what drinks always teach, topping the place up.
- What it attends to can still change its mind mid-trip, by the usual margin (M1 design §5.3); while out of sight, the trip is weighed by its place's recall, or that of the place of its kind within 5 tiles that took it in, so a trip keeps its pull when its place moves to another tile of the same lake.

### 7.5 What the player sees

- **The Brain tab** lists the places in their own list, below the grouped memory (M1 design §6.1), best remembered first, as PLACES. A line is unchanged: what, how far and which way, and how well remembered, as the number alone. Places are not folded under the thing.

  ```
   WATER
     good for thirst                     +.61
   PLACES
     water · 34 tiles NE                  .92
     berry bush · 12 tiles W              .40
  ```
- **The doing line** says a trip is from memory: `Going to drink at the water it remembers · 34 tiles to go`, `Going to eat the berry bush it remembers · 12 tiles to go`. The detail view adds `· from memory`: `DRINK → water (40,12) · from memory · walking (34 tiles)`.
- **The map** shows where it's heading with the flashing destination mark, as for any walk. A key that marks the selected sprite's places on the map was left as a possible small issue of its own.

### 7.6 Learning by watching

Nothing extra in Slice 20. A child that follows its parent to water and drinks there remembers the place itself, by §7.1. Since Slice 28, a sprite that watches another eased at a place remembers it too, as if eased there itself (§18.2).

### 7.7 How we know it worked

Set before measuring:
- **The issue's test:** a sprite that found water and wandered away goes back to it when thirsty, from out of sight (`crates/terra-sim/tests/remembered_places.rs`).
- **No worse than `main`** on A1–A4, allowing 10% for noise.
- **For information, no bar:** Slice 17's scarce-water runs again, water on 8% and 4% of the map.

**Measured (2026-10-06),** on the slice's final code, in release builds of `main` and of this slice, on seeds 1–10 with the lab (`scenarios/*.ron --seeds 10`):

| | `main` | Slice 20 |
|---|---|---|
| A1–A3 lab runs | — | the same, line for line: in these small worlds the water and the bushes are always in sight, so no place is ever offered |
| A4: median alive at tick 10,000 | 100% | 100% |
| A4: hunger and thirst deaths, ticks 10,000–50,000, all seeds | 3 | 2 |
| A5: ticks a second, 100 sprites, median of 3 runs | 337 | 327, met |

Scarce water, for information: the default world with its water bands scaled down (deep 5/8 of the water, shallow 3/8, then 8% sand). The generator's bands are fixed in code, so this was a scratch build of each, not committed; a world config that sets them is the harsher world's to add (Slice 29). 300 sprites over the 10 seeds, 50,000 ticks:

| Water | Deaths by tick 10,000, `main` → Slice 20 | Grown-up deaths, ticks 10,000–50,000 | Of them, hunger or thirst | Ticks a second, 100 sprites, median of 3 |
|---|---|---|---|---|
| 8% of tiles | 30 → 23 | 89 → 2 | all | — |
| 4% of tiles | 92 → 55 | 139 → 1 | all | 371 → 325 |

Where water is scarce, trips are common, and their way-finding costs about 12% of a tick; in the default world, where they're rare, about 3%. Both are far above A5's 200.

**What it shows.** A sprite that survives childhood now fends for itself, as the owner wants of a grown sprite: with water scarce, grown-up deaths fall from 89 and 139 to 2 and 1. Childhood is still the hard part, since a newborn remembers nothing until it first finds water. That's what Slice 29, the harsher world, builds on (§6, G5).

---

## 8. The title screen (Slice 21)

The screen the game opens on, before the player's world is running. It's front end only: the simulation gains one thing, a way to make a preset's world at another size (§8.1).

### 8.1 The opening scene

- **At launch the screen is dark.** The Cursor's frame of light (M1 design §0) fades in over a sprite above the middle of the screen. The world is revealed in a circle spreading out from the light, its edge drawn as `░`, until it fills the screen. Then the title fades in, and the menu appears under it and under the Cursor.
- **It takes about 5½ seconds,** tuned by feel: the light arrives by 1 s, the light fills the screen by 4 s, the title fades in from there and the menu shows at 5.5 s.
- **The sprite in the Cursor's frame stays still,** drawn as a sprite is (`☺`, or `@` in `--ascii`), for as long as the title screen is open, the scene and the menu alike (Fauxpaw, 2026-10-17). The light arrives on the tile nearest the middle of the rows above the title's boxes that a sprite could stand on and nothing is on, or at that middle when there's no such tile. No box covers the frame, however many choices, presets or saves it holds; only the help screen, over everything, does. It moves only when the terminal is resized so that it would be hidden, to where it would have arrived on that size.
- **That sprite is the scene's own,** drawn over the world as the Cursor is over a map, and the Cursor leads none of the world's sprites. A led sprite only follows the Cursor (M1 design §6.5), so one of the world's led for the whole title screen would die of thirst in about ten minutes (on seeds 3, 7 and 11 of a 100×30 title world, at ticks 3,048, 3,401 and 3,022). The world's sprites pass beneath it.
- **It plays at every launch,** and when the player comes back from a world (§8.4). Any key or click skips to its end and does nothing else, so a key pressed to skip never also picks from the menu.
- **The world** is a new one with a random seed, made from the default preset of the data pack in use (the built-in one, or `--data`'s, so a mod's species and items show), whatever preset `--preset` names, on a map the size of the terminal. The preset's densities scale to that map, as they do for any map (M1 design §3.9), and its first population stays, so the screen is lively. A map side stays within what a generated map may be: at least 32 tiles, so a 30-row terminal shows the top of a 32-row map.
- It **runs at 4× from launch,** so the light finds the sprites already about their lives, and goes on behind the menu for as long as the title screen is open, at most 5 ticks a frame (Fauxpaw, 2026-10-17). It's never saved, and nothing on the title screen touches it, the Cursor included.
- **What it is in the terrarium's story:** the player arriving at the glass, before reaching in. The dark is the terrarium without the player's light, and the light finding a sprite is the first touch. Nothing on screen says so.

### 8.2 The menu

A box in the middle of the screen, over the moving world, holds the title lettering and the menu. The choices are numbered and picked as the Place menu's are (M1 design §6.5): `↑` `↓` and `Enter`, the wheel, a number, or a click.

| Choice | What it does |
|---|---|
| **Continue** | Loads the newest save in the saves folder (any kind: the quicksave, an autosave or one by name), named beside it with how long ago it was saved: `autosave-1 · 2 hours ago`. Hidden when there's no save. |
| **New world** | Opens the New world box (§8.3). |
| **Load** | A list of every save, newest first, as `Ctrl+O` opens in a world (M1 design §6.7): picked by number, arrows, wheel or click, a click off it closes it, and with none it says where it looked, `No saves in …` (#148). |
| **Help** | The help overlay, as `?` opens in a world. |
| **Quit** | Closes the game. |

- **`Esc` asks "Quit? (y/n)",** as in a world; only `y` quits, and the pointer leaves the question waiting (#148). `Ctrl+C` quits at once.
- **A world started or loaded from the title screen starts as one does from the flags:** a new world at 1×, a loaded one paused (M1 design §6.7). The session log starts afresh from it (M1 design §2.7). A save that can't be read, or a preset that isn't valid, is refused on the status line and the title screen stays.
- A terminal under 100×30 shows "Terminal too small", as a world does (M1 design §6.1), and `Esc` still asks to quit under it. Like a world's, it takes no clicks, so the menu it hides can't be clicked, though its keys still work (#148).
- Everything drawn stays within CP437 (M1 design §6.2): a save's name with other characters shows them as `?`, as in a world's list (#148).

### 8.3 New world

A small box over the menu, with:
- **Seed:** a random one filled in. Typing a digit replaces it, then more digits add to it, as a save's name is typed (M1 design §6.7); `Backspace` rubs out, and `Tab` offers another random seed. A seed is a whole number from 0 to 18446744073709551615.
- **Preset:** `↑` `↓` pick one from a list: `default` first (the built-in preset, or `--data`'s own `presets/default.ron`), then each `.ron` file in the `presets` folder of the game's folder (`%APPDATA%\terra-sprites\presets` on Windows), by name. A preset is checked against the data pack in use when the world is made.
- **Mode:** Story or Lab, `←` `→` picking, Story unless changed; holding `?` explains the one picked. The world keeps it (§16.1; M4 design §11).
- `Enter` starts the world; `Esc` goes back to the menu.

### 8.4 Leaving a world

- **`Esc` from Select** (M1 design §6.5, §6.6) asks "Quit? y quit the game  t title screen  any other key stays". `y` quits, as before; `t` goes back to the title screen, whose scene plays again on a new world. Any other key, `Esc` included, cancels, but `F3`, which only shows or hides the debug overlay (M1 design §6.1, #311).
- **Both save first,** as quitting does (M1 design §6.7): an autosave, unless the world hasn't run since it was last saved. The session log is written too. If either can't be, the title screen says why on its status line, and quitting says why on the terminal once the game has closed.
- `Ctrl+C` still quits at once.

### 8.5 Flags

- **`--seed`, `--preset`, `--lab`, `--load` and `--replay` make or load a world,** so they skip the title screen and go straight in, as before (`--lab` and `--load` from §16). Their world can still leave for the title screen with `t`.
- **`--fast-forward` runs a world with no screen at all,** and never shows the title screen (§16.3).
- **`--data`, `--ascii` and `--theme`** change what the game's worlds are made of or how they're drawn, not which world it opens on, so the title screen still shows, drawn with them, and a new world made from it uses `--data`'s pack.

---

## 9. Life stages and ageing (Slice 22)

Settled with the owner on 2026-10-11 ([#140](https://github.com/Keazra/terra-sprites/issues/140)). A sprite grows up and grows old at ages its genes set, a gene can switch on or off as it does, and once it's old its genes weaken until its body gives out. The numbers are data, the `ageing` section of `physiology.ron`: first guesses, tuned in play and in the viability run with A4 holding.

### 9.1 Life stages

- **A sprite's life stage is young, adult or old.** Before that it's in an egg, an item and not yet a sprite (§10.4, #196). "Stage" alone is an object's (M1 design §3.5); a sprite's is its life stage. An old sprite still counts as an adult for §5's rule.
- **It's young until it grows up, an adult until it grows old, then old** (§9.3). One whose genes have it grow old before it grows up does both at once, and is never an adult.
- **The stage changes at the start of step 3** (M1 design §2.4), so genes that switch with it work from that tick's chemistry on.
- **A world's first sprites start as adults,** at ages drawn evenly through adulthood, with a newborn's body (M1 design §4.7), so a fresh world isn't all children. A sprite the player places is a newborn. A scenario's sprites start just grown up, unless it lists them as newborns.
- **An egg comes first:** since Slice 23, a sprite's life starts in an egg, which hatches into a young sprite (§10.4). Since #196 an egg is an item, so it has no life stage: a save or genome file that says `Egg`, from before, reads it as young.

### 9.2 Genes that switch with life stage

- **A gene can carry `from` and `until`,** a life stage each: it works from `from` (young if it has none) until `until` (never, if it has none). Outside them it's **dormant**: in the genome, but not working. A dormant gene sets nothing, so the next gene that sets the same value works in its place (M1 design §4.3), as the starter's two `watch_rate` genes do, one until adult and one from it (#148).
- **Traits and starting levels can't switch.** Traits work all life, since two of them set the stages, and a starting level applies only at birth. Such a gene that carries a switch is flagged and doesn't work (M1 design §4.3), as is one that switches off before it switches on.
- **When its stage changes, a sprite's genes are expressed afresh:** its chemistry, brain parameters, concepts and instinct links become those of the genes working now. What it has learned, what it attends to and what it remembers doing stay as they were. So an instinct that switches on mid-life joins the instinct pathway without touching what was learned (#49). Its last decision is let go, since the concepts it was made with may be gone, so on the tick it grows up or grows old it learns nothing by watching, and one watching it is shown nothing (§18.1, #148).
- **Genome files with switches are format 2;** format 1 still loads, and a format 1 file with a switch is refused.

### 9.3 When a sprite grows up and grows old

- **Two trait genes set the ages,** so each can evolve on its own: `grows_up` (1,500–13,500 ticks; the starter's 4,500 is about an hour at 1×) and `grows_old` (6,000–60,000; the starter's 22,500 is about 5 hours). A genome with no gene for one, as an M1 genome has none for `grows_up`, takes the middle of its range (M1 design §4.8), so grows up at 7,500 ticks (#148).
- **`grows_old` varies more at spawn** (2026-10-12): by up to 40%, physiology's `trait_variation`, where every other gene varies by `spawn_variation`'s 10% (M1 design §4.9). So the world's first sprites, and those the Cursor places from the starter genome, grow old at 3 to 7 hours and die of it at 4 to 8 (§2).
- **`grows_old` is M1's lifespan,** which it replaces (§2). A genome or save that says `lifespan` reads as `grows_old`. The `age` body sensor is age as a share of the age the sprite grows old at, so it reaches 1 as it grows old, sooner for a worn sprite (§9.4) or one with broken genes (§12.4), at about 22,500 ticks rather than M1's 60,000.
- **Growing up costs food.** Growing takes `growth` energy (0.3) whenever it happens, spread evenly over childhood, so a sprite that grows up sooner is a hungrier child.
- **A later old age can cost a little,** all its life: `upkeep` energy a tick for every 10,000 ticks of `grows_old`. **It's 0,** the owner's call: on 2026-10-11 until genes evolved, since the cost only made every sprite hungrier and even 0.00001 undid A3's training (median hits after training 45 against 81 untrained; 1.5 against 70 without it); and for good on 2026-10-13, so lives end as the body and brain fail (§12.5). A pack can set it.

### 9.4 Old age: genes weaken

The owner (2026-10-11): old age is the genome wearing out, and a hard life damages genes. Research backs the shape below: damage to DNA happens all life and is mostly repaired, repair falls off in old age, species die at about the same level of damage, and chronic stress makes cells age sooner (sources in the PR).

- **Every gene has a health,** from 1 (whole) to 0. It only ever falls, and it scales the gene's effect: a feeling's gain, a receptor's gain, a reaction's rate, an instinct's weight, and how far a speed or sense radius gene reaches above the least its trait can be. A gene that's a setting, not a strength, doesn't weaken: half-lives, starting levels, brain parameters, and the timing genes.
- **Before old age, repair keeps up.** Once old, damage lands on random genes, a `hit` (0.01 of health) at a time, so health falls in small, frequent steps (the owner, 2026-10-11). **An instinct takes half,** so the mind outlasts the body.
- **The pace:** an unworn sprite's genes fall to `gives_out` (0.5 on average) in about `decay.ticks` (3,600; a unit test, `an_old_sprite_s_genes_decay_until_its_body_gives_out_in_about_the_ticks_set`, holds it to 3,100–4,100), and old age kills it soon after, so old age lasts about an hour at 1× (Slice 23; it was 46,500, about 10 hours, until sprites had young). When that takes more than one hit a tick, as it does even for the unworn starter (about 1.7 a tick), the whole hits land each tick and the odds are of one more.
- **A hard life wears the body for good,** and wear never heals: `wear.starving` (0.0002) a tick while energy is under `wear.low` (.25), `wear.parched` (0.0002) while hydration is, and `wear.injured` (0.001) a tick for each unit of injury, so a hit or a prick wears the body through the injury it does, for as long as it takes to heal. It reads the body, not feelings, which genes could switch off.
- **Wear brings old age sooner and speeds the decay.** The most worn sprite grows old at (1 − `sooner`) of its `grows_old` (`sooner` is .33), and its genes decay `wear_speedup` (3) times as fast; less worn, in proportion.
- **A bad injury can scar a gene at any age:** while injury is above `scar.above` (.5), each tick has a `scar.chance` (0.002) that a random gene loses `scar.hit` (0.05; half that on an instinct).
- **Healing slows** in proportion to its genes' average health, since repair is what fails. That's the average over the genes that can weaken, settings left out.
- **The body gives out** once that average is at `gives_out`: old age then injures it (`injury.old_age`), and it dies of old age. A genome with no gene that can weaken has nothing to wear down, so its body gives out as soon as it's old. An old sprite whose thirst feeling has weakened may die of thirst first.
- **It isn't inherited.** The damage is in the body, not in its eggs. Inherited gene damage is Slice 24's hidden flaws (§12.3), and broken genes bring old age sooner as wear does (§12.4); stress chemicals damaging genes are [#195](https://github.com/Keazra/terra-sprites/issues/195). Mild stress lengthening life (hormesis) is a later idea, and botched repairs leading to cancer is [#178](https://github.com/Keazra/terra-sprites/issues/178).
- **Gene health, wear and life stage are world state:** saved, in the state hash, and checked every tick (M1 design §7.1): one health per gene, each from 0 to 1, and wear from 0 to 1.

### 9.5 The young

- **They walk slower:** `young_speed` (.67) of their speed at birth, rising steadily to all of it as they grow up.
- **They're more curious and learn faster.** The starter genome sets `curiosity` to .5 until adult (adults .3), and `plasticity` to 1.5 until adult: every learning rate is multiplied by it (M1 design §5.6).
- **Only adults fight for room:** the starter's "crowded, so hit a neighbour" instinct works from adult. The young still hit back when hit.
- **`plasticity` is a new brain parameter** (ID 38, range .25–4, default 1). A pack from before it gets the built-in range.

### 9.6 What the player sees

- **On the map, the young are drawn small:** `ö` in CP437 and `a` in ASCII, the theme keys `young_sprite` and `selected_young_sprite`. Selected, a young sprite is in reverse video, as a grown one is (#282). A theme without them draws the young as grown. The old show by moving slowly.
- **The log** says "Pip grew up" and "Pip grew old".
- **The sprite list** has a Life stage column after Age: young, adult or old.
- **The Body tab** shows the stage after the age, with when the next one comes: `age 3,120 · young · grows up 4,500`, `adult · grows old 22,500` (sooner for a worn sprite), or `old`. After the physical levels, `wear .12`, and `· genes .83`, its genes' average health, once any has weakened.
- **The Genome tab** shows a weakened gene's health after it, `· health .62`, and a gene that switches with its stages, `· from adult` or `· until adult`. A dormant gene is dimmed, with "dormant: switched off at this life stage".

### 9.7 How we know it worked

**Measured (2026-10-11)** in release builds of `main` at `a9b5b06` and of this slice at `3083765`, on seeds 1–10, with:

```
cargo run --release -p terra-sim --example lab -- scenarios/a3-correct-training.ron --seeds 10
cargo run --release -p terra-sim --example lab -- scenarios/viability.ron --seeds 10
cargo run --release -p terra-sim --example speed
```

| What | `main` | Slice 22 |
|---|---|---|
| A3: hits on sprites after training (ticks 10,000–20,000), median against the untrained control | 2.5 against 70 | 4.5 against 79, met (at most half). With the long-life cost it was 45 against 81, which is why that's held at 0 |
| Viability, ticks 0–50,000: deaths of old age | 0 | 262 of 300 |
| Viability, ticks 0–50,000: deaths of thirst / hunger | 2 / 0 | 6 / 0 |
| A4: hunger and thirst's share of grown-up deaths | met (under 10 died) | 2%, met (under 25%) |
| A5: ticks a second, 100 sprites (`speed`, a few runs) | 318 | 293–303, met (at least 200) |

**What it shows.** The world's first sprites now start at ages through adulthood and grow old within the first 9,000 ticks, so by tick 50,000 almost all have died of old age; Slice 23's breeding replaces them. When genes weakened in steps of .1, thirst killed 26 by tick 50,000 (11% for A4), most likely old sprites that lost a gene for finding water early; in steps of .01 every gene fades together. A4 still holds, though old age now fills most of its count.

---

## 10. Breeding (Slice 23)

Settled with the owner on 2026-10-12 ([#141](https://github.com/Keazra/terra-sprites/issues/141)), in a round of ten questions and two decision cards. Grown sprites have young, alone or with a partner as their genes lean, and only ever between adults (§5). Seeing the slice built, the owner set its direction: sprites are a social species, and a female laying alone should be very rare. Pairs can't yet carry a world on their own (§10.8), so for now a female lays alone only after a long while with no male in sight (§10.3), and Slice 32 ([#186](https://github.com/Keazra/terra-sprites/issues/186)), courtship and family units, makes it very rare (§15). The numbers are data, the `breeding` section of `physiology.ron` and the starter genome: first guesses, tuned against §10.8. A save from before this slice keeps the pack it was made with (M1 design §2.8), which has no breeding, so its sprites never have young.

### 10.1 Sexes, by neighbours

The owner chose sex by neighbours, as in clownfish and gobies, over hermaphrodites, two fixed sexes and mating types.

- **Every sprite hatches male,** and no young sprite changes.
- **A body sensor, `female_near`,** is 1 while a female adult is within the sprite's sense radius, and 0 otherwise. `female` is 1 for a female. The round had it count how many females are near; one is enough to keep a male male, so it's on or off. `male_near` is the same for a male adult; it feeds longing (§10.3). Slice 32 makes a female or male adult heard calling lately count as near too (§15.1).
- **A hormone, the `feminiser`, decides when a male turns.** The starter genes raise it while the sprite, grown up, sees no female adult and lower it fast while it sees one: a male long without females (about 800 ticks) turns. The genes set the pace, so evolution can tune it.
- **Past `turns_female` (0.8), an adult male turns female for good,** at the start of step 3 (M1 design §2.4), as its life stage changes. It's the engine's rule, like growing up: the genes move the hormone, the body does the rest.
- **Only a female lays eggs.** A pair needs a male and a female; a female can breed alone (§10.3).

### 10.2 Fertility and arousal

The game's rule, "the world decides what happens to the body, the genes decide how it feels":

- **The body says whether a sprite can breed.** In a pack with breeding, it's **fertile** while it's grown (old counts), its energy and hydration are both above `fed` (0.4), it's at least `rest` (1,500, and never less than 1) ticks since its last egg, laid or fathered, and the world has room for another sprite (§10.6). The `fertile` body sensor reads it. `fed` was 0.6 in the round; in tuning, at 0.6 a pair was seldom fertile at the same time, so it was 0.5, and since Slice 32 it's 0.4 (#186): with courtship, at 0.5 a fertile male and female were seldom within sight of each other at once, and too few eggs came from pairs for the world to fill.
- **The genes say whether it wants to.** **Arousal** is a drive and a need, second-order like loneliness (#50). The starter genes raise it fast while the sprite is fertile, drop it at once when it breeds (the `bred` pulse), and drop it within a few ticks of its no longer being fertile, so it reads as fertility (§10.3). It draws the eye to sprites.
- **Only mating relieves it, in what a sprite learns.** Arousal's fall teaches worth only when the sprite's latest touch was a Mate, and only about that partner: it learns which sprites it bred with were good for it, and what it learns never spreads to sprites in general or to new things, so it never learns that a young sprite is. Whatever the genes, arousal reads 0 in a young sprite and in an adult's choices about one (§5).
- It was called **broodiness** in the round; the owner renamed it on 2026-10-12. Brooding is sitting on eggs, which may come later.

### 10.3 Mating and breeding alone

- **Two actions, chosen by instinct** (the owner, Q1): `Mate`, with the sprite attended to, and `BreedAlone`. They show on the doing line and the Brain tab.
- **`Mate` is offered only when both are adults** (§5): the sprite, and the one the verb would be aimed at, a running action's target or else the candidate (M1 design §5.5, #173). It works, once, if both are fertile, one is male and the other female, and she has a free tile beside her: she lays an egg there (§10.4), on a tile a sprite could stand on, shallow water included, with no sprite or item on it (#196). The partner learns about the one that mated with it too, as if touched. Otherwise it's a fruitless try.
- **`BreedAlone` is offered only to a grown sprite.** It works for a fertile female with a free tile beside her who has gone `alone_after` (4,500) ticks with no male adult seen or heard, the body's own rule since Slice 32 (§15.7); for a male, or a female not fertile or not alone long enough, it's a fruitless try. Before Slice 32 it worked once her **longing** had reached `breeds_alone` (0.5), which a pack from then keeps.
- **Longing** is a hormone the starter genes raise while a grown sprite sees no male adult and lower fast while it sees one: about 1,100 ticks with no male in sight took it to 0.5. Since Slice 32, which counts hearing too, it builds over about an hour and never reaches 0.5: with no male seen or heard it's about 0.28 after an hour, levelling off a little above 0.4 (§15.7). It came after the owner saw that 9 eggs in 10 were laid alone. Most of those came from chance: a sprite now and then tries what it isn't drawn to (M1 design §5.5), and for a fertile female, breeding alone always worked. The body's rule stops that; the genes set how fast longing builds, so evolution can still tune it.
- **The egg costs `cost` (0.3) energy** (Q2): split between a pair, or all hers alone. Per bite of food, both pass on as much of the parents' genes, so neither wins on cost; what makes partners pay is that a lone egg doubles its mother's genes, so each hidden flaw she carries is broken in about half her lone young (§12.3).
- **Body language** (the owner's card, 2026-10-12): a sprite sees how aroused the sprite it attends to is, `target_arousal`, and whether it's of the other sex, `target_other_sex`. Before it, in tuning, most tries to mate met a partner of the same sex or one not fertile, and nearly every egg was laid alone. These are brain inputs `brain_io.ron` lists like any other: one that reads `Target(...)` reads the sprite attended to, and one that reads `Differs(...)` how far that differs from the sprite's own; both are 0 unless attention is on a sprite that has hatched, and any that reads arousal is 0 unless both are grown (§5). Attention can't read them, as it can't any input about what it attends to (M1 design §5.3).
- **The starter's instincts:** arousal draws attention to sprites; arousal with an aroused sprite of the other sex in view leads to Mate; and in a female longing for a male, arousal leads to breeding alone.

### 10.4 Eggs and hatching

- **An egg is an item until it hatches,** the object type `egg` (the owner, 2026-10-13, #196): small, like a berry. It holds the sprite it hatches into: its genes as copying left them (§12, §13), its parents, the name the player gave it, if any, and when it was laid. It's laid on a random free tile beside its mother. In Slice 23 it was a sprite at a first life stage, Egg (Q5).
- **It hatches 750 ticks after it was laid,** where it lies, as a young male with a newborn's body, and the item is gone. The sprite keeps the egg's ID, and its age counts from then. The 750 is the egg type's one stage in `objects.ron`, which ends in `Hatch`: only one type may hatch, and only a sprite lays one. It was physiology's `breeding.hatches`, and a pack that still says that lays egg items that hatch after it.
- **An egg due to hatch waits** while the Cursor holds it, and hatches on the tick it's put down; and while a sprite stands on it, until the sprite steps off (#196).
- **Other sprites see an egg as a thing of its own kind,** the category `egg`, not as a sprite: so "crowded, so hit a neighbour" never aims at one, and `nearby_sprites`, `female_near` and `male_near` don't count one. Its verb table is empty, so whatever a sprite tries on it is a fruitless try: an egg can't be eaten or hurt until something in the world is meant to hurt eggs. A sprite walking onto one doesn't move it, since no verb pushes it. A rolling item knocks it on but never crushes it: it's hard (M1 design §3.5.4). Arousal reads 0 in a choice about an egg, as about any sprite not grown up (§5).
- **The Cursor picks an egg up and puts it down** like any item, in Grab mode, **and never throws one** (the owner, #190): the egg type says `throwable: false`, so the screen refuses at once, "Can't throw an egg", and the world refuses a throw of one. Pets and zaps don't reach an egg: they act on sprites. The player can name one, and the sprite keeps the name.
- **A sprite remembers its parents:** the one that laid it, then its partner.
- **A save from before #196 loads with each egg it holds turned into an egg item,** on its tile, or the nearest free one if an item lies there, hatching when it would have. Its pack's `egg` pseudo type becomes the egg type, hatching after its `breeding.hatches`. One with no egg type to become, or no tile to lie on, hatches as it loads.

### 10.5 The child's genes

- **Mixed** (Q6): each gene, with its switch, came from one parent or the other at random, where the two genomes lined up gene for gene, as the starter's do; where they didn't, the child took its mother's. Since Slice 24 the genes line up by what each does and mix in stretches (§12.2), and copying errors change them now and then (§12.1).
- **The damage old age did to a parent's genes isn't passed on** (§9.4); a child's genes are whole, but for broken ones (§12.4).

### 10.6 What stops the world filling up

- **Food and a limit** (Q7): births need fed and watered parents, so food holds numbers down. The **limit**, the preset's `most_sprites` (100 in the default world, eggs counted, on the map or in the Cursor, #196), is the safety net: in a full world no sprite is fertile, so none is aroused and none tries in vain. In the terrarium's story, the limit is the glass.
- **The Cursor may place a sprite past the limit:** that's the player's call, not the sprites'.
- **A lab scenario can set `most_sprites`.** A1 to A3 and the thorn trap watch only their own sprites, so they hold no more than they start with (an egg may replace a sprite that died).

### 10.7 What the player sees

- **The log:** `Pip #12 laid an egg (with Bo #7)`, `Pip #12 laid an egg alone`, `Sprite #31 hatched (Pip #12 and Bo #7's)`, `Pip #12 turned female`.
- **The map:** an egg is `o` in CP437 and `0` in ASCII, where `o` is the ball: the theme's `objects` entry for `"egg"` (#196). A selected egg is drawn in reverse video. A theme from before, with the tiles `egg` and `selected_egg`, still loads, and draws the egg item with its `egg` glyph unless its `objects` give one.
- **An egg's card:** a click on an egg in Select mode selects it, and every sprite tab shows its card in place of a sprite's: `egg · hatches in 750 ticks`, or while it waits `egg · hatches once it's put down` or `egg · hatches once nothing stands on it`; then `parents Pip #12 and Bo #7`. When it hatches, the selection goes on to the young sprite. `T` tracks an egg and `G` saves its genome, but the Cursor doesn't follow one, and Tab passes it by. The World tab counts the eggs on the map with the other items; the top bar counts only sprites.
- **The Body tab:** after the traits, the sex, `· fertile` while it is, and for a male, how near it is to turning, `male · fertile · turning 40%`; then `parents Pip #12 and Bo #7`.
- **The sprite list** shows young, adult or old as the life stage, and no eggs; **the doing line** says `Going to mate with Bo #7`, `Mated with Bo #7`, `Breeding alone`, `Laid an egg alone`. Slice 32 rewords the first two (§15.8).
- **The Cursor's mark** shows what it holds as the map draws it: an egg as an egg, and a young sprite it leads small.
- **Arousal** colours a sprite light magenta when it's its strongest drive above half.
- The full family tree is Slice 25 (§17).

### 10.8 How we know it worked

Agreed before measuring (Q9):

- **Keeps going:** the default world, left alone for 100,000 ticks (about 22 hours at 1×, five or so generations), seeds 1–10: sprites alive at the end in **at least 8 of 10 seeds**, and never more than the limit.
- **Nothing else got worse:** A1–A4 still met, and A5 at the limit, 100 sprites, still at least 200 ticks a second.
- **For information:** how many hatchlings grow up. Today almost all would; Slice 29 makes childhood hard (about 20%).

`cargo run --release -p terra-sim --example generations` runs the first.

**Measured (2026-10-12)** in release builds of this slice at `0385ebe`, with lives of 4 to 8 hours (§2) and slow-fading lessons and the fix for A3 (#187, PR #189) merged in, on seeds 1–10 (A3 on 1–30), on the owner's PC, with:

```
cargo run --release -p terra-sim --example generations
cargo run --release -p terra-sim --example lab -- scenarios/viability.ron --seeds 10
cargo run --release -p terra-sim --example lab -- scenarios/a3-correct-training.ron --seeds 30
cargo run --release -p terra-sim --example speed
```

The default world left alone for 100,000 ticks:

| Seed | Alive at the end (and eggs) | Most at once | Eggs with a partner | Eggs alone | Hatched | Grew up | Generations |
|---|---|---|---|---|---|---|---|
| 1 | 96 (4) | 100 | 295 | 57 | 348 | 333 | 10 |
| 2 | 96 (3) | 100 | 235 | 89 | 321 | 315 | 10 |
| 3 | 99 (1) | 100 | 240 | 73 | 312 | 303 | 10 |
| 4 | 99 (1) | 100 | 241 | 66 | 306 | 293 | 11 |
| 5 | 96 (4) | 100 | 289 | 53 | 338 | 321 | 10 |
| 6 | 96 (1) | 100 | 226 | 100 | 325 | 317 | 9 |
| 7 | 97 (3) | 100 | 259 | 61 | 317 | 308 | 10 |
| 8 | 95 (5) | 100 | 256 | 78 | 329 | 319 | 12 |
| 9 | 96 (3) | 100 | 274 | 57 | 328 | 314 | 11 |
| 10 | 98 (2) | 100 | 276 | 84 | 358 | 337 | 11 |

How long hatchlings lived, in hours at 1×, of those old age killed (`generations` counts them, 205 to 258 a seed), and the world's first sprites, who start at ages through adulthood (§9.1):

| Seed | Shortest | A tenth by | Half by | Nine tenths by | Longest | First sprites |
|---|---|---|---|---|---|---|
| 1 | 3.8 | 5.1 | 5.9 | 7.7 | 8.1 | 4.4–8.0 |
| 2 | 4.2 | 5.6 | 6.6 | 7.6 | 7.8 | 4.1–7.8 |
| 3 | 3.0 | 4.7 | 7.6 | 7.9 | 8.0 | 4.2–7.9 |
| 4 | 3.8 | 4.8 | 7.4 | 7.9 | 8.0 | 2.9–8.0 |
| 5 | 4.1 | 4.9 | 6.3 | 7.5 | 7.8 | 4.5–7.7 |
| 6 | 3.6 | 5.2 | 7.0 | 7.6 | 8.0 | 4.3–8.0 |
| 7 | 4.2 | 5.1 | 6.8 | 7.7 | 7.9 | 4.5–7.8 |
| 8 | 4.1 | 5.0 | 6.5 | 7.8 | 7.9 | 4.0–7.7 |
| 9 | 4.1 | 5.1 | 6.2 | 7.7 | 8.0 | 4.1–8.0 |
| 10 | 3.1 | 4.3 | 6.0 | 6.6 | 7.5 | 4.1–7.8 |

- **Keeps going: met.** Sprites are alive at the end in all 10 seeds, 9 to 12 generations deep, and no world went over its limit of 100.
- **A4, grown-up hunger and thirst: met.** Of 964 deaths in ticks 10,000–50,000 over all seeds, 1 was of thirst (0.1%, under 25%). The rest were old age, but for 12 sprites hurt by another. Over the whole 100,000 ticks, 4 of 2,518 deaths after tick 10,000 were of hunger or thirst.
- **A4, alive at tick 10,000: 68% (median) of the world's first 30.** They still start at ages through adulthood (§9.1), but now grow old later, so 7 to 12 of them a seed die before tick 10,000, all of old age but 2. M1's stand-in for childhood no longer measures childhood: it ends at growing up (§2), and Slice 29 makes that A4's measure. Counted that way, almost every hatchling survives (below).
- **A3: met, and no longer by luck.** Before the fix for #187, this slice's longing genes left A3 failing in most sets of seeds: one shock wore off. With it and slow fades, over seeds 1–30, trained sprites hit a median of once in ticks 10,000–20,000 against 42 times untrained, and every random set of 10 of those seeds passes.
- **A5: met,** 418–440 ticks a second with 100 sprites (`speed`, three runs) on the owner's PC. Measured in the cloud at `05a8c67`, it was 257–266, against 302–308 for `main` with the A3 fix.
- **A1 and A2: met** (their tests pass, with `most_sprites` as §10.6 says).
- **Lives: about as the owner set them, a little long.** The median hatchling lived 5.9 to 7.6 hours a seed, 6.5 in the median seed; a tenth died by 4.3 to 5.6 hours, and nine tenths by 6.6 to 7.9. The few lives under 4 hours are sprites a hard life wore into old age sooner (§9.4). Lives run long because a sprite that lives longer has more young, who take its `grows_old` (§10.5): nothing pulls the other way, and since Slice 24 nothing is meant to: a later old age costs nothing (§12.5). Before, with lives of about 4 hours, a tenth died by 3.7 hours and nine tenths by 4.3 (all seeds together).
- **For information:** almost every hatchling grows up, as expected until Slice 29: in ticks 0–50,000 over all ten seeds, 15 sprites of any age died of something other than old age (13 hurt by another, 2 of thirst). Between 69% and 85% of eggs come from a pair, 78% in the median seed: a female lays alone only once she has longed for a male a while (§10.3). Most tries to mate come to nothing (513 to 696 a seed, against 226 to 295 that lay an egg): Mate is offered between any two adults, but lays an egg only when both are fertile and of different sexes.

---

## 11. The view past the wall (Slice 31)

The terrarium is a ship flying through space (#181, a milestone of its own after Body & verbs). This slice lets the view go past the terrarium's wall and see space outside, with stars going by ([#180](https://github.com/Keazra/terra-sprites/issues/180)). Settled with the owner on 2026-10-12, in the ship's design pass, rounds 1 and 2. It touches only the front end: the simulation, its saves and its replays don't change.

### 11.1 Past the wall

- **The viewport scrolls past the wall by a quarter of the map view** on each side, the same however it moves: at 140×40, about 23 tiles sideways and 7 up or down (round 1, Q3). Half the view would let Track centre a sprite at the wall, but leave half the screen empty at the far edge.
- **Track** keeps the selected sprite in the middle of the map view as far as that margin allows, and `Tab`'s centring stops there too (M1 design §6.1, §6.5).
- **A map that fits in the map view with its glass,** so at least two tiles narrower or shorter than it, is drawn in the middle of it, with space around it, rather than in a map view shrunk to fit (M1 design §6.1). It doesn't scroll in a direction it fits.

### 11.2 The glass

- **The wall is drawn as a line of glass one tile outside the map,** all round: `═` along the top and bottom, `║` down the sides and `╔ ╗ ╚ ╝` at the corners, light cyan (round 1, Q3, Q4). It isn't a tile: sprites, objects and the Cursor never reach it.
- **The map view's border is single-lined on every side,** since the glass shows where the wall is. Its double-lined sides go (M1 design §6.1).

### 11.3 Space and the stars

Past the glass is space: blank, in the terminal's own background (black on most), with stars drifting right to left, since the ship flies towards the right of the screen (round 1, Q5). Sideways drift looks smoother than up or down, because a terminal's characters are twice as tall as they are wide.

| Depth | CP437 | ASCII | Colour | Share of space's tiles | Drift at 1× | Shift when scrolling |
|---|---|---|---|---|---|---|
| Far | `·` | `.` | dark gray | about 5% | 0.55 tiles a second | 8% of the scroll |
| Near | `∙` | `+` | gray | about 2.2% | 1.3 tiles a second | 20% |
| Nearest | `*` | `*` | white | about 0.9% | 3.2 tiles a second | 45% |

- **The stars move with the game:** still while it's paused, quicker at higher speeds, and no quicker than at 4×, so 16× doesn't blur. A step while paused doesn't move them.
- **Scrolling shifts them a little** against the view, the far ones least: the share in the table sideways, and half of it up and down. That gives depth.
- **They belong to the screen, like the theme.** They're fixed to the map view and wrap around it, from a pattern of the front end's own, not the world's dice, so they're never part of the world, its saves, its replays or its state hash. A star is drawn only on space, never over the glass or anything inside it.
- The numbers are those of the preview the owner tried in round 1, and the theme sets the glyphs and colours.

### 11.4 The pointer over space

- **The Cursor can't reach past the wall** (round 2, Q11). While it follows the pointer and the tile pointed at (M1 design §6.5) is glass or space, the Cursor stays on its last tile on the map, as when the pointer leaves the map view, but isn't drawn there. A click, `Q` or `E` does nothing, rather than act on that last tile, and the status line says "space". A sprite it leads stands still and an item it holds stays held. It shows again when the pointer comes back over the map.
- **A Cursor that follows a sprite carries on as before** (M1 design §6.5): it stays on its sprite and is drawn wherever the pointer is, and a click, over space too, or `Q` and `E` reach the sprite. While it leads the followed sprite, Follow steps aside and the Cursor follows the pointer, so the rule above applies.
- **While the player aims a throw or a shove, the pointer pulls on past the wall** (#184). The Cursor sits on what it aims (M1 design §6.5), so it isn't out of reach: a click over space reaches it, as `Q` and `E` do, so a right click there sends the aim. The pull runs from the Cursor to the pointer, counted in tiles as if the map went on, so a thing against the wall can be pulled back over the glass and sent across the map. An aim that ends over space, sent or not, leaves the Cursor where it was, out of reach, so a sprite it still leads stands still.
- **`F`, or a middle click, over space reaches no tile** (#184): following, the Cursor stops, as anywhere; otherwise, out of reach, it follows the selected sprite, if there is one, rather than the sprite on its last tile (M1 design §6.5). The wheel over space cycles the modes, as anywhere over the map view.
- **Out of sight, the Cursor draws no leash either** (#184). Wherever it's drawn, its arrows and marks show only over the map's tiles, never over the glass, as they stopped at the map view's border before.
- **Off the map view, the pointer is past nothing** (#184): moved on from space to the inspector, say, the Cursor shows again on its last tile, as when the pointer leaves the map view from a tile, and `Q` and `E` reach it.
- **A picture with no player in it withdraws the Cursor** (#241), as the renderer's clips do: out of reach, as over space, with no pointer anywhere. It isn't drawn, and the status line says nothing of what it's over, not "space", since nothing is pointed at. The pointer coming back over the map brings it back. The map view can also be drawn alone, its tiles and what's past the wall filling the picture, with no border, top bar, panels or status line. Or a picture can leave out any of the screen's other parts, the top bar, the inspector, the event log and the status line, and the map view, with the inspector beside it while that's drawn, takes the room they leave (#291); the game itself always draws them all.

### 11.5 Themes

Both built-in themes gain the glass's six glyphs and the stars' three depths, as theme keys (`docs/reference/themes.md`). In ASCII the glass is `-` along the top and bottom, `|` down the sides and `+` at the corners. A theme without the new keys draws the default theme's.

### 11.6 How we know it worked

- **UI tests** (ratatui's `TestBackend`): scrolling stops at the margin on every side; the glass runs all round the map, one tile out; the border is single-lined; stars appear only on space; stars stand still while paused and drift as far at 16× as at 4× in the same time; a small map is centred, and one as wide as the map view, or a tile narrower, scrolls to show its glass; with the pointer moved over space after the Cursor was on a sprite, the Cursor isn't drawn and a click, `Q` or `E` sends no command (`docs/agents/how-we-work.md`: test every input that reaches a rule); with Follow on, a click over space and `Q` still reach the followed sprite.
- **The simulation is untouched:** its tests and golden files pass unchanged.
- **Played** in Windows Terminal at 140×40 and at the smallest size the game allows.

The ship's own walls come later: its prow is iron, and walls between sectors are hull (#181). Until then, every wall of the terrarium faces space.

---

## 12. Mixing and mutating genes (Slice 24)

Settled with the owner on 2026-10-13 ([#142](https://github.com/Keazra/terra-sprites/issues/142)), in a round of ten questions. Children now differ from their parents: copying errors change a gene's numbers now and then, a pair's genes line up by what each gene does and mix in stretches, and hidden flaws passed down families make the quality of a sprite's genes part of how it ages (§3, reading C). The slice comes in two parts: 24a, this section, and 24b, copy and change (§13). The numbers are data, the `mutation` and `flaws` sections of `physiology.ron` and `breeding.crossover`: first guesses, tuned against §12.7. A save from before this slice keeps its pack (M1 design §2.8), which has neither section, so its eggs copy their parents' genes exactly and carry no flaws; its pairs' genes line up by what each does (§12.2), but mix gene by gene, as in Slice 23.

### 12.1 Copying errors

- **When an egg is laid, each gene value has a 1 in 100 chance of being copied wrongly** (`mutation.chance`, 0.01). The starter has 137 values a change can move (a threshold of 0 stays 0), so about three hatchlings in four have a change, most of them one or two. The owner (Q1): "these are creatures that only live for a few hours", and mutations should be "fun and visible-ish": rare and large, with a few small.
- **Three changes in four are large,** a factor from half to double, as likely to halve as to double (`large.share` 0.75, `large.most` 2). The rest are small, within a fifth either way (`small` 0.2).
- **A copying error changes only a number,** never what a gene refers to, how it works or when it works: how fast hunger rises, how strong an instinct is, when a sprite grows old. Each value is then held within its range, as spawn variation's are (M1 design §4.9). A gene that would change the body directly stays flagged and does nothing, whatever an error does to it (M1 design §4.3); property tests with random genomes pin both.
- **The world's dice decide** (M1 design §2.3), so saves and replays agree.
- **How often is a rule of the world,** the same for every sprite (Q2): if it were a gene, nothing would make careful copying cost anything, and evolution would turn it down until the species stopped changing. It may become a gene once it can have a cost ([#194](https://github.com/Keazra/terra-sprites/issues/194)).
- **A sprite remembers which of its genes copying changed, and what each was,** for the Genome tab (§12.6): world state, saved, in the state hash, and checked every tick (M1 design §7.1): what each gene was does what it does now.

### 12.2 Lining up and mixing

This replaces §10.5's first rule.

- **A pair's genes line up by what each gene does** (Q3), as real chromosomes pair matching genes: a gene of the father's pairs with one of the mother's that refers to the same things, works the same way and works in the same life stages, whatever its numbers, the first such with the first such. So "low energy → hunger" pairs with "low energy → hunger" wherever each sits. The child has its mother's genes in her order, and a gene only the father has after the one it follows in his genome.
- **The child's genes come from one parent in stretches:** it starts with either at random, and at each gene switches to the other with `breeding.crossover`'s chance (0.1), so about every 10 genes, as chromosomes cross over. Genes that sit together and work together, like hunger's rise and its fall, tend to be inherited together. A pack whose `breeding` has no `crossover` mixes gene by gene (0.5), as Slice 23 did.
- **A gene only one parent has goes to the child when its stretch is from that parent,** so to about half the young.
- **A lone egg's genes are its mother's** (§10.3), copied as any egg's are.
- **What age did to a parent's genes still isn't passed on** (§9.4): a child's genes are whole, but for broken ones (§12.4).

### 12.3 Hidden flaws

Gene quality, as carriers (Q5), as in real animals.

- **Each gene that can weaken (§9.4) has two copies, one from each parent,** and either may carry a **flaw**. One sound copy is enough, so a sprite with one flawed copy, a **carrier**, shows nothing. A setting that doesn't weaken, such as a half-life or a starting level, can't be flawed, nor can a flagged gene, which does nothing whatever its copies are (M1 design §4.3).
- **A parent passes on one of its two copies of each gene,** at random, so a carrier passes its flaw to about half its young. A gene only one parent has gets a sound copy from the other.
- **A gene with both copies flawed is broken** (§12.4). Close kin often carry the same flaw from a shared ancestor, so their young are likelier to get it twice, as inbred animals do, with no rule of its own.
- **A lone egg gets one of its mother's copies of each gene, doubled,** as a Komodo dragon's or a shark's does. So each flaw she carries is broken in about half her lone young, where a pair's egg would need both parents to carry it. That's the cost of breeding alone §10.3 promised.
- **Flaws arise in copying:** as an egg's genes are copied, each sound copy of a gene that can weaken gets a flaw with the chance `flaws.arise` (0.004), so about one egg in two has a new one.
- **The world's first sprites carry a few,** as wild animals do, so family matters from the first generations: one copy of each gene that can weaken is flawed with the chance `flaws.founders` (0.035), about three each, all hidden. So does a sprite the Cursor places from the starter genome. One placed from a genome file carries none, since the file holds genes, not flaws (`docs/reference/genomes.md`), and nor does a scenario's.
- **Flaws are world state:** saved, in the state hash, and checked every tick (M1 design §7.1): at most two flawed copies of each gene.

### 12.4 Broken genes

What poor genes do (Q6): they're weaker, and they bring old age sooner.

- **A broken gene works at half strength:** it's born with a health of `flaws.health` (0.5), as if old age had weakened it (§9.4), and its health only falls from there. A broken "thirst → drink" leaves a sprite slow to drink when thirsty.
- **Broken genes bring old age sooner and speed its decay,** as a hard life's wear does (§9.4): each counts as `flaws.wear` (0.05) of wear towards when the sprite grows old and how fast its genes decay, up to the most worn. It isn't wear: the Body tab's `wear` doesn't count it. By `physiology.ron`'s numbers, for a sprite that would grow old at 22,500 ticks, one broken gene takes about 10 minutes off its life at 1×, and five about 45.
- **The owner's words for it** (M2 planning): the genome's decay is "amplified or damaged by stress throughout their lives, and the 'quality' of their genes". Wear is the stress (§9.4); broken genes are the quality. Stress chemicals damaging genes are [#195](https://github.com/Keazra/terra-sprites/issues/195).

### 12.5 No cost for a long life

- **A later old age costs nothing** (the owner, Q7, 2026-10-13): "I prefer to let Sprites die due to failure of the body/brain as opposed to an artificial energy cost", and it could be interesting to see sprites make their lives last long, as humans have. `ageing.upkeep` stays 0 in the built-in pack, for a pack that wants the cost (§9.3).
- **So lives may grow longer over the generations,** up to `grows_old`'s ceiling of 60,000 ticks, old age at about 13 hours at 1×. A sprite that lives longer has more young, who take its `grows_old`; what holds that back is the body: broken genes and wear bring old age sooner (§12.4), and every life still needs the world's food and water.
- This settles §3's "a long life costs something" the other way. §12.7 reports how long lives were, with no bar on their length.

### 12.6 What the player sees

- **The Genome tab opens with what's new in the sprite** (Q9, B), then goes on as before after a blank line:

```
│ FROM PIP #12 AND BO #7                     │
│ grows old 25,104, was 23,880               │
│ low energy → hunger +.00512 past .515, was │
│   +.00428                                  │
│ broken: thirst → drink                     │
│ carries flaws: hunger → eat ·              │
│   pain → retreat                           │
│                                            │
│ TRAITS                                     │
```

  Where its genes came from, `FROM PIP #12 ALONE` for a lone egg; each gene copying changed, as the tab writes it now, with the numbers it had; then the genes it has broken and those it carries a flaw in. A sprite with no parents, from the world's beginning or placed by the Cursor, has no list, though it may carry flaws.
- **A broken gene shows its health where it sits,** `· health .5`, as one old age weakened does (§9.6), and the Body tab's `genes .97` counts it.
- **No new log lines:** copying errors come in about three eggs in four and would bury the rest.

### 12.7 How we know it worked

Agreed before measuring (Q10), without the bar on life length (Q7):

- **The species changes (M2's G4):** in the default world left alone for **200,000 ticks** (about 45 hours at 1×), seeds 1–10, at least **3 gene numbers** whose middle value among the living moved **by at least 10%** from the starter's, **the same way in all 10 worlds**. Chance moves a number the same way in all ten about 1 time in 500, so with 83 numbers, 3 is well past chance. Since Slice 29, it's judged on the 220-hour long run, over the seeds that last (§19.6, #226).
- **Keeps going:** sprites alive at the end in **at least 8 of 10 seeds**, never more than the limit.
- **Nothing else got worse:** A1–A4 still met (A3 on seeds 1–30), and A5 at the limit, 100 sprites, at least 200 ticks a second.
- **For information:** how many flaws sprites carry and how many genes are broken, over time; how long lone eggs' young live against pairs' young; which numbers moved most; how long lives were.

`cargo run --release -p terra-sim --example generations -- --ticks 200000` runs the first two and reports the rest. **Before this slice** (`main` at `dc9466e`, with this slice's `generations.rs` and the `Genome::genes` it reads added, seeds 1–10), no number moved a tenth the same way in all 10 seeds: `grows_old` rose about 30% in 9 of 10, since a longer life leaves more young (§12.5), and the median hatchling old age killed in the second half lived 5 to 7 hours in only 2 of 10 seeds, most 7 to 7.8.

**Measured (2026-10-13)** in release builds of this slice at `fc4f72f`, in the cloud, on seeds 1–10 (A3 on 1–30), with:

```
cargo run --release -p terra-sim --example generations -- --ticks 200000
cargo run --release -p terra-sim --example lab -- scenarios/viability.ron --seeds 10
cargo run --release -p terra-sim --example lab -- scenarios/a3-correct-training.ron --seeds 30
cargo run --release -p terra-sim --example speed
```

The default world left alone for 200,000 ticks; flaws are the average among the hatched sprites alive at the end, and lives the median of those old age killed, in hours at 1×:

| Seed | Alive at the end (and eggs) | Eggs alone | Hatched | Generations | Flaws carried | Broken genes | Lone eggs' young lived | Pairs' young lived |
|---|---|---|---|---|---|---|---|---|
| 1 | 100 (0) | 73 | 665 | 19 | 7.2 | 0.99 | 6.3 | 6.7 |
| 2 | 97 (3) | 130 | 686 | 19 | 7.3 | 1.81 | 6.1 | 6.8 |
| 3 | 100 (0) | 69 | 622 | 20 | 7.4 | 1.04 | 6.4 | 7.1 |
| 4 | 96 (3) | 80 | 655 | 19 | 8.1 | 1.04 | 6.1 | 6.9 |
| 5 | 100 (0) | 95 | 665 | 19 | 8.6 | 1.02 | 5.9 | 6.8 |
| 6 | 98 (2) | 185 | 747 | 19 | 7.2 | 2.53 | 5.5 | 5.4 |
| 7 | 95 (4) | 73 | 693 | 20 | 7.5 | 2.05 | 5.8 | 6.6 |
| 8 | 98 (2) | 153 | 644 | 20 | 5.2 | 1.68 | 6.8 | 7.3 |
| 9 | 98 (1) | 131 | 653 | 20 | 6.8 | 0.98 | 6.2 | 6.8 |
| 10 | 98 (2) | 135 | 659 | 20 | 5.6 | 2.73 | 6.3 | 7.0 |

- **The species changes (G4): not met.** 1 of the 83 numbers moved a tenth the same way in all 10 seeds: `grows_old`, up 11% to 36%, 24% in the median seed, about as much as on `main` without copying errors. A few others moved in some seeds only: the "always → wander" instinct weakened in 5 (to a third, in one), thirst's threshold fell in 3, and how fast fertility raises arousal grew in 4. Most moved less than a tenth in every seed. Our reading: in today's easy world almost nothing but a long life is favoured, so copying errors scatter numbers both ways and few take over a population. **The owner's call (2026-10-13): measure G4 again later,** once Slice 29's harsher world gives selection something to work on and Slice 27's fast-forward runs more generations; copying errors stay rare (noted on [#145](https://github.com/Keazra/terra-sprites/issues/145) and [#147](https://github.com/Keazra/terra-sprites/issues/147)).
- **Keeps going: met.** Sprites are alive at the end in all 10 seeds, 19 or 20 generations deep, and no world went over its limit of 100.
- **A4: met.** Of 1,021 deaths in ticks 10,000–50,000 of the viability run over all seeds, 6 were of thirst and none of hunger (0.6%, under 25%). Over the long run's 200,000 ticks, 37 of 5,898 deaths after tick 10,000 were of hunger or thirst.
- **A3: met.** Over seeds 1–30, trained sprites hit a median of once in ticks 10,000–20,000, against 42.5 times untrained, and all of 100 random sets of 10 of those seeds pass.
- **A5: met,** 245–260 ticks a second with 100 sprites (`speed`, four runs) in the cloud, where Slice 23 measured 257–266.
- **A1 and A2: met** (their tests pass).
- **For information, copying errors four times as often** (`mutation.chance` 0.04, otherwise `fc4f72f`, same seeds): G4 is still not met. 1 of 83 numbers moved a tenth the same way in all 10 seeds, `grows_old`, up 16% to 113% (35% in the median seed). More numbers moved in most seeds: the "always → wander" weight fell in 9 (to six tenths of the starter's in the median seed) and the sense radius in 6. So selection does act on more than life length, but in 20 generations the rate limits how far it gets. The cost: sprites alive at the end had 1.7 to 5.6 broken genes rather than 1.0 to 2.7, and 91 of 5,799 deaths after tick 10,000 were of hunger or thirst (1.6%). Errors stay rare, as the owner chose, and Slice 27's fast-forward gives selection more generations instead; the error rate as a gene is [#194](https://github.com/Keazra/terra-sprites/issues/194).
- **For information, flaws build up:** the hatched sprites alive halfway through carried 4.2 to 6.4 hidden flaws each and had 0.37 to 0.94 broken genes; at the end, the table's 5.2 to 8.6 and 0.98 to 2.73.
- **Lone eggs' young live a little less long:** shorter than pairs' young in 9 of 10 seeds, by about 40 minutes in the median seed.
- **Lives:** the 5,650 hatchlings old age killed lived a median 6.7 hours, a tenth of them under 4.9 and nine tenths under 7.6 (on `main`, 7.0, 5.4 and 7.8), and the longest 7.8 to 13.2 hours a seed. In the second half the median was 6.0 to 7.5 hours a seed. Broken genes pull lives in a little even as `grows_old` rises (§12.5).

### 12.8 Later

- **24b, copy and change** (Q4): §13.
- **The error rate as a gene:** [#194](https://github.com/Keazra/terra-sprites/issues/194).
- **Stress damaging genes:** [#195](https://github.com/Keazra/terra-sprites/issues/195), with temperament ([#53](https://github.com/Keazra/terra-sprites/issues/53)): a stress chemical that helps in the moment and harms when it lasts.

---

## 13. Copy and change (Slice 24b)

Settled with the owner on 2026-10-13 ([#142](https://github.com/Keazra/terra-sprites/issues/142)): what a change can change in Slice 24's first round (Q4), and the rest in a second round of four questions, where the owner chose how often (Q1) and left the other recommendations standing. Now and then, as an egg's genes are copied, a whole gene is doubled, lost, or changed in what it does. That's how the species can grow instincts the starter genome never had, and in time come to be born knowing what its ancestors had to learn, as the M1 design promised. The numbers are data, the `copy_and_change` section of `physiology.ron`: first guesses, tuned against §13.3. A save from before this part keeps its pack (M1 design §2.8), which has no such section, so its eggs never have a gene doubled, lost or changed.

### 13.1 Doubled, lost and changed

- **As an egg's genes are copied, each gene is now and then doubled** (`copy_and_change.doubled`, 0.00029), **lost** (`lost`, 0.00029) **or changed in what it does** (`changed`, 0.00029). One roll of the world's dice for each gene decides which, if any, so at most one happens to it. About one hatchling of the starter's in ten has one (Q1: the owner chose one in 10 over the recommended one in 5), each kind about as often as the others, so about one in thirty has a changed gene. Then copying errors and flaws come as before (§12.1, §12.3), on the genes as they now are.
- **A doubled gene's copy sits right after it,** with its switch and its flaws, so the two tend to be inherited together (§12.2). What a double does follows from how genes already work (M1 design §4.3): a doubled feeling, such as "low energy → hunger", or a reaction or receptor, works twice as strongly, since those add up; a doubled instinct or setting is a spare that does nothing while the first works, but a later change can turn it into something new.
- **Any gene can be lost,** vital ones too (Q2). Now and then a hatchling is born without its urge to eat, drink or rest: it has to learn it, from the player or by chance, or it dies young. Nature keeps no list of genes that can't be lost, and the engine enforces physics only. A copy always keeps at least one gene this build can read: when every one before it has been lost, the last isn't. So a genome never ends up with nothing this build can read, though the gene it keeps may do nothing.
- **A changed gene changes one thing it does,** each as likely as the others: what an instinct reacts to (one of its inputs, negated or not as it was) or its action ("hunger → eat" becoming "hunger → play"); what an attention instinct reacts to, what it draws the eye to, or whom it's about ("loneliness → attends to sprite" becoming "thirst → attends to sprite", or "anxiety → attends to its young (sprite)" becoming "anxiety → attends to its partner (sprite)", Slice 32); or, for any gene, the life stages it works in, one of the six ways of switching that ever work (§9.2). A trait works all its life, and a starting level only at birth, so neither has life stages to change. Each choice is one the pack has, other than what the gene had. Attention takes only State inputs, since it feels nothing else (M1 design §4.3).
- **Three changes in four are made on a fresh copy** (`on_copy`, 0.75), right after the original, so the original still works; the rest change the gene in its place.
- **Every changed gene is checked again,** as a genome file's genes are, and one the world can't have, such as an action reserved for a later milestone, isn't made. No change makes a gene that worked change the body directly: a gene that would is flagged and does nothing, as ever (M1 design §4.3). Property tests with random genomes pin both, and that a flagged gene carries no flaws.
- **Mating stays between adults,** whatever a changed gene says: the engine enforces it for any genome (§5).
- **A genome never grows past twice the starter's length** (`longest`, 2, so 236 genes since Slice 28, 226 in 32b, 188 in 32a and 134 before): a copy is made only while it fits. A pair's child whose parents' genes together would be longer drops genes only one parent has, the last first, until it fits. A genome already longer, as a genome file's may be, doesn't grow.
- **A gene this build can't read, or one that names a category the world lacks, is kept exactly as it is** (M1 design §2.8, v19 §5.7): never doubled, lost or changed, and no roll of the dice is spent on it.
- **A sprite remembers what copying doubled, lost and changed,** with what each lost or changed gene was and when it worked, for the Genome tab and the log (§13.2): world state, saved, in the state hash, and checked every tick (M1 design §7.1): a doubled gene does what the gene before it does; a changed gene differs from what it was, and one changed on a copy follows its original.

### 13.2 What the player sees

- **The Genome tab's list of what's new** (§12.6) gains three kinds of line, after where the genes came from: each gene changed, with what it was (in its place) or what it's a copy of (on a copy); then the genes doubled; then those lost.

```
│ FROM PIP #12 AND BO #7                     │
│ new: thirst → attends to sprite, from      │
│   loneliness → attends to sprite           │
│ doubled: low energy → hunger               │
│ lost: pain → retreat                       │
│ grows old 25,104, was 23,880               │
│                                            │
│ TRAITS                                     │
```

- **A line in the log when a hatchling has a changed gene** (Q3, B), not for doubles or losses, so it won't bury the log:

```
Ann #31 hatched (Pip #12 and Bo #7's), with something new: thirst → attends to sprite
```

### 13.3 How we know it worked

Agreed before measuring (Q4). The setup is §12.7's: the default world left alone for **200,000 ticks** (about 45 hours at 1×, about 20 generations), seeds 1–10.

- **New genes take hold:** in **at least 5 of 10 seeds**, a gene the starter doesn't have (a changed one, not a plain double), by what it does and when it works, is carried by **at least a tenth** of the hatched sprites alive at the end.
- **Safe:** tests with random genomes show that no change makes a gene that worked change the body directly, and no genome grows past twice the starter's length.
- **Keeps going:** sprites alive at the end in **at least 8 of 10 seeds**, never more than the limit.
- **Nothing else got worse:** A1–A4 still met (A3 on seeds 1–30), and A5 at least 200 ticks a second.
- **For information:** which new genes spread and how far, and in how many of their carriers each works (a spare copy changed to work only at another life stage may do nothing yet); how many genes the living have lost and doubled; and genome lengths.

"Born knowing what its ancestors learned", such as "thirst draws the eye to water", needs two changes in a row on one line of descent, so it isn't expected within these 45 hours: it belongs to Slice 27's fast-forward and Slice 29's harsher world, where §12.7's G4 is measured again too.

`cargo run --release -p terra-sim --example generations -- --ticks 200000` runs the first and third and reports the rest. G4's numbers now line up with the starter's by name, the nth of a name with the starter's nth, so a sprite with a gene doubled or lost still counts. Since Slice 29, a name says when its gene works too (`until Adult`, `from Adult`), so each half of a gene split by life stage, as the starter's Wander is, lines up with its own (§19.6).

**Measured (2026-10-13)** in release builds of this part at `d8a33c8`, in the cloud, on seeds 1–10 (A3 on 1–30), with:

```
cargo run --release -p terra-sim --example generations -- --ticks 200000
cargo run --release -p terra-sim --example lab -- scenarios/viability.ron --seeds 10
cargo run --release -p terra-sim --example lab -- scenarios/a3-correct-training.ron --seeds 30
cargo run --release -p terra-sim --example speed
```

The default world left alone for 200,000 ticks; the genes are those of the hatched sprites alive at the end. "Carried" is the share with the gene, and "works" the share in which a copy of it is expressed at the sprite's life stage now:

| Seed | Alive at the end (and eggs) | Eggs alone | Generations | New genes carried by a tenth | Of those, working in a tenth | Most widely carried new gene (carried, works) | Starter genes lacked, each | Extra copies, each |
|---|---|---|---|---|---|---|---|---|
| 1 | 98 (1) | 105 | 21 | 1 | 0 | was_hit → attends to sprite from Adult (10%, 0%) | 1.15 | 0.18 |
| 2 | 94 (5) | 479 | 18 | 2 | 1 | female_near → Wander (85%, 85%) | 2.14 | 0.82 |
| 3 | 98 (2) | 153 | 22 | 3 | 2 | pricked → pain until Old (24%, 20%) | 1.17 | 0.37 |
| 4 | 97 (3) | 162 | 18 | 1 | 0 | h0's half-life from Old (12%, 0%) | 1.72 | 0.08 |
| 5 | 99 (1) | 180 | 19 | 0 | 0 | low fertile → arousal from Adult (2%, 2%) | 0.77 | 0.18 |
| 6 | 97 (2) | 209 | 18 | 1 | 0 | drank → thirst until Adult (25%, 6%) | 1.01 | 0.02 |
| 7 | 99 (1) | 277 | 18 | 3 | 3 | low hydration → thirst from Adult (42%, 36%) | 0.45 | 0.16 |
| 8 | 95 (5) | 627 | 18 | 1 | 0 | arousal's half-life from Old (14%, 0%) | 2.05 | 1.15 |
| 9 | 94 (6) | 293 | 19 | 0 | 0 | plasticity (2%, 1%) | 1.96 | 0.51 |
| 10 | 97 (3) | 120 | 22 | 1 | 0 | hunger → exploration_mod from Adult until Old (12%, 7%) | 0.31 | 0.10 |

- **New genes take hold: met,** in 8 of 10 seeds. In 5 of those 8, though, the gene that spread works in under a tenth of the living: a copy changed to work only at another life stage, either a spare that the original still overrides, such as a half-life or an attention instinct, or a feeling that works only in the young. A copy that does nothing costs nothing, so it drifts. A new gene that works was carried by a tenth or more in 3 seeds: "always → wander" changed in its place to "a female near → wander", in 85% of the living in seed 2; a second thirst feeling for adults, "low hydration → thirst from Adult", in 42% in seed 7; and "pricked → pain until Old", "low female_near → feminiser", "played_social → boredom from Adult until Old" and "always → approach", each working in 11% to 20%.
- **Safe: met.** The property tests with random genomes pass: no change makes a gene that worked change the body directly, and no genome grows past twice the starter's length, a pair's child's included. The living's genomes were 63 to 70 genes long.
- **Keeps going: met.** Sprites are alive at the end in all 10 seeds, 18 to 22 generations deep, and no world went over its limit of 100.
- **A4: met.** Of 1,032 deaths in ticks 10,000–50,000 of the viability run over all seeds, 3 were of thirst and none of hunger (0.3%, under 25%).
- **A3: met.** Over seeds 1–30, trained sprites hit a median of once in ticks 10,000–20,000, against 48.5 times untrained, and all of 100 random sets of 10 of those seeds pass.
- **A5: met,** 207–239 ticks a second with 100 sprites (`speed`, four runs). `main` at `b39b719`, timed alongside, ran 225–238 and this part 208–242, so it costs no speed; the cloud machine was slower than when §12.7 measured 245–260.
- **A1 and A2: met** (their tests pass).
- **For information, what copy and change cost.** Against §12.7's run, which had copying errors and flaws but no copy and change: hunger and thirst killed 322 of 6,325 sprites that died after tick 10,000 (5.1%, against 0.6%), and 7% to 16% of them in seeds 3, 6, 8 and 9; 105 to 627 eggs a seed were laid alone (69 to 185), tries to mate that came to nothing rose to 1,984–8,442 a seed (1,169–3,428), and the living had 2.05 to 7.51 broken genes each (0.98 to 2.73). Seeds 8 and 9, run again with the report's list of the starter's genes the living lack, show why: the only starter genes a tenth or more of the living lacked were "always → wander" and "male_near → longing", the gene by which a male near lowers the longing that makes a female breed alone. Each was lacked by every sprite alive at the end in at least one of the two seeds, and the living lacked about two of the starter's genes each. Our reading: without the first, a sprite doesn't roam for food it can't see, and selection was already weakening it (§12.7); copy and change lets it go in one step. Without the second, a female's longing builds with males around, so she breeds alone more often; a lone egg's young have each flaw their mother carries broken in about half of them (§12.3), hence more broken genes.
- **For information, lives:** the 5,777 hatchlings old age killed lived a median 6.4 hours, a tenth of them under 4.7 and nine tenths under 7.5 (§12.7: 6.7, 4.9 and 7.6). In the second half the median was 5.1 to 7.2 hours a seed.
- **For information, G4:** 0 of the 83 numbers moved a tenth the same way in all 10 seeds (§12.7: 1, `grows_old`, which now rose in 8, by 33% in the median seed).

### 13.4 Later

- **The error rate as a gene:** the owner wants it pinned (round 1, Q2); [#194](https://github.com/Keazra/terra-sprites/issues/194).
- **Born knowing what its ancestors learned:** measured with Slice 27's fast-forward and Slice 29's harsher world (§13.3).
- **Breeding alone can take over:** in one of seeds 8 and 9 every sprite alive at the end had lost the gene by which a male near lowers a female's longing, and in the other at least a tenth had (§13.3); seed 8 laid four times as many lone eggs as in §12.7's run. Courtship and families, Slice 32 ([#186](https://github.com/Keazra/terra-sprites/issues/186)), should check pairs still pay. It makes breeding alone the body's rule, which no gene can loosen (§15.7).

---

## 14. The population graph (Slice 26)

Settled with the owner on 2026-10-13 ([#144](https://github.com/Keazra/terra-sprites/issues/144)), in a round of four questions whose recommendations they kept. A graph shows how many sprites the world has had over its whole run, so the player can watch the population rise and fall (G3). Today the default world breeds up to its limit of 100 by about tick 36,000 on seed 7 and stays near it (§14.4's graph), so the total climbs and then runs nearly flat until Slice 29's harsher world. What keeps moving is the mix: waves of young growing up, and adults growing old. So the graph stacks the life stages (Q1, B).

### 14.1 The population record

- **The world counts its sprites by life stage,** young, adults and old, **and its eggs,** which are items until they hatch (§10.4), on the map or in the Cursor, at the first tick it runs and then every 100 ticks, with **the oldest and newest generations among its sprites** (§17.1, [#208](https://github.com/Keazra/terra-sprites/issues/208)), not its eggs, and what happened since the count before: the eggs laid and hatched, the sprites the Cursor spawned, and the deaths by cause. A count keeps its generations when the record squeezes (below); they're two small numbers, a few bytes a count in a save.
- **It's world state:** saved (M1 design §2.8), in the state hash, the same in a replay (M1 design §2.7), and checked every tick (M1 design §7.1): its counts go forward in time and never reach 512, and a count's generations run from the oldest to the newest, with none in a count of no sprites (#208). Nothing in the world reads it, so the sprites live exactly as they did before it (§14.7).
- **It always covers the whole run, and a save stays small.** Once it holds 512 counts, it keeps the first and every other one after it (for a record that began off the beat, as an older save's may, the first and those on the new beat), folds what happened in the dropped ones' stretches into the next one kept, and counts half as often: every 200 ticks from about tick 51,000, every 400 from about 102,000, and so on. It never holds more than 511 counts.
- **A save from before this slice** has no counts, so its record begins at the first tick it runs after loading. The save format stays 1: the record is new, with a default (M1 design §2.8).
- **A save from before the record kept generations** has counts without them, so their moments read no generations (§14.3), and the counts after the load have them (#208). The save format stays 1, with a default. A replay recorded before parts from its recording, as one does between builds that share a version number.
- **The world keeps it, not the screen,** so a loaded world and a replay show the whole graph, and a world fast-forwarded in Slice 27 will open with it.

### 14.2 The graph

- **`p` opens it over the panels,** as `l` opens the sprite list (M1 design §6.1), and `p` or `Esc` closes it. Time runs on behind it, and `space`, `.`, `+` and `-` work as ever (Q2), so the player can watch it at 4×. Other keys do nothing while it's open, except those below, and the pointer picks moments rather than moving the Cursor on the map beneath. Its title says the ticks it covers.
- **The bands stack from the bottom, youngest to oldest:** the eggs, then the young, adults and old (Q1, B; the eggs at the bottom are the owner's call, 2026-10-14, as it reads more aptly). Each is in its theme colour: eggs light yellow, as an egg is on the map, young light green, adults cyan and old gray. A legend under the graph names them in the same order.
- **Each column is split by share** (the owner's pick, 2026-10-14, after trying the built graph: near the limit, a stack of how many was a flat block whose thin bands were hard to read). It fills the height, split by each band's share of it, eggs included, and the vertical axis is labelled `100%`, `50%` and `0%`. Every band with any in it gets at least half a character, room allowing, so a small one is never lost, and the rest go by largest remainder. The legend ends `as shares of each column`. How many there were is in the lines under it (§14.3) and in the small graph (§14.4).
- **Each row of characters is two halves:** a character of one band is `█`, and one of two bands is `▄` in the lower band's colour on the upper's, or on nothing at the top of the stack.
- **Each column is a stretch of time,** showing the count at its end. Until the run is as long as the graph is wide at 100 ticks a column, the graph grows from the left; after that the whole run is squeezed in, its columns standing for equal stretches. The time axis is marked at each quarter, labelled with its tick, rounded to the hundred.
- **It fills the screen,** at any size the game allows.

At 100×30, the default world (seed 7) at tick 48,000, with a moment at tick 32,300 picked, which reads the count at 32,700, the end of its column. Each column is about 530 ticks. The page has no colour, so the bands are shades here, as in the round: `░` eggs, `▓` young, `█` adults and `▒` old; a character of two bands shows the lower.

```
 Terra Sprites │ tick 48,000 │ ► 1x │ seed 7 │ sprites 99 │ not saved                        ? help
┌─ Population ── ticks 0 to 48,000 ──────────────────────────────────────────────────── esc close ─┐
│ 100% ┤▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
│      │█▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒█▒█▒▒██████▒▒▒▒▒▒▒▒▒█▒▒▒▒▒▒▒▒█▒███▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒▒│
│      │███▒▒▒▒▒▒▒▒▒███▒▒▒▒▒▒█████████▒▒█████████████████████████████████████████████▒▒▒███████████│
│      │████▒▒▒▒▒▒█████████████████████████████████████████████████████████████████████████████████│
│      │███████████████████████████████████████████████████████████████████████████████████████████│
│      │███████████████████████████████████████████████████████████████████████████████████████████│
│      │███████████████████████████████████████████████████████████████████████████████████████████│
│      │███████████████████████████████████████████████████████████████████████████████████████████│
│      │███████████████████████████████████████████████████████████████████████████████████████████│
│  50% ┤███████████████████████████████████████████████████████████████████████████████████████████│
│      │███████████████████████████████████████████████████████████████████████████████████████████│
│      │██████████████▓▓▓▓█████████████████████████████████████████████████████████████████████████│
│      │██████████▓▓▓▓▓▓▓▓▓▓▓█▓██▓█████████████████████████████▓███████████████████████████████████│
│      │███████▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓███████▓▓▓▓██▓█████▓▓██▓█▓▓▓██▓▓▓▓▓███████▓████████████████████│
│      │███████▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓█████████████████│
│      │███████▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓██████████▓▓▓▓▓│
│      │█████▓▓░░▓▓▓▓▓▓▓▓░▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓███▓▓▓▓▓▓▓▓▓▓│
│      │█░▓▓▓▓▓░░▓░░▓▓░▓▓░░▓▓▓▓▓▓▓▓▓▓▓▓▓░▓▓▓▓▓▓▓░▓▓▓▓▓▓░░▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│
│      │█░░▓▓░░░░░░░▓▓░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░▓▓░░▓░░░░░░░░░░░░░│
│   0% └─────────────────────┬─────────────────────┬─────────────────▲───┬─────────────────────┬───│
│      0                11,600                23,200                34,800                46,400   │
│                                                                                                  │
│ ░ eggs  ▓ young  █ adults  ▒ old  as shares of each column                                       │
│ tick 32,700: 84 sprites · generations 3 to 6                                                     │
│ 3 eggs (3%) · 20 young (23%) · 53 adults (61%) · 11 old (13%)                                    │
│ since tick 32,100: 1 laid · 2 hatched · 2 died (old age 2)                                       │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
 (128,80) dirt · berry (fresh) │ SELECT         ←→ pick a moment  space pause  +/- speed  esc close
```

### 14.3 Reading a moment

- **`←` and `→`, or `A` and `D`, or the pointer over the graph pick a column** (Q3, B); a click picks too. `▲` under the graph marks the column read: with nothing picked, the last, which reads now, as `now, tick 48,000: …`.
- **The three lines under the graph read it:** the tick, how many sprites and **the generations alive then** (#208), `generations 3 to 6`, or `generation 1` when there's one, as the World tab says them (§17.5), or no generations with no sprites or in a count from before the record kept them (§14.1); then the eggs, which aren't sprites until they hatch (§10.4), and the count in each stage, each with its share of the column; then what was laid, hatched, spawned and died since the column before, with the causes, or "none laid, hatched or died". The first column reads from the first count, and the first count itself says so. A column narrower than the time between counts, on a wide screen once the record has squeezed, shows the count before it again, and reads it.
- **Three lines, not two** (the owner's pick, 2026-10-14): #208 put the generations on the first line, after the count of sprites, but that line then ran to about 110 characters, past the box at the game's smallest screen, 100 columns, cutting off the old. So the eggs and the stages have a line of their own, and the graph gives up a row for it.
- **A pick is a moment,** the end of the column's stretch of time, so as time runs and the graph squeezes, `▲` stays on the column that takes it in. Picking the last column goes back to reading now, which moves on with time. The arrows stop at the first column and at now, and closing the graph forgets the pick.
- **The status line's key hints** read `←→ pick a moment  space pause  +/- speed  esc close`.

### 14.4 The World tab

- **A small copy heads the World tab** (Q2, C), the tab the game starts on: a heading with the ticks it covers, six rows and the axis, as wide as the tab, then the tab as before. It shows how many, not shares: its height runs from 0 to the world's limit, most sprites (§10.6), drawn as a dotted dark gray line where nothing else is (the round said gray, but old sprites are gray), or to the most there have ever been, if the Cursor spawned sprites past it, labelled at the top, the middle and 0. Each half shows the band at its middle, and the top of the stack shows if it reaches into a half at all, so a lone egg or sprite is never lost. It has no lines that read a moment; `p` opens the full graph for that.

The same world and tick:

```
┌─ Body Brain Chem Genome [World] ───────────┐
│ population, ticks 0 to 48,000              │
│ 100 ┤·······················▄▄▄▒▒███▒▒▒▒█▒█│
│     │                  ▄▄▒▒████████████████│
│     │        ▄▄▄▄▄▒▒▒██████████████████████│
│  50 ┤  ▄▒▒█▒███████████████████████████████│
│     │██████████████████████▓▓▓▓▓▓▓▓████████│
│     │██▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓│
│   0 └────────┬────────┬────────┬────────┬──│
│                                            │
│ …                                          │
```

### 14.5 Themes

Both built-in themes gain a `population` section (`docs/reference/themes.md`): the colour of each life stage and of eggs, the whole and half blocks, `█` and `▄`, or `#` and `_` in ASCII, and the dot of the small graph's limit line, `·` or `.`, in dark gray. A theme without the section, or without some of it, draws what it leaves out as the default theme does, as with space (§11.5).

### 14.6 The information policy

The graph and its small copy are a panel of their own, `Graph`, about the world (M1 design §6.4). Denied, the full graph's box stays, empty and titled ` Population `, and the World tab leaves the small graph out.

### 14.7 How we know it worked

Agreed before building (Q4):

- **UI tests** (ratatui's `TestBackend`) of the full graph and the small one, each column split by share in the theme's colours, the eggs at the bottom, with a small band kept, the graph growing from the left and then squeezing, a picked moment, which stays picked as time runs, a death read in its moment, a column narrower than the time between counts, the small graph's height past the limit, nothing beneath showing through, time running while it's open, the graph after a save and a load, and the policy hiding it; snapshots of the graph in both themes. The simulation's own tests cover the record: counts at the right ticks, eggs apart from sprites, layings, hatchings, spawns and deaths counted in their stretch, squeezing over 110,000 ticks, saving, and a save from before it.
- **The sprites live exactly as before:** the `generations` run, seeds 1–4 for 30,000 ticks, prints the same, byte for byte, on `main` and on this slice.
- **No slower:** the speed check (A5) at least 200 ticks a second, and within 5% of `main` measured the same way.

**Measured (2026-10-13)** in release builds of this slice at `6b4bd45`, beside `main` at `82fad3f`, in the cloud, with:

```
cargo run --release -p terra-sim --example generations -- --seeds 1-4 --ticks 30000
cargo run --release -p terra-sim --example speed
```

- **The sprites live exactly as before: met.** The `generations` report is the same, byte for byte, on both, and again beside `main` at `3b2cd0f`, once eggs were items. The state hash differs, as it should, since the record is world state.
- **No slower: met.** The speed check ran 292 and 291 ticks a second on `main`, and 286 and 299 on this slice, run in turn.
- **The UI tests and snapshots pass,** as do the simulation's tests of the record.

**Generations on the graph** ([#208](https://github.com/Keazra/terra-sprites/issues/208)) were checked the same way, measured on 2026-10-14 in release builds of its branch at `28eed2d` beside `main` at `97c1175`, in the cloud, with the same two commands:

- **The sprites live exactly as before: met.** The `generations` report is the same, byte for byte, on both.
- **No slower: met.** The speed check ran 190 and 174 ticks a second on `main`, and 188 and 178 on the branch, run in turn: the same, within the noise. As for Slice 25 (§17.7), this container was too slow for 200 on either.
- **UI tests** read the generations at a moment and now, none at a moment from a save made before them, and the three lines in both themes' snapshots; **the simulation's tests** cover the generations in each count, a squeeze keeping them, a replay playing them back, and a save from before them.

### 14.8 Later

- **Generations on the graph** are built ([#208](https://github.com/Keazra/terra-sprites/issues/208), §14.1, §14.3).
- **A gene's middle value over time, and the counts in a file,** are fast-forward's tables (Slice 27, §16.4).

---

## 15. Courtship and families (Slice 32)

Settled with the owner on 2026-10-13 ([#186](https://github.com/Keazra/terra-sprites/issues/186)), in two rounds of questions. Until this slice, two aroused adults of different sexes who happened to look at each other mated, and that was all. Nothing kept the pair together, and a mother walked away from her egg. Sprites met only by chance, since a sprite sees about 1% of the default map. So a female who had gone a while without a male laid alone, and in Slice 24b's long runs breeding alone spread once a gene that held it back was lost (§13.3).

The owner's direction: **lone eggs very rare, and a social species that lives in family units**, whose sprites find mates through courtship and attraction. So:

- sprites that want a mate **call**;
- a call brings two strangers close enough to **get to know each other**;
- then he **dances**, she chooses, and if she likes him she dances back;
- once her **receptivity** is high enough, they mate;
- after their first egg they're **partners**;
- both keep near their egg, and near their young until the young grow up, held together by **anxiety**;
- a female can breed alone only after about an hour with no male seen or heard.

**The world and the genes.** The engine enforces physics and content safety only (§5), so the world says who's whose, how far a call carries, when a body can breed, and who may court whom. The genes say what a sprite feels about all of it, so evolution can change it.

| The world (data, so a mod can change it) | The genes (vary from sprite to sprite, and evolve) |
|---|---|
| Who's whose: a sprite knows its partner, parents, young and eggs, and its kin | How strongly its own draw its eye, and how much it avoids its kin |
| A call carries round walls, and the hearer knows who called and from where | When to call, and whether to answer |
| A pair can mate once her receptivity is high enough | What raises it: being danced for, and dancing back |
| A female can breed alone only after about an hour with no male seen or heard | Whether she wants to |
| Only adults court, mate or answer a call to mate (§5) | — |

**Two parts.** **32a** builds calls, getting to know each other, courtship, partners, kin and the lone-egg rule. **32b** builds families: anxiety, eggs and the young. 32b builds on eggs as items ([#196](https://github.com/Keazra/terra-sprites/issues/196)). The numbers are data, the `calls` and `breeding` sections of `physiology.ron`, `terrain.ron`, and the starter genome: first guesses, tuned against §15.9. A save from before this slice keeps the pack it was made with (M1 design §2.8), which has no calls or courtship.

### 15.1 Calls

- **`Call` is a new action,** with no target, like Rest. The sprite calls once and stands listening, `calls.listen` ticks (10) in all, the tick it calls included, and the action ends (#186). It costs `calls.cost` energy (0.002), so sprites don't call all the time.
- **A call travels like a wave** (the owner, round 2). It spreads from the caller over every tile that carries sound, around anything that doesn't, and through doorways, and it weakens along the way. Each terrain says in `terrain.ron` how much sound it soaks up per tile, `sound`, and a terrain with none, such as rock or, in M4, the ship's hull, stops it. A call is heard wherever its way there costs at most `calls.reach` (300). Its way is measured as a sprite's is (M1 design §3.1): a step costs the tile's `sound`, a diagonal step 14/10 of it, and it doesn't cut a corner past a tile that stops it. In M2's terrarium every open terrain, water included, soaks up 10, so a call carries about 30 tiles straight and 21 on the diagonal, three times as far as a sprite sees. That covers about six times as much ground: about 6 other sprites in a full default world, against about 1 in sight. Objects don't stop sound. On the ship, plating can carry a call further than grass (M4 design §3.2).
- **A hearer knows who called and where from.** It feels a `heard` pulse. For `calls.heard_for` ticks (150) the caller is offered among the sprites it could attend to, at the edge of sight, as a remembered place is (§7.3), so a sprite in sight draws the eye first. What draws the eye to a heard call fades over that time. Body language reads a heard caller as if it were in sight: its arousal, its sex, whether it's kin.
- **Going to a call is a trip** (§7.4), to the tile the call came from, by the sprite's own way there. With no way there, it doesn't set off, and forgets the call. Once the caller is in sight, the trip is an ordinary action aimed at it. If the caller calls again from elsewhere on the way, the trip heads there instead; arriving where it called from with the caller still out of sight, the sprite has lost the trail, fails, and forgets the call (#186).
- **A call stops at a sector's hull** but goes through its doorways, so on the ship calls stop at the hull as counting and sight do (M4 design §3.1).
- **Hearing counts for sex by neighbours** (round 1, Q4). A sprite that has heard a female adult call within `heard_for` ticks counts as having a female near (`female_near`), and the same for a male (`male_near`). So a male that hears females stays male, even where sprites are thinly spread (§10.1).
- **The starter genes:** an aroused sprite with no adult of the other sex in sight or heard calls. The brain reads `male_near` as well as `female_near` for it (#186). An aroused sprite that hears a possible mate turns its eye to the call and goes to it. Family uses calls to keep in touch (§15.6).

### 15.2 Getting to know each other

- **A sprite has a familiarity with each sprite it remembers:** how well it knows that individual, from 0 to 1. Until now familiarity was only of object types and of sprites as a whole (M1 design §5.6).
- **It grows while the sprite sees the other,** by its own sight, so one that sees further may know the other better than it's known. It grows by the brain parameter `befriend_rate` (0.0013 a tick, so about 750 ticks, 10 minutes at 1×, from strangers to well known), and faster when they play together: a social Play between them adds `befriend_play` (0.1). It fades by `befriend_fade`, as slowly as a good lesson (0.000025). Seeing a sprite is enough to remember it: each tick, after the fading, familiarity with a sprite in sight grows from at least `forget_below`, so one in sight is never forgotten (#186). One it knows only by sight, having learned nothing of its worth or to fear it, is judged as sprites in general are, as a stranger was before, and isn't counted in that summary; the Brain tab shows only how well it's known (#186). A sprite is forgotten once all it has learned about it, familiarity included, has faded below `forget_below`, or once it dies (M1 design §5.6). Knowing an individual is learned, so with learning off, in a lab's control run (M1 design §5.6), sprites grow no more familiar with each other, though familiarity with object types still grows (#148).
- **Body language reads it:** `target_familiar`, how well the sprite knows the one it attends to.
- **The starter genes court only a sprite they know well,** so strangers brought together by a call spend time together first. If that proves too restricting when measured, the owner's call (round 1) is to drop it from the starter genes.

### 15.3 Courtship

- **`Court` is a new action,** aimed at the sprite attended to: the sprite walks up to it and **dances** once, as Play walks up and plays. The one danced for feels a `courted` pulse, and the dancer a `danced` pulse. It's offered only when both are adults (§5), as Mate is.
- **Receptivity** is a new hormone. The starter genes raise it a little when a sprite is danced for, much more when it dances back, and it fades within about ten minutes. **Mate lays an egg only when the female's receptivity is at least `breeding.receptive` (0.5),** on top of §10.3's rules. Otherwise it's a fruitless try. This is the owner's "hormonal angle" (round 1, Q2): evolution can change what raises receptivity and how fast, so courtship is kept by its worth, not by a rule.
- **He dances, she chooses** (round 1, Q1). In the starter genes, an aroused male dances for an aroused female he knows, waiting a little between dances, until she's receptive: body language reads her receptivity, `target_receptivity` (#186). A female lately danced for by a male she knows dances back, more readily the healthier he looks. Being danced for raises her receptivity a little and dancing back much more, so two or three dances each make her receptive, and his dances alone seldom do (#186: when his alone could, many pairs' eggs came with no dance back). What leaves her inclined to dance back is being danced for by anyone, for a minute or so; she dances back for the familiar sprite she's looking at, which is almost always the one beside her who danced. Then arousal with an aroused sprite of the other sex leads to Mate, as before (§10.3), but the starter genes hold Mate back while she isn't receptive and with kin (§15.5, #186). To get to know each other, an aroused sprite plays with a stranger of the other sex, and an aroused one goes to its partner (#186). A sprite of either sex can carry genes to court; the starter's are a male's and a female's because sprites change sex (§10.1).
- **She chooses by how healthy he looks** (round 2, Q2). Body language reads it as `target_condition`, from a body sensor, `condition` (#186: "health" is already a gene's soundness): the sprite's energy, hydration and freedom from injury, times its genes' average health (§9.4). So a male with broken genes (§12.4) looks poorer, and choosy females pass on fewer flaws. How much it matters to her is in her genes.
- **Gifts of food,** as part of how sprites get along, wait for sprites that can carry things ([#135](https://github.com/Keazra/terra-sprites/issues/135)).

### 15.4 Partners

- **Two sprites that have an egg together are partners** (round 1, Q5). Each remembers the other as its partner, which replaces any partner it had: that one is freed, and the log says they parted (#186). It's world state, saved and in the state hash.
- **Partners stay together for life,** in the starter genes: they keep near each other (§15.6), and court only each other while both live.
- **A partner is freed** when the other dies, with a `lost_partner` pulse, or after `breeding.apart` ticks (4,500, an hour at 1×) in which neither has seen or heard the other. Each keeps when it last saw or heard its partner, a call counting as heard for as long as it's remembered (`heard_for`, 150 ticks), and both are freed together once both times are that long ago. Then each may pair again.
- **Mourning is in the genes:** in the starter, losing a partner raises a hormone, `grief`, that holds arousal down for about 1,500 ticks (20 minutes at 1×). Geese and swans pine after losing a partner, and swans often pair again.
- **What a sprite feels about its partner:** a body sensor, `partner_near`, is 1 while it sees its partner or has heard it call within `heard_for` ticks, and another, `paired`, is 1 while it has one (#186), so partners can court only each other; body language reads `target_partner`. A partner who died is still named, as the Body tab shows, until the sprite pairs again.

### 15.5 Kin

- **A sprite knows its close kin:** its parents, its young, and its brothers and sisters, sprites with a parent in common. Body language reads `target_kin`.
- **Avoiding kin is in the genes** (round 1, Q6). The starter genes weigh strongly against courting kin, so it happens only now and then, as when no one else is about. Nothing forbids it. Kin share hidden flaws, so a child of two kin is more often born with broken genes (§12.3), and evolution can keep or lose the avoidance.

### 15.6 Families and anxiety (32b)

- **Both parents keep near their egg** until it hatches (round 1, Q7), and near their young until the young grow up (Q8). One parent can leave to eat or drink while the other stays with the egg. Once grown, nothing holds the young: as they grow up, they stop being their parents' young, or family, so the pull ends then, rather than fading as they grow (#186); what anxiety is left fades within minutes, and they leave when their needs take them away. An orphan fends for itself.
- **Anxiety** is a new drive and need, second-order like loneliness (round 2, Q4). It's what holds a family together, and what it rises with is in the genes. The world gives the senses: body sensors for an egg of its own out of sight (`egg_away`), out of sight and left with no other parent by it when the sprite last saw it, or never seen (`egg_alone`; by it is in the sprite's sight and seeing the egg, #186), a young child out of sight (`young_away`), a parent out of sight for a young sprite (`parent_away`), and a partner out of sight (`partner_away`); and a pulse, `found_family`, when one of those goes off because the one it was about is back in sight, not because it hatched, died, grew up or was freed (#186). Out of sight is the sprite's own sight: within its sense radius, rounded, counted in tiles as the larger of across and down, which is how §15.9 measures it too. An egg the Cursor holds is in no one's sight. In the starter genes, a parent's anxiety rises while its egg is out of sight, and four times as fast if it left the egg alone. It rises while its young or its partner are out of sight, and faster in a young sprite while no parent is in sight. It falls by 0.5 at once on `found_family`, and fades within a few minutes. Hunger and thirst come first (M1 design §5.6): past 0.4, they drain it, the more the hungrier or thirstier, up to 0.03 a tick, so a hungry parent mostly still goes to eat (#186; §15.9 measures what hunger and thirst kill).
- **Finding them eases it,** so a sprite learns its family is good for it, and care grows with practice. Anxiety is care: the starter's never comes from the breeding drive, and whatever the genes, an attention instinct about the sprite's young, family or kin that reads the breeding drive draws no eye to a sprite not grown up (§5).
- **Its own draw its eye.** An attention instinct can name **whom** it's about, as well as the category: its family (its partner, its parents while it's young, its young and its eggs), its kin, or one kind of family: its partner, its parents or its young (#186). Then it draws the eye to such a sprite or egg, and not to others of its kind. Copy and change (§13) can change whom a gene is about. In the starter genes, each family sense draws the eye to the one it's about: a young one away, to its young; no parent in sight, to its parents; its partner away, to its partner; and its egg away, to its eggs. Anxiety draws it to all its family, and arousal draws it away from kin. With family alone, anxious partners whose young had wandered off looked at each other, not at the young one calling (#186). The egg category offers a parent its own egg when it reaches one, not only the nearest.
- **Who's family is a brain input,** `target_family`: 1 while the sprite or egg it attends to is its family. Like what it knows of others (§15.2), it's the sprite's own mind, so `Target(Family)` is refused. In the starter genes, an anxious sprite goes over to its family, and a sprite is held back from hitting its family: a family together is a crowd, and crowded adults hit their partners and young until they were held back (#186). It's a lean, not a rule: any two of being cornered, crowded (adults only) and just hit, each at full strength, outweigh it, so a sprite can still turn on its family, as can one trying something new (#148).
- **A parent remembers where it last saw its egg,** and whether it saw another parent by it then. With the egg out of sight, it's offered at the edge of sight like a remembered place (§7.3), drawing the eye as fully as one just seen, so an anxious parent goes back to it on a trip (§7.4). One that finds the egg gone, as when the Cursor has moved it, forgets the place, and so does one that hatched. The memory is the sprite's, in the world's state, so it's saved.
- **Calls keep the family in touch:** in the starter genes, an anxious sprite calls, and one that hears its family call goes to it.

### 15.7 Breeding alone

- **A female can breed alone only after `breeding.alone_after` ticks (4,500, an hour at 1×) with no male adult seen or heard** (round 1, Q9), a call counting as heard for as long as it's remembered (`heard_for`). It's the body's rule, counted in the world, and it replaces the gate on longing, `breeds_alone` (§10.3). So no gene can make lone eggs common: in Slice 24b, the loss of the gene by which a male near lowers longing did just that (§13.3). Komodo dragons and some sharks have laid eggs alone in zoos, only when kept from males.
- **Longing stays as whether she wants to.** The starter's "arousal and longing in a female → breed alone" is unchanged, but longing now builds over about the same hour (#186): when it built in a few minutes, a female alone tried and failed to breed alone over and over, and never called.
- **A world down to its last few females can still recover,** which a rule of no lone eggs at all wouldn't allow.

### 15.8 What the player sees

- **The map:** a dance flashes `♫` over the dancer and a call `♪` over the caller, in arousal's light magenta, as other emotes flash (M1 design §6.3). In ASCII they're `^` and `"` (#186: `~` is shallow water there). Anxiety colours a sprite when it's its strongest drive above half, in light green, picked in the build with swatches (#186).
- **The doing line:** `Courting Bo #7`, then `Danced for Bo #7` or `Couldn't dance for Bo #7`; `Calling`, then `Called`; `Going to Bo #7's call · 24 tiles to go`, which the detail view marks `· by ear`, as a trip from memory is marked `· from memory` (#186). Mating is reworded, as #190 left for this slice: `Going to pair with Bo #7`, `Paired with Bo #7`, `Couldn't pair with Bo #7`. A parent going back: `Going back to its egg · 12 tiles to go`.
- **The log:** `Pip #12 and Bo #7 are now partners` on their first egg together, `Pip #12 lost Bo #7` when a partner dies, and `Pip #12 and Bo #7 parted` when an hour apart frees them. Dances and calls stay out of the log, since they're frequent.
- **The Body tab:** a line after the parents, `partner Kit #20`, or once the partner has died, until the sprite pairs again, `partner Kit #20 · died` (#186).

  ```
  │ male · fertile · turning 40%               │
  │ parents Pip #12 and Bo #7                  │
  │ partner Kit #20                            │
  ```
- **The Brain tab** lists the new actions, Court and Call, and how well the sprite knows each sprite it remembers.
- The full family tree is Slice 25 (§17), which names a sprite's partner over their young (§17.2).

### 15.9 How we know it worked

Agreed before measuring (round 1, Q11; round 2, Q5). The default world left alone for **200,000 ticks** (about 45 hours at 1×, about 20 generations), seeds 1–10, the run where breeding alone took over in §13.3:

- **Keeps going:** sprites alive at the end in **at least 8 of 10 seeds**, never more than the limit.
- **Lone eggs very rare:** at most **1 egg in 50** laid alone over all ten seeds, and no seed above 1 in 20. In §10.8's runs it was 15% to 31%.
- **Courted first:** at least **9 eggs in 10** laid by a pair come after the two danced for each other, all seeds together.
- **Families stay together** (32b), as the median over all of them:
  - partners have each other in sight for at least **half** the time both are alive and grown;
  - an egg has a parent in sight for at least **three quarters** of its time;
  - a young sprite with a living parent has one in sight for at least **half** its childhood.
- **Safe:** tests with random genomes show that courting, mating and answering a call to mate only ever happen between adults.
- **Nothing else got worse:** A1–A4 still met (A3 on seeds 1–30), and A5 at least 200 ticks a second with 100 sprites.
- **For information:** males and females among the adults; how often partners pair again; how far calls bring mates; hunger and thirst's share of grown-up deaths (5.1% in §13.3); broken genes a sprite; how many hatchlings grow up.

32a is measured on all but the families' row; 32b on all of them.

`cargo run --release -p terra-sim --example generations -- --ticks 200000` runs the first three and reports the rest; its courtship table counts a pair's egg as courted when each danced for the other since the pair's last egg.

**Measured for 32a (2026-10-14)** in release builds of this part at `1d55383`, in the cloud, on seeds 1–10 (A3 on 1–30), with:

```
cargo run --release -p terra-sim --example generations -- --ticks 200000
cargo run --release -p terra-sim --example lab -- scenarios/viability.ron --seeds 10
cargo run --release -p terra-sim --example lab -- scenarios/a3-correct-training.ron --seeds 30
cargo run --release -p terra-sim --example speed
```

The default world left alone for 200,000 ticks. "Courted" is the share of a pair's eggs laid after the two danced for each other; "again" counts pairings by a sprite that had had a partner:

| Seed | Alive at the end (and eggs) | Eggs alone | Courted | Partners made (again) | Calls an hour | Adults at the end, male and female | Generations |
|---|---|---|---|---|---|---|---|
| 1 | 95 (5) | 18 (2.7%) | 91.3% | 502 (446) | 980 | 41, 32 | 23 |
| 2 | 100 (0) | 10 (1.6%) | 92.4% | 455 (384) | 978 | 47, 34 | 22 |
| 3 | 98 (2) | 8 (1.3%) | 92.5% | 466 (389) | 978 | 50, 30 | 22 |
| 4 | 99 (1) | 7 (1.1%) | 95.9% | 489 (413) | 706 | 41, 42 | 24 |
| 5 | 95 (4) | 3 (0.5%) | 94.7% | 492 (439) | 742 | 46, 39 | 23 |
| 6 | 94 (6) | 11 (2.0%) | 91.5% | 433 (383) | 876 | 39, 45 | 22 |
| 7 | 96 (3) | 5 (0.8%) | 91.0% | 513 (455) | 797 | 48, 39 | 21 |
| 8 | 98 (2) | 17 (2.8%) | 93.0% | 481 (432) | 800 | 43, 35 | 23 |
| 9 | 99 (1) | 19 (2.8%) | 88.9% | 510 (440) | 816 | 44, 46 | 23 |
| 10 | 95 (5) | 10 (1.6%) | 91.3% | 518 (485) | 831 | 50, 32 | 22 |

- **Keeps going: met.** Sprites are alive at the end in all 10 seeds, 21 to 24 generations deep, and no world went over its limit of 100.
- **Lone eggs very rare: met.** 108 of 6,232 eggs were laid alone (1.7%, at most 2%), and no seed laid more than 2.8% alone (at most 5%). `main` at `861d157`, run the same way, laid 1,921 of 6,744 alone (28%), 14% to 56% a seed.
- **Courted first: met.** 92.2% of the 6,124 eggs laid by pairs came after the two danced for each other (at least 90%). One egg in all ten seeds was laid by kin.
- **Safe: met.** The property test with random genomes passes, now with one sprite out of the others' sight but within a call's reach: courting and mating happen only between adults, and the self-check finds no Mate or Court that isn't.
- **A4: met.** Of 730 deaths in ticks 10,000–50,000 of the viability run over all seeds, 2 were of hunger and 2 of thirst (0.5%, under 25%).
- **A3: met.** Over seeds 1–30, trained sprites hit a median of 2 times in ticks 10,000–20,000, against 57.5 untrained, and all of 100 random sets of 10 of those seeds pass.
- **A5: met,** 236–246 ticks a second with 100 sprites (`speed`, three runs), against 309–321 for `main` at `27927d7` timed alongside, about a quarter slower. A world 60,000 ticks old or more steps about 200 a second, against `main`'s 255–300: its sprites remember about 45 others each, and the number levels off there ([#210](https://github.com/Keazra/terra-sprites/issues/210)).
- **A1 and A2: met** (their tests pass).
- **For information, partners:** a seed made 433 to 518 pairs, of which 383 to 485 were by a sprite that had had a partner; 241 to 312 parted, after an hour apart or as one paired again, and 157 to 203 lost a partner to death. Partners staying together is 32b's to measure.
- **For information, the rest:** hunger and thirst killed 62 of 5,393 sprites that died after tick 10,000 (1.1%; `main` 1.0%, §13.3 5.1%). The living had 0.46 to 1.18 broken genes each at the end (`main` 0.67 to 6.56): with few lone eggs, few young have their mother's flaws doubled (§12.3). The 4,955 hatchlings old age killed lived a median 6.6 hours, a tenth under 4.8 and nine tenths under 7.7 (`main` 6.5, 4.8 and 7.6). How far calls bring mates wasn't measured. G4: 1 of the 111 numbers moved a tenth the same way in all 10 seeds, `grows_old` again, which rose in all 10.

**Measured for 32b (2026-10-14)** in release builds of this part at `341d08f` (A5 also after it, with the faster lookups, which change no outcome), in the cloud, with the same commands, seeds and bars, and the families' row now measured. "Partners", "eggs" and "young" are the median share of their time that partners each saw the other, that an egg had a parent in sight, and that a young sprite with a living parent saw one:

| Seed | Alive at the end (and eggs) | Eggs alone | Courted | Partners made (again) | Partners | Eggs | Young | Calls an hour | Adults at the end, male and female | Generations |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 97 (2) | 2 (0.3%) | 93.9% | 326 (149) | 57% | 95% | 74% | 1,977 | 46, 43 | 19 |
| 2 | 96 (4) | 0 (0.0%) | 91.5% | 312 (143) | 56% | 94% | 72% | 2,550 | 55, 36 | 17 |
| 3 | 98 (2) | 1 (0.1%) | 93.2% | 324 (128) | 57% | 95% | 75% | 2,697 | 53, 26 | 20 |
| 4 | 97 (2) | 3 (0.4%) | 94.0% | 345 (168) | 56% | 95% | 73% | 3,033 | 47, 39 | 20 |
| 5 | 95 (3) | 5 (0.7%) | 93.8% | 336 (128) | 53% | 95% | 72% | 2,545 | 49, 34 | 17 |
| 6 | 97 (3) | 6 (0.9%) | 92.8% | 314 (128) | 58% | 93% | 74% | 2,975 | 48, 27 | 18 |
| 7 | 97 (3) | 1 (0.1%) | 94.2% | 296 (100) | 56% | 95% | 77% | 2,349 | 50, 33 | 21 |
| 8 | 97 (3) | 14 (2.1%) | 94.7% | 305 (110) | 60% | 96% | 78% | 2,949 | 42, 36 | 18 |
| 9 | 96 (4) | 0 (0.0%) | 93.6% | 314 (107) | 61% | 94% | 70% | 2,507 | 46, 39 | 19 |
| 10 | 98 (2) | 8 (1.1%) | 92.4% | 333 (148) | 60% | 94% | 73% | 2,794 | 52, 32 | 18 |

- **Keeps going: met.** Sprites are alive at the end in all 10 seeds, 17 to 21 generations deep (32a 21 to 24), and no world went over its limit of 100.
- **Lone eggs very rare: met.** 40 of 6,895 eggs were laid alone (0.6%, at most 2%), and no seed laid more than 2.1% alone (at most 5%).
- **Courted first: met.** 93.4% of the 6,855 eggs laid by pairs came after the two danced for each other (at least 90%). None was laid by kin.
- **Families stay together: met.** Partners had each other in sight for a median 57% of the time both were alive and grown (at least half), over 3,193 partnerships; an egg had a parent in sight for 95% of its time (at least three quarters), over 6,895 eggs; and a young sprite with a living parent saw one for 74% of its childhood (at least half), over 6,867 young. No seed's median fell below 53%, 93% and 70%.
- **Safe: met.** The property test with random genomes passes, now with attention instincts about eggs and about whom they name, its family, kin, partner, parents or young, run once with 1,000 genomes: courting and mating happen only between adults, and the self-check finds no Mate or Court that isn't.
- **A4: met.** Of 973 deaths in ticks 10,000–50,000 of the viability run over all seeds, 2 were of hunger and 20 of thirst (2.3%, under 25%; 32a 0.5%).
- **A3: met.** Over seeds 1–30, trained sprites hit a median of 0 times in ticks 10,000–20,000, against 40 untrained, and all of 100 random sets of 10 of those seeds pass.
- **A5: met** on the development machine (M1 design §7.4): 296 ticks a second on `speed`'s own seed 7, on Fauxpaw's PC, with the same state hash as the cloud's. In the cloud, `speed` steps 191 to 198 ticks a second on seed 7 (three runs), against 206 to 223 for 32a timed alongside, and 217 to 252 on seeds 1–6 (`--seed`), against 32a's 236 to 262. Seed 7's world grows more of its sprites old than 32a's does (5% of its sprites' ticks, against 3%), and an old sprite's brain is expressed afresh each tick a gene of it weakens (§9.4). Finding a chemical or locus by its ID at once, rather than searching the list, won back about half of 32b's cost on seeds 1–6, and changes no outcome.
- **Since then ([#210](https://github.com/Keazra/terra-sprites/issues/210)):** an old sprite's weakened genes now weaken only its instincts' links, in place, rather than its brain being expressed afresh, and a few lookups were made quicker; no outcome changes. Timed in the cloud at `41540d0`, in turn with `main` at `97c1175`: `speed` 194 to 205 ticks a second (three runs), against `main`'s 146 to 161; and `--fast-forward 20 --seed 7`, row for row the same report, 206 ticks a second against 172. Over 300 ticks of `speed`, callgrind counts 10.84 billion instructions, against 12.01 billion for `main` and 10.23 billion before Slice 32 (`27927d7`).
- **A1 and A2: met** (their tests pass).
- **For information, partners:** a seed made 296 to 345 pairs (32a 433 to 518), of which 100 to 168 were by a sprite that had had a partner (32a 383 to 485); 56 to 77 parted (32a 241 to 312), and 222 to 262 lost a partner to death. Partners now mostly stay together until one dies.
- **For information, the rest:** hunger and thirst killed 134 of 6,070 sprites that died after tick 10,000 (2.2%; 32a 1.1%, §13.3 5.1%), from 1.2% to 5.5% a seed: twice 32a's share but under §13.3's, likely because anxiety holds a sprite by its family a little longer before it goes to eat or drink (not traced). There were about three times as many calls (1,977 to 3,033 an hour, 32a 706 to 980), as anxious sprites call their family. The adults at the end were 488 males to 345 females over all ten seeds (32a 449 to 374), up to two to one in a seed. The living had 0.43 to 1.10 broken genes each at the end (32a 0.46 to 1.18). The 5,547 hatchlings old age killed lived a median 6.4 hours, a tenth under 4.8 and nine tenths under 7.4 (32a 6.6, 4.8 and 7.7). G4: 1 of the 132 numbers moved a tenth the same way in all 10 seeds, `grows_old`, which rose in all 10.

### 15.10 Later

- **True echoes:** a call heard twice, or from the wrong side, in a hall.
- **Clans:** grown young who stay with their family, like elephants or meerkats (round 1, Q8).
- **Eggs that need warmth:** an egg grows only while a parent is beside it, for Slice 29's harsher world ([#147](https://github.com/Keazra/terra-sprites/issues/147)).
- **Sharing food** as part of getting along, once sprites can carry ([#135](https://github.com/Keazra/terra-sprites/issues/135)).
- **A dance's vigour** as a gene females can choose on, as a peacock's tail.

---

## 16. Fast-forward and the speed cap (Slice 27)

Settled with the owner on 2026-10-14 ([#145](https://github.com/Keazra/terra-sprites/issues/145), with [#121](https://github.com/Keazra/terra-sprites/issues/121)), in a round of seven questions. They chose both for the first, Lab mode from the start and an end-game unlock in Story mode (Q1, C); asked for holding `?` to explain a mode (Q2); and kept the other recommendations. Two sides of how time runs: Story mode, the game as it's meant to be played, tops out at 4×, so a life never feels rushed; and a Lab world can be run for many generations with no screen, then opened in the game (G6).

### 16.1 Story mode and Lab mode

- **New world has a Mode row** under the seed: Story first and picked unless `←` `→` pick Lab. The picked one shows in brackets, with what it means in a few words beside it: "4x at most" for Story, "every speed, for experiments" for Lab. The box's hints take two lines: `tab another seed  ←→ mode  ↑↓ preset` and `hold ? what the mode means  enter start  esc back`.

```
╔═ New world ══════════════════════════════════════════╗
║ Seed    8261934477120934                             ║
║ Mode    [Story]  Lab   4x at most                    ║
║                                                      ║
║ Preset  ► default                                    ║
║           big-lake                                   ║
║                                                      ║
║                                                      ║
║ tab another seed  ←→ mode  ↑↓ preset                 ║
║ hold ? what the mode means  enter start  esc back    ║
╚══════════════════════════════════════════════════════╝
```

- **Holding `?` explains the mode picked** (the owner, Q2), in the presets' place, and `←` `→` switch it while it's held:

```
║ Mode     Story  [Lab]  every speed, for experiments  ║
║                                                      ║
║ Lab mode: a world for experiments.                   ║
║ · Every speed, up to Max; it can be fast-forwarded.  ║
║ · A world keeps its mode for its whole life.         ║
```

  Story's reads "Story mode: the game as it's meant to be played.", "· Speeds up to 4x, so a life never feels rushed." and the same last line. Letting go of `?` puts it away; a terminal that never says a key was let go of (M1 design §6.6) puts it away at the next key other than `←` `→`, which does its own job too. The box is tall enough for it, so it doesn't jump. The owner's example also named the inventory, Story's alone (M4 design §2.2): its line, and the ship's other differences, join the explanation when M4 builds them, as M2 has no inventory yet.
- **A world keeps its mode for its whole life** (M4 design §11): it's world state, saved (M1 design §2.8), in the state hash, and carried by a replay that starts from a new world (M1 design §2.7). A save or replay from before this slice is a Story world, the game it was, and the save format stays 1.
- **From the command line, a new world is a Story world unless `--lab` is given** (Q2, A). The try-a-PR script passes it on: `.\scripts\try-pr.ps1 201 -Seed 7 --lab`.
- **The top bar says "lab"** after the speed in a Lab world, as `► 16x lab` or `|| paused 4x lab`; a Story world's is as before.
- **In M2 the modes differ only in speed and fast-forward.** Nothing in the simulation reads the mode yet: the same seed makes the same world in both, and the sprites live exactly as they did (§16.5). The ship's differences come with M4 (M4 design §2.2).

### 16.2 The speed cap

- **In a Story world, `+` stops at 4×,** pressed or held, and the status line says "4x is Story mode's fastest". The slow speeds, ⅛× to ½×, are in both modes. A Lab world's speeds are M1's, up to Max (M1 design §6.6).
- **The cap holds wherever time runs:** the map, the population graph (§14.2), and a replay, which plays at its world's speeds. Loading a Story world over a clock running faster slows it to 4×. The title screen's world runs at 4×, as before (§8.1).
- **The training aids still fit** (#121 asked): a pet's reach back is two seconds of the player's time, 10 ticks at 4× (M1 design §6.5), and a step while paused a second's worth, 5 ticks at 4× (M1 design §6.6). Both were built for every speed, and the cap only takes away their faster ends.
- **The end-game unlock** (the owner, Q1): once the whole ship is reactivated, a Story world offers to lift the cap, and if the player accepts, it runs at every speed from then on, staying a Story world otherwise. It needs the ship, so M4 builds it (M4 design §2.3).

### 16.3 Fast-forward

A command, run with no screen at all (Q3, A): `cargo run --release -- --fast-forward 200 --seed 7`.

- **It runs for hours of play at 1×** (Q4, A), 4,500 ticks an hour, a whole number from 1: 200 hours is 900,000 ticks, about 90 generations.
- **It makes a new world, from `--seed`, `--preset` and `--data` as the game does, or carries on a save with `--load mira`.** Fast-forward is faster than 4×, so it's a Lab tool (Q5, A): the world it makes is a Lab world, and a Story save is refused: "mira is a Story world: only Lab worlds fast-forward". An evolved species still reaches a Story world as genome files: `g` saves one from a sprite, and the Place menu's "sprite from a genome file" works in both modes in M2.
- **`--load <save>` also works on its own,** to open a save in the game from the command line as Load does: a save's name in the saves folder, or a path to a `.tspr` file. It goes straight in, as `--seed` does (§8.5).
- **What it shows while it runs** (Q6):

```
PS> cargo run --release -- --fast-forward 20 --seed 7
Fast-forwarding a new Lab world, seed 7, default preset, for 20 hours (90,000 ticks).
Ctrl+C stops it early and saves what has run.

 Hours  Sprites  Young  Adults  Old  Eggs  Generation   Died  Old age  Hunger  Thirst  Hurt
     2       43     16      19    8     3           1      8        8       0       0     0
     4       57     13      35    9     2           3     13       11       0       1     1
   ...
    20       95     16      62   17     3          11     24       24       0       0     0

Done: 20 hours in 5 minutes, 258 ticks a second.
Saved as seed-7-20h in your saves folder (…): the title screen's Continue opens it.
The tables are beside it: seed-7-20h-population.csv and seed-7-20h-genes.csv.

Moved most from the starter's, as the middle value among the living:
  GrowsOld: trait                        22500 → 29425     up 31%
  always → Wander from Adult: weight       0.3 → 0.2498    down 17%
```

  - **A row each tenth of the run,** so always ten. "Sprites" counts the sprites at every life stage, and "Eggs" the eggs beside them, as the population graph does (§14): an egg isn't a sprite until it hatches. "Generation" is how many generations the longest line of descent has run since the run started: the world's sprites then are the 0th, its eggs then the 1st, an egg is one on from the deeper of its parents, and a sprite the Cursor spawns is the 0th. "Died" and the causes after it count since the row before: hunger is starvation, thirst dehydration, and hurt anything an object did.
  - **A last line, redrawn in place between rows,** reads like `► 143 hours · generation 64 · 95 sprites · 3 eggs · 312 ticks a second · about 13 minutes left`. It shows only on a terminal, so a report sent to a file has the rows alone.
  - **At the end** it says how long the run took and how fast it ran, where it saved, the gene values that moved most (§16.4), and its gene warnings (§16.6, #274).
- **It saves in the saves folder and keeps the world it came from:** `seed-7-200h` for a new world, `mira-200h` for mira, or the name `--save <name>` gives, stored as any save by name is (M1 design §6.7). A name a save already has gets `-2`, `-3` and so on, so nothing is saved over, and the report and the tables give the name it was saved by.
- **`Ctrl+C` stops it early and saves what has run,** with a last row where it stopped, under the whole hours it reached: `seed-7-143h` for a run stopped at 143.5 hours. A second `Ctrl+C` stops at once, without saving. A long run is never lost by stopping it.
- **It runs exactly the world the game would:** the same seed fast-forwarded is, tick for tick, the world the game has with `--lab` after running as long. A fast-forward's world is always a Lab world, and the mode is world state, so the game's Story world from the same seed is the same but for its mode.
- **Flags that can't go together** are refused before anything runs: `--replay` with any flag that makes or loads a world, `--load` with `--seed`, `--preset`, `--data` or `--lab`, and `--save` without `--fast-forward`. So are more hours than a run can count its ticks for, 409,927,646,082,434 (#148).

### 16.4 The tables

Both are beside the save, named after it, and open in Excel or Google Sheets. Each starts with the mark that tells a spreadsheet it's UTF-8, for the arrows in the genes' names.

- **The population table,** `seed-7-200h-population.csv`, has a row for each count the world keeps for its graph (§14.1): every 100 ticks at first, further apart as the run grows, never more than 511 rows, and one more for the end when it falls between counts. Its columns are the hours of play, the tick, the eggs and each life stage, then what happened since the row before: eggs laid and hatched, sprites spawned, deaths, and deaths by cause, a column for each cause any sprite has died of, named as the screen names it ("Hurt by thornbush"), physiology's three always.
- **The genes table,** `seed-7-200h-genes.csv`, has a column for each of the starter genome's gene values, from the save's own data pack, named as the generations run names its gene numbers (§12.7): `always → Wander from Adult: weight`, a gene with a life-stage switch named by when it works (§19.6). Its first row is the starter's own values; then a row at the start, every hour of play, and where it stopped if that fell between hours, each with how many sprites were alive and each value's middle among them, blank where none carries it. A gene that's doubled lines up, the nth of a name with the starter's nth of it (§13). After how many sprites were alive come two columns (#274): the hidden flaws carried and the broken genes, each on average among the hatched sprites alive (§16.6), blank where none is, so a long run shows whether they level off.
- **The gene values that moved most** end the report: up to five whose middle value at the end moved a tenth or more from the starter's, furthest first, with how far up or down. Halving counts as far as doubling, and crossing 0 furthest of all. With none, it says "No gene value has moved a tenth from the starter's yet", and with no sprite alive, "No sprite is left alive to measure the genes of". The gene warnings follow (§16.6).
- **The generations run and fast-forward read gene values with the same code,** which moved from the run into the simulation's library for it; the run's report is byte for byte as before (§16.5).

### 16.5 How we know it worked

Agreed before building (Q7):

1. **The cap.** Story mode never passes 4×, by any key, held or pressed, with the graph open or not, and Lab mode reaches Max. Tests.
2. **The same world.** A world fast-forwarded is exactly the world the game would have with `--lab` after running the same seed as long: the same tick and state hash. A test, over an hour of play.
3. **It opens.** The save is the newest, so Continue loads it; it loads in Lab mode, with its whole population record, and its clock goes past 4×. A test, and the owner trying it.
4. **No slower than the game.** Fast-forward runs within a tenth of the speed check's ticks a second, so the report and the tables cost next to nothing.
5. **For information, not a bar:** seeds 1 to 10 run for 220 hours each (about 100 generations), and how many gene values moved a tenth the same way in all ten. G4 is judged for real once Slice 29's harsher world gives selection something to work on (Slice 24's call, §12.7).

**Measured (2026-10-14)** in release builds in the cloud:

- **1, 2 and 3: met.** The tests pass, among them New world's Mode row and its explanation, the flags, the tables, and the report's rows and lines. Played in the cloud: a 2-hour and a 20-hour run, a run stopped with `Ctrl+C`, and a saved run carried on with `--load`.
- **4: met, by the owner's call (2026-10-14), though not as written.** Against the speed check it falls short: the speed check ran 285 and 284 ticks a second (22,500 ticks), and fast-forward 212 and 214, a quarter slower, on seed 7 after 20 hours at 100 sprites. The bar was there to show that the report and the tables cost next to nothing, and they do: that same world ran 216 and 217 ticks a second fast-forwarded and 215 and 219 stepped plainly, in turn. A world that has run 20 hours is slower to step than the speed check's freshly crowded one, fast-forwarded or not. The plain stepping was timed with a throwaway example, not kept.
- **The sprites live exactly as before:** the `generations` run, seeds 1–2 for 20,000 ticks, prints the same, byte for byte, on `main` at `27927d7` and on this slice. The state hash differs, as it should, since the mode is world state.
- **5: to come.** Ten seeds of 990,000 ticks take most of a day in the cloud. The result is posted on [#145](https://github.com/Keazra/terra-sprites/issues/145) when it ends, and joins this section with Slice 29 (§12.7), which measures G4 again.

### 16.6 Gene warnings (#274)

Designed by an agent on 2026-10-19 from the owner's principles and earlier decisions ([#274](https://github.com/Keazra/terra-sprites/issues/274), with [#237](https://github.com/Keazra/terra-sprites/issues/237)'s), with the warning levels the owner agreed on 2026-10-20, as proposed, before anything was built. The owner's notes asked for a "genetic canary": something that says, without anyone reading the tables, when evolution is heading somewhere bad, such as flaws piling up or `grows_old` running to an extreme. So the end of a fast-forward and the baseline report (M1 design §7.6) give **gene warnings**: lines that ask for a look, never a verdict. Each question settled, and what it rests on, is in [the change file](changes/2026-10-19-gene-warnings.md).

- **What's measured,** at the end of the run, among the hatched sprites alive, eggs aside. The world's first sprites are left out: their genes are the starter's, so they could only water the numbers down.
  - **The hidden flaws they carry and their broken genes,** each on average (§12.3, §12.4).
  - **The share of them lacking each of the starter's genes,** by what it does and when it works, as §13.3 counts it: a sprite lacks a gene when it has fewer copies of it than the starter has, through a loss or a change.
  - **Each gene value held within a range:** a trait's or a brain parameter's, from physiology, or a level held between 0 and 1, such as a reaction's rate. It's taken as its middle among the living, as the genes table counts it (§16.4), with how far it has gone from the starter's value towards the end of its range it moved towards, as a share of the room there was. A value the starter has at one end of its range has no room that way.
- **The needs genes** are twelve: the eleven #237 named, and `fullness Level → hunger`, which Slice 29 added to ease hunger as the gut fills (§19.4). They're the starter's genes that raise or ease hunger, thirst or tiredness, fade one, or turn one into its action. The data says which they are, not a list in the code. The needs (`brain_io.ron`) that a starter gene raises from the body's own state, a physical chemical, are the body's needs: hunger from energy, thirst from hydration, tiredness from stamina. A needs gene is a starter gene that adds to one of them, is its half-life, or is an instinct with it as an input. So a data pack with another species gets its own needs genes with no new code, and a test pins the built-in pack's twelve.
- **A gene warning** comes, with at least 10 hatched sprites alive (as A4 judges nothing on fewer than 10 deaths, M1 design §7.4), when:
  - **more than half of them lack a needs gene,** #237's own line for careful copying (its decision 4);
  - **or a gene value's middle has gone nine tenths of the way to the end of its range,** where physiology's limit, not selection, is now what holds it. For the starter today that's `GrowsUp` at 1,800 ticks or 12,600, `GrowsOld` at 7,650 or 56,250, `Speed` at 4.3 or 11.5 and `SenseRadius` at 6.4 or 13.6, and the brain parameters the same way: `act_threshold` (§19.2) at 0.025 or 1.825, and `trace_decay`, which starts at the bottom of its range, only at 0.941.
- **Reported with no warning:**
  - **flaws and broken genes,** until long runs show where they level off (§16.7). Die-outs came from gene changes, not flaws ([#221](https://github.com/Keazra/terra-sprites/issues/221));
  - **the starter's other genes** a tenth or more of the hatched sprites alive lack: a gene the species does better without is evolution (#237's decision 1);
  - **gene values with no range,** such as an instinct's weight or a half-life: moving is what G4 asks of them, and the gene values that moved most list them (§16.4).
- **A warning is no verdict.** Every criterion is judged as before, the baseline's launcher files an issue only for a broken run, never for a warning, and a warning lasts only while the genes stay where they are.
- **The end of a fast-forward** gives, after the gene values that moved most (§16.4), a line on flaws, then its warnings:

```
The hatched sprites alive carry 6.8 hidden flaws and 1.2 broken genes each, on average.
Gene warnings:
  drank Level → thirst: lacked by 62% of the hatched sprites alive, a needs gene (warns past half)
  GrowsUp: trait: 4500 → 1500, all the way to the end of its range (warns at 1800)
```

  With none, it says "No gene warnings: no needs gene is lacked by half the hatched sprites alive, and no gene value is near the end of its range." With fewer than 10 hatched sprites alive, "Too few hatched sprites are alive to give gene warnings." With no sprite alive, the line before has already said so, and nothing follows.
- **The baseline report** (M1 design §7.6) gives the warnings near the top, after what moved and anything broken, or says "No gene warnings." Further down, a Genes section has a row for each seed of the viability run: the hatched sprites alive, the flaws they carry, their broken genes, the needs gene most of them lack and the gene value nearest the end of its range. Under it are the starter's genes a tenth or more of them lack, with the seeds. What moved covers each seed's flaws, broken genes and warnings, so a warning that comes or goes shows at the top.

```
## Gene warnings (M2 design §16.6)

- Seed 4: 62% of the hatched sprites alive lack `drank Level → thirst`, a needs gene. It warns past half.
- Seed 3: `GrowsUp: trait` is 1500, all the way from the starter's 4500 to the end of its range, 1500. It warns at 1800.
```

- **How long a run:** the baseline's viability run as it is, 50,000 ticks on seeds 1–10, about 11 hours of play: it costs no extra run, and catches, on each commit of `main`, a change that makes the species throw something away fast. About a hundred generations is a fast-forward of 220 hours, on the owner's PC (`docs/agents/how-we-work.md`, "Cloud sessions").
- **One measure, in the simulation's library,** beside the gene values' (§16.4): given a world, it counts all of the above and gives the warnings, reading the ranges and the needs genes from the world's own data pack. The baseline, fast-forward and the generations run all use it. Slice 30's 220-hour report, built first, counts the share of the living lacking each starter gene as §13.3 does; this measure takes that count into the library unchanged, so the warnings, that report and careful copying's trigger (#237's decision 4) count alike, and the generations run's report stays byte for byte as it was. The viability run's result carries the genes of its living at its end, so the baseline runs nothing extra.

### 16.7 Later

- **A warning for flaws,** once the genes tables of the 220-hour runs show where flaws and broken genes level off (§16.6).
- **A key in a Lab world that blanks the map and runs flat out,** with `Esc` to stop and look (Q3, B), if fast-forward from the command line ever isn't enough. Max already runs at about three quarters of what the computer can do.
- **Runs measured in generations** (Q4, B), once a generation has a definition worth stopping on.
- **The uncapped population** ([#205](https://github.com/Keazra/terra-sprites/issues/205)) runs with this: a preset with the limit at 1,000 sprites, fast-forwarded overnight, then its population table read and the world opened.

---

## 17. The family tree (Slice 25)

Settled with the owner on 2026-10-14 ([#143](https://github.com/Keazra/terra-sprites/issues/143)), in a round of nine questions. They kept every recommendation, and changed one detail: the sprite the arrows are on blinks, rather than standing in reverse video ("Probably a blinking cursor", Q5). Grieving for and remembering the dead, recent or long ago, is pinned for later (Q8, [#206](https://github.com/Keazra/terra-sprites/issues/206)). Any sprite's parents, grandparents and young can be seen, living or dead (G2).

### 17.1 Generations and the record of the dead

- **Each sprite and egg has a generation** (Q6, A). The world's first sprites, and any the Cursor places, are **generation 1**; a hatchling is one more than its parents', or than the newer parent's when theirs differ. An egg has its generation from when it's laid.
- **And where it came from:** one the world began with, one the Cursor placed, or one that hatched. A sprite with no parents says which.
- **The world keeps a record of each sprite that dies** (Q4): its name, its parents, its generation, whether the Cursor placed it, when it was born and died, and how, at what life stage and as which sex. A record is about 100 bytes in a save.
- **It keeps the newest 10,000** (Q4, B): past that, the longest dead are forgotten first, but never a sprite the player named. About 3 sprites die every 1,000 ticks once the default world is full, so in play nothing is forgotten, and a long fast-forward (Slice 27) can't make a save grow without end.
- **It's world state:** saved (M1 design §2.8), in the state hash, the same in a replay (M1 design §2.7), and checked every tick there's been a death (M1 design §7.1): the dead aren't in the world, each was born before it died, its parents are older than itself, and its generation comes after theirs, living or remembered.
- **Nothing in the world reads it** (Q8, A). It's the player's, like a breeder's studbook, so the sprites live exactly as they did before it (§17.7). Sprites know their own parents already, and Slice 32 teaches them their kin (§15.5).
- **The young of a sprite** are every sprite, egg and remembered dead with it as a parent, in the order they were laid.
- **A save from before this slice** has no record, so a sprite that had died before it was saved is forgotten: the tree still names it from its young, but knows nothing more, and says it **lived long ago**. Its living sprites and eggs are given generations on loading, in ID order, as if a parent the world doesn't know were of the first generation. The save format stays 1: the record and generations are new, with defaults (M1 design §2.8).

### 17.2 The tree

- **`k` opens it over the panels** (Q1, A), as `p` opens the population graph, on the selected sprite or egg, living or dead: `k` is for kin, as `f` is Follow and `p` the graph. With none selected, it reads "No sprite selected: click one, or press Tab", as the tabs do. `k` or `Esc` closes it. Time runs on behind it, and `space`, `.`, `+` and `-` work as ever, as do `Tab` and `Shift+Tab`, which select the next sprite and redraw the tree around it. Its title names the sprite in the middle.
- **It reaches from grandparents to young** (Q2, A). Four grandparents fit side by side, each parent centred over its own, then the sprite in the middle, with its life stage, sex, age and generation. Its young are below it, grouped under their other parent (`with Sprite #27476`, `partner Sprite #27476` for its partner's (§15.4), or `alone`), the groups and the young in each in the order they were laid, and its **brothers and sisters**, the young of either parent, are at the foot, with the parent they share (`through Mira #10386`) when it's only one. Lines join the generations, `└──┬──┘` and the like. To see further, the player walks the tree (§17.3).
- **What it says of each** (Q3, A), under its name as the log names it, on up to two lines:

| Sprite | Line |
|---|---|
| Alive | `young`, `adult` or `old`, and `· 3 young` if it has any |
| An egg | `egg · hatches in 500`, or `egg · hatches once it's put down` while the Cursor holds it, or `… once nothing stands on it` |
| Dead | `died of old age`, `died young of dehydration`, `died hurt by thornbush`, all in dark gray |
| Forgotten (§17.1) | `lived long ago`, in dark gray |
| The sprite in the middle | `adult · female · age 19,331 · generation 4`; an egg, `egg · hatches in 500 · generation 5`; one dead, `died of old age at age 27,401 · generation 3` |

- **"Young" is said of a death** only when the sprite died before growing up: that's the death Slice 29's harsh childhood is about.
- **A sprite with no parents says where it came from** in their place: `one of the first sprites`, or `placed by the Cursor`. A parent with none says it above itself.
- **An egg is named as the sprite it hatches into,** `Sprite #53074`, since it keeps its ID when it hatches (§10.4), and its line starts `egg`.
- **A row that doesn't fit scrolls.** Each young sprite, and each brother or sister, takes 24 columns, so four fit at 100 columns. With more, the row shows as many as fit and scrolls sideways as the arrows go along it, with `◄` or `►` at an edge where there are more.
- **A sprite can be on the tree twice.** Sprites don't avoid their kin until Slice 32, so a line can be inbred: a sprite's other parent may also be its brother, or its parents' parent the same sprite. Each place on the tree is a card of its own.

The default world (seed 7) at tick 48,000, the round's mock-up as built. Its dead are dark gray in the game, and the sprite in the middle blinks until the arrows choose another.

```
 Terra Sprites │ tick 48,000 │ ► 1x │ seed 7 │ sprites 99 │ not saved                        ? help
┌─ Family tree ── Sprite #30212 ─────────────────────────────────────────────────────── esc close ─┐
│                                                                                                  │
│        Sprite #531            Sprite #3639             Sprite #525             Sprite #552       │
│      died of old age         died of old age         died of old age         died of old age     │
│                                                                                                  │
│             └───────────┬───────────┘                       └───────────┬───────────┘            │
│                   Sprite #10386                                   Sprite #4072                   │
│                  died of old age                                 died of old age                 │
│                                                                                                  │
│                         └───────────────────────┬───────────────────────┘                        │
│                                           Sprite #30212                                          │
│                            adult · female · age 19,331 · generation 4                            │
│                                                 │                                                │
│                         ┌───────────────────────┼───────────────────────┐                        │
│                with Sprite #27476      with Sprite #24222      with Sprite #37867                │
│                         │                       │                       │                        │
│                   Sprite #40652           Sprite #45763           Sprite #53074                  │
│                  adult · 1 young              adult           egg · hatches in 501               │
│                                                                                                  │
│                                                                                                  │
│ BROTHERS AND SISTERS                                                                             │
│                               Sprite #24222           Sprite #32305                              │
│                               old · 2 young               adult                                  │
│                           through Sprite #4072    through Sprite #10386                          │
│                                                                                                  │
│                                                                                                  │
│                                                                                                  │
└──────────────────────────────────────────────────────────────────────────────────────────────────┘
 (128,80) dirt · berry (fresh) │ SELECT     ←→↑↓ choose  enter centre on it  space pause  esc close
```

Sprite #24222 is both the father of her second young and her brother, through Sprite #4072.

- **The status line's key hints** read `←→↑↓ choose  enter centre on it  space pause  esc close`.

### 17.3 Walking the tree

- **The arrows, or `W` `A` `S` `D`, choose a sprite on it** (Q5, A): left and right along a row, and up and down to the nearest in the next row up or down that has any, the left one of two as near. The pointer chooses the one it's over, and leaves the Cursor where it is on the map beneath, as the graph's does (§14.2, #148). The tree opens on the sprite in the middle.
- **The one chosen blinks:** its name is in reverse video for half of each second (the owner, Q5).
- **`Enter`, or a click, walks the tree to it:** the tree centres on it, and it's selected, so the inspector shows it, and the map centres on it if it's alive or an egg on the map. A dead one is selected as a sprite that died while selected is (M1 design §6.1): its tabs read `Sprite #10386 died of old age at age 22,787`. One that **lived long ago** can't be, and the status line says so: `Sprite #531 lived long ago: the world no longer remembers it`. `k` or `Esc` closes the tree on whatever was selected last.
- **A sprite named before it died keeps its name** in the record, so after a load the screen names the dead as it did before, on the tree, in the log and in a dead sprite's tabs.

### 17.4 The lineage colours

- **`b` has a third colour mode, lineage** (Q7, A), after strongest drive and plain (M1 design §6.3), which the M1 design promised M2 would add. It colours the selected sprite's lineage on the map: its parents and grandparents **light blue**, its brothers and sisters **light red**, its young and theirs **light magenta**, and every other sprite **dark gray**, so a family stands out at a glance. The selected sprite keeps its own colour, and with none selected, every sprite does. The status line says `Colours: lineage`.
- **The round called it Family;** the game calls it lineage, since a family is the household of Slice 32 (§15.6), and this is by blood.
- **One that's two of these at once,** as in an inbred line, takes the first it is of elders, young and brothers and sisters.
- **The colours were picked from swatches** of each colour of the 16 over grass, dirt, sand, shallow and deep water and rock: these four read on all of them, and stay apart from the drive colours a player knows (yellow, cyan, red, blue, magenta).
- **The help screen** gains a line, `LINEAGE  ☺ parents, grandparents  ☺ brothers, sisters  ☺ young, theirs  ☺ the rest`, each `☺` in its colour.
- **Themes gain a `lineage` section** (`docs/reference/themes.md`), its four colours, `elders`, `siblings`, `young` and `others`. A theme without it, or without some of it, draws what it leaves out as the default theme does, as with the population graph (§14.5).

### 17.5 The inspector

- **The Body tab** says where a sprite came from, and its generation, after its sex: `parents Mira #10386 and Sprite #4072 · generation 4`, on two lines when it's too long for one, or `one of the first sprites · generation 1`, or `placed by the Cursor · generation 1`. An egg's card says its parents and generation the same way.
- **The World tab** says the generations alive after the number of sprites, `generations  3 to 6`, or `generation        1` when there's one, the number set to the right as the tab's others are. With no sprites, it leaves the line out. The population graph reads them the same way, now and at each moment it counted (§14.3, #208).

### 17.6 The information policy

The tree is a panel of its own, `FamilyTree`, about each sprite on it (M1 design §6.4). One the policy hides is a blank card that keeps its place, and so is where it came from; a group of young, or a brother or sister, doesn't name it as the other parent or the one shared, and the arrows and the pointer pass its card by, so it's never chosen (#148). With the sprite in the middle hidden, the box stays, empty and titled ` Family tree `. The lineage colours ask it too: a sprite hidden from the tree, or every sprite when the selected one is, draws in its own colour.

### 17.7 How we know it worked

Agreed before building (Q9):

- **UI tests** (ratatui's `TestBackend`) render the tree: grandparents, parents and young, living and dead, an egg, a sprite with no parents, a forgotten ancestor, more young than fit, walking the tree and the selection it leaves, the tree after a save and a load, and the lineage colours. Snapshots of the tree in both themes.
- **The world's own tests** cover the record: written at each death, saved, in the state hash, the same in a replay, a save from before this slice, and forgetting past 10,000 with the named kept.
- **The sprites live exactly as before:** the `generations` run (seeds 1–4, 30,000 ticks) prints the same, byte for byte, on `main` and on this branch.
- **No slower:** the speed check (A5) at least 200 ticks a second, and within 5% of `main` measured the same way.

**Measured (2026-10-14)** in release builds of this slice beside `main` at `27927d7`, in the cloud, with:

```
cargo run --release -p terra-sim --example generations -- --seeds 1-4 --ticks 30000
cargo run --release -p terra-sim --example speed
```

- **The sprites live exactly as before: met.** The `generations` report is the same, byte for byte, on both. The state hash differs, as it should, since generations and the record are world state.
- **No slower: met.** The speed check ran 192 and 194 ticks a second on `main`, and 191 and 201 on this slice, run in turn: the same, within the noise. This container was slower than Slice 26's (§14.7), so neither reached 200 every time; on the owner's PC A5 runs at about 540 (M1 acceptance).
- **The UI tests, the snapshots and the world's tests pass.**

### 17.8 Later

- **Generations on the population graph** (Q6, A, [#208](https://github.com/Keazra/terra-sprites/issues/208)): built once both were in, as `generations 3 to 6` in the lines that read a moment, which became three lines to fit (§14.3).
- **Grieving for and remembering the dead,** recent or long ago ([#206](https://github.com/Keazra/terra-sprites/issues/206), the owner, Q8).
- **Further back or down:** great-grandparents or grandchildren drawn on the tree (Q2, B and C), if walking it turns out too slow.

---

## 18. Learning by watching (Slice 28)

Settled with the owner on 2026-10-14 ([#146](https://github.com/Keazra/terra-sprites/issues/146), from [#66](https://github.com/Keazra/terra-sprites/issues/66)), in two rounds of questions: the owner took the recommendations but for two, going further on what draws a young sprite's eye (Q1) and having everyone learn by watching (Q7). Until this slice a sprite learned only from what happened to itself. Since Slice 32 the young keep near their parents, seeing one for about three quarters of their childhood (§15.9), but they learned nothing from them. In the default world on `main` at `97c1175`, left alone for 60,000 ticks on seeds 1–10, the 1,539 young that saw a parent for most of their childhood learned that water is good for thirst a median 3,243 ticks after hatching, and the 453 that didn't at 3,259. 83% of all of them grew up without learning that food is good for hunger.

The owner's direction: **the young pay attention to the grown-ups, their own parents most, and seeing them do something makes them want to try it too.** It's the guidance from fellow sprites that the harsher world leans on (§6, G5). So:

- a sprite **watches** the sprite its eye is on, or one using the kind of thing its eye is on;
- when that sprite uses something, the one watching **takes on a share of what it knows about the thing**: what it's good for, for the need it used it for, whether it's bad, and whether doing that to it works;
- **what happens to the one it watches** teaches it too: a pet or a shock from the Cursor, or a hit from another sprite;
- it **remembers the places** where it sees the other eased;
- **everyone learns by watching**, but in the starter genes the young take in more, and pay the grown-ups more attention, their parents most;
- **a parent doing something nearby draws its young's eye**, until the young has learned what it shows;
- **nothing about mating is ever learned by watching** (§5).

**The world and the genes.**

| The world (the same for every sprite) | The genes (vary from sprite to sprite, and evolve) |
|---|---|
| A sprite sees what the sprite its eye is on does, and how it goes about it, which shows what that sprite knows about the thing | Whom it watches: grown-ups, its parents, anyone |
| A young sprite sees when a parent is doing something that would show it a lesson it hasn't learned | How much that draws its eye |
| What it takes on is about things, and whom to fear; whom to like it finds out for itself | How much it takes in, and at what life stage |
| Nothing about mating is ever learned by watching (§5) | — |

Nothing here names an object or a species, so a new fruit, or a modded species, is learned by watching with no code, and a mod can make a species that watches keenly, or one of loners. It's the first way one sprite learns from another; NPCs in the larger game ([#47](https://github.com/Keazra/terra-sprites/issues/47)) that pick up a skill or a fear from each other would run on the same rule (§18.1).

### 18.1 Watching

- **A sprite watches the one its eye is on** (round 1, Q1). At step 4 (M1 design §2.4), after every sprite has learned from what happened to itself, a sprite takes on what another's **moment** was about (§18.2) when it sees that sprite, by its own sight as §15.2 counts it, and its eye at its latest step 5 was on that sprite, or on what the moment was about: the same object type, the same sprite, or the Cursor. A sprite heard calling, out of sight, isn't watched. A sprite can watch several in one tick, as when its eye is on a bush and two sprites in sight eat from bushes.
- **How much it takes in is a brain parameter, `watch_rate`** (ID 44, from 0 to 1, default 0): the share of the way its view moves towards the other's each time (§18.2). At 0 a sprite doesn't learn by watching. `learning_rate_mod` scales it, as it does every learning rate; `plasticity` doesn't, since it's a share rather than a rate, and the starter's is already higher for the young. A pack from before it gets the built-in range and default.
- **Grown-ups:** an attention instinct can be about **grown-ups**, `Grown` (ID 6), beside family, kin, partner, parents and young (§15.6): it draws the eye to a sprite grown up, adult or old. Copy and change (§13) can change whom a gene is about to it.
- **A parent showing it something** (the owner, Q1): an attention instinct can be about **a parent showing it something**, `Showing` (ID 7): a young sprite's parent whose action under way is a try at a thing or a sprite that the parent has learned a lesson about and the young hasn't. That is, some value the young would take on by watching that try (§18.2) is past `lesson_threshold` from 0 for the parent, and not yet for the young, on the same side or at 0. It's read at step 5, as the young takes its turn, from the action the parent has under way then, its way there included, and what each knows. So the young looks as its mother sets off for the water, and is watching when she drinks; and she stops drawing its eye that way once it has learned the lesson, or if it learned the opposite itself, which watching can't undo. It's a pull to that one parent, not to both, and one the young sees: a parent heard calling out of sight shows it nothing (#148). Never a try at Mate or Court (§18.3), nor one at the young itself. Like the other family ties (§15.6), the world says who's showing; the genes say how much it draws the eye.
- **The starter genes:** everyone learns by watching (the owner, Q7), the young more: a `watch_rate` of .5 until adult (Q3: half the gap), and .25 from adult, as round 1 offered. The young pay the grown-ups more attention (the owner, Q7): until adult, `always` draws a young sprite's eye to grown-ups (.3), and to its parents as much again, so it watches the grown-ups when nothing presses, its parents most; and to a parent showing it something strongly (1). The weights are first guesses, tuned against §18.6, so watching doesn't crowd out eating and drinking. With grown sprites watching too, the Cursor's training can spread from the sprite it trains to the sprites watching it, which A3 measures.

### 18.2 What it takes on

- **A sprite's moment** at step 4 is what it learned about there, from what happened to itself:
  - the thing it tried a verb on at its latest step 6, with the verb (M1 design §5.6);
  - the try the Cursor rewarded or corrected this tick, with its verb;
  - the sprite that hit it, which it learned to fear;
  - the Cursor, when it learned to fear it this tick, from a zap, a shock or a crash after a shove it saw (M1 design §5.6);
  - a place where it was eased (§7).
- **About a thing, the one watching takes on what the other knows of it** (Q2): its worth for the need the other used it for, its good and its bad, and the habit of the verb tried, which it kept as a **taught habit** (Q3), apart from what it learns itself, fading as slowly (M1 design §5.6, #187); since Slice 29 it's a **watched habit**, apart from taught ones too (§19.7). That watched habit settles either way towards the other's first-hand habit, its own and taught together, never its watched one (§19.7), and its decisions weigh it together with its own and taught ones. Once it has taken on something about the thing's worth, good or bad, it knows the thing's object type, as if it had touched one, so its own view of it counts rather than its category's summary (M1 design §5.6). Watching one use a thing it knows nothing of teaches nothing new, and leaves the category's summary standing, though what the watcher took on by watching of the thing's badness and habits settles back towards 0 (§19.7).
- **Of a thing's worth, only that for the need the other used it for:** the try's **motive**, the need whose instinct did most to choose it (M1 design §5.5), as the other chose it at its latest step 5. A try no need chose, such as one out of curiosity, or one the other's brain didn't choose, as in a test's script, shows no worth; good, bad and the habit pass on whatever chose it. So a sprite seen eating for its hunger shows what the food is worth for hunger, and nothing about what else it might be good for. Round 1 had every need's worth pass on. Measured on seed 10 of the default world, by tick 20,000 every sprite held berries worth .83 for anxiety on average, as well as .86 for hunger; with watching off, 19% held .03 for anxiety and 97% held .55 for hunger. A bush drops a berry only when it's full of fruit, and a new bush grows only from a dropped berry left to rot, so with the sprites eating more the bushes dwindled, to about 240 from tick 60,000 on against about 540 with watching off, and breeding fell until §15.9's bar on lone eggs failed (§18.6).
- **About a sprite or the Cursor, only fear** (Q2, Q4): the one watching takes on the other's fear of that one, and the habit of the verb tried on it, but not what it's worth to the other: whom to like it finds out for itself. So if a sprite hits its mother, the young learns to be wary of that sprite; if it sees the Cursor shock her, to be wary of the Cursor.
- **About a place, the one watching remembers it** as if it had been eased there itself (Q5), under §7.2's limits.
- **Only ever up to the other's view.** Since Slice 29, a thing's badness and habits taken on by watching are kept apart and settle either way, and are never passed on (§19.7); what follows holds for the rest. Each value moves `watch_rate` of the way towards the other's, but only when the other's is further from 0 the same way: never past it, never back towards 0, and never across it. Watching never makes a sprite forget what it found out for itself (Q2). The other's view is read once both have learned from their own step 4, and before anyone takes on anything this tick, so what one sprite takes on can't pass through another in the same tick, whatever order they're in.
- **Not about itself:** a moment about the one watching, as when the one it watches hit it or was hit by it, teaches it nothing by watching.
- **What's passed on is the other's knowledge, not the event.** A mother who knows water well teaches it even when she only sips; one that knows little teaches little (Q2). In the arena the grown-ups know water well but food poorly at tick 20,000, since they eat before they're hungry (round 2), so there watching passes on water far more than food; over generations in the default world, food passes on too (§18.6).

### 18.3 Nothing about mating

Content safety (§5), for every genome and pack: a try at Mate or Court is never a moment, and a value about the breeding drive, its worth for the drive, never moves by watching. So a young sprite watching its parents court or mate learns nothing from it, and no genome can have one learn by watching that anything is good for the drive.

### 18.4 What the player sees

- **The log** says where a lesson came from when watching tipped it past half from nothing (Q8), as lessons are logged now (M1 design §5.6): `Kit #21 learned from watching Pip #12: water is good for thirst`, `Kit #21 learned from watching Pip #12: thornbushes are bad`.
- **The Brain tab** is as before: a lesson learned by watching is listed with the rest, and its attention list shows whom a sprite is watching. On the Genes tab an attention gene about grown-ups reads `grown-ups (sprite)`, and one about a parent showing it something `its parent showing something (sprite)`.
- **The map** gains nothing: watching isn't an action, and a young one beside its mother looks it already.

### 18.5 Following a parent

Nothing new (Q6). Families already keep the young near (§15.6), and they learn the places they're shown (§18.2). If the young turn out to lose their parents on trips, going back to where a young one last saw its parent is the next step, brought to the owner first.

### 18.6 How we know it worked

Agreed before measuring (round 1, Q9; round 2, Q10):

- **Learns sooner:** in the arena `scenarios/watching.ron`, six grown sprites live for 20,000 ticks between water and berry bushes, then each gets a newborn beside it; the control run takes the grown-ups away as their young appear, so its young are orphans. The young with a parent learn that water is good for thirst in **at most half** the orphans' ticks, the median of seeds 1–10. On `main` at `97c1175` it was 3,523 ticks with a parent and 3,576 for the orphans.
- **Safe:** tests with random genomes show nothing about mating is learned by watching.
- **Nothing else got worse:** A1–A4 still met (A3 on seeds 1–30), Slice 32's family numbers still met (§15.9), the default world keeps going for 200,000 ticks, and A5 at least 200 ticks a second with 100 sprites.
- **For information, no bar:** in the arena, food and thornbush lessons and the young's deaths, with a parent and without. Food can't be a bar: by tick 20,000 only about half the grown-ups have learned that berries are good for hunger (3.5 of 6 for berries, 2.5 for bushes), so there's little to watch; Slice 29's hungrier world checks that food passes on (round 2, Q10). In the default world, how soon hatchlings learn water and food, against the table above, for those that saw a parent most of their childhood and those that didn't, and how many die in childhood, and of what.

`cargo run --release -p terra-sim --example lab -- scenarios/watching.ron --seeds 10` runs the arena, its young's rows at the end of each run.

**Measured (2026-10-14)** in release builds of this slice at `b9d3144`, in the cloud, on seeds 1–10 (A3 on 1–30), with:

```
cargo run --release -p terra-sim --example lab -- scenarios/watching.ron --seeds 10
cargo run --release -p terra-sim --example generations -- --ticks 200000
cargo run --release -p terra-sim --example lab -- scenarios/viability.ron --seeds 10
cargo run --release -p terra-sim --example lab -- scenarios/a3-correct-training.ron --seeds 30
cargo run --release -p terra-sim --example speed
```

The first build, which passed on a thing's worth for every need, met every bar but the lone eggs: 3.3% of all eggs were laid alone, and 17.9% in seed 10, as the bushes dwindled (§18.2). These are the numbers with only the motive's worth passed on. Merging `main`'s speed fix and the graph's generations afterwards changes no outcome.

The arena: ticks from hatching until a young sprite learned that water is good for thirst, the median of each seed's young, with a parent and as orphans:

| Seed | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | Median |
|---|---|---|---|---|---|---|---|---|---|---|---|
| With a parent | 1,403 | 1,334 | 617 | 1,027 | 745 | 848 | 815 | 844 | 1,486 | 1,657 | 938 |
| Orphans | 3,888 | 3,007 | 3,133 | 2,961 | 3,388 | 3,083 | 3,456 | 3,488 | 3,062 | 2,822 | 3,108 |

The default world left alone for 200,000 ticks, in §15.9's terms:

| Seed | Alive at the end (and eggs) | Eggs alone | Courted | Partners | Eggs | Young | Generations |
|---|---|---|---|---|---|---|---|
| 1 | 95 (5) | 5 (0.8%) | 91.8% | 52% | 95% | 80% | 18 |
| 2 | 89 (3) | 4 (0.6%) | 89.4% | 54% | 94% | 75% | 20 |
| 3 | 98 (1) | 1 (0.1%) | 92.3% | 55% | 95% | 79% | 19 |
| 4 | 94 (6) | 3 (0.4%) | 89.3% | 50% | 95% | 76% | 20 |
| 5 | 96 (4) | 3 (0.4%) | 94.4% | 48% | 95% | 78% | 20 |
| 6 | 100 (0) | 7 (1.1%) | 94.0% | 55% | 94% | 78% | 18 |
| 7 | 96 (3) | 7 (1.0%) | 93.8% | 54% | 95% | 76% | 19 |
| 8 | 99 (1) | 8 (1.2%) | 91.6% | 56% | 95% | 76% | 22 |
| 9 | 94 (6) | 1 (0.1%) | 93.3% | 55% | 95% | 79% | 21 |
| 10 | 97 (2) | 4 (0.6%) | 91.1% | 56% | 95% | 80% | 20 |

- **Learns sooner: met.** The young with a parent learned that water is good for thirst in a median 938 ticks, against 3,108 for the orphans (at most half, 1,554). Every one of the 58 young learned it in both runs. On `main` it was 3,523 and 3,576.
- **Safe: met.** The property test with random genomes passes, now with a random `watch_rate` and attention instincts about grown-ups and a parent showing something: no sprite that isn't grown up knows anything about mating, what anything is worth for the breeding drive or a habit of mating or courting, which it could only have learned by watching.
- **Slice 32's families: met.** Sprites are alive at the end in all 10 seeds, 18 to 22 generations deep, and no world went over its limit. 43 of 6,732 eggs were laid alone (0.6%, at most 2%), no seed more than 1.2% (at most 5%). 92.1% of pairs' eggs came after the two danced for each other (at least 90%). Partners had each other in sight for a median 54% of the time (at least half), an egg had a parent in sight for 95% of its time (at least three quarters), and a young sprite saw a living parent for 78% of its childhood (at least half; 32b 74%).
- **A4: met.** Of 926 deaths in ticks 10,000–50,000 of the viability run over all seeds, 4 were of hunger and 33 of thirst (4.0%, under 25%; `main` 2.3%).
- **A3: met.** Over seeds 1–30, trained sprites hit a median of 0.5 times in ticks 10,000–20,000, against 38.5 untrained, and all of 100 random sets of 10 of those seeds pass.
- **A1 and A2: met** (their tests pass). In the lab, A1's learner touched the thornbush a median of once, against 40.5 for its control, and A2's runs met their bar too, the late trainer's included.
- **A5: met,** 220–232 ticks a second with 100 sprites (`speed`, three runs, after merging `main`'s speed fix), against 219–235 for `main` at `11cf11d` timed alongside: no cost that shows above the cloud's noise.
- **For information, the arena:** 44 of the 58 young with a parent learned that berries are good for hunger, a median 2,412 ticks after hatching, against 18 orphans, most of whom didn't learn it within the 5,000 ticks. The orphans learned that thornbushes are bad sooner (837 ticks against 1,571), from pricking themselves; about as many learned it (46 and 50). Three young with a parent starved, against one orphan.
- **For information, the default world** left alone for 60,000 ticks, counted as in the table at the top of §18: the 1,560 young that saw a parent for most of their childhood learned that water is good for thirst a median 808 ticks after hatching, and that food is good for hunger at 1,950; the 340 that didn't, at 1,131 and 2,369. 10% of all of them grew up without learning that food is good for hunger (`main` 83%). 3 of the 1,900 died in childhood, as on `main`.
- **For information, the rest:** hunger and thirst killed 183 of 5,916 sprites that died after tick 10,000 (3.1%; 32b 2.2%), the most in seed 2, 70 of them, nearly all of thirst (10.9% of its deaths). The adults at the end were 516 males to 302 females over all ten seeds (32b 488 to 345). On seed 10, the berry bushes held at 550 to 600 throughout the 200,000 ticks, against about 240 from tick 60,000 in the first build. The living had 0.64 to 1.70 broken genes each at the end (32b 0.43 to 1.10). The 5,420 hatchlings old age killed lived a median 6.2 hours, a tenth under 4.8 and nine tenths under 7.4 (32b 6.4, 4.8 and 7.4). G4: none of the 137 numbers moved a tenth the same way in all 10 seeds; `grows_old` rose in 9.

### 18.7 Later

- **Copying the act itself** (Q2): seeing a sprite do something to a thing makes the young want to do the same, whatever came of it, so quirks run in families like traditions.
- **Going back to where it last saw its parent** (Q6), if the young lose their parents on trips.
- **Parents teaching on purpose,** feeding and correcting their young ([#67](https://github.com/Keazra/terra-sprites/issues/67)).

---

## 19. A harsher world (Slice 29)

Settled with the owner over three tickets: [#237](https://github.com/Keazra/terra-sprites/issues/237) (2026-10-17, why the species seemed to wear down), [#224](https://github.com/Keazra/terra-sprites/issues/224) (2026-10-15 and 17, over-eating) and [#225](https://github.com/Keazra/terra-sprites/issues/225) (2026-10-18 and 19, the dials for a hard childhood), with what G5 counts from [#223](https://github.com/Keazra/terra-sprites/issues/223) (2026-10-15) and G1's bar from [#222](https://github.com/Keazra/terra-sprites/issues/222) (2026-10-14). Until this slice nearly every young sprite grew up, with or without anyone to learn from: #223's cloud runs of `main` at `a5fd7ad`, with a scratch build that wasn't committed, found that all 300 newborns of an unattended run did, as §19.6 counts it, and 98.8% of the young hatched in the default world (seeds 1 and 2, 200,000 ticks each). So selection saw only breeding, and the genes the species lost, such as the one that stops a sprite drinking once it has had a sip, spread because a sprite did better without them in an easy world (#237). The fix is a world where needs matter, not a change to how genes are copied.

The owner's principle (§0): **grown sprites fend for themselves; the young survive by luck or guidance.** So:

- the world has **less water**, in fewer, bigger lakes, with the first sprites starting near it (§19.1);
- a sprite **acts only when something moves it,** and otherwise stands where it is; the young, with no pull to wander, stay put (§19.2);
- **hunger and thirst start sooner** (§19.3);
- **one gut** holds food and water, and a full sprite is held back by its body and by a feeling, not by a rule (§19.4);
- a young sprite's **thirst is paced to its childhood,** so a species can't escape it by growing up sooner (§19.5);
- **berry bushes put up seedlings,** so they hold out where sprites eat every berry, and three bars say whether childhood is hard enough (§19.6);
- **what a sprite takes on by watching is kept apart** from what it found out itself (§19.7).

**The world and the genes.**

| The world (the same for every sprite) | The genes (vary from sprite to sprite, and evolve) |
|---|---|
| How much water there is and where the first sprites start: the world's settings | How strong a pull it takes to act, and how restless it is |
| A sprite with nothing to do stands; rest costs less than standing awake | How fast standing about bores it |
| The gut's room, and how fast each thing in it moves into the body | When hunger and thirst start, and what fullness does |
| A bite or a drink that doesn't fit isn't taken, and says so | How full it feels when a bite or a drink is Too full |
| A young sprite's thirst is paced to the childhood its genes give it | How long that childhood is |

Nothing here names a food or a drink: a pack's physiology sets how fast the gut's two contents, food and water, move into the body and whether each waits for room, so it chooses for all its food and all its drink (§19.4).

### 19.1 Less water, nearer the first sprites

- **Two world settings** (#225 round 1): `water`, the share of the map's tiles under water, from 1% to 60%, and `lake_size`, from 1 to 5, how many times broader the map's hills and hollows are, so the same water lies in fewer, bigger lakes. A preset that leaves them out gets 32% and 1, the watery world of the game before this slice.
- **The other bands follow the water.** This changes M1 design §3.2's fixed bands: deep and shallow water keep its proportion of 20 to 12, sand is an 8% band of heights just above the water, rock the highest 10%, and land the rest, the driest 40% of it dirt as before. So the default world is 2.5% deep water, 1.5% shallow, 8% sand, 78% land and 10% rock, and `water: 32` gives M1's bands.
- **The default world** has 4% of its tiles under water, in lakes three times the size. The watery world stays as a preset, `data/presets/gentle.ron`, played with `--preset data/presets/gentle.ron`.
- **The first sprites start within 10 tiles of drinkable water,** as the crow flies, counted in king steps (a diagonal step counts as one): the world setting `start_near_water`. This changes where M1 design §3.2 places them. They take the free tiles that near while any is free, and then the rest of the map's free tiles, so a small, dry map still gets its whole first population. Leave it out for anywhere on the map, as the gentle world does. The unattended run's newborns are spread over the whole map, near water or not (§19.6).

### 19.2 Acting only when moved

- **A sprite acts only when something moves it** (#237 rule 3): a need, an instinct, a habit or what something's worth. With nothing pressing, it stands where it is, where before it picked an act at random. It's how the basal ganglia hold actions back in real brains.
- **How strong a pull it takes is a brain parameter in the genes,** `act_threshold` (ID 45, from 0 to 2, default 0): a sprite with nothing to do starts only a verb scoring at least this. At 0 it always starts one, as before. The starter's sprites need **0.25** (#225 round 1).
- **The young stay put** (#225 round 1). `always → Wander` stays, a restless streak evolution can strengthen, weaken or lose, but the starter's splits by life stage: **0.15 until adult**, too weak to start anything on, and **0.3 from adult**.
- **While its eye stays on its action's target, a busy sprite still switches by `switch_margin` alone,** as before (M1 design §5.5): the threshold holds back a sprite with nothing to do, and one whose eye has left its action's target (below). So a busy sprite whose action has lost its pull can switch to a verb under the threshold, a young one to its 0.15 Wander.
- **When its eye leaves its running action's target** (M1 design §5.3) and no verb reaches the threshold, it carries on that action, where before it chose again and the action ended. The tick is recorded as carrying it on, as a trip with no way there is (§7.4): its step 5's decision and its trace keep the running verb, its motive and its target. Its eye stays where it moved, so it still watches what drew it (§18.1) and grows familiar with it (M1 design §5.6), and the Brain tab marks the row its eye is on.
- **Standing about builds boredom faster,** through a gene: a new body sensor, `standing` (locus 18), is 1 while a sprite stands where it is: no action running, and neither led by the Cursor nor sliding from a shove, which would move it whatever it chose. The starter's `standing → boredom` adds **0.0005** a tick, so a still sprite soon goes off to play.
- **Rests cost a fifth less** than standing awake (#237 rule 6): physiology's `metabolism.resting`, 0.8, multiplies the basal and sense costs while it rests. Rest stays apart from standing about. Real sleep, as a feeling of its own, is M3's ([#288](https://github.com/Keazra/terra-sprites/issues/288)).
- **What a standing sprite does with its eye** doesn't change: it still attends, and watches (§18.1).
- **The player sees** a standing sprite's doing line, on the Body tab and in the sprite list, say what it's watching, `Watching Bo #7` or `Watching the berry bush`, or `Standing` if nothing; in the detail view `STAND → sprite #7`. It goes by the `standing` sensor's rule, except that an action that ended on the latest tick still shows how it ended until the next tick. Before this slice a sprite was without an action only before its first decision, and an ending stayed until the next action started (M1 design §6.1). The Brain tab's decision reads `DECISION: none`.

### 19.3 When hunger and thirst start

Hunger and thirst start once energy or hydration falls **below 0.7** (#225 round 1), where they started below half, so a sprite goes looking while it still has the strength to get there. It's the threshold of the starter's `energy → hunger` and `hydration → thirst` genes, 0.3 inverted, so evolution can move it.

### 19.4 One gut

The owner's design (#224): "Anything that's consumed, whether food or water, should go into a single gut. The higher the gut is to full, the less appetising items that slake food and water become. That way, if they overeat on food, they're less likely to continue eating more food - but water may still be appealing enough if they're dehydrated."

- **One gut,** of **0.5**: room for about a berry and a drink together (#225 round 1). Physiology's `digestion.gut` sets it, at most 1, since each thing in the gut stops at 1. A pack that leaves it out keeps two guts with no limit.
- **A slurry, each thing at its own pace.** What's in the gut is mixed, not queued. Every tick the gut's food and its water each move into the body at their own `pace`, which physiology's `digestion` sets for each: water fast and food slowly (0.002 and 0.001 a tick, as before). Every food shares food's pace, whichever object it came from.
- **Food waits for room; water passes through.** Food moves into the body only as the body has room for it, so nothing a sprite eats is wasted. Water the body can't hold is lost, as before. Physiology's `digestion` says, for food and for water, whether it `waits`, so a pack chooses for all its food and all its drink.
- **A full gut takes no more.** A bite or a drink that doesn't fit ends **Too full**: the food or water stays where it was, on the bush or on the ground, and it teaches nothing, unlike a fruitless try. A new pulse, `too_full` (locus 49), fires.
- **Fullness, by instinct.** A new chemical, `fullness` (ID 52), a **hormone**: the starter's genes make it from everything in the gut, reaching 1 as the gut fills, jump it by 1 on a `too_full` pulse, and let it fade within about 20 ticks. It drains hunger above 0.3 but not thirst, and it leans the sprite away from eating and drinking alike (`fullness → Eat` and `→ Drink`, written −2, which works as −1, the most a link leans, M1 design §5.7), though strong thirst can outweigh it: on instinct alone, thirst of about 0.85 or more drinks past a berry's fullness, and a sprite that knows water is good drinks past it sooner. A brain input, `fullness` (ID 96), reads it. It isn't a need, so its fading teaches nothing. #224 called it a drive; it's a hormone so that a fed sprite isn't coloured by it on the map, as drives are. Only the gut's room is physics: evolution can weaken or lose fullness, and a sprite that does still meets the full gut, so it only wastes its own time.
- **The player sees** the Body tab's doing line say `Too full to eat the berry`, `Too full to eat from the berry bush` or `Too full to drink`, and the action end `was too full` in its list of what it did.

### 19.5 A childhood's thirst, paced to its length

A young sprite's water drains at the grown rate **× 4,500 ÷ the ticks its genes take to grow up**, and once it has dried out, it dies of thirst at that pace too: dehydration's injury × 4,500 ÷ its childhood (#225 round 2, and the 220-hour check). Physiology's `ageing.thirst_childhood` is the 4,500. The starter's 4,500-tick childhood is unchanged by it. Unpaced, a young sprite that never drinks dries out at about 3,000 ticks and dies of thirst about 1,000 ticks later (#223). With only the drain paced, as #225 round 2 had it, a newborn that never drinks dries out about two thirds of the way through its childhood, so a childhood shorter than about 3,000 ticks ends within the 1,000 ticks or so of dying that follow, and outlasts thirst: in #225's 220-hour check, seeds 3, 7 and 8 had evolved childhoods of 1,500–2,700 ticks, whose last 500–900 ticks were too few to die in, and their newborns grew up alone 87–98% of the time. With dying paced too, lone newborns grow up 18–22% at any childhood length.

### 19.6 The bars, and bushes that seed

**Who's counted** (#223):

| | Who | Counted in | Bar |
|---|---|---|---|
| **Unattended** | A young sprite with nobody to learn from: no player and no grown sprite | **The unattended run** (`scenarios/unattended.ron`): the default world on seeds 1–10 as it generates, with its 30 first sprites as newborns, placed as the Cursor places them, spread over the whole map, and nobody else, until each has grown up or died, about 5,000 ticks | **10% to 30%** grow up, in the middle seed of ten |
| **Guided** | The young hatched in the default world left alone, nearly all with a parent nearby and grown-ups to watch | The viability run (50,000 ticks), all ten seeds together; young still growing at the end aren't counted | **At least 60%** grow up |
| **Grown** | Sprites that grew up: adult or old, by life stage, not by tick | The viability run, from its first tick: the first sprites start as adults | Of the grown sprites that die, **fewer than 1 in 20** are adults dying of hunger or thirst. Old sprites' are reported apart |

- **Not the player.** The owner (#223): "The player is an X factor to Sprite survival, so this is measuring against the world running without interference."
- **The guided bar has no upper end in M2** (#225 round 1). #223 set 60% to 90%, so that childhood stays dangerous even in a family; M3's dangers bring it back into that band.
- **A4 and G5 are one check** in the baseline report (M1 design §7.4, §7.6), which runs the unattended run beside the viability run. "Alive at tick 10,000" is gone.
- **The lab counts childhoods.** Each window of a lab run counts how many of the world's first sprites that started young, and how many of the young that appeared later, grew up or died young, and every death by the life stage it came at: rows `grew up: first`, `died young: later`, `died, adult: dehydration` and so on.
- **The long run** (#222, #237, #224, #225): `cargo run --release -p terra-sim --example childhood` runs the default world, left alone, for 220 hours (990,000 ticks) on seeds 1–10, and reports every 10 hours, then the bars. A seed takes about 3 hours on the owner's PC once its world fills, so it runs there, at about half the PC's threads.
  - **In each seed:** the grown bar above, over the whole run and over its last 50 hours, in every seed that lasts (#226); and #224's, **at most 1 in 10 of the bites young sprites and adults try are by one too full to eat,** over the run and over its last 50 hours. A bite tried is an Eat that reached its food and made its one try (M1 design §5.5), a Too full one too; one that ended on its way isn't. Old sprites' bites, and how many of those were Too full, are reported beside the bar, not in it (#225 round 1).
  - **Across seeds, each met in at least 9 of 10:** **the world lasts** (G1, #222): at the end it still has sprites or eggs, it never had fewer than 10 sprites, eggs not counted, at any count of its population record, and its sprites over its last 50 hours averaged at least half of what they averaged over hours 50–100; and **the bushes hold** (#224), averaging over the last 50 hours at least half of hours 50–100. A seed that misses one of these has failed it only if the bar isn't met. The 50-hour averages need a run of 150 hours or more, and a shorter one says they weren't judged.
  - **Evolution (G4), over the seeds that last** (#226): at least 3 gene numbers whose middle value among the living at the end moved a tenth from the starter's, the same way in every one of them. A seed that didn't last, or had no sprite alive at the end to measure, drops out of it rather than failing it. Each number is named by what its gene does and when it works, so the young's and grown sprites' Wander weights are two numbers. This is G4's first test: if it isn't met, the race below is its second, so it fails no seed, and the long run's verdict names it apart.
  - `generations` judges whether each seed lasts by the same rule, its 50-hour averages only on a run of 150 hours (675,000 ticks) or more, and reports G4 over its own seeds that last, though the long run's is the one that counts.
- **Reported in the long run, with no bar:** the young hatched that grew up, whose bar is the viability run's (#226); adults' time on the move over the last 50 hours, which #237 asked to be a quarter and [#318](https://github.com/Keazra/terra-sprites/issues/318) now owns, and the young's (#237 rule 5); what kills the hatched young, and how those that saw a parent for most of their childhood fare against those that didn't; and the share of the living lacking each starter gene that a tenth or more of some seed's living lack (#237 rule 4), always with the twelve **needs genes** (§16.6), marked: the starter's for hunger (`energy → hunger`, `ate → hunger`, `fullness → hunger`, hunger's half-life, `hunger → Eat`), for thirst (`hydration → thirst`, `drank → thirst`, thirst's half-life, `thirst → Drink`) and for tiredness (its two emitters and `tiredness → Rest`). A gene with a life-stage switch is listed by its switch, so the young's and grown sprites' Wander genes show apart.
- **Lone newborns** (#225's 220-hour check): for each seed whose world didn't die out, with hatched sprites alive at the end of the long run, its unattended run again, its newborns taking after the hatched sprites alive then, their genomes and hidden flaws. The share that grow up is a reading, reported beside the unattended band of 10% to 30%; it isn't the test of falling behind, below.
- **Falling behind** (#237 rules 1 and 8): a species has **fallen behind**, become worse at living than the starter's genes in the same world, when its world fails its bars or it loses **the race** against the starter's genes. The race is #226's second test for G4 (Q7): for each seed that lasted the long run, the same fresh world runs the viability run twice, its first 30 sprites given the starter's genome once and genomes of sprites alive at the end of that seed's long run once. G4 is met by it if the evolved side's young grow up more often in at least 8 of those seeds. Slice 30 runs the race every time, not only when G4's first test fails. A gene the species does better without is evolution, not wear.
- **Careful copying is held in reserve** (#237 rules 2 and 4). No bias in how genes are copied: selection is the bias. Only if a world in the long run fails a bar (or the race is lost overall) while more than half its living lack a needs gene would those genes be copied carefully, lost or changed a tenth as often, like *Creatures*' gene flags. It isn't built. The long run names the seeds that would trip it, judging one that died out on its living at the last hour any hatched sprite was alive.
- **Bushes that seed** (#225 round 2). Bushes renewed only from berries left uneaten, so the more sprites ate, the fewer bushes there were (§18.2). Now a grown berry bush puts up a seedling now and then: every 2,000 ticks, 1 in 10, within 4 tiles, on fertile land with fewer than 3 bushes within 4 tiles, where a bush wouldn't cut a path. It's a rule in the bush's data (`SpreadTo`), not code.

### 19.7 What's taken on by watching, kept apart

This changes §18.2 for a thing's badness and habits (#225 round 2).

- **Kept apart.** What a sprite takes on by watching, of how bad a thing is and of each habit about it, is kept apart from what it found out itself and what the Cursor taught it, as taught habits are (M1 design §5.6). A decision weighs them together.
- **It settles either way.** Each time it watches, that part moves `watch_rate` of the way towards the other's first-hand view, up or down, rather than only further from 0 the same way. So a young sprite that watched its mother avoid a bush she has since found harmless comes round when it sees her eat from it. What it found out itself is never moved by watching.
- **Never passed on.** Of a thing's badness and habits, what a sprite shows another is only what it found out itself and was taught, never what it took on by watching, so a thing can't come to seem bad from sprite to sprite with nobody having been hurt by it.
- **It fades** as its first-hand kind does: the watched badness as badness, the watched habits as taught ones.
- **Unchanged:** worth and good still only move further from 0, as §18.2 says, and fear of a sprite or the Cursor too. They aren't kept apart, so what a sprite takes on of them by watching is passed on, as §18.2 has it: fear of a sprite or the Cursor still spreads from sprite to sprite.

### 19.8 How we know it worked

Agreed before measuring (#222, #223, #224, #225):

- **Unattended:** 10% to 30% grow up, in the middle seed of ten. #225's pick gave about 20–22%.
- **Guided:** at least 60% of the young hatched in the viability run grow up.
- **Grown:** adults dying of hunger or thirst under 1 in 20 of grown-up deaths in the viability run. #226's acceptance asks for it in every seed of the long run that lasts: on seed 4, where water is furthest away, it was 6.4% over #225's 220-hour check (2.7% over all seeds), so the long run watches it.
- **The long run's bars,** above, on the owner's PC: G1's among them (#222), and G4, deferred to this slice from Slice 24 (§12.7) and judged over the seeds that last (#226).
- **Nothing else got worse:** A1–A3 still met (A3 on seeds 1–30) and every test passes. Slice 32's family bars (§15.9), which #226 counts over the first 200,000 ticks of the long run, come from `cargo run --release -p terra-sim --example generations -- --ticks 200000`, on the owner's PC.

`cargo run --release -p terra-sim --example lab -- scenarios/unattended.ron --seeds 10` runs the unattended run, and `scenarios/viability.ron` the viability run, each with its counts by life stage.

**Measured** (2026-10-19, in the cloud at 0aedf5b), with `cargo run --profile baseline -p terra-sim --example baseline` on seeds 1–10 and `cargo run --release -p terra-sim --example lab -- scenarios/a3-correct-training.ron --seeds 30`:

- **Unattended:** 20% in the middle seed (9, 13, 7, 2, 8, 3, 11, 4, 0 and 5 of 30 on seeds 1–10), nearly all the rest dying of thirst: met.
- **Guided:** 1,587 of 1,622, 97.8%: met.
- **Grown:** adults' deaths of hunger or thirst 47 of 1,132 grown-up deaths, 4.2%: met. Old sprites', apart: 64.
- **A1–A3:** A1 1 thornbush contact (control 38), A2 8,414 plays (control 169), A3 3.5 hits (control 49.5) on seeds 1–10; A3 over seeds 1–30, 5.5 hits (control 40.5): all met.

**Measured on the owner's PC** (2026-10-20, at 9bc50de), with `cargo run --release -p terra-sim --example childhood` and `cargo run --release -p terra-sim --example generations -- --ticks 200000`, seeds 1–10:

- **The world lasts (G1, #222):** in 10 of 10 seeds, none under 29 sprites at a count: met, so #212 is too. **The bushes hold:** in 10 of 10: met.
- **Grown, in every seed that lasts:** not met. Adults' deaths of hunger or thirst were 2.2% of grown-up deaths over all seeds (560 of 25,384), but seed 8 had 5.2% over the run and 9.2% over its last 50 hours, and seed 10 had 8.2% over its last 50 hours. Old sprites', apart: 750.
- **Too full, at most 1 in 10 bites:** not met. 3.0% over all seeds, but 13.2% over seed 10's last 50 hours (seed 4 8.6%). In most seeds the share climbs over the hours, from about 1% at hour 10.
- **Slice 32's families:** partners had each other in sight for a median 46% of the time: not met (54% before this slice). Eggs laid alone 0.0%, 91.8% of pairs' eggs after a dance each way, an egg with a parent in sight 93% of its time and a young sprite seeing a living parent 93% of its childhood: met.
- **G4:** 2 of 148 numbers moved a tenth the same way in all 10 seeds (`grows_old`, and the grown sprites' gain of `low female_near → feminiser`), short of 3, so the race decides it in Slice 30 (#226).
- **For information:** the young hatched grew up 98.0% of the time, those that saw a parent for most of their childhood 99.0% and the rest 77.9%. Lone newborns given the end's genes grew up 32% in the middle seed. No seed tripped careful copying. Adults were on the move 15.9% of the time over the last 50 hours ([#331](https://github.com/Keazra/terra-sprites/issues/331)). At 200,000 ticks, hunger and thirst killed 5.8% of all sprites that died after tick 10,000, and the adults at the end were 561 males to 267 females.

The grown bar's and the too-full bar's misses came late in the run and grew over the generations. **The owner's call (2026-10-20):** "Lets merge and revisit harshness at a later time, when more features are in." The misses wait in [#332](https://github.com/Keazra/terra-sprites/issues/332).

### 19.9 Later

- **Roaming that pays** ([#318](https://github.com/Keazra/terra-sprites/issues/318)): adults on the move a quarter of the time. #225's check failed it in every seed (7–25%), with the world sitting full. Slice 29b builds room nearby for it ([#331](https://github.com/Keazra/terra-sprites/issues/331)).
- **Harshness, revisited once more features are in** ([#332](https://github.com/Keazra/terra-sprites/issues/332)): the grown bar on seeds 8 and 10, too-full bites on seed 10 and partners in sight (§19.8).
- **Careful copying,** if a long run trips its trigger (§19.6).
- **Parents that look after their young.** As genes change, parents stop looking for their young, because nothing threatens the young yet. M3's dangers would give that care a use.
- **The gentle world on the title screen,** as a choice when making a world.
