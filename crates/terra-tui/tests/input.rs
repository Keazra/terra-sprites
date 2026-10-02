use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode};
use ratatui::layout::Position;
use terra_tui::app::CursorMode;
use terra_tui::input::{Action, Button, Keys};

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn kind(code: KeyCode, kind: KeyEventKind) -> KeyEvent {
    KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind)
}

#[test]
fn time_control_keys_map_to_their_actions() {
    let cases = [
        (press(KeyCode::Char(' ')), Some(Action::TogglePause)),
        (press(KeyCode::Char('.')), Some(Action::StepOnce)),
        (
            press(KeyCode::Char('+')),
            Some(Action::Faster { held: false }),
        ),
        (
            press(KeyCode::Char('=')),
            Some(Action::Faster { held: false }),
        ), // `+` without Shift
        (
            press(KeyCode::Char('-')),
            Some(Action::Slower { held: false }),
        ),
        (
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            Some(Action::Quit),
        ),
    ];
    for (key, expected) in cases {
        assert_eq!(Keys::new().action_for(key), expected, "{key:?}");
    }
}

#[test]
fn key_releases_are_ignored() {
    // Windows reports releases as well as presses; acting on both would double every action.
    let release = kind(KeyCode::Char(' '), KeyEventKind::Release);
    assert_eq!(Keys::new().action_for(release), None);
}

#[test]
fn a_key_the_terminal_reports_as_repeating_is_held() {
    let mut keys = Keys::new();
    assert_eq!(
        keys.action_for(press(KeyCode::Char('-'))),
        Some(Action::Slower { held: false })
    );
    assert_eq!(
        keys.action_for(kind(KeyCode::Char('-'), KeyEventKind::Repeat)),
        Some(Action::Slower { held: true })
    );
}

#[test]
fn where_releases_are_reported_a_second_press_without_a_release_is_held() {
    // Windows reports auto-repeat as more presses, but always reports releases.
    let mut keys = Keys::with_release_reporting(true);
    let minus = KeyCode::Char('-');
    assert_eq!(
        keys.action_for(press(minus)),
        Some(Action::Slower { held: false })
    );
    assert_eq!(
        keys.action_for(press(minus)),
        Some(Action::Slower { held: true })
    );
    assert_eq!(
        keys.action_for(press(minus)),
        Some(Action::Slower { held: true })
    );
    assert_eq!(keys.action_for(kind(minus, KeyEventKind::Release)), None);
    assert_eq!(
        keys.action_for(press(minus)),
        Some(Action::Slower { held: false })
    );
}

#[test]
fn where_releases_are_not_reported_repeated_presses_stay_fresh() {
    // Without releases, a hold can't be told from taps, so never guess "held".
    let mut keys = Keys::with_release_reporting(false);
    for _ in 0..3 {
        assert_eq!(
            keys.action_for(press(KeyCode::Char('-'))),
            Some(Action::Slower { held: false })
        );
    }
}

#[test]
fn seeing_a_release_turns_on_held_detection() {
    let mut keys = Keys::with_release_reporting(false);
    let plus = KeyCode::Char('+');
    keys.action_for(press(plus));
    keys.action_for(kind(plus, KeyEventKind::Release)); // this terminal reports releases after all
    assert_eq!(
        keys.action_for(press(plus)),
        Some(Action::Faster { held: false })
    );
    assert_eq!(
        keys.action_for(press(plus)),
        Some(Action::Faster { held: true })
    );
}

#[test]
fn holding_space_toggles_pause_only_once() {
    let mut keys = Keys::with_release_reporting(true);
    let space = KeyCode::Char(' ');
    assert_eq!(keys.action_for(press(space)), Some(Action::TogglePause));
    assert_eq!(
        keys.action_for(press(space)),
        None,
        "held via a second press"
    );
    assert_eq!(
        keys.action_for(kind(space, KeyEventKind::Repeat)),
        None,
        "held via repeat"
    );
}

#[test]
fn holding_step_keeps_stepping() {
    let mut keys = Keys::with_release_reporting(true);
    let dot = KeyCode::Char('.');
    assert_eq!(keys.action_for(press(dot)), Some(Action::StepOnce));
    assert_eq!(
        keys.action_for(press(dot)),
        Some(Action::StepOnce),
        "held via a second press"
    );
    assert_eq!(
        keys.action_for(kind(dot, KeyEventKind::Repeat)),
        Some(Action::StepOnce),
        "held via repeat"
    );
}

#[test]
fn a_modifier_or_lock_key_on_its_own_does_nothing() {
    // With the kitty keyboard protocol on (design v27 §6.6), the terminal
    // reports Shift and the lock keys as keys of their own. Pressing Shift for
    // a capital `Y` mustn't cancel the quit prompt first.
    let mut keys = Keys::with_release_reporting(true);
    for code in [
        KeyCode::Modifier(ModifierKeyCode::LeftShift),
        KeyCode::Modifier(ModifierKeyCode::RightControl),
        KeyCode::CapsLock,
        KeyCode::NumLock,
        KeyCode::ScrollLock,
    ] {
        assert_eq!(keys.action_for(press(code)), None, "{code:?}");
        assert_eq!(
            keys.action_for(kind(code, KeyEventKind::Release)),
            None,
            "{code:?}"
        );
    }
}

#[test]
fn a_shifted_plus_reported_as_shift_and_equals_is_faster() {
    // The kitty protocol reports the key, `=`, with Shift held.
    let mut keys = Keys::with_release_reporting(true);
    let shifted_equals = KeyEvent::new(KeyCode::Char('='), KeyModifiers::SHIFT);
    assert_eq!(
        keys.action_for(shifted_equals),
        Some(Action::Faster { held: false })
    );
}

#[test]
fn a_plus_released_as_equals_is_still_released() {
    // Releasing Shift before the key makes Windows report the release as `=`.
    let mut keys = Keys::with_release_reporting(true);
    assert_eq!(
        keys.action_for(press(KeyCode::Char('+'))),
        Some(Action::Faster { held: false })
    );
    keys.action_for(kind(KeyCode::Char('='), KeyEventKind::Release));
    assert_eq!(
        keys.action_for(press(KeyCode::Char('+'))),
        Some(Action::Faster { held: false })
    );
}

#[test]
fn wasd_and_the_arrows_scroll_the_viewport_one_tile() {
    let one = |dx, dy| Some(Action::Scroll { dx, dy });
    let cases = [
        (KeyCode::Char('w'), one(0, -1)),
        (KeyCode::Char('a'), one(-1, 0)),
        (KeyCode::Char('s'), one(0, 1)),
        (KeyCode::Char('d'), one(1, 0)),
        (KeyCode::Up, one(0, -1)),
        (KeyCode::Left, one(-1, 0)),
        (KeyCode::Down, one(0, 1)),
        (KeyCode::Right, one(1, 0)),
    ];
    for (code, expected) in cases {
        assert_eq!(Keys::new().action_for(press(code)), expected, "{code:?}");
    }
}

#[test]
fn shift_scrolls_five_tiles() {
    let five = |dx, dy| Some(Action::Scroll { dx, dy });
    let shifted = |code| KeyEvent::new(code, KeyModifiers::SHIFT);
    let cases = [
        (shifted(KeyCode::Up), five(0, -5)),
        (shifted(KeyCode::Left), five(-5, 0)),
        (shifted(KeyCode::Down), five(0, 5)),
        (shifted(KeyCode::Right), five(5, 0)),
        // Terminals report Shift+w as `W` with the Shift modifier.
        (shifted(KeyCode::Char('W')), five(0, -5)),
        (shifted(KeyCode::Char('A')), five(-5, 0)),
        (shifted(KeyCode::Char('S')), five(0, 5)),
        (shifted(KeyCode::Char('D')), five(5, 0)),
    ];
    for (key, expected) in cases {
        assert_eq!(Keys::new().action_for(key), expected, "{key:?}");
    }
}

#[test]
fn v_switches_the_detail_view_on_a_fresh_press_only() {
    let mut keys = Keys::with_release_reporting(true);
    let v = KeyCode::Char('v');
    assert_eq!(keys.action_for(press(v)), Some(Action::ToggleDetail));
    assert_eq!(keys.action_for(kind(v, KeyEventKind::Repeat)), None);
}

#[test]
fn holding_a_scroll_key_keeps_scrolling() {
    let mut keys = Keys::with_release_reporting(true);
    let right = Some(Action::Scroll { dx: 1, dy: 0 });
    assert_eq!(keys.action_for(press(KeyCode::Char('d'))), right);
    assert_eq!(
        keys.action_for(press(KeyCode::Char('d'))),
        right,
        "held via a second press"
    );
    assert_eq!(
        keys.action_for(kind(KeyCode::Char('d'), KeyEventKind::Repeat)),
        right,
        "held via repeat"
    );
}

#[test]
fn escape_and_y_answer_the_quit_prompt_and_q_no_longer_quits() {
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Esc)),
        Some(Action::Back)
    );
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Char('y'))),
        Some(Action::Confirm)
    );
    // `q` is the left click now (design v21 §6.5).
    assert_ne!(
        Keys::new().action_for(press(KeyCode::Char('q'))),
        Some(Action::Quit)
    );
}

#[test]
fn z_x_and_c_pick_select_train_and_grab() {
    // Design v21 §6.5, v23.
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Char('z'))),
        Some(Action::Mode(CursorMode::Select))
    );
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Char('x'))),
        Some(Action::Mode(CursorMode::Train))
    );
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Char('c'))),
        Some(Action::Mode(CursorMode::Grab))
    );
}

#[test]
fn q_and_e_are_the_left_and_right_click_and_shift_amplifies_them() {
    // Design v21 §6.5.
    let press_of = |key: KeyEvent| Keys::new().action_for(key);
    let pressed = |button, amplified| Some(Action::Press { button, amplified });
    assert_eq!(
        press_of(press(KeyCode::Char('q'))),
        pressed(Button::Left, false)
    );
    assert_eq!(
        press_of(press(KeyCode::Char('e'))),
        pressed(Button::Right, false)
    );
    let shifted = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT);
    assert_eq!(press_of(shifted('Q')), pressed(Button::Left, true));
    assert_eq!(press_of(shifted('E')), pressed(Button::Right, true));
    // With Caps Lock on, Windows reports `Q` without Shift: not amplified.
    assert_eq!(
        press_of(press(KeyCode::Char('Q'))),
        pressed(Button::Left, false)
    );
}

#[test]
fn f_is_follow_and_acts_once_per_press() {
    // Design v26 §6.5: `F` turns Follow on or off where the Cursor is.
    let mut keys = Keys::with_release_reporting(true);
    let follow = Some(Action::Follow { at: None });
    assert_eq!(keys.action_for(press(KeyCode::Char('f'))), follow);
    assert_eq!(
        keys.action_for(press(KeyCode::Char('f'))),
        None,
        "held, it doesn't flicker"
    );
    assert_eq!(Keys::new().action_for(press(KeyCode::Char('F'))), follow);
}

#[test]
fn holding_q_or_e_acts_once() {
    // Design v21 §6.5: once per press, like a click.
    for c in ['q', 'e'] {
        let mut keys = Keys::with_release_reporting(true);
        assert!(keys.action_for(press(KeyCode::Char(c))).is_some());
        assert_eq!(
            keys.action_for(kind(KeyCode::Char(c), KeyEventKind::Repeat)),
            None
        );
        assert_eq!(
            keys.action_for(press(KeyCode::Char(c))),
            None,
            "pressed again, unreleased"
        );
    }
}

#[test]
fn holding_escape_counts_once_so_it_cannot_confirm_its_own_prompt() {
    let mut keys = Keys::with_release_reporting(true);
    assert_eq!(keys.action_for(press(KeyCode::Esc)), Some(Action::Back));
    assert_eq!(
        keys.action_for(press(KeyCode::Esc)),
        None,
        "held via a second press"
    );
    assert_eq!(
        keys.action_for(kind(KeyCode::Esc, KeyEventKind::Repeat)),
        None,
        "held via repeat"
    );
}

#[test]
fn any_other_key_is_reported_so_it_can_cancel_a_prompt() {
    for code in [
        KeyCode::Char('k'),
        KeyCode::Char('n'),
        KeyCode::Char('0'),
        KeyCode::F(5),
    ] {
        assert_eq!(
            Keys::new().action_for(press(code)),
            Some(Action::Dismiss),
            "{code:?}"
        );
    }
}

#[test]
fn every_mouse_event_points_and_a_left_press_also_clicks() {
    use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let mouse = |kind| MouseEvent {
        kind,
        column: 12,
        row: 7,
        modifiers: KeyModifiers::NONE,
    };
    let action = |kind| terra_tui::input::mouse_action(mouse(kind));
    let point = Some(Action::Point(Position::new(12, 7)));
    assert_eq!(action(MouseEventKind::Moved), point);
    assert_eq!(action(MouseEventKind::Drag(MouseButton::Left)), point);
    assert_eq!(
        action(MouseEventKind::Down(MouseButton::Left)),
        Some(Action::left_click(Position::new(12, 7)))
    );
    assert_eq!(
        action(MouseEventKind::Down(MouseButton::Right)),
        Some(Action::Click {
            at: Position::new(12, 7),
            button: Button::Right,
            amplified: false
        }),
        "design v21 §6.5"
    );
    // Letting go of the right button sends an aimed throw or shove (design
    // v25 §6.5); dragging it pulls, as any move points.
    assert_eq!(
        action(MouseEventKind::Up(MouseButton::Right)),
        Some(Action::Release {
            button: Button::Right,
            at: Some(Position::new(12, 7)),
        })
    );
    assert_eq!(action(MouseEventKind::Drag(MouseButton::Right)), point);
    // The middle button does what `F` does, where it points (design v26
    // §6.5).
    assert_eq!(
        action(MouseEventKind::Down(MouseButton::Middle)),
        Some(Action::Follow {
            at: Some(Position::new(12, 7)),
        })
    );
    // Every mouse event says where the pointer is, so the cursor follows it.
    for kind in [
        MouseEventKind::Up(MouseButton::Left),
        MouseEventKind::Drag(MouseButton::Middle),
        MouseEventKind::Up(MouseButton::Middle),
    ] {
        assert_eq!(action(kind), point, "{kind:?}");
    }
}

#[test]
fn ctrl_amplifies_a_click() {
    // Design v21 §6.5: Windows Terminal keeps Shift+click for selecting text.
    use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let ctrl_click = terra_tui::input::mouse_action(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Right),
        column: 3,
        row: 4,
        modifiers: KeyModifiers::CONTROL,
    });
    assert_eq!(
        ctrl_click,
        Some(Action::Click {
            at: Position::new(3, 4),
            button: Button::Right,
            amplified: true
        })
    );
}

#[test]
fn caps_lock_letters_without_shift_scroll_one_tile() {
    // With Caps Lock on, Windows reports `W` without the Shift modifier.
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Char('W'))),
        Some(Action::Scroll { dx: 0, dy: -1 })
    );
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Char('D'))),
        Some(Action::Scroll { dx: 1, dy: 0 })
    );
}

#[test]
fn a_capital_y_confirms_too() {
    let shifted_y = KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::SHIFT);
    assert_eq!(Keys::new().action_for(shifted_y), Some(Action::Confirm));
}

#[test]
fn ctrl_with_any_key_but_c_dismisses() {
    let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
    assert_eq!(Keys::new().action_for(ctrl('z')), Some(Action::Dismiss));
    assert_eq!(Keys::new().action_for(ctrl('c')), Some(Action::Quit));
}

#[test]
fn tab_and_shift_tab_select_the_next_and_previous_sprite() {
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Tab)),
        Some(Action::SelectNext)
    );
    // Terminals report Shift+Tab as its own key, with Shift held.
    let shift_tab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    assert_eq!(
        Keys::new().action_for(shift_tab),
        Some(Action::SelectPrevious)
    );
}

#[test]
fn the_square_brackets_switch_inspector_tabs() {
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Char(']'))),
        Some(Action::NextTab)
    );
    assert_eq!(
        Keys::new().action_for(press(KeyCode::Char('['))),
        Some(Action::PreviousTab)
    );
}

#[test]
fn page_up_and_page_down_scroll_the_inspector_tab_a_page() {
    assert_eq!(
        Keys::new().action_for(press(KeyCode::PageDown)),
        Some(Action::ScrollTab { pages: 1 })
    );
    assert_eq!(
        Keys::new().action_for(press(KeyCode::PageUp)),
        Some(Action::ScrollTab { pages: -1 })
    );
}

#[test]
fn the_mouse_wheel_turns_notches_where_the_pointer_is() {
    use ratatui::crossterm::event::{MouseEvent, MouseEventKind};
    let wheel = |kind| {
        terra_tui::input::mouse_action(MouseEvent {
            kind,
            column: 70,
            row: 5,
            modifiers: KeyModifiers::NONE,
        })
    };
    let at = Position::new(70, 5);
    assert_eq!(
        wheel(MouseEventKind::ScrollDown),
        Some(Action::Wheel { at, notches: 1 })
    );
    assert_eq!(
        wheel(MouseEventKind::ScrollUp),
        Some(Action::Wheel { at, notches: -1 })
    );
}

#[test]
fn tab_with_shift_held_selects_the_previous_sprite_too() {
    // Some terminals report Shift+Tab as Tab with Shift, not as its own key.
    let shift_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
    assert_eq!(
        Keys::new().action_for(shift_tab),
        Some(Action::SelectPrevious)
    );
}

#[test]
fn letting_go_of_e_lets_go_of_the_right_button_and_other_releases_do_nothing() {
    // Design v25 §6.5: holding `E` aims a throw or a shove, as holding the
    // right button does, and letting it go sends it.
    let mut keys = Keys::new();
    keys.action_for(press(KeyCode::Char('e')));
    let let_go = Action::Release {
        button: Button::Right,
        at: None,
    };
    let release = kind(KeyCode::Char('e'), KeyEventKind::Release);
    assert_eq!(keys.action_for(release), Some(let_go));
    keys.action_for(press(KeyCode::Char('q')));
    let release = kind(KeyCode::Char('q'), KeyEventKind::Release);
    assert_eq!(keys.action_for(release), None);
}

#[test]
fn the_place_menu_and_naming_keys_have_their_actions() {
    for (code, action) in [
        (KeyCode::Char('1'), Action::Pick(1)),
        (KeyCode::Char('9'), Action::Pick(9)),
        (KeyCode::Enter, Action::Enter),
        (KeyCode::Char('r'), Action::Rename),
        (KeyCode::Char('g'), Action::ExportGenome),
    ] {
        assert_eq!(
            Keys::new().action_for(press(code)),
            Some(action),
            "{code:?}"
        );
    }
}
