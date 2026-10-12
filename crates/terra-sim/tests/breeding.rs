//! Breeding (M2 design §5): mating happens only between adults, enforced by
//! the engine for any genome and data pack.

mod common;
use common::{WorldBuilder, at, builtin, genome};

use proptest::prelude::*;
use terra_sim::{EventKind, LifeStage, Pos, Target, Verb, World};

/// A small field of grass with a sprite of `genes` on each tile of
/// `sprites`, those on `young` as newborns and the rest just grown up.
fn field(genes: &str, sprites: &[Pos], young: &[Pos], seed: u64) -> World {
    let data = builtin();
    let genome = genome(genes, &data);
    let placed = sprites
        .iter()
        .map(|&pos| (pos, Some(genome.clone())))
        .collect();
    WorldBuilder::new(&["........", "........", "........", "........"])
        .data(data)
        .sprites_with(placed)
        .newborns(young)
        .seed(seed)
        .build()
}

/// An instinct to mate with whatever sprite it sees, and nothing else.
const KEEN: &str = r#"
    AttentionInstinct(input: "always", category: "sprite", weight: 1.0),
    Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Mate, weight: 1.0),
"#;

/// A Mate action as it started: the life stage of the sprite mating, and
/// of the sprite it's aimed at, or `None` if it's aimed at no sprite.
type Mating = (LifeStage, Option<LifeStage>);

/// Steps `world` `ticks` times, returning each Mate action started.
fn matings(world: &mut World, ticks: u64) -> Vec<Mating> {
    let mut found = Vec::new();
    for _ in 0..ticks {
        for event in world.step() {
            let EventKind::ActionStarted {
                id,
                verb: Verb::Mate,
            } = event.kind
            else {
                continue;
            };
            // One that died in the same tick: the self-check saw its action.
            let Some(sprite) = world.sprite(id) else {
                continue;
            };
            let partner = match sprite.action().and_then(|a| a.target) {
                Some(Target::Sprite(other)) => match world.sprite(other) {
                    Some(other) => Some(other.life_stage()),
                    None => continue,
                },
                _ => None,
            };
            found.push((sprite.life_stage(), partner));
        }
    }
    found
}

#[test]
fn an_adult_keen_to_mate_mates_with_an_adult() {
    let mut world = field(KEEN, &[at(2, 1), at(3, 1)], &[], 1);
    let found = matings(&mut world, 20);
    assert!(!found.is_empty(), "no adult mated");
    assert!(
        found
            .iter()
            .all(|&(stage, partner)| stage == LifeStage::Adult && partner == Some(LifeStage::Adult)),
        "{found:?}"
    );
}

#[test]
fn an_adult_never_mates_with_a_young_sprite_nor_a_young_one_at_all() {
    // M2 design §5: Mate isn't offered unless both are adults, so the
    // instinct has nothing to choose.
    let mut world = field(KEEN, &[at(2, 1), at(3, 1)], &[at(3, 1)], 1);
    assert_eq!(matings(&mut world, 200), []);
}

/// A state input a random gene may read.
fn state_input() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec![
        "always",
        "hunger",
        "thirst",
        "boredom",
        "loneliness",
        "crowdedness",
        "pain",
        "age",
        "nearby_sprites",
        "h3",
    ])
}

/// A random switch: all its life, or only before or after growing up.
fn switch() -> impl Strategy<Value = &'static str> {
    prop::sample::select(vec!["", ", until: Adult", ", from: Adult", ", from: Old"])
}

/// A random instinct, leaning to Mate.
fn instinct() -> impl Strategy<Value = String> {
    let verb = prop::sample::select(vec![
        "Mate", "Mate", "Mate", "Approach", "Play", "Hit", "Wander", "Rest",
    ]);
    let attended = prop::sample::select(vec!["", r#", ("attended_sprite", false)"#]);
    (state_input(), any::<bool>(), attended, verb, -1.0f32..1.0, switch()).prop_map(
        |(input, negated, attended, verb, weight, switch)| {
            format!(
                r#"Instinct(inputs: [("{input}", {negated}){attended}], verb: {verb}, weight: {weight:?}{switch}),"#
            )
        },
    )
}

/// A random attention instinct, leaning to sprites.
fn attention() -> impl Strategy<Value = String> {
    let category = prop::sample::select(vec!["sprite", "sprite", "bush", "water", "toy"]);
    (state_input(), category, -1.0f32..1.0, switch()).prop_map(
        |(input, category, weight, switch)| {
            format!(
                r#"AttentionInstinct(input: "{input}", category: "{category}", weight: {weight:?}{switch}),"#
            )
        },
    )
}

/// A random genome of brain genes: each sprite grows up and old at random
/// ages, so some cross a life stage mid-run.
fn random_genes() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(instinct(), 1..8),
        prop::collection::vec(attention(), 0..4),
        1500.0f32..2500.0,
        6000.0f32..7000.0,
    )
        .prop_map(|(instincts, attention, grows_up, grows_old)| {
            format!(
                r#"Trait(trait: "grows_up", value: {grows_up:?}),
                   Trait(trait: "grows_old", value: {grows_old:?}),
                   Instinct(inputs: [("always", false), ("attended_sprite", false)], verb: Mate, weight: 1.0, from: Young),
                   {}
                   {}"#,
                attention.join("\n"),
                instincts.join("\n")
            )
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn whatever_the_genome_only_adults_ever_mate_and_only_with_adults(
        genes in random_genes(),
        young in prop::collection::vec(any::<bool>(), 4),
        seed: u64,
    ) {
        // M2 design §5, G7. Every tick also runs the self-check, which
        // fails on a Mate action that isn't between adults.
        let tiles = [at(2, 1), at(3, 1), at(2, 2), at(3, 2)];
        let newborns: Vec<Pos> = tiles
            .iter()
            .zip(&young)
            .filter(|&(_, &young)| young)
            .map(|(&pos, _)| pos)
            .collect();
        let mut world = field(&genes, &tiles, &newborns, seed);
        for (stage, partner) in matings(&mut world, 300) {
            prop_assert!(stage.is_grown(), "a {stage:?} sprite mated");
            prop_assert!(
                partner.is_some_and(LifeStage::is_grown),
                "a sprite mated with {partner:?}"
            );
        }
    }
}
