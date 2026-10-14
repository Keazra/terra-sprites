# Courtship as built

Slice 32a ([#186](https://github.com/Keazra/terra-sprites/issues/186)): what building calls, getting to know each other, courtship, partners, kin and the lone-egg rule settled that [2026-10-13's design](2026-10-13-courtship-and-families.md) left open or got wrong. Each is a build call, made on 2026-10-14 and reported in the slice's PR.

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **An instinct's link is held within −1 to 1,** as the code always has, which the design didn't say: a weight past 1 works as 1. Slice 32a's first courtship genes had weights of 2 and 3, which worked as 1 and kept Court from ever winning. | Measuring 32a | §5.7 |
| 2 | **Sprites in general leave out those known only by sight,** who are judged by the summary as strangers are (M2 §15.2, which made seeing a sprite enough to remember it). | Build | §5.6 |

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A call is a listening bout:** the sprite calls on the first tick and stands listening for `calls.listen` ticks (10), as a short rest, so a call can be seen on the map and the doing line, and a sprite doesn't call every tick. | Build | §15.1, §15.8 |
| 2 | **A heard caller is drawn as at the edge of sight:** it's offered in the sprite category, the eye drawn to it by what the sprite has learned of it as much as the call is still fresh, and not by nearness. So a sprite in sight draws the eye first, as a remembered place does. | Build | §15.1 |
| 3 | **Body language reads `target_condition`,** from a new body sensor, `condition`, where the design said `target_health`: "health" is already a gene's soundness (§12.4). It's the same measure: energy × hydration × (1 − injury) × the genes' average health. | Build | §15.3 |
| 4 | **New brain inputs** the starter genes needed: `target_receptivity`, so a male stops dancing for a female already receptive; `paired`, a body sensor that's 1 while a sprite has a partner, so partners court only each other; and `male_near`, which breeding's loci had but the brain couldn't read, so a female calls only with no male about. | Build | §15.1, §15.3, §15.4 |
| 5 | **Seeing a sprite keeps it remembered:** each tick a sprite sees another, its familiarity grows from at least `forget_below`, after the tick's fading, so one in sight is never forgotten, and a stranger first seen is remembered. | Build | §15.2 |
| 6 | **Pairing again frees the old partner:** a sprite that has an egg with another than its partner pairs with the new one, and its old partner is freed with a `parted` line in the log. | Build | §15.4 |
| 7 | **A partner who died is named until the sprite pairs again,** as the Body tab's "partner Kit #20 · died", though the sprite is free. | Build | §15.4, §15.8 |
| 8 | **The ASCII dance is `^`,** not `~`, which is shallow water's glyph there. | Build | §15.8 |
| 9 | **The doing line words a dance `Courting Bo #7` while walking, `Danced for Bo #7` once done, and `Couldn't dance for Bo #7`;** a call is `Calling` while it listens and `Called` once done. | Build | §15.8 |
| 10 | **Flaw and copy-and-change rates scale with the starter's genes,** which grew from 67 to 94 with courtship: `flaws.arise` 0.005 (from 0.007), `flaws.founders` 0.043 (from 0.06) and `copy_and_change` 0.00039 each (from 0.00052), so an egg still gets about one new flaw in two, the first sprites about three, and one hatchling in ten a doubled, lost or changed gene. | Build, to keep §12.3 and §13.1's targets | §12.3, §13.1 |
| 11 | **Fertile from 0.4 fed** (was 0.5): with courtship, at 0.5 a fertile male and female were seldom within sight of each other at once, males were few, and about half the eggs were laid alone. At 0.4 the world fills, and lone eggs are rare (§15.9). | Measuring 32a | §10.2 |
| 12 | **Longing builds over about an hour,** as the body's rule allows (`gain` 0.0002, was 0.001): when it built in minutes, a lone female tried and failed to breed alone over and over, and never called. | Measuring 32a | §15.7 |
| 13 | **Dancing back is what makes her receptive:** being danced for raises receptivity by 0.04 and dancing back by 0.2 (were 0.08 and 0.12), fading over 600 ticks (was 300). A dance leaves a spare hormone, `h1`, so a male waits a little before dancing again, and being danced for leaves `h2`, so a sprite dances back for a minute or so after, not only on the next tick, for the familiar sprite it's looking at. When his dances alone could make her receptive, many pairs' eggs came with no dance back. | Measuring 32a | §15.3 |
| 14 | **A sprite known only by sight is judged as sprites in general,** as a stranger was before, and isn't counted in that summary: seeing many sprites doesn't water down what it learned of the few it knows. | Build | §15.2 |
| 15 | **A trip to a call forgets the call** when there's no way there, or on arriving with the caller still out of sight; it heads anew if the caller calls again from elsewhere. The detail view marks it `· by ear`. | Build | §15.1, §15.8 |
| 16 | **More starter genes than §15.3 named:** Mate held back while she isn't receptive and with kin; an aroused sprite plays with a stranger of the other sex, to get to know it, and goes to its partner. | Build | §15.3 |
| 17 | **The family tree names a sprite's partner over their young,** `partner Sprite #27476` in place of `with Sprite #27476`, where §17.8 said `· partner`: after the name, it wouldn't fit over a single young's 24 columns. The Body tab names it as `partner Kit #20`, so the two read alike. | Slice 25's note on #186 (whichever merged second adds it) | §17.2, §17.8 |
