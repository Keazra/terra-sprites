# `brain_io.ron`

What a sprite's brain can feel (design §5.2). The file has three parts.

**`inputs`:** the brain's **State inputs**, each reading one chemical or locus.

| Field | Meaning |
|---|---|
| `id` | Permanent, never reused. 1 to 35, then from 64: IDs 36 to 63 are kept for the Target inputs. |
| `name` | What genomes call it. |
| `reads` | `Chem("name")` for a drive or a hormone, or `Locus("name")` for a body sensor or a pulse. Wrapped in `Target(...)`, such as `Target(Chem("arousal"))`, it reads the same in the body of the sprite it's looking at: its body language (M2 design §10). Wrapped in `Differs(...)`, such as `Differs(Locus("female"))`, it reads how far that sprite's level differs from its own. Both are 0 while it looks at anything but a sprite, such as an egg. A sprite heard calling, out of sight, reads as if it were in sight (M2 design §15.1). Three read what the sprite knows of the one it's looking at, rather than its body (M2 design §15): `Familiar`, how well it knows it, from 0 to 1; `Partner`, 1 if it's its partner; and `Kin`, 1 if it's its parent, its young, or its brother or sister. Any input that reads breeding's drive ([physiology](physiology.md)'s `breeding.drive`), in the sprite or the one it looks at, is 0 unless both are grown up (M2 design §5). |

The brain can't read physical chemicals (it feels the body only through drives), `reward` and `punishment` (learning uses them), `moving` and `resting` (they'd feed an action back into itself) or receptor targets. Reading one is an error.

The **Target inputs** aren't listed: they come from the game. There's one for each category, `attended_<name>` (such as `attended_bush`), saying what the sprite is looking at, then `target_distance` (42) and `target_adjacent` (43). The verbs (`Eat`, `Drink`, `Hit`, `Play`, `Approach`, `Retreat`, `Rest`, `Wander`, `Mate`, `BreedAlone`, `Court`, `Call`) are fixed too. `Court` and `Call` are offered only in a pack with courtship, which has its [loci](loci.md) and the [chemical](chemicals.md) `receptivity` (M2 design §15).

**`needs`:** the drives whose relief teaches a sprite what things are good for: when eating makes hunger fall, the sprite learns the thing it ate is good when hungry. Each must be a drive that's a State input. Pain is a drive but not a need, since its fall is a hurt fading.

**`first_order`:** the needs that come first. While one of them presses, what a sprite merely likes pulls less, though never not at all. Each must be in `needs`.

## Working example

The new `chill` drive from [chemicals](chemicals.md), felt by the brain as a State input and made a need. The rest of the inputs are as in `data/brain_io.ron`.

<!-- example: brain_io -->
```ron
(id: 88, name: "chill", reads: Chem("chill")),
```

<!-- example: brain_io_needs -->
```ron
needs: ["hunger", "thirst", "tiredness", "boredom", "loneliness", "crowdedness", "arousal", "chill"],
```
