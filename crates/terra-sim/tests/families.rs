//! Families (M2 design §15.6): an attention instinct can be about whom a
//! thing is to the sprite, its family or one kind of it, so a young sprite
//! looks to its parent before a nearer stranger, and a parent to its own egg
//! before a nearer one; a parent goes back to where it last saw its egg, and
//! forgets the place once it finds the egg gone.

mod common;
use common::{ALONE, FEMALE, WorldBuilder, at, genome, hatch, laid, lone};

use terra_sim::{Command, DataPack, EntityId, EventKind, Outcome, Pos, Progress, Verb, World};

/// A world drawn from `rows`, with a sprite of its own genes on each tile,
/// all grown up, made with `data`.
fn world_in(data: DataPack, rows: &[&str], sprites: &[(Pos, &str)]) -> World {
    let placed = sprites
        .iter()
        .map(|&(pos, genes)| (pos, Some(genome(genes, &data))))
        .collect();
    WorldBuilder::new(rows)
        .data(data)
        .sprites_with(placed)
        .build()
}

/// A field of grass `width` by `height` tiles.
fn field(width: usize, height: usize) -> Vec<String> {
    (0..height).map(|_| ".".repeat(width)).collect()
}

/// The ID of the sprite standing on `pos`.
fn id_at(world: &World, pos: Pos) -> EntityId {
    world.sprite_at(pos).expect("a sprite there").id()
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
    world.submit(Command::TakeHold { sprite: id });
    world.submit(Command::MoveCursor { tile });
    for _ in 0..400 {
        world.step();
        if world.sprite(id).expect("the led sprite").pos() == tile {
            world.submit(Command::LetGo);
            world.step();
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
