# The view past the wall

Slice 31 ([#184](https://github.com/Keazra/terra-sprites/issues/184), from [#180](https://github.com/Keazra/terra-sprites/issues/180)): the view goes past the terrarium's wall into space, with stars going by. Settled with the owner on 2026-10-12, in rounds 1 and 2 of the ship's design pass, which designed #180 together with the ship ([#181](https://github.com/Keazra/terra-sprites/issues/181)). The same PR adds the ship's words settled so far to `CONTEXT.md`.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A new slice, 31, alongside M2.** It touches only the front end, so it can be built beside any slice; the ship is a milestone of its own after Body & verbs. | Owner, round 1, Q7 | §1.3, §4 |
| 2 | **The viewport scrolls a quarter of the map view past the wall** on every side; Track and `Tab` stop there. | Owner, round 1, Q3 | §11.1 |
| 3 | **A map smaller than the map view is drawn in its middle,** with space around, instead of in a map view shrunk to fit. | This write-up, to go with 2 | §11.1 |
| 4 | **The wall is drawn as glass one tile outside the map,** and the map view's border is single-lined all round. | Owner, round 1, Q3, Q4 | §11.2 |
| 5 | **Stars in three depths drift right to left** with the game's speed, capped at 4×, and shift a little as the view scrolls. They're scenery: never in the world, its saves or its replays. | Owner, round 1, Q5 | §11.3 |
| 6 | **The pointer over glass or space hides the Cursor,** which stays on its last tile; a click does nothing and the status line says "space". | Owner, round 2, Q11 | §11.4 |
| 7 | **Themes gain the glass and the three depths of star.** | Owner, round 2, Q11 | §11.5 |

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **Pointers to M2 §11** where the map view stops at the wall, shrinks to fit a small map, double-lines its border, and where the Cursor stays when the pointer leaves the map. | As M2 rows 2–4 and 6 | §6.1, §6.5 |
