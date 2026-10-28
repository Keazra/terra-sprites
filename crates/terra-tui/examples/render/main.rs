//! The renderer (#241): draws the game off-screen into a GIF, an MP4 or an
//! animated SVG, through the game's own UI code, so a clip looks exactly
//! like the game. `cargo run --release -p terra-tui --example render --
//! --help` says how to use it.

mod boxes;
mod clip;
mod gif_file;
mod glyphs;
mod look;
mod mp4_file;
mod options;
mod picture;
mod svg_file;
#[cfg(test)]
mod test_util;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use terra_sim::{DataPack, Playback};
use terra_tui::args::Args;
use terra_tui::settings::SavedSettings;
use terra_tui::start;
use terra_tui::theme::{Pick, Theme};

use clip::{Clip, Framing, Source};
use glyphs::{Fonts, Glyphs};
use options::{Format, Options, Scene, USAGE, WorldScene};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || options::asks_for_help(&args) {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let options = match Options::parse(args) {
        Ok(options) => options,
        Err(err) => {
            eprintln!("render: {err}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match render(&options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("render: {err}");
            ExitCode::FAILURE
        }
    }
}

/// The most an animated SVG for a README should be, in bytes (#241).
const README_SVG_BYTES: u64 = 2_000_000;

/// Draws the clip `options` ask for and writes it: in each font, for
/// samples.
fn render(options: &Options) -> Result<(), String> {
    let framing = &options.framing;
    // The samples' fonts load before the world runs, so a bad one says so
    // at once.
    let samples = match options.scene {
        Scene::Samples(_) => Some(sample_fonts(&options.fonts)?),
        _ => None,
    };
    let clip = match &options.scene {
        Scene::Title { seconds } => {
            let seed = options.world.seed.unwrap_or_default();
            let size = (framing.screen.width, framing.screen.height);
            let world = start::title_world(&options.world, seed, size)?;
            let theme = theme(options, world.data())?;
            clip::title_clip(world, theme, seed, *seconds, framing)
        }
        Scene::World(scene) | Scene::Samples(scene) => world_clip(options, scene, framing)?,
    };
    if let Some(fonts) = samples {
        for (name, fonts) in fonts {
            draw(&clip, fonts, options, &sample_path(&options.out, &name))?;
        }
        return Ok(());
    }
    let fonts = match options.fonts.first() {
        Some(font) => Fonts::from_files(font, options.bold_font.as_deref())?,
        None => Fonts::cascadia(),
    };
    draw(&clip, fonts, options, &options.out)
}

/// The fonts samples are drawn in, Cascadia Mono first, each named for its
/// file. Two of the same name would write the same file, so they're
/// refused; Windows sees `Mono` and `mono` as one name.
fn sample_fonts(paths: &[PathBuf]) -> Result<Vec<(String, Fonts)>, String> {
    let mut fonts = vec![("cascadia-mono".to_string(), Fonts::cascadia())];
    for path in paths {
        let name = path.file_stem().map_or_else(
            || "font".to_string(),
            |stem| stem.to_string_lossy().into_owned(),
        );
        if fonts
            .iter()
            .any(|(taken, _)| taken.eq_ignore_ascii_case(&name))
        {
            return Err(format!(
                "two fonts are named {name}, so their samples would be the same file: \
                 rename one"
            ));
        }
        fonts.push((name, Fonts::from_files(path, None)?));
    }
    Ok(fonts)
}

/// Draws `clip` in `fonts` and writes it to `out`, saying what it wrote.
fn draw(clip: &Clip, fonts: Fonts, options: &Options, out: &Path) -> Result<(), String> {
    let mut glyphs = Glyphs::new(fonts, options.cell);
    write(clip, &mut glyphs, options.format, out)?;
    let (width, height) = picture::pixels(&glyphs, options.framing.screen);
    let bytes = std::fs::metadata(out).map_or(0, |file| file.len());
    let frames = match clip.frames.len() {
        1 => "a still".to_string(),
        n => format!("{n} frames over {:.1} seconds", clip.length().as_secs_f64()),
    };
    println!(
        "Wrote {}: {frames}, {width}x{height} pixels, {}",
        out.display(),
        said_bytes(bytes)
    );
    if !glyphs.missing().is_empty() {
        let missing: String = glyphs.missing().iter().collect();
        eprintln!("render: the font has no glyph for {missing}, drawn as its missing-glyph box");
    }
    if options.format == Format::Svg && bytes > README_SVG_BYTES {
        eprintln!(
            "render: past the {} a README carries well: fewer ticks, a slower \
             --speed or a longer --frame makes it smaller",
            said_bytes(README_SVG_BYTES)
        );
    }
    Ok(())
}

/// Where samples in the font `name` go: `out` with the name after its own,
/// `fonts.gif` as `fonts-cascadia-mono.gif`.
fn sample_path(out: &Path, name: &str) -> PathBuf {
    let stem = out
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let extension = out.extension().map_or_else(String::new, |extension| {
        extension.to_string_lossy().into_owned()
    });
    out.with_file_name(format!("{stem}-{name}.{extension}"))
}

/// A world's clip, from the seed or the replay `options` give.
fn world_clip(options: &Options, scene: &WorldScene, framing: &Framing) -> Result<Clip, String> {
    let source = source(&options.world)?;
    let theme = theme(options, source.world().data())?;
    clip::world_clip(source, theme, scene, framing)
}

/// The theme the flags ask for (design §6.2, #307): the glyph set and the
/// palette the game would start with, with `--colours`'s palette over
/// `--theme`'s. The renderer reads neither the settings nor `NO_COLOR`, so
/// a clip comes out the same on every computer.
fn theme(options: &Options, data: &DataPack) -> Result<Theme, String> {
    let nothing_saved = Ok(SavedSettings::default());
    let (settings, _) = start::settings(&options.world, nothing_saved, None, None, data)?;
    let look = settings.look;
    match options.colours {
        Some(palette) => start::theme(look.glyphs(), &Pick::BuiltIn(palette), data),
        None => Ok(look.theme()),
    }
}

/// The world the flags ask for: the replay's, or a new one from the seed.
fn source(args: &Args) -> Result<Source, String> {
    let Some(path) = &args.replay else {
        let seed = args.seed.unwrap_or_default();
        return Ok(Source::World(start::new_world(args, seed)?));
    };
    let cant = |why: String| format!("can't play the replay {}: {why}", path.display());
    let bytes = std::fs::read(path).map_err(|err| cant(err.to_string()))?;
    let playback = Playback::new(&bytes).map_err(|err| cant(err.to_string()))?;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    Ok(Source::Replay { name, playback })
}

/// Writes `clip` to `path` in `format`.
fn write(clip: &Clip, glyphs: &mut Glyphs, format: Format, path: &Path) -> Result<(), String> {
    match format {
        Format::Gif => {
            let mut out = create(path)?;
            gif_file::write(clip, glyphs, &mut out)?;
            out.flush().map_err(|err| cant_write(path, err))
        }
        Format::Svg => {
            let mut out = create(path)?;
            svg_file::write(clip, glyphs, &mut out)?;
            out.flush().map_err(|err| cant_write(path, err))
        }
        Format::Mp4 => mp4_file::write(clip, glyphs, path, mp4_file::FFMPEG),
    }
}

/// A new file at `path`, to write to.
fn create(path: &Path) -> Result<BufWriter<File>, String> {
    File::create(path)
        .map(BufWriter::new)
        .map_err(|err| cant_write(path, err))
}

fn cant_write(path: &Path, err: std::io::Error) -> String {
    format!("can't write {}: {err}", path.display())
}

/// A file's size, said: "1.4 MB".
fn said_bytes(bytes: u64) -> String {
    match bytes {
        0..1_000 => format!("{bytes} bytes"),
        1_000..1_000_000 => format!("{:.0} KB", bytes as f64 / 1e3),
        _ => format!("{:.1} MB", bytes as f64 / 1e6),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratatui::layout::Size;
    use terra_sim::DataPack;
    use terra_tui::args::Args;
    use terra_tui::start;
    use terra_tui::theme::{ColourPalette, GlyphSet, Theme};
    use terra_tui::ui;

    use crate::clip::{self, Framing, View};
    use crate::options::Options;
    use crate::test_util::glyphs;
    use crate::{gif_file, svg_file};

    /// FNV-1a, 64 bits: enough to tell two files apart.
    fn hash(bytes: &[u8]) -> u64 {
        bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    #[test]
    fn the_same_clip_comes_out_the_same_on_every_computer() {
        // The opening scene's clip, as a GIF and an SVG, with lettering,
        // box lines, bold and most of the map's glyphs in it: CI runs this
        // on Linux and Windows, so the same command makes the same file on
        // both (#241). A change to how the game draws, or how the renderer
        // does, changes these.
        let world = start::title_world(&Args::default(), 7, (100, 30)).unwrap();
        let framing = Framing {
            screen: Size::new(100, 30),
            view: View::Screen(ui::Parts::ALL),
            frame: Duration::from_millis(100),
        };
        let clip = clip::title_clip(world, Theme::cp437(), 7, None, &framing);
        let (mut gif, mut svg) = (Vec::new(), Vec::new());
        gif_file::write(&clip, &mut glyphs(), &mut gif).unwrap();
        svg_file::write(&clip, &mut glyphs(), &mut svg).unwrap();
        assert_eq!(
            (hash(&gif), hash(&svg)),
            (0xf92a_99c9_840f_15fd, 0x58cb_4cc7_52c3_425a),
            "{} and {} bytes",
            gif.len(),
            svg.len()
        );
    }

    #[test]
    fn a_clip_draws_the_glyphs_and_the_palette_the_flags_pick() {
        // Design §6.2 (#307): `--colours` picks the palette over `--theme`'s,
        // whose file then gives only the glyphs. The renderer reads neither
        // the settings nor `NO_COLOR`, so a clip is the same anywhere.
        let data = DataPack::builtin().unwrap();
        let theme = |line: &str| {
            let options = Options::parse(line.split_whitespace().map(String::from)).unwrap();
            super::theme(&options, &data).unwrap()
        };
        assert_eq!(theme("world -o a.gif"), Theme::cp437());
        assert_eq!(
            theme("world --colours mono -o a.gif"),
            Theme::built_in(GlyphSet::Cp437, ColourPalette::Mono)
        );
        assert_eq!(
            theme("title --ascii --colours high-contrast -o a.gif"),
            Theme::built_in(GlyphSet::Ascii, ColourPalette::HighContrast)
        );
        // The repository's own ASCII file, read and not written.
        assert_eq!(
            theme("world --theme ../../themes/ascii.ron --colours colour-blind -o a.gif"),
            Theme::built_in(GlyphSet::Ascii, ColourPalette::ColourBlind)
        );
        assert_eq!(
            theme("world --theme ../../themes/ascii.ron -o a.gif"),
            Theme::ascii()
        );
    }

    #[test]
    fn two_sample_fonts_of_one_name_are_refused() {
        // Both would write fonts-cascadia-mono.gif (#241); the check comes
        // before the file is read, so it needn't exist.
        let err = super::sample_fonts(&["new/Cascadia-Mono.ttf".into()])
            .err()
            .unwrap();
        assert!(err.contains("rename one"), "{err}");
    }
}
