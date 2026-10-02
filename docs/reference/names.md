# `names.ron`

The syllables random names are made of (design §6.5). When the player names a sprite with `r`, the game offers a random name, and `Tab` offers another: a first syllable, sometimes a middle one, and a last, with a capital at the start.

| Field | Meaning |
|---|---|
| `first`, `middle`, `last` | Lists of syllables. None may be empty, and each syllable is letters only, from the CP437 character set (so `é` is fine and `ł` isn't). |
| `middle_chance` | From 0 to 1: how often a name gets a middle syllable. |

A name is at most 16 letters, so the longest first, middle and last syllables together must fit.

## Working example

Short, soft names: "Lumi", "Nalo", "Sorabi".

<!-- example: names -->
```ron
(
    first: ["Lu", "Na", "So", "Ki", "Ama"],
    middle: ["ra", "li"],
    last: ["mi", "lo", "bi", "ne"],
    middle_chance: 0.25,
)
```
