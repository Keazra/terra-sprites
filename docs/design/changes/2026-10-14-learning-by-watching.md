# Learning by watching

Slice 28 ([#146](https://github.com/Keazra/terra-sprites/issues/146), from [#66](https://github.com/Keazra/terra-sprites/issues/66)): a sprite takes on a share of what the sprite its eye is on knows about what it uses, and of what happens to it; in the starter genes only the young do, watching the grown-ups, their parents most. Settled with the owner in two rounds of questions on 2026-10-14: nine in round 1, and the bar in round 2 once `main` was measured on the new arena.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A sprite watches the one its eye is on,** or one using the kind of thing its eye is on, while it sees it; a sprite heard calling isn't watched. It takes on what the other's moment at step 4 was about, after everyone has learned from what happened to itself. | Q1 | §18.1 |
| 2 | **`watch_rate`, a new brain parameter** (ID 44, 0 to 1, default 0): the share of the way a watcher's view moves towards the other's. `learning_rate_mod` scales it; `plasticity` doesn't. A pack from before it gets the built-in range. | Q3, Q7 | §18.1 |
| 3 | **An attention instinct can be about grown-ups,** `Grown` (ID 6): a sprite grown up, adult or old. | Q1 | §18.1 |
| 4 | **The starter's young watch the grown-ups, their parents most:** `watch_rate` .5, and `always` drawing the eye to grown-ups and as much again to its parents, all until adult. Grown sprites behave as before. | Q1, Q3, Q7 | §18.1 |
| 5 | **A moment is about** the thing tried at the latest step 6, the try the Cursor rewarded or corrected, the sprite that hit it, the Cursor when it learned to fear it, and a place where it was eased. | Q2, Q4, Q5 | §18.2 |
| 6 | **About a thing, the watcher takes on what the other knows of it:** its worth for each need, good, bad, and the habit of the verb tried, kept as a taught habit; it then knows the thing's object type. | Q2, Q3 | §18.2 |
| 7 | **About a sprite or the Cursor, only fear** and the habit of the verb tried on it: whom to like it finds out for itself. So a hit, a pet or a shock to the one watched passes on, settling [#106](https://github.com/Keazra/terra-sprites/issues/106) for whoever learns by watching. | Q2, Q4 | §18.2 |
| 8 | **A place where the one watched was eased is remembered** as if the watcher had been eased there. | Q5 | §7.6, §18.2 |
| 9 | **Only ever up to the other's view:** a value moves only when the other's is further from 0 the same way, never past it, back towards 0 or across it. Views are read before anyone takes on anything, so nothing passes through two sprites in a tick. A moment about the watcher itself teaches it nothing. | Q2 | §18.2 |
| 10 | **Nothing about mating is learned by watching,** for every genome and pack: a try at Mate or Court is never a moment, and worth for the breeding drive never moves by watching. | §5 | §18.3 |
| 11 | **The log says** `Kit #21 learned from watching Pip #12: water is good for thirst` for a lesson watching tipped past half; the Genes tab reads an attention gene about grown-ups as `grown-ups (sprite)`. | Q8 | §18.4 |
| 12 | **Nothing new for following a parent.** | Q6 | §18.5 |
| 13 | **How we know it worked:** in the new arena, the young with a parent learn water is good for thirst in at most half the orphans' ticks (`main`: 3,523 and 3,576); safe with random genomes; A1–A4, Slice 32's family numbers, 200,000 ticks of the default world and A5 no worse. Food is for information: the grown-ups barely know it. | Q9; round 2, Q10 | §18.6 |
| 14 | **The lab can give each sprite a newborn** at a tick (`young: (at: N)`), and run a control of orphans (`control: Orphans`), with the young's lessons and the median ticks to each in the report. | Q9 | §18.6 |
| 15 | **Later:** copying the act itself, grown sprites watching each other, going back to where a young one last saw its parent, and parents teaching on purpose (#67). | Q2, Q6, Q7 | §18.7 |
