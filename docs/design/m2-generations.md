# Terra Sprites — M2 "Generations" design

- **Status:** Draft: the plan, with what each slice settles added as it's settled.
- **Date:** 2026-10-08
- **Revisions:** numbered up to v5 (2026-10-07), kept in [`archive/`](archive/). Since then this file is edited in place, with each change recorded in [`changes/`](changes/).
- **Builds on:** [M1 design](m1-a-sprite-lives.md), which stays the rulebook for everything M2 doesn't change
- **Covers:** what milestone 2 contains, its slices and their order, the time scale of a sprite's life, the direction for ageing, what "M2 is done" means, memory of places (Slice 20), the title screen (Slice 21), and where places sit on the Brain tab

---

## Changes

Each change since v5 is a file in [`changes/`](changes/), named by the date it was settled: what changed, where the decision came from, and which sections it touches. v1 to v5, each with its own table at the top, are in [`archive/`](archive/). A rule marked "(v5)" came in that revision; one added since is marked with its issue, "(#N)", or its date.

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

Each slice is a GitHub issue and its own PR, worked as in `docs/agents/how-we-work.md`: its open questions are settled first, in plain words, and recorded in this document, in a file in [`changes/`](changes/) and in `CONTEXT.md`. Each issue lists its slice's questions. Numbering carries on from M1's last, slice 19, so every slice number stays unique.

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

- **Up to `remembered_places.held` places, 8 in the built-in physiology, and at most `per_kind` of one object type, 3.** Places of the same object type within `remembered_places.merge` tiles of each other (5, counted as tiles of Chebyshev distance) are one place, so a pond is one place, not fifty water tiles: relief at a tile of it moves the place there, and takes in every other place of its kind within 5 tiles of that tile, so no two places of a kind are ever within 5 tiles of each other.
- **Why a limit a kind.** A big lake's shores are many places 5 tiles apart, and a sprite drinks far more often than it eats. Without the limit, water filled about 7 of a sprite's 8 places by tick 20,000 in a default world (seed 3), and pushed out the bushes it had eaten from: the owner saw sprites remember only water. With it, 26 of 30 sprites remember a bush by tick 10,000, against 18 of 30 by tick 20,000 without it.
- **Relief there remembers a place fully,** its **recall**, how well it's remembered, at 1, whether it's new or a visit tops it up.
- **It fades each tick** by the `place_fade` brain parameter: its recall is multiplied by 1 − `place_fade`. The default, 0.0002, halves it in about 3,500 ticks, about 45 minutes at 1×, as worth for a need fades (M1 design §5.6). Its range is 0 to 0.01, so evolution can tune it; a long memory has a natural cost in trips to places that have changed.
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
- Once Slice 27 builds Lab mode, the choice between the full game and Lab mode goes in this box.

### 8.4 Leaving a world

- **`Esc` from Select** (M1 design §6.5, §6.6) asks "Quit? y quit the game  t title screen  any other key stays". `y` quits, as before; `t` goes back to the title screen, whose scene plays again on a new world. Any other key, `Esc` included, cancels.
- **Both save first,** as quitting does (M1 design §6.7): an autosave, unless the world hasn't run since it was last saved. The session log is written too. If either can't be, the title screen says why on its status line, and quitting says why on the terminal once the game has closed.
- `Ctrl+C` still quits at once.

### 8.5 Flags

- **`--seed`, `--preset` and `--replay` make or load a world,** so they skip the title screen and go straight in, as before. Their world can still leave for the title screen with `t`.
- **`--data`, `--ascii` and `--theme`** change what the game's worlds are made of or how they're drawn, not which world it opens on, so the title screen still shows, drawn with them, and a new world made from it uses `--data`'s pack.
