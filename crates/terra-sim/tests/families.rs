//! Families (M2 design §15.6): a sprite senses its family out of sight, an
//! egg left alone by what it last saw, and finding the one it missed; an
//! attention instinct can be about whom a thing is to the sprite, its family
//! or one kind of it, so a young sprite looks to its parent before a nearer
//! stranger, and a parent to its own egg before a nearer one; the brain
//! reads whether what it attends to is family; a parent goes back to where
//! it last saw its egg, and forgets the place once it finds the egg gone.

mod common;
use common::{ALONE, FEMALE, RECEPTIVE, at, builtin, hatch, id_at, laid, lone, world_in};

use terra_sim::{
    ActionView, Command, DataPack, EntityId, EventKind, Outcome, Pos, Progress, Target, Verb, World,
};

/// The hormones the family senses are copied into by [`SENSES`].
const EGG_AWAY: &str = "h3";
const EGG_ALONE: &str = "h4";
const YOUNG_AWAY: &str = "h5";
const PARENT_AWAY: &str = "h6";
const PARTNER_AWAY: &str = "h7";
const FOUND: &str = "h8";

/// Copies each family sense into a hormone every tick, so a test can read
/// it: 1 while the sense is on, and half or less a tick after it's off.
const SENSES: &str = r#"
    Emitter(locus: Locus("egg_away"), mode: Level, gain: 4.0, chem: "h3"),
    Emitter(locus: Locus("egg_alone"), mode: Level, gain: 4.0, chem: "h4"),
    Emitter(locus: Locus("young_away"), mode: Level, gain: 4.0, chem: "h5"),
    Emitter(locus: Locus("parent_away"), mode: Level, gain: 4.0, chem: "h6"),
    Emitter(locus: Locus("partner_away"), mode: Level, gain: 4.0, chem: "h7"),
    Emitter(locus: Locus("found_family"), mode: Level, gain: 4.0, chem: "h8"),
    HalfLife(chem: "h3", ticks: 1),
    HalfLife(chem: "h4", ticks: 1),
    HalfLife(chem: "h5", ticks: 1),
    HalfLife(chem: "h6", ticks: 1),
    HalfLife(chem: "h7", ticks: 1),
    HalfLife(chem: "h8", ticks: 1),
"#;

/// Whether the sense copied into `hormone` is on in the sprite `id`.
fn on(world: &World, id: EntityId, hormone: &str) -> bool {
    world.sprite(id).expect("the sprite").chemical(hormone) == Some(1.0)
}

/// Mates with the sprite it sees until it has a partner, then rests.
const PAIR: &str = r#"
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("paired", true), ("attended_sprite", false)], verb: Mate, weight: 1.0),
    Instinct(inputs: [("paired", false)], verb: Rest, weight: 1.0),
    BrainParam(param: "tau_base", value: 0.02),
"#;

/// Rests, and does nothing else.
const RESTER: &str = r#"
    Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),
    BrainParam(param: "tau_base", value: 0.02),
"#;

/// A field of grass `width` by `height` tiles.
fn field(width: usize, height: usize) -> Vec<String> {
    (0..height).map(|_| ".".repeat(width)).collect()
}

/// Where the egg `egg` lies.
fn egg_at(world: &World, egg: EntityId) -> Pos {
    world
        .egg(egg)
        .and_then(|e| e.pos())
        .expect("the egg on the map")
}

/// The Cursor leads the sprite `id` to `tile`, and lets it go there.
fn lead(world: &mut World, id: EntityId, tile: Pos) {
    lead_watching(world, id, tile, |_| {});
}

/// The same, calling `watch` with the world after each tick.
fn lead_watching(world: &mut World, id: EntityId, tile: Pos, mut watch: impl FnMut(&World)) {
    world.submit(Command::TakeHold { sprite: id });
    world.submit(Command::MoveCursor { tile });
    for _ in 0..400 {
        world.step();
        watch(world);
        if world.sprite(id).expect("the led sprite").pos() == tile {
            world.submit(Command::LetGo);
            world.step();
            watch(world);
            return;
        }
    }
    panic!("it never got to {tile:?}");
}

/// Breeds alone at once, and goes over to whatever its eye is drawn to.
/// Its egg draws its eye while it's out of sight, as in the starter genes.
const MOTHER: &str = r#"
    AttentionInstinct(input: "egg_away", category: "egg", whom: Family, weight: 1.0),
    Instinct(inputs: [("always", false)], verb: Approach, weight: 1.0),
    BrainParam(param: "tau_base", value: 0.02),
"#;

/// A mother, out of sight of her egg, a lone egg laid in `data`: the world,
/// the mother, the egg and where it lies.
fn mother_led_away(data: DataPack, rows: &[&str], to: Pos) -> (World, EntityId, EntityId, Pos) {
    let genes = format!("{MOTHER}{ALONE}{FEMALE}");
    let mut world = world_in(data, rows, &[(at(2, 1), &genes)]);
    let mother = id_at(&world, at(2, 1));
    let egg = laid(&mut world, 100).expect("an egg").egg;
    let tile = egg_at(&world, egg);
    lead(&mut world, mother, to);
    let sprite = world.sprite(mother).unwrap();
    assert!(!sprite.sees(tile), "led out of sight of its egg");
    (world, mother, egg, tile)
}

#[test]
fn a_parent_goes_back_to_where_it_last_saw_its_egg() {
    // M2 design §15.6: the egg out of sight is offered like a remembered
    // place, and going to it is a trip; once in sight it's an ordinary
    // action aimed at the egg.
    let rows = field(40, 3);
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let (mut world, mother, egg, tile) = mother_led_away(lone(), &rows, at(36, 1));
    let mut by_memory = false;
    let mut came = false;
    for _ in 0..300 {
        world.step();
        let sprite = world.sprite(mother).expect("the mother");
        let action = sprite.action().expect("an action");
        if action.egg_at == Some(tile) {
            assert_eq!(action.verb, Verb::Approach);
            assert!(action.remembered, "a trip");
            by_memory = true;
        }
        let beside = sprite.pos().x.abs_diff(tile.x) <= 1 && sprite.pos().y.abs_diff(tile.y) <= 1;
        if beside && action.progress == Progress::Ended(Outcome::Applied) {
            came = true;
            break;
        }
    }
    assert!(by_memory, "it set off from memory");
    assert!(came, "it came back to its egg");
    assert!(world.egg(egg).is_some(), "the egg hasn't hatched yet");
}

#[test]
fn a_parent_that_finds_its_egg_gone_forgets_where_it_was() {
    // M2 design §15.6: the Cursor moves the egg out of sight of where it
    // was, and the mother, got there, has lost it.
    let rows = field(40, 30);
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let (mut world, mother, egg, tile) = mother_led_away(lone(), &rows, at(36, 1));
    world.submit(Command::PickUp { item: egg });
    world.submit(Command::PutDown { tile: at(2, 28) });
    world.step();
    let mut got_there = false;
    for _ in 0..300 {
        world.step();
        if world.sprite(mother).unwrap().pos() == tile {
            got_there = true;
            break;
        }
        let at = world.sprite(mother).unwrap().pos();
        let sees = world.sprite(mother).unwrap().sees(egg_at(&world, egg));
        assert!(!sees, "it saw the moved egg on the way, at {at:?}");
    }
    assert!(got_there, "it went where it last saw the egg");
    for _ in 0..100 {
        world.step();
        let action = world.sprite(mother).unwrap().action();
        let back = action.is_some_and(|action| action.egg_at == Some(tile));
        assert!(!back, "it forgot where the egg was");
    }
}

#[test]
fn a_parent_s_eye_goes_to_its_own_egg_before_a_nearer_one() {
    // M2 design §15.6: the egg category offers a parent its own egg, not
    // only the nearest.
    let rows = field(30, 3);
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let genes = format!(
        r#"
        AttentionInstinct(input: "always", category: "egg", whom: Family, weight: 1.0),
        BrainParam(param: "tau_base", value: 0.02),
        {ALONE}{FEMALE}"#
    );
    let mut world = world_in(lone(), &rows, &[(at(2, 1), &genes), (at(27, 1), &genes)]);
    let (mother, other) = (id_at(&world, at(2, 1)), id_at(&world, at(27, 1)));
    let mut eggs = Vec::new();
    for _ in 0..200 {
        for event in world.step() {
            if let EventKind::Laid { egg, by, .. } = event.kind {
                eggs.push((by, egg));
            }
        }
        if eggs.len() == 2 {
            break;
        }
    }
    let own = eggs.iter().find(|(by, _)| *by == mother).unwrap().1;
    let theirs = eggs.iter().find(|(by, _)| *by == other).unwrap().1;
    // Her own egg 6 tiles off, theirs beside her, and the other mother out
    // of sight.
    for (egg, tile) in [(own, at(8, 1)), (theirs, at(3, 2))] {
        world.submit(Command::PickUp { item: egg });
        world.submit(Command::PutDown { tile });
        world.step();
    }
    world.step();
    assert_eq!(world.sprite(mother).unwrap().attending_to(), Some(at(8, 1)));
}

/// Where a young sprite looks with an attention instinct about its parents
/// of `weight`: its mother 5 tiles off, or a stranger beside it. The tile it
/// attends to, its mother's and the stranger's.
fn young_looks(weight: f32) -> (Option<Pos>, Pos, Pos) {
    let rows = field(14, 5);
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let genes = format!(
        r#"
        AttentionInstinct(input: "always", category: "sprite", whom: Parent, weight: {weight:?}),
        BrainParam(param: "tau_base", value: 0.02),
        BrainParam(param: "curiosity", value: 0.0),
        {ALONE}{FEMALE}"#
    );
    let rester = r#"Instinct(inputs: [("always", false)], verb: Rest, weight: 1.0),"#;
    let mut world = world_in(lone(), &rows, &[(at(1, 2), &genes), (at(12, 2), rester)]);
    let (mother, stranger) = (id_at(&world, at(1, 2)), id_at(&world, at(12, 2)));
    let egg = laid(&mut world, 100).expect("an egg").egg;
    let pos = egg_at(&world, egg);
    lead(&mut world, mother, at(pos.x + 5, pos.y));
    let beside = if pos.y < 4 {
        at(pos.x, pos.y + 1)
    } else {
        at(pos.x, pos.y - 1)
    };
    lead(&mut world, stranger, beside);
    hatch(&mut world, egg);
    world.step();
    let young = world.sprite(egg).expect("the young one");
    assert_eq!(young.parents(), [mother]);
    let (mother, stranger) = (
        world.sprite(mother).unwrap().pos(),
        world.sprite(stranger).unwrap().pos(),
    );
    let pos = young.pos();
    let far = |p: Pos| p.x.abs_diff(pos.x).max(p.y.abs_diff(pos.y));
    assert!(far(stranger) < far(mother), "the stranger is nearer");
    assert!(young.sees(mother), "its mother is in sight");
    (young.attending_to(), mother, stranger)
}

#[test]
fn a_young_sprite_s_eye_goes_to_its_parent_before_a_nearer_stranger() {
    // M2 design §15.6: an attention instinct about its parents draws the
    // eye to them, or, negative, away from them, and does nothing for the
    // stranger.
    let (attends, mother, _) = young_looks(1.0);
    assert_eq!(attends, Some(mother));
    let (attends, _, stranger) = young_looks(-1.0);
    assert_eq!(attends, Some(stranger));
}

#[test]
fn eggs_remembered_saved_and_loaded_are_the_same_world() {
    let rows = field(40, 3);
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let (world, _, _, _) = mother_led_away(lone(), &rows, at(36, 1));
    let loaded = World::load(&world.save()).expect("the save loads");
    assert_eq!(loaded.state_hash(), world.state_hash());
}

/// A pair, she of `hers` and he of `his`, each also of [`PAIR`], side by
/// side in a field 40 tiles by 3, and a sprite of `third` out of their
/// sight, once they've had an egg and are partners: the world, her, him,
/// the third sprite and where the egg lies.
fn partners(hers: &str, his: &str, third: &str) -> (World, EntityId, EntityId, EntityId, Pos) {
    let rows = field(40, 3);
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let (hers, his) = (
        format!("{PAIR}{FEMALE}{RECEPTIVE}{hers}"),
        format!("{PAIR}{his}"),
    );
    let sprites = [
        (at(20, 1), hers.as_str()),
        (at(21, 1), &his),
        (at(38, 1), third),
    ];
    let mut world = world_in(builtin(), &rows, &sprites);
    let (she, he, third) = (
        id_at(&world, at(20, 1)),
        id_at(&world, at(21, 1)),
        id_at(&world, at(38, 1)),
    );
    let laying = laid(&mut world, 300).expect("an egg");
    assert_eq!((laying.by, laying.with), (she, Some(he)));
    assert_eq!(world.sprite(she).unwrap().partner(), Some((he, false)));
    let egg = egg_at(&world, laying.egg);
    (world, she, he, third, egg)
}

#[test]
fn a_parent_leaves_its_egg_alone_by_whether_it_saw_another_parent_by_it() {
    // M2 design §15.6: what the parent last saw of the egg: another parent
    // in its own sight, which saw the egg too, or none.
    let (mut world, she, he, _, egg) = partners(SENSES, "", RESTER);
    let row = |x: u16| at(x, egg.y);
    // He stays between the egg and her, so as she leaves, the last she
    // sees of the egg, she sees him by it.
    lead(&mut world, he, row(egg.x - 2));
    lead(&mut world, she, row(egg.x - 15));
    assert!(on(&world, she, EGG_AWAY), "her egg is out of sight");
    assert!(!on(&world, she, EGG_ALONE), "she left it with its father");
    // Now he watches it from the far side, out of her sight, and she comes
    // back to see it with no one by it.
    lead(&mut world, he, row(egg.x + 10));
    lead(&mut world, she, row(egg.x - 9));
    assert!(!on(&world, she, EGG_AWAY), "her egg is in sight");
    lead(&mut world, she, row(egg.x - 15));
    assert!(on(&world, she, EGG_ALONE), "she saw no one by it");
}

#[test]
fn partners_sense_each_other_away_and_find_each_other_and_look_to_each_other() {
    // M2 design §15.6: her partner out of sight, then found; and an
    // attention instinct about her partner draws her eye to him before a
    // nearer stranger.
    let genes = format!(
        r#"{SENSES}
        AttentionInstinct(input: "always", category: "sprite", whom: Partner, weight: 1.0),
        BrainParam(param: "curiosity", value: 0.0),"#
    );
    let (mut world, she, he, stranger, _) = partners(&genes, "", RESTER);
    let home = world.sprite(she).unwrap().pos();
    let row = |x: u16| at(x, home.y);
    lead(&mut world, he, row(home.x + 15));
    assert!(on(&world, she, PARTNER_AWAY), "he's out of her sight");
    let mut found = false;
    lead_watching(&mut world, he, row(home.x + 6), |world| {
        found |= on(world, she, FOUND);
    });
    assert!(found, "she found him");
    assert!(!on(&world, she, PARTNER_AWAY), "he's in sight");
    lead(&mut world, stranger, row(home.x + 3));
    world.step();
    assert_eq!(
        world.sprite(she).unwrap().attending_to(),
        Some(row(home.x + 6))
    );
}

/// A lone mother of `genes` and her young one, just hatched, the young one
/// 6 tiles from her, and a resting stranger between them, in a field 24
/// tiles by 3: the world, the mother, the young one and the stranger.
fn mother_young_and_stranger(genes: &str) -> (World, EntityId, EntityId, EntityId) {
    let rows = field(24, 3);
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let genes = format!(
        r#"{genes}
        BrainParam(param: "tau_base", value: 0.02),
        BrainParam(param: "curiosity", value: 0.0),
        {ALONE}{FEMALE}"#
    );
    let mut world = world_in(lone(), &rows, &[(at(2, 1), &genes), (at(22, 1), RESTER)]);
    let (mother, stranger) = (id_at(&world, at(2, 1)), id_at(&world, at(22, 1)));
    let egg = laid(&mut world, 100).expect("an egg").egg;
    let pos = egg_at(&world, egg);
    // Arranged before it hatches, so the young one is where its egg was.
    lead(&mut world, mother, at(pos.x + 6, pos.y));
    lead(&mut world, stranger, at(pos.x + 3, pos.y));
    hatch(&mut world, egg);
    world.step();
    let young = world.sprite(egg).expect("the young one");
    assert_eq!(young.parents(), [mother]);
    (world, mother, egg, stranger)
}

/// Where the sprite `id` stands.
fn pos(world: &World, id: EntityId) -> Pos {
    world.sprite(id).expect("the sprite").pos()
}

/// Where the sprite `id` attends to.
fn attends(world: &World, id: EntityId) -> Option<Pos> {
    world.sprite(id).expect("the sprite").attending_to()
}

#[test]
fn a_mother_and_her_young_look_to_each_other_as_young_and_kin_before_a_nearer_stranger() {
    // M2 design §15.6: an attention instinct about its young draws a
    // parent's eye to them, and one about its kin draws each to the other;
    // about anyone, each looks at the nearer stranger.
    let about = |whom: &str| {
        format!(r#"AttentionInstinct(input: "always", category: "sprite", {whom}weight: 1.0),"#)
    };
    let (world, mother, young, _) = mother_young_and_stranger(&about("whom: Young, "));
    assert_eq!(attends(&world, mother), Some(pos(&world, young)));
    let (world, mother, young, _) = mother_young_and_stranger(&about("whom: Kin, "));
    assert_eq!(attends(&world, mother), Some(pos(&world, young)));
    assert_eq!(attends(&world, young), Some(pos(&world, mother)));
    let (world, mother, young, stranger) = mother_young_and_stranger(&about(""));
    assert_eq!(attends(&world, mother), Some(pos(&world, stranger)));
    assert_eq!(attends(&world, young), Some(pos(&world, stranger)));
}

#[test]
fn the_breeding_drive_draws_no_eye_to_a_young_one_even_as_its_parent() {
    // M2 design §5, §15.6: an attention instinct about its young that reads
    // the breeding drive pulls nothing towards a young sprite.
    let genes = r#"
        Emitter(locus: Locus("always"), mode: Level, gain: 1.0, chem: "arousal"),
        AttentionInstinct(input: "arousal", category: "sprite", whom: Young, weight: 1.0),
    "#;
    let (world, mother, young, stranger) = mother_young_and_stranger(genes);
    assert_eq!(world.sprite(mother).unwrap().chemical("arousal"), Some(1.0));
    assert_ne!(attends(&world, mother), Some(pos(&world, young)));
    assert_eq!(attends(&world, mother), Some(pos(&world, stranger)));
}

/// What the young one does in [`mother_young_and_stranger`] with genes
/// to go over to family while young and otherwise rest, looking at whom
/// `whom` says.
fn young_does(whom: &str) -> (ActionView, EntityId, EntityId) {
    let genes = format!(
        r#"
        AttentionInstinct(input: "always", category: "sprite", {whom}weight: 1.0),
        Instinct(inputs: [("target_family", false)], verb: Approach, weight: 1.0, until: Adult),
        Instinct(inputs: [("always", false)], verb: Rest, weight: 0.5),"#
    );
    let (mut world, mother, young, stranger) = mother_young_and_stranger(&genes);
    for _ in 0..5 {
        if let Some(action) = world.sprite(young).unwrap().action() {
            return (action, mother, stranger);
        }
        world.step();
    }
    panic!("the young one did nothing");
}

#[test]
fn the_brain_reads_whether_what_it_attends_to_is_its_family() {
    // M2 design §15.6: `target_family` is 1 while the sprite attended to is
    // its family, here its mother, and 0 for a stranger.
    let (action, mother, _) = young_does("whom: Parent, ");
    assert_eq!(action.verb, Verb::Approach);
    assert_eq!(action.target, Some(Target::Sprite(mother)));
    let (action, _, _) = young_does("");
    assert_eq!(action.verb, Verb::Rest, "the stranger isn't family");
}

#[test]
fn a_young_one_and_its_mother_sense_each_other_away_and_find_each_other() {
    // M2 design §15.6: a young one of its own out of sight; no parent in
    // sight; and finding each other again.
    let rows = field(40, 3);
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let genes = format!(r#"{SENSES}{RESTER}{ALONE}{FEMALE}"#);
    let mut world = world_in(lone(), &rows, &[(at(2, 1), &genes)]);
    let mother = id_at(&world, at(2, 1));
    let young = laid(&mut world, 100).expect("an egg").egg;
    hatch(&mut world, young);
    world.step();
    assert!(!on(&world, mother, YOUNG_AWAY));
    assert!(!on(&world, young, PARENT_AWAY));
    let home = pos(&world, mother);
    lead(&mut world, mother, at(30, 1));
    assert!(
        on(&world, mother, YOUNG_AWAY),
        "her young one is out of sight"
    );
    assert!(on(&world, young, PARENT_AWAY), "its mother is out of sight");
    let (mut she_found, mut it_found) = (false, false);
    lead_watching(&mut world, mother, home, |world| {
        she_found |= on(world, mother, FOUND);
        it_found |= on(world, young, FOUND);
    });
    assert!(she_found && it_found, "each found the other");
    assert!(!on(&world, mother, YOUNG_AWAY));
    assert!(!on(&world, young, PARENT_AWAY));
}

#[test]
fn an_egg_that_hatches_out_of_sight_is_not_found_and_not_gone_back_to() {
    // M2 design §15.6: the egg's hatching ends the egg's sense and starts
    // the young one's, with nothing found, and the parent forgets where the
    // egg lay.
    let rows = field(40, 3);
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let genes = format!(
        r#"{SENSES}
        AttentionInstinct(input: "always", category: "egg", whom: Family, weight: 1.0),
        Instinct(inputs: [("always", false)], verb: Approach, weight: 1.0),
        BrainParam(param: "tau_base", value: 0.02),
        {ALONE}{FEMALE}"#
    );
    let mut world = world_in(lone(), &rows, &[(at(2, 1), &genes)]);
    let mother = id_at(&world, at(2, 1));
    let egg = laid(&mut world, 100).expect("an egg").egg;
    let tile = egg_at(&world, egg);
    // Held out of sight until it hatches.
    world.submit(Command::TakeHold { sprite: mother });
    world.submit(Command::MoveCursor { tile: at(36, 1) });
    let mut hatched = None;
    for tick in 0..1_000 {
        let events = world.step();
        assert!(!on(&world, mother, FOUND), "found nothing");
        let hatching =
            |e: &terra_sim::Event| matches!(e.kind, EventKind::Hatched { id, .. } if id == egg);
        if events.iter().any(hatching) {
            hatched = Some(tick);
        }
        if hatched.is_some_and(|hatched| tick > hatched + 3) {
            break;
        }
    }
    assert!(hatched.is_some(), "the egg hatched");
    assert!(!on(&world, mother, EGG_AWAY), "no egg");
    assert!(on(&world, mother, YOUNG_AWAY), "a young one out of sight");
    world.submit(Command::LetGo);
    for _ in 0..200 {
        world.step();
        let action = world.sprite(mother).unwrap().action();
        let back = action.is_some_and(|action| action.egg_at == Some(tile));
        assert!(!back, "it went back to where the egg was");
    }
}
