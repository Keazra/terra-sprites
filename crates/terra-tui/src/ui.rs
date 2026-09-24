//! Draws the screen. Rendering is a pure function of the app state (design §6.8).

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Position, Rect, Size},
    style::{Modifier, Style},
    text::Line,
};
use terra_sim::{Map, Pos, Terrain, World};

use crate::app::App;
use crate::clock::Speed;
use crate::theme::SemanticTile;

/// Draws one frame: the top bar, the map view and the status line.
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
        world.map(),
    );
    frame.render_widget(status_line(app, world.map(), status.width), status);
}

/// How many tiles the map view shows on a screen of `screen` cells: all the
/// space between the top bar and the status line, inside the frame, but no
/// more than the map itself.
pub fn map_view_size(screen: Size, map: &Map) -> Size {
    tile_area(screen.into(), map).as_size()
}

/// The tile drawn at screen cell `(column, row)`, or `None` if no tile is drawn there.
pub fn tile_at(screen: Size, app: &App, map: &Map, column: u16, row: u16) -> Option<Pos> {
    let tiles = tile_area(screen.into(), map);
    tiles.contains(Position::new(column, row)).then(|| Pos {
        x: app.viewport().x + (column - tiles.x),
        y: app.viewport().y + (row - tiles.y),
    })
}

/// The screen cells inside the map view's border, where tiles are drawn.
fn tile_area(screen: Rect, map: &Map) -> Rect {
    map_view_area(screen, map).inner(Margin::new(1, 1))
}

/// The map view, border included: below the top bar, at the left, shrunk to fit a small map.
fn map_view_area(screen: Rect, map: &Map) -> Rect {
    let width = (map.width().saturating_add(2)).min(screen.width);
    let height = (map.height().saturating_add(2)).min(screen.height.saturating_sub(2));
    Rect::new(screen.x, screen.y + 1.min(screen.height), width, height)
}

fn render_map_view(buf: &mut Buffer, area: Rect, app: &App, map: &Map) {
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

    for row in 0..inner.height {
        for col in 0..inner.width {
            let pos = Pos {
                x: origin.x + col,
                y: origin.y + row,
            };
            let glyph = app.theme.glyph(SemanticTile::Terrain(map.terrain(pos)));
            let mut style = Style::default().fg(glyph.fg);
            if pos == app.cursor() {
                style = style.add_modifier(Modifier::REVERSED);
            }
            buf[(inner.x + col, inner.y + row)]
                .set_char(glyph.symbol)
                .set_style(style);
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
        " Terra Sprites │ tick {} │ {time} │ seed {}",
        group_thousands(world.tick()),
        app.seed
    );
    Line::from(text).style(Style::default().add_modifier(Modifier::REVERSED))
}

/// The keys that work now, shown at the right of the status line when there is room.
const KEY_HINTS: &str = "arrows/hjkl move  space pause  . step  +/- speed  q quit ";

/// The tile under the cursor, then key hints if they fit in `width` cells.
fn status_line(app: &App, map: &Map, width: u16) -> Line<'static> {
    let cursor = app.cursor();
    let terrain = terrain_name(map.terrain(cursor));
    let tile = format!(" ({},{}) {terrain}", cursor.x, cursor.y);
    let used = tile.chars().count() + KEY_HINTS.chars().count();
    match usize::from(width).checked_sub(used) {
        Some(gap) if gap >= 2 => Line::from(format!("{tile}{}{KEY_HINTS}", " ".repeat(gap))),
        _ => Line::from(tile),
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

/// `1234567` → `"1,234,567"`.
fn group_thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
