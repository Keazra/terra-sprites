# A harsher world

Slice 29 ([#147](https://github.com/Keazra/terra-sprites/issues/147)), as settled with the owner in [#237](https://github.com/Keazra/terra-sprites/issues/237) (2026-10-17), [#224](https://github.com/Keazra/terra-sprites/issues/224) (2026-10-15 and 17) and [#225](https://github.com/Keazra/terra-sprites/issues/225) (2026-10-18 and 19), with what G5 counts from [#223](https://github.com/Keazra/terra-sprites/issues/223) (2026-10-15). Adults on the move a quarter of the time moved to [#318](https://github.com/Keazra/terra-sprites/issues/318).

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A new section, §19, A harsher world,** with what this slice builds and how it's measured. | #147 | §19 |
| 2 | **Less water, nearer the first sprites:** two world settings, `water` (the share of tiles under water) and `lake_size`, at 4% in lakes three times the size in the default world; the first sprites start within 10 tiles of drinkable water (`start_near_water`). The watery world before this slice stays as `data/presets/gentle.ron`. | #225 round 1 | §19.1 |
| 3 | **A sprite acts only when something moves it,** and otherwise stands, by a brain parameter, `act_threshold` (0.25 in the starter). The starter's wandering splits by life stage (0.15 young, 0.3 grown), standing builds boredom at 0.0005 a tick, and rests cost a fifth less than standing awake. A standing sprite's doing line says what it's watching. | #237 rules 3 and 6, #225 round 1 | §19.2 |
| 4 | **Hunger and thirst start below 0.7,** where they started below half. | #225 round 1 | §19.3 |
| 5 | **One gut of 0.5 for food and water:** each thing moves into the body at its own pace, food only as the body has room, water passing through; a bite or a drink that doesn't fit ends Too full and teaches nothing. Fullness is a hormone the starter makes from the gut and from a `too_full` pulse, leaning against eating and drinking alike at −2 and draining hunger. #224 called it a drive; a hormone doesn't colour a fed sprite. | #224, #225 round 1 | §19.4 |
| 6 | **A young sprite's thirst and dying are paced to its childhood,** × 4,500 ÷ the ticks its genes take to grow up. | #225 round 2 and its 220-hour check | §19.5 |
| 7 | **G5 and A4 count by life stage:** 10% to 30% of the unattended run's newborns grow up, in the middle seed; at least 60% of the young hatched in the viability run grow up, with no upper end in M2; adults dying of hunger or thirst are under 1 in 20 of grown-up deaths, old sprites' reported apart. The lab counts childhoods and deaths by life stage, and the long run (`--example childhood`) reports #224's bars too. | #223, #225 round 1 | §2, §4, §6, §19.6 |
| 8 | **Berry bushes put up seedlings** now and then, as a data rule. | #225 round 2 | §19.6 |
| 9 | **Needs genes, falling behind and careful copying** are named; careful copying is held in reserve, not built. | #237 rules 2, 4, 7 and 8 | §19.6 |
| 10 | **What's taken on by watching, of a thing's badness and habits, is kept apart,** settles either way towards the other's first-hand view, and is never passed on. | #225 round 2 | §18.2, §19.7 |

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A4 is M2's G5 too:** the unattended run, the guided young and the grown-up deaths by life stage replace survival at tick 10,000 and the deaths after it. | #223 | §7.4, §7.6 |
| 2 | **A3's arena is walled, 7 by 5 inside:** sprites that act only when moved seldom met in the old open arena, and its controls hit no one. | Slice 29 | §7.3 |
