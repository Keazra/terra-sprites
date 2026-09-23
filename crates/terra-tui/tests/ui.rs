use ratatui::{Terminal, backend::TestBackend};
use terra_sim::{DataPack, World};
use terra_tui::{clock::Clock, ui};

fn world() -> World {
    World::new(DataPack::builtin().expect("built-in data pack is valid"), 7)
}

/// Renders one frame and returns the top line as text.
fn top_bar(world: &World, clock: &Clock) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| ui::render(frame, world, clock))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.width)
        .map(|x| buffer[(x, 0)].symbol())
        .collect()
}

#[test]
fn the_top_bar_shows_the_tick_and_speed() {
    let bar = top_bar(&world(), &Clock::new());
    assert!(bar.contains("Terra Sprites"), "{bar}");
    assert!(bar.contains("tick 0"), "{bar}");
    assert!(bar.contains("► 1x"), "{bar}");
}

#[test]
fn the_top_bar_groups_tick_digits_in_thousands() {
    let mut world = world();
    for _ in 0..1_234 {
        world.step();
    }
    let bar = top_bar(&world, &Clock::new());
    assert!(bar.contains("tick 1,234"), "{bar}");
}

#[test]
fn the_top_bar_shows_when_time_is_paused() {
    let mut clock = Clock::new();
    clock.toggle_pause();
    let bar = top_bar(&world(), &clock);
    assert!(bar.contains("|| paused"), "{bar}");
    assert!(!bar.contains('►'), "{bar}");
}
