# What the Cursor teaches lasts

A3 had passed or failed by luck since Milestone 1: one shock wore off for 9 sprites in 10, and it took three to last ([#187](https://github.com/Keazra/terra-sprites/issues/187)). On 2026-10-12 the owner chose to fix it before breeding merged. This is the fix recommended to them, "Lasting lessons": what the Cursor teaches is kept apart and lasts, in [PR #189](https://github.com/Keazra/terra-sprites/pull/189).

**Measured** over A3's seeds 1–30, with `cargo run --release -p terra-sim --example lab -- scenarios/a3-correct-training.ron --seeds 30`: trained sprites' median of hits in ticks 10,000–20,000 was 23 on `main` at `3b246f3` and 0 with taught habits at `acb2809`, against 69 for the control both times. Every random set of 10 of those seeds passes, against 84 in 100 on `main`.

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **A Reward or Correct for what a sprite just tried teaches its taught habit,** kept apart from the habit it learns by itself, at `taught_rate`, three times `habit_rate`. It fades by `taught_fade`, by half in about 35,000 ticks (8 hours at 1×). The decision, the Brain tab and the lessons count the two together, within ±1. A pet or shock that finds no try within reach still teaches the sprite's own habits along the trace, as before, and a sprite nobody trains learns exactly as before. Saves from before load with no taught habits; a replay recorded before parts from its recording at its start, as one does between builds that share a version number. | #187 | §5.5, §5.6, §5.8.1, §7.2 |
| 2 | **Two new brain parameters,** `taught_rate` (ID 39, 0–1, default 0.9) and `taught_fade` (ID 40, 0–0.01, default 0.00002), so how trainable a sprite is can differ between sprites and evolve once genomes carry them. The starter genome doesn't yet, as for most brain parameters: a new gene would change every sprite's spawn variation. A data pack from before them gets the built-in range and default. Appendix A also lists 37 and 38, which came with M2 (M2 design §7.2, §9.5). | #187 | Appendix A, Appendix B |
