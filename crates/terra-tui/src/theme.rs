//! Themes map semantic tiles to glyphs and colours, and each text role to a
//! style, may give the panels their shades (design §6.2), and give motion
//! its colours, say how a drive's bar shows its drive's pace and give the
//! characters of a hatching's shell (§6.9).
//! They are UI assets and never affect the simulation.
//! A theme is a glyph set, its characters, and a palette, the rest, chosen
//! apart (#307). Two glyph sets and five palettes are embedded in the
//! binary, and `--theme <file>` loads one the player has edited (design v34
//! §6.7).

use std::collections::BTreeMap;
use std::path::PathBuf;

use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;
use terra_sim::{DataPack, Terrain};

use crate::app::{CursorMode, StatusMark};
use crate::campbell;
use crate::family_tree::Lineage;
use crate::graph::Band;
use crate::listed::listed_enum;
use crate::space::{Depth, Pane};

/// What the map view draws for a tile, named by meaning rather than by character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticTile {
    Terrain(Terrain),
    Sprite,
    /// The selected sprite (design §6.1).
    SelectedSprite,
    /// A young sprite, drawn small (M2 design §9.6).
    YoungSprite,
    /// The selected sprite, while it's young.
    SelectedYoungSprite,
    /// Retired: a sprite that was still an egg. An egg is an item until it
    /// hatches (M2 design §10.4), drawn as the theme's `objects` say, so a
    /// theme file from before still loads, and draws the egg item with this
    /// glyph if its `objects` have no "egg".
    Egg,
    /// Retired with `Egg`: read, and not drawn. A selected egg is drawn in
    /// reverse video.
    SelectedEgg,
    /// The Decision marker: where the selected sprite is heading (design
    /// §6.1). It flashes.
    DecisionMarker,
    /// An emote, which takes turns with a sprite's glyph (design §6.3).
    Emote(Emote),
}

/// Something that just happened to a sprite, which the map shows by
/// swapping its glyph for a while (design §6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Emote {
    /// Something hurt it.
    Hurt,
    /// The Cursor petted or hugged it (design v21 §6.3).
    Pleased,
    /// The Cursor zapped or shocked it (design v21 §6.3).
    Shocked,
    /// It gave up an action: failed, blocked or timed out.
    Failed,
    /// It's resting.
    Resting,
    /// It danced for another sprite (M2 design §15.8).
    Dancing,
    /// It called (M2 design §15.8).
    Calling,
}

impl SemanticTile {
    /// Every semantic tile. Each theme must draw all of them.
    pub const ALL: [SemanticTile; 14] = [
        SemanticTile::Terrain(Terrain::Grass),
        SemanticTile::Terrain(Terrain::Dirt),
        SemanticTile::Terrain(Terrain::Sand),
        SemanticTile::Terrain(Terrain::ShallowWater),
        SemanticTile::Terrain(Terrain::DeepWater),
        SemanticTile::Terrain(Terrain::Rock),
        SemanticTile::Sprite,
        SemanticTile::SelectedSprite,
        SemanticTile::DecisionMarker,
        SemanticTile::Emote(Emote::Hurt),
        SemanticTile::Emote(Emote::Pleased),
        SemanticTile::Emote(Emote::Shocked),
        SemanticTile::Emote(Emote::Failed),
        SemanticTile::Emote(Emote::Resting),
    ];

    /// The tiles a theme may leave out, each with the tile drawn in its
    /// place: a theme from before the young had a look of their own draws
    /// them as grown (M2 design §9.6), and one from before courtship draws a
    /// dance and a call as pleased (§15.8).
    pub const OPTIONAL: [(SemanticTile, SemanticTile); 4] = [
        (SemanticTile::YoungSprite, SemanticTile::Sprite),
        (
            SemanticTile::SelectedYoungSprite,
            SemanticTile::SelectedSprite,
        ),
        (
            SemanticTile::Emote(Emote::Dancing),
            SemanticTile::Emote(Emote::Pleased),
        ),
        (
            SemanticTile::Emote(Emote::Calling),
            SemanticTile::Emote(Emote::Pleased),
        ),
    ];
}

/// How a theme draws one semantic tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    pub symbol: char,
    pub fg: Color,
    pub bold: bool,
    /// Drawn in reverse video.
    pub reversed: bool,
}

/// How a theme draws the population graph (M2 design §14): the colour of
/// each life stage and of eggs, the blocks they're stacked in, each band's
/// own character for a palette with no colour, and the dotted line of the
/// world's limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopulationLook {
    pub young: Color,
    pub adult: Color,
    pub old: Color,
    pub egg: Color,
    /// A whole character of one band.
    pub full: char,
    /// The lower half of a character: drawn in the band below, over the band
    /// above, or over nothing at the top of the stack.
    pub half: char,
    /// Each band's own character, which a palette with no colour draws it
    /// in, so the bands are told apart without colour (design §6.2, #307).
    pub bands: PopulationBands,
    /// A dot of the line at the world's limit.
    pub limit: Glyph,
}

/// The character of each band of the population graph where the palette
/// has no colour (design §6.2, #307): in the CP437 glyph set, `░` eggs, `▓`
/// young, `█` adults and `▒` old, as M2 design §14.2 draws them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopulationBands {
    pub young: char,
    pub adult: char,
    pub old: char,
    pub egg: char,
}

impl PopulationLook {
    /// The colour the graph draws `band` in.
    pub(crate) fn colour(&self, band: Band) -> Color {
        match band {
            Band::Young => self.young,
            Band::Adults => self.adult,
            Band::Old => self.old,
            Band::Eggs => self.egg,
        }
    }

    /// The character the graph draws `band` in where the palette has no
    /// colour.
    pub(crate) fn character(&self, band: Band) -> char {
        let bands = self.bands;
        match band {
            Band::Young => bands.young,
            Band::Adults => bands.adult,
            Band::Old => bands.old,
            Band::Eggs => bands.egg,
        }
    }
}

/// How a theme draws the lineage colours (M2 design §17.4): the colour of
/// the selected sprite's parents and grandparents, its brothers and
/// sisters, its young and theirs, and every other sprite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineageLook {
    pub elders: Color,
    pub siblings: Color,
    pub young: Color,
    pub others: Color,
}

impl LineageLook {
    /// The colour of a sprite that stands in the selected one's lineage as
    /// `lineage` says, or of one outside it.
    pub(crate) fn colour(&self, lineage: Option<Lineage>) -> Color {
        match lineage {
            Some(Lineage::Elder) => self.elders,
            Some(Lineage::Sibling) => self.siblings,
            Some(Lineage::Young) => self.young,
            None => self.others,
        }
    }
}

/// How a theme styles the screen's text by its text role (design §6.2,
/// #302): the headings, minor text, gains and losses, the key in each hint,
/// and what's chosen, such as the open tab. Colour never carries a meaning
/// alone: the signs, arrows and capitals stay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextLook {
    /// A heading, such as `ATTENTION` or `Observed`.
    pub heading: Style,
    /// Minor text: a log line's tick, `and 3 more`, a `·` between parts,
    /// a bar's empty part, what a hint's key does.
    pub muted: Style,
    /// A gain, such as `+.99`.
    pub good: Style,
    /// A loss, such as `-.19`.
    pub bad: Style,
    /// The key in a hint, such as the `Z` of `Z select`.
    pub key: Style,
    /// What's chosen, such as the open tab.
    pub selected: Style,
    /// What a tooltip can explain, such as a drive's name or a gene's, and
    /// the name of a log line's sprite (design §6.1, #304). The spot under
    /// the pointer is drawn in it in reverse video.
    pub spot: Style,
}

impl TextLook {
    /// The style of a signed number such as `+.99` or `-.19`: a gain good,
    /// a loss bad, and anything else, a zero such as `+.00` too, in
    /// `otherwise`.
    pub fn signed(&self, number: &str, otherwise: Style) -> Style {
        let zero = number
            .chars()
            .filter(char::is_ascii_digit)
            .all(|d| d == '0');
        match number.chars().next() {
            _ if zero => otherwise,
            Some('+') => self.good,
            Some('-') => self.bad,
            _ => otherwise,
        }
    }
}

/// The shades the panels beside and under the map sit on (design §6.1,
/// #302): the inspector and the event log on `shade`, their title rows on
/// `title`. A theme without them draws lines between the panels instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelLook {
    pub shade: Color,
    pub title: Color,
}

/// Motion's colours (design §6.9): exact ones in the default theme, since
/// a fade blends them. A colour a theme names blends from how Campbell, the
/// renderer's scheme, draws it. And how a drive's bar shows how fast its
/// drive moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotionLook {
    /// What a death's tile flashes to, fading back.
    pub flash: Color,
    /// The grey a death's ghost fades to.
    pub ash: Color,
    /// What a hatching's sparks light the glyphs round the egg with, and
    /// what the tile of an egg about to hatch glows towards.
    pub spark: Color,
    /// The tint the trail leaves on the tiles the selected sprite has just
    /// left.
    pub trail: Color,
    /// The dark a pop-up fades in from and out to, and the background a
    /// fade takes on a tile that has none of its own.
    pub dark: Color,
    /// How a drive's bar shows how fast its drive moves.
    pub bar: BarWaves,
}

/// The pieces of an egg's shell as it hatches, while motion is on (design
/// §6.9): the egg cracked, on the young's tile for a moment, then its two
/// halves either side of the egg's tile and four crumbs on the tiles at
/// its corners. Characters only, from the glyph set: they're drawn in the
/// egg's colour, fading to their tile's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellLook {
    /// The egg cracked across, before the young appears.
    pub cracked: char,
    /// The half left of the egg's tile.
    pub left_half: char,
    /// The half right of it.
    pub right_half: char,
    /// The crumb up and left of it.
    pub up_left: char,
    /// The crumb up and right of it.
    pub up_right: char,
    /// The crumb down and left of it.
    pub down_left: char,
    /// The crumb down and right of it.
    pub down_right: char,
}

/// How a drive's bar shows how fast its drive moves, while motion is on
/// (design §6.9): waves that come taller, more often and further the
/// faster it rises against its usual rise, and a quiet dip while it falls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarWaves {
    /// Crashing waves: a crest runs up the bar to its end, breaks there in
    /// a flare, and its spray runs on into the empty blocks past the level.
    Crashing,
    /// A running ripple: a crest runs up the bar and off its end.
    Ripple,
}

/// The default theme's file: the CP437 glyph set and the default palette.
const CP437: &str = include_str!("../../../themes/cp437.ron");

/// The ASCII glyph set's file, which draws the default palette too.
const ASCII: &str = include_str!("../../../themes/ascii.ron");

/// The high-contrast palette's file.
const HIGH_CONTRAST: &str = include_str!("../../../themes/high_contrast.ron");

/// The colour-blind palette's file.
const COLOUR_BLIND: &str = include_str!("../../../themes/colour_blind.ron");

/// The mono palette's file.
const MONO: &str = include_str!("../../../themes/mono.ron");

/// The Game Kid palette's file.
const GAME_KID: &str = include_str!("../../../themes/game_kid.ron");

listed_enum! {
    /// A glyph set built into the game (design §6.2, #307): the characters a
    /// theme draws with.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum GlyphSet {
        /// CP437's, the default.
        Cp437 => "CP437",
        /// Plain ASCII's, as `--ascii` asks.
        Ascii => "ASCII",
    }

    /// Every glyph set built in, in the order the game offers them.
    pub const ALL;

    /// Its name, as the game shows it (design §6.7).
    pub fn name(self) -> &'static str;
}

impl GlyphSet {
    /// The built-in theme its characters are taken from.
    pub fn source(self) -> Theme {
        match self {
            GlyphSet::Cp437 => Theme::builtin("cp437", CP437),
            GlyphSet::Ascii => Theme::builtin("ascii", ASCII),
        }
    }
}

listed_enum! {
    /// A palette built into the game (design §6.2, #307): the colours, bold
    /// and reverse video a theme draws its glyphs in, and the rest of a theme
    /// but its characters. Not the command palette, `:`'s box.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum ColourPalette {
        /// The default theme's colours.
        Default => "default",
        /// Bright exact colours, each at 7:1 or more against black, and lines
        /// between the panels.
        HighContrast => "high contrast",
        /// Okabe and Ito's colours, for readers who can't tell red from green.
        ColourBlind => "colour-blind",
        /// No colour at all, only bold and reverse video, where `NO_COLOR`
        /// starts the game.
        Mono => "mono",
        /// Greens on the terminal's black, after the original Game Boy's
        /// four.
        GameKid => "Game Kid",
    }

    /// Every palette built in, in the order the game offers them.
    pub const ALL;

    /// Its name, as the game shows it (design §6.7).
    pub fn name(self) -> &'static str;
}

impl ColourPalette {
    /// The built-in theme it's taken from. Each draws the CP437 glyph set,
    /// so its file reads as a whole theme too.
    pub fn source(self) -> Theme {
        match self {
            ColourPalette::Default => Theme::builtin("cp437", CP437),
            ColourPalette::HighContrast => Theme::builtin("high contrast", HIGH_CONTRAST),
            ColourPalette::ColourBlind => Theme::builtin("colour-blind", COLOUR_BLIND),
            ColourPalette::Mono => Theme::builtin("mono", MONO),
            ColourPalette::GameKid => Theme::builtin("Game Kid", GAME_KID),
        }
    }
}

/// A glyph set or a palette as the player picks it (design §6.2, §6.7,
/// #307): one built in, `T`, or a theme file of their own, which gives
/// whichever part it's picked as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick<T> {
    /// One built into the game.
    BuiltIn(T),
    /// A theme file, not yet read.
    File(PathBuf),
}

/// One of the two parts a theme is chosen in, as the game builds them in
/// (design §6.2, #307): a glyph set or a palette.
pub trait Part: Copy + Eq + 'static {
    /// Every one built in, in the order the game offers them.
    fn all() -> &'static [Self];

    /// Its name, as the game shows it.
    fn name(self) -> &'static str;

    /// The built-in theme it's taken from.
    fn source(self) -> Theme;
}

impl Part for GlyphSet {
    fn all() -> &'static [GlyphSet] {
        &GlyphSet::ALL
    }

    fn name(self) -> &'static str {
        GlyphSet::name(self)
    }

    fn source(self) -> Theme {
        GlyphSet::source(self)
    }
}

impl Part for ColourPalette {
    fn all() -> &'static [ColourPalette] {
        &ColourPalette::ALL
    }

    fn name(self) -> &'static str {
        ColourPalette::name(self)
    }

    fn source(self) -> Theme {
        ColourPalette::source(self)
    }
}

/// The egg's object type in the built-in data pack (M2 design §10.4), which
/// a theme from before eggs were items drew with its `egg` tile.
const EGG: &str = "egg";

/// What a theme draws for an object it has no glyph for, so a data pack's new
/// object types still show.
const UNKNOWN_OBJECT: Glyph = Glyph {
    symbol: '?',
    fg: Color::White,
    bold: false,
    reversed: false,
};

/// The cursor's arrows (design §6.5), named by the way they point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arrows {
    pub up: char,
    pub down: char,
    pub left: char,
    pub right: char,
}

/// What the cursor's status marks, `Y` and `N`, can show (design §6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusMarks {
    /// Nothing to report.
    pub idle: char,
    /// A click has just sent a command.
    pub sent: char,
    /// The world has just applied one.
    pub applied: char,
    /// The world has just refused one.
    pub rejected: char,
    /// In Grab mode, empty: `Y`, a click would grab something (design v23
    /// §6.5).
    pub grab: char,
    /// In Grab mode, empty: `N`.
    pub empty: char,
    /// In Grab mode, holding or leading: `Y`, a click would let go or put
    /// it down.
    pub release: char,
}

impl StatusMarks {
    /// The glyph the status marks show for `mark`, or `None` for the thing
    /// the Cursor holds or leads, which shows its own glyph.
    pub fn glyph(self, mark: StatusMark) -> Option<char> {
        Some(match mark {
            StatusMark::Idle => self.idle,
            StatusMark::Sent => self.sent,
            StatusMark::Applied => self.applied,
            StatusMark::Rejected => self.rejected,
            StatusMark::Grab => self.grab,
            StatusMark::Empty => self.empty,
            StatusMark::Release => self.release,
            StatusMark::Holding => return None,
        })
    }
}

/// A mapping from semantic tiles, and the cursor, to glyphs and colours:
/// a glyph set and a palette, which may come from different theme files
/// ([`Theme::compose`], design §6.2, #307).
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    tiles: BTreeMap<SemanticTile, Glyph>,
    /// By object type name, then visual state.
    objects: BTreeMap<String, BTreeMap<String, Glyph>>,
    /// A theme from before eggs were items: its `egg` tile, which draws the
    /// egg item if `objects` has no "egg" (M2 design §10.4).
    old_egg: Option<Glyph>,
    arrows: Arrows,
    followed_arrows: Arrows,
    visible_frame: Arrows,
    status_marks: StatusMarks,
    mode_marks: BTreeMap<CursorMode, Glyph>,
    leash: Glyph,
    aim: Glyph,
    aim_end: Glyph,
    attention_marker: Color,
    /// By drive name, the colour of a sprite whose strongest drive it is.
    drives: BTreeMap<String, Color>,
    /// The glass round the map, by piece (M2 design §11.2).
    glass: BTreeMap<Pane, Glyph>,
    /// The stars past it, by depth (M2 design §11.3).
    stars: BTreeMap<Depth, Glyph>,
    /// The population graph (M2 design §14).
    population: PopulationLook,
    /// The lineage colours (M2 design §17.4).
    lineage: LineageLook,
    /// The screen's text, by its text role (design §6.2).
    text: TextLook,
    /// The panels' shades, if it has them (design §6.1).
    panels: Option<PanelLook>,
    /// Motion's colours, and the bars' waves (design §6.9).
    motion: MotionLook,
    /// A hatching's shell, its glyph set's (design §6.9).
    shell: ShellLook,
    /// Whether its palette has colour at all: mono's has none (design
    /// §6.2, #307).
    colour: bool,
    /// The colour its palette draws the terminal's own text colour in, if
    /// it gives one, as Game Kid does (design §6.2).
    foreground: Option<Color>,
    /// Whether its palette's colours can tell the drives and the lineage
    /// apart: Game Kid's greens can't (design §6.2, §6.3).
    colour_modes: bool,
}

impl Theme {
    /// The default theme: CP437 glyphs in the default palette.
    pub fn cp437() -> Theme {
        GlyphSet::Cp437.source()
    }

    /// The same meanings in plain ASCII, as `--ascii` draws them: ASCII
    /// glyphs in the default palette (design §6.2, #307).
    pub fn ascii() -> Theme {
        Theme::built_in(GlyphSet::Ascii, ColourPalette::Default)
    }

    /// A built-in glyph set in a built-in palette (design §6.2, #307).
    pub fn built_in(glyphs: GlyphSet, palette: ColourPalette) -> Theme {
        Theme::compose(&glyphs.source(), &palette.source())
    }

    /// `glyphs`' glyph set in `palette`'s palette (design §6.2, #307): every
    /// character `glyphs` draws, a hatching's shell's too (§6.9), each in
    /// the colour, bold and reverse video `palette` gives what it stands
    /// for, and `palette`'s drives, lineage, graph colours, text roles,
    /// attention marker, panel shades and motion's section. An object one of them has no glyph for falls back
    /// in that one, as [`Theme::object_glyph`] says.
    pub fn compose(glyphs: &Theme, palette: &Theme) -> Theme {
        let tiles = redrawn(&palette.tiles, |tile| glyphs.glyph(tile));
        let mut objects: BTreeMap<String, BTreeMap<String, Glyph>> = BTreeMap::new();
        for (object, state) in glyphs.object_states().chain(palette.object_states()) {
            let glyph = drawn(
                glyphs.object_glyph(object, state),
                palette.object_glyph(object, state),
            );
            objects
                .entry(object.to_string())
                .or_default()
                .insert(state.to_string(), glyph);
        }
        let old_egg = (glyphs.old_egg.is_some() || palette.old_egg.is_some()).then(|| {
            drawn(
                glyphs.object_glyph(EGG, "default"),
                palette.object_glyph(EGG, "default"),
            )
        });
        let graph = palette.population;
        Theme {
            tiles,
            objects,
            old_egg,
            arrows: glyphs.arrows,
            followed_arrows: glyphs.followed_arrows,
            visible_frame: glyphs.visible_frame,
            status_marks: glyphs.status_marks,
            mode_marks: redrawn(&palette.mode_marks, |mode| glyphs.mode_mark(mode)),
            leash: drawn(glyphs.leash, palette.leash),
            aim: drawn(glyphs.aim, palette.aim),
            aim_end: drawn(glyphs.aim_end, palette.aim_end),
            glass: redrawn(&palette.glass, |pane| glyphs.glass(pane)),
            stars: redrawn(&palette.stars, |depth| glyphs.star(depth)),
            population: PopulationLook {
                full: glyphs.population.full,
                half: glyphs.population.half,
                bands: glyphs.population.bands,
                limit: drawn(glyphs.population.limit, graph.limit),
                ..graph
            },
            shell: glyphs.shell,
            ..palette.clone()
        }
    }

    /// Every object type and visual state this theme has a glyph for.
    fn object_states(&self) -> impl Iterator<Item = (&str, &str)> {
        self.objects.iter().flat_map(|(object, states)| {
            states
                .keys()
                .map(move |state| (object.as_str(), state.as_str()))
        })
    }

    /// How this theme draws `tile`.
    pub fn glyph(&self, tile: SemanticTile) -> Glyph {
        self.tiles[&tile]
    }

    /// How this theme draws an object of type `object` in visual state `state`
    /// (design §6.2). With no glyph for that state, it uses the object's
    /// `"default"` look; for an egg, a theme from before eggs were items
    /// uses its `egg` tile (M2 design §10.4); and failing that, a `?`.
    pub fn object_glyph(&self, object: &str, state: &str) -> Glyph {
        self.object_entry(object, state)
            .or_else(|| self.object_entry(object, "default"))
            .or(self.old_egg.filter(|_| object == EGG))
            .unwrap_or(UNKNOWN_OBJECT)
    }

    /// The glyph this theme gives exactly this object type and visual state, if any.
    pub fn object_entry(&self, object: &str, state: &str) -> Option<Glyph> {
        self.objects.get(object)?.get(state).copied()
    }

    /// The Attention marker: the background of the tile the selected sprite
    /// attends to.
    pub fn attention_marker(&self) -> Color {
        self.attention_marker
    }

    /// The colour of a sprite whose strongest drive is `drive`, if the
    /// theme gives it one (design §6.3).
    pub fn drive_colour(&self, drive: &str) -> Option<Color> {
        self.drives.get(drive).copied()
    }

    /// The Cursor's arrows.
    pub fn arrows(&self) -> Arrows {
        self.arrows
    }

    /// The Cursor's arrows while it follows a sprite: solid (design
    /// v26 §6.2).
    pub fn followed_arrows(&self) -> Arrows {
        self.followed_arrows
    }

    /// The Cursor's sides while sprites can see it, in place of the arrows,
    /// following or not: a frame of light (design v29 §6.5).
    pub fn visible_frame(&self) -> Arrows {
        self.visible_frame
    }

    /// What the cursor's status marks show.
    pub fn status_marks(&self) -> StatusMarks {
        self.status_marks
    }

    /// The mode mark for `mode`. The cursor's arrows and status marks take its colour.
    pub fn mode_mark(&self, mode: CursorMode) -> Glyph {
        self.mode_marks[&mode]
    }

    /// The leash, a dot of the line from the Cursor to a sprite it leads
    /// (design v23 §6.5).
    pub fn leash(&self) -> Glyph {
        self.leash
    }

    /// A dot of the aim line, along the path a throw or a shove will take
    /// (design v25 §6.5).
    pub fn aim(&self) -> Glyph {
        self.aim
    }

    /// The aim line's end: where a throw or a shove would stop if nothing's
    /// in the way (design v25 §6.5).
    pub fn aim_end(&self) -> Glyph {
        self.aim_end
    }

    /// A piece of the glass round the map (M2 design §11.2).
    pub fn glass(&self, pane: Pane) -> Glyph {
        self.glass[&pane]
    }

    /// A star of `depth`, in space past the glass (M2 design §11.3).
    pub fn star(&self, depth: Depth) -> Glyph {
        self.stars[&depth]
    }

    /// How the population graph is drawn (M2 design §14).
    pub fn population(&self) -> PopulationLook {
        self.population
    }

    /// The lineage colours (M2 design §17.4).
    pub fn lineage(&self) -> LineageLook {
        self.lineage
    }

    /// How the screen's text is styled by its text role (design §6.2, #302).
    pub fn text(&self) -> TextLook {
        self.text
    }

    /// The shades the inspector and the event log sit on, or `None` for a
    /// theme that draws lines between the panels instead (design §6.1,
    /// #302): one without them, or without colour, whose shades couldn't
    /// be seen (§6.2, #307).
    pub fn panels(&self) -> Option<PanelLook> {
        self.panels.filter(|_| self.colour)
    }

    /// Motion's colours, and how a drive's bar shows its drive's pace
    /// (design §6.9).
    pub fn motion(&self) -> MotionLook {
        self.motion
    }

    /// A hatching's shell: the egg cracked, its halves and its crumbs
    /// (design §6.9).
    pub fn shell(&self) -> ShellLook {
        self.shell
    }

    /// The colour this theme's palette draws what the game leaves in the
    /// terminal's own text colour, or `None` to leave it to the terminal
    /// (design §6.2).
    pub fn foreground(&self) -> Option<Color> {
        self.foreground
    }

    /// What this theme's palette does to the finished screen, which the
    /// screen's drawing calls last (design §6.2): with no colour, it draws
    /// none ([`without_colour`]), and with a foreground, every grey in its
    /// shades ([`in_shades_of`]).
    pub fn finish(&self, buffer: &mut Buffer) {
        if !self.colour {
            without_colour(buffer);
        } else if let Some(foreground) = self.foreground {
            in_shades_of(buffer, foreground);
        }
    }

    /// Whether this theme's palette can show the colour modes (design §6.2,
    /// §6.3): a sprite's colour by its strongest drive, or by its lineage.
    /// One that can't, such as mono, with no colour, or Game Kid, whose
    /// greens can't tell them apart, draws every sprite in its own colour,
    /// and `b` and the help say where they're read instead.
    pub fn shows_colour_modes(&self) -> bool {
        self.colour && self.colour_modes
    }

    /// Whether this theme's palette has colour at all (design §6.2, #307).
    /// One without, such as mono, is drawn in the terminal's own colours,
    /// with only bold and reverse video ([`without_colour`]); the tile the
    /// selected sprite attends to is underlined, since its background can't
    /// be seen, and motion's colours don't play.
    pub fn has_colour(&self) -> bool {
        self.colour
    }

    /// A theme from the text of a theme file, as `--theme <file>` loads it
    /// (design v34 §6.7), checked against the data pack it will draw.
    ///
    /// It must draw every semantic tile and mode mark, with glyphs CP437
    /// has (design §6.2). It may leave out object types and drives, which
    /// fall back, but every object type, visual state and drive it names
    /// must be in `data`, so a misspelt one is caught rather than drawn as
    /// `?`. The error says what's wrong, in the file's own words.
    pub fn from_ron(text: &str, data: &DataPack) -> Result<Theme, String> {
        let theme = Theme::parse(text)?;
        for (object, states) in &theme.objects {
            if !data.object_type_names().any(|name| name == object) {
                return Err(format!("the data pack has no object type \"{object}\""));
            }
            let known = data.visual_states(object);
            if let Some(state) = states.keys().find(|state| !known.contains(&state.as_str())) {
                return Err(format!(
                    "\"{object}\" has no visual state \"{state}\": it has {}",
                    quoted(&known)
                ));
            }
        }
        if let Some(drive) = theme
            .drives
            .keys()
            .find(|drive| !data.drives().any(|name| name == drive.as_str()))
        {
            let drives: Vec<&str> = data.drives().collect();
            return Err(format!(
                "the data pack has no drive \"{drive}\": its drives are {}",
                quoted(&drives)
            ));
        }
        Ok(theme)
    }

    fn builtin(name: &str, text: &str) -> Theme {
        Theme::parse(text).unwrap_or_else(|e| panic!("the built-in {name} theme is invalid: {e}"))
    }

    /// A theme from a theme file's text, complete and within CP437.
    fn parse(text: &str) -> Result<Theme, String> {
        let file: ThemeFile = ron::from_str(text).map_err(|e| e.to_string())?;
        let mut tiles = glyphs(file.tiles);
        // Eggs are items now (M2 design §10.4).
        let old_egg = tiles.remove(&SemanticTile::Egg);
        tiles.remove(&SemanticTile::SelectedEgg);
        if let Some(&tile) = SemanticTile::ALL
            .iter()
            .find(|tile| !tiles.contains_key(tile))
        {
            return Err(format!("it has no glyph for {}", tile_name(tile)));
        }
        for (tile, instead) in SemanticTile::OPTIONAL {
            let glyph = tiles[&instead];
            tiles.entry(tile).or_insert(glyph);
        }
        let mode_marks = glyphs(file.cursor.mode_marks);
        if let Some(&mode) = CursorMode::ALL
            .iter()
            .find(|mode| !mode_marks.contains_key(mode))
        {
            return Err(format!("it has no mode mark for {}", snake_case(&mode)));
        }
        let objects = file
            .objects
            .into_iter()
            .map(|(object, states)| (object, glyphs(states)))
            .collect();
        // Any piece of glass or depth of stars a theme leaves out, it draws
        // as the default theme does (M2 design §11.5), such as one made
        // before there was space.
        let builtin: ThemeFile = ron::from_str(CP437)
            .unwrap_or_else(|e| panic!("the built-in cp437 theme is invalid: {e}"));
        let mut space = builtin.space.expect("the built-in cp437 theme draws space");
        assert!(
            Pane::ALL.iter().all(|pane| space.glass.contains_key(pane))
                && Depth::ALL
                    .iter()
                    .all(|depth| space.stars.contains_key(depth)),
            "the built-in cp437 theme draws every piece of glass and depth of stars"
        );
        if let Some(theirs) = file.space {
            space.glass.extend(theirs.glass);
            space.stars.extend(theirs.stars);
        }
        // And any part of the population graph (M2 design §14).
        let population = file
            .population
            .unwrap_or_default()
            .look(builtin.population.unwrap_or_default())
            .expect("the built-in cp437 theme draws every part of the population graph");
        // And any of the lineage colours (M2 design §17.4).
        let lineage = file
            .lineage
            .unwrap_or_default()
            .look(builtin.lineage.unwrap_or_default())
            .expect("the built-in cp437 theme has every lineage colour");
        // And any of the text's styles (design §6.2, #302). The panels'
        // shades aren't the default theme's: a theme without them draws
        // lines between the panels.
        let text = file
            .text
            .unwrap_or_default()
            .look(builtin.text.unwrap_or_default())
            .expect("the built-in cp437 theme styles every text role");
        // And any of motion's colours, and the bars' waves (design §6.9).
        let motion = file
            .motion
            .unwrap_or_default()
            .look(builtin.motion.unwrap_or_default())
            .expect("the built-in cp437 theme has all of motion's section");
        // And any piece of a hatching's shell (design §6.9).
        let shell = file
            .shell
            .unwrap_or_default()
            .look(builtin.shell.unwrap_or_default())
            .expect("the built-in cp437 theme has every piece of the shell");
        let theme = Theme {
            tiles,
            objects,
            old_egg,
            arrows: file.cursor.arrows,
            followed_arrows: file.cursor.followed_arrows,
            visible_frame: file.cursor.visible_frame,
            status_marks: file.cursor.status_marks,
            mode_marks,
            leash: glyph(file.cursor.leash),
            aim: glyph(file.cursor.aim.path),
            aim_end: glyph(file.cursor.aim.end),
            attention_marker: file.attention_marker.into(),
            drives: file
                .drives
                .into_iter()
                .map(|(drive, colour)| (drive, colour.into()))
                .collect(),
            glass: glyphs(space.glass),
            stars: glyphs(space.stars),
            population,
            lineage,
            text,
            panels: file.panels.map(|panels| PanelLook {
                shade: panels.shade.into(),
                title: panels.title.into(),
            }),
            motion,
            shell,
            colour: file.colour,
            foreground: file.foreground.map(Color::from),
            colour_modes: file.colour_modes,
        };
        if let Some((what, symbol)) = theme
            .symbols()
            .into_iter()
            .find(|&(_, symbol)| !terra_sim::is_cp437(symbol))
        {
            return Err(format!(
                "its glyph '{symbol}' for {what} isn't in CP437, which everything on screen stays within"
            ));
        }
        Ok(theme)
    }

    /// Every character this theme draws, with what it's for, in the file's
    /// words.
    fn symbols(&self) -> Vec<(String, char)> {
        let tiles = self
            .tiles
            .iter()
            .map(|(&tile, glyph)| (tile_name(tile), glyph.symbol))
            .chain(
                self.old_egg
                    .map(|glyph| (tile_name(SemanticTile::Egg), glyph.symbol)),
            );
        let objects = self.objects.iter().flat_map(|(object, states)| {
            states
                .iter()
                .map(move |(state, glyph)| (format!("\"{object}\" \"{state}\""), glyph.symbol))
        });
        let arrows = [
            ("arrows", self.arrows),
            ("followed_arrows", self.followed_arrows),
            ("visible_frame", self.visible_frame),
        ]
        .into_iter()
        .flat_map(|(name, a)| {
            [
                ("up", a.up),
                ("down", a.down),
                ("left", a.left),
                ("right", a.right),
            ]
            .map(|(side, symbol)| (format!("the cursor's {name} {side}"), symbol))
        });
        let m = self.status_marks;
        let marks = [
            ("idle", m.idle),
            ("sent", m.sent),
            ("applied", m.applied),
            ("rejected", m.rejected),
            ("grab", m.grab),
            ("empty", m.empty),
            ("release", m.release),
        ]
        .map(|(mark, symbol)| (format!("the cursor's {mark} status mark"), symbol));
        let mode_marks = self
            .mode_marks
            .iter()
            .map(|(mode, glyph)| (format!("the {} mode mark", snake_case(mode)), glyph.symbol));
        let lines = [
            ("the leash", self.leash),
            ("the aim line", self.aim),
            ("the aim line's end", self.aim_end),
        ]
        .map(|(what, glyph)| (what.to_string(), glyph.symbol));
        let glass = self
            .glass
            .iter()
            .map(|(pane, glyph)| (format!("the glass's {}", snake_case(pane)), glyph.symbol));
        let stars = self
            .stars
            .iter()
            .map(|(depth, glyph)| (format!("the {} stars", snake_case(depth)), glyph.symbol));
        let bands = self.population.bands;
        let graph = [
            ("the population graph's full", self.population.full),
            ("the population graph's half", self.population.half),
            ("the population graph's egg band", bands.egg),
            ("the population graph's young band", bands.young),
            ("the population graph's adult band", bands.adult),
            ("the population graph's old band", bands.old),
            ("the population graph's limit", self.population.limit.symbol),
        ]
        .map(|(what, symbol)| (what.to_string(), symbol));
        let s = self.shell;
        let shell = [
            ("cracked", s.cracked),
            ("left_half", s.left_half),
            ("right_half", s.right_half),
            ("up_left", s.up_left),
            ("up_right", s.up_right),
            ("down_left", s.down_left),
            ("down_right", s.down_right),
        ]
        .map(|(piece, symbol)| (format!("the shell's {piece}"), symbol));
        tiles
            .chain(objects)
            .chain(arrows)
            .chain(marks)
            .chain(mode_marks)
            .chain(lines)
            .chain(glass)
            .chain(stars)
            .chain(graph)
            .chain(shell)
            .collect()
    }
}

/// A semantic tile as a theme file names it, such as `terrain(deep_water)`.
fn tile_name(tile: SemanticTile) -> String {
    match tile {
        SemanticTile::Terrain(terrain) => format!("terrain({})", snake_case(&terrain)),
        SemanticTile::Emote(emote) => format!("emote({})", snake_case(&emote)),
        tile => snake_case(&tile),
    }
}

/// A unit variant's name as theme files spell it: `ShallowWater` is
/// `shallow_water`.
fn snake_case(variant: &impl std::fmt::Debug) -> String {
    let mut name = String::new();
    for (i, c) in format!("{variant:?}").chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                name.push('_');
            }
            name.push(c.to_ascii_lowercase());
        } else {
            name.push(c);
        }
    }
    name
}

/// Names, each in quotes, separated by commas.
fn quoted(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Turns a theme file's entries into glyphs.
fn glyphs<K: Ord>(entries: BTreeMap<K, GlyphEntry>) -> BTreeMap<K, Glyph> {
    entries
        .into_iter()
        .map(|(key, entry)| (key, glyph(entry)))
        .collect()
}

/// The character of `from`, drawn in the look of `look`: its colour, bold
/// and reverse video (design §6.2, #307).
fn drawn(from: Glyph, look: Glyph) -> Glyph {
    Glyph {
        symbol: from.symbol,
        ..look
    }
}

/// Each of `looks`, drawn with the character `from` gives its key.
fn redrawn<K: Ord + Copy>(
    looks: &BTreeMap<K, Glyph>,
    from: impl Fn(K) -> Glyph,
) -> BTreeMap<K, Glyph> {
    looks
        .iter()
        .map(|(&key, &look)| (key, drawn(from(key), look)))
        .collect()
}

/// Turns a theme file's entry into a glyph.
fn glyph(entry: GlyphEntry) -> Glyph {
    Glyph {
        symbol: entry.glyph,
        fg: entry.fg.into(),
        bold: entry.bold,
        reversed: entry.reversed,
    }
}

/// A theme file, as written in `themes/*.ron`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    tiles: BTreeMap<SemanticTile, GlyphEntry>,
    /// By drive name, from the data pack's `chemicals.ron`, the colour of a
    /// sprite whose strongest drive it is (design §6.3).
    drives: BTreeMap<String, Colour>,
    /// By object type name (from the data pack's `objects.ron`), then visual state.
    objects: BTreeMap<String, BTreeMap<String, GlyphEntry>>,
    cursor: CursorFile,
    /// The Attention marker: the background of the tile the selected sprite
    /// attends to.
    attention_marker: Colour,
    /// The glass and the stars past the wall (M2 design §11.5). A theme
    /// without them, or without some of them, such as one made before there
    /// was space, draws the default theme's.
    #[serde(default, deserialize_with = "implicit_some")]
    space: Option<SpaceFile>,
    /// The population graph (M2 design §14). A theme without it, or
    /// without some of it, draws the default theme's.
    #[serde(default, deserialize_with = "implicit_some")]
    population: Option<PopulationFile>,
    /// The lineage colours (M2 design §17.4). A theme without them, or
    /// without some of them, draws the default theme's.
    #[serde(default, deserialize_with = "implicit_some")]
    lineage: Option<LineageFile>,
    /// The screen's text, by its text role (design §6.2, #302). A theme without
    /// it, or without some of it, styles it as the default theme does.
    #[serde(default, deserialize_with = "implicit_some")]
    text: Option<TextFile>,
    /// The panels' shades (design §6.1, #302). A theme without them draws
    /// lines between the panels, rather than the default theme's shades.
    #[serde(default, deserialize_with = "implicit_some")]
    panels: Option<PanelsFile>,
    /// Motion's colours, and how a drive's bar shows its drive's pace
    /// (design §6.9). A theme without them, or without some of them, draws
    /// the default theme's.
    #[serde(default, deserialize_with = "implicit_some")]
    motion: Option<MotionFile>,
    /// A hatching's shell (design §6.9). A theme without it, or without
    /// some of it, draws the default theme's.
    #[serde(default, deserialize_with = "implicit_some")]
    shell: Option<ShellFile>,
    /// `false` for a palette with no colour at all, as mono is (design
    /// §6.2, #307): every colour is drawn as the terminal's own, keeping
    /// bold and reverse video. A theme without it has colour.
    #[serde(default = "has_colour")]
    colour: bool,
    /// The colour of what the game leaves in the terminal's own text
    /// colour, as Game Kid gives it (design §6.2). A theme without it
    /// leaves that to the terminal.
    #[serde(default, deserialize_with = "implicit_some")]
    foreground: Option<Colour>,
    /// `false` for a palette whose colours can't tell the drives or the
    /// lineage apart, as Game Kid's greens can't (design §6.2, §6.3). A
    /// theme without it can, if it has colour.
    #[serde(default = "shows_colour_modes")]
    colour_modes: bool,
}

/// A theme file has colour unless it says otherwise.
fn has_colour() -> bool {
    true
}

/// And its colours can show the colour modes.
fn shows_colour_modes() -> bool {
    true
}

/// A section a theme file may leave out, written without `Some(…)` when
/// it's there.
fn implicit_some<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    section: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(section).map(Some)
}

/// A theme file's `space` section: what's drawn past the wall (M2 design
/// §11).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpaceFile {
    /// The glass round the map, by piece.
    #[serde(default)]
    glass: BTreeMap<Pane, GlyphEntry>,
    /// The stars past it, by depth.
    #[serde(default)]
    stars: BTreeMap<Depth, GlyphEntry>,
}

/// A theme file's `population` section: how the population graph is
/// drawn (M2 design §14).
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PopulationFile {
    /// Each life stage's colour.
    #[serde(default)]
    stages: StagesFile,
    /// A whole character of one stage.
    #[serde(default, deserialize_with = "implicit_some")]
    full: Option<char>,
    /// The lower half of a character.
    #[serde(default, deserialize_with = "implicit_some")]
    half: Option<char>,
    /// Each stage's own character, for a palette with no colour.
    #[serde(default)]
    bands: BandsFile,
    /// A dot of the line at the world's limit.
    #[serde(default, deserialize_with = "implicit_some")]
    limit: Option<GlyphEntry>,
}

impl PopulationFile {
    /// The graph as this section draws it, with `default`'s part for any it
    /// leaves out: `None` if both leave a part out.
    fn look(self, default: PopulationFile) -> Option<PopulationLook> {
        let (theirs, ours) = (self.stages, default.stages);
        let (their_bands, our_bands) = (self.bands, default.bands);
        Some(PopulationLook {
            young: theirs.young.or(ours.young)?.into(),
            adult: theirs.adult.or(ours.adult)?.into(),
            old: theirs.old.or(ours.old)?.into(),
            egg: theirs.egg.or(ours.egg)?.into(),
            full: self.full.or(default.full)?,
            half: self.half.or(default.half)?,
            bands: PopulationBands {
                young: their_bands.young.or(our_bands.young)?,
                adult: their_bands.adult.or(our_bands.adult)?,
                old: their_bands.old.or(our_bands.old)?,
                egg: their_bands.egg.or(our_bands.egg)?,
            },
            limit: glyph(self.limit.or(default.limit)?),
        })
    }
}

/// Each life stage's own character on the population graph, for a palette
/// with no colour.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct BandsFile {
    #[serde(default, deserialize_with = "implicit_some")]
    young: Option<char>,
    #[serde(default, deserialize_with = "implicit_some")]
    adult: Option<char>,
    #[serde(default, deserialize_with = "implicit_some")]
    old: Option<char>,
    #[serde(default, deserialize_with = "implicit_some")]
    egg: Option<char>,
}

/// The colour of each life stage on the population graph.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct StagesFile {
    #[serde(default, deserialize_with = "implicit_some")]
    young: Option<Colour>,
    #[serde(default, deserialize_with = "implicit_some")]
    adult: Option<Colour>,
    #[serde(default, deserialize_with = "implicit_some")]
    old: Option<Colour>,
    #[serde(default, deserialize_with = "implicit_some")]
    egg: Option<Colour>,
}

/// A theme file's `lineage` section: the lineage colours (M2 design §17.4).
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LineageFile {
    #[serde(default, deserialize_with = "implicit_some")]
    elders: Option<Colour>,
    #[serde(default, deserialize_with = "implicit_some")]
    siblings: Option<Colour>,
    #[serde(default, deserialize_with = "implicit_some")]
    young: Option<Colour>,
    #[serde(default, deserialize_with = "implicit_some")]
    others: Option<Colour>,
}

impl LineageFile {
    /// The colours this section gives, with `default`'s for any it leaves
    /// out: `None` if both leave a colour out.
    fn look(self, default: LineageFile) -> Option<LineageLook> {
        Some(LineageLook {
            elders: self.elders.or(default.elders)?.into(),
            siblings: self.siblings.or(default.siblings)?.into(),
            young: self.young.or(default.young)?.into(),
            others: self.others.or(default.others)?.into(),
        })
    }
}

/// A theme file's `text` section: the screen's text, by its text role (design
/// §6.2, #302).
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct TextFile {
    #[serde(default, deserialize_with = "implicit_some")]
    heading: Option<StyleEntry>,
    #[serde(default, deserialize_with = "implicit_some")]
    muted: Option<StyleEntry>,
    #[serde(default, deserialize_with = "implicit_some")]
    good: Option<StyleEntry>,
    #[serde(default, deserialize_with = "implicit_some")]
    bad: Option<StyleEntry>,
    #[serde(default, deserialize_with = "implicit_some")]
    key: Option<StyleEntry>,
    #[serde(default, deserialize_with = "implicit_some")]
    selected: Option<StyleEntry>,
    #[serde(default, deserialize_with = "implicit_some")]
    spot: Option<StyleEntry>,
}

impl TextFile {
    /// The styles this section gives, with `default`'s for any it leaves
    /// out: `None` if both leave one out.
    fn look(self, default: TextFile) -> Option<TextLook> {
        let style = |theirs: Option<StyleEntry>, ours: Option<StyleEntry>| {
            theirs.or(ours).map(StyleEntry::style)
        };
        Some(TextLook {
            heading: style(self.heading, default.heading)?,
            muted: style(self.muted, default.muted)?,
            good: style(self.good, default.good)?,
            bad: style(self.bad, default.bad)?,
            key: style(self.key, default.key)?,
            selected: style(self.selected, default.selected)?,
            spot: style(self.spot, default.spot)?,
        })
    }
}

/// How a theme file styles text: a colour, bold and reverse video, any of
/// them left out.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StyleEntry {
    #[serde(default, deserialize_with = "implicit_some")]
    fg: Option<Colour>,
    #[serde(default)]
    bold: bool,
    #[serde(default)]
    reversed: bool,
}

impl StyleEntry {
    fn style(self) -> Style {
        let mut style = Style::default();
        if let Some(fg) = self.fg {
            style = style.fg(fg.into());
        }
        if self.bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        if self.reversed {
            style = style.add_modifier(Modifier::REVERSED);
        }
        style
    }
}

/// A theme file's `panels` section: the shades the inspector and the event
/// log sit on (design §6.1, #302).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PanelsFile {
    /// What the panels sit on.
    shade: Colour,
    /// What their title rows sit on.
    title: Colour,
}

/// A theme file's `motion` section: motion's colours, and how a drive's
/// bar shows how fast its drive moves (design §6.9).
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct MotionFile {
    #[serde(default, deserialize_with = "implicit_some")]
    flash: Option<Colour>,
    #[serde(default, deserialize_with = "implicit_some")]
    ash: Option<Colour>,
    #[serde(default, deserialize_with = "implicit_some")]
    spark: Option<Colour>,
    #[serde(default, deserialize_with = "implicit_some")]
    trail: Option<Colour>,
    #[serde(default, deserialize_with = "implicit_some")]
    dark: Option<Colour>,
    #[serde(default, deserialize_with = "implicit_some")]
    bar: Option<BarWaves>,
}

impl MotionFile {
    /// What this section gives, with `default`'s for any it leaves out:
    /// `None` if both leave one out.
    fn look(self, default: MotionFile) -> Option<MotionLook> {
        Some(MotionLook {
            flash: self.flash.or(default.flash)?.into(),
            ash: self.ash.or(default.ash)?.into(),
            spark: self.spark.or(default.spark)?.into(),
            trail: self.trail.or(default.trail)?.into(),
            dark: self.dark.or(default.dark)?.into(),
            bar: self.bar.or(default.bar)?,
        })
    }
}

/// A theme file's `shell` section: a hatching's shell (design §6.9).
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShellFile {
    #[serde(default, deserialize_with = "implicit_some")]
    cracked: Option<char>,
    #[serde(default, deserialize_with = "implicit_some")]
    left_half: Option<char>,
    #[serde(default, deserialize_with = "implicit_some")]
    right_half: Option<char>,
    #[serde(default, deserialize_with = "implicit_some")]
    up_left: Option<char>,
    #[serde(default, deserialize_with = "implicit_some")]
    up_right: Option<char>,
    #[serde(default, deserialize_with = "implicit_some")]
    down_left: Option<char>,
    #[serde(default, deserialize_with = "implicit_some")]
    down_right: Option<char>,
}

impl ShellFile {
    /// The pieces this section gives, with `default`'s for any it leaves
    /// out: `None` if both leave one out.
    fn look(self, default: ShellFile) -> Option<ShellLook> {
        Some(ShellLook {
            cracked: self.cracked.or(default.cracked)?,
            left_half: self.left_half.or(default.left_half)?,
            right_half: self.right_half.or(default.right_half)?,
            up_left: self.up_left.or(default.up_left)?,
            up_right: self.up_right.or(default.up_right)?,
            down_left: self.down_left.or(default.down_left)?,
            down_right: self.down_right.or(default.down_right)?,
        })
    }
}

/// A theme file's `cursor` section.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CursorFile {
    arrows: Arrows,
    /// The arrows while the Cursor follows a sprite.
    followed_arrows: Arrows,
    /// The sides while sprites can see the Cursor, named as the arrows they
    /// replace.
    visible_frame: Arrows,
    status_marks: StatusMarks,
    mode_marks: BTreeMap<CursorMode, GlyphEntry>,
    /// A dot of the leash, from the Cursor to a sprite it leads.
    leash: GlyphEntry,
    /// The aim line, while the player aims a throw or a shove.
    aim: AimFile,
}

/// A theme file's aim line (design v25 §6.5).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AimFile {
    /// A dot of the path.
    path: GlyphEntry,
    /// Where it would stop.
    end: GlyphEntry,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GlyphEntry {
    glyph: char,
    fg: Colour,
    #[serde(default)]
    bold: bool,
    #[serde(default)]
    reversed: bool,
}

/// A colour as a theme file writes it: one of the 16 terminal colours by
/// name, which follow the player's terminal, or an exact one, `"#rrggbb"`
/// (design §6.2, #302).
#[derive(Clone, Copy)]
enum Colour {
    Named(Named),
    Exact(u8, u8, u8),
}

impl<'de> Deserialize<'de> for Colour {
    fn deserialize<D: serde::Deserializer<'de>>(colour: D) -> Result<Colour, D::Error> {
        // A name RON reads as one of the 16, and anything else as text, or
        // as whatever it is, a number or a tuple, so what isn't a colour is
        // refused below, named, saying what's allowed.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Written {
            Named(Named),
            Text(String),
            Other(ron::Value),
        }
        let refused = |written: String| {
            serde::de::Error::custom(format!(
                "{written} isn't a colour: name one of the 16, such as light_cyan, \
                 or write an exact one as \"#rrggbb\""
            ))
        };
        match Written::deserialize(colour)? {
            Written::Named(named) => Ok(Colour::Named(named)),
            Written::Text(text) => exact(&text).ok_or_else(|| refused(format!("\"{text}\""))),
            Written::Other(value) => Err(refused(ron::to_string(&value).unwrap_or_default())),
        }
    }
}

/// An exact colour written `"#rrggbb"`, in hexadecimal.
fn exact(text: &str) -> Option<Colour> {
    let hex = text
        .strip_prefix('#')
        .filter(|hex| hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()))?;
    let byte = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok();
    Some(Colour::Exact(byte(0)?, byte(2)?, byte(4)?))
}

/// The 16 terminal colours, as named in theme files.
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Named {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
}

impl From<Colour> for Color {
    fn from(colour: Colour) -> Color {
        let named = match colour {
            Colour::Exact(r, g, b) => return Color::Rgb(r, g, b),
            Colour::Named(named) => named,
        };
        match named {
            Named::Black => Color::Black,
            Named::Red => Color::Red,
            Named::Green => Color::Green,
            Named::Yellow => Color::Yellow,
            Named::Blue => Color::Blue,
            Named::Magenta => Color::Magenta,
            Named::Cyan => Color::Cyan,
            Named::Gray => Color::Gray,
            Named::DarkGray => Color::DarkGray,
            Named::LightRed => Color::LightRed,
            Named::LightGreen => Color::LightGreen,
            Named::LightYellow => Color::LightYellow,
            Named::LightBlue => Color::LightBlue,
            Named::LightMagenta => Color::LightMagenta,
            Named::LightCyan => Color::LightCyan,
            Named::White => Color::White,
        }
    }
}

/// Whether the terminal shows a theme's exact colours (design §6.2): one
/// that says so in `COLORTERM` (`colorterm`), as most do, and every terminal
/// on Windows, whose console has since Windows 10. Elsewhere, such as in
/// macOS's Terminal, which draws them wrongly, they're drawn as the nearest
/// of 256 ([`in_256_colours`]).
pub fn shows_exact_colours(colorterm: Option<&str>, windows: bool) -> bool {
    windows || matches!(colorterm, Some("truecolor" | "24bit"))
}

/// Each exact colour drawn in `buffer` as the nearest of the 256 colours
/// nearly every terminal shows, for a terminal that doesn't show exact
/// colours (design §6.2). Named colours are the terminal's own, and stay.
pub fn in_256_colours(buffer: &mut Buffer) {
    let nearest = |colour| match colour {
        Color::Rgb(r, g, b) => Color::Indexed(nearest_256(r, g, b)),
        other => other,
    };
    for cell in &mut buffer.content {
        cell.fg = nearest(cell.fg);
        cell.bg = nearest(cell.bg);
    }
}

/// Every colour in `buffer` drawn as the terminal's own, keeping bold and
/// reverse video, for a palette with no colour at all, such as mono (design
/// §6.2, #307). The screen's drawing calls it last, so no colour can reach
/// the screen, not even one the game draws itself, such as the title
/// screen's greys.
pub fn without_colour(buffer: &mut Buffer) {
    for cell in &mut buffer.content {
        cell.fg = Color::Reset;
        cell.bg = Color::Reset;
        cell.underline_color = Color::Reset;
    }
}

/// Every grey in `buffer` drawn in a shade of `foreground`, for a palette
/// that gives one, such as Game Kid (design §6.2): the terminal's own text
/// colour, Campbell's `#cccccc`, as `foreground` itself; the terminal's
/// black, Campbell's `#0c0c0c`, as it is, and anything darker as that; and
/// a grey between or past them as far along from that black to
/// `foreground`, lighter or darker in its hue. So the game's own greys,
/// such as the title screen's, and the text it leaves in the terminal's own
/// colour, are drawn in the palette. What isn't grey stays, and so does the
/// terminal's own background. The screen's drawing calls it last.
pub fn in_shades_of(buffer: &mut Buffer, foreground: Color) {
    let Some(to) = campbell::rgb(foreground) else {
        return;
    };
    let [black, own] = [campbell::BACKGROUND[0], campbell::FOREGROUND[0]].map(i32::from);
    let shade = |colour: Color, terminal_s: Option<campbell::Rgb>| {
        let Some([r, g, b]) = campbell::rgb(colour).or(terminal_s) else {
            return colour;
        };
        if r != g || g != b {
            return colour;
        }
        let along = (i32::from(r) - black).max(0);
        let [r, g, b] = to.map(|channel| {
            let way = along * (i32::from(channel) - black);
            let steps = (way + (own - black) / 2).div_euclid(own - black);
            (black + steps).clamp(0, 255) as u8
        });
        Color::Rgb(r, g, b)
    };
    for cell in &mut buffer.content {
        cell.fg = shade(cell.fg, Some(campbell::FOREGROUND));
        cell.bg = shade(cell.bg, None);
        cell.underline_color = shade(cell.underline_color, None);
    }
}

/// The levels of each of red, green and blue in the 256 colours' 6×6×6
/// cube, which starts at colour 16.
const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// The nearest of the 256 colours to `(r, g, b)`: from the cube, or from
/// the 24 greys after it, `#080808` to `#eeeeee`, whichever is nearer.
fn nearest_256(r: u8, g: u8, b: u8) -> u8 {
    let distance = |(x, y, z): (u8, u8, u8)| {
        [(r, x), (g, y), (b, z)]
            .iter()
            .map(|&(wanted, got)| (i32::from(wanted) - i32::from(got)).pow(2))
            .sum::<i32>()
    };
    let step = |value: u8| {
        (0..CUBE.len())
            .min_by_key(|&i| (i32::from(CUBE[i]) - i32::from(value)).abs())
            .unwrap_or(0)
    };
    let (i, j, k) = (step(r), step(g), step(b));
    let cube = (CUBE[i], CUBE[j], CUBE[k]);
    let mean = (u32::from(r) + u32::from(g) + u32::from(b)) / 3;
    let grey_step = ((mean.saturating_sub(8) + 5) / 10).min(23) as u8;
    let grey = 8 + 10 * grey_step;
    if distance((grey, grey, grey)) < distance(cube) {
        232 + grey_step
    } else {
        16 + 36 * i as u8 + 6 * j as u8 + k as u8
    }
}
