//! Naming a sprite (design §6.5): the name offered, and the name typed.

use terra_sim::{Command, EntityId, MAX_NAME_CHARS, World};

use super::{App, Draft, Flow, Naming, Screen, Selection};
use crate::cp437;
use crate::input::Action;

impl App {
    /// Whether keys type letters, rather than act: while naming a sprite or
    /// a save, filtering the sprite list (#313), or in the help or the
    /// command palette, which search what's typed (#303).
    pub fn typing(&self) -> bool {
        matches!(
            self.screen,
            Screen::Naming | Screen::SaveNaming | Screen::Help | Screen::Palette
        ) || self.screen == Screen::SpriteList && self.list_filter.is_some()
    }

    /// The name being typed, while naming (design §6.5).
    pub fn name_draft(&self) -> Option<&str> {
        self.naming
            .as_ref()
            .map(|naming| naming.draft.text.as_str())
    }

    /// The sprite being named, while naming.
    pub fn naming_sprite(&self) -> Option<EntityId> {
        self.naming.as_ref().map(|naming| naming.sprite)
    }

    /// Starts naming the selected sprite, offering a random name to start
    /// from (design §6.5).
    pub(super) fn start_naming(&mut self, world: &World) {
        let Some(Selection::Living(sprite)) = self.selection else {
            return self.refuse("Select a sprite to name it".into());
        };
        let draft = self.random_name(sprite, world);
        self.naming = Some(Naming {
            sprite,
            draft: Draft::offered(draft),
        });
        self.screen = Screen::Naming;
    }

    /// Another random name for `sprite`, made up by the screen, so naming
    /// never draws from the world's randomness (design §6.5).
    fn random_name(&mut self, sprite: EntityId, world: &World) -> String {
        self.names_offered += 1;
        // Mixes the session's seed, the sprite and how many names have been
        // offered, so each offer differs; the odd constants are the usual
        // 64-bit multiplicative hash ones, spreading nearby numbers apart.
        let seed = self
            .seed
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .wrapping_add(sprite.0.wrapping_mul(0xbf58_476d_1ce4_e5b9))
            .wrapping_add(self.names_offered);
        world.data().random_name(seed)
    }

    /// What an action does while the player types a name: letters CP437
    /// can show, and spaces, are typed, up to the most a name may have, the first
    /// replacing the offered name; `Backspace` rubs one out; `Tab` offers
    /// another; `Enter` sends the name and `Esc` gives up.
    pub(super) fn apply_naming(&mut self, action: Action, world: &World) -> Flow {
        let Some(naming) = self.naming.as_mut() else {
            self.screen = Screen::Normal;
            return Flow::Continue;
        };
        match action {
            // Only letters CP437 can show, and spaces between them (design v28 §6.5).
            Action::Type(c) if (c.is_alphabetic() || c == ' ') && cp437::contains(c) => {
                naming.draft.type_char(c, MAX_NAME_CHARS);
            }
            Action::Erase => naming.draft.erase(),
            Action::AnotherName => {
                let sprite = naming.sprite;
                let draft = self.random_name(sprite, world);
                let naming = self.naming.as_mut().expect("naming");
                naming.draft = Draft::offered(draft);
            }
            Action::Enter => {
                let naming = self.naming.take().expect("naming");
                self.screen = Screen::Normal;
                if naming.draft.text.trim_matches(' ').is_empty() {
                    self.refuse("A name needs a letter in it".into());
                } else {
                    self.commands.push(Command::Rename {
                        sprite: naming.sprite,
                        name: naming.draft.text,
                    });
                }
            }
            Action::Back => {
                self.naming = None;
                self.screen = Screen::Normal;
            }
            Action::Point(cell) => self.point(cell),
            _ => {}
        }
        Flow::Continue
    }
}
