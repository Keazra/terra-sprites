# `physiology.ron`

The body's fixed rules, which genes can't change (design §4.4, Appendix B): how fast a sprite burns energy and dries out, how it heals and dies, the ranges its genes are held to, and how strong the Cursor's touch is. Levels are fractions of full, from 0 to 1, and rates are per tick.

| Field | Meaning |
|---|---|
| `newborn` | `(energy, hydration, stamina)`: what a new sprite starts with. Injury and the gut start empty. |
| `first_population` | `(low, high)`: the world's first sprites start with energy and hydration each drawn between these fractions of the newborn level. |
| `metabolism` | Energy used each tick: `basal`, plus `per_sense_tile` for each tile of sense radius, plus `per_step` for each step at speed 8 (scaling with the square of speed ÷ 8). |
| `digestion` | `(food, water)`: how much of the gut moves into energy and hydration each tick. |
| `hydration_loss` | Hydration lost each tick. |
| `stamina` | `(per_step, idle, resting)`: stamina used by a step, and regained standing and resting. |
| `healing` | Injury healed each tick. |
| `injury` | `(starvation, dehydration, old_age)`: injury each tick while energy is at 0, hydration is at 0, or the sprite is past its lifespan. A sprite dies at full injury. |
| `cause_fade` | Each cause of injury is remembered, halving every this many ticks, so a death names what did most of the harm lately (design §4.10). |
| `traits` | `(min, max)` for `speed`, `sense_radius` and `lifespan`: what a `Trait` gene is held within. |
| `brain` | For every brain parameter, `(range: (min, max), default: d)`: what a `BrainParam` gene is held within, and the value a genome without one gets. Every parameter must be listed, and only those, except `place_fade`, which packs from before M2 lack: they get the built-in one. |
| `receptor_targets` | `(min, max)` for every receptor target in [`loci.ron`](loci.md). |
| `nearby_sprites` | `(radius, full)`: the `nearby_sprites` sensor counts other sprites within `radius` tiles, reading 1 at `full` of them. |
| `spawn_variation` | Each gene value of a new sprite is multiplied by a random factor between 1 − this and 1 + this, so no two are quite alike. |
| `lesson_threshold` | How far from 0 a learned value must get before the event log announces the lesson. |
| `relief_deadband`, `touch_window` | The smallest fall in a need that counts as relief, and how many ticks after a try the thing tried is still what a feeling is about. |
| `forget_below` | A remembered sprite is forgotten once everything learned about it is nearer 0 than this, and a remembered place once it has faded below this. |
| `remembered_places` | `(held, per_kind, merge)`: a sprite remembers at most `held` places (M2 design §7) and at most `per_kind` of one kind, and places of one kind `merge` tiles apart or fewer are one place, so a pond is one place. `held` and `per_kind` must be at least 1. Optional: a pack without it gets `(held: 8, per_kind: 3, merge: 5)`. |
| `actions` | `(rest_bout, retreat_bout, timeout)`: a rest's length in ticks, a retreat's in steps, and how many ticks before any action gives up. |
| `movement` | `(flood_refresh, occupied_penalty, replan_after)`: how often a sprite re-plans its way, how much it avoids tiles with sprites on, and how long it waits when blocked before going round. |
| `cursor` | The Cursor's touch: the reward a `pet` and a `hug` give; the punishment and pain a `zap` and a `shock` give; `max_reach_back`, how many ticks back a pet looks for what the sprite just tried; and `furthest`, how far it throws or shoves a `small`, `medium` or `large` thing. |

Every number is checked when the pack loads: fractions from 0 to 1, rates not negative, each range's `min` no more than its `max`, and each default within its range.

## Working example

The built-in `data/physiology.ron` is the working example: it lists every field, with a comment on each. To make sprites thirstier, raise `hydration_loss`; to make a pet teach faster, raise `cursor.pet`.
