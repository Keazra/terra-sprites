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

## Working example

A new drive, `chill`, added to the list. To matter, it needs genes that raise and lower it ([genomes](genomes.md)), and a brain input so sprites can feel it ([brain inputs](brain-io.md)).

<!-- example: chemicals -->
```ron
(id: 25, name: "chill", class: Signal),
```

The built-in list in `data/chemicals.ron` is the full working example.
