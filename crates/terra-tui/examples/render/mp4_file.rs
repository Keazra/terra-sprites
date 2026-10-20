//! A clip as an MP4 (#241), for video sites: made by ffmpeg, if it's
//! installed, from the clip's frames in full colour.

use std::io::{ErrorKind, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::clip::Clip;
use crate::glyphs::Glyphs;
use crate::picture;

/// The program that makes MP4s.
pub const FFMPEG: &str = "ffmpeg";

/// How many threads x264 encodes in. Its output depends on how many, so
/// it's the same on every machine, for the same clip to make the same
/// file.
const THREADS: u32 = 4;

/// Writes `clip` to the file `out` as an MP4, its characters drawn by
/// `glyphs`, through `program`, which is ffmpeg.
pub fn write(clip: &Clip, glyphs: &mut Glyphs, out: &Path, program: &str) -> Result<(), String> {
    let first = clip.frames.first().ok_or("the clip has no frames")?;
    let (width, height) = picture::pixels(glyphs, first.screen.area.as_size());
    let tick = video_frame(clip);
    let mut ffmpeg = Command::new(program)
        .args(arguments(width, height, tick, out))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| match err.kind() {
            ErrorKind::NotFound => format!(
                "making an MP4 takes ffmpeg, and there's no {program} here: install it \
                 (https://ffmpeg.org/download.html) and see that it's on the PATH, or make \
                 a GIF or an SVG, which need nothing else"
            ),
            _ => format!("couldn't start {program}: {err}"),
        })?;
    // What ffmpeg says, read as it says it, so it never waits to say more.
    let mut said = ffmpeg.stderr.take().expect("ffmpeg's errors are piped");
    let saying = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = said.read_to_string(&mut text);
        text
    });
    let mut input = ffmpeg.stdin.take().expect("ffmpeg's input is piped");
    let mut sent = Ok(());
    for frame in &clip.frames {
        let rgb = picture::rgb(glyphs, &frame.screen);
        let repeats = frame.shows_for.as_millis() / tick.as_millis();
        sent = (0..repeats).try_for_each(|_| input.write_all(&rgb));
        if sent.is_err() {
            break;
        }
    }
    // Closing its input tells ffmpeg the clip is over.
    drop(input);
    let status = ffmpeg
        .wait()
        .map_err(|err| format!("{program} didn't finish: {err}"))?;
    let said = saying.join().unwrap_or_default();
    if !status.success() {
        let said = said.trim();
        return Err(format!(
            "{program} couldn't make the MP4 ({status}): {said}"
        ));
    }
    sent.map_err(|err| format!("couldn't send {program} the frames: {err}"))
}

/// How long a frame of the video shows: as long as every frame of the
/// clip shows a whole number of them.
fn video_frame(clip: &Clip) -> Duration {
    let gcd = |mut a: u128, mut b: u128| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    };
    let millis = clip
        .frames
        .iter()
        .map(|frame| frame.shows_for.as_millis())
        .fold(0, gcd)
        .max(1);
    Duration::from_millis(millis as u64)
}

/// ffmpeg's arguments to make the MP4 `out` from frames `width` by
/// `height` pixels, each shown for `frame`, sent to it in full colour.
fn arguments(width: usize, height: usize, frame: Duration, out: &Path) -> Vec<String> {
    let input = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgb24",
        "-s",
        &format!("{width}x{height}"),
        "-framerate",
        &format!("1000/{}", frame.as_millis()),
        "-i",
        "-",
    ];
    // Video is in pixels two by two, so an odd edge gets a row of the
    // terminal's background; its colours are those of HD video, said so.
    let filters = "pad=ceil(iw/2)*2:ceil(ih/2)*2:color=0x0C0C0C,\
                   scale=out_color_matrix=bt709:out_range=tv,format=yuv420p";
    let output = [
        "-vf",
        filters,
        "-c:v",
        "libx264",
        "-tune",
        "animation",
        "-crf",
        "18",
        "-threads",
        &THREADS.to_string(),
        "-colorspace",
        "bt709",
        "-color_primaries",
        "bt709",
        "-color_trc",
        "bt709",
        "-color_range",
        "tv",
        "-movflags",
        "+faststart",
        // Nothing in the file that changes from run to run.
        "-map_metadata",
        "-1",
        "-fflags",
        "+bitexact",
        "-flags:v",
        "+bitexact",
    ];
    input
        .into_iter()
        .chain(output)
        .map(str::to_string)
        .chain([out.display().to_string()])
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::style::Color;

    use super::*;
    use crate::clip::Frame;
    use crate::test_util::{self, glyphs};

    /// A blank frame of three cells, for `millis`.
    fn frame(millis: u64) -> Frame {
        test_util::frame(3, "", Color::Reset, millis)
    }

    #[test]
    fn a_video_frame_is_as_long_as_every_frame_is_a_whole_number_of() {
        let clip = Clip {
            frames: vec![frame(300), frame(200), frame(1000)],
        };
        assert_eq!(video_frame(&clip), Duration::from_millis(100));
        let clip = Clip {
            frames: vec![frame(50), frame(150)],
        };
        assert_eq!(video_frame(&clip), Duration::from_millis(50));
    }

    #[test]
    fn ffmpeg_is_sent_the_frames_in_full_colour_at_their_rate() {
        let arguments = arguments(
            1000,
            600,
            Duration::from_millis(100),
            &PathBuf::from("a.mp4"),
        );
        let said = arguments.join(" ");
        assert!(said.contains("-f rawvideo -pix_fmt rgb24 -s 1000x600 -framerate 1000/100 -i -"));
        assert!(said.contains("-c:v libx264"));
        assert!(
            said.contains("-threads 4"),
            "the same file on every machine"
        );
        assert_eq!(arguments.last().map(String::as_str), Some("a.mp4"));
    }

    #[test]
    fn without_ffmpeg_it_says_how_to_get_it() {
        let clip = Clip {
            frames: vec![frame(100)],
        };
        let out = PathBuf::from("never-written.mp4");
        let err = write(&clip, &mut glyphs(), &out, "no-such-ffmpeg-here").unwrap_err();
        assert!(err.contains("takes ffmpeg"), "{err}");
        assert!(err.contains("GIF or an SVG"), "{err}");
        assert!(!out.exists());
    }
}
