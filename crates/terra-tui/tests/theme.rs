mod common;
use common::{DRIVE_COLOURS, pack};

use ratatui::style::Color;
use terra_sim::Terrain;
use terra_tui::app::{CursorMode, StatusMark};
use terra_tui::cp437;
use terra_tui::space::{Depth, Pane};
use terra_tui::theme::{SemanticTile, Theme};

#[test]
fn the_themes_draw_terrain_as_the_design_table_says() {
    // Design §6.2. Terminals have no brown, so dirt is dark yellow.
    let expected = [
        (Terrain::Grass, '.', '.', Color::Green),
        (Terrain::Dirt, ',', ',', Color::Yellow),
        (Terrain::Sand, ':', ':', Color::LightYellow),
        (Terrain::ShallowWater, '~', '~', Color::Cyan),
        (Terrain::DeepWater, '≈', '=', Color::Blue),
        (Terrain::Rock, '#', '#', Color::Gray),
    ];
    let (cp437_theme, ascii_theme) = (Theme::cp437(), Theme::ascii());
    for (terrain, cp437_glyph, ascii_glyph, colour) in expected {
        let tile = SemanticTile::Terrain(terrain);
        let (a, b) = (cp437_theme.glyph(tile), ascii_theme.glyph(tile));
        assert_eq!((a.symbol, a.fg), (cp437_glyph, colour), "cp437 {terrain:?}");
        assert_eq!((b.symbol, b.fg), (ascii_glyph, colour), "ascii {terrain:?}");
    }
}

/// Every glyph a theme draws the map with, with the kind of thing it stands
/// for: a terrain, or an object type in any of its visual states.
fn map_glyphs(theme: &Theme) -> Vec<(char, String)> {
    let pack = pack();
    // The selected sprite, and a young one, are still sprites, so they may
    // share the sprite's glyph.
    let kind = |tile: SemanticTile| match tile {
        SemanticTile::SelectedSprite
        | SemanticTile::YoungSprite
        | SemanticTile::SelectedYoungSprite => format!("{:?}", SemanticTile::Sprite),
        tile => format!("{tile:?}"),
    };
    let optional = SemanticTile::OPTIONAL.map(|(tile, _)| tile);
    let terrain = SemanticTile::ALL
        .iter()
        .chain(&optional)
        .map(|&tile| (theme.glyph(tile).symbol, kind(tile)));
    let objects = pack.object_type_names().flat_map(|name| {
        pack.visual_states(name)
            .into_iter()
            .map(move |state| (theme.object_glyph(name, state).symbol, name.to_string()))
    });
    terrain.chain(objects).collect()
}

#[test]
fn every_terrain_and_object_type_has_its_own_glyphs_in_each_theme() {
    // Design §6.2: colour marks state; the glyph says what kind of thing it is.
    for (name, theme) in [("cp437", Theme::cp437()), ("ascii", Theme::ascii())] {
        let glyphs = map_glyphs(&theme);
        for (symbol, kind) in &glyphs {
            for (other_symbol, other_kind) in &glyphs {
                assert!(
                    symbol != other_symbol || kind == other_kind,
                    "{name} draws both {kind} and {other_kind} as {symbol:?}"
                );
            }
        }
    }
}

#[test]
fn theme_glyphs_are_cp437_and_the_ascii_themes_are_plain_ascii() {
    for (symbol, kind) in map_glyphs(&Theme::cp437()) {
        assert!(cp437::contains(symbol), "{symbol:?} for {kind}");
    }
    for (symbol, kind) in map_glyphs(&Theme::ascii()) {
        assert!(symbol.is_ascii_graphic(), "{symbol:?} for {kind}");
    }
}

#[test]
fn the_themes_draw_objects_as_the_design_table_says() {
    // Design §6.2: (object, visual state, cp437, ascii, colour, bold).
    let expected = [
        ("berry_bush", "seedling", '\'', '\'', Color::Green, false),
        ("berry_bush", "default", '♣', '&', Color::Green, false),
        ("berry_bush", "fruiting", '♣', '&', Color::Red, true),
        ("thornbush", "default", '♠', '*', Color::Magenta, false),
        ("berry", "default", '•', '%', Color::Red, false),
        ("ball", "default", '○', 'o', Color::White, false),
        // M2 design §10.7.
        ("egg", "default", 'o', '0', Color::LightYellow, false),
    ];
    let (cp437_theme, ascii_theme) = (Theme::cp437(), Theme::ascii());
    for (object, state, cp437_glyph, ascii_glyph, colour, bold) in expected {
        let (a, b) = (
            cp437_theme.object_glyph(object, state),
            ascii_theme.object_glyph(object, state),
        );
        assert_eq!(
            (a.symbol, a.fg, a.bold),
            (cp437_glyph, colour, bold),
            "cp437 {object} {state}"
        );
        assert_eq!(
            (b.symbol, b.fg, b.bold),
            (ascii_glyph, colour, bold),
            "ascii {object} {state}"
        );
    }
}

#[test]
fn each_theme_covers_every_visual_state_of_the_built_in_objects() {
    let pack = pack();
    for (name, theme) in [("cp437", Theme::cp437()), ("ascii", Theme::ascii())] {
        for object in pack.object_type_names() {
            for state in pack.visual_states(object) {
                assert!(
                    theme.object_entry(object, state).is_some(),
                    "{name} has no glyph for {object} ({state})"
                );
            }
        }
    }
}

#[test]
fn a_theme_falls_back_to_the_objects_default_look_then_to_a_question_mark() {
    let theme = Theme::cp437();
    assert_eq!(
        theme.object_glyph("berry", "squashed"),
        theme.object_glyph("berry", "default")
    );
    assert_eq!(theme.object_glyph("shrub", "default").symbol, '?');
}

#[test]
fn cp437_covers_the_glyphs_the_design_uses_and_nothing_outside_the_code_page() {
    for c in "☺♣♠•○►♥≈═║╔╗╚╝╒╕╘╛╓╖╙╜│─┌┐└┘·∙ az~#".chars()
    {
        assert!(cp437::contains(c), "{c:?} is in CP437");
    }
    for c in "⅛é€✓\u{7}".chars() {
        let expected = c == 'é'; // é is CP437 0x82; the rest are not in the code page
        assert_eq!(cp437::contains(c), expected, "{c:?}");
    }
}

/// A theme's cursor glyphs for Select mode: the four arrows (up, down, left,
/// right), the mode mark and the idle status mark.
fn select_cursor(theme: &Theme) -> [char; 6] {
    let (arrows, status) = (theme.arrows(), theme.status_marks());
    let mark = theme.mode_mark(CursorMode::Select).symbol;
    [
        arrows.up,
        arrows.down,
        arrows.left,
        arrows.right,
        mark,
        status.idle,
    ]
}

#[test]
fn the_themes_draw_the_select_cursor_as_the_design_table_says() {
    // Design §6.2: arrows, the Select mode mark, and the idle status mark.
    assert_eq!(
        select_cursor(&Theme::cp437()),
        ['↑', '↓', '←', '→', '♦', '·']
    );
    assert_eq!(
        select_cursor(&Theme::ascii()),
        ['^', 'v', '<', '>', 'S', '-']
    );
    for glyph in select_cursor(&Theme::cp437()) {
        assert!(cp437::contains(glyph), "{glyph:?}");
    }
}

#[test]
fn grab_mode_is_a_yellow_arch_or_g() {
    // Design §6.2, §6.5.
    let (cp437, ascii) = (Theme::cp437(), Theme::ascii());
    assert_eq!(cp437.mode_mark(CursorMode::Grab).symbol, '∩');
    assert_eq!(ascii.mode_mark(CursorMode::Grab).symbol, 'G');
    for theme in [cp437, ascii] {
        assert_eq!(theme.mode_mark(CursorMode::Grab).fg, Color::Yellow);
    }
}

#[test]
fn the_themes_draw_grab_s_status_marks_as_the_design_table_says() {
    // Design §6.2: grab, empty and release; holding shows the thing itself.
    let marks = |theme: &Theme| {
        let status = theme.status_marks();
        [status.grab, status.empty, status.release]
    };
    assert_eq!(marks(&Theme::cp437()), ['↑', '░', '↓']);
    assert_eq!(marks(&Theme::ascii()), ['^', '_', 'v']);
    for glyph in marks(&Theme::cp437()) {
        assert!(cp437::contains(glyph), "{glyph:?}");
    }
    assert_eq!(
        Theme::cp437().status_marks().glyph(StatusMark::Holding),
        None
    );
}

#[test]
fn the_leash_is_yellow_dots_or_semicolons() {
    // Design §6.2, v23.
    assert_eq!(Theme::cp437().leash().symbol, '·');
    assert_eq!(Theme::ascii().leash().symbol, ';');
    for theme in [Theme::cp437(), Theme::ascii()] {
        assert_eq!(theme.leash().fg, Color::Yellow);
    }
}

#[test]
fn the_select_mode_is_white_in_both_themes() {
    // Design §6.5: the mode mark's colour is the whole cursor's colour.
    for theme in [Theme::cp437(), Theme::ascii()] {
        assert_eq!(theme.mode_mark(CursorMode::Select).fg, Color::White);
    }
}

/// A theme's glyphs for what the Cursor reports and how it looks following
/// a sprite: the status marks (idle, sent, applied, rejected), then the
/// solid arrows (up, down, left, right).
fn reports_and_follow(theme: &Theme) -> [char; 8] {
    let (status, arrows) = (theme.status_marks(), theme.followed_arrows());
    [
        status.idle,
        status.sent,
        status.applied,
        status.rejected,
        arrows.up,
        arrows.down,
        arrows.left,
        arrows.right,
    ]
}

#[test]
fn the_themes_draw_the_status_marks_and_follow_as_the_design_table_says() {
    // Design v22 §6.2.
    assert_eq!(
        reports_and_follow(&Theme::cp437()),
        ['·', '+', '☼', '?', '▲', '▼', '◄', '►']
    );
    assert_eq!(
        reports_and_follow(&Theme::ascii()),
        ['-', '+', '*', '?', '^', 'v', '<', '>']
    );
    for glyph in reports_and_follow(&Theme::cp437()) {
        assert!(cp437::contains(glyph), "{glyph:?}");
    }
}

#[test]
fn the_aim_line_is_yellow_dots_ending_in_a_small_circle_or_semicolons_and_an_o() {
    // Design v25 §6.2: `°` rather than `○`, which is the ball's.
    assert_eq!(Theme::cp437().aim().symbol, '·');
    assert_eq!(Theme::cp437().aim_end().symbol, '°');
    assert_eq!(Theme::ascii().aim().symbol, ';');
    assert_eq!(Theme::ascii().aim_end().symbol, 'O');
    for theme in [Theme::cp437(), Theme::ascii()] {
        assert_eq!(theme.aim().fg, Color::Yellow);
        assert_eq!(theme.aim_end().fg, Color::Yellow);
    }
}

#[test]
fn each_theme_colours_every_drive_of_the_built_in_pack_as_the_design_table_says() {
    // Design §6.3, arousal M2 design §10, and anxiety §15.8.
    let pack = pack();
    let drives: Vec<&str> = pack.drives().collect();
    assert_eq!(drives, DRIVE_COLOURS.map(|(drive, _)| drive));
    for (name, theme) in [("cp437", Theme::cp437()), ("ascii", Theme::ascii())] {
        for (drive, colour) in DRIVE_COLOURS {
            assert_eq!(theme.drive_colour(drive), Some(colour), "{name} {drive}");
        }
        assert_eq!(theme.drive_colour("wanderlust"), None);
    }
}

const ASCII_THEME: &str = include_str!("../../../themes/ascii.ron");

/// The ascii theme's file with `from` replaced by `to`, which must be in it.
fn ascii_with(from: &str, to: &str) -> String {
    assert!(ASCII_THEME.contains(from), "the ascii theme has {from:?}");
    ASCII_THEME.replacen(from, to, 1)
}

/// The ascii theme as one from before the part from `start` on was written,
/// loaded: cut from `start`, which must be in it, to its closing bracket,
/// so `key` is no longer in it.
fn ascii_from_before(start: &str, key: &str) -> Theme {
    let from = ASCII_THEME
        .find(start)
        .unwrap_or_else(|| panic!("the ascii theme has {start:?}"));
    let end = ASCII_THEME.rfind(')').expect("the theme's end");
    let edited = format!("{}{}", &ASCII_THEME[..from], &ASCII_THEME[end..]);
    assert!(!edited.contains(key), "{edited}");
    Theme::from_ron(&edited, &pack()).expect("loads")
}

#[test]
fn a_theme_file_loads_and_draws_as_written() {
    // Design v34 §6.7: `--theme <file>` loads a theme the player has edited.
    let pack = pack();
    let theme = Theme::from_ron(ASCII_THEME, &pack).expect("the ascii theme loads");
    assert_eq!(theme.glyph(SemanticTile::Sprite).symbol, '@');
    let edited = ascii_with(
        "\"ball\":      { \"default\": (glyph: 'o', fg: white) }",
        "\"ball\":      { \"default\": (glyph: 'Q', fg: light_cyan, bold: true) }",
    );
    let theme = Theme::from_ron(&edited, &pack).expect("the edited theme loads");
    let ball = theme.object_glyph("ball", "default");
    assert_eq!(
        (ball.symbol, ball.fg, ball.bold),
        ('Q', Color::LightCyan, true)
    );
}

#[test]
fn both_built_in_themes_load_as_a_theme_file_would() {
    // `--theme` checks a file against the pack too; the built-in themes
    // pass the same checks.
    let pack = pack();
    for (name, text) in [
        ("cp437", include_str!("../../../themes/cp437.ron")),
        ("ascii", ASCII_THEME),
    ] {
        Theme::from_ron(text, &pack).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
fn a_theme_that_is_not_ron_is_refused_saying_where() {
    let error = Theme::from_ron("(tiles: {", &pack()).expect_err("refused");
    assert!(error.contains("1:"), "{error}");
}

#[test]
fn a_theme_missing_a_tile_or_a_mode_mark_is_refused_naming_it() {
    let pack = pack();
    let cases = [
        (
            "terrain(rock):          (glyph: '#', fg: gray),",
            "terrain(rock)",
        ),
        (
            "emote(resting):         (glyph: 'z', fg: light_blue),",
            "emote(resting)",
        ),
        ("grab:   (glyph: 'G', fg: yellow),", "grab"),
    ];
    for (line, named) in cases {
        let error = Theme::from_ron(&ascii_with(line, ""), &pack).expect_err("refused");
        assert!(error.contains(named), "{named}: {error}");
    }
}

#[test]
fn a_theme_glyph_outside_cp437_is_refused() {
    // All the screen draws stays within CP437 (design §6.2).
    let pack = pack();
    for (from, to, named) in [
        (
            "sprite:                 (glyph: '@'",
            "sprite:                 (glyph: 'λ'",
            "sprite",
        ),
        ("leash: (glyph: ';'", "leash: (glyph: '€'", "the leash"),
        ("idle: '-'", "idle: '✓'", "idle status mark"),
        ("left: '|'", "left: '¦'", "visible_frame left"),
        (
            "top_left:     (glyph: '+'",
            "top_left:     (glyph: '┏'",
            "glass's top_left",
        ),
        ("far:     (glyph: '.'", "far:     (glyph: '✦'", "far stars"),
    ] {
        let error = Theme::from_ron(&ascii_with(from, to), &pack).expect_err("refused");
        let glyph = to.chars().rev().nth(1).unwrap();
        assert!(
            error.contains(glyph) && error.contains("CP437") && error.contains(named),
            "{error}"
        );
    }
}

#[test]
fn a_theme_naming_what_the_pack_lacks_is_refused_naming_it() {
    // A misspelt name would otherwise draw a `?` without saying why.
    let pack = pack();
    for (from, to, named) in [
        ("\"ball\":", "\"bal\":", "bal"),
        ("\"fruiting\":", "\"fruting\":", "fruting"),
        ("\"hunger\":", "\"hungre\":", "hungre"),
    ] {
        let error = Theme::from_ron(&ascii_with(from, to), &pack).expect_err("refused");
        assert!(error.contains(named), "{named}: {error}");
    }
}

#[test]
fn a_theme_may_leave_out_objects_and_drives() {
    // They fall back to the object's default look, then `?`, and to the
    // sprite's own colour (design §6.2, §6.3).
    let pack = pack();
    let edited = ascii_with(
        "        \"ball\":      { \"default\": (glyph: 'o', fg: white) },\n",
        "",
    );
    let edited = edited.replacen("        \"boredom\":     dark_gray,\n", "", 1);
    let theme = Theme::from_ron(&edited, &pack).expect("loads");
    assert_eq!(theme.object_glyph("ball", "default").symbol, '?');
    assert_eq!(theme.drive_colour("boredom"), None);
}

#[test]
fn a_theme_from_before_the_young_draws_them_as_grown() {
    // M2 design §9.6: a theme without the young sprite's keys still loads,
    // drawing a young sprite, selected or not, as a grown one.
    let pack = pack();
    let edited = ascii_with(
        "        young_sprite:           (glyph: 'a', fg: white),\n",
        "",
    );
    let edited = edited.replacen(
        "        selected_young_sprite:  (glyph: 'a', fg: white, reversed: true),\n",
        "",
        1,
    );
    let theme = Theme::from_ron(&edited, &pack).expect("loads");
    assert_eq!(
        theme.glyph(SemanticTile::YoungSprite),
        theme.glyph(SemanticTile::Sprite)
    );
    assert_eq!(
        theme.glyph(SemanticTile::SelectedYoungSprite),
        theme.glyph(SemanticTile::SelectedSprite)
    );
    assert_ne!(
        theme.glyph(SemanticTile::YoungSprite),
        Theme::from_ron(ASCII_THEME, &pack)
            .expect("loads")
            .glyph(SemanticTile::YoungSprite),
        "the edit took the young sprite out"
    );
}

#[test]
fn a_theme_from_before_eggs_were_items_draws_them_with_its_egg_tile() {
    // M2 design §10.7: a theme with the retired `egg` tiles still loads,
    // and draws the egg item with its `egg` glyph while its objects have
    // none of their own.
    let pack = pack();
    let without = ascii_with(
        "        \"egg\":       { \"default\": (glyph: '0', fg: light_yellow) },\n",
        "",
    );
    let old = without.replacen(
        "        young_sprite:",
        "        egg: (glyph: 'e', fg: yellow),\n        \
         selected_egg: (glyph: 'e', fg: yellow, reversed: true),\n        \
         young_sprite:",
        1,
    );
    let theme = Theme::from_ron(&old, &pack).expect("loads");
    let egg = theme.object_glyph("egg", "default");
    assert_eq!((egg.symbol, egg.fg), ('e', Color::Yellow));
    // With neither, an egg is a `?`, as any object a theme leaves out.
    let theme = Theme::from_ron(&without, &pack).expect("loads");
    assert_eq!(theme.object_glyph("egg", "default").symbol, '?');
}

/// A theme's glass, by piece, then its stars, far to nearest: glyph and
/// colour.
fn space(theme: &Theme) -> Vec<(char, Color)> {
    let glass = Pane::ALL.map(|pane| theme.glass(pane));
    let stars = Depth::ALL.map(|depth| theme.star(depth));
    glass
        .iter()
        .chain(&stars)
        .map(|glyph| (glyph.symbol, glyph.fg))
        .collect()
}

#[test]
fn the_themes_draw_the_glass_and_the_stars_as_the_design_tables_say() {
    // M2 design §11.2, §11.3, §11.5: horizontal, vertical, then the four
    // corners, from top left; the stars far, near and nearest.
    let expected = |glass: &str, far, near| {
        let glass = glass.chars().map(|c| (c, Color::LightCyan));
        let stars = [
            (far, Color::DarkGray),
            (near, Color::Gray),
            ('*', Color::White),
        ];
        glass.chain(stars).collect::<Vec<_>>()
    };
    assert_eq!(space(&Theme::cp437()), expected("═║╔╗╚╝", '·', '∙'));
    assert_eq!(space(&Theme::ascii()), expected("-|++++", '.', '+'));
}

#[test]
fn a_theme_from_before_space_draws_the_default_theme_s_glass_and_stars() {
    // M2 design §11.5.
    let theme = ascii_from_before("    space: (", "space:");
    assert_eq!(space(&theme), space(&Theme::cp437()));
}

#[test]
fn a_theme_draws_what_it_leaves_out_of_space_as_the_default_theme_does() {
    // M2 design §11.5: a piece of glass or a depth of stars at a time.
    let pack = pack();
    let edited = ascii_with(
        "            top_left:     (glyph: '+', fg: light_cyan),\n",
        "",
    );
    let edited = edited.replacen("            near:    (glyph: '+', fg: gray),\n", "", 1);
    let theme = Theme::from_ron(&edited, &pack).expect("loads");
    let (default, ascii) = (Theme::cp437(), Theme::ascii());
    assert_eq!(theme.glass(Pane::TopLeft), default.glass(Pane::TopLeft));
    assert_eq!(theme.glass(Pane::TopRight), ascii.glass(Pane::TopRight));
    assert_eq!(theme.star(Depth::Near), default.star(Depth::Near));
    assert_eq!(theme.star(Depth::Far), ascii.star(Depth::Far));
    // A misspelt key is refused, rather than quietly drawn as the default.
    let misspelt = ascii_with("top_left:", "top_lft:");
    let error = Theme::from_ron(&misspelt, &pack).expect_err("refused");
    assert!(error.contains("top_lft"), "{error}");
}

/// A theme's population graph: each stage's colour, young to eggs, then its
/// full and half blocks and the limit's dot and colour.
fn graph(theme: &Theme) -> ([Color; 4], char, char, (char, Color)) {
    let look = theme.population();
    (
        [look.young, look.adult, look.old, look.egg],
        look.full,
        look.half,
        (look.limit.symbol, look.limit.fg),
    )
}

#[test]
fn the_themes_draw_the_population_graph_as_the_design_table_says() {
    // M2 design §14.
    let stages = [
        Color::LightGreen,
        Color::Cyan,
        Color::Gray,
        Color::LightYellow,
    ];
    assert_eq!(
        graph(&Theme::cp437()),
        (stages, '█', '▄', ('·', Color::DarkGray))
    );
    assert_eq!(
        graph(&Theme::ascii()),
        (stages, '#', '_', ('.', Color::DarkGray))
    );
}

#[test]
fn a_theme_from_before_the_population_graph_draws_the_default_theme_s() {
    // M2 design §14.
    let theme = ascii_from_before("    // The population graph", "population:");
    assert_eq!(theme.population(), Theme::cp437().population());
}

#[test]
fn a_theme_draws_what_it_leaves_out_of_the_graph_as_the_default_theme_does() {
    // M2 design §14: a stage's colour or a block at a time.
    let pack = pack();
    let edited = ascii_with(", egg: light_yellow)", ")");
    let edited = edited.replacen("        half: '_',\n", "", 1);
    let edited = edited.replacen("adult: cyan", "adult: magenta", 1);
    let theme = Theme::from_ron(&edited, &pack).expect("loads");
    let (default, ascii) = (Theme::cp437().population(), Theme::ascii().population());
    let look = theme.population();
    assert_eq!((look.adult, look.egg), (Color::Magenta, default.egg));
    assert_eq!((look.full, look.half), (ascii.full, default.half));
    // A misspelt key is refused, rather than quietly drawn as the default.
    let misspelt = ascii_with("half: '_'", "halve: '_'");
    let error = Theme::from_ron(&misspelt, &pack).expect_err("refused");
    assert!(error.contains("halve"), "{error}");
}

/// A theme's lineage colours: parents and grandparents, brothers and
/// sisters, young and theirs, and the rest.
fn lineage(theme: &Theme) -> [Color; 4] {
    let look = theme.lineage();
    [look.elders, look.siblings, look.young, look.others]
}

#[test]
fn the_themes_draw_the_lineage_colours_as_the_design_table_says() {
    // M2 design §17.4.
    let colours = [
        Color::LightBlue,
        Color::LightRed,
        Color::LightMagenta,
        Color::DarkGray,
    ];
    assert_eq!(lineage(&Theme::cp437()), colours);
    assert_eq!(lineage(&Theme::ascii()), colours);
}

#[test]
fn a_theme_from_before_the_lineage_colours_draws_the_default_theme_s() {
    // M2 design §17.4.
    let theme = ascii_from_before("    // The lineage colours", "lineage:");
    assert_eq!(lineage(&theme), lineage(&Theme::cp437()));
}

#[test]
fn a_theme_draws_the_lineage_colours_it_leaves_out_as_the_default_theme_does() {
    // M2 design §17.4: a colour at a time.
    let pack = pack();
    let edited = ascii_with(
        "lineage: (elders: light_blue, siblings: light_red, young: light_magenta, others: dark_gray)",
        "lineage: (young: green)",
    );
    let theme = Theme::from_ron(&edited, &pack).expect("loads");
    let [elders, siblings, _, others] = lineage(&Theme::cp437());
    assert_eq!(lineage(&theme), [elders, siblings, Color::Green, others]);
    // A key the game doesn't know is refused, rather than quietly drawn as
    // the default.
    let misspelt = ascii_with("others: dark_gray", "strangers: dark_gray");
    let error = Theme::from_ron(&misspelt, &pack).expect_err("refused");
    assert!(error.contains("strangers"), "{error}");
}
