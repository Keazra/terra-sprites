# Lab scenarios

A lab scenario is a headless test world: run it over several seeds and it reports what sprites did, without the screen (design §7.1). The scenarios in `scenarios/` are the acceptance tests and the main tuning tool. Run one with:

```bash
cargo run --release -p terra-sim --example lab -- scenarios/a1-thornbush.ron --seeds 10
```

| Field | Meaning |
|---|---|
| `world` | `Generated`, a world from the built-in default preset; or `Drawn(rows: [...], key: {...})`, a map drawn by hand. |
| `ticks` | How long each seed runs. |
| `windows` | `[(from, to)]`: the tick ranges to count in. Each runs from `from` up to, but not including, `to`, and must end by `ticks`. |
| `control` | Optional. `NoLearning` runs each seed a second time with learning switched off; `NoTrainer` a second time without the trainer. |
| `trainer` | Optional: a stand-in for the player, below. |

**Drawn maps.** Each row is a line of tiles in the ascii theme's ground glyphs: `.` grass, `,` dirt, `:` sand, `~` shallow water, `=` deep water, `#` rock. The `key` gives any other character a meaning, on grass: `Sprite` (a sprite from the starter genome) or `Object("type")` (an object at the start of its life). A thing that can't stand where it's drawn is an error.

**The trainer** answers a sprite's action as the player would with the Cursor. `trainer: (on: (Play, "ball"), give: Pet, delay: 0, reach_back: 3, until: 9900)`:
- `on`: the verb and object type it answers, here an applied Play on a ball.
- `give`: `Pet`, `Hug`, `Zap` or `Shock`, to the sprite that did it.
- `delay`: optional, how many ticks it waits first. With none, it answers on the very next tick.
- `reach_back`: optional, for a pet or hug only, how far back the feeling looks for what the sprite just tried. Left out, it's physiology's `touch_window`.
- `until`: it answers actions before this tick.

**The report** gives each seed's counts, then the median across seeds: for each window, the actions applied by verb and target (`eat thornbush 23`), deaths by cause, the Cursor's touches (`given: pet`), and lessons learned (`lesson: ball is good`). With a control, both runs are printed.

## Working example

A small meadow where a trainer pets a sprite each time it plays with a ball, compared with the same seeds untrained.

<!-- example: scenario -->
```ron
(
    world: Drawn(
        rows: [
            "############",
            "#..........#",
            "#..S...o...#",
            "#....~~....#",
            "#.B..~~..o.#",
            "#..........#",
            "############",
        ],
        key: {'S': Sprite, 'o': Object("ball"), 'B': Object("berry_bush")},
    ),
    ticks: 6000,
    trainer: (on: (Play, "ball"), give: Pet, until: 3000),
    control: NoTrainer,
    windows: [(0, 3000), (3000, 6000)],
)
```

The files in `scenarios/` are working examples too, each with a comment saying what it tests.
