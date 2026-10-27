//! Motion (design §6.9, #306): small effects drawn over the finished screen
//! in real time, so the eye goes to what just happened.
//!
//! How it fits together, for each effect to come:
//!
//! - **What the app keeps** is [`Motion`]: the switch; how each sprite and
//!   egg was drawn after the last ticks taken in ([`Seen`]), since the world
//!   removes a dead sprite before it says so; and the deaths and hatchings
//!   playing, each with when it began, in the app's `running_for`. Nothing
//!   is stepped as it's drawn (§6.8).
//! - **Drawing** builds each tachyonfx effect afresh every frame and plays
//!   it once to its age (`play`), so the same app drawn twice draws the same
//!   screen. `ui::draw_tiles` calls [`play_on_map`] once the tiles are drawn
//!   and before the marks go over them, and `ui::render_with` calls
//!   [`play_on_panels`] once the panels and pop-ups are drawn and what can
//!   be pointed at is marked, before the tooltip, which doesn't move.
//! - **Time** is real time, which the frame loop hands the app (§6.6):
//!   [`age`] says how far into an effect a frame is, and [`eased`] how far
//!   that's got, fast and then slowing. A loop works out its own phase, in
//!   whole milliseconds, as the water does.
//! - **It thins out at 4× and up** ([`thins_out`]): fewer deaths and
//!   hatchings at once, and shorter.
//! - **Colours** are the theme's `motion` section's. A fade blends exact
//!   colours, so a named one is blended as Campbell draws it
//!   ([`campbell::exact`]), and a tile with no background of its own takes
//!   the theme's dark.
//! - **The same on every computer:** only colour effects that depend on
//!   nothing but their age, eased by polynomials (`QuadOut`) and blended in
//!   RGB, and no `sin`, `cos`, `powf` or `exp`.
//!
//! Still to come (#306): the tab sweep, the pop-ups' fades and the drives'
//! easing bars, played on the panels; and the trail and the pan, on the map.

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use tachyonfx::{ColorSpace, Effect, EffectTimer, Interpolation, fx};
use terra_sim::{EntityId, EventKind, Pos, World};

use crate::app::App;
use crate::campbell::{self, FOREGROUND};
use crate::clock::Speed;
use crate::policy::{Panel, Subject};
use crate::space::{Pattern, Place};
use crate::spots::Spots;
use crate::theme::MotionLook;
use crate::ui;

/// How long a death plays, and a hatching (design §6.9): half as long at
/// 4× and up.
const DEATH: Duration = Duration::from_millis(1200);
const HATCHING: Duration = Duration::from_millis(1000);

/// The most deaths and hatchings that play at once (design §6.9), and at
/// 4× and up.
const MOST: usize = 4;
const MOST_THINNED: usize = 2;

/// How long each step of the water's wave is shown: 16 steps make its 4 s
/// cycle (design §6.9). Between steps the water stays still, so a clip's
/// frames between them are the same frame.
const WAVE_STEP_MS: u128 = 250;

/// Which of the water's shades each step of its wave shows, from its first
/// (0) to its second (`SHADES`): it rises, rests and falls back.
const WAVE: [u8; 16] = [0, 0, 1, 2, 3, 4, 5, 6, 6, 6, 5, 4, 3, 2, 1, 0];

/// The water's last shade, its second colour: seven fixed shades, the two
/// ends included.
const SHADES: u8 = 6;

/// The steps a hatching's sparks start on, in milliseconds.
const SPARK_STEP_MS: u64 = 50;

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
    /// Sprite `id` hatched from the egg on `tile`, which was drawn in `egg`.
    Hatching { id: EntityId, tile: Pos, egg: Color },
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
        }
    }

    /// Whether motion is on.
    pub(crate) fn on(&self) -> bool {
        self.on
    }

    /// Turns motion on or off. Off, it forgets what it saw and what was
    /// playing, so the screen is drawn as it was before there was motion.
    pub(crate) fn switch(&mut self, on: bool) {
        self.on = on;
        if !on {
            self.seen.clear();
            self.moments.clear();
        }
    }

    /// Notes how each sprite and egg is drawn, after the ticks just taken
    /// in (design §6.9).
    pub(crate) fn note(&mut self, seen: BTreeMap<EntityId, Seen>) {
        if self.on {
            self.seen = seen;
        }
    }

    /// Starts what `event` sets off at `now`, at `speed`: a death, for a
    /// sprite seen on the map, or a hatching, for an egg seen there. The
    /// newest takes the oldest's place once as many play as may at once. An
    /// event about a sprite or an egg not seen starts nothing.
    pub(crate) fn took_in(&mut self, event: &EventKind, now: Duration, speed: Speed) {
        if !self.on {
            return;
        }
        let (happening, full) = match *event {
            EventKind::Died { id, .. } => match self.seen.get(&id) {
                Some(&Seen::Sprite {
                    tile,
                    symbol,
                    colour,
                }) => (
                    Happening::Death {
                        id,
                        tile,
                        symbol,
                        colour,
                    },
                    DEATH,
                ),
                _ => return,
            },
            EventKind::Hatched { id, .. } => match self.seen.get(&id) {
                Some(&Seen::Egg { tile, colour }) => (
                    Happening::Hatching {
                        id,
                        tile,
                        egg: colour,
                    },
                    HATCHING,
                ),
                _ => return,
            },
            _ => return,
        };
        self.forget_finished(now);
        while self.moments.len() >= most(speed) {
            self.moments.pop_front();
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
    }

    /// The deaths and hatchings playing at `now`, oldest first: the newest
    /// of them, as many as may play at once at `speed`.
    fn playing(&self, now: Duration, speed: Speed) -> impl Iterator<Item = &Moment> {
        let skip = self.moments.len().saturating_sub(most(speed));
        self.moments
            .iter()
            .skip(skip)
            .filter(move |moment| age(moment.began, now) < moment.lasts)
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

/// A screen cell of the map view that `ui::draw_tiles` drew bare ground in:
/// no sprite, object or mark. `plain` while it's neither the Cursor's tile
/// nor the one the selected sprite attends to, which have their own looks.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Bare {
    pub(crate) cell: Position,
    pub(crate) tile: Pos,
    pub(crate) plain: bool,
}

/// Plays the map's motion over the tiles `ui::draw_tiles` drew in `inner`,
/// `bare` among them, before the marks go over them (design §6.9): the
/// water, and then each death and hatching, oldest first.
pub(crate) fn play_on_map(buf: &mut Buffer, inner: Rect, app: &App, world: &World, bare: &[Bare]) {
    if !app.motion() {
        return;
    }
    let on = OnMap {
        inner,
        app,
        world,
        look: app.theme.motion(),
        bare,
    };
    let now = app.running_for();
    shimmer(buf, &on, now);
    for moment in app.motion_kept().playing(now, app.clock.speed()) {
        let age = age(moment.began, now);
        match moment.happening {
            // What tells the player something about a sprite asks first
            // (design §6.4).
            Happening::Death { id, .. } | Happening::Hatching { id, .. }
                if !app.can_view(Panel::Emotes, Subject::Sprite(id)) => {}
            Happening::Death {
                tile,
                symbol,
                colour,
                ..
            } => death(buf, &on, tile, (symbol, colour), age, moment.lasts),
            Happening::Hatching { id, tile, egg } => {
                hatching(buf, &on, tile, Young { id, egg }, moment, age);
            }
        }
    }
}

/// Plays the panels' motion over the finished screen, `screen`, once what
/// can be pointed at among `spots` is marked, before the tooltip, which
/// doesn't move (design §6.9). Nothing plays there yet: the tab sweep, the
/// pop-ups' fades and the easing bars are to come (#306).
pub(crate) fn play_on_panels(_buf: &mut Buffer, _screen: Rect, _app: &App, _spots: &Spots) {}

/// The map view that motion plays over: where its tiles are drawn,
/// `inner`, the app and world drawn there, the theme's colours for motion,
/// and the cells that show bare ground.
struct OnMap<'a> {
    inner: Rect,
    app: &'a App,
    world: &'a World,
    look: MotionLook,
    bare: &'a [Bare],
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
        self.bare.iter().any(|spot| spot.cell == cell)
    }
}

/// Draws bare water in its shade at `now`: each water drifts between its
/// two shades on a 4 s cycle, its phase by where the tile is, so waves roll
/// across the lake (design §6.9). Under a sprite, an object or a mark, and
/// on the Cursor's tile and the attended one, it keeps its own colour.
fn shimmer(buf: &mut Buffer, on: &OnMap, now: Duration) {
    let step = now.as_millis() / WAVE_STEP_MS;
    let steps = WAVE.len() as u128;
    for spot in on.bare.iter().filter(|spot| spot.plain) {
        let Some([from, to]) = on.look.water(on.world.map().terrain(spot.tile)) else {
            continue;
        };
        let along = (u128::from(spot.tile.x) + 2 * u128::from(spot.tile.y)) % steps;
        let phase = (step % steps + steps - along) % steps;
        let shade = WAVE[phase as usize];
        buf[spot.cell].set_fg(blend(from, to, shade));
    }
}

/// The water's shade `shade` of `SHADES` from `from` to `to`, each as
/// Campbell draws it, in whole numbers.
fn blend(from: Color, to: Color, shade: u8) -> Color {
    let (from, to) = (
        campbell::rgb(from).unwrap_or(FOREGROUND),
        campbell::rgb(to).unwrap_or(FOREGROUND),
    );
    let mix = |from: u8, to: u8| {
        let (from, to, shade, shades) = (
            u16::from(from),
            u16::from(to),
            u16::from(shade),
            u16::from(SHADES),
        );
        let mixed = (from * (shades - shade) + to * shade + shades / 2) / shades;
        u8::try_from(mixed).unwrap_or(u8::MAX)
    };
    Color::Rgb(
        mix(from[0], to[0]),
        mix(from[1], to[1]),
        mix(from[2], to[2]),
    )
}

/// `cell`'s colour as an exact one, for a fade to blend from (design §6.9).
fn exact_fg(buf: &mut Buffer, cell: Position) {
    let fg = campbell::exact(buf[cell].fg, FOREGROUND);
    buf[cell].set_fg(fg);
}

/// Plays a death on `tile`, `age` into it, for as long as it `lasts`
/// (design §6.9): the tile's background flashes and fades back, and the
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
    let dark = campbell::rgb(look.dark).unwrap_or(campbell::BACKGROUND);
    let flash = campbell::exact(look.flash, dark);
    let back = eased(age, lasts);
    for (n, cell) in on.cells(tile).into_iter().enumerate() {
        if on.shows_bare(cell) {
            let ghost = if n == 0 { symbol } else { ' ' };
            buf[cell]
                .set_char(ghost)
                .set_fg(campbell::exact(colour, FOREGROUND));
            let ash = campbell::exact(look.ash, FOREGROUND);
            play(fx::fade_to_fg(ash, timer(lasts)), age, buf, cell);
        }
        let own = campbell::exact(buf[cell].bg, dark);
        let bg = ColorSpace::Rgb.lerp(&flash, &own, back);
        play(fx::paint_bg(bg, timer(lasts)), age, buf, cell);
    }
}

/// The sprite a hatching hatched, and the colour its egg was drawn in.
struct Young {
    id: EntityId,
    egg: Color,
}

/// Plays a hatching from the egg on `tile`, `age` into `moment` (design
/// §6.9): the young starts in the egg's colour and warms to its own over
/// the first half, and the tiles round the egg spark at scattered moments
/// and fade back. On the young's own tile its warming plays rather than a
/// spark.
fn hatching(buf: &mut Buffer, on: &OnMap, tile: Pos, young: Young, moment: &Moment, age: Duration) {
    let lasts = moment.lasts;
    let warming = lasts / 2;
    let sprite = on.world.sprite(young.id);
    let drawn = sprite
        .as_ref()
        .map_or(Vec::new(), |sprite| on.cells(sprite.pos()));
    let emoting = sprite.is_some_and(|sprite| ui::is_emote(on.app, &sprite));
    if age < warming && !emoting {
        let egg = campbell::exact(young.egg, FOREGROUND);
        for &cell in &drawn {
            exact_fg(buf, cell);
            play(fx::fade_from_fg(egg, timer(warming)), age, buf, cell);
        }
    }
    let spark = campbell::exact(on.look.spark, FOREGROUND);
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
                    exact_fg(buf, cell);
                    play(fx::fade_from_fg(spark, timer(sparks_for)), into, buf, cell);
                }
            }
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

    #[test]
    fn water_s_shades_run_from_one_end_to_the_other() {
        let (from, to) = (Color::Rgb(0, 0x37, 0xDA), Color::Rgb(0x2F, 0x62, 0xF2));
        assert_eq!(blend(from, to, 0), from);
        assert_eq!(blend(from, to, SHADES), to);
        assert_eq!(blend(Color::Blue, to, 0), from);
        assert_eq!(blend(from, to, 3), Color::Rgb(0x18, 0x4D, 0xE6));
    }
}
