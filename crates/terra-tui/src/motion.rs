//! Motion (design §6.9, #306): small effects drawn over the finished screen
//! in real time, so the eye goes to what just happened.
//!
//! How it fits together:
//!
//! - **What the app keeps** is [`Motion`]: the switch; how each sprite and
//!   egg was drawn after the last ticks taken in ([`Seen`]), since the world
//!   removes a dead sprite before it says so; the deaths and hatchings
//!   playing, and the eggs about to hatch, cracking; the tab sweeping in,
//!   which `App::open` starts; what filled the screen at the last frame,
//!   which the app's clock compares with what does now to see a pop-up
//!   open or close, and how the last frame drew the pop-ups that closed;
//!   the selected sprite's drives' bars and the tiles it has left, its
//!   trail, noted as ticks are taken in; and the pan under way. Each effect
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
//!   tooltip, which doesn't move. A bar's blocks are no effect: the Body tab
//!   draws as many as `App::drive_bar` says.
//! - **Time** is real time, which the frame loop hands the app (§6.6):
//!   [`age`] says how far into an effect a frame is, and [`eased`] how far
//!   that's got, fast and then slowing. A loop works out its own phase, in
//!   whole milliseconds, as the water does.
//! - **It thins out at 4× and up** ([`thins_out`]): fewer deaths and
//!   hatchings at once, and shorter, as the trail is, and the bars and the
//!   view jump.
//! - **Colours** are the theme's `motion` section's. A fade blends exact
//!   colours, so a named one is blended as Campbell draws it
//!   ([`campbell::exact`]), and a tile with no background of its own takes
//!   the theme's dark. On the panels, what an effect leaves as it was keeps
//!   its own colour, named or the terminal's, so tachyonfx never sees the
//!   terminal's own colour, which it would take for white or black.
//! - **The same on every computer:** only colour effects that depend on
//!   nothing but their age, eased by polynomials (`QuadOut`) and blended in
//!   RGB, and no `sin`, `cos`, `powf` or `exp`.

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use tachyonfx::{ColorSpace, Effect, EffectTimer, Interpolation, fx};
use terra_sim::{EntityId, EventKind, Pos, World};

use crate::app::{App, Origin, Screen, Tab};
use crate::campbell::{self, FOREGROUND, Rgb};
use crate::clock::Speed;
use crate::inspector;
use crate::policy::{Panel, Subject};
use crate::space::{Pattern, Place};
use crate::spots::Spots;
use crate::theme::MotionLook;
use crate::ui;

/// How long a death plays, and a hatching (design §6.9): half as long at
/// 4× and up.
const DEATH: Duration = Duration::from_millis(1200);
const HATCHING: Duration = Duration::from_millis(1000);

/// How long an egg about to hatch takes to crack, its tile coming to glow
/// (design §6.9).
const CRACK: Duration = Duration::from_millis(300);

/// How far a cracked egg's tile glows, from its own background towards the
/// sparks' colour.
const GLOW: f32 = 0.4;

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
    /// The eggs on the map about to hatch on the next tick, from when the
    /// screen first saw so, while they crack and then hold.
    cracks: BTreeMap<EntityId, Duration>,
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

/// A drive's bar easing from `from` blocks filled to `to`, from when it
/// began.
#[derive(Debug, Clone, Copy)]
struct Bar {
    from: u8,
    to: u8,
    began: Duration,
}

impl Bar {
    /// A bar still at `blocks`, from `now`.
    fn at(blocks: u8, now: Duration) -> Bar {
        Bar {
            from: blocks,
            to: blocks,
            began: now,
        }
    }

    /// How many blocks it fills at `now`: a whole block at a time, quick at
    /// first and slowing (design §6.9).
    fn shown(&self, now: Duration) -> u8 {
        let way = f32::from(self.to) - f32::from(self.from);
        let moved = (way * eased(age(self.began, now), BAR)).round();
        (f32::from(self.from) + moved) as u8
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
    /// and had `cracked` if the screen saw it about to hatch.
    Hatching {
        id: EntityId,
        tile: Pos,
        egg: Color,
        cracked: bool,
    },
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
            cracks: BTreeMap::new(),
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
    pub(crate) fn switch(&mut self, on: bool) {
        self.on = on;
        if !on {
            self.seen.clear();
            self.moments.clear();
            self.cracks.clear();
            self.sweep = None;
            self.opened = None;
            self.closed = None;
            self.bars = None;
            self.trail = None;
            self.pan = None;
        }
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
                        cracked: self.cracks.remove(&id).is_some(),
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

    /// Notes the eggs on the map about to hatch on the next tick, after the
    /// ticks just taken in at `now` and `speed`: `due`, by ID. Each starts
    /// to crack as it's first seen so, and one that no longer is, held or
    /// stood on, stops. Not at 4× and up, where a frame may run its last
    /// tick and the next (design §6.9).
    pub(crate) fn note_cracks(&mut self, due: &[EntityId], now: Duration, speed: Speed) {
        if !self.on || thins_out(speed) {
            self.cracks.clear();
            return;
        }
        self.cracks.retain(|id, _| due.contains(id));
        for &id in due {
            self.cracks.entry(id).or_insert(now);
        }
    }

    /// Notes the blocks the selected sprite's drives' bars fill, by the
    /// drive's name, after the ticks just taken in at `now` and `speed`:
    /// `sprite`, with its ID, or none if no sprite in the world is
    /// selected. A bar whose level changed eases to it from where it's
    /// drawn, but at 4× and up, where it jumps, and a sprite seen for the
    /// first time shows its levels at once (design §6.9).
    pub(crate) fn note_bars(
        &mut self,
        sprite: Option<(EntityId, Vec<(String, u8)>)>,
        now: Duration,
        speed: Speed,
    ) {
        let Some((id, drives)) = sprite.filter(|_| self.on) else {
            self.bars = None;
            return;
        };
        let eases = !thins_out(speed);
        match &mut self.bars {
            Some(bars) if bars.sprite == id => {
                for (name, to) in drives {
                    let bar = bars.drives.entry(name).or_insert(Bar::at(to, now));
                    if bar.to != to {
                        let from = if eases { bar.shown(now) } else { to };
                        *bar = Bar {
                            from,
                            to,
                            began: now,
                        };
                    }
                }
            }
            _ => {
                let drives = drives
                    .into_iter()
                    .map(|(name, blocks)| (name, Bar::at(blocks, now)))
                    .collect();
                self.bars = Some(Bars { sprite: id, drives });
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

    /// How many blocks drive `drive`'s bar fills on sprite `id`'s Body tab
    /// at `now` and `speed`, its level `live` blocks: easing to it, if it's
    /// the level last taken in, and otherwise at it.
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

/// A screen cell of the map view that `ui::draw_tiles` drew an item lying
/// on the map in, with the item's ID: an egg there may crack (design §6.9).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Lying {
    pub(crate) cell: Position,
    pub(crate) id: EntityId,
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
/// `bare` and the items `lying` there among them, before the marks go over
/// them (design §6.9): the water, the trail, each egg about to hatch
/// cracking, and then each death and hatching, oldest first.
pub(crate) fn play_on_map(
    buf: &mut Buffer,
    inner: Rect,
    app: &App,
    world: &World,
    (bare, lying): (&[Bare], &[Lying]),
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
    };
    let now = app.running_for();
    shimmer(buf, &on, now);
    trail(buf, &on, now);
    crack(buf, &on, lying, now);
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
            Happening::Hatching {
                id,
                tile,
                egg,
                cracked,
            } => {
                let young = Young { id, egg, cracked };
                hatching(buf, &on, tile, young, moment, age);
            }
        }
    }
}

/// Plays the panels' motion over the finished screen once what can be
/// pointed at among `spots` is marked, before the tooltip, which doesn't
/// move (design §6.9): the tab sweeping in, where the inspector is drawn
/// just now, under any pop-up; then a pop-up that closed fading out to the
/// dark, as the last frame drew it, and what was under it fading back from
/// there, but where a pop-up is drawn now; and a pop-up that opened fading
/// in, but under the debug overlay, which doesn't fade.
pub(crate) fn play_on_panels(buf: &mut Buffer, app: &App, spots: &Spots) {
    if !app.motion() {
        return;
    }
    let (kept, now) = (app.motion_kept(), app.running_for());
    let dark = campbell::rgb(app.theme.motion().dark).unwrap_or(campbell::BACKGROUND);
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
        play_over(effect, age, buf, body, dark, under_a_pop_up);
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
                play_over(fade_out, age, buf, area, dark, under_a_pop_up);
            } else {
                play_over(fade_in(half), age - half, buf, area, dark, under_a_pop_up);
            }
        }
    }
    if let Some(began) = kept.opening(now) {
        for area in spots.fading() {
            let over_it = |cell: Position| !spots.fades_at(cell);
            play_over(fade_in(POP_UP), age(began, now), buf, area, dark, over_it);
        }
    }
}

/// Plays `effect` over `area` of `buf`, `age` into it, but on the cells
/// `kept` keeps as they are (design §6.9). It blends exact colours in RGB:
/// a named one as Campbell draws it, the terminal's own text colour as
/// Campbell's and its own background as `dark`. What the effect leaves as
/// it was keeps the colour it had, named or the terminal's own, so a fade
/// that's played out, or a cell it doesn't reach, is drawn as before.
fn play_over(
    effect: Effect,
    age: Duration,
    buf: &mut Buffer,
    area: Rect,
    dark: Rgb,
    kept: impl Fn(Position) -> bool,
) {
    let area = area.intersection(buf.area);
    let exact = |cell: &Cell| {
        (
            campbell::exact(cell.fg, FOREGROUND),
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
    let own_looks = [on.app.shown_cursor(), ui::attended_by(on.app, on.world)];
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

/// Plays each egg about to hatch cracking, among the items `lying` on the
/// map (design §6.9): from when the screen first saw it would hatch on the
/// next tick, its tile's background comes to glow, part way to the sparks'
/// colour, quick and then slowing, and holds so until the young appears.
/// Colour only: its glyph stays. It asks the policy first, as its hatching
/// will (§6.4).
fn crack(buf: &mut Buffer, on: &OnMap, lying: &[Lying], now: Duration) {
    if thins_out(on.app.clock.speed()) {
        return;
    }
    let cracks = &on.app.motion_kept().cracks;
    for spot in lying {
        let Some(&began) = cracks.get(&spot.id) else {
            continue;
        };
        if !on.app.can_view(Panel::Emotes, Subject::Sprite(spot.id)) {
            continue;
        }
        let age = age(began, now);
        let bg = glow(buf, on, spot.cell, eased(age, CRACK));
        play(fx::paint_bg(bg, timer(CRACK)), age, buf, spot.cell);
    }
}

/// The background of screen cell `cell` `way` of the way from its own to
/// the glow of a cracked egg, from 0 to 1.
fn glow(buf: &Buffer, on: &OnMap, cell: Position, way: f32) -> Color {
    let dark = campbell::rgb(on.look.dark).unwrap_or(campbell::BACKGROUND);
    let own = campbell::exact(buf[cell].bg, dark);
    let glow = ColorSpace::Rgb.lerp(&own, &campbell::exact(on.look.spark, dark), GLOW);
    ColorSpace::Rgb.lerp(&own, &glow, way)
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

/// The sprite a hatching hatched, the colour its egg was drawn in, and
/// whether it had cracked.
struct Young {
    id: EntityId,
    egg: Color,
    cracked: bool,
}

/// Plays a hatching from the egg on `tile`, `age` into `moment` (design
/// §6.9): the young starts in the egg's colour and warms to its own over
/// the first half, the glow of an egg that cracked fading from its tile as
/// it does, and the tiles round the egg spark at scattered moments and fade
/// back. On the young's own tile its warming plays rather than a spark.
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
    if age < warming && young.cracked {
        for &cell in &drawn {
            let bg = glow(buf, on, cell, 1.0 - eased(age, warming));
            play(fx::paint_bg(bg, timer(warming)), age, buf, cell);
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
