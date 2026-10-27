//! The renderer's command line (#241).

use std::path::{Path, PathBuf};
use std::time::Duration;

use ratatui::layout::Size;
use terra_sim::{EntityId, Mode};
use terra_tui::app::Tab;
use terra_tui::args::Args;
use terra_tui::clock::Speed;
use terra_tui::ui;

use crate::clip::{Framing, View};

/// Whether the command line asks how to use the renderer.
pub fn asks_for_help(args: &[String]) -> bool {
    args.first().is_some_and(|first| first == "help")
        || args
            .iter()
            .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
}

/// How to use the renderer, shown with `--help` and with any mistake.
pub const USAGE: &str = "\
usage: render title [options] -o <file>
       render world [--seed <n> | --replay <file>] [options] -o <file>
       render samples [--font <file.ttf>]... [options] -o <file>

  title                the title screen's opening scene: a light finds a
                       sprite in the dark
  world                a world from a seed, or a replay, from one tick to
                       another
  samples              a still of a world's screen in Cascadia Mono and in
                       each --font, a file each, to compare fonts: each
                       file is named after its font, and the renderer says
                       which of the screen's characters each lacks

  -o, --out <file>     where to write it: .gif, .mp4 (needs ffmpeg) or .svg
  --seed <n>           the world's seed (default 7)
  --replay <file>      play a replay back instead of a seed's world
  --preset <file>      a world config to make the world from
  --lab                a Lab world, for speeds past 4x
  --from <tick>        the tick the clip starts on (default: where the
                       world or the replay starts): the world runs there
                       first, unseen
  --to <tick>          the tick it ends on (default 100 ticks on, or where
                       a replay ends)
  --speed <speed>      the game's speed: 1/8x, 1/4x, 1/2x, 1x, 2x, 4x
                       (default), 8x or 16x; or max, 100 ticks a second, or
                       max:<n>, n ticks a second. Past 4x needs a Lab world
  --frame <ms>         how long each frame shows, in milliseconds: a
                       multiple of 10, from 20 (default 100)
  --seconds <s>        how long the opening scene's clip is (default: the
                       scene and two seconds of the menu)
  --size <WxH>         the picture's size in screen cells (default 100x30,
                       the smallest screen with every panel)
  --map-view           the map view alone, with no border or panels, at
                       any size
  --hide <parts>       leave parts of the screen out, for the map view and
                       the inspector to take their room: top-bar,
                       inspector, event-log or status-line, a comma between
                       several
  --cell <WxH>         a screen cell's size in pixels (default 10x20)
  --track <n|none>     the sprite the view tracks (default: the one Tab
                       selects first), or none
  --cursor             show the Cursor, following the tracked sprite; by
                       default it's out of the picture
  --tab <name>         the inspector tab: body, brain, chem, genome or world
  --no-motion          draw it without motion: the water still, no trail
                       behind the sprite tracked, and no flash where a
                       sprite dies
  --ascii              draw with the ascii theme
  --theme <file>       draw with a theme of your own
  --font <file.ttf>    draw with this font instead of Cascadia Mono
  --bold-font <file>   and its bold face (default: --font's own)

title takes --seed, --seconds, --size, --cell, --frame, the theme and
the font; the rest are for a world. The opening scene is always the
default preset's world.

9:16 for Shorts: --size 54x48 --map-view --cell 20x40 is 1080x1920, and
the opening scene at --size 108x96 is too.";

/// What the renderer draws, and how.
#[derive(Debug, Clone)]
pub struct Options {
    pub scene: Scene,
    /// Where it's written.
    pub out: PathBuf,
    pub format: Format,
    /// The world's flags, as the game takes them: the seed, the replay,
    /// the preset, the mode and the theme.
    pub world: Args,
    /// The picture's size and shape, and how long each frame shows.
    pub framing: Framing,
    /// A screen cell's size, in pixels.
    pub cell: Size,
    /// The fonts to draw with in place of Cascadia Mono: one for a clip,
    /// with its bold face, or any number for samples.
    pub fonts: Vec<PathBuf>,
    pub bold_font: Option<PathBuf>,
}

/// What the clip shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scene {
    /// The title screen's opening scene (M2 design §8.1), for `seconds`,
    /// or until the menu has shown for a while.
    Title { seconds: Option<Duration> },
    /// A world from one tick to another.
    World(WorldScene),
    /// A still of a world's screen at its first tick, in each font, to
    /// compare them.
    Samples(WorldScene),
}

/// A world's clip: the stretch of ticks, how fast, and what's in view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldScene {
    /// `None`: where the world starts, or the replay.
    pub from: Option<u64>,
    pub end: End,
    pub pace: Pace,
    pub track: Track,
    /// Whether the Cursor shows, following the tracked sprite.
    pub cursor: bool,
    /// The inspector tab open, if not the one selecting a sprite opens.
    pub tab: Option<Tab>,
    /// Whether motion plays, as it does in the game unless it's turned off
    /// (design §6.9).
    pub motion: bool,
}

impl WorldScene {
    /// A world's clip as the command line has it unless told otherwise:
    /// from where the world starts to the usual end, at 4×, tracking the
    /// sprite `Tab` selects first, with no Cursor, on the tab that opens,
    /// with motion.
    pub const USUAL: WorldScene = WorldScene {
        from: None,
        end: End::Usual,
        pace: Pace::Game(Speed::X4),
        track: Track::First,
        cursor: false,
        tab: None,
        motion: true,
    };
}

/// Where a world's clip ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// 100 ticks on, or where a replay ends.
    Usual,
    At(u64),
    /// Where it starts: a still.
    Start,
}

/// How fast the world runs in the clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pace {
    /// At one of the game's speeds, as the game would at it.
    Game(Speed),
    /// At Max, this many ticks a second: the game's own Max goes as fast
    /// as the computer does, which no clip could show the same twice.
    Max(u32),
}

/// The sprite the view tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Track {
    /// The one `Tab` selects first.
    First,
    Sprite(EntityId),
    /// None: the view stays where it starts, on the map's middle.
    Nothing,
}

/// What's written, by the file's extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Gif,
    Mp4,
    Svg,
}

/// The seed a world or the opening scene is made from unless told.
const DEFAULT_SEED: u64 = 7;
/// How many ticks a seed's clip runs unless told.
pub const DEFAULT_TICKS: u64 = 100;
/// Max's ticks a second unless told.
const DEFAULT_MAX_RATE: u32 = 100;
/// The largest picture, in pixels a side.
const MOST_PIXELS: u32 = 16_384;

impl Options {
    /// Reads the command line, not including the program's name.
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
        let mut args = args.into_iter();
        let scene_name = args
            .next()
            .ok_or("say what to draw: title, world or samples")?;
        let mut world = Args::default();
        let mut out = None;
        let mut framing = Framing {
            screen: Size::new(100, 30),
            view: View::Screen(ui::Parts::ALL),
            frame: Duration::from_millis(100),
        };
        // The view is settled once every flag is in, whatever their order.
        let (mut map_view_alone, mut parts) = (false, ui::Parts::ALL);
        let mut cell = Size::new(10, 20);
        let mut fonts = Vec::new();
        let mut bold_font = None;
        let mut seconds = None;
        // The flags given that only a world's clip takes.
        let mut for_a_world = Vec::new();
        let mut scene = WorldScene::USUAL;
        while let Some(flag) = args.next() {
            let mut value = || {
                args.next()
                    .ok_or_else(|| format!("{flag} needs a value after it"))
            };
            if matches!(
                flag.as_str(),
                "--replay"
                    | "--preset"
                    | "--lab"
                    | "--from"
                    | "--to"
                    | "--speed"
                    | "--track"
                    | "--cursor"
                    | "--tab"
                    | "--map-view"
                    | "--hide"
                    | "--no-motion"
            ) {
                for_a_world.push(flag.clone());
            }
            match flag.as_str() {
                "-o" | "--out" => out = Some(PathBuf::from(value()?)),
                "--seed" => world.seed = Some(number(&flag, &value()?)?),
                "--replay" => world.replay = Some(value()?.into()),
                "--preset" => world.preset = Some(value()?.into()),
                "--lab" => world.mode = Mode::Lab,
                "--from" => scene.from = Some(number(&flag, &value()?)?),
                "--to" => scene.end = End::At(number(&flag, &value()?)?),
                "--speed" => scene.pace = pace(&value()?)?,
                "--frame" => framing.frame = Duration::from_millis(number(&flag, &value()?)?),
                "--seconds" => seconds = Some(duration(&value()?)?),
                "--size" => framing.screen = size(&flag, &value()?)?,
                "--map-view" => map_view_alone = true,
                "--hide" => hide(&value()?, &mut parts)?,
                "--cell" => cell = size(&flag, &value()?)?,
                "--track" => scene.track = track(&value()?)?,
                "--cursor" => scene.cursor = true,
                "--tab" => scene.tab = Some(tab(&value()?)?),
                "--no-motion" => scene.motion = false,
                "--ascii" => world.ascii = true,
                "--theme" => world.theme = Some(value()?.into()),
                "--font" => fonts.push(value()?.into()),
                "--bold-font" => bold_font = Some(value()?.into()),
                _ => return Err(format!("`{flag}` isn't a flag the renderer knows")),
            }
        }
        // A replay brings its own world, as in the game (design §2.7).
        let makes_one = [
            ("--seed", world.seed.is_some()),
            ("--preset", world.preset.is_some()),
            ("--lab", world.mode == Mode::Lab),
        ];
        if world.replay.is_some()
            && let Some((flag, _)) = makes_one.iter().find(|(_, given)| *given)
        {
            return Err(format!(
                "{flag} makes a new world, and --replay plays a recorded one: use one or \
                 the other"
            ));
        }
        if world.replay.is_none() {
            world.seed = world.seed.or(Some(DEFAULT_SEED));
        }
        let out = out.ok_or("say where to write it with -o <file>")?;
        let format = format_of(&out)?;
        let scene = match scene_name.as_str() {
            "title" => {
                if let Some(flag) = for_a_world.first() {
                    return Err(format!(
                        "{flag} is for a world's clip: the opening scene is always the \
                         title screen's own (M2 design §8.1)"
                    ));
                }
                Scene::Title { seconds }
            }
            "world" => Scene::World(scene),
            "samples" if scene.end != End::Usual => {
                return Err("samples are a still at --from: leave out --to".into());
            }
            "samples" => Scene::Samples(WorldScene {
                end: End::Start,
                ..scene
            }),
            other => {
                return Err(format!(
                    "`{other}` isn't something to draw: title, world or samples"
                ));
            }
        };
        if seconds.is_some() && !matches!(scene, Scene::Title { .. }) {
            return Err("--seconds is for the opening scene's clip: a world's takes --to".into());
        }
        match &scene {
            Scene::Samples(_) if bold_font.is_some() => {
                return Err("samples draw each font as its own bold: leave out --bold-font".into());
            }
            Scene::Title { .. } | Scene::World(_) if fonts.len() > 1 => {
                return Err("a clip is drawn in one --font: samples take several".into());
            }
            Scene::Title { .. } | Scene::World(_) if bold_font.is_some() && fonts.is_empty() => {
                return Err("--bold-font goes with --font".into());
            }
            _ => {}
        }
        if let Scene::World(scene) = &scene
            && let End::At(to) = scene.end
            && to < scene.from.unwrap_or(0)
        {
            return Err("--to is a tick at or after --from".into());
        }
        if map_view_alone && parts != ui::Parts::ALL {
            return Err(
                "--map-view draws the map view alone, with no other parts to --hide".into(),
            );
        }
        if let Scene::World(scene) | Scene::Samples(scene) = &scene
            && scene.tab.is_some()
            && !parts.inspector
        {
            return Err(
                "--tab opens one of the inspector's tabs, and --hide inspector leaves it out"
                    .into(),
            );
        }
        framing.view = if map_view_alone {
            View::MapViewAlone
        } else {
            View::Screen(parts)
        };
        let (screen, smallest) = (framing.screen, ui::MIN_SIZE);
        if !map_view_alone && (screen.width < smallest.width || screen.height < smallest.height) {
            return Err(format!(
                "the game draws its screen on {}x{} cells at least, and this is {}x{}: \
                 give a bigger --size, or --map-view for the map view alone",
                smallest.width, smallest.height, screen.width, screen.height
            ));
        }
        if screen.width == 0 || screen.height == 0 {
            return Err("--size needs at least a cell each way".into());
        }
        if !(4..=64).contains(&cell.width) || !(8..=128).contains(&cell.height) {
            return Err("--cell is from 4x8 to 64x128 pixels".into());
        }
        let pixels = |cells: u16, each: u16| u32::from(cells) * u32::from(each);
        if pixels(screen.width, cell.width) > MOST_PIXELS
            || pixels(screen.height, cell.height) > MOST_PIXELS
        {
            return Err(format!(
                "the picture would be more than {MOST_PIXELS} pixels a side"
            ));
        }
        // A GIF counts time in hundredths of a second, and browsers slow
        // anything quicker than two of them.
        let frame = framing.frame.as_millis();
        if frame < 20 || !frame.is_multiple_of(10) {
            return Err("--frame is a multiple of 10 milliseconds, from 20".into());
        }
        Ok(Options {
            scene,
            out,
            format,
            world,
            framing,
            cell,
            fonts,
            bold_font,
        })
    }
}

/// The switch in `ui::Parts` that draws one part of the screen.
type Drawn = fn(&mut ui::Parts) -> &mut bool;

/// The parts of the screen `--hide` leaves out, by name, each with the
/// switch that draws it.
const HIDDEN_BY_NAME: [(&str, Drawn); 4] = [
    ("top-bar", |parts| &mut parts.top_bar),
    ("inspector", |parts| &mut parts.inspector),
    ("event-log", |parts| &mut parts.event_log),
    ("status-line", |parts| &mut parts.status_line),
];

/// Leaves out the parts of the screen named, a comma between several.
fn hide(value: &str, parts: &mut ui::Parts) -> Result<(), String> {
    for name in value.split(',') {
        let wanted = name.trim().to_lowercase();
        let Some((_, drawn)) = HIDDEN_BY_NAME.iter().find(|(known, _)| *known == wanted) else {
            let names: Vec<&str> = HIDDEN_BY_NAME.iter().map(|(known, _)| *known).collect();
            return Err(format!(
                "--hide leaves out any of {}, not `{name}`",
                names.join(", ")
            ));
        };
        *drawn(parts) = false;
    }
    Ok(())
}

/// A whole number from the command line.
fn number<T: std::str::FromStr>(flag: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("{flag} takes a whole number, not `{value}`"))
}

/// A length of time in seconds, such as `7.5`.
fn duration(value: &str) -> Result<Duration, String> {
    value
        .parse::<f64>()
        .ok()
        .and_then(|seconds| Duration::try_from_secs_f64(seconds).ok())
        .filter(|seconds| !seconds.is_zero())
        .ok_or_else(|| format!("--seconds takes a number of seconds, not `{value}`"))
}

/// A size as `WxH`.
fn size(flag: &str, value: &str) -> Result<Size, String> {
    let (width, height) = value
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("{flag} takes a size such as 100x30, not `{value}`"))?;
    Ok(Size::new(number(flag, width)?, number(flag, height)?))
}

/// A speed as the game's top bar shows it, or `max` with its rate.
fn pace(value: &str) -> Result<Pace, String> {
    let lower = value.to_lowercase();
    if let Some(rate) = lower.strip_prefix("max") {
        let rate = match rate.strip_prefix(':') {
            Some(rate) => rate.parse().ok().filter(|&rate: &u32| rate > 0),
            None if rate.is_empty() => Some(DEFAULT_MAX_RATE),
            None => None,
        };
        return rate
            .map(Pace::Max)
            .ok_or_else(|| format!("max's rate is a whole number of ticks a second: `{value}`"));
    }
    Speed::ALL
        .into_iter()
        .find(|speed| *speed != Speed::Max && speed.label().eq_ignore_ascii_case(&lower))
        .map(Pace::Game)
        .ok_or_else(|| format!("`{value}` isn't one of the game's speeds: 1/8x to 16x, or max"))
}

/// A sprite by its number, `12` or `#12`, or `none`.
fn track(value: &str) -> Result<Track, String> {
    if value.eq_ignore_ascii_case("none") {
        return Ok(Track::Nothing);
    }
    value
        .trim_start_matches('#')
        .parse()
        .map(|id| Track::Sprite(EntityId(id)))
        .map_err(|_| format!("--track takes a sprite's number or none, not `{value}`"))
}

/// An inspector tab by its name.
fn tab(value: &str) -> Result<Tab, String> {
    Tab::ALL
        .into_iter()
        .find(|tab| tab.label().eq_ignore_ascii_case(value))
        .ok_or_else(|| {
            let names: Vec<String> = Tab::ALL
                .iter()
                .map(|tab| tab.label().to_lowercase())
                .collect();
            format!("--tab is one of {}, not `{value}`", names.join(", "))
        })
}

/// The format a file's extension asks for.
fn format_of(path: &Path) -> Result<Format, String> {
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase());
    match extension.as_deref() {
        Some("gif") => Ok(Format::Gif),
        Some("mp4") => Ok(Format::Mp4),
        Some("svg") => Ok(Format::Svg),
        _ => Err(format!(
            "can't tell what to write as {}: end it in .gif, .mp4 or .svg",
            path.display()
        )),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use ratatui::layout::Size;
    use terra_sim::{EntityId, Mode};
    use terra_tui::app::Tab;
    use terra_tui::clock::Speed;

    use super::*;

    fn parse(line: &str) -> Result<Options, String> {
        Options::parse(line.split_whitespace().map(String::from))
    }

    #[test]
    fn the_title_scene_takes_the_defaults() {
        let options = parse("title -o title.gif").unwrap();
        assert_eq!(options.scene, Scene::Title { seconds: None });
        assert_eq!(options.out, PathBuf::from("title.gif"));
        assert_eq!(options.format, Format::Gif);
        assert_eq!(options.world.seed, Some(7));
        assert_eq!(options.framing.screen, Size::new(100, 30));
        assert_eq!(options.cell, Size::new(10, 20));
        assert_eq!(options.framing.frame, Duration::from_millis(100));
        assert_eq!(options.framing.view, View::Screen(ui::Parts::ALL));
        assert!(options.fonts.is_empty());
    }

    #[test]
    fn a_world_clip_takes_its_ticks_speed_size_and_sprite() {
        let options = parse(
            "world --seed 12 --lab --from 500 --to 800 --speed 16x --frame 50 \
             --size 54x48 --map-view --cell 20x40 --track 9 --cursor --tab brain -o clip.MP4",
        )
        .unwrap();
        let Scene::World(scene) = &options.scene else {
            panic!("a world: {:?}", options.scene);
        };
        assert_eq!(
            (scene.from, scene.end, scene.pace),
            (Some(500), End::At(800), Pace::Game(Speed::X16))
        );
        assert_eq!(scene.track, Track::Sprite(EntityId(9)));
        assert!(scene.cursor);
        assert_eq!(scene.tab, Some(Tab::Brain));
        assert_eq!(options.world.seed, Some(12));
        assert_eq!(options.world.mode, Mode::Lab);
        assert_eq!(options.format, Format::Mp4);
        assert_eq!(
            (options.framing.screen, options.cell, options.framing.frame),
            (
                Size::new(54, 48),
                Size::new(20, 40),
                Duration::from_millis(50)
            )
        );
        assert_eq!(options.framing.view, View::MapViewAlone);
    }

    #[test]
    fn a_world_clip_can_leave_parts_of_the_screen_out() {
        // #291: the map view takes the room they leave.
        let options = parse("world --hide event-log,inspector --hide top-bar -o a.gif").unwrap();
        assert_eq!(
            options.framing.view,
            View::Screen(ui::Parts {
                top_bar: false,
                inspector: false,
                event_log: false,
                status_line: true,
            })
        );
        let options = parse("world --hide Status-Line -o a.gif").unwrap();
        assert_eq!(
            options.framing.view,
            View::Screen(ui::Parts {
                status_line: false,
                ..ui::Parts::ALL
            })
        );
    }

    #[test]
    fn a_world_clip_plays_motion_unless_told_not_to() {
        // Design §6.9: on, as in the game, and `--no-motion` turns it off.
        let motion = |line: &str| match parse(line).unwrap().scene {
            Scene::World(scene) | Scene::Samples(scene) => scene.motion,
            other => panic!("{other:?}"),
        };
        assert!(motion("world -o a.gif"));
        assert!(!motion("world --no-motion -o a.gif"));
        assert!(!motion("samples --no-motion -o a.gif"));
        assert!(USAGE.contains("--no-motion"));
    }

    #[test]
    fn a_replay_starts_where_it_starts_unless_told() {
        let options = parse("world --replay last_session.replay -o a.gif").unwrap();
        let Scene::World(scene) = &options.scene else {
            panic!("a world: {:?}", options.scene);
        };
        assert_eq!((scene.from, scene.end), (None, End::Usual));
        assert_eq!(options.world.seed, None, "the replay's own");
    }

    #[test]
    fn samples_are_a_still_in_each_font() {
        let options = parse("samples --font a.ttf --font b.ttf --from 40 -o fonts.gif").unwrap();
        let Scene::Samples(scene) = &options.scene else {
            panic!("samples: {:?}", options.scene);
        };
        assert_eq!((scene.from, scene.end), (Some(40), End::Start));
        assert_eq!(
            options.fonts,
            [PathBuf::from("a.ttf"), PathBuf::from("b.ttf")]
        );
    }

    #[test]
    fn max_runs_a_hundred_ticks_a_second_unless_told_how_many() {
        let pace = |line: &str| match parse(line).unwrap().scene {
            Scene::World(scene) => scene.pace,
            other => panic!("{other:?}"),
        };
        assert_eq!(pace("world --speed max -o a.svg"), Pace::Max(100));
        assert_eq!(pace("world --speed MAX:250 -o a.svg"), Pace::Max(250));
        assert_eq!(
            pace("world --speed 1/8x -o a.svg"),
            Pace::Game(Speed::Eighth)
        );
    }

    #[test]
    fn what_cannot_be_drawn_is_refused_with_why() {
        let refused = |line: &str, why: &str| {
            let err = parse(line).expect_err(line);
            assert!(err.contains(why), "{line}: {err}");
        };
        refused("title", "-o <file>");
        refused("title -o title.png", ".gif, .mp4 or .svg");
        refused("world --size 19x48 -o a.gif", "--map-view");
        refused(
            "title --map-view -o a.gif",
            "--map-view is for a world's clip",
        );
        refused(
            "title --replay a.replay -o a.gif",
            "--replay is for a world's clip",
        );
        refused("title --from 5 -o a.gif", "--from is for a world's clip");
        refused(
            "title --no-motion -o a.gif",
            "--no-motion is for a world's clip",
        );
        refused(
            "title --preset a.ron -o a.gif",
            "--preset is for a world's clip",
        );
        refused("world --seconds 3 -o a.gif", "--seconds");
        refused("samples --to 5 -o a.gif", "--to");
        refused(
            "world --font a.ttf --font b.ttf -o a.gif",
            "samples take several",
        );
        refused(
            "samples --font a.ttf --bold-font b.ttf -o a.gif",
            "--bold-font",
        );
        refused("world --bold-font b.ttf -o a.gif", "goes with --font");
        refused("world --frame 25 -o a.gif", "multiple of 10");
        refused("world --from 300 --to 200 -o a.gif", "after --from");
        refused("world --speed 3x -o a.gif", "3x");
        refused("world --track bob -o a.gif", "bob");
        for flag in ["--seed 7", "--preset a.ron", "--lab"] {
            refused(
                &format!("world {flag} --replay a.replay -o a.gif"),
                "--replay plays a recorded one",
            );
        }
        refused("title --seconds 1e300 -o a.gif", "1e300");
        refused("title --seconds 1e-12 -o a.gif", "1e-12");
        refused("sing -o a.gif", "sing");
        refused(
            "world --hide map -o a.gif",
            "any of top-bar, inspector, event-log, status-line, not `map`",
        );
        refused(
            "title --hide event-log -o a.gif",
            "--hide is for a world's clip",
        );
        refused(
            "world --map-view --hide event-log -o a.gif",
            "--map-view draws the map view alone",
        );
        refused(
            "world --hide inspector --tab brain -o a.gif",
            "--tab opens one of the inspector's tabs",
        );
        refused(
            "samples --hide inspector --tab brain -o a.gif",
            "--tab opens one of the inspector's tabs",
        );
    }
}
