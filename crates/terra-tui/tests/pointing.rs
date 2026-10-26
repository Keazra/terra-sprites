//! What the pointer is over, said on the status line, and a click on a
//! line of the event log (#304): each part of the screen notes what it is
//! as it's drawn.

mod common;
use common::*;

use ratatui::layout::{Position, Size};
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

/// The status line of a `size` screen once the pointer is on `text`.
fn pointing_at_text(app: &mut App, world: &World, size: Size, text: &str) -> String {
    let at = cell_of_text(app, world, size, text);
    apply(app, world, Action::Point(at));
    row(&render_on(app, world, size), size.height - 1)
}

#[test]
fn a_drives_bar_says_what_raises_it_and_what_lowers_it() {
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    let status = pointing_at_text(&mut app, &world, SCREEN, "hunger");
    assert!(
        status.starts_with(" hunger: rises as energy runs low, and eating brings it down at once"),
        "{status:?}"
    );
    // At the bar's end too, past the name.
    let bar = cell_of_text(&app, &world, SCREEN, "tiredness");
    apply(
        &mut app,
        &world,
        Action::Point(Position::new(bar.x + 20, bar.y)),
    );
    let status = status_line(&app, &world);
    assert!(
        status.starts_with(" tiredness: rises as stamina"),
        "{status:?}"
    );
}

#[test]
fn a_chemical_says_what_it_is_on_its_line_and_in_the_hormones_grid() {
    let (world, mut app, _) = selected(SCREEN, Tab::Chem);
    let status = pointing_at_text(&mut app, &world, SCREEN, "energy");
    assert!(
        status.starts_with(" energy: what it runs on: food in its gut turns into it"),
        "{status:?}"
    );
    let status = pointing_at_text(&mut app, &world, SCREEN, "h3 ");
    assert!(
        status.starts_with(" h3: a spare hormone its genes may use for anything"),
        "{status:?}"
    );
    // A later cell in the grid, to its last column, and not the gap after.
    let h7 = cell_of_text(&app, &world, SCREEN, "h7 ");
    apply(
        &mut app,
        &world,
        Action::Point(Position::new(h7.x + 7, h7.y)),
    );
    let status = status_line(&app, &world);
    assert!(status.starts_with(" h7: "), "{status:?}");
    apply(
        &mut app,
        &world,
        Action::Point(Position::new(h7.x + 8, h7.y)),
    );
    let status = status_line(&app, &world);
    assert!(!status.starts_with(" h"), "{status:?}");
    // The named hormones are lower down.
    let (world, mut app, _) = selected(TALL, Tab::Chem);
    let status = pointing_at_text(&mut app, &world, TALL, "receptivity");
    assert!(
        status.starts_with(" receptivity: rises when she's danced for"),
        "{status:?}"
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
    let status = pointing_at_text(&mut app, &world, SCREEN, "hunger");
    assert!(
        status.starts_with(" hunger: a drive, an urge it feels"),
        "{status:?}"
    );
}

#[test]
fn words_too_long_for_the_status_line_end_in_dots() {
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
    let status = pointing_at_text(&mut app, &world, SCREEN, "hunger");
    assert!(status.starts_with(" hunger: ever so hungry"), "{status:?}");
    assert!(status.trim_end().ends_with("..."), "{status:?}");
    assert!(
        status.chars().count() <= usize::from(SCREEN.width),
        "{status:?}"
    );
}

#[test]
fn a_gene_says_what_it_does() {
    let (world, mut app, _) = selected(TALL, Tab::Genome);
    let says = |app: &mut App, text: &str| pointing_at_text(app, &world, TALL, text);
    // Spawn variation moves the numbers, so only the words are checked.
    let status = says(&mut app, "hunger halves");
    assert!(
        status.starts_with(" A half-life: hunger fades by itself, by half every "),
        "{status:?}"
    );
    let status = says(&mut app, "low energy → hunger");
    assert!(
        status.starts_with(" An emitter: hunger rises while energy is below ."),
        "{status:?}"
    );
    let status = says(&mut app, "ate → hunger");
    assert!(
        status.starts_with(" An emitter: ate lowers hunger"),
        "{status:?}"
    );
    let status = says(&mut app, "fullness → hunger");
    assert!(
        status.starts_with(" An emitter: hunger falls while fullness is above ."),
        "{status:?}"
    );
}

#[test]
fn each_trait_on_the_line_they_share_says_what_it_is() {
    let (world, mut app, _) = selected(TALL, Tab::Genome);
    let status = pointing_at_text(&mut app, &world, TALL, "speed");
    assert!(
        status.starts_with(" A trait: speed is how fast it walks"),
        "{status:?}"
    );
    let status = pointing_at_text(&mut app, &world, TALL, "sense");
    assert!(
        status.starts_with(" A trait: sense is how far it senses"),
        "{status:?}"
    );
    // Between them, the dot is neither.
    let sense = cell_of_text(&app, &world, TALL, "sense");
    apply(
        &mut app,
        &world,
        Action::Point(Position::new(sense.x - 2, sense.y)),
    );
    let status = row(&render_on(&app, &world, TALL), TALL.height - 1);
    assert!(!status.contains("A trait"), "{status:?}");
}

#[test]
fn a_tab_says_what_it_shows_and_its_keys() {
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    let status = pointing_at_text(&mut app, &world, SCREEN, "Brain");
    assert!(
        status.starts_with(
            " Brain tab: what it notices and decides, and what it has learned · [ ] switch tabs"
        ),
        "{status:?}"
    );
}

#[test]
fn the_event_filters_say_what_they_do() {
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    let status = pointing_at_text(&mut app, &world, SCREEN, "selected major");
    assert!(
        status.starts_with(" Which events the log shows · click, or m, for the next"),
        "{status:?}"
    );
}

#[test]
fn back_on_the_map_the_status_line_names_the_tile_again() {
    let (world, mut app, _) = selected(SCREEN, Tab::Body);
    pointing_at_text(&mut app, &world, SCREEN, "hunger");
    let tile = app.cell_of(at(5, 4)).expect("in view");
    apply(
        &mut app,
        &world,
        Action::Point(Position::new(tile.x + 1, tile.y + 1)),
    );
    let status = status_line(&app, &world);
    assert!(status.starts_with(" (5,4) grass │ SELECT"), "{status:?}");
}

#[test]
fn a_log_line_says_which_sprite_it_is_about_and_lights_it_up_on_the_map() {
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
    let status = pointing_at_text(&mut app, &world, SCREEN, "grew up");
    assert!(
        status.starts_with(&format!(" About {label} · click to select it")),
        "{status:?}"
    );
    assert!(lit(&app, at(7, 4)), "the sprite it's about");
    assert!(!lit(&app, at(2, 2)), "not another");
    // A click selects it.
    let line = cell_of_text(&app, &world, SCREEN, "grew up");
    apply(&mut app, &world, Action::left_click(line));
    assert_eq!(app.selection(), Some(Selection::Living(second)));
    let status = status_line(&app, &world);
    assert!(
        status.starts_with(&format!(" About {label}, the selected sprite")),
        "{status:?}"
    );
    assert_ne!(first, second);
}

#[test]
fn a_click_on_a_line_about_a_sprite_no_longer_living_selects_nothing() {
    let world = field(&[], &[at(2, 2)]);
    let mut app = app_on(&world);
    let gone = EntityId(9_999);
    app.record(&[died(1, gone, DeathCause::Starvation, 40)], &world);
    let status = pointing_at_text(&mut app, &world, SCREEN, "died");
    assert!(
        status.starts_with(" About Sprite #9999, who has died"),
        "{status:?}"
    );
    let line = cell_of_text(&app, &world, SCREEN, "died");
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
    let line = cell_of_text(&app, &world, SCREEN, "grew up");
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
        let bar = cell_of_text(&app, &world, SCREEN, "hunger");
        apply(&mut app, &world, open);
        apply(&mut app, &world, Action::Point(bar));
        let status = status_line(&app, &world);
        assert!(!status.contains("hunger"), "{open:?}: {status:?}");
    }
}
