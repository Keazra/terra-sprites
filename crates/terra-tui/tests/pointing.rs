//! Tooltips, saying what the pointer is over, and a click on a line of
//! the event log (#304): each part of the screen notes what it is as it's
//! drawn.

mod common;
use common::*;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect, Size};
use ratatui::style::Modifier;
use terra_sim::{DeathCause, EntityId, Event, EventKind, World};
use terra_tui::app::{App, Selection, Tab};
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

/// The screen cell where `text` starts on the first row of a `size` screen
/// that has it.
fn cell_of_text(app: &App, world: &World, size: Size, text: &str) -> Position {
    let screen = render_on(app, world, size);
    for y in 0..size.height {
        let row = row(&screen, y);
        if let Some(x) = row.find(text) {
            return Position::new(row[..x].chars().count() as u16, y);
        }
    }
    panic!("{text:?} isn't on screen:\n{}", lines(&screen).join("\n"));
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
/// drawn there: a box with a corner diagonally next to it.
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
fn a_tab_says_what_it_shows_and_its_keys() {
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    let said = pointing_at_text(&mut app, &world, SCREEN, "Brain");
    assert_eq!(said.name, "Brain tab");
    assert_eq!(
        said.words,
        "What it notices and decides, and what it has learned click to open it · [ ] switch tabs"
    );
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
    assert_eq!(fg("Brain", 0), spot, "a tab's name");
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
fn a_log_line_s_name_says_which_sprite_it_is_about_and_lights_it_up_on_the_map() {
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
    let said = pointing_at_text(&mut app, &world, SCREEN, &format!("{label} grew up"));
    assert_eq!(said.name, label);
    assert_eq!(said.words, "Whom this line is about click to select it");
    assert!(lit(&app, at(7, 4)), "the sprite it's about");
    assert!(!lit(&app, at(2, 2)), "not another");
    // What happened, after the name, says nothing.
    let rest = cell_of_text(&app, &world, SCREEN, "grew up");
    assert!(pointing_at(&mut app, &world, SCREEN, rest).is_none());
    // A click on the name selects it.
    let line = cell_of_text(&app, &world, SCREEN, &format!("{label} grew up"));
    apply(&mut app, &world, Action::left_click(line));
    assert_eq!(app.selection(), Some(Selection::Living(second)));
    let said = pointing_at(&mut app, &world, SCREEN, line).expect("a tooltip");
    assert_eq!(said.words, "The selected sprite, whom this line is about");
    assert_ne!(first, second);
}

#[test]
fn a_click_on_a_line_about_a_sprite_no_longer_living_selects_nothing() {
    let world = field(&[], &[at(2, 2)]);
    let mut app = app_on(&world);
    let gone = EntityId(9_999);
    app.record(&[died(1, gone, DeathCause::Starvation, 40)], &world);
    let said = pointing_at_text(&mut app, &world, SCREEN, "Sprite #9999");
    assert_eq!(said.words, "Whom this line is about, who has died");
    let line = cell_of_text(&app, &world, SCREEN, "Sprite #9999");
    apply(&mut app, &world, Action::left_click(line));
    assert_eq!(app.selection(), None);
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

#[test]
fn a_click_on_the_debug_overlay_never_lands_on_the_map_beneath() {
    // Where the overlay's box is drawn, over the map view's top right.
    let empty = WorldBuilder::grass(60, 30).build();
    let mut app = app_on(&empty);
    apply(&mut app, &empty, Action::DebugOverlay);
    let corner = cell_of_text(&app, &empty, SCREEN, " Debug ");
    let cell = Position::new(corner.x + 3, corner.y + 2);
    let tile = app.tile_at(cell).expect("a tile beneath the box");
    // A sprite there, which a click selects with the overlay hidden.
    let world = WorldBuilder::grass(60, 30).starters(&[tile]).build();
    let id = sprite_on(&world, tile);
    let click = Action::left_click(Position::new(cell.x + 1, cell.y + 1));
    for shown in [false, true] {
        let mut app = app_on(&world);
        if shown {
            apply(&mut app, &world, Action::DebugOverlay);
        }
        render(&app, &world);
        apply(&mut app, &world, click);
        let selected = app.selection() == Some(Selection::Living(id));
        assert_eq!(selected, !shown, "overlay shown: {shown}");
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
