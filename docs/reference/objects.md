# `objects.ron`

Every kind of plant, item and toy, and what it does (design §3.5). Each entry is an **object type**. A new kind of thing that fits the vocabulary below needs no new code: it's one entry here, plus how it looks in each theme.

## Fields

| Field | Meaning |
|---|---|
| `id` | Permanent, never reused. The built-in types are 1–4, and 100–102 for the pseudo types. |
| `name` | What rules, presets and themes call it. |
| `plural` | How the screen says the kind in general: `"berry bushes"`. Left out for a thing you don't count, which reads with "is" ("water is good for thirst"). |
| `category` | What sprites perceive it as: a name from [`categories.ron`](categories.md). |
| `tags` | `[Solid, Fixture]` for a thing nothing can walk through or move, like a bush; left out for an item, like a berry or a ball, which sprites can stand on and the Cursor can carry. Only both or neither. Then any tags from [`tags.ron`](tags.md), such as `Thorny`. |
| `size` | `Small`, `Medium` or `Large`. Required on every type with objects. |
| `hardness` | From 0 to 1, given with `size`. A rolling item bounces off something bigger, knocks on something its size, and crushes something smaller and softer (design §3.5.4). |
| `counters` | Named whole numbers with maximums, from 0: `{"fruit": 6}`. |
| `stages` | `[(name: "seedling", ticks: (min, max), next: Stage("mature") or Expire)]`. Each stage lasts a random time between `min` and `max` ticks. A type with no stages is permanent. |
| `rules` | `[(trigger: ..., if: [conditions], do: [effects])]`: what it does by itself, in order. |
| `verbs` | `{Verb: [effects]}`: what happens when a sprite does that verb to it. The verbs are `Eat`, `Drink`, `Hit` and `Play`. |
| `visual` | `[(if: [conditions], state: "name")]`: the first that holds names the look the themes draw; if none does, the look is `"default"`. |
| `place` | `(label: "berry bush seedling", if: [conditions])` if the Cursor's Place menu offers it, under that label. `if` is optional: what the tile must be like, such as `[KeepsPathsOpen]`. |
| `pseudo` | `true` for `water`, `sprite` and `cursor`: things that aren't objects but can be tried on, so they have a verb table and nothing else. |

## The vocabulary

**Triggers:**
- `Every(n)`: every `n` ticks, staggered so objects of a type don't all fire on the same tick.
- `OnStageEnter("stage")`: on the tick the object enters that stage.
- `OnExpire`: when its last stage ends. An expiring object runs only its `OnExpire` rules, then is removed unless a rule replaced it.

**Conditions** (`cmp` is `Lt`, `Le`, `Eq`, `Ne`, `Ge` or `Gt`):
- `InStage("stage")`, `Counter("name", cmp, n)`.
- `Chance(p)`: true with chance `p`, from 0 to 1. It draws from the world's random numbers.
- **Location conditions,** about where the object is: `Fertility(cmp, f)`, the ground's fertility ([terrain](terrain.md)); `DensityBelow("type", radius, max)`, fewer than `max` objects of that type within `radius` tiles in any direction, counting itself; and `KeepsPathsOpen`, true where a solid object wouldn't wall off the tiles around it. Ask `KeepsPathsOpen` before making anything solid. An object the Cursor is holding is nowhere, so its location conditions are false.

**Effects:**

| Effect | What it does |
|---|---|
| `AddCounter("name", n)` | Adds `n`, which may be negative, staying within 0 and the maximum. |
| `SpawnNearby("type", radius)` | Makes a new object of that type on a free tile within `radius`, chosen at random. Nothing if there's no room. |
| `SpreadTo("type", radius, [conditions])` | Picks one tile at random within `radius` and makes the new object there if it fits and the conditions, judged at that tile, hold. |
| `ReplaceWith("type")` | Becomes a new object of that type on the same tile, if that type can stand there. |
| `DestroySelf` | Removes the object. |
| `RequireCounter("name", n)` | **Verbs only.** If the counter is below `n`, the verb fails here: nothing after it runs, and the sprite feels the try was fruitless. |
| `Inject(Actor or Target, "chemical", amount)` | **Verbs only.** Adds to a physical chemical ([chemicals](chemicals.md)) in the sprite doing the verb (`Actor`) or, on the `sprite` pseudo type, the sprite it's done to (`Target`). Injury it adds is blamed on this type if it kills. |
| `Signal(Actor or Target, "locus")` | **Verbs only.** Sends a pulse ([loci](loci.md)) that the sprite feels next tick. |
| `Push(tiles)` | **Verbs only, on items.** Sets the item rolling up to that many tiles away from the sprite. |

## How rules run

- **Conditions are judged left to right, and stop at the first false one.** So in `[Counter("fruit", Ge, 6), Chance(0.2)]` the chance is only drawn when the counter is full.
- **Put location conditions before `Chance`.** Then a chance is drawn only where the rest holds, rather than on every tile tried. The built-in berry sprouts with `[Fertility(Ge, 0.5), DensityBelow("berry_bush", 4, 3), KeepsPathsOpen, Chance(0.1)]`.
- **Verb-only effects need a sprite.** `RequireCounter`, `Inject`, `Signal` and `Push` act on the sprite doing the verb, so they can only go in `verbs`, never in `rules`.
- Effects run in order, and `DestroySelf`, or a `ReplaceWith` that works, ends the object's turn.
- Each tick, objects take their turns in order of ID: first the stage clock moves on, then the rules run in the order listed. An object made during the tick first runs its rules next tick.
- **A verb with no entry does nothing.** A sprite can try any verb on anything; eating a ball is a fruitless try, which teaches it something. That's why the built-in ball has no `Eat`.

## What's an error

Loading stops, naming the type and the problem, for:
- an unknown name (type, category, tag, chemical, locus, stage or counter), or an `id` or `name` used twice;
- a verb-only effect in a rule, an `Inject` of a chemical that isn't physical, or a `Signal` of a locus that isn't a pulse;
- `Chance` in `visual` or `place`, since drawing and placing never use random numbers;
- a stage whose `min` is below 1 or above its `max`, a counter maximum of 0, `Every(0)`, or a chance outside 0 to 1;
- tags other than both `Solid` and `Fixture` or neither, or a `Push` on anything but an item;
- a pseudo type with anything but verbs (and `size`, `hardness`), or with a verb that does anything but `Inject` and `Signal`;
- a verb table for anything but `Eat`, `Drink`, `Hit` and `Play`.

## Working example

A mushroom: a small item that sprites perceive as fruit. It grows from a button into a cap, can be eaten or kicked a tile, and when it dies it sometimes drops a new one nearby on fertile, uncrowded ground. Add the entry to the list in `objects.ron`, name it in a preset's `objects` ([presets](presets.md)) to have the world start with some, and give it a look in both `themes/` files, or it draws as a white `?`.

<!-- example: objects -->
```ron
(id: 5, name: "mushroom", plural: "mushrooms", category: "fruit", size: Small, hardness: 0.1,
 stages: [
     (name: "button", ticks: (500, 800),   next: Stage("cap")),
     (name: "cap",    ticks: (2000, 3000), next: Expire),
 ],
 rules: [
     // Location conditions first, then the chance.
     (trigger: OnExpire, if: [Fertility(Ge, 0.5), DensityBelow("mushroom", 3, 2), Chance(0.3)],
      do: [SpawnNearby("mushroom", 2)]),
 ],
 verbs: {
     Eat:  [Inject(Actor, "food", 0.2), Signal(Actor, "ate"), DestroySelf],
     Play: [Push(1), Signal(Actor, "played")],
 },
 visual: [(if: [InStage("button")], state: "button")],
 place: (label: "mushroom"),
),
```

The built-in types in `data/objects.ron` are working examples of everything else: the berry bush's fruit counter, the thornbush's spreading and its `Thorny` tag, the ball's `Push`, and the pseudo types.
