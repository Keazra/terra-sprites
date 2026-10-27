//! Motion (design §6.9, #306): small effects drawn over the finished
//! screen in real time, with one switch that turns them all off.
//!
//! The shared constructors in `common` turn motion off, so the other tests
//! keep their pictures; these turn it on.

mod common;
use common::{
    SCORCHED, SCREEN, WorldBuilder, app_on, apply, at, ball_world, cell_of_text, died, field,
    frame, frame_on, lone_line, pack_changing, pointer_on, render, scratch_folder, select,
    sized_app, sprite_on,
};

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::Color;
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{DeathCause, EntityId, Event, EventKind, Pos, Terrain, World};
use terra_tui::app::{App, ColourMode, Screen, Tab, Ticks};
use terra_tui::args::{Args, USAGE};
use terra_tui::campbell::{self, FOREGROUND};
use terra_tui::clock::Speed;
use terra_tui::input::Action;
use terra_tui::policy::{InfoPolicy, Panel, Subject};
use terra_tui::start;
use terra_tui::theme::{self, MotionLook, Theme};
use terra_tui::title::Title;
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
    twins_in(world, Theme::cp437(), SCREEN)
}

/// Two apps on `world` in `theme` on a screen of `size`, alike but for
/// motion, every sprite in its own colour.
fn twins_in(world: &World, theme: Theme, size: Size) -> (App, App) {
    let [mut on, mut off] = [0, 1].map(|_| sized_app(world, theme.clone(), size));
    on.set_motion(true);
    plain(&mut on, world);
    plain(&mut off, world);
    (on, off)
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
    // A tab sweeping in, a pop-up fading in over it and a bar easing.
    let (before, after) = (hungry(&[(at(3, 2), 0.2)]), hungry(&[(at(3, 2), 0.8)]));
    let mut app = watching(&before);
    take_in(&mut app, &after);
    apply(&mut app, &after, Action::NextTab);
    apply(&mut app, &after, Action::PreviousTab);
    apply(&mut app, &after, Action::OpenThemes);
    app.animate(ms(60));
    let screen = render(&app, &after);
    assert_eq!(render(&app, &after), screen);
}

// A tab opened (design §6.9).

/// Where the open tab's lines are drawn in `app`'s inspector just now:
/// below its title row, and right of the column down its left.
fn tab_body(app: &App) -> Rect {
    let area = app.inspector_area().expect("the inspector shows");
    Rect::new(area.x + 1, area.y + 1, area.width - 1, area.height - 1)
}

/// The default theme's panel shade.
fn shade() -> Color {
    Theme::cp437()
        .panels()
        .expect("the default theme has shades")
        .shade
}

/// Both twins do `action`.
fn both(on: &mut App, off: &mut App, world: &World, action: Action) {
    apply(on, world, action);
    apply(off, world, action);
}

/// A frame of each twin on a screen of `size`, `step` on.
fn frames_on(
    on: &mut App,
    off: &mut App,
    world: &World,
    step: Duration,
    size: Size,
) -> (Buffer, Buffer) {
    (
        frame_on(on, world, step, size),
        frame_on(off, world, step, size),
    )
}

/// A frame of each twin on a 100×30 screen, `step` on.
fn frames(on: &mut App, off: &mut App, world: &World, step: Duration) -> (Buffer, Buffer) {
    frames_on(on, off, world, step, SCREEN)
}

/// A field with one sprite on it.
fn one_on_a_field() -> World {
    field(&[], &[at(3, 2)])
}

/// Twins in `theme` on a screen of `size` that have selected the sprite on
/// `world`, which opens its Body tab, and let it sweep in.
fn selected_in(world: &World, theme: Theme, size: Size) -> (App, App) {
    let (mut on, mut off) = twins_in(world, theme, size);
    both(&mut on, &mut off, world, Action::SelectNext);
    frames_on(&mut on, &mut off, world, ms(1000), size);
    assert_eq!(on.tab(), Tab::Body);
    (on, off)
}

/// Twins on a 100×30 screen, as `selected_in`.
fn selected(world: &World) -> (App, App) {
    selected_in(world, Theme::cp437(), SCREEN)
}

/// Whether screen cell `cell` hides what's drawn there in `faded`, as a
/// tab still to sweep in does.
fn hidden_in(screen: &Buffer, cell: Position, faded: Color) -> bool {
    (screen[cell].fg, screen[cell].bg) == (faded, faded)
}

/// Asserts that `shown` differs from `plain` only within `area`.
fn differs_only_within(shown: &Buffer, plain: &Buffer, area: Rect) {
    for cell in shown.area.positions().filter(|&cell| !area.contains(cell)) {
        assert_eq!(shown[cell], plain[cell], "{cell:?}, outside {area:?}");
    }
}

/// Which side the tab the twins just opened sweeps in from, a tenth of a
/// second in: `Some(true)` from the left, its left column shown as it is
/// and its right still hidden in `faded`, `Some(false)` from the right, and
/// `None` if neither.
fn swept_from_left(on: &mut App, off: &mut App, world: &World, faded: Color) -> Option<bool> {
    let (shown, plain) = frames(on, off, world, ms(100));
    let body = tab_body(on);
    let column = |x: u16| -> (bool, bool) {
        let cells: Vec<Position> = (body.top()..body.bottom())
            .map(|y| Position::new(x, y))
            .collect();
        (
            cells.iter().all(|&cell| shown[cell] == plain[cell]),
            cells.iter().all(|&cell| hidden_in(&shown, cell, faded)),
        )
    };
    match (column(body.left()), column(body.right() - 1)) {
        ((true, _), (_, true)) => Some(true),
        ((_, true), (true, _)) => Some(false),
        _ => None,
    }
}

#[test]
fn a_tab_opened_sweeps_in_out_of_the_panel_s_shade() {
    let world = one_on_a_field();
    let (mut on, mut off) = selected(&world);
    both(&mut on, &mut off, &world, Action::NextTab);
    let body = tab_body(&on);
    let (start, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    // Nothing of it shows yet, and its characters are as they are: only
    // colours change.
    let shade = shade();
    for cell in body.positions() {
        assert!(hidden_in(&start, cell, shade), "{cell:?}");
        assert_eq!(start[cell].symbol(), plain[cell].symbol());
    }
    differs_only_within(&start, &plain, body);
    // A quarter of a second, and it's all in.
    let (nearly, plain) = frames(&mut on, &mut off, &world, ms(200));
    assert_ne!(nearly, plain);
    differs_only_within(&nearly, &plain, body);
    let (end, plain) = frames(&mut on, &mut off, &world, ms(50));
    assert_eq!(end, plain);
}

#[test]
fn a_tab_sweeps_in_from_the_left_when_it_s_right_of_the_old_one_and_from_the_right_when_left() {
    let world = one_on_a_field();
    // By where the tabs are, not the key: `[` from Body wraps round to
    // World, on Body's right, and `]` from World to Body, on its left.
    for (keys, from_left) in [
        (&[Action::NextTab][..], true),
        (&[Action::PreviousTab], true),
        (&[Action::NextTab, Action::PreviousTab], false),
        (&[Action::PreviousTab, Action::NextTab], false),
    ] {
        let (mut on, mut off) = selected(&world);
        for (n, &key) in keys.iter().enumerate() {
            if n > 0 {
                frames(&mut on, &mut off, &world, ms(1000));
            }
            both(&mut on, &mut off, &world, key);
        }
        frames(&mut on, &mut off, &world, Duration::ZERO);
        let side = swept_from_left(&mut on, &mut off, &world, shade());
        assert_eq!(side, Some(from_left), "{keys:?}");
    }
}

#[test]
fn a_tab_sweeps_in_for_a_quarter_of_a_second_at_every_speed() {
    let world = one_on_a_field();
    for faster in [false, true] {
        let (mut on, mut off) = selected(&world);
        if faster {
            at_4x(&mut on);
            at_4x(&mut off);
        } else {
            on.clock.slower();
            off.clock.slower();
        }
        both(&mut on, &mut off, &world, Action::NextTab);
        let (nearly, plain) = frames(&mut on, &mut off, &world, ms(200));
        assert_ne!(nearly, plain, "4x: {faster}");
        let (end, plain) = frames(&mut on, &mut off, &world, ms(50));
        assert_eq!(end, plain, "4x: {faster}");
    }
}

#[test]
fn no_tab_sweeps_in_a_new_app_after_a_load_or_on_the_tab_already_open() {
    let world = one_on_a_field();
    // A new app opens on the World tab.
    let (mut on, mut off) = twins(&world);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert_eq!(shown, plain, "a new app");
    // A click on the tab already open takes it back to its top, at once.
    let (mut on, mut off) = selected(&world);
    let body = tab_body(&on);
    let name = cell_of_text(&on, &world, SCREEN, " Body ");
    assert_eq!(name.y, body.y - 1, "in the inspector's title row");
    let on_it = Action::left_click(Position::new(name.x + 2, name.y));
    both(&mut on, &mut off, &world, on_it);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert_eq!(shown, plain, "the tab already open");
    // A load keeps the tab open as it was.
    for (n, app) in [&mut on, &mut off].into_iter().enumerate() {
        app.set_save_folder(scratch_folder(&format!("tab-after-load-{n}")));
        apply(app, &world, Action::Quicksave);
        apply(app, &world, Action::Quickload);
        app.take_loaded().expect("the quicksave loaded");
        assert_eq!(app.tab(), Tab::Body);
    }
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert_eq!(shown, plain, "a load");
}

#[test]
fn with_motion_off_a_tab_opens_at_once() {
    let world = one_on_a_field();
    let (mut on, mut off) = selected(&world);
    both(&mut on, &mut off, &world, Action::NextTab);
    on.set_motion(false);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert_eq!(shown, plain);
}

#[test]
fn a_tab_sweeps_in_over_the_inspector_filling_the_screen() {
    // Drawn twice a frame, beside the map and then over it all, it sweeps
    // in once, where it's seen (design §6.1).
    let world = one_on_a_field();
    let (mut on, mut off) = selected(&world);
    both(&mut on, &mut off, &world, Action::Inspector);
    assert!(on.full_screen());
    both(&mut on, &mut off, &world, Action::NextTab);
    let body = tab_body(&on);
    assert_eq!(body.width, 99, "every column");
    let (start, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    let shade = shade();
    for cell in body.positions() {
        assert!(hidden_in(&start, cell, shade), "{cell:?}");
    }
    differs_only_within(&start, &plain, body);
    let side = swept_from_left(&mut on, &mut off, &world, shade);
    assert_eq!(side, Some(true));
    let (end, plain) = frames(&mut on, &mut off, &world, ms(150));
    assert_eq!(end, plain);
}

#[test]
fn in_a_theme_without_shades_a_tab_sweeps_in_out_of_motion_s_dark() {
    // The panel's background is the terminal's own, which the dark stands
    // for, and stays its own: never black or white, as tachyonfx would
    // draw it.
    let world = one_on_a_field();
    let (mut on, mut off) = selected_in(&world, Theme::ascii(), SCREEN);
    let dark = Theme::ascii().motion().dark;
    both(&mut on, &mut off, &world, Action::NextTab);
    let body = tab_body(&on);
    let (start, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    for cell in body.positions() {
        assert_eq!((start[cell].fg, start[cell].bg), (dark, Color::Reset));
    }
    differs_only_within(&start, &plain, body);
    for step in [40, 40, 40, 40, 40] {
        let (shown, plain) = frames(&mut on, &mut off, &world, ms(step));
        for cell in body.positions() {
            assert_eq!(shown[cell].bg, Color::Reset, "{cell:?}");
            if plain[cell].fg == Color::Reset {
                assert_ne!(shown[cell].fg, Color::White, "{cell:?}");
            }
        }
    }
    let (end, plain) = frames(&mut on, &mut off, &world, ms(50));
    assert_eq!(end, plain);
}

#[test]
fn a_tab_opened_while_the_inspector_is_hidden_doesn_t_sweep_in_when_it_opens() {
    // Under 100 columns the inspector hides until `i` opens it over the
    // map (design §6.1, #305).
    let world = one_on_a_field();
    let narrow = Size::new(80, 30);
    let (mut on, mut off) = selected_in(&world, Theme::cp437(), narrow);
    assert!(on.inspector_area().is_none(), "hidden");
    both(&mut on, &mut off, &world, Action::NextTab);
    assert_eq!(on.tab(), Tab::Brain);
    both(&mut on, &mut off, &world, Action::Inspector);
    let (shown, plain) = frames_on(&mut on, &mut off, &world, Duration::ZERO, narrow);
    assert_eq!(shown, plain);
    // Open there, a tab sweeps in, under the debug overlay, which is drawn
    // over it there.
    both(&mut on, &mut off, &world, Action::DebugOverlay);
    both(&mut on, &mut off, &world, Action::NextTab);
    let (shown, plain) = frames_on(&mut on, &mut off, &world, Duration::ZERO, narrow);
    let body = tab_body(&on);
    let debug = box_titled(&plain, " Debug ");
    assert!(debug.intersects(body), "{debug:?} over {body:?}");
    let shade = shade();
    for cell in body.positions() {
        if debug.contains(cell) {
            assert_eq!(shown[cell], plain[cell], "{cell:?}");
        } else {
            assert!(hidden_in(&shown, cell, shade), "{cell:?}");
        }
    }
    differs_only_within(&shown, &plain, body);
}

/// The box drawn on `screen` with `title` at the left of its top edge, as
/// `┌─ Debug ──┐`.
fn box_titled(screen: &Buffer, title: &str) -> Rect {
    let rows = common::lines(screen);
    let (y, x) = rows
        .iter()
        .enumerate()
        .find_map(|(y, row)| row.find(title).map(|x| (y, row[..x].chars().count())))
        .unwrap_or_else(|| panic!("{title:?} isn't on screen:\n{}", rows.join("\n")));
    let (left, top) = (x as u16 - 2, y as u16);
    assert_eq!(screen[(left, top)].symbol(), "┌", "{title:?}");
    let right = (left..screen.area.right())
        .find(|&x| screen[(x, top)].symbol() == "┐")
        .expect("the box's top right corner");
    let bottom = (top..screen.area.bottom())
        .find(|&y| screen[(left, y)].symbol() == "└")
        .expect("the box's bottom left corner");
    Rect::new(left, top, right - left + 1, bottom - top + 1)
}

// Pop-ups (design §6.9).

/// The default theme's dark, which a pop-up fades in from.
fn dark() -> Color {
    Theme::cp437().motion().dark
}

/// Asserts that every cell of `shown` that differs from `plain` is drawn
/// in the dark, as all a pop-up shows is as it opens, and that some do.
fn all_that_differs_is_dark(shown: &Buffer, plain: &Buffer) {
    let differ: Vec<Position> = shown
        .area
        .positions()
        .filter(|&cell| shown[cell] != plain[cell])
        .collect();
    assert!(differ.len() > 10, "{differ:?}");
    let dark = dark();
    for cell in differ {
        assert_eq!(shown[cell].fg, dark, "{cell:?}");
        assert_eq!(shown[cell].symbol(), plain[cell].symbol(), "{cell:?}");
    }
}

#[test]
fn a_pop_up_fades_in_out_of_the_dark() {
    let world = one_on_a_field();
    let (mut on, mut off) = selected(&world);
    both(&mut on, &mut off, &world, Action::Help);
    let help = on.overlay().expect("room for help");
    let (start, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    let dark = dark();
    for cell in help.positions() {
        assert_eq!(start[cell].fg, dark, "{cell:?}");
        // What has no background of its own keeps the terminal's.
        assert_eq!(start[cell].bg, plain[cell].bg, "{cell:?}");
    }
    differs_only_within(&start, &plain, help);
    // Fast and then slowing: well past half way at half time.
    let (half, plain) = frames(&mut on, &mut off, &world, ms(75));
    let corner = Position::new(help.x, help.y);
    let Color::Rgb(red, ..) = half[corner].fg else {
        panic!("{:?}", half[corner].fg)
    };
    assert!(red > (0x0C + 0xCC) / 2 && red < 0xCC, "{red:#x}");
    differs_only_within(&half, &plain, help);
    let (end, plain) = frames(&mut on, &mut off, &world, ms(75));
    assert_eq!(end, plain);
}

#[test]
fn what_was_under_a_pop_up_fades_back_from_the_dark_as_it_closes() {
    let world = one_on_a_field();
    let (mut on, mut off) = selected(&world);
    both(&mut on, &mut off, &world, Action::Help);
    let help = on.overlay().expect("room for help");
    frames(&mut on, &mut off, &world, ms(1000));
    both(&mut on, &mut off, &world, Action::Help);
    let (start, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    // The map, the inspector and the event log, each from the dark.
    let dark = dark();
    for cell in help.positions() {
        assert_eq!(start[cell].fg, dark, "{cell:?}");
    }
    let shaded = Position::new(70, 12);
    assert_eq!(plain[shaded].bg, shade());
    assert_eq!(start[shaded].bg, dark);
    differs_only_within(&start, &plain, help);
    let (end, plain) = frames(&mut on, &mut off, &world, ms(150));
    assert_eq!(end, plain);
}

#[test]
fn each_pop_up_fades_in_as_it_opens_and_out_as_it_closes() {
    // The menus, help, the command palette, the sprite list, the
    // population graph and the family tree (design §6.1).
    let world = one_on_a_field();
    for opens in [
        Action::OpenThemes,
        Action::Help,
        Action::Palette,
        Action::SpriteList,
        Action::Population,
        Action::FamilyTree,
    ] {
        let (mut on, mut off) = selected(&world);
        both(&mut on, &mut off, &world, opens);
        let (start, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
        all_that_differs_is_dark(&start, &plain);
        let (end, plain) = frames(&mut on, &mut off, &world, ms(150));
        assert_eq!(end, plain, "{opens:?}");
        both(&mut on, &mut off, &world, Action::Back);
        let (start, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
        all_that_differs_is_dark(&start, &plain);
        let (end, plain) = frames(&mut on, &mut off, &world, ms(150));
        assert_eq!(end, plain, "{opens:?}");
    }
}

#[test]
fn tooltips_and_the_debug_overlay_don_t_fade() {
    let world = one_on_a_field();
    let (mut on, mut off) = selected(&world);
    both(&mut on, &mut off, &world, Action::DebugOverlay);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert_eq!(shown, plain, "the debug overlay opened");
    let debug = box_titled(&plain, " Debug ");
    // Help, opened under it, fades in, and it stays as it is.
    both(&mut on, &mut off, &world, Action::Help);
    let help = on.overlay().expect("room for help");
    assert!(help.intersects(debug));
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    all_that_differs_is_dark(&shown, &plain);
    for cell in debug.positions() {
        assert_eq!(shown[cell], plain[cell], "{cell:?}");
    }
    // What was under help fades back as it closes, but not where the
    // debug overlay is still drawn.
    frames(&mut on, &mut off, &world, ms(1000));
    both(&mut on, &mut off, &world, Action::Help);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    all_that_differs_is_dark(&shown, &plain);
    for cell in debug.positions() {
        assert_eq!(shown[cell], plain[cell], "{cell:?}");
    }
    frames(&mut on, &mut off, &world, ms(1000));
    both(&mut on, &mut off, &world, Action::DebugOverlay);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert_eq!(shown, plain, "the debug overlay closed");
    // A tooltip, by a drive's name on the Body tab.
    let name = cell_of_text(&on, &world, SCREEN, "hunger");
    both(&mut on, &mut off, &world, Action::Point(name));
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    let text = common::lines(&plain).join("\n");
    assert!(text.contains("─ hunger ─"), "a tooltip:\n{text}");
    assert_eq!(shown, plain, "a tooltip");
}

#[test]
fn a_pop_up_opened_from_another_fades_in_over_it() {
    // The palette's `?` opens help in its place: help fades in, and none of
    // what was under the palette shows to fade back.
    let world = one_on_a_field();
    let (mut on, mut off) = selected(&world);
    both(&mut on, &mut off, &world, Action::Palette);
    frames(&mut on, &mut off, &world, ms(1000));
    type_in_both(&mut on, &mut off, &world, "help");
    frames(&mut on, &mut off, &world, ms(1000));
    both(&mut on, &mut off, &world, Action::Enter);
    assert_eq!(on.screen(), Screen::Help);
    let help = on.overlay().expect("room for help");
    let (start, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    let dark = dark();
    for cell in help.positions() {
        assert_eq!(start[cell].fg, dark, "{cell:?}");
    }
    differs_only_within(&start, &plain, help);
}

/// Both twins type `text` into what's open.
fn type_in_both(on: &mut App, off: &mut App, world: &World, text: &str) {
    common::type_in(on, world, text);
    common::type_in(off, world, text);
}

#[test]
fn with_motion_off_pop_ups_open_and_close_at_once() {
    let world = one_on_a_field();
    let (mut on, mut off) = selected(&world);
    on.set_motion(false);
    both(&mut on, &mut off, &world, Action::Help);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert_eq!(shown, plain, "opened");
    both(&mut on, &mut off, &world, Action::Help);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert_eq!(shown, plain, "closed");
    // Nor does a pop-up open as motion is turned on fade in late.
    both(&mut on, &mut off, &world, Action::Help);
    frames(&mut on, &mut off, &world, Duration::ZERO);
    on.set_motion(true);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert_eq!(shown, plain, "turned on");
}

#[test]
fn the_title_screen_s_help_is_drawn_whole_at_once() {
    // Its app never animates (design §6.9), so nothing there may wait on
    // motion.
    let world = start::title_world(&Args::default(), 3, (100, 30)).expect("a world");
    let mut title = Title::new(world, Theme::cp437(), 12345, Size::new(100, 30));
    title.animate(ms(10_000));
    let draw = |title: &Title| {
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("a terminal");
        terminal
            .draw(|frame| terra_tui::title::render(frame, title))
            .expect("drawn");
        terminal.backend().buffer().clone()
    };
    title.apply(Action::Pick(3));
    let shown = draw(&title);
    let at = common::lines(&shown)
        .iter()
        .enumerate()
        .find_map(|(y, row)| {
            row.find("EMOTES")
                .map(|x| Position::new(row[..x].chars().count() as u16, y as u16))
        })
        .expect("help's EMOTES heading");
    let look = Theme::cp437().text().heading;
    assert_eq!(Some(shown[at].fg), look.fg);
    title.animate(ms(1000));
    assert_eq!(draw(&title), shown);
}

// A drive's bar (design §6.9).

/// The genes of a sprite whose hunger starts at `hunger`.
fn hunger_at(hunger: f32) -> String {
    format!(r#"(format: 1, genes: [InitialConcentration(chem: "hunger", value: {hunger})])"#)
}

/// A field with a sprite on each of `tiles`, each hungry as `hunger` says.
fn hungry(sprites: &[(Pos, f32)]) -> World {
    let genes: Vec<(Pos, String)> = sprites
        .iter()
        .map(|&(tile, hunger)| (tile, hunger_at(hunger)))
        .collect();
    let genes: Vec<(Pos, &str)> = genes
        .iter()
        .map(|(tile, genes)| (*tile, genes.as_str()))
        .collect();
    WorldBuilder::grass(10, 6).genes(&genes).build()
}

/// The hunger bar on the Body tab on `screen`: how many of its blocks are
/// filled, and the level it says beside it.
fn hunger_bar(screen: &Buffer) -> (usize, String) {
    let rows = common::lines(screen);
    let (y, x) = rows
        .iter()
        .enumerate()
        .find_map(|(y, row)| {
            let at = row.find(" hunger ")?;
            Some((y as u16, row[..at].chars().count() as u16 + 1))
        })
        .unwrap_or_else(|| panic!("no hunger bar:\n{}", rows.join("\n")));
    let bar: String = (x + 12..x + 22).map(|x| screen[(x, y)].symbol()).collect();
    let filled = bar.chars().filter(|&c| c == '█').count();
    assert_eq!(
        bar.chars().filter(|&c| c == '░').count(),
        10 - filled,
        "{bar}"
    );
    let said: String = (x + 22..screen.area.right())
        .map(|x| screen[(x, y)].symbol())
        .collect();
    (filled, said.trim().to_string())
}

/// An app with motion on that has selected the one sprite on `world`, let
/// its tab sweep in, and taken in a tick of it.
fn watching(world: &World) -> App {
    let mut app = moving(world);
    apply(&mut app, world, Action::SelectNext);
    frame(&mut app, world, ms(1000));
    app.record(&[], world);
    app
}

/// Takes in a frame's ticks that leave the world as `world` is, as the game
/// does.
fn take_in(app: &mut App, world: &World) {
    let mut ticks = Ticks::default();
    ticks.note(Vec::new(), world);
    app.take_in(ticks, world);
}

#[test]
fn a_drive_s_bar_eases_to_its_new_value_a_block_at_a_time() {
    let (before, after) = (hungry(&[(at(3, 2), 0.2)]), hungry(&[(at(3, 2), 0.8)]));
    let mut app = watching(&before);
    let shown = hunger_bar(&frame(&mut app, &before, Duration::ZERO));
    assert_eq!(shown, (2, ".20".to_string()));
    take_in(&mut app, &after);
    // The level beside it is true at once; only the bar eases.
    let start = hunger_bar(&frame(&mut app, &after, Duration::ZERO));
    assert_eq!(start, (2, ".80".to_string()));
    let mut filled = vec![start.0];
    for _ in 0..8 {
        let (blocks, said) = hunger_bar(&frame(&mut app, &after, ms(50)));
        assert_eq!(said, ".80");
        filled.push(blocks);
    }
    // Whole blocks, never back, and there in 0.4 s.
    assert!(filled.windows(2).all(|two| two[0] <= two[1]), "{filled:?}");
    assert_eq!(filled[8], 8, "{filled:?}");
    assert!(filled[2] > 2 && filled[2] < 8, "{filled:?}");
    // Quick at first and slowing: most of the way in the first half.
    assert!(filled[4] - 2 > 8 - filled[4], "{filled:?}");
}

#[test]
fn a_bar_eases_on_from_where_it_s_got_to() {
    // A new level before the last is reached eases from the bar as it's
    // drawn, without jumping.
    let worlds = [0.2, 0.8, 0.5].map(|hunger| hungry(&[(at(3, 2), hunger)]));
    let mut app = watching(&worlds[0]);
    take_in(&mut app, &worlds[1]);
    let (part, _) = hunger_bar(&frame(&mut app, &worlds[1], ms(100)));
    assert!(part > 2 && part < 8, "{part}");
    take_in(&mut app, &worlds[2]);
    let (then, said) = hunger_bar(&frame(&mut app, &worlds[2], Duration::ZERO));
    assert_eq!((then, said.as_str()), (part, ".50"));
    let (end, _) = hunger_bar(&frame(&mut app, &worlds[2], ms(400)));
    assert_eq!(end, 5);
}

#[test]
fn a_bar_snaps_on_first_sight_on_a_new_selection_with_motion_off_and_at_4x() {
    let (before, after) = (hungry(&[(at(3, 2), 0.2)]), hungry(&[(at(3, 2), 0.8)]));
    // First seen at .8: drawn full at once, not filling from empty.
    let mut app = watching(&after);
    let (blocks, _) = hunger_bar(&frame(&mut app, &after, Duration::ZERO));
    assert_eq!(blocks, 8, "first sight");
    // Another sprite selected shows its own at once.
    let two = hungry(&[(at(3, 2), 0.2), (at(6, 2), 0.8)]);
    let mut app = watching(&two);
    select(&mut app, &two, sprite_on(&two, at(3, 2)));
    app.record(&[], &two);
    let (blocks, _) = hunger_bar(&frame(&mut app, &two, ms(1000)));
    assert_eq!(blocks, 2);
    select(&mut app, &two, sprite_on(&two, at(6, 2)));
    app.record(&[], &two);
    let (blocks, _) = hunger_bar(&frame(&mut app, &two, Duration::ZERO));
    assert_eq!(blocks, 8, "a new selection");
    // With motion off, and at 4×, it jumps.
    for faster in [false, true] {
        let mut app = watching(&before);
        if faster {
            at_4x(&mut app);
        } else {
            app.set_motion(false);
        }
        take_in(&mut app, &after);
        let (blocks, _) = hunger_bar(&frame(&mut app, &after, Duration::ZERO));
        assert_eq!(blocks, 8, "4x: {faster}");
    }
}
