# `chemicals.ron`

The chemicals in a sprite's body (design §4.1, Appendix A). Each has a level from 0 to 1.

| Field | Meaning |
|---|---|
| `id` | Permanent, never reused. The built-in pack groups them: physical from 1, signal from 16, hormones from 32. |
| `name` | What objects, genomes and `brain_io.ron` call it. |
| `class` | `Physical`, `Signal` or `Hormone`. |

**The classes** keep a sprite's body honest: genes decide how things feel, never what they do to the body.
- **Physical** chemicals are the body's actual state: energy, hydration, stamina, what's in the gut, injury. Only physiology and object verbs change them, and they're the only ones a verb may `Inject`. The brain doesn't feel them directly.
- **Signal** chemicals are written by genes. `reward` and `punishment` are what learning feeds on; every other signal chemical is a **drive**, an urge the sprite feels, which colours it on the map when it's the strongest (design §6.3). The help screen lists the drives in this file's order.
- **Hormones** are spare channels genes may use for anything. The starter genome uses `h0` for wariness after pain.

**Names the game needs:** physical `energy`, `hydration`, `stamina`, `food`, `water` and `injury`, and signal `reward`, `punishment` and `pain`. A pack without one fails to load, saying which.

**Breeding** (M2 design §10) reads two hormones: `feminiser`, as a male adult turns female once it's above physiology's `breeding.turns_female`, and `longing`, which in a pack from before courtship a female must have reached `breeding.breeds_alone` to breed alone (the built-in pack's starter genes still lead a female to breed alone by it, but her body allows it only after `breeding.alone_after`). The built-in list also has the drive `arousal`, which only genes use. A pack without both hormones (or without breeding's [loci](loci.md)) has no breeding.

**Courtship** (M2 design §15) reads one more: `receptivity`, as a pair can mate only once the female's is at least physiology's `breeding.receptive`. The built-in list also has `grief`, which only genes use: in the starter genome losing a partner raises it, and it holds arousal down. A pack without `receptivity` (or without courtship's [loci](loci.md)) breeds with no calls or courtship.

**Families** (M2 design §15.6) read no chemical. The built-in list has the drive `anxiety`, which only genes use: in the starter genome it rises while a sprite's family is out of sight.

## Working example

A new drive, `chill`, added to the list. To matter, it needs genes that raise and lower it ([genomes](genomes.md)), and a brain input so sprites can feel it ([brain inputs](brain-io.md)).

<!-- example: chemicals -->
```ron
(id: 27, name: "chill", class: Signal),
```

The built-in list in `data/chemicals.ron` is the full working example.
