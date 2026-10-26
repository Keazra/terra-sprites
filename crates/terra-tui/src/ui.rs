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
    App, Areas, ColourMode, CursorMode, ItemLook, PlaceItem, Screen, Selection, ShownItems,
    item_look, spoken_duration, strongest_drive,
};
use crate::family_tree::{self, Lineage};
use crate::inspector::{self, INSPECTOR_WIDTH, NOTHING_SELECTED, first_shown};
use crate::policy::{Panel, Subject};
use crate::space::{self, Place};
use crate::text::{display_name, terrain_name, thousands};
use crate::theme::{Glyph, PanelLook, SemanticTile, TextLook};

/// The narrowest terminal that has room for the inspector beside the map view.
const MIN_WIDTH_FOR_INSPECTOR: u16 = 100;
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
        return render_too_small(frame.buffer_mut(), area, app);
    }
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
    let inspector = inspector_area(area, parts);
    if let Some(inspector) = inspector {
        render_inspector(frame.buffer_mut(), inspector, app, world);
    }
    let event_log = event_log_area(area, parts);
    if let Some(event_log) = event_log {
        render_event_log(frame.buffer_mut(), event_log, app, world);
    }
    // Where the inspector's line meets the event log's, in a theme without
    // shades (design §6.1).
    if let (Some(inspector), Some(event_log), None) = (inspector, event_log, app.theme.panels()) {
        frame.buffer_mut()[(inspector.x, event_log.y)].set_symbol("┴");
    }
    render_menu(frame.buffer_mut(), area, app, world);
    match app.screen() {
        Screen::Help => render_help(frame.buffer_mut(), app, world),
        Screen::SpriteList => render_sprite_list(frame.buffer_mut(), app, world),
        Screen::Population => render_population(frame.buffer_mut(), app, world),
        Screen::FamilyTree => render_family_tree(frame.buffer_mut(), app, world),
        _ => {}
    }
    // Over whatever's open, so it's in a screenshot of any of them (design
    // §6.1).
    if app.debug_overlay() {
        render_debug_overlay(frame.buffer_mut(), map_view_area(area, parts), app);
    }
    if parts.status_line {
        frame.render_widget(status_line(app, world, status.width), status);
    }
}

/// What the status line asks when `Esc` would leave a world (M2 design
/// §8.4), each answer's key bold and what it does muted (design §6.2).
fn quit_prompt(look: TextLook) -> Line<'static> {
    const ANSWERS: &[Hint] = &[
        ("y", "quit the game"),
        ("t", "title screen"),
        ("any other key", "stays"),
    ];
    let mut spans = vec![Span::raw("Quit? ")];
    spans.extend(hint_spans(ANSWERS, look));
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

/// The smallest screen the game draws on (design §6.1).
pub const MIN_SIZE: Size = Size::new(100, 30);

/// Whether a screen of `screen` cells is too small for the game, or for
/// the title screen (M2 design §8.2).
pub(crate) fn too_small(screen: Size) -> bool {
    screen.width < MIN_SIZE.width || screen.height < MIN_SIZE.height
}

/// Says the terminal is too small, and how big it needs to be, in the
/// middle of the screen (design §6.1). The game carries on beneath.
fn render_too_small(buf: &mut Buffer, area: Rect, app: &App) {
    let mut questions = vec![];
    // `Esc` still asks to quit, so the question shows here too.
    if app.screen() == Screen::QuitPrompt {
        questions.push(quit_prompt(app.theme.text()).to_string());
    }
    if let Some(question) = app.load_question() {
        questions.push(question);
    }
    render_too_small_with(buf, area, questions);
}

/// Says the terminal is too small, as the game does, with any question
/// waiting under it: the title screen's too (M2 design §8.2).
pub(crate) fn render_too_small_with(buf: &mut Buffer, area: Rect, questions: Vec<String>) {
    let mut lines = vec![
        "Terminal too small".to_string(),
        format!(
            "needs {}x{}, this is {}x{}",
            MIN_SIZE.width, MIN_SIZE.height, area.width, area.height
        ),
    ];
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
            event_log: None,
            overlay: None,
        };
    }
    Areas {
        tiles: map_view_area(screen.into(), parts),
        inspector: inspector_area(screen.into(), parts),
        event_log: event_log_area(screen.into(), parts),
        overlay: Some(overlay_area(screen.into(), parts)),
    }
}

/// The map view: below the top bar if it's drawn, at the left, leaving room
/// for the inspector and the event log when they're drawn and there is some,
/// and out to the screen's edges, with no frame (design §6.1). A small map
/// is drawn in the middle of it, with space round it (M2 design §11.1).
fn map_view_area(screen: Rect, parts: Parts) -> Rect {
    let width = match inspector_area(screen, parts) {
        Some(inspector) => inspector.x - screen.x,
        None => screen.width,
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
/// it's drawn: at the right, below the top bar if that's drawn, on a
/// terminal wide enough for it.
fn inspector_area(screen: Rect, parts: Parts) -> Option<Rect> {
    let room = screen.width >= MIN_WIDTH_FOR_INSPECTOR && screen.height > 2;
    (parts.inspector && room).then(|| {
        Rect::new(
            screen.right() - INSPECTOR_WIDTH,
            screen.y + parts.top_rows(),
            INSPECTOR_WIDTH,
            panels_height(screen, parts),
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
    for y in inner.top()..inner.bottom() {
        for x in inner.left()..inner.right() {
            let pos = match app.place_from(inner.as_position(), Position::new(x, y)) {
                Place::Tile(pos) => pos,
                Place::Glass(pane) => {
                    let glass = app.theme.glass(pane);
                    buf[(x, y)]
                        .set_char(glass.symbol)
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
            buf[(x, y)].set_char(glyph.symbol).set_style(style);
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
    for (depth, at) in space::stars(inner.as_size(), app.viewport(), app.drift()) {
        let cell = Position::new(inner.x + at.x, inner.y + at.y);
        if app.place_from(inner.as_position(), cell) == Place::Space {
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
    let pieces = [
        (-1, -1, mark.symbol),
        (0, -1, arrows.down),
        (1, -1, y),
        (-1, 0, arrows.right),
        (1, 0, arrows.left),
        (-1, 1, n),
        (0, 1, arrows.up),
        (1, 1, mark.symbol),
    ];
    for (dx, dy, glyph) in pieces {
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
        put(buf, cell, mark.symbol, mark.fg);
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
            put(buf, cell, dot.symbol, dot.fg);
        }
    }
}

/// Draws `symbol` at screen cell `cell`, in `fg`.
fn put(buf: &mut Buffer, cell: Position, symbol: char, fg: Color) {
    buf[(cell.x, cell.y)]
        .set_char(symbol)
        .set_style(Style::default().fg(fg));
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
fn render_menu(buf: &mut Buffer, screen: Rect, app: &App, world: &World) {
    let (Some(menu), Some(title)) = (app.menu_area(world), app.menu_title()) else {
        return;
    };
    let area = menu.intersection(screen);
    if area.width < 3 || area.height < 3 {
        return;
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
}

/// Draws the help screen over everything between the top bar and the
/// status line (design §6.1).
fn render_help(buf: &mut Buffer, app: &App, world: &World) {
    let Some(area) = app.overlay() else {
        return;
    };
    let inner = clear_box(buf, area, " Help ", ESC_CLOSE, app.theme.text());
    let lines = crate::help::lines(app, world, usize::from(inner.width));
    draw_lines(buf, inner, &lines);
}

/// Draws the sprite list over everything between the top bar and the
/// status line (design §6.1).
fn render_sprite_list(buf: &mut Buffer, app: &App, world: &World) {
    let Some(area) = app.overlay() else {
        return;
    };
    // While it's filtered, `Esc` clears the filter first (#313).
    let corner = match app.list_filter() {
        Some(_) => ("esc", "clear"),
        None => ESC_CLOSE,
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
    let inner = clear_box(buf, area, &title, ESC_CLOSE, app.theme.text());
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
    let inner = clear_box(buf, area, &title, ESC_CLOSE, app.theme.text());
    if app.selection().is_none() {
        buf.set_string(inner.x, inner.y, NOTHING_SELECTED, Style::default());
    }
    // Nothing more if the policy hides the sprite in the middle.
    if let Some(tree) = app.tree(world) {
        draw_lines(buf, inner, &tree.lines);
    }
}

/// Draws the debug overlay (`F3`, design §6.1, #311) in the map view's
/// top-right corner, `map_view`, as far as the map view goes.
fn render_debug_overlay(buf: &mut Buffer, map_view: Rect, app: &App) {
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
        return;
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
fn blank(buf: &mut Buffer, area: Rect) {
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
fn draw_border(buf: &mut Buffer, area: Rect, title: &str) {
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
    let population = if app.can_view(Panel::Counts, Subject::World) {
        format!(" │ sprites {}", world.sprites().count())
    } else {
        String::new()
    };
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
    let text = format!(
        " Terra Sprites │ tick {} │ {time} │ seed {}{population} │ {saved}",
        thousands(world.tick()),
        app.seed,
    );
    const HELP: Hint = ("?", "help");
    let help = hint_width(&[HELP]) + 1;
    let gap = usize::from(width).saturating_sub(text.chars().count() + help);
    // At least a space between the two, so `? help` never runs into them.
    let mut spans = vec![Span::raw(text)];
    if gap > 0 {
        spans.push(Span::raw(" ".repeat(gap)));
        // Its key bold, as every hint's is (design §6.2); the rest stays in
        // the bar's reverse video.
        let key = app.theme.text().key;
        spans.extend([
            Span::styled(HELP.0, key),
            Span::raw(format!(" {} ", HELP.1)),
        ]);
    }
    Line::from(spans).style(Style::default().add_modifier(Modifier::REVERSED))
}

/// A key hint: a key, and what it does (design v22 §6.1), drawn with the
/// key bold and what it does muted (design §6.2).
type Hint = (&'static str, &'static str);

/// The hint in an overlay's corner (design §6.1).
pub(crate) const ESC_CLOSE: Hint = ("esc", "close");

/// The keys that work now, shown at the right of the status line as far as
/// there's room. The mode keys come first, as one hint, so they're the last
/// to go (design v22 §6.1).
const KEY_HINTS: [&[Hint]; 6] = [
    &[("Z", "select"), ("X", "train"), ("C", "grab")],
    &[("WASD", "scroll")],
    &[("space", "pause")],
    &[(".", "step")],
    &[("+/-", "speed")],
    &[("esc", "quit")],
];

/// How many columns `hints` take, two spaces between each: `Z select  X
/// train`.
fn hint_width(hints: &[Hint]) -> usize {
    let each = hints
        .iter()
        .map(|(key, does)| key.chars().count() + 1 + does.chars().count());
    each.sum::<usize>() + 2 * hints.len().saturating_sub(1)
}

/// `hints` as the status line draws them: each key in the theme's key
/// style and what it does muted, two spaces between each (design §6.2).
fn hint_spans(hints: &[Hint], look: TextLook) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for (i, (key, does)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(*key, look.key));
        spans.push(Span::styled(format!(" {does}"), look.muted));
    }
    spans
}

/// The tile under the cursor, with any sprite and object on it, and the
/// cursor mode, then at the right the key hints that fit in `width` cells, or
/// why a click was refused in their place. An open prompt takes the line over.
fn status_line(app: &App, world: &World, width: u16) -> Line<'static> {
    let look = app.theme.text();
    if app.screen() == Screen::QuitPrompt {
        let mut prompt = quit_prompt(look);
        prompt.spans.insert(0, Span::raw(" "));
        return prompt;
    }
    if let Some(question) = app.load_question() {
        return yes_or_no(&question, look);
    }
    // Naming a save takes the line over too (design §6.7).
    if let Some(draft) = app.save_name_draft() {
        let prompt = format!(" Save as: {draft}_");
        const HINTS: &[Hint] = &[("enter", "save"), ("esc", "cancel")];
        return prompt_line(app, prompt, HINTS, width);
    }
    // Naming takes the line over, with its key hints if they fit (design
    // v28 §6.5). Like the Cursor's own lines, it names the sprite only if
    // the policy lets it (design §6.4, #174).
    if let (Some(draft), Some(sprite)) = (app.name_draft(), app.naming_sprite()) {
        let label = cursor_part(app, Subject::Sprite(sprite), || {
            format!(" {}", app.names().label(sprite))
        });
        let prompt = format!(" Name{label}: {draft}_");
        const HINTS: &[Hint] = &[("enter", "ok"), ("tab", "another"), ("esc", "cancel")];
        return prompt_line(app, prompt, HINTS, width);
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
    let tile = format!("{under} {mode}{followed}{grip}{placing}");
    // Its parts' dots minor (design §6.2).
    let tile_line = |tile: &str| inspector::dotted(tile, Style::default(), look.muted);
    // At the right, after a gap of 2 and before a space at the end: why a
    // click did nothing, for a while, or else the key hints that fit (design
    // v22 §6.1). The reason matters more than the end of the tile's part,
    // which is cut short to make room for it.
    let width = usize::from(width);
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
    // While the player aims, how to send it or not (design v25 §6.1).
    let placing = app.placing().is_some() && app.mode() == CursorMode::Grab;
    let choices: Vec<Vec<Hint>> = match (app.aiming(), app.grip(world)) {
        (true, Some(Grip::Holds(_))) => vec![vec![("let go", "to throw"), ("esc", "cancel")]],
        (true, Some(Grip::Leads(_))) => vec![vec![("let go", "to shove"), ("esc", "cancel")]],
        // A Place menu item waiting, how to place it or not (design v28 §6.5).
        _ if placing => vec![vec![("click", "to place"), ("right-click", "put away")]],
        // While the sprite list is open, how to use it (design §6.1),
        // without `/ filter` where it would crowd the rest out; and while
        // it's filtered, how to clear the filter (#313).
        _ if app.screen() == Screen::SpriteList => match app.list_filter() {
            Some(_) => vec![vec![
                ("↑↓", "choose"),
                ("enter", "go to it"),
                ("tab", "sort"),
                ("esc", "clear"),
            ]],
            None => vec![
                vec![
                    ("/", "filter"),
                    ("↑↓", "choose"),
                    ("enter", "go to it"),
                    ("tab", "sort"),
                    ("esc", "close"),
                ],
                vec![
                    ("↑↓", "choose"),
                    ("enter", "go to it"),
                    ("tab", "sort"),
                    ("esc", "close"),
                ],
            ],
        },
        // And the population graph (M2 design §14).
        _ if app.screen() == Screen::Population => vec![vec![
            ("←→", "pick a moment"),
            ("space", "pause"),
            ("+/-", "speed"),
            ("esc", "close"),
        ]],
        // And the family tree (M2 design §17.2).
        _ if app.screen() == Screen::FamilyTree => vec![vec![
            ("←→↑↓", "choose"),
            ("enter", "centre on it"),
            ("space", "pause"),
            ("esc", "close"),
        ]],
        _ => vec![hints_within(room)],
    };
    let hints = choices
        .into_iter()
        .find(|hints| hint_width(hints) <= room)
        .unwrap_or_default();
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

/// A prompt that takes the status line over, with `hints` after it if they
/// fit in `width`.
fn prompt_line(app: &App, prompt: String, hints: &[Hint], width: u16) -> Line<'static> {
    let fits = prompt.chars().count() + 3 + hint_width(hints) <= usize::from(width);
    let mut spans = vec![Span::raw(prompt)];
    if fits {
        spans.push(Span::raw("   "));
        spans.extend(hint_spans(hints, app.theme.text()));
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

/// As many of the key hints as fit in `room` columns, whole and in order.
fn hints_within(room: usize) -> Vec<Hint> {
    let mut hints = Vec::new();
    for hint in KEY_HINTS {
        let longer = [hints.as_slice(), hint].concat();
        if hint_width(&longer) > room {
            break;
        }
        hints = longer;
    }
    hints
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
fn render_inspector(buf: &mut Buffer, area: Rect, app: &App, world: &World) {
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
    let inner = inspector::tab_area(area);
    let lines = inspector::lines(app, world);
    let first = first_shown(app.tab_scroll(), lines.len(), usize::from(inner.height));
    let shown = lines.iter().skip(first);
    for (row, line) in (inner.y..inner.bottom()).zip(shown) {
        buf.set_line(inner.x, row, line, inner.width);
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
fn render_event_log(buf: &mut Buffer, area: Rect, app: &App, world: &World) {
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
    }
    let inner = Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(1),
    );
    let lines = app.event_log().filter_map(|(event, count)| {
        // The policy may hide what an event is about (design §6.4).
        let subject = inspector::event_sprites(event)
            .first()
            .map_or(Subject::World, |&id| Subject::Sprite(id));
        if !app.can_view(Panel::EventLog, subject) {
            return None;
        }
        let text = inspector::event_line(event, &app.words(world))?;
        Some((event.tick, text, count))
    });
    let muted = app.theme.text().muted;
    for (row, (tick, text, count)) in (inner.y..inner.bottom()).zip(lines) {
        let times = if count > 1 {
            // An x: CP437 has no × (design §6.2).
            format!(" x{count}")
        } else {
            String::new()
        };
        // The tick is minor beside what happened (design §6.2).
        let line = Line::from(vec![
            Span::styled(format!(" {:>7}", thousands(tick)), muted),
            Span::raw(format!("  {text}{times}")),
        ]);
        buf.set_line(inner.x, row, &line, inner.width);
    }
}
