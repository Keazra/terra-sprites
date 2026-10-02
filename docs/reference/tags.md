# `tags.ron`

What touching a thing does, for everything that shares a tag (design §3.5.6). A **contact** is a sprite eating a thing, hitting it, playing with it or crashing into it after a shove; walking past never counts. An object type lists its tags in [`objects.ron`](objects.md), after the built-in `Solid` and `Fixture`, so a new thorny plant is one word in its entry.

| Field | Meaning |
|---|---|
| `name` | What object types call it. Defined once. |
| `contact` | A map from each contact, `Eat`, `Hit`, `Play` or `Crash`, to its effects. A contact left out does nothing. |

- **Effects are `Inject(Actor, chemical, amount)` and `Signal(Actor, locus)` only,** on the sprite making contact. The chemical must be physical and the locus a pulse, as in a verb ([objects](objects.md)).
- A contact runs the tags' effects, in the order the thing lists its tags, then the thing's own verb. A try a tag answers isn't fruitless.
- Sprites never perceive tags. They see the thing's category, and learn what it does by touching it.

## Working example

`Thorny` as the built-in pack has it, and a `Stinging` tag: a stinging thing hurts a sprite that hits it or is shoved into it, but is safe to eat or play with.

<!-- example: tags -->
```ron
[
    (name: "Thorny",
     contact: {
         Eat:   [Inject(Actor, "injury", 0.05), Signal(Actor, "pricked")],
         Hit:   [Inject(Actor, "injury", 0.03), Signal(Actor, "pricked")],
         Play:  [Inject(Actor, "injury", 0.03), Signal(Actor, "pricked")],
         Crash: [Inject(Actor, "injury", 0.03), Signal(Actor, "pricked")],
     }),
    (name: "Stinging",
     contact: {
         Hit:   [Inject(Actor, "injury", 0.02), Signal(Actor, "pricked")],
         Crash: [Inject(Actor, "injury", 0.02), Signal(Actor, "pricked")],
     }),
]
```
