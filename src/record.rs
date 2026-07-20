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

use crate::movie::StoryStage;

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
                        "-loglevel".into(),
                        "error".into(),
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
                        "-loglevel".into(),
                        "error".into(),
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
                .stderr(Stdio::inherit()) // let ffmpeg errors reach the terminal
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
                // frames arrive top-down (ffmpeg's orientation), but `export_png`
                // flips internally — so pre-flip to land upright on disk.
                let mut im = img.clone();
                flip_rows(&mut im);
                let path = self.dir.join(format!("frame_{:05}.png", self.frame));
                im.export_png(path.to_str().expect("non-utf8 record path"));
            }
            Sink::Pipe { child, .. } => {
                let w = child
                    .stdin
                    .as_mut()
                    .expect("ffmpeg stdin")
                    .write_all(&img.bytes);
                if let Err(e) = w {
                    // ffmpeg closed the pipe — its own error (dimensions, codec,
                    // …) printed above via inherited stderr. Fail with a pointer.
                    panic!(
                        "ffmpeg stopped reading frames ({e}). See its error above; \
                         try `--scale 1` or a `--preset test` render, or check ffmpeg is current."
                    );
                }
            }
        }
        self.frame += 1;
    }

    pub fn frames(&self) -> u32 {
        self.frame
    }

    /// Close the sink and write the historical whole-movie `markers.json`.
    /// Kept for Rust API compatibility; the Manic player uses
    /// [`Recorder::finish_range`] for stage-aware clip metadata.
    pub fn finish(self, sections: &[(f32, String)], marks: &[(f32, String)]) {
        let markers = legacy_markers_json(self.fps, sections, marks);
        self.finish_with_markers(markers);
    }

    /// Close the sink and write clip-relative `markers.json` (source range,
    /// story stages, sections, and beat marks) for editor alignment.
    pub fn finish_range(
        self,
        sections: &[(f32, String)],
        marks: &[(f32, String)],
        stages: &[StoryStage],
        from: f32,
        to: f32,
    ) {
        let markers = markers_json(self.fps, sections, marks, stages, from, to);
        self.finish_with_markers(markers);
    }

    fn finish_with_markers(mut self, markers: String) {
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

fn legacy_markers_json(fps: u32, sections: &[(f32, String)], marks: &[(f32, String)]) -> String {
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

fn markers_json(
    fps: u32,
    sections: &[(f32, String)],
    marks: &[(f32, String)],
    stages: &[StoryStage],
    from: f32,
    to: f32,
) -> String {
    fn list(items: &[(f32, String)], from: f32, to: f32) -> String {
        items
            .iter()
            .filter(|(t, _)| *t >= from && *t < to)
            .map(|(t, name)| {
                format!(
                    "    {{\"t\": {:.3}, \"source_t\": {t:.3}, \"name\": {:?}}}",
                    t - from,
                    name
                )
            })
            .collect::<Vec<_>>()
            .join(",\n")
    }
    let stage_list = stages
        .iter()
        .filter(|stage| stage.end > from && stage.start < to)
        .map(|stage| {
            let start = stage.start.max(from) - from;
            let end = stage.end.min(to) - from;
            format!(
                "    {{\"t\": {start:.3}, \"end\": {end:.3}, \"duration\": {:.3}, \"source_t\": {:.3}, \"name\": {:?}}}",
                (end - start).max(0.0),
                stage.start,
                stage.name,
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"fps\": {fps},\n  \"source_range\": {{\"from\": {from:.3}, \"to\": {to:.3}, \"duration\": {:.3}}},\n  \"stages\": [\n{stage_list}\n  ],\n  \"sections\": [\n{}\n  ],\n  \"marks\": [\n{}\n  ]\n}}\n",
        (to - from).max(0.0),
        list(sections, from, to),
        list(marks, from, to)
    )
}

/// Mirror an image top-to-bottom in place (row-swap of RGBA pixels). Used to
/// cancel `Image::export_png`'s internal flip so PNG frames land upright.
fn flip_rows(img: &mut Image) {
    let (w, h) = (img.width as usize, img.height as usize);
    let stride = w * 4;
    for y in 0..h / 2 {
        let (top, bot) = (y * stride, (h - 1 - y) * stride);
        for i in 0..stride {
            img.bytes.swap(top + i, bot + i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::markers_json;
    use crate::movie::StoryStage;

    #[test]
    fn selected_recording_metadata_is_filtered_clipped_and_relative() {
        let points = vec![
            (0.0, "question".to_string()),
            (2.0, "experiment".to_string()),
            (4.0, "proof".to_string()),
        ];
        let stages = vec![
            StoryStage {
                name: "question".into(),
                start: 0.0,
                end: 2.0,
            },
            StoryStage {
                name: "experiment".into(),
                start: 2.0,
                end: 4.0,
            },
            StoryStage {
                name: "proof".into(),
                start: 4.0,
                end: 6.0,
            },
        ];
        let raw = markers_json(30, &[], &points, &stages, 2.0, 5.5);
        assert!(
            raw.contains("\"source_range\": {\"from\": 2.000, \"to\": 5.500, \"duration\": 3.500}"),
            "{raw}"
        );
        assert!(!raw.contains("\"name\": \"question\""), "{raw}");
        assert!(
            raw.contains("{\"t\": 0.000, \"source_t\": 2.000, \"name\": \"experiment\"}"),
            "{raw}"
        );
        assert!(
            raw.contains(
                "{\"t\": 2.000, \"end\": 3.500, \"duration\": 1.500, \"source_t\": 4.000, \"name\": \"proof\"}"
            ),
            "{raw}"
        );
    }
}
