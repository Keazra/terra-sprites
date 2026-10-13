# Building the view past the wall

What building Slice 31 ([#184](https://github.com/Keazra/terra-sprites/issues/184)) settled that the ship's design pass ([2026-10-12-terra.md](2026-10-12-terra.md)) left open: what `F` and the wheel do over space, the leash and the Cursor's arms at the glass, the Cursor once the pointer leaves the map view through space, and aiming over space. They're the builder's defaults, for the owner to try in the build, from the slice's PR.

## M2 "Generations"

| # | Change | Source | Sections |
|---|---|---|---|
| 1 | **`F` or a middle click over space reaches no tile:** following, the Cursor stops; otherwise it follows the selected sprite, if any, not the sprite on the Cursor's last tile. The wheel over space cycles the modes. | #184, to keep §11.4's "rather than act on that last tile" | §11.4 |
| 2 | **Out of sight, the Cursor draws no leash,** and its arrows and marks show only over the map's tiles, never over the glass. | #184 | §11.4 |
| 3 | **Off the map view, the Cursor shows again on its last tile,** even when the pointer left through space, and `Q` and `E` reach it. | #184, from the PR's review | §11.4 |
| 4 | **Aiming, the pointer pulls on past the wall,** counted in tiles as if the map went on. The Cursor isn't out of reach: a click over space reaches what it aims, as `Q` and `E` do, so a right click there sends the aim. An aim that ends over space leaves the Cursor where it was, out of reach. | #184, from the PR's review | §11.4 |
