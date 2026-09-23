use terra_sim::{DataPack, World};

fn new_world(seed: u64) -> World {
    World::new(
        DataPack::builtin().expect("built-in data pack is valid"),
        seed,
    )
}

#[test]
fn a_new_world_starts_at_tick_zero_and_each_step_advances_one_tick() {
    let mut world = new_world(1);
    assert_eq!(world.tick(), 0);

    world.step();
    assert_eq!(world.tick(), 1);

    for _ in 0..10 {
        world.step();
    }
    assert_eq!(world.tick(), 11);
}

#[test]
fn worlds_with_the_same_seed_have_identical_state_hashes_on_every_tick() {
    let mut a = new_world(42);
    let mut b = new_world(42);

    for _ in 0..1_000 {
        assert_eq!(
            a.state_hash(),
            b.state_hash(),
            "diverged at tick {}",
            a.tick()
        );
        a.check_invariants().expect("invariants hold");
        a.step();
        b.step();
    }
}

#[test]
fn worlds_with_different_seeds_have_different_state_hashes() {
    let mut a = new_world(1);
    let mut b = new_world(2);

    for _ in 0..10 {
        assert_ne!(
            a.state_hash(),
            b.state_hash(),
            "seeds collided at tick {}",
            a.tick()
        );
        a.step();
        b.step();
    }
}
