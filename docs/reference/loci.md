# `loci.ron`

The values genes and objects read and write, other than chemical levels (design §4.2). Chemical levels are loci too, written `Chem("name")`, so they aren't listed here.

| Field | Meaning |
|---|---|
| `id` | Permanent, never reused. |
| `name` | What objects, genomes and `brain_io.ron` call it. |
| `kind` | `BodySensor`, `Pulse` or `ReceptorTarget`. |

**The kinds:**
- **`BodySensor`:** physiology fills it in every tick. The game needs `always` (always 1), `age` (as a fraction of the age it grows old at, capped at 1), `nearby_sprites`, `moving` and `resting`, and computes each. Breeding (M2 design §10) fills in `fertile` (1 while it's fertile), `female` (1 if it's female), `female_near` (1 while it sees a female adult within its sense radius) and `male_near` (the same, for a male adult). Courtship (M2 design §15) fills in `partner_near` (1 while it sees its partner, or has just heard it call), `paired` (1 while it has a partner) and `condition` (how healthy it looks, which a mate may choose by: its energy times its hydration times how unhurt it is, times how whole its genes are). Families (M2 design §15.6) fill in, each 1 while it's so: `egg_away`, an egg of its own is out of its sight; `egg_alone`, when the sprite last saw one of those, it saw no other parent by it (one in its sight that saw the egg), or it has never seen it; `young_away`, a young one of its own is out of sight; `parent_away`, it's young, has a living parent, and sees none; and `partner_away`, its partner is out of sight. A pack with courtship, all five and the pulse `found_family` has families; without them, these stay 0. A pack with families may also have `parent_showing` (M2 design §18.1): 1 while the sprite is young and a parent in its sight tried something at its latest step that it has learned a lesson about and the sprite hasn't, never a try at mating or courting; without it, it stays 0.
- **`Pulse`:** an event that lasts one tick, such as `ate` or `was_hit`. Object verbs and tags `Signal` pulses, and a sprite feels one on the tick after it's sent. A pulse sent to a sprite by another (a `Signal(Target, ...)`) remembers who sent it, which is how a sprite knows who hit it. The game itself sends `cornered`, `was_hit`, `fruitless`, `petted` and `shocked`, so those must be here. It also sends `bred` to both parents as an egg is laid. A pack without breeding's three sensors and `bred` (or the [chemical](chemicals.md) `feminiser`) has no breeding. In a pack with courtship's three sensors and pulses, and the chemical `receptivity`, the game sends `heard` to a sprite that hears a call, from the caller; `courted` to one another dances for, from the dancer; `danced` to the dancer; and `lost_partner` to a sprite whose partner dies. Without them a pack breeds, but has no calls or courtship (M2 design §15). In a pack with families, it sends `found_family` to a sprite that finds family it was missing: one of the family senses above goes off with the one it was about back in sight, not hatched, dead, grown up or parted.
- **`ReceptorTarget`:** written by receptor genes to adjust the brain: `learning_rate_mod`, `exploration_mod` and `curiosity_mod`, all required. Each rests at 1 when no receptor writes it, within the range [physiology](physiology.md) gives it.

Which of these the brain feels is up to [`brain_io.ron`](brain-io.md).

## Working example

A new pulse, `splashed`, added to the list. A verb can then send it, such as `Play: [Signal(Actor, "splashed")]` on water, and genes can react to it.

<!-- example: loci -->
```ron
(id: 49, name: "splashed", kind: Pulse),
```

The built-in list in `data/loci.ron` is the full working example.
