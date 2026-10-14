# `terrain.ron`

What each kind of ground is like (design §3.1). The kinds are fixed: `grass`, `dirt`, `sand`, `shallow_water`, `deep_water` and `rock`. Each must be listed, and the file can't add new ones. What the file sets is how each behaves.

| Field | Meaning |
|---|---|
| `walkable` | Whether sprites can walk on it. |
| `step_cost` | What a step onto it costs, above 0. A grass step is 10, and a diagonal step costs 14/10 of a straight one. Sprites take cheaper ways round dearer ground. |
| `fertility` | From 0 to 1: how well plants grow there. Rules ask for it with `Fertility(cmp, f)` ([objects](objects.md)). |
| `drinkable` | Whether a sprite can drink from it. |
| `allows_fixtures` | Whether a fixture, such as a bush, may stand there. |
| `sound` | Optional, on any kind, walkable or not: how much a call weakens crossing a tile of it, above 0, as `step_cost` is for walking (M2 design §15.1). A call is heard wherever its way there costs at most `calls.reach` in [`physiology.ron`](physiology.md), and a diagonal step costs 14/10 again, so in the built-in pack, with 10 for every kind but rock, a call carries about 30 tiles. A kind without it stops calls, as rock does there, and a call can't cut a corner past it. |

- **A walkable kind gives all five of the first fields.** An unwalkable one gives only `walkable: false`, and maybe `sound`.
- **`dirt` and `shallow_water` must be walkable.** World generation carves paths through walls as one or the other, so every part of the map can be reached (design §3.2).

## Working example

Muddy ground: dirt is slow going, and sand grows a little.

<!-- example: terrain -->
```ron
{
    grass:         (walkable: true, step_cost: 10, fertility: 1.0, drinkable: false, allows_fixtures: true),
    dirt:          (walkable: true, step_cost: 20, fertility: 0.5, drinkable: false, allows_fixtures: true),
    sand:          (walkable: true, step_cost: 15, fertility: 0.2, drinkable: false, allows_fixtures: true),
    shallow_water: (walkable: true, step_cost: 25, fertility: 0.0, drinkable: true,  allows_fixtures: false),
    deep_water:    (walkable: false),
    rock:          (walkable: false),
}
```
