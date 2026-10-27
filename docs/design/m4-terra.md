# Terra Sprites — M4 "Terra" design

- **Status:** Draft: the plan, from the ship's design pass with the owner (rounds 1 to 6, 2026-10-12). Each slice adds what it settles as it's built.
- **Date:** 2026-10-12
- **History:** edited in place; each change is a file in [`changes/`](changes/)
- **Builds on:** [M1 design](m1-a-sprite-lives.md), the rulebook for everything this doesn't change; [M2 design](m2-generations.md), whose Slice 31 (§11) draws the view past the wall; and [M3 design](m3-body-and-verbs.md), Body & verbs, which comes first: its Touch, Push and Pull are how sprites work panels and switches, and its Grab and Drop how they plant a garden. M4 builds on M3's checks B8–B10 (things saying what verbs do, curiosity, and teaching Push and Pull, M3 design §5.2); M3 never tries a panel as such, so this design's T5 is the first check to (§12)
- **Covers:** what the ship is, its sectors and how they're made, devices, sealed doors and how the ship opens, Story mode and Lab mode on the ship, the keeper's inventory, what the screen shows, what a bigger world costs, old worlds, and what "M4 is done" means. From [#181](https://github.com/Keazra/terra-sprites/issues/181), with [#26](https://github.com/Keazra/terra-sprites/issues/26)'s zones

---

## 0. Vision

The terrarium was always a ship. M4 shows it: **Terra**, an ark flying through space, and its sprites are the Terra Sprites. The land M1 and M2 show becomes one of its biomes, the Meadow. Around it are other biomes, the ship's own bays, and the main deck that joins them, most of them sealed at the start.

The player is **the keeper**, the ark's AI. Its builders are gone and it woke with its memory damaged, so half its own ship is a mystery to it: devices whose jobs nobody remembers, and sectors behind doors it can't open. Its sprites can. The ship opens as the sprites learn, and the player's teaching is what opens it.

The owner's principles carry over (`docs/agents/how-we-work.md`). Three matter most here:
- **The engine enforces physics only.** A biome, a bay, a device and a door are data. What a device does is a rule in its data, as what eating a berry does is in the berry's.
- **The ship is a character.** It juts into the land: ribs of hull, causeways, staging areas full of devices, all doing things the game never explains.
- **The world can't depend on where the player looks.** The same seed makes the same world, whoever watches it and wherever they scroll (§10).

---

## 1. Scope

### 1.1 In M4

Slice numbers run on across milestones, so these are numbered on from M3's last, Slice 50, when M4 is planned. Each gets an issue then, in a GitHub milestone "M4 — Terra".

| Slice | What it adds | Needs |
|---|---|---|
| **Turns when due** | The engine gives an object its turn only when it has something to do (§10) | nothing; built early, in M2 (2026-10-17) |
| **Sectors and doors** | The map in sectors, with hull between them and open doorways, and counting that stops at the hull; one sector on screen, its neighbours dimmed; a click switches; Track, Follow and `Tab` cross with a sprite; the mode in the world; old worlds as a ship of one sector (§3, §9, §11) | M2's Slice 31 |
| **The built-in ship** | Terra's plan in data: the bays drawn by hand, the main deck in three, plating, hull, glass and the prow (§3, §5) | Sectors and doors |
| **Biome recipes** | Biomes made from recipes, with their staging areas and set pieces: the five biomes (§4) | The built-in ship |
| **Devices** | Rules that reach a whole sector, and switches. Staging areas keep their biomes to their recipes; Hydroponics and the Reservoir work (§6) | Biome recipes |
| **Sealed doors** | Panels; sealed sectors asleep and dark; what the keeper remembers, with the builders' log and the ship plan; Story and Lab mode's doors (§7, §9) | Devices, and M3's verbs |
| **The keeper's inventory** | Placing from the inventory in Story mode; the food dispenser and the vendor; the Cursor holding an egg (§8) | Devices |
| **The bays' jobs** | The Hatchery's egg layer and incubators, the Store and its vendor, Observation's window, the Engines (§5) | The keeper's inventory |
| **The Bridge** | The story's end (§2.3) | Sealed doors |
| **Terra acceptance** | The criteria of §12, measured and played | all |

### 1.2 Not in M4

- **Day and night.** Wild Terra (M5) decides it; until then the ship's lights stay on (round 5, Q2).
- **Weather and critters.** Wild Terra brings them, from the staging areas' devices (§6.4).
- **Caves carved into hills,** generated, for the Highland's next pass (round 4, Q7).
- **Doors that shut again,** to keep groups of sprites apart; lifts between sectors far apart; plans put together from parts each seed. A modder can write another plan.
- **A refill station,** like *Creatures 1*'s honey pots: an empty jug dropped at the Reservoir's tap fills with water, so sprites can carry water.
- **Landing,** and the larger game ([#47](https://github.com/Keazra/terra-sprites/issues/47)).

---

## 2. The story and the modes

### 2.1 The keeper and the ark

- **Terra is an ark,** carrying life to a new world. Where it's going stays unsaid until the Bridge opens (§2.3).
- **The player is the keeper,** the ship's AI. It reaches in through the Cursor, hard light cast by the ship's projectors; to the sprites, it's an advanced creature, as in M1.
- **Its builders are gone,** and it woke with its memory damaged. That's why the devices are as strange to it as to the sprites, and why most of the ship is sealed: the parts of its own ship it can't reach yet.
- **Nothing on screen tells the story outright.** The builders' log (§7.3) and the closing scene (§2.3) are the only words it gets.

### 2.2 Story mode and Lab mode

The mode is picked in the new world box (M2 design §8.3) and kept for the world's life (§11).

| | **Story mode** | **Lab mode** |
|---|---|---|
| Speed | 4× at most (M2 design §16.2), until the end-game unlock (§2.3) | every speed, up to Max, and fast-forward (M2 design §16) |
| Sealed doors | only a sprite opens one, by working its panel (§7.2) | the keeper can open any, by activating it; none ever shuts (round 4, Q2) |
| Sealed sectors | dark, and the view can't go there | the view can go there and look in, but the sector stands still and the Cursor can't reach in until its door is open |
| The ship plan | the keeper's memory: a sealed sector is dark on it (§9.3) | every sector shows |
| Placing | takes from the keeper's inventory (§8) | makes things from nothing, as today |
| New sprites | from eggs only: laid by sprites, or from the Hatchery's stock (§5, §8.4) | as today: a new sprite, or one from a genome file, too |

### 2.3 The end

- **The Bridge is the last door to open,** behind the prow at the ship's front.
- **Opening it ends the story, and play goes on** (round 5, Q3). The keeper gets its whole memory back: the builders' last entry in the log, and where the ship is going. A short closing scene plays, a counterpart to the title screen's opening scene (M2 design §8.1), and then the game carries on with the whole ship open.
- **What the scene shows, and where the ship is going,** are written when the Bridge is built.
- **The end-game unlock** (the owner, 2026-10-14, Slice 27's round, Q1): once the entire ship is reactivated, the game offers to lift Story mode's speed cap, and if the player accepts, that world runs at every speed, up to Max, from then on. It stays a Story world in everything else, and declining keeps the cap. Whether the offer comes with the Bridge or once every sector is awake is settled when the Bridge is built (M2 design §16.2).

---

## 3. The ship

### 3.1 Sectors

- **The ship is one world, as the terrarium is today:** one map, one population, one clock, one save (round 1, Q1). Its sectors sit side by side in it, each with walls of its own, joined by doors through the walls between them. This is the shape of *Creatures 3*'s ark.
- **Every open sector lives at once,** on screen or not. A sealed one sleeps until its door opens (§7.1).
- **Two kinds of sector:** a **biome**, large and made from a recipe with the seed (§4), and a **bay**, small and drawn by hand for a job in running the ship (§5). Bays are what #26 called zones.
- **The main deck** runs along the ship's length, and every sector opens onto it. It's three bays, the aft, middle and fore deck, joined by doors (round 3, Q3).
- **Neighbours can share a door too,** such as a causeway from the Marsh into the Highland, but the deck is the main way round (round 2, Q6).
- **Sight, walking and memory already work across sectors:** sprites see through an open doorway, walk through it, and remember water in the next sector. A thirsty sprite that drank at the Marsh's lake last week walks through the doorway to it, as on any trip to water it remembers.
- **Counting stops at the hull.** What counts the things within some distance counts only those in its own sector: a sprite's sense of company (`nearby_sprites`), its sense of a mate near by (M2's Slice 23), and an object rule's `DensityBelow`. Sight stops at hull already, since a sprite's view floods out from it (`perception.rs`). In a world of one sector, nothing changes.
- **The sprite limit,** the preset's `most_sprites` (100 in the default world), is for the whole ship (round 2, Q5).

### 3.2 What the ship is made of

- **Plating:** the ship's metal floor, a terrain that sprites walk on and nothing grows on. It can't hold a Nest bare, its data says, so soil gives families a better home (M3 design §15.5).
- **Hull:** the ship's frame, a solid terrain like rock. A sector's walls are hull of its own, so the wall between two sectors, a few tiles thick, is their two walls back to back, with each doorway through both (round 2, Q9).
- **Glass:** the ship's outer wall, along its flanks and around the engines at the stern, with space past it, as Slice 31 draws it (M2 design §11.2, §11.3). On the ship, the map is the rectangle around the whole ship, and **every tile in it is in one sector, walls included, or is space.** Sprites can't reach or sense space, and the glass is drawn one tile outside a sector wherever it faces space.
- **The prow:** the ship's front, a blade of iron like a scythe, swept back past the flanks. It's only a look (round 3, Q4): drawn past the wall as glass is, iron instead of stars, with no debris.
- **Set pieces:** small hand-made pieces of the ship, a few tiles big, that a sector's recipe places into its land: a rib of hull from the glass, a causeway of plating over water, a pump house (round 2, Q2).

### 3.3 The plan

- **One built-in plan** (round 2, Q7). Every new world has the same bays in the same places. Each biome's slot names the biome it takes, and the seed makes its land, so set pieces land differently each time.
- **It's data,** as everything else is: the plan names each sector, its place, its recipe or drawing, its doors and which of them start sealed. A modder can write another.
- **The ship comes to about 3× today's terrarium** (round 3, Q6), and what's open at the start, the Meadow, the aft deck, Hydroponics and the Reservoir, about today's terrarium in all, so M2's tuning for the harsher world holds (Slice 29).

| Where | Sectors |
|---|---|
| **The aft deck** | the Engines (sealed), the Meadow, the Scrub (sealed), Hydroponics, and the Reservoir through it |
| **The middle deck** (sealed) | the Marsh, the Hatchery, and Observation through it, all sealed |
| **The fore deck** (sealed) | the Highland, the Lake, the Store and the Bridge, all sealed |

### 3.4 Making the world

- **Every open sector is one connected piece,** as today's whole map is (M1 design, Carving). Each sealed sector is joined up within itself, ready for when it opens.
- **Making it stays quick.** A map four times today's terrarium took under a tenth of a second to make (measured on 2026-10-12).

---

## 4. Biomes

### 4.1 Recipes

- **A biome is made from a recipe** in the game's data: its size, its mix of terrain, how many of each plant and thing, its set pieces and roughly where they go ("a causeway from each door, a pump house by deep water, a rib every 20-odd tiles along the glass"), and its staging area.
- **Each recipe sets its own size,** from a quarter to about two-thirds of today's terrarium (round 3, Q6).
- **A new biome is one data file,** as easy to add as an object type is today.

### 4.2 Staging areas

- **A staging area is the part of a biome that the ship built to make it work** (round 2's red lines): plating and hull jutting in from the main deck, holding the devices that keep the biome to its recipe (§6.2).
- **It's part of its biome,** as in *Creatures 3* (round 3, Q1): bright with the biome when the biome's on screen, dimmed with it from the deck.
- **It's hand-made,** like a bay, and the biome's recipe names it, so the Marsh always has its pump.
- **The biome's door onto the deck opens into it,** at its foot, and a doorway leads out of it onto the land. Sprites walk in as anywhere.
- **Every biome has one;** bays don't, since they're the ship already.

### 4.3 The five biomes

First drafts (round 4, Q7). Their numbers are tuned when each is built. They use today's terrain and objects, and each biome's own plants and critters wait for their slices.

| Biome | Size | Its land | Its staging area's devices keep… | What it asks of sprites |
|---|---|---|---|---|
| **Meadow** | about 200×130 | grass, ponds, a little rock, berry bushes, a ball or two: today's terrarium | its plants reseeded and its ponds full | nothing hard: food and water close together, where life starts |
| **Marsh** | about 180×110 | shallow and deep water with islets, joined by a causeway | its water up | finding the islets, where the bushes are |
| **Scrub** | about 180×110, sparse | sand and rock, thornbushes, a rare pool | it dry | remembering where water is (M2's Slice 20) |
| **Highland** | about 150×100 | rock and hills, with narrow paths between them | its paths open, and later its cold | finding the way, and the sheltered spots where bushes grow |
| **Lake** | about 160×100 | mostly deep water, a shore round it, islands | its water clean | reaching the islands, by a causeway from the shore |

---

## 5. Bays

Drawn by hand, tile by tile, in the game's data, as the tests' hand-drawn worlds are (round 4, Q8).

| Bay | Where | At the start | Its job, and what sprites find there |
|---|---|---|---|
| **The main deck** | the ship's length, in three | the aft deck open; the middle and fore deck sealed | the ship's crossroads, where sprites from different biomes meet: columns, consoles, hard-light projectors, and on the aft deck a food dispenser (§8.3) |
| **Hydroponics** | off the aft deck | open, in disrepair | a garden to restore (§6.3): beds under lights, which grow whatever's planted in them once its lights and water are back on |
| **Reservoir** | through Hydroponics | open | the ship's water: a tank sprites can drink from, and the pump that waters Hydroponics, switched off at the start |
| **Engines** | the aft end of the deck | sealed | the ship's drive: loud and warm, with nothing to eat. With Wild Terra, it's where to keep warm |
| **Hatchery** | off the middle deck | sealed | the ark's stock of eggs: an egg layer, and incubators where an egg hatches sooner (§8.4) |
| **Observation** | through the Hatchery | sealed | a wall of glass onto space, and nothing to eat: a place to watch the stars |
| **Store** | off the fore deck | sealed | crates the builders left, and a vendor among them that makes balls and, later, other toys (§8.3): where new kinds of thing arrive |
| **Bridge** | the fore end, behind the prow | sealed, last | the keeper's own seat, and the story's end (§2.3) |

---

## 6. Devices

### 6.1 What a device is

- **A device is an object type in the data, a fixture,** and everything it does is a rule in its data, as with a berry bush (round 2, Q3). It may change the land and affect sprites near it.
- **The game never says what a device is for.** The player sees its name and has to watch what happens around it. Sprites learn its worth as they learn anything's.
- **The engine gains two kinds of rule, and no lists:** one that reaches the whole of the device's sector rather than the tiles near it (§6.2), and a switch (§6.3). What each device does stays in its data.

### 6.2 Keeping a biome to its recipe

- **A staging area's devices keep its biome as its recipe says** (round 3, Q2): where its plants die out, they seed them again; the Marsh's pump keeps its water up; the Scrub's keep it dry.
- **A player who watches may notice** that a bare patch of the Meadow comes back, and comes back slower while one of its staging area's devices is switched off (§6.3).

### 6.3 Switches, and Hydroponics

- **Devices never wear out or break** (round 4, Q6). Some have a switch that turns them off and on again, worked like a panel (§7.2) by a sprite, or activated by the keeper (Select mode's right click, or `E`).
- **A device's data says whether it has a switch** and what it does while off. A curious sprite that pulls the Marsh pump's lever stops it, and the Marsh slowly dries out until a sprite pulls it again, or the keeper activates it.
- **Hydroponics is a garden to restore, then fill** (round 5, Q1).
  - **Its beds are a terrain,** fertile like grass while the bay's lights and water are on, and soil that can hold a Nest (M3 design §15.5). With either off, they dry out and nothing new takes root.
  - **At the start it's in disrepair:** its lights are off, the Reservoir's pump that waters it is off, and its beds hold only a few withered bushes. Switching both on brings it back, and it's the first thing a new keeper learns about devices.
  - **What's planted in a bed grows.** A berry dropped on a bed always sprouts, where on grass it's 1 in 10. Once other biomes have plants of their own, their fruit grows in beds too, so the garden holds whatever the keeper and the sprites bring.
  - **Sprites tend it with M3's verbs:** a sprite that carries a berry and drops it on a bed has planted it, and learns to as it learns anything (M3 design §9.2).

### 6.4 Wild Terra aboard

When Wild Terra comes (M5), its weather and critters are the ship's doing, and so device data (round 4, Q9): sprinklers for rain over the Meadow, mist over the Marsh, none over the Scrub, and cold from the Highland's vents. Critters live in the biomes, and a staging area's devices restock them when they die out, as they reseed plants. A sprite that switches a device off stops its rain.

---

## 7. Doors, and how the ship opens

### 7.1 Doors and sleeping sectors

- **A door is a way through the wall between two sectors:** an open doorway one to three tiles wide, or a sealed door.
- **A sealed door is a solid object across the doorway,** whose data says what opens it, as a berry bush's data says what eating it does (round 3, Q8). Once open, it never shuts again.
- **A sealed sector sleeps:** nothing in it takes a turn until a door into it opens. Nothing can get in, so nothing is lost. At the start, that's most of the ship.
- **Its clock stops while it sleeps.** Its things don't age, and when it wakes, each one's next turn comes as long after waking as it was due after the sector fell asleep, so nothing in it comes due at once. Since doors never shut again, a sector only sleeps from the world's start, and wakes as it was made.

### 7.2 Panels

- **Each sealed door has a panel on each side,** so it opens from whichever sector reaches it first: a device that a sprite works with M3's verbs: a plate that a touch opens, a button that's pushed, or a lever that's pulled, as its data says (round 4, Q5), which M3's B8 checks on a made-up thing of its own (M3 design §6.1). Different doors can ask for different lessons, and a new panel needs no new code.
- **The first doors are the easiest.** The Engines' and the Scrub's doors, and the door from the aft deck to the middle deck, each take a touch, which sprites give new things anyway. The doors further forward take a push or a pull, which a sprite has to be taught.
- **Once is enough:** one sprite working a panel once opens its door for good.
- **In Story mode, only a sprite can work a panel.** The keeper's hard light can't work the ship's old locks: activating one does nothing, and the status line says "The lock doesn't answer you: only a sprite can work it". In Lab mode the keeper opens the door itself (§2.2).
- **The order the ship opens in** follows from where sprites can reach: the aft deck's doors first, then the ones those lead to.

### 7.3 What the keeper remembers

When a door into a sealed sector opens (round 4, Q4):
- **The sector's name and its place on the ship plan** fill in (§9.3).
- **A major event in the event log** names the sprite who opened it: "Mira #12 opened the door to the Marsh". It's one of the things a sprite can be known for.
- **A line of the builders' log** shows once on the status line, then stays under the sector's name on the ship plan: one or two lines per sector, in its data. For the Marsh, say: "Hygrostat 3 holding. The reeds took on the second try." It hints at what the ship is for, and never says outright what a device does.

---

## 8. The keeper's inventory

In Story mode, placing takes things from the keeper's **inventory** instead of making them from nothing, and devices around the ship make the things that fill it (the owner's answer to round 5, Q4). In Lab mode the Place menu makes things from nothing, as today (M1 design §6.5).

### 8.1 What goes in, and how

- **Any thing the Cursor can pick up,** such as a berry or a ball, and an egg (§8.4). Never a fixture or a hatched sprite. So in Story mode a bush is planted from a berry, which sprouts in a bed (§6.3), rather than placed as a seedling.
- **Putting away:** while the Cursor holds a thing in Grab mode, the Place menu (`C` again) starts with "Put away: berry", and picking it puts the berry in the inventory.
- **Taking out:** the rest of the menu lists what's in the inventory, with how many of each, "berry ×3". Picking one makes it the place item (M1 design §6.5), and placing it takes it out of the inventory; putting the place item away returns it.

### 8.2 How much, and how it keeps

- **Up to 10 things in all,** set by the preset. With no cap, the keeper could stock up on food, and the harsher world (M2's Slice 29) would ask nothing of the player.
- **Things put away keep:** they're out of the world, so a berry doesn't rot until it's placed again.
- **At the start it holds a few,** set by the preset, such as three berries and a ball.
- **It's part of the world:** it's in `WorldState`, saved, replayed and counted in the state hash, and putting away and placing are recorded commands, since what the keeper places changes the world (M1 design §2.3, §2.7).

### 8.3 What makes things

Devices make things as *Creatures*' machines do (researched for round 6, Q2), with the rules a berry bush already uses to drop berries, so each is data.

| Device | Where | Like, in *Creatures* | What it makes |
|---|---|---|---|
| **Food dispenser** | the aft deck, and later other bays | the cheese machines all over *Creatures 3*'s ark, which a norn or the hand pushes for a lump of cheese | a berry, when a sprite pushes it or the keeper activates it |
| **Vendor** | the Store | the Empathic Vendor, whose foods the hand picks from, and the Agent Creator | the keeper activates it and picks from its list, in its data: a ball now, other toys later. One thing at a time, with a wait between, as *Docking Station*'s injector has a timer |
| **Seed launchers** | each staging area | the seed and egg launchers in each terrarium | its biome's plants, where they've died out (§6.2) |
| **Egg layer** | the Hatchery | every game's egg layers | an egg (§8.4) |

- **Every maker counts before it makes,** as the games' population limits do: the dispenser makes a berry only while few lie near it. Terra's rules already have the condition, `DensityBelow`, which stops a berry sprouting where there are enough bushes (`data/objects.ron`).
- **Hydroponics** makes berries too, once it's restored, from the bushes in its beds: plants, not devices (§6.3).

### 8.4 Eggs and new sprites in Story mode

- **Story mode makes no sprite from nothing** (round 6, Q3). New sprites come from eggs: those sprites lay (M2's Slice 23), and the Hatchery's. The Place menu's "new sprite" and "sprite from a genome file" are Lab mode's.
- **The Hatchery is the ark's store of eggs.** Once its door opens, activating its egg layer gives an egg from the starter genome, now and then, while the world has room for another sprite, as a sprite can lay one only then (M2's Slice 23).
- **An egg put in an incubator hatches sooner,** as in *Creatures 1*.
- **The Cursor can hold an egg** (round 6, Q3), as *Creatures 1*'s hand could: pick it up, carry it and put it down, never throw or shove it. So an egg can go in the inventory, where it doesn't hatch until it's placed again, and the keeper can carry one to an incubator. M2's Slice 23 doesn't let the Cursor move an egg; M4 does.
- **A world with no sprites and no eggs left, in it or in the inventory, before the Hatchery opens is lost,** as a terrarium left untended would be. A major event in the event log says so: "No sprite or egg is left, and the Hatchery is sealed". It's the one way to lose in Story mode; the world runs on, and a save can be loaded.

---

## 9. On screen

### 9.1 One sector at a time

- **The map view shows one sector at a time, the active one,** scrolling within it as the map does today, with Slice 31's margin past its walls (M2 design §11.1). Scrolling never flips to the next sector, which would be easy to do by accident (round 2, Q8).
- **The top bar and the status line name the active sector.**
- **Past a wall:**
  - **Hull,** the active sector's own and then, **dimmed**, its neighbour's, and the neighbouring sector past it, dimmed too, as far as the margin reaches: its land, things and sprites, still moving (round 3, Q7).
  - **Glass, then space,** where the sector faces out (§3.2).
  - **Iron,** where it faces the prow.
- **Sealed sectors are dark,** not dimmed: only their walls, their doors and their staging areas' outlines show. The keeper's memory of them is gone.

### 9.2 Switching

- **A click on a dimmed tile makes its sector the active one,** and so does `Q` there, which is a click (M1 design §6.5). That click does nothing else, so the keeper can't drop a berry into the Marsh by mistake; the next one does. A right click or `E` does nothing there, as over space.
- **The view stays where it is,** so nothing jumps: the sector clicked brightens, the one left dims, and the view's limits become the new sector's. If the view is past the new sector's margin, it stays there until it scrolls, and then it can scroll back towards the sector but no further out.
- **The pointer over a dimmed sector** hides the Cursor, as over space (M2 design §11.4), and the status line names it: "Highland: click to go there".
- **The view goes with a sprite across a door:** Track switches to the sector a tracked sprite walks into, `Tab` and the sprite list to the chosen sprite's sector, and Follow switches the active sector with the followed sprite without moving the view.
- **With Follow on, the active sector is the followed sprite's.** A click on a dimmed tile, or `Q`, doesn't switch: it does what it does over space (M2 design §11.4): it reaches the followed sprite, or nothing while the Cursor leads it. Picking another sector on the ship plan turns Follow off.
- **In Story mode, a sector can be gone to once a sprite has opened a door into it,** and whenever the keeper likes after that. In Lab mode, any sector, from the start (§2.2).
- **UI tests,** as Slice 31's (M2 design §11.6): a click or `Q` on a dimmed tile switches sectors and sends no command, and `E` there sends none; with Follow on, neither switches, and both reach the followed sprite; a view left past the new sector's margin scrolls back but no further out.

### 9.3 The ship plan

- **`p` shows the ship plan over the map view:** the ship's outline, with each open sector's name, how many sprites are in it, and the active one marked (round 4, Q3).
- **A click on a sector, or the arrow keys and `Enter`, takes the view there;** `Esc` or `p` closes it. The view lands where the keeper last left that sector, or in its middle the first time.
- **In Story mode, it's the keeper's memory:** a sealed sector is dark on it, with only its outline, and fills in when its door opens, with its line of the builders' log. In Lab mode every sector shows.

---

## 10. What a bigger world costs

Measured on 2026-10-12, in release builds, with 100 sprites, on a cloud machine.

- **A bigger ship costs its things, not its land.** Nothing in a tick visits every tile, and a sprite only looks at the tiles around it, but every plant and berry took a turn every tick (until Turns when due, below), and the preset scales how many there are with the area. A map four times today's took 7.9 ms a tick by tick 10,000, against today's 5.1; with the same number of things, it cost the same as today's.
- **The simulation can't see the screen,** and mustn't (round 3, Q5). The world can't run differently depending on where the player looks: replays, saves and the determinism check (M1's Slice 15) would break, and a sprite would freeze the moment the player looked away.
- **So every open sector lives at once, and sealed ones sleep** (§3.1, §7.1), which saves about 0.4 ms a tick on a 4× map at the start.
- **The engine gives each object its turn only when it has something to do,** rather than every tick: most turns today do nothing, since a berry only waits to rot. It saves 0.5 to 1.4 ms a tick, and nothing has to catch up. Built early, in M2 (2026-10-17): an object's turn comes on a tick it enters its first stage, its stage ends or one of its periodic rules is due, the objects due still take theirs in ascending ID order, and a turn that wasn't due would have done nothing, so every world stays as it was, the golden files included. On today's map it saves 1.4% of a tick's instructions; how that was measured is in [the change file](changes/2026-10-17-turns-when-due.md).
- **Sectors with no sprites in them don't sleep.** It would save no more than sealed sectors do, and catching up exactly afterwards is fiddly.

---

## 11. Saves, replays and old worlds

- **What sectors there are, which doors are sealed, and the inventory** are part of the world, in `WorldState`, so they're saved, replayed and hashed. Which sector is active, and the ship plan's state, belong to the screen, as the viewport does.
- **So is the mode,** picked when the world is made and fixed for its life, since it changes what commands do: whether a panel answers the keeper, and whether placing takes from the inventory. It has been world state since Slice 27 (M2 design §16.1), and so will be whether a Story world has lifted its cap (§2.3).
- **Old worlds are a ship of one sector** (round 2, Q10): a preset with no plan, and every save made before the ship, is today's terrarium, with glass all round. Lab scenarios and tests carry on as they are.
- **An old world keeps the mode it was saved in,** and one saved before Slice 27 brought the modes in loads in Story mode, the game it was. With no plan there's no Hatchery and no device to fill the inventory, so in both modes placing and new sprites work as in M2: the inventory and Story mode's eggs (§8) come with a plan.
- **A preset names the plan it uses.** The default preset uses Terra's.

---

## 12. What "M4 is done" means

A first draft; the acceptance slice settles the numbers.

| # | Criterion | Checked by |
|---|---|---|
| T1 | **The ship.** A new world in Story mode has Terra's plan, with the Meadow, the aft deck, Hydroponics and the Reservoir open, and the rest sealed and dark | Test |
| T2 | **Speed.** The speed check (M1's A5: 100 sprites, at least 200 ticks a second) still passes with every sector open | Benchmark |
| T3 | **Determinism.** The determinism check (M1's Slice 15) passes on a ship world, and a save and a replay round-trip with sprites in several sectors | CI |
| T4 | **Crossing.** A thirsty sprite that remembers water in the next sector walks through the door to it, and a sprite beside the hull senses no company or mate on its other side | Test |
| T5 | **Opening.** Sprites open the first touch panel on their own in most seeds, and a sprite the player teaches opens a push or pull panel | Lab scenario, and played |
| T6 | **Old worlds.** Every M1 and M2 save and preset loads as a ship of one sector, in the mode it was saved in, and plays as it did | Test |
| T7 | **The whole story.** The owner opens the ship to the Bridge in Story mode | Played |
