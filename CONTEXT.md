# Terra Sprites

An ASCII artificial-life game: sprites live, learn and die in a terrarium, a small simulated world that the player watches and touches through the Cursor.

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
An entity that isn't a sprite, such as a berry bush, a berry or a ball. Its **object type** says what category it's in, how it lives, what verbs do to it, and what it's called, one and many ("berry bush", "berry bushes").
_Avoid_: thing, prop

**Category**:
What a sprite perceives a thing as, such as a bush, fruit, water or a sprite. Things share a category because they look alike to a sprite, never because of what they do to it, so a sprite still has to learn what each one does. A bush and a berry are different categories, though berries grow on bushes. Each object type is in one category, and every sprite is a sprite.
_Avoid_: kind, class, genus, plant (a botanist's class, not what a sprite tells apart)

**Fixture**:
An object attached to the ground, so nothing can push, pull or carry it, such as a bush. Being a fixture and being solid are separate: a crate could be solid yet pushable.
_Avoid_: obstacle

**Item**:
An object that is neither solid nor a fixture, so a sprite can stand on it and the Cursor can pick it up, such as a berry or a ball.
_Avoid_: loose item, pickup

**Tag**:
A property an object type either has or lacks. Having the tag means yes; lacking it means no. Solid and fixture are built in: they're physics. Others, such as thorny, are defined in the data, each saying what every contact with a thing that has it does, and every thing with a tag behaves the same way.
_Avoid_: flag, attribute, keyword

**Contact**:
A sprite touching a thing: eating it, hitting it, playing with it, or crashing into it. Walking past never counts.
_Avoid_: collision, touch (the Cursor's touch is a pet or a shock)

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
An item moving across the map one tile a tick after a sprite pushes it or the Cursor throws it, such as a kicked ball, until it has gone as far as the push sent it.
_Avoid_: fly, slide (a shoved sprite slides)

**Bounce**:
A rolling item turning away from what it ran into: straight back if it met it head on, glancing off at the same angle if it met it slantwise.
_Avoid_: collision, rebound

**Size**:
How big an object type, or a sprite, is: small, medium or large. What a rolling item does to what it meets depends on their sizes.

**Hardness**:
How hard an object type is, from 0 to 1. A rolling item crushes something smaller and softer than itself.

**Knock on**:
A rolling item swapping rolls with an item it ran into that is its own size, or smaller but at least as hard, as equal balls exchange momentum: the item ahead rolls on the same way, and the rolling item takes the roll the other had, or stops if it had none.
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
What the player calls a sprite: one they make up, or one generated at random from the data's syllables. A sprite has no name until the player gives it one, and until then shows by its ID, as "Sprite #530"; named, it shows as "Mira #530".
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

**Unmatched gene**:
A gene that names a category this world doesn't have, such as one from a newer version of the game. It's kept exactly as it is, and has no effect.
_Avoid_: orphan gene, unknown gene (that one can't be read at all)

**Chemical**:
A named level in a sprite's body, from 0 to 1. Every chemical is physical, a signal or a hormone.
_Avoid_: stat

**Physical chemical**:
A chemical that is the body's actual state, such as energy, hydration or injury. Genes may read it but never change it.

**Signal chemical**:
A chemical the genome makes to tell the brain something: a drive or a learning signal.

**Drive**:
A signal chemical the sprite feels as an urge: hunger, thirst, pain, tiredness, boredom, loneliness or crowdedness.
_Avoid_: emotion, mood

**Need**:
A drive whose relief teaches a sprite what things are good for: hunger, thirst, tiredness, boredom, loneliness or crowdedness. Pain is a drive but not a need.
_Avoid_: want, desire

**First-order need**:
A need that comes before a sprite's likes: hunger or thirst. The more it presses, the less what the sprite merely likes pulls on it, though never to nothing. The other needs are second-order.
_Avoid_: primary need, survival need, blocker (it quiets, never blocks)

**Relief**:
A need falling, by enough in a tick to count. It teaches that the thing touched is good for that need.
_Avoid_: satisfaction, drive reduction

**Learning signal**:
The reward or punishment chemical: the general good and bad that always count, such as a pet or a hurt. Learning uses both up every tick.

**Hormone**:
One of sixteen unnamed chemicals that only the genome uses, spare for evolution to put to work.

**Locus**:
Anything a gene can read or write: a chemical, a body sensor, a pulse or a receptor target.
_Avoid_: slot, input (brain inputs are a different list)

**Body sensor**:
A locus that physiology fills in every tick, such as the sprite's age or how many sprites are near it.

**Pulse**:
A locus that marks something that just happened to the sprite, such as eating or being petted. It lasts one tick. A pulse another sprite's verb caused records that sprite as its source.
_Avoid_: event (events are what the world reports)

**Attacker**:
The sprite whose hit a sprite has just felt: the source of its `was_hit` pulse. While the pulse lasts, the attacker is the sprite it would aim at.
_Avoid_: aggressor, enemy

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

**Wear** (M2):
Lasting harm a hard life leaves on a sprite, from hunger, thirst, injury and fear, which healing doesn't undo. Once a sprite is old, the more wear it carries, the faster it declines.
_Avoid_: stress (that's what causes it), damage, injury (injury heals)

**Cause of death**:
What caused most of a sprite's recent injury, with the most recent counting most: starvation, dehydration, old age, or being hurt by an object type, such as a thornbush, or by another sprite.

**Hurt**:
Injury or pain that something did to a sprite: a thornbush it touched, another sprite's hit, or the Cursor's zap or shock. A zap or shock hurts without injuring. Starvation, dehydration and old age injure a sprite but don't hurt it.
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
The thing in each category around a sprite that draws its eye most, of those it can reach: the one its verbs would aim at. Among things of one object type, it's the nearest.

**Scripted action**:
An action a hand-made world starts a sprite on, in place of what it would choose, so a test or a lab scenario can set up an exact situation. Nothing to do with lifecycle rules.

**Swap**:
Two sprites trading tiles in one tick, because each was stepping into the other's.
_Avoid_: pass, collision

**Committed path**:
A way around blocking sprites that a stuck sprite found, and keeps to until it arrives, is blocked again or its action ends.
_Avoid_: detour, reroute

**Retreat**:
Backing away from a target, a step at a time, straight away from it, for a few steps. The screen says "backing away".
_Avoid_: flee, run away (for the verb)

**Cornered**:
Said of a retreating sprite with no step that takes it further from its target. Its retreat ends, and it feels a `cornered` pulse.
_Avoid_: trapped, stuck (a stuck walker is blocked)

**Target**:
The thing an action is aimed at, such as the berry bush a sprite is going to eat from. An action keeps the same target from start to end. Wander and Rest have none.
_Avoid_: goal (a goal tile is where the sprite stands to act)

### The brain

**Brain**:
What chooses a sprite's actions: it notices one category of thing nearby, and picks a verb to do about it. Instinct and learning compete in it.
_Avoid_: AI, mind, controller

**Brain input**:
Something the brain feels, such as hunger, a pulse, or how far away its target is. Each has a permanent number.
_Avoid_: sense, locus (a locus is what genes read)

**Attention**:
The brain noticing one category nearby, such as water, out of every category it could reach, and within it the one thing that draws the eye most, such as one particular sprite. What it attends to is where its verbs aim.
_Avoid_: focus, perception

**Concept**:
Something the brain recognises from its inputs: one input on its own, or a combination such as "hungry and next to the target". Concepts are what instincts are weighed on.
_Avoid_: neuron, feature

**Instinct**:
A built-in leaning a sprite is born with, set by its genome, such as "hunger leads to eating" or "loneliness draws attention to other sprites". It never changes in a sprite's life: learning can outvote it, never erase it.
_Avoid_: reflex, rule (rules belong to objects)

**Brain parameter**:
A setting of how the brain works, such as how much chance is in its choices, set by a gene within limits physiology fixes.
_Avoid_: hyperparameter

**Link**:
How strongly one thing leads to another in an instinct: a concept to a verb ("hunger → eat"), or a brain input to a category to attend to ("loneliness → attends to sprite"). The genome sets it, and it doesn't learn.
_Avoid_: weight, synapse, connection

**Trace**:
The brain's record of what it attended to and chose on each of its last few ticks, most recent strongest. Habits are credited back along it.
_Avoid_: history, memory (memory is what was learned)

**Felt**:
The good a sprite took in on its last tick, its relief and reward, less the punishment. The design calls it `last_r`.
_Avoid_: last r, reinforcement (on screen)

**Worth**:
What an object type, or a particular sprite, is to a sprite, learned from experience: good for some of its needs, and good or bad in general. It draws the sprite's eye and steps towards the thing, or keeps it from touching it.
_Avoid_: value, valence, preference

**Bad**:
What a sprite learns about something that hurt it when it touched it, such as a thornbush it bit: don't touch it, don't go over to it. A bad thing isn't frightening, since it won't come after you.
_Avoid_: dangerous, harmful

**Fear**:
What a sprite learns about someone that hurt it by its own doing, such as a sprite that hit it. A frightening sprite catches its eye and makes it back away while it's near; fear never makes it attack. On screen: "Sprite #7 is frightening".
_Avoid_: bad (bad is about touching), wariness (a passing mood), threat

**Individual**:
A particular sprite that another remembers, with what it has learned about it: its worth and how frightening it is. Individuals are learned fast, fade, and are forgotten once faded or dead.
_Avoid_: acquaintance, contact, relationship

**Category summary**:
What a sprite thinks of the things in a category it hasn't met: the average of those it knows in it, which counts for nothing while it knows only one. So one thornbush doesn't make every bush bad. On screen it reads as the category: "bushes are bad".
_Avoid_: stereotype, prior

**Sprites in general**:
The sprite category's summary: what a sprite thinks of sprites it doesn't know, from the individuals it remembers. So one bully doesn't make it shy of everyone, but three might.
_Avoid_: the sprite kind (on screen), stereotype

**Habit**:
What a sprite has learned about doing one verb to one object type, or to sprites, such as "eating balls doesn't work" or "hitting sprites is bad".
_Avoid_: skill (a later design, #48), reflex

**Thing touched**:
What a feeling is about: the thing the sprite tried a verb on a moment ago, or the particular sprite. Worth is learned about it and nothing else. A sprite that hit it isn't one: that teaches fear of the sprite instead. For the Cursor's touch, "a moment ago" stretches to its reach back.
_Avoid_: target (for this), culprit

**Fruitless try**:
A try at a verb on a thing that has no rule for it, or whose rule can't be met, such as eating a ball or a bare bush. Nothing happens, and the sprite feels it.
_Avoid_: fizzle, failed attempt (failed is an outcome)

**Motive**:
The need whose instinct did most to make a sprite choose a verb. A fruitless try disappoints it.
_Avoid_: reason, goal

**Familiarity**:
How well a sprite knows an object type, or sprites, from having attended to it. Its opposite is **novelty**: how new the thing still is to the sprite. An apple is new to a sprite that knows only berries.
_Avoid_: knowledge

**Curiosity**:
A sprite's pull towards things that are new to it. Being hurt when it investigates teaches it that new things are bad.
_Avoid_: exploration (exploring is how sure its choices are)

**Wariness**:
A passing mood, raised by a run of hurts, that makes a sprite less curious for a while.
_Avoid_: fear (fear is of someone in particular)

**Memory**:
What a sprite has learned from experience: the worth of things and of individuals, its fears, and its habits.
_Avoid_: learned links, knowledge

**Lesson**:
A worth, fear or habit that has reached half a point from nothing. The event log announces each lesson once, as good or bad, or frightening.
_Avoid_: milestone (on screen)

### The screen

**Title screen** (M2):
The screen the game opens on, before any world is running: a new random world moving behind the title and its menu (Continue, New world, Load, Help, Quit). Flags that make or load a world skip it, and `t` at a world's quit prompt goes back to it (M2 design §8).
_Avoid_: main menu, start screen, splash screen

**Opening scene** (M2):
How the title screen starts: in the dark, the Cursor's light finds a sleeping sprite and the world fills in around it, the sprite wakes pleased, and the title and menu appear. Any key skips it (M2 design §8.1).
_Avoid_: intro, splash, cutscene

**Map view**:
The panel that shows the map.
_Avoid_: map (for the panel)

**Viewport**:
The part of the map currently shown in the map view.
_Avoid_: camera

**Track**:
The viewport following the selected sprite, so it stays in the middle of the map view. `T` turns it on or off; scrolling by hand turns it off. It follows whichever sprite is selected, and does nothing while none is.
_Avoid_: follow (that's the Cursor fixed on a sprite, with `F`), lock on

**Cursor**:
The player's hard-light projection into the terrarium: to the sprites the player is an advanced creature, and this is how they reach in. It shows on the map as a 3×3 grid that follows the pointer, unless it follows a sprite, and a click acts as the cursor mode says. Sprites see it only where the player has made it **visible**; otherwise its touch is a feeling from nowhere. It holds an item or leads a sprite, one thing at a time.
_Avoid_: hand, orb, selection (the selection is the chosen sprite)

**Visible (the Cursor)**:
Whether sprites can see the Cursor, switched with `H` for each cursor mode on its own; every mode starts hidden. While it's visible it draws as a frame of light in place of its arrows, and the status line says "seen". A visible Cursor is a thing of its own kind that sprites can go to or back away from, and they learn about it as about a particular sprite: a pet teaches them to like it, a zap, a shock or a shove into something that hurts teaches them to fear it, as well as what the touch teaches about the thing touched. Trying anything else on it is a fruitless try. A hidden Cursor's touch is a feeling from nowhere, as before.
_Avoid_: seen mode (the switch isn't a cursor mode)

**Getting used to the Cursor**:
How a sprite's fear of a visible Cursor wears off while the Cursor stays near and doesn't correct it, fastest with the Cursor on its own tile: it comes to see it can't do anything about it. Fear of sprites doesn't wear off this way.
_Avoid_: habituation (the general idea, a later design), taming

**Pointer**:
Where the mouse is on screen. The Cursor follows it over the map view, unless it follows a sprite, sitting one tile up and one left of it so the pointer's arrow doesn't hide it: that tile is the one the pointer **points at**, and what a click acts on.
_Avoid_: mouse cursor

**Cursor mode**:
What a click on the map does: Select, Train or Grab. `Q` and `E` do what the left and right click do, where the Cursor is.
_Avoid_: tool

**Selection**:
The sprite the inspector shows, chosen by clicking it or with `Tab`. It's the inspector's: the Cursor can follow another sprite.
_Avoid_: focus, target

**Follow**:
The Cursor fixed on a sprite, so it moves with the sprite in every mode and a pet or a shock needs no aim. `F`, or a middle click, turns it on or off in every mode: it follows the sprite under the Cursor, or else the selected one. It doesn't select the sprite, selecting another leaves it as it is, and the sprite's death ends it. While the Cursor leads the followed sprite, Follow steps aside and the Cursor follows the pointer, within the leash; otherwise it holds.
_Avoid_: lock on (its name before v26), track (that's the view following the selected sprite, with `T`)

**Activate**:
What Select mode's right click (or `E`) does: work the thing clicked, such as a device (a hatchery, a food dispenser). Nothing can be activated until devices arrive.
_Avoid_: function, use

**Status marks**:
The two corners of the Cursor, top right and bottom left, that report on it: in Train mode they flash `+` when a click is sent, then `☼` when it applied or `?` when it was refused; in Grab mode they show what the Cursor holds or leads. The other two corners are the mode marks, which show the cursor mode.
_Avoid_: indicators, lights, flash (on its own)

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

**Event filter**:
Which events the event log shows: all of them, the selected sprite's (what it did and what was done to it), or the major ones (deaths, lessons learned, refused commands). `m`, or a click on the filters on the log's border, goes to the next.
_Avoid_: log level, view

**Colour mode**:
What colours a sprite on the map: its strongest drive once one is above half (each drive's colour is the theme's), or plain, its own colour. `b` goes to the next.
_Avoid_: tint, palette

**Help screen**:
The overlay `?` opens: every key, grouped, the colour and emote legends, and the game's folder for its files.
_Avoid_: manual, cheat sheet

**Sprite list**:
The overlay `l` opens: every sprite, one to a row, with its age, its strongest drive and what it's doing, sorted by number, name, age or drive. Choosing one selects it and centres the viewport on it.
_Avoid_: roster, census

**Information policy**:
What decides what the screen may show: every display asks it first, about a panel and a subject (the world, a sprite or a tile). In M1 it shows everything; a later mode that hides information is a new policy, and never changes the world.
_Avoid_: fog of war, permissions

**Emote**:
A glyph that takes turns with a sprite's own glyph on the map for about a second, showing something that just happened to it, such as the red `!` of being hurt; the resting `z` shows for as long as the rest lasts, and at least a second.
_Avoid_: icon, bubble, flash

**Semantic tile**:
What the map view draws for a tile, named by meaning (such as grass terrain) rather than by character.

**Theme**:
A mapping from semantic tiles to glyphs and colours. Two are built in; the player's own go in the themes folder and are picked with `Ctrl+T`.
_Avoid_: skin

### The Cursor

**Train mode**:
The cursor mode for teaching: a left click rewards the target, a right click corrects it. The target is the sprite the Cursor follows, or else the one under it.
_Avoid_: Reward mode, Correct mode (merged into Train)

**Grab mode**:
The cursor mode for moving things: a left click grabs what's under the Cursor, and the next lets go of the sprite or puts the item down; a right click throws or shoves it (slice 11b). Pressing `C` again opens the Place menu, for new things (slice 11c).
_Avoid_: Hand mode

**Grab**:
To take hold of what's under the Cursor: a sprite is led, an item picked up. A sprite on the tile comes before an item, and a fixture can't be grabbed.
_Avoid_: lift, pick up (for a sprite)

**Lead**:
To hold a sprite through the Cursor without lifting it: it stays on the map and walks after the Cursor at its own pace, choosing nothing for itself, until the player **lets go**. Sprites are led, never lifted.
_Avoid_: drag, carry, pick up

**Leash**:
How far the Cursor may go from a sprite it leads, 5 tiles in a square, and the flashing dotted line that shows it, from the Cursor to the sprite.
_Avoid_: lead (that's the verb), rope, tether

**Hold**:
What the Cursor does with an item it has picked up: the item leaves the map, its life going on, until the Cursor **puts it down** on a tile.
_Avoid_: carry, in hand

**Throw**:
To let go of a held item with a push, so it rolls away like a kicked ball, as far and in the direction the player **aims**.
_Avoid_: toss, fling

**Shove**:
To let go of a led sprite with a push, so it **slides** as far and in the direction the player **aims**. It doesn't hurt, unless the sprite crashes into something that does.
_Avoid_: throw (for a sprite), push (a sprite's push rolls an item)

**Slide**:
A shoved sprite moving a tile a tick in the shove's direction, choosing nothing, until it has gone as far as the shove sent it or something stops it.
_Avoid_: roll (an item rolls), fly, glide

**Aim**:
To set a throw's or a shove's direction and distance by pulling the pointer back from the Cursor, away from where the thing should go, with the right button or `E` held, like a pool cue: the further the pull, the further it goes, up to how far the Cursor can send a thing its size. The press grabs what's under the Cursor if it has hold of nothing, and while the player aims, the Cursor sits still on the thing, so a led sprite stands still.
_Avoid_: drag, flick, swing

**Aim line**:
The steady line of dots that shows, while the player aims, where a throw or a shove will go, ending in a small circle where it would stop if nothing's in the way.
_Avoid_: trajectory, leash (that's the line to a led sprite), cue

**Crash**:
A sliding sprite stopping against a solid object or another sprite. It's a contact, so it counts as touching that thing: a crash into a thornbush pricks, and teaches that thornbushes are bad. Stopping against rock, deep water or the terrarium's wall isn't a crash.
_Avoid_: collision, bump, bounce (an item bounces)

**Place menu**:
The Grab-mode menu, opened by pressing `C` again, for putting new things into the world: the object types whose data offers them, a new sprite, and a sprite from a genome file.
_Avoid_: spawn menu, build menu

**Place item**:
What the player picked from the Place menu, waiting on the Cursor until a Grab-mode click places it, or a right click puts it away. It isn't held: what the Cursor leads or holds stays as it was.
_Avoid_: held item (that's an item picked up), brush

**Place**:
To make a new object on a tile through the Cursor, at the start of its first stage. Only the types whose data offers them can be placed, and their data says what the tile must meet; the built-in ones ask nothing, so a placed bush may wall things off.
_Avoid_: put down (that's a held item), plant, build

**Spawn**:
To make a new sprite on a tile through the Cursor: from the starter genome with spawn variation, or from a genome file.
_Avoid_: create, birth (that's breeding, later), place (for a sprite)

**Genome file**:
A genome written as text, which `g` saves from the selected sprite and the Place menu reads back to spawn one like it.
_Avoid_: DNA file, save (that's the whole world)

**Reward**:
The Cursor's good touch, which raises the sprite's reward chemical: a **pet**, or amplified, a **hug**.
_Avoid_: tickle, positive

**Correct**:
The Cursor's bad touch, which hurts the sprite without injuring it and raises its punishment chemical: a **zap**, or amplified, a **shock**.
_Avoid_: slap, punish, negative

**Amplified**:
A Reward or Correct given with Shift (on `Q` or `E`) or Ctrl (on a click): a hug rather than a pet, a shock rather than a zap.
_Avoid_: strong, heavy

**Reach back**:
How many ticks back a Reward looks for the sprite's latest attempt, which its feeling is then about: about two seconds of the player's time at the speed they're playing. A Correct has none: it looks back only the touch window, so a late shock can't land on the wrong thing.
_Avoid_: reach (a flood's reach is where a sprite can walk), touch window (that's for every other feeling)

### Saves

**Save**:
The whole world written to a file, so it can be loaded later and carry on exactly as it would have: every sprite's chemistry, brain and action, every object, the Cursor's grip, the dice, and the data pack the world was made with. A save is opaque: it's for the game to read, not the player.
_Avoid_: snapshot (that's a replay's start), save game, genome file (that's one sprite's genes)

**Quicksave**:
The one save `F5` writes and `F9` loads, named `quicksave`, overwritten each time.
_Avoid_: quick save, slot

**Autosave**:
A save the game makes by itself: after every 10 minutes that time runs, unpaused, and on quitting. The last three are kept, the newest as `autosave-1`.
_Avoid_: backup, checkpoint (that's a replay's hash)

**Save format**:
The layout of a save's world, numbered by its schema version. A change that only adds things keeps the number; one that breaks it bumps the number and comes with a step that upgrades the old layout. A save in a newer format than the build is refused.
_Avoid_: save version (the build that wrote it is the sim version)

### Replays

**Replay**:
A file from which a session can be played back exactly: where the world started, every command the player's clicks sent, and the world's hash now and then to check against. It plays only in the build that recorded it, and is opaque, like a save.
_Avoid_: recording (that's the world keeping one), demo, movie

**Session log**:
The replay every session writes as it goes, `last_session.replay`, starting afresh from each load, and written at each autosave, on quitting and on a panic.
_Avoid_: log (that's the event log), crash dump

**Snapshot**:
A replay's start when it isn't a world generated afresh from its seed: a save of the world as the recording began, such as a loaded save.
_Avoid_: save (that's the player's file), checkpoint

**Checkpoint**:
The world's hash, kept in a replay where the recording starts and every 1,000 ticks, so playback can check it's still the same world.
_Avoid_: save, snapshot, autosave

**Playback**:
Playing a replay (`--replay <file>`): its commands at their ticks, with the player's own input closed to the world. Time, the view and the inspector still work, and the player can take it over.
_Avoid_: replay mode, rerun

**Taking over**:
Ending playback where it has reached (`Ctrl+R`), so the world is the player's from that tick: the replay's commands not yet played are dropped, and the session log starts afresh from a snapshot of it. The replay file itself doesn't change.
_Avoid_: branching, forking, rewinding

**Divergence**:
Playback finding the world different from its recording at a checkpoint. It's reported with the last checkpoint that matched, as the world parted somewhere between the two.
_Avoid_: desync, mismatch (for this; a mismatched version or pack is refused before playback starts)

### Testing and tuning

**Lab scenario**:
A file describing a world to run for many seeds and what to count in it, such as the thornbush bites in the first and the last 5,000 ticks. It is both a test and the main tool for tuning.
_Avoid_: benchmark, experiment

**Trainer**:
What a lab scenario can use in place of a player: it answers a kind of applied action with a pet or a zap for the one who did it, through the same commands a player's clicks send.
_Avoid_: teacher, bot

**Control run**:
The same seed run again for comparison, without the one thing being measured: without learning (A1), or without the trainer (A2, A3).
_Avoid_: baseline (that's a measurement on `main`)

**Viability run**:
The default world run for many seeds with no player, counting who survives and what killed those who didn't (A4).
_Avoid_: soak, ecological soak

**Grown-up death**:
A death in the viability run after tick 10,000. The first 10,000 ticks count as childhood, since every sprite there starts as a newborn. A newborn dying of thirst is natural, but a grown sprite dying of hunger or thirst means it can't fend for itself (A4).
_Avoid_: adult death (sprites have no life stages until M2)

**Soak**:
A run of a million ticks with a random script of the Cursor's commands, looking only for panics and broken invariants (A7).
_Avoid_: stress test; a soak for any long run without commands (that's a viability run)

**Baseline run**:
The measurements taken on one commit of `main`, so that each commit can be compared with the last: the viability run, the soak, and the A1–A3 lessons' numbers.
_Avoid_: daily report, nightly soak, health check

**Baseline report**:
A baseline run's numbers, for one commit.

**Broken**:
What a baseline report calls a seed that panicked or broke an invariant. A criterion that isn't met yet isn't broken.
_Avoid_: crash (that's a shoved sprite's), failed

**Briefing**:
The observer's plain-language account of a baseline report: what was tested, what changed since the last one, and what might explain it.
_Avoid_: summary, analysis

**Observer**:
The AI agent that reads a baseline report, and the code and commits behind it, and writes the briefing. It changes nothing itself.
_Avoid_: monitor, watcher

### Zones (a later milestone)

**Zone**:
A discrete area of the terrarium that the player gives a purpose, such as a hatchery.
_Avoid_: biome, region (a region is about reachability)

**Device**:
An object in a zone that makes the zone's purpose possible, such as a hatchery's incubator.
_Avoid_: machine, building
