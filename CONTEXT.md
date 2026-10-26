# Terra Sprites

An ASCII artificial-life game: sprites live, learn and die in a terrarium, a small simulated world that the player watches and touches through the Cursor.

## Language

### The world

**Map**:
The world's fixed-size grid of tiles. On the ship (#181) it holds every sector.
_Avoid_: grid, board, level

**Tile**:
One cell of the map. It has exactly one terrain.
_Avoid_: cell, square

**Wall**:
The terrarium's edge, which nothing crosses: today the map's edge. On the ship (#181), each sector has walls of its own, crossed only through a door, and a wall that faces outward is glass onto space, except at the prow.
_Avoid_: border, boundary

**Terrain**:
The kind of ground a tile has: grass, dirt, sand, shallow water, deep water or rock; on the ship (#181), plating, hull and Hydroponics' beds too.
_Avoid_: tile type, biome (that's a kind of sector), ground

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
A sprite touching a thing: eating it, hitting it, playing with it, or crashing into it. Walking past never counts. Planned (M3): touching, grabbing, gripping, pushing and pulling it count too, and playing with it goes, since Play stops being a verb (M3 design §5.3, A1).
_Avoid_: collision, touch (for contact in general: Touch is a sprite verb from M3's Slice 37; the Cursor's touch is a pet or a shock)

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
How big an object type, or a sprite, is: small, medium or large. What a rolling item does to what it meets depends on their sizes. Planned (M3, Slice 35): Size also gains tiny, berries become tiny, and how many arms a sprite needs to grab and hold a thing depends on size: tiny things stack in one arm up to the handful; a small thing, such as an egg, needs one arm; a medium thing, such as the ball, needs both; a large thing isn't held (M3 design §9.1).

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
A sprite's ordered list of genes, fixed for its whole life. It decides how the body feels, never what happens to it. Its genes' health can fall as it ages, but the genome itself doesn't change. Its eggs get it mixed in stretches with their father's, now and then with a copying error, or with a gene doubled, lost or changed.
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
A number in a gene that spawn variation or a copying error may change, such as a strength or a threshold. The rest of a gene, such as which chemical it touches, never varies. Followed across a species, each is named for what its gene does, when it works and its field, as `hunger → Eat: weight` or `always → Wander until Adult: weight` (M2 design §12.7, §16.4). A trait's or a brain parameter's is held within a range physiology gives, and a level within 0 to 1; the rest can grow or shrink without end, but never cross 0 (M2 design §16.6).

**Spawn variation**:
The small random change to the gene values of a sprite made from the starter genome, larger for `grows_old` so that lives differ.
_Avoid_: mutation, copying error (that's an egg's)

**Copying error** (M2):
A gene value changed as an egg's genes are copied: rare, and usually large, from half to double. It never changes what a gene does, so a flagged gene stays flagged. The egg remembers what the gene was.
_Avoid_: mutation in what the player reads (the data's `mutation` section and the code may say it), typo

**Copy and change** (M2):
Now and then, as an egg's genes are copied, a whole gene is doubled, lost, or changed in what it does: what an instinct reacts to or the action it leads to, what draws the eye to what, or the life stages it works in. Most changes are made on a fresh copy, so the original still works. It's how the species grows genes the starter genome never had. The egg remembers each, and the Genome tab lists them as new, doubled and lost.
_Avoid_: mutation in what the player reads, copy change (one of them is a doubled, lost or changed gene), duplication, deletion, rearrangement

**Stretch** (M2):
A run of neighbouring genes a pair's egg takes from one parent before it switches to the other, about every 10 genes, as chromosomes cross over. The parents' genes line up by what each does, not where it sits.
_Avoid_: block, chunk, crossover alone (that's the chance of a switch)

**Flaw** (M2):
A hidden fault in one of the two copies of a gene, one from each parent. One sound copy is enough, so a flaw shows only in a broken gene. It arises now and then in copying, and the world's first sprites carry a few. Only a gene that can weaken, and isn't flagged, has copies to flaw.
_Avoid_: defect, recessive, bad allele

**Carrier** (M2):
A sprite with a flaw in one copy of a gene. It shows nothing, but passes the flaw to about half its young.
_Avoid_: heterozygote

**Broken gene** (M2):
A gene with both copies flawed: born at half health, it works at half strength, and it brings old age sooner and speeds decay as wear does. A lone egg breaks each flaw its mother carries about half the time, and close kin share flaws.
_Avoid_: dead gene, failed gene, knockout

**Falling behind** (M2, Slice 29):
A species that has become worse at living than the starter's genes in the same world: its world fails its bars, or it loses the race against the starter's genes (the same fresh worlds run once with the starter's genome and once with genomes from the end of its long run, M2 design §19.6). What a species does better without is evolution; falling behind is the harm.
_Avoid_: wear and decay for this (those are a sprite's), genetic decline, degeneration

**The race** (M2, Slice 30b):
G4's second test, which the long run runs every time: after each seed whose world didn't die out, that seed's fresh world runs as the viability run twice, once with the starter's sprites and once with sprites taking after the hatched sprites alive at the long run's end, on the same tiles, with their ages and starting levels drawn as the starter's sprites' are, so only the genes differ. A seed is won when the evolved side's young grow up more often; the race is won with at least 8 of the seeds that lasted and were judged. Lost, the species has fallen behind (M2 design §19.6).
_Avoid_: competition, tournament; lone newborns (that's the unattended run given the end's genes, a reading with no bar)

**Careful copying** (M2, held in reserve):
Copying the needs genes so that they're lost or changed a tenth as often as other genes, as *Creatures*' gene flags do. It's built only if a world in the long run fails a bar while more than half its living lack a needs gene. A bar held over to a later revisit, the race among them, doesn't count.
_Avoid_: protected genes, mutation bias (it changes how often, never which way)

**Flagged gene**:
A gene that would change the body directly, which no gene may do. It stays in the genome but has no effect.

**Unexpressed gene**:
A gene that sets something an earlier gene in the genome already set, so it has no effect.

**Unknown gene**:
A gene this version of the game can't read. It's kept exactly as it is, and has no effect.

**Dormant gene** (M2):
A gene switched off at the sprite's life stage, such as one that works only from adulthood in a young sprite. It's in the genome, but has no effect until its stage comes.
_Avoid_: inactive gene, silenced gene

**Switch** (M2):
The life stages a gene works in: from one stage (young, if it names none) until another (never, if it names none). Traits and starting levels can't switch.
_Avoid_: toggle, trigger

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

**Sleepiness** (M3, Slice 39):
A drive of its own, apart from tiredness: it builds the whole time a sprite is awake, whatever it does, and only sleep eases it. Sleep comes in a few long stretches, costs a fifth less energy than standing awake, and dulls the senses, so a sleeping sprite notices only what's close or loud. Tiredness from exertion stays, eased by short rests. Planned (M3 design §13).
_Avoid_: tiredness (that's from exertion), fatigue, rest (a rest is a breather)

**Needs gene** (M2, #237):
One of the starter genome's genes that keeps a sprite fed, watered and rested: it raises or eases hunger, thirst or tiredness, fades one, or turns one into its action, as `drank → thirst` or `hunger → Eat`. The starter has twelve: for hunger, `energy → hunger`, `ate → hunger`, `fullness → hunger`, hunger's half-life and `hunger → Eat`; for thirst, `hydration → thirst`, `drank → thirst`, thirst's half-life and `thirst → Drink`; and for tiredness, its two emitters and `tiredness → Rest`. #237 named eleven, before Slice 29 added `fullness → hunger`. They're found from the data, not listed: the needs a starter gene raises from the body's own state are the body's needs, so another species' data pack has its own (M2 design §16.6). A species that lost one spread it because it did better without it, which is evolution, not wear.
_Avoid_: vital gene, essential gene, survival gene, core gene

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
A body property the genome sets, within limits and at a cost that physiology fixes: speed, sense radius, when it grows up or when it grows old.

**Physiology**:
The body's fixed rules: metabolism, digestion, water loss, healing, and the injury that starvation, dehydration and old age cause. Genes can't change them.
_Avoid_: physics (for the body)

**Injury**:
The physical chemical that measures harm. A sprite dies when its injury reaches 1.
_Avoid_: damage, health

**Gut** (M2, Slice 29):
The one place where everything a sprite eats or drinks waits before its body takes it in. Each thing moves on at its own pace: food only as the body has room for it, while water the body can't hold passes through. It holds about a berry and a drink.
_Avoid_: stomach, belly, chamber

**Full** (M2, Slice 29):
Said of a sprite whose gut has no room for what's in front of it. Its body won't take the bite or the drink.
_Avoid_: sated, stuffed

**Fullness** (M2, Slice 29):
The starter genes' hormone for how full the gut feels: everything in the gut raises it, and a bite or a drink that finds no room raises it at once. It drains hunger, and leans a sprite away from eating and drinking alike, though strong thirst can outweigh it. It isn't a need, so its fading teaches nothing, and it isn't a drive, so it doesn't colour a sprite.
_Avoid_: satiety, satiation

**Life stage** (M2):
Where a sprite is in its life: young, adult or old. Before it hatches it's an egg, which has none. Its genes set when it grows up and when it grows old, and some genes switch with it. Code may call it `stage` where only a sprite's is meant.
_Avoid_: stage alone in what the player reads (that's an object's), age group

**Grows up** (M2):
The age at which a sprite stops being young: its `grows_up` trait. Growing up costs energy, spread over childhood.
_Avoid_: matures, comes of age

**Grows old** (M2):
The age at which a sprite becomes old: its `grows_old` trait, brought sooner by wear. It replaced lifespan.
_Avoid_: lifespan (the M1 word for it), dies of old age (that comes later)

**Gene health** (M2):
How whole one of a sprite's genes is, from 1 down to 0. It only falls, and the gene works in proportion to it. Not inherited, though a broken gene is born at half. A sprite has no health of its own: how hurt it is, is its injury.
_Avoid_: gene damage (that's what lowers it), copying error (that's inherited), gene quality (that's its flaws)

**Decay** (M2):
Old age's harm to the genome: once a sprite is old, its repair stops keeping up, and damage lands on random genes, lowering their health. An instinct takes half.
_Avoid_: decline, senescence

**Scar** (M2):
A little health a bad injury takes off a random gene for good, at any age.
_Avoid_: wound, mutation

**Old age**:
What kills an old sprite once its genes' average health has fallen to where its body gives out: it injures it a little every tick.
_Avoid_: senescence, past its lifespan (lifespan is now when it grows old)

**Wear** (M2):
Lasting harm a hard life leaves on a sprite's body, from starving, going without water and injury, which healing doesn't undo. The more wear it carries, the sooner it grows old and the faster its genes decay.
_Avoid_: stress (that's what causes it), damage, injury (injury heals)

**Egg** (M2):
An item a mother lays on a free tile beside her, holding the sprite it hatches into: its genes, parents and name. It's an object, not a sprite: it feels, learns and needs nothing, and nothing a sprite does to it does anything. The Cursor picks it up and puts it down, but never throws, pets or zaps it. Other sprites see it as a thing of its own kind. Planned (M3): a sprite can carry one in an arm, and a sprite or the Cursor can throw one; a thrown egg breaks 3 times in 4, by its data, and the sprite inside dies (Slice 40). From Slice 41 it starts inside its mother, and needs a parent beside it to grow once laid (M3 design §14, §15).
_Avoid_: unborn sprite, larva, egg stage (an egg was a life stage until #196)

**Pregnancy** (M3, Slice 41):
The while a mother carries her egg inside her, after a mating that works and before she lays it beside her: she's hungrier and slower, and can't conceive again until she has laid. If she dies pregnant, the egg dies with her. Planned (M3 design §15).
_Avoid_: gestation, carrying (that's having something in its arms)

**Hatch** (M2):
When an egg becomes a young sprite where it lies, as its type's stage ends (750 ticks after it was laid), with a newborn's body and the egg's ID; the item is gone. Its age counts from then. One that's due waits while the Cursor holds it or a sprite stands on it.
_Avoid_: born (a hatchling is born as it hatches; the world's first sprites and the Cursor's are born without hatching), spawn (that's the Cursor's)

**Parents** (M2):
The sprite that laid an egg, its mother, then its partner, if it had one, its father. A sprite the world began with, or that the Cursor made, has none.
_Avoid_: sire and dam

**Origin** (M2):
Where a sprite came from: one of the world's first sprites, placed by the Cursor, or hatched. One with no parents says which in their place.
_Avoid_: source, spawn kind

**Generation** (M2):
How far a sprite is from the world's first: 1 for one the world began with or the Cursor placed, and for a hatchling one more than its parents', or than the newer parent's when theirs differ. An egg has its generation from when it's laid.
_Avoid_: gen, age (that's ticks lived)

**Lineage** (M2):
A sprite's line by blood, as the family tree and the lineage colours show it: its parents and grandparents (its elders), its brothers and sisters, and its young and theirs. Code may call brothers and sisters siblings. It's wider than kin, which is what Slice 32 plans for a sprite itself to know.
_Avoid_: family (the household), relatives, ancestry

**Sex** (M2):
Male or female. Every sprite hatches male, and an adult male turns female for good once its feminiser is high enough. Only a female lays eggs.
_Avoid_: gender, mating type

**Feminiser** (M2):
The hormone that turns a male adult female. The starter genes raise it while the sprite sees no female adult and lower it fast while it sees one, so a male long without females turns, as clownfish do.
_Avoid_: sex hormone

**Longing** (M2):
The starter genes' hormone for whether a female wants to breed alone. They raise it over about an hour while the sprite sees or hears no male adult, and lower it fast while it does. Since Slice 32 her body allows it only after physiology's `breeding.alone_after`, an hour with no male adult seen or heard; a pack from before then gates it on longing reaching `breeds_alone` instead.
_Avoid_: loneliness (the drive for any sprite's company)

**Fertile** (M2):
Able to breed: grown up, its energy and hydration above `fed` (0.4), rested since its last egg (1,500 ticks), in a world with room for another sprite, and with room nearby. It's the body's say, not the genes'.
_Avoid_: ready, in season

**Room nearby** (M2, Slice 29b):
The body's rule, beside most sprites, that a grown sprite is fertile only where fewer than 4 other grown sprites, adult or old, a partner included, are within 10 tiles in a straight line. Young sprites and eggs don't count. Physiology's `breeding.room_nearby` sets the two numbers. A sprite whose body could breed but for it is **too crowded to breed**; one that the world's limit holds back, whatever is near it, is **world full**.
_Avoid_: crowdedness (the drive about sprites within 3 tiles), crowded (alone, which reads as that drive), personal space

**Wandering switched off** (M2, #318):
A grown sprite whose `always → Wander` instinct, as its brain takes it, pulls less than its act threshold, so it never wanders on its own: a copying error weakened the gene, two hidden flaws broke it, or copy and change lost it. It plays where it stands, drinks and rests, and stays near its partner. Room nearby is meant to keep such sprites from winning the race for room in a full world (#318), which Slice 29b's long run checks.
_Avoid_: homebody (#318's word for it), lazy, idle (a sprite with nothing to do stands by design)

**Arousal** (M2):
The drive to breed, which `physiology.ron` names as breeding's drive. The starter genes raise it fast while the sprite is fertile and drop it within a few ticks of its not being, or once it has bred, so others read it as fertility. It's a need, but only mating relieves it in what a sprite learns, and only about the partner. Whatever the genes, it works only between adults.
_Avoid_: broodiness (its first name; brooding is sitting on eggs), desire, libido

**Body language** (M2):
What a sprite sees of the sprite it attends to: its arousal (`target_arousal`) and whether it's of the other sex (`target_other_sex`). Brain inputs that read another's body are listed in `brain_io.ron` as `Target(...)` or `Differs(...)`. Slice 32 adds how healthy it looks (`target_condition`), how receptive it is (`target_receptivity`), and whether it's familiar, its partner or kin, read of a heard caller too.
_Avoid_: signals, display

**Mate** (M2):
The action of breeding with the sprite beside it. It works only between two fertile adults, a male and a female, once her receptivity is high enough: she lays an egg, its genes mixed from both, they split its cost, and they're partners. Mate is never offered toward a sprite that isn't an adult. The screen says "pair with".
_Avoid_: courtship (what comes before it), copulate

**Breed alone** (M2):
The action of a female laying an egg with no partner, its genes hers with one of her two copies of each doubled, its whole cost hers. Offered only to a grown sprite.
_Avoid_: parthenogenesis in what the player reads, cloning, self-mating

**Call** (M2, Slice 32):
An action with no target: the sprite calls once, at a little cost in energy, then stands a moment listening. A call travels like a wave, round rock and walls and through doorways, about 30 tiles over open ground, three times as far as a sprite sees, and each terrain says how much sound it soaks up. A hearer knows who called and where from, and can go there as on a trip. Sprites call to find a mate and to keep their family in touch. Planned (M3, Slice 34): the map draws the selected sprite's calls, and the calls it hears, as rings of sound rolling out round walls and through doorways, and a key shows every call (M3 design §8.5).
_Avoid_: cry, song, speech (that's M6's Speak)

**Courtship** (M2, Slice 32):
What comes before mating: two sprites get to know each other, then one dances for the other with the action **Court**, and the other, if it likes what it sees, dances back. Each dance raises receptivity. Court is never offered toward a sprite that isn't an adult.
_Avoid_: flirting, wooing, display (alone)

**Receptivity** (M2, Slice 32):
The hormone a female's body needs for Mate to lay an egg: at least physiology's `breeding.receptive`. The starter genes raise it a little when she's danced for, much more when she dances back, and it fades within about ten minutes.
_Avoid_: readiness (fertile is the body's ready), heat, oestrus

**Partner** (M2, Slice 32):
The sprite it had its latest egg with. In the starter genes partners court and mate only each other, an aroused one goes to the other, and one out of sight makes it anxious (see **Family**). A partner is freed when the other dies, or after an hour neither seeing nor hearing the other, and may pair again; one that died is still named until the sprite pairs again.
_Avoid_: mate (that's the action), spouse, bond

**Kin** (M2, Slice 32):
A sprite's parents, its young, and its brothers and sisters, the sprites with a parent in common. The starter genes weigh strongly against courting kin, but nothing forbids it.
_Avoid_: relatives, family (family is the household)

**Family** (M2, Slice 32):
A sprite's partner, its parents while it's young, its young, and its eggs: those anxiety draws it back to. An attention instinct can name whom it's about: its family, its kin, one kind of family, its partner, its parents or its young, or, since Slice 28, grown-ups or a parent showing it something. Then it draws the eye to them and not to others of their kind. A parent remembers where it last saw each egg of its own, and goes back there.
_Avoid_: clan, herd, kin (kin is by blood)

**Anxiety** (M2, Slice 32):
A drive and second-order need that rises while a sprite's family is out of sight, as the starter genes have it: fastest for an egg it left alone, then in a young sprite while no parent is in sight, then for its young or its partner, and slowest for an egg it left with the other parent by it. Finding the one it missed eases it, and hunger and thirst drain it. It's care: the starter's never comes from the breeding drive, and whatever the genes, the breeding drive draws no eye to a young sprite. Planned (M3, Slice 34): one of its family who dies out of sight is still missed, as if only away, until the sprite gives up on it after an hour out of sight, and then grieves (M3 design §8).
_Avoid_: fear (fear is of someone), worry, loneliness (that's for any company)

**Grief** (M2, Slice 32):
The starter genes' hormone for losing a partner, which holds arousal down for a while before the sprite may pair again. Planned (M3, Slice 34): felt for any kin a sprite sees die, an egg breaking included, and for one it gives up on after an hour missing, no longer only a lost partner. No sprite knows of a death it didn't see. It becomes a drive and second-order need, as anxiety is, with its own colour on the map: the starter genes grieve a partner, their own young or, while young, a parent most, and other kin lightly; it keeps the sprite from play and mating, draws it to its family, whose company eases it, and fades in about 20 minutes (M3 design §8).
_Avoid_: sadness, mourning (as a name for the hormone)

**Most sprites** (M2):
The most sprites a world holds, eggs included: the preset's `most_sprites` (100 in the default world). A full world has no fertile sprite. The Cursor may still place one past it.
_Avoid_: population cap, carrying capacity (food sets that)

**Die-out** (M2, #222):
A world dying out: no sprites and no eggs left in it. A fall it recovers from isn't one.

**Population record** (M2):
The counts the world keeps of its sprites by life stage, and of its eggs, over its whole run, for the population graph: one every 100 ticks from the first tick it runs, each with the oldest and newest generations among its sprites, and the eggs laid and hatched, the sprites spawned and the deaths since the one before. It's world state, so it's saved and replayed. Past 512 counts it keeps every other one and counts half as often, so it always covers the whole run (M2 design §14.1).
_Avoid_: history, statistics, census

**Record of the dead** (M2):
What the world keeps of each sprite that dies: its name, parents, generation, origin, when it was born and died, how, at what life stage and as which sex. It's world state, but nothing in the world reads it. It keeps the newest 10,000, forgetting the longest dead first, but never one the player named (M2 design §17.1). Planned (M3, Slice 34): where each died, too.
_Avoid_: graveyard, obituary, history

**Body** (M3, Slice 42):
What a dead sprite leaves where it died, as its species' data says: an object type of its own that fades after a while. The inspector shows who it was and how it died, and sprites can grip and drag it, but it isn't food. Planned (M3 design §16). A living sprite's body is its physiology.
_Avoid_: corpse, carcass, remains

**Lived long ago** (M2):
What the family tree says of a dead sprite the world no longer remembers: forgotten from the record of the dead, or dead before a save that had no record. The tree names it from its young and knows nothing more of it.
_Avoid_: unknown, lost

**Cause of death**:
What caused most of a sprite's recent injury, with the most recent counting most: starvation, dehydration, old age, or being hurt by an object type, such as a thornbush, or by another sprite. Planned (M3): an egg broken (Slice 40) or dying with its mother (Slice 41), each named in the egg's data.

**Hurt**:
Injury or pain that something did to a sprite: a thornbush it touched, another sprite's hit, or the Cursor's zap or shock. A zap or shock hurts without injuring. Starvation, dehydration and old age injure a sprite but don't hurt it.
_Avoid_: damage, pricked (a thornbush word; the data names things, the screen says "hurt")

**Limb** (M3, Slice 33):
A part of a sprite's body that can be hurt and heals: the head, the left arm, the right arm, the left leg and the right leg, each tracked on its own. A hurt arm grabs and throws worse; hurt legs walk slowly, and a sprite with both legs badly hurt crawls; a limb hurt badly enough is useless until it heals, and with no usable arm a sprite can't grab, grip, push, pull, throw or drop. A hurt head makes eating and drinking harder, but never kills: a sprite still dies when its injury reaches 1. None is lost for good in M3; limb loss is [#235](https://github.com/Keazra/terra-sprites/issues/235). A hurt lands on the limb the act used, as each thing's data says, and heals in about half an hour of play; a sore limb works about three quarters as well, so a limp fades as it heals. Planned (M3 design §7).
_Avoid_: appendage, extremity, organ (organs are a separate design, #31)

**Fumble** (M3, Slice 33):
A try that a sore head or arm misses, by the world's dice: a bite, a sip, a hit, a play or a grab. The thing stays as it was, ready for another try, and the sprite learns nothing about it from the miss, since the clumsiness was its own. A sore head misses about one bite in four. Planned (M3 design §7.4).
_Avoid_: failed try (failing is an outcome), slip

### Actions and movement

**Action**:
What a sprite is doing, such as wandering to a spot or resting, from when it starts until it ends. A sprite does one at a time.
_Avoid_: task, behaviour, activity

**Standing** (M2, Slice 29):
What a sprite does with no action running: since Slice 29 it acts only when something pulls on it at least its `act_threshold`, and otherwise stands where it is, still watching. One the Cursor leads or a shove slides isn't standing. Standing about builds boredom in the starter genes. Its doing line reads "Watching Bo #7", or "Standing" if its eye is on nothing, though an action that ended on the latest tick still shows how it ended. "Watching" there says where its eye is, on anything; learning from another sprite is **Watching**, below.
_Avoid_: idle, waiting (that's an action stuck behind another sprite), resting (that's an action)

**Verb**:
A kind of action a sprite can take: Approach, Eat, Drink, Hit, Play, Retreat, Rest or Wander. Mate and BreedAlone join in M2 (Slice 23), and Call and Court in Slice 32. Planned (M3 design §6.1): Grab, Throw and Drop join with hands (Slice 35); Touch takes Play's place, Play becoming a feeling, boredom eased (Slice 37); Grip, Push and Pull join in Slice 38; and Take in replaces Eat and Drink in Slice 46. Speak is reserved for M6 (Words).
_Avoid_: command (the player's commands are different), move

**Take in** (M3, Slice 46):
The one verb for eating and drinking, in place of Eat and Drink: each thing's data says what taking it in is called ("eat", "drink") and what it does, so a sprite goes to drink the water, never to eat it. Planned (M3 design §20).
_Avoid_: consume, ingest

**Holding** (M3, Slice 35):
A sprite keeping things in its arms after one or more Grabs: each arm is its own slot (tiny things stack in one arm up to the handful, a small thing takes one arm, a medium thing both). Holding is a state, not a verb, and the sprite feels it. On screen, a thing is **in its arms**. Moving while holding is carrying, which isn't the Cursor's Hold or Lead. A sprite's Grab, Throw and Drop are verbs; the Cursor's Grab, Hold and Throw are separate entries under The Cursor. Planned (M3 design §9).
_Avoid_: Carry (as a verb), inventory (that's the keeper's), inventory slot, grip (that's without lifting)

**Hand-over** (M3, Slice 43):
A Drop aimed at another sprite: the thing goes into its free arm, or at its feet. Both feel it, the giver that it gave and the other that it was given something. It's how families share food. Planned (M3 design §17).
_Avoid_: give (as a verb of its own), gift, trade

**Grip** (M3, Slice 38):
A sprite's hand on a thing without lifting it, so it can grip what it can't grab, such as another sprite. While gripping, a push moves the thing a tile straight away and the sprite steps after it; a pull is the reverse; a fixture doesn't move but does what its data says. Choosing anything but a push, a pull or a hit lets go. A gripped sprite struggles to get away, and breaks free by the world's dice. Planned (M3 design §12).
_Avoid_: hold (that's the Cursor's), grab (that lifts), grapple

**Outcome**:
How an action went: still walking, applied, blocked, failed, timed out or too full.
_Avoid_: result, status

**Too full** (M2, Slice 29):
How an Eat or a Drink ends when the sprite is full. The food or water stays where it was, and unlike a fruitless try it teaches nothing. The Body tab reads "Too full to eat the berry", "Too full to eat from the berry bush" or "Too full to drink".
_Avoid_: refused (the Cursor's word for a click that didn't apply), failed

**Squish** (M3, Slice 36):
What eating a fallen berry where it lies can do: the berry is destroyed, the sprite isn't fed, and the berry rots at once where it lay. It starts at 1 in 2, from each food's data, so picking a berry up before eating it wins as sprites learn. Planned (M3 design §10).
_Avoid_: miss, fail, spill

**Timeout**:
The longest any action may last. An action still going at the timeout ends as timed out. A trip has as long as its walk takes on top.

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
What a sprite learns about someone that hurt it by its own doing, such as a sprite that hit it. A frightening sprite catches its eye and makes it back away while it's near; fear never makes it attack. On screen: "Sprite #7 is frightening". Planned (M3, Slice 38): fear of a sprite eases while it stays near without hurting, as fear of the Cursor does.
_Avoid_: bad (bad is about touching), wariness (a passing mood), threat

**Individual**:
A particular sprite that another remembers, with what it has learned about it: its worth, how frightening it is, and how familiar it is. Seeing a sprite is enough to remember it (Slice 32), but one known only by sight is judged as sprites in general are. Individuals are learned fast, fade, and are forgotten once faded or dead. On the Brain tab each one is nested under Sprites, by the name the player gave it, or by its number.
_Avoid_: acquaintance, contact, relationship

**Category summary**:
What a sprite thinks of the things in a category it hasn't met: the average of those it knows in it, which counts for nothing while it knows only one. So one thornbush doesn't make every bush bad. On the Brain tab the heading is the category and the line under it drops the name: "bad".
_Avoid_: stereotype, prior

**Sprites in general**:
The sprite category's summary: what a sprite thinks of sprites it doesn't know, from the individuals it remembers. So one bully doesn't make it shy of everyone, but three might. On the Brain tab it is the first step in under Sprites, and each remembered sprite is nested under that.
_Avoid_: the sprite kind (on screen), stereotype

**Habit**:
What a sprite has learned about doing one verb to one object type, or to sprites, such as "eating balls doesn't work" or "hitting sprites is bad". Planned (M3, Slice 36): habits are kept apart by whether the thing was in the sprite's arms, so "eating berries off the ground" and "eating berries in its arms" are two.
_Avoid_: skill (a later design, #48), reflex

**Taught habit**:
The part of a habit the Cursor taught: a Reward or Correct for what the sprite just tried. It is kept apart from the habit the sprite learns by itself, so what it learns afterwards doesn't wash it out, and it fades as slowly as a bad lesson, so in effect it lasts a life. A sprite acts on the two together, and the Brain tab shows them together as its habit. Since Slice 28 a habit learned by watching is kept apart the same way, and since Slice 29 apart from the taught one too.
_Avoid_: training (that's what the player does), conditioning

**Watching** (M2, Slice 28):
Learning from another sprite: when a sprite's eye is on another that uses something, or has something done to it, it takes on a share of what that sprite knows about it, only ever up to the other's view, and remembers a place it sees the other eased at. Since Slice 29, what it takes on of a thing's badness and habits is kept apart from what it found out itself, settles either way towards what the other found out at first hand, and is never passed on. Of what a thing is worth, it takes on only that for the other's motive. It's about things and whom to fear, never whom to like, and never anything about mating. In the starter genes every sprite watches, the young most, looking to the grown-ups, their parents more, and most of all to a parent doing something that would show them a lesson they haven't learned.
_Avoid_: imitation, copying (copying the act itself is for later), observing (the observer is the agent that reads baseline reports)

**Thing touched**:
What a feeling is about: the thing the sprite tried a verb on a moment ago, or the particular sprite. Worth is learned about it and nothing else. A sprite that hit it isn't one: that teaches fear of the sprite instead. For the Cursor's touch, "a moment ago" stretches to its reach back.
_Avoid_: target (for this), culprit

**Fruitless try**:
A try at a verb on a thing that has no rule for it, or whose rule can't be met, such as eating a ball or a bare bush, mating with a sprite that isn't fertile or is of the same sex, or a male breeding alone. Nothing happens, and the sprite feels it.
_Avoid_: fizzle, failed attempt (failed is an outcome)

**Motive**:
The need whose instinct did most to make a sprite choose a verb. A fruitless try disappoints it, and a sprite watching takes on what the thing tried is worth for it alone.
_Avoid_: reason, goal

**Familiarity**:
How well a sprite knows an object type, or sprites, from having attended to it. Its opposite is **novelty**: how new the thing still is to the sprite. An apple is new to a sprite that knows only berries. Since Slice 32 a sprite has a familiarity with each individual it remembers too, from time spent seeing it and from playing together, which fades slowly; strangers are those it hardly knows. The brain reads it of the sprite it attends to as `target_familiar`.
_Avoid_: knowledge, acquaintance

**Curiosity**:
A sprite's pull towards things that are new to it. Being hurt when it investigates teaches it that new things are bad. Planned (M3, Slice 37): an instinct, *looking at something new to it: touch it*, that quietens as the thing grows familiar; and novelty becomes a sense genes can read ([#262](https://github.com/Keazra/terra-sprites/issues/262)).
_Avoid_: exploration (exploring is how sure its choices are)

**Play** (M3, Slice 37):
What a sprite feels when something it does eases its boredom as a toy does: each thing's data says which acts on it are play, by sending the `played` feeling, so the game's code never knows what a toy is. Until Slice 37, Play is a verb; from it, Play reads as Touch in old genomes, saves and mods. Planned (M3 design §11).
_Avoid_: game, fun

**Social play** (M3, Slice 37):
Touching another sprite, or a ball one sprite sent reaching another: both feel it, get to know each other, and it eases loneliness, the one reached a little less. Planned (M3 design §11.6).
_Avoid_: roughhousing (that's a later design, #46)

**Empathy** (M3, Slice 37):
The pang a sprite feels when its own act hurts another sprite, which the starter genes turn into a little punishment, unless it's hitting back at one that just hurt it or that it fears. How strongly it's felt is a gene. Planned (M3 design §11.5).
_Avoid_: guilt, remorse, pity

**Wariness**:
A passing mood, raised by a run of hurts, that makes a sprite less curious for a while.
_Avoid_: fear (fear is of someone in particular)

**Memory**:
What a sprite has learned from experience: the worth of things and of individuals, its fears, its habits, and its remembered places.
_Avoid_: learned links, knowledge

**Remembered place** (M2):
Where something that stays put, such as water or a bush, eased one of a sprite's needs. A sprite holds a few, and only a few of each kind; places of the same kind close together count as one, so a pond is one place. Each fades unless a visit tops it up, and one the sprite sees gone is forgotten. How well it's remembered is its **recall**. With nothing of its kind in sight, a remembered place draws the sprite's eye like a thing at the edge of sight. The Brain tab lists them as places, and the number on the line is the recall alone.
_Avoid_: landmark, waypoint, place (alone, outside the Brain tab: that's putting a thing down with the Cursor)

**Trip** (M2):
An action aimed at a place out of sight that the sprite knows of: a remembered place, the tile a call came from, or where it last saw an egg of its own. The sprite knows the way: setting off, it plans a path as far as the place's distance plus its sense radius, keeps to it, and has as long as the walk takes on top of the timeout. Once the place, the caller or the egg comes into sight, the trip is an ordinary action. The screen says "the water it remembers".
_Avoid_: journey, expedition, recall (recall is how well the place is remembered)

**Lesson**:
A worth, fear or habit that has reached half a point from nothing. The event log announces each lesson once, as good or bad, or frightening.
_Avoid_: milestone (on screen)

### The screen

**Title screen** (M2):
The screen the game opens on, before any world is running: a new random world moving behind the title and its menu (Continue, New world, Load, Help, Quit). Flags that make or load a world skip it, and `t` at a world's quit prompt goes back to it (M2 design §8).
_Avoid_: main menu, start screen, splash screen

**Opening scene** (M2):
How the title screen starts: in the dark, the Cursor's light finds a sprite and the world fills in around it, then the title and menu appear below the Cursor, which keeps its sprite, still, for as long as the title screen is open. Any key skips it (M2 design §8.1).
_Avoid_: intro, splash, cutscene

**Story mode** (M2):
The game as it's meant to be played, at 4× speed at most, so a life never feels rushed. It's the mode New world picks unless the player picks Lab, and a world keeps its mode for its whole life; a save from before the modes is a Story world. Once the entire ship is reactivated, the player may accept lifting the cap for that world (M2 design §16; M4 design §2.2, §2.3).
_Avoid_: full game, play mode, campaign

**Lab mode** (M2):
The game for experiments: every speed, up to Max, and fast-forward. The top bar says "lab" after the speed (M2 design §16). What it allows on the ship is in the M4 design, §2.2. It isn't a lab scenario, which is a test file.
_Avoid_: free mode, sandbox, creative mode

**Fast-forward** (M2):
Running a Lab world with no screen, from the command line, for hours of play at 1× as fast as the computer allows, then saving it for the game to open, with a population table and a genes table beside it. Its report's generation is how many generations the longest line of descent has run since it started (M2 design §16.3).
_Avoid_: headless mode, turbo, skip ahead

**Map view**:
The panel that shows the map.
_Avoid_: map (for the panel)

**Viewport**:
The part of the map currently shown in the map view. It can go a quarter of the map view past the wall, where the glass and space show; a map small enough to fit with its glass sits in the middle of it, with space all round (Slice 31, #180).
_Avoid_: camera

**Glass**:
How the wall is drawn where it faces space: a line of light cyan one tile outside the map (Slice 31). It isn't a tile, so nothing on the map reaches it. On the ship (#181), it's drawn one tile outside a sector, over a tile of space.
_Avoid_: window, border (that's the map view's)

**Track**:
The viewport following the selected sprite, or egg, so it stays in the middle of the map view. `T` turns it on or off; scrolling by hand turns it off. It follows whichever is selected, and does nothing while nothing is, or while the Cursor holds the selected egg.
_Avoid_: follow (that's the Cursor fixed on a sprite, with `F`), lock on

**Cursor**:
The player's hard-light projection into the terrarium: to the sprites the player is an advanced creature, and this is how they reach in. On the ship (#181), the player is its keeper, and the Cursor is cast by the ship's projectors. It shows on the map as a 3×3 grid that follows the pointer, unless it follows a sprite, and a click acts as the cursor mode says. Sprites see it only where the player has made it **visible**; otherwise its touch is a feeling from nowhere. It holds an item or leads a sprite, one thing at a time.
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
The sprite the inspector shows, chosen by clicking it or with `Tab`, or an egg, chosen by clicking it; when the egg hatches, the hatchling is selected. A dead sprite can be chosen on the family tree. It's the inspector's: the Cursor can follow another sprite.
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
The side panel of tabs about the selected sprite (Body, Brain, Chem, Genome) or the world (World). With an egg selected, the sprite tabs show its card: when it hatches and its parents.
_Avoid_: sidebar, details

**Observed list**:
What the player has watched the selected sprite finish since selecting it, at the end of the Body tab. It only grows while the sprite is selected.
_Avoid_: history, log (the event log is different)

**Sparkline** (#272):
The eight shades at the end of a drive's line on the Body tab, or of a chemical's on the Chem tab, showing how its level moved over about the last 50 ticks, oldest on the left. Planned: #328 builds them, kept by the screen for every sprite and never saved.
_Avoid_: history graph, graph (that's the population graph), trend (the arrow beside it is one tick's change)

**Detail view**:
A view the player switches on and off that shows the exact workings behind what the screen describes in words, such as a sprite's exact verb and destination.
_Avoid_: debug mode (it's in every build)

**Debug overlay**:
The small box `F3` shows in the map view's top-right corner, over any menu or overlay in a world: how the game is running on this computer, the ticks a second it achieved against the speed asked for, the simulation's and the drawing's time each frame, and whether the terminal reports key releases. It's in every build, for a screenshot when the game feels slow. Unlike the help screen and the other overlays, it covers only a corner.
_Avoid_: debug mode (it's in every build), FPS counter, performance panel

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
What colours a sprite on the map: its strongest drive once one is above half (each drive's colour is the theme's), plain, its own colour, or lineage (M2), which marks the selected sprite's lineage and grays the rest. `b` goes to the next.
_Avoid_: tint, palette

**Help screen**:
The overlay `?` opens: every key, grouped, the colour and emote legends, and the game's folder for its files.
_Avoid_: manual, cheat sheet

**Sprite list**:
The overlay `l` opens: every sprite, one to a row, with its age, life stage, strongest drive, generation and what it's doing, sorted by number, name, age or drive. `/` filters it to the rows with a column that has the words typed, from their start. Choosing one selects it and centres the viewport on it.
_Avoid_: roster, census

**Population graph** (M2):
The overlay `p` opens: the world's population over its whole run, each column split by the share of each life stage and of eggs. A moment picked with the arrows or the pointer is read under it: how many sprites, each stage and the eggs with their shares, and what was laid, hatched, spawned and died since the column before. A small copy heads the World tab, showing how many, with a dotted line at most sprites (M2 design §14).
_Avoid_: population chart, history graph, census

**Family tree** (M2):
The overlay `k` opens: the selected sprite's grandparents, parents, young, and brothers and sisters, living or dead, with lines joining the generations. The arrows or the pointer choose a sprite on it, which blinks, and `Enter` or a click walks the tree to it (M2 design §17).
_Avoid_: pedigree, genealogy, lineage view

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
To take hold of what's under the Cursor: a sprite is led, an item picked up. A sprite on the tile comes before an item, and a fixture can't be grabbed. A sprite's Grab is a verb under #164.
_Avoid_: lift, pick up (for a sprite)

**Lead**:
To hold a sprite through the Cursor without lifting it: it stays on the map and walks after the Cursor at its own pace, choosing nothing for itself, until the player **lets go**. Sprites are led, never lifted.
_Avoid_: drag, carry (for the Cursor), pick up

**Leash**:
How far the Cursor may go from a sprite it leads, 5 tiles in a square, and the flashing dotted line that shows it, from the Cursor to the sprite.
_Avoid_: lead (that's the verb), rope, tether

**Hold**:
What the Cursor does with an item it has picked up: the item leaves the map, its life going on, until the Cursor **puts it down** on a tile. A sprite's holding (M3) is a different state, and so is its grip.
_Avoid_: carry (for the Cursor; a sprite's carrying is moving while holding), in hand

**Throw**:
To let go of a held item with a push, so it rolls away like a kicked ball, as far and in the direction the player **aims**. A sprite's Throw is a verb under #164.
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
To make a new object on a tile through the Cursor, at the start of its first stage. Only the types whose data offers them can be placed, and their data says what the tile must meet; the built-in ones ask nothing, so a placed bush may wall things off. On the ship, Story mode takes it from the inventory instead (#181).
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
The Cursor's bad touch, which hurts the sprite without injuring it and raises its punishment chemical: a **zap**, or amplified, a **shock**. A sprite's own **correcting** of its young is its kin's version (M3).
_Avoid_: slap, punish, negative

**Correcting** (M3, Slice 44):
A parent stepping in when its young tries something the parent has learned is bad: the young stops, and learns from it as from the Cursor's Correct, as a lesson of its own. It's a leaning in the genes, it never injures, and it never teaches anything about mating. Planned (M3 design §18).
_Avoid_: punishing, scolding, Correct (that's the Cursor's)

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

**Clip** (#241):
A stretch of the game drawn by the renderer into a picture that moves, a GIF, an MP4 or an animated SVG: the game's own screens, from a seed or a replay, in Cascadia Mono and the Campbell colours. The same command makes the same file.
_Avoid_: recording (that's the world keeping one), video (an MP4 is one kind), screen capture

**Withdrawn Cursor** (#241):
The Cursor taken out of a picture with no player in it, such as a clip: out of reach, as over space, with no pointer anywhere, so it isn't drawn and the status line says nothing of what it's over (M2 design §11.4).
_Avoid_: hidden Cursor (any Cursor out of reach is hidden)

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
The default world run for many seeds with no player, counting the young hatched that grow up, the deaths by life stage, and what killed whom (A4).
_Avoid_: soak, ecological soak

**Unattended** (M2, Slice 29):
Said of a young sprite with nobody to learn from: no player and no grown sprite.
_Avoid_: orphan (an orphan's parents are gone, but it may still watch other grown-ups), left alone (that's a world with no player)

**Unattended run** (M2, Slice 29):
The default world on seeds 1–10 as it generates, with its first sprites newborns, placed as the Cursor places them and spread over the whole map, and nobody else, until each has grown up or died. A4 and G5 ask that 10% to 30% grow up, in the middle seed.
_Avoid_: newborn test, orphan run

**Grown-up death**:
The death of a sprite that had grown up, adult or old, judged by life stage. The world's first sprites start as adults, so their deaths count from the start. A newborn dying of thirst is natural, but an adult dying of hunger or thirst means it can't fend for itself (A4); old sprites' are reported apart.
_Avoid_: adult death (old sprites' count too); a death after tick 10,000 (A4 counted by tick until Slice 29)

**Long run** (M2, Slice 29):
The default world left alone for 220 hours on seeds 1–10, on the owner's PC: the `childhood` example. It reports Slice 29's bars, G1's (whether each seed's world lasts), the starter genes the living lack, lone newborns given the end's genes, and G4: gene values moved, over the seeds that last, and the race.
_Avoid_: soak (that has the Cursor's commands), viability run (that's 50,000 ticks)

**Soak**:
A run of a million ticks with a random script of the Cursor's commands, looking only for panics and broken invariants (A7).
_Avoid_: stress test; a soak for any long run without commands (that's a viability run, or the long run)

**Baseline run**:
The measurements taken on one commit of `main`, so that each commit can be compared with the last: the viability run, the soak, and the A1–A3 lessons' numbers.
_Avoid_: daily report, nightly soak, health check

**Baseline report**:
A baseline run's numbers, for one commit.

**Broken**:
What a baseline report calls a seed that panicked or broke an invariant. A criterion that isn't met yet isn't broken.
_Avoid_: crash (that's a shoved sprite's), failed

**Gene warning** (#274):
A line in a baseline report, or at the end of a fast-forward, saying the species' genes have gone somewhere worth a look: more than half the hatched sprites alive lack a needs gene, or a gene value's middle has run nearly to the end of its range. It's no verdict: it asks for a look, and the run is judged as before.
_Avoid_: canary (the idea's name in the owner's notes), alarm, failure, broken (that's a seed's)

**Briefing**:
The observer's plain-language account of a baseline report: what was tested, what changed since the last one, and what might explain it.
_Avoid_: summary, analysis

**Observer**:
The AI agent that reads a baseline report, and the code and commits behind it, and writes the briefing. It changes nothing itself.
_Avoid_: monitor, watcher

### The ship (M4)

**Ship** (#181):
The terrarium seen for what it is: Terra, an ark flying through space, made of sectors joined by doors. Its sprites are the Terra Sprites.
_Avoid_: station, vessel, level

**Keeper** (#181):
The ship's AI, which the player is. Its builders are gone and it woke with its memory damaged, so half its own ship is a mystery to it. It reaches into the terrarium through the Cursor; to the sprites, it's an advanced creature.
_Avoid_: captain, overseer

**Inventory** (#181):
The things the keeper has put away, out of the world, to place again later; devices around the ship make the things that fill it. Story mode places from it (M4 design §2.2).
_Avoid_: stock, store (that's a bay), holding (that's a sprite's)

**Sector** (#181):
One part of the ship, with walls of its own, which the map view shows on its own: a biome or a bay. Every tile of the ship is in one sector, walls included, or is space. Sprites cross between sectors through doors. Every open sector lives at once, on screen or not; a sealed one sleeps until its door opens.
_Avoid_: level, area, screen, zone, module, compartment

**Active sector** (#181):
The sector the map view shows, bright, with its neighbours dimmed past its walls. A click on a dimmed tile makes that sector the active one. It belongs to the screen, not the world.
_Avoid_: current sector, focused sector

**Biome** (#181):
A large sector generated from the seed, to a recipe in the game's data that says its size and its mix of terrain, plants and set pieces, such as the Meadow or the Marsh. Its staging area is part of it.
_Avoid_: terrain (a tile's ground), biosphere, environment

**Bay** (#181):
A small sector with a job in running the ship, drawn by hand tile by tile in the game's data, such as Hydroponics Bay or the Reservoir. Bays are what #26 called zones.
_Avoid_: room, zone, lab, block

**Main deck** (#181):
The hall along the ship's length that every sector opens onto, and that each biome's staging area juts in from. It's three bays, the aft, middle and fore deck, joined by doors.
_Avoid_: hub, concourse, corridor

**Staging area** (#181):
The part of a biome that the ship built to make it work: plating and hull jutting in from the main deck, whose devices keep the biome's climate, water and fauna to its recipe. The biome's door onto the main deck opens into it.
_Avoid_: annex, outpost

**Door** (#181):
A way through the wall between two sectors: an open doorway, or a sealed door that stays shut until it's opened, and never closes again.
_Avoid_: portal, gate

**Panel** (#181):
The device on each side of a sealed door that opens it: a plate to touch, a button to push or a lever to pull, as its data says. One sprite working it once opens the door for good.
_Avoid_: lock, keypad

**Ship plan** (#181):
The ship's outline, shown over the map view, with each open sector's name and how many sprites are in it; picking a sector takes the view there.
_Avoid_: minimap, map (the map is the world's tiles)

**Builders' log** (#181):
Lines the ship's builders left, one or two per sector in its recipe's data, which the keeper remembers when the sector's door first opens. They hint at what the ship is for, and never say outright what a device does.
_Avoid_: lore, journal

**Plating** (#181):
The ship's metal floor: a terrain that sprites walk on and nothing grows on.
_Avoid_: deck (that's the main deck), floor

**Hull** (#181):
The ship's frame: a solid terrain, like rock, which the walls between sectors are made of.

**Prow** (#181):
The ship's front: a blade of iron like a scythe. Unlike the rest of the ship's outer wall, it isn't glass.
_Avoid_: bow, nose

**Set piece** (#181):
A small hand-made piece of the ship, a few tiles big, that a sector's recipe places into its land: a rib of hull, a causeway, a pump house.
_Avoid_: prefab, stamp, vault

**Device**:
An object of the ship's that does a job, such as an incubator, a pump, or one whose job nobody remembers. It's a fixture, and what it does is in its data: it may change the land or affect sprites near it, and the game never says what it's for. Some can be activated, and some have a switch that a sprite or the keeper can turn off and on again. A device never wears out or breaks.
_Avoid_: machine, machinery, building

**Space** (#180):
What lies outside the ship: stars going by. It's scenery, so sprites can't sense it and the Cursor can't reach into it. Today it's past the map; on the ship (#181), where the map is the rectangle around the whole ship, it's also the map's tiles in no sector.
_Avoid_: void, sky
