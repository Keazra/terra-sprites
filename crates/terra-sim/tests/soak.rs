//! The soak's script of the Cursor's commands (design §7.4 A7, §7.6).

use terra_sim::{Command, DataPack, Soak, World, WorldConfig};

fn new_world(seed: u64) -> World {
    let data = DataPack::builtin().expect("built-in data pack is valid");
    World::new(WorldConfig::builtin(&data), data, seed)
}

/// Runs `world` for `ticks` ticks with `script`, returning every command it
/// gave, with the tick it was given before.
fn play(world: &mut World, script: &mut Soak, ticks: u64) -> Vec<(u64, Command)> {
    let mut given = Vec::new();
    for _ in 0..ticks {
        for command in script.commands(world) {
            given.push((world.tick(), command.clone()));
            world.submit(command);
        }
        world.step();
    }
    given
}

#[test]
fn the_same_seed_gives_the_same_script_and_the_same_world() {
    let (mut a, mut b) = (new_world(3), new_world(3));
    let (mut script_a, mut script_b) = (Soak::new(&a, 3), Soak::new(&b, 3));
    let given = play(&mut a, &mut script_a, 1_500);
    assert_eq!(given, play(&mut b, &mut script_b, 1_500));
    assert!(given.len() > 50, "it gave only {} commands", given.len());
    assert_eq!(a.state_hash(), b.state_hash());
}

#[test]
fn another_seed_gives_another_script() {
    let (mut a, mut b) = (new_world(3), new_world(3));
    let (mut script_a, mut script_b) = (Soak::new(&a, 3), Soak::new(&b, 4));
    assert_ne!(
        play(&mut a, &mut script_a, 500),
        play(&mut b, &mut script_b, 500)
    );
}

#[test]
fn a_soak_gives_every_kind_of_command_and_the_world_refuses_the_strays_unbroken() {
    let data = DataPack::builtin().expect("built-in data pack is valid");
    let sprites = WorldConfig::builtin(&data).sprites();
    // Debug assertions are on in tests, so a broken invariant panics here.
    let run = Soak::run(data, 5, 5_000);
    assert_eq!(run.ticks, 5_000);
    let kinds = [
        "pet",
        "hug",
        "zap",
        "shock",
        "take hold",
        "let go",
        "shove",
        "pick up",
        "put down",
        "throw",
        "move the Cursor",
        "show or hide the Cursor",
        "place",
        "spawn",
        "rename",
    ];
    for kind in kinds {
        assert!(
            run.given.contains_key(kind),
            "it never gave {kind}: {run:?}"
        );
    }
    // The strays, such as touches on sprites that aren't there, and the
    // commands that came at the wrong moment, such as a throw with nothing
    // held, are refused.
    for kind in ["pet", "throw", "rename", "place"] {
        assert!(
            run.refused.contains_key(kind),
            "the world refused no {kind}: {run:?}"
        );
    }
    assert!(run.most_sprites <= 2 * sprites, "{run:?}");
}
