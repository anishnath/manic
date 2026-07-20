//! The `manic` CLI: run a `.manic` file live, record it, or check it.
//!
//!   manic FILE.manic                 # live preview window
//!   manic check FILE.manic           # parse + report errors, no window
//!   manic stages FILE.manic          # list named story stages + durations
//!   manic FILE.manic --still 2.0     # export one PNG frame at t=2s
//!   manic FILE.manic --record out    # render to out/out.mp4 (needs ffmpeg)
//!   manic FILE.manic --stage proof   # preview/record one named stage
//!   manic FILE.manic --canvas square # reframe one responsive source
//!   manic check FILE.manic --canvas all # visual audit across four formats
//!
//! Recording/still/CRT are the same flags the engine understands
//! (`--record DIR`, `--still S`, `--fps N`, `--scale F`, `--from/--to`,
//! `--gif`, `--png`, `--alpha`, `--crt`); they pass straight through.

use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let subcommand = matches!(
        args.first().map(String::as_str),
        Some("check" | "play" | "stages")
    );
    let check = args.first().map(String::as_str) == Some("check");
    let list_stages = args.first().map(String::as_str) == Some("stages");

    let file = args.iter().find(|a| a.ends_with(".manic")).cloned();
    let Some(file) = file else {
        eprintln!(
            "manic — a language for animated explainers\n\n\
             usage:\n  \
             manic FILE.manic                 live preview\n  \
             manic check FILE.manic           parse + report errors\n  \
             manic stages FILE.manic          list named stages + durations\n  \
             manic FILE.manic --stage NAME    preview/record one stage\n  \
             manic FILE.manic --still 2.0     export one PNG at t=2s\n  \
             manic FILE.manic --record out    render out/out.mp4 (needs ffmpeg)\n  \
             manic FILE.manic --canvas square reframe responsive source\n  \
             manic check FILE.manic --canvas all audit portrait/feed/square/landscape\n\n\
             flags: --canvas FORMAT  --stage NAME  --from-stage NAME  --to-stage NAME\n       --fps N  --scale F  --from S --to S  --gif  --png  --alpha  --crt  --intro"
        );
        exit(2);
    };
    let _ = subcommand; // `play` is accepted for readability; behaviour is flag-driven

    let src = match std::fs::read_to_string(&file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("manic: cannot read {file}: {e}");
            exit(2);
        }
    };

    let canvas_value = args
        .iter()
        .position(|a| a == "--canvas")
        .map(|i| {
            args.get(i + 1)
                .ok_or_else(|| "--canvas expects a format".to_string())
        })
        .transpose();
    let canvas_value = match canvas_value {
        Ok(value) => value,
        Err(message) => {
            eprintln!("manic: --canvas: {message}");
            exit(2);
        }
    };

    if canvas_value.map(String::as_str) == Some("all") {
        if !check {
            eprintln!("manic: --canvas all is a publishing audit; use it with `manic check`");
            exit(2);
        }
        visual_check_all(&file, &src);
    }

    let canvas = canvas_value
        .map(|value| manic_lang::expand::canvas_override_dims(value))
        .transpose();
    let canvas = match canvas {
        Ok(value) => value,
        Err(message) => {
            eprintln!("manic: --canvas: {message}");
            exit(2);
        }
    };

    let movie = match canvas {
        Some((w, h)) => manic::parse_with_canvas(&src, w, h),
        None => manic::parse(&src),
    };
    let movie = match movie {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}\n", manic::lang::diag::render(&src, &e));
            exit(1);
        }
    };

    if list_stages {
        if let Err(message) = movie.validate() {
            eprintln!("error — {file}: {message}\n");
            exit(1);
        }
        match stages_report(&file, &movie) {
            Ok(report) => {
                print!("{report}");
                exit(0);
            }
            Err(message) => {
                eprintln!("manic: {message}");
                exit(1);
            }
        }
    }

    if check {
        let n = movie.base().entities.len().saturating_sub(1)
            + movie.base().entities_3d.len().saturating_sub(usize::from(
                movie.base().get_3d(manic::movie::CAMERA3_ID).is_some(),
            ));
        // Whole-file sanity check: parse already built the scene; now verify the
        // timeline references only real entities (catches the render-time
        // "unknown entity id" panic class before it ever reaches a window).
        if let Err(msg) = movie.validate() {
            eprintln!("error — {file}: {msg}\n");
            exit(1);
        }
        println!("ok — {file}: parses + validates, {n} entities");
        exit(0);
    }

    manic::run(movie);
}

fn stages_report(file: &str, movie: &manic::movie::Movie) -> Result<String, String> {
    let stages = movie.stage_ranges();
    if stages.is_empty() {
        return Err(format!(
            "{file} has no named story stages; add `step(\"name\") {{ ... }}`"
        ));
    }
    let name_width = stages
        .iter()
        .map(|stage| stage.name.chars().count())
        .max()
        .unwrap_or(5)
        .max(5);
    let mut out = format!(
        "stages — {file} ({} stages, {:.2}s authored)\n\n",
        stages.len(),
        movie.content_duration()
    );
    out.push_str(&format!(
        "  #  {:name_width$}    start      end      duration\n",
        "stage"
    ));
    for (index, stage) in stages.iter().enumerate() {
        out.push_str(&format!(
            "{:>3}  {:name_width$}  {:>7.2}s  {:>7.2}s  {:>8.2}s\n",
            index + 1,
            stage.name,
            stage.start,
            stage.end,
            stage.duration(),
        ));
    }
    Ok(out)
}

fn visual_check_all(file: &str, src: &str) -> ! {
    let formats = [
        ("portrait", 1080, 1920),
        ("feed", 1080, 1350),
        ("square", 1080, 1080),
        ("landscape", 1280, 720),
    ];
    let mut found = 0usize;

    for (format, width, height) in formats {
        let movie = match manic::parse_with_canvas(src, width, height) {
            Ok(movie) => movie,
            Err(error) => {
                eprintln!("error [{format}] — {file}");
                eprintln!("{}\n", manic::lang::diag::render(src, &error));
                exit(1);
            }
        };
        if let Err(message) = movie.validate() {
            eprintln!("error [{format}] — {file}: {message}\n");
            exit(1);
        }

        let diagnostics = manic::audit::visual_diagnostics(&movie, format);
        if diagnostics.is_empty() {
            println!("ok — {file} [{format}]: visual checks passed");
            continue;
        }
        found += diagnostics.len();
        for diagnostic in diagnostics {
            eprintln!(
                "{} [{} · {} @ {:.2}s]: `{}` {}",
                diagnostic.severity.as_str(),
                diagnostic.format,
                diagnostic.stage,
                diagnostic.at,
                diagnostic.entity,
                diagnostic.message
            );
            if let Some(other) = diagnostic.other {
                eprintln!("  entities: `{}` and `{other}`", diagnostic.entity);
            } else {
                eprintln!("  entity: `{}`", diagnostic.entity);
            }
            eprintln!("  suggestion: {}\n", diagnostic.suggestion);
        }
    }

    if found == 0 {
        println!("ok — {file}: visual audit passed all 4 formats");
        exit(0);
    }
    eprintln!("visual check failed — {file}: {found} issue(s) across 4 formats");
    exit(1);
}

#[cfg(test)]
mod tests {
    use super::stages_report;

    #[test]
    fn stage_report_is_human_readable_and_includes_holds() {
        let movie = manic::parse(
            "step(\"question\") { wait(1); } wait(0.5);\n\
             step(\"proof\") { wait(2); } wait(0.25);",
        )
        .unwrap();
        let report = stages_report("lesson.manic", &movie).unwrap();
        assert!(report.contains("2 stages, 3.75s authored"), "{report}");
        assert!(report.contains("question"));
        assert!(report.contains("0.00s"));
        assert!(report.contains("1.50s"));
        assert!(report.contains("proof"));
        assert!(report.contains("2.25s"));
    }

    #[test]
    fn stage_report_explains_how_to_add_structure() {
        let movie = manic::parse("wait(1);").unwrap();
        let error = stages_report("plain.manic", &movie).unwrap_err();
        assert!(error.contains("step(\"name\")"), "{error}");
    }
}
