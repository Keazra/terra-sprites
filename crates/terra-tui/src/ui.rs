//! Draws the screen. Rendering is a pure function of the app state (design §6.8).

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Position, Rect, Size},
    style::{Modifier, Style},
    text::Line,
};
use terra_sim::{DataPack, Event, EventKind, Map, ObjectView, Pos, Progress, Terrain, World};

use crate::app::{App, Areas, Screen, Selection};
use crate::clock::Speed;
use crate::inspector::{self, INSPECTOR_WIDTH, first_shown};
use crate::text::{cause_name, display_name, group_thousands, sprite_label};
use crate::theme::SemanticTile;

/// The narrowest terminal that has room for the inspector beside the map view.
const MIN_WIDTH_FOR_INSPECTOR: u16 = 100;
/// The event log's height, in rows, border included: three events (design §6.1).
const EVENT_LOG_HEIGHT: u16 = 5;
/// The shortest terminal that has room for the event log under the map view.
const MIN_HEIGHT_FOR_EVENT_LOG: u16 = 30;

/// Draws one frame: the top bar, the map view, the inspector and the event
/// log if there's room, and the status line.
pub fn render(frame: &mut Frame, app: &App, world: &World) {
    let area = frame.area();
    let [top_bar, _, status] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area);
    frame.render_widget(top_bar_line(app, world), top_bar);
    render_map_view(
        frame.buffer_mut(),
        map_view_area(area, world.map()),
        app,
        world,
    );
    if let Some(inspector) = inspector_area(area) {
        render_inspector(frame.buffer_mut(), inspector, app, world);
    }
    if let Some(event_log) = event_log_area(area) {
        render_event_log(frame.buffer_mut(), event_log, app, world);
    }
    frame.render_widget(status_line(app, world, status.width), status);
}

/// Where the app's panels are drawn on a screen of `screen` cells. The map
/// view draws its tiles inside its border, between the top bar and the
/// status line, no bigger than the map itself.
pub fn areas(screen: Size, map: &Map) -> Areas {
    Areas {
        tiles: map_view_area(screen.into(), map).inner(Margin::new(1, 1)),
        inspector: inspector_area(screen.into()),
    }
}

/// The map view, border included: below the top bar, at the left, shrunk to
/// fit a small map, and leaving room for the inspector and the event log when
/// there is some.
fn map_view_area(screen: Rect, map: &Map) -> Rect {
    let room = match inspector_area(screen) {
        Some(inspector) => inspector.x - screen.x,
        None => screen.width,
    };
    let width = (map.width().saturating_add(2)).min(room);
    let height = (map.height().saturating_add(2)).min(panels_height(screen));
    Rect::new(screen.x, screen.y + 1.min(screen.height), width, height)
}

/// The rows for the map view and the inspector: between the top bar and the
/// event log, or the status line if there's no room for the log.
fn panels_height(screen: Rect) -> u16 {
    let log = if event_log_area(screen).is_some() {
        EVENT_LOG_HEIGHT
    } else {
        0
    };
    screen.height.saturating_sub(2 + log)
}

/// The inspector, border included: at the right, below the top bar, on a
/// terminal wide enough for it.
fn inspector_area(screen: Rect) -> Option<Rect> {
    (screen.width >= MIN_WIDTH_FOR_INSPECTOR && screen.height > 2).then(|| {
        Rect::new(
            screen.right() - INSPECTOR_WIDTH,
            screen.y + 1,
            INSPECTOR_WIDTH,
            panels_height(screen),
        )
    })
}

/// The event log, border included: the full width, just above the status
/// line, on a terminal tall enough for it.
fn event_log_area(screen: Rect) -> Option<Rect> {
    (screen.height >= MIN_HEIGHT_FOR_EVENT_LOG).then(|| {
        Rect::new(
            screen.x,
            screen.bottom() - 1 - EVENT_LOG_HEIGHT,
            screen.width,
            EVENT_LOG_HEIGHT,
        )
    })
}

fn render_map_view(buf: &mut Buffer, area: Rect, app: &App, world: &World) {
    let map = world.map();
    if area.width < 2 || area.height < 2 {
        return;
    }
    let inner = area.inner(Margin::new(1, 1));
    let origin = app.viewport();
    // The wall is in view on any side where the viewport reaches the map's edge.
    let walls = Sides {
        left: origin.x == 0,
        right: origin.x + inner.width >= map.width(),
        top: origin.y == 0,
        bottom: origin.y + inner.height >= map.height(),
    };
    draw_border(buf, area, " Map ", walls);

    // Normally the view fits the map, but if the terminal grew since the app
    // last fitted its view, stop at the wall rather than read past it.
    let cols = inner.width.min(map.width() - origin.x);
    let rows = inner.height.min(map.height() - origin.y);
    let destination = heading_for(app, world).filter(|_| app.flash_on());
    let attended = attended_by(app, world);
    for row in 0..rows {
        for col in 0..cols {
            let pos = Pos {
                x: origin.x + col,
                y: origin.y + row,
            };
            // A sprite is drawn over any item on its tile.
            let glyph = if let Some(sprite) = world.sprite_at(pos) {
                let tile = if app.selection() == Some(Selection::Living(sprite.id())) {
                    SemanticTile::SelectedSprite
                } else {
                    SemanticTile::Sprite
                };
                app.theme.glyph(tile)
            } else if destination == Some(pos) {
                app.theme.glyph(SemanticTile::DecisionMarker)
            } else if let Some(object) = world.object_at(pos) {
                app.theme
                    .object_glyph(object.type_name(), object.visual_state())
            } else {
                app.theme.glyph(SemanticTile::Terrain(map.terrain(pos)))
            };
            let mut style = Style::default().fg(glyph.fg);
            if glyph.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if glyph.reversed || pos == app.cursor() {
                style = style.add_modifier(Modifier::REVERSED);
            }
            if attended == Some(pos) {
                style = style.bg(app.theme.attention_marker());
            }
            buf[(inner.x + col, inner.y + row)]
                .set_char(glyph.symbol)
                .set_style(style);
        }
    }
    draw_cursor(buf, inner, app);
}

/// Where the selected sprite is heading, while its action is under way
/// (design §6.1): where the map flashes the Decision marker.
fn heading_for(app: &App, world: &World) -> Option<Pos> {
    let Some(Selection::Living(id)) = app.selection() else {
        return None;
    };
    let action = world.sprite(id)?.action()?;
    let under_way = !matches!(action.progress, Progress::Ended(_));
    under_way.then_some(action.destination).flatten()
}

/// The tile of the one thing the selected sprite attends to (design §5.3):
/// where the map shades the Attention marker.
fn attended_by(app: &App, world: &World) -> Option<Pos> {
    let Some(Selection::Living(id)) = app.selection() else {
        return None;
    };
    world.sprite(id)?.attending_to()
}

/// Draws the 3×3 cursor around its target tile, which the tile loop has already
/// drawn in reverse video. The arrows and marks take the mode mark's colour. The
/// cursor is drawn only while its target is in view, and pieces outside the map
/// view's tiles are left off.
///
/// ```text
/// M ↓ Y      M: the mode mark
/// → ☺ ←      Y, N: the status marks
/// N ↑ M
/// ```
fn draw_cursor(buf: &mut Buffer, tiles: Rect, app: &App) {
    let Some(centre) = app.cell_of(app.cursor()) else {
        return;
    };
    let (arrows, status) = (app.theme.arrows(), app.theme.status_marks());
    let mark = app.theme.mode_mark(app.mode());
    let pieces = [
        (-1, -1, mark.symbol),
        (0, -1, arrows.down),
        (1, -1, status.idle),
        (-1, 0, arrows.right),
        (1, 0, arrows.left),
        (-1, 1, status.idle),
        (0, 1, arrows.up),
        (1, 1, mark.symbol),
    ];
    for (dx, dy, glyph) in pieces {
        let x = centre.x.checked_add_signed(dx);
        let y = centre.y.checked_add_signed(dy);
        if let Some((x, y)) = x.zip(y)
            && tiles.contains(Position::new(x, y))
        {
            buf[(x, y)]
                .set_char(glyph)
                .set_style(Style::default().fg(mark.fg));
        }
    }
}

/// Which sides of the map view's border are the terrarium's wall.
struct Sides {
    left: bool,
    right: bool,
    top: bool,
    bottom: bool,
}

/// Draws a box around `area` with `title` in its top edge. Sides that are the
/// wall use double lines; the others single lines.
fn draw_border(buf: &mut Buffer, area: Rect, title: &str, walls: Sides) {
    let horizontal = |wall| if wall { "═" } else { "─" };
    let vertical = |wall| if wall { "║" } else { "│" };
    // Corners by (horizontal side is the wall, vertical side is the wall).
    let corner = |h, v, [both, h_only, v_only, neither]: [&'static str; 4]| match (h, v) {
        (true, true) => both,
        (true, false) => h_only,
        (false, true) => v_only,
        (false, false) => neither,
    };
    let (left, right) = (area.left(), area.right() - 1);
    let (top, bottom) = (area.top(), area.bottom() - 1);
    for x in left + 1..right {
        buf[(x, top)].set_symbol(horizontal(walls.top));
        buf[(x, bottom)].set_symbol(horizontal(walls.bottom));
    }
    for y in top + 1..bottom {
        buf[(left, y)].set_symbol(vertical(walls.left));
        buf[(right, y)].set_symbol(vertical(walls.right));
    }
    buf[(left, top)].set_symbol(corner(walls.top, walls.left, ["╔", "╒", "╓", "┌"]));
    buf[(right, top)].set_symbol(corner(walls.top, walls.right, ["╗", "╕", "╖", "┐"]));
    buf[(left, bottom)].set_symbol(corner(walls.bottom, walls.left, ["╚", "╘", "╙", "└"]));
    buf[(right, bottom)].set_symbol(corner(walls.bottom, walls.right, ["╝", "╛", "╜", "┘"]));
    // The title sits one cell in from the corner, as in "┌─ Map ──".
    let room = usize::from(area.width.saturating_sub(3));
    buf.set_stringn(left + 2, top, title, room, Style::default());
}

fn top_bar_line(app: &App, world: &World) -> Line<'static> {
    let clock = &app.clock;
    let time = if clock.is_paused() {
        "|| paused".to_string()
    } else {
        format!("► {}", speed_label(clock.speed()))
    };
    let text = format!(
        " Terra Sprites │ tick {} │ {time} │ seed {} │ sprites {}",
        group_thousands(world.tick()),
        app.seed,
        world.sprites().count()
    );
    Line::from(text).style(Style::default().add_modifier(Modifier::REVERSED))
}

/// The keys that work now, shown at the right of the status line when there is room.
const KEY_HINTS: &str = "WASD scroll  space pause  . step  +/- speed  esc quit ";

/// The tile under the cursor, with any sprite and object on it, and the
/// cursor mode, then key hints if they fit in
/// `width` cells. An open prompt takes the line over.
fn status_line(app: &App, world: &World, width: u16) -> Line<'static> {
    if app.screen() == Screen::QuitPrompt {
        return Line::from(" Quit? (y/n)");
    }
    let cursor = app.cursor();
    let terrain = terrain_name(world.map().terrain(cursor));
    let sprite = world
        .sprite_at(cursor)
        .map(|sprite| format!(" · {}", sprite_label(sprite.id())))
        .unwrap_or_default();
    let object = world
        .object_at(cursor)
        .map(|object| format!(" · {}", object_label(&object)))
        .unwrap_or_default();
    let mode = app.mode().label();
    let tile = format!(
        " ({},{}) {terrain}{sprite}{object} │ {mode}",
        cursor.x, cursor.y
    );
    let used = tile.chars().count() + KEY_HINTS.chars().count();
    match usize::from(width).checked_sub(used) {
        Some(gap) if gap >= 2 => Line::from(format!("{tile}{}{KEY_HINTS}", " ".repeat(gap))),
        _ => Line::from(tile),
    }
}

/// An object's display name, with its stage if its type has stages: `berry bush (mature)`.
fn object_label(object: &ObjectView) -> String {
    let name = display_name(object.type_name());
    match object.stage() {
        Some(stage) => format!("{name} ({stage})"),
        None => name,
    }
}

/// Draws the inspector (design §6.1): its title, and the open tab inside its border.
fn render_inspector(buf: &mut Buffer, area: Rect, app: &App, world: &World) {
    let no_walls = Sides {
        left: false,
        right: false,
        top: false,
        bottom: false,
    };
    draw_border(buf, area, &inspector::title(app), no_walls);
    let inner = area.inner(Margin::new(1, 1));
    let lines = inspector::lines(app, world);
    let first = first_shown(app.tab_scroll(), lines.len(), usize::from(inner.height));
    let shown = lines.iter().skip(first);
    for (row, line) in (inner.y..inner.bottom()).zip(shown) {
        buf.set_line(inner.x, row, line, inner.width);
    }
}

/// Draws the event log (design §6.1): the latest events, newest first.
fn render_event_log(buf: &mut Buffer, area: Rect, app: &App, world: &World) {
    let no_walls = Sides {
        left: false,
        right: false,
        top: false,
        bottom: false,
    };
    draw_border(buf, area, " Events ", no_walls);
    let inner = area.inner(Margin::new(1, 1));
    let lines = app
        .event_log()
        .filter_map(|event| Some((event.tick, event_text(event, world.data())?)));
    for (row, (tick, text)) in (inner.y..inner.bottom()).zip(lines) {
        let line = format!(" {:>7}  {text}", group_thousands(tick));
        buf.set_stringn(
            inner.x,
            row,
            line,
            usize::from(inner.width),
            Style::default(),
        );
    }
}

/// What an event says in the event log, if the log shows it.
fn event_text(event: &Event, data: &DataPack) -> Option<String> {
    match &event.kind {
        EventKind::Died { id, cause, age } => Some(format!(
            "{} died ({}, age {})",
            sprite_label(*id),
            cause_name(*cause, data),
            group_thousands(*age)
        )),
        EventKind::ActionEnded { id, action, .. } => inspector::logged_line(*id, action, data),
        EventKind::ObjectSpawned { .. }
        | EventKind::ObjectRemoved { .. }
        | EventKind::ActionStarted { .. } => None,
    }
}

fn terrain_name(terrain: Terrain) -> &'static str {
    match terrain {
        Terrain::Grass => "grass",
        Terrain::Dirt => "dirt",
        Terrain::Sand => "sand",
        Terrain::ShallowWater => "shallow water",
        Terrain::DeepWater => "deep water",
        Terrain::Rock => "rock",
    }
}

fn speed_label(speed: Speed) -> &'static str {
    match speed {
        Speed::Eighth => "1/8x",
        Speed::Quarter => "1/4x",
        Speed::Half => "1/2x",
        Speed::X1 => "1x",
        Speed::X2 => "2x",
        Speed::X4 => "4x",
        Speed::X8 => "8x",
        Speed::X16 => "16x",
        Speed::Max => "Max",
    }
}
