//! The renderer's command line (#241).

use std::path::{Path, PathBuf};
use std::time::Duration;

use ratatui::layout::Size;
use terra_sim::{EntityId, Mode};
use terra_tui::app::Tab;
use terra_tui::args::Args;
use terra_tui::clock::{Clock, Speed};
use terra_tui::ui;

/// How to use the renderer, shown with `--help` and with any mistake.
pub const USAGE: &str = "\
usage: render title [options] -o <file>
       render world [--seed <n> | --replay <file>] [options] -o <file>
       render samples [--font <file.ttf>] [options] -o <file>

  title                the title screen's opening scene: a light finds a
                       sleeping sprite in the dark
  world                a world from a seed, or a replay, from one tick to
                       another
  samples              a still of a world's screen, to try a font

  -o, --out <file>     where to write it: .gif, .mp4 (needs ffmpeg) or .svg
  --seed <n>           the world's seed (default 7)
  --replay <file>      play a replay back instead of a seed's world
  --preset <file>      a world config to make the world from
  --lab                a Lab world, for speeds past 4x
  --from <tick>        the tick the clip starts on (default 0): the world
                       runs there first, unseen
  --to <tick>          the tick it ends on (default 100 ticks on, or where
                       a replay ends)
  --speed <speed>      the game's speed: 1/8x, 1/4x, 1/2x, 1x, 2x, 4x
                       (default), 8x or 16x; or max, 100 ticks a second, or
                       max:<n>, n ticks a second. Past 4x needs a Lab world
  --frame <ms>         how long each frame shows, in milliseconds: a
                       multiple of 10, from 20 (default 100)
  --seconds <s>        how long the title scene is (default: the scene and
                       two seconds of the menu)
  --size <WxH>         the picture's size in screen cells (default 100x30,
                       the smallest screen the game draws its panels on)
  --map                the map alone, with no panels, at any size
  --cell <WxH>         a screen cell's size in pixels (default 10x20)
  --track <n|none>     the sprite the view tracks (default: the one Tab
                       selects first), or none
  --cursor             show the Cursor, following the tracked sprite; by
                       default it's out of the picture
  --tab <name>         the inspector tab: body, brain, chem, genome or world
  --ascii              draw with the ascii theme
  --theme <file>       draw with a theme of your own
  --font <file.ttf>    draw with this font instead of Cascadia Mono
  --bold-font <file>   and its bold face (default: --font's own)

9:16 for Shorts: --size 54x48 --map --cell 20x40 is 1080x1920, and the
title scene at --size 108x96 is too.";

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
    /// The picture's size, in screen cells.
    pub screen: Size,
    /// A screen cell's size, in pixels.
    pub cell: Size,
    /// The map view's tiles alone, with no panels.
    pub map_alone: bool,
    /// How long each frame shows.
    pub frame: Duration,
    /// A font to draw with, and its bold face, in place of Cascadia Mono.
    pub font: Option<PathBuf>,
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
    /// A still of a world's screen at its first tick, to try a font.
    Samples(WorldScene),
}

/// A world's clip: the stretch of ticks, how fast, and what's in view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldScene {
    pub from: u64,
    /// `None`: 100 ticks on, or where a replay ends.
    pub to: Option<u64>,
    pub pace: Pace,
    pub track: Track,
    /// Whether the Cursor shows, following the tracked sprite.
    pub cursor: bool,
    /// The inspector tab open, if not the one selecting a sprite opens.
    pub tab: Option<Tab>,
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

/// The seed a world or the title scene is made from unless told.
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
        if matches!(scene_name.as_str(), "--help" | "-h" | "help") {
            return Err("how to use it:".into());
        }
        let mut world = Args::default();
        let mut out = None;
        let mut screen = Size::new(100, 30);
        let mut cell = Size::new(10, 20);
        let mut map_alone = false;
        let mut frame = Duration::from_millis(100);
        let mut font = None;
        let mut bold_font = None;
        let mut seconds = None;
        let mut scene = WorldScene {
            from: 0,
            to: None,
            pace: Pace::Game(Speed::X4),
            track: Track::First,
            cursor: false,
            tab: None,
        };
        while let Some(flag) = args.next() {
            let mut value = || {
                args.next()
                    .ok_or_else(|| format!("{flag} needs a value after it"))
            };
            match flag.as_str() {
                "-o" | "--out" => out = Some(PathBuf::from(value()?)),
                "--seed" => world.seed = Some(number(&flag, &value()?)?),
                "--replay" => world.replay = Some(value()?.into()),
                "--preset" => world.preset = Some(value()?.into()),
                "--lab" => world.mode = Mode::Lab,
                "--from" => scene.from = number(&flag, &value()?)?,
                "--to" => scene.to = Some(number(&flag, &value()?)?),
                "--speed" => scene.pace = pace(&value()?)?,
                "--frame" => frame = Duration::from_millis(number(&flag, &value()?)?),
                "--seconds" => seconds = Some(duration(&value()?)?),
                "--size" => screen = size(&flag, &value()?)?,
                "--map" => map_alone = true,
                "--cell" => cell = size(&flag, &value()?)?,
                "--track" => scene.track = track(&value()?)?,
                "--cursor" => scene.cursor = true,
                "--tab" => scene.tab = Some(tab(&value()?)?),
                "--ascii" => world.ascii = true,
                "--theme" => world.theme = Some(value()?.into()),
                "--font" => font = Some(value()?.into()),
                "--bold-font" => bold_font = Some(value()?.into()),
                _ => return Err(format!("`{flag}` isn't a flag the renderer knows")),
            }
        }
        if world.seed.is_some() && world.replay.is_some() {
            return Err("a world comes from --seed or --replay, not both".into());
        }
        if world.replay.is_none() {
            world.seed = world.seed.or(Some(DEFAULT_SEED));
        }
        let out = out.ok_or("say where to write it with -o <file>")?;
        let format = format_of(&out)?;
        let scene = match scene_name.as_str() {
            "title" if map_alone => {
                return Err("the title scene is the whole screen: leave out --map".into());
            }
            "title" => Scene::Title { seconds },
            "world" => Scene::World(scene),
            "samples" => Scene::Samples(scene),
            other => {
                return Err(format!(
                    "`{other}` isn't something to draw: title, world or samples"
                ));
            }
        };
        if let Scene::World(scene) | Scene::Samples(scene) = &scene
            && scene.to.is_some_and(|to| to < scene.from)
        {
            return Err("--to is a tick at or after --from".into());
        }
        let smallest = ui::MIN_SIZE;
        if !map_alone && (screen.width < smallest.width || screen.height < smallest.height) {
            return Err(format!(
                "the game draws its screen on {}x{} cells at least, and this is {}x{}: \
                 give a bigger --size, or --map for the map alone",
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
        if frame.as_millis() < 20 || !frame.as_millis().is_multiple_of(10) {
            return Err("--frame is a multiple of 10 milliseconds, from 20".into());
        }
        Ok(Options {
            scene,
            out,
            format,
            world,
            screen,
            cell,
            map_alone,
            frame,
            font,
            bold_font,
        })
    }
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
        .filter(|seconds| seconds.is_finite() && *seconds > 0.0)
        .map(Duration::from_secs_f64)
        .ok_or_else(|| format!("--seconds takes a number of seconds, not `{value}`"))
}

/// A size as `WxH`.
fn size(flag: &str, value: &str) -> Result<Size, String> {
    let (width, height) = value
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("{flag} takes a size such as 100x30, not `{value}`"))?;
    Ok(Size::new(number(flag, width)?, number(flag, height)?))
}

/// The game's speeds, slowest first.
pub fn speeds() -> Vec<Speed> {
    let mut clock = Clock::new();
    while clock.speed() != Speed::Eighth {
        clock.slower();
    }
    let mut speeds = vec![clock.speed()];
    while clock.speed() != Speed::Max {
        clock.faster();
        speeds.push(clock.speed());
    }
    speeds
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
    speeds()
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
        assert_eq!(options.screen, Size::new(100, 30));
        assert_eq!(options.cell, Size::new(10, 20));
        assert_eq!(options.frame, Duration::from_millis(100));
        assert!(!options.map_alone);
    }

    #[test]
    fn a_world_clip_takes_its_ticks_speed_size_and_sprite() {
        let options = parse(
            "world --seed 12 --lab --from 500 --to 800 --speed 16x --frame 50 \
             --size 54x48 --map --cell 20x40 --track 9 --cursor --tab brain -o clip.MP4",
        )
        .unwrap();
        let Scene::World(scene) = &options.scene else {
            panic!("a world: {:?}", options.scene);
        };
        assert_eq!(
            (scene.from, scene.to, scene.pace),
            (500, Some(800), Pace::Game(Speed::X16))
        );
        assert_eq!(scene.track, Track::Sprite(EntityId(9)));
        assert!(scene.cursor);
        assert_eq!(scene.tab, Some(Tab::Brain));
        assert_eq!(options.world.seed, Some(12));
        assert_eq!(options.world.mode, Mode::Lab);
        assert_eq!(options.format, Format::Mp4);
        assert_eq!(
            (options.screen, options.cell, options.frame),
            (
                Size::new(54, 48),
                Size::new(20, 40),
                Duration::from_millis(50)
            )
        );
        assert!(options.map_alone);
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
        refused("world --size 54x48 -o a.gif", "--map");
        refused("title --map -o a.gif", "the title scene");
        refused("world --frame 25 -o a.gif", "multiple of 10");
        refused("world --from 300 --to 200 -o a.gif", "after --from");
        refused("world --speed 3x -o a.gif", "3x");
        refused("world --track bob -o a.gif", "bob");
        refused(
            "world --seed 7 --replay a.replay -o a.gif",
            "--seed or --replay",
        );
        refused("sing -o a.gif", "sing");
    }
}
