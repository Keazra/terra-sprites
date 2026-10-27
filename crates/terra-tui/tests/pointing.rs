//! Tooltips, saying what the pointer is over, and a click on a line of
//! the event log (#304): each part of the screen notes what it is as it's
//! drawn.

mod common;
use common::*;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::{Color, Modifier};
use terra_sim::{DeathCause, EntityId, Event, EventKind, Pos, World};
use terra_tui::app::{App, CursorMode, Screen, Selection, Tab};
use terra_tui::input::Action;
use terra_tui::theme::Theme;

/// A screen tall enough for the whole of a starter sprite's Genome tab.
const TALL: Size = Size::new(100, 60);

/// An app on a starter world, a screen of `size`, with its one sprite, on
/// (2, 2), selected and `tab` open.
fn selected(size: Size, tab: Tab) -> (World, App, EntityId) {
    let world = field(&[], &[at(2, 2)]);
    let mut app = sized_app(&world, Theme::cp437(), size);
    let id = sprite_on(&world, at(2, 2));
    select(&mut app, &world, id);
    open(&mut app, &world, tab);
    (world, app, id)
}

/// A tooltip as it's drawn: the name on its top edge, and the words inside
/// it, its lines joined with spaces.
#[derive(Debug)]
struct Tooltip {
    name: String,
    words: String,
    area: Rect,
}

/// The tooltip beside the pointer at `pointer` on `screen`, if one is
/// drawn there: a box with a corner diagonally next to it, or, where
/// there's no room for one beside it, a box across its column just below
/// or above it (#305).
fn tooltip(screen: &Buffer, pointer: Position) -> Option<Tooltip> {
    let at = |x: i32, y: i32| {
        let (x, y) = (u16::try_from(x).ok()?, u16::try_from(y).ok()?);
        screen.cell((x, y)).map(|cell| cell.symbol().to_string())
    };
    let is = |x: i32, y: i32, symbol: &str| at(x, y).as_deref() == Some(symbol);
    let run = |from: i32, step: i32, stop: &dyn Fn(i32) -> bool| {
        let mut along = from;
        while !stop(along) {
            along += step;
            if !(0..400).contains(&along) {
                return None;
            }
        }
        Some(along)
    };
    let (x, y) = (i32::from(pointer.x), i32::from(pointer.y));
    // Its corners: the top ones below the pointer, or the bottom ones
    // above it, and to the left of it or to the right.
    let (left, top, right, bottom) = if is(x - 1, y + 1, "┐") {
        let left = run(x - 2, -1, &|along| is(along, y + 1, "┌"))?;
        (
            left,
            y + 1,
            x - 1,
            run(y + 2, 1, &|along| is(x - 1, along, "┘"))?,
        )
    } else if is(x + 1, y + 1, "┌") {
        let right = run(x + 2, 1, &|along| is(along, y + 1, "┐"))?;
        (
            x + 1,
            y + 1,
            right,
            run(y + 2, 1, &|along| is(x + 1, along, "└"))?,
        )
    } else if is(x - 1, y - 1, "┘") {
        let left = run(x - 2, -1, &|along| is(along, y - 1, "└"))?;
        (
            left,
            run(y - 2, -1, &|along| is(x - 1, along, "┐"))?,
            x - 1,
            y - 1,
        )
    } else if is(x + 1, y - 1, "└") {
        let right = run(x + 2, 1, &|along| is(along, y - 1, "┘"))?;
        (
            x + 1,
            run(y - 2, -1, &|along| is(x + 1, along, "┌"))?,
            right,
            y - 1,
        )
    } else if let (Some(left), Some(right)) = (
        run(x, -1, &|along| is(along, y + 1, "┌")),
        run(x, 1, &|along| is(along, y + 1, "┐")),
    ) {
        (
            left,
            y + 1,
            right,
            run(y + 2, 1, &|along| is(left, along, "└"))?,
        )
    } else if let (Some(left), Some(right)) = (
        run(x, -1, &|along| is(along, y - 1, "└")),
        run(x, 1, &|along| is(along, y - 1, "┘")),
    ) {
        (
            left,
            run(y - 2, -1, &|along| is(left, along, "┌"))?,
            right,
            y - 1,
        )
    } else {
        return None;
    };
    let text = |y: i32| -> String {
        (left + 1..right)
            .filter_map(|x| at(x, y))
            .collect::<String>()
    };
    let words: Vec<String> = (top + 1..bottom)
        .map(|y| text(y).trim().to_string())
        .collect();
    let area = Rect::new(
        left as u16,
        top as u16,
        (right - left + 1) as u16,
        (bottom - top + 1) as u16,
    );
    Some(Tooltip {
        name: text(top).trim_matches(|c| c == '─' || c == ' ').to_string(),
        words: words.join(" "),
        area,
    })
}

/// The tooltip on a `size` screen once the pointer is on cell `cell`.
fn pointing_at(app: &mut App, world: &World, size: Size, cell: Position) -> Option<Tooltip> {
    apply(app, world, Action::Point(cell));
    tooltip(&render_on(app, world, size), cell)
}

/// The tooltip on a `size` screen once the pointer is on `text`.
fn pointing_at_text(app: &mut App, world: &World, size: Size, text: &str) -> Tooltip {
    let cell = cell_of_text(app, world, size, text);
    pointing_at(app, world, size, cell).unwrap_or_else(|| panic!("a tooltip on {text:?}"))
}

#[test]
fn a_drives_name_says_what_raises_it_and_what_lowers_it() {
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    let said = pointing_at_text(&mut app, &world, SCREEN, "hunger");
    assert_eq!(said.name, "hunger");
    assert_eq!(
        said.words,
        "Rises as energy runs low, and eating brings it down at once"
    );
    // Its bar says nothing: only what's in the spot colour does.
    let name = cell_of_text(&app, &world, SCREEN, "tiredness");
    let bar = Position::new(name.x + 20, name.y);
    assert!(pointing_at(&mut app, &world, SCREEN, bar).is_none());
}

#[test]
fn a_chemical_says_what_it_is_on_its_line_and_in_the_hormones_grid() {
    let (world, mut app, _) = selected(SCREEN, Tab::Chem);
    let said = pointing_at_text(&mut app, &world, SCREEN, "energy");
    assert!(
        said.words
            .starts_with("What it runs on: food in its gut turns into it"),
        "{said:?}"
    );
    let said = pointing_at_text(&mut app, &world, SCREEN, "h3 ");
    assert_eq!(said.name, "h3");
    assert!(
        said.words
            .starts_with("A spare hormone its genes may use for anything"),
        "{said:?}"
    );
    // A later cell in the grid, to its name's last column, and not its
    // level after.
    let h7 = cell_of_text(&app, &world, SCREEN, "h7 ");
    let said = pointing_at(&mut app, &world, SCREEN, Position::new(h7.x + 1, h7.y));
    assert_eq!(said.map(|said| said.name).as_deref(), Some("h7"));
    let level = Position::new(h7.x + 5, h7.y);
    assert!(pointing_at(&mut app, &world, SCREEN, level).is_none());
    // The named hormones are lower down.
    let (world, mut app, _) = selected(TALL, Tab::Chem);
    let said = pointing_at_text(&mut app, &world, TALL, "receptivity");
    assert!(
        said.words.starts_with("Rises when she's danced for"),
        "{said:?}"
    );
}

#[test]
fn a_chemical_its_pack_says_nothing_of_says_what_sort_it_is() {
    let quiet: &[(&str, &str)] = &[(
        r#"about: "rises as energy runs low, and eating brings it down at once""#,
        r#"about: """#,
    )];
    let world = WorldBuilder::grass(10, 6)
        .data(pack_changing(&[("chemicals.ron", quiet)]))
        .starters(&[at(2, 2)])
        .build();
    let mut app = app_on(&world);
    select(&mut app, &world, sprite_on(&world, at(2, 2)));
    let said = pointing_at_text(&mut app, &world, SCREEN, "hunger");
    assert_eq!(said.words, "A drive, an urge it feels");
}

#[test]
fn words_too_long_for_one_line_wrap_inside_the_tooltip() {
    let long = format!(r#"about: "{}""#, "ever so hungry, ".repeat(10));
    let wordy: &[(&str, &str)] = &[(
        r#"about: "rises as energy runs low, and eating brings it down at once""#,
        &long,
    )];
    let world = WorldBuilder::grass(10, 6)
        .data(pack_changing(&[("chemicals.ron", wordy)]))
        .starters(&[at(2, 2)])
        .build();
    let mut app = app_on(&world);
    select(&mut app, &world, sprite_on(&world, at(2, 2)));
    let said = pointing_at_text(&mut app, &world, SCREEN, "hunger");
    let all = format!("Ever so hungry, {}", "ever so hungry, ".repeat(9));
    assert_eq!(said.words, all.trim_end(), "none of it cut");
    assert!(said.area.width <= 40 && said.area.height > 3, "{said:?}");
}

#[test]
fn a_gene_s_name_says_what_it_does_and_its_numbers_say_nothing() {
    let (world, mut app, _) = selected(TALL, Tab::Genome);
    let says = |app: &mut App, text: &str| pointing_at_text(app, &world, TALL, text);
    // Spawn variation moves the numbers, so only the words are checked.
    let said = says(&mut app, "hunger halves");
    assert_eq!(said.name, "hunger halves");
    assert!(
        said.words
            .starts_with("A half-life: hunger fades by itself, by half every "),
        "{said:?}"
    );
    let said = says(&mut app, "low energy → hunger");
    assert_eq!(said.name, "low energy → hunger");
    assert!(
        said.words
            .starts_with("An emitter: hunger rises while energy is below ."),
        "{said:?}"
    );
    let said = says(&mut app, "ate → hunger");
    assert!(
        said.words.starts_with("An emitter: ate lowers hunger"),
        "{said:?}"
    );
    let said = says(&mut app, "fullness → hunger");
    assert!(
        said.words
            .starts_with("An emitter: hunger falls while fullness is above ."),
        "{said:?}"
    );
    // The number after a name, a space past it, isn't the gene's.
    let name = cell_of_text(&app, &world, TALL, "low energy → hunger");
    let number = Position::new(
        name.x + "low energy → hunger ".chars().count() as u16,
        name.y,
    );
    assert!(pointing_at(&mut app, &world, TALL, number).is_none());
}

#[test]
fn each_trait_s_name_on_the_line_they_share_says_what_it_is() {
    let (world, mut app, _) = selected(TALL, Tab::Genome);
    let said = pointing_at_text(&mut app, &world, TALL, "speed");
    assert_eq!(said.name, "speed");
    assert!(
        said.words
            .starts_with("A trait: speed is how fast it walks"),
        "{said:?}"
    );
    let said = pointing_at_text(&mut app, &world, TALL, "sense");
    assert!(
        said.words
            .starts_with("A trait: sense is how far it senses"),
        "{said:?}"
    );
    // Between them, the dot is neither, and nor is a trait's number.
    let sense = cell_of_text(&app, &world, TALL, "sense");
    let dot = Position::new(sense.x - 2, sense.y);
    assert!(pointing_at(&mut app, &world, TALL, dot).is_none());
    let number = Position::new(sense.x + 6, sense.y);
    assert!(pointing_at(&mut app, &world, TALL, number).is_none());
}

#[test]
fn a_tab_says_nothing_and_keeps_its_look_but_takes_a_click() {
    // A tab's name says what it shows, so Fauxpaw had it say nothing
    // more on 2026-10-27, and keep the look it had.
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    let look = |screen: &Buffer, cell: Position| {
        let cell = &screen[cell];
        (cell.fg, cell.bg, cell.modifier)
    };
    let (body, brain) = (
        cell_of_text(&app, &world, SCREEN, "Body"),
        cell_of_text(&app, &world, SCREEN, "Brain"),
    );
    let screen = render(&app, &world);
    let (open, shut) = (look(&screen, body), look(&screen, brain));
    let spot = app.theme.text().spot.fg.expect("a colour");
    assert_ne!(open.0, spot, "the open tab isn't in the spot colour");
    assert_ne!(shut.0, spot, "nor is a tab that's shut");
    assert!(
        open.2.contains(Modifier::REVERSED),
        "the open one is selected"
    );
    for (cell, was) in [(brain, shut), (body, open)] {
        assert!(pointing_at(&mut app, &world, SCREEN, cell).is_none());
        let screen = render(&app, &world);
        assert_eq!(look(&screen, cell), was, "pointed at, it isn't lit");
        assert_eq!(look(&screen, body), open, "the open tab keeps its look");
    }
    apply(&mut app, &world, Action::left_click(brain));
    assert_eq!(app.tab(), Tab::Brain, "a click opens it");
}

#[test]
fn the_event_filters_say_what_they_do() {
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    let said = pointing_at_text(&mut app, &world, SCREEN, "selected major");
    assert_eq!(said.name, "Event filter");
    assert!(
        said.words.starts_with("Which events the log shows")
            && said.words.ends_with("click, or m, for the next"),
        "{said:?}"
    );
}

#[test]
fn a_tooltip_opens_below_the_pointer_and_to_its_left() {
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    let cell = cell_of_text(&app, &world, SCREEN, "hunger");
    let said = pointing_at(&mut app, &world, SCREEN, cell).expect("a tooltip");
    assert_eq!(said.area.right(), cell.x, "{said:?}");
    assert_eq!(said.area.y, cell.y + 1, "{said:?}");
}

#[test]
fn the_status_line_keeps_to_the_tile_and_back_on_the_map_there_is_no_tooltip() {
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    pointing_at_text(&mut app, &world, SCREEN, "hunger");
    let status = status_line(&app, &world);
    assert!(!status.contains("rises as energy"), "{status:?}");
    assert!(status.contains("SELECT"), "{status:?}");
    let tile = app.cell_of(at(5, 4)).expect("in view");
    let on_map = Position::new(tile.x + 1, tile.y + 1);
    assert!(pointing_at(&mut app, &world, SCREEN, on_map).is_none());
    let status = status_line(&app, &world);
    assert!(status.starts_with(" (5,4) grass │ SELECT"), "{status:?}");
}

#[test]
fn what_a_tooltip_can_explain_is_drawn_in_the_spot_colour() {
    let (world, app, _) = selected(SCREEN, Tab::Body);
    let spot = Theme::cp437().text().spot.fg.expect("a colour");
    let screen = render(&app, &world);
    let fg = |text: &str, past: u16| {
        let cell = cell_of_text(&app, &world, SCREEN, text);
        screen[(cell.x + past, cell.y)].fg
    };
    assert_eq!(fg("hunger", 0), spot, "a drive's name");
    assert_eq!(fg("hunger", 5), spot, "to its end");
    assert_ne!(fg("hunger", 6), spot, "not past it");
    assert_ne!(fg("Brain", 0), spot, "not a tab's name, which says nothing");
    assert_eq!(fg("selected major", 0), spot, "the event filters");
    let (world, app, _) = selected(TALL, Tab::Genome);
    let screen = render_on(&app, &world, TALL);
    let name = cell_of_text(&app, &world, TALL, "low energy → hunger");
    let width = "low energy → hunger".chars().count() as u16;
    for x in name.x..name.x + width {
        assert_eq!(screen[(x, name.y)].fg, spot, "a gene's name, at {x}");
    }
    assert_ne!(
        screen[(name.x + width + 1, name.y)].fg,
        spot,
        "not its number"
    );
}

#[test]
fn the_spot_the_pointer_is_on_is_lit_in_reverse_video_a_gene_s_name_alone() {
    let (world, mut app, _) = selected(TALL, Tab::Genome);
    let name = cell_of_text(&app, &world, TALL, "low energy → hunger");
    apply(&mut app, &world, Action::Point(name));
    let screen = render_on(&app, &world, TALL);
    let lit = |x: u16| screen[(x, name.y)].modifier.contains(Modifier::REVERSED);
    let width = "low energy → hunger".chars().count() as u16;
    assert!((name.x..name.x + width).all(lit), "the name");
    assert!(!lit(name.x + width + 1), "not its number");
    assert!(!lit(name.x - 1), "nor the space before it");
}

#[test]
fn a_log_line_s_name_lights_its_sprite_up_on_the_map_and_says_nothing() {
    let world = field(&[], &[at(2, 2), at(7, 4)]);
    let mut app = app_on(&world);
    let (first, second) = (sprite_on(&world, at(2, 2)), sprite_on(&world, at(7, 4)));
    let grew_up = Event {
        tick: 1,
        kind: EventKind::GrewUp { id: second },
    };
    app.record(&[grew_up], &world);
    let label = format!("Sprite #{}", second.0);
    let lit = |app: &App, tile| {
        let cell = app.cell_of(tile).expect("in view");
        render(app, &world)[cell]
            .modifier
            .contains(Modifier::REVERSED)
    };
    assert!(!lit(&app, at(7, 4)));
    // Its name says whom the line is about, so it opens no tooltip
    // (Fauxpaw, 2026-10-27), but it lights up, and so does its sprite.
    let line = cell_of_text(&app, &world, SCREEN, &format!("{label} grew up"));
    assert!(pointing_at(&mut app, &world, SCREEN, line).is_none());
    let screen = render(&app, &world);
    assert!(
        screen[line].modifier.contains(Modifier::REVERSED),
        "its name"
    );
    assert!(lit(&app, at(7, 4)), "the sprite it's about");
    assert!(!lit(&app, at(2, 2)), "not another");
    // What happened, after the name, lights nothing.
    let rest = cell_of_text(&app, &world, SCREEN, "grew up");
    assert!(pointing_at(&mut app, &world, SCREEN, rest).is_none());
    assert!(!lit(&app, at(7, 4)), "not from the rest of the line");
    // A click on the name selects it.
    apply(&mut app, &world, Action::left_click(line));
    assert_eq!(app.selection(), Some(Selection::Living(second)));
    assert!(pointing_at(&mut app, &world, SCREEN, line).is_none());
    assert_ne!(first, second);
}

#[test]
fn a_line_about_a_sprite_no_longer_living_is_plain_and_takes_no_click() {
    // Water runs out in two ticks, and then dehydration kills in two more.
    let pack = pack_changing(&[(
        "physiology.ron",
        &[
            ("hydration_loss: 0.00033", "hydration_loss: 0.5"),
            ("dehydration: 0.0011", "dehydration: 0.5"),
        ],
    )]);
    let mut world = WorldBuilder::grass(10, 5)
        .starters(&[at(1, 1)])
        .data(pack)
        .seed(7)
        .build();
    let dying = sprite_on(&world, at(1, 1));
    let mut app = app_on(&world);
    while world.dead(dying).is_none() {
        assert!(world.tick() < 100, "it dies of thirst");
        let events = world.step();
        app.record(&events, &world);
    }
    // And one the world has forgotten.
    let gone = EntityId(9_999);
    app.record(
        &[died(world.tick(), gone, DeathCause::Starvation, 40)],
        &world,
    );
    let spot = Theme::cp437().text().spot.fg.expect("a colour");
    for name in [format!("Sprite #{} died", dying.0), "Sprite #9999".into()] {
        let line = cell_of_text(&app, &world, SCREEN, &name);
        let screen = render(&app, &world);
        assert_ne!(screen[line].fg, spot, "{name}: not in the spot colour");
        assert!(
            pointing_at(&mut app, &world, SCREEN, line).is_none(),
            "{name}"
        );
        let screen = render(&app, &world);
        let lit = screen[line].modifier.contains(Modifier::REVERSED);
        assert!(!lit, "{name}: not lit");
        let cursor = app.cursor();
        apply(&mut app, &world, Action::left_click(line));
        assert_eq!(app.selection(), None, "{name}");
        assert_eq!(app.cursor(), cursor, "{name}: nor does it reach the map");
    }
}

#[test]
fn a_click_on_a_log_line_moves_the_view_only_if_its_sprite_is_out_of_view() {
    let (near, far) = (at(60, 30), at(2, 2));
    let world = WorldBuilder::grass(120, 60).starters(&[near, far]).build();
    let mut app = app_on(&world);
    let ids = [sprite_on(&world, near), sprite_on(&world, far)];
    assert!(app.cell_of(near).is_some() && app.cell_of(far).is_none());
    let grew_up = |id| Event {
        tick: 1,
        kind: EventKind::GrewUp { id },
    };
    app.record(&[grew_up(ids[0]), grew_up(ids[1])], &world);
    let line = |app: &App, id: EntityId| {
        cell_of_text(app, &world, SCREEN, &format!("Sprite #{} grew up", id.0))
    };
    let view = app.viewport();
    let near_line = line(&app, ids[0]);
    apply(&mut app, &world, Action::left_click(near_line));
    assert_eq!(app.selection(), Some(Selection::Living(ids[0])));
    assert_eq!(app.viewport(), view, "in view, the view stays");
    // Off the log, so the first line's tooltip is gone from over the next.
    apply(&mut app, &world, Action::Point(Position::new(0, 0)));
    let far_line = line(&app, ids[1]);
    apply(&mut app, &world, Action::left_click(far_line));
    assert_eq!(app.selection(), Some(Selection::Living(ids[1])));
    assert!(
        app.cell_of(far).is_some(),
        "out of view, the view goes to it"
    );
}

#[test]
fn a_click_on_a_log_line_leaves_follow_as_it_is() {
    let world = field(&[], &[at(2, 2), at(7, 4)]);
    let mut app = app_on(&world);
    let (followed, other) = (sprite_on(&world, at(2, 2)), sprite_on(&world, at(7, 4)));
    let cell = app.cell_of(at(2, 2)).expect("in view");
    apply(
        &mut app,
        &world,
        Action::middle_click(Position::new(cell.x + 1, cell.y + 1)),
    );
    assert_eq!(app.followed(), Some(followed));
    let grew_up = Event {
        tick: 1,
        kind: EventKind::GrewUp { id: other },
    };
    app.record(&[grew_up], &world);
    let line = cell_of_text(
        &app,
        &world,
        SCREEN,
        &format!("Sprite #{} grew up", other.0),
    );
    apply(&mut app, &world, Action::left_click(line));
    assert_eq!(app.selection(), Some(Selection::Living(other)));
    assert_eq!(app.followed(), Some(followed));
}

/// A world with a starter sprite on a tile the debug overlay's box is drawn
/// over, the map view's top right, and an app on it, drawn, with the
/// overlay `shown` or not: the world, the app, the tile and the sprite.
fn under_the_overlay(shown: bool) -> (World, App, Pos, EntityId) {
    let empty = WorldBuilder::grass(60, 30).build();
    let mut app = app_on(&empty);
    apply(&mut app, &empty, Action::DebugOverlay);
    let corner = cell_of_text(&app, &empty, SCREEN, " Debug ");
    let tile = app
        .tile_at(Position::new(corner.x + 3, corner.y + 2))
        .expect("a tile beneath the box");
    let world = WorldBuilder::grass(60, 30).starters(&[tile]).build();
    let id = sprite_on(&world, tile);
    let mut app = app_on(&world);
    if shown {
        apply(&mut app, &world, Action::DebugOverlay);
    }
    render(&app, &world);
    (world, app, tile, id)
}

#[test]
fn the_pointer_on_the_debug_overlay_never_lands_on_the_map_beneath() {
    // With the overlay hidden, the pointer on the sprite's tile moves the
    // Cursor there, a click selects it, the wheel changes the mode and a
    // middle click follows it. With its box over the tile, none of them
    // does (design §6.1).
    for shown in [false, true] {
        let (_, app, tile, _) = under_the_overlay(shown);
        let on_it = pointer_on(&app, tile);
        let actions = [
            ("the pointer", Action::Point(on_it)),
            ("a click", Action::left_click(on_it)),
            (
                "the wheel",
                Action::Wheel {
                    at: on_it,
                    notches: 1,
                },
            ),
            ("a middle click", Action::middle_click(on_it)),
        ];
        for (what, action) in actions {
            let (world, mut app, ..) = under_the_overlay(shown);
            let state = |app: &App| (app.cursor(), app.mode(), app.followed(), app.selection());
            let before = state(&app);
            apply(&mut app, &world, action);
            assert_eq!(
                state(&app) == before,
                shown,
                "{what}, overlay shown: {shown}"
            );
        }
    }
}

#[test]
fn a_sprite_under_a_pop_up_is_not_lit_from_its_line_in_the_log() {
    for shown in [false, true] {
        let (world, mut app, tile, id) = under_the_overlay(shown);
        let grew_up = Event {
            tick: 1,
            kind: EventKind::GrewUp { id },
        };
        app.record(&[grew_up], &world);
        let name = cell_of_text(&app, &world, SCREEN, &format!("Sprite #{} grew up", id.0));
        apply(&mut app, &world, Action::Point(name));
        let cell = app.cell_of(tile).expect("in view");
        let lit = render(&app, &world)[cell]
            .modifier
            .contains(Modifier::REVERSED);
        assert_eq!(lit, !shown, "overlay shown: {shown}");
    }
}

#[test]
fn a_pop_up_takes_the_pointer_before_what_is_under_it() {
    let opens = [
        Action::Help,
        Action::SpriteList,
        Action::Population,
        Action::FamilyTree,
    ];
    for open in opens {
        let (world, mut app, _) = selected(SCREEN, Tab::Body);
        let name = cell_of_text(&app, &world, SCREEN, "hunger");
        apply(&mut app, &world, open);
        let said = pointing_at(&mut app, &world, SCREEN, name);
        assert!(said.is_none(), "{open:?}: {said:?}");
    }
}

#[test]
fn off_the_world_s_screen_no_tooltip_opens_and_nothing_is_lit() {
    // Only on the world's own screen (design §6.1): the palette, the Place
    // menu, the settings screen, naming and the quit prompt take keys, so a tooltip's keys
    // wouldn't do what it says. The log's filters have room for one off
    // each of their boxes.
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    let filters = cell_of_text(&app, &world, SCREEN, "selected major");
    let lit = |app: &App| {
        render(app, &world)[filters]
            .modifier
            .contains(Modifier::REVERSED)
    };
    let unlit = lit(&app);
    assert!(pointing_at(&mut app, &world, SCREEN, filters).is_some());
    assert_ne!(lit(&app), unlit, "on the world's screen, it's lit");
    let place_menu = [Action::Mode(CursorMode::Grab); 2];
    let opens: [(&str, &[Action]); 5] = [
        ("the palette", &[Action::Palette]),
        ("naming", &[Action::Rename]),
        ("the quit prompt", &[Action::Back]),
        ("the Place menu", &place_menu),
        ("the settings screen", &[Action::OpenSettings]),
    ];
    for (what, keys) in opens {
        let (world, mut app, _) = selected(SCREEN, Tab::Body);
        for &key in keys {
            apply(&mut app, &world, key);
        }
        assert_ne!(app.screen(), Screen::Normal, "{what}");
        let said = pointing_at(&mut app, &world, SCREEN, filters);
        assert!(said.is_none(), "{what}: {said:?}");
        let screen = render(&app, &world);
        let lit = screen[filters].modifier.contains(Modifier::REVERSED);
        assert_eq!(lit, unlit, "{what}: not lit");
    }
}

#[test]
fn on_the_quit_prompt_a_log_line_lights_no_sprite_up() {
    let world = field(&[], &[at(2, 2)]);
    let id = sprite_on(&world, at(2, 2));
    let grew_up = Event {
        tick: 1,
        kind: EventKind::GrewUp { id },
    };
    for quitting in [false, true] {
        let mut app = app_on(&world);
        app.record(std::slice::from_ref(&grew_up), &world);
        let name = cell_of_text(&app, &world, SCREEN, &format!("Sprite #{} grew up", id.0));
        if quitting {
            apply(&mut app, &world, Action::Back);
            assert_eq!(app.screen(), Screen::QuitPrompt);
        }
        apply(&mut app, &world, Action::Point(name));
        let cell = app.cell_of(at(2, 2)).expect("in view");
        let lit = render(&app, &world)[cell]
            .modifier
            .contains(Modifier::REVERSED);
        assert_eq!(lit, !quitting, "quitting: {quitting}");
    }
}

/// Where the debug overlay's box is drawn on a `SCREEN`, from its top
/// corners down to its bottom right one.
fn overlay_box(app: &App, world: &World) -> Rect {
    let screen = render(app, world);
    let title = cell_of_text(app, world, SCREEN, " Debug ");
    let is = |x: u16, y: u16, symbol: &str| screen[(x, y)].symbol() == symbol;
    let left = (0..title.x)
        .rev()
        .find(|&x| is(x, title.y, "┌"))
        .expect("┌");
    let right = (title.x..SCREEN.width)
        .find(|&x| is(x, title.y, "┐"))
        .expect("┐");
    let bottom = (title.y..SCREEN.height)
        .find(|&y| is(right, y, "┘"))
        .expect("┘");
    Rect::new(left, title.y, right - left + 1, bottom - title.y + 1)
}

#[test]
fn beside_the_debug_overlay_every_tile_can_still_be_pointed_at() {
    // The pointer points at the tile up and to the left of it, so on the
    // box's left edge it points at the tile beside the box, and just below
    // the box at one under its bottom edge, which is nothing (design §6.1).
    let (world, mut app, ..) = under_the_overlay(true);
    let overlay = overlay_box(&app, &world);
    let beside = app
        .tile_at(Position::new(overlay.x - 1, overlay.y + 1))
        .expect("a tile beside the box");
    apply(
        &mut app,
        &world,
        Action::Point(Position::new(overlay.x, overlay.y + 2)),
    );
    assert_eq!(app.cursor(), beside, "on its left edge");
    let below = Position::new(overlay.x + 4, overlay.bottom());
    apply(&mut app, &world, Action::Point(below));
    assert_eq!(app.cursor(), beside, "just below it, the Cursor stays");
}

#[test]
fn a_tooltip_keeps_off_the_debug_overlay() {
    // The Chem tab's first names are beside the box, so a tooltip below
    // them and to their left would cover it.
    let (world, mut app, _) = selected(SCREEN, Tab::Chem);
    apply(&mut app, &world, Action::DebugOverlay);
    let overlay = overlay_box(&app, &world);
    let drawn = |screen: &Buffer| -> Vec<String> {
        overlay
            .positions()
            .map(|cell| screen[cell].symbol().to_string())
            .collect()
    };
    let shown = drawn(&render(&app, &world));
    // Where it would cover the box, it goes elsewhere, or where there's
    // nowhere else, isn't drawn.
    let mut drawn_somewhere = 0;
    let names = ["energy", "hydration", "hunger", "selected major"];
    let cells = names.map(|text| (text, cell_of_text(&app, &world, SCREEN, text)));
    for (text, cell) in cells {
        if let Some(said) = pointing_at(&mut app, &world, SCREEN, cell) {
            assert!(!said.area.intersects(overlay), "{text}: {said:?}");
            drawn_somewhere += 1;
        }
        assert_eq!(drawn(&render(&app, &world)), shown, "{text}");
    }
    assert!(drawn_somewhere >= 2, "{drawn_somewhere}");
}

#[test]
fn a_key_puts_the_tooltip_away_until_the_pointer_moves() {
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    let name = cell_of_text(&app, &world, SCREEN, "hunger");
    assert!(pointing_at(&mut app, &world, SCREEN, name).is_some());
    apply(&mut app, &world, Action::TogglePause);
    let screen = render(&app, &world);
    assert!(tooltip(&screen, name).is_none(), "put away");
    let lit = screen[name].modifier.contains(Modifier::REVERSED);
    assert!(!lit, "and its spot no longer lit");
    assert!(
        pointing_at(&mut app, &world, SCREEN, name).is_some(),
        "back once the pointer moves"
    );
}

#[test]
fn closing_the_graph_or_the_tree_leaves_no_tooltip_where_the_pointer_was() {
    for open in [Action::Population, Action::FamilyTree] {
        let (world, mut app, _) = selected(SCREEN, Tab::Body);
        let name = cell_of_text(&app, &world, SCREEN, "hunger");
        assert!(pointing_at(&mut app, &world, SCREEN, name).is_some());
        apply(&mut app, &world, open);
        render(&app, &world);
        apply(&mut app, &world, Action::Point(Position::new(5, 5)));
        apply(&mut app, &world, Action::Back);
        let screen = render(&app, &world);
        assert!(tooltip(&screen, name).is_none(), "{open:?}");
        assert!(
            !screen[name].modifier.contains(Modifier::REVERSED),
            "{open:?}: not lit"
        );
    }
}

#[test]
fn a_log_line_that_does_not_name_its_sprite_is_all_of_it_its_spot() {
    // A sprite named Mira is labelled `Mira #N`, but the line saying so
    // names it as it was.
    let world = field(&[], &[at(2, 2)]);
    let mut app = app_on(&world);
    let id = sprite_on(&world, at(2, 2));
    let renamed = Event {
        tick: 1,
        kind: EventKind::Renamed {
            id,
            name: "Mira".into(),
        },
    };
    app.record(&[renamed], &world);
    let line = format!("You named Sprite #{} Mira", id.0);
    let start = cell_of_text(&app, &world, SCREEN, &line);
    let spot = Theme::cp437().text().spot.fg.expect("a colour");
    let screen = render(&app, &world);
    let width = line.chars().count() as u16;
    for x in start.x..start.x + width {
        assert_eq!(screen[(x, start.y)].fg, spot, "at {x}");
    }
    // Its end lights up its sprite as its start does.
    let end = Position::new(start.x + width - 1, start.y);
    apply(&mut app, &world, Action::Point(end));
    let tile = app.cell_of(at(2, 2)).expect("in view");
    let screen = render(&app, &world);
    assert!(screen[tile].modifier.contains(Modifier::REVERSED));
}

/// Genes the starter genome has none of: a gene with no effect, a starting
/// level, an instinct with a long name and an unknown gene.
const ODD_GENES: &str = r#"(format: 1, genes: [
    Emitter(locus: Locus("ate"), mode: Level, gain: 0.3, chem: "food"),
    InitialConcentration(chem: "boredom", value: 0.2),
    Instinct(inputs: [("loneliness", false), ("crowdedness", true), ("target_adjacent", true)], verb: Eat, weight: 0.5),
    Gene(type: 900, version: 1, payload: "c0ffee"),
])"#;

/// An app in `theme` on a tall screen, with a sprite of `ODD_GENES`
/// selected and its Genome tab open.
fn odd_genes(theme: Theme) -> (World, App) {
    let world = WorldBuilder::grass(10, 6)
        .genes(&[(at(2, 2), ODD_GENES)])
        .build();
    let mut app = sized_app(&world, theme, TALL);
    select(&mut app, &world, sprite_on(&world, at(2, 2)));
    open(&mut app, &world, Tab::Genome);
    (world, app)
}

#[test]
fn a_starting_level_and_an_unknown_gene_say_what_they_are_too() {
    let (world, mut app) = odd_genes(Theme::cp437());
    let spot = app.theme.text().spot.fg.expect("a colour");
    let screen = render_on(&app, &world, TALL);
    let name = "boredom starts at";
    let start = cell_of_text(&app, &world, TALL, name);
    let width = name.chars().count() as u16;
    for x in start.x..start.x + width {
        assert_eq!(screen[(x, start.y)].fg, spot, "its name, at {x}");
    }
    let number = (start.x + width + 1, start.y);
    assert_ne!(screen[number].fg, spot, "not its number");
    let said = pointing_at_text(&mut app, &world, TALL, name);
    assert_eq!(said.name, "boredom's start");
    assert_eq!(
        said.words,
        "A starting level: boredom starts at .2 when it hatches"
    );
    // An unknown gene is all name, the commas between its parts too.
    apply(&mut app, &world, Action::Point(Position::new(0, 0)));
    let line = "type 900, version 1, 3 bytes";
    let start = cell_of_text(&app, &world, TALL, line);
    for past in [0, 8, 19, line.len() - 1] {
        let cell = Position::new(start.x + past as u16, start.y);
        let said = pointing_at(&mut app, &world, TALL, cell).expect("a tooltip");
        assert_eq!(said.words, "A gene this version of the game can't read");
        apply(&mut app, &world, Action::Point(Position::new(0, 0)));
    }
}

#[test]
fn a_gene_with_no_effect_keeps_its_muted_look_whatever_the_theme_s_muted_is() {
    let ascii = include_str!("../../../themes/ascii.ron");
    let shipped = "muted:    (fg: dark_gray)";
    assert!(ascii.contains(shipped));
    for muted in ["(fg: dark_gray)", "(bold: true)", "(reversed: true)"] {
        let edited = ascii.replacen(shipped, &format!("muted:    {muted}"), 1);
        let theme = Theme::from_ron(&edited, &pack()).expect("it loads");
        let (world, mut app) = odd_genes(theme);
        let look = app.theme.text();
        let name = cell_of_text(&app, &world, TALL, "ate → food");
        let screen = render_on(&app, &world, TALL);
        let cell = &screen[name];
        assert_eq!(cell.fg, look.muted.fg.unwrap_or(Color::Reset), "{muted}");
        assert!(cell.modifier.contains(look.muted.add_modifier), "{muted}");
        // Pointed at, it's lit like any other.
        apply(&mut app, &world, Action::Point(name));
        let screen = render_on(&app, &world, TALL);
        assert_eq!(screen[name].fg, look.spot.fg.expect("a colour"), "{muted}");
    }
}

#[test]
fn a_name_too_long_for_its_tooltip_ends_with_dots() {
    let (world, mut app) = odd_genes(Theme::cp437());
    let said = pointing_at_text(&mut app, &world, TALL, "loneliness");
    assert!(said.name.starts_with("loneliness & "), "{said:?}");
    assert!(said.name.ends_with("..."), "{said:?}");
    let room = usize::from(said.area.width) - 6;
    assert!(said.name.chars().count() <= room, "{said:?}");
}

/// That sprite `id` grew up, at tick 1: a line in the event log about it.
fn grew_up(id: EntityId) -> Event {
    Event {
        tick: 1,
        kind: EventKind::GrewUp { id },
    }
}

/// A screen with room for three columns of the inspector's lines while it
/// fills the screen (#305).
const WIDE: Size = Size::new(140, 40);

/// A screen under 100 columns, where the inspector pops over the map view,
/// with room for the event log (#305).
const NARROW: Size = Size::new(80, 30);

#[test]
fn filling_the_screen_a_gene_in_a_later_column_says_what_it_is_and_a_tab_takes_a_click() {
    let (world, mut app, _) = selected(WIDE, Tab::Genome);
    apply(&mut app, &world, Action::Inspector);
    assert!(app.full_screen());
    let spot = Theme::cp437().text().spot.fg.expect("a colour");
    let screen = render_on(&app, &world, WIDE);
    // The second column of lines is 47 cells right of the first.
    let name = (2..WIDE.height - 1)
        .flat_map(|y| (48..93).map(move |x| Position::new(x, y)))
        .find(|&cell| screen[cell].fg == spot)
        .expect("a gene's name in the second column");
    let drawn: String = (name.x..93)
        .map(|x| screen[(x, name.y)].symbol().to_string())
        .collect();
    let first_word = drawn.split_whitespace().next().expect("a word");
    let said = pointing_at(&mut app, &world, WIDE, name).expect("a tooltip");
    assert!(said.name.contains(first_word), "{said:?} at {drawn:?}");
    let brain = cell_of_text(&app, &world, WIDE, "Brain");
    apply(&mut app, &world, Action::left_click(brain));
    assert_eq!(app.tab(), Tab::Brain);
}

#[test]
fn filling_the_screen_the_inspector_leaves_nothing_beneath_it_to_point_at() {
    let world = field(&[], &[at(2, 2)]);
    let mut app = app_on(&world);
    let id = sprite_on(&world, at(2, 2));
    app.record(&[grew_up(id)], &world);
    let line = cell_of_text(&app, &world, SCREEN, &format!("Sprite #{} grew up", id.0));
    apply(&mut app, &world, Action::Inspector);
    assert!(app.full_screen());
    // The log's line beneath lights nothing up.
    let reversed = |app: &App| {
        render(app, &world)[line]
            .modifier
            .contains(Modifier::REVERSED)
    };
    assert!(!reversed(&app));
    assert!(pointing_at(&mut app, &world, SCREEN, line).is_none());
    assert!(!reversed(&app), "the log's line beneath lights nothing up");
    apply(&mut app, &world, Action::left_click(line));
    assert_eq!(app.selection(), None, "and takes no click");
}

#[test]
fn zoomed_a_log_line_lights_up_all_of_its_sprite_s_tile() {
    let world = field(&[], &[at(2, 2), at(7, 4)]);
    let mut app = app_on(&world);
    apply(&mut app, &world, Action::Zoom);
    let id = sprite_on(&world, at(7, 4));
    app.record(&[grew_up(id)], &world);
    let name = cell_of_text(&app, &world, SCREEN, &format!("Sprite #{} grew up", id.0));
    apply(&mut app, &world, Action::Point(name));
    let cell = app.cell_of(at(7, 4)).expect("in view");
    let screen = render(&app, &world);
    for x in [cell.x, cell.x + 1] {
        let lit = screen[(x, cell.y)].modifier.contains(Modifier::REVERSED);
        assert!(lit, "its cell at {x}");
    }
}

/// A world with a starter sprite on a tile the inspector is drawn over
/// while it's open over the map view, and an app on it on a `NARROW`
/// screen, the inspector open and a line in the log about the sprite: the
/// world, the app, the tile and where the sprite's name is in the line.
fn under_the_inspector() -> (World, App, Pos, Position) {
    let empty = WorldBuilder::grass(120, 60).build();
    let mut app = sized_app(&empty, Theme::cp437(), NARROW);
    apply(&mut app, &empty, Action::Inspector);
    let inspector = app.inspector_area().expect("open over the map");
    let tile = app
        .tile_at(Position::new(inspector.x + 10, inspector.y + 5))
        .expect("a tile beneath it");
    let world = WorldBuilder::grass(120, 60).starters(&[tile]).build();
    let id = sprite_on(&world, tile);
    let mut app = sized_app(&world, Theme::cp437(), NARROW);
    app.record(&[grew_up(id)], &world);
    apply(&mut app, &world, Action::Inspector);
    let name = cell_of_text(&app, &world, NARROW, &format!("Sprite #{} grew up", id.0));
    (world, app, tile, name)
}

#[test]
fn a_sprite_under_the_inspector_open_over_the_map_is_not_lit_from_its_line_in_the_log() {
    let (world, mut app, tile, name) = under_the_inspector();
    apply(&mut app, &world, Action::Point(name));
    let cell = app.cell_of(tile).expect("in view");
    assert!(app.inspector_area().expect("open").contains(cell));
    let lit = |app: &App| {
        render_on(app, &world, NARROW)[cell]
            .modifier
            .contains(Modifier::REVERSED)
    };
    assert!(!lit(&app), "the inspector over it isn't lit");
    // Shut, the sprite is lit as ever.
    apply(&mut app, &world, Action::Back);
    assert!(!app.inspector_open());
    apply(&mut app, &world, Action::Point(name));
    assert!(lit(&app), "shut, it's lit");
}

#[test]
fn a_click_on_a_log_line_about_a_sprite_under_the_inspector_brings_it_out_beside_it() {
    let (world, mut app, tile, name) = under_the_inspector();
    let view = app.viewport();
    apply(&mut app, &world, Action::left_click(name));
    let id = sprite_on(&world, tile);
    assert_eq!(app.selection(), Some(Selection::Living(id)));
    assert_ne!(app.viewport(), view, "the view moves");
    let cell = app.cell_of(tile).expect("in view");
    let inspector = app.inspector_area().expect("still open");
    assert!(!inspector.contains(cell), "beside the inspector");
}

#[test]
fn the_wheel_on_the_debug_overlay_over_the_inspector_scrolls_no_tab() {
    let (world, mut app, _) = selected(NARROW, Tab::Genome);
    apply(&mut app, &world, Action::Inspector);
    assert!(app.inspector_open());
    apply(&mut app, &world, Action::DebugOverlay);
    let corner = cell_of_text(&app, &world, NARROW, " Debug ");
    let on_the_box = Position::new(corner.x + 2, corner.y + 2);
    let inspector = app.inspector_area().expect("open");
    assert!(inspector.contains(on_the_box), "the box is over it");
    let wheel = |at| Action::Wheel { at, notches: 1 };
    apply(&mut app, &world, wheel(on_the_box));
    assert_eq!(app.tab_scroll(), 0, "the box takes the wheel");
    // Below the box, on the inspector, the wheel scrolls the tab.
    let below = Position::new(on_the_box.x, inspector.bottom() - 1);
    apply(&mut app, &world, wheel(below));
    assert!(app.tab_scroll() > 0, "below it, the tab scrolls");
}

/// Each cell of `screen` drawn in the spot colour (design §6.2).
fn spot_cells(screen: &Buffer) -> Vec<Position> {
    let spot = Theme::cp437().text().spot.fg.expect("a colour");
    screen
        .area
        .positions()
        .filter(|&cell| screen[cell].fg == spot)
        .collect()
}

/// Says where a cell drawn in the spot colour on a `size` screen opens no
/// tooltip when the pointer is on it, a cell at a time.
fn without_a_tooltip(app: &mut App, world: &World, size: Size) -> Vec<Position> {
    let cells = spot_cells(&render_on(app, world, size));
    assert!(!cells.is_empty(), "something to point at at {size:?}");
    cells
        .into_iter()
        .filter(|&cell| {
            let said = pointing_at(app, world, size, cell);
            !said.is_some_and(|said| !said.name.is_empty() && !said.area.contains(cell))
        })
        .collect()
}

#[test]
fn on_a_screen_short_of_room_every_spot_still_says_what_it_is_or_lights_up_its_sprite() {
    // Under 100 columns, with the inspector open over the map, and short
    // of rows, where a whole tooltip doesn't fit beside much of what can
    // be pointed at (#305).
    for (size, open) in [
        (Size::new(60, 30), true),
        (Size::new(80, 24), true),
        (Size::new(100, 10), false),
        (Size::new(100, 12), false),
    ] {
        for tab in [Tab::Body, Tab::Chem, Tab::Genome] {
            let (world, mut app, _) = selected(size, tab);
            if open {
                apply(&mut app, &world, Action::Inspector);
                assert!(app.inspector_open());
            }
            let missing = without_a_tooltip(&mut app, &world, size);
            assert!(missing.is_empty(), "{size:?} {tab:?}: {missing:?}");
        }
    }
    // With no room for the inspector, the event log is as wide as the
    // screen: its filters still say what they are, and the name in a line
    // of it, which says nothing, still lights up its sprite.
    for width in [24, 30, 40] {
        let size = Size::new(width, 30);
        let world = field(&[], &[at(2, 2)]);
        let mut app = sized_app(&world, Theme::cp437(), size);
        app.record(&[grew_up(sprite_on(&world, at(2, 2)))], &world);
        let tile = app.cell_of(at(2, 2)).expect("in view");
        let lit = |app: &App| {
            render_on(app, &world, size)[tile]
                .modifier
                .contains(Modifier::REVERSED)
        };
        assert!(!lit(&app));
        let cells = spot_cells(&render_on(&app, &world, size));
        assert!(!cells.is_empty(), "something to point at at {size:?}");
        let (mut idle, mut lighting) = (Vec::new(), 0);
        for cell in cells {
            let said = pointing_at(&mut app, &world, size, cell);
            let says = said.is_some_and(|said| !said.name.is_empty() && !said.area.contains(cell));
            if lit(&app) {
                lighting += 1;
            } else if !says {
                idle.push(cell);
            }
        }
        assert!(idle.is_empty(), "{size:?}: {idle:?}");
        assert!(lighting > 0, "{size:?}: the name lights up its sprite");
    }
}
