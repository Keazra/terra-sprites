# Building the view past the wall

What building Slice 31 ([#184](https://github.com/Keazra/terra-sprites/issues/184)) settled that the ship's design pass ([2026-10-12-terra.md](2026-10-12-terra.md)) left open: what `F` and the wheel do over space, and the leash and the Cursor's arms at the glass. They're the builder's defaults, for the owner to try in the build, from the slice's PR.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **`F` or a middle click over space reaches no tile:** following, the Cursor stops; otherwise it follows the selected sprite, if any, not the sprite on the Cursor's last tile. The wheel over space cycles the modes. | Slice 31's PR, to keep §11.4's "does nothing to that last tile" | §11.4 |
| 2 | **Out of sight, the Cursor draws no leash,** and its arrows and marks show only over the map's tiles, never over the glass. | Slice 31's PR | §11.4 |
