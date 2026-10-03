//! Saving and loading from the game (design §6.7): `F5` and `F9`, saves
//! by name, the list of saves, the question before loading over a world
//! that has run, and the autosaves.

use std::path::{Path, PathBuf};
use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use terra_sim::{Command, DataPack, Map, Pos, Scenario, World};
use terra_tui::app::{AUTOSAVE_EVERY, App, Areas, CursorMode, Flow, Screen};
use terra_tui::input::{Action, Keys};
use terra_tui::saves::{self, QUICKSAVE};
use terra_tui::theme::Theme;
use terra_tui::ui;

fn pack() -> DataPack {
    DataPack::builtin().expect("valid pack")
}

fn at(x: u16, y: u16) -> Pos {
    Pos { x, y }
}

/// A grass world with two starter sprites and a ball.
fn world() -> World {
    let rows = vec!["..........."; 8];
    let map = Map::from_ascii(&rows, &pack()).expect("valid drawing");
    let scenario = Scenario {
        map,
        objects: &[(at(5, 5), "ball")],
        sprites: &[(at(2, 2), None), (at(8, 3), None)],
        scripted: &[],
    };
    World::from_scenario(scenario, pack(), 7).expect("valid scenario")
}

/// A fresh, empty folder for saves.
fn scratch_folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("terra-saves-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

/// An app for `world`, saving in `folder`.
fn app_for(world: &World, folder: &Path) -> App {
    let areas = Areas {
        tiles: Rect::new(1, 2, 11, 8),
        inspector: None,
        event_log: None,
        overlay: None,
    };
    let mut app = App::new(world.map(), Theme::cp437(), world.seed(), areas);
    app.set_save_folder(folder.to_path_buf());
    app
}

fn apply(app: &mut App, world: &World, action: Action) {
    assert_eq!(app.apply(action, world), Flow::Continue);
}

fn run(world: &mut World, ticks: u64) {
    for _ in 0..ticks {
        world.step();
    }
}

fn type_text(app: &mut App, world: &World, text: &str) {
    for c in text.chars() {
        apply(app, world, Action::Type(c));
    }
}

/// The top bar and the status line, as drawn on a 120×32 screen.
fn bars(app: &App, world: &World) -> (String, String) {
    let mut terminal = Terminal::new(TestBackend::new(120, 32)).expect("a test terminal");
    terminal
        .draw(|frame| ui::render(frame, app, world))
        .expect("drawn");
    let buffer = terminal.backend().buffer();
    let row = |y: u16| -> String { (0..120).map(|x| buffer[(x, y)].symbol()).collect() };
    (row(0), row(31))
}

#[test]
fn f5_saves_the_quicksave_and_the_top_bar_says_so() {
    let folder = scratch_folder("f5");
    let mut world = world();
    let mut app = app_for(&world, &folder);
    run(&mut world, 5);
    assert!(bars(&app, &world).0.contains("│ not saved"));

    apply(&mut app, &world, Action::Quicksave);
    assert_eq!(app.notice(), Some("Saved as quicksave"));
    let saved = std::fs::read(saves::path_for(&folder, QUICKSAVE)).expect("the quicksave");
    let loaded = World::load(&saved).expect("it loads");
    assert_eq!(loaded.state_hash(), world.state_hash());
    assert!(bars(&app, &world).0.contains("│ saved just now"));

    app.animate(Duration::from_secs(3 * 60 + 5));
    assert!(bars(&app, &world).0.contains("│ saved 3m ago"));
}

#[test]
fn f9_with_no_quicksave_says_so() {
    let folder = scratch_folder("f9-none");
    let world = world();
    let mut app = app_for(&world, &folder);
    apply(&mut app, &world, Action::Quickload);
    assert_eq!(
        app.refusal(),
        Some("There's no quicksave yet: F5 makes one")
    );
    assert!(app.take_loaded().is_none());
}

#[test]
fn f9_loads_the_quicksave_after_asking_if_the_world_has_run_since() {
    let folder = scratch_folder("f9-ask");
    let mut world = world();
    let mut app = app_for(&world, &folder);
    run(&mut world, 10);
    apply(&mut app, &world, Action::Quicksave);
    let saved_hash = world.state_hash();
    app.animate(Duration::from_secs(4 * 60));
    run(&mut world, 30);

    apply(&mut app, &world, Action::Quickload);
    assert_eq!(app.screen(), Screen::LoadPrompt);
    assert_eq!(
        app.load_question().as_deref(),
        Some("Load quicksave? The world has run 4m since it was last saved (y/n)")
    );
    assert!(bars(&app, &world).1.contains("Load quicksave?"));
    assert!(
        app.take_loaded().is_none(),
        "nothing loads before the answer"
    );

    apply(&mut app, &world, Action::Confirm);
    let loaded = app.take_loaded().expect("the quicksave loaded");
    assert_eq!(loaded.tick(), 10);
    assert_eq!(loaded.state_hash(), saved_hash);
    // The game pauses, so the player sees where they are.
    assert!(app.clock.is_paused());
    assert_eq!(app.notice(), Some("Loaded quicksave, at tick 10"));
    assert!(bars(&app, &loaded).0.contains("│ saved just now"));
}

#[test]
fn any_other_key_keeps_the_world_as_it_is() {
    let folder = scratch_folder("f9-cancel");
    let mut world = world();
    let mut app = app_for(&world, &folder);
    apply(&mut app, &world, Action::Quicksave);
    run(&mut world, 3);
    apply(&mut app, &world, Action::Quickload);
    apply(&mut app, &world, Action::Dismiss);
    assert_eq!(app.screen(), Screen::Normal);
    assert!(app.load_question().is_none());
    assert!(app.take_loaded().is_none());
}

#[test]
fn a_world_that_hasnt_run_since_its_save_loads_without_asking() {
    let folder = scratch_folder("f9-quiet");
    let mut world = world();
    let mut app = app_for(&world, &folder);
    run(&mut world, 4);
    apply(&mut app, &world, Action::Quicksave);
    apply(&mut app, &world, Action::Quickload);
    assert_eq!(app.screen(), Screen::Normal);
    assert!(app.take_loaded().is_some());
}

#[test]
fn a_world_never_saved_asks_saying_so() {
    let folder = scratch_folder("never");
    let mut world = world();
    let mut app = app_for(&world, &folder);
    saves::write(&folder, QUICKSAVE, &world.save()).expect("written");
    run(&mut world, 2);
    apply(&mut app, &world, Action::Quickload);
    assert_eq!(
        app.load_question().as_deref(),
        Some("Load quicksave? This world has never been saved (y/n)")
    );
}

#[test]
fn ctrl_s_saves_by_the_name_typed_offering_the_seed_and_tick() {
    let folder = scratch_folder("save-as");
    let mut world = world();
    let mut app = app_for(&world, &folder);
    run(&mut world, 12);
    apply(&mut app, &world, Action::SaveAs);
    assert_eq!(app.screen(), Screen::SaveNaming);
    assert!(app.typing(), "keys type the name");
    assert_eq!(app.save_name_draft(), Some("seed 7 tick 12"));
    assert!(bars(&app, &world).1.contains("Save as: seed 7 tick 12_"));

    // The first letter typed replaces the name offered.
    type_text(&mut app, &world, "Bush garden");
    apply(&mut app, &world, Action::Erase);
    type_text(&mut app, &world, "ns");
    apply(&mut app, &world, Action::Enter);
    assert_eq!(app.screen(), Screen::Normal);
    assert_eq!(app.notice(), Some("Saved as Bush gardens"));
    assert!(folder.join("Bush gardens.tspr").is_file());
}

#[test]
fn a_save_needs_a_name_and_esc_gives_up() {
    let folder = scratch_folder("save-as-empty");
    let world = world();
    let mut app = app_for(&world, &folder);
    apply(&mut app, &world, Action::SaveAs);
    apply(&mut app, &world, Action::Erase);
    for _ in 0..30 {
        apply(&mut app, &world, Action::Erase);
    }
    apply(&mut app, &world, Action::Enter);
    assert_eq!(app.refusal(), Some("A save needs a name"));

    apply(&mut app, &world, Action::SaveAs);
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.screen(), Screen::Normal);
    assert!(saves::list(&folder).is_empty());
}

#[test]
fn ctrl_o_lists_the_saves_newest_first_and_picking_one_loads_it() {
    let folder = scratch_folder("open");
    let mut world = world();
    let mut app = app_for(&world, &folder);
    saves::write(&folder, "older", &world.save()).expect("written");
    // File times can be coarse: make the order plain.
    std::thread::sleep(Duration::from_millis(1100));
    run(&mut world, 6);
    apply(&mut app, &world, Action::Quicksave);
    run(&mut world, 2);

    apply(&mut app, &world, Action::OpenSaves);
    assert_eq!(app.screen(), Screen::LoadMenu);
    assert_eq!(app.menu_title(), Some(" Load "));
    assert_eq!(app.menu_items(&world), ["quicksave", "older"]);

    apply(&mut app, &world, Action::Pick(2));
    // The world has run since its save, so the player is asked.
    assert_eq!(app.screen(), Screen::LoadPrompt);
    apply(&mut app, &world, Action::Confirm);
    let loaded = app.take_loaded().expect("loaded");
    assert_eq!(loaded.tick(), 0);
}

#[test]
fn the_load_list_says_when_there_are_no_saves() {
    let folder = scratch_folder("open-empty");
    let world = world();
    let mut app = app_for(&world, &folder);
    apply(&mut app, &world, Action::OpenSaves);
    assert!(app.menu_items(&world).is_empty());
    assert_eq!(
        app.menu_empty(),
        format!("No saves in {}", folder.display())
    );
    apply(&mut app, &world, Action::Back);
    assert_eq!(app.screen(), Screen::Normal);
}

#[test]
fn a_damaged_save_is_refused_on_the_status_line() {
    let folder = scratch_folder("damaged");
    let world = world();
    let mut app = app_for(&world, &folder);
    std::fs::write(saves::path_for(&folder, QUICKSAVE), b"TSPR not really").expect("written");
    apply(&mut app, &world, Action::Quickload);
    assert_eq!(
        app.refusal(),
        Some("Couldn't load quicksave: it isn't a Terra Sprites save")
    );
    assert!(app.take_loaded().is_none());
    assert!(bars(&app, &world).1.contains("Couldn't load quicksave"));
}

#[test]
fn a_save_that_cant_be_written_is_refused_on_the_status_line() {
    let folder = scratch_folder("unwritable");
    // A file where the saves folder should be.
    let blocked = folder.join("saves");
    std::fs::write(&blocked, b"").expect("written");
    let world = world();
    let mut app = app_for(&world, &blocked);
    apply(&mut app, &world, Action::Quicksave);
    let refusal = app.refusal().expect("refused");
    assert!(refusal.starts_with("Couldn't save: "), "{refusal}");
}

#[test]
fn a_loaded_world_keeps_the_cursor_as_it_was() {
    let folder = scratch_folder("cursor");
    let mut world = world();
    let ball = world.object_at(at(5, 5)).expect("the ball").id();
    world.submit(Command::PickUp { item: ball });
    world.submit(Command::MoveCursor { tile: at(4, 6) });
    world.submit(Command::ShowCursor { visible: true });
    world.step();
    saves::write(&folder, QUICKSAVE, &world.save()).expect("written");
    let fresh = self::world();
    let mut app = app_for(&fresh, &folder);
    apply(&mut app, &fresh, Action::Quickload);
    let loaded = app.take_loaded().expect("loaded");
    assert_eq!(app.mode(), CursorMode::Grab, "the Cursor holds the ball");
    assert_eq!(app.cursor(), at(4, 6));
    assert!(app.visible());
    assert!(app.take_commands().is_empty(), "nothing to tell the world");
    assert!(loaded.cursor().holds().is_some());
}

#[test]
fn time_running_for_ten_minutes_autosaves_keeping_the_last_three() {
    let folder = scratch_folder("autosave");
    let mut world = world();
    let mut app = app_for(&world, &folder);
    let mut ticks = Vec::new();
    for _ in 0..4 {
        run(&mut world, 3);
        app.animate(AUTOSAVE_EVERY - Duration::from_secs(1));
        app.autosave_if_due(&world);
        app.animate(Duration::from_secs(1));
        app.autosave_if_due(&world);
        assert_eq!(app.notice(), Some("Autosaved"));
        ticks.push(world.tick());
    }
    let names: Vec<String> = saves::list(&folder).into_iter().map(|f| f.name).collect();
    assert_eq!(names.len(), 3, "{names:?}");
    for (n, &tick) in ticks.iter().rev().take(3).enumerate() {
        let path = saves::path_for(&folder, &saves::autosave_name(n + 1));
        let saved = World::load(&std::fs::read(path).expect("an autosave")).expect("it loads");
        assert_eq!(saved.tick(), tick, "autosave-{}", n + 1);
    }
}

#[test]
fn paused_time_doesnt_count_towards_an_autosave() {
    let folder = scratch_folder("autosave-paused");
    let mut world = world();
    let mut app = app_for(&world, &folder);
    run(&mut world, 3);
    apply(&mut app, &world, Action::TogglePause);
    app.animate(AUTOSAVE_EVERY * 2);
    app.autosave_if_due(&world);
    assert!(saves::list(&folder).is_empty());
}

#[test]
fn quitting_autosaves_unless_the_world_hasnt_run_since_its_last_save() {
    let folder = scratch_folder("quit");
    let mut world = world();
    let mut app = app_for(&world, &folder);
    app.autosave(&world);
    assert!(
        saves::list(&folder).is_empty(),
        "a new world has nothing to keep"
    );
    run(&mut world, 3);
    apply(&mut app, &world, Action::Quicksave);
    app.autosave(&world);
    assert_eq!(saves::list(&folder).len(), 1, "the quicksave holds it");
    run(&mut world, 3);
    app.autosave(&world);
    assert!(saves::path_for(&folder, "autosave-1").is_file());
}

#[test]
fn the_keys_for_saving_and_loading() {
    let mut keys = Keys::with_release_reporting(false);
    let key = |code, modifiers| KeyEvent::new(code, modifiers);
    assert_eq!(
        keys.action_for(key(KeyCode::F(5), KeyModifiers::NONE)),
        Some(Action::Quicksave)
    );
    assert_eq!(
        keys.action_for(key(KeyCode::F(9), KeyModifiers::NONE)),
        Some(Action::Quickload)
    );
    assert_eq!(
        keys.action_for(key(KeyCode::Char('s'), KeyModifiers::CONTROL)),
        Some(Action::SaveAs)
    );
    assert_eq!(
        keys.action_for(key(KeyCode::Char('o'), KeyModifiers::CONTROL)),
        Some(Action::OpenSaves)
    );
}

#[test]
fn a_save_name_becomes_a_file_name_any_system_can_hold() {
    let folder = Path::new("saves");
    let cases = [
        (" a/b:c? ", "a-b-c-"),
        // Windows drops dots and spaces at the end of a file's name.
        ("world. . ", "world"),
        ("...", "-"),
        // And keeps these names for devices, whatever follows a dot.
        ("con", "con-"),
        ("LPT1", "LPT1-"),
        ("Aux.old", "Aux-.old"),
        ("console", "console"),
    ];
    for (name, file) in cases {
        assert_eq!(saves::name_for(name), file, "{name:?}");
        assert_eq!(
            saves::path_for(folder, name),
            folder.join(format!("{file}.tspr"))
        );
    }
}

#[test]
fn a_save_is_announced_by_the_name_the_load_list_shows() {
    let folder = scratch_folder("save-as-unsafe");
    let world = world();
    let mut app = app_for(&world, &folder);
    apply(&mut app, &world, Action::SaveAs);
    type_text(&mut app, &world, "my/world:1");
    apply(&mut app, &world, Action::Enter);
    assert_eq!(app.notice(), Some("Saved as my-world-1"));
    let listed: Vec<String> = saves::list(&folder).into_iter().map(|s| s.name).collect();
    assert_eq!(listed, ["my-world-1"]);
}

#[test]
fn ctrl_c_quits_while_a_name_is_typed_with_caps_lock_on() {
    let mut keys = Keys::with_release_reporting(false);
    for c in ['c', 'C'] {
        let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
        assert_eq!(keys.typed_action(key), Some(Action::Quit), "{c}");
    }
}

#[test]
fn a_save_name_takes_only_what_the_screen_can_show() {
    let folder = scratch_folder("save-as-cp437");
    let world = world();
    let mut app = app_for(&world, &folder);
    apply(&mut app, &world, Action::SaveAs);
    type_text(&mut app, &world, "Café ☃ 1\t");
    assert_eq!(app.save_name_draft(), Some("Café  1"));
}
