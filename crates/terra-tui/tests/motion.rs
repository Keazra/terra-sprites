//! Motion (design §6.9, #306): small effects drawn over the finished
//! screen in real time, with one switch that turns them all off.
//!
//! The shared constructors in `common` turn motion off, so the other tests
//! keep their pictures; these turn it on.

mod common;
use common::screen::{HUNGRY_GENOME, SPEED_10, one_sprite_among};
use common::{
    Changes, LONE_MOTHER, SCORCHED, SCREEN, WorldBuilder, app_on, apply, at, ball_world,
    cell_of_text, died, field, frame, frame_on, grow, lone_line, mono, pack_changing, pointer_on,
    render, scratch_folder, select, sized_app, sprite_on,
};

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::{Color, Modifier};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{DeathCause, EntityId, Event, EventKind, Pos, ScriptedAction, Terrain, World};
use terra_tui::app::{App, ColourMode, CursorMode, Screen, Selection, Tab, Ticks};
use terra_tui::args::{Args, USAGE};
use terra_tui::campbell::{self, FOREGROUND};
use terra_tui::clock::Speed;
use terra_tui::input::Action;
use terra_tui::policy::{InfoPolicy, Panel, Subject};
use terra_tui::start;
use terra_tui::theme::{BarWaves, MotionLook, Theme};
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

/// The ascii theme's file with `panels` in place of its panels' shades, the
/// default palette's: none, for a theme without shades, or a line of its own.
fn ascii_with_panels(panels: &str) -> String {
    let shades = "    panels: (shade: \"#181a22\", title: \"#282c3a\"),\n";
    assert!(
        ASCII_THEME.contains(shades),
        "the ascii theme has the shades"
    );
    ASCII_THEME.replacen(shades, panels, 1)
}

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
        bar: BarWaves::Crashing,
    };
    assert_eq!(Theme::cp437().motion(), expected);
    assert_eq!(Theme::ascii().motion(), expected);
}

#[test]
fn a_theme_s_motion_section_may_leave_colours_out_and_name_them() {
    // What a theme leaves out of the section, it draws as the default theme
    // does (design §6.9), and a colour the game doesn't know is refused.
    let pack = common::pack();
    let theme = Theme::from_ron(
        &ascii_and(r##"    motion: (ash: gray, spark: "#2020ff"),"##),
        &pack,
    )
    .expect("loads");
    let (ours, default) = (theme.motion(), Theme::cp437().motion());
    assert_eq!(ours.ash, Color::Gray);
    assert_eq!(ours.spark, Color::Rgb(0x20, 0x20, 0xff));
    assert_eq!(ours.flash, default.flash);
    assert_eq!(ours.trail, default.trail);
    let err =
        Theme::from_ron(&ascii_and("    motion: (glow: white),"), &pack).expect_err("refused");
    assert!(err.contains("glow"), "{err}");
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

#[test]
fn with_motion_on_water_stays_still_in_its_theme_s_colours() {
    // Design §6.9: water has no motion until it moves like real water
    // (#399), so a lake draws the same in every frame, in the colours the
    // theme gives it, as with motion off.
    let world = lake().build();
    let mut app = moving(&world);
    cursor_to(&mut app, &world, at(0, 0));
    // A frame every tenth of a second, for 4.5 seconds.
    let mut screens = vec![frame(&mut app, &world, Duration::ZERO)];
    for _ in 0..45 {
        screens.push(frame(&mut app, &world, ms(100)));
    }
    for (n, screen) in screens.iter().enumerate() {
        let shallow = colours_of(&app, &world, screen, Terrain::ShallowWater);
        let deep = colours_of(&app, &world, screen, Terrain::DeepWater);
        assert!(
            shallow.iter().all(|&c| c == Color::Cyan),
            "{n}: {shallow:?}"
        );
        assert!(deep.iter().all(|&c| c == Color::Blue), "{n}: {deep:?}");
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
fn a_death_on_the_cursor_s_tile_flashes_through_its_reverse_video() {
    // Design §6.9: the Cursor's tile is drawn in reverse video, as a
    // selected egg is, so the flash takes the reverse video's place there
    // too: at once the ghost is drawn in its colour on the flash, and the
    // reverse video comes back as it fades.
    let (mut app, ids, after) = before_and_after(&[at(3, 2)]);
    cursor_to(&mut app, &after, at(3, 2));
    app.record(&deaths(&ids), &after);
    let MotionLook { flash, ash, .. } = app.theme.motion();
    let first = cell(&app, at(3, 2));
    let screen = frame(&mut app, &after, Duration::ZERO);
    assert!(reversed(&screen, first));
    assert_eq!(screen[first].symbol(), "☺");
    assert_eq!((screen[first].fg, screen[first].bg), (flash, WHITE));
    // Its reverse video back by its end: the ghost, faded to ash, filling
    // the tile and its strokes dark.
    let near_end = frame(&mut app, &after, ms(1190));
    assert!(reversed(&near_end, first));
    assert!(
        within(near_end[first].fg, ash, 2),
        "{:?}",
        near_end[first].fg
    );
    assert!(
        within(near_end[first].bg, DARK, 1),
        "{:?}",
        near_end[first].bg
    );
    let over = frame(&mut app, &after, ms(10));
    assert!(reversed(&over, first));
    assert_eq!((over[first].symbol(), over[first].fg), (".", Color::Green));
    assert_eq!(over[first].bg, Color::Reset);
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

/// A policy that shows everything but the emotes of the sprites `hidden`.
struct HidesEmotesOf(Vec<EntityId>);

impl InfoPolicy for HidesEmotesOf {
    fn can_view(&self, panel: Panel, subject: Subject) -> bool {
        let hidden = matches!(subject, Subject::Sprite(id) if self.0.contains(&id));
        !(panel == Panel::Emotes && hidden)
    }
}

#[test]
fn a_death_the_policy_hides_takes_no_place_from_one_the_player_sees() {
    // Design §6.4, §6.9: only what can be drawn counts towards the four.
    let tiles = [at(1, 1), at(3, 1), at(5, 1), at(7, 1), at(1, 4)];
    let (mut app, ids, after) = before_and_after(&tiles);
    app.set_policy(HidesEmotesOf(ids[1..].to_vec()));
    app.record(&deaths(&ids), &after);
    let screen = frame(&mut app, &after, Duration::ZERO);
    assert!(flashing(&app, &screen, at(1, 1)));
    // Two at 4× and up: the seen one first, then two hidden.
    let (mut app, ids, after) = before_and_after(&tiles);
    at_4x(&mut app);
    app.set_policy(HidesEmotesOf(ids[1..].to_vec()));
    app.record(&deaths(&ids[..3]), &after);
    let screen = frame(&mut app, &after, Duration::ZERO);
    assert!(flashing(&app, &screen, at(1, 1)));
}

#[test]
fn a_death_off_the_map_view_takes_no_place_from_one_the_player_sees() {
    // Design §6.9: the view starts from column 48 of a map 150 across, so
    // the sprite on column 60 is in it and those in the west aren't.
    let seen = at(60, 5);
    let unseen = [at(1, 5), at(2, 5), at(3, 5), at(4, 5)];
    let before = WorldBuilder::grass(150, 12)
        .starters(&[&[seen][..], &unseen[..]].concat())
        .build();
    let after = WorldBuilder::grass(150, 12).build();
    let mut app = moving(&before);
    assert!(app.cell_of(seen).is_some());
    assert!(unseen.iter().all(|&tile| app.cell_of(tile).is_none()));
    app.record(&[], &before);
    let ids: Vec<EntityId> = [&[seen][..], &unseen[..]]
        .concat()
        .iter()
        .map(|&tile| sprite_on(&before, tile))
        .collect();
    app.record(&deaths(&ids), &after);
    let screen = frame(&mut app, &after, Duration::ZERO);
    assert!(flashing(&app, &screen, seen));
}

#[test]
fn a_death_under_the_inspector_takes_no_place_from_one_the_player_sees() {
    // Design §6.9: on a screen under 100 columns the inspector opens over
    // the map view, and a tile under it isn't seen.
    let size = Size::new(80, 24);
    let empty = WorldBuilder::grass(150, 40).build();
    let mut probe = sized_app(&empty, Theme::cp437(), size);
    apply(&mut probe, &empty, Action::Inspector);
    let inspector = probe.inspector_area().expect("open over the map");
    let under = |tile| probe.cell_of(tile).map(|cell| inspector.contains(cell));
    let tiles: Vec<Pos> = (0..150)
        .flat_map(|x| (0..40).map(move |y| at(x, y)))
        .collect();
    let seen = *tiles
        .iter()
        .find(|&&tile| under(tile) == Some(false))
        .unwrap();
    let unseen: Vec<Pos> = tiles
        .iter()
        .copied()
        .filter(|&tile| under(tile) == Some(true))
        .step_by(3)
        .take(4)
        .collect();
    let before = WorldBuilder::grass(150, 40)
        .starters(&[&[seen][..], &unseen[..]].concat())
        .build();
    let after = WorldBuilder::grass(150, 40).build();
    let mut app = sized_app(&before, Theme::cp437(), size);
    app.set_motion(true);
    plain(&mut app, &before);
    apply(&mut app, &before, Action::Inspector);
    assert_eq!(app.inspector_area(), Some(inspector));
    app.record(&[], &before);
    let ids: Vec<EntityId> = [&[seen][..], &unseen[..]]
        .concat()
        .iter()
        .map(|&tile| sprite_on(&before, tile))
        .collect();
    app.record(&deaths(&ids), &after);
    let screen = frame_on(&mut app, &after, Duration::ZERO, size);
    assert!(flashing(&app, &screen, seen));
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
    // It appears once the egg has shown cracked, 150 ms in (design §6.9),
    // and warms over the next half second.
    let first = cell(&on, world.sprite(young).expect("hatched").pos());
    let egg = campbell::exact(on.theme.object_glyph("egg", "default").fg, FOREGROUND);
    let start = frame(&mut on, &world, ms(150));
    assert_eq!(start[first].fg, egg);
    let quarter = frame(&mut on, &world, ms(250));
    let fg = quarter[first].fg;
    assert!(
        fg != egg && fg != WHITE && matches!(fg, Color::Rgb(..)),
        "{fg:?}"
    );
    let half = frame(&mut on, &world, ms(250));
    let plain = frame(&mut off, &world, ms(650));
    assert_eq!(half[first].fg, plain[first].fg);
}

// A hatching's shell (design §6.9).

/// A 12×6 field with a berry bush on (3, 3) and, on (5, 2), a sprite who
/// lays alone and quickly: her first egg hatches on `NEST`, the only one
/// to hatch then, with the bush left of it and her up and right of it.
fn by_a_bush() -> World {
    let [quick, eggs] = common::QUICK;
    WorldBuilder::grass(12, 6)
        .objects(&[(at(3, 3), "berry_bush")])
        .genes(&[(at(5, 2), LONE_MOTHER)])
        .data(pack_changing(&[quick, eggs, common::LONE]))
        .seed(7)
        .build()
}

/// Where `by_a_bush`'s first egg hatches.
const NEST: Pos = at(4, 3);

/// Twins on `by_a_bush`, in `theme`, that have taken in each tick until
/// its first egg hatched on `NEST`, and the world then.
fn hatched_by_a_bush(theme: Theme) -> (App, App, World) {
    let mut world = by_a_bush();
    let (mut on, mut off) = twins_in(&world, theme, SCREEN);
    let young = until_a_hatching(&mut world, &mut on, &mut off);
    assert_eq!(world.sprite(young).map(|sprite| sprite.pos()), Some(NEST));
    (on, off, world)
}

/// The egg's colour in the default theme, light yellow, as Campbell draws
/// it.
const EGG: Color = Color::Rgb(0xF9, 0xF1, 0xA5);

#[test]
fn a_hatching_shows_the_egg_cracked_for_150_ms_then_the_young() {
    // Design §6.9, as Fauxpaw picked it from the mock-ups: for a moment the
    // egg shows cracked across, `Θ`, in its own colour, its tile glowing
    // where its last flash left it, seven tenths of the way to the sparks'
    // colour; then the young appears, the glow fading from there.
    let (mut on, mut off, world) = hatched_by_a_bush(Theme::cp437());
    let first = cell(&on, NEST);
    for step in [0, 149] {
        let (shown, _) = frames(&mut on, &mut off, &world, ms(step));
        let cracked = (shown[first].symbol(), shown[first].fg, shown[first].bg);
        assert_eq!(cracked, ("Θ", EGG, LAST_FLASH), "{step} ms on");
    }
    let (shown, plain) = frames(&mut on, &mut off, &world, ms(1));
    assert_eq!(shown[first].symbol(), plain[first].symbol(), "the young");
    assert_ne!(shown[first].symbol(), "Θ");
    assert_eq!(shown[first].bg, LAST_FLASH);
}

/// The symbols `app` draws on `screen` on the tiles from `from` to `to`, a
/// row or a column, each tile's first cell's.
fn symbols(app: &App, screen: &Buffer, from: Pos, to: Pos) -> String {
    let tiles = (from.y..=to.y).flat_map(|y| (from.x..=to.x).map(move |x| at(x, y)));
    tiles.map(|tile| screen[cell(app, tile)].symbol()).collect()
}

/// The twins' Cursors moved out of the way of a hatching on `NEST`: the
/// Cursor's tile has a look of its own.
fn cursors_away(on: &mut App, off: &mut App, world: &World) {
    for app in [on, off] {
        cursor_to(app, world, at(10, 0));
    }
}

#[test]
fn the_shell_splits_into_halves_either_side_of_the_egg_s_tile_that_slide_out_at_360_ms() {
    // Design §6.9: once the egg has shown cracked, its halves lie either
    // side of the egg's tile, where the young stands, and 210 ms later they
    // slide a tile further out. The bush left of it, a seedling yet, `'`,
    // keeps its look: no half is drawn over it.
    let (mut on, mut off, world) = hatched_by_a_bush(Theme::cp437());
    cursors_away(&mut on, &mut off, &world);
    let row = |app: &App, screen: &Buffer| symbols(app, screen, at(1, 3), at(7, 3));
    let (cracked, _) = frames(&mut on, &mut off, &world, ms(149));
    assert_eq!(row(&on, &cracked), "..'Θ...");
    for step in [1, 209] {
        let (shown, plain) = frames(&mut on, &mut off, &world, ms(step));
        let young = plain[cell(&on, NEST)].symbol();
        assert_eq!(row(&on, &shown), format!("..'{young}).."), "{step} ms on");
    }
    let (shown, plain) = frames(&mut on, &mut off, &world, ms(1));
    let young = plain[cell(&on, NEST)].symbol();
    assert_eq!(row(&on, &shown), format!(".('{young}.)."), "slid out");
}

/// Grass's colour, green, as Campbell draws it.
const GRASS: Color = Color::Rgb(0x13, 0xA1, 0x0E);

#[test]
fn the_shell_leaves_crumbs_on_the_tiles_at_the_egg_s_tile_s_corners_for_600_ms() {
    // Design §6.9: `` ` `` up and left of the egg's tile, where the young
    // stands, `'` up and right, `,` down and left and `.` down and right,
    // from when the egg stops showing cracked until 600 ms on. Up and right
    // stands the young's mother, who keeps her look: no crumb is drawn over
    // her.
    let (mut on, mut off, world) = hatched_by_a_bush(Theme::cp437());
    cursors_away(&mut on, &mut off, &world);
    let corners = |app: &App, screen: &Buffer| {
        let above = symbols(app, screen, at(3, 2), at(5, 2));
        (above, symbols(app, screen, at(3, 4), at(5, 4)))
    };
    let mother = cell(&on, at(5, 2));
    let (cracked, plain) = frames(&mut on, &mut off, &world, ms(149));
    let her = plain[mother].symbol();
    assert_eq!(corners(&on, &cracked), (format!("..{her}"), "...".into()));
    let down_right = cell(&on, at(5, 4));
    for step in [1, 599] {
        let (shown, plain) = frames(&mut on, &mut off, &world, ms(step));
        let her = plain[mother].symbol();
        let crumbs = (format!("`.{her}"), ",..".into());
        assert_eq!(corners(&on, &shown), crumbs, "{step} ms on");
        // The `.` is the shell's, not the grass's: it starts in the egg's
        // colour, which a spark may light up, never in the grass's.
        if step == 1 {
            let fg = shown[down_right].fg;
            assert!(fg != Color::Green && fg != GRASS, "{fg:?}");
        }
    }
    let (gone, plain) = frames(&mut on, &mut off, &world, ms(1));
    let her = plain[mother].symbol();
    assert_eq!(corners(&on, &gone), (format!("..{her}"), "...".into()));
}

#[test]
fn the_shell_s_halves_fade_evenly_from_the_egg_s_colour_to_their_tile_s() {
    // Design §6.9: from when the egg stops showing cracked, a half holds
    // the egg's colour for 110 ms and fades evenly to its tile's own by 700
    // ms, when it goes: half way at 405 ms, from light yellow to green
    // `#86c959`. Seen out on the far tiles, where no spark lights it.
    let (mut on, mut off, world) = hatched_by_a_bush(Theme::cp437());
    cursors_away(&mut on, &mut off, &world);
    let halves = [cell(&on, at(2, 3)), cell(&on, at(6, 3))];
    let (slid, _) = frames(&mut on, &mut off, &world, ms(360));
    for half in halves {
        let Color::Rgb(red, ..) = slid[half].fg else {
            panic!("{:?}", slid[half].fg)
        };
        assert!(red < 0xF9 && red > 0xF9 - (0xF9 - 0x13) / 4, "{red:#x}");
    }
    let (half_way, _) = frames(&mut on, &mut off, &world, ms(195));
    for half in halves {
        assert_eq!(half_way[half].fg, Color::Rgb(0x86, 0xC9, 0x59));
    }
    let (faded, _) = frames(&mut on, &mut off, &world, ms(294));
    for half in halves {
        assert!(near(faded[half].fg, GRASS), "{:?}", faded[half].fg);
    }
    let (gone, plain) = frames(&mut on, &mut off, &world, ms(1));
    for half in halves {
        assert_eq!(gone[half], plain[half]);
    }
}

#[test]
fn no_piece_of_the_shell_is_drawn_on_the_cursor_s_tile() {
    // Design §6.9: it has a look of its own, as the trail leaves it. The
    // Cursor starts on (6, 3), where the right half slides at 360 ms.
    let (mut on, mut off, world) = hatched_by_a_bush(Theme::cp437());
    assert_eq!(on.cursor(), at(6, 3));
    let (left, right) = (cell(&on, at(2, 3)), cell(&on, at(6, 3)));
    let (shown, plain) = frames(&mut on, &mut off, &world, ms(360));
    assert_eq!(shown[left].symbol(), "(");
    assert_eq!(shown[right], plain[right]);
}

#[test]
fn at_4x_the_shell_cracks_and_splits_in_half_the_time() {
    // Design §6.9: as the hatching plays in half the time, so does its
    // shell: the egg shows cracked for 75 ms, the halves slide out 105 ms
    // later, and the crumbs go 300 ms after the crack and the halves 350.
    let mut world = by_a_bush();
    let (mut on, mut off) = twins(&world);
    at_4x(&mut on);
    at_4x(&mut off);
    let young = until_a_hatching(&mut world, &mut on, &mut off);
    assert_eq!(world.sprite(young).map(|sprite| sprite.pos()), Some(NEST));
    cursors_away(&mut on, &mut off, &world);
    let (first, up_left) = (cell(&on, NEST), cell(&on, at(3, 2)));
    let row = |app: &App, screen: &Buffer| symbols(app, screen, at(1, 3), at(7, 3));
    let (shown, _) = frames(&mut on, &mut off, &world, ms(74));
    assert_eq!(shown[first].symbol(), "Θ");
    for (step, (halves, crumb)) in [
        (1, ("..'Y)..", "`")),
        (104, ("..'Y)..", "`")),
        (1, (".('Y.).", "`")),
        (194, (".('Y.).", "`")),
        (1, (".('Y.).", ".")),
        (49, (".('Y.).", ".")),
        (1, ("..'Y...", ".")),
    ] {
        let (shown, plain) = frames(&mut on, &mut off, &world, ms(step));
        let halves = halves.replace('Y', plain[first].symbol());
        assert_eq!((row(&on, &shown), shown[up_left].symbol()), (halves, crumb));
    }
}

#[test]
fn the_shell_is_drawn_from_the_glyph_set_in_ascii_a_q_cracked() {
    // Design §6.2, §6.9: ASCII has no `Θ`, so the egg shows cracked as a
    // ring with a break in it.
    let (mut on, mut off, world) = hatched_by_a_bush(Theme::ascii());
    cursors_away(&mut on, &mut off, &world);
    let first = cell(&on, NEST);
    let (cracked, _) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert_eq!(cracked[first].symbol(), "Q");
    let (split, plain) = frames(&mut on, &mut off, &world, ms(150));
    let young = plain[first].symbol();
    let row = symbols(&on, &split, at(1, 3), at(7, 3));
    assert_eq!(row, format!("..'{young}).."));
    assert_eq!(split[cell(&on, at(3, 2))].symbol(), "`");
}

#[test]
fn in_mono_the_shell_still_cracks_and_splits_in_the_terminal_s_own_colours() {
    // Design §6.9: the egg cracked and its pieces are characters, which
    // mono draws, though their fade, the young's warming and the sparks,
    // which are only colour, don't play: nothing but the characters differs
    // from the screen with motion off.
    let (mut on, mut off, world) = hatched_by_a_bush(mono());
    cursors_away(&mut on, &mut off, &world);
    let first = cell(&on, NEST);
    let only_characters_differ = |shown: &Buffer, plain: &Buffer| {
        let mut shown = shown.clone();
        for (cell, was) in shown.content.iter_mut().zip(&plain.content) {
            cell.set_symbol(was.symbol());
        }
        assert_eq!(&shown, plain);
    };
    let (cracked, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert_eq!(cracked[first].symbol(), "Θ");
    only_characters_differ(&cracked, &plain);
    let (split, plain) = frames(&mut on, &mut off, &world, ms(360));
    let young = plain[first].symbol();
    let row = symbols(&on, &split, at(1, 3), at(7, 3));
    assert_eq!(row, format!(".('{young}.)."));
    only_characters_differ(&split, &plain);
    let (over, plain) = frames(&mut on, &mut off, &world, ms(500));
    assert_eq!(over, plain);
}

#[test]
fn in_mono_a_death_takes_no_place_from_a_hatching_s_shell() {
    // Design §6.9: only what can be drawn counts towards how many play at
    // once, and in mono a death, which is nothing but colour, draws
    // nothing. Two play at once at 4×: two deaths after the hatching leave
    // it playing.
    let mut world = by_a_bush();
    let (mut on, mut off) = twins_in(&world, mono(), SCREEN);
    at_4x(&mut on);
    at_4x(&mut off);
    let young = until_a_hatching(&mut world, &mut on, &mut off);
    let mother = sprite_on(&world, at(5, 2));
    on.record(&deaths(&[mother, young]), &world);
    let screen = frame(&mut on, &world, Duration::ZERO);
    assert_eq!(screen[cell(&on, NEST)].symbol(), "Θ");
}

/// Runs `world` one tick, both apps taking it in: its events.
fn one_tick(world: &mut World, on: &mut App, off: &mut App) -> Vec<Event> {
    let events = world.step();
    on.record(&events, world);
    off.record(&events, world);
    events
}

/// Runs `world` until an egg lying on the map, with no sprite on it, will
/// hatch in `left` ticks, the one it hatches on included, both apps taking
/// in each tick: the egg's ID and its tile.
fn until_due_in(world: &mut World, on: &mut App, off: &mut App, left: u64) -> (EntityId, Pos) {
    for _ in 0..400 {
        one_tick(world, on, off);
        let due = world.eggs().find_map(|egg| {
            let tile = egg.pos()?;
            (egg.hatches() + 1 == world.tick() + left && world.sprite_at(tile).is_none())
                .then_some((egg.id(), tile))
        });
        if let Some(due) = due {
            return due;
        }
    }
    panic!("no egg came due in {left} ticks");
}

/// Runs `world` until an egg lying on the map, with no sprite on it, will
/// hatch on the next tick, as `until_due_in`.
fn until_about_to_hatch(world: &mut World, on: &mut App, off: &mut App) -> (EntityId, Pos) {
    until_due_in(world, on, off, 1)
}

/// The change that makes eggs hatch 120 ticks after they're laid, with
/// `QUICK`'s first, so the screen sees one before it starts to flash.
const SLOW_EGGS: Changes = (
    "objects.ron",
    &[(
        "ticks: (750, 750), next: Hatch",
        "ticks: (120, 120), next: Hatch",
    )],
);

/// What an egg about to hatch flashes to on a tile with no background of
/// its own, in the default theme (design §6.9): three tenths of the way
/// from the dark to the sparks' colour at 100 ticks from hatching, and
/// seven tenths on the last tick.
const FIRST_FLASH: Color = Color::Rgb(0x54, 0x4E, 0x38);
const LAST_FLASH: Color = Color::Rgb(0xB6, 0xA6, 0x73);

/// The dark, motion's background for a tile with none of its own, which a
/// flash fades back to.
const DARK: Color = Color::Rgb(0x0C, 0x0C, 0x0C);

#[test]
fn an_egg_starts_to_flash_100_ticks_before_it_hatches() {
    // Design §6.9, Fauxpaw's playtest of Look 5: from 100 ticks out, its
    // tile's background glows three tenths of the way to the sparks' colour
    // at once and fades, quickly at first, over 1.605 s, and flashes again,
    // while paused too. Colour only: the egg's glyph is as it is.
    let [quick, _] = common::QUICK;
    let mut world = grow(
        &[quick, SLOW_EGGS, common::LONE],
        (12, 6),
        &[(at(1, 4), LONE_MOTHER)],
        0,
    );
    let (mut on, mut off) = twins(&world);
    let (egg, tile) = until_due_in(&mut world, &mut on, &mut off, 101);
    let first = cell(&on, tile);
    for step in [0, 400, 800, 1605] {
        let shown = frame(&mut on, &world, ms(step));
        let plain = frame(&mut off, &world, ms(step));
        assert_eq!(shown[first], plain[first], "101 ticks out, {step} ms on");
    }
    one_tick(&mut world, &mut on, &mut off);
    assert_eq!(world.egg(egg).and_then(|egg| egg.pos()), Some(tile));
    assert!(world.sprite_at(tile).is_none());
    on.clock.pause();
    let plain = frame(&mut off, &world, Duration::ZERO);
    let start = frame(&mut on, &world, Duration::ZERO);
    assert_eq!(start[first].bg, FIRST_FLASH);
    let fading = frame(&mut on, &world, ms(802));
    let faded = frame(&mut on, &world, ms(802));
    assert_eq!(faded[first].bg, DARK, "faded back by its end");
    let again = frame(&mut on, &world, ms(1));
    assert_eq!(again[first].bg, FIRST_FLASH, "the next flash");
    // Fast and then slowing: well past half way back at half time.
    let Color::Rgb(red, ..) = fading[first].bg else {
        panic!("{:?}", fading[first].bg)
    };
    assert!(red > 0x0C && red < (0x54 + 0x0C) / 2, "{red:#x}");
    for screen in [&start, &fading, &faded, &again] {
        let glyph = (screen[first].symbol(), screen[first].fg);
        assert_eq!(glyph, (plain[first].symbol(), plain[first].fg));
    }
}

/// How bright `colour` is: the sum of its parts, or none for the
/// terminal's own.
fn brightness(colour: Color) -> u32 {
    match colour {
        Color::Rgb(r, g, b) => u32::from(r) + u32::from(g) + u32::from(b),
        _ => 0,
    }
}

/// Draws `app`'s frames a millisecond apart, for at most `within`
/// milliseconds, until a flash starts on `tile`, where its background
/// brightens from the frame before: how long that took, and what it
/// flashed to.
fn next_flash(app: &mut App, world: &World, tile: Pos, within: u64) -> Option<(u64, Color)> {
    let first = cell(app, tile);
    let mut was = render(app, world)[first].bg;
    for n in 1..=within {
        let bg = frame(app, world, ms(1))[first].bg;
        if brightness(bg) > brightness(was) {
            return Some((n, bg));
        }
        was = bg;
    }
    None
}

/// When each flash on `tile` starts over the next `for_ms` milliseconds,
/// in milliseconds from now, `app` drawing a frame each millisecond, and
/// what it flashes to.
fn flashes(app: &mut App, world: &World, tile: Pos, for_ms: u64) -> Vec<(u64, Color)> {
    let mut starts = Vec::new();
    let mut so_far = 0;
    while let Some((n, bg)) = next_flash(app, world, tile, for_ms - so_far) {
        so_far += n;
        starts.push((so_far, bg));
    }
    starts
}

/// How long each of `flashes` lasts, but the last.
fn periods(flashes: &[(u64, Color)]) -> Vec<u64> {
    flashes.windows(2).map(|two| two[1].0 - two[0].0).collect()
}

#[test]
fn an_egg_s_flashes_come_faster_and_brighter_as_it_nears_hatching() {
    // Design §6.9: a flash lasts 120 ms on the last tick and 15 ms more for
    // each tick further off, 255 ms at 10 ticks, and glows from three
    // tenths of the way to the sparks' colour at 100 ticks to seven tenths
    // on the last, evenly: 0.664 of the way at 10.
    let mut world = lone_line(0);
    let (mut on, mut off) = twins(&world);
    let (egg, tile) = until_due_in(&mut world, &mut on, &mut off, 10);
    let ten_out = flashes(&mut on, &world, tile, 600);
    assert!(ten_out.len() >= 2, "{ten_out:?}");
    assert_eq!(
        periods(&ten_out),
        vec![255; ten_out.len() - 1],
        "{ten_out:?}"
    );
    let tenth = Color::Rgb(0xAD, 0x9E, 0x6E);
    assert!(ten_out.iter().all(|&(_, bg)| bg == tenth), "{ten_out:?}");
    let (due, tile) = until_about_to_hatch(&mut world, &mut on, &mut off);
    assert_eq!(due, egg);
    let last_tick = flashes(&mut on, &world, tile, 400);
    assert!(last_tick.len() >= 3, "{last_tick:?}");
    assert_eq!(
        periods(&last_tick),
        vec![120; last_tick.len() - 1],
        "{last_tick:?}"
    );
    assert!(
        last_tick.iter().all(|&(_, bg)| bg == LAST_FLASH),
        "{last_tick:?}"
    );
}

#[test]
fn a_flash_keeps_its_phase_as_a_tick_taken_in_shortens_it() {
    // Design §6.9: a tick taken in shortens the flash under way, which goes
    // on from as far through as it had got, with no jump or new start: 100
    // ms into one of 255 ms, 10 ticks out, is as far through one of 240 ms,
    // 9 ticks out, with 146 ms to go.
    let mut world = lone_line(0);
    let (mut on, mut off) = twins(&world);
    let (egg, tile) = until_due_in(&mut world, &mut on, &mut off, 10);
    let first = cell(&on, tile);
    next_flash(&mut on, &world, tile, 255).expect("a flash starts");
    let before = frame(&mut on, &world, ms(100))[first].bg;
    one_tick(&mut world, &mut on, &mut off);
    let egg = world.egg(egg).expect("not hatched yet");
    assert_eq!(
        (egg.pos(), egg.hatches() + 1 - world.tick()),
        (Some(tile), 9)
    );
    let after = frame(&mut on, &world, Duration::ZERO)[first].bg;
    let (Color::Rgb(r, g, b), Color::Rgb(r2, g2, b2)) = (before, after) else {
        panic!("{before:?}, {after:?}")
    };
    let jump = r.abs_diff(r2).max(g.abs_diff(g2)).max(b.abs_diff(b2));
    assert!(jump <= 1, "{before:?} to {after:?}");
    let next = flashes(&mut on, &world, tile, 400);
    assert!(next.len() >= 2, "{next:?}");
    assert!(matches!(next[0].0, 145 | 146), "{next:?}");
    assert_eq!(periods(&next), vec![240; next.len() - 1], "{next:?}");
}

#[test]
fn an_egg_picked_up_stops_flashing_and_put_down_flashes_afresh() {
    // Design §6.9: held, an egg is off the map and the screen forgets its
    // flashing; put down, its first flash starts at once, so the next comes
    // a whole flash later: 225 ms, 8 ticks out.
    let mut world = lone_line(0);
    let (mut on, mut off) = twins(&world);
    let (egg, tile) = until_due_in(&mut world, &mut on, &mut off, 10);
    next_flash(&mut on, &world, tile, 255).expect("a flash starts");
    frame(&mut on, &world, ms(100));
    apply(&mut on, &world, Action::Mode(CursorMode::Grab));
    let pointer = pointer_on(&on, tile);
    apply(&mut on, &world, Action::left_click(pointer));
    common::tick(&mut on, &mut world);
    assert_eq!(world.cursor().holds().map(|held| held.id()), Some(egg));
    let map = world.map();
    let free = (0..map.height())
        .flat_map(|y| (0..map.width()).map(move |x| at(x, y)))
        .find(|&tile| world.sprite_at(tile).is_none() && world.object_at(tile).is_none())
        .expect("a free tile");
    let pointer = pointer_on(&on, free);
    apply(&mut on, &world, Action::left_click(pointer));
    common::tick(&mut on, &mut world);
    let egg = world.egg(egg).expect("not hatched yet");
    assert_eq!(
        (egg.pos(), egg.hatches() + 1 - world.tick()),
        (Some(free), 8)
    );
    let next = next_flash(&mut on, &world, free, 300).map(|(n, _)| n);
    assert_eq!(next, Some(225));
}

#[test]
fn a_hatching_takes_over_from_the_flashing_its_last_flash_fading_as_the_young_warms() {
    // Design §6.9, #306: on its last tick, while no tick comes, an egg
    // flashes every 120 ms until it hatches, which starts where the last
    // flash peaks, seven tenths of the way to the sparks' colour, and holds
    // so while the egg shows cracked; the glow fades as the young warms,
    // from 150 ms to 650.
    let mut world = lone_line(0);
    let (mut on, mut off) = twins(&world);
    let (egg, tile) = until_about_to_hatch(&mut world, &mut on, &mut off);
    frame(&mut on, &world, ms(5000));
    let later = flashes(&mut on, &world, tile, 400);
    assert!(later.len() >= 3, "{later:?}");
    assert_eq!(periods(&later), vec![120; later.len() - 1], "{later:?}");
    let events = one_tick(&mut world, &mut on, &mut off);
    assert!(
        events
            .iter()
            .any(|event| matches!(event.kind, EventKind::Hatched { id, .. } if id == egg)),
        "{events:?}"
    );
    let young = cell(&on, world.sprite(egg).expect("hatched").pos());
    let start = frame(&mut on, &world, Duration::ZERO);
    assert_eq!(start[young].bg, LAST_FLASH);
    let appears = frame(&mut on, &world, ms(150));
    assert_eq!(appears[young].bg, LAST_FLASH);
    let warming = frame(&mut on, &world, ms(250))[young].bg;
    assert!(
        brightness(warming) < brightness(LAST_FLASH) && brightness(warming) > brightness(DARK),
        "{warming:?}"
    );
    let warm = frame(&mut on, &world, ms(250));
    let plain = frame(&mut off, &world, ms(650));
    assert_eq!(warm[young].bg, plain[young].bg);
}

/// Runs `world` until an egg lying on the map, with no sprite on it, is
/// past its tick, both apps taking in each tick: the egg's ID and its tile.
/// A sprite stood on it when its stages ended and has stepped off, so it
/// hatches on the next tick.
fn until_past_due_and_clear(world: &mut World, on: &mut App, off: &mut App) -> (EntityId, Pos) {
    for _ in 0..2000 {
        one_tick(world, on, off);
        let due = world.eggs().find_map(|egg| {
            let tile = egg.pos()?;
            (egg.hatches() < world.tick() && world.sprite_at(tile).is_none())
                .then_some((egg.id(), tile))
        });
        if let Some(due) = due {
            return due;
        }
    }
    panic!("no egg was stood on past its tick");
}

#[test]
fn an_egg_stood_on_past_its_tick_flashes_once_clear_and_its_young_starts_at_the_last_flash() {
    // Design §6.9: an egg a sprite stands on as its stages end hatches on
    // the tick after the sprite steps off. Clear, it flashes as on its last
    // tick until then, every 120 ms and seven tenths of the way, and the
    // young starts where the last flash peaks.
    let mut world = lone_line(0);
    let (mut on, mut off) = twins(&world);
    let (egg, tile) = until_past_due_and_clear(&mut world, &mut on, &mut off);
    let last_tick = flashes(&mut on, &world, tile, 400);
    assert!(last_tick.len() >= 3, "{last_tick:?}");
    assert_eq!(
        periods(&last_tick),
        vec![120; last_tick.len() - 1],
        "{last_tick:?}"
    );
    assert!(
        last_tick.iter().all(|&(_, bg)| bg == LAST_FLASH),
        "{last_tick:?}"
    );
    let events = one_tick(&mut world, &mut on, &mut off);
    assert!(
        events
            .iter()
            .any(|event| matches!(event.kind, EventKind::Hatched { id, .. } if id == egg)),
        "{events:?}"
    );
    let young = cell(&on, world.sprite(egg).expect("hatched").pos());
    let start = frame(&mut on, &world, Duration::ZERO);
    assert_eq!(start[young].bg, LAST_FLASH);
}

/// Selects the egg on `tile` in both twins with a click, as the player
/// does, which leaves the Cursor on its tile.
fn click_egg(world: &World, on: &mut App, off: &mut App, egg: EntityId, tile: Pos) {
    both(on, off, world, Action::left_click(pointer_on(on, tile)));
    assert_eq!(on.selection(), Some(Selection::Living(egg)));
    assert_eq!(on.cursor(), tile);
}

/// Points both twins away from `tile`, so the Cursor leaves it.
fn point_away(world: &World, on: &mut App, off: &mut App, tile: Pos) {
    let away = at(if tile.x > 5 { 0 } else { 11 }, 0);
    both(on, off, world, Action::Point(pointer_on(on, away)));
    assert_eq!(on.cursor(), away);
}

/// Whether `colour` is exact and within `steps` of `to` in each part, as a
/// fade is a millisecond or so before its end.
fn within(colour: Color, to: Color, steps: u8) -> bool {
    match (colour, to) {
        (Color::Rgb(r, g, b), Color::Rgb(r2, g2, b2)) => {
            r.abs_diff(r2) <= steps && g.abs_diff(g2) <= steps && b.abs_diff(b2) <= steps
        }
        _ => false,
    }
}

/// Whether screen cell `cell` is drawn in reverse video.
fn reversed(screen: &Buffer, cell: Position) -> bool {
    screen[cell].modifier.contains(Modifier::REVERSED)
}

#[test]
fn a_selected_egg_s_flash_shows_through_its_reverse_video() {
    // Design §6.9, found filming Look 5: a selected egg is drawn in reverse
    // video, its colour filling its tile and its background showing only
    // in its glyph's strokes, so a flash glowing the background hardly
    // showed. There the flash takes the reverse video's place: at once the
    // egg is drawn in its colour on the glow, as an egg not selected is,
    // and the reverse video comes back as the flash fades.
    let [quick, _] = common::QUICK;
    let mut world = grow(
        &[quick, SLOW_EGGS, common::LONE],
        (12, 6),
        &[(at(1, 4), LONE_MOTHER)],
        0,
    );
    let (mut on, mut off) = twins(&world);
    let (egg, tile) = until_due_in(&mut world, &mut on, &mut off, 100);
    click_egg(&world, &mut on, &mut off, egg, tile);
    point_away(&world, &mut on, &mut off, tile);
    let first = cell(&on, tile);
    on.clock.pause();
    let plain = frame(&mut off, &world, Duration::ZERO);
    assert!(reversed(&plain, first));
    let start = frame(&mut on, &world, Duration::ZERO);
    assert!(reversed(&start, first));
    assert_eq!(start[first].symbol(), plain[first].symbol());
    // Reversed, the cell's foreground fills the tile and its background
    // draws the glyph.
    assert_eq!((start[first].fg, start[first].bg), (FIRST_FLASH, EGG));
    let fading = frame(&mut on, &world, ms(802));
    assert!(reversed(&fading, first));
    let filling = brightness(fading[first].fg);
    assert!(
        filling > brightness(FIRST_FLASH) && filling < brightness(EGG),
        "{:?}",
        fading[first].fg
    );
    let faded = frame(&mut on, &world, ms(802));
    assert!(reversed(&faded, first));
    assert!(within(faded[first].fg, EGG, 1), "back by its end");
    assert_eq!(faded[first].bg, DARK, "back by its end");
    let again = frame(&mut on, &world, ms(1));
    assert_eq!(
        (again[first].fg, again[first].bg),
        (FIRST_FLASH, EGG),
        "the next"
    );
}

#[test]
fn a_selected_egg_s_young_starts_on_the_last_flash_and_its_reverse_video_comes_back() {
    // Design §6.9: the young keeps the egg's ID, so it's selected too, and
    // drawn in reverse video but while it shows an emote, as a hatchling
    // often does; the Cursor's tile, where the click that selected the egg
    // left it, is drawn so either way. There the glow of the last flash
    // takes the reverse video's place while the egg shows cracked, and
    // fades into it as the young warms, from 150 ms to 650.
    let mut world = lone_line(0);
    let (mut on, mut off) = twins(&world);
    let (egg, tile) = until_about_to_hatch(&mut world, &mut on, &mut off);
    click_egg(&world, &mut on, &mut off, egg, tile);
    frame(&mut on, &world, ms(5000));
    let events = one_tick(&mut world, &mut on, &mut off);
    assert!(
        events
            .iter()
            .any(|event| matches!(event.kind, EventKind::Hatched { id, .. } if id == egg)),
        "{events:?}"
    );
    assert_eq!(on.selection(), Some(Selection::Living(egg)));
    let young = cell(&on, world.sprite(egg).expect("hatched").pos());
    let cracked = frame(&mut on, &world, Duration::ZERO);
    assert!(reversed(&cracked, young));
    assert_eq!((cracked[young].fg, cracked[young].bg), (LAST_FLASH, EGG));
    let appears = frame(&mut on, &world, ms(150));
    assert!(reversed(&appears, young));
    assert_eq!(appears[young].fg, LAST_FLASH);
    // Three quarters back at half time: the strokes nearly dark again.
    let warming = frame(&mut on, &world, ms(250));
    assert!(reversed(&warming, young));
    let strokes = brightness(warming[young].bg);
    assert!(
        strokes > brightness(DARK) && strokes < brightness(EGG) / 2,
        "{:?}",
        warming[young].bg
    );
    let warm = frame(&mut on, &world, ms(250));
    let plain = frame(&mut off, &world, ms(650));
    assert_eq!(warm[young], plain[young]);
}

#[test]
fn at_4x_an_egg_flashes_as_at_1x() {
    // Design §6.9: the flashes come at the period of the ticks left, so at
    // 4× and up they quicken with the game, and the 100 ticks pass in 20 s.
    let mut world = lone_line(0);
    let (mut on, mut off) = twins(&world);
    at_4x(&mut on);
    at_4x(&mut off);
    let (_, tile) = until_about_to_hatch(&mut world, &mut on, &mut off);
    let last_tick = flashes(&mut on, &world, tile, 400);
    assert!(last_tick.len() >= 3, "{last_tick:?}");
    assert_eq!(
        periods(&last_tick),
        vec![120; last_tick.len() - 1],
        "{last_tick:?}"
    );
    assert!(
        last_tick.iter().all(|&(_, bg)| bg == LAST_FLASH),
        "{last_tick:?}"
    );
}

#[test]
fn no_egg_flashes_in_mono_or_when_the_policy_says_no() {
    // Design §6.9: mono has no colour to flash, and a hatching asks
    // `Emotes` first, as the flashing before it does.
    for in_mono in [true, false] {
        let mut world = lone_line(0);
        let theme = if in_mono { mono() } else { Theme::cp437() };
        let (mut on, mut off) = twins_in(&world, theme, SCREEN);
        if !in_mono {
            on.set_policy(NoEmotes);
            off.set_policy(NoEmotes);
        }
        let (_, tile) = until_about_to_hatch(&mut world, &mut on, &mut off);
        let first = cell(&on, tile);
        for step in [0, 40, 80, 1000] {
            let shown = frame(&mut on, &world, ms(step));
            let plain = frame(&mut off, &world, ms(step));
            assert_eq!(shown[first], plain[first], "mono: {in_mono}, {step} ms");
        }
    }
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
    // A tab sweeping in, a pop-up fading in over it and a bar easing.
    let (before, after) = (hungry(&[(at(3, 2), 0.2)]), hungry(&[(at(3, 2), 0.8)]));
    let mut app = watching(&before);
    take_in(&mut app, &after);
    apply(&mut app, &after, Action::NextTab);
    apply(&mut app, &after, Action::PreviousTab);
    apply(&mut app, &after, Action::OpenSettings);
    app.animate(ms(60));
    let screen = render(&app, &after);
    assert_eq!(render(&app, &after), screen);
    // The trail fading, and the view panning.
    let (worlds, id) = walk(&[at(2, 2), at(3, 2)]);
    let mut app = following(&worlds, id);
    app.record(&[], &worlds[1]);
    app.animate(ms(400));
    assert_eq!(render(&app, &worlds[1]), render(&app, &worlds[1]));
    let world = wide();
    let mut app = moving(&world);
    apply(&mut app, &world, Action::SelectNext);
    app.animate(ms(120));
    assert_eq!(render(&app, &world), render(&app, &world));
    // A drive's bar's crest running up it, and breaking.
    let world = hunger_on_the_move(0.396, 0.004);
    let mut app = watching(&world);
    for step in [130, 520] {
        app.animate(ms(step));
        assert_eq!(render(&app, &world), render(&app, &world));
    }
    // An egg cracked, and its shell split and fading.
    let (mut app, _, world) = hatched_by_a_bush(Theme::cp437());
    for step in [100, 300] {
        app.animate(ms(step));
        assert_eq!(render(&app, &world), render(&app, &world));
    }
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
fn a_tab_sweeps_in_from_the_side_it_s_on_of_the_old_one() {
    let world = one_on_a_field();
    // From the right when it's right of the old one, as `]` opens, and
    // from the left when it's left of it. By where the tabs are, not the
    // key: `[` from Body wraps round to World, on Body's right, and `]`
    // from World to Body, on its left.
    for (keys, from_left) in [
        (&[Action::NextTab][..], false),
        (&[Action::PreviousTab], false),
        (&[Action::NextTab, Action::PreviousTab], true),
        (&[Action::PreviousTab, Action::NextTab], true),
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
    assert_eq!(side, Some(false), "from the right, as `]` opens");
    let (end, plain) = frames(&mut on, &mut off, &world, ms(150));
    assert_eq!(end, plain);
}

#[test]
fn in_a_theme_without_shades_a_tab_sweeps_in_out_of_motion_s_dark() {
    // The panel's background is the terminal's own, which the dark stands
    // for, and stays its own: never black or white, as tachyonfx would
    // draw it.
    let world = one_on_a_field();
    let theme = Theme::from_ron(&ascii_with_panels(""), &common::pack()).expect("loads");
    assert_eq!(theme.panels(), None);
    let dark = theme.motion().dark;
    let (mut on, mut off) = selected_in(&world, theme, SCREEN);
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
fn a_tab_sweeps_in_out_of_a_shade_the_theme_names_as_campbell_draws_it() {
    // Design §6.9: a colour the theme names blends from how Campbell draws
    // it, so a tab still to sweep in never shows the terminal's own grey,
    // nor tachyonfx's.
    let theme = Theme::from_ron(
        &ascii_with_panels("    panels: (shade: dark_gray, title: gray),\n"),
        &common::pack(),
    )
    .expect("loads");
    let world = one_on_a_field();
    let (mut on, mut off) = selected_in(&world, theme, SCREEN);
    both(&mut on, &mut off, &world, Action::NextTab);
    let body = tab_body(&on);
    let (start, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    let grey = Color::Rgb(0x76, 0x76, 0x76);
    // Hidden in Campbell's dark grey, or in the named one where the cell
    // was drawn in it, which Campbell draws the same.
    let hidden = |shown: Color, was: Color| shown == grey || shown == was && was == Color::DarkGray;
    for cell in body.positions() {
        assert!(
            hidden(start[cell].fg, plain[cell].fg),
            "{cell:?}: {:?}",
            start[cell]
        );
        assert!(
            hidden(start[cell].bg, plain[cell].bg),
            "{cell:?}: {:?}",
            start[cell]
        );
    }
    differs_only_within(&start, &plain, body);
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

/// The default theme's dark, which a pop-up fades in from and out to.
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
fn a_pop_up_fades_out_to_the_dark_as_it_closes_and_what_was_under_it_back() {
    // Design §6.9: the pop-up as it was last drawn fades out to the dark
    // over the first half, and what was under it fades back over the
    // second, so its characters give way to theirs in the dark.
    let world = one_on_a_field();
    let (mut on, mut off) = selected(&world);
    both(&mut on, &mut off, &world, Action::Help);
    let help = on.overlay().expect("room for help");
    frames(&mut on, &mut off, &world, Duration::ZERO);
    let (open, plain) = frames(&mut on, &mut off, &world, ms(1000));
    assert_eq!(open, plain, "faded in");
    both(&mut on, &mut off, &world, Action::Help);
    let (start, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    for cell in help.positions() {
        assert_eq!(start[cell], open[cell], "{cell:?}");
    }
    differs_only_within(&start, &plain, help);
    // Fast and then slowing: well past half way to the dark at half its
    // half.
    let (quarter, plain) = frames(&mut on, &mut off, &world, ms(37));
    let corner = Position::new(help.x, help.y);
    assert_eq!(quarter[corner].symbol(), open[corner].symbol());
    let Color::Rgb(red, ..) = quarter[corner].fg else {
        panic!("{:?}", quarter[corner].fg)
    };
    assert!(red > 0x0C && red < (0x0C + 0xCC) / 2, "{red:#x}");
    differs_only_within(&quarter, &plain, help);
    // Half way, all of it dark: the map, the inspector and the event log,
    // each to fade back from it.
    let (half, plain) = frames(&mut on, &mut off, &world, ms(38));
    let dark = dark();
    for cell in help.positions() {
        assert_eq!(half[cell].fg, dark, "{cell:?}");
        assert_eq!(half[cell].symbol(), plain[cell].symbol(), "{cell:?}");
    }
    let shaded = Position::new(70, 12);
    assert_eq!(plain[shaded].bg, shade());
    assert_eq!(half[shaded].bg, dark);
    differs_only_within(&half, &plain, help);
    let (end, plain) = frames(&mut on, &mut off, &world, ms(75));
    assert_eq!(end, plain);
}

#[test]
fn each_pop_up_fades_in_as_it_opens_and_out_as_it_closes() {
    // The menus, help, the command palette, the sprite list, the
    // population graph and the family tree (design §6.1). The settings
    // screen, which says whether motion is on, has a test of its own.
    let world = one_on_a_field();
    for opens in [
        Action::OpenSaves,
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
        let (open, plain) = frames(&mut on, &mut off, &world, ms(150));
        assert_eq!(open, plain, "{opens:?}");
        both(&mut on, &mut off, &world, Action::Back);
        // It starts as it was last drawn, and what was under it shows
        // once it has faded out.
        let (start, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
        assert_ne!(start, plain, "{opens:?}");
        for cell in start.area.positions() {
            assert!(
                start[cell] == open[cell] || start[cell] == plain[cell],
                "{opens:?}: {cell:?}"
            );
        }
        let (half, plain) = frames(&mut on, &mut off, &world, ms(75));
        all_that_differs_is_dark(&half, &plain);
        let (end, plain) = frames(&mut on, &mut off, &world, ms(75));
        assert_eq!(end, plain, "{opens:?}");
    }
}

#[test]
fn the_settings_screen_fades_in_as_it_opens_and_out_as_it_closes() {
    // As the menus do (design §6.7, §6.9). Its Motion row says whether
    // motion is on, so it's drawn against itself with motion on, once it
    // has opened and once what was under it is back, paused, so the stars
    // stay still, and once the twins' notices have gone.
    let world = one_on_a_field();
    let (mut on, _) = selected(&world);
    apply(&mut on, &world, Action::TogglePause);
    let plain = frame(&mut on, &world, ms(3000));
    apply(&mut on, &world, Action::OpenSettings);
    let start = frame(&mut on, &world, Duration::ZERO);
    let open = frame(&mut on, &world, ms(150));
    all_that_differs_is_dark(&start, &open);
    assert_eq!(frame(&mut on, &world, ms(1000)), open);
    apply(&mut on, &world, Action::Back);
    let start = frame(&mut on, &world, Duration::ZERO);
    assert_ne!(start, plain);
    for cell in start.area.positions() {
        assert!(
            start[cell] == open[cell] || start[cell] == plain[cell],
            "{cell:?}"
        );
    }
    all_that_differs_is_dark(&frame(&mut on, &world, ms(75)), &plain);
    assert_eq!(frame(&mut on, &world, ms(75)), plain);
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
    // Help fades out as it closes, and what was under it back, but not
    // where the debug overlay is still drawn.
    frames(&mut on, &mut off, &world, ms(1000));
    both(&mut on, &mut off, &world, Action::Help);
    for step in [0, 75] {
        let (shown, plain) = frames(&mut on, &mut off, &world, ms(step));
        if step > 0 {
            all_that_differs_is_dark(&shown, &plain);
        }
        for cell in debug.positions() {
            assert_eq!(shown[cell], plain[cell], "{cell:?}");
        }
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

/// Where the hunger bar on the Body tab on `screen` starts: its first
/// block's screen cell.
fn hunger_bar_at(screen: &Buffer) -> Position {
    let rows = common::lines(screen);
    let (y, x) = rows
        .iter()
        .enumerate()
        .find_map(|(y, row)| {
            let at = row.find(" hunger ")?;
            Some((y as u16, row[..at].chars().count() as u16 + 1))
        })
        .unwrap_or_else(|| panic!("no hunger bar:\n{}", rows.join("\n")));
    Position::new(x + 12, y)
}

/// The hunger bar on the Body tab on `screen`, its ten blocks, and the
/// level it says beside it.
fn hunger_line(screen: &Buffer) -> (String, String) {
    let Position { x, y } = hunger_bar_at(screen);
    let bar: String = (x..x + 10).map(|x| screen[(x, y)].symbol()).collect();
    let said: String = (x + 10..screen.area.right())
        .map(|x| screen[(x, y)].symbol())
        .collect();
    (bar, said.trim().to_string())
}

/// The hunger bar on the Body tab on `screen`: how many thirds of a block
/// it fills (design §6.1), and the level it says beside it.
fn hunger_bar(screen: &Buffer) -> (usize, String) {
    let (bar, said) = hunger_line(screen);
    let thirds: Vec<usize> = bar
        .chars()
        .map(|block| match block {
            '░' => 0,
            '▒' => 1,
            '▓' => 2,
            '█' => 3,
            _ => panic!("not a bar: {bar}"),
        })
        .collect();
    // Full blocks, then at most one filled in part, then empty ones.
    assert!(thirds.windows(2).all(|two| two[0] >= two[1]), "{bar}");
    assert!(
        thirds.iter().filter(|&&part| part % 3 != 0).count() <= 1,
        "{bar}"
    );
    (thirds.iter().sum(), said)
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
fn a_drive_s_bar_eases_to_its_new_value() {
    // Thirds of a block: .2 is 6 of them, .8 is 24.
    let (before, after) = (hungry(&[(at(3, 2), 0.2)]), hungry(&[(at(3, 2), 0.8)]));
    let mut app = watching(&before);
    let shown = hunger_bar(&frame(&mut app, &before, Duration::ZERO));
    assert_eq!(shown, (6, ".20".to_string()));
    take_in(&mut app, &after);
    // The level beside it is true at once; only the bar eases.
    let start = hunger_bar(&frame(&mut app, &after, Duration::ZERO));
    assert_eq!(start, (6, ".80".to_string()));
    let mut filled = vec![start.0];
    for _ in 0..8 {
        let (thirds, said) = hunger_bar(&frame(&mut app, &after, ms(50)));
        assert_eq!(said, ".80");
        filled.push(thirds);
    }
    // Never back, and there in 0.4 s.
    assert!(filled.windows(2).all(|two| two[0] <= two[1]), "{filled:?}");
    assert_eq!(filled[8], 24, "{filled:?}");
    assert!(filled[2] > 6 && filled[2] < 24, "{filled:?}");
    // Quick at first and slowing: most of the way in the first half.
    assert!(filled[4] - 6 > 24 - filled[4], "{filled:?}");
}

#[test]
fn a_block_fills_a_third_at_a_time_as_its_bar_eases() {
    // Design §6.1, §6.9: from .2 to .3, 6 thirds of a block to 9, the
    // third block fills ░ ▒ ▓ █ rather than all at once.
    let (before, after) = (hungry(&[(at(3, 2), 0.2)]), hungry(&[(at(3, 2), 0.3)]));
    let mut app = watching(&before);
    take_in(&mut app, &after);
    let mut third: Vec<char> = Vec::new();
    for step in [0, 50, 50, 50, 50, 50, 50, 50, 50] {
        let (bar, _) = hunger_line(&frame(&mut app, &after, ms(step)));
        let block = bar.chars().nth(2).expect("ten blocks");
        if third.last() != Some(&block) {
            third.push(block);
        }
    }
    assert_eq!(third, ['░', '▒', '▓', '█']);
}

#[test]
fn a_bar_eases_on_from_where_it_s_got_to() {
    // A new level before the last is reached eases from the bar as it's
    // drawn, without jumping.
    let worlds = [0.2, 0.8, 0.5].map(|hunger| hungry(&[(at(3, 2), hunger)]));
    let mut app = watching(&worlds[0]);
    take_in(&mut app, &worlds[1]);
    let (part, _) = hunger_bar(&frame(&mut app, &worlds[1], ms(100)));
    assert!(part > 6 && part < 24, "{part}");
    take_in(&mut app, &worlds[2]);
    let (then, said) = hunger_bar(&frame(&mut app, &worlds[2], Duration::ZERO));
    assert_eq!((then, said.as_str()), (part, ".50"));
    let (end, _) = hunger_bar(&frame(&mut app, &worlds[2], ms(400)));
    assert_eq!(end, 15);
}

#[test]
fn a_bar_snaps_on_first_sight_on_a_new_selection_with_motion_off_and_at_4x() {
    let (before, after) = (hungry(&[(at(3, 2), 0.2)]), hungry(&[(at(3, 2), 0.8)]));
    // First seen at .8: drawn full at once, not filling from empty.
    let mut app = watching(&after);
    let (thirds, _) = hunger_bar(&frame(&mut app, &after, Duration::ZERO));
    assert_eq!(thirds, 24, "first sight");
    // Another sprite selected shows its own at once.
    let two = hungry(&[(at(3, 2), 0.2), (at(6, 2), 0.8)]);
    let mut app = watching(&two);
    select(&mut app, &two, sprite_on(&two, at(3, 2)));
    app.record(&[], &two);
    let (thirds, _) = hunger_bar(&frame(&mut app, &two, ms(1000)));
    assert_eq!(thirds, 6);
    select(&mut app, &two, sprite_on(&two, at(6, 2)));
    app.record(&[], &two);
    let (thirds, _) = hunger_bar(&frame(&mut app, &two, Duration::ZERO));
    assert_eq!(thirds, 24, "a new selection");
    // With motion off, and at 4×, it jumps.
    for faster in [false, true] {
        let mut app = watching(&before);
        if faster {
            at_4x(&mut app);
        } else {
            app.set_motion(false);
        }
        take_in(&mut app, &after);
        let (thirds, _) = hunger_bar(&frame(&mut app, &after, Duration::ZERO));
        assert_eq!(thirds, 24, "4x: {faster}");
    }
}

// A drive's bar showing how fast it rises (design §6.9).

/// The genes of a sprite whose hunger starts at `from` and changes by `by`
/// each tick, whatever it does.
fn hunger_moving(from: f32, by: f32) -> String {
    format!(
        r#"(format: 1, genes: [
            InitialConcentration(chem: "hunger", value: {from}),
            Emitter(locus: Locus("always"), mode: Level, gain: {by}, chem: "hunger"),
        ])"#
    )
}

/// A field with one sprite on it whose hunger starts at `from` and changes
/// by `by` each tick, a tick on, so its hunger has moved once: the same
/// sprite, by its ID, whatever the numbers.
fn hunger_on_the_move(from: f32, by: f32) -> World {
    let genes = hunger_moving(from, by);
    let mut world = WorldBuilder::grass(10, 6)
        .genes(&[(at(3, 2), genes.as_str())])
        .build();
    world.step();
    world
}

/// The hunger bar's ten blocks on the Body tab on `screen`.
fn hunger_blocks(screen: &Buffer) -> Vec<Cell> {
    let Position { x, y } = hunger_bar_at(screen);
    (x..x + 10).map(|x| screen[(x, y)].clone()).collect()
}

/// How lit a block's glyph is: how bright its colour is as Campbell draws
/// it, a named one too.
fn lit(block: &Cell) -> u32 {
    brightness(campbell::exact(block.fg, FOREGROUND))
}

/// The block whose glyph is lit the most of the first `filled`: the crest.
fn crest(blocks: &[Cell], filled: usize) -> usize {
    let lit: Vec<u32> = blocks[..filled].iter().map(lit).collect();
    let most = lit.iter().max().copied().unwrap_or(0);
    lit.iter().position(|&one| one == most).unwrap_or(0)
}

#[test]
fn a_rising_drive_s_crest_runs_up_its_bar_to_the_end() {
    // Design §6.9: hunger at .4, 12 thirds of a block, four blocks full,
    // and rising. A sprite first seen rising rises as usual for it, and
    // the crest runs from before the bar's first block to its end, the
    // fourth, in 0.52 s, one block every 0.13 s: at 0.12, 0.24, 0.36 and
    // 0.48 s, before it breaks, it's over each block in turn.
    let world = hunger_on_the_move(0.396, 0.004);
    let mut app = watching(&world);
    let (bar, _) = hunger_line(&render(&app, &world));
    assert_eq!(bar, "████░░░░░░");
    let plain = lit(&hunger_blocks(&frame_with_motion_off(&world))[0]);
    let mut crests = Vec::new();
    let mut blocks = Vec::new();
    for _ in 0..4 {
        blocks = hunger_blocks(&frame(&mut app, &world, ms(120)));
        let at = crest(&blocks, 4);
        assert!(lit(&blocks[at]) > plain, "{blocks:?}");
        crests.push(at);
    }
    assert_eq!(crests, [0, 1, 2, 3]);
    // There it's the crest, a block and a bit wide, lighting the block
    // before the end too, as the break's flare never does.
    assert!(lit(&blocks[2]) > plain, "{blocks:?}");
}

/// The glyphs of `blocks`.
fn glyphs(blocks: &[Cell]) -> String {
    blocks.iter().map(|block| block.symbol()).collect()
}

#[test]
fn a_crest_breaks_at_the_bar_s_end_and_its_spray_runs_on_past_the_level() {
    // Design §6.9: once the crest reaches the end, 0.52 s on, it breaks,
    // the end block flaring at once and settling, and its spray tints the
    // empty block past the level and dies away, over 0.695 s, its glyph
    // still ░ so the level never looks higher. Rising as usual, the spray
    // reaches one block; then all is still until the next wave.
    let world = hunger_on_the_move(0.396, 0.004);
    let mut app = watching(&world);
    let plain = hunger_blocks(&frame_with_motion_off(&world));
    let broke = hunger_blocks(&frame(&mut app, &world, ms(520)));
    assert!(lit(&broke[3]) > lit(&plain[3]), "the end flares");
    assert_ne!(broke[3].bg, plain[3].bg, "its background too");
    let (mut flares, mut sprayed) = (vec![lit(&broke[3])], false);
    for _ in 0..34 {
        let blocks = hunger_blocks(&frame(&mut app, &world, ms(20)));
        assert_eq!(glyphs(&blocks), "████░░░░░░");
        flares.push(lit(&blocks[3]));
        sprayed |= blocks[4].fg != plain[4].fg;
        assert_eq!(blocks[4].bg, plain[4].bg, "the spray tints only the glyph");
        assert_eq!(blocks[5..], plain[5..], "and reaches one block");
    }
    assert!(sprayed, "the spray runs past the level");
    assert!(flares.windows(2).all(|two| two[0] >= two[1]), "{flares:?}");
    let still = hunger_blocks(&frame(&mut app, &world, ms(20)));
    assert_eq!(still, plain, "the break is over by 1.215 s");
    assert_eq!(hunger_blocks(&frame(&mut app, &world, ms(600))), plain);
}

/// An app watching the one sprite of `hunger_on_the_move`, its hunger at
/// .4 and rising as usual for it, .004 a tick, that takes in a tick of
/// `then`, the same sprite, once its first wave has broken, 1.3 s on.
fn rising_then(then: &World) -> App {
    let usual = hunger_on_the_move(0.396, 0.004);
    let mut app = watching(&usual);
    for _ in 0..5 {
        take_in(&mut app, &usual);
    }
    frame(&mut app, &usual, ms(1300));
    take_in(&mut app, then);
    app
}

/// What the waves on the hunger bar of the one sprite on `world`, four
/// blocks full, did over `for_ms` milliseconds of `app`'s frames 10 ms
/// apart: how lit the crest got on the first three blocks, how many empty
/// blocks the spray reached, and how many crests set off up the bar.
fn waves_over(app: &mut App, world: &World, for_ms: u64) -> (u32, usize, usize) {
    let plain = hunger_blocks(&frame_with_motion_off(world));
    let (mut tallest, mut reach, mut crests, mut was_lit) = (0, 0, 0, false);
    for _ in 0..for_ms / 10 {
        let blocks = hunger_blocks(&frame(app, world, ms(10)));
        tallest = tallest.max(blocks[..3].iter().map(lit).max().unwrap_or(0));
        let sprayed = (4..10).filter(|&i| blocks[i].fg != plain[i].fg);
        reach = reach.max(sprayed.map(|i| i - 3).max().unwrap_or(0));
        let lit_up = blocks[0] != plain[0];
        crests += usize::from(lit_up && !was_lit);
        was_lit = lit_up;
    }
    (tallest, reach, crests)
}

#[test]
fn the_faster_a_drive_rises_against_its_usual_the_taller_and_more_often_its_waves() {
    // Design §6.9: hunger rising .004 a tick, as usual for it, and then 2.5
    // and 5 times as fast, at .4 each time. Its crest comes taller, its
    // waves more often, every 1.925 s, 1.475 s and 0.8 s, and its spray
    // reaches one, two and three empty blocks past the level.
    let seen: Vec<(u32, usize, usize)> = [(0.396, 0.004), (0.39, 0.01), (0.38, 0.02)]
        .into_iter()
        .map(|(from, by)| {
            let then = hunger_on_the_move(from, by);
            waves_over(&mut rising_then(&then), &then, 4000)
        })
        .collect();
    let tallest: Vec<u32> = seen.iter().map(|&(tallest, _, _)| tallest).collect();
    assert!(tallest.windows(2).all(|two| two[0] < two[1]), "{seen:?}");
    let reach: Vec<usize> = seen.iter().map(|&(_, reach, _)| reach).collect();
    assert_eq!(reach, [1, 2, 3], "{seen:?}");
    let crests: Vec<usize> = seen.iter().map(|&(_, _, crests)| crests).collect();
    assert_eq!(crests, [2, 3, 5], "{seen:?}");
}

#[test]
fn a_bar_s_waves_change_only_as_ticks_are_taken_in() {
    // Design §6.9: a frame that takes in no tick, as most do at 1×, leaves
    // them as they were, though the world's last tick says hunger rose
    // five times as fast as usual; the next tick taken in shows it.
    let usual = hunger_on_the_move(0.396, 0.004);
    let gently = waves_over(&mut rising_then(&usual), &usual, 4000);
    let then = hunger_on_the_move(0.38, 0.02);
    let mut app = watching(&usual);
    for _ in 0..5 {
        take_in(&mut app, &usual);
    }
    frame(&mut app, &usual, ms(1300));
    for _ in 0..5 {
        app.take_in(Ticks::default(), &then);
    }
    assert_eq!(waves_over(&mut app, &then, 4000), gently);
    take_in(&mut app, &then);
    let (_, reach, _) = waves_over(&mut app, &then, 4000);
    assert_eq!(reach, 3);
}

/// Twins watching the one sprite of `hunger_on_the_move`, its hunger at .4
/// and rising as usual for it, 0.3 s into its first wave, as the crest
/// runs up the bar: the first then takes in a tick of `then`, the same
/// sprite, and the other takes in none.
fn mid_crest(then: &World) -> (App, App) {
    let usual = hunger_on_the_move(0.396, 0.004);
    let (mut ticked, mut still) = (watching(&usual), watching(&usual));
    for app in [&mut ticked, &mut still] {
        frame(app, &usual, ms(300));
    }
    take_in(&mut ticked, then);
    (ticked, still)
}

/// Plays the twins of `mid_crest` on, frame by frame 10 ms apart, until the
/// first wave has shown, 1.215 s into it, and checks that they draw the
/// hunger bar alike all the while, and not plainly: the tick `ticked` took
/// in, of `then`, left the wave under way as it began.
fn plays_out(ticked: &mut App, then: &World, still: &mut App) {
    let usual = hunger_on_the_move(0.396, 0.004);
    let plain = hunger_blocks(&frame_with_motion_off(then));
    let mut played = false;
    for at in (310..1215).step_by(10) {
        let shown = hunger_blocks(&frame(ticked, then, ms(10)));
        let unticked = hunger_blocks(&frame(still, &usual, ms(10)));
        assert_eq!(shown, unticked, "{at} ms into the wave");
        played |= shown != plain;
    }
    assert!(played, "the wave plays");
}

/// When crests set off up the hunger bar of the one sprite on `world`, four
/// blocks full, over `app`'s frames 10 ms apart from `from` milliseconds
/// into its first wave to `to`: each frame where its first block lights
/// up.
fn set_off(app: &mut App, world: &World, (from, to): (u64, u64)) -> Vec<u64> {
    let plain = hunger_blocks(&frame_with_motion_off(world)).remove(0);
    let mut was_lit = hunger_blocks(&render(app, world))[0] != plain;
    let mut times = Vec::new();
    for at in (from + 10..=to).step_by(10) {
        let lit = hunger_blocks(&frame(app, world, ms(10)))[0] != plain;
        if lit && !was_lit {
            times.push(at);
        }
        was_lit = lit;
    }
    times
}

/// Checks that the crests `set_off` saw set off at `crests` came at the
/// moments `due`, in milliseconds into the first wave: each within a frame
/// of its own.
fn sets_off_at(crests: &[u64], due: &[u64]) {
    let near = crests.len() == due.len()
        && crests
            .iter()
            .zip(due)
            .all(|(at, due)| at.abs_diff(*due) <= 10);
    assert!(near, "set off at {crests:?}, due at {due:?}");
}

#[test]
fn a_quicker_tick_mid_wave_lets_it_play_out_and_the_next_come_once_it_has_shown() {
    // Design §6.9: hunger rising as usual for it, 0.3 s into a wave that
    // shows for 1.215 s, and then a tick five times as fast, whose waves
    // come every 0.8 s, as one lands mid-wave in play at 1×. The wave under
    // way plays out as it began, without a jump; the next sets off once it
    // has shown, at 1.215 s, not at once nor at 1.925 s as at the usual
    // pace; and each after it 0.8 s on.
    let fast = hunger_on_the_move(0.38, 0.02);
    let (mut ticked, mut still) = mid_crest(&fast);
    plays_out(&mut ticked, &fast, &mut still);
    let crests = set_off(&mut ticked, &fast, (1210, 3000));
    sets_off_at(&crests, &[1215, 2015, 2815]);
}

#[test]
fn a_quicker_tick_after_a_wave_brings_the_next_at_its_pace_or_at_once_if_overdue() {
    // Design §6.9: hunger rising as usual for it, and a tick taken in 1.3 s
    // into a wave, once it has shown. Rising 2.5 times as fast, its waves
    // come every 1.475 s, so the next sets off 1.475 s after the last
    // began, not at once nor at 1.925 s as at the usual pace.
    let faster = hunger_on_the_move(0.39, 0.01);
    let mut app = rising_then(&faster);
    let crests = set_off(&mut app, &faster, (1300, 4500));
    sets_off_at(&crests, &[1475, 2950, 4425]);
    // Five times as fast, every 0.8 s, the next is overdue, so it sets off
    // at once, and each after it 0.8 s on.
    let fastest = hunger_on_the_move(0.38, 0.02);
    let mut app = rising_then(&fastest);
    let plain = hunger_blocks(&frame_with_motion_off(&fastest));
    let blocks = hunger_blocks(&render(&app, &fastest));
    assert!(lit(&blocks[0]) > lit(&plain[0]), "at once: {blocks:?}");
    let crests = set_off(&mut app, &fastest, (1300, 3000));
    sets_off_at(&crests, &[2100, 2900]);
}

#[test]
fn a_steady_tick_mid_wave_lets_it_play_out_and_brings_no_other() {
    // Design §6.9: hunger rising as usual for it, 0.3 s into a wave, and
    // then a tick it holds steady over. The wave under way plays out as it
    // began, breaking at the bar's end, and no other comes, where at the
    // usual pace the next would set off at 1.925 s and 3.85 s.
    let steady = hunger_on_the_move(0.4, 0.0);
    let (mut ticked, mut still) = mid_crest(&steady);
    plays_out(&mut ticked, &steady, &mut still);
    let usual = hunger_on_the_move(0.396, 0.004);
    let crests = set_off(&mut still, &usual, (1210, 5300));
    sets_off_at(&crests, &[1925, 3850]);
    let plain = hunger_blocks(&frame_with_motion_off(&steady));
    for at in (1220..=5300).step_by(10) {
        let blocks = hunger_blocks(&frame(&mut ticked, &steady, ms(10)));
        assert_eq!(blocks, plain, "{at} ms into the wave");
    }
}

#[test]
fn a_sprite_selected_while_paused_shows_its_drive_s_last_change_at_once() {
    // Design §6.9: paused, no tick comes, and the frame that selects the
    // sprite takes in none, as most frames at 1× do, yet its hunger rose
    // over the last tick, so its crest sets off up the bar at once, from
    // before its first block to its end in 0.52 s.
    let world = hunger_on_the_move(0.396, 0.004);
    let mut app = moving(&world);
    apply(&mut app, &world, Action::TogglePause);
    apply(&mut app, &world, Action::SelectNext);
    app.take_in(Ticks::default(), &world);
    let plain = lit(&hunger_blocks(&frame_with_motion_off(&world))[0]);
    // Once the tab has swept in: 0.3, 0.4 and 0.48 s on.
    let mut crests = Vec::new();
    for step in [300, 100, 80] {
        let blocks = hunger_blocks(&frame(&mut app, &world, ms(step)));
        let at = crest(&blocks, 4);
        assert!(lit(&blocks[at]) > plain, "{blocks:?}");
        crests.push(at);
    }
    assert_eq!(crests, [1, 2, 3]);
}

#[test]
fn a_drive_rising_as_fast_as_it_usually_does_waves_gently_however_fast_that_is() {
    // Design §6.9: the waves weigh a rise against the drive's own usual
    // rise, the average of its latest, which a sprite selected starts
    // afresh. Hunger rising .02 a tick from the first, at .4, waves as
    // gently as hunger rising .004 does.
    let watched = |from, by| {
        let world = hunger_on_the_move(from, by);
        let mut app = watching(&world);
        for _ in 0..5 {
            take_in(&mut app, &world);
        }
        waves_over(&mut app, &world, 4000)
    };
    let slow = watched(0.396, 0.004);
    assert_eq!(slow.1, 1, "{slow:?}");
    assert_eq!(watched(0.38, 0.02), slow);
    // So does one selected after a sprite whose hunger rises five times
    // slower.
    let (slow_genes, fast_genes) = (hunger_moving(0.396, 0.004), hunger_moving(0.38, 0.02));
    let mut world = WorldBuilder::grass(10, 6)
        .genes(&[
            (at(3, 2), slow_genes.as_str()),
            (at(6, 2), fast_genes.as_str()),
        ])
        .build();
    world.step();
    let mut app = moving(&world);
    for tile in [at(3, 2), at(6, 2)] {
        select(&mut app, &world, sprite_on(&world, tile));
        frame(&mut app, &world, ms(1000));
        app.record(&[], &world);
        for _ in 0..5 {
            take_in(&mut app, &world);
        }
    }
    assert_eq!(waves_over(&mut app, &world, 4000), slow);
}

#[test]
fn a_falling_drive_s_bar_dips_slowly_back_down_and_a_steady_one_s_stays_still() {
    // Design §6.9: hunger falling .004 a tick to .4, four blocks full. A
    // faint dark dip runs from past the bar's end back down it in 1.1 s,
    // one block every 0.22 s, dimming only the filled blocks' glyphs, and
    // again 2.6 s after it began.
    let world = hunger_on_the_move(0.404, -0.004);
    let mut app = watching(&world);
    let plain = hunger_blocks(&frame_with_motion_off(&world));
    let mut dips = Vec::new();
    for _ in 0..4 {
        let blocks = hunger_blocks(&frame(&mut app, &world, ms(220)));
        let shown: Vec<u32> = blocks[..4].iter().map(lit).collect();
        let least = shown.iter().min().copied().unwrap_or(0);
        assert!(least < lit(&plain[0]), "{blocks:?}");
        dips.push(shown.iter().position(|&one| one == least).unwrap_or(0));
        let backgrounds =
            |blocks: &[Cell]| -> Vec<Color> { blocks.iter().map(|block| block.bg).collect() };
        assert_eq!(backgrounds(&blocks), backgrounds(&plain), "only the glyphs");
        assert_eq!(blocks[4..], plain[4..], "only the filled blocks");
    }
    assert_eq!(dips, [3, 2, 1, 0]);
    assert_eq!(hunger_blocks(&frame(&mut app, &world, ms(240))), plain);
    assert_eq!(hunger_blocks(&frame(&mut app, &world, ms(1400))), plain);
    assert_ne!(
        hunger_blocks(&frame(&mut app, &world, ms(400))),
        plain,
        "again"
    );
    // Hunger holding steady shows nothing.
    let world = hunger_on_the_move(0.4, 0.0);
    let mut app = watching(&world);
    for _ in 0..30 {
        assert_eq!(hunger_blocks(&frame(&mut app, &world, ms(100))), plain);
    }
}

#[test]
fn waves_play_only_on_the_bars_and_not_at_4x_nor_in_mono() {
    // Design §6.9: they change nothing but a bar's blocks. At 4× and up,
    // where the bars jump, and in mono, which has no colour, they don't
    // play; Game Kid has colour, so they play there, in its greens.
    let world = hunger_on_the_move(0.396, 0.004);
    let (mut on, mut off) = selected(&world);
    on.record(&[], &world);
    off.record(&[], &world);
    let (shown, plain) = frames(&mut on, &mut off, &world, ms(130));
    let Position { x, y } = hunger_bar_at(&shown);
    let bar = Rect::new(x, y, 10, 1);
    assert_ne!(shown, plain, "a wave plays");
    differs_only_within(&shown, &plain, bar);
    // At 4×.
    let plain = hunger_blocks(&plain);
    let mut app = watching(&world);
    at_4x(&mut app);
    take_in(&mut app, &world);
    for _ in 0..30 {
        let blocks = hunger_blocks(&frame(&mut app, &world, ms(50)));
        assert_eq!(blocks, plain, "4x");
    }
    // In mono and Game Kid.
    for (theme, plays) in [(mono(), false), (common::game_kid(), true)] {
        let mut off = watching(&world);
        off.theme = theme.clone();
        off.set_motion(false);
        let plain = hunger_blocks(&render(&off, &world));
        let mut app = moving(&world);
        app.theme = theme;
        apply(&mut app, &world, Action::SelectNext);
        frame(&mut app, &world, ms(1000));
        app.record(&[], &world);
        let mut played = false;
        for _ in 0..30 {
            let blocks = hunger_blocks(&frame(&mut app, &world, ms(50)));
            played |= blocks != plain;
            for block in blocks.iter().filter(|_| plays) {
                for colour in [block.fg, block.bg] {
                    let Some(rgb) = campbell::rgb(colour) else {
                        continue;
                    };
                    let dark = rgb.iter().all(|&channel| channel <= 25);
                    assert!(dark || common::on_the_ramp(rgb), "{colour:?} in Game Kid");
                }
            }
        }
        assert_eq!(played, plays, "{}", if plays { "Game Kid" } else { "mono" });
    }
}

#[test]
fn a_theme_may_choose_the_running_ripple_for_a_drive_s_bar() {
    // Design §6.9: `bar: ripple` in a theme's `motion` section. The
    // default theme's is `crashing`, which a theme leaving it out takes,
    // and a bar the game doesn't know is refused, naming it.
    let pack = common::pack();
    let ripple = Theme::from_ron(&ascii_and("    motion: (bar: ripple),"), &pack).expect("loads");
    assert_eq!(ripple.motion().bar, BarWaves::Ripple);
    assert_eq!(Theme::cp437().motion().bar, BarWaves::Crashing);
    assert_eq!(Theme::ascii().motion().bar, BarWaves::Crashing);
    let err =
        Theme::from_ron(&ascii_and("    motion: (bar: splash),"), &pack).expect_err("refused");
    assert!(err.contains("splash"), "{err}");
    // Rising as usual, its crest runs up the bar and off its end in 0.52
    // s, a block every 0.104 s, with no flare nor spray, and comes again
    // 1.75 s after it set off.
    let world = hunger_on_the_move(0.396, 0.004);
    let mut off = watching(&world);
    off.theme = ripple.clone();
    off.set_motion(false);
    let plain = hunger_blocks(&render(&off, &world));
    let mut app = moving(&world);
    app.theme = ripple;
    apply(&mut app, &world, Action::SelectNext);
    frame(&mut app, &world, ms(1000));
    app.record(&[], &world);
    let mut crests = Vec::new();
    for _ in 0..4 {
        let blocks = hunger_blocks(&frame(&mut app, &world, ms(104)));
        assert_eq!(blocks[4..], plain[4..], "no spray");
        crests.push(crest(&blocks, 4));
    }
    assert_eq!(crests, [0, 1, 2, 3]);
    let gone = hunger_blocks(&frame(&mut app, &world, ms(104)));
    assert_eq!(gone, plain, "off the end, it's gone");
    for _ in 0..61 {
        assert_eq!(hunger_blocks(&frame(&mut app, &world, ms(20))), plain);
    }
    assert_ne!(
        hunger_blocks(&frame(&mut app, &world, ms(20))),
        plain,
        "again"
    );
}

/// The Body tab of the one sprite on `world` drawn with motion off.
fn frame_with_motion_off(world: &World) -> Buffer {
    let mut app = watching(world);
    app.set_motion(false);
    render(&app, world)
}

// The trail (design §6.9).

/// The worlds a sprite walking through `tiles` on a 10×6 field leaves,
/// one a tile, as the screen sees it after each frame's ticks; and its
/// ID, the same in each.
fn walk(tiles: &[Pos]) -> (Vec<World>, EntityId) {
    let worlds: Vec<World> = tiles.iter().map(|&tile| field(&[], &[tile])).collect();
    let id = sprite_on(&worlds[0], tiles[0]);
    for (world, &tile) in worlds.iter().zip(tiles) {
        assert_eq!(sprite_on(world, tile), id, "one sprite, walking");
    }
    (worlds, id)
}

/// An app with motion on that has selected the sprite walking in `walk`,
/// the Cursor out of its way, and taken in the first of `worlds`.
fn following(worlds: &[World], id: EntityId) -> App {
    let mut app = moving(&worlds[0]);
    cursor_to(&mut app, &worlds[0], at(9, 4));
    select(&mut app, &worlds[0], id);
    frame(&mut app, &worlds[0], ms(1000));
    app.record(&[], &worlds[0]);
    app
}

/// Whether each cell of `tile` on `screen` is tinted with the trail's
/// colour, as a tile just left is.
fn trailing(app: &App, screen: &Buffer, tile: Pos) -> bool {
    let trail = app.theme.motion().trail;
    cells(app, tile)
        .iter()
        .all(|&at_cell| screen[at_cell].bg == trail)
}

/// Whether no cell of `tile` on `screen` has a background of its own, as
/// grass the trail has left.
fn untinted(app: &App, screen: &Buffer, tile: Pos) -> bool {
    cells(app, tile)
        .iter()
        .all(|&at_cell| screen[at_cell].bg == Color::Reset)
}

#[test]
fn the_tiles_the_selected_sprite_leaves_keep_a_tint_that_fades() {
    let (worlds, id) = walk(&[at(2, 2), at(3, 2), at(4, 2)]);
    let mut app = following(&worlds, id);
    app.record(&[], &worlds[1]);
    app.animate(ms(500));
    app.record(&[], &worlds[2]);
    let screen = frame(&mut app, &worlds[2], Duration::ZERO);
    // Just left: the trail's colour.
    assert!(trailing(&app, &screen, at(3, 2)));
    // Left half a second before: on its way back to the dark under the
    // grass, quick at first, so past half way a third of the way in.
    let (Color::Rgb(tint, ..), Color::Rgb(dark, ..)) =
        (app.theme.motion().trail, app.theme.motion().dark)
    else {
        panic!("exact colours")
    };
    let Color::Rgb(older, ..) = screen[cell(&app, at(2, 2))].bg else {
        panic!("{:?}", screen[cell(&app, at(2, 2))].bg)
    };
    assert!(older > dark && older < (tint + dark) / 2, "{older:#x}");
    // The sprite's own tile and those it hasn't been on keep their look.
    assert_eq!(screen[cell(&app, at(4, 2))].bg, Color::Reset);
    assert!(untinted(&app, &screen, at(5, 2)));
    // Each tile is over 1.5 s after it was left.
    let screen = frame(&mut app, &worlds[2], ms(1000));
    assert!(untinted(&app, &screen, at(2, 2)));
    assert!(!untinted(&app, &screen, at(3, 2)));
    let screen = frame(&mut app, &worlds[2], ms(500));
    assert!(untinted(&app, &screen, at(3, 2)));
}

#[test]
fn at_4x_a_trail_tile_fades_in_half_the_time() {
    let (worlds, id) = walk(&[at(2, 2), at(3, 2)]);
    let mut app = following(&worlds, id);
    at_4x(&mut app);
    app.record(&[], &worlds[1]);
    let fading = frame(&mut app, &worlds[1], ms(700));
    assert!(!untinted(&app, &fading, at(2, 2)));
    let over = frame(&mut app, &worlds[1], ms(50));
    assert!(untinted(&app, &over, at(2, 2)));
}

#[test]
fn the_trail_tints_at_most_eight_tiles_the_newest() {
    let tiles: Vec<Pos> = (0..10).map(|x| at(x, 2)).collect();
    let (worlds, id) = walk(&tiles);
    let mut app = following(&worlds, id);
    for world in &worlds[1..] {
        app.record(&[], world);
    }
    let screen = frame(&mut app, &worlds[9], Duration::ZERO);
    let tinted: Vec<Pos> = tiles[..9]
        .iter()
        .copied()
        .filter(|&tile| !untinted(&app, &screen, tile))
        .collect();
    assert_eq!(tinted, tiles[1..9]);
}

#[test]
fn the_trail_leaves_the_sprite_s_tile_and_the_cursor_s_their_own_looks() {
    // Back onto a tile it left, and the Cursor on another.
    let (worlds, id) = walk(&[at(2, 2), at(3, 2), at(4, 2), at(3, 2)]);
    let mut app = following(&worlds, id);
    for world in &worlds[1..] {
        app.record(&[], world);
    }
    let screen = frame(&mut app, &worlds[3], Duration::ZERO);
    assert!(trailing(&app, &screen, at(4, 2)));
    assert!(trailing(&app, &screen, at(2, 2)));
    let own = cell(&app, at(3, 2));
    assert_eq!(screen[own].bg, Color::Reset, "the sprite's own tile");
    cursor_to(&mut app, &worlds[3], at(4, 2));
    let screen = frame(&mut app, &worlds[3], Duration::ZERO);
    assert!(untinted(&app, &screen, at(4, 2)), "the Cursor's tile");
}

#[test]
fn the_trail_leaves_the_attended_tile_its_shade() {
    // The sprite stood on a berry, and attends to it from where it went.
    let berry = [(at(0, 1), "berry")];
    let before = WorldBuilder::grass(10, 5)
        .objects(&berry)
        .genes(&[(at(0, 1), HUNGRY_GENOME)])
        .seed(7)
        .build();
    let (after, _) = one_sprite_among(HUNGRY_GENOME, &berry, &[], 1);
    let id = before.sprites().next().expect("the sprite").id();
    let attended = after.sprite(id).and_then(|sprite| sprite.attending_to());
    assert_eq!(attended, Some(at(0, 1)));
    let mut app = following(&[before], id);
    app.record(&[], &after);
    let screen = frame(&mut app, &after, Duration::ZERO);
    let shaded = screen[cell(&app, at(0, 1))].bg;
    assert_eq!(shaded, app.theme.attention_marker());
}

#[test]
fn the_trail_goes_with_a_new_selection_and_the_sprite_s_death() {
    let tiles = [at(2, 2), at(7, 4)];
    let worlds = [field(&[], &tiles), field(&[], &[at(3, 2), at(7, 4)])];
    let (one, two) = (
        sprite_on(&worlds[0], at(2, 2)),
        sprite_on(&worlds[0], at(7, 4)),
    );
    let mut app = following(&worlds, one);
    app.record(&[], &worlds[1]);
    let screen = frame(&mut app, &worlds[1], Duration::ZERO);
    assert!(trailing(&app, &screen, at(2, 2)));
    select(&mut app, &worlds[1], two);
    let screen = frame(&mut app, &worlds[1], Duration::ZERO);
    assert!(untinted(&app, &screen, at(2, 2)));
    // Selected again, it starts afresh.
    select(&mut app, &worlds[1], one);
    let screen = frame(&mut app, &worlds[1], Duration::ZERO);
    assert!(untinted(&app, &screen, at(2, 2)));
    // Its death ends it.
    let mut app = following(&worlds, one);
    app.record(&[], &worlds[1]);
    let gone = field(&[], &[at(7, 4)]);
    app.record(&deaths(&[one]), &gone);
    let screen = frame(&mut app, &gone, Duration::ZERO);
    assert!(untinted(&app, &screen, at(2, 2)));
}

/// A policy that shows everything but the map's markers.
struct NoMarkers;

impl InfoPolicy for NoMarkers {
    fn can_view(&self, panel: Panel, _subject: Subject) -> bool {
        panel != Panel::Markers
    }
}

#[test]
fn the_trail_asks_the_policy_first_and_is_gone_with_motion_off() {
    // Design §6.4: it tells the player where the selected sprite has been,
    // as the markers tell where it's heading, and asks `Markers`.
    let (worlds, id) = walk(&[at(2, 2), at(3, 2)]);
    for hidden in [true, false] {
        let mut app = following(&worlds, id);
        if hidden {
            app.set_policy(NoMarkers);
        } else {
            app.set_motion(false);
        }
        app.record(&[], &worlds[1]);
        let screen = frame(&mut app, &worlds[1], Duration::ZERO);
        assert!(untinted(&app, &screen, at(2, 2)), "hidden: {hidden}");
    }
}

#[test]
fn a_sprite_walking_in_play_leaves_a_trail_behind_it() {
    let start = at(1, 2);
    let mut world = WorldBuilder::grass(12, 5)
        .genes(&[(start, SPEED_10)])
        .scripts(&[(
            start,
            ScriptedAction::Wander {
                destination: at(10, 2),
            },
        )])
        .build();
    let id = sprite_on(&world, start);
    let mut app = following(std::slice::from_ref(&world), id);
    let mut been = vec![start];
    for _ in 0..4 {
        common::tick(&mut app, &mut world);
        app.animate(ms(100));
        been.push(world.sprite(id).expect("walking").pos());
    }
    let here = been.pop().expect("where it is");
    assert_ne!(here, start, "it walked");
    let screen = frame(&mut app, &world, Duration::ZERO);
    for tile in been.into_iter().filter(|&tile| tile != here) {
        assert!(!untinted(&app, &screen, tile), "{tile:?}");
    }
}

// The pan (design §6.9).

/// A grass world 150 tiles across and 12 down, wider than the map view,
/// with a sprite near its west end, #1, one near its east end, #2, and #3
/// a little way east of #1. The view
/// starts on the middle of the map, from column 48, and centring on the
/// sprite in the west takes it to column -13, as far past the wall as it
/// goes.
fn wide() -> World {
    WorldBuilder::grass(150, 12)
        .starters(&[at(10, 5), at(140, 5), at(13, 5)])
        .build()
}

/// The map view's cells on a 100×30 screen.
fn map_view() -> Rect {
    ui::areas(SCREEN).tiles
}

/// Whether `shown` and `plain` draw the same map view on a 100×30 screen.
fn same_map_view(shown: &Buffer, plain: &Buffer) -> bool {
    same_map_view_on(shown, plain, SCREEN)
}

/// Whether `shown` and `plain` draw the same map view on a screen of
/// `size`.
fn same_map_view_on(shown: &Buffer, plain: &Buffer, size: Size) -> bool {
    ui::areas(size)
        .tiles
        .positions()
        .all(|cell| shown[cell] == plain[cell])
}

/// Scrolls `app` by `dx` tiles across, as the arrows do.
fn scroll_by(app: &mut App, world: &World, dx: i32) {
    apply(app, world, Action::Scroll { dx, dy: 0 });
}

#[test]
fn the_view_pans_to_a_sprite_far_away_quickly_at_first_and_then_slowing() {
    let world = wide();
    let (mut on, mut off) = twins(&world);
    assert_eq!(on.viewport().x, 48);
    both(&mut on, &mut off, &world, Action::SelectNext);
    // The viewport jumps at once; only the drawing pans.
    assert_eq!(on.viewport().x, -13);
    assert_eq!(off.viewport().x, -13);
    // The Cursor follows #1, so it's drawn on the map with its marks.
    both(&mut on, &mut off, &world, Action::Follow { at: None });
    assert_eq!(on.cursor(), at(10, 5));
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert!(!same_map_view(&shown, &plain), "it starts where it was");
    // Half the 0.3 s in, three quarters of the way, in whole tiles: as the
    // view is from column 2. Everything on the map moves with it, the
    // Cursor too.
    scroll_by(&mut off, &world, 15);
    let (shown, plain) = frames(&mut on, &mut off, &world, ms(150));
    assert!(same_map_view(&shown, &plain), "from column 2");
    // A tenth of a second later, nearly there: past the wall, with the
    // glass and the stars, from column -11.
    scroll_by(&mut off, &world, -13);
    let (shown, plain) = frames(&mut on, &mut off, &world, ms(100));
    assert!(same_map_view(&shown, &plain), "from column -11");
    // There at 0.3 s.
    scroll_by(&mut off, &world, -2);
    let (shown, plain) = frames(&mut on, &mut off, &world, ms(50));
    assert!(same_map_view(&shown, &plain), "from column -13");
    assert_eq!(on.viewport(), off.viewport());
}

/// Where `symbol` is drawn in the map view on `screen`, if it is.
fn drawn_at(screen: &Buffer, symbol: &str) -> Option<Position> {
    map_view()
        .positions()
        .find(|&cell| screen[cell].symbol() == symbol)
}

/// One frame of the game on a 100×30 screen: the app fitted to it, drawn,
/// its clock moved on 33 ms and a frame's ticks taken in, none of which
/// changed `world`; and the tiles the world was told the Cursor moved to.
fn game_frame(app: &mut App, world: &World) -> Vec<Pos> {
    app.fit(ui::areas(SCREEN));
    render(app, world);
    app.animate(ms(33));
    take_in(app, world);
    app.take_commands()
        .into_iter()
        .filter_map(|command| match command {
            terra_sim::Command::MoveCursor { tile } => Some(tile),
            _ => None,
        })
        .collect()
}

#[test]
fn the_cursor_goes_to_the_tile_under_a_still_pointer_only_once_the_pan_ends() {
    // Design §6.5: while sprites can see the Cursor, the world hears of
    // each tile it moves to, and the tiles the pan passes aren't any.
    let world = wide();
    let mut app = moving(&world);
    apply(&mut app, &world, Action::ToggleVisible);
    let pointer = pointer_on(&app, at(90, 6));
    apply(&mut app, &world, Action::Point(pointer));
    assert_eq!(app.cursor(), at(90, 6));
    app.take_commands();
    apply(&mut app, &world, Action::SelectNext);
    let mut told = app.take_commands();
    told.retain(|command| matches!(command, terra_sim::Command::MoveCursor { .. }));
    assert_eq!(told, Vec::new());
    // Nine frames take it most of the way, the Cursor where it was.
    for _ in 0..9 {
        assert_eq!(game_frame(&mut app, &world), Vec::new());
        assert_eq!(app.cursor(), at(90, 6));
    }
    // The pan ends in the tenth, and the Cursor goes under the pointer at
    // once, where column 90 was: 42 columns into a view from column -13.
    assert_eq!(game_frame(&mut app, &world), vec![at(29, 6)]);
    for _ in 0..3 {
        assert_eq!(game_frame(&mut app, &world), Vec::new());
    }
    assert_eq!(app.cursor(), at(29, 6));
}

#[test]
fn follow_stopped_while_it_pans_moves_the_cursor_only_once_the_pan_ends() {
    // Design §6.9: the world never hears of a tile the pan passes, nor of
    // the one under the pointer before it began.
    let world = wide();
    let mut app = moving(&world);
    apply(&mut app, &world, Action::ToggleVisible);
    apply(&mut app, &world, Action::SelectNext);
    for _ in 0..12 {
        game_frame(&mut app, &world);
    }
    let pointer = pointer_on(&app, at(20, 6));
    apply(&mut app, &world, Action::Point(pointer));
    apply(&mut app, &world, Action::Follow { at: None });
    assert_eq!(app.cursor(), at(10, 5), "on #1");
    // `Tab` to #2, in the east, and `F` two frames into the pan.
    apply(&mut app, &world, Action::SelectNext);
    app.take_commands();
    let mut told = Vec::new();
    for _ in 0..2 {
        told.extend(game_frame(&mut app, &world));
    }
    apply(&mut app, &world, Action::Follow { at: None });
    assert_eq!(app.followed(), None);
    assert_eq!(app.cursor(), at(10, 5), "where it was, while it pans");
    for _ in 0..12 {
        told.extend(game_frame(&mut app, &world));
    }
    // Once it's there, under the pointer: 33 columns into a view from
    // column 109.
    assert_eq!(told, vec![at(142, 6)]);
    assert_eq!(app.cursor(), at(142, 6));
}

/// Where the Cursor is when an aim is ended with `Esc` two frames into the
/// pan `Tab` starts, with motion on or off, and the tiles the world is told
/// it moved to from then on, until well after the pan.
fn aim_ended_while_it_pans(motion: bool) -> (Pos, Vec<Pos>) {
    let mut world = WorldBuilder::grass(150, 12)
        .objects(&[(at(12, 6), "ball")])
        .starters(&[at(10, 5), at(140, 5)])
        .build();
    let mut app = moving(&world);
    app.set_motion(motion);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    apply(&mut app, &world, Action::ToggleVisible);
    apply(&mut app, &world, Action::SelectNext);
    for _ in 0..12 {
        game_frame(&mut app, &world);
    }
    // The Cursor holds the ball, and the player aims it, pulling east.
    let on_the_ball = pointer_on(&app, at(12, 6));
    apply(&mut app, &world, Action::left_click(on_the_ball));
    common::tick(&mut app, &mut world);
    assert!(world.cursor().holds().is_some());
    apply(&mut app, &world, Action::right_click(on_the_ball));
    let pulled = pointer_on(&app, at(16, 6));
    apply(&mut app, &world, Action::Point(pulled));
    assert!(app.aiming());
    // `Tab` to #2, in the east, and `Esc` two frames into the pan.
    apply(&mut app, &world, Action::SelectNext);
    app.take_commands();
    let mut told = Vec::new();
    for _ in 0..2 {
        told.extend(game_frame(&mut app, &world));
    }
    apply(&mut app, &world, Action::Back);
    assert!(!app.aiming());
    let cursor = app.cursor();
    for _ in 0..12 {
        told.extend(game_frame(&mut app, &world));
    }
    (cursor, told)
}

#[test]
fn an_aim_ended_while_it_pans_moves_the_cursor_only_once_the_pan_ends() {
    // Design §6.9: the Cursor stays on what it aimed until the pan ends,
    // and then goes where it would have gone at once with motion off.
    let (on, told_on) = aim_ended_while_it_pans(true);
    let (off, told_off) = aim_ended_while_it_pans(false);
    assert_eq!(on, at(12, 6), "on the ball, where aiming began");
    assert_eq!(told_off, vec![off], "at once, with motion off");
    assert_eq!(told_on, told_off, "only that tile, with motion on");
}

#[test]
fn while_it_pans_the_pointer_and_a_click_go_by_what_s_drawn() {
    let world = wide();
    let mut app = moving(&world);
    apply(&mut app, &world, Action::SelectNext);
    let screen = frame(&mut app, &world, ms(150));
    // #3, drawn mid-pan where the view from column 2 has it.
    let drawn = drawn_at(&screen, "☺").expect("#3 is drawn");
    let pointer = Position::new(drawn.x + 1, drawn.y + 1);
    apply(&mut app, &world, Action::Point(pointer));
    assert_eq!(app.cursor(), at(13, 5));
    apply(&mut app, &world, Action::left_click(pointer));
    let third = sprite_on(&world, at(13, 5));
    assert_eq!(
        app.selection(),
        Some(terra_tui::app::Selection::Living(third))
    );
}

#[test]
fn a_pan_under_way_is_taken_on_from_where_it_s_drawn() {
    // `Tab` again, to #2 in the east, when it's half way to #1 in the west.
    let world = wide();
    let (mut on, mut off) = twins(&world);
    both(&mut on, &mut off, &world, Action::SelectNext);
    frames(&mut on, &mut off, &world, ms(150));
    both(&mut on, &mut off, &world, Action::SelectNext);
    assert_eq!(on.viewport().x, 109);
    // From column 2, where it had got to, not from where it was going.
    scroll_by(&mut off, &world, -107);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert!(same_map_view(&shown, &plain), "from column 2");
    scroll_by(&mut off, &world, 80);
    let (shown, plain) = frames(&mut on, &mut off, &world, ms(150));
    assert!(same_map_view(&shown, &plain), "from column 82");
}

#[test]
fn scrolling_the_tiles_squared_and_a_new_screen_size_jump_even_while_it_pans() {
    let world = wide();
    // Scrolled by hand, from a still view and from one panning.
    let (mut on, mut off) = twins(&world);
    both(&mut on, &mut off, &world, Action::Scroll { dx: 20, dy: 0 });
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert!(same_map_view(&shown, &plain), "scrolled");
    both(&mut on, &mut off, &world, Action::SelectNext);
    frames(&mut on, &mut off, &world, ms(100));
    both(&mut on, &mut off, &world, Action::Scroll { dx: 1, dy: 0 });
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert!(same_map_view(&shown, &plain), "scrolled while it panned");
    // Tiles squared while it pans.
    let (mut on, mut off) = twins(&world);
    both(&mut on, &mut off, &world, Action::SelectNext);
    frames(&mut on, &mut off, &world, ms(100));
    both(&mut on, &mut off, &world, Action::Zoom);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert!(same_map_view(&shown, &plain), "square tiles");
    // The screen a new size while it pans.
    let (mut on, mut off) = twins(&world);
    both(&mut on, &mut off, &world, Action::SelectNext);
    frames(&mut on, &mut off, &world, ms(100));
    let bigger = Size::new(120, 30);
    on.fit(ui::areas(bigger));
    off.fit(ui::areas(bigger));
    let (shown, plain) = frames_on(&mut on, &mut off, &world, Duration::ZERO, bigger);
    assert!(
        same_map_view_on(&shown, &plain, bigger),
        "a new screen size"
    );
}

#[test]
fn tracking_square_tiles_and_a_new_screen_size_still_jump() {
    // Design §6.9: Track centres the view on its sprite again for the new
    // view, and that jumps too, rather than panning as the next tick is
    // taken in. A new screen size, with the sprite in the middle of a wide
    // map:
    let world = WorldBuilder::grass(300, 12).starters(&[at(150, 5)]).build();
    let (mut on, mut off) = twins(&world);
    both(&mut on, &mut off, &world, Action::SelectNext);
    both(&mut on, &mut off, &world, Action::Track);
    frames(&mut on, &mut off, &world, ms(1000));
    let (was, bigger) = (on.viewport(), Size::new(160, 30));
    for app in [&mut on, &mut off] {
        app.fit(ui::areas(bigger));
        take_in(app, &world);
    }
    assert!(
        (on.viewport().x - was.x).abs() > 2,
        "{was:?} to {:?}",
        on.viewport()
    );
    let (shown, plain) = frames_on(&mut on, &mut off, &world, Duration::ZERO, bigger);
    assert!(
        same_map_view_on(&shown, &plain, bigger),
        "a new screen size"
    );
    // Square tiles, with #1 near the wall, where the margin past it halves.
    let world = wide();
    let (mut on, mut off) = twins(&world);
    both(&mut on, &mut off, &world, Action::SelectNext);
    frames(&mut on, &mut off, &world, ms(1000));
    both(&mut on, &mut off, &world, Action::Track);
    frames(&mut on, &mut off, &world, ms(1000));
    let was = on.viewport();
    both(&mut on, &mut off, &world, Action::Zoom);
    assert!(
        (on.viewport().x - was.x).abs() > 2,
        "{was:?} to {:?}",
        on.viewport()
    );
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert!(same_map_view(&shown, &plain), "square tiles");
    for app in [&mut on, &mut off] {
        take_in(app, &world);
    }
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert!(same_map_view(&shown, &plain), "and the next tick");
}

#[test]
fn at_4x_and_with_motion_turned_off_the_view_jumps() {
    let world = wide();
    let (mut on, mut off) = twins(&world);
    at_4x(&mut on);
    at_4x(&mut off);
    both(&mut on, &mut off, &world, Action::SelectNext);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert!(same_map_view(&shown, &plain), "at 4×");
    // Turned off while it pans.
    let (mut on, mut off) = twins(&world);
    both(&mut on, &mut off, &world, Action::SelectNext);
    frames(&mut on, &mut off, &world, ms(100));
    on.set_motion(false);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert!(same_map_view(&shown, &plain), "turned off");
}

#[test]
fn track_pans_to_its_sprite_and_then_jumps_a_tile_at_a_time_with_it() {
    // #1 walks a tile east; with the view on the middle of the map, it's
    // in sight, so `Tab` leaves the view where it is.
    let worlds =
        [at(60, 5), at(61, 5)].map(|tile| WorldBuilder::grass(150, 12).starters(&[tile]).build());
    let (mut on, mut off) = twins(&worlds[0]);
    both(&mut on, &mut off, &worlds[0], Action::SelectNext);
    assert_eq!(on.viewport().x, 48);
    both(&mut on, &mut off, &worlds[0], Action::Track);
    assert_eq!(on.viewport().x, 33);
    let (shown, plain) = frames(&mut on, &mut off, &worlds[0], Duration::ZERO);
    assert!(!same_map_view(&shown, &plain), "Track pans to it");
    let (shown, plain) = frames(&mut on, &mut off, &worlds[0], ms(300));
    assert!(same_map_view(&shown, &plain), "and gets there");
    on.record(&[], &worlds[1]);
    off.record(&[], &worlds[1]);
    assert_eq!(on.viewport().x, 34);
    let (shown, plain) = frames(&mut on, &mut off, &worlds[1], Duration::ZERO);
    assert!(same_map_view(&shown, &plain), "a tile, at once");
}

#[test]
fn the_sprite_list_pans_to_the_sprite_it_goes_to() {
    let world = wide();
    let (mut on, mut off) = twins(&world);
    both(&mut on, &mut off, &world, Action::SpriteList);
    both(&mut on, &mut off, &world, Action::Enter);
    assert_eq!(on.screen(), Screen::Normal);
    assert_eq!(on.viewport().x, -13);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert!(!same_map_view(&shown, &plain), "it pans");
    let (shown, plain) = frames(&mut on, &mut off, &world, ms(300));
    assert!(same_map_view(&shown, &plain), "and gets there");
}

#[test]
fn walking_the_family_tree_pans_to_the_one_walked_to() {
    // By tick 45, #1 has two young: #2, grown up, and #3, an egg lying
    // beside her (`family_tree.rs`).
    let [quick, eggs] = common::QUICK;
    let world = common::grow(
        &[quick, eggs, common::LONE],
        (120, 50),
        &[(at(60, 25), common::LONE_MOTHER)],
        45,
    );
    let mut app = moving(&world);
    let tiles = map_view();
    let middle = Position::new(tiles.x + tiles.width / 2, tiles.y + tiles.height / 2);
    let egg = world
        .egg(EntityId(3))
        .and_then(|egg| egg.pos())
        .expect("#3");
    apply(&mut app, &world, Action::SelectNext);
    apply(&mut app, &world, Action::Scroll { dx: -60, dy: -30 });
    apply(&mut app, &world, Action::FamilyTree);
    for action in [
        Action::Scroll { dx: 0, dy: 1 },
        Action::Scroll { dx: 1, dy: 0 },
        Action::Enter,
    ] {
        apply(&mut app, &world, action);
    }
    assert_eq!(
        app.selection(),
        Some(terra_tui::app::Selection::Living(EntityId(3)))
    );
    frame(&mut app, &world, Duration::ZERO);
    assert_ne!(app.cell_of(egg), Some(middle), "on its way");
    frame(&mut app, &world, ms(300));
    assert_eq!(app.cell_of(egg), Some(middle), "there");
}

#[test]
fn tab_to_a_sprite_the_pan_is_passing_pans_on_to_it() {
    // Out of sight is where the view is going, not where it's drawn: #2
    // is in the map view half way to #1, but not once it gets there.
    let world = WorldBuilder::grass(150, 12)
        .starters(&[at(10, 5), at(60, 5)])
        .build();
    let mut app = moving(&world);
    apply(&mut app, &world, Action::SelectNext);
    let screen = frame(&mut app, &world, ms(100));
    assert!(drawn_at(&screen, "☺").is_some(), "#2 is drawn");
    apply(&mut app, &world, Action::SelectNext);
    assert_eq!(app.viewport().x, 33);
}

// Mono (design §6.2, §6.9, #307).

#[test]
fn in_mono_motion_s_colours_don_t_play() {
    // Design §6.9: no death's flash or ghost, no tab sweeping in and no
    // pop-up fading, which would leave a closed one's characters a moment:
    // mono draws them as with motion off.
    let (mut app, ids, after) = before_and_after(&[at(3, 2)]);
    app.theme = mono();
    app.record(&deaths(&ids), &after);
    let screen = frame(&mut app, &after, Duration::ZERO);
    assert_eq!(screen[cell(&app, at(3, 2))].symbol(), ".", "no ghost");
    let world = one_on_a_field();
    let (mut on, mut off) = selected_in(&world, mono(), SCREEN);
    for action in [Action::NextTab, Action::Help, Action::Help, Action::Palette] {
        both(&mut on, &mut off, &world, action);
        for step in [0, 37, 38, 75, 200] {
            let (shown, plain) = frames(&mut on, &mut off, &world, ms(step));
            assert_eq!(shown, plain, "{action:?}, {step} ms on");
        }
    }
}

#[test]
fn in_mono_the_pan_and_the_bars_still_move() {
    // Design §6.9: they move no colour.
    let world = wide();
    let (mut on, mut off) = twins_in(&world, mono(), SCREEN);
    both(&mut on, &mut off, &world, Action::SelectNext);
    let (shown, plain) = frames(&mut on, &mut off, &world, Duration::ZERO);
    assert!(!same_map_view(&shown, &plain), "it starts where it was");
    let (before, after) = (hungry(&[(at(3, 2), 0.2)]), hungry(&[(at(3, 2), 0.8)]));
    let mut app = watching(&before);
    app.theme = mono();
    take_in(&mut app, &after);
    let start = hunger_bar(&frame(&mut app, &after, Duration::ZERO));
    assert_eq!(start, (6, ".80".to_string()), "eases from where it was");
    let end = hunger_bar(&frame(&mut app, &after, ms(400)));
    assert_eq!(end, (24, ".80".to_string()));
}
