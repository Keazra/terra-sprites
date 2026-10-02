# `categories.ron`

What a sprite perceives a thing as (design §3.5.5). A sprite pays attention to categories, its instincts and genes name them, and what it has learned about one bush it judges other bushes by. Every object type names one category ([objects](objects.md)).

| Field | Meaning |
|---|---|
| `id` | Permanent. 3 was `thornbush`, retired when thornbushes joined `bush`, so it is never used again. |
| `name` | What object types, genomes and brain inputs call it. |
| `plural` | How the screen says the category in general: "bushes are bad". Left out for one you don't count, which reads with "is": "fruit is good for hunger". |

- **Things share a category because they look alike to a sprite,** never because of what they do. A berry bush and a thornbush are both `bush`, so a sprite that has learned thornbushes hurt has to tell them apart by trying. A bush and a berry are different categories, though berries grow on bushes.
- **`water`, `sprite` and `cursor` must be in the list.** Water tiles, sprites and the Cursor aren't objects, but every world has them, and the engine perceives them as the categories with these names.
- **Each category is a brain input,** `attended_<name>`, with the ID `35 + id` for IDs 1 to 6 and `37 + id` from 7. So category IDs go up to 26.
- **This list belongs to the core game.** A mod picks categories for its object types from it and doesn't add to it (design §1.4): a genome names categories, so one a mod added would mean nothing to sprites from another pack.

## Working example

The built-in list.

<!-- example: categories -->
```ron
[
    (id: 1, name: "bush",   plural: "bushes"),
    (id: 2, name: "fruit"),
    (id: 4, name: "water"),
    (id: 5, name: "toy",    plural: "toys"),
    (id: 6, name: "sprite", plural: "sprites"),
    (id: 7, name: "cursor"),
]
```
