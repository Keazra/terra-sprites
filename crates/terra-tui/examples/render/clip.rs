//! A clip: the game's screens, drawn frame by frame through its own UI
//! code into buffers, as its tests draw them (#241).

use std::cell::Cell;
use std::time::Duration;

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Size;
use ratatui::{Frame as Drawing, Terminal};
use terra_sim::{EntityId, Mode, Playback, World};
use terra_tui::app::{App, Selection, Tab, Ticks};
use terra_tui::clock::{Clock, Speed};
use terra_tui::input::Action;
use terra_tui::session::Session;
use terra_tui::theme::Theme;
use terra_tui::title::{self, Title};
use terra_tui::ui;

use crate::options::{End, Pace, Track, WorldScene};

/// A screen of the clip, and how long it shows.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub screen: Buffer,
    pub shows_for: Duration,
}

/// The game's screens in order, as it drew them.
#[derive(Debug, Clone, Default)]
pub struct Clip {
    pub frames: Vec<Frame>,
}

impl Clip {
    /// Adds `screen`, shown for `shows_for`: to the last frame, if it looks
    /// the same, so a still stretch is a single frame.
    fn push(&mut self, screen: &Buffer, shows_for: Duration) {
        match self.frames.last_mut() {
            Some(last) if last.screen == *screen => last.shows_for += shows_for,
            _ => self.frames.push(Frame {
                screen: screen.clone(),
                shows_for,
            }),
        }
    }

    /// How long the clip runs.
    pub fn length(&self) -> Duration {
        self.frames.iter().map(|frame| frame.shows_for).sum()
    }
}

/// How a clip is framed: the screen's size in cells, whether it's the map
/// view alone, and how long each frame shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Framing {
    pub screen: Size,
    pub map_view_alone: bool,
    pub frame: Duration,
}

/// Where a world's clip comes from.
pub enum Source {
    /// A world made from a seed.
    World(World),
    /// A replay played back, and its file's name.
    Replay { name: String, playback: Playback },
}

impl Source {
    /// The world, as it stands.
    pub fn world(&self) -> &World {
        match self {
            Source::World(world) => world,
            Source::Replay { playback, .. } => playback.world(),
        }
    }
}

/// How long the opening scene's clip shows the menu, once the scene is
/// over, unless told how long the clip is.
const MENU_SHOWS_FOR: Duration = Duration::from_secs(2);

/// The longest the app's notices are waited out before the first frame,
/// as a player pressing a key a while before would have.
const MOST_SETTLING: Duration = Duration::from_secs(10);

/// The title screen's opening scene (M2 design §8.1) on `world`, with
/// `seed` as New world's seed: for `seconds`, or until the menu has
/// shown for a while.
pub fn title_clip(
    world: World,
    theme: Theme,
    seed: u64,
    seconds: Option<Duration>,
    framing: &Framing,
) -> Clip {
    let mut title = Title::new(world, theme, seed);
    title.resize(framing.screen);
    let mut terminal = terminal(framing.screen);
    let mut clip = Clip::default();
    let (mut shown, mut menu_shown) = (Duration::ZERO, Duration::ZERO);
    loop {
        let screen = draw(&mut terminal, |frame| title::render(frame, &title));
        clip.push(&screen, framing.frame);
        let done = match seconds {
            Some(seconds) => shown + framing.frame >= seconds,
            None => menu_shown >= MENU_SHOWS_FOR,
        };
        if done {
            return clip;
        }
        title.animate(framing.frame);
        shown += framing.frame;
        if title.scene_over() {
            menu_shown += framing.frame;
        }
    }
}

/// A world's clip, from `source`: run unseen to the scene's first tick,
/// then drawn frame by frame to its last, the world running as the game's
/// clock runs it at the scene's speed.
pub fn world_clip(
    source: Source,
    theme: Theme,
    scene: &WorldScene,
    framing: &Framing,
) -> Result<Clip, String> {
    // A replay starts where its recording did, from a save perhaps.
    let start = source.world().tick();
    let from = scene.from.unwrap_or(start);
    if from < start {
        return Err(format!(
            "the replay starts at tick {start}, after --from {from}"
        ));
    }
    let (mut session, replay) = match source {
        Source::World(world) => (Session::Live(Box::new(world)), None),
        Source::Replay { name, playback } => (Session::replay(playback), Some(name)),
    };
    while session.world().tick() < from {
        session.step();
    }
    let to = match scene.end {
        End::At(to) => to,
        End::Start => from,
        End::Usual => match session.playback() {
            Some(playback) => playback.end().max(from),
            None => from + crate::options::DEFAULT_TICKS,
        },
    };
    let world = session.world();
    // The whole screen, or the map view alone.
    let (areas, render): (_, fn(&mut Drawing, &App, &World)) = if framing.map_view_alone {
        (
            ui::map_view_alone_areas(framing.screen),
            ui::render_map_view_alone,
        )
    } else {
        (ui::areas(framing.screen), ui::render)
    };
    let mut app = App::new(world.map(), theme, world.seed(), areas);
    if let (Some(name), Some(playback)) = (&replay, session.playback()) {
        app.start_replay(name, playback);
        // It starts paused, waiting for the player.
        app.clock.toggle_pause();
    }
    set_pace(&mut app.clock, scene.pace, world.mode())?;
    set_view(&mut app, world, scene)?;
    settle(&mut app, framing.frame);

    let mut terminal = terminal(framing.screen);
    let mut clip = Clip::default();
    // At Max, the thousandths of a tick the clip owes, from frame to frame.
    let mut thousandths_owed = 0;
    loop {
        let world = session.world();
        let screen = draw(&mut terminal, |frame| render(frame, &app, world));
        clip.push(&screen, framing.frame);
        // A replay's end, or where it parts from the recording, pauses it.
        if world.tick() >= to || app.clock.is_paused() {
            return Ok(clip);
        }
        app.animate(framing.frame);
        // At Max, a set number of ticks a second, so the clip is the same
        // each time.
        let most_ticks = match scene.pace {
            Pace::Game(_) => u64::MAX,
            Pace::Max(rate) => {
                thousandths_owed += framing.frame.as_millis() as u64 * u64::from(rate);
                let whole = thousandths_owed / 1000;
                thousandths_owed %= 1000;
                whole
            }
        };
        let mut ticks = Ticks::default();
        let (tick, ran) = (Cell::new(world.tick()), Cell::new(0));
        app.clock.advance(
            framing.frame,
            || {
                let events = session.step();
                ticks.note(events, session.world());
                tick.set(session.world().tick());
                ran.set(ran.get() + 1);
            },
            || tick.get() >= to || ran.get() >= most_ticks,
        );
        app.take_in(ticks, session.world());
        if let Some(playback) = session.playback() {
            app.take_in_replay(playback);
        }
    }
}

/// Sets `clock` to `pace`, as fast as a world in `mode` may go.
fn set_pace(clock: &mut Clock, pace: Pace, mode: Mode) -> Result<(), String> {
    let speed = match pace {
        Pace::Game(speed) => speed,
        Pace::Max(_) => Speed::Max,
    };
    let fastest = Speed::fastest_in(mode);
    clock.set_fastest(fastest);
    while clock.speed() < speed && !clock.faster() {}
    while clock.speed() > speed {
        clock.slower();
    }
    if clock.speed() == speed {
        return Ok(());
    }
    Err(format!(
        "a Story world runs at {} at most, not {}: a Lab world (--lab) runs at every speed",
        fastest.label(),
        speed.label()
    ))
}

/// Selects the sprite the view tracks and tracks it, with the Cursor
/// following it or out of the picture, and opens the tab asked for.
fn set_view(app: &mut App, world: &World, scene: &WorldScene) -> Result<(), String> {
    let chosen = match scene.track {
        Track::Nothing => None,
        Track::First => {
            app.apply(Action::SelectNext, world);
            app.selection().map(Selection::id)
        }
        Track::Sprite(id) => Some(select(app, world, id)?),
    };
    if chosen.is_some() {
        app.apply(Action::Track, world);
    }
    if scene.cursor && chosen.is_some() {
        app.apply(Action::Follow { at: None }, world);
    } else {
        app.withdraw_cursor();
    }
    if let Some(tab) = scene.tab {
        for _ in 0..Tab::ALL.len() {
            if app.tab() == tab {
                break;
            }
            app.apply(Action::NextTab, world);
        }
    }
    Ok(())
}

/// Selects sprite `id` as `Tab` would, going round the sprites.
fn select(app: &mut App, world: &World, id: EntityId) -> Result<EntityId, String> {
    for _ in 0..world.sprites().count() {
        app.apply(Action::SelectNext, world);
        if app.selection() == Some(Selection::Living(id)) {
            return Ok(id);
        }
    }
    Err(format!(
        "there's no sprite #{} at tick {} to track",
        id.0,
        world.tick()
    ))
}

/// Lets the app's time pass until what the status line says of setting the
/// view up has gone, as if the player had done it a while before.
fn settle(app: &mut App, frame: Duration) {
    let mut waited = Duration::ZERO;
    while (app.notice().is_some() || app.refusal().is_some()) && waited < MOST_SETTLING {
        app.animate(frame);
        waited += frame;
    }
}

/// An off-screen terminal of `screen` cells.
fn terminal(screen: Size) -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(screen.width, screen.height)).expect("a test terminal")
}

/// Draws a frame with `render`, giving the screen.
fn draw(terminal: &mut Terminal<TestBackend>, render: impl FnOnce(&mut Drawing)) -> Buffer {
    terminal.draw(render).expect("a test terminal draws");
    terminal.backend().buffer().clone()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui::buffer::Buffer;
    use ratatui::layout::Size;
    use terra_sim::{DataPack, Map, Pos, Scenario, ScriptedAction, World};
    use terra_tui::args::Args;
    use terra_tui::clock::Speed;
    use terra_tui::start;
    use terra_tui::theme::Theme;

    use super::*;
    use crate::options::{End, Pace, Track, WorldScene};

    fn pack() -> DataPack {
        DataPack::builtin().expect("built-in data pack is valid")
    }

    /// A 40×20 field of grass with a rock at every fifth tile of every
    /// fifth row, so a moving view shows, and starter sprites on
    /// `sprites`, wandering as scripted.
    fn field(sprites: &[Pos], scripted: &[(Pos, ScriptedAction)]) -> World {
        let rows: Vec<String> = (0..20)
            .map(|y| {
                (0..40)
                    .map(|x| if x % 5 == 2 && y % 5 == 2 { '#' } else { '.' })
                    .collect()
            })
            .collect();
        let rows: Vec<&str> = rows.iter().map(String::as_str).collect();
        let map = Map::from_ascii(&rows, &pack()).expect("valid drawing");
        let sprites: Vec<(Pos, _)> = sprites.iter().map(|&pos| (pos, None)).collect();
        let scenario = Scenario {
            map,
            objects: &[],
            sprites: &sprites,
            scripted,
            newborns: &[],
        };
        World::from_scenario(scenario, pack(), 7).expect("valid scenario")
    }

    fn scene(from: u64, to: u64, pace: Pace, track: Track) -> WorldScene {
        WorldScene {
            from: Some(from),
            end: End::At(to),
            pace,
            track,
            cursor: false,
            tab: None,
        }
    }

    fn framing(screen: Size, map_view_alone: bool) -> Framing {
        Framing {
            screen,
            map_view_alone,
            frame: Duration::from_millis(100),
        }
    }

    fn row(screen: &Buffer, y: u16) -> String {
        (0..screen.area.width)
            .map(|x| screen[(x, y)].symbol())
            .collect()
    }

    fn live(world: World) -> Source {
        Source::World(world)
    }

    #[test]
    fn a_world_s_clip_runs_unseen_to_its_first_tick_then_shows_each_to_its_last() {
        let world = field(&[Pos { x: 20, y: 10 }], &[]);
        let scene = scene(3, 6, Pace::Game(Speed::X4), Track::First);
        let clip = world_clip(
            live(world),
            Theme::cp437(),
            &scene,
            &framing(Size::new(100, 30), false),
        )
        .unwrap();
        let top_bar = |frame: &Frame| row(&frame.screen, 0);
        let first = top_bar(&clip.frames[0]);
        let last = top_bar(clip.frames.last().unwrap());
        assert!(
            first.starts_with(" Terra Sprites │ tick 3 │ ► 4x │ seed 7"),
            "{first}"
        );
        assert!(last.starts_with(" Terra Sprites │ tick 6 │"), "{last}");
        assert!(
            clip.frames
                .iter()
                .all(|frame| frame.screen.area.as_size() == Size::new(100, 30))
        );
    }

    #[test]
    fn a_replay_from_a_save_starts_where_its_recording_did() {
        // A recording begun at tick 5, as a session log is after a load,
        // starts from a snapshot there: its clip starts there too, unless
        // told, and runs to its end (#241).
        let mut world = field(&[Pos { x: 20, y: 10 }], &[]);
        for _ in 0..5 {
            world.step();
        }
        world.start_recording();
        for _ in 0..10 {
            world.step();
        }
        let playback = Playback::new(&world.replay().expect("recording")).expect("it plays");
        let scene = WorldScene {
            from: None,
            end: End::Usual,
            pace: Pace::Game(Speed::X4),
            track: Track::First,
            cursor: false,
            tab: None,
        };
        let source = Source::Replay {
            name: "last_session.replay".into(),
            playback,
        };
        let clip = world_clip(
            source,
            Theme::cp437(),
            &scene,
            &framing(Size::new(100, 30), false),
        )
        .unwrap();
        let first = row(&clip.frames[0].screen, 0);
        let last = row(&clip.frames.last().unwrap().screen, 0);
        assert!(first.contains("tick 5 "), "{first}");
        assert!(last.contains("tick 15 "), "{last}");
    }

    #[test]
    fn frames_that_look_the_same_are_one_frame_shown_for_longer() {
        // An empty field, alone: nothing changes as two ticks run at 1×,
        // 1.25 ticks a second, so the frames drawn every tenth of a second
        // from 0 to 1.6 seconds, when the second tick runs, are all the same.
        let world = field(&[], &[]);
        let scene = scene(0, 2, Pace::Game(Speed::X1), Track::First);
        let clip = world_clip(
            live(world),
            Theme::cp437(),
            &scene,
            &framing(Size::new(20, 10), true),
        )
        .unwrap();
        assert_eq!(clip.frames.len(), 1);
        assert_eq!(clip.frames[0].shows_for, Duration::from_millis(1700));
        assert_eq!(clip.length(), Duration::from_millis(1700));
    }

    #[test]
    fn the_view_tracks_the_chosen_sprite() {
        // Two sprites; the second walks off east. The map view alone, 21×11, has
        // it in the middle, selected (☻), as the rocks go by.
        let (stays, walks) = (Pos { x: 12, y: 6 }, Pos { x: 14, y: 10 });
        let world = field(
            &[stays, walks],
            &[(
                walks,
                ScriptedAction::Wander {
                    destination: Pos { x: 30, y: 10 },
                },
            )],
        );
        let id = world.sprite_at(walks).unwrap().id();
        let scene = scene(0, 12, Pace::Game(Speed::X4), Track::Sprite(id));
        let clip = world_clip(
            live(world),
            Theme::cp437(),
            &scene,
            &framing(Size::new(21, 11), true),
        )
        .unwrap();
        let (first, last) = (&clip.frames[0].screen, &clip.frames.last().unwrap().screen);
        assert_eq!(first[(10, 5)].symbol(), "☻");
        assert_eq!(last[(10, 5)].symbol(), "☻");
        assert_ne!(row(first, 2), row(last, 2), "the rocks moved");
        assert!(!row(first, 0).contains("Terra Sprites"), "no top bar");
    }

    #[test]
    fn a_sprite_that_isn_t_there_or_a_story_world_past_4x_is_refused() {
        let scene_at = |pace, track| scene(0, 5, pace, track);
        let tracking_nobody = scene_at(
            Pace::Game(Speed::X4),
            Track::Sprite(terra_sim::EntityId(99)),
        );
        let err = world_clip(
            live(field(&[Pos { x: 5, y: 5 }], &[])),
            Theme::cp437(),
            &tracking_nobody,
            &framing(Size::new(20, 10), true),
        )
        .err()
        .unwrap();
        assert!(err.contains("#99"), "{err}");
        let too_fast = scene_at(Pace::Game(Speed::X16), Track::First);
        let story = World::new(terra_sim::WorldConfig::builtin(&pack()), pack(), 7);
        let err = world_clip(
            live(story),
            Theme::cp437(),
            &too_fast,
            &framing(Size::new(20, 10), true),
        )
        .err()
        .unwrap();
        assert!(err.contains("--lab"), "{err}");
    }

    #[test]
    fn the_opening_scene_starts_in_the_dark_and_ends_on_the_title() {
        let args = Args::default();
        let world = start::title_world(&args, 7, (100, 30)).unwrap();
        let clip = title_clip(
            world,
            Theme::cp437(),
            7,
            None,
            &framing(Size::new(100, 30), false),
        );
        let first = &clip.frames[0].screen;
        // Dark but for the light, just arrived round the sleeper.
        let lit = first
            .content()
            .iter()
            .filter(|cell| cell.symbol() != " ")
            .count();
        assert!(lit < 50, "{lit} of 3,000 cells lit");
        let last = &clip.frames.last().unwrap().screen;
        let lettering = (0..30)
            .map(|y| row(last, y))
            .find(|row| row.contains("▀█▀ █▀▀"));
        assert!(lettering.is_some(), "the title");
        let length = clip.length();
        assert!(
            (Duration::from_secs(7)..=Duration::from_secs(8)).contains(&length),
            "{length:?}"
        );
    }
}
