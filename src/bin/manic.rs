//! The `manic` CLI: run a `.manic` file live, record it, or check it.
//!
//!   manic FILE.manic                 # live preview window
//!   manic check FILE.manic           # parse + report errors, no window
//!   manic FILE.manic --still 2.0     # export one PNG frame at t=2s
//!   manic FILE.manic --record out    # render to out/out.mp4 (needs ffmpeg)
//!   manic FILE.manic --canvas square # reframe one responsive source
//!   manic check FILE.manic --canvas all # visual audit across four formats
//!
//! Recording/still/CRT are the same flags the engine understands
//! (`--record DIR`, `--still S`, `--fps N`, `--scale F`, `--from/--to`,
//! `--gif`, `--png`, `--alpha`, `--crt`); they pass straight through.

use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let subcommand = matches!(args.first().map(String::as_str), Some("check" | "play"));
    let check = args.first().map(String::as_str) == Some("check");

    let file = args.iter().find(|a| a.ends_with(".manic")).cloned();
    let Some(file) = file else {
        eprintln!(
            "manic — a language for animated explainers\n\n\
             usage:\n  \
             manic FILE.manic                 live preview\n  \
             manic check FILE.manic           parse + report errors\n  \
             manic FILE.manic --still 2.0     export one PNG at t=2s\n  \
             manic FILE.manic --record out    render out/out.mp4 (needs ffmpeg)\n  \
             manic FILE.manic --canvas square reframe responsive source\n  \
             manic check FILE.manic --canvas all audit portrait/feed/square/landscape\n\n\
             flags: --canvas FORMAT  --fps N  --scale F  --from S --to S  --gif  --png  --alpha  --crt  --intro"
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
