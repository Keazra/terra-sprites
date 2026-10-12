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
| `injury` | `(starvation, dehydration, old_age)`: injury each tick while energy is at 0, hydration is at 0, or an old sprite's body has given out. A sprite dies at full injury. |
| `ageing` | Growing up and growing old (M2 design §9): `young_speed`, the share of its speed a newborn walks at; `growth`, the energy growing up takes, spread over childhood; `upkeep`, energy a tick for every 10,000 ticks of `grows_old`; `wear: (low, starving, parched, injured)`, the lasting wear a tick while energy or hydration is under `low`, and for each unit of injury; `wear_speedup` and `sooner`, how much faster the most worn sprite's genes decay and how much sooner, as a share of `grows_old`, it grows old; `decay: (ticks, hit, gives_out)`, the ticks an unworn old sprite's genes take to fall to `gives_out` on average, `hit` health at a time; and `scar: (above, chance, hit)`, the chance each tick that injury above `above` takes `hit` off a random gene. Optional: a pack without it gets the built-in one. |
| `breeding` | Having young (M2 design §10): `hatches`, the ticks an egg takes to hatch; `cost`, the energy an egg costs, split between a pair or all one's alone; `fed`, the energy and hydration a sprite needs above to be fertile; `rest`, the ticks after its last egg before it's fertile again; `turns_female`, the `feminiser` level at which an adult male turns female for good; `breeds_alone`, the `longing` level a female must have reached to breed alone; and `drive`, the need that drives breeding, one of [`brain_io.ron`](brain-io.md)'s `needs`. Whatever the genes, and whether or not the pack breeds, the engine has the drive work only between adults: it reads 0 in a sprite not grown up and in a choice about one, and its fall teaches worth only after a Mate, about that partner (M2 design §5). Optional: a pack without it gets the built-in one. A pack without the `feminiser` and `longing` chemicals or the `fertile`, `female`, `female_near`, `male_near` and `bred` [loci](loci.md) has no breeding; one with them must name a `drive` that's a need. |
| `cause_fade` | Each cause of injury is remembered, halving every this many ticks, so a death names what did most of the harm lately (design §4.10). |
| `traits` | `(min, max)` for `speed`, `sense_radius`, `grows_old` and `grows_up`: what a `Trait` gene is held within. A pack from before M2 may call `grows_old` `lifespan`, and leave out `grows_up`. |
| `brain` | For every brain parameter, `(range: (min, max), default: d)`: what a `BrainParam` gene is held within, and the value a genome without one gets. Every parameter must be listed, and only those, except `place_fade` and `plasticity`, which packs from before M2 lack, and `taught_rate` and `taught_fade`, which packs from before #187 lack: they get the built-in ones. |
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
