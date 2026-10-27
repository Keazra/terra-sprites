//! Saves and loading (design §6.7): saving as a name typed, the
//! autosaves and the quicksave, the load menu, the question before loading
//! over a world that has run, and the world loaded.

use std::path::{Path, PathBuf};
use std::time::Duration;

use terra_sim::{Pos, World};

use super::{
    AUTOSAVE_EVERY, App, Areas, CursorMode, Draft, Flow, Screen, spoken_duration, typeable,
};
use crate::clock::Speed;
use crate::input::Action;
use crate::policy::Omniscient;
use crate::saves::{self, MAX_SAVE_NAME_CHARS, QUICKSAVE, SaveFile};
use crate::text::thousands;

impl App {
    /// Sets where saves go (design §6.7).
    pub fn set_save_folder(&mut self, folder: PathBuf) {
        self.save_folder = Some(folder);
    }

    /// Where saves go, if anywhere.
    pub fn save_folder(&self) -> Option<&Path> {
        self.save_folder.as_deref()
    }

    /// Saves the world as `name` in the saves folder (design §6.7), and says
    /// so, or why it couldn't.
    pub(super) fn save_as(&mut self, name: &str, world: &World) {
        let Some(folder) = self.save_folder.clone() else {
            return self.refuse("There's no folder to save in".into());
        };
        match saves::write(&folder, name, &self.save_of(world)) {
            Ok(_) => {
                self.saved_now(world);
                self.tell_player(format!("Saved as {}", saves::name_for(name)));
            }
            Err(err) => self.refuse(format!("Couldn't save: {err}")),
        }
    }

    /// `world`'s save, with the tile at the middle of the view, so a load
    /// shows what the player saw (design §6.7): with the middle past the
    /// wall, the tile nearest it.
    fn save_of(&self, world: &World) -> Vec<u8> {
        let (area, map) = (self.tile_area, self.map_size);
        let nearest = |origin: i32, view: u16, len: u16| {
            (origin + i32::from(view / 2)).clamp(0, i32::from(len) - 1) as u16
        };
        let middle = Pos {
            x: nearest(self.viewport.x, self.seen_columns(), map.width),
            y: nearest(self.viewport.y, area.height, map.height),
        };
        world.save_with_view(middle)
    }

    /// Notes that the world, as it stands, has just been saved or loaded.
    fn saved_now(&mut self, world: &World) {
        self.saved = Some((world.tick(), self.running_for));
        self.unsaved_run = Duration::ZERO;
    }

    /// Whether the world has run since it was last saved or loaded, or, if
    /// it never was, since it began: whether a load would lose anything.
    fn has_run_since_save(&self, world: &World) -> bool {
        world.tick() != self.saved.map_or(0, |(tick, _)| tick)
    }

    /// When the world was last saved or loaded, as real time since, or
    /// `None` if it never has been: for the top bar (design §6.1).
    pub fn saved_ago(&self) -> Option<Duration> {
        self.saved.map(|(_, at)| self.running_for - at)
    }

    /// Autosaves if time has run, unpaused, for `AUTOSAVE_EVERY` since the
    /// last autosave (design §6.7). The frame loop calls it every frame.
    /// Says whether it was due, as the session log is written then too
    /// (design §2.7).
    pub fn autosave_if_due(&mut self, world: &World) -> bool {
        let due = self.since_autosave >= AUTOSAVE_EVERY;
        if due {
            // A failure shows on the status line.
            let _ = self.autosave(world);
        }
        due
    }

    /// Saves the world as the newest autosave (design §6.7), as on quitting,
    /// unless it hasn't run since it was last saved: then the autosaves
    /// already hold it, or a save does, and an older autosave stays.
    /// A replay never autosaves: it isn't the player's world. Gives why it
    /// couldn't, if it couldn't, as the status line says, for the frame
    /// loop to say on, as the player leaves the world (M2 design §8.4).
    pub fn autosave(&mut self, world: &World) -> Option<String> {
        self.since_autosave = Duration::ZERO;
        if self.replay.is_some() || !self.has_run_since_save(world) {
            return None;
        }
        let folder = self.save_folder.clone()?;
        match saves::autosave(&folder, &self.save_of(world)) {
            Ok(_) => {
                self.saved_now(world);
                self.tell_player("Autosaved".into());
                None
            }
            Err(err) => {
                let why = format!("Couldn't autosave: {err}");
                self.refuse(why.clone());
                Some(why)
            }
        }
    }

    /// `Ctrl+S`: starts naming a save, offering the seed and the tick
    /// (design §6.7).
    pub(super) fn start_save_naming(&mut self, world: &World) {
        self.end_aim();
        let offered = format!("seed {} tick {}", world.seed(), world.tick());
        self.save_naming = Some(Draft::offered(offered));
        self.screen = Screen::SaveNaming;
    }

    /// The save's name being typed, while naming one.
    pub fn save_name_draft(&self) -> Option<&str> {
        self.save_naming.as_ref().map(|draft| draft.text.as_str())
    }

    /// What an action does while naming a save: letters type, the first
    /// replacing the name offered; `Backspace` rubs one out; `Enter` saves;
    /// `Esc` gives up.
    pub(super) fn apply_save_naming(&mut self, action: Action, world: &World) -> Flow {
        let Some(draft) = self.save_naming.as_mut() else {
            self.screen = Screen::Normal;
            return Flow::Continue;
        };
        match action {
            // Only what the screen can show (design §7.2).
            Action::Type(c) if typeable(c) => draft.type_char(c, MAX_SAVE_NAME_CHARS),
            Action::Erase => draft.erase(),
            Action::Enter => {
                let draft = self.save_naming.take().expect("naming a save").text;
                self.screen = Screen::Normal;
                if draft.trim().is_empty() {
                    self.refuse("A save needs a name".into());
                } else {
                    self.save_as(&draft, world);
                }
            }
            Action::Back => {
                self.save_naming = None;
                self.screen = Screen::Normal;
            }
            Action::Point(cell) => self.point(cell),
            _ => {}
        }
        Flow::Continue
    }

    /// `F9`: loads the quicksave, if there is one (design §6.7).
    pub(super) fn quickload(&mut self, world: &World) {
        let Some(folder) = self.save_folder.clone() else {
            return self.refuse("There's no folder to load from".into());
        };
        let path = saves::path_for(&folder, QUICKSAVE);
        if !path.is_file() {
            return self.refuse("There's no quicksave yet: F5 makes one".into());
        }
        let file = SaveFile {
            name: QUICKSAVE.into(),
            path,
            modified: None,
        };
        self.ask_to_load(file, world);
    }

    /// `Ctrl+O`: opens the list of saves to load, newest first (design
    /// §6.7).
    pub(super) fn list_saves(&mut self) {
        self.end_aim();
        self.save_files = self
            .save_folder
            .as_deref()
            .map(saves::list)
            .unwrap_or_default();
        self.open_menu(Screen::LoadMenu);
    }

    /// What the load menu says with no saves to list.
    pub fn no_saves(&self) -> String {
        saves::none_listed(self.save_folder.as_deref())
    }

    /// Loads `file`, first asking if the world has run since it was last
    /// saved, as what has happened since would be lost (design §6.7).
    pub(super) fn ask_to_load(&mut self, file: SaveFile, world: &World) {
        if !self.has_run_since_save(world) {
            return self.load(file);
        }
        self.to_load = Some(file);
        self.screen = Screen::LoadPrompt;
    }

    /// The question before loading, while it waits for an answer.
    pub fn load_question(&self) -> Option<String> {
        let file = self.to_load.as_ref()?;
        let since = if self.saved.is_some() {
            format!(
                "The world has run {} since it was last saved",
                spoken_duration(self.unsaved_run)
            )
        } else {
            "This world has never been saved".into()
        };
        Some(format!("Load {}? {since} (y/n)", file.name))
    }

    /// What an action does while the question before loading waits: `y`
    /// loads, as does `F9` when the question is the quicksave's; the mouse
    /// carries on as usual; any other key cancels, but `F3`, which never
    /// reaches it (`apply`).
    pub(super) fn apply_load_prompt(&mut self, action: Action) -> Flow {
        let quicksave = self
            .to_load
            .as_ref()
            .is_some_and(|file| file.name == QUICKSAVE);
        let yes = action == Action::Confirm || (action == Action::Quickload && quicksave);
        match action {
            _ if yes => {
                self.screen = Screen::Normal;
                if let Some(file) = self.to_load.take() {
                    self.load(file);
                }
            }
            Action::Point(cell) => self.point(cell),
            _ if action.is_pointer() => {}
            _ => {
                self.to_load = None;
                self.screen = Screen::Normal;
            }
        }
        Flow::Continue
    }

    /// Reads `file` and loads the world in it, for the frame loop to take,
    /// or says on the status line why it couldn't (design §2.9).
    fn load(&mut self, file: SaveFile) {
        match saves::load(&file) {
            Ok(world) => self.loaded = Some((world, file.name)),
            Err(why) => self.refuse(why),
        }
    }

    /// Takes `world`, loaded from the save called `name` on the title
    /// screen (M2 design §8.2), for the frame loop to take with
    /// `take_loaded`, so it starts as a load in a world does (design §6.7).
    pub fn load_from_title(&mut self, world: World, name: String) {
        self.loaded = Some((world, name));
    }

    /// The world just loaded, if one was, for the frame loop to play from
    /// now on. The app starts afresh on it, as for a new world, paused so
    /// the player can see where they are (design §6.7). What's the
    /// player's rather than the world's carries over: the speed, the
    /// settings (the look, motion and square tiles, #307), the colours, the
    /// open tab and the folders.
    pub fn take_loaded(&mut self) -> Option<World> {
        let (world, name) = self.loaded.take()?;
        let areas = Areas {
            tiles: self.tile_area,
            inspector: self.inspector.room(),
            pops_over: self.inspector.pops_over(),
            event_log: self.event_log_area,
            overlay: self.overlay,
        };
        let mut fresh = App::new(world.map(), self.theme.clone(), world.seed(), areas);
        fresh.clock = std::mem::take(&mut self.clock);
        fresh.clock.pause();
        // A Story world loaded from a Lab one runs at 4× at most (M2
        // design §16).
        fresh.clock.set_fastest(Speed::fastest_in(world.mode()));
        fresh.policy = std::mem::replace(&mut self.policy, Box::new(Omniscient));
        fresh.colour_mode = self.colour_mode;
        fresh.event_filter = self.event_filter;
        fresh.list_sort = self.list_sort;
        fresh.tab = self.tab;
        // Open over the map or shut, as it was, but never filling the
        // screen, so the world loaded is in view (design §6.1, #305).
        fresh.inspector = self.inspector;
        fresh.zoom = self.zoom;
        fresh.motion.switch(self.motion.on());
        fresh.running_for = self.running_for;
        fresh.data_folder = self.data_folder.take();
        fresh.genome_folder = self.genome_folder.take();
        fresh.save_folder = self.save_folder.take();
        fresh.theme_folder = self.theme_folder.take();
        // The settings (design §6.7, #307): the look as picked, what the
        // player has changed, and where it's saved.
        fresh.look = self.look.clone();
        fresh.saved_settings = std::mem::take(&mut self.saved_settings);
        fresh.settings_file = self.settings_file.take();
        fresh.settings_unreadable = self.settings_unreadable;
        // The Cursor as the world left it: where it was, what it has hold
        // of, in Grab mode, which holds and leads, and whether sprites can
        // see it.
        let cursor = world.cursor();
        if cursor.leads().is_some() || cursor.holds().is_some() {
            fresh.mode = CursorMode::Grab;
        }
        if let Some(tile) = cursor.tile() {
            fresh.cursor = tile;
            fresh.pointed = tile;
            fresh.told_tile = Some(tile);
            fresh.centre_on(tile);
        }
        // The view as the player left it, if the save kept it.
        if let Some(view) = world.view() {
            fresh.centre_on(view);
        }
        if cursor.visible() {
            fresh.visible_in.insert(fresh.mode);
        }
        fresh.told_visible = cursor.visible();
        fresh.saved_now(&world);
        fresh.tell_player(format!(
            "Loaded {name}, at tick {}",
            thousands(world.tick())
        ));
        *self = fresh;
        // Its names show at once, paused, before any tick is taken in, and
        // the names of the dead it remembers (M2 design §17.3).
        self.note_names(&world);
        self.note_names_of_the_dead(&world);
        self.settle_cursor(&world);
        Some(world)
    }
}
