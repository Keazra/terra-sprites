//! The world the frame loop runs (design §2.7): a live one, which records
//! its session log, or a replay played back, which no input changes.

use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;

use terra_sim::{Command, Event, Playback, World};

/// What the frame loop runs.
pub enum Session {
    /// A world the player plays, recording its session log.
    Live(Box<World>),
    /// A replay played back.
    Replay(Box<Playback>),
}

impl Session {
    /// A live session on `world`, recording its session log from now.
    pub fn live(mut world: World) -> Session {
        world.start_recording();
        Session::Live(Box::new(world))
    }

    /// A replay's playback.
    pub fn replay(playback: Playback) -> Session {
        Session::Replay(Box::new(playback))
    }

    /// The world, as it plays or plays back.
    pub fn world(&self) -> &World {
        match self {
            Session::Live(world) => world,
            Session::Replay(playback) => playback.world(),
        }
    }

    /// The playback, if this is a replay.
    pub fn playback(&self) -> Option<&Playback> {
        match self {
            Session::Live(_) => None,
            Session::Replay(playback) => Some(playback),
        }
    }

    /// Runs one tick.
    pub fn step(&mut self) -> Vec<Event> {
        match self {
            Session::Live(world) => world.step(),
            Session::Replay(playback) => playback.step(),
        }
    }

    /// Submits the player's commands to a live world. A replay takes none:
    /// it plays only what it recorded (design §2.7).
    pub fn submit(&mut self, commands: Vec<Command>) {
        if let Session::Live(world) = self {
            for command in commands {
                world.submit(command);
            }
        }
    }

    /// Plays on from a world the player loaded, recording the session log
    /// afresh from it (design §2.7). A replay loads nothing.
    pub fn load(&mut self, world: World) {
        if let Session::Live(_) = self {
            *self = Session::live(world);
        }
    }

    /// Plays on from where a replay has reached, as a live session, which
    /// records its session log afresh from there (design v36 §2.7). The
    /// replay's commands not yet played are dropped. A live session stays
    /// as it is.
    pub fn take_over(&mut self) {
        if let Session::Replay(playback) = self {
            // The world plays on from its own save, which is what the new
            // session log starts from, so the log replays the very world
            // played on.
            let world =
                World::load(&playback.world().save()).expect("a world loads from its own save");
            *self = Session::live(world);
        }
    }

    /// Writes the session log to `path`, beside it first and then in its
    /// place, so a write cut short never breaks the last one. A replay
    /// writes none: it might be playing that very file.
    pub fn write_session_log(&self, path: &Path) -> io::Result<()> {
        let Some(bytes) = self.replay_bytes() else {
            return Ok(());
        };
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder)?;
        }
        let mut partial = path.as_os_str().to_owned();
        partial.push(".partial");
        std::fs::write(&partial, bytes)?;
        std::fs::rename(&partial, path)
    }

    /// A live world's session log so far.
    fn replay_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Session::Live(world) => world.replay(),
            Session::Replay(_) => None,
        }
    }
}

/// Runs `run` on `session`. If it panics, the session log is written to
/// `log` before the panic carries on, so a session that ends in a panic
/// still leaves its replay (design §2.9). The panic hook has restored the
/// terminal by then.
pub fn writing_session_log_on_panic<R>(
    session: &mut Session,
    log: Option<&Path>,
    run: impl FnOnce(&mut Session) -> R,
) -> R {
    match panic::catch_unwind(AssertUnwindSafe(|| run(session))) {
        Ok(result) => result,
        Err(panicked) => {
            if let Some(log) = log
                && let Err(err) = session.write_session_log(log)
            {
                eprintln!(
                    "terra-sprites: couldn't write the replay {}: {err}",
                    log.display()
                );
            }
            panic::resume_unwind(panicked)
        }
    }
}
