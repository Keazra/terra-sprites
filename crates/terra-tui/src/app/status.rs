//! What shows for a while: the emotes (design §6.3), the Decision
//! marker's flashing (§6.1), the Cursor's status marks (v22 §6.5), and what
//! the status line says of a refusal or a notice.

use std::time::Duration;

use terra_sim::{EntityId, World};

use super::{App, CursorMode, EMOTE_FOR, StatusMark};
use crate::policy::{Panel, Subject};
use crate::theme::Emote;

/// How long the Decision marker shows, and then doesn't: once a second in
/// all, like a text cursor (design §6.1).
const FLASH_HALF: Duration = Duration::from_millis(500);

/// How long an emote shows, and then the sprite does: twice as fast as the
/// Decision marker, so the two can't be confused (design §6.3).
const EMOTE_HALF: Duration = Duration::from_millis(250);

/// How long the status marks flash, in real time (design v21 §6.5).
const MARK_FLASH_FOR: Duration = Duration::from_millis(300);

/// How long the status line says why a click was refused, in real time
/// (design v22 §6.1).
const REFUSAL_FOR: Duration = Duration::from_secs(3);

impl App {
    /// The emote sprite `id` shows now, if any, taking turns with the
    /// sprite (design §6.3): the newest for a second after what set it off,
    /// or else Resting, for as long as it rests.
    pub fn emote(&self, id: EntityId) -> Option<Emote> {
        let flashing = |at: Duration| {
            let since = self.running_for.saturating_sub(at);
            (since.as_millis() / EMOTE_HALF.as_millis()).is_multiple_of(2)
        };
        if let Some(&(emote, at)) = self.emotes.get(&id)
            && self.shows(at, EMOTE_FOR)
        {
            return flashing(at).then_some(emote);
        }
        let rest = self.resting.get(&id)?;
        let resting = rest.until().is_none_or(|until| self.running_for < until);
        (resting && flashing(rest.since)).then_some(Emote::Resting)
    }

    /// What the Cursor's status marks show now (design v22 §6.5): of the
    /// flashes under way, the one that began last.
    pub fn status_mark(&self) -> StatusMark {
        let sent = self.sent_at.map(|at| (StatusMark::Sent, at));
        // Last, so a report wins a tie: within a frame, it's the later news.
        [sent, self.report]
            .into_iter()
            .flatten()
            .filter(|&(_, from)| self.shows(from, MARK_FLASH_FOR))
            .max_by_key(|&(_, at)| at)
            .map_or(StatusMark::Idle, |(mark, _)| mark)
    }

    /// What the Cursor's two status marks show now, top right (`Y`) then
    /// bottom left (`N`) (design v23 §6.5). In Grab mode they show what the
    /// Cursor has hold of, as the queue will leave it, unless a refusal is
    /// flashing; in the other modes, both show `status_mark`.
    pub fn status_marks(&self, world: &World) -> [StatusMark; 2] {
        let flash = self.status_mark();
        if self.mode != CursorMode::Grab || flash == StatusMark::Rejected {
            return [flash; 2];
        }
        // A Place menu item waiting on the Cursor shows `↓` and its glyph, as
        // something carried would (design v28 §6.5).
        if self.placing.is_some() {
            return [StatusMark::Release, StatusMark::Holding];
        }
        match self.grip(world) {
            Some(_) => [StatusMark::Release, StatusMark::Holding],
            None => [StatusMark::Grab, StatusMark::Empty],
        }
    }

    /// Why the player's latest click was refused, while the status line
    /// says so (design v22 §6.1). It's one of the Cursor's own lines, so it
    /// shows only if the policy lets each sprite it names be (design §6.4,
    /// #174).
    pub fn refusal(&self) -> Option<&str> {
        let (why, names, from) = self.refusal.as_ref()?;
        let named = |&id: &EntityId| self.can_view(Panel::CursorLines, Subject::Sprite(id));
        (names.iter().all(named) && self.shows(*from, REFUSAL_FOR)).then_some(why.as_str())
    }

    /// Whether something that shows from `from`, for `lasting`, shows now.
    pub(super) fn shows(&self, from: Duration, lasting: Duration) -> bool {
        self.running_for
            .checked_sub(from)
            .is_some_and(|since| since < lasting)
    }

    /// Flashes the world's report on a command in the status marks, once
    /// the latest `+` has shown for its time, so both flashes show however
    /// soon the report comes. A refusal still to show, or showing, isn't
    /// replaced by an applied command: it's what needs noticing (design v22
    /// §6.5). A click that sends clears it, so it's never an earlier
    /// click's.
    pub(super) fn flash_report(&mut self, mark: StatusMark) {
        let now = self.running_for;
        if let Some((StatusMark::Rejected, at)) = self.report
            && mark == StatusMark::Applied
            && now < at + MARK_FLASH_FOR
        {
            return;
        }
        let from = self.sent_at.map_or(now, |at| now.max(at + MARK_FLASH_FOR));
        self.report = Some((mark, from));
    }

    /// Whether the Decision marker is in its "on" half just now.
    pub fn flash_on(&self) -> bool {
        (self.running_for.as_millis() / FLASH_HALF.as_millis()).is_multiple_of(2)
    }

    /// A click with nothing to act on: it sends nothing, so `?` flashes at
    /// once, and the status line says why (design v22 §6.5).
    pub(super) fn refuse(&mut self, why: String) {
        self.report = Some((StatusMark::Rejected, self.running_for));
        self.refusal = Some((why, Vec::new(), self.running_for));
        self.notice = None;
    }

    /// Says what the player's action did on the status line, for a while.
    pub(super) fn tell_player(&mut self, what: String) {
        self.notice = Some((what, self.running_for));
        self.refusal = None;
    }

    /// What the player's latest action did, while the status line says so:
    /// where a genome was saved.
    pub fn notice(&self) -> Option<&str> {
        let (what, from) = self.notice.as_ref()?;
        self.shows(*from, REFUSAL_FOR).then_some(what.as_str())
    }
}
