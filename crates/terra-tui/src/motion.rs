//! Motion (design §6.9, #306): small effects drawn over the finished screen
//! in real time, so the eye goes to what just happened.
//!
//! How it fits together:
//!
//! - **What the app keeps** is [`Motion`]: the switch; how each sprite and
//!   egg was drawn after the last ticks taken in ([`Seen`]), since the world
//!   removes a dead sprite before it says so; the deaths and hatchings
//!   playing, and the eggs about to hatch, flashing, with how far through a
//!   flash each had got as the last ticks were taken in; the tab sweeping in,
//!   which `App::open` starts; what filled the screen at the last frame,
//!   which the app's clock compares with what does now to see a pop-up
//!   open or close, and how the last frame drew the pop-ups that closed;
//!   the selected sprite's drives' bars, with each drive's latest rises and
//!   the latest wave on its bar, and the tiles it has left, its trail,
//!   noted as ticks are taken in; and the pan under way. Each effect
//!   keeps when it began, in the app's `running_for`. Nothing is stepped as
//!   it's drawn (§6.8).
//! - **The pan** is no effect but where the map view is drawn from
//!   ([`Motion::shown_from`]): the viewport jumps at once, and the map view,
//!   with everything drawn on it and what the pointer points at, goes by
//!   where the pan has reached. The Cursor goes to the tile under a still
//!   pointer only once the pan ends, so the world never hears of the tiles
//!   between.
//! - **Drawing** builds each tachyonfx effect afresh every frame and plays
//!   it once to its age (`play` on the map, `play_over` on the panels), so
//!   the same app drawn twice draws the same screen. `ui::draw_tiles` calls
//!   [`play_on_map`] once the tiles are drawn and before the marks go over
//!   them, and `ui::render_with` calls [`play_on_panels`] once the panels
//!   and pop-ups are drawn and what can be pointed at is marked, before the
//!   tooltip, which doesn't move. A bar's blocks and its waves are no
//!   effect: the Body tab fills as many thirds of a block as
//!   `App::drive_bar` says, and colours them as [`bar_waves`] says.
//! - **Time** is real time, which the frame loop hands the app (§6.6):
//!   [`age`] says how far into an effect a frame is, and [`eased`] how far
//!   that's got, fast and then slowing.
//! - **It thins out at 4× and up** ([`thins_out`]): fewer deaths and
//!   hatchings at once, and shorter, as the trail is, the bars and the
//!   view jump, and the bars' waves don't play.
//! - **A hatching's shell** is characters, the glyph set's ([`ShellLook`]):
//!   the egg cracked, and then its pieces on the tiles round it that show
//!   bare ground. Being characters, it plays in a palette with no colour
//!   too, where the rest of motion's colours on the map don't.
//! - **Colours** are the theme's `motion` section's, but for a bar's waves,
//!   which glow from its drive's colour towards white. A fade blends exact
//!   colours, so a named one is blended as Campbell draws it
//!   ([`campbell::exact`]), and a tile with no background of its own takes
//!   the theme's dark. The terminal's own text colour is blended as the
//!   palette draws it ([`own_text`]): as Campbell's, or in Game Kid as its
//!   green. On the panels, what an effect leaves as it was keeps its own
//!   colour, named or the terminal's, so tachyonfx never sees the
//!   terminal's own colour, which it would take for white or black.
//! - **The same on every computer:** only effects that depend on nothing
//!   but their age, eased by polynomials (`QuadOut`) or evenly, as the
//!   shell's pieces fade, and blended in RGB, and no `sin`, `cos`, `powf` or
//!   `exp`.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier};
use tachyonfx::{ColorSpace, Effect, EffectTimer, Interpolation, fx};
use terra_sim::{EntityId, EventKind, Pos, World};

use crate::app::{App, Origin, Screen, Tab};
use crate::campbell::{self, FOREGROUND, Rgb};
use crate::clock::Speed;
use crate::inspector;
use crate::policy::{Panel, Subject};
use crate::space::{Pattern, Place};
use crate::spots::Spots;
use crate::text;
use crate::theme::{BarWaves, MotionLook, ShellLook};
use crate::ui;

/// How long a death plays, and a hatching (design §6.9): half as long at
/// 4× and up.
const DEATH: Duration = Duration::from_millis(1200);
const HATCHING: Duration = Duration::from_millis(1000);

/// How long a hatching shows the egg cracked before the young appears, at
/// 1× (design §6.9): half as long at 4× and up, as the hatching is.
const CRACK: Duration = Duration::from_millis(150);

/// When a hatching's shell's halves slide a tile further out, from when
/// the egg stops showing cracked, at 1× (design §6.9).
const HALVES_SLIDE: Duration = Duration::from_millis(210);

/// When a hatching's shell's halves start to fade from the egg's colour to
/// their tile's own, and when they've faded and go, from when the egg
/// stops showing cracked, at 1× (design §6.9). They fade evenly between.
const HALVES_FADE: (Duration, Duration) = (Duration::from_millis(110), Duration::from_millis(700));

/// And its crumbs' (design §6.9).
const CRUMBS_FADE: (Duration, Duration) = (Duration::from_millis(60), Duration::from_millis(600));

/// How many ticks before it hatches an egg on the map starts to flash
/// (design §6.9): from 100 ticks left down to the last.
pub(crate) const FLASH_FROM: u64 = 100;

/// How long each flash of an egg about to hatch lasts on its last tick,
/// and how much longer for each tick further off (design §6.9): 120 ms on
/// the last, 255 ms at 10 ticks and 1.605 s at 100, so its flashes come
/// faster and faster.
const FLASH_QUICKEST: Duration = Duration::from_millis(120);
const FLASH_SLOWER: Duration = Duration::from_millis(15);

/// How far an egg's flash glows from its tile's own background towards the
/// sparks' colour (design §6.9): three tenths of the way at 100 ticks from
/// hatching, growing evenly to seven tenths on the last tick, which is
/// where a hatching's glow fades from.
const FLASH_FAINTEST: f32 = 0.3;
const FLASH_BRIGHTEST: f32 = 0.7;

/// The most deaths and hatchings that play at once (design §6.9), and at
/// 4× and up.
const MOST: usize = 4;
const MOST_THINNED: usize = 2;

/// How many blocks a drive's bar is long (design §6.1).
const BLOCKS: usize = inspector::BLOCKS as usize;

/// The steps a hatching's sparks start on, in milliseconds.
const SPARK_STEP_MS: u64 = 50;

/// How long a tab opened takes to sweep in (design §6.9), at every speed.
const TAB_SWEEP: Duration = Duration::from_millis(250);

/// How many columns the edge of a tab sweeping in takes to go from its
/// shade to all of it.
const SWEEP_EDGE: u16 = 6;

/// How long a pop-up takes to fade in, and to fade out as it closes, half
/// of it, and what was under it to fade back, the other half (design §6.9),
/// at every speed.
const POP_UP: Duration = Duration::from_millis(150);

/// How long a drive's bar takes to ease to its new value (design §6.9).
const BAR: Duration = Duration::from_millis(400);

/// How much a drive's bar's waves show, from 0 to 1, by how many times its
/// usual rise its drive rose over the tick last taken in (design §6.9): a
/// little at its usual rise, more the faster, and all of it from five times
/// its usual on, evenly between.
const AMP_AT: [(f32, f32); 4] = [(0.0, 0.0), (1.0, 0.25), (2.5, 0.55), (5.0, 1.0)];

/// How many of its latest rises a drive's usual rise is the average of
/// (design §6.9): its last hundred, or as many as it has had since the
/// sprite was selected.
const USUAL_OF: usize = 100;

/// How far each block of a breaking wave's spray past the bar's end shows
/// of the first's (design §6.9): seven tenths of the last's each.
const SPRAY_FADES: [f32; 3] = [1.0, 0.7, 0.49];

/// How long a running ripple's crest takes to run up a drive's bar and off
/// its end, in milliseconds (design §6.9).
const RIPPLE_RUN: f32 = 520.0;

/// How long a falling drive's dip takes to run back down its bar, and how
/// long from one dip's start to the next's, in milliseconds (design §6.9).
const DIP_RUN: f32 = 1100.0;
const DIP_EVERY: f32 = 2600.0;

/// How long each tile of the trail takes to fade (design §6.9): half as
/// long at 4× and up.
const TRAIL: Duration = Duration::from_millis(1500);

/// The most tiles the trail tints at once (design §6.9).
const TRAIL_TILES: usize = 8;

/// How long the pan takes (design §6.9).
const PAN: Duration = Duration::from_millis(300);

/// How many tiles, across or down, the view may move to centre on a sprite
/// and still jump rather than pan (design §6.9).
const PAN_PAST: i32 = 2;

/// Whether motion's colours play over the screen (design §6.9): while
/// motion is on, in a palette with colour. Mono has none, so its deaths, a
/// hatching's warming and sparks, flashing eggs, trail, tab sweeps and
/// pop-up fades don't play, though the pan, the bars and a hatching's
/// shell, which are no colour, still do (§6.2, #307).
pub(crate) fn colours_play(app: &App) -> bool {
    app.motion() && app.theme.has_colour()
}

/// The terminal's own text colour as the theme's palette draws it, for a
/// fade to blend from (design §6.2, §6.9): its foreground, if it gives one,
/// as Game Kid does, or else as Campbell draws it.
fn own_text(app: &App) -> Rgb {
    app.theme
        .foreground()
        .and_then(campbell::rgb)
        .unwrap_or(FOREGROUND)
}

/// Whether motion thins out at `speed` (design §6.9): at 4× and up, which
/// takes in Lab's faster speeds.
pub(crate) fn thins_out(speed: Speed) -> bool {
    speed >= Speed::X4
}

/// How many deaths and hatchings play at once at `speed`.
fn most(speed: Speed) -> usize {
    if thins_out(speed) { MOST_THINNED } else { MOST }
}

/// How long an effect that lasts `full` lasts at `speed`.
fn at_speed(full: Duration, speed: Speed) -> Duration {
    if thins_out(speed) { full / 2 } else { full }
}

/// How far into an effect that began at `began` the moment `now` is.
pub(crate) fn age(began: Duration, now: Duration) -> Duration {
    now.saturating_sub(began)
}

/// How far an effect `age` into one that lasts `lasts` has got, from 0 to
/// 1, eased out: fast and then slowing (design §6.9).
pub(crate) fn eased(age: Duration, lasts: Duration) -> f32 {
    let (age, lasts) = (millis(age).as_millis(), millis(lasts).as_millis());
    if lasts == 0 {
        return 1.0;
    }
    Interpolation::QuadOut.alpha(age.min(lasts) as f32 / lasts as f32)
}

/// `duration` in tachyonfx's whole milliseconds.
fn millis(duration: Duration) -> tachyonfx::Duration {
    tachyonfx::Duration::from_millis(u32::try_from(duration.as_millis()).unwrap_or(u32::MAX))
}

/// An effect's timer: it lasts `lasts` and eases out (design §6.9).
fn timer(lasts: Duration) -> EffectTimer {
    EffectTimer::new(millis(lasts), Interpolation::QuadOut)
}

/// Plays `effect` on screen cell `cell`, `age` into it: built afresh for
/// this frame and processed once, blending in RGB (design §6.9).
fn play(effect: Effect, age: Duration, buf: &mut Buffer, cell: Position) {
    let area = Rect::new(cell.x, cell.y, 1, 1);
    let mut effect = effect.with_area(area).with_color_space(ColorSpace::Rgb);
    effect.process(millis(age), buf, area);
}

/// What the app keeps for motion (design §6.8, §6.9).
#[derive(Debug, Clone)]
pub(crate) struct Motion {
    /// Whether it's on: one switch turns all motion off.
    on: bool,
    /// How each sprite and egg on the map was drawn after the last ticks
    /// taken in, while motion is on.
    seen: BTreeMap<EntityId, Seen>,
    /// The deaths and hatchings playing, oldest first.
    moments: VecDeque<Moment>,
    /// The eggs on the map within `FLASH_FROM` ticks of hatching, flashing.
    flashing: BTreeMap<EntityId, Flashing>,
    /// The tab sweeping in, if one is.
    sweep: Option<Sweep>,
    /// What filled the screen besides the map when the app last moved its
    /// clock on, which a pop-up opening or closing changes.
    screen: Screen,
    /// When what's up now opened, while it fades in.
    opened: Option<Duration>,
    /// How the pop-ups that just closed were last drawn, where they were,
    /// and when they closed, while they fade out and what was under them
    /// fades back.
    closed: Option<(Vec<Buffer>, Duration)>,
    /// The selected sprite's drives' bars, while it's in the world.
    bars: Option<Bars>,
    /// The selected sprite's trail, while it's in the world.
    trail: Option<Trail>,
    /// The map view panning to where the viewport is, until the app's
    /// clock sees it end.
    pan: Option<Pan>,
}

/// An egg on the map about to hatch, flashing (design §6.9): `left` ticks
/// from hatching, as the ticks last taken in left it, and `into` a flash at
/// `at`, from which it goes on at the period of the ticks left.
#[derive(Debug, Clone, Copy)]
struct Flashing {
    left: u64,
    into: Duration,
    at: Duration,
}

impl Flashing {
    /// An egg `left` ticks from hatching, first seen so at `now`: its first
    /// flash starts then.
    fn new(left: u64, now: Duration) -> Flashing {
        Flashing {
            left,
            into: Duration::ZERO,
            at: now,
        }
    }

    /// How far into a flash it is at `now`: on from where it had got, at
    /// the period of the ticks left, and while paused too.
    fn reached(&self, now: Duration) -> Duration {
        let period = flash_period(self.left).as_nanos();
        let into = (self.into + age(self.at, now)).as_nanos() % period;
        nanos(into)
    }

    /// The same egg `left` ticks from hatching at `now`, as a tick taken in
    /// leaves it: the flash keeps its phase, as far through as it had got,
    /// and goes on from there at the new period, with no jump or new start
    /// (design §6.9).
    fn taken_on(self, left: u64, now: Duration) -> Flashing {
        if left == self.left {
            return self;
        }
        let (was, period) = (flash_period(self.left), flash_period(left));
        let into = self.reached(now).as_nanos() * period.as_nanos() / was.as_nanos();
        Flashing {
            left,
            into: nanos(into),
            at: now,
        }
    }
}

/// How long each flash of an egg `left` ticks from hatching lasts (design
/// §6.9).
fn flash_period(left: u64) -> Duration {
    let further = left.clamp(1, FLASH_FROM) - 1;
    FLASH_QUICKEST + FLASH_SLOWER * u32::try_from(further).unwrap_or(u32::MAX)
}

/// How far each flash of an egg `left` ticks from hatching glows towards
/// the sparks' colour, from 0 to 1 (design §6.9).
fn flash_strength(left: u64) -> f32 {
    let nearer = (FLASH_FROM - left.clamp(1, FLASH_FROM)) as f32 / (FLASH_FROM - 1) as f32;
    FLASH_FAINTEST + (FLASH_BRIGHTEST - FLASH_FAINTEST) * nearer
}

/// `n` nanoseconds, less than a period of a flash.
fn nanos(n: u128) -> Duration {
    Duration::from_nanos(u64::try_from(n).unwrap_or(u64::MAX))
}

/// The map view panning from where it was drawn, `from`, to wherever the
/// viewport is, from when it began (design §6.9). The viewport may move
/// on while it pans, as Track follows a sprite, and the pan goes there.
#[derive(Debug, Clone, Copy)]
struct Pan {
    from: Origin,
    began: Duration,
}

/// The tiles the selected sprite has just left, as the screen saw it move
/// from frame to frame (design §6.9).
#[derive(Debug, Clone)]
struct Trail {
    sprite: EntityId,
    /// The tile it was on when last seen.
    on: Pos,
    /// The tiles it has left and not yet faded, oldest first: each tile
    /// once, as it last left it.
    left: VecDeque<Left>,
}

/// A tile the selected sprite left, from when it left it, fading for as
/// long as it lasts.
#[derive(Debug, Clone, Copy)]
struct Left {
    tile: Pos,
    began: Duration,
    lasts: Duration,
}

/// The selected sprite's drives' bars on the Body tab, by the drive's name,
/// as they ease to the levels last taken in (design §6.9).
#[derive(Debug, Clone)]
struct Bars {
    sprite: EntityId,
    drives: BTreeMap<String, Bar>,
}

/// A drive's bar easing from `from` thirds of a block filled to `to`, from
/// when it began (design §6.1), and its waves (§6.9).
#[derive(Debug, Clone)]
struct Bar {
    from: u8,
    to: u8,
    began: Duration,
    waves: Waves,
}

/// A drive of the selected sprite as the ticks last taken in left it: its
/// name, the thirds of a block its bar fills, and how much it changed over
/// the last tick.
#[derive(Debug, Clone)]
pub(crate) struct DriveLevel {
    pub(crate) name: String,
    pub(crate) thirds: u8,
    pub(crate) change: f32,
}

impl Bar {
    /// A bar still at `thirds`, from `now`, with no waves yet.
    fn at(thirds: u8, now: Duration) -> Bar {
        Bar {
            from: thirds,
            to: thirds,
            began: now,
            waves: Waves::default(),
        }
    }

    /// How many thirds of a block it fills at `now`: a third at a time, so
    /// a block fills `░` `▒` `▓` `█` as it goes, quick at first and slowing
    /// (design §6.9).
    fn shown(&self, now: Duration) -> u8 {
        let way = f32::from(self.to) - f32::from(self.from);
        let moved = (way * eased(age(self.began, now), BAR)).round();
        (f32::from(self.from) + moved) as u8
    }
}

/// A drive's bar's waves (design §6.9): its drive's latest rises, whose
/// average is its usual rise, how it moved over the tick last taken in,
/// and the latest wave to begin. Only ticks taken in change them, never a
/// frame drawn (§6.8).
#[derive(Debug, Clone, Default)]
struct Waves {
    /// At most `USUAL_OF`, oldest first.
    rises: VecDeque<f32>,
    /// None while its drive is steady.
    pace: Option<Pace>,
    latest: Option<Wave>,
}

/// How a drive moved over the tick last taken in (design §6.9).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Pace {
    /// Rising, and how much its waves show for how fast, from 0 to 1.
    Rising(f32),
    Falling,
}

/// A wave on a drive's bar, from when it began, as it goes at `pace`.
#[derive(Debug, Clone, Copy)]
struct Wave {
    began: Duration,
    pace: Pace,
}

impl Waves {
    /// Takes in its drive's `change` over a tick taken in at `now`, as
    /// `style` plays: it rose, if the Body tab's arrow says so, and by how
    /// many times its usual rise, which the rise then joins; it fell; or
    /// it held steady. The wave under way plays on as it was, and the next
    /// begins once it has shown, at the new pace; at once, if that's
    /// overdue. Where waves don't `play`, at 4× and up, none is under way
    /// (design §6.9).
    fn took_in(&mut self, change: f32, now: Duration, style: BarWaves, play: bool) {
        let pace = self.pace_of(change);
        self.latest = self.latest_by(now, style);
        self.pace = pace;
        let (Some(pace), true) = (pace, play) else {
            self.latest = self.latest.filter(|_| play);
            return;
        };
        let due = self
            .latest
            .is_none_or(|wave| wave.began + after(style, wave.pace, pace) <= now);
        if due {
            self.latest = Some(Wave { began: now, pace });
        }
    }

    /// How its drive moved, by `change` over a tick: rising, and how fast
    /// against its usual rise, which the rise then joins, or falling, or
    /// steady, as the Body tab's arrow says (design §6.1). A sprite's
    /// first rise is as usual for it.
    fn pace_of(&mut self, change: f32) -> Option<Pace> {
        text::change(change)?;
        if change < 0.0 {
            return Some(Pace::Falling);
        }
        let ratio = self.usual().map_or(1.0, |usual| change / usual);
        if self.rises.len() >= USUAL_OF {
            self.rises.pop_front();
        }
        self.rises.push_back(change);
        Some(Pace::Rising(amp(ratio)))
    }

    /// Its drive's usual rise a tick: the average of its latest rises, if
    /// it has risen since the sprite was selected.
    fn usual(&self) -> Option<f32> {
        let rises = self.rises.len();
        (rises > 0).then(|| self.rises.iter().sum::<f32>() / rises as f32)
    }

    /// The latest wave to begin by `now`, as `style` plays: the one taken
    /// in last, or one after it at the pace taken in last. None but the
    /// one taken in last comes while its drive is steady.
    fn latest_by(&self, now: Duration, style: BarWaves) -> Option<Wave> {
        let wave = self.latest?;
        let Some(pace) = self.pace else {
            return Some(wave);
        };
        let (since, first) = (age(wave.began, now), after(style, wave.pace, pace));
        if since < first {
            return Some(wave);
        }
        let every = every(style, pace);
        let more = (since - first).as_nanos() / every.as_nanos();
        Some(Wave {
            began: wave.began + first + every * u32::try_from(more).unwrap_or(u32::MAX),
            pace,
        })
    }

    /// The wave showing at `now`, as `style` plays, and how far into it.
    fn showing(&self, now: Duration, style: BarWaves) -> Option<(Pace, Duration)> {
        let wave = self.latest_by(now, style)?;
        let into = age(wave.began, now);
        (into < shows(style, wave.pace)).then_some((wave.pace, into))
    }
}

/// How much a drive's bar's waves show for a rise `ratio` times its usual
/// (design §6.9), from 0 to 1.
fn amp(ratio: f32) -> f32 {
    let Some(i) = AMP_AT.iter().position(|&(at, _)| ratio < at) else {
        return 1.0;
    };
    let Some(below) = i.checked_sub(1) else {
        return 0.0;
    };
    let ((x0, y0), (x1, y1)) = (AMP_AT[below], AMP_AT[i]);
    y0 + (y1 - y0) * (ratio - x0) / (x1 - x0)
}

/// `ms` milliseconds, to the nearest microsecond.
fn millis_of(ms: f32) -> Duration {
    Duration::from_micros((ms * 1000.0).round() as u64)
}

/// How long a crest takes to run up a drive's bar rising at `amp`, and how
/// long it then takes to break (design §6.9).
fn crashing(amp: f32) -> (f32, f32) {
    (560.0 - 160.0 * amp, 760.0 - 260.0 * amp)
}

/// How long from one wave's start to the next's, at `pace`, as `style`
/// plays (design §6.9): the sooner the faster a drive rises.
fn every(style: BarWaves, pace: Pace) -> Duration {
    millis_of(match (style, pace) {
        (_, Pace::Falling) => DIP_EVERY,
        (BarWaves::Crashing, Pace::Rising(amp)) => 2300.0 - 1500.0 * amp,
        (BarWaves::Ripple, Pace::Rising(amp)) => 2100.0 - 1400.0 * amp,
    })
}

/// How long a wave at `pace` shows, as `style` plays (design §6.9): a dip's
/// or a ripple's run, or a crest's run and its break, which the next cuts
/// short if it comes sooner.
fn shows(style: BarWaves, pace: Pace) -> Duration {
    match (style, pace) {
        (_, Pace::Falling) => millis_of(DIP_RUN),
        (BarWaves::Crashing, Pace::Rising(amp)) => {
            let (run, crash) = crashing(amp);
            millis_of(run + crash).min(every(style, pace))
        }
        (BarWaves::Ripple, Pace::Rising(_)) => millis_of(RIPPLE_RUN),
    }
}

/// How long after a wave at `was` began the next, at `pace`, begins: once
/// it has shown, and no sooner than waves at `pace` come.
fn after(style: BarWaves, was: Pace, pace: Pace) -> Duration {
    shows(style, was).max(every(style, pace))
}

/// What a wave changes of a block of a drive's bar: its glyph's colour,
/// its background, either or both (design §6.9). Never its glyph.
pub(crate) type Tint = (Option<Color>, Option<Color>);

/// The colours a drive's bar's waves are drawn from, exact ones: its
/// drive's, the crest's glow, the spray's tint, its empty part's and what
/// it sits on.
#[derive(Debug, Clone, Copy)]
struct BarColours {
    drive: Color,
    glow: Color,
    tint: Color,
    empty: Color,
    shade: Color,
}

/// `way` of the way from `from` to `to`, from 0 to 1, blended in RGB.
fn mix(from: Color, to: Color, way: f32) -> Color {
    ColorSpace::Rgb.lerp(&from, &to, way)
}

/// What a wave at `pace`, `into` it, does to the blocks of a drive's bar
/// that fills `thirds` of a block, as `style` plays (design §6.9).
fn wave_tints(
    (style, pace): (BarWaves, Pace),
    into: Duration,
    thirds: u8,
    colours: BarColours,
) -> [Tint; BLOCKS] {
    let mut tints = [(None, None); BLOCKS];
    // The bar's end: its last block filled in full or in part.
    let Some(end) = usize::from(thirds)
        .min(BLOCKS * 3)
        .div_ceil(3)
        .checked_sub(1)
    else {
        return tints;
    };
    let into = into.as_micros() as f32 / 1000.0;
    let (drive, glow) = (colours.drive, colours.glow);
    // A crest of `height` at `x`, a block and a bit wide, lighting the
    // filled blocks under it towards the glow, and their backgrounds
    // towards the drive's colour.
    let crest = |tints: &mut [Tint; BLOCKS], x: f32, (height, amp): (f32, f32)| {
        for (i, tint) in tints.iter_mut().enumerate().take(end + 1) {
            let under = 1.0 - (i as f32 - x).abs() / 1.4;
            if under > 0.0 {
                *tint = (
                    Some(mix(drive, glow, height * under)),
                    Some(mix(colours.shade, drive, 0.42 * amp * under)),
                );
            }
        }
    };
    match (style, pace) {
        (BarWaves::Crashing, Pace::Rising(amp)) => {
            let (run, crash) = crashing(amp);
            let height = 0.3 + 0.7 * amp;
            if into < run {
                // Up the bar from before its first block to its end.
                let x = -1.0 + into / run * (end + 1) as f32;
                crest(&mut tints, x, (height, amp));
            } else {
                // It breaks at once, the end flaring and settling, and its
                // spray runs on past the level, slowing, and dies away.
                let broken = ((into - run) / crash).min(1.0);
                let settled = Interpolation::QuadOut.alpha(broken);
                let flare = 1.0 - settled;
                tints[end] = (
                    Some(mix(drive, glow, (height + 0.2).min(1.0) * flare)),
                    Some(mix(colours.shade, drive, (0.35 + 0.35 * amp) * flare)),
                );
                let reach = spray_reach(amp);
                let front = settled * (reach as f32 + 0.6);
                for (j, fades) in SPRAY_FADES.iter().enumerate().take(reach) {
                    let Some(tint) = tints.get_mut(end + 1 + j) else {
                        break;
                    };
                    let at = (1.0 - ((j + 1) as f32 - front).abs() / 1.1).max(0.0);
                    let spray = (0.35 + 0.6 * amp) * fades * at * (1.0 - broken * 0.6);
                    if spray > 0.0 {
                        tint.0 = Some(mix(colours.empty, colours.tint, spray));
                    }
                }
            }
        }
        (BarWaves::Ripple, Pace::Rising(amp)) => {
            // Up the bar from before its first block and off its end.
            let x = into / RIPPLE_RUN * (end + 2) as f32 - 1.0;
            crest(&mut tints, x, (0.3 + 0.7 * amp, amp));
        }
        (_, Pace::Falling) => {
            // A faint dark dip from past the end back down the bar, slowly.
            let x = (end + 1) as f32 - into / DIP_RUN * (end + 2) as f32;
            for (i, tint) in tints.iter_mut().enumerate().take(end + 1) {
                let under = 1.0 - (i as f32 - x).abs() / 1.6;
                if under > 0.0 {
                    tint.0 = Some(mix(drive, colours.shade, 0.5 * under));
                }
            }
        }
    }
    tints
}

/// How many empty blocks past the level a breaking wave's spray reaches on
/// a drive's bar rising at `amp` (design §6.9): one, two from half its
/// most, and three from nine tenths.
fn spray_reach(amp: f32) -> usize {
    if amp >= 0.9 {
        3
    } else if amp >= 0.5 {
        2
    } else {
        1
    }
}

/// A tab opened, sweeping in from one side of the inspector, from when it
/// began.
#[derive(Debug, Clone, Copy)]
struct Sweep {
    from_left: bool,
    began: Duration,
}

/// How a sprite or an egg on the map was drawn, for a death or a hatching
/// to come.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Seen {
    /// A sprite: its tile, its own glyph and the colour it was drawn in.
    Sprite {
        tile: Pos,
        symbol: char,
        colour: Color,
    },
    /// An egg on the map: its tile and colour.
    Egg { tile: Pos, colour: Color },
}

/// A death or a hatching, as the map plays it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Happening {
    /// Sprite `id` died on `tile`, last drawn as `symbol` in `colour`.
    Death {
        id: EntityId,
        tile: Pos,
        symbol: char,
        colour: Color,
    },
    /// Sprite `id` hatched from the egg on `tile`, which was drawn in `egg`,
    /// and was `flashed` if the screen saw it flashing as it hatched.
    Hatching {
        id: EntityId,
        tile: Pos,
        egg: Color,
        flashed: bool,
    },
}

impl Happening {
    /// The sprite it's about and the tile it plays on.
    fn at(self) -> (EntityId, Pos) {
        match self {
            Happening::Death { id, tile, .. } | Happening::Hatching { id, tile, .. } => (id, tile),
        }
    }
}

/// A death or a hatching, from when it began, in the app's `running_for`,
/// for as long as it lasts.
#[derive(Debug, Clone, Copy)]
struct Moment {
    happening: Happening,
    began: Duration,
    lasts: Duration,
}

impl Motion {
    /// Motion on, as every app starts (design §6.9), with nothing seen or
    /// playing yet.
    pub(crate) fn new() -> Motion {
        Motion {
            on: true,
            seen: BTreeMap::new(),
            moments: VecDeque::new(),
            flashing: BTreeMap::new(),
            sweep: None,
            screen: Screen::Normal,
            opened: None,
            closed: None,
            bars: None,
            trail: None,
            pan: None,
        }
    }

    /// Whether motion is on.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// Turns motion on or off. Off, it forgets what it saw and what was
    /// playing, so the screen is drawn as it was before there was motion.
    /// Says whether that ended a pan, the moment the Cursor goes to the
    /// tile under a still pointer, as when one ends (design §6.9).
    pub(crate) fn switch(&mut self, on: bool) -> bool {
        self.on = on;
        let ended_a_pan = !on && self.pan.is_some();
        if !on {
            self.seen.clear();
            self.moments.clear();
            self.flashing.clear();
            self.sweep = None;
            self.opened = None;
            self.closed = None;
            self.bars = None;
            self.trail = None;
            self.pan = None;
        }
        ended_a_pan
    }

    /// Starts tab `to` sweeping in at `now`, opened from `from` while the
    /// inspector shows (design §6.9): from the side it's on, from the
    /// right when it's right of the old one and from the left when it's
    /// left of it. Reopening the tab already open sweeps nothing.
    pub(crate) fn opened_tab(&mut self, from: Tab, to: Tab, now: Duration) {
        let place = |tab: Tab| Tab::ALL.iter().position(|&one| one == tab);
        if self.on && from != to {
            self.sweep = Some(Sweep {
                from_left: place(to) < place(from),
                began: now,
            });
        }
    }

    /// Notes how each sprite and egg is drawn, after the ticks just taken
    /// in (design §6.9).
    pub(crate) fn note(&mut self, seen: BTreeMap<EntityId, Seen>) {
        if self.on {
            self.seen = seen;
        }
    }

    /// What `event` sets off, while motion is on, and how long it lasts at
    /// 1×: a death, for a sprite seen on the map, or a hatching, for an egg
    /// seen there. An event about a sprite or an egg not seen sets off
    /// nothing.
    fn set_off_by(&self, event: &EventKind) -> Option<(Happening, Duration)> {
        if !self.on {
            return None;
        }
        match *event {
            EventKind::Died { id, .. } => match self.seen.get(&id) {
                Some(&Seen::Sprite {
                    tile,
                    symbol,
                    colour,
                }) => Some((
                    Happening::Death {
                        id,
                        tile,
                        symbol,
                        colour,
                    },
                    DEATH,
                )),
                _ => None,
            },
            EventKind::Hatched { id, .. } => match self.seen.get(&id) {
                Some(&Seen::Egg { tile, colour }) => Some((
                    Happening::Hatching {
                        id,
                        tile,
                        egg: colour,
                        flashed: self.flashing.contains_key(&id),
                    },
                    HATCHING,
                )),
                _ => None,
            },
            _ => None,
        }
    }

    /// If `event` sets off a death or a hatching, its sprite and tile, and
    /// those of each one playing: what `took_in` asks could be drawn. None
    /// if it sets off nothing.
    pub(crate) fn at_stake(&self, event: &EventKind) -> Vec<(EntityId, Pos)> {
        let Some((new, _)) = self.set_off_by(event) else {
            return Vec::new();
        };
        self.moments
            .iter()
            .map(|moment| moment.happening)
            .chain([new])
            .map(Happening::at)
            .collect()
    }

    /// Starts what `event` sets off at `now`, at `speed`, as `set_off_by`
    /// says. Only those that `could_draw` says could be drawn just now
    /// count towards how many play at once: one of them takes the place of
    /// the oldest of them once as many play as may (design §6.9), so a
    /// death or hatching the player can't see never takes the place of one
    /// they can.
    pub(crate) fn took_in(
        &mut self,
        event: &EventKind,
        now: Duration,
        speed: Speed,
        could_draw: impl Fn(EntityId, Pos) -> bool,
    ) {
        let Some((happening, full)) = self.set_off_by(event) else {
            return;
        };
        if let Happening::Hatching { id, .. } = happening {
            self.flashing.remove(&id);
        }
        self.forget_finished(now);
        let drawn = |moment: &Moment| {
            let (id, tile) = moment.happening.at();
            could_draw(id, tile)
        };
        let (id, tile) = happening.at();
        if could_draw(id, tile) {
            while self.moments.iter().filter(|&moment| drawn(moment)).count() >= most(speed) {
                let Some(oldest) = self.moments.iter().position(drawn) else {
                    break;
                };
                self.moments.remove(oldest);
            }
        }
        self.moments.push_back(Moment {
            happening,
            began: now,
            lasts: at_speed(full, speed),
        });
    }

    /// Forgets the deaths and hatchings over by `now`.
    pub(crate) fn forget_finished(&mut self, now: Duration) {
        self.moments
            .retain(|moment| age(moment.began, now) < moment.lasts);
        if let Some(trail) = &mut self.trail {
            trail.left.retain(|left| age(left.began, now) < left.lasts);
        }
        self.sweep = self.sweeping(now);
        self.opened = self.opening(now);
        if self.closing(now).is_none() {
            self.closed = None;
        }
    }

    /// Notes what fills the screen besides the map just now, `screen`, at
    /// `now`. Whatever opens fades in, and the pop-ups that close fade out,
    /// as the last frame drew them, which `drawn` gives, and what was under
    /// them fades back (design §6.9). Only a change does, so turning motion
    /// on fades nothing.
    pub(crate) fn saw(
        &mut self,
        screen: Screen,
        drawn: impl FnOnce() -> Vec<Buffer>,
        now: Duration,
    ) {
        if screen == self.screen {
            return;
        }
        self.screen = screen;
        if self.on {
            self.opened = Some(now);
            self.closed = Some((drawn(), now));
        }
    }

    /// Notes the eggs on the map within `FLASH_FROM` ticks of hatching,
    /// after the ticks just taken in at `now`: `due`, each by ID with the
    /// ticks it has left. Each starts to flash as it's first seen so, one
    /// already flashing goes on from where it had got at the period of the
    /// ticks it has left, and one that no longer is, held or stood on,
    /// stops (design §6.9).
    pub(crate) fn note_flashing(&mut self, due: &[(EntityId, u64)], now: Duration) {
        if !self.on {
            self.flashing.clear();
            return;
        }
        self.flashing
            .retain(|id, _| due.iter().any(|&(egg, _)| egg == *id));
        for &(id, left) in due {
            self.flashing
                .entry(id)
                .and_modify(|egg| *egg = egg.taken_on(left, now))
                .or_insert(Flashing::new(left, now));
        }
    }

    /// Notes the selected sprite's drives' levels, after `ticks` ticks just
    /// taken in at `now` and `speed`, as `style` plays its bars' waves:
    /// `sprite`, with its ID, or none if no sprite in the world is
    /// selected. A bar whose level changed eases to it from where it's
    /// drawn, but at 4× and up, where it jumps, and a sprite seen for the
    /// first time shows its levels at once. Each drive's change over the
    /// last tick sets its bar's waves, as each tick taken in does, and as
    /// a sprite's first sight does, which no tick has shown yet (design
    /// §6.9).
    pub(crate) fn note_bars(
        &mut self,
        sprite: Option<(EntityId, Vec<DriveLevel>)>,
        ticks: usize,
        (now, speed): (Duration, Speed),
        style: BarWaves,
    ) {
        let Some((id, drives)) = sprite.filter(|_| self.on) else {
            self.bars = None;
            return;
        };
        let eases = !thins_out(speed);
        let first_sight = self.bars.as_ref().is_none_or(|bars| bars.sprite != id);
        let bars = match &mut self.bars {
            Some(bars) if !first_sight => bars,
            _ => self.bars.insert(Bars {
                sprite: id,
                drives: BTreeMap::new(),
            }),
        };
        for DriveLevel {
            name,
            thirds: to,
            change,
        } in drives
        {
            let bar = bars.drives.entry(name).or_insert(Bar::at(to, now));
            if bar.to != to {
                bar.from = if eases { bar.shown(now) } else { to };
                bar.to = to;
                bar.began = now;
            }
            if first_sight || ticks > 0 {
                bar.waves.took_in(change, now, style, eases);
            }
        }
    }

    /// Forgets the bars, as another sprite is selected, whose show their
    /// levels at once.
    pub(crate) fn forget_bars(&mut self) {
        self.bars = None;
    }

    /// Notes the tile the selected sprite is on, after the ticks just
    /// taken in at `now` and `speed`: `sprite`, with its ID, or none if no
    /// sprite in the world is selected. A tile it has left since it was
    /// last seen joins its trail, the newest of at most `TRAIL_TILES`
    /// (design §6.9).
    pub(crate) fn note_trail(
        &mut self,
        sprite: Option<(EntityId, Pos)>,
        now: Duration,
        speed: Speed,
    ) {
        let Some((id, on)) = sprite.filter(|_| self.on) else {
            self.trail = None;
            return;
        };
        match &mut self.trail {
            Some(trail) if trail.sprite == id => {
                if trail.on == on {
                    return;
                }
                let left = std::mem::replace(&mut trail.on, on);
                trail.left.retain(|old| old.tile != left && old.tile != on);
                trail.left.push_back(Left {
                    tile: left,
                    began: now,
                    lasts: at_speed(TRAIL, speed),
                });
                while trail.left.len() > TRAIL_TILES {
                    trail.left.pop_front();
                }
            }
            _ => {
                self.trail = Some(Trail {
                    sprite: id,
                    on,
                    left: VecDeque::new(),
                });
            }
        }
    }

    /// Forgets the trail, as another sprite is selected (design §6.9).
    pub(crate) fn forget_trail(&mut self) {
        self.trail = None;
    }

    /// Notes the view centred on a sprite at `now`: the viewport moved
    /// from `was` to `to` while the map view was drawn from `drawn`. Moved
    /// more than `PAN_PAST` tiles, the map view pans there from where it's
    /// drawn, a pan already under way included; a shorter move leaves a pan
    /// under way to go on there, and otherwise jumps (design §6.9).
    pub(crate) fn centred(&mut self, (was, drawn, to): (Origin, Origin, Origin), now: Duration) {
        if self.on && (to.x - was.x).abs().max((to.y - was.y).abs()) > PAN_PAST {
            self.pan = Some(Pan {
                from: drawn,
                began: now,
            });
        }
    }

    /// Ends the pan under way, if one is, as the view jumps: scrolled, the
    /// tiles squared, or the screen a new size (design §6.9).
    pub(crate) fn stop_pan(&mut self) {
        self.pan = None;
    }

    /// Where the map view is drawn from at `now` and `speed`, the viewport
    /// at `viewport`: where the pan has reached, in whole tiles, while one
    /// is under way, and the viewport otherwise (design §6.9).
    pub(crate) fn shown_from(&self, viewport: Origin, now: Duration, speed: Speed) -> Origin {
        let Some(pan) = self.panning(now, speed) else {
            return viewport;
        };
        let way = eased(age(pan.began, now), PAN);
        let along = |from: i32, to: i32| from + ((to - from) as f32 * way).round() as i32;
        Origin {
            x: along(pan.from.x, viewport.x),
            y: along(pan.from.y, viewport.y),
        }
    }

    /// The pan under way at `now` and `speed`, if one is: at 4× and up,
    /// the view jumps instead (design §6.9).
    fn panning(&self, now: Duration, speed: Speed) -> Option<Pan> {
        self.pan
            .filter(|pan| !thins_out(speed) && age(pan.began, now) < PAN)
    }

    /// Whether a pan is under way at `now` and `speed`.
    pub(crate) fn pans(&self, now: Duration, speed: Speed) -> bool {
        self.panning(now, speed).is_some()
    }

    /// Whether the pan has just ended by `now` and `speed`, which forgets
    /// it: the moment the Cursor goes to the tile under a still pointer
    /// (design §6.9).
    pub(crate) fn pan_ended(&mut self, now: Duration, speed: Speed) -> bool {
        let ended = self.pan.is_some() && !self.pans(now, speed);
        if ended {
            self.pan = None;
        }
        ended
    }

    /// How many thirds of a block drive `drive`'s bar fills on sprite `id`'s
    /// Body tab at `now` and `speed`, its level `live` thirds: easing to it,
    /// if it's the level last taken in, and otherwise at it.
    pub(crate) fn bar(
        &self,
        id: EntityId,
        drive: &str,
        live: u8,
        now: Duration,
        speed: Speed,
    ) -> u8 {
        if !self.on || thins_out(speed) {
            return live;
        }
        self.bars
            .as_ref()
            .filter(|bars| bars.sprite == id)
            .and_then(|bars| bars.drives.get(drive))
            .filter(|bar| bar.to == live)
            .map_or(live, |bar| bar.shown(now))
    }

    /// The wave showing on drive `drive`'s bar on sprite `id`'s Body tab at
    /// `now` and `speed`, as `style` plays, and how far into it: none at 4×
    /// and up (design §6.9).
    fn bar_wave(
        &self,
        (id, drive): (EntityId, &str),
        (now, speed): (Duration, Speed),
        style: BarWaves,
    ) -> Option<(Pace, Duration)> {
        if !self.on || thins_out(speed) {
            return None;
        }
        self.bars
            .as_ref()
            .filter(|bars| bars.sprite == id)
            .and_then(|bars| bars.drives.get(drive))
            .and_then(|bar| bar.waves.showing(now, style))
    }

    /// When what's up opened, while it fades in at `now`.
    fn opening(&self, now: Duration) -> Option<Duration> {
        self.opened.filter(|&began| age(began, now) < POP_UP)
    }

    /// How the pop-ups that just closed were last drawn, and when they
    /// closed, while they fade out and what was under them fades back at
    /// `now`.
    fn closing(&self, now: Duration) -> Option<(&[Buffer], Duration)> {
        self.closed
            .as_ref()
            .filter(|(_, began)| age(*began, now) < POP_UP)
            .map(|(drawn, began)| (drawn.as_slice(), *began))
    }

    /// The tab sweeping in at `now`, if one is.
    fn sweeping(&self, now: Duration) -> Option<Sweep> {
        self.sweep.filter(|sweep| age(sweep.began, now) < TAB_SWEEP)
    }

    /// The deaths and hatchings playing at `now`, oldest first, of those
    /// `could_draw` says could be drawn just now: the newest of them, as
    /// many as may play at once at `speed`.
    fn playing(
        &self,
        now: Duration,
        speed: Speed,
        could_draw: impl Fn(EntityId, Pos) -> bool,
    ) -> Vec<&Moment> {
        let drawn: Vec<&Moment> = self
            .moments
            .iter()
            .filter(|moment| age(moment.began, now) < moment.lasts)
            .filter(|moment| {
                let (id, tile) = moment.happening.at();
                could_draw(id, tile)
            })
            .collect();
        let skip = drawn.len().saturating_sub(most(speed));
        drawn.into_iter().skip(skip).collect()
    }
}

/// How each sprite and egg on `world`'s map is drawn by `app` just now: a
/// sprite as `ui::draw_tiles` draws it when it isn't emoting, and an egg
/// as its object type's look.
pub(crate) fn seen(app: &App, world: &World) -> BTreeMap<EntityId, Seen> {
    let lineage = ui::lineage_on_map(app, world);
    let mut seen = BTreeMap::new();
    for sprite in world.sprites() {
        let glyph = ui::sprite_look(app, &sprite, lineage.as_ref());
        seen.insert(
            sprite.id(),
            Seen::Sprite {
                tile: sprite.pos(),
                symbol: glyph.symbol,
                colour: glyph.fg,
            },
        );
    }
    for egg in world.objects().filter(|object| object.is_egg()) {
        let glyph = app.theme.object_glyph(egg.type_name(), egg.visual_state());
        seen.insert(
            egg.id(),
            Seen::Egg {
                tile: egg.pos(),
                colour: glyph.fg,
            },
        );
    }
    seen
}

/// A screen cell of the map view that `ui::draw_tiles` drew an item lying
/// on the map in, with the item's ID: an egg there may flash (design §6.9).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Lying {
    pub(crate) cell: Position,
    pub(crate) id: EntityId,
}

/// Whether `app` could draw a death or a hatching of sprite `id` on `tile`
/// just now (design §6.9): on the map view, not under the inspector open
/// over it (§6.1), and only if the policy lets what tells the player
/// something about the sprite show (§6.4). Only those count towards how
/// many play at once.
pub(crate) fn could_draw(app: &App, id: EntityId, tile: Pos) -> bool {
    app.cell_of(tile).is_some()
        && !app.under_the_inspector(tile)
        && app.can_view(Panel::Emotes, Subject::Sprite(id))
}

/// What drive `drive`'s bar's waves do to its blocks on sprite `id`'s Body
/// tab just now, `thirds` of a block filled, its filled part drawn in
/// `filled` and its empty part in `empty` (design §6.9): for each block,
/// the colour its glyph and its background take, where a wave changes
/// them. None while no wave shows, nor at 4× and up, nor in a palette with
/// no colour, since waves are nothing but colour.
pub(crate) fn bar_waves(
    app: &App,
    (id, drive): (EntityId, &str),
    thirds: u8,
    (filled, empty): (Color, Color),
) -> Option<[Tint; BLOCKS]> {
    if !colours_play(app) {
        return None;
    }
    let look = app.theme.motion();
    let at = (app.running_for(), app.clock.speed());
    let (pace, into) = app.motion_kept().bar_wave((id, drive), at, look.bar)?;
    // Exact colours to blend, the terminal's own as the palette draws its
    // text and its dark, and the crest's glow and the spray's tint towards
    // white, as Campbell draws it (§6.2).
    let dark = campbell::rgb(look.dark).unwrap_or(campbell::BACKGROUND);
    let text = own_text(app);
    let drive_colour = campbell::exact(filled, text);
    let white = campbell::exact(Color::White, text);
    let shade = app
        .theme
        .panels()
        .map_or(Color::Reset, |panels| panels.shade);
    let colours = BarColours {
        drive: drive_colour,
        glow: mix(drive_colour, white, 0.55),
        tint: mix(drive_colour, white, 0.2),
        empty: campbell::exact(empty, text),
        shade: campbell::exact(shade, dark),
    };
    Some(wave_tints((look.bar, pace), into, thirds, colours))
}

/// Plays the map's motion over the tiles `ui::draw_tiles` drew in `inner`,
/// before the marks go over them (design §6.9): the trail, each egg about
/// to hatch flashing among the items `lying` there, and then each death
/// and hatching, oldest first. `bare` are the screen cells it drew bare
/// ground in, with no sprite, object or mark, where a death's ghost and a
/// hatching's shell show. In a palette with no colour only the hatching's
/// cracked egg and shell play, which are characters (§6.2). Last, a cell in
/// reverse video shows its glows through it.
pub(crate) fn play_on_map(
    buf: &mut Buffer,
    inner: Rect,
    app: &App,
    world: &World,
    (bare, lying): (&[Position], &[Lying]),
) {
    if !app.motion() {
        return;
    }
    let on = OnMap {
        inner,
        app,
        world,
        look: app.theme.motion(),
        bare,
        colours: colours_play(app),
        reversed: RefCell::default(),
    };
    let now = app.running_for();
    if on.colours {
        trail(buf, &on, now);
        flash(buf, &on, lying, now);
    }
    let drawn = |id, tile| could_draw(app, id, tile);
    for moment in app.motion_kept().playing(now, app.clock.speed(), drawn) {
        let age = age(moment.began, now);
        match moment.happening {
            Happening::Death {
                tile,
                symbol,
                colour,
                ..
            } if on.colours => death(buf, &on, tile, (symbol, colour), age, moment.lasts),
            Happening::Death { .. } => {}
            Happening::Hatching {
                id,
                tile,
                egg,
                flashed,
            } => {
                let young = Young { id, egg, flashed };
                hatching(buf, &on, tile, young, moment, age);
            }
        }
    }
    on.reversed.take().show(buf, own_text(app));
}

/// Plays the panels' motion over the finished screen once what can be
/// pointed at among `spots` is marked, before the tooltip, which doesn't
/// move (design §6.9): the tab sweeping in, where the inspector is drawn
/// just now, under any pop-up; then a pop-up that closed fading out to the
/// dark, as the last frame drew it, and what was under it fading back from
/// there, but where a pop-up is drawn now; and a pop-up that opened fading
/// in, but under the debug overlay, which doesn't fade.
pub(crate) fn play_on_panels(buf: &mut Buffer, app: &App, spots: &Spots) {
    if !colours_play(app) {
        return;
    }
    let (kept, now) = (app.motion_kept(), app.running_for());
    let dark = campbell::rgb(app.theme.motion().dark).unwrap_or(campbell::BACKGROUND);
    let own = (own_text(app), dark);
    let [r, g, b] = dark;
    let dark_colour = Color::Rgb(r, g, b);
    let under_a_pop_up = |cell: Position| spots.under_a_pop_up(cell);
    if let (Some(sweep), Some(inspector)) = (kept.sweeping(now), app.inspector_area()) {
        // Out of the panel's shade, as Campbell draws it, or the dark
        // where it has none.
        let faded = app
            .theme
            .panels()
            .map_or(dark_colour, |panels| campbell::exact(panels.shade, dark));
        let direction = if sweep.from_left {
            tachyonfx::Motion::LeftToRight
        } else {
            tachyonfx::Motion::RightToLeft
        };
        let effect = fx::sweep_in(direction, SWEEP_EDGE, 0, faded, timer(TAB_SWEEP));
        let age = age(sweep.began, now);
        let body = inspector::tab_area(inspector);
        play_over(effect, age, buf, body, own, under_a_pop_up);
    }
    let fade_in = |lasts| fx::fade_from(dark_colour, dark_colour, timer(lasts));
    if let Some((drawn, began)) = kept.closing(now) {
        // Out to the dark and back, so the pop-up's characters give way to
        // what was under it while all is dark.
        let (age, half) = (age(began, now), POP_UP / 2);
        for pop_up in drawn {
            let area = pop_up.area;
            if age < half {
                for cell in area.intersection(buf.area).positions() {
                    if !under_a_pop_up(cell) {
                        buf[cell] = pop_up[cell].clone();
                    }
                }
                let fade_out = fx::fade_to(dark_colour, dark_colour, timer(half));
                play_over(fade_out, age, buf, area, own, under_a_pop_up);
            } else {
                play_over(fade_in(half), age - half, buf, area, own, under_a_pop_up);
            }
        }
    }
    if let Some(began) = kept.opening(now) {
        for area in spots.fading() {
            let over_it = |cell: Position| !spots.fades_at(cell);
            play_over(fade_in(POP_UP), age(began, now), buf, area, own, over_it);
        }
    }
}

/// Plays `effect` over `area` of `buf`, `age` into it, but on the cells
/// `kept` keeps as they are (design §6.9). It blends exact colours in RGB:
/// a named one as Campbell draws it, and the terminal's own text colour
/// and background as `text` and `dark`, as the palette draws its text and
/// its dark. What the effect leaves as it was keeps the colour it had,
/// named or the terminal's own, so a fade that's played out, or a cell it
/// doesn't reach, is drawn as before.
fn play_over(
    effect: Effect,
    age: Duration,
    buf: &mut Buffer,
    area: Rect,
    (text, dark): (Rgb, Rgb),
    kept: impl Fn(Position) -> bool,
) {
    let area = area.intersection(buf.area);
    let exact = |cell: &Cell| {
        (
            campbell::exact(cell.fg, text),
            campbell::exact(cell.bg, dark),
        )
    };
    let drawn: Vec<(Position, Cell)> = area
        .positions()
        .map(|cell| (cell, buf[cell].clone()))
        .collect();
    for (cell, was) in &drawn {
        let (fg, bg) = exact(was);
        buf[*cell].set_fg(fg).set_bg(bg);
    }
    let mut effect = effect.with_area(area).with_color_space(ColorSpace::Rgb);
    effect.process(millis(age), buf, area);
    for (cell, was) in drawn {
        if kept(cell) {
            buf[cell] = was;
            continue;
        }
        let (fg, bg) = exact(&was);
        let now = &mut buf[cell];
        if now.fg == fg {
            now.fg = was.fg;
        }
        if now.bg == bg {
            now.bg = was.bg;
        }
    }
}

/// The map view that motion plays over: where its tiles are drawn,
/// `inner`, the app and world drawn there, the theme's colours for motion,
/// the cells that show bare ground, whether motion's colours play, and the
/// cells in reverse video that have glowed so far.
struct OnMap<'a> {
    inner: Rect,
    app: &'a App,
    world: &'a World,
    look: MotionLook,
    bare: &'a [Position],
    colours: bool,
    reversed: RefCell<Reversed>,
}

impl OnMap<'_> {
    /// The screen cells of `tile`, as the map view draws it: two while
    /// tiles are two cells wide, and none out of view.
    fn cells(&self, tile: Pos) -> Vec<Position> {
        let Some(first) = self.app.cell_of(tile) else {
            return Vec::new();
        };
        let corner = self.inner.as_position();
        (0..self.app.tile_width())
            .map(|x| Position::new(first.x + x, first.y))
            .filter(|&cell| {
                self.inner.contains(cell) && self.app.place_from(corner, cell) == Place::Tile(tile)
            })
            .collect()
    }

    /// Whether screen cell `cell` shows bare ground.
    fn shows_bare(&self, cell: Position) -> bool {
        self.bare.contains(&cell)
    }

    /// The tiles with looks of their own, which the trail and a hatching's
    /// shell leave as they are (design §6.9): the Cursor's and the attended
    /// one.
    fn own_looks(&self) -> [Option<Pos>; 2] {
        [
            self.app.shown_cursor(),
            ui::attended_by(self.app, self.world),
        ]
    }
}

/// Tints the tiles the selected sprite has just left, each fading back to
/// its own background from when it was left (design §6.9): on any tile but
/// the Cursor's and the attended one, which have looks of their own, nor
/// the sprite's own, which leaves the trail as it steps back onto it. It
/// asks the policy first, as the map's markers do (§6.4).
fn trail(buf: &mut Buffer, on: &OnMap, now: Duration) {
    let Some(trail) = &on.app.motion_kept().trail else {
        return;
    };
    if ui::marked(on.app) != Some(trail.sprite) {
        return;
    }
    let own_looks = on.own_looks();
    let dark = campbell::rgb(on.look.dark).unwrap_or(campbell::BACKGROUND);
    let tint = campbell::exact(on.look.trail, dark);
    for left in &trail.left {
        let age = age(left.began, now);
        if age >= left.lasts || own_looks.contains(&Some(left.tile)) {
            continue;
        }
        let back = eased(age, left.lasts);
        for cell in on.cells(left.tile) {
            let own = campbell::exact(buf[cell].bg, dark);
            let bg = ColorSpace::Rgb.lerp(&tint, &own, back);
            play(fx::paint_bg(bg, timer(left.lasts)), age, buf, cell);
        }
    }
}

/// Plays each egg about to hatch flashing, among the items `lying` on the
/// map (design §6.9): from `FLASH_FROM` ticks before it hatches, its tile
/// glows towards the sparks' colour at once and fades back, quick and then
/// slowing, again and again until the young appears, each flash sooner and
/// brighter than the last. Colour only: its glyph stays. It asks the policy
/// first, as its hatching will (§6.4).
fn flash(buf: &mut Buffer, on: &OnMap, lying: &[Lying], now: Duration) {
    let flashing = &on.app.motion_kept().flashing;
    for spot in lying {
        let Some(egg) = flashing.get(&spot.id) else {
            continue;
        };
        if !on.app.can_view(Panel::Emotes, Subject::Sprite(spot.id)) {
            continue;
        }
        let (into, period) = (egg.reached(now), flash_period(egg.left));
        let faded = eased(into, period);
        glow(
            buf,
            on,
            spot.cell,
            on.look.spark,
            (flash_strength(egg.left), faded),
        );
    }
}

/// Glows screen cell `cell` towards the colour `light`, `peak` of the way
/// at first, from 0 to 1, and back as it has `faded`, from 0 to 1 (design
/// §6.9): an egg's flash, or a death's. It glows the background behind the
/// cell's glyph, on top of any glow before it this frame. A cell in reverse
/// video is noted, to show its glows through it once they're all in.
fn glow(buf: &mut Buffer, on: &OnMap, cell: Position, light: Color, (peak, faded): (f32, f32)) {
    let dark = campbell::rgb(on.look.dark).unwrap_or(campbell::BACKGROUND);
    let own = campbell::exact(buf[cell].bg, dark);
    let light = campbell::exact(light, dark);
    buf[cell].set_bg(ColorSpace::Rgb.lerp(&own, &light, peak * (1.0 - faded)));
    if buf[cell].modifier.contains(Modifier::REVERSED) {
        on.reversed.borrow_mut().note(cell, own, faded);
    }
}

/// The cells in reverse video that glowed this frame (design §6.9): for
/// each, its background before any glow, and how far the freshest glow on
/// it has faded, from 0 to 1. A selected egg or sprite and the Cursor's
/// tile are drawn in reverse video (§6.2, §6.5, M2 design §10.7), which
/// shows the glyph's colour behind the glyph and the background only in
/// its strokes, where a glow would hardly show, so the glows on such a
/// cell take the reverse video's place instead.
#[derive(Debug, Default)]
struct Reversed(BTreeMap<Position, (Color, f32)>);

impl Reversed {
    /// Notes a glow on `cell`, whose background was `own` before it, that
    /// has `faded` so far.
    fn note(&mut self, cell: Position, own: Color, faded: f32) {
        let (_, freshest) = self.0.entry(cell).or_insert((own, faded));
        *freshest = freshest.min(faded);
    }

    /// Shows the glows on each cell noted through its reverse video, in
    /// `buf`, `text` being the terminal's own text colour.
    fn show(self, buf: &mut Buffer, text: Rgb) {
        for (cell, (own, faded)) in self.0 {
            let glyph = campbell::exact(buf[cell].fg, text);
            let (fill, strokes) = through_reverse_video(glyph, buf[cell].bg, own, faded);
            buf[cell].set_fg(fill).set_bg(strokes);
        }
    }
}

/// How a cell in reverse video shows its glows (design §6.9): the
/// foreground, which fills it, and the background, which draws the glyph's
/// strokes. While the freshest glow hasn't `faded` at all, the glyph is
/// drawn in its colour, `glyph`, on `glow`, the background the glows left,
/// as on a cell that isn't reversed; and once it has, the reverse video is
/// back, `glyph` filling the cell and `own`, its background before them,
/// drawing the strokes.
fn through_reverse_video(glyph: Color, glow: Color, own: Color, faded: f32) -> (Color, Color) {
    (
        ColorSpace::Rgb.lerp(&glow, &glyph, faded),
        ColorSpace::Rgb.lerp(&glyph, &own, faded),
    )
}

/// `cell`'s colour as an exact one, for a fade to blend from (design §6.9),
/// the terminal's own as `text`.
fn exact_fg(buf: &mut Buffer, cell: Position, text: Rgb) {
    let fg = campbell::exact(buf[cell].fg, text);
    buf[cell].set_fg(fg);
}

/// Plays a death on `tile`, `age` into it, for as long as it `lasts`
/// (design §6.9): the tile flashes and fades back, and the
/// ghost, the sprite's glyph `symbol` in the `colour` it was drawn in,
/// fades to ash, while the tile shows bare ground.
fn death(
    buf: &mut Buffer,
    on: &OnMap,
    tile: Pos,
    (symbol, colour): (char, Color),
    age: Duration,
    lasts: Duration,
) {
    let look = on.look;
    let text = own_text(on.app);
    let back = eased(age, lasts);
    for (n, cell) in on.cells(tile).into_iter().enumerate() {
        if on.shows_bare(cell) {
            let ghost = if n == 0 { symbol } else { ' ' };
            buf[cell]
                .set_char(ghost)
                .set_fg(campbell::exact(colour, text));
            let ash = campbell::exact(look.ash, text);
            play(fx::fade_to_fg(ash, timer(lasts)), age, buf, cell);
        }
        glow(buf, on, cell, look.flash, (1.0, back));
    }
}

/// The sprite a hatching hatched, the colour its egg was drawn in, and
/// whether it was flashing.
#[derive(Clone, Copy)]
struct Young {
    id: EntityId,
    egg: Color,
    flashed: bool,
}

/// `full`, how long a part of a hatching takes at 1×, in one that `lasts`:
/// half as long at 4× and up, as the hatching is (design §6.9).
fn paced(full: Duration, lasts: Duration) -> Duration {
    let ms = |duration: Duration| u32::try_from(duration.as_millis()).unwrap_or(u32::MAX);
    full * ms(lasts) / ms(HATCHING)
}

/// Plays a hatching from the egg on `tile`, `age` into `moment` (design
/// §6.9): for a moment the egg shows cracked where the young is drawn, and
/// then the young appears and the shell splits round the egg's tile. Where
/// motion's colours play, the young warms from the egg's colour as the
/// last flash of an egg that was flashing fades, and the tiles round the
/// egg spark at scattered moments and fade back.
fn hatching(buf: &mut Buffer, on: &OnMap, tile: Pos, young: Young, moment: &Moment, age: Duration) {
    let lasts = moment.lasts;
    let crack = paced(CRACK, lasts);
    let sprite = on.world.sprite(young.id);
    let drawn = sprite
        .as_ref()
        .map_or(Vec::new(), |sprite| on.cells(sprite.pos()));
    let emoting = sprite.is_some_and(|sprite| ui::is_emote(on.app, &sprite));
    let egg = campbell::exact(young.egg, own_text(on.app));
    match age.checked_sub(crack) {
        None => cracked(buf, on, &drawn, (egg, young.flashed)),
        Some(since) => {
            if on.colours {
                let warming = lasts / 2;
                warms(buf, on, &drawn, (young, emoting), (since, warming));
            }
            shell(buf, on, tile, egg, (since, lasts));
        }
    }
    if on.colours {
        sparks(buf, on, tile, &drawn, moment, age);
    }
}

/// Draws the egg cracked on the screen cells `drawn` where the young is,
/// while the crack lasts (design §6.9): in its tile's first cell, as the
/// young's glyph is, in the egg's colour, `egg`. Where motion's colours
/// play, an egg that `flashed` glows where its last flash left it, seven
/// tenths of the way to the sparks' colour.
fn cracked(buf: &mut Buffer, on: &OnMap, drawn: &[Position], (egg, flashed): (Color, bool)) {
    let symbol = on.app.theme.shell().cracked;
    for (n, &cell) in drawn.iter().enumerate() {
        if n == 0 {
            buf[cell].set_char(symbol).set_fg(egg);
        }
        if flashed && on.colours {
            glow(buf, on, cell, on.look.spark, (FLASH_BRIGHTEST, 0.0));
        }
    }
}

/// Plays the young warming on the screen cells `drawn`, `since` it
/// appeared, over `warming` (design §6.9): from the egg's colour to its
/// own, but while it's `emoting`, whose emote keeps its colours, and the
/// last flash of an egg that was flashing fading from its tile.
fn warms(
    buf: &mut Buffer,
    on: &OnMap,
    drawn: &[Position],
    (young, emoting): (Young, bool),
    (since, warming): (Duration, Duration),
) {
    if since >= warming {
        return;
    }
    let text = own_text(on.app);
    if !emoting {
        let egg = campbell::exact(young.egg, text);
        for &cell in drawn {
            exact_fg(buf, cell, text);
            play(fx::fade_from_fg(egg, timer(warming)), since, buf, cell);
        }
    }
    if young.flashed {
        let faded = eased(since, warming);
        for &cell in drawn {
            glow(buf, on, cell, on.look.spark, (FLASH_BRIGHTEST, faded));
        }
    }
}

/// Plays a hatching's sparks, `age` into `moment` (design §6.9): the egg's
/// tile, `tile`, and the eight round it light up at scattered moments and
/// fade back, but the screen cells `drawn` where the young is, which warm
/// instead.
fn sparks(
    buf: &mut Buffer,
    on: &OnMap,
    tile: Pos,
    drawn: &[Position],
    moment: &Moment,
    age: Duration,
) {
    let lasts = moment.lasts;
    let text = own_text(on.app);
    let spark = campbell::exact(on.look.spark, text);
    let sparks_for = lasts * 2 / 5;
    let map = on.world.map();
    for y in tile.y.saturating_sub(1)..=tile.y.saturating_add(1) {
        for x in tile.x.saturating_sub(1)..=tile.x.saturating_add(1) {
            if x >= map.width() || y >= map.height() {
                continue;
            }
            let round = Pos { x, y };
            let starts = spark_starts(round, moment.began, lasts - sparks_for);
            let Some(into) = age.checked_sub(starts).filter(|&into| into < sparks_for) else {
                continue;
            };
            for cell in on.cells(round) {
                if !drawn.contains(&cell) {
                    exact_fg(buf, cell, text);
                    play(fx::fade_from_fg(spark, timer(sparks_for)), into, buf, cell);
                }
            }
        }
    }
}

/// A piece of a hatching's shell: where it lies from the egg's tile,
/// across and down, its character, and how far it has faded from the egg's
/// colour to its tile's own, from 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Piece {
    dx: i32,
    dy: i32,
    symbol: char,
    faded: f32,
}

/// The pieces of a hatching's shell drawn in `look`, `since` the egg
/// stopped showing cracked, in a hatching that `lasts` (design §6.9): its
/// halves either side of the egg's tile, a tile further out once they've
/// slid, and its crumbs on the tiles at its corners, each until it has
/// faded.
fn pieces(look: ShellLook, since: Duration, lasts: Duration) -> Vec<Piece> {
    let mut pieces = Vec::new();
    if let Some(faded) = fading(since, HALVES_FADE, lasts) {
        let out = if since < paced(HALVES_SLIDE, lasts) {
            1
        } else {
            2
        };
        pieces.push(Piece {
            dx: -out,
            dy: 0,
            symbol: look.left_half,
            faded,
        });
        pieces.push(Piece {
            dx: out,
            dy: 0,
            symbol: look.right_half,
            faded,
        });
    }
    if let Some(faded) = fading(since, CRUMBS_FADE, lasts) {
        let crumbs = [
            (-1, -1, look.up_left),
            (1, -1, look.up_right),
            (-1, 1, look.down_left),
            (1, 1, look.down_right),
        ];
        for (dx, dy, symbol) in crumbs {
            pieces.push(Piece {
                dx,
                dy,
                symbol,
                faded,
            });
        }
    }
    pieces
}

/// How far a piece of a hatching's shell that fades `from` one time `to`
/// another at 1× has faded `since` the egg stopped showing cracked, in a
/// hatching that `lasts`, from 0 to 1, evenly between: none once it has,
/// and it's gone (design §6.9).
fn fading(since: Duration, (from, to): (Duration, Duration), lasts: Duration) -> Option<f32> {
    let (from, to) = (paced(from, lasts), paced(to, lasts));
    let into = since.saturating_sub(from).as_micros() as f32;
    (since < to).then(|| into / (to - from).as_micros() as f32)
}

/// Plays a hatching's shell from the egg on `tile`, `since` the egg
/// stopped showing cracked, in a hatching that `lasts` (design §6.9): each
/// piece in its tile's first cell, as a mark is, the rest of a tile two
/// cells wide blank, fading from the egg's colour, `egg`, to the one its
/// tile's own glyph has. Only on tiles that show bare ground, and not the
/// Cursor's or the attended one, which have looks of their own: a piece
/// over anything else is left out.
fn shell(
    buf: &mut Buffer,
    on: &OnMap,
    tile: Pos,
    egg: Color,
    (since, lasts): (Duration, Duration),
) {
    let own_looks = on.own_looks();
    let text = own_text(on.app);
    let map = on.world.map();
    for piece in pieces(on.app.theme.shell(), since, lasts) {
        let x = u16::try_from(i32::from(tile.x) + piece.dx).ok();
        let y = u16::try_from(i32::from(tile.y) + piece.dy).ok();
        let Some(at) = x.zip(y).map(|(x, y)| Pos { x, y }) else {
            continue;
        };
        if at.x >= map.width() || at.y >= map.height() || own_looks.contains(&Some(at)) {
            continue;
        }
        let cells = on.cells(at);
        if cells.is_empty() || !cells.iter().all(|&cell| on.shows_bare(cell)) {
            continue;
        }
        for (n, cell) in cells.into_iter().enumerate() {
            if n > 0 {
                buf[cell].set_char(' ');
                continue;
            }
            let own = campbell::exact(buf[cell].fg, text);
            buf[cell]
                .set_char(piece.symbol)
                .set_fg(mix(egg, own, piece.faded));
        }
    }
}

/// When the spark on `tile` starts, into a hatching that began at `began`:
/// somewhere in the first `within`, by a pattern of the tile and the
/// moment, so the sparks scatter but every frame of it is the same on every
/// computer (design §6.9). It starts on a whole step of `SPARK_STEP_MS`,
/// so a clip's frames, which are whole tenths of a second apart, show a few
/// shades of a spark rather than new ones each time, and its colours fit
/// a GIF's 256.
fn spark_starts(tile: Pos, began: Duration, within: Duration) -> Duration {
    let moment = u64::try_from(began.as_millis()).unwrap_or(u64::MAX);
    let seed = moment ^ (u64::from(tile.x) << 32) ^ (u64::from(tile.y) << 48);
    let steps = u64::try_from(within.as_millis()).unwrap_or(u64::MAX) / SPARK_STEP_MS;
    Duration::from_millis(Pattern(seed).next() % (steps + 1) * SPARK_STEP_MS)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn it_thins_out_at_4x_and_up() {
        assert!(!thins_out(Speed::X2));
        assert!(thins_out(Speed::X4));
        assert!(thins_out(Speed::Max));
        assert_eq!(at_speed(DEATH, Speed::X1), Duration::from_millis(1200));
        assert_eq!(at_speed(HATCHING, Speed::X8), Duration::from_millis(500));
    }

    #[test]
    fn an_effect_eases_out_from_nothing_to_all_of_it() {
        let lasts = Duration::from_millis(400);
        assert_eq!(eased(Duration::ZERO, lasts), 0.0);
        assert_eq!(eased(Duration::from_millis(200), lasts), 0.75);
        assert_eq!(eased(Duration::from_millis(900), lasts), 1.0);
        assert_eq!(eased(Duration::ZERO, Duration::ZERO), 1.0);
        let began = Duration::from_millis(300);
        assert_eq!(age(began, Duration::from_millis(250)), Duration::ZERO);
        assert_eq!(
            age(began, Duration::from_millis(450)),
            Duration::from_millis(150)
        );
    }

    #[test]
    fn a_cell_in_reverse_video_shows_its_glows_by_the_freshest() {
        // Design §6.9: the glows add up on its background, and its
        // background before any of them, with the freshest glow's fading,
        // decide how far its reverse video is back.
        let (dark, glow) = (Color::Rgb(12, 12, 12), Color::Rgb(182, 166, 115));
        let egg = Color::Rgb(0xF9, 0xF1, 0xA5);
        let shown = |notes: &[(Color, f32)]| {
            let cell = Position::new(1, 0);
            let mut buf = Buffer::empty(Rect::new(0, 0, 3, 1));
            buf[cell].set_fg(Color::LightYellow).set_bg(glow);
            let mut reversed = Reversed::default();
            for &(own, faded) in notes {
                reversed.note(cell, own, faded);
            }
            reversed.show(&mut buf, FOREGROUND);
            (buf[cell].fg, buf[cell].bg)
        };
        let later = Color::Rgb(90, 80, 60);
        assert_eq!(shown(&[(dark, 0.6), (later, 0.0)]), (glow, egg));
        // Half way, rounded down: (182 + 249) / 2 is 215.
        let half = (Color::Rgb(215, 203, 140), Color::Rgb(130, 126, 88));
        assert_eq!(shown(&[(dark, 0.6), (later, 0.5), (later, 0.8)]), half);
        assert_eq!(through_reverse_video(egg, glow, dark, 1.0), (egg, dark));
    }

    /// Whether `a` and `b` are the same to a millionth.
    fn about(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn a_rise_shows_more_of_its_waves_the_faster_it_is_against_its_usual() {
        // Design §6.9: a quarter at its usual rise, .55 at 2.5 times it and
        // all of it from 5 times, evenly between, and none at all for none.
        for (ratio, expected) in [
            (0.0, 0.0),
            (0.5, 0.125),
            (1.0, 0.25),
            (1.75, 0.4),
            (2.5, 0.55),
            (3.75, 0.775),
            (5.0, 1.0),
            (12.0, 1.0),
        ] {
            assert!(about(amp(ratio), expected), "{ratio}: {}", amp(ratio));
        }
    }

    #[test]
    fn a_drive_s_usual_rise_is_the_average_of_its_last_hundred_rises() {
        let rising = |pace: Option<Pace>| match pace {
            Some(Pace::Rising(amp)) => amp,
            _ => panic!("not rising: {pace:?}"),
        };
        let mut waves = Waves::default();
        assert!(
            about(rising(waves.pace_of(0.004)), 0.25),
            "the first, as usual"
        );
        for _ in 0..99 {
            waves.pace_of(0.004);
        }
        // Falling and steady ticks aren't rises.
        assert_eq!(waves.pace_of(-0.01), Some(Pace::Falling));
        assert_eq!(waves.pace_of(0.00004), None);
        assert!(about(rising(waves.pace_of(0.02)), 1.0), "five times");
        for _ in 0..98 {
            waves.pace_of(0.02);
        }
        // One .004 left of the last hundred: .01984 a tick.
        assert!(about(rising(waves.pace_of(0.02)), amp(0.02 / 0.01984)));
        assert!(about(rising(waves.pace_of(0.02)), 0.25), "all .02 now");
    }

    #[test]
    fn a_hatching_s_shell_slides_and_fades_as_fauxpaw_picked_it() {
        // Design §6.9, from the mock-up Fauxpaw picked: from when the egg
        // stops showing cracked, its halves lie a tile either side, slide
        // a tile further out at 210 ms, hold the egg's colour to 110 ms
        // and fade evenly to their tile's by 700 ms, when they go; its
        // crumbs hold to 60 ms and fade by 600 ms. All in half the time at
        // 4× and up, as the hatching is.
        let look = crate::theme::Theme::cp437().shell();
        let shell = |since: Duration, lasts| {
            pieces(look, since, lasts)
                .into_iter()
                .map(|piece| (piece.dx, piece.dy, piece.symbol, piece.faded))
                .collect::<Vec<_>>()
        };
        let halves = |out: i32, faded: f32| [(-out, 0, '(', faded), (out, 0, ')', faded)];
        let crumbs = |faded: f32| {
            [
                (-1, -1, '`', faded),
                (1, -1, '\'', faded),
                (-1, 1, ',', faded),
                (1, 1, '.', faded),
            ]
        };
        let both = |(out, halves_faded): (i32, f32), crumbs_faded: f32| {
            let mut all = halves(out, halves_faded).to_vec();
            all.extend(crumbs(crumbs_faded));
            all
        };
        let same = |got: Vec<(i32, i32, char, f32)>, expected: Vec<(i32, i32, char, f32)>| {
            got.len() == expected.len()
                && got
                    .iter()
                    .zip(&expected)
                    .all(|(a, b)| (a.0, a.1, a.2) == (b.0, b.1, b.2) && (a.3 - b.3).abs() < 1e-4)
        };
        let ms = Duration::from_millis;
        for (lasts, halved) in [(HATCHING, 1), (HATCHING / 2, 2)] {
            let at = |n: u64| Duration::from_micros(n * 1000 / halved);
            let cases = [
                (at(0), both((1, 0.0), 0.0)),
                (at(60), both((1, 0.0), 0.0)),
                (at(110), both((1, 0.0), 50.0 / 540.0)),
                (at(209), both((1, 99.0 / 590.0), 149.0 / 540.0)),
                (at(210), both((2, 100.0 / 590.0), 0.277_777_8)),
                (at(330), both((2, 220.0 / 590.0), 0.5)),
                (at(405), both((2, 0.5), 345.0 / 540.0)),
                (at(599), both((2, 489.0 / 590.0), 539.0 / 540.0)),
                (at(600), halves(2, 490.0 / 590.0).to_vec()),
                (at(699), halves(2, 589.0 / 590.0).to_vec()),
                (at(700), Vec::new()),
                (ms(5000), Vec::new()),
            ];
            for (since, expected) in cases {
                let got = shell(since, lasts);
                assert!(
                    same(got.clone(), expected.clone()),
                    "{since:?} into {lasts:?}: {got:?}, not {expected:?}"
                );
            }
        }
    }

    #[test]
    fn sparks_start_on_whole_steps_scattered_over_the_tiles() {
        let began = Duration::from_millis(12_300);
        let within = Duration::from_millis(600);
        let starts: Vec<Duration> = (0..3)
            .flat_map(|y| (0..3).map(move |x| spark_starts(Pos { x, y }, began, within)))
            .collect();
        assert!(starts.iter().all(|start| *start <= within));
        assert!(starts.iter().all(|start| start.as_millis() % 50 == 0));
        let different: BTreeSet<Duration> = starts.iter().copied().collect();
        assert!(different.len() >= 4, "{starts:?}");
    }
}
