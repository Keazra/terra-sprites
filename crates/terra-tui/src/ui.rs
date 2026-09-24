//! Draws the screen. Rendering is a pure function of the app state (design §6.8).

use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Position, Rect, Size},
    style::{Modifier, Style},
    text::Line,
};
use terra_sim::{Map, ObjectView, Pos, Terrain, World};

use crate::app::{App, Screen};
use crate::clock::Speed;
use crate::theme::SemanticTile;

/// The inspector's width, in columns, border included (design §6.1).
const INSPECTOR_WIDTH: u16 = 46;
/// The narrowest terminal that has room for the inspector beside the map view.
const INSPECTOR_FROM: u16 = 100;

/// Draws one frame: the top bar, the map view, the inspector if there's room,
/// and the status line.
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
        render_world_tab(frame.buffer_mut(), inspector, world);
    }
    frame.render_widget(status_line(app, world, status.width), status);
}

/// Where the map view draws its tiles on a screen of `screen` cells: inside
/// its border, between the top bar and the status line, and no bigger than
/// the map itself.
pub fn tile_area(screen: Size, map: &Map) -> Rect {
    map_view_area(screen.into(), map).inner(Margin::new(1, 1))
}

/// The map view, border included: below the top bar, at the left, shrunk to
/// fit a small map, and leaving room for the inspector when there is some.
fn map_view_area(screen: Rect, map: &Map) -> Rect {
    let room = match inspector_area(screen) {
        Some(inspector) => inspector.x - screen.x,
        None => screen.width,
    };
    let width = (map.width().saturating_add(2)).min(room);
    let height = (map.height().saturating_add(2)).min(screen.height.saturating_sub(2));
    Rect::new(screen.x, screen.y + 1.min(screen.height), width, height)
}

/// The inspector, border included: at the right, between the top bar and the
/// status line, on a terminal wide enough for it.
fn inspector_area(screen: Rect) -> Option<Rect> {
    (screen.width >= INSPECTOR_FROM && screen.height > 2).then(|| {
        Rect::new(
            screen.right() - INSPECTOR_WIDTH,
            screen.y + 1,
            INSPECTOR_WIDTH,
            screen.height - 2,
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
    for row in 0..rows {
        for col in 0..cols {
            let pos = Pos {
                x: origin.x + col,
                y: origin.y + row,
            };
            let glyph = match world.object_at(pos) {
                Some(object) => app
                    .theme
                    .object_glyph(object.type_name(), object.visual_state()),
                None => app.theme.glyph(SemanticTile::Terrain(map.terrain(pos))),
            };
            let mut style = Style::default().fg(glyph.fg);
            if glyph.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if pos == app.cursor() {
                style = style.add_modifier(Modifier::REVERSED);
            }
            buf[(inner.x + col, inner.y + row)]
                .set_char(glyph.symbol)
                .set_style(style);
        }
    }
    draw_cursor(buf, inner, app);
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
    let mut text = format!(
        " Terra Sprites │ tick {} │ {time} │ seed {}",
        group_thousands(world.tick()),
        app.seed
    );
    // The food counts (design §6.1), for packs that have these types.
    for (label, object_type) in [("bushes", "berry_bush"), ("berries", "berry")] {
        if world
            .data()
            .object_type_names()
            .any(|name| name == object_type)
        {
            let count = world
                .objects()
                .filter(|o| o.type_name() == object_type)
                .count();
            text += &format!(" │ {label} {}", group_thousands(count as u64));
        }
    }
    Line::from(text).style(Style::default().add_modifier(Modifier::REVERSED))
}

/// The keys that work now, shown at the right of the status line when there is room.
const KEY_HINTS: &str = "WASD scroll  space pause  . step  +/- speed  esc quit ";

/// The tile under the cursor and the cursor mode, then key hints if they fit in
/// `width` cells. An open prompt takes the line over.
fn status_line(app: &App, world: &World, width: u16) -> Line<'static> {
    if app.screen() == Screen::QuitPrompt {
        return Line::from(" Quit? (y/n)");
    }
    let cursor = app.cursor();
    let terrain = terrain_name(world.map().terrain(cursor));
    let object = world
        .object_at(cursor)
        .map(|object| format!(" · {}", object_label(&object)))
        .unwrap_or_default();
    let mode = app.mode().label();
    let tile = format!(" ({},{}) {terrain}{object} │ {mode}", cursor.x, cursor.y);
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

/// A name from the data, as shown on screen: `berry_bush` → `berry bush`.
fn display_name(name: &str) -> String {
    name.replace('_', " ")
}

/// Draws the World tab (design §6.1): the data pack, then each object type
/// with its count, the count in each stage (for a type with more than one)
/// and the total of each counter.
fn render_world_tab(buf: &mut Buffer, area: Rect, world: &World) {
    let no_walls = Sides {
        left: false,
        right: false,
        top: false,
        bottom: false,
    };
    draw_border(buf, area, " World ", no_walls);
    let data = world.data();
    let mut lines = vec![format!(
        " {:<12}{} v{}",
        "data pack",
        data.name(),
        data.version()
    )];
    for object_type in data.object_type_names() {
        let objects: Vec<ObjectView> = world
            .objects()
            .filter(|o| o.type_name() == object_type)
            .collect();
        lines.push(format!(
            " {:<12}{:>7}",
            display_name(object_type),
            group_thousands(objects.len() as u64)
        ));
        let stages = data.stage_names(object_type);
        if stages.len() > 1 {
            let counts: Vec<String> = stages
                .iter()
                .map(|&stage| {
                    let n = objects.iter().filter(|o| o.stage() == Some(stage)).count();
                    format!("{stage} {}", group_thousands(n as u64))
                })
                .collect();
            lines.push(format!("   {}", counts.join(" · ")));
        }
        let totals: Vec<String> = data
            .counter_names(object_type)
            .iter()
            .map(|&counter| {
                let total: u64 = objects
                    .iter()
                    .map(|o| u64::from(o.counter(counter).unwrap_or(0)))
                    .sum();
                format!("{counter} {}", group_thousands(total))
            })
            .collect();
        if !totals.is_empty() {
            lines.push(format!("   {}", totals.join(" · ")));
        }
    }
    let inner = area.inner(Margin::new(1, 1));
    for (row, line) in (inner.y..inner.bottom()).zip(&lines) {
        buf.set_stringn(
            inner.x,
            row,
            line,
            usize::from(inner.width),
            Style::default(),
        );
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
