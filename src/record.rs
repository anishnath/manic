//! Deterministic offline output. Frame `f` is rendered at `t = f / fps`,
//! wall clock ignored, so output is bit-identical across runs.
//!
//! Default sink pipes raw RGBA frames straight into an ffmpeg child process
//! (no intermediate PNGs). `--gif` uses the same pipe with an ffmpeg palette
//! filter. Falls back to a PNG sequence when ffmpeg is missing, or when
//! `--png` / `--alpha` ask for one (alpha needs a lossless-with-alpha
//! container, so it always goes through PNGs).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use macroquad::texture::Image;

enum Sink {
    Png,
    Pipe { child: Child, output: PathBuf },
}

pub struct Recorder {
    pub dir: PathBuf,
    pub fps: u32,
    frame: u32,
    sink: Sink,
}

fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

impl Recorder {
    /// `w`/`h` are the physical frame size in pixels. `force_png` (or
    /// `alpha`) selects the PNG sink even when ffmpeg is present.
    pub fn new(
        dir: impl Into<PathBuf>,
        fps: u32,
        w: u32,
        h: u32,
        force_png: bool,
        gif: bool,
    ) -> std::io::Result<Recorder> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        let sink = if !force_png && ffmpeg_available() {
            let output = dir.join(if gif { "out.gif" } else { "out.mp4" });
            let video_size = format!("{w}x{h}");
            let framerate = fps.to_string();
            let child = Command::new("ffmpeg")
                .args(if gif {
                    vec![
                        "-y".into(),
                        "-f".into(),
                        "rawvideo".into(),
                        "-pixel_format".into(),
                        "rgba".into(),
                        "-video_size".into(),
                        video_size,
                        "-framerate".into(),
                        framerate,
                        "-i".into(),
                        "-".into(),
                        "-filter_complex".into(),
                        "split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse=dither=bayer:bayer_scale=5".into(),
                    ]
                } else {
                    vec![
                        "-y".into(),
                        "-f".into(),
                        "rawvideo".into(),
                        "-pixel_format".into(),
                        "rgba".into(),
                        "-video_size".into(),
                        video_size,
                        "-framerate".into(),
                        framerate,
                        "-i".into(),
                        "-".into(),
                        "-c:v".into(),
                        "libx264".into(),
                        "-crf".into(),
                        "18".into(),
                        "-preset".into(),
                        "slow".into(),
                        "-pix_fmt".into(),
                        "yuv420p".into(),
                    ]
                })
                .arg(&output)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            Sink::Pipe { child, output }
        } else {
            Sink::Png
        };
        Ok(Recorder {
            dir,
            fps,
            frame: 0,
            sink,
        })
    }

    /// Consume one rendered frame.
    pub fn capture(&mut self, img: &Image) {
        match &mut self.sink {
            Sink::Png => {
                let path = self.dir.join(format!("frame_{:05}.png", self.frame));
                img.export_png(path.to_str().expect("non-utf8 record path"));
            }
            Sink::Pipe { child, .. } => {
                child
                    .stdin
                    .as_mut()
                    .expect("ffmpeg stdin")
                    .write_all(&img.bytes)
                    .expect("write frame to ffmpeg");
            }
        }
        self.frame += 1;
    }

    pub fn frames(&self) -> u32 {
        self.frame
    }

    /// Close the sink and write `markers.json` (sections + beat marks) for
    /// narration alignment in the editor.
    pub fn finish(mut self, sections: &[(f32, String)], marks: &[(f32, String)]) {
        let markers = markers_json(self.fps, sections, marks);
        let _ = std::fs::write(self.dir.join("markers.json"), markers);

        match &mut self.sink {
            Sink::Pipe { child, output } => {
                drop(child.stdin.take());
                match child.wait() {
                    Ok(s) if s.success() => {
                        println!("{} frames -> {}", self.frame, output.display());
                    }
                    other => eprintln!("ffmpeg exited abnormally: {other:?}"),
                }
            }
            Sink::Png => {
                let pattern = self.dir.join("frame_%05d.png");
                println!("{} frames written to {}/", self.frame, self.dir.display());
                println!(
                    "stitch with:\n  ffmpeg -framerate {} -i {} -c:v libx264 -crf 18 -pix_fmt yuv420p {}",
                    self.fps,
                    pattern.display(),
                    self.dir.join("out.mp4").display()
                );
                println!(
                    "  (alpha: -c:v qtrle {} instead)",
                    self.dir.join("out.mov").display()
                );
            }
        }
        println!("markers: {}", self.dir.join("markers.json").display());
    }
}

fn markers_json(fps: u32, sections: &[(f32, String)], marks: &[(f32, String)]) -> String {
    fn list(items: &[(f32, String)]) -> String {
        items
            .iter()
            .map(|(t, name)| format!("    {{\"t\": {t:.3}, \"name\": {:?}}}", name))
            .collect::<Vec<_>>()
            .join(",\n")
    }
    format!(
        "{{\n  \"fps\": {fps},\n  \"sections\": [\n{}\n  ],\n  \"marks\": [\n{}\n  ]\n}}\n",
        list(sections),
        list(marks)
    )
}
