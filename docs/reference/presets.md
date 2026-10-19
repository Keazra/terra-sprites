# World config and presets

A **world config** says how big a generated world is and what it starts with (design §3.9). The built-in one is `data/presets/default.ron`. A **preset** is a file in the same format: `--preset <file>` starts a world from it instead, with no rebuilding, and `--seed <n>` still picks which world of that kind you get. `data/presets/gentle.ron` is the watery world from before Slice 29: `--preset data/presets/gentle.ron`.

| Field | Meaning |
|---|---|
| `width`, `height` | The map's size in tiles, each from 32 to 1024. |
| `sprites` | The first population, from 20 to 100, whatever the map's size. Left out, the world has no sprites. |
| `objects` | A map from object type names ([objects](objects.md)) to how many of each go in every `per_tiles` tiles, so a bigger map gets more. A type not named gets none, and pseudo types (`water`, `sprite`, `cursor`) can't be named. |
| `per_tiles` | The area `objects` counts are for, above 0. Needed whenever `objects` is given. |
| `most_sprites` | The most sprites the world holds, eggs included (M2 design §10), from 1 to 1,000 and at least `sprites`. In a full world no sprite is fertile. Left out, 100. |
| `water` | The share of the map's tiles under water, as a percentage, from 1 to 60 (M2 design §19.1). Left out, 32, the watery world of the game before Slice 29. The default world has 4. |
| `lake_size` | How big the lakes are, from 1 to 5: the map's hills and hollows are this many times as broad, so the same water lies in fewer, bigger lakes. Left out, 1. The default world has 3. |
| `start_near_water` | The first sprites start within this many tiles of drinkable water, as the crow flies, while any such tile is free; then anywhere. Left out, anywhere on the map. The default world has 10. |

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

## Presets on the title screen

The title screen's New world box lists `default` first, then each `.ron` file in the `presets` folder of the game's folder, by name (M2 design §8.3). On Windows that's `$env:APPDATA\terra-sprites\presets` in PowerShell; `?` in the game shows where the game's folder is. Put `small.ron` there and it shows as `small`. A preset that doesn't fit the data pack in use is refused when you press `Enter`, with why, and the title screen stays.
