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

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use terra_sim::Playback;
use terra_tui::args::Args;
use terra_tui::start;

use clip::{Clip, Source, View};
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

/// Draws the clip `options` ask for and writes it.
fn render(options: &Options) -> Result<(), String> {
    let view = View {
        screen: options.screen,
        map_alone: options.map_alone,
        frame: options.frame,
    };
    let clip = match &options.scene {
        Scene::Title { seconds } => {
            let seed = options.world.seed.unwrap_or_default();
            let size = (options.screen.width, options.screen.height);
            let world = start::title_world(&options.world, seed, size)?;
            let theme = start::theme(&options.world, world.data())?;
            clip::title_clip(world, theme, seed, *seconds, &view)
        }
        Scene::World(scene) => world_clip(options, scene, &view)?,
        // A still: the first tick's screen alone.
        Scene::Samples(scene) => {
            let still = WorldScene {
                to: Some(scene.from),
                ..scene.clone()
            };
            world_clip(options, &still, &view)?
        }
    };
    let fonts = match (&options.font, &options.bold_font) {
        (Some(font), bold) => Fonts::from_files(font, bold.as_deref())?,
        (None, Some(_)) => return Err("--bold-font goes with --font".into()),
        (None, None) => Fonts::cascadia(),
    };
    let mut glyphs = Glyphs::new(fonts, options.cell);
    write(&clip, &mut glyphs, options)?;
    if !glyphs.missing().is_empty() {
        let missing: String = glyphs.missing().iter().collect();
        eprintln!("render: the font has no glyph for {missing}, drawn as its missing-glyph box");
    }
    let (width, height) = picture::pixels(&glyphs, options.screen);
    let bytes = std::fs::metadata(&options.out).map_or(0, |file| file.len());
    let frames = match clip.frames.len() {
        1 => "a still".to_string(),
        n => format!("{n} frames over {:.1} seconds", clip.length().as_secs_f64()),
    };
    println!(
        "Wrote {}: {frames}, {width}x{height} pixels, {}",
        options.out.display(),
        said_bytes(bytes)
    );
    Ok(())
}

/// A world's clip, from the seed or the replay `options` give.
fn world_clip(options: &Options, scene: &WorldScene, view: &View) -> Result<Clip, String> {
    let source = source(&options.world)?;
    let theme = start::theme(&options.world, source.world().data())?;
    clip::world_clip(source, theme, scene, view)
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

/// Writes `clip` where `options` say, in their format.
fn write(clip: &Clip, glyphs: &mut Glyphs, options: &Options) -> Result<(), String> {
    let path = &options.out;
    match options.format {
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
