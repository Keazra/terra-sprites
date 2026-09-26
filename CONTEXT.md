# Terra Sprites

An ASCII artificial-life game: sprites live, learn and die in a terrarium, a small simulated world that the player watches and touches through the hand.

## Language

### The world

**Map**:
The world's fixed-size grid of tiles.
_Avoid_: grid, board, level

**Tile**:
One cell of the map. It has exactly one terrain.
_Avoid_: cell, square

**Wall**:
The map's edge: the terrarium's wall, which nothing crosses.
_Avoid_: border, boundary

**Terrain**:
The kind of ground a tile has: grass, dirt, sand, shallow water, deep water or rock.
_Avoid_: tile type, biome, ground

**Walkable**:
Said of a terrain that sprites can stand on and step onto on foot. Shallow water is walkable; deep water and rock are not.

**Solid**:
Said of anything that can't be moved through, such as rock or a bush. Deep water isn't walkable, but it isn't solid: crossing it is a matter of ability, not physics.
_Avoid_: blocking, impassable

**Physics**:
The fixed rules of how things move and block: which tiles can be entered, what is solid, and what a step costs.
_Avoid_: physiology (the body's rules)

**Step cost**:
What stepping onto a tile costs a sprite, set by the tile's terrain. A diagonal step costs more than an orthogonal one.
_Avoid_: move cost, weight

**Corner-cutting**:
A diagonal step past an unwalkable tile on either side. It is never allowed.

**Region**:
A largest set of walkable tiles that can all reach one another by legal steps. Two tiles that touch only at a corner, with unwalkable tiles on both sides, are in different regions.
_Avoid_: island, area, zone

**Mainland**:
The largest region. Once a world is generated, it is the only one.
_Avoid_: continent

**Carving**:
Turning unwalkable tiles walkable to join a region to the mainland.
_Avoid_: bridging, tunnelling

**Seed**:
The number that, together with a world config and a data pack, determines a new world exactly.

**World config**:
The settings a new world is made from, such as the map's size. A **preset** is a file that holds one.
_Avoid_: settings, options

### Objects

**Entity**:
A sprite or an object. Each has an ID that is never reused.

**Object**:
An entity that isn't a sprite, such as a berry bush, a berry or a ball. Its **object type** says how it lives and what verbs do to it.
_Avoid_: thing, prop

**Fixture**:
An object attached to the ground, so nothing can push, pull or carry it, such as a bush. Being a fixture and being solid are separate: a crate could be solid yet pushable.
_Avoid_: obstacle

**Item**:
An object that is neither solid nor a fixture, so a sprite can stand on it and the hand can carry it, such as a berry or a ball.
_Avoid_: loose item, pickup

**Tag**:
A property an object type either has or lacks, such as solid or fixture. Having the tag means yes; lacking it means no.
_Avoid_: flag, attribute

**Stage**:
A phase of an object's life, such as a bush's seedling and mature stages. Each lasts a random time within its type's range.
_Avoid_: phase, age

**Counter**:
A named whole number an object keeps, between 0 and a maximum, such as the fruit on a bush.

**Lifecycle rule**:
Something an object does by itself: when a trigger fires and its conditions hold, its effects happen.
_Avoid_: behaviour, script

**Verb table**:
What each verb does when a sprite applies it to a target. Water and sprites have verb tables too, though they aren't objects.

**Expire**:
An object expires when its last stage ends.
_Avoid_: die (sprites die), rot

**Visual state**:
The name a theme draws an object by, such as a bush's seedling, bare or fruiting look.
_Avoid_: sprite (a sprite is a creature), appearance

**Roll**:
An item moving across the map one tile a tick after a sprite pushes it, such as a kicked ball, until it has gone as far as the push sent it.
_Avoid_: slide, fly

**Bounce**:
A rolling item turning away from what it ran into: straight back if it met it head on, glancing off at the same angle if it met it slantwise.
_Avoid_: collision, rebound

**Size**:
How big an object type, or a sprite, is: small, medium or large. What a rolling item does to what it meets depends on their sizes.

**Hardness**:
How hard an object type is, from 0 to 1. A rolling item crushes something smaller and softer than itself.

**Knock on**:
A rolling item stopping and passing its roll to the item it ran into, which rolls on the same way, as in billiards.
_Avoid_: push (a push is what a sprite does)

**Crush**:
A rolling item destroying a smaller, softer item it runs into, and rolling on.
_Avoid_: squash, break

### Sprites

**Sprite**:
A living creature of the terrarium, with a genome, a body and a brain. Sprites die; objects expire.
_Avoid_: agent, critter, pet

**Genome**:
A sprite's ordered list of genes, fixed for its whole life. It decides how the body feels, never what happens to it.
_Avoid_: DNA

**Gene**:
One instruction in a genome, such as "low energy raises hunger". Its type says what kind of instruction it is.
_Avoid_: allele

**Name**:
What the player calls a sprite: one they make up, or one generated at random. A sprite has no name until the player gives it one, and until then shows by its ID, as "Sprite #530".
_Avoid_: label

**Starter genome**:
The genome in the data pack that sprites are made from when they have no parents.

**Gene value**:
A number in a gene that variation may change, such as a strength or a threshold. The rest of a gene, such as which chemical it touches, never varies.

**Spawn variation**:
The small random change to the gene values of a sprite made from the starter genome.
_Avoid_: mutation (that comes with inheritance)

**Flagged gene**:
A gene that would change the body directly, which no gene may do. It stays in the genome but has no effect.

**Unexpressed gene**:
A gene that sets something an earlier gene in the genome already set, so it has no effect.

**Unknown gene**:
A gene this version of the game can't read. It's kept exactly as it is, and has no effect.

**Chemical**:
A named level in a sprite's body, from 0 to 1. Every chemical is physical, a signal or a hormone.
_Avoid_: stat

**Physical chemical**:
A chemical that is the body's actual state, such as energy, hydration or injury. Genes may read it but never change it.

**Signal chemical**:
A chemical the genome makes to tell the brain something: a drive or a learning signal.

**Drive**:
A signal chemical the sprite feels as an urge: hunger, thirst, pain, tiredness, boredom, loneliness or crowdedness.
_Avoid_: need, emotion, mood

**Learning signal**:
The reward or punishment chemical, which learning uses up every tick.

**Hormone**:
One of sixteen unnamed chemicals that only the genome uses, spare for evolution to put to work.

**Locus**:
Anything a gene can read or write: a chemical, a body sensor, a pulse or a receptor target.
_Avoid_: slot, input (brain inputs are a different list)

**Body sensor**:
A locus that physiology fills in every tick, such as the sprite's age or how many sprites are near it.

**Pulse**:
A locus that marks something that just happened to the sprite, such as eating or being petted. It lasts one tick.
_Avoid_: event (events are what the world reports)

**Trait**:
A body property the genome sets, within limits and at a cost that physiology fixes: speed, sense radius or lifespan.

**Physiology**:
The body's fixed rules: metabolism, digestion, water loss, healing, and the injury that starvation, dehydration and old age cause. Genes can't change them.
_Avoid_: physics (for the body)

**Injury**:
The physical chemical that measures harm. A sprite dies when its injury reaches 1.
_Avoid_: damage, health

**Old age**:
Being past its lifespan, which injures a sprite a little every tick.
_Avoid_: senescence

**Cause of death**:
What caused most of a sprite's recent injury, with the most recent counting most: starvation, dehydration, old age, or being hurt by a kind of object, such as a thornbush or another sprite.

**Hurt**:
Injury that something did to a sprite: a thornbush it touched, another sprite's hit, or the hand's Correct. Starvation, dehydration and old age injure a sprite but don't hurt it.
_Avoid_: damage, pricked (a thornbush word; the data names things, the screen says "hurt")

### Actions and movement

**Action**:
What a sprite is doing, such as wandering to a spot or resting, from when it starts until it ends. A sprite does one at a time.
_Avoid_: task, behaviour, activity

**Verb**:
A kind of action: Approach, Eat, Drink, Hit, Play, Retreat, Rest or Wander.
_Avoid_: command (the player's commands are different), move

**Outcome**:
How an action went: still walking, applied, blocked, failed or timed out.
_Avoid_: result, status

**Timeout**:
The longest any action may last. An action still going at the timeout ends as timed out.

**Move points**:
What a moving sprite gains each tick from its speed, and spends on steps.
_Avoid_: action points

**Perception flood**:
The tiles a sprite can reach within its sense radius, and what each costs to walk to. It's how the sprite sees what's around it and finds its way. Shortened to "flood".
_Avoid_: vision, field of view, pathfinding

**Reachable**:
Said of a tile the sprite's flood reached, or of a thing with a goal tile the flood reached. A thing the sprite can see but not reach is never its target.

**Destination**:
The tile a wandering sprite is walking to, picked when it starts to wander.

**Goal tile**:
A tile a sprite can act on its target from, such as any walkable tile beside a bush.

**Candidate**:
The nearest reachable thing of each kind around a sprite: the one its verbs would aim at.

**Scripted action**:
An action a hand-made world starts a sprite on, in place of what it would choose, so a test or a lab scenario can set up an exact situation. Nothing to do with lifecycle rules.

**Swap**:
Two sprites trading tiles in one tick, because each was stepping into the other's.
_Avoid_: pass, collision

**Committed path**:
A way around blocking sprites that a stuck sprite found, and keeps to until it arrives, is blocked again or its action ends.
_Avoid_: detour, reroute

**Target**:
The thing an action is aimed at, such as the berry bush a sprite is going to eat from. An action keeps the same target from start to end. Wander and Rest have none.
_Avoid_: goal (a goal tile is where the sprite stands to act)

### The brain

**Brain**:
What chooses a sprite's actions: it notices one kind of thing nearby, and picks a verb to do about it.
_Avoid_: AI, mind, controller

**Brain input**:
Something the brain feels, such as hunger, a pulse, or how far away its target is. Each has a permanent number.
_Avoid_: sense, locus (a locus is what genes read)

**Attention**:
The brain noticing one kind of thing nearby, such as berries or water, out of every kind it could reach. What it attends to is where its verbs aim.
_Avoid_: focus, perception

**Concept**:
Something the brain recognises from its inputs: one input on its own, or a combination such as "hungry and next to the target". Concepts are what the brain's choices are weighed on.
_Avoid_: neuron, feature

**Instinct**:
A built-in leaning a sprite is born with, set by its genome, such as "hunger leads to eating" or "hunger draws attention to berry bushes". Learning can later change it.
_Avoid_: reflex, rule (rules belong to objects)

**Brain parameter**:
A setting of how the brain works, such as how much chance is in its choices, set by a gene within limits physiology fixes.
_Avoid_: hyperparameter

### The screen

**Map view**:
The panel that shows the map.
_Avoid_: map (for the panel)

**Viewport**:
The part of the map currently shown in the map view.
_Avoid_: camera

**Cursor**:
The 3×3 marker on the map that follows the pointer. A click acts on the tile at its centre, as the cursor mode says.
_Avoid_: selection (the selection is the chosen sprite)

**Pointer**:
Where the mouse is on screen. The cursor follows it over the map view.
_Avoid_: mouse cursor

**Cursor mode**:
What a click on the map does: Select, Hand, Reward or Correct.
_Avoid_: tool

**Selection**:
The sprite the inspector shows, chosen by clicking it or with `Tab`.
_Avoid_: focus, target

**Inspector**:
The side panel of tabs about the selected sprite (Body, Brain, Chem, Genome) or the world (World).
_Avoid_: sidebar, details

**Observed list**:
What the player has watched the selected sprite finish since selecting it, at the end of the Body tab. It only grows while the sprite is selected.
_Avoid_: history, log (the event log is different)

**Detail view**:
A view the player switches on and off that shows the exact workings behind what the screen describes in words, such as a sprite's exact verb and destination.
_Avoid_: debug mode (it's in every build)

**Decision marker**:
The flashing `X` on the map where the selected sprite is heading: the destination of the action it decided on.
_Avoid_: destination (that's the tile itself), cursor, highlight

**Attention marker**:
The steady grey shading on the map of the one thing the selected sprite is paying attention to.
_Avoid_: highlight, attention mark

**Event log**:
The panel that lists what just happened to sprites, newest first.
_Avoid_: events panel, console, feed

**Emote**:
A glyph that takes turns with a sprite's own glyph on the map for about a second, showing something that just happened to it, such as the red `!` of being hurt.
_Avoid_: icon, bubble, flash

**Semantic tile**:
What the map view draws for a tile, named by meaning (such as grass terrain) rather than by character.

**Theme**:
A mapping from semantic tiles to glyphs and colours.
_Avoid_: skin

### The hand

**Hand**:
The player's way of acting on the terrarium: rewarding, correcting, grabbing, dropping and placing. It holds at most one thing.

**Reward**:
A pet or hug from the hand, which raises the sprite's reward chemical.
_Avoid_: tickle, positive

**Correct**:
An electric shock from the hand, which hurts the sprite and raises its punishment chemical.
_Avoid_: slap, punish, negative

### Zones (a later milestone)

**Zone**:
A discrete area of the terrarium that the player gives a purpose, such as a hatchery.
_Avoid_: biome, region (a region is about reachability)

**Device**:
An object in a zone that makes the zone's purpose possible, such as a hatchery's incubator.
_Avoid_: machine, building
