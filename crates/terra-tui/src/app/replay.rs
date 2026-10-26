//! Playback (design §2.7): playing a replay back, pausing where it ends
//! or parts from the world, and taking it over (v36).

use terra_sim::{Divergence, Playback, World};

use super::{App, CursorMode, Replay, ReplayStops};
use crate::text::thousands;

impl App {
    /// Starts following `playback`, a replay called `name`, paused at its
    /// start (design §2.7): from here on the world can't be changed, only
    /// watched.
    pub fn start_replay(&mut self, name: &str, playback: &Playback) {
        self.replay = Some(Replay {
            end: playback.end(),
            ended: false,
            divergence: None,
        });
        self.mode = CursorMode::Select;
        self.clock.pause();
        // A replay from a save starts with the save's names, the dead's
        // too.
        self.note_names(playback.world());
        self.note_names_of_the_dead(playback.world());
        let from = if playback.started_fresh() {
            "a new world".to_string()
        } else {
            format!("a save at tick {}", thousands(playback.start()))
        };
        self.tell_player(format!(
            "Replaying {name}: from {from} to tick {}",
            thousands(playback.end())
        ));
        self.take_in_replay(playback);
    }

    /// `Ctrl+R`: takes over the replay at the tick it has reached, so the
    /// world is the player's from here on (design v36 §2.7). The player
    /// takes over the recorded Cursor too: where it is, what it has hold of
    /// (in Grab mode if it holds or leads something), and whether sprites
    /// can see it. Time runs on as it was. Outside a replay it does nothing.
    pub(super) fn take_over(&mut self, world: &World) {
        if self.replay.take().is_none() {
            return;
        }
        self.taken_over = true;
        self.commands.clear();
        self.queued.clear();
        let cursor = world.cursor();
        self.mode = if cursor.leads().is_some() || cursor.holds().is_some() {
            CursorMode::Grab
        } else {
            CursorMode::Select
        };
        self.told_tile = None;
        if let Some(tile) = cursor.tile() {
            self.cursor = tile;
            self.told_tile = Some(tile);
        }
        self.visible_in.clear();
        if cursor.visible() {
            self.visible_in.insert(self.mode);
        }
        self.told_visible = cursor.visible();
        self.settle_cursor(world);
        self.tell_player(format!(
            "You took over the replay at tick {}: it's your world now",
            thousands(world.tick())
        ));
    }

    /// Whether the player has just taken over the replay, for the frame
    /// loop to play on from its world, recording it afresh (design v36
    /// §2.7).
    pub fn take_taken_over(&mut self) -> bool {
        std::mem::take(&mut self.taken_over)
    }

    /// Whether a replay is playing back.
    pub fn replaying(&self) -> bool {
        self.replay.is_some()
    }

    /// While a replay plays back, when the frame loop should stop running
    /// ticks, so the screen pauses there (design §2.7).
    pub fn replay_stops(&self) -> Option<ReplayStops> {
        self.replay.map(|replay| ReplayStops {
            end: (!replay.ended).then_some(replay.end),
            divergence: replay.divergence.is_none(),
        })
    }

    /// Says on the status line that the session log couldn't be written.
    pub fn note_session_log_failed(&mut self, why: &str) {
        self.refuse(format!("Couldn't write the replay: {why}"));
    }

    /// Takes in how playback stands after a frame's ticks (design §2.7):
    /// at the recording's end, it pauses and says so, once, and time can
    /// then run on; where the world was first found different from the
    /// recording, it pauses and says between which checkpoints it parted.
    pub fn take_in_replay(&mut self, playback: &Playback) {
        let Some(mut replay) = self.replay else {
            return;
        };
        let tick = playback.world().tick();
        if !replay.ended && tick >= replay.end {
            replay.ended = true;
            self.clock.pause();
            self.tell_player(format!(
                "The replay ends here, at tick {}: time can run on, with nothing more done",
                thousands(replay.end)
            ));
        }
        if replay.divergence.is_none()
            && let Some(divergence) = playback.divergence()
        {
            replay.divergence = Some(divergence);
            self.clock.pause();
            self.tell_player(diverged(divergence));
        }
        self.replay = Some(replay);
    }

    /// What the top bar says of a replay, in place of the save status:
    /// where it ends, that it has ended, or that it diverged.
    pub fn replay_status(&self) -> Option<String> {
        let replay = self.replay?;
        Some(match replay.divergence {
            Some(divergence) => format!("replay diverged by tick {}", thousands(divergence.tick)),
            None if replay.ended => "replay ended".into(),
            None => format!("replay to tick {}", thousands(replay.end)),
        })
    }
}

/// What the status line says when playback first finds the world different
/// from the recording (design §2.7).
fn diverged(divergence: Divergence) -> String {
    let by = thousands(divergence.tick);
    match divergence.matched {
        Some(matched) => format!(
            "The replay parted from its recording between ticks {} and {by}",
            thousands(matched)
        ),
        None => format!("The replay was different from its recording from tick {by}"),
    }
}
