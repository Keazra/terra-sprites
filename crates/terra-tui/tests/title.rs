//! The title screen (M2 design §8): the opening scene, the menu over the
//! world it wakes, New world, Load, Help and quitting.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Size};
use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{DataPack, Map, Mode, Pos, World};
use terra_tui::args::Args;
use terra_tui::cp437;
use terra_tui::input::{Action, Button};
use terra_tui::saves::SaveFile;
use terra_tui::start::{self, Preset};
use terra_tui::theme::{SemanticTile, Theme};
use terra_tui::title::{self, Choice, Title, TitleFlow};

const WIDTH: u16 = 100;
const HEIGHT: u16 = 30;
/// The first seed New world offers, in these tests.
const OFFERED: u64 = 12345;

/// A title screen on a 100×30 terminal, as the game opens it, with no
/// saves.
fn title() -> Title {
    title_on(WIDTH, HEIGHT)
}

/// A title screen on a terminal `width` by `height`, with no saves.
fn title_on(width: u16, height: u16) -> Title {
    let world = start::title_world(&Args::default(), 3, (width, height)).expect("a world");
    // On a 30-row terminal the map is 32 rows, the fewest a map may have.
    Title::new(world, Theme::cp437(), OFFERED, Size::new(width, height))
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

/// A turn of the wheel by `notches`, down being positive, with the pointer
/// over the menu.
fn wheel(notches: i32) -> Action {
    Action::Wheel {
        at: Position::new(50, 15),
        notches,
    }
}

/// Where the Cursor is on the screen, by the default theme's frame of
/// light, and what's inside the frame.
fn cursor_and_inside(buffer: &Buffer) -> Option<(Position, String)> {
    cursor_and_inside_in(buffer, &Theme::cp437())
}

/// Where the Cursor is on the screen, by `theme`'s frame of light, and
/// what's inside the frame.
fn cursor_and_inside_in(buffer: &Buffer, theme: &Theme) -> Option<(Position, String)> {
    let frame = theme.visible_frame();
    let is = |x: u16, y: u16, side: char| buffer[(x, y)].symbol() == side.to_string();
    let area = buffer.area;
    (area.top() + 1..area.bottom() - 1)
        .flat_map(|y| (area.left() + 1..area.right() - 1).map(move |x| (x, y)))
        .find(|&(x, y)| {
            is(x - 1, y, frame.left)
                && is(x + 1, y, frame.right)
                && is(x, y - 1, frame.up)
                && is(x, y + 1, frame.down)
        })
        .map(|(x, y)| (Position::new(x, y), buffer[(x, y)].symbol().to_string()))
}

/// Whether the Cursor's frame is in sight above the box at the top of
/// `screen`, with `☺` inside, saying what's wrong if not.
fn cursor_above_the_box(screen: &Buffer) -> Result<(), String> {
    let shown = text(screen);
    let Some((at, inside)) = cursor_and_inside(screen) else {
        return Err(format!("no Cursor in sight:\n{shown}"));
    };
    let Some(top) = find(screen, "╔") else {
        return Err(format!("no box:\n{shown}"));
    };
    if inside != "☺" || at.y + 1 >= top.y {
        return Err(format!("{inside} at {at:?}, the box at {top:?}:\n{shown}"));
    }
    Ok(())
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
fn the_scene_opens_dark_with_the_cursor_s_light_over_a_sprite() {
    let mut title = title();
    title.animate(Duration::from_millis(500));
    let screen = render(&title);
    let shown = text(&screen);
    assert!(
        lit_cells(&screen) < 40,
        "only the light and what's under it, and the status line:\n{shown}"
    );
    let inside = cursor_and_inside(&screen).map(|(_, inside)| inside);
    assert_eq!(inside.as_deref(), Some("☺"), "{shown}");
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
fn the_sprite_under_the_cursor_stays_still_with_its_happy_face_the_whole_time() {
    // M2 design §8.1 (Fauxpaw, 2026-10-17): it never sleeps, wakes or
    // wanders off, through the scene and for as long as the menu shows.
    let mut title = title();
    let mut shown_for = Duration::ZERO;
    let mut seen = Vec::new();
    for until in [500, 2000, 2600, 3500, 4200, 5600, 60_000] {
        while shown_for < Duration::from_millis(until) {
            title.animate(Duration::from_millis(100));
            shown_for += Duration::from_millis(100);
        }
        let screen = render(&title);
        seen.push((until, cursor_and_inside(&screen), text(&screen)));
    }
    let (_, first, shown) = &seen[0];
    let first = first.clone().expect("the Cursor's light");
    assert_eq!(first.1, "☺", "{shown}");
    for (at, cursor, shown) in &seen {
        assert_eq!(cursor.as_ref(), Some(&first), "at {at} ms:\n{shown}");
    }
}

#[test]
fn the_light_arrives_on_a_free_tile_a_sprite_could_stand_on() {
    // M2 design §8.1.
    let title = title();
    let (at, _) = cursor_and_inside(&render(&title)).expect("the Cursor's light");
    let world = title.world();
    let pos = Pos { x: at.x, y: at.y };
    assert!(world.sprite_at(pos).is_none() && world.object_at(pos).is_none());
    let terrain = world.data().terrain(world.map().terrain(pos));
    assert!(
        terrain.step_cost().is_some(),
        "{:?}",
        world.map().terrain(pos)
    );
}

#[test]
fn the_cursor_and_its_sprite_stay_clear_of_the_menu() {
    // M2 design §8.1: above the menu's box, whether or not Continue makes
    // it a row taller, and on a bigger terminal.
    let now = SystemTime::now();
    for (width, height) in [(WIDTH, HEIGHT), (140, 45)] {
        for saves in [
            vec![],
            vec![save("quicksave", now, Duration::from_secs(60))],
        ] {
            let mut title = title_on(width, height);
            title.set_saves(saves, now);
            title.animate(Duration::from_secs(10));
            cursor_above_the_box(&render_at(&title, width, height)).unwrap();
        }
    }
}

/// A title screen on a 100×30 terminal whose world has rock along its top
/// five rows, so no tile above the title's boxes is one a sprite could
/// stand on.
fn title_under_rock() -> Title {
    let rows: Vec<String> = (0..32)
        .map(|y| if y < 5 { "#" } else { "." }.repeat(100))
        .collect();
    let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
    let data = DataPack::builtin().expect("the built-in data");
    let map = Map::from_ascii(&rows, &data).expect("a map");
    let world = World::from_map(map, data, 3).expect("a world");
    Title::new(world, Theme::cp437(), OFFERED, Size::new(WIDTH, HEIGHT))
}

#[test]
fn with_no_free_tile_above_the_boxes_the_cursor_s_sprite_shows_at_their_middle() {
    // M2 design §8.1: still above every box, over the rock.
    let mut title = title_under_rock();
    title.animate(Duration::from_secs(10));
    let screen = render(&title);
    cursor_above_the_box(&screen).unwrap();
    let (at, _) = cursor_and_inside(&screen).unwrap();
    assert_eq!(at, Position::new(50, 2), "{}", text(&screen));
}

#[test]
fn new_world_and_load_leave_the_cursor_in_sight_however_tall_they_grow() {
    // M2 design §8.1: only the help screen covers it.
    let now = SystemTime::now();
    let saves: Vec<SaveFile> = (0..13)
        .map(|n| save(&format!("save-{n}"), now, Duration::from_secs(60 * n)))
        .collect();
    let presets: Vec<Preset> = (0..9)
        .map(|n| Preset {
            name: format!("preset-{n}"),
            path: Some(PathBuf::from(format!("preset-{n}.ron"))),
        })
        .collect();
    for mut title in [title(), title_under_rock()] {
        title.set_saves(saves.clone(), now);
        title.set_presets(presets.clone());
        title.animate(Duration::from_secs(10));
        for choice in [Choice::NewWorld, Choice::Load] {
            let number = title.choices().iter().position(|&c| c == choice).unwrap() + 1;
            title.apply(Action::Pick(number as u8));
            cursor_above_the_box(&render(&title)).unwrap();
            title.apply(Action::Back);
        }
    }
}

#[test]
fn a_resize_moves_the_cursor_only_when_it_would_hide_it() {
    // M2 design §8.1.
    let mut title = title();
    title.animate(Duration::from_secs(10));
    let before = cursor_and_inside(&render(&title));
    for size in [
        Size::new(120, 30),
        Size::new(80, 24),
        Size::new(WIDTH, HEIGHT),
    ] {
        title.resize(size);
    }
    assert_eq!(cursor_and_inside(&render(&title)), before, "it stays put");

    // A taller terminal's map puts it lower, and a wider one's further
    // right, where the box or the edge would hide it on a smaller one.
    let now = SystemTime::now();
    for (width, height) in [(100, 50), (200, 30)] {
        let mut title = title_on(width, height);
        title.set_saves(vec![save("quicksave", now, Duration::from_secs(60))], now);
        title.animate(Duration::from_secs(10));
        title.resize(Size::new(WIDTH, HEIGHT));
        cursor_above_the_box(&render(&title)).unwrap();
    }
}

#[test]
fn the_cursor_s_sprite_is_drawn_as_the_theme_draws_a_sprite() {
    // M2 design §8.1: `@` in `--ascii`.
    let theme = Theme::ascii();
    let world = start::title_world(&Args::default(), 3, (WIDTH, HEIGHT)).expect("a world");
    let mut title = Title::new(world, theme.clone(), OFFERED, Size::new(WIDTH, HEIGHT));
    title.animate(Duration::from_secs(10));
    let screen = render(&title);
    let inside = cursor_and_inside_in(&screen, &theme).map(|(_, inside)| inside);
    let sprite = theme.glyph(SemanticTile::Sprite).symbol.to_string();
    assert_eq!(inside, Some(sprite), "{}", text(&screen));
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
fn the_world_runs_at_4x_from_launch_while_the_light_spreads() {
    // M2 design §8.1: the light finds the sprites already about their lives.
    let mut title = title();
    for _ in 0..20 {
        title.animate(Duration::from_millis(100));
    }
    // 4× is 5 ticks a second.
    assert_eq!(title.world().tick(), 10, "running in the dark");
    for _ in 0..40 {
        title.animate(Duration::from_millis(100));
    }
    assert_eq!(title.world().tick(), 30);
}

#[test]
fn the_world_runs_untouched_by_cursor_and_inside() {
    // M2 design §8.1: the sprite in the Cursor's frame is the scene's own,
    // so the Cursor leads none of the world's, and they go about their
    // lives.
    let mut title = title();
    let at_launch: Vec<_> = title.world().sprites().map(|s| (s.id(), s.pos())).collect();
    for _ in 0..30 {
        title.animate(Duration::from_millis(100));
        assert_eq!(title.world().cursor().leads(), None);
    }
    let world = title.world();
    let moved = at_launch
        .iter()
        .filter(|&&(id, pos)| world.sprite(id).map(|s| s.pos()) != Some(pos))
        .count();
    // In 15 ticks most have stepped somewhere; a quarter is a floor well
    // under that, so this is about the world running, not how fast.
    assert!(
        moved > at_launch.len() / 4,
        "{moved} of {}",
        at_launch.len()
    );
}

#[test]
fn after_a_long_frame_the_world_runs_at_most_5_ticks_and_drops_the_rest() {
    // M2 design §8.1: at most 5 ticks a frame.
    let mut title = title();
    // Three seconds at 4× is 15 ticks.
    title.animate(Duration::from_secs(3));
    assert_eq!(title.world().tick(), 5);
    title.animate(Duration::from_millis(200));
    assert_eq!(
        title.world().tick(),
        6,
        "what the long frame owed was dropped, not caught up"
    );
}

#[test]
fn a_skip_leaves_the_world_running_and_the_cursor_s_sprite_where_it_was() {
    let mut title = title();
    title.animate(Duration::from_millis(500));
    let before = cursor_and_inside(&render(&title));
    title.apply(Action::Enter);
    title.animate(Duration::from_secs(1));
    // 2 ticks before the skip, and the 5 a frame runs at most after it.
    assert_eq!(title.world().tick(), 7);
    assert_eq!(cursor_and_inside(&render(&title)), before);
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
fn the_wheel_moves_through_the_menu() {
    // M2 design §8.2: picked as the Place menu's are, the wheel too.
    let mut title = after_the_scene();
    // New world, Load, Help, Quit: past the end, it stays on the last.
    for _ in 0..4 {
        title.apply(wheel(1));
    }
    title.apply(wheel(-1));
    assert_eq!(title.apply(Action::Enter), TitleFlow::Stay);
    assert!(text(&render(&title)).contains("EMOTES"), "Help opens");
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

#[test]
fn the_mouse_doesn_t_answer_the_quit_prompt() {
    // M2 design §8.2: `Esc` asks as in a world, where the mouse doesn't
    // answer the question; only a key does (M1 design §6.6).
    let mut title = after_the_scene();
    title.apply(Action::Back);
    let over_the_menu = Position::new(50, 15);
    let mouse = [
        Action::left_click(over_the_menu),
        Action::right_click(over_the_menu),
        Action::middle_click(over_the_menu),
        wheel(1),
    ];
    for action in mouse {
        assert_eq!(title.apply(action), TitleFlow::Stay, "{action:?}");
        let shown = text(&render(&title));
        assert!(shown.contains("Quit? (y/n)"), "still asks after {action:?}");
    }
    assert_eq!(title.apply(Action::Confirm), TitleFlow::Quit);
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
            preset: None,
            mode: Mode::Story,
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
            preset: Some("small.ron".into()),
            mode: Mode::Story,
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
            preset: None,
            mode: Mode::Story,
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
fn new_world_makes_a_story_world_unless_the_arrows_pick_lab() {
    let mut title = new_world(presets());
    let shown = text(&render(&title));
    assert!(shown.contains("Mode    [Story]  Lab "), "{shown}");
    assert!(shown.contains("4x at most"), "{shown}");
    title.apply(Action::Scroll { dx: 1, dy: 0 });
    title.apply(Action::Scroll { dx: 1, dy: 0 });
    let shown = text(&render(&title));
    assert!(shown.contains("Mode     Story  [Lab]"), "{shown}");
    assert!(shown.contains("every speed, for experiments"), "{shown}");
    assert_eq!(
        title.apply(Action::Enter),
        TitleFlow::New {
            seed: OFFERED,
            preset: None,
            mode: Mode::Lab,
        }
    );
    let mut title = new_world(presets());
    title.apply(Action::Scroll { dx: 1, dy: 0 });
    title.apply(Action::Scroll { dx: -1, dy: 0 });
    let TitleFlow::New { mode, .. } = title.apply(Action::Enter) else {
        panic!("a new world");
    };
    assert_eq!(mode, Mode::Story);
}

/// The lines New world shows while `?` is held, explaining a mode.
const STORY_EXPLAINED: [&str; 3] = [
    "Story mode: the game as it's meant to be played.",
    "· Speeds up to 4x, so a life never feels rushed.",
    "· A world keeps its mode for its whole life.",
];
const LAB_EXPLAINED: [&str; 3] = [
    "Lab mode: a world for experiments.",
    "· Every speed, up to Max; it can be fast-forwarded.",
    "· A world keeps its mode for its whole life.",
];

#[test]
fn holding_the_question_mark_explains_the_mode_picked() {
    let mut title = new_world(presets());
    assert!(text(&render(&title)).contains("hold ? what the mode means"));
    title.apply(Action::Type('?'));
    let shown = text(&render(&title));
    for line in STORY_EXPLAINED {
        assert!(shown.contains(line), "{line:?} in\n{shown}");
    }
    assert!(!shown.contains("small"), "it takes the presets' place");
    // The arrows switch the mode while it's held, and it follows.
    title.apply(Action::Scroll { dx: 1, dy: 0 });
    let shown = text(&render(&title));
    for line in LAB_EXPLAINED {
        assert!(shown.contains(line), "{line:?} in\n{shown}");
    }
    assert!(!shown.contains(STORY_EXPLAINED[0]), "{shown}");
    title.apply(Action::HelpReleased);
    let shown = text(&render(&title));
    assert!(
        !shown.contains(LAB_EXPLAINED[0]),
        "gone once let go:\n{shown}"
    );
    assert!(shown.contains("small"), "{shown}");
    assert!(shown.contains("12345"), "the seed is as it was:\n{shown}");
}

/// The New world box's top-left and bottom-right corners on `screen`.
fn new_world_box(screen: &Buffer) -> (Position, Position) {
    let top_left = find(screen, "╔═ New world").expect("the box shows");
    let shown = text(screen);
    let rows: Vec<&str> = shown.lines().collect();
    let bottom = (usize::from(top_left.y)..rows.len())
        .find(|&y| rows[y].chars().nth(usize::from(top_left.x)) == Some('╚'))
        .expect("the box's bottom");
    let right = rows[bottom]
        .chars()
        .enumerate()
        .skip(usize::from(top_left.x))
        .find(|&(_, c)| c == '╝')
        .map(|(x, _)| x)
        .expect("its bottom-right corner");
    (top_left, Position::new(right as u16, bottom as u16))
}

#[test]
fn the_new_world_box_stays_put_while_the_question_mark_is_held() {
    // M2 design §16.1: the box is tall enough for the explanation, so it
    // doesn't jump, even with fewer presets than the explanation has lines.
    let mut title = new_world(vec![]);
    let before = new_world_box(&render(&title));
    title.apply(Action::Type('?'));
    let held = render(&title);
    assert_eq!(new_world_box(&held), before);
    let shown = text(&held);
    for line in STORY_EXPLAINED {
        assert!(shown.contains(line), "{line:?} in\n{shown}");
    }
    title.apply(Action::HelpReleased);
    assert_eq!(new_world_box(&render(&title)), before);
}

#[test]
fn another_key_puts_the_explanation_away_and_does_its_job() {
    // Where the terminal never says `?` was let go of.
    let mut title = new_world(presets());
    title.apply(Action::Type('?'));
    title.apply(Action::Type('7'));
    let shown = text(&render(&title));
    assert!(!shown.contains(STORY_EXPLAINED[0]), "{shown}");
    assert!(shown.contains("7_"), "the 7 is typed:\n{shown}");
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
fn a_save_named_outside_cp437_shows_with_question_marks_as_in_a_world() {
    // M2 design §8.2: everything drawn stays within CP437, and Load lists
    // the saves as `Ctrl+O` does in a world, which shows such a name so.
    let (mut title, _) = with_saves(&[("日記 mira", 2)]);
    let menu = render(&title);
    let shown = text(&menu);
    assert!(
        shown.contains("?? mira · 2 hours ago"),
        "Continue:\n{shown}"
    );
    title.apply(Action::Pick(3));
    let load = render(&title);
    let shown = text(&load);
    assert!(shown.contains("1 ?? mira"), "Load:\n{shown}");
    for screen in [&menu, &load] {
        for cell in screen.content() {
            let c = cell.symbol().chars().next().unwrap_or(' ');
            assert!(cp437::contains(c), "{c:?} isn't CP437:\n{}", text(screen));
        }
    }
}

#[test]
fn load_with_no_saves_says_where_it_looked_and_esc_goes_back() {
    // As `Ctrl+O`'s list says it in a world (M2 design §8.2).
    let mut title = after_the_scene();
    let folder = PathBuf::from("/home/someone/.local/share/terra-sprites/saves");
    title.set_save_folder(folder.clone());
    title.apply(Action::Pick(2));
    let shown = text(&render(&title));
    let said = format!("No saves in {}", folder.display());
    assert!(shown.contains(&said), "{said:?} in\n{shown}");
    title.apply(Action::Back);
    assert_eq!(title.apply(Action::Pick(4)), TitleFlow::Quit);

    let mut title = after_the_scene();
    title.apply(Action::Pick(2));
    let shown = text(&render(&title));
    assert!(shown.contains("No folder for saves"), "{shown}");
}

/// The title screen with Load open on three saves, and the saves.
fn loading_three() -> (Title, Vec<SaveFile>) {
    let (mut title, saves) = with_saves(&[
        ("autosave-1", 2),
        ("quicksave", 5),
        ("before the flood", 26),
    ]);
    // Continue is 1, so Load is 3.
    title.apply(Action::Pick(3));
    (title, saves)
}

#[test]
fn the_wheel_moves_through_load_as_it_does_through_ctrl_o_s_list() {
    // M2 design §8.2: Load is the list `Ctrl+O` opens in a world.
    let (mut title, saves) = loading_three();
    title.apply(wheel(1));
    title.apply(wheel(1));
    title.apply(wheel(1));
    title.apply(wheel(-1));
    assert_eq!(
        title.apply(Action::Enter),
        TitleFlow::Load(saves[1].clone())
    );
}

#[test]
fn a_click_on_a_save_in_load_loads_it_and_a_click_off_the_list_closes_it() {
    let (mut title, saves) = loading_three();
    let at = find(&render(&title), "before the flood").expect("listed");
    assert_eq!(
        title.apply(Action::left_click(at)),
        TitleFlow::Load(saves[2].clone())
    );
    assert_eq!(
        title.apply(Action::left_click(Position::new(0, 0))),
        TitleFlow::Stay
    );
    let shown = text(&render(&title));
    assert!(!shown.contains("before the flood"), "closed:\n{shown}");
    assert!(shown.contains("1 Continue"), "back on the menu:\n{shown}");
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
fn a_terminal_too_small_takes_no_clicks_or_turns_of_the_wheel() {
    // M2 design §8.2: as a world's too-small screen takes none (design
    // §6.1), the menu hidden behind the message can't be picked from.
    let mut title = after_the_scene();
    title.resize(Size::new(80, 24));
    for y in 0..24 {
        for x in 0..80 {
            let click = Action::left_click(Position::new(x, y));
            assert_eq!(title.apply(click), TitleFlow::Stay, "a click on {x},{y}");
        }
    }
    for _ in 0..3 {
        title.apply(wheel(1));
    }
    title.resize(Size::new(WIDTH, HEIGHT));
    let shown = text(&render(&title));
    assert!(
        shown.contains("4 Quit") && !shown.contains("Seed"),
        "the menu, with nothing opened from it:\n{shown}"
    );
    let TitleFlow::Stay = title.apply(Action::Enter) else {
        panic!("the wheel moved the highlight off New world");
    };
    assert!(title.typing(), "New world is still the one highlighted");
}

#[test]
fn esc_still_asks_to_quit_on_a_terminal_too_small() {
    // M2 design §8.2.
    let mut title = after_the_scene();
    title.resize(Size::new(80, 24));
    assert_eq!(title.apply(Action::Back), TitleFlow::Stay);
    let shown = text(&render_at(&title, 80, 24));
    assert!(shown.contains("Terminal too small"), "{shown}");
    assert!(
        shown.contains("Quit? (y/n)"),
        "the question shows:\n{shown}"
    );
    assert_eq!(title.apply(Action::Confirm), TitleFlow::Quit);
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
    title.apply(Action::Type('?'));
    screens.push(render(&title));
    title.apply(Action::Scroll { dx: 1, dy: 0 });
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
