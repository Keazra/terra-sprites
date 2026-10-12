# Terra Sprites — M2 "Generations" design

- **Status:** Draft: the plan, with what each slice settles added as it's settled.
- **Date:** 2026-10-12
- **History:** edited in place since v5; each change is a file in [`changes/`](changes/), whose README says where v1 to v5 are
- **Builds on:** [M1 design](m1-a-sprite-lives.md), which stays the rulebook for everything M2 doesn't change
- **Covers:** what milestone 2 contains, its slices and their order, the time scale of a sprite's life, the direction for ageing, what "M2 is done" means, memory of places (Slice 20), the title screen (Slice 21), where places sit on the Brain tab, life stages and ageing (Slice 22), breeding (Slice 23), and the view past the wall (Slice 31)

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

- **These are the starter genome's times.** The genome brings them about (§3), so they vary from sprite to sprite and can evolve.
- **Lives differ by up to 2 hours either way.** The owner (2026-10-12): "4 hours should be the shortest, and eight at the longest, for an average of 6 hours." A new sprite's `grows_old` varies by up to 40% at spawn where its other genes vary by 10% (§9.3), and a hatchling takes one parent's (§10.5). Old age still lasts about an hour, and a hard life can still bring it sooner (§9.4).
- **Growing old is where M1's lifespan went** (#140). The `grows_old` gene sets when a sprite grows old, and its genes then weaken until its body gives out and old age kills it (§9.4). How long that takes depends on the life it has had, not on a second gene. A save or genome that says `lifespan` reads it as `grows_old`, whose range is about **6,000–60,000**.
- **A generation takes about 15–20 minutes at 4×,** Story mode's top speed (Slice 27), so an evening sees several. Long runs of evolution belong to the fast-forward mode.
- **Lives got shorter in Slice 23, with breeding.** Until sprites had young, nothing replaced the dead: with 4-hour lives, the default world would have been empty after about 4 hours of play. So in Slice 22 sprites grew up and grew old at 1 and 3 hours, but old age lasted about 10 hours, and Slice 23 shortened it to about 1 (§9.4), for lives of about 4 hours. Later in Slice 23 the owner set them at 4 to 8 (2026-10-12), by growing old later.
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

**Settled in Slice 22 (#140): A, B and D.** Genes switch with life stage (§9.2), and old age is the genome wearing out: each gene has a health that weakens its effect, damage lands on random genes once a sprite is old, and a hard life brings that sooner and speeds it up (§9.4). Rather than failing outright, genes weaken a little at a time, so a sprite slows and dulls as it ages.

**A long life costs something.** Anything the genome controls about ageing needs a cost, or evolution breeds sprites that never grow old (M1 design §4.1). In M1, lifespan costs nothing, so with inheritance it would creep up to the most allowed. Real animals pay for long life: repair takes energy, so the long-lived spend more on upkeep and breed more slowly. **A later old age costs a little energy every tick,** as a bigger sense radius does (M1 design §4.8): how much, and when, is in §9.3.

---

## 4. The slices

Each slice is a GitHub issue and its own PR, worked as in `docs/agents/how-we-work.md`: its open questions are settled first, in plain words, and recorded in this document, in a file in [`changes/`](changes/) and in `CONTEXT.md`. Each issue lists its slice's questions. Numbering carries on from M1's last, slice 19, so every slice number stays unique.

| Slice | What the player sees | Needs first |
|---|---|---|
| **20. Memory of places** | A thirsty sprite heads back to water it found earlier, even out of sight | — |
| **21. Title screen** | A screen at launch, before any world; flags that make or load a world skip it. Opens with a design pass: mock-ups to pick from, then a rough prototype played in Windows Terminal | — |
| **22. Life stages and ageing** | The young look and act young; sprites grow old and decline (§3) | — |
| **23. Breeding** | Adults have young, alone or with a partner as their genes decide; only between adults (§5); lives of 4 to 8 hours (§2) | 22 |
| **24. Mixing and mutating genes** | Children differ from their parents; gene quality joins ageing | 23 |
| **25. Family tree** | A sprite's parents, grandparents and children, living or dead | 23 |
| **26. Population graph** | The population over time | 23 |
| **27. Fast-forward and the speed cap** | A world run for many generations with no screen; 4× in Story mode, faster in Lab mode | 23 (and 21, if Lab mode is chosen there) |
| **28. Learning by watching** | The young copy what adults do, their parents most | 22, 25 |
| **29. A harsher world** | About 20% of unattended newborns grow up | 20, 28 |
| **30. M2 acceptance** | The criteria of §6, measured and played | all |
| **31. The view past the wall** | The view scrolls past the glass into space, with stars going by (§11) | — |

- **20, 21 and 22 can start at once.** 21 touches only the front end. 20 and 22 both touch the brain and the body, so running them side by side means merging carefully.
- **24, 25, 26 and 27 can run side by side** once 23 is in.

---

## 5. Content safety: mating only between adults

The owner's one content-safety rule ([#69](https://github.com/Keazra/terra-sprites/issues/69), `docs/agents/how-we-work.md`): **attraction and mating attempts towards a sprite that isn't an adult are impossible,** for every genome and every data pack. Data can be edited and genes evolve, so the engine enforces it, the one exception to "the engine enforces physics only".

Slice 23 builds it:
- `Mate` is never offered unless both sprites are adults (an old sprite counts as one, §9.1). It's a hard check in verb availability, not a learned or genetic lean. Nor is `BreedAlone`, unless the sprite is grown (§10.3).
- Attraction, whatever form it takes, never targets a non-adult. **The breeding drive works only between adults** (Slice 23): `physiology.ron` names it (`breeding.drive`, the starter's `arousal`), and whatever the genes, it reads 0 in a sprite not grown up, and in an adult's choices about one: it draws no eye to it, moves the sprite to do nothing to it, and its body language shows none, to other sprites or on screen: the engine's views read it as 0 in a young sprite. Its fall teaches worth only after a Mate, and only about that partner, never sprites in general or new things (§10.2).
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

- **It knows the way.** Setting off, it searches a flood out to the place's distance plus its sense radius, which reaches the place wherever the walk winds, weighing tiles with sprites on as its flood does (M1 design §3.6). It keeps to that path, as to a committed path (§3.7). With no way there, such as a bush grown across the only path, it forgets the place and doesn't set off: it carries on with what it was doing, decides nothing that tick, and chooses again at its next step 5. A running action it changed its mind about ends only once the new one is sure to start, so a place it can't get to never interrupts anything.
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

Nothing extra in Slice 20. A child that follows its parent to water and drinks there remembers the place itself, by §7.1. Slice 28 decides whether watching another sprite drink teaches the place too.

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

- **At launch the screen is dark.** The Cursor's frame of light (M1 design §0) fades in over one sprite near the middle of the screen, asleep (`z`). The world is revealed in a circle spreading out from the light, its edge drawn as `░`, until it fills the screen. The sprite wakes pleased (`♥`), then the title fades in, and the menu appears under it.
- **It takes about 5½ seconds,** tuned by feel: the light arrives by 1 s, the sprite wakes at 2.5 s, the light fills the screen by 4 s, the title fades in from there and the menu shows at 5.5 s.
- **It plays at every launch,** and when the player comes back from a world (§8.4). Any key or click skips to its end and does nothing else, so a key pressed to skip never also picks from the menu.
- **The world** is a new one with a random seed, made from the default preset of the data pack in use (the built-in one, or `--data`'s, so a mod's species and items show), whatever preset `--preset` names, on a map the size of the terminal. The preset's densities scale to that map, as they do for any map (M1 design §3.9), and its first population stays, so the screen is lively. A map side stays within what a generated map may be: at least 32 tiles, so a 30-row terminal shows the top of a 32-row map.
- It **holds still until the light has filled the screen,** then runs at 4× behind the menu for as long as the title screen is open, at most 5 ticks a frame. It's never saved, and nothing the player does on the title screen touches it.
- **What it is in the terrarium's story:** the player arriving at the glass, before reaching in. The dark is the terrarium without the player's light, and the light finding a sprite is the first touch. Nothing on screen says so.

### 8.2 The menu

A box in the middle of the screen, over the moving world, holds the title lettering and the menu. The choices are numbered and picked as the Place menu's are (M1 design §6.5): `↑` `↓` and `Enter`, the wheel, a number, or a click.

| Choice | What it does |
|---|---|
| **Continue** | Loads the newest save in the saves folder (any kind: the quicksave, an autosave or one by name), named beside it with how long ago it was saved: `autosave-1 · 2 hours ago`. Hidden when there's no save. |
| **New world** | Opens the New world box (§8.3). |
| **Load** | A list of every save, newest first, as `Ctrl+O` opens in a world (M1 design §6.7). |
| **Help** | The help overlay, as `?` opens in a world. |
| **Quit** | Closes the game. |

- **`Esc` asks "Quit? (y/n)",** as in a world; only `y` quits. `Ctrl+C` quits at once.
- **A world started or loaded from the title screen starts as one does from the flags:** a new world at 1×, a loaded one paused (M1 design §6.7). The session log starts afresh from it (M1 design §2.7). A save that can't be read, or a preset that isn't valid, is refused on the status line and the title screen stays.
- A terminal under 100×30 shows "Terminal too small", as a world does (M1 design §6.1), and `Esc` still asks to quit under it.
- Everything drawn stays within CP437 (M1 design §6.2).

### 8.3 New world

A small box over the menu, with:
- **Seed:** a random one filled in. Typing a digit replaces it, then more digits add to it, as a save's name is typed (M1 design §6.7); `Backspace` rubs out, and `Tab` offers another random seed. A seed is a whole number from 0 to 18446744073709551615.
- **Preset:** `↑` `↓` pick one from a list: `default` first (the built-in preset, or `--data`'s own `presets/default.ron`), then each `.ron` file in the `presets` folder of the game's folder (`%APPDATA%\terra-sprites\presets` on Windows), by name. A preset is checked against the data pack in use when the world is made.
- `Enter` starts the world; `Esc` goes back to the menu.
- Once Slice 27 builds Lab mode, the choice between Story mode and Lab mode goes in this box, and the world keeps it (2026-10-12; M4 design §11).

### 8.4 Leaving a world

- **`Esc` from Select** (M1 design §6.5, §6.6) asks "Quit? y quit the game  t title screen  any other key stays". `y` quits, as before; `t` goes back to the title screen, whose scene plays again on a new world. Any other key, `Esc` included, cancels.
- **Both save first,** as quitting does (M1 design §6.7): an autosave, unless the world hasn't run since it was last saved. The session log is written too. If either can't be, the title screen says why on its status line, and quitting says why on the terminal once the game has closed.
- `Ctrl+C` still quits at once.

### 8.5 Flags

- **`--seed`, `--preset` and `--replay` make or load a world,** so they skip the title screen and go straight in, as before. Their world can still leave for the title screen with `t`.
- **`--data`, `--ascii` and `--theme`** change what the game's worlds are made of or how they're drawn, not which world it opens on, so the title screen still shows, drawn with them, and a new world made from it uses `--data`'s pack.

---

## 9. Life stages and ageing (Slice 22)

Settled with the owner on 2026-10-11 ([#140](https://github.com/Keazra/terra-sprites/issues/140)). A sprite grows up and grows old at ages its genes set, a gene can switch on or off as it does, and once it's old its genes weaken until its body gives out. The numbers are data, the `ageing` section of `physiology.ron`: first guesses, tuned in play and in the viability run with A4 holding.

### 9.1 Life stages

- **A sprite's life stage is young, adult or old,** after an egg from Slice 23 (§10.4). "Stage" alone is an object's (M1 design §3.5); a sprite's is its life stage. An old sprite still counts as an adult for §5's rule.
- **It's young until it grows up, an adult until it grows old, then old** (§9.3). One whose genes have it grow old before it grows up does both at once, and is never an adult.
- **The stage changes at the start of step 3** (M1 design §2.4), so genes that switch with it work from that tick's chemistry on.
- **A world's first sprites start as adults,** at ages drawn evenly through adulthood, with a newborn's body (M1 design §4.7), so a fresh world isn't all children. A sprite the player places is a newborn. A scenario's sprites start just grown up, unless it lists them as newborns.
- **An egg comes first:** since Slice 23, a sprite's life starts as an egg, which hatches into a young sprite (§10.4). An egg isn't an adult, and in what genes and switches see it counts as young.

### 9.2 Genes that switch with life stage

- **A gene can carry `from` and `until`,** a life stage each: it works from `from` (young if it has none) until `until` (never, if it has none). Outside them it's **dormant**: in the genome, but not working.
- **Traits and starting levels can't switch.** Traits work all life, since two of them set the stages, and a starting level applies only at birth. Such a gene that carries a switch is flagged and doesn't work (M1 design §4.3), as is one that switches off before it switches on.
- **When its stage changes, a sprite's genes are expressed afresh:** its chemistry, brain parameters, concepts and instinct links become those of the genes working now. What it has learned, what it attends to and what it remembers doing stay as they were. So an instinct that switches on mid-life joins the instinct pathway without touching what was learned (#49).
- **Genome files with switches are format 2;** format 1 still loads, and a format 1 file with a switch is refused.

### 9.3 When a sprite grows up and grows old

- **Two trait genes set the ages,** so each can evolve on its own: `grows_up` (1,500–13,500 ticks; the starter's 4,500 is about an hour at 1×) and `grows_old` (6,000–60,000; the starter's 22,500 is about 5 hours).
- **`grows_old` varies more at spawn** (2026-10-12): by up to 40%, physiology's `trait_variation`, where every other gene varies by `spawn_variation`'s 10% (M1 design §4.9). So the world's first sprites, and those the Cursor places from the starter genome, grow old at 3 to 7 hours and die of it at 4 to 8 (§2).
- **`grows_old` is M1's lifespan,** which it replaces (§2). A genome or save that says `lifespan` reads as `grows_old`. The `age` body sensor is age as a share of it, so it reaches 1 as a sprite grows old, at about 22,500 ticks rather than M1's 60,000.
- **Growing up costs food.** Growing takes `growth` energy (0.3) whenever it happens, spread evenly over childhood, so a sprite that grows up sooner is a hungrier child.
- **A later old age will cost a little,** all its life: `upkeep` energy a tick for every 10,000 ticks of `grows_old` (§3). **It's 0 until genes evolve** (Slice 24, noted on [#142](https://github.com/Keazra/terra-sprites/issues/142)), the owner's call on 2026-10-11: until then nothing can push old age later, so the cost only made every sprite hungrier, and even 0.00001 undid A3's training (median hits after training 45 against 81 untrained; 1.5 against 70 without it).

### 9.4 Old age: genes weaken

The owner (2026-10-11): old age is the genome wearing out, and a hard life damages genes. Research backs the shape below: damage to DNA happens all life and is mostly repaired, repair falls off in old age, species die at about the same level of damage, and chronic stress makes cells age sooner (sources in the PR).

- **Every gene has a health,** from 1 (whole) to 0. It only ever falls, and it scales the gene's effect: a feeling's gain, a receptor's gain, a reaction's rate, an instinct's weight, and how far a speed or sense radius gene reaches above the least its trait can be. A gene that's a setting, not a strength, doesn't weaken: half-lives, starting levels, brain parameters, and the timing genes.
- **Before old age, repair keeps up.** Once old, damage lands on random genes, a `hit` (0.01 of health) at a time, so health falls in small, frequent steps (the owner, 2026-10-11). **An instinct takes half,** so the mind outlasts the body.
- **The pace:** an unworn sprite's genes fall to `gives_out` (0.5 on average) in about `decay.ticks` (3,600; a unit test, `an_old_sprite_s_genes_decay_until_its_body_gives_out_in_about_the_ticks_set`, holds it to 3,100–4,100), and old age kills it soon after, so old age lasts about an hour at 1× (Slice 23; it was 46,500, about 10 hours, until sprites had young). When that takes more than one hit a tick, as it does for a worn sprite, the whole hits land each tick and the odds are of one more.
- **A hard life wears the body for good,** and wear never heals: `wear.starving` (0.0002) a tick while energy is under `wear.low` (.25), `wear.parched` (0.0002) while hydration is, and `wear.injured` (0.001) a tick for each unit of injury, so a hit or a prick wears the body through the injury it does, for as long as it takes to heal. It reads the body, not feelings, which genes could switch off.
- **Wear brings old age sooner and speeds the decay.** The most worn sprite grows old at (1 − `sooner`) of its `grows_old` (`sooner` is .33), and its genes decay `wear_speedup` (3) times as fast; less worn, in proportion.
- **A bad injury can scar a gene at any age:** while injury is above `scar.above` (.5), each tick has a `scar.chance` (0.002) that a random gene loses `scar.hit` (0.05; half that on an instinct).
- **Healing slows** in proportion to its genes' average health, since repair is what fails.
- **The body gives out** once that average is at `gives_out`: old age then injures it (`injury.old_age`), and it dies of old age. An old sprite whose thirst feeling has weakened may die of thirst first.
- **It isn't inherited.** The damage is in the body, not in its eggs. Inherited gene damage is Slice 24's; stress chemicals such as adrenaline damaging genes join then (noted on [#142](https://github.com/Keazra/terra-sprites/issues/142)). Mild stress lengthening life (hormesis) is a later idea, and botched repairs leading to cancer is [#178](https://github.com/Keazra/terra-sprites/issues/178).
- **Gene health, wear and life stage are world state:** saved, in the state hash, and checked every tick (M1 design §7.1): one health per gene, each from 0 to 1, and wear from 0 to 1.

### 9.5 The young

- **They walk slower:** `young_speed` (.67) of their speed at birth, rising steadily to all of it as they grow up.
- **They're more curious and learn faster.** The starter genome sets `curiosity` to .5 until adult (adults .3), and `plasticity` to 1.5 until adult: every learning rate is multiplied by it (M1 design §5.6).
- **Only adults fight for room:** the starter's "crowded, so hit a neighbour" instinct works from adult. The young still hit back when hit.
- **`plasticity` is a new brain parameter** (ID 38, range .25–4, default 1). A pack from before it gets the built-in range.

### 9.6 What the player sees

- **On the map, the young are drawn small:** `ö` in CP437 and `a` in ASCII, the theme keys `young_sprite` and `selected_young_sprite`. A theme without them draws the young as grown. The old show by moving slowly.
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

Settled with the owner on 2026-10-12 ([#141](https://github.com/Keazra/terra-sprites/issues/141)), in a round of ten questions and two decision cards. Grown sprites have young, alone or with a partner as their genes lean, and only ever between adults (§5). Seeing the slice built, the owner set its direction: sprites are a social species, and a female laying alone should be very rare. Pairs can't yet carry a world on their own (§10.8), so for now a female lays alone only after a long while with no male in sight (§10.3), and Slice 32 ([#186](https://github.com/Keazra/terra-sprites/issues/186)), courtship and family units, makes it very rare. The numbers are data, the `breeding` section of `physiology.ron` and the starter genome: first guesses, tuned against §10.8. A save from before this slice keeps the pack it was made with (M1 design §2.8), which has no breeding, so its sprites never have young.

### 10.1 Sexes, by neighbours

The owner chose sex by neighbours, as in clownfish and gobies, over hermaphrodites, two fixed sexes and mating types.

- **Every sprite hatches male,** and no young sprite changes.
- **A body sensor, `female_near`,** is 1 while a female adult is within the sprite's sense radius, and 0 otherwise. `female` is 1 for a female. The round had it count how many females are near; one is enough to keep a male male, so it's on or off. `male_near` is the same for a male adult; it feeds longing (§10.3).
- **A hormone, the `feminiser`, decides when a male turns.** The starter genes raise it while the sprite, grown up, sees no female adult and lower it fast while it sees one: a male long without females (about 800 ticks) turns. The genes set the pace, so evolution can tune it.
- **Past `turns_female` (0.8), an adult male turns female for good,** at the start of step 3 (M1 design §2.4), as its life stage changes. It's the engine's rule, like growing up: the genes move the hormone, the body does the rest.
- **Only a female lays eggs.** A pair needs a male and a female; a female can breed alone (§10.3).

### 10.2 Fertility and arousal

The game's rule, "the world decides what happens to the body, the genes decide how it feels":

- **The body says whether a sprite can breed.** In a pack with breeding, it's **fertile** while it's grown (old counts), its energy and hydration are both above `fed` (0.5), it's at least `rest` (1,500, and never less than 1) ticks since its last egg, laid or fathered, and the world has room for another sprite (§10.6). The `fertile` body sensor reads it. `fed` was 0.6 in the round; in tuning, at 0.6 a pair was seldom fertile at the same time, so it's 0.5.
- **The genes say whether it wants to.** **Arousal** is a drive and a need, second-order like loneliness (#50). The starter genes raise it fast while the sprite is fertile, drop it at once when it breeds (the `bred` pulse), and drop it within a few ticks of its no longer being fertile, so it reads as fertility (§10.3). It draws the eye to sprites.
- **Only mating relieves it, in what a sprite learns.** Arousal's fall teaches worth only when the sprite's latest touch was a Mate, and only about that partner: it learns which sprites it bred with were good for it, and what it learns never spreads to sprites in general or to new things, so it never learns that a young sprite is. Whatever the genes, arousal reads 0 in a young sprite and in an adult's choices about one (§5).
- It was called **broodiness** in the round; the owner renamed it on 2026-10-12. Brooding is sitting on eggs, which may come later.

### 10.3 Mating and breeding alone

- **Two actions, chosen by instinct** (the owner, Q1): `Mate`, with the sprite attended to, and `BreedAlone`. They show on the doing line and the Brain tab.
- **`Mate` is offered only when both are adults** (§5): the sprite, and the one the verb would be aimed at, a running action's target or else the candidate (M1 design §5.5, #173). It works, once, if both are fertile, one is male and the other female, and she has a free tile beside her: she lays an egg there (§10.4). The partner learns about the one that mated with it too, as if touched. Otherwise it's a fruitless try.
- **`BreedAlone` is offered only to a grown sprite.** It works for a fertile female whose **longing** has reached `breeds_alone` (0.5), with a free tile beside her; for a male, or a female not fertile or not longing enough, it's a fruitless try.
- **Longing** is a hormone the starter genes raise while a grown sprite sees no male adult and lower fast while it sees one: about 1,100 ticks with no male in sight takes it to 0.5. It came after the owner saw that 9 eggs in 10 were laid alone. Most of those came from chance: a sprite now and then tries what it isn't drawn to (M1 design §5.5), and for a fertile female, breeding alone always worked. The body's rule stops that; the genes set how fast longing builds, so evolution can still tune it.
- **The egg costs `cost` (0.3) energy** (Q2): split between a pair, or all hers alone. Per bite of food, both pass on as much of the parents' genes, so neither wins on cost; what makes partners pay is Slice 24, when a sprite breeding alone passes on every broken gene it has.
- **Body language** (the owner's card, 2026-10-12): a sprite sees how aroused the sprite it attends to is, `target_arousal`, and whether it's of the other sex, `target_other_sex`. Before it, in tuning, most tries to mate met a partner of the same sex or one not fertile, and nearly every egg was laid alone. These are brain inputs `brain_io.ron` lists like any other: one that reads `Target(...)` reads the sprite attended to, and one that reads `Differs(...)` how far that differs from the sprite's own; both are 0 unless attention is on a sprite that has hatched, and any that reads arousal is 0 unless both are grown (§5). Attention can't read them, as it can't any input about what it attends to (M1 design §5.3).
- **The starter's instincts:** arousal draws attention to sprites; arousal with an aroused sprite of the other sex in view leads to Mate; and in a female longing for a male, arousal leads to breeding alone.

### 10.4 Eggs and hatching

- **An egg is the new sprite, at the first life stage, Egg** (Q5). It's laid on a random free tile beside its mother.
- **An egg feels, learns, moves and needs nothing:** its body stays as it was laid and it decides nothing. Step 3 only counts it towards hatching, and steps 4 to 6 pass it by.
- **It hatches after `hatches` (750) ticks,** where it lies, as a young male with a newborn's body; its age counts from then.
- **Other sprites see an egg as a thing of its own kind,** the category `egg` and the pseudo type `egg`, not as a sprite: so "crowded, so hit a neighbour" never aims at one, and `nearby_sprites`, `female_near` and `male_near` don't count one. Its verb table is empty, so whatever a sprite tries on it is a fruitless try: an egg can't be hurt until something in the world is meant to hurt eggs.
- **The Cursor can't touch, lead or hold an egg;** it says it's still an egg. Whether it can move one is for an issue of its own. The player can name one.
- **A sprite remembers its parents:** the one that laid it, then its partner.

### 10.5 The child's genes

- **Mixed** (Q6): each gene, with its switch, comes from one parent or the other at random, where the two genomes line up gene for gene, as the starter's do. Where they don't, as with one placed from a genome file, the child takes its mother's.
- **The damage old age did to a parent's genes isn't passed on** (§9.4); a child's genes are whole. Mutation is Slice 24's.

### 10.6 What stops the world filling up

- **Food and a limit** (Q7): births need fed and watered parents, so food holds numbers down. The **limit**, the preset's `most_sprites` (100 in the default world, eggs counted), is the safety net: in a full world no sprite is fertile, so none is aroused and none tries in vain. In the terrarium's story, the limit is the glass.
- **The Cursor may place a sprite past the limit:** that's the player's call, not the sprites'.
- **A lab scenario can set `most_sprites`.** A1 to A3 and the thorn trap watch only their own sprites, so they hold no more than they start with (an egg may replace a sprite that died).

### 10.7 What the player sees

- **The log:** `Pip #12 laid an egg (with Bo #7)`, `Pip #12 laid an egg alone`, `Sprite #31 hatched (Pip #12 and Bo #7's)`, `Pip #12 turned female`.
- **The map:** an egg is `o` in CP437 and `0` in ASCII, where `o` is the ball; the theme keys `egg` and `selected_egg`. A theme without them draws eggs as the young.
- **The Body tab:** after the traits, the sex, `· fertile` while it is, and for a male, how near it is to turning, `male · fertile · turning 40%`; then `parents Pip #12 and Bo #7`. An egg shows `egg · hatches in 750 ticks` for its life stage.
- **The sprite list** shows `egg` as the life stage; **the doing line** says `Going to mate with Bo #7`, `Mated with Bo #7`, `Breeding alone`, `Laid an egg alone`.
- **Arousal** colours a sprite light magenta when it's its strongest drive above half.
- The full family tree is Slice 25.

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
- **Lives: about as the owner set them, a little long.** The median hatchling lived 5.9 to 7.6 hours a seed, 6.5 in the median seed; a tenth died by 4.3 to 5.6 hours, and nine tenths by 6.6 to 7.9. The few lives under 4 hours are sprites a hard life wore into old age sooner (§9.4). Lives run long because a sprite that lives longer has more young, who take its `grows_old` (§10.5): nothing pulls the other way until Slice 24 gives a later old age its cost (§9.3). Before, with lives of about 4 hours, a tenth died by 3.7 hours and nine tenths by 4.3 (all seeds together).
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

Past the glass is space: black, with stars drifting right to left, since the ship flies towards the right of the screen (round 1, Q5). Sideways drift looks smoother than up or down, because a terminal's characters are twice as tall as they are wide.

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

### 11.5 Themes

Both built-in themes gain the glass's six glyphs and the stars' three depths, as theme keys (`docs/reference/themes.md`). In ASCII the glass is `-` along the top and bottom, `|` down the sides and `+` at the corners. A theme without the new keys draws the built-in theme's.

### 11.6 How we know it worked

- **UI tests** (ratatui's `TestBackend`): scrolling stops at the margin on every side; the glass runs all round the map, one tile out; the border is single-lined; stars appear only on space; stars stand still while paused and drift as far at 16× as at 4× in the same time; a small map is centred, and one as wide as the map view, or a tile narrower, scrolls to show its glass; with the pointer moved over space after the Cursor was on a sprite, the Cursor isn't drawn and a click, `Q` or `E` sends no command (`docs/agents/how-we-work.md`: test every input that reaches a rule); with Follow on, a click over space and `Q` still reach the followed sprite.
- **The simulation is untouched:** its tests and golden files pass unchanged.
- **Played** in Windows Terminal at 140×40 and at the smallest size the game allows.

The ship's own walls come later: its prow is iron, and walls between sectors are hull (#181). Until then, every wall of the terrarium faces space.
