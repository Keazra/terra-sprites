//! The Place menu's commands (design §2.5, slice 11c): the Cursor places a
//! new object or spawns a new sprite, and the player names sprites.

use terra_sim::{
    Blocker, Command, DataPack, EntityId, Event, EventKind, Genome, Map, NameProblem, Pos,
    Rejection, Scenario, Terrain, World,
};

fn builtin() -> DataPack {
    DataPack::builtin().expect("built-in data pack is valid")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// A world drawn from `rows`, with `objects`, and a starter sprite on each
/// of `sprites`.
fn world(rows: &[&str], objects: &[(Pos, &str)], sprites: &[Pos]) -> World {
    world_in(builtin(), rows, objects, sprites)
}

fn world_in(data: DataPack, rows: &[&str], objects: &[(Pos, &str)], sprites: &[Pos]) -> World {
    let map = Map::from_ascii(rows, &data).expect("valid drawing");
    let sprites: Vec<(Pos, Option<Genome>)> = sprites.iter().map(|&pos| (pos, None)).collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &[],
    };
    World::from_scenario(scenario, data, 1).expect("a valid scenario")
}

/// The stable ID of the object type called `name`.
fn type_id(world: &World, name: &str) -> u16 {
    world
        .data()
        .object_type_id(name)
        .expect("a type of that name")
}

fn place(world: &World, tile: Pos, name: &str) -> Command {
    Command::Place {
        tile,
        object_type: type_id(world, name),
    }
}

/// The reason the world gave for refusing `command`, if it did.
fn refused(events: &[Event], command: &Command) -> Option<Rejection> {
    events.iter().find_map(|e| match &e.kind {
        EventKind::CommandRejected { command: c, reason } if c == command => Some(*reason),
        _ => None,
    })
}

/// Submits `command`, runs a tick, and returns its events.
fn run(world: &mut World, command: Command) -> Vec<Event> {
    world.submit(command.clone());
    world.step()
}

#[test]
fn the_place_menu_lists_the_types_the_data_lets_the_cursor_place() {
    let data = builtin();
    let placeable: Vec<(&str, &str)> = data.placeable().collect();
    assert_eq!(
        placeable,
        [
            ("berry_bush", "berry bush seedling"),
            ("berry", "berry"),
            ("ball", "ball"),
        ]
    );
}

#[test]
fn a_placed_item_appears_on_the_tile_new() {
    let mut world = world(&["....."], &[], &[]);
    let events = {
        let command = place(&world, at(2, 0), "berry");
        run(&mut world, command)
    };
    let berry = world.object_at(at(2, 0)).expect("a berry there");
    assert_eq!(berry.type_name(), "berry");
    assert_eq!(berry.stage(), Some("fresh"));
    let placed = EventKind::Placed {
        id: berry.id(),
        object_type: "berry".into(),
        pos: at(2, 0),
    };
    assert!(events.iter().any(|e| e.kind == placed), "{events:?}");
}

#[test]
fn a_placed_bush_starts_as_a_seedling() {
    let mut world = world(&["....."], &[], &[]);
    {
        let command = place(&world, at(2, 0), "berry_bush");
        run(&mut world, command)
    };
    let bush = world.object_at(at(2, 0)).expect("a bush there");
    assert_eq!(bush.stage(), Some("seedling"));
    assert_eq!(bush.visual_state(), "seedling");
}

#[test]
fn a_placed_bush_may_wall_off_a_corridor() {
    // The owner's choice for slice 11c: the built-in bush asks nothing of
    // its tile, so the player may pen sprites in on purpose.
    let rows = ["#####", ".....", "#####"];
    let mut world = world(&rows, &[], &[]);
    let command = place(&world, at(2, 1), "berry_bush");
    let events = run(&mut world, command.clone());
    assert_eq!(refused(&events, &command), None);
    assert!(world.object_at(at(2, 1)).is_some());
}

#[test]
fn a_placement_rule_in_the_data_can_keep_paths_open() {
    let objects = builtin_source("objects.ron").replace(
        r#"place: (label: "berry bush seedling"),"#,
        r#"place: (label: "berry bush seedling", if: [KeepsPathsOpen]),"#,
    );
    let data = pack_with("objects.ron", &objects);
    let rows = ["#####", ".....", "#####"];
    let mut world = world_in(data, &rows, &[], &[]);
    let bush = type_id(&world, "berry_bush");
    let command = Command::Place {
        tile: at(2, 1),
        object_type: bush,
    };
    let events = run(&mut world, command.clone());
    assert_eq!(
        refused(&events, &command),
        Some(Rejection::PlaceRule {
            object_type: bush,
            rule: terra_sim::PlaceRule::KeepsPathsOpen,
        })
    );
    assert!(world.object_at(at(2, 1)).is_none());
}

#[test]
fn placing_is_refused_where_the_thing_cant_go() {
    let rows = ["..#..", "....."];
    let mut world = world(&rows, &[(at(0, 0), "berry")], &[at(4, 1)]);
    // The sprite first, before it walks off.
    for (tile, name, blocker) in [
        (at(4, 1), "berry_bush", Blocker::Sprite),
        (at(0, 0), "ball", Blocker::Object(type_id(&world, "berry"))),
        (at(2, 0), "ball", Blocker::Terrain(Terrain::Rock)),
    ] {
        let command = place(&world, tile, name);
        let events = run(&mut world, command.clone());
        let item_type = type_id(&world, name);
        assert_eq!(
            refused(&events, &command),
            Some(Rejection::InTheWay { item_type, blocker }),
            "{name} on {tile:?}"
        );
    }
    // An item may go under a sprite (design §3.4).
    let under = world.sprites().next().expect("the sprite").pos();
    let command = place(&world, under, "ball");
    let events = run(&mut world, command.clone());
    assert_eq!(refused(&events, &command), None);
}

#[test]
fn placing_is_refused_off_the_map_and_for_a_type_the_data_doesnt_let_be_placed() {
    let mut world = world(&["....."], &[], &[]);
    let command = place(&world, at(9, 0), "berry");
    let events = run(&mut world, command.clone());
    assert_eq!(refused(&events, &command), Some(Rejection::OffTheMap));
    for name in ["thornbush", "water", "sprite"] {
        let command = place(&world, at(1, 0), name);
        let events = run(&mut world, command.clone());
        assert_eq!(
            refused(&events, &command),
            Some(Rejection::NotPlaceable),
            "{name}"
        );
    }
    let command = Command::Place {
        tile: at(1, 0),
        object_type: 9999,
    };
    let events = run(&mut world, command.clone());
    assert_eq!(refused(&events, &command), Some(Rejection::NotPlaceable));
}

#[test]
fn placing_leaves_what_the_cursor_has_hold_of_alone() {
    let mut world = world(&["....."], &[], &[at(0, 0)]);
    let led = world.sprite_at(at(0, 0)).unwrap().id();
    run(&mut world, Command::TakeHold { sprite: led });
    {
        let command = place(&world, at(3, 0), "berry");
        run(&mut world, command)
    };
    assert_eq!(world.cursor().leads(), Some(led));
    assert!(world.object_at(at(3, 0)).is_some());
}

#[test]
fn a_spawned_sprite_is_a_newborn_from_the_starter_genome_with_variation() {
    let mut world = world(&["....."], &[], &[at(0, 0)]);
    let first = world.sprite_at(at(0, 0)).unwrap();
    let first_genome = first.genome().to_ron(world.data());
    let events = run(
        &mut world,
        Command::SpawnSprite {
            tile: at(3, 0),
            genome: None,
        },
    );
    let spawned = world.sprite_at(at(3, 0)).expect("a sprite there");
    assert_eq!(spawned.age(), 1, "born at this tick's step 1");
    let id = spawned.id();
    let said = EventKind::Spawned { id, pos: at(3, 0) };
    assert!(events.iter().any(|e| e.kind == said), "{events:?}");
    assert_eq!(spawned.name(), None, "sprites start unnamed (design §6.5)");
    assert_ne!(
        spawned.genome().to_ron(world.data()),
        first_genome,
        "each is varied on its own draws"
    );
}

#[test]
fn a_sprite_spawned_from_a_genome_carries_that_genome_exactly() {
    let data = builtin();
    let text = builtin_source("genomes/starter.ron");
    let genome = Genome::from_ron(&text, &data).expect("the starter genome");
    let mut world = world(&["....."], &[], &[]);
    run(
        &mut world,
        Command::SpawnSprite {
            tile: at(1, 0),
            genome: Some(genome.clone()),
        },
    );
    let spawned = world.sprite_at(at(1, 0)).expect("a sprite there");
    assert_eq!(spawned.genome(), &genome);
}

#[test]
fn an_exported_genome_reimports_identically() {
    let world = world(&["....."], &[], &[at(0, 0)]);
    let sprite = world.sprite_at(at(0, 0)).unwrap();
    let written = sprite.genome().to_ron(world.data());
    let again = Genome::from_ron(&written, world.data()).expect("it reads back");
    assert_eq!(&again, sprite.genome());
    assert_eq!(again.to_ron(world.data()), written);
}

#[test]
fn spawning_is_refused_where_a_sprite_cant_stand() {
    let rows = ["..#..", "....."];
    let mut world = world(
        &rows,
        &[(at(0, 1), "berry_bush"), (at(1, 1), "ball")],
        &[at(4, 1)],
    );
    let bush = type_id(&world, "berry_bush");
    for (tile, blocker) in [
        (at(4, 1), Blocker::Sprite),
        (at(0, 1), Blocker::Object(bush)),
        (at(2, 0), Blocker::Terrain(Terrain::Rock)),
    ] {
        let command = Command::SpawnSprite { tile, genome: None };
        let events = run(&mut world, command.clone());
        assert_eq!(
            refused(&events, &command),
            Some(Rejection::NoRoom(blocker)),
            "{tile:?}"
        );
    }
    let command = Command::SpawnSprite {
        tile: at(9, 9),
        genome: None,
    };
    let events = run(&mut world, command.clone());
    assert_eq!(refused(&events, &command), Some(Rejection::OffTheMap));
    // A sprite may stand on an item (design §3.4).
    let command = Command::SpawnSprite {
        tile: at(1, 1),
        genome: None,
    };
    let events = run(&mut world, command.clone());
    assert_eq!(refused(&events, &command), None);
}

#[test]
fn a_renamed_sprite_shows_its_name_and_dies_with_it() {
    let mut world = world(&["....."], &[], &[at(0, 0)]);
    let id = world.sprite_at(at(0, 0)).unwrap().id();
    let events = run(
        &mut world,
        Command::Rename {
            sprite: id,
            name: "Mira".into(),
        },
    );
    let renamed = EventKind::Renamed {
        id,
        name: "Mira".into(),
    };
    assert!(events.iter().any(|e| e.kind == renamed), "{events:?}");
    assert_eq!(world.sprite(id).unwrap().name(), Some("Mira"));
    // Starved to death, it takes its name with it (design §2.5).
    let mut died = None;
    for _ in 0..200_000 {
        died = world.step().into_iter().find_map(|e| match e.kind {
            EventKind::Died { id: dead, name, .. } if dead == id => Some(name),
            _ => None,
        });
        if died.is_some() {
            break;
        }
    }
    assert_eq!(died, Some(Some("Mira".to_string())));
}

#[test]
fn a_name_must_be_one_to_sixteen_cp437_characters() {
    let mut world = world(&["....."], &[], &[at(0, 0)]);
    let id = world.sprite_at(at(0, 0)).unwrap().id();
    for (name, problem) in [
        ("", NameProblem::Empty),
        ("   ", NameProblem::Empty),
        ("Abcdefghijklmnopq", NameProblem::TooLong),
        ("Mira🙂", NameProblem::NotCp437),
    ] {
        let command = Command::Rename {
            sprite: id,
            name: name.into(),
        };
        let events = run(&mut world, command.clone());
        assert_eq!(
            refused(&events, &command),
            Some(Rejection::BadName(problem)),
            "{name:?}"
        );
    }
    assert_eq!(world.sprite(id).unwrap().name(), None);
    // Sixteen is fine, accented letters too, and the ends are trimmed.
    let command = Command::Rename {
        sprite: id,
        name: "  Ébène-Ñoël abcd ".into(),
    };
    run(&mut world, command);
    assert_eq!(world.sprite(id).unwrap().name(), Some("Ébène-Ñoël abcd"));
    let command = Command::Rename {
        sprite: EntityId(999),
        name: "Mira".into(),
    };
    let events = run(&mut world, command.clone());
    assert_eq!(refused(&events, &command), Some(Rejection::Gone));
}

#[test]
fn naming_never_touches_the_worlds_randomness() {
    let mut named = world(&["....."], &[], &[at(0, 0)]);
    let mut unnamed = world(&["....."], &[], &[at(0, 0)]);
    let id = named.sprite_at(at(0, 0)).unwrap().id();
    named.submit(Command::Rename {
        sprite: id,
        name: "Mira".into(),
    });
    for _ in 0..50 {
        named.step();
        unnamed.step();
    }
    assert_eq!(
        named.sprite(id).unwrap().pos(),
        unnamed.sprite(id).unwrap().pos()
    );
}

#[test]
fn a_random_name_is_made_of_the_datas_syllables_and_is_always_a_valid_name() {
    let data = builtin();
    let names: Vec<String> = (0..200).map(|seed| data.random_name(seed)).collect();
    for name in &names {
        let chars = name.chars().count();
        assert!((2..=terra_sim::MAX_NAME_CHARS).contains(&chars), "{name}");
        assert!(name.chars().next().unwrap().is_uppercase(), "{name}");
        assert!(name.chars().all(char::is_alphabetic), "{name}");
    }
    assert_eq!(
        data.random_name(7),
        data.random_name(7),
        "a seed gives one name"
    );
    let mut distinct = names.clone();
    distinct.sort();
    distinct.dedup();
    assert!(distinct.len() > 150, "only {} distinct", distinct.len());
}

fn builtin_source(path: &str) -> String {
    DataPack::builtin_sources()
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, text)| text.to_string())
        .expect("a built-in file")
}

fn pack_with(path: &str, text: &str) -> DataPack {
    let sources: Vec<(&str, &str)> = DataPack::builtin_sources()
        .iter()
        .map(|&(name, original)| (name, if name == path { text } else { original }))
        .collect();
    DataPack::from_sources(&sources).expect("a valid pack")
}

#[test]
fn a_new_objects_look_is_its_first_stages() {
    let data = builtin();
    assert_eq!(data.new_look("berry_bush"), Some("seedling"));
    assert_eq!(data.new_look("ball"), Some("default"));
    assert_eq!(data.new_look("unicorn"), None);
}
