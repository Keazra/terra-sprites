# `loci.ron`

The values genes and objects read and write, other than chemical levels (design §4.2). Chemical levels are loci too, written `Chem("name")`, so they aren't listed here.

| Field | Meaning |
|---|---|
| `id` | Permanent, never reused. |
| `name` | What objects, genomes and `brain_io.ron` call it. |
| `kind` | `BodySensor`, `Pulse` or `ReceptorTarget`. |

**The kinds:**
- **`BodySensor`:** physiology fills it in every tick. The game needs `always` (always 1), `age` (as a fraction of lifespan), `nearby_sprites`, `moving` and `resting`, and computes each.
- **`Pulse`:** an event that lasts one tick, such as `ate` or `was_hit`. Object verbs and tags `Signal` pulses, and a sprite feels one on the tick after it's sent. A pulse sent to a sprite by another (a `Signal(Target, ...)`) remembers who sent it, which is how a sprite knows who hit it. The game itself sends `cornered`, `was_hit`, `fruitless`, `petted` and `shocked`, so those must be here.
- **`ReceptorTarget`:** written by receptor genes to adjust the brain: `learning_rate_mod`, `exploration_mod` and `curiosity_mod`, all required. Each rests at 1 when no receptor writes it, within the range [physiology](physiology.md) gives it.

Which of these the brain feels is up to [`brain_io.ron`](brain-io.md).

## Working example

A new pulse, `splashed`, added to the list. A verb can then send it, such as `Play: [Signal(Actor, "splashed")]` on water, and genes can react to it.

<!-- example: loci -->
```ron
(id: 43, name: "splashed", kind: Pulse),
```

The built-in list in `data/loci.ron` is the full working example.
