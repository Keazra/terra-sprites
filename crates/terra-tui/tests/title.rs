//! The title screen (M2 design §8): the opening scene, the menu over the
//! world it wakes, New world, Load, Help and quitting.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::{Terminal, backend::TestBackend};
use terra_tui::args::Args;
use terra_tui::cp437;
use terra_tui::input::{Action, Button};
use terra_tui::saves::SaveFile;
use terra_tui::start::{self, Preset};
use terra_tui::theme::Theme;
use terra_tui::title::{self, Choice, Title, TitleFlow};

const WIDTH: u16 = 100;
const HEIGHT: u16 = 30;
/// The first seed New world offers, in these tests.
const OFFERED: u64 = 12345;

/// A title screen on a 100×30 terminal, as the game opens it, with no
/// saves.
fn title() -> Title {
    let world = start::title_world(&Args::default(), 3, (WIDTH, HEIGHT)).expect("a world");
    let mut title = Title::new(world, Theme::cp437(), OFFERED);
    // The map is 32 rows, the fewest a map may have, on a 30-row terminal.
    title.resize(Size::new(WIDTH, HEIGHT));
    title
}

/// A title screen whose opening scene has played.
fn after_the_scene() -> Title {
    let mut title = title();
    title.animate(Duration::from_secs(10));
    title
}

fn render_at(title: &Title, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| title::render(frame, title)).unwrap();
    terminal.backend().buffer().clone()
}

fn render(title: &Title) -> Buffer {
    render_at(title, WIDTH, HEIGHT)
}

fn text(buffer: &Buffer) -> String {
    let area = buffer.area;
    (area.top()..area.bottom())
        .map(|y| {
            (area.left()..area.right())
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Where `needle` starts on the screen.
fn find(buffer: &Buffer, needle: &str) -> Option<Position> {
    text(buffer).lines().enumerate().find_map(|(y, line)| {
        let chars: Vec<char> = line.chars().collect();
        let wanted: Vec<char> = needle.chars().collect();
        (0..chars.len().saturating_sub(wanted.len() - 1))
            .find(|&x| chars[x..x + wanted.len()] == wanted[..])
            .map(|x| Position::new(x as u16, y as u16))
    })
}

fn lit_cells(buffer: &Buffer) -> usize {
    buffer
        .content()
        .iter()
        .filter(|cell| cell.symbol() != " " && cell.symbol() != "░")
        .count()
}

/// A save in the saves list, saved `ago` before `now`.
fn save(name: &str, now: SystemTime, ago: Duration) -> SaveFile {
    SaveFile {
        name: name.into(),
        path: PathBuf::from(format!("{name}.tspr")),
        modified: Some(now - ago),
    }
}

fn with_saves(names: &[(&str, u64)]) -> (Title, Vec<SaveFile>) {
    let now = SystemTime::now();
    let saves: Vec<SaveFile> = names
        .iter()
        .map(|&(name, hours)| save(name, now, Duration::from_secs(hours * 3600)))
        .collect();
    let mut title = after_the_scene();
    title.set_saves(saves.clone(), now);
    (title, saves)
}

#[test]
fn the_scene_opens_dark_with_the_cursor_s_light_over_a_sleeping_sprite() {
    let mut title = title();
    title.animate(Duration::from_millis(500));
    let screen = render(&title);
    let shown = text(&screen);
    assert!(
        lit_cells(&screen) < 40,
        "only the light and what's under it, and the status line:\n{shown}"
    );
    assert!(shown.contains('z'), "the sprite sleeps:\n{shown}");
    assert!(shown.contains('═') && shown.contains('║'), "{shown}");
    assert!(!shown.contains("New world"), "no menu yet");
}

#[test]
fn the_light_spreads_until_the_terrarium_fills_the_screen() {
    let mut title = title();
    title.animate(Duration::from_millis(1500));
    let spreading = lit_cells(&render(&title));
    // Just before the title starts to fade in over it.
    title.animate(Duration::from_millis(2400));
    let lit = render(&title);
    assert!(spreading < lit_cells(&lit), "{spreading}");
    let blank = lit
        .content()
        .iter()
        .filter(|cell| cell.symbol() == " " || cell.symbol() == "░")
        .count();
    assert_eq!(blank, 0, "{}", text(&lit));
}

#[test]
fn the_sprite_wakes_pleased_as_the_light_spreads() {
    let mut title = title();
    title.animate(Duration::from_millis(3000));
    assert!(text(&render(&title)).contains('♥'));
}

#[test]
fn the_title_and_the_menu_appear_once_the_scene_ends() {
    let mut title = title();
    title.animate(Duration::from_millis(4500));
    assert!(!title.scene_over());
    assert!(!text(&render(&title)).contains("New world"));
    title.animate(Duration::from_millis(1500));
    assert!(title.scene_over());
    let shown = text(&render(&title));
    for item in ["1 New world", "2 Load", "3 Help", "4 Quit", "▀█▀ █▀▀ █▀█"] {
        assert!(shown.contains(item), "{item} in\n{shown}");
    }
    assert_eq!(
        title.choices(),
        [Choice::NewWorld, Choice::Load, Choice::Help, Choice::Quit]
    );
}

#[test]
fn any_key_or_click_skips_the_scene_and_does_nothing_else() {
    let mut title = title();
    title.animate(Duration::from_millis(100));
    // 4 is Quit's number, once the menu shows.
    assert_eq!(title.apply(Action::Pick(4)), TitleFlow::Stay);
    assert!(title.scene_over());
    assert_eq!(title.apply(Action::Pick(4)), TitleFlow::Quit);

    let mut title = title_fresh_clicked();
    assert!(title.scene_over());
    assert_eq!(title.apply(Action::Pick(4)), TitleFlow::Quit);
}

fn title_fresh_clicked() -> Title {
    let mut title = title();
    let click = Action::Click {
        at: Position::new(3, 3),
        button: Button::Left,
        amplified: false,
    };
    // Pointing isn't a key or a click.
    assert_eq!(
        title.apply(Action::Point(Position::new(3, 3))),
        TitleFlow::Stay
    );
    assert!(!title.scene_over());
    assert_eq!(title.apply(click), TitleFlow::Stay);
    title
}

#[test]
fn the_world_holds_still_until_the_light_has_spread_then_runs_at_4x() {
    let mut title = title();
    for _ in 0..40 {
        title.animate(Duration::from_millis(100));
    }
    assert_eq!(title.world().tick(), 0, "still while the light spreads");
    for _ in 0..20 {
        title.animate(Duration::from_millis(100));
    }
    // 4× is 5 ticks a second.
    assert_eq!(title.world().tick(), 10);
}

#[test]
fn a_skipped_scene_s_world_runs_from_the_skip() {
    let mut title = title();
    title.apply(Action::Enter);
    title.animate(Duration::from_secs(1));
    assert_eq!(title.world().tick(), 5);
}

#[test]
fn continue_names_the_newest_save_and_loads_it() {
    let (mut title, saves) = with_saves(&[("autosave-1", 2), ("quicksave", 30)]);
    assert_eq!(title.choices()[0], Choice::Continue);
    let shown = text(&render(&title));
    assert!(shown.contains("1 Continue"), "{shown}");
    assert!(shown.contains("autosave-1 · 2 hours ago"), "{shown}");
    assert_eq!(
        title.apply(Action::Enter),
        TitleFlow::Load(saves[0].clone())
    );
}

#[test]
fn without_a_save_there_is_no_continue() {
    let (title, _) = with_saves(&[]);
    assert!(!title.choices().contains(&Choice::Continue));
    assert!(!text(&render(&title)).contains("Continue"));
}

#[test]
fn the_arrows_move_through_the_menu_and_enter_picks() {
    let mut title = after_the_scene();
    let down = Action::Scroll { dx: 0, dy: 1 };
    title.apply(down);
    title.apply(down);
    title.apply(down);
    // Past the end, it stays on the last.
    title.apply(down);
    assert_eq!(title.apply(Action::Enter), TitleFlow::Quit);
}

#[test]
fn a_click_on_a_choice_picks_it() {
    let mut title = after_the_scene();
    let at = find(&render(&title), "Quit").expect("Quit shows");
    let click = Action::Click {
        at,
        button: Button::Left,
        amplified: false,
    };
    assert_eq!(title.apply(click), TitleFlow::Quit);
}

#[test]
fn a_click_finds_the_menu_where_it_s_drawn_after_the_terminal_grows() {
    let mut title = after_the_scene();
    title.resize(Size::new(140, 45));
    let at = find(&render_at(&title, 140, 45), "Quit").expect("Quit shows");
    let click = Action::Click {
        at,
        button: Button::Left,
        amplified: false,
    };
    assert_eq!(title.apply(click), TitleFlow::Quit);
}

#[test]
fn esc_asks_before_quitting_and_ctrl_c_quits_at_once() {
    let mut title = after_the_scene();
    assert_eq!(title.apply(Action::Back), TitleFlow::Stay);
    assert!(text(&render(&title)).contains("Quit? (y/n)"));
    // Any other key cancels, `Esc` included.
    assert_eq!(title.apply(Action::Back), TitleFlow::Stay);
    assert!(!text(&render(&title)).contains("Quit? (y/n)"));
    title.apply(Action::Back);
    assert_eq!(title.apply(Action::Confirm), TitleFlow::Quit);

    let mut title = after_the_scene();
    assert_eq!(title.apply(Action::Quit), TitleFlow::Quit);
}

/// The title screen with New world open, offering `presets`.
fn new_world(presets: Vec<Preset>) -> Title {
    let mut title = after_the_scene();
    title.set_presets(presets);
    title.apply(Action::Pick(1));
    title
}

fn presets() -> Vec<Preset> {
    vec![
        Preset {
            name: "default".into(),
            path: None,
        },
        Preset {
            name: "small".into(),
            path: Some("small.ron".into()),
        },
    ]
}

#[test]
fn new_world_offers_the_default_when_told_of_no_presets() {
    let mut title = new_world(vec![]);
    let shown = text(&render(&title));
    assert!(shown.contains("default"), "{shown}");
    assert_eq!(
        title.apply(Action::Enter),
        TitleFlow::New {
            seed: OFFERED,
            preset: None
        }
    );
}

#[test]
fn new_world_offers_a_random_seed_and_the_presets() {
    let title = new_world(presets());
    assert!(title.typing(), "keys type the seed");
    let shown = text(&render(&title));
    assert!(shown.contains("New world"), "{shown}");
    assert!(shown.contains("12345"), "the seed offered:\n{shown}");
    assert!(
        shown.contains("default") && shown.contains("small"),
        "{shown}"
    );
}

#[test]
fn new_world_starts_from_the_seed_typed_and_the_preset_picked() {
    let mut title = new_world(presets());
    // Typing replaces the seed offered, and only digits type.
    for c in ['4', 'x', '2', '7'] {
        title.apply(Action::Type(c));
    }
    title.apply(Action::Erase);
    assert!(text(&render(&title)).contains("42"));
    title.apply(Action::Scroll { dx: 0, dy: 1 });
    assert_eq!(
        title.apply(Action::Enter),
        TitleFlow::New {
            seed: 42,
            preset: Some("small.ron".into())
        }
    );
}

#[test]
fn new_world_starts_from_the_seed_offered_with_the_default_preset() {
    let mut title = new_world(presets());
    assert_eq!(
        title.apply(Action::Enter),
        TitleFlow::New {
            seed: OFFERED,
            preset: None
        }
    );
}

#[test]
fn tab_offers_another_random_seed() {
    let mut title = new_world(presets());
    title.apply(Action::AnotherName);
    let TitleFlow::New { seed, .. } = title.apply(Action::Enter) else {
        panic!("a new world");
    };
    assert_ne!(seed, OFFERED);
}

#[test]
fn a_new_world_needs_a_seed_that_fits() {
    let mut title = new_world(presets());
    for _ in 0..5 {
        title.apply(Action::Erase);
    }
    assert_eq!(title.apply(Action::Enter), TitleFlow::Stay);
    assert!(
        text(&render(&title)).contains("Type a seed, or Tab for a random one"),
        "{}",
        text(&render(&title))
    );
    // u64::MAX is 18446744073709551615: one more doesn't fit.
    for c in "18446744073709551616".chars() {
        title.apply(Action::Type(c));
    }
    assert_eq!(title.apply(Action::Enter), TitleFlow::Stay);
    let shown = text(&render(&title));
    assert!(shown.contains("A seed is a whole number"), "{shown}");
}

#[test]
fn esc_leaves_new_world_for_the_menu() {
    let mut title = new_world(presets());
    title.apply(Action::Back);
    assert!(!title.typing());
    assert!(!text(&render(&title)).contains("12345"));
    assert_eq!(title.apply(Action::Pick(4)), TitleFlow::Quit);
}

#[test]
fn load_lists_every_save_newest_first_and_loads_the_one_picked() {
    let (mut title, saves) = with_saves(&[("autosave-1", 2), ("before the flood", 26)]);
    // Continue is 1, so Load is 3.
    title.apply(Action::Pick(3));
    let shown = text(&render(&title));
    assert!(shown.contains("1 autosave-1"), "{shown}");
    assert!(shown.contains("2 before the flood"), "{shown}");
    assert!(shown.contains("1 day ago"), "{shown}");
    assert_eq!(
        title.apply(Action::Pick(2)),
        TitleFlow::Load(saves[1].clone())
    );
}

#[test]
fn load_with_no_saves_says_so_and_esc_goes_back() {
    let mut title = after_the_scene();
    title.apply(Action::Pick(2));
    assert!(text(&render(&title)).contains("No saves yet"));
    title.apply(Action::Back);
    assert_eq!(title.apply(Action::Pick(4)), TitleFlow::Quit);
}

#[test]
fn help_opens_the_help_screen_and_esc_closes_it() {
    let mut title = after_the_scene();
    title.apply(Action::Pick(3));
    let shown = text(&render(&title));
    assert!(
        shown.contains("Help") && shown.contains("EMOTES"),
        "{shown}"
    );
    title.apply(Action::Back);
    assert!(!text(&render(&title)).contains("EMOTES"));
}

#[test]
fn a_refusal_shows_on_the_status_line() {
    let mut title = after_the_scene();
    title.refuse("Couldn't load autosave-1: it isn't a save".into());
    let shown = text(&render(&title));
    assert!(
        shown.contains("Couldn't load autosave-1: it isn't a save"),
        "{shown}"
    );
}

#[test]
fn a_terminal_too_small_says_so() {
    let title = after_the_scene();
    let shown = text(&render_at(&title, 80, 24));
    assert!(shown.contains("Terminal too small"), "{shown}");
    assert!(shown.contains("needs 100x30, this is 80x24"), "{shown}");
}

#[test]
fn everything_drawn_is_cp437() {
    let (mut title, _) = with_saves(&[("autosave-1", 2)]);
    title.set_presets(presets());
    let mut screens = vec![];
    let mut scene = self::title();
    for millis in [300, 1500, 1500, 1500, 2000] {
        scene.animate(Duration::from_millis(millis));
        screens.push(render(&scene));
    }
    screens.push(render(&title));
    title.apply(Action::Pick(2));
    screens.push(render(&title));
    title.apply(Action::Back);
    title.apply(Action::Pick(3));
    screens.push(render(&title));
    for screen in &screens {
        for cell in screen.content() {
            let c = cell.symbol().chars().next().unwrap();
            assert!(cp437::contains(c), "{c:?} isn't CP437:\n{}", text(screen));
        }
    }
}
