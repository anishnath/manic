//! Render **presets**: named bundles of output defaults — quality (`scale`),
//! frame rate, container, and whether the output carries manic's branding. A
//! preset is only the *baseline*; any runtime flag (`--scale`, `--fps`,
//! `--gif`, `--no-brand`, …) overrides its fields. `--preset <name>` selects one;
//! the default is `studio`.
//!
//! Branding (the pre-roll intro + the "Made With Manic" watermark) is applied to
//! **recorded** output under a branded preset — never to the live preview or a
//! still, and never authored in the DSL. So the fast verify loop
//! (`cargo run --bin manic -- examples/x.manic`) stays clean, while
//! `--record` under `studio`/`reel` produces the branded video.

/// A named set of output defaults.
#[derive(Debug, Clone, Copy)]
pub struct Preset {
    pub name: &'static str,
    /// Supersample factor for recorded output (1.5 × a 720p canvas → 1080p).
    pub scale: f32,
    pub fps: u32,
    /// Encode as GIF instead of MP4.
    pub gif: bool,
    /// Prepend the intro banner + pin the watermark on recorded output.
    pub branded: bool,
}

/// Default: full-quality, branded 1080p MP4.
pub const STUDIO: Preset = Preset { name: "studio", scale: 1.5, fps: 60, gif: false, branded: true };
/// Fast, unbranded — for quick verification.
pub const TEST: Preset = Preset { name: "test", scale: 1.0, fps: 30, gif: false, branded: false };
/// Branded, for vertical/social clips (pair with a `canvas("9:16")` file).
pub const REEL: Preset = Preset { name: "reel", scale: 1.5, fps: 60, gif: false, branded: true };

/// Look a preset up by name (with a few friendly aliases).
pub fn by_name(name: &str) -> Option<Preset> {
    match name.trim().to_ascii_lowercase().as_str() {
        "studio" | "default" => Some(STUDIO),
        "test" | "draft" | "dev" | "preview" => Some(TEST),
        "reel" | "social" | "vertical" | "story" => Some(REEL),
        _ => None,
    }
}

/// The default preset (`studio`).
pub fn default() -> Preset {
    STUDIO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_presets() {
        assert!(by_name("studio").unwrap().branded);
        assert!(!by_name("test").unwrap().branded);
        assert!(by_name("reel").unwrap().branded);
        assert_eq!(by_name("draft").unwrap().name, "test"); // alias
        assert!(by_name("nope").is_none());
        assert_eq!(default().name, "studio");
    }
}
