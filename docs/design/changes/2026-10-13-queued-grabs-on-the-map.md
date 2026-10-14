# A queued pick-up or put-down shows on the map

The owner, trying eggs as items on 2026-10-13 ([#203](https://github.com/Keazra/terra-sprites/issues/203)): "When you left click, it shows it as being picked up, but the egg remains where it is. When you drop it, nothing happens." It happens while the game is paused: a click waits for time to move, and only the Cursor's marks followed it. Fixed in [PR #204](https://github.com/Keazra/terra-sprites/pull/204).

## M1 "A Sprite Lives"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **The map follows the Grab-mode commands waiting for the next tick,** as the Cursor's marks do: an item a queued `PickUp` takes is gone from its tile, and one a queued `PutDown` or `Throw` lets go of shows on the tile it's put down on. The status line's word on the Cursor's tile, the leash and aim line's dots, and the next click follow them too: a click acts on an item where the map shows it. If the world refuses one, the map goes back to the world's word. | #203 | §6.5 |
