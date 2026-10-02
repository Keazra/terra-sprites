# Genomes

A sprite's genes, as text (design §2.8, §4.3, §5.7). The pack's `data/genomes/starter.ron` is what every sprite without parents is made from. `g` saves the selected sprite's genome to a file in the game's `genomes` folder, and the Place menu can place a sprite from any file there.

```ron
(
    format: 1,
    genes: [ ... ],
)
```

- **`format`** is 1. A file with a newer format than the game knows is refused.
- **Genes are written by type name,** with chemicals, loci, traits and categories by name. A field at its default can be left out: `invert: false`, `threshold: 0`.
- **The genes decide how things feel, not what they do to the body.** A gene that would change a physical chemical directly is kept but **flagged**, and does nothing. The genome tab of the inspector marks it.

## Gene types

| Gene | What it does |
|---|---|
| `HalfLife(chem, ticks)` | The chemical halves every `ticks` ticks. Signal chemicals and hormones only. |
| `Reaction(reactants: [(chem, n)], products: [(chem, n)], rate)` | One or two reactants turn into up to two products, at `rate` from 0 to 1. A physical chemical may only be a catalyst, on both sides with the same count. |
| `Emitter(locus, mode, invert, threshold, gain, chem)` | Adds `gain × (signal − threshold)`, when that's above 0, to a signal chemical or hormone each tick. `locus` is `Chem("name")` or `Locus("name")`. `mode` is `Level` (the value, or 1 − value with `invert: true`), `Rise` or `Fall` (how much it went up or down since last tick). A negative gain removes the chemical. |
| `Receptor(chem, threshold, gain, target)` | Pushes a receptor target ([loci](loci.md)) above or below 1 by `gain × (level − threshold)`, when that's above 0. |
| `InitialConcentration(chem, value)` | The level a signal chemical or hormone starts at. |
| `Trait(trait, value)` | `speed`, `sense_radius` or `lifespan`, held within [physiology](physiology.md)'s range. |
| `BrainParam(param, value)` | One of the brain parameters [physiology](physiology.md) lists, held within its range. |
| `Instinct(inputs: [(input, negated)], verb, weight)` | When up to three brain inputs ([brain inputs](brain-io.md)) are high together (or low, for `negated: true`), lean towards `verb`. `attended_toy` means "looking at a toy". |
| `AttentionInstinct(input, category, weight)` | When a State input is on, look at things of that category. |

**Duplicates.** `HalfLife`, `InitialConcentration`, `Trait`, `BrainParam`, `Instinct` and `AttentionInstinct` each set one value: the first gene for it counts, and a later one is **unexpressed**. `Reaction`, `Emitter` and `Receptor` add up.

**Genes by number.** Any gene can be written as `Gene(type: n, version: v, payload: "hex")`. One the game doesn't know is kept as an **unknown** gene: it does nothing, and is saved back unchanged, so a genome from a newer game survives a trip through an older one. A gene naming a category this pack doesn't have is kept as **unmatched**, the same way.

**The starter genome must be clean:** a flagged, unknown or unmatched gene in it stops the pack loading, since it would silently do nothing.

## Working example

A sprite that gets hungry, eats when hungry, and is drawn to fruit by hunger, an instinct the starter genome leaves for sprites to learn. The last gene is one the game doesn't know, kept as written.

<!-- example: genome -->
```ron
(
    format: 1,
    genes: [
        Trait(trait: "speed", value: 8.0),
        Emitter(locus: Chem("energy"), mode: Level, invert: true, threshold: 0.5, gain: 0.004, chem: "hunger"),
        Emitter(locus: Locus("ate"), mode: Level, gain: -0.5, chem: "hunger"),
        HalfLife(chem: "hunger", ticks: 2000),
        AttentionInstinct(input: "hunger", category: "fruit", weight: 1.0),
        Instinct(inputs: [("hunger", false)], verb: Eat, weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Wander, weight: 0.3),
        Gene(type: 900, version: 1, payload: "c0ffee"),
    ],
)
```

`data/genomes/starter.ron` is a full working example, with a comment on what each group of genes is for.
