# `pack.ron`

Names the data pack. A world records the pack it was made with (design §2.8), and these two fields say which pack that was.

| Field | Meaning |
|---|---|
| `name` | The pack's name. The built-in pack is `"core"`. |
| `version` | The pack's version, as text. |

Neither may be empty.

## Working example

<!-- example: pack -->
```ron
(
    name: "core",
    version: "1",
)
```
