//! Draws the screen. Rendering is a pure function of the app state (design §6.8).

use std::collections::BTreeMap;

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Position, Rect, Size},
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use terra_sim::{EntityId, Grip, LifeStage, Mode, Pos, Progress, SpriteView, World};

use crate::app::{
    App, Areas, ColourMode, ItemLook, PlaceItem, Screen, Selection, ShownItems, in_the_world,
    item_look, spoken_duration, strongest_drive,
};
use crate::controls::{self, Context, Control, Hint, PaletteRow, Says};
use crate::family_tree::{self, Lineage};
use crate::inspector::{self, INSPECTOR_WIDTH, NOTHING_SELECTED, TabColumns, first_shown};
use crate::policy::{Panel, Subject};
use crate::space::{self, Place};
use crate::spots::{Spot, Spots};
use crate::text::{display_name, terrain_name, thousands};
use crate::theme::{Glyph, PanelLook, SemanticTile, TextLook};
use crate::tooltip;

/// The narrowest terminal that has room for the inspector beside the map
/// view. On a narrower one it pops over the map view's right side, while
/// it's open (design §6.1, #305).
const MIN_WIDTH_FOR_INSPECTOR: u16 = 100;
/// The fewest rows the inspector is drawn in, its title row included: on a
/// screen with less room for it, the map view is drawn alone with the top
/// bar and the status line (design §6.1, #305).
const MIN_INSPECTOR_ROWS: u16 = 8;
/// The event log's height, in rows, its title row included: three events
/// (design §6.1).
const EVENT_LOG_HEIGHT: u16 = 4;
/// The shortest terminal that has room for the event log under the map view.
const MIN_HEIGHT_FOR_EVENT_LOG: u16 = 30;

/// Which of the screen's parts are drawn, besides the map view: all of them
/// in the game, and as few as a picture of it wants, such as one of the
/// renderer's clips. The map view takes the room a part leaves, with the
/// inspector beside it while that's drawn (M2 design §11.4, #291).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parts {
    pub top_bar: bool,
    pub inspector: bool,
    pub event_log: bool,
    pub status_line: bool,
}

impl Parts {
    /// The whole screen, as the game draws it.
    pub const ALL: Parts = Parts {
        top_bar: true,
        inspector: true,
        event_log: true,
        status_line: true,
    };

    /// The rows the top bar takes: one, or none.
    fn top_rows(self) -> u16 {
        u16::from(self.top_bar)
    }

    /// The rows the status line takes: one, or none.
    fn bottom_rows(self) -> u16 {
        u16::from(self.status_line)
    }
}

/// Draws one frame: the top bar, the map view, the inspector and the event
/// log if there's room, and the status line.
pub fn render(frame: &mut Frame, app: &App, world: &World) {
    render_with(frame, app, world, Parts::ALL);
}

/// Draws one frame with only the screen's `parts`, and the map view in the
/// room the rest leave (M2 design §11.4). The app is to be fitted to `areas_with` the
/// same parts, so the Cursor and the marks land where it says.
pub fn render_with(frame: &mut Frame, app: &App, world: &World, parts: Parts) {
    let area = frame.area();
    if too_small(area.as_size()) {
        app.keep_spots(Spots::default());
        return render_too_small(frame.buffer_mut(), area, app, world);
    }
    // What each part is, where it's drawn, noted as it's drawn (design §6.1).
    let mut spots = Spots::default();
    let [top_bar, _, status] = Layout::vertical([
        Constraint::Length(parts.top_rows()),
        Constraint::Min(0),
        Constraint::Length(parts.bottom_rows()),
    ])
    .areas(area);
    if parts.top_bar {
        frame.render_widget(top_bar_line(app, world, top_bar.width), top_bar);
    }
    draw_tiles(frame.buffer_mut(), map_view_area(area, parts), app, world);
    // Beside the map view, or over its right side while it's open there
    // (design §6.1, #305), as the app has it, so it's drawn where clicks
    // find it.
    let inspector = inspector_area(area, parts).filter(|_| !app.inspector_shut());
    if let Some(inspector) = inspector {
        render_inspector(frame.buffer_mut(), inspector, app, world, &mut spots);
    }
    let event_log = event_log_area(area, parts);
    if let Some(event_log) = event_log {
        render_event_log(frame.buffer_mut(), event_log, app, world, &mut spots);
    }
    // Where the inspector's line meets the event log's, in a theme without
    // shades (design §6.1).
    if let (Some(inspector), Some(event_log), None) = (inspector, event_log, app.theme.panels()) {
        frame.buffer_mut()[(inspector.x, event_log.y)].set_symbol("┴");
    }
    // The inspector, filling the screen (design §6.1, #305), under
    // whatever's opened from it. It hides all that's beneath it, so only
    // its own spots take the pointer there.
    let overlay = app.overlay();
    if let Some(full_screen) = overlay.filter(|_| app.full_screen()) {
        spots = Spots::default();
        render_inspector(frame.buffer_mut(), full_screen, app, world, &mut spots);
    }
    // A pop-up takes the pointer before what's under it (design §6.1).
    let mut pop_ups = vec![render_menu(frame.buffer_mut(), area, app, world)];
    match app.screen() {
        Screen::Help => {
            if let Some(area) = overlay {
                draw_help(frame.buffer_mut(), area, app, world);
                pop_ups.push(overlay);
            }
        }
        Screen::Palette => pop_ups.push(render_palette(
            frame.buffer_mut(),
            map_view_area(area, parts),
            app,
        )),
        Screen::SpriteList => {
            render_sprite_list(frame.buffer_mut(), app, world);
            pop_ups.push(overlay);
        }
        Screen::Population => {
            render_population(frame.buffer_mut(), app, world);
            pop_ups.push(overlay);
        }
        Screen::FamilyTree => {
            render_family_tree(frame.buffer_mut(), app, world);
            pop_ups.push(overlay);
        }
        _ => {}
    }
    // Over whatever's open, so it's in a screenshot of any of them (design
    // §6.1).
    if app.debug_overlay() {
        pop_ups.push(render_debug_overlay(
            frame.buffer_mut(),
            map_view_area(area, parts),
            app,
        ));
    }
    for pop_up in pop_ups.into_iter().flatten() {
        spots.note(pop_up, Spot::PopUp);
    }
    // What can be pointed at, what is, and a tooltip saying what it is,
    // between the bars (design §6.1, #304).
    tooltip::mark_spots(frame.buffer_mut(), app, &spots);
    tooltip::light_up_spot(frame.buffer_mut(), app, &spots);
    if let Some(Spot::LogLine(id)) = tooltip::pointer(app).and_then(|cell| spots.at(cell)) {
        light_up(frame.buffer_mut(), app, world, *id, &spots);
    }
    let between_bars = Rect::new(
        area.x,
        area.y + parts.top_rows(),
        area.width,
        area.height
            .saturating_sub(parts.top_rows() + parts.bottom_rows()),
    );
    tooltip::draw(frame.buffer_mut(), between_bars, app, world, &spots);
    if parts.status_line {
        frame.render_widget(status_line(app, world, status.width), status);
    }
    app.keep_spots(spots);
}

/// What the status line asks when `Esc` would leave a world (M2 design
/// §8.4), each answer's key bold and what it does muted (design §6.2).
fn quit_prompt(app: &App, world: &World) -> Line<'static> {
    let answers = all_hints(controls::hints(Context::Quitting), app, world);
    let mut spans = vec![Span::raw("Quit? ")];
    spans.extend(hint_spans(&answers, app.theme.text()));
    Line::from(spans)
}

/// `question`, which ends `(y/n)`, as the status line asks it, with the two
/// keys bold (design §6.2).
fn yes_or_no(question: &str, look: TextLook) -> Line<'static> {
    let Some(asked) = question.strip_suffix("(y/n)") else {
        return Line::from(format!(" {question}"));
    };
    Line::from(vec![
        Span::raw(format!(" {asked}(")),
        Span::styled("y", look.key),
        Span::raw("/"),
        Span::styled("n", look.key),
        Span::raw(")"),
    ])
}

/// The smallest screen the game draws on: what the map alone needs, with
/// the top bar and the status line round a few rows of it (design §6.1,
/// #305).
pub const MIN_SIZE: Size = Size::new(20, 5);

/// Whether a screen of `screen` cells is too small for the game.
pub(crate) fn too_small(screen: Size) -> bool {
    smaller_than(screen, MIN_SIZE)
}

/// Whether a screen of `screen` cells is too small for what `needs` a
/// screen that size: the game, or the title screen (M2 design §8.2).
pub(crate) fn smaller_than(screen: Size, needs: Size) -> bool {
    screen.width < needs.width || screen.height < needs.height
}

/// Says the terminal is too small, and how big it needs to be, in the
/// middle of the screen (design §6.1). The game carries on beneath.
fn render_too_small(buf: &mut Buffer, area: Rect, app: &App, world: &World) {
    let mut questions = vec![];
    // `Esc` still asks to quit, so the question shows here too.
    if app.screen() == Screen::QuitPrompt {
        questions.push(quit_prompt(app, world).to_string());
    }
    if let Some(question) = app.load_question() {
        questions.push(question);
    }
    render_too_small_with(buf, area, MIN_SIZE, questions);
}

/// Says the terminal is too small, and that it `needs` more, as the game
/// does, with any question waiting under it: the title screen's too (M2
/// design §8.2).
pub(crate) fn render_too_small_with(
    buf: &mut Buffer,
    area: Rect,
    needs: Size,
    questions: Vec<String>,
) {
    let needs = format!("needs {}x{},", needs.width, needs.height);
    let this = format!("this is {}x{}", area.width, area.height);
    let mut lines = vec!["Terminal too small".to_string()];
    // On two lines if it's too narrow for one (design §6.1, #305).
    if needs.len() + this.len() < usize::from(area.width) {
        lines.push(format!("{needs} {this}"));
    } else {
        lines.extend([needs, this]);
    }
    lines.extend(questions);
    let top = area.y + area.height.saturating_sub(lines.len() as u16) / 2;
    for (row, line) in (top..area.bottom()).zip(lines) {
        let width = line.chars().count() as u16;
        let x = area.x + area.width.saturating_sub(width) / 2;
        buf.set_stringn(x, row, line, usize::from(area.width), Style::default());
    }
}

/// Where the app's panels are drawn on a screen of `screen` cells. The map
/// view draws the map between the top bar and the status line, out to the
/// screen's edges, with space past its wall (M2 design §11).
pub fn areas(screen: Size) -> Areas {
    areas_with(screen, Parts::ALL)
}

/// Where the app's panels are drawn on a screen of `screen` cells that
/// draws only `parts` (`render_with`, M2 design §11.4).
pub fn areas_with(screen: Size, parts: Parts) -> Areas {
    // A screen too small for the game draws no panels, so nothing can be
    // clicked on it.
    if too_small(screen) {
        return Areas {
            tiles: Rect::default(),
            inspector: None,
            pops_over: false,
            event_log: None,
            overlay: None,
        };
    }
    Areas {
        tiles: map_view_area(screen.into(), parts),
        inspector: inspector_area(screen.into(), parts),
        pops_over: pops_over(screen.into()),
        event_log: event_log_area(screen.into(), parts),
        overlay: Some(overlay_area(screen.into(), parts)),
    }
}

/// Whether the inspector pops over the map view's right side on a screen
/// `screen`, shown only while it's open, rather than beside it (design
/// §6.1, #305).
fn pops_over(screen: Rect) -> bool {
    screen.width < MIN_WIDTH_FOR_INSPECTOR
}

/// The map view: below the top bar if it's drawn, at the left, leaving room
/// for the inspector beside it and the event log when they're drawn and
/// there is some, and out to the screen's edges, with no frame (design
/// §6.1). An inspector that pops over it leaves it the whole width (#305).
/// A small map is drawn in the middle of it, with space round it (M2 design
/// §11.1).
fn map_view_area(screen: Rect, parts: Parts) -> Rect {
    let width = match inspector_area(screen, parts) {
        Some(inspector) if !pops_over(screen) => inspector.x - screen.x,
        _ => screen.width,
    };
    let height = panels_height(screen, parts);
    Rect::new(
        screen.x,
        screen.y + parts.top_rows().min(screen.height),
        width,
        height,
    )
}

/// The rows for the map view and the inspector: between the top bar and the
/// event log, or the status line if the log isn't drawn, each part where it
/// is drawn and the screen's edge where it isn't.
fn panels_height(screen: Rect, parts: Parts) -> u16 {
    let log = if event_log_area(screen, parts).is_some() {
        EVENT_LOG_HEIGHT
    } else {
        0
    };
    screen
        .height
        .saturating_sub(parts.top_rows() + parts.bottom_rows() + log)
}

/// The inspector, its title row and the column down its left included, if
/// it's drawn: at the right, below the top bar if that's drawn, down to the
/// event log or the status line, on a terminal with room for it. Beside the
/// map view, or on one under 100 columns over its right side (design §6.1,
/// #305).
fn inspector_area(screen: Rect, parts: Parts) -> Option<Rect> {
    let rows = panels_height(screen, parts);
    let room = screen.width >= INSPECTOR_WIDTH && rows >= MIN_INSPECTOR_ROWS;
    (parts.inspector && room).then(|| {
        Rect::new(
            screen.right() - INSPECTOR_WIDTH,
            screen.y + parts.top_rows(),
            INSPECTOR_WIDTH,
            rows,
        )
    })
}

/// Where the help screen and the sprite list are drawn: the full width,
/// between the top bar and the status line, or the screen's edges where
/// they aren't drawn.
fn overlay_area(screen: Rect, parts: Parts) -> Rect {
    Rect::new(
        screen.x,
        screen.y + parts.top_rows(),
        screen.width,
        screen
            .height
            .saturating_sub(parts.top_rows() + parts.bottom_rows()),
    )
}

/// The event log, its title row included, if it's drawn: the full width,
/// just above the status line or the screen's bottom edge, on a terminal
/// tall enough for it.
fn event_log_area(screen: Rect, parts: Parts) -> Option<Rect> {
    (parts.event_log && screen.height >= MIN_HEIGHT_FOR_EVENT_LOG).then(|| {
        Rect::new(
            screen.x,
            screen.bottom() - parts.bottom_rows() - EVENT_LOG_HEIGHT,
            screen.width,
            EVENT_LOG_HEIGHT,
        )
    })
}

/// Where the app's panels are drawn when the map view is drawn alone
/// (`render_map_view_alone`), on a screen of `screen` cells: its tiles fill
/// the screen, and there are no other panels.
pub fn map_view_alone_areas(screen: Size) -> Areas {
    Areas {
        tiles: Rect::from((Position::ORIGIN, screen)),
        inspector: None,
        pops_over: false,
        event_log: None,
        overlay: None,
    }
}

/// Draws the map view alone over the whole frame, its tiles and what's past
/// its wall with no border, top bar, panels or status line: for pictures of
/// the map, such as the renderer's clips (#241). The app is to be fitted to
/// `map_view_alone_areas`, so the Cursor and the marks land where it says.
pub fn render_map_view_alone(frame: &mut Frame, app: &App, world: &World) {
    let area = frame.area();
    draw_tiles(frame.buffer_mut(), area, app, world);
}

/// Draws what the map view shows in `inner`: the tiles in view, the glass
/// and space past the wall, and the marks over them. It has no frame: the
/// glass shows where the wall is (M2 design §11.2).
fn draw_tiles(buf: &mut Buffer, inner: Rect, app: &App, world: &World) {
    let map = world.map();
    let destination = heading_for(app, world).filter(|_| app.flash_on());
    let attended = attended_by(app, world);
    // A held item being aimed, where it will be thrown from (design v25
    // §6.5).
    let thrown = app
        .thrown_from(world)
        .and_then(|(item, from)| Some((from, item_look(world, item)?)));
    // With a queued pick-up or put-down done (design §6.5, #203).
    let items = app.shown_items(world);
    // Out of reach in space, the Cursor isn't drawn (M2 design §11.4).
    let cursor = app.shown_cursor();
    let lineage = lineage_on_map(app, world);
    let width = app.tile_width();
    for y in inner.top()..inner.bottom() {
        for x in inner.left()..inner.right() {
            // Which of its tile's cells this is: the second while tiles are
            // two cells wide (M2 design §11.7).
            let second = (x - inner.x) % width == 1;
            let pos = match app.place_from(inner.as_position(), Position::new(x, y)) {
                Place::Tile(pos) => pos,
                Place::Glass(pane) => {
                    let glass = app.theme.glass(pane);
                    let column = app.column_from(inner.as_position(), Position::new(x, y));
                    // Two cells wide, the glass down the sides keeps to the
                    // map's side of its tile, so it meets the map.
                    let outer = if column < 0 { !second } else { second };
                    let side = width > 1 && outer && !(0..i32::from(map.width())).contains(&column);
                    let symbol = if side { ' ' } else { glass.symbol };
                    buf[(x, y)]
                        .set_char(symbol)
                        .set_style(Style::default().fg(glass.fg));
                    continue;
                }
                Place::Space => {
                    buf[(x, y)].set_char(' ').set_style(Style::default());
                    continue;
                }
            };
            // A sprite is drawn over any item on its tile.
            let glyph = if let Some(sprite) = world.sprite_at(pos) {
                let emote = app
                    .emote(sprite.id())
                    .filter(|_| app.can_view(Panel::Emotes, Subject::Sprite(sprite.id())));
                let selected = app.selection() == Some(Selection::Living(sprite.id()));
                let tile = match emote {
                    Some(emote) => SemanticTile::Emote(emote),
                    None => sprite_tile(sprite.life_stage(), selected),
                };
                app.theme.glyph(tile)
            } else if let Some((_, look)) = thrown.filter(|&(from, _)| from == pos) {
                app.theme.object_glyph(look.type_name, look.visual_state)
            } else if destination == Some(pos) {
                app.theme.glyph(SemanticTile::DecisionMarker)
            } else if let Some(item) = items.at(pos) {
                let glyph = app.theme.object_glyph(item.type_name, item.visual_state);
                // Only an egg can be selected, and it's drawn in reverse
                // video (M2 design §10.7).
                let selected = app.selection() == Some(Selection::Living(item.id));
                Glyph {
                    reversed: glyph.reversed || selected,
                    ..glyph
                }
            } else {
                app.theme.glyph(SemanticTile::Terrain(map.terrain(pos)))
            };
            // Terrain repeats in a tile's second cell; a sprite, an object
            // or a mark leaves it blank, its style kept. What else goes
            // there, such as what a sprite holds, is M3's to settle (M2
            // design §11.7).
            let on_terrain = world.sprite_at(pos).is_none()
                && thrown.is_none_or(|(from, _)| from != pos)
                && destination != Some(pos)
                && items.at(pos).is_none();
            let symbol = if second && !on_terrain {
                ' '
            } else {
                glyph.symbol
            };
            // A sprite takes the colour mode's colour, unless it's emoting
            // (design §6.3).
            let fg = match world.sprite_at(pos) {
                Some(sprite) if !is_emote(app, &sprite) => {
                    sprite_colour(app, &sprite, glyph.fg, lineage.as_ref())
                }
                _ => glyph.fg,
            };
            let mut style = Style::default().fg(fg);
            if glyph.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if glyph.reversed || cursor == Some(pos) {
                style = style.add_modifier(Modifier::REVERSED);
            }
            if attended == Some(pos) {
                style = style.bg(app.theme.attention_marker());
            }
            buf[(x, y)].set_char(symbol).set_style(style);
        }
    }
    draw_stars(buf, inner, app);
    if app.flash_on() {
        draw_leash(buf, app, world);
    }
    draw_aim_line(buf, app, world);
    draw_cursor(buf, app, world);
    draw_aim_end(buf, app, world);
}

/// Draws the stars on space, far ones first, so a nearer star hides a
/// farther one, and never over the map or its glass (M2 design §11.3).
fn draw_stars(buf: &mut Buffer, inner: Rect, app: &App) {
    // In tiles, so they follow the zoom: each in its tile's first cell.
    let width = app.tile_width();
    let view = Size::new(inner.width.div_ceil(width), inner.height);
    for (depth, at) in space::stars(view, app.viewport(), app.drift()) {
        let cell = Position::new(inner.x + at.x * width, inner.y + at.y);
        if inner.contains(cell) && app.place_from(inner.as_position(), cell) == Place::Space {
            let star = app.theme.star(depth);
            buf[cell]
                .set_char(star.symbol)
                .set_style(Style::default().fg(star.fg));
        }
    }
}

/// Whether `sprite` is drawn as an emote just now.
fn is_emote(app: &App, sprite: &SpriteView) -> bool {
    let id = sprite.id();
    app.emote(id).is_some() && app.can_view(Panel::Emotes, Subject::Sprite(id))
}

/// The selected sprite and those of its lineage the map marks, while it
/// shows the lineage colours and a sprite is selected, living or dead (M2
/// design §17.4).
type Marked = (EntityId, BTreeMap<EntityId, Lineage>);

/// The lineage the map colours, if it does: not while the policy hides
/// the selected sprite from the family tree.
fn lineage_on_map(app: &App, world: &World) -> Option<Marked> {
    if app.colour_mode() != ColourMode::Lineage {
        return None;
    }
    let id = app.selection()?.id();
    if !family_tree::shown(app, id) {
        return None;
    }
    Some((id, family_tree::lineage(world, id)))
}

/// The colour `sprite` is drawn in, in the colour mode (design §6.3): its
/// strongest drive's above .5, or else `own`, its glyph's colour; or in the
/// lineage colours, where it stands in the selected sprite's lineage. The
/// selected sprite keeps `own`, as every sprite does with none selected (M2
/// design §17.4). A drive the theme has no colour for draws `own` too, and so
/// does a sprite whose colour, or place on the family tree, the policy
/// hides (design §6.4).
fn sprite_colour(app: &App, sprite: &SpriteView, own: Color, lineage: Option<&Marked>) -> Color {
    if !app.can_view(Panel::MapColours, Subject::Sprite(sprite.id())) {
        return own;
    }
    match app.colour_mode() {
        ColourMode::Plain => own,
        ColourMode::Drive => strongest_drive(sprite)
            .and_then(|drive| app.theme.drive_colour(drive.name))
            .unwrap_or(own),
        // As the family tree does, the lineage colours ask the policy
        // about each sprite they mark.
        ColourMode::Lineage => match lineage {
            Some((selected, marked))
                if *selected != sprite.id() && family_tree::shown(app, sprite.id()) =>
            {
                app.theme
                    .lineage()
                    .colour(marked.get(&sprite.id()).copied())
            }
            _ => own,
        },
    }
}

/// Where the selected sprite is heading, while its action is under way
/// (design §6.1): where the map flashes the Decision marker.
fn heading_for(app: &App, world: &World) -> Option<Pos> {
    let id = marked(app)?;
    let action = world.sprite(id)?.action()?;
    let under_way = !matches!(action.progress, Progress::Ended(_));
    under_way.then_some(action.destination).flatten()
}

/// The tile of the one thing the selected sprite attends to (design §5.3):
/// where the map shades the Attention marker.
fn attended_by(app: &App, world: &World) -> Option<Pos> {
    world.sprite(marked(app)?)?.attending_to()
}

/// The selected sprite, if it's living and the policy lets the map mark
/// where it's heading and what it attends to (design §6.4).
fn marked(app: &App) -> Option<EntityId> {
    match app.selection() {
        Some(Selection::Living(id)) if app.can_view(Panel::Markers, Subject::Sprite(id)) => {
            Some(id)
        }
        _ => None,
    }
}

/// Draws the 3×3 cursor around its target tile, which the tile loop has already
/// drawn in reverse video. The arrows, solid while following, or while
/// sprites can see the Cursor a frame of light (design v29 §6.5), and the
/// marks take the mode mark's colour. The cursor is drawn only while its target is
/// in view, and not out of reach in space (M2 design §11.4); pieces off the
/// map's tiles in view, over its glass or past the map view's edge, are
/// left off.
///
/// ```text
/// M ↓ Y      M: the mode mark
/// → ☺ ←      Y, N: the status marks
/// N ↑ M
/// ```
///
/// Round a tile two cells wide, its arrows above and below come in twos,
/// one over each cell, as does a frame of light's (M2 design §11.7):
///
/// ```text
/// M ↓↓ Y
/// → ☺  ←
/// N ↑↑ M
/// ```
fn draw_cursor(buf: &mut Buffer, app: &App, world: &World) {
    let Some(centre) = app.shown_cursor().and_then(|cursor| app.cell_of(cursor)) else {
        return;
    };
    let arrows = if app.visible() {
        app.theme.visible_frame()
    } else if app.followed().is_some() {
        app.theme.followed_arrows()
    } else {
        app.theme.arrows()
    };
    let [y, n] = app.status_marks(world).map(|status| {
        let theme = app.theme.status_marks();
        theme
            .glyph(status)
            .unwrap_or_else(|| held_glyph(app, world))
    });
    let mark = app.theme.mode_mark(app.mode());
    // Round its tile, two cells wide while zoomed, with an arrow over and
    // under each (M2 design §11.7).
    let right = app.tile_width() as i16;
    let above_and_below = (0..right).flat_map(|dx| [(dx, -1, arrows.down), (dx, 1, arrows.up)]);
    let pieces = [
        (-1, -1, mark.symbol),
        (right, -1, y),
        (-1, 0, arrows.right),
        (right, 0, arrows.left),
        (-1, 1, n),
        (right, 1, mark.symbol),
    ];
    for (dx, dy, glyph) in pieces.into_iter().chain(above_and_below) {
        let x = centre.x.checked_add_signed(dx);
        let y = centre.y.checked_add_signed(dy);
        if let Some(cell) = x.zip(y).map(|(x, y)| Position::new(x, y))
            && app.tile_at(cell).is_some()
        {
            put(buf, cell, glyph, mark.fg);
        }
    }
}

/// Draws the leash, while the Cursor leads a sprite: a dotted line from the
/// Cursor to the sprite, over empty ground only, so sprites and objects stay
/// visible, and stopping at the Cursor's 3×3 (design v23 §6.5). Like the
/// Cursor, it isn't drawn while the Cursor is out of reach in space (M2
/// design §11.4).
fn draw_leash(buf: &mut Buffer, app: &App, world: &World) {
    if let Some(led) = app.leash().filter(|_| app.shown_cursor().is_some()) {
        let tiles = line_between(led, app.cursor());
        draw_dots(buf, app, world, &tiles, app.theme.leash());
    }
}

/// Draws the aim line, while the player aims a throw or a shove: steady
/// dots along the thing's path, over empty ground only, as the leash is, and
/// clear of the Cursor (design v25 §6.5). Its end is `draw_aim_end`'s.
fn draw_aim_line(buf: &mut Buffer, app: &App, world: &World) {
    let line = app.aim_line(world);
    let path = &line[..line.len().saturating_sub(1)];
    draw_dots(buf, app, world, path, app.theme.aim());
}

/// Draws the aim line's end, where the thing would stop if nothing's in the
/// way, over empty ground: after the Cursor, so a short aim's end shows on
/// its arms rather than under them, though never on its target tile (design
/// v25 §6.5).
fn draw_aim_end(buf: &mut Buffer, app: &App, world: &World) {
    let Some(&end) = app.aim_line(world).last() else {
        return;
    };
    let mark = app.theme.aim_end();
    let clear = empty(world, &app.shown_items(world), end) && end != app.cursor();
    if let Some(cell) = app.cell_of(end).filter(|_| clear) {
        put_on_tile(buf, app, cell, mark.symbol, mark.fg);
    }
}

/// Draws `dot` on each of `tiles` in view that's empty ground and clear of
/// the Cursor's 3×3, as the leash and the aim line are drawn.
fn draw_dots(buf: &mut Buffer, app: &App, world: &World, tiles: &[Pos], dot: Glyph) {
    let (cursor, items) = (app.cursor(), app.shown_items(world));
    for &tile in tiles {
        let by_the_cursor = tile.x.abs_diff(cursor.x) <= 1 && tile.y.abs_diff(cursor.y) <= 1;
        let clear = empty(world, &items, tile) && !by_the_cursor;
        if let Some(cell) = app.cell_of(tile).filter(|_| clear) {
            put_on_tile(buf, app, cell, dot.symbol, dot.fg);
        }
    }
}

/// Draws `symbol` at screen cell `cell`, in `fg`.
fn put(buf: &mut Buffer, cell: Position, symbol: char, fg: Color) {
    buf[(cell.x, cell.y)]
        .set_char(symbol)
        .set_style(Style::default().fg(fg));
}

/// Draws `symbol` on the tile whose first screen cell is `cell`, in `fg`,
/// a mark leaving the rest of a tile two cells wide blank (M2 design
/// §11.7).
fn put_on_tile(buf: &mut Buffer, app: &App, cell: Position, symbol: char, fg: Color) {
    put(buf, cell, symbol, fg);
    for x in 1..app.tile_width() {
        let rest = Position::new(cell.x + x, cell.y);
        if app.tile_at(rest).is_some() {
            put(buf, rest, ' ', fg);
        }
    }
}

/// Whether `tile` holds no sprite and, as the map shows them, no item.
fn empty(world: &World, items: &ShownItems, tile: Pos) -> bool {
    world.sprite_at(tile).is_none() && items.at(tile).is_none()
}

/// The tiles of a straight line from `from` to `to`, leaving both ends out:
/// Bresenham's, one tile a step.
fn line_between(from: Pos, to: Pos) -> Vec<Pos> {
    let (x1, y1) = (i32::from(to.x), i32::from(to.y));
    let (mut x, mut y) = (i32::from(from.x), i32::from(from.y));
    let (dx, dy) = ((x1 - x).abs(), -(y1 - y).abs());
    let (sx, sy) = ((x1 - x).signum(), (y1 - y).signum());
    let mut error = dx + dy;
    let mut tiles = Vec::new();
    while (x, y) != (x1, y1) {
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx;
        }
        if twice <= dx {
            error += dx;
            y += sy;
        }
        if (x, y) != (x1, y1) {
            tiles.push(Pos {
                x: x as u16,
                y: y as u16,
            });
        }
    }
    tiles
}

/// Draws the open menu, if any (design v28 §6.5): over the map, from its top-left
/// tile, its items numbered and the highlighted one in the theme's style for
/// what's chosen (design §6.2), inside a border; as far as the screen goes.
fn render_menu(buf: &mut Buffer, screen: Rect, app: &App, world: &World) -> Option<Rect> {
    let (Some(menu), Some(title)) = (app.menu_area(world), app.menu_title()) else {
        return None;
    };
    let area = menu.intersection(screen);
    if area.width < 3 || area.height < 3 {
        return None;
    }
    blank(buf, area);
    draw_border(buf, area, title);
    let items = app.menu_items(world);
    let room = usize::from(area.width - 2);
    if items.is_empty() {
        let none = format!(" {}", app.menu_empty());
        buf.set_stringn(area.x + 1, area.y + 1, none, room, Style::default());
    }
    let first = app.menu_first(world);
    for (row, (index, item)) in items.iter().enumerate().skip(first).enumerate() {
        let y = area.y + 1 + row as u16;
        if y >= area.bottom() - 1 {
            break;
        }
        // Items past 9 have no number key, but the arrows reach them.
        let number = if index < 9 {
            (index + 1).to_string()
        } else {
            " ".into()
        };
        let line = format!(" {number} {item} ");
        let style = if index == app.menu_choice() {
            app.theme.text().selected
        } else {
            Style::default()
        };
        buf.set_stringn(area.x + 1, y, line, room, style);
    }
    Some(area)
}

/// How far into the help's top edge its title starts, past the corner and
/// a line.
pub(crate) const HELP_TITLE_AT: u16 = 2;

/// Draws the help screen in `area`, over everything between the top bar
/// and the status line (design §6.1): its pages named in its top edge, the
/// open one in the theme's style for what's chosen, and in its bottom edge
/// what's typed into its search, or that typing searches (#303). The
/// title screen's help is drawn so too. While it searches, `Esc` clears
/// the search first, and its corner says so.
pub(crate) fn draw_help(buf: &mut Buffer, area: Rect, app: &App, world: &World) {
    let look = app.theme.text();
    let search = app.help_search();
    let corner = match search {
        Some(_) => controls::CLEAR.corner(),
        None => controls::CLOSE.corner(),
    };
    let inner = clear_box(buf, area, "", corner, look);
    let mut title = vec![Span::raw(crate::help::TITLE)];
    for (page, label) in crate::help::page_names() {
        if search.is_none() && page == app.help_page() {
            title.push(Span::styled(label, look.selected));
        } else {
            title.push(Span::raw(label));
        }
    }
    let room = area.width.saturating_sub(3);
    buf.set_line(area.x + HELP_TITLE_AT, area.y, &Line::from(title), room);
    let bottom = match search {
        Some(typed) => Line::raw(format!(" search: {typed}_ ")),
        None => Line::styled(" type to search ", look.muted),
    };
    let bottom_row = area.bottom().saturating_sub(1);
    buf.set_line(area.x + HELP_TITLE_AT, bottom_row, &bottom, room);
    let lines = crate::help::lines(app, world, usize::from(inner.width));
    // Scrolled no further than its last line at the bottom.
    let first = app
        .help_scroll()
        .min(lines.len().saturating_sub(usize::from(inner.height)));
    draw_lines(buf, inner, &lines[first..]);
}

/// How wide the command palette is, at most (#303).
const PALETTE_WIDTH: u16 = 50;

/// How many controls the palette shows at once (#303).
const PALETTE_ROWS: usize = 10;

/// Draws the command palette near the top of the map view, `map_view`
/// (#303): what's typed, then each key it found with what it does and
/// its heading, the key at the right in the theme's key style, or muted
/// where the palette can't press it, and the highlighted row in its style
/// for what's chosen. Gives the box, if it's drawn.
fn render_palette(buf: &mut Buffer, map_view: Rect, app: &App) -> Option<Rect> {
    let look = app.theme.text();
    let rows = app.palette_rows();
    let width = PALETTE_WIDTH.min(map_view.width);
    let height = rows.len().clamp(1, PALETTE_ROWS) as u16 + 3;
    let x = map_view.x + (map_view.width - width) / 2;
    let area = Rect::new(x, map_view.y + 1, width, height).intersection(map_view);
    if area.width < 12 || area.height < 4 {
        return None;
    }
    let inner = clear_box(buf, area, " Commands ", controls::CLOSE.corner(), look);
    let typed = app.palette_text().unwrap_or_default();
    let prompt = Line::from(vec![
        Span::styled(":", look.key),
        Span::raw(format!("{typed}_")),
    ]);
    buf.set_line(inner.x, inner.y, &prompt, inner.width);
    let room = usize::from(inner.height.saturating_sub(1));
    if rows.is_empty() {
        let none = Line::styled(" Nothing found", look.muted);
        buf.set_line(inner.x, inner.y + 1, &none, inner.width);
        return Some(area);
    }
    // The highlight stays in view.
    let choice = app.palette_choice();
    let first = (choice + 1).saturating_sub(room);
    for (y, (index, row)) in rows.iter().enumerate().skip(first).take(room).enumerate() {
        let PaletteRow { control, key } = row;
        let heading = control
            .headings
            .first()
            .map_or("", |heading| heading.label());
        let about = format!(" · {}", heading.to_lowercase());
        let used =
            1 + key.does.chars().count() + about.chars().count() + key.written.chars().count() + 1;
        let gap = usize::from(inner.width).saturating_sub(used).max(1);
        let key_look = if controls::pressable(key, app) {
            look.key
        } else {
            look.muted
        };
        let mut line = Line::from(vec![
            Span::raw(format!(" {}", key.does)),
            Span::styled(about, look.muted),
            Span::raw(" ".repeat(gap)),
            Span::styled(key.written, key_look),
            Span::raw(" "),
        ]);
        if index == choice {
            line = line.style(look.selected);
        }
        buf.set_line(inner.x, inner.y + 1 + y as u16, &line, inner.width);
    }
    Some(area)
}

/// Draws the sprite list over everything between the top bar and the
/// status line (design §6.1).
fn render_sprite_list(buf: &mut Buffer, app: &App, world: &World) {
    let Some(area) = app.overlay() else {
        return;
    };
    // While it's filtered, `Esc` clears the filter first (#313).
    let corner = match app.list_filter() {
        Some(_) => controls::CLEAR.corner(),
        None => controls::CLOSE.corner(),
    };
    let title = crate::sprite_list::title(app, world);
    let inner = clear_box(buf, area, &title, corner, app.theme.text());
    let lines = crate::sprite_list::lines(
        app,
        world,
        usize::from(inner.width),
        app.list_first(world),
        app.list_rows(),
    );
    draw_lines(buf, inner, &lines);
}

/// Draws the population graph over everything between the top bar and the
/// status line (M2 design §14), or an empty box if the policy hides it.
fn render_population(buf: &mut Buffer, app: &App, world: &World) {
    let Some(area) = app.overlay() else {
        return;
    };
    let shown = app.can_view(Panel::Graph, Subject::World);
    let title = if shown {
        format!(" Population ── {} ", crate::graph::covers(world))
    } else {
        " Population ".to_string()
    };
    let inner = clear_box(
        buf,
        area,
        &title,
        controls::CLOSE.corner(),
        app.theme.text(),
    );
    if !shown {
        return;
    }
    let lines = crate::graph::lines(
        app,
        world,
        usize::from(inner.width),
        usize::from(inner.height),
    );
    draw_lines(buf, inner, &lines);
}

/// Draws the family tree over everything between the top bar and the
/// status line (M2 design §17.2), titled with the sprite in its middle, or
/// says how to select one; or an empty box if the policy hides it.
fn render_family_tree(buf: &mut Buffer, app: &App, world: &World) {
    let Some(area) = app.overlay() else {
        return;
    };
    let label = app
        .selection()
        .map(|selection| family_tree::name_of(app, world, selection.id()))
        .filter(|label| !label.is_empty());
    let title = match label {
        Some(label) => format!(" Family tree ── {label} "),
        None => " Family tree ".to_string(),
    };
    let inner = clear_box(
        buf,
        area,
        &title,
        controls::CLOSE.corner(),
        app.theme.text(),
    );
    if app.selection().is_none() {
        buf.set_string(inner.x, inner.y, NOTHING_SELECTED, Style::default());
    }
    // Nothing more if the policy hides the sprite in the middle.
    if let Some(tree) = app.tree(world) {
        draw_lines(buf, inner, &tree.lines);
    }
}

/// Draws the debug overlay (`F3`, design §6.1, #311) in the map view's
/// top-right corner, `map_view`, as far as the map view goes. Gives the
/// box, if it's drawn.
fn render_debug_overlay(buf: &mut Buffer, map_view: Rect, app: &App) -> Option<Rect> {
    let lines = app.debug_lines();
    let width = crate::debug_overlay::WIDTH;
    let height = lines.len() as u16 + 2;
    let corner = Rect::new(
        map_view.right().saturating_sub(width),
        map_view.y,
        width,
        height,
    );
    let area = corner.intersection(map_view);
    if area.width < 3 || area.height < 3 {
        return None;
    }
    let inside = clear_box(buf, area, " Debug ", ("F3", ""), app.theme.text());
    for (row, line) in (inside.y..inside.bottom()).zip(lines) {
        buf.set_stringn(
            inside.x,
            row,
            line,
            usize::from(inside.width),
            Style::default(),
        );
    }
    Some(area)
}

/// Draws `lines` down `inner` from its top, as many as fit, each cut to its
/// width.
fn draw_lines(buf: &mut Buffer, inner: Rect, lines: &[Line]) {
    for (row, line) in (inner.y..inner.bottom()).zip(lines) {
        buf.set_line(inner.x, row, line, inner.width);
    }
}

/// Blanks every cell of `area`, colours and all, so nothing drawn beneath
/// an overlay shows through it: a coloured background, such as the
/// Cursor's or a graph's, or reverse video.
pub(crate) fn blank(buf: &mut Buffer, area: Rect) {
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf[(x, y)].reset();
        }
    }
}

/// Where text `width` cells wide sits at the right of `area`'s top row: two
/// cells in from its edge, before a box's corner and the line beside it, as
/// a title sits after them.
pub(crate) fn top_right(area: Rect, width: u16) -> Option<Rect> {
    let x = area.right().checked_sub(width + 2)?;
    Some(Rect::new(x, area.y, width, 1))
}

/// Blanks `area` and draws a single-lined box round it, with `title` at
/// the left of its top edge and the key hint `corner` at the right, its key
/// bold and what it does muted (design §6.2), as an overlay over the panels
/// beneath. Gives the area inside the box.
pub(crate) fn clear_box(
    buf: &mut Buffer,
    area: Rect,
    title: &str,
    corner: (&str, &str),
    look: TextLook,
) -> Rect {
    blank(buf, area);
    draw_border(buf, area, title);
    let (key, does) = corner;
    let mut spans = vec![Span::raw(" "), Span::styled(key, look.key)];
    if !does.is_empty() {
        spans.push(Span::styled(format!(" {does}"), look.muted));
    }
    spans.push(Span::raw(" "));
    let corner = Line::from(spans);
    if let Some(at) = top_right(area, corner.width() as u16) {
        buf.set_line(at.x, at.y, &corner, at.width);
    }
    area.inner(Margin::new(1, 1))
}

/// The glyph of what the Cursor has hold of, as the queue will leave it: a
/// sprite's, or an item's, held or still on the map (design v23 §6.5); or
/// of the Place menu item waiting on it (design v28 §6.5).
fn held_glyph(app: &App, world: &World) -> char {
    let theme = &app.theme;
    if let Some(item) = app.placing_item() {
        let glyph = match item {
            PlaceItem::Object { name, .. } => {
                let look = world.data().new_look(name).unwrap_or("default");
                theme.object_glyph(name, look)
            }
            PlaceItem::NewSprite | PlaceItem::Genome(_) => theme.glyph(SemanticTile::Sprite),
        };
        return glyph.symbol;
    }
    let glyph = match app.grip(world) {
        Some(Grip::Holds(id)) => {
            item_look(world, id).map(|look| theme.object_glyph(look.type_name, look.visual_state))
        }
        // A led sprite as the map draws it: the young small (M2 design
        // §9.6).
        Some(Grip::Leads(id)) => {
            let stage = world.sprite(id).map(|sprite| sprite.life_stage());
            Some(theme.glyph(stage.map_or(SemanticTile::Sprite, |s| sprite_tile(s, false))))
        }
        None => None,
    };
    glyph.map_or('?', |glyph| glyph.symbol)
}

/// How a sprite in `stage` is drawn, `selected` or not: the young small (M2
/// design §9.6).
fn sprite_tile(stage: LifeStage, selected: bool) -> SemanticTile {
    match (selected, stage) {
        (true, LifeStage::Young) => SemanticTile::SelectedYoungSprite,
        (false, LifeStage::Young) => SemanticTile::YoungSprite,
        (true, _) => SemanticTile::SelectedSprite,
        (false, _) => SemanticTile::Sprite,
    }
}

/// Draws a single-lined box around `area` with `title` in its top edge.
pub(crate) fn draw_border(buf: &mut Buffer, area: Rect, title: &str) {
    let (left, right) = (area.left(), area.right() - 1);
    let (top, bottom) = (area.top(), area.bottom() - 1);
    for x in left + 1..right {
        buf[(x, top)].set_symbol("─");
        buf[(x, bottom)].set_symbol("─");
    }
    for y in top + 1..bottom {
        buf[(left, y)].set_symbol("│");
        buf[(right, y)].set_symbol("│");
    }
    buf[(left, top)].set_symbol("┌");
    buf[(right, top)].set_symbol("┐");
    buf[(left, bottom)].set_symbol("└");
    buf[(right, bottom)].set_symbol("┘");
    // The title sits one cell in from the corner, as in "┌─ Map ──".
    let room = usize::from(area.width.saturating_sub(3));
    buf.set_stringn(left + 2, top, title, room, Style::default());
}

/// The top bar (design §6.1): the tick, the speed, the seed and the
/// population, and at the right how to open help.
fn top_bar_line(app: &App, world: &World, width: u16) -> Line<'static> {
    let clock = &app.clock;
    // Paused, it still shows the speed: `+` and `-` change it, and it sets
    // how far `.` steps (design v27 §6.6). A Lab world says so after it
    // (M2 design §16).
    let lab = if world.mode() == Mode::Lab {
        " lab"
    } else {
        ""
    };
    let time = if clock.is_paused() {
        format!("|| paused {}{lab}", clock.speed().label())
    } else {
        format!("► {}{lab}", clock.speed().label())
    };
    // The population is a count the policy may hide (design §6.4).
    let population = app
        .can_view(Panel::Counts, Subject::World)
        .then(|| format!("sprites {}", world.sprites().count()));
    // When the world was last saved or loaded (design §6.1, §6.7); in a
    // replay, how playback stands instead (design §2.7).
    let saved = match app.saved_ago() {
        _ if app.replaying() => app.replay_status().unwrap_or_default(),
        None => "not saved".to_string(),
        // "Just now" only briefly, then seconds, whole and then in tens, so
        // the bar never looks stuck (#129).
        Some(ago) if ago.as_secs() < 3 => "saved just now".to_string(),
        Some(ago) if ago.as_secs() < 10 => format!("saved {}s ago", ago.as_secs()),
        Some(ago) if ago.as_secs() < 60 => format!("saved {}s ago", ago.as_secs() / 10 * 10),
        Some(ago) => format!("saved {} ago", spoken_duration(ago)),
    };
    // Each piece, and how soon it goes when the bar is short of room for
    // `? help`: the seed first, then the game's name, when it was saved and
    // how many sprites there are, so the time and the help's key stay
    // (design §6.1, #305).
    let mut pieces: Vec<(Option<u8>, String)> = vec![
        (Some(1), "Terra Sprites".to_string()),
        (None, format!("tick {}", thousands(world.tick()))),
        (None, time),
        (Some(0), format!("seed {}", app.seed)),
    ];
    pieces.extend(population.map(|count| (Some(3), count)));
    pieces.push((Some(2), saved));
    let joined = |pieces: &[(Option<u8>, String)]| {
        let shown: Vec<&str> = pieces.iter().map(|(_, piece)| piece.as_str()).collect();
        format!(" {}", shown.join(" │ "))
    };
    let help = help_hint(app, world);
    let wide = |hint: &Vec<Span>| hint.iter().map(Span::width).sum::<usize>();
    for soonest in 0..4 {
        if joined(&pieces).chars().count() + wide(&help.plain) < usize::from(width) {
            break;
        }
        pieces.retain(|&(goes, _)| goes != Some(soonest));
    }
    let text = joined(&pieces);
    // `? help`, naming the page `?` would open, `? help (Inspector)`, or
    // without it if the bar's short of room (#303). At least a space comes
    // between it and the rest, so it never runs into them.
    let room = usize::from(width).saturating_sub(text.chars().count());
    let mut spans = vec![Span::raw(text)];
    if let Some(hint) = [help.named, help.plain]
        .into_iter()
        .find(|hint| wide(hint) < room)
    {
        spans.push(Span::raw(" ".repeat(room - wide(&hint))));
        spans.extend(hint);
    }
    Line::from(spans).style(Style::default().add_modifier(Modifier::REVERSED))
}

/// The top bar's `? help`, as it's drawn with the page `?` would open
/// named after it, and without.
struct HelpHint {
    named: Vec<Span<'static>>,
    plain: Vec<Span<'static>>,
}

/// The top bar's `? help` (#303): its key bold, as every hint's is (design
/// §6.2), unless `?` would type rather than open help; the rest in the
/// bar's reverse video, where muting would hide it. While help is open, `?`
/// closes it, so no page is named.
fn help_hint(app: &App, world: &World) -> HelpHint {
    let key = app.theme.text().key;
    let mut named = Vec::new();
    let mut plain = Vec::new();
    for hint in controls::hints_of(&controls::HELP, app, world) {
        let key = if hint.ready { key } else { Style::default() };
        let page = if hint.ready && app.screen() != Screen::Help {
            format!(" ({})", app.help_page_here().label())
        } else {
            String::new()
        };
        named.extend([
            Span::styled(hint.key, key),
            Span::raw(format!(" {}{page} ", hint.does)),
        ]);
        plain.extend([
            Span::styled(hint.key, key),
            Span::raw(format!(" {} ", hint.does)),
        ]);
    }
    HelpHint { named, plain }
}

/// How many columns `hints` take, two spaces between each: `Z select  X
/// train`.
fn hint_width(hints: &[Hint]) -> usize {
    let each = hints
        .iter()
        .map(|hint| hint.key.chars().count() + 1 + hint.does.chars().count());
    each.sum::<usize>() + 2 * hints.len().saturating_sub(1)
}

/// `hints` as the status line draws them: each key in the theme's key
/// style and what it does muted (design §6.2), or all of a hint muted where
/// it does nothing just now (#303), two spaces between each.
pub(crate) fn hint_spans(hints: &[Hint], look: TextLook) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, hint) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let key = if hint.ready { look.key } else { look.muted };
        spans.push(Span::styled(hint.key, key));
        spans.push(Span::styled(format!(" {}", hint.does), look.muted));
    }
    spans
}

/// Every hint `controls` give in `app` on `world`, in order.
pub(crate) fn all_hints(controls: &[Control], app: &App, world: &World) -> Vec<Hint> {
    controls
        .iter()
        .flat_map(|control| controls::hints_of(control, app, world))
        .collect()
}

/// The tile under the cursor, with any sprite and object on it, and the
/// cursor mode, then at the right the key hints that fit in `width` cells, or
/// why a click was refused in their place. An open prompt takes the line over.
fn status_line(app: &App, world: &World, width: u16) -> Line<'static> {
    let look = app.theme.text();
    if app.screen() == Screen::QuitPrompt {
        let mut prompt = quit_prompt(app, world);
        prompt.spans.insert(0, Span::raw(" "));
        return prompt;
    }
    if let Some(question) = app.load_question() {
        return yes_or_no(&question, look);
    }
    // Naming a save takes the line over too (design §6.7).
    if let Some(draft) = app.save_name_draft() {
        let prompt = format!(" Save as: {draft}_");
        return prompt_line(app, world, prompt, Context::SaveNaming, width);
    }
    // Naming takes the line over, with its key hints if they fit (design
    // v28 §6.5). Like the Cursor's own lines, it names the sprite only if
    // the policy lets it (design §6.4, #174).
    if let (Some(draft), Some(sprite)) = (app.name_draft(), app.naming_sprite()) {
        let label = cursor_part(app, Subject::Sprite(sprite), || {
            format!(" {}", app.names().label(sprite))
        });
        let prompt = format!(" Name{label}: {draft}_");
        return prompt_line(app, world, prompt, Context::Naming, width);
    }
    let cursor = app.cursor();
    let terrain = terrain_name(world.map().terrain(cursor));
    let sprite = world
        .sprite_at(cursor)
        .map(|sprite| format!(" · {}", app.names().label(sprite.id())))
        .unwrap_or_default();
    // As the map shows it, with a queued pick-up or put-down done (design
    // §6.5, #203).
    let object = app
        .shown_items(world)
        .at(cursor)
        .map(|item| format!(" · {}", item_label(&item)))
        .unwrap_or_default();
    // Whether sprites can see the Cursor in this mode (design v29 §6.1).
    let seen = if app.visible() { " · seen" } else { "" };
    let mode = format!("{}{seen}", app.mode().label());
    let followed = app
        .followed()
        .map(|id| {
            cursor_part(app, Subject::Sprite(id), || {
                format!(" │ following {}", app.names().label(id))
            })
        })
        .unwrap_or_default();
    // What the Cursor has hold of, in every mode (design v23 §6.1).
    let grip = match app.grip(world) {
        Some(Grip::Leads(id)) => cursor_part(app, Subject::Sprite(id), || {
            format!(" │ leading: {}", app.names().label(id))
        }),
        Some(Grip::Holds(id)) => cursor_part(app, Subject::World, || {
            let name = item_look(world, id).map_or("?", |look| look.type_name);
            format!(" │ holding: {}", display_name(name))
        }),
        None => String::new(),
    };
    // A Place menu item waiting on the Cursor, in every mode (design v28 §6.5).
    let placing = app
        .placing()
        .map(|label| format!(" │ placing: {label}"))
        .unwrap_or_default();
    // The tile under the Cursor is information the policy may hide (design
    // §6.4). Out of reach past the wall, there's only space, and a
    // withdrawn Cursor is under nothing (M2 design §11.4).
    let under = if app.cursor_withdrawn() {
        String::new()
    } else if app.out_of_reach() {
        " space │".to_string()
    } else if app.can_view(Panel::TileInfo, Subject::Tile(cursor)) {
        format!(" ({},{}) {terrain}{sprite}{object} │", cursor.x, cursor.y)
    } else {
        String::new()
    };
    let width = usize::from(width);
    let tile = format!("{under} {mode}{followed}{grip}{placing}");
    // Its parts' dots minor (design §6.2).
    let tile_line = |tile: &str| inspector::dotted(tile, Style::default(), look.muted);
    // At the right, after a gap of 2 and before a space at the end: why a
    // click did nothing, for a while, or else the key hints that fit (design
    // v22 §6.1). The reason matters more than the end of the tile's part,
    // which is cut short to make room for it.
    if let Some(why) = app.refusal().or(app.notice()) {
        let room = width.saturating_sub(why.chars().count() + 3);
        let tile: String = tile.chars().take(room).collect();
        let gap = width.saturating_sub(tile.chars().count() + why.chars().count() + 1);
        let mut line = tile_line(&tile);
        line.spans
            .push(Span::raw(format!("{}{why} ", " ".repeat(gap))));
        return line;
    }
    let room = width.saturating_sub(tile.chars().count() + 3);
    // The hints for where the player is: in the world, while aiming or
    // placing (design v25, v28 §6.1), or with an overlay open (#303).
    let context = controls::context(app, world);
    let hints = hints_within(controls::hints(context), app, world, room);
    let mut line = tile_line(&tile);
    if hints.is_empty() {
        return line;
    }
    let gap = width.saturating_sub(tile.chars().count() + hint_width(&hints) + 1);
    line.spans.push(Span::raw(" ".repeat(gap)));
    line.spans.extend(hint_spans(&hints, look));
    line.spans.push(Span::raw(" "));
    line
}

/// Lights up sprite or egg `id` on the map, in `buf`, as a selected one is
/// drawn, in reverse video, while the pointer is on a line of the event log
/// about it (design §6.1): unless a pop-up the frame drew, its `spots`, is
/// over it.
fn light_up(buf: &mut Buffer, app: &App, world: &World, id: EntityId, spots: &Spots) {
    let Some(cell) = in_the_world(world, id)
        .flatten()
        .and_then(|pos| app.cell_of(pos))
    else {
        return;
    };
    if spots.at(cell).is_none() {
        buf[cell].modifier.insert(Modifier::REVERSED);
    }
}

/// A prompt that takes the status line over, with the hints for `context`
/// after it if they all fit in `width`.
fn prompt_line(
    app: &App,
    world: &World,
    prompt: String,
    context: Context,
    width: u16,
) -> Line<'static> {
    let hints = all_hints(controls::hints(context), app, world);
    let fits = prompt.chars().count() + 3 + hint_width(&hints) <= usize::from(width);
    let mut spans = vec![Span::raw(prompt)];
    if fits {
        spans.push(Span::raw("   "));
        spans.extend(hint_spans(&hints, app.theme.text()));
    }
    Line::from(spans)
}

/// A part of the status line among the Cursor's own lines, about `subject`,
/// or nothing if the policy hides it (design §6.4, #174).
fn cursor_part(app: &App, subject: Subject, part: impl FnOnce() -> String) -> String {
    if app.can_view(Panel::CursorLines, subject) {
        part()
    } else {
        String::new()
    }
}

/// As many of the hints `controls` give as fit in `room` columns, in order
/// (design v22 §6.1): short of room, a hint that goes first does, then
/// whole controls' hints from the end, so the hints of one control, such as
/// the mode keys, go together.
fn hints_within(controls: &[Control], app: &App, world: &World, room: usize) -> Vec<Hint> {
    let mut kept: Vec<(bool, Vec<Hint>)> = controls
        .iter()
        .map(|control| {
            let first = matches!(control.says, Says::WordGoingFirst(_));
            (first, controls::hints_of(control, app, world))
        })
        .collect();
    let width = |kept: &[(bool, Vec<Hint>)]| {
        let hints: Vec<Hint> = kept.iter().flat_map(|(_, hints)| hints.clone()).collect();
        hint_width(&hints)
    };
    while !kept.is_empty() && width(&kept) > room {
        match kept.iter().position(|(first, _)| *first) {
            Some(first) => kept.remove(first),
            None => kept.pop().expect("not empty"),
        };
    }
    kept.into_iter().flat_map(|(_, hints)| hints).collect()
}

/// An item's display name, with its stage if its type has stages: `berry bush (mature)`.
fn item_label(item: &ItemLook) -> String {
    let name = display_name(item.type_name);
    match item.stage {
        Some(stage) => format!("{name} ({stage})"),
        None => name,
    }
}

/// Draws the inspector (design §6.1): its title row, with the selected
/// sprite and the tabs, and the open tab below it. In a theme with shades,
/// it sits on them with no lines; in one without, a line runs down its left,
/// between it and the map view.
fn render_inspector(buf: &mut Buffer, area: Rect, app: &App, world: &World, spots: &mut Spots) {
    // Over the map view, or filling the screen, nothing beneath shows
    // through past a short line (design §6.1, #305).
    blank(buf, area);
    match app.theme.panels() {
        Some(panels) => shade(buf, area, panels),
        None => {
            for y in area.top()..area.bottom() {
                buf[(area.x, y)].set_symbol("│");
            }
        }
    }
    let title = inspector::title(app);
    buf.set_line(area.x + 1, area.y, &title, area.width.saturating_sub(1));
    // Each tab where its name is, so a click there opens it (design §6.1).
    for (tab, columns) in inspector::tab_places(app.tab()) {
        let start = area.x + 1 + columns.start as u16;
        let place = Rect::new(start, area.y, columns.len() as u16, 1).intersection(area);
        spots.note(place, Spot::Tab(tab));
    }
    // In columns, where there's room for more than one (design §6.1, #305),
    // each line's spots where it's drawn.
    let columns = TabColumns::of(area);
    let lines = inspector::shown(app, world);
    let first = first_shown(app.tab_scroll(), lines.len(), columns.at_once());
    for (shown, line) in lines.into_iter().skip(first).enumerate() {
        let Some(row) = columns.place(shown) else {
            break;
        };
        buf.set_line(row.x, row.y, &line.line, row.width);
        for (cells, spot) in line.spots {
            let place = Rect::new(row.x + cells.start, row.y, cells.len() as u16, 1);
            let place = place.intersection(row);
            if line.minor {
                spots.note_unmarked(place, spot);
            } else {
                spots.note(place, spot);
            }
        }
    }
}

/// Shades a panel, `area`, and its title row, its first, as `panels` say
/// (design §6.1). What's drawn on it later keeps the shade unless it sets
/// a background of its own.
fn shade(buf: &mut Buffer, area: Rect, panels: PanelLook) {
    buf.set_style(area, Style::default().bg(panels.shade));
    let title = Rect::new(area.x, area.y, area.width, area.height.min(1));
    buf.set_style(title, Style::default().bg(panels.title));
}

/// Draws the event log (design §6.1): a title row, with the filters at its
/// right, and the latest events below it, newest first. In a theme with
/// shades, it sits on them; in one without, its title row is a line.
fn render_event_log(buf: &mut Buffer, area: Rect, app: &App, world: &World, spots: &mut Spots) {
    match app.theme.panels() {
        Some(panels) => shade(buf, area, panels),
        None => {
            for x in area.left()..area.right() {
                buf[(x, area.y)].set_symbol("─");
            }
        }
    }
    let room = usize::from(area.width.saturating_sub(1));
    buf.set_stringn(area.x + 1, area.y, " Events ", room, Style::default());
    // The filters, at the right of its title row, where a click on them
    // lands (design §6.1).
    let labels = app.event_filter().labels();
    if let Some(at) = top_right(area, labels.chars().count() as u16) {
        buf.set_stringn(at.x, at.y, labels, usize::from(at.width), Style::default());
        spots.note(at, Spot::Filters);
    }
    let inner = Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(1),
    );
    let lines = app.event_log().filter_map(|(event, count)| {
        // The policy may hide what an event is about (design §6.4).
        let about = inspector::event_sprites(event).first().copied();
        let subject = about.map_or(Subject::World, Subject::Sprite);
        if !app.can_view(Panel::EventLog, subject) {
            return None;
        }
        let text = inspector::event_line(event, &app.words(world))?;
        Some((event.tick, text, count, about))
    });
    let muted = app.theme.text().muted;
    for (row, (tick, text, count, about)) in (inner.y..inner.bottom()).zip(lines) {
        let times = if count > 1 {
            // An x: CP437 has no × (design §6.2).
            format!(" x{count}")
        } else {
            String::new()
        };
        // The tick is minor beside what happened (design §6.2).
        let tick = format!(" {:>7}", thousands(tick));
        let happened = format!("  {text}{times}");
        // The name of the sprite it's about, where a tooltip says which
        // sprite that is, and a click selects it (design §6.1, #304); the
        // whole of what happened if it isn't named.
        if let Some(id) = about {
            let label = app.names().label(id);
            let (start, width) = match happened.find(&label) {
                Some(at) => (happened[..at].chars().count(), label.chars().count()),
                None => (2, text.chars().count()),
            };
            let x = inner.x + (tick.chars().count() + start) as u16;
            let place = Rect::new(x, row, width as u16, 1);
            spots.note(place.intersection(inner), Spot::LogLine(id));
        }
        let line = Line::from(vec![Span::styled(tick, muted), Span::raw(happened)]);
        buf.set_line(inner.x, row, &line, inner.width);
    }
}
