//! The inspector's unit tests: the title's fitting, the words of action
//! and log lines, and the Brain tab's wrapped rows.

use terra_sim::{
    ActionView, EntityId, Event, EventKind, Hurt, Outcome, Pos, Progress, Target, Verb,
};

use super::actions::{action_line, logged_line};
use super::body::bar_parts;
use super::brain::{concept_name, scored};
use super::*;
use crate::text::Words;

#[test]
fn a_label_too_long_for_the_title_is_shortened_before_the_tabs_are() {
    // The five tabs take 30 of the row's 45 columns, the open one's spaces
    // included, which leaves 15: a label of 13 with a space each side.
    let fit = |sprite| fitted_label(sprite, 15);
    assert_eq!(fit(Some(("Sprite #530", "Sprite #530"))), " Sprite #530   ");
    assert_eq!(fit(Some(("Mira #12", "Mira"))), " Mira #12      ");
    assert_eq!(
        fit(Some(("WWWWWWWWA #538", "WWWWWWWWA"))),
        " WWWWWWWWA     ",
        "the name alone, when the number doesn't fit beside it"
    );
    assert_eq!(
        fit(Some(("Abcdefghijklmnop #538", "Abcdefghijklmnop"))),
        " Abcdefghij... ",
        "as much of the name as fits"
    );
    assert_eq!(
        fit(Some(("Old Bo of Mira #538", "Old Bo of Mira"))),
        " Old Bo of...  ",
        "with no space before the dots"
    );
    assert_eq!(fit(None), " ".repeat(15));
}

/// The tab written at `column` of the title row with `open` open.
fn tab_at(open: Tab, column: usize) -> Option<Tab> {
    tab_places(open)
        .into_iter()
        .find(|(_, place)| place.contains(&column))
        .map(|(tab, _)| tab)
}

#[test]
fn a_tab_is_found_in_the_columns_the_title_row_writes_it_in() {
    // The tabs fill the row's last 30 columns, from column 15: with Body
    // open, " Body  Brain Chem Genome World".
    let body_open = |column| tab_at(Tab::Body, column);
    assert_eq!(body_open(14), None);
    assert_eq!(body_open(15), Some(Tab::Body), "the open tab's space");
    assert_eq!(body_open(20), Some(Tab::Body));
    assert_eq!(body_open(21), None, "between two tabs");
    assert_eq!(body_open(22), Some(Tab::Brain));
    assert_eq!(body_open(44), Some(Tab::World));
    assert_eq!(body_open(45), None);
    // With World open, "Body Brain Chem Genome  World ".
    let world_open = |column| tab_at(Tab::World, column);
    assert_eq!(world_open(18), Some(Tab::Body));
    assert_eq!(world_open(19), None);
    assert_eq!(world_open(31), Some(Tab::Genome));
    assert_eq!(world_open(37), None);
    assert_eq!(world_open(38), Some(Tab::World));
    assert_eq!(world_open(44), Some(Tab::World));
}

fn wander(progress: Progress) -> ActionView {
    ActionView {
        verb: Verb::Wander,
        destination: Some(Pos { x: 61, y: 40 }),
        target: None,
        target_type: None,
        attempted: false,
        target_gone: false,
        hurt: Hurt::default(),
        progress,
        remembered: false,
        heard: None,
        egg_at: None,
    }
}

fn rest(progress: Progress) -> ActionView {
    ActionView {
        verb: Verb::Rest,
        destination: None,
        target: None,
        target_type: None,
        attempted: false,
        target_gone: false,
        hurt: Hurt::default(),
        progress,
        remembered: false,
        heard: None,
        egg_at: None,
    }
}

// A timed-out action is followed by the next on the same tick, so the
// screen rarely shows its ending: every wording is tested here instead.
#[test]
fn the_action_line_says_what_a_sprite_is_doing_plainly_or_exactly() {
    let cases = [
        (
            wander(Progress::Walking { steps_left: 5 }),
            "Wandering off · 5 tiles to go",
            "WANDER → (61,40) · walking (5 tiles)",
        ),
        (
            wander(Progress::Walking { steps_left: 1 }),
            "Wandering off · 1 tile to go",
            "WANDER → (61,40) · walking (1 tile)",
        ),
        (
            wander(Progress::Waiting { blocked_ticks: 2 }),
            "Wandering off · waiting to get past",
            "WANDER → (61,40) · blocked (2 ticks)",
        ),
        (
            wander(Progress::Ended(Outcome::Applied)),
            "Arrived",
            "WANDER → (61,40) · applied",
        ),
        (
            wander(Progress::Ended(Outcome::Blocked)),
            "Gave up: the way was blocked",
            "WANDER → (61,40) · blocked",
        ),
        (
            wander(Progress::Ended(Outcome::TimedOut)),
            "Gave up: it took too long",
            "WANDER → (61,40) · timed_out",
        ),
        (
            wander(Progress::Ended(Outcome::Failed)),
            "Gave up: it couldn't get there",
            "WANDER → (61,40) · failed",
        ),
        (
            rest(Progress::Resting { ticks: 4, of: 10 }),
            "Resting · 6 ticks left",
            "REST · 4 of 10 ticks",
        ),
        (
            rest(Progress::Resting { ticks: 9, of: 10 }),
            "Resting · 1 tick left",
            "REST · 9 of 10 ticks",
        ),
        (
            rest(Progress::Ended(Outcome::Applied)),
            "Rested",
            "REST · applied",
        ),
    ];
    for (view, plain, exact) in cases {
        assert_eq!(action_line(&view, false, &pack()), plain, "{view:?}");
        assert_eq!(action_line(&view, true, &pack()), exact, "{view:?}");
    }
    let nowhere = ActionView {
        destination: None,
        ..wander(Progress::Ended(Outcome::Failed))
    };
    assert_eq!(action_line(&nowhere, true, &pack()), "WANDER · failed");
}

/// The built-in pack's words, with no sprite named.
fn pack() -> Words<'static> {
    let data = Box::leak(Box::new(
        terra_sim::DataPack::builtin().expect("built-in data pack is valid"),
    ));
    let names = Box::leak(Box::new(crate::text::Names::default()));
    let news = Box::leak(Box::new(std::collections::BTreeMap::new()));
    Words { data, names, news }
}

/// An aimed action, `verb`, at `target` of the type `target_type`.
fn aimed(verb: Verb, target: Target, target_type: u16, progress: Progress) -> ActionView {
    ActionView {
        verb,
        destination: None,
        target: Some(target),
        target_type: Some(target_type),
        attempted: false,
        target_gone: false,
        hurt: Hurt::default(),
        progress,
        remembered: false,
        heard: None,
        egg_at: None,
    }
}

#[test]
fn the_action_line_says_a_trip_is_to_a_place_it_remembers() {
    // M2 design §7.
    let walking = Progress::Walking { steps_left: 34 };
    let remembered = |view: ActionView| ActionView {
        remembered: true,
        ..view
    };
    let water = aimed(
        Verb::Drink,
        Target::Water(Pos { x: 40, y: 12 }),
        100,
        walking,
    );
    let bush = aimed(Verb::Eat, Target::Object(EntityId(812)), 1, walking);
    let cases = [
        (
            remembered(water),
            "Going to drink at the water it remembers · 34 tiles to go",
            "DRINK → water (40,12) · from memory · walking (34 tiles)",
        ),
        (
            remembered(bush),
            "Going to eat the berry bush it remembers · 34 tiles to go",
            "EAT → berry_bush #812 · from memory · walking (34 tiles)",
        ),
    ];
    for (view, plain, exact) in cases {
        assert_eq!(action_line(&view, false, &pack()), plain, "{view:?}");
        assert_eq!(action_line(&view, true, &pack()), exact, "{view:?}");
    }
}

#[test]
fn the_action_line_names_what_an_eat_drink_or_approach_is_aimed_at() {
    use Outcome::*;
    use Progress::*;
    // Object types: berry_bush 1, berry 2, water 100, sprite 101.
    let bush = |p| aimed(Verb::Eat, Target::Object(EntityId(812)), 1, p);
    let berry = |p| aimed(Verb::Eat, Target::Object(EntityId(9)), 2, p);
    let water = |p| aimed(Verb::Drink, Target::Water(Pos { x: 40, y: 12 }), 100, p);
    let sprite = |p| aimed(Verb::Approach, Target::Sprite(EntityId(530)), 101, p);
    let tried = |view: ActionView| ActionView {
        attempted: true,
        ..view
    };
    let gone = |view: ActionView| ActionView {
        target_gone: true,
        ..view
    };
    let cases = [
        (
            bush(Walking { steps_left: 3 }),
            "Going to eat the berry bush · 3 tiles to go",
            "EAT → berry_bush #812 · walking (3 tiles)",
        ),
        (
            bush(Waiting { blocked_ticks: 1 }),
            "Going to eat the berry bush · waiting to get past",
            "EAT → berry_bush #812 · blocked (1 tick)",
        ),
        (
            tried(bush(Ended(Applied))),
            "Ate from the berry bush",
            "EAT → berry_bush #812 · applied",
        ),
        (
            tried(bush(Ended(Failed))),
            "Couldn't eat from the berry bush",
            "EAT → berry_bush #812 · failed",
        ),
        (
            gone(tried(berry(Ended(Applied)))),
            "Ate the berry",
            "EAT → berry #9 · applied",
        ),
        (
            gone(berry(Ended(Failed))),
            "Gave up: the berry was gone",
            "EAT → berry #9 · failed",
        ),
        // Its gut had no room (M2 design §19.4): a bush is eaten from,
        // a berry whole.
        (
            tried(bush(Ended(TooFull))),
            "Too full to eat from the berry bush",
            "EAT → berry_bush #812 · too_full",
        ),
        (
            tried(berry(Ended(TooFull))),
            "Too full to eat the berry",
            "EAT → berry #9 · too_full",
        ),
        (
            bush(Ended(Failed)),
            "Gave up: it couldn't get there",
            "EAT → berry_bush #812 · failed",
        ),
        (
            bush(Ended(Interrupted)),
            "Changed its mind",
            "EAT → berry_bush #812 · interrupted",
        ),
        (
            water(Walking { steps_left: 2 }),
            "Going to drink · 2 tiles to go",
            "DRINK → water (40,12) · walking (2 tiles)",
        ),
        (
            tried(water(Ended(Applied))),
            "Drank",
            "DRINK → water (40,12) · applied",
        ),
        (
            tried(water(Ended(TooFull))),
            "Too full to drink",
            "DRINK → water (40,12) · too_full",
        ),
        (
            sprite(Walking { steps_left: 4 }),
            "Going over to Sprite #530 · 4 tiles to go",
            "APPROACH → sprite #530 · walking (4 tiles)",
        ),
        (
            tried(sprite(Ended(Applied))),
            "Got to Sprite #530",
            "APPROACH → sprite #530 · applied",
        ),
        (
            gone(sprite(Ended(Failed))),
            "Gave up: Sprite #530 was gone",
            "APPROACH → sprite #530 · failed",
        ),
    ];
    for (view, plain, exact) in cases {
        assert_eq!(action_line(&view, false, &pack()), plain, "{view:?}");
        assert_eq!(action_line(&view, true, &pack()), exact, "{view:?}");
    }
}

/// `view` after its attempt, which hurt its own sprite.
fn tried_and_hurt(view: ActionView) -> ActionView {
    let hurt = Hurt {
        actor: true,
        target: false,
    };
    ActionView {
        attempted: true,
        hurt,
        ..view
    }
}

#[test]
fn play_and_hit_lines_say_kicked_for_a_push_and_got_hurt_for_a_hurt() {
    use Outcome::*;
    use Progress::*;
    // Object types: thornbush 3, ball 4, sprite 101.
    let ball = |verb, p| aimed(verb, Target::Object(EntityId(40)), 4, p);
    let thorns = |verb, p| aimed(verb, Target::Object(EntityId(77)), 3, p);
    let sprite = |verb, p| aimed(verb, Target::Sprite(EntityId(7)), 101, p);
    let tried = |view: ActionView| ActionView {
        attempted: true,
        ..view
    };
    let cases = [
        (
            ball(Verb::Play, Walking { steps_left: 3 }),
            "Going to play with the ball · 3 tiles to go",
            "PLAY → ball #40 · walking (3 tiles)",
        ),
        (
            tried(ball(Verb::Play, Ended(Applied))),
            "Kicked the ball",
            "PLAY → ball #40 · applied",
        ),
        (
            tried(ball(Verb::Hit, Ended(Applied))),
            "Hit the ball",
            "HIT → ball #40 · applied",
        ),
        (
            tried(sprite(Verb::Play, Ended(Applied))),
            "Played with Sprite #7",
            "PLAY → sprite #7 · applied",
        ),
        (
            sprite(Verb::Hit, Walking { steps_left: 1 }),
            "Going to hit Sprite #7 · 1 tile to go",
            "HIT → sprite #7 · walking (1 tile)",
        ),
        (
            tried(sprite(Verb::Hit, Ended(Applied))),
            "Hit Sprite #7",
            "HIT → sprite #7 · applied",
        ),
        (
            tried_and_hurt(thorns(Verb::Hit, Ended(Applied))),
            "Hit the thornbush, and got hurt",
            "HIT → thornbush #77 · applied",
        ),
        (
            tried_and_hurt(thorns(Verb::Play, Ended(Applied))),
            "Played with the thornbush, and got hurt",
            "PLAY → thornbush #77 · applied",
        ),
        (
            tried_and_hurt(thorns(Verb::Eat, Ended(Applied))),
            "Tried to eat the thornbush, and got hurt",
            "EAT → thornbush #77 · applied",
        ),
        (
            sprite(Verb::Play, Ended(Interrupted)),
            "Changed its mind",
            "PLAY → sprite #7 · interrupted",
        ),
        (
            tried(ball(Verb::Play, Ended(Failed))),
            "Couldn't play with the ball",
            "PLAY → ball #40 · failed",
        ),
        (
            tried(sprite(Verb::Hit, Ended(Failed))),
            "Couldn't hit Sprite #7",
            "HIT → sprite #7 · failed",
        ),
        (
            tried_and_hurt(thorns(Verb::Eat, Ended(Failed))),
            "Couldn't eat from the thornbush, and got hurt",
            "EAT → thornbush #77 · failed",
        ),
        // A pack may feed a sprite through a play (M2 design §19.4).
        (
            tried(ball(Verb::Play, Ended(TooFull))),
            "Too full",
            "PLAY → ball #40 · too_full",
        ),
    ];
    for (view, plain, exact) in cases {
        assert_eq!(action_line(&view, false, &pack()), plain, "{view:?}");
        assert_eq!(action_line(&view, true, &pack()), exact, "{view:?}");
    }
}

#[test]
fn play_and_hit_are_observed_in_the_past_tense_like_the_other_verbs() {
    use Outcome::*;
    use Progress::Ended;
    let ball = |verb, o| aimed(verb, Target::Object(EntityId(40)), 4, Ended(o));
    let thorns = |verb, o| aimed(verb, Target::Object(EntityId(77)), 3, Ended(o));
    let sprite = |verb, o| aimed(verb, Target::Sprite(EntityId(7)), 101, Ended(o));
    let tried = |view: ActionView| ActionView {
        attempted: true,
        ..view
    };
    let cases = [
        (tried(ball(Verb::Play, Applied)), "Kicked the ball"),
        (tried(sprite(Verb::Play, Applied)), "Played with Sprite #7"),
        (tried(sprite(Verb::Hit, Applied)), "Hit Sprite #7"),
        (
            tried_and_hurt(thorns(Verb::Eat, Applied)),
            "Tried to eat the thornbush, and got hurt",
        ),
        (
            tried_and_hurt(thorns(Verb::Play, Applied)),
            "Played with the thornbush, and got hurt",
        ),
        (
            sprite(Verb::Play, Interrupted),
            "Went to play with Sprite #7, but changed its mind",
        ),
        (
            ball(Verb::Hit, TimedOut),
            "Went to hit the ball, but gave up: it took too long",
        ),
        (
            tried(sprite(Verb::Hit, Failed)),
            "Went to hit Sprite #7, but couldn't",
        ),
        (
            tried_and_hurt(thorns(Verb::Eat, Failed)),
            "Went to eat the thornbush, but it was empty, and got hurt",
        ),
    ];
    for (view, line) in cases {
        assert_eq!(observed_line(&view, &pack()), line, "{view:?}");
    }
}

#[test]
fn a_retreat_says_it_is_backing_away_and_whether_it_was_cornered() {
    use Outcome::*;
    use Progress::*;
    // Object types: thornbush 3, sprite 101.
    let sprite = |p| aimed(Verb::Retreat, Target::Sprite(EntityId(7)), 101, p);
    let thorns = |p| aimed(Verb::Retreat, Target::Object(EntityId(77)), 3, p);
    let gone = |view: ActionView| ActionView {
        target_gone: true,
        ..view
    };
    let cases = [
        (
            sprite(Walking { steps_left: 4 }),
            "Backing away from Sprite #7 · 4 steps to go",
            "RETREAT → sprite #7 · walking (4 steps)",
        ),
        (
            thorns(Walking { steps_left: 1 }),
            "Backing away from the thornbush · 1 step to go",
            "RETREAT → thornbush #77 · walking (1 step)",
        ),
        (
            sprite(Waiting { blocked_ticks: 2 }),
            "Backing away from Sprite #7 · waiting for room",
            "RETREAT → sprite #7 · blocked (2 ticks)",
        ),
        (
            sprite(Ended(Applied)),
            "Backed away from Sprite #7",
            "RETREAT → sprite #7 · applied",
        ),
        (
            sprite(Ended(Blocked)),
            "Backed into a corner",
            "RETREAT → sprite #7 · blocked",
        ),
        (
            gone(sprite(Ended(Failed))),
            "Gave up: Sprite #7 was gone",
            "RETREAT → sprite #7 · failed",
        ),
    ];
    for (view, plain, exact) in cases {
        assert_eq!(action_line(&view, false, &pack()), plain, "{view:?}");
        assert_eq!(action_line(&view, true, &pack()), exact, "{view:?}");
    }
    let observed = [
        (sprite(Ended(Applied)), "Backed away from Sprite #7"),
        (
            sprite(Ended(Blocked)),
            "Backed away from Sprite #7, but was cornered",
        ),
        (
            thorns(Ended(Interrupted)),
            "Backed away from the thornbush, but changed its mind",
        ),
    ];
    for (view, line) in observed {
        assert_eq!(observed_line(&view, &pack()), line, "{view:?}");
    }
}

#[test]
fn a_name_takes_an_before_a_vowel_whatever_its_case() {
    assert_eq!(with_article("ball"), "a ball");
    assert_eq!(with_article("apple"), "an apple");
    assert_eq!(with_article("Apple"), "an Apple");
    assert_eq!(with_article("Egg"), "an Egg");
}

#[test]
fn the_action_line_says_a_sprite_calls_courts_goes_to_a_call_and_pairs() {
    // M2 design §15.8.
    use Outcome::*;
    use Progress::*;
    let bo = Target::Sprite(EntityId(7));
    let call = |progress| ActionView {
        verb: Verb::Call,
        ..rest(progress)
    };
    let to_call = |verb, progress| ActionView {
        remembered: true,
        heard: Some(Pos { x: 30, y: 4 }),
        egg_at: None,
        ..aimed(verb, bo, 101, progress)
    };
    let tried = |view: ActionView| ActionView {
        attempted: true,
        ..view
    };
    let cases = [
        (
            call(Resting { ticks: 3, of: 10 }),
            "Calling",
            "CALL · 3 of 10 ticks",
        ),
        (call(Ended(Applied)), "Called", "CALL · applied"),
        (
            aimed(Verb::Court, bo, 101, Walking { steps_left: 2 }),
            "Courting Sprite #7 · 2 tiles to go",
            "COURT → sprite #7 · walking (2 tiles)",
        ),
        (
            aimed(Verb::Court, bo, 101, Ended(Applied)),
            "Danced for Sprite #7",
            "COURT → sprite #7 · applied",
        ),
        (
            to_call(Verb::Approach, Walking { steps_left: 24 }),
            "Going to Sprite #7's call · 24 tiles to go",
            "APPROACH → sprite #7 · by ear · walking (24 tiles)",
        ),
        (
            to_call(Verb::Court, Waiting { blocked_ticks: 2 }),
            "Going to Sprite #7's call · waiting to get past",
            "COURT → sprite #7 · by ear · blocked (2 ticks)",
        ),
        (
            aimed(Verb::Mate, bo, 101, Walking { steps_left: 1 }),
            "Going to pair with Sprite #7 · 1 tile to go",
            "MATE → sprite #7 · walking (1 tile)",
        ),
        (
            aimed(Verb::Mate, bo, 101, Ended(Applied)),
            "Paired with Sprite #7",
            "MATE → sprite #7 · applied",
        ),
        (
            tried(aimed(Verb::Mate, bo, 101, Ended(Failed))),
            "Couldn't pair with Sprite #7",
            "MATE → sprite #7 · failed",
        ),
    ];
    for (view, plain, exact) in cases {
        assert_eq!(action_line(&view, false, &pack()), plain, "{view:?}");
        assert_eq!(action_line(&view, true, &pack()), exact, "{view:?}");
    }
    let danced = aimed(Verb::Court, bo, 101, Ended(Applied));
    assert_eq!(
        observed_line(&danced, &pack()),
        "Danced for Sprite #7",
        "observed"
    );
    assert_eq!(
        done_to_line(EntityId(3), &danced, &pack()).as_deref(),
        Some("Sprite #3 danced for it")
    );
    assert_eq!(observed_line(&call(Ended(Applied)), &pack()), "Called");
}

#[test]
fn the_action_line_says_a_parent_goes_back_to_its_egg() {
    // M2 design §15.8.
    use Progress::*;
    let back = |progress| ActionView {
        remembered: true,
        egg_at: Some(Pos { x: 30, y: 4 }),
        ..aimed(Verb::Approach, Target::Object(EntityId(9)), 103, progress)
    };
    let view = back(Walking { steps_left: 12 });
    assert_eq!(
        action_line(&view, false, &pack()),
        "Going back to its egg · 12 tiles to go"
    );
    assert_eq!(
        action_line(&view, true, &pack()),
        "APPROACH → egg #9 · from memory · walking (12 tiles)"
    );
    let view = back(Waiting { blocked_ticks: 2 });
    assert_eq!(
        action_line(&view, false, &pack()),
        "Going back to its egg · waiting to get past"
    );
}

#[test]
fn the_event_log_says_who_are_partners_and_who_lost_or_parted_from_one() {
    // M2 design §15.8.
    let (pip, bo) = (EntityId(12), EntityId(7));
    let line = |kind| event_line(&Event { tick: 1, kind }, &pack());
    assert_eq!(
        line(EventKind::Paired { a: pip, b: bo }).as_deref(),
        Some("Sprite #12 and Sprite #7 are now partners")
    );
    assert_eq!(
        line(EventKind::LostPartner {
            id: pip,
            partner: bo
        })
        .as_deref(),
        Some("Sprite #12 lost Sprite #7")
    );
    assert_eq!(
        line(EventKind::Parted { a: pip, b: bo }).as_deref(),
        Some("Sprite #12 and Sprite #7 parted")
    );
}

#[test]
fn the_event_log_keeps_an_attempt_that_hurt_even_if_it_failed() {
    use Outcome::*;
    use Progress::Ended;
    let thorns = |verb, o| aimed(verb, Target::Object(EntityId(77)), 3, Ended(o));
    let me = EntityId(3);
    assert_eq!(
        logged_line(me, &tried_and_hurt(thorns(Verb::Eat, Failed)), &pack()).as_deref(),
        Some("Sprite #3 tried to eat a thornbush and got hurt")
    );
    let unhurt = ActionView {
        attempted: true,
        ..thorns(Verb::Hit, Failed)
    };
    assert_eq!(
        logged_line(me, &unhurt, &pack()),
        None,
        "a failed hit that hurt nobody"
    );
}

#[test]
fn a_finished_action_is_observed_in_the_past_tense_saying_how_it_went_if_badly() {
    use Outcome::*;
    use Progress::Ended;
    let bush = |o| aimed(Verb::Eat, Target::Object(EntityId(812)), 1, Ended(o));
    let berry = |o| aimed(Verb::Eat, Target::Object(EntityId(9)), 2, Ended(o));
    let water = |o| {
        aimed(
            Verb::Drink,
            Target::Water(Pos { x: 4, y: 1 }),
            100,
            Ended(o),
        )
    };
    let sprite = |o| aimed(Verb::Approach, Target::Sprite(EntityId(530)), 101, Ended(o));
    let tried = |view: ActionView| ActionView {
        attempted: true,
        ..view
    };
    let gone = |view: ActionView| ActionView {
        target_gone: true,
        ..view
    };
    let cases = [
        (wander(Ended(Applied)), "Wandered off"),
        (
            wander(Ended(Blocked)),
            "Wandered off, but gave up: the way was blocked",
        ),
        (
            wander(Ended(TimedOut)),
            "Wandered off, but gave up: it took too long",
        ),
        (
            wander(Ended(Failed)),
            "Wandered off, but gave up: it couldn't get there",
        ),
        (rest(Ended(Applied)), "Rested"),
        (rest(Ended(Interrupted)), "Rested, but changed its mind"),
        (rest(Ended(PulledAway)), "Rested, but was pulled away"),
        (
            bush(PulledAway),
            "Went to eat the berry bush, but was pulled away",
        ),
        (tried(bush(Applied)), "Ate from the berry bush"),
        (gone(tried(berry(Applied))), "Ate the berry"),
        (
            tried(bush(Failed)),
            "Went to eat the berry bush, but it was empty",
        ),
        (
            bush(Interrupted),
            "Went to eat the berry bush, but changed its mind",
        ),
        (
            gone(berry(Failed)),
            "Went to eat the berry, but it was gone",
        ),
        (tried(water(Applied)), "Drank"),
        (
            water(Failed),
            "Went to drink, but gave up: it couldn't get there",
        ),
        (
            tried(bush(TooFull)),
            "Went to eat the berry bush, but was too full",
        ),
        (tried(water(TooFull)), "Went to drink, but was too full"),
        (tried(sprite(Applied)), "Went over to Sprite #530"),
        (
            gone(sprite(Failed)),
            "Went over to Sprite #530, but Sprite #530 was gone",
        ),
    ];
    for (view, line) in cases {
        assert_eq!(observed_line(&view, &pack()), line, "{view:?}");
    }
}

#[test]
fn a_scored_name_too_long_for_its_row_wraps_under_itself() {
    let name = concept_name(&[
        ("attended_sprite", true),
        ("target_adjacent", true),
        ("hunger", false),
    ]);
    let lines = scored("   ", &name, "+.40");
    assert_eq!(
        lines,
        [
            "   not attended sprite &                +.40",
            "     not target adjacent & hunger",
        ],
        "each input kept whole"
    );
    assert!(lines.iter().all(|l| l.chars().count() < WIDTH));
}

#[test]
fn a_wrapped_line_has_the_room_the_number_leaves_on_the_first() {
    let name = concept_name(&[
        ("attended_sprite", true),
        ("target_distance", true),
        ("tiredness", true),
    ]);
    // The second row is 35 columns, which only fits because no number
    // follows it.
    assert_eq!(
        scored("   ", &name, "+.40"),
        [
            "   not attended sprite &                +.40",
            "     not target distance & not tiredness",
        ]
    );
}

// Levels never leave 0 to 1 (design §4.4), but a bar mustn't crash the
// screen if one ever did.
#[test]
fn a_drive_bar_is_ten_cells_whatever_the_level() {
    assert_eq!(bar_parts(0.42).concat(), "████░░░░░░");
    assert_eq!(bar_parts(1.05).concat(), "██████████");
    assert_eq!(bar_parts(-0.3).concat(), "░░░░░░░░░░");
    assert_eq!(bar_parts(f32::NAN).concat(), "░░░░░░░░░░");
}

/// A gene of every kind, the words lit on its line, which a tooltip
/// explains, and what its tooltip says it does (#304).
fn genes_of_every_kind() -> Vec<(terra_sim::GeneView<'static>, &'static str, &'static str)> {
    use terra_sim::{EmitterMode, GeneView, Trait, Whom};
    let emitter = |mode, invert, threshold, gain| GeneView::Emitter {
        locus: "injury",
        mode,
        invert,
        threshold,
        gain,
        chem: "pain",
    };
    vec![
        (
            emitter(EmitterMode::Level, true, 0.3, 0.004),
            "low injury → pain",
            "An emitter: pain rises while injury is below .7",
        ),
        (
            emitter(EmitterMode::Level, false, 0.6, -0.1),
            "injury → pain",
            "An emitter: pain falls while injury is above .6",
        ),
        (
            emitter(EmitterMode::Level, false, 0.0, 0.3),
            "injury → pain",
            "An emitter: injury raises pain",
        ),
        (
            emitter(EmitterMode::Rise, false, 0.0, 10.0),
            "injury rises → pain",
            "An emitter: pain rises as injury rises",
        ),
        (
            emitter(EmitterMode::Fall, false, 0.1, -1.0),
            "injury falls → pain",
            "An emitter: pain falls as injury falls, once past .1",
        ),
        (
            GeneView::HalfLife {
                chem: "hunger",
                ticks: 2_000,
            },
            "hunger halves",
            "A half-life: hunger fades by itself, by half every 2,000 ticks",
        ),
        (
            GeneView::Reaction {
                reactants: vec![("hunger", 1), ("food", 1)],
                products: vec![("food", 1)],
                rate: 0.1,
            },
            "hunger + food → food",
            "A reaction: each tick, hunger + food turns into food",
        ),
        (
            GeneView::Receptor {
                chem: "h0",
                threshold: 0.0,
                gain: -2.0,
                target: "curiosity_mod",
            },
            "h0 → curiosity mod",
            "A receptor: h0 lowers its curiosity mod",
        ),
        (
            GeneView::InitialConcentration {
                chem: "h4",
                value: 0.5,
            },
            "h4 starts at",
            "A starting level: h4 starts at .5 when it hatches",
        ),
        (
            GeneView::BrainParam {
                param: "tau_base",
                value: 0.1,
            },
            "tau base",
            "A brain setting: its tau base is .1, one of the numbers its brain works by",
        ),
        (
            GeneView::Instinct {
                inputs: vec![("thirst", false), ("target_adjacent", true)],
                verb: Verb::Drink,
                weight: 0.5,
            },
            "thirst & not target adjacent → drink",
            "An instinct: thirst & not target adjacent leans it towards drink",
        ),
        (
            GeneView::AttentionInstinct {
                input: "arousal",
                category: "sprite",
                whom: Whom::Kin,
                weight: -1.0,
            },
            "arousal → attends to its kin (sprite)",
            "An attention instinct: arousal draws its eye away from its kin (sprite)",
        ),
        (
            GeneView::Trait {
                which: Trait::GrowsUp,
                value: 4_500.0,
            },
            "grows up",
            "A trait: grows up is the age it grows up at",
        ),
        (
            GeneView::Unknown {
                type_id: 40,
                version: 1,
                bytes: 3,
            },
            "type 40, version 1, 3 bytes",
            "A gene this version of the game can't read",
        ),
    ]
}

#[test]
fn a_gene_says_what_it_does_in_a_sentence_of_its_kind() {
    for (gene, _, words) in genes_of_every_kind() {
        assert_eq!(gene_about(&gene), words, "{gene:?}");
    }
}

#[test]
fn every_kind_of_gene_has_the_words_of_its_line_but_its_numbers_as_its_spot() {
    // #304: what's lit on the Genome tab, and what a tooltip explains, one
    // spot over the name's words and what's between them.
    for (gene, lit, _) in genes_of_every_kind() {
        let name = genome::name_on_its_line(&gene);
        let lines = wrapped(&genome::gene_text(&gene), 1, Style::new(), Style::new());
        let mut spotted = Vec::new();
        for row in genome::name_spotted(lines, &name, 7, false) {
            let drawn: Vec<char> = row.line.to_string().chars().collect();
            for (columns, spot) in row.spots {
                assert_eq!(spot, Spot::Gene(7), "{gene:?}");
                let columns = usize::from(columns.start)..usize::from(columns.end);
                spotted.push(drawn[columns].iter().collect::<String>());
            }
        }
        assert_eq!(spotted, [lit], "{gene:?}");
    }
}
