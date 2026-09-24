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
Said of a terrain that sprites can stand on and step onto. Shallow water is walkable; deep water and rock are not.

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

### The screen

**Map view**:
The panel that shows the map.
_Avoid_: map (for the panel)

**Viewport**:
The part of the map currently shown in the map view.
_Avoid_: camera

**Cursor**:
The one-tile marker the player moves over the map. The hand acts on its tile.
_Avoid_: pointer, selection (the selection is the chosen sprite)

**Semantic tile**:
What the map view draws for a tile, named by meaning (such as grass terrain) rather than by character.

**Theme**:
A mapping from semantic tiles to glyphs and colours.
_Avoid_: skin
