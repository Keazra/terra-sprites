# What the Cursor teaches lasts

A3 had passed or failed by luck since Milestone 1: one shock wore off for 9 sprites in 10, and it took three to last ([#187](https://github.com/Keazra/terra-sprites/issues/187)). The owner chose to fix it before breeding merged, and chose "Lasting lessons" on 2026-10-12.

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **What the Cursor teaches a habit is kept apart, as a taught habit,** from the habit the sprite learns by itself. A pet or shock goes to the taught habit of the sprite's latest attempt at `taught_rate`, three times `habit_rate`, and it fades by `taught_fade`, by half in about 35,000 ticks (8 hours at 1×). The decision and the Brain tab count the two together, within ±1. A sprite nobody trains behaves exactly as before. | #187; owner, 2026-10-12 | §5.5, §5.6, §7.2 |
| 2 | **Two new brain parameters,** `taught_rate` (ID 39, 0–1, default 0.9) and `taught_fade` (ID 40, 0–0.01, default 0.00002), so how trainable a sprite is can differ between sprites and evolve. A pack or save from before them gets the built-in range and default. | #187 | Appendix A, Appendix B |
