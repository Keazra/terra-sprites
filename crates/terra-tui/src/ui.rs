//! Draws the screen. Rendering is a pure function of the app state (design §6.8).

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::Line,
};
use terra_sim::World;

use crate::clock::{Clock, Speed};

/// Draws one frame: the top bar, and (from slice 2) the map beneath it.
pub fn render(frame: &mut Frame, world: &World, clock: &Clock) {
    let [top_bar, _map] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(frame.area());
    frame.render_widget(top_bar_line(world, clock), top_bar);
}

fn top_bar_line(world: &World, clock: &Clock) -> Line<'static> {
    let time = if clock.is_paused() {
        "|| paused".to_string()
    } else {
        format!("► {}", speed_label(clock.speed()))
    };
    let text = format!(
        " Terra Sprites │ tick {} │ {time}",
        group_thousands(world.tick())
    );
    Line::from(text).style(Style::default().add_modifier(Modifier::REVERSED))
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
