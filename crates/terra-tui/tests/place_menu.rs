//! The Place menu, naming and genome files on screen (design v28 §6.5): `C` again
//! in Grab mode opens the menu, the chosen item waits on the Cursor until a
//! click places it, `r` names the selected sprite and `g` exports its genome.

mod common;
use common::{
    WorldBuilder, app, apply, at, click, field, grab_app, key, lines, pointing_at, render,
    scratch_folder, status_line, tick,
};

use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEventKind};
use ratatui::layout::{Position, Size};
use terra_sim::{Command, EntityId, Event, EventKind, Genome, World};
use terra_tui::app::{App, CursorMode, Screen, StatusMark};
use terra_tui::input::{Action, Button, Keys};
use terra_tui::ui;

/// Opens the Place menu and picks its `n`th item by number.
fn pick(app: &mut App, world: &World, n: u8) {
    apply(app, world, Action::Mode(CursorMode::Grab));
    assert_eq!(app.screen(), Screen::PlaceMenu);
    apply(app, world, Action::Pick(n));
}

fn ball_type(world: &World) -> u16 {
    world.data().object_type_id("ball").expect("a ball type")
}

#[test]
fn c_again_in_grab_mode_opens_the_place_menu_and_esc_closes_it() {
    let world = field(&[], &[]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    assert_eq!(app.screen(), Screen::Normal, "the first C picks Grab mode");
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    assert_eq!(app.screen(), Screen::PlaceMenu);
    assert_eq!(
        app.menu_items(&world),
        [
            "berry bush seedling",
            "berry",
            "ball",
            "new sprite",
            "sprite from a genome file",
        ]
    );
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.mode(), CursorMode::Grab, "Esc closes the menu first");
}

#[test]
fn following_a_sprite_a_click_anywhere_places_the_item_at_its_feet() {
    // Design v28 §6.5: the item goes where the Cursor is, as a held one would.
    let world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    apply(
        &mut app,
        &world,
        Action::middle_click(pointing_at(at(4, 2))),
    );
    pick(&mut app, &world, 3);
    click(&mut app, &world, at(8, 5), Button::Left);
    assert_eq!(
        app.take_commands(),
        vec![Command::Place {
            tile: at(4, 2),
            object_type: ball_type(&world)
        }]
    );
}

#[test]
fn a_chosen_item_waits_on_the_cursor_and_the_next_click_places_it() {
    let mut world = field(&[], &[]);
    let mut app = grab_app(&world);
    pick(&mut app, &world, 3);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.placing(), Some("ball"));
    assert_eq!(
        app.status_marks(&world),
        [StatusMark::Release, StatusMark::Holding]
    );
    click(&mut app, &world, at(4, 2), Button::Left);
    let object_type = ball_type(&world);
    assert_eq!(
        app.take_commands(),
        vec![Command::Place {
            tile: at(4, 2),
            object_type
        }]
    );
    assert_eq!(app.placing(), None, "one click places it");
    world.submit(Command::Place {
        tile: at(4, 2),
        object_type,
    });
    tick(&mut app, &mut world);
    assert_eq!(
        world.object_at(at(4, 2)).map(|o| o.type_name()),
        Some("ball")
    );
}

#[test]
fn the_menu_is_worked_with_the_arrows_and_enter_too() {
    let world = field(&[], &[]);
    let mut app = grab_app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    assert_eq!(app.menu_choice(), 0);
    apply(&mut app, &world, Action::Scroll { dx: 0, dy: 1 });
    apply(&mut app, &world, Action::Scroll { dx: 0, dy: 1 });
    apply(&mut app, &world, Action::Scroll { dx: 0, dy: -1 });
    assert_eq!(app.menu_choice(), 1);
    apply(&mut app, &world, Action::Enter);
    assert_eq!(app.placing(), Some("berry"));
}

#[test]
fn the_mouse_wheel_moves_the_menu_highlight() {
    // #128: as the arrows do, a notch a row, and no further than the ends.
    let world = field(&[], &[]);
    let mut app = grab_app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    let area = app.menu_area(&world).expect("the Place menu");
    let wheel = |notches| Action::Wheel {
        at: Position::new(area.x + 2, area.y + 1),
        notches,
    };
    apply(&mut app, &world, wheel(-1));
    assert_eq!(app.menu_choice(), 0);
    apply(&mut app, &world, wheel(1));
    apply(&mut app, &world, wheel(1));
    apply(&mut app, &world, wheel(-1));
    assert_eq!(app.menu_choice(), 1);
    apply(&mut app, &world, wheel(9));
    assert_eq!(app.menu_choice(), 4, "the last item");
    assert_eq!(app.screen(), Screen::PlaceMenu);
    apply(&mut app, &world, wheel(-3));
    apply(&mut app, &world, Action::Enter);
    assert_eq!(app.placing(), Some("berry"));
}

#[test]
fn a_new_sprite_is_spawned_from_the_starter_genome() {
    let world = field(&[], &[]);
    let mut app = grab_app(&world);
    pick(&mut app, &world, 4);
    assert_eq!(app.placing(), Some("new sprite"));
    click(&mut app, &world, at(1, 1), Button::Left);
    assert_eq!(
        app.take_commands(),
        vec![Command::SpawnSprite {
            tile: at(1, 1),
            genome: None
        }]
    );
}

#[test]
fn a_right_click_puts_the_waiting_item_away() {
    let world = field(&[(at(4, 2), "ball")], &[]);
    let mut app = grab_app(&world);
    pick(&mut app, &world, 2);
    click(&mut app, &world, at(4, 2), Button::Right);
    assert_eq!(app.placing(), None);
    assert!(!app.aiming(), "it doesn't grab and aim the ball there");
    assert_eq!(app.take_commands(), Vec::new());
}

#[test]
fn a_waiting_item_s_key_hints_say_how_to_place_it_or_put_it_away() {
    // Design v28 §6.5, in Grab mode, where clicks place.
    let world = field(&[], &[]);
    let mut app = grab_app(&world);
    assert!(!status_line(&app, &world).contains("click to place"));
    pick(&mut app, &world, 3);
    let status = status_line(&app, &world);
    assert!(
        status.ends_with("click to place  right-click put away"),
        "{status:?}"
    );
    // In another mode, clicks don't place: the usual hints.
    apply(&mut app, &world, Action::Mode(CursorMode::Select));
    assert!(!status_line(&app, &world).contains("click to place"));
}

#[test]
fn esc_puts_the_waiting_item_away_before_it_lets_go_or_leaves_the_mode() {
    // Design v33 §6.5: the item waits on the Cursor as something held
    // would, so Esc lets go of it first, as a right click does.
    let mut world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    tick(&mut app, &mut world);
    pick(&mut app, &world, 2);
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.placing(), None);
    assert_eq!(app.take_commands(), Vec::new(), "the lead is kept");
    assert_eq!(app.mode(), CursorMode::Grab);
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.take_commands(), vec![Command::LetGo]);
}

#[test]
fn a_waiting_item_is_kept_through_other_modes_and_shown_on_the_status_line() {
    let world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    pick(&mut app, &world, 2);
    apply(&mut app, &world, Action::Mode(CursorMode::Select));
    click(&mut app, &world, at(4, 2), Button::Left);
    assert!(app.selection().is_some(), "Select mode's click selects");
    assert_eq!(app.placing(), Some("berry"));
    assert!(status_line(&app, &world).contains("placing: berry"));
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    assert_eq!(
        app.screen(),
        Screen::Normal,
        "C from another mode only picks Grab"
    );
    assert_eq!(app.placing(), Some("berry"));
}

#[test]
fn placing_while_leading_keeps_the_lead() {
    let mut world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    let sprite = world.sprite_at(at(4, 2)).unwrap().id();
    click(&mut app, &world, at(4, 2), Button::Left);
    tick(&mut app, &mut world);
    pick(&mut app, &world, 2);
    click(&mut app, &world, at(5, 2), Button::Left);
    let commands = app.take_commands();
    assert!(
        commands.contains(&Command::Place {
            tile: at(5, 2),
            object_type: world.data().object_type_id("berry").unwrap()
        }),
        "{commands:?}"
    );
    assert!(!commands.contains(&Command::LetGo), "{commands:?}");
    for command in commands {
        world.submit(command);
    }
    let events = world.step();
    app.record(&events, &world);
    assert_eq!(world.cursor().leads(), Some(sprite));
}

#[test]
fn a_refused_placement_says_why() {
    let mut world = field(&[(at(4, 2), "berry")], &[]);
    let mut app = grab_app(&world);
    pick(&mut app, &world, 3);
    click(&mut app, &world, at(4, 2), Button::Left);
    tick(&mut app, &mut world);
    assert_eq!(app.status_mark(), StatusMark::Rejected);
    assert_eq!(
        app.refusal(),
        Some("Couldn't place the ball: a berry is there")
    );
}

#[test]
fn r_names_the_selected_sprite_with_a_random_name_to_start_from() {
    let mut world = field(&[], &[at(4, 2)]);
    let mut app = app(&world);
    let sprite = world.sprite_at(at(4, 2)).unwrap().id();
    click(&mut app, &world, at(4, 2), Button::Left);
    apply(&mut app, &world, Action::Rename);
    assert_eq!(app.screen(), Screen::Naming);
    let offered = app.name_draft().expect("a draft").to_string();
    assert!(!offered.is_empty());
    apply(&mut app, &world, Action::AnotherName);
    let mut seen = vec![offered];
    for _ in 0..5 {
        seen.push(app.name_draft().unwrap().to_string());
        apply(&mut app, &world, Action::AnotherName);
    }
    seen.dedup();
    assert!(seen.len() > 1, "Tab offers another: {seen:?}");
    // Typing replaces the offered name.
    for c in "Mirx".chars() {
        apply(&mut app, &world, Action::Type(c));
    }
    assert_eq!(app.name_draft(), Some("Mirx"));
    apply(&mut app, &world, Action::Erase);
    apply(&mut app, &world, Action::Type('a'));
    apply(&mut app, &world, Action::Enter);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(
        app.take_commands(),
        vec![Command::Rename {
            sprite,
            name: "Mira".into()
        }]
    );
    world.submit(Command::Rename {
        sprite,
        name: "Mira".into(),
    });
    tick(&mut app, &mut world);
    assert_eq!(app.names().label(sprite), format!("Mira #{}", sprite.0));
}

#[test]
fn a_typed_name_stays_within_sixteen_cp437_characters() {
    let world = field(&[], &[at(4, 2)]);
    let mut app = app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    apply(&mut app, &world, Action::Rename);
    apply(&mut app, &world, Action::Type('Z'));
    apply(&mut app, &world, Action::Type('🙂'));
    for _ in 0..20 {
        apply(&mut app, &world, Action::Type('o'));
    }
    let draft = app.name_draft().unwrap();
    assert_eq!(draft.chars().count(), 16);
    assert!(draft.starts_with("Zo"), "{draft}");
}

#[test]
fn esc_leaves_naming_without_sending_anything() {
    let world = field(&[], &[at(4, 2)]);
    let mut app = app(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    apply(&mut app, &world, Action::Rename);
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.take_commands(), Vec::new());
}

#[test]
fn r_with_no_sprite_selected_says_so() {
    let world = field(&[], &[]);
    let mut app = app(&world);
    apply(&mut app, &world, Action::Rename);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.refusal(), Some("Select a sprite to name it"));
}

#[test]
fn while_naming_keys_type_letters_rather_than_act() {
    let mut keys = Keys::with_release_reporting(false);
    let mut typed = |code| keys.typed_action(key(code, KeyEventKind::Press));
    assert_eq!(typed(KeyCode::Char('q')), Some(Action::Type('q')));
    assert_eq!(typed(KeyCode::Char(' ')), Some(Action::Type(' ')));
    assert_eq!(typed(KeyCode::Backspace), Some(Action::Erase));
    assert_eq!(typed(KeyCode::Enter), Some(Action::Enter));
    assert_eq!(typed(KeyCode::Tab), Some(Action::AnotherName));
    assert_eq!(typed(KeyCode::Esc), Some(Action::Back));
}

#[test]
fn the_r_that_opens_the_prompt_counts_as_let_go_and_a_held_r_types_nothing() {
    // Windows reports releases, and those arrive while the prompt is open.
    let mut keys = Keys::with_release_reporting(true);
    let r = |kind| key(KeyCode::Char('r'), kind);
    assert_eq!(
        keys.action_for(r(KeyEventKind::Press)),
        Some(Action::Rename)
    );
    assert_eq!(keys.typed_action(r(KeyEventKind::Repeat)), None);
    assert_eq!(keys.typed_action(r(KeyEventKind::Release)), None);
    assert_eq!(
        keys.typed_action(r(KeyEventKind::Press)),
        Some(Action::Type('r'))
    );
    keys.typed_action(r(KeyEventKind::Release));
    assert_eq!(
        keys.typed_action(key(KeyCode::Enter, KeyEventKind::Press)),
        Some(Action::Enter)
    );
    // Back on the map, the next `r` is a fresh press.
    assert_eq!(
        keys.action_for(r(KeyEventKind::Press)),
        Some(Action::Rename)
    );
}

#[test]
fn only_letters_and_spaces_are_typed() {
    let world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Select));
    click(&mut app, &world, at(4, 2), Button::Left);
    apply(&mut app, &world, Action::Rename);
    for c in "Al-7 ║Bo".chars() {
        apply(&mut app, &world, Action::Type(c));
    }
    assert_eq!(app.name_draft(), Some("Al Bo"));
}

#[test]
fn g_exports_the_selected_sprites_genome_and_it_comes_back_as_a_menu_item() {
    let mut world = field(&[], &[at(4, 2)]);
    let dir = scratch_folder("export");
    let mut app = grab_app(&world);
    app.set_genome_folder(dir.clone());
    apply(&mut app, &world, Action::Mode(CursorMode::Select));
    click(&mut app, &world, at(4, 2), Button::Left);
    let sprite = world.sprite_at(at(4, 2)).unwrap().id();
    apply(&mut app, &world, Action::ExportGenome);
    let files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1, "{files:?}");
    let text = std::fs::read_to_string(&files[0]).unwrap();
    let genome = world.sprite(sprite).unwrap().genome().clone();
    assert_eq!(Genome::from_ron(&text, world.data()).unwrap(), genome);
    let notice = app.notice().expect("it says where");
    assert!(notice.starts_with("Saved Sprite #"), "{notice}");

    // The fifth item lists the folder's genome files.
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    pick(&mut app, &world, 5);
    assert_eq!(app.screen(), Screen::GenomeMenu);
    let items = app.menu_items(&world);
    assert_eq!(items.len(), 1);
    apply(&mut app, &world, Action::Pick(1));
    assert_eq!(app.placing(), Some(items[0].as_str()));
    click(&mut app, &world, at(1, 1), Button::Left);
    assert_eq!(
        app.take_commands(),
        vec![Command::SpawnSprite {
            tile: at(1, 1),
            genome: Some(genome.clone())
        }]
    );
    world.submit(Command::SpawnSprite {
        tile: at(1, 1),
        genome: Some(genome.clone()),
    });
    tick(&mut app, &mut world);
    assert_eq!(world.sprite_at(at(1, 1)).unwrap().genome(), &genome);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_genome_file_that_doesnt_read_is_refused_with_why() {
    let world = field(&[], &[]);
    let dir = scratch_folder("bad");
    std::fs::write(dir.join("broken.ron"), "(format: 9, genes: [])").unwrap();
    let mut app = grab_app(&world);
    app.set_genome_folder(dir.clone());
    pick(&mut app, &world, 5);
    assert_eq!(app.menu_items(&world), ["broken"]);
    apply(&mut app, &world, Action::Pick(1));
    assert_eq!(app.placing(), None);
    let why = app.refusal().expect("a reason");
    assert!(why.starts_with("Couldn't read broken: "), "{why}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_place_menu_draws_over_the_map_with_its_items_numbered() {
    // As small as a preset lets a map be (`data/presets/default.ron`).
    let world = WorldBuilder::grass(32, 32).build();
    let mut app = grab_app(&world);
    app.fit(ui::areas(Size::new(100, 30)));
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    let screen = lines(&render(&app, &world)).join("\n");
    assert!(screen.contains("1 berry bush seedling"), "{screen}");
    assert!(screen.contains("5 sprite from a genome file"), "{screen}");
}

#[test]
fn a_menu_longer_than_the_map_view_scrolls_to_keep_the_highlight_in_view() {
    let world = field(&[], &[]);
    let dir = scratch_folder("many");
    for n in 0..12 {
        std::fs::write(dir.join(format!("file-{n:02}.ron")), "").unwrap();
    }
    let mut app = grab_app(&world);
    app.set_genome_folder(dir.clone());
    pick(&mut app, &world, 5);
    // The map view is 6 rows: 4 items show inside the border.
    let area = app.menu_area(&world).expect("the genome menu");
    assert_eq!(area.height, 6);
    assert_eq!(app.menu_first(&world), 0);
    for _ in 0..6 {
        apply(&mut app, &world, Action::Scroll { dx: 0, dy: 1 });
    }
    assert_eq!(app.menu_choice(), 6);
    assert_eq!(app.menu_first(&world), 3);
    // The wheel scrolls it too (#128).
    let inside = Position::new(area.x + 2, area.y + 1);
    for notches in [1, 1, -1, 1] {
        apply(
            &mut app,
            &world,
            Action::Wheel {
                at: inside,
                notches,
            },
        );
    }
    assert_eq!(app.menu_choice(), 8);
    assert_eq!(app.menu_first(&world), 5);
    for _ in 0..8 {
        apply(
            &mut app,
            &world,
            Action::Wheel {
                at: inside,
                notches: -1,
            },
        );
    }
    assert_eq!((app.menu_choice(), app.menu_first(&world)), (0, 0));
    // A click on the top row picks the first item shown.
    let top_row = Position::new(area.x + 2, area.y + 1);
    apply(&mut app, &world, Action::left_click(top_row));
    assert_eq!(app.screen(), Screen::Normal);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pick_zero_picks_nothing() {
    let world = field(&[], &[]);
    let mut app = grab_app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    apply(&mut app, &world, Action::Pick(0));
    assert_eq!(app.screen(), Screen::PlaceMenu);
    assert_eq!(app.placing(), None);
}

#[test]
fn a_folder_named_like_a_genome_file_isnt_listed() {
    let world = field(&[], &[]);
    let dir = scratch_folder("folder");
    std::fs::create_dir_all(dir.join("nested.ron")).unwrap();
    std::fs::write(dir.join("mira.ron"), "").unwrap();
    let mut app = grab_app(&world);
    app.set_genome_folder(dir.clone());
    pick(&mut app, &world, 5);
    assert_eq!(app.menu_items(&world), ["mira"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_naming_prompt_shows_its_key_hints() {
    // The narrowest screen the game draws on, 100 columns, has room for the
    // longest name and its hints (design §6.1).
    let world = field(&[], &[at(4, 2)]);
    let mut app = grab_app(&world);
    apply(&mut app, &world, Action::Mode(CursorMode::Select));
    click(&mut app, &world, at(4, 2), Button::Left);
    apply(&mut app, &world, Action::Rename);
    let line = status_line(&app, &world);
    assert!(line.starts_with(" Name Sprite #"), "{line}");
    assert!(line.contains("enter ok  tab another  esc cancel"), "{line}");
}

#[test]
fn a_rename_names_the_sprite_from_its_event() {
    let world = field(&[], &[]);
    let mut app = grab_app(&world);
    let id = EntityId(77);
    let renamed = Event {
        tick: 1,
        kind: EventKind::Renamed {
            id,
            name: "Tobek".into(),
        },
    };
    app.record(&[renamed], &world);
    assert_eq!(app.names().label(id), "Tobek #77");
}
