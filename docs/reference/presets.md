# World config and presets

A **world config** says how big a generated world is and what it starts with (design §3.9). The built-in one is `data/presets/default.ron`. A **preset** is a file in the same format: `--preset <file>` starts a world from it instead, with no rebuilding, and `--seed <n>` still picks which world of that kind you get.

| Field | Meaning |
|---|---|
| `width`, `height` | The map's size in tiles, each from 32 to 1024. |
| `sprites` | The first population, from 20 to 100, whatever the map's size. Left out, the world has no sprites. |
| `objects` | A map from object type names ([objects](objects.md)) to how many of each go in every `per_tiles` tiles, so a bigger map gets more. A type not named gets none, and pseudo types (`water`, `sprite`, `cursor`) can't be named. |
| `per_tiles` | The area `objects` counts are for, above 0. Needed whenever `objects` is given. |

Plants start partway through their lives, so a new world doesn't all ripen at once (design §3.2).

## Working example

A small, crowded world with no thorns.

<!-- example: preset -->
```ron
(
    width: 96,
    height: 64,
    sprites: 20,
    objects: {
        "berry_bush": 120,
        "ball": 10,
    },
    per_tiles: 15360,
)
```

Save it as `small.ron` and run `cargo run --release -- --preset small.ron --seed 7`.
