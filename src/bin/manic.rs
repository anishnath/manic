//! The `manic` CLI: run a `.manic` file live, record it, or check it.
//!
//!   manic FILE.manic                 # live preview window
//!   manic check FILE.manic           # parse + report errors, no window
//!   manic FILE.manic --still 2.0     # export one PNG frame at t=2s
//!   manic FILE.manic --record out    # render to out/out.mp4 (needs ffmpeg)
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
             manic FILE.manic --record out    render out/out.mp4 (needs ffmpeg)\n\n\
             flags: --fps N  --scale F  --from S --to S  --gif  --png  --alpha  --crt"
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

    let movie = match manic::parse(&src) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}\n", manic::lang::diag::render(&src, &e));
            exit(1);
        }
    };

    if check {
        let n = movie.base().entities.len().saturating_sub(1); // minus the camera
        println!("ok — {file}: parses, {n} entities");
        exit(0);
    }

    manic::run(movie);
}
