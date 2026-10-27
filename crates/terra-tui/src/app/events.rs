//! What the screen takes in of each tick (design §6.1): the event log and
//! its filter, the selected sprite's observed list, the names sprites have
//! been given, and what sets off each emote (§6.3).

use terra_sim::{
    ActionView, Command, CursorTouch, EntityId, Event, EventKind, Outcome, Progress, Target, Verb,
    World,
};

use super::{
    App, Crash, EMOTE_FOR, EventFilter, OBSERVED_LENGTH, Observed, Rest, Selection, Shoved,
    StatusMark, Ticks, along,
};
use crate::inspector;
use crate::text::Words;
use crate::theme::Emote;

/// How many events the event log keeps.
const EVENT_LOG_LENGTH: usize = 100;

/// Where the Cursor's doing came from, as a sprite felt it in the tick just
/// run (design v29 §6.1): "out of nowhere", or, while sprites could see it,
/// "by the Cursor" (or "from" it, as `preposition` says).
fn whence(world: &World, preposition: &str) -> String {
    if world.cursor().visible() {
        format!("{preposition} the Cursor")
    } else {
        "out of nowhere".into()
    }
}

impl App {
    /// Takes in what happened during a tick, for the event log (design §6.1),
    /// and the death of the selected sprite. Object events and most action
    /// events are left out of the log: they happen dozens of times a minute
    /// and would bury everything else. The World tab counts objects instead,
    /// and the Body tab shows the selected sprite's action. The log keeps
    /// every Play and Hit, and any action that hurt a sprite.
    ///
    /// An action the selected sprite finishes, or another's done to it, goes
    /// on the front of its observed list, or counts up the line there if it
    /// reads the same.
    ///
    /// While motion is on, a death or a hatching starts its motion, and the
    /// screen then notes how each sprite and egg is drawn, for the next
    /// (design §6.9).
    pub fn record(&mut self, events: &[Event], world: &World) {
        self.take_in_tick(events, world);
        self.note_looks(world);
        self.note_flashing(world);
        self.note_bars(world);
        self.note_trail(world);
    }

    /// Takes in one tick's `events`, as `record` says, but for noting how
    /// sprites and eggs are drawn: of a frame's ticks, the screen notes that
    /// once they're all taken in, since `world` is as the last left it.
    fn take_in_tick(&mut self, events: &[Event], world: &World) {
        self.note_names(world);
        for event in events {
            self.set_off(&event.kind);
            // The events carry the names, so the log names a sprite as it
            // was then: one named and killed in one tick is gone before
            // `note_names` sees it.
            if let EventKind::Renamed { id, name }
            | EventKind::Died {
                id,
                name: Some(name),
                ..
            } = &event.kind
            {
                self.names.note(*id, name);
            }
            if let EventKind::ActionEnded { id, ref action, .. } = event.kind {
                self.note_hurt(id, action);
                self.note_gave_up(id, action);
                self.note_done_to_selected(event.tick, id, action, world);
            }
            self.note_crash(&event.kind);
            self.note_rest(&event.kind);
            self.note_courtship(&event.kind);
            if let (EventKind::Rewarded { id, .. } | EventKind::Corrected { id, .. }, Some(touch)) =
                (&event.kind, CursorTouch::reported(&event.kind))
            {
                self.note_touch(event.tick, *id, touch, world);
                self.flash_report(StatusMark::Applied);
            }
            // Let go, the selected sprite felt it as a pull from nowhere,
            // unless it could see the Cursor (design v23, v29 §6.1).
            if let EventKind::LetGo { sprite } | EventKind::Shoved { sprite } = event.kind
                && self.selection == Some(Selection::Living(sprite))
            {
                let how = whence(world, "by");
                self.observe(event.tick, format!("Was pulled along {how}"));
            }
            // A shove, felt as one from nowhere, is observed once the slide
            // ends, with what it crashed into (design v25 §6.1).
            match &event.kind {
                EventKind::Shoved { sprite }
                    if self.selection == Some(Selection::Living(*sprite)) =>
                {
                    self.shoved = Some(Shoved {
                        sprite: *sprite,
                        crash: None,
                        how: whence(world, "by"),
                    });
                }
                EventKind::Crashed { sprite, into, hurt } => {
                    if let Some(shoved) = &mut self.shoved
                        && shoved.sprite == *sprite
                    {
                        shoved.crash = Some(Crash {
                            into: into.clone(),
                            hurt: *hurt,
                        });
                    }
                }
                _ => {}
            }
            if let EventKind::CommandRejected { command, .. } = &event.kind {
                // The marks report the Cursor's clicks; naming isn't one
                // (design v28 §6.5).
                if !matches!(command, Command::Rename { .. }) {
                    self.flash_report(StatusMark::Rejected);
                }
                // And why, on the status line, as the log words it (design
                // v22 §6.1).
                if let Some(why) = inspector::event_line(event, &self.words(world)) {
                    let names = inspector::refusal_sprites(event);
                    self.refusal = Some((why, names, self.running_for));
                }
            }
            if let EventKind::Died { id, cause, age, .. } = event.kind {
                self.resting.remove(&id);
                if self.selection == Some(Selection::Living(id)) {
                    self.selection = Some(Selection::Dead { id, cause, age });
                }
                // The followed sprite's death ends Follow (design v26 §6.5).
                if self.follow == Some(id) {
                    self.follow = None;
                }
            }
            // What the log says of it, if it's logged at all; a line that
            // reads as the one before merges into it with a count (design v21
            // §6.1).
            if let EventKind::Hatched { id, .. } = event.kind
                && let Some(new) = world
                    .sprite(id)
                    .and_then(|it| inspector::something_new(&it))
            {
                self.news.insert(id, new);
            }
            let words = Words {
                data: world.data(),
                names: &self.names,
                news: &self.news,
            };
            let Some(line) = inspector::event_line(event, &words) else {
                continue;
            };
            match self.event_log.front_mut() {
                Some((front, count)) if inspector::event_line(front, &words) == Some(line) => {
                    *front = event.clone();
                    *count += 1;
                }
                _ => self.event_log.push_front((event.clone(), 1)),
            }
        }
        self.event_log.truncate(EVENT_LOG_LENGTH);
        let log = &self.event_log;
        self.news.retain(|&id, _| {
            log.iter().any(
                |(event, _)| matches!(event.kind, EventKind::Hatched { id: its, .. } if its == id),
            )
        });
        self.note_slide_ended(world);
        // The world has applied what was queued for the ticks it has run.
        self.queued.retain(|&(tick, _)| tick >= world.tick());
        // An aim ends when what it aimed is gone: a held berry expired, or
        // a led sprite died (design v25 §6.5).
        if self.grip(world).is_none() {
            self.end_aim();
        }
        self.settle_cursor(world);
    }

    /// Notes the names the world's sprites and eggs have now, so a sprite
    /// named and then dead keeps its name in the log.
    pub(super) fn note_names(&mut self, world: &World) {
        for sprite in world.sprites() {
            if let Some(name) = sprite.name() {
                self.names.note(sprite.id(), name);
            }
        }
        for egg in world.eggs() {
            if let Some(name) = egg.name() {
                self.names.note(egg.id(), name);
            }
        }
    }

    /// Notes the names of the dead the world remembers (M2 design §17.3), for
    /// a world just loaded: those that die while the screen watches, it
    /// notes as they die.
    pub(super) fn note_names_of_the_dead(&mut self, world: &World) {
        for dead in world.dead_sprites() {
            if let Some(name) = dead.name() {
                self.names.note(dead.id(), name);
            }
        }
    }

    /// Once the selected sprite's slide has ended, its observed list tells of
    /// the shove: "Was shoved out of nowhere", and what it crashed into, and
    /// whether that hurt (design v25 §6.1).
    fn note_slide_ended(&mut self, world: &World) {
        let Some(shoved) = self.shoved.take() else {
            return;
        };
        let id = shoved.sprite;
        if world
            .sprite(id)
            .is_some_and(|sprite| sprite.slide().is_some())
        {
            self.shoved = Some(shoved);
            return;
        }
        if self.selection != Some(Selection::Living(id)) {
            return;
        }
        let how = shoved.how;
        let line = match shoved.crash {
            None => format!("Was shoved {how}"),
            Some(Crash { into, hurt }) => {
                let hurt = if hurt { ", and got hurt" } else { "" };
                let into = inspector::crashed_into(&into, &self.words(world));
                format!("Was shoved {how}, into {into}{hurt}")
            }
        };
        self.observe(world.tick().saturating_sub(1), line);
    }

    /// Starts the Hurt emote on each sprite that sprite `actor`'s `action` hurt.
    fn note_hurt(&mut self, actor: EntityId, action: &ActionView) {
        let hurt_target = match action.target {
            Some(Target::Sprite(id)) if action.hurt.target => Some(id),
            _ => None,
        };
        let hurt_actor = action.hurt.actor.then_some(actor);
        for id in hurt_actor.into_iter().chain(hurt_target) {
            self.start_emote(id, Emote::Hurt);
        }
    }

    /// Starts the Hurt emote on a sprite whose slide crashed into something
    /// that hurts, such as a thornbush (design v38 §6.3).
    fn note_crash(&mut self, event: &EventKind) {
        if let EventKind::Crashed {
            sprite, hurt: true, ..
        } = *event
        {
            self.start_emote(sprite, Emote::Hurt);
        }
    }

    /// Starts the Failed emote on sprite `actor` if it gave its `action` up:
    /// failed, blocked or timed out (design §6.3). An action that also hurt
    /// it shows Hurt instead, which matters more.
    fn note_gave_up(&mut self, actor: EntityId, action: &ActionView) {
        let gave_up = matches!(
            action.progress,
            Progress::Ended(Outcome::Failed | Outcome::Blocked | Outcome::TimedOut)
        );
        if gave_up && !action.hurt.actor {
            self.start_emote(actor, Emote::Failed);
        }
    }

    /// Notes a rest starting or ending, for the Resting emote (design §6.3).
    fn note_rest(&mut self, event: &EventKind) {
        let now = self.running_for;
        match *event {
            EventKind::ActionStarted {
                id,
                verb: Verb::Rest,
            } => {
                self.resting.insert(
                    id,
                    Rest {
                        since: now,
                        ended: None,
                    },
                );
            }
            EventKind::ActionEnded {
                id,
                verb: Verb::Rest,
                ..
            } => {
                if let Some(rest) = self.resting.get_mut(&id) {
                    rest.ended.get_or_insert(now);
                }
            }
            _ => {}
        }
    }

    /// Starts the Dancing emote on a sprite that danced for another, and
    /// the Calling emote on one that calls (M2 design §15.8).
    fn note_courtship(&mut self, event: &EventKind) {
        match *event {
            EventKind::ActionStarted {
                id,
                verb: Verb::Call,
            } => self.start_emote(id, Emote::Calling),
            EventKind::ActionEnded {
                id,
                verb: Verb::Court,
                outcome: Outcome::Applied,
                ..
            } => self.start_emote(id, Emote::Dancing),
            _ => {}
        }
    }

    /// Starts `emote` on sprite `id` now: the newest emote wins (design
    /// §6.3).
    fn start_emote(&mut self, id: EntityId, emote: Emote) {
        self.emotes.insert(id, (emote, self.running_for));
    }

    /// Takes in what a frame's ticks did: their events, as `record` does,
    /// and the Pleased emote on each sprite that felt a strong reward
    /// (design §6.3). One already showing Pleased carries on, rather than
    /// starting again each tick a reward lasts. How sprites and eggs are
    /// drawn is noted once, after the last tick (§6.9).
    pub fn take_in(&mut self, ticks: Ticks, world: &World) {
        // Tick by tick, so of two emotes in one frame the later tick's wins.
        for (events, pleased) in ticks.0 {
            self.take_in_tick(&events, world);
            for id in pleased {
                let showing = matches!(self.emotes.get(&id), Some(&(Emote::Pleased, at)) if self.shows(at, EMOTE_FOR));
                if !showing {
                    self.start_emote(id, Emote::Pleased);
                }
            }
        }
        self.note_looks(world);
        self.note_flashing(world);
        self.note_bars(world);
        self.note_trail(world);
    }

    /// The Cursor's touch on sprite `id`, on `tick`: its emote, and a line
    /// on the observed list if it's the selected sprite's, told as the
    /// sprite felt it: from nowhere, unless it could see the Cursor (design
    /// v21, v29 §6.1, §6.3).
    fn note_touch(&mut self, tick: u64, id: EntityId, touch: CursorTouch, world: &World) {
        let (emote, line) = match touch {
            CursorTouch::Pet => (Emote::Pleased, "a gentle touch"),
            CursorTouch::Hug => (Emote::Pleased, "a warm embrace"),
            CursorTouch::Zap => (Emote::Shocked, "a zap"),
            CursorTouch::Shock => (Emote::Shocked, "a jolt"),
        };
        self.start_emote(id, emote);
        if self.selection == Some(Selection::Living(id)) {
            let from = whence(world, "from");
            self.observe(tick, format!("Felt {line} {from}"));
        }
    }

    /// Puts sprite `actor`'s `action`, finished on `tick`, on the selected
    /// sprite's observed list, if it was the selected sprite's own or done
    /// to it.
    fn note_done_to_selected(
        &mut self,
        tick: u64,
        actor: EntityId,
        action: &ActionView,
        world: &World,
    ) {
        let Some(Selection::Living(selected)) = self.selection else {
            return;
        };
        if actor == selected {
            let line = inspector::observed_line(action, &self.words(world));
            self.observe(tick, line);
        } else if action.target == Some(Target::Sprite(selected))
            && let Some(line) = inspector::done_to_line(actor, action, &self.words(world))
        {
            self.observe(tick, line);
        }
    }

    /// Puts `line`, finished on `tick`, on the front of the observed list.
    fn observe(&mut self, tick: u64, line: String) {
        match self.observed.front_mut() {
            Some(front) if front.line == line => {
                front.count += 1;
                front.tick = tick;
            }
            _ => {
                self.observed.push_front(Observed {
                    line,
                    count: 1,
                    tick,
                });
                self.observed.truncate(OBSERVED_LENGTH);
            }
        }
    }

    /// What the player has watched the selected sprite finish since
    /// selecting it, newest first (design §6.1).
    pub fn observed(&self) -> impl Iterator<Item = &Observed> {
        self.observed.iter()
    }

    /// The events the event log shows, newest first, each the latest of
    /// the same event in a row, with how many there were: those the filter
    /// lets through (design §6.1).
    pub fn event_log(&self) -> impl Iterator<Item = (&Event, u32)> {
        self.event_log
            .iter()
            .filter(|(event, _)| self.filter_shows(event))
            .map(|(event, count)| (event, *count))
    }

    /// Whether the event filter lets `event` through (design §6.1).
    fn filter_shows(&self, event: &Event) -> bool {
        match self.event_filter {
            EventFilter::All => true,
            EventFilter::Selected => self
                .selection
                .is_some_and(|selected| inspector::event_sprites(event).contains(&selected.id())),
            EventFilter::Major => matches!(
                event.kind,
                EventKind::Died { .. }
                    | EventKind::LearnedMilestone { .. }
                    | EventKind::CommandRejected { .. }
            ),
        }
    }

    /// Which events the event log shows (design §6.1).
    pub fn event_filter(&self) -> EventFilter {
        self.event_filter
    }

    /// Switches the event log to the next filter (design §6.1).
    pub(super) fn next_filter(&mut self) {
        self.event_filter = along(&EventFilter::ALL, self.event_filter, 1);
    }
}
