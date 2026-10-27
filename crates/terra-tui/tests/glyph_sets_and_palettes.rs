//! A theme is a glyph set and a palette, chosen apart (design §6.2, #307):
//! the characters from one theme, and the colours, bold and reverse video
//! from another.

mod common;
use common::{hue_and_saturation, on_the_ramp, pack};

use ratatui::style::{Color, Modifier};
use terra_tui::app::CursorMode;
use terra_tui::campbell;
use terra_tui::space::{Depth, Pane};
use terra_tui::theme::{ColourPalette, GlyphSet, SemanticTile, Theme};

const ASCII_THEME: &str = include_str!("../../../themes/ascii.ron");

/// Every semantic tile a theme draws, the optional ones too.
fn every_tile() -> Vec<SemanticTile> {
    let optional = SemanticTile::OPTIONAL.map(|(tile, _)| tile);
    SemanticTile::ALL.iter().chain(&optional).copied().collect()
}

/// A theme of the player's own: the ascii theme's characters, in colours
/// of its own, with shades and a heading of its own.
fn recoloured() -> Theme {
    let text = ASCII_THEME
        .replace("fg: white", "fg: light_cyan, bold: true")
        .replace("fg: green", "fg: \"#123456\"")
        .replace("fg: yellow", "fg: light_red")
        .replace("light_yellow", "magenta")
        .replace("dark_gray", "\"#202020\"")
        .replace(
            "heading:  (fg: light_cyan, bold: true)",
            "heading:  (fg: yellow)",
        )
        .replace(
            "panels: (shade: \"#181a22\", title: \"#282c3a\")",
            "panels: (shade: \"#101010\", title: \"#303030\")",
        );
    Theme::from_ron(&text, &pack()).expect("the recoloured theme loads")
}

#[test]
fn a_theme_draws_one_theme_s_glyphs_in_another_s_palette() {
    // Design §6.2, #307: the glyph set gives every character a theme
    // holds; the palette, everything else.
    let (glyphs, palette) = (Theme::cp437(), recoloured());
    let theme = Theme::compose(&glyphs, &palette);
    for tile in every_tile() {
        let (drawn, from, coloured) = (theme.glyph(tile), glyphs.glyph(tile), palette.glyph(tile));
        assert_eq!(drawn.symbol, from.symbol, "{tile:?}");
        assert_eq!(
            (drawn.fg, drawn.bold, drawn.reversed),
            (coloured.fg, coloured.bold, coloured.reversed),
            "{tile:?}"
        );
    }
    let pack = pack();
    for object in pack.object_type_names() {
        for state in pack.visual_states(object) {
            let (drawn, from, coloured) = (
                theme.object_glyph(object, state),
                glyphs.object_glyph(object, state),
                palette.object_glyph(object, state),
            );
            assert_eq!(drawn.symbol, from.symbol, "{object} {state}");
            assert_eq!(
                (drawn.fg, drawn.bold),
                (coloured.fg, coloured.bold),
                "{object} {state}"
            );
        }
    }
    assert_eq!(theme.arrows(), glyphs.arrows());
    assert_eq!(theme.followed_arrows(), glyphs.followed_arrows());
    assert_eq!(theme.visible_frame(), glyphs.visible_frame());
    assert_eq!(theme.status_marks(), glyphs.status_marks());
    for mode in CursorMode::ALL {
        let drawn = theme.mode_mark(mode);
        assert_eq!(drawn.symbol, glyphs.mode_mark(mode).symbol);
        assert_eq!(drawn.fg, palette.mode_mark(mode).fg);
    }
    for (drawn, from, coloured) in [
        (theme.leash(), glyphs.leash(), palette.leash()),
        (theme.aim(), glyphs.aim(), palette.aim()),
        (theme.aim_end(), glyphs.aim_end(), palette.aim_end()),
    ] {
        assert_eq!((drawn.symbol, drawn.fg), (from.symbol, coloured.fg));
    }
    for pane in Pane::ALL {
        let drawn = theme.glass(pane);
        assert_eq!(drawn.symbol, glyphs.glass(pane).symbol, "{pane:?}");
        assert_eq!(drawn.fg, palette.glass(pane).fg, "{pane:?}");
    }
    for depth in Depth::ALL {
        let drawn = theme.star(depth);
        assert_eq!(drawn.symbol, glyphs.star(depth).symbol, "{depth:?}");
        assert_eq!(drawn.fg, palette.star(depth).fg, "{depth:?}");
    }
    let (graph, from, coloured) = (
        theme.population(),
        glyphs.population(),
        palette.population(),
    );
    assert_eq!(
        (graph.full, graph.half, graph.limit.symbol),
        (from.full, from.half, from.limit.symbol)
    );
    assert_eq!(
        [
            graph.young,
            graph.adult,
            graph.old,
            graph.egg,
            graph.limit.fg
        ],
        [
            coloured.young,
            coloured.adult,
            coloured.old,
            coloured.egg,
            coloured.limit.fg
        ]
    );
    for drive in pack.drives() {
        assert_eq!(theme.drive_colour(drive), palette.drive_colour(drive));
    }
    assert_eq!(theme.attention_marker(), palette.attention_marker());
    assert_eq!(theme.lineage(), palette.lineage());
    assert_eq!(theme.text(), palette.text());
    assert_eq!(theme.panels(), palette.panels());
    assert_eq!(theme.motion(), palette.motion());
    // Game Kid's foreground and its colour modes go with its palette.
    let game_kid = ColourPalette::GameKid.source();
    for (glyphs, palette) in [
        (&game_kid, Theme::cp437()),
        (&Theme::cp437(), game_kid.clone()),
    ] {
        let theme = Theme::compose(glyphs, &palette);
        assert_eq!(theme.foreground(), palette.foreground());
        assert_eq!(theme.shows_colour_modes(), palette.shows_colour_modes());
    }
    // And it really is a mix of the two.
    assert_eq!(theme.glyph(SemanticTile::Sprite).symbol, '☺');
    assert_eq!(
        theme
            .glyph(SemanticTile::Terrain(terra_sim::Terrain::Grass))
            .fg,
        Color::Rgb(0x12, 0x34, 0x56)
    );
}

#[test]
fn a_theme_composed_from_itself_is_itself() {
    // So a theme file given as both, as `--theme` gives it, draws as
    // written.
    let theme = recoloured();
    assert_eq!(Theme::compose(&theme, &theme), theme);
    assert_eq!(
        Theme::compose(&Theme::cp437(), &Theme::cp437()),
        Theme::cp437()
    );
}

#[test]
fn the_ascii_glyphs_sit_on_the_default_palette_s_shades() {
    // Design §6.2, #307: the shades go with the palette, not the glyph set,
    // so ASCII glyphs in the default palette sit on shades.
    let ascii = Theme::ascii();
    assert_eq!(
        ascii,
        Theme::built_in(GlyphSet::Ascii, ColourPalette::Default)
    );
    assert_eq!(ascii.glyph(SemanticTile::Sprite).symbol, '@');
    assert_eq!(ascii.panels(), Theme::cp437().panels());
    assert_eq!(
        Theme::cp437(),
        Theme::built_in(GlyphSet::Cp437, ColourPalette::Default)
    );
    // Its file, given as both with `--theme`, draws the same.
    assert_eq!(Theme::from_ron(ASCII_THEME, &pack()), Ok(ascii));
}

/// Every colour `theme` draws a character of the map or the text in, with
/// what it's for: the tiles, the objects, the Cursor, the glass and the
/// stars, the drives, the lineage, the population graph, the text roles,
/// and the ghost's ash while motion is on. A text role with no colour of
/// its own is `Reset`, the terminal's own.
fn foreground_colours(theme: &Theme) -> Vec<(String, Color)> {
    let pack = pack();
    let mut colours: Vec<(String, Color)> = every_tile()
        .into_iter()
        .map(|tile| (format!("{tile:?}"), theme.glyph(tile).fg))
        .collect();
    for object in pack.object_type_names() {
        for state in pack.visual_states(object) {
            let fg = theme.object_glyph(object, state).fg;
            colours.push((format!("{object} {state}"), fg));
        }
    }
    for mode in CursorMode::ALL {
        colours.push((format!("{mode:?} mode"), theme.mode_mark(mode).fg));
    }
    for (what, glyph) in [
        ("the leash", theme.leash()),
        ("the aim line", theme.aim()),
        ("the aim's end", theme.aim_end()),
    ] {
        colours.push((what.into(), glyph.fg));
    }
    for pane in Pane::ALL {
        colours.push((format!("{pane:?} glass"), theme.glass(pane).fg));
    }
    for depth in Depth::ALL {
        colours.push((format!("{depth:?} stars"), theme.star(depth).fg));
    }
    for drive in pack.drives() {
        let colour = theme.drive_colour(drive).unwrap_or(Color::Reset);
        colours.push((drive.into(), colour));
    }
    let lineage = theme.lineage();
    let graph = theme.population();
    let motion = theme.motion();
    for (what, colour) in [
        ("elders", lineage.elders),
        ("siblings", lineage.siblings),
        ("the young", lineage.young),
        ("others", lineage.others),
        ("young on the graph", graph.young),
        ("adults on the graph", graph.adult),
        ("the old on the graph", graph.old),
        ("eggs on the graph", graph.egg),
        ("the graph's limit", graph.limit.fg),
        ("the ghost's ash", motion.ash),
    ] {
        colours.push((what.into(), colour));
    }
    let text = theme.text();
    for (role, style) in [
        ("heading", text.heading),
        ("muted", text.muted),
        ("good", text.good),
        ("bad", text.bad),
        ("key", text.key),
        ("selected", text.selected),
        ("spot", text.spot),
    ] {
        colours.push((format!("{role} text"), style.fg.unwrap_or(Color::Reset)));
    }
    colours
}

/// The red, green and blue of an exact colour.
fn rgb(colour: Color) -> [u8; 3] {
    match colour {
        Color::Rgb(r, g, b) => [r, g, b],
        other => panic!("{other:?} isn't exact"),
    }
}

/// A colour channel from 0 to 255 as linear light, from 0 to 1 (sRGB).
fn linear(channel: u8) -> f64 {
    let c = f64::from(channel) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG's relative luminance of `[r, g, b]`.
fn luminance([r, g, b]: [u8; 3]) -> f64 {
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

/// WCAG's contrast ratio between two colours, from 1 to 21.
fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn high_contrast_draws_the_map_and_the_text_at_seven_to_one_in_exact_colours() {
    // Design §6.2: bright exact colours, the same in every terminal, each
    // at WCAG's ratio for the clearest text, 7:1, against the black the
    // renderer draws on, Campbell's.
    let theme = ColourPalette::HighContrast.source();
    for (what, colour) in foreground_colours(&theme) {
        let Color::Rgb(..) = colour else {
            panic!("{what} is {colour:?}, not an exact colour");
        };
        let ratio = contrast(rgb(colour), campbell::BACKGROUND);
        assert!(ratio >= 7.0, "{what}, {colour:?}, is {ratio:.2}:1");
    }
}

#[test]
fn high_contrast_makes_headings_and_keys_bold_minor_text_light_and_draws_lines() {
    // Design §6.2: minor text light grey rather than dark, headings and
    // keys bold, a brighter attention marker, and lines between the
    // panels in place of shades.
    let theme = ColourPalette::HighContrast.source();
    let text = theme.text();
    assert!(text.heading.add_modifier.contains(Modifier::BOLD));
    assert!(text.key.add_modifier.contains(Modifier::BOLD));
    let muted = rgb(text.muted.fg.expect("a colour"));
    assert!(
        muted[0] == muted[1] && muted[1] == muted[2],
        "{muted:?} is a grey"
    );
    assert!(luminance(muted) > 0.4, "{muted:?} is light");
    let marker = luminance(rgb(theme.attention_marker()));
    let default = campbell::rgb(Theme::cp437().attention_marker()).expect("named");
    assert!(marker > luminance(default), "brighter than the default's");
    assert_eq!(theme.panels(), None);
}

/// Okabe and Ito's colours for colour-blind readers, but black: orange, sky
/// blue, bluish green, yellow, blue, vermillion and reddish purple.
const OKABE_ITO: [[u8; 3]; 7] = [
    [0xE6, 0x9F, 0x00],
    [0x56, 0xB4, 0xE9],
    [0x00, 0x9E, 0x73],
    [0xF0, 0xE4, 0x42],
    [0x00, 0x72, 0xB2],
    [0xD5, 0x5E, 0x00],
    [0xCC, 0x79, 0xA7],
];

/// Whether `colour` is `from` taken some way towards `to`, to the nearest
/// whole step of each channel.
fn towards(colour: [u8; 3], from: [u8; 3], to: [u8; 3]) -> bool {
    let (c, f, t) = (
        colour.map(f64::from),
        from.map(f64::from),
        to.map(f64::from),
    );
    // How far, from the channel that moves most.
    let i = (0..3)
        .max_by(|&a, &b| (t[a] - f[a]).abs().total_cmp(&(t[b] - f[b]).abs()))
        .unwrap_or(0);
    let way = (c[i] - f[i]) / (t[i] - f[i]);
    (0.0..=1.0).contains(&way) && (0..3).all(|j| (f[j] + (t[j] - f[j]) * way - c[j]).abs() <= 1.0)
}

/// Linear red, green and blue as CIELAB, under daylight (D65).
fn lab([r, g, b]: [f64; 3]) -> [f64; 3] {
    let x = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047;
    let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
    let z = (0.0193339 * r + 0.1191920 * g + 0.9503041 * b) / 1.08883;
    let f = |t: f64| {
        let d: f64 = 6.0 / 29.0;
        if t > d * d * d {
            t.cbrt()
        } else {
            t / (3.0 * d * d) + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// How a reader with no red-sensing cones sees linear red, green and blue:
/// Machado, Oliveira and Fernandes (2009), at severity 1.0.
const PROTANOPIA: [[f64; 3]; 3] = [
    [0.152286, 1.052583, -0.204868],
    [0.114503, 0.786281, 0.099216],
    [-0.003882, -0.048116, 1.051998],
];

/// And with no green-sensing cones.
const DEUTERANOPIA: [[f64; 3]; 3] = [
    [0.367322, 0.860646, -0.227968],
    [0.280085, 0.672501, 0.047413],
    [-0.011820, 0.042940, 0.968881],
];

/// `colour` in CIELAB as a reader sees it whose eyes `seeing` simulates, in
/// linear light, or as any other reader does.
fn seen(colour: [u8; 3], seeing: Option<&[[f64; 3]; 3]>) -> [f64; 3] {
    let light = colour.map(linear);
    let light = match seeing {
        Some(m) => [0, 1, 2].map(|i| {
            (0..3)
                .map(|j| m[i][j] * light[j])
                .sum::<f64>()
                .clamp(0.0, 1.0)
        }),
        None => light,
    };
    lab(light)
}

/// The CIELAB distance between two colours (ΔE*ab, 1976).
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

/// A colour, named for what it tells.
type Named = (String, [u8; 3]);

/// A set of colours that must be told apart, named for what they are.
type ToldApart = (&'static str, Vec<Named>);

/// The colours colour tells apart, each set by what it is: the drives,
/// with the sprite's own colour, which a sprite with none above half is
/// drawn in, and the lineage, with the selected sprite's own, if the
/// palette shows them; a gain and a loss; the graph's stages; and a bush
/// bare and fruiting, the same glyph.
fn told_apart(theme: &Theme) -> Vec<ToldApart> {
    let (lineage, graph, text) = (theme.lineage(), theme.population(), theme.text());
    let named = |colours: &[(&str, Color)]| {
        colours
            .iter()
            .map(|&(what, colour)| (what.to_string(), rgb(colour)))
            .collect::<Vec<_>>()
    };
    let mut sets = Vec::new();
    if theme.shows_colour_modes() {
        let own = (
            "own colour".to_string(),
            rgb(theme.glyph(SemanticTile::Sprite).fg),
        );
        let mut drives: Vec<Named> = pack()
            .drives()
            .map(|drive| {
                (
                    drive.to_string(),
                    rgb(theme.drive_colour(drive).expect("a colour")),
                )
            })
            .collect();
        drives.push(own.clone());
        let mut kin = named(&[
            ("elders", lineage.elders),
            ("siblings", lineage.siblings),
            ("young", lineage.young),
            ("others", lineage.others),
        ]);
        kin.push(own);
        sets.extend([("drives", drives), ("lineage", kin)]);
    }
    let fg = |style: ratatui::style::Style| style.fg.expect("a colour");
    sets.extend([
        (
            "gains",
            named(&[("good", fg(text.good)), ("bad", fg(text.bad))]),
        ),
        (
            "stages",
            named(&[
                ("young", graph.young),
                ("adult", graph.adult),
                ("old", graph.old),
                ("egg", graph.egg),
            ]),
        ),
        (
            "bushes",
            named(&[
                ("bare", theme.object_glyph("berry_bush", "default").fg),
                ("fruiting", theme.object_glyph("berry_bush", "fruiting").fg),
            ]),
        ),
    ]);
    sets
}

#[test]
fn colour_blind_is_built_from_okabe_and_ito_s_colours() {
    // Design §6.2: exact colours from their set, with greys and white,
    // lightened or darkened where a thing needs a colour of its own or is
    // too dark on black.
    let theme = ColourPalette::ColourBlind.source();
    for (what, colour) in foreground_colours(&theme) {
        if colour == Color::Reset {
            continue; // the terminal's own, as a key's is
        }
        let c = rgb(colour);
        let grey = c[0] == c[1] && c[1] == c[2];
        let theirs = OKABE_ITO
            .iter()
            .any(|&o| towards(c, o, [255, 255, 255]) || towards(c, o, [0, 0, 0]));
        assert!(grey || theirs, "{what}, {colour:?}, isn't one of theirs");
    }
    assert_eq!(
        theme.panels(),
        Theme::cp437().panels(),
        "the default's shades"
    );
}

#[test]
fn colour_blind_keeps_apart_what_colour_tells_for_readers_blind_to_red_or_green() {
    // Design §6.2: the drives, the lineage, a gain against a loss and the
    // graph's stages, simulated as protanopes and deuteranopes see them
    // (Machado, Oliveira and Fernandes 2009). A CIELAB distance of 10 is
    // about four just-noticeable differences (2.3 each): two small glyphs
    // on black far enough apart to tell at a glance, not only side by side.
    let theme = ColourPalette::ColourBlind.source();
    for (set, colours) in told_apart(&theme) {
        for (i, (a, ca)) in colours.iter().enumerate() {
            for (b, cb) in &colours[i + 1..] {
                for (reader, seeing) in [
                    ("anyone", None),
                    ("a protanope", Some(&PROTANOPIA)),
                    ("a deuteranope", Some(&DEUTERANOPIA)),
                ] {
                    let apart = distance(seen(*ca, seeing), seen(*cb, seeing));
                    assert!(
                        apart >= 10.0,
                        "{set}: {a} and {b} are {apart:.1} apart for {reader}"
                    );
                }
            }
        }
    }
}

#[test]
fn colour_blind_never_pairs_red_against_green() {
    // Design §6.2: among the colours that tell things apart, none is red
    // while another is green. A colour is red or green when its CIELAB hue
    // is within 30° of pure red's or pure green's, and it has some colour
    // at all (a chroma of 20 or more).
    let hue = |colour: [u8; 3]| {
        let [_, a, b] = seen(colour, None);
        (b.atan2(a).to_degrees().rem_euclid(360.0), a.hypot(b))
    };
    let near = |colour: [u8; 3], pure: [u8; 3]| {
        let ((h, chroma), (p, _)) = (hue(colour), hue(pure));
        let off = (h - p).abs();
        chroma >= 20.0 && off.min(360.0 - off) < 30.0
    };
    let red = |c| near(c, [255, 0, 0]);
    let green = |c| near(c, [0, 255, 0]);
    let theme = ColourPalette::ColourBlind.source();
    for (set, colours) in told_apart(&theme) {
        let reds: Vec<&String> = colours
            .iter()
            .filter(|(_, c)| red(*c))
            .map(|(w, _)| w)
            .collect();
        let greens: Vec<&String> = colours
            .iter()
            .filter(|(_, c)| green(*c))
            .map(|(w, _)| w)
            .collect();
        assert!(
            reds.is_empty() || greens.is_empty(),
            "{set}: {reds:?} red against {greens:?} green"
        );
    }
    // The default palette does, which is what this one is for.
    let default = Theme::cp437();
    let (good, bad) = (default.text().good.fg, default.text().bad.fg);
    let campbell = |c: Option<Color>| campbell::rgb(c.expect("a colour")).expect("named");
    assert!(green(campbell(good)) && red(campbell(bad)));
}

#[test]
fn mono_has_no_colour_only_bold_reverse_video_and_lines() {
    // Design §6.2: what colour told is read from bold and reverse video,
    // and the signs, arrows and capitals, which stay.
    let theme = ColourPalette::Mono.source();
    assert!(!theme.has_colour());
    assert!(Theme::cp437().has_colour());
    let text = theme.text();
    for (role, style) in [
        ("heading", text.heading),
        ("key", text.key),
        ("spot", text.spot),
    ] {
        assert!(style.add_modifier.contains(Modifier::BOLD), "{role}");
    }
    assert!(text.selected.add_modifier.contains(Modifier::REVERSED));
    assert!(theme.glyph(SemanticTile::SelectedSprite).reversed);
    assert!(theme.object_glyph("berry_bush", "fruiting").bold);
    assert_eq!(theme.panels(), None);
}

/// The original Game Boy's four greens, darkest to lightest, which Game
/// Kid is after.
const HANDHELD_GREENS: [[u8; 3]; 4] = [
    [0x0F, 0x38, 0x0F],
    [0x30, 0x62, 0x30],
    [0x8B, 0xAC, 0x0F],
    [0x9B, 0xBC, 0x0F],
];

/// Every colour `theme` draws with, with what it's for, a named one as
/// Campbell draws it: the map's and the text's, the attention marker, the
/// panels' shades, the foreground, and motion's colours but the dark, which
/// is the terminal's black a pop-up fades from. A colour left to the
/// terminal's own is drawn in the foreground.
fn every_colour(theme: &Theme) -> Vec<(String, [u8; 3])> {
    let foreground = theme.foreground().expect("a foreground");
    let mut colours = foreground_colours(theme);
    let motion = theme.motion();
    colours.extend([
        ("the foreground".to_string(), foreground),
        ("the attention marker".into(), theme.attention_marker()),
        ("a death's flash".into(), motion.flash),
        ("a hatching's sparks".into(), motion.spark),
        ("the trail".into(), motion.trail),
    ]);
    if let Some(panels) = theme.panels() {
        colours.push(("the panels' shade".into(), panels.shade));
        colours.push(("their title rows' shade".into(), panels.title));
    }
    colours
        .into_iter()
        .map(|(what, colour)| {
            let drawn = if colour == Color::Reset {
                foreground
            } else {
                colour
            };
            (what, campbell::rgb(drawn).expect("a colour"))
        })
        .collect()
}

#[test]
fn game_kid_draws_everything_in_the_handheld_s_four_greens_and_greens_on_their_ramp() {
    // Design §6.2: after the original Game Boy's four greens, on the
    // terminal's own black, with greens on the same ramp, lighter or
    // darker, where text must be read on black or things kept apart.
    let theme = ColourPalette::GameKid.source();
    assert!(theme.has_colour());
    let colours = every_colour(&theme);
    for (what, colour) in &colours {
        let (hue, saturation) = hue_and_saturation(*colour);
        assert!(
            on_the_ramp(*colour),
            "{what}, {colour:?}, has a hue of {hue:.0}° and a saturation of {saturation:.2}"
        );
    }
    for green in HANDHELD_GREENS {
        assert!(
            colours.iter().any(|(_, colour)| *colour == green),
            "{green:?} isn't used"
        );
    }
    assert_eq!(
        campbell::rgb(theme.motion().dark),
        Some(campbell::BACKGROUND),
        "the terminal's black"
    );
    assert!(theme.panels().is_some(), "shades from the dark greens");
}

#[test]
fn game_kid_s_text_reads_on_black_minor_text_at_three_to_one_the_rest_at_four_and_a_half() {
    // Design §6.2: WCAG's ratios for text, against the black the renderer
    // draws on, Campbell's; the text with no role of its own is drawn in
    // the foreground.
    let theme = ColourPalette::GameKid.source();
    let text = theme.text();
    let foreground = theme.foreground().expect("a foreground");
    for (role, colour, least) in [
        ("body", Some(foreground), 4.5),
        ("heading", text.heading.fg, 4.5),
        ("key", text.key.fg, 4.5),
        ("good", text.good.fg, 4.5),
        ("bad", text.bad.fg, 4.5),
        ("selected", text.selected.fg, 4.5),
        ("spot", text.spot.fg, 4.5),
        ("muted", text.muted.fg, 3.0),
    ] {
        let colour = campbell::rgb(colour.unwrap_or(foreground)).expect("a colour");
        let ratio = contrast(colour, campbell::BACKGROUND);
        assert!(ratio >= least, "{role}, {colour:?}, is {ratio:.2}:1");
    }
    assert!(text.heading.add_modifier.contains(Modifier::BOLD));
    assert!(text.key.add_modifier.contains(Modifier::BOLD));
    assert!(text.selected.add_modifier.contains(Modifier::REVERSED));
}

#[test]
fn game_kid_can_t_show_the_colour_modes_and_keeps_the_rest_apart_by_lightness() {
    // Design §6.2, §6.3: greens can't tell nine drives, or the lineage,
    // apart, so it says it can't show them, as mono does, and `b` and the
    // help say where they're read instead. A gain against a loss, the
    // graph's stages and a bush bare and fruiting are told apart, as
    // colour-blind's are, by a CIELAB distance of 10 or more, about four
    // just-noticeable differences: here it's nearly all lightness.
    let theme = ColourPalette::GameKid.source();
    assert!(theme.has_colour(), "motion still plays");
    assert!(!theme.shows_colour_modes());
    let sets = told_apart(&theme);
    assert_eq!(
        sets.iter().map(|(set, _)| *set).collect::<Vec<_>>(),
        ["gains", "stages", "bushes"]
    );
    for (set, colours) in sets {
        for (i, (a, ca)) in colours.iter().enumerate() {
            for (b, cb) in &colours[i + 1..] {
                let apart = distance(seen(*ca, None), seen(*cb, None));
                assert!(apart >= 10.0, "{set}: {a} and {b} are {apart:.1} apart");
            }
        }
    }
}

#[test]
fn every_built_in_palette_loads_as_a_theme_file_would_drawing_the_cp437_glyphs() {
    // Each is a whole theme file, checked as `--theme` checks one, whose
    // glyphs are the CP437 set's, so a copy of one reads as a whole theme.
    let pack = pack();
    for (palette, file) in [
        (
            ColourPalette::Default,
            include_str!("../../../themes/cp437.ron"),
        ),
        (
            ColourPalette::HighContrast,
            include_str!("../../../themes/high_contrast.ron"),
        ),
        (
            ColourPalette::ColourBlind,
            include_str!("../../../themes/colour_blind.ron"),
        ),
        (
            ColourPalette::Mono,
            include_str!("../../../themes/mono.ron"),
        ),
        (
            ColourPalette::GameKid,
            include_str!("../../../themes/game_kid.ron"),
        ),
    ] {
        let name = palette.name();
        let theme = Theme::from_ron(file, &pack).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(theme, palette.source(), "{name}");
        assert_eq!(Theme::compose(&Theme::cp437(), &theme), theme, "{name}");
    }
    assert_eq!(ColourPalette::ALL.len(), 5);
    assert_eq!(ColourPalette::GameKid.name(), "Game Kid");
    // A palette with no colour has none in any glyph set.
    assert!(!Theme::built_in(GlyphSet::Ascii, ColourPalette::Mono).has_colour());
    assert!(Theme::compose(&ColourPalette::Mono.source(), &Theme::cp437()).has_colour());
}
