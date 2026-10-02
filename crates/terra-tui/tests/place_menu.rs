//! The Place menu, naming and genome files on screen (design v28 §6.5): `C` again
//! in Grab mode opens the menu, the chosen item waits on the Cursor until a
//! click places it, `r` names the selected sprite and `g` exports its genome.

use std::path::PathBuf;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::layout::{Position, Rect, Size};
use terra_sim::{Command, DataPack, Genome, Map, Pos, Scenario, World};
use terra_tui::app::{App, Areas, CursorMode, Flow, Screen, StatusMark};
use terra_tui::input::{Action, Button, Keys};
use terra_tui::theme::Theme;
use terra_tui::ui;

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// An all-grass world, 10×6, with `objects`, and starter sprites on
/// `sprites`.
fn field(objects: &[(Pos, &str)], sprites: &[Pos]) -> World {
    let rows = vec![".........."; 6];
    let map = Map::from_ascii(&rows, &pack()).expect("valid drawing");
    let sprites: Vec<(Pos, _)> = sprites.iter().map(|&pos| (pos, None)).collect();
    let scenario = Scenario {
        map,
        objects,
        sprites: &sprites,
        scripted: &[],
    };
    World::from_scenario(scenario, pack(), 1).expect("valid scenario")
}

/// The app, with the map view's tiles drawn from screen cell (0, 0), so a
/// tile's cell is its position.
fn app_for(world: &World) -> App {
    let areas = Areas {
        tiles: Rect::new(0, 0, 10, 6),
        inspector: None,
    };
    App::new(world.map(), Theme::cp437(), 1, areas)
}

/// The app in Grab mode.
fn grab_app(world: &World) -> App {
    let mut app = app_for(world);
    apply(&mut app, world, Action::Mode(CursorMode::Grab));
    app
}

fn apply(app: &mut App, world: &World, action: Action) {
    assert_eq!(app.apply(action, world), Flow::Continue);
}

/// The screen cell the pointer is on to point at `tile`: one down and right of
/// it (design v27 §6.5). The map view's tiles are drawn from screen cell (0, 0).
fn pointing_at(tile: Pos) -> Position {
    Position::new(tile.x + 1, tile.y + 1)
}

fn click(app: &mut App, world: &World, tile: Pos, button: Button) {
    let action = Action::Click {
        at: pointing_at(tile),
        button,
        amplified: false,
    };
    apply(app, world, action);
}

/// Hands the app's commands to the world and runs a tick, as a frame does.
fn tick(app: &mut App, world: &mut World) {
    for command in app.take_commands() {
        world.submit(command);
    }
    let events = world.step();
    app.record(&events, world);
}

/// Opens the Place menu and picks its `n`th item by number.
fn pick(app: &mut App, world: &World, n: u8) {
    apply(app, world, Action::Mode(CursorMode::Grab));
    assert_eq!(app.screen(), Screen::PlaceMenu);
    apply(app, world, Action::Pick(n));
}

fn ball_type(world: &World) -> u16 {
    world.data().object_type_id("ball").expect("a ball type")
}

/// A fresh, empty folder for genome files.
fn scratch_folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("terra-place-menu-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

#[test]
fn c_again_in_grab_mode_opens_the_place_menu_and_esc_closes_it() {
    let world = field(&[], &[]);
    let mut app = app_for(&world);
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
    let mut app = app_for(&world);
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
    let mut app = app_for(&world);
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
    let mut app = app_for(&world);
    click(&mut app, &world, at(4, 2), Button::Left);
    apply(&mut app, &world, Action::Rename);
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.take_commands(), Vec::new());
}

#[test]
fn r_with_no_sprite_selected_says_so() {
    let world = field(&[], &[]);
    let mut app = app_for(&world);
    apply(&mut app, &world, Action::Rename);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.refusal(), Some("Select a sprite to name it"));
}

fn key(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind,
        state: KeyEventState::NONE,
    }
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
    let rows = vec!["................................"; 32];
    let map = Map::from_ascii(&rows, &pack()).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[],
        sprites: &[],
        scripted: &[],
    };
    let world = World::from_scenario(scenario, pack(), 1).expect("valid scenario");
    let mut app = grab_app(&world);
    app.fit(ui::areas(Size::new(60, 20), world.map()));
    apply(&mut app, &world, Action::Mode(CursorMode::Grab));
    let screen = rendered(&app, &world, 60, 20);
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
    // A click on the top row picks the first item shown.
    apply(
        &mut app,
        &world,
        Action::Click {
            at: Position::new(area.x + 2, area.y + 1),
            button: Button::Left,
            amplified: false,
        },
    );
    assert_eq!(app.screen(), Screen::Normal);
    let _ = std::fs::remove_dir_all(&dir);
}

fn status_line(app: &App, world: &World) -> String {
    let screen = rendered(app, world, 100, 20);
    screen.lines().last().unwrap_or_default().to_string()
}

/// The screen drawn into a `width` × `height` buffer, as text, a line a row.
fn rendered(app: &App, world: &World, width: u16, height: u16) -> String {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| terra_tui::ui::render(frame, app, world))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
