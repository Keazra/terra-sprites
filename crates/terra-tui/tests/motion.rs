//! Motion (design §6.9, #306): small effects drawn over the finished
//! screen in real time, with one switch that turns them all off.
//!
//! The shared constructors in `common` turn motion off, so the other tests
//! keep their pictures; these turn it on.

mod common;
use common::{
    SCORCHED, SCREEN, WorldBuilder, app_on, apply, at, ball_world, died, field, frame, lone_line,
    pack_changing, pointer_on, render, scratch_folder, sprite_on,
};

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use terra_sim::{DeathCause, EntityId, Event, EventKind, Pos, Terrain, World};
use terra_tui::app::{App, ColourMode, Ticks};
use terra_tui::args::{Args, USAGE};
use terra_tui::campbell::{self, FOREGROUND};
use terra_tui::clock::Speed;
use terra_tui::input::Action;
use terra_tui::policy::{InfoPolicy, Panel, Subject};
use terra_tui::theme::{self, MotionLook, Theme};
use terra_tui::ui;

#[test]
fn motion_is_on_in_a_new_app_and_its_switch_turns_it_off() {
    // Design §6.9: on by default, for every world the game opens.
    let world = common::field(&[], &[]);
    let mut app = App::new(world.map(), Theme::cp437(), 7, ui::areas(SCREEN));
    assert!(app.motion());
    app.set_motion(false);
    assert!(!app.motion());
    app.set_motion(true);
    assert!(app.motion());
    // The tests' own apps start with it off, so their pictures stay still.
    assert!(!app_on(&world).motion());
}

#[test]
fn no_motion_starts_the_game_with_motion_off() {
    // Design §6.7: `--no-motion`, with a world of any kind, and the usage
    // says so.
    let parse = |args: &[&str]| Args::parse(args.iter().map(|arg| arg.to_string()));
    assert!(!parse(&[]).expect("valid").no_motion);
    for flags in [
        &["--no-motion"][..],
        &["--seed", "7", "--no-motion"],
        &["--no-motion", "--replay", "last_session.replay"],
        &["--load", "quicksave", "--no-motion"],
    ] {
        assert!(parse(flags).expect("valid").no_motion, "{flags:?}");
    }
    assert!(USAGE.contains("--no-motion"), "{USAGE}");
}

#[test]
fn a_load_keeps_the_switch_as_the_player_left_it() {
    // Design §6.7: what's the player's rather than the world's carries
    // over a load, and motion's switch is the player's.
    for on in [false, true] {
        let folder = scratch_folder(&format!("load-keeps-{on}"));
        let world = ball_world();
        let mut app = app_on(&world);
        app.set_save_folder(folder);
        app.set_motion(on);
        apply(&mut app, &world, Action::Quicksave);
        apply(&mut app, &world, Action::Quickload);
        app.take_loaded().expect("the quicksave loaded");
        assert_eq!(app.motion(), on);
    }
}

/// The ascii theme's file with `section` added at its end.
fn ascii_and(section: &str) -> String {
    let end = ASCII_THEME.rfind(')').expect("the theme's end");
    format!("{}{section}\n{}", &ASCII_THEME[..end], &ASCII_THEME[end..])
}

const ASCII_THEME: &str = include_str!("../../../themes/ascii.ron");

#[test]
fn the_default_theme_gives_motion_its_exact_colours_and_ascii_takes_them() {
    // Design §6.9: a theme's `motion` section, which the ASCII theme leaves
    // out, so it uses the default theme's.
    let rgb = |hex: u32| Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8);
    let expected = MotionLook {
        flash: rgb(0xd0d0d0),
        ash: rgb(0x5c5c5c),
        spark: rgb(0xffe9a0),
        trail: rgb(0x2b3047),
        dark: rgb(0x0c0c0c),
        shallow_water: [rgb(0x3a96dd), rgb(0x62b4f0)],
        deep_water: [rgb(0x0037da), rgb(0x2f62f2)],
    };
    assert_eq!(Theme::cp437().motion(), expected);
    assert_eq!(Theme::ascii().motion(), expected);
    assert_eq!(
        expected.water(Terrain::DeepWater),
        Some([rgb(0x0037da), rgb(0x2f62f2)])
    );
    assert_eq!(expected.water(Terrain::Grass), None);
}

#[test]
fn a_theme_s_motion_section_may_leave_colours_out_and_name_them() {
    // What a theme leaves out of the section, it draws as the default theme
    // does (design §6.9), and a colour the game doesn't know is refused.
    let pack = common::pack();
    let theme = Theme::from_ron(
        &ascii_and(r##"    motion: (ash: gray, deep_water: (blue, "#2020ff")),"##),
        &pack,
    )
    .expect("loads");
    let (ours, default) = (theme.motion(), Theme::cp437().motion());
    assert_eq!(ours.ash, Color::Gray);
    assert_eq!(ours.deep_water, [Color::Blue, Color::Rgb(0x20, 0x20, 0xff)]);
    assert_eq!(ours.flash, default.flash);
    assert_eq!(ours.shallow_water, default.shallow_water);
    let err =
        Theme::from_ron(&ascii_and("    motion: (glow: white),"), &pack).expect_err("refused");
    assert!(err.contains("glow"), "{err}");
}

#[test]
fn each_water_s_two_shades_stay_apart_in_256_colours() {
    // Design §6.2: where exact colours aren't shown, each is drawn as the
    // nearest of 256, and water must still drift between two there.
    let motion = Theme::cp437().motion();
    for [from, to] in [motion.shallow_water, motion.deep_water] {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 2, 1));
        buffer[(0, 0)].set_fg(from);
        buffer[(1, 0)].set_fg(to);
        theme::in_256_colours(&mut buffer);
        assert_ne!(buffer[(0, 0)].fg, buffer[(1, 0)].fg, "{from:?} and {to:?}");
    }
}

/// `n` milliseconds.
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

/// An app on `world` on a 100×30 screen, with motion on and every sprite
/// in its own colour.
fn moving(world: &World) -> App {
    let mut app = app_on(world);
    app.set_motion(true);
    plain(&mut app, world);
    app
}

/// Draws every sprite in its own colour, whatever its drives (design §6.3).
fn plain(app: &mut App, world: &World) {
    while app.colour_mode() != ColourMode::Plain {
        apply(app, world, Action::CycleColours);
    }
}

/// Speeds `app`'s clock up to 4×, where motion thins out (design §6.9).
fn at_4x(app: &mut App) {
    while app.clock.speed() < Speed::X4 {
        app.clock.faster();
    }
}

/// The first screen cell of `tile` as `app` draws it.
fn cell(app: &App, tile: Pos) -> Position {
    app.cell_of(tile).expect("in view")
}

/// Each screen cell of `tile` as `app` draws it: two while tiles are two
/// cells wide.
fn cells(app: &App, tile: Pos) -> Vec<Position> {
    let first = cell(app, tile);
    (0..app.tile_width())
        .map(|x| Position::new(first.x + x, first.y))
        .collect()
}

/// Moves the Cursor onto `tile`, as pointing at it does.
fn cursor_to(app: &mut App, world: &World, tile: Pos) {
    let pointer = pointer_on(app, tile);
    apply(app, world, Action::Point(pointer));
    assert_eq!(app.cursor(), tile);
}

/// White, a sprite's own colour in the default theme, as Campbell draws it
/// once motion blends it.
const WHITE: Color = Color::Rgb(0xF2, 0xF2, 0xF2);

// Water (design §6.9).

/// A 12×6 world, grass to the west of a lake: shallow water at its shore
/// and deep in its middle.
fn lake() -> WorldBuilder<'static> {
    WorldBuilder::new(&[
        "...~~~~~~~~~",
        "...~~=====~~",
        "...~~=====~~",
        "...~~=====~~",
        "...~~=====~~",
        "...~~~~~~~~~",
    ])
}

/// The colour each of `world`'s tiles of `terrain` is drawn in on `screen`.
fn colours_of(app: &App, world: &World, screen: &Buffer, terrain: Terrain) -> Vec<Color> {
    let map = world.map();
    let mut colours = Vec::new();
    for y in 0..map.height() {
        for x in 0..map.width() {
            if map.terrain(at(x, y)) == terrain {
                colours.push(screen[cell(app, at(x, y))].fg);
            }
        }
    }
    colours
}

#[test]
fn water_drifts_between_its_two_shades_in_waves_across_the_lake() {
    let world = lake().build();
    let mut app = moving(&world);
    cursor_to(&mut app, &world, at(0, 0));
    let motion = app.theme.motion();
    // A frame every tenth of a second, over a cycle and a bit.
    let mut screens = vec![frame(&mut app, &world, Duration::ZERO)];
    for _ in 0..45 {
        screens.push(frame(&mut app, &world, ms(100)));
    }
    for (terrain, ends) in [
        (Terrain::ShallowWater, motion.shallow_water),
        (Terrain::DeepWater, motion.deep_water),
    ] {
        let at_each: Vec<Vec<Color>> = screens
            .iter()
            .map(|screen| colours_of(&app, &world, screen, terrain))
            .collect();
        let shades: BTreeSet<String> = at_each.iter().flatten().map(|c| format!("{c:?}")).collect();
        // A few fixed shades, from one end to the other.
        assert!((6..=8).contains(&shades.len()), "{terrain:?}: {shades:?}");
        for end in ends {
            assert!(shades.contains(&format!("{end:?}")), "{terrain:?}: {end:?}");
        }
        assert!(
            at_each
                .iter()
                .flatten()
                .all(|c| matches!(c, Color::Rgb(..))),
            "exact while motion is on"
        );
        // In waves, not the whole lake at once: at any moment, more than
        // one shade.
        let now: BTreeSet<String> = at_each[0].iter().map(|c| format!("{c:?}")).collect();
        assert!(now.len() > 1, "{terrain:?}: {now:?}");
        // A 4 s cycle.
        assert_eq!(at_each[40], at_each[0], "{terrain:?}");
        assert_ne!(at_each[20], at_each[0], "{terrain:?}");
    }
    // The waves roll across the lake: a moment later, each tile of the
    // shore's top row shows what the tile west of it showed.
    let row = |screen: &Buffer| -> Vec<Color> {
        (3..12).map(|x| screen[cell(&app, at(x, 0))].fg).collect()
    };
    let first = row(&screens[0]);
    assert!(
        screens[1..20]
            .iter()
            .any(|screen| row(screen)[1..] == first[..first.len() - 1]),
        "{first:?}"
    );
}

#[test]
fn water_shimmers_only_where_bare_water_shows() {
    // Not under a sprite, an object, a mark or the Cursor (design §6.9).
    let world = lake().starters(&[at(4, 0)]).build();
    let mut app = moving(&world);
    cursor_to(&mut app, &world, at(3, 5));
    for screen in [
        frame(&mut app, &world, Duration::ZERO),
        frame(&mut app, &world, ms(1300)),
    ] {
        for at_cell in cells(&app, at(4, 0)) {
            let fg = screen[at_cell].fg;
            assert!(!matches!(fg, Color::Rgb(..)), "the sprite's tile: {fg:?}");
        }
        for at_cell in cells(&app, at(3, 5)) {
            assert_eq!(screen[at_cell].fg, Color::Cyan, "the Cursor's tile");
        }
        assert!(matches!(screen[cell(&app, at(5, 0))].fg, Color::Rgb(..)));
    }
}

#[test]
fn with_motion_off_water_keeps_its_named_colours() {
    let world = lake().build();
    let mut app = app_on(&world);
    cursor_to(&mut app, &world, at(0, 0));
    for screen in [
        frame(&mut app, &world, Duration::ZERO),
        frame(&mut app, &world, ms(1700)),
    ] {
        let shallow = colours_of(&app, &world, &screen, Terrain::ShallowWater);
        let deep = colours_of(&app, &world, &screen, Terrain::DeepWater);
        assert!(shallow.iter().all(|&c| c == Color::Cyan), "{shallow:?}");
        assert!(deep.iter().all(|&c| c == Color::Blue), "{deep:?}");
    }
}

// Deaths (design §6.9).

/// A 10×6 field with sprites on `tiles`, an app with motion on that has
/// taken in a tick of it, their IDs, and the same field with them gone, as
/// after they died: the world removes a dead sprite before it says so.
fn before_and_after(tiles: &[Pos]) -> (App, Vec<EntityId>, World) {
    let before = field(&[], tiles);
    let ids = tiles.iter().map(|&tile| sprite_on(&before, tile)).collect();
    let mut app = moving(&before);
    app.record(&[], &before);
    (app, ids, field(&[], &[]))
}

/// Each of `ids` died of thirst, in that order, on tick 1.
fn deaths(ids: &[EntityId]) -> Vec<Event> {
    ids.iter()
        .map(|&id| died(1, id, DeathCause::Dehydration, 30))
        .collect()
}

/// Whether `tile`'s background flashes on `screen`, as a death's does at
/// once.
fn flashing(app: &App, screen: &Buffer, tile: Pos) -> bool {
    screen[cell(app, tile)].bg == app.theme.motion().flash
}

#[test]
fn a_death_flashes_its_tile_and_leaves_a_ghost_fading_to_ash() {
    let (mut app, ids, after) = before_and_after(&[at(3, 2)]);
    app.record(&deaths(&ids), &after);
    let motion = app.theme.motion();
    let first = cell(&app, at(3, 2));
    let screen = frame(&mut app, &after, Duration::ZERO);
    for at_cell in cells(&app, at(3, 2)) {
        assert_eq!(screen[at_cell].bg, motion.flash);
    }
    // The ghost: its glyph, in the colour it was drawn in.
    assert_eq!((screen[first].symbol(), screen[first].fg), ("☺", WHITE));
    // Fast and then slowing: well past half way back at half time.
    let half = frame(&mut app, &after, ms(600));
    let Color::Rgb(red, ..) = half[first].bg else {
        panic!("{:?}", half[first].bg)
    };
    assert!(red > 0x0C && red < (0xD0 + 0x0C) / 2, "{red:#x}");
    assert_eq!(half[first].symbol(), "☺");
    let fg = half[first].fg;
    assert!(fg != WHITE && fg != motion.ash, "{fg:?}");
    // Over after 1.2 s: the grass as it was.
    let over = frame(&mut app, &after, ms(600));
    assert_eq!((over[first].symbol(), over[first].bg), (".", Color::Reset));
    assert_eq!(over[first].fg, Color::Green);
}

#[test]
fn a_death_fades_to_ash_by_its_end() {
    let (mut app, ids, after) = before_and_after(&[at(3, 2)]);
    app.record(&deaths(&ids), &after);
    let first = cell(&app, at(3, 2));
    let ash = app.theme.motion().ash;
    let near_end = frame(&mut app, &after, ms(1190));
    let Color::Rgb(red, ..) = near_end[first].fg else {
        panic!("{:?}", near_end[first].fg)
    };
    let Color::Rgb(ash_red, ..) = ash else {
        panic!("{ash:?}")
    };
    assert!(red.abs_diff(ash_red) <= 1, "{red:#x}");
}

#[test]
fn at_4x_a_death_is_over_in_half_the_time() {
    let (mut app, ids, after) = before_and_after(&[at(3, 2)]);
    at_4x(&mut app);
    app.record(&deaths(&ids), &after);
    let first = cell(&app, at(3, 2));
    let playing = frame(&mut app, &after, ms(550));
    assert_eq!(playing[first].symbol(), "☺");
    let over = frame(&mut app, &after, ms(50));
    assert_eq!((over[first].symbol(), over[first].bg), (".", Color::Reset));
}

#[test]
fn at_most_four_deaths_play_at_once_the_newest_taking_the_oldest_s_place() {
    let tiles = [at(1, 1), at(3, 1), at(5, 1), at(7, 1), at(1, 4)];
    let (mut app, ids, after) = before_and_after(&tiles);
    app.record(&deaths(&ids), &after);
    let screen = frame(&mut app, &after, Duration::ZERO);
    let shown: Vec<Pos> = tiles
        .into_iter()
        .filter(|&tile| flashing(&app, &screen, tile))
        .collect();
    assert_eq!(shown, tiles[1..]);
    // Two at 4× and up.
    let (mut app, ids, after) = before_and_after(&tiles);
    at_4x(&mut app);
    app.record(&deaths(&ids), &after);
    let screen = frame(&mut app, &after, Duration::ZERO);
    let shown: Vec<Pos> = tiles
        .into_iter()
        .filter(|&tile| flashing(&app, &screen, tile))
        .collect();
    assert_eq!(shown, tiles[3..]);
}

#[test]
fn each_death_of_a_frame_s_ticks_finds_where_its_sprite_was() {
    // The screen notes the sprites once the frame's ticks are all taken
    // in, so a death on its second tick still finds its sprite.
    let (mut app, ids, after) = before_and_after(&[at(2, 2), at(6, 2)]);
    let mut ticks = Ticks::default();
    ticks.note(vec![died(1, ids[0], DeathCause::Dehydration, 30)], &after);
    ticks.note(vec![died(2, ids[1], DeathCause::Starvation, 31)], &after);
    app.take_in(ticks, &after);
    let screen = frame(&mut app, &after, Duration::ZERO);
    assert!(flashing(&app, &screen, at(2, 2)));
    assert!(flashing(&app, &screen, at(6, 2)));
}

#[test]
fn a_death_the_screen_never_saw_or_of_a_living_sprite_does_no_harm() {
    // Never seen: the app took in no tick with it in the world.
    let world = field(&[], &[at(3, 2)]);
    let id = sprite_on(&world, at(3, 2));
    let mut app = moving(&field(&[], &[]));
    let empty = field(&[], &[]);
    app.record(&deaths(&[id]), &empty);
    app.record(
        &[Event {
            tick: 1,
            kind: EventKind::Hatched {
                id,
                parents: Vec::new(),
            },
        }],
        &empty,
    );
    let screen = frame(&mut app, &empty, Duration::ZERO);
    assert!(!flashing(&app, &screen, at(3, 2)));
    // Still in the world: drawn as it is.
    let mut app = moving(&world);
    app.record(&[], &world);
    app.record(&deaths(&[id]), &world);
    let screen = frame(&mut app, &world, ms(100));
    assert_eq!(screen[cell(&app, at(3, 2))].symbol(), "☺");
}

#[test]
fn a_ghost_is_drawn_only_while_its_tile_shows_bare_ground() {
    let before = field(&[], &[at(3, 2)]);
    let id = sprite_on(&before, at(3, 2));
    let mut app = moving(&before);
    app.record(&[], &before);
    // A ball rolled onto the tile as the sprite died.
    let after = field(&[(at(3, 2), "ball")], &[]);
    app.record(&deaths(&[id]), &after);
    let screen = frame(&mut app, &after, Duration::ZERO);
    let ball = app.theme.object_glyph("ball", "default");
    let first = cell(&app, at(3, 2));
    assert_eq!(screen[first].symbol(), ball.symbol.to_string());
    assert_eq!(screen[first].fg, ball.fg);
    assert!(flashing(&app, &screen, at(3, 2)));
}

/// A policy that shows everything but emotes.
struct NoEmotes;

impl InfoPolicy for NoEmotes {
    fn can_view(&self, panel: Panel, _subject: Subject) -> bool {
        panel != Panel::Emotes
    }
}

#[test]
fn a_death_asks_the_policy_first() {
    // Design §6.4: what it tells the player is about its sprite, and asks
    // `Emotes` about it.
    let (mut app, ids, after) = before_and_after(&[at(3, 2)]);
    app.set_policy(NoEmotes);
    app.record(&deaths(&ids), &after);
    let screen = frame(&mut app, &after, Duration::ZERO);
    let first = cell(&app, at(3, 2));
    assert_eq!(
        (screen[first].symbol(), screen[first].bg),
        (".", Color::Reset)
    );
}

#[test]
fn with_motion_off_a_death_is_drawn_as_before() {
    let (mut app, ids, after) = before_and_after(&[at(3, 2)]);
    app.record(&deaths(&ids), &after);
    app.set_motion(false);
    let screen = frame(&mut app, &after, Duration::ZERO);
    let first = cell(&app, at(3, 2));
    assert_eq!(
        (screen[first].symbol(), screen[first].bg),
        (".", Color::Reset)
    );
    // Turned on again, it doesn't pick up where it was.
    app.set_motion(true);
    let screen = frame(&mut app, &after, Duration::ZERO);
    assert!(!flashing(&app, &screen, at(3, 2)));
}

#[test]
fn a_young_sprite_dying_in_play_leaves_its_own_ghost() {
    // In a world run tick by tick, as the game does: the screen notes where
    // each sprite is after each tick it takes in (design §6.9).
    let mut world = WorldBuilder::grass(10, 6)
        .starters(&[at(2, 1)])
        .newborns(&[at(2, 1)])
        .data(pack_changing(&[SCORCHED]))
        .build();
    let mut app = moving(&world);
    // Out of the way: the Cursor's marks are drawn over the tiles round it.
    cursor_to(&mut app, &world, at(9, 5));
    let mut last_seen = None;
    for _ in 0..200 {
        let before = world.sprites().next().map(|sprite| sprite.pos());
        let events = world.step();
        app.record(&events, &world);
        if events
            .iter()
            .any(|event| matches!(event.kind, EventKind::Died { .. }))
        {
            last_seen = before;
            break;
        }
    }
    let tile = last_seen.expect("it died of thirst");
    let screen = frame(&mut app, &world, Duration::ZERO);
    let first = cell(&app, tile);
    assert!(flashing(&app, &screen, tile));
    assert_eq!((screen[first].symbol(), screen[first].fg), ("ö", WHITE));
}

// Hatchings (design §6.9).

/// Two apps on `world`, alike but for motion, on as in `moving` and off.
fn twins(world: &World) -> (App, App) {
    let mut off = app_on(world);
    plain(&mut off, world);
    (moving(world), off)
}

/// Runs `world` until an egg hatches, both apps taking in each tick: the
/// young's ID. A hatchling often gives up on what it first tries, and its
/// emote takes turns with its glyph (design §6.3).
fn until_a_hatching(world: &mut World, on: &mut App, off: &mut App) -> EntityId {
    for _ in 0..400 {
        let events = world.step();
        on.record(&events, world);
        off.record(&events, world);
        if let Some(id) = events.iter().find_map(|event| match event.kind {
            EventKind::Hatched { id, .. } => Some(id),
            _ => None,
        }) {
            return id;
        }
    }
    panic!("no egg hatched");
}

/// Whether `colour` is exact and near `to`, a little way into a fade from
/// it.
fn near(colour: Color, to: Color) -> bool {
    match (colour, to) {
        (Color::Rgb(r, g, b), Color::Rgb(r2, g2, b2)) => {
            r.abs_diff(r2) <= 24 && g.abs_diff(g2) <= 24 && b.abs_diff(b2) <= 24
        }
        _ => false,
    }
}

/// The tiles round `tile`, and it, that are on the map.
fn round(world: &World, tile: Pos) -> Vec<Pos> {
    let map = world.map();
    let mut tiles = Vec::new();
    for y in tile.y.saturating_sub(1)..=(tile.y + 1).min(map.height() - 1) {
        for x in tile.x.saturating_sub(1)..=(tile.x + 1).min(map.width() - 1) {
            tiles.push(at(x, y));
        }
    }
    tiles
}

#[test]
fn a_hatching_sparks_round_the_egg_at_scattered_moments() {
    let mut world = lone_line(0);
    let (mut on, mut off) = twins(&world);
    let young = until_a_hatching(&mut world, &mut on, &mut off);
    let egg = world.sprite(young).expect("hatched").pos();
    let spark = on.theme.motion().spark;
    let tiles = round(&world, egg);
    // Which frames each tile is lit in, a frame every 1/30 s.
    let mut lit: BTreeMap<Pos, Vec<u32>> = BTreeMap::new();
    let mut sparked = false;
    let (mut shown, mut plain) = (
        frame(&mut on, &world, Duration::ZERO),
        frame(&mut off, &world, Duration::ZERO),
    );
    for n in 0..30 {
        for &tile in &tiles {
            let differs = cells(&on, tile)
                .into_iter()
                .any(|at_cell| shown[at_cell].fg != plain[at_cell].fg);
            if differs {
                lit.entry(tile).or_default().push(n);
            }
            sparked |= cells(&on, tile)
                .into_iter()
                .any(|at_cell| near(shown[at_cell].fg, spark));
        }
        shown = frame(&mut on, &world, ms(33));
        plain = frame(&mut off, &world, ms(33));
    }
    // Each lights up, but not all at once.
    let all: BTreeSet<Pos> = tiles.into_iter().collect();
    assert_eq!(lit.keys().copied().collect::<BTreeSet<_>>(), all, "{lit:?}");
    let firsts: BTreeSet<u32> = lit.values().map(|frames| frames[0]).collect();
    assert!(firsts.len() >= 3, "{lit:?}");
    // In the theme's colour for sparks, at their brightest.
    assert!(sparked);
    // Over after 1 s: drawn as with motion off.
    let after = frame(&mut on, &world, ms(100));
    let without = frame(&mut off, &world, ms(100));
    assert_eq!(after, without);
}

#[test]
fn a_hatched_young_warms_from_the_egg_s_colour_to_its_own() {
    let mut world = lone_line(0);
    let (mut on, mut off) = twins(&world);
    // One that isn't emoting, so its own glyph shows.
    let young = loop {
        let young = until_a_hatching(&mut world, &mut on, &mut off);
        if on.emote(young).is_none() {
            break young;
        }
    };
    let first = cell(&on, world.sprite(young).expect("hatched").pos());
    let egg = campbell::exact(on.theme.object_glyph("egg", "default").fg, FOREGROUND);
    let start = frame(&mut on, &world, Duration::ZERO);
    assert_eq!(start[first].fg, egg);
    let quarter = frame(&mut on, &world, ms(250));
    let fg = quarter[first].fg;
    assert!(
        fg != egg && fg != WHITE && matches!(fg, Color::Rgb(..)),
        "{fg:?}"
    );
    let half = frame(&mut on, &world, ms(250));
    let plain = frame(&mut off, &world, ms(500));
    assert_eq!(half[first].fg, plain[first].fg);
}

#[test]
fn a_hatching_asks_the_policy_first() {
    let mut world = lone_line(0);
    let (mut on, mut off) = twins(&world);
    on.set_policy(NoEmotes);
    off.set_policy(NoEmotes);
    until_a_hatching(&mut world, &mut on, &mut off);
    for step in [0, 100, 300] {
        let shown = frame(&mut on, &world, ms(step));
        let plain = frame(&mut off, &world, ms(step));
        assert_eq!(shown, plain);
    }
}

#[test]
fn the_same_app_drawn_twice_draws_the_same_screen() {
    // Drawing stays pure (design §6.8): motion is drawn from when each
    // effect began, not stepped as it's drawn.
    let (mut app, ids, after) = before_and_after(&[at(3, 2)]);
    app.record(&deaths(&ids), &after);
    app.animate(ms(300));
    assert_eq!(render(&app, &after), render(&app, &after));
    let world = lake().build();
    let mut app = moving(&world);
    app.animate(ms(700));
    assert_eq!(render(&app, &world), render(&app, &world));
}
