//! Deterministic bundled-only shaping and rasterization for ordinary text.
//!
//! LaTeX remains on RaTeX. This engine owns every ordinary-text layout so the
//! same shaped glyph jobs drive measurement, wrapping, bidi placement, reveal,
//! rasterization, native recording, and the WASM renderer.

use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use cosmic_text::{
    fontdb, Align as CosmicAlign, Attrs, Buffer, CacheKey, Fallback, Family, FontSystem, Metrics,
    Shaping, SwashCache, SwashContent, Weight, Wrap,
};
use unicode_script::Script;
use unicode_segmentation::UnicodeSegmentation;

use crate::primitives::{Align, FontKind};
use crate::style::{
    NOTO_ARABIC_BYTES, NOTO_DEVANAGARI_BYTES, NOTO_MATH_BYTES, NOTO_SANS_BYTES,
    NOTO_SYMBOLS2_BYTES, NOTO_SYMBOLS_BYTES, PLEX_BOLD_BYTES, PLEX_REGULAR_BYTES,
};

const CACHE_LAYOUT_LIMIT: usize = 512;
const CACHE_RASTER_LIMIT: usize = 1024;
const RASTER_PAD: i32 = 8;

#[derive(Debug)]
struct BundledFallback;

impl Fallback for BundledFallback {
    fn common_fallback(&self) -> &[&'static str] {
        &[
            "Noto Sans Math",
            "Noto Sans",
            "Noto Sans Symbols",
            "Noto Sans Symbols2",
            "Noto Sans Arabic",
            "Noto Sans Devanagari",
        ]
    }

    fn forbidden_fallback(&self) -> &[&'static str] {
        &[]
    }

    fn script_fallback(&self, script: Script, _locale: &str) -> &[&'static str] {
        match script {
            Script::Arabic => &["Noto Sans Arabic"],
            Script::Devanagari => &["Noto Sans Devanagari"],
            _ => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct LayoutKey {
    text: String,
    kind: u8,
    raster_bits: u32,
    width_bits: Option<u32>,
    align: u8,
}

impl LayoutKey {
    pub(crate) fn new(
        text: &str,
        kind: FontKind,
        raster_size: f32,
        max_width: Option<f32>,
        align: Align,
    ) -> Self {
        Self {
            text: text.replace("\\n", "\n"),
            kind: font_kind_id(kind),
            raster_bits: raster_size.max(1.0).to_bits(),
            width_bits: max_width
                .filter(|width| width.is_finite() && *width > 0.0)
                .map(f32::to_bits),
            align: match align {
                Align::Center => 0,
                Align::Left => 1,
            },
        }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    fn raster_size(&self) -> f32 {
        f32::from_bits(self.raster_bits)
    }

    fn max_width(&self) -> Option<f32> {
        self.width_bits.map(f32::from_bits)
    }

    fn font_kind(&self) -> FontKind {
        match self.kind {
            0 => FontKind::Display,
            2 => FontKind::MonoBold,
            _ => FontKind::Mono,
        }
    }
}

#[derive(Debug, Clone)]
struct GlyphJob {
    line_i: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    start: usize,
    end: usize,
    cache_key: CacheKey,
    x: i32,
    y: i32,
}

#[derive(Debug)]
pub(crate) struct ShapedLayout {
    pub(crate) key: LayoutKey,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) ascent: f32,
    pub(crate) descent: f32,
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) line_count: usize,
    pub(crate) rtl: bool,
    pub(crate) graphemes: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) families: BTreeSet<String>,
    glyphs: Vec<GlyphJob>,
}

impl ShapedLayout {
    /// Stable CPU-layout fingerprint used by native/backend/WASM parity tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn fingerprint(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.key.hash(&mut hasher);
        self.width.to_bits().hash(&mut hasher);
        self.height.to_bits().hash(&mut hasher);
        self.ascent.to_bits().hash(&mut hasher);
        self.descent.to_bits().hash(&mut hasher);
        self.rtl.hash(&mut hasher);
        self.families.hash(&mut hasher);
        for glyph in &self.glyphs {
            glyph.line_i.hash(&mut hasher);
            glyph.start.hash(&mut hasher);
            glyph.end.hash(&mut hasher);
            format!("{:?}", glyph.cache_key.font_id).hash(&mut hasher);
            glyph.cache_key.glyph_id.hash(&mut hasher);
            glyph.x.hash(&mut hasher);
            glyph.y.hash(&mut hasher);
        }
        hasher.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RasterKey {
    pub(crate) layout: LayoutKey,
    pub(crate) visible_graphemes: usize,
}

#[derive(Debug)]
pub(crate) struct RasterImage {
    pub(crate) width: u16,
    pub(crate) height: u16,
    pub(crate) pixels: Vec<u8>,
    pub(crate) pad: f32,
}

impl RasterImage {
    #[cfg(test)]
    fn alpha_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let alpha = self
            .pixels
            .iter()
            .skip(3)
            .step_by(4)
            .copied()
            .collect::<Vec<_>>();
        Sha256::digest(alpha)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

/// One mutable shaping/raster cache per playback session. It is deliberately
/// host-font-free: callers provide the exact embedded database below.
pub(crate) struct TextEngine {
    font_system: FontSystem,
    swash: SwashCache,
    layouts: HashMap<LayoutKey, Arc<ShapedLayout>>,
    rasters: HashMap<RasterKey, Arc<RasterImage>>,
}

impl std::fmt::Debug for TextEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextEngine")
            .field("bundled_faces", &self.font_system.db().len())
            .field("layouts", &self.layouts.len())
            .field("rasters", &self.rasters.len())
            .finish()
    }
}

impl TextEngine {
    pub(crate) fn new() -> Self {
        let mut db = fontdb::Database::new();
        for bytes in [
            PLEX_REGULAR_BYTES,
            PLEX_BOLD_BYTES,
            NOTO_MATH_BYTES,
            NOTO_SANS_BYTES,
            NOTO_SYMBOLS_BYTES,
            NOTO_SYMBOLS2_BYTES,
            NOTO_ARABIC_BYTES,
            NOTO_DEVANAGARI_BYTES,
        ] {
            db.load_font_source(fontdb::Source::Binary(Arc::new(bytes.to_vec())));
        }
        db.set_monospace_family("IBM Plex Mono");
        db.set_sans_serif_family("Noto Sans");
        let font_system = FontSystem::new_with_locale_and_db_and_fallback(
            "en-US".to_string(),
            db,
            BundledFallback,
        );
        Self {
            font_system,
            swash: SwashCache::new(),
            layouts: HashMap::new(),
            rasters: HashMap::new(),
        }
    }

    pub(crate) fn layout(&mut self, key: LayoutKey) -> Arc<ShapedLayout> {
        if let Some(layout) = self.layouts.get(&key) {
            return Arc::clone(layout);
        }
        if self.layouts.len() >= CACHE_LAYOUT_LIMIT {
            self.layouts.clear();
            self.rasters.clear();
        }

        let raster = key.raster_size();
        let metrics = Metrics::relative(raster, 1.4);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        buffer.set_size(key.max_width(), None);
        buffer.set_wrap(if key.max_width().is_some() {
            Wrap::WordOrGlyph
        } else {
            Wrap::None
        });
        let weight = match key.font_kind() {
            FontKind::Mono => Weight::NORMAL,
            FontKind::Display | FontKind::MonoBold => Weight::BOLD,
        };
        let attrs = Attrs::new()
            .family(Family::Name("IBM Plex Mono"))
            .weight(weight);
        let alignment = match key.align {
            0 => Some(CosmicAlign::Center),
            _ => Some(CosmicAlign::Left),
        };
        buffer.set_text(key.text(), &attrs, Shaping::Advanced, alignment);
        buffer.shape_until_scroll(&mut self.font_system, false);

        let mut glyphs = Vec::new();
        let mut families = BTreeSet::new();
        let mut width = 0.0f32;
        let mut height = metrics.line_height;
        let mut ascent = 0.0f32;
        let mut descent = 0.0f32;
        let mut line_count = 0usize;
        let mut rtl = false;
        for run in buffer.layout_runs() {
            width = width.max(run.line_w);
            height = height.max(run.line_top + run.line_height);
            if run.line_i == 0 {
                ascent = ascent.max(run.line_y - run.line_top);
                descent = descent.max(run.line_height - (run.line_y - run.line_top));
            }
            line_count += 1;
            rtl |= run.rtl;
            for glyph in run.glyphs {
                if let Some(face) = self.font_system.db().face(glyph.font_id) {
                    if let Some((family, _)) = face.families.first() {
                        families.insert(family.clone());
                    }
                }
                let physical = glyph.physical((0.0, run.line_y), 1.0);
                glyphs.push(GlyphJob {
                    line_i: run.line_i,
                    start: glyph.start,
                    end: glyph.end,
                    cache_key: physical.cache_key,
                    x: physical.x,
                    y: physical.y,
                });
            }
        }
        // When alignment is applied inside a bounded buffer, glyph x positions
        // are relative to that full buffer. Preserve the buffer width in the
        // cached layout/raster instead of cropping centred/rightward lines.
        width = key.max_width().unwrap_or(width).max(width);
        let shaped = Arc::new(ShapedLayout {
            key: key.clone(),
            width,
            height,
            ascent,
            descent,
            line_count: line_count.max(1),
            rtl,
            graphemes: grapheme_count(key.text()),
            families,
            glyphs,
        });
        self.layouts.insert(key, Arc::clone(&shaped));
        shaped
    }

    pub(crate) fn raster(
        &mut self,
        layout: &Arc<ShapedLayout>,
        visible_graphemes: usize,
    ) -> (RasterKey, Arc<RasterImage>) {
        let visible_graphemes = visible_graphemes.min(layout.graphemes);
        let key = RasterKey {
            layout: layout.key.clone(),
            visible_graphemes,
        };
        if let Some(image) = self.rasters.get(&key) {
            return (key, Arc::clone(image));
        }
        if self.rasters.len() >= CACHE_RASTER_LIMIT {
            self.rasters.clear();
        }

        let width = (layout.width.ceil() as i32 + RASTER_PAD * 2).clamp(1, u16::MAX as i32) as u16;
        let height =
            (layout.height.ceil() as i32 + RASTER_PAD * 2).clamp(1, u16::MAX as i32) as u16;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let line_ends = visible_line_byte_ends(layout.key.text(), visible_graphemes);

        for glyph in &layout.glyphs {
            let Some(visible_end) = line_ends.get(glyph.line_i) else {
                continue;
            };
            if glyph.end > *visible_end {
                continue;
            }
            let Some(image) = self
                .swash
                .get_image(&mut self.font_system, glyph.cache_key)
                .clone()
            else {
                continue;
            };
            let origin_x = glyph.x + image.placement.left + RASTER_PAD;
            let origin_y = glyph.y - image.placement.top + RASTER_PAD;
            let channels = match image.content {
                SwashContent::Mask => 1,
                SwashContent::SubpixelMask | SwashContent::Color => 4,
            };
            for py in 0..image.placement.height as i32 {
                let y = origin_y + py;
                if !(0..height as i32).contains(&y) {
                    continue;
                }
                for px in 0..image.placement.width as i32 {
                    let x = origin_x + px;
                    if !(0..width as i32).contains(&x) {
                        continue;
                    }
                    let src_i =
                        (py as usize * image.placement.width as usize + px as usize) * channels;
                    let alpha = match image.content {
                        SwashContent::Mask => image.data[src_i],
                        SwashContent::SubpixelMask => image.data[src_i..src_i + 3]
                            .iter()
                            .copied()
                            .max()
                            .unwrap_or(0),
                        SwashContent::Color => image.data[src_i + 3],
                    };
                    let dst_i = (y as usize * width as usize + x as usize) * 4;
                    let dst_alpha = pixels[dst_i + 3];
                    let out_alpha = alpha.saturating_add(
                        ((dst_alpha as u16 * (255u16 - alpha as u16)) / 255u16) as u8,
                    );
                    pixels[dst_i] = 255;
                    pixels[dst_i + 1] = 255;
                    pixels[dst_i + 2] = 255;
                    pixels[dst_i + 3] = out_alpha;
                }
            }
        }

        let image = Arc::new(RasterImage {
            width,
            height,
            pixels,
            pad: RASTER_PAD as f32,
        });
        self.rasters.insert(key.clone(), Arc::clone(&image));
        (key, image)
    }

    #[cfg(test)]
    fn cache_sizes(&self) -> (usize, usize) {
        (self.layouts.len(), self.rasters.len())
    }
}

fn font_kind_id(kind: FontKind) -> u8 {
    match kind {
        FontKind::Display => 0,
        FontKind::Mono => 1,
        FontKind::MonoBold => 2,
    }
}

fn grapheme_count(text: &str) -> usize {
    text.split('\n')
        .map(|line| line.graphemes(true).count())
        .sum()
}

fn visible_line_byte_ends(text: &str, visible_graphemes: usize) -> Vec<usize> {
    let mut remaining = visible_graphemes;
    text.split('\n')
        .map(|line| {
            let count = line.graphemes(true).count();
            if remaining >= count {
                remaining -= count;
                line.len()
            } else {
                let boundary = line
                    .grapheme_indices(true)
                    .nth(remaining)
                    .map(|(at, _)| at)
                    .unwrap_or(line.len());
                remaining = 0;
                boundary
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(text: &str) -> LayoutKey {
        LayoutKey::new(text, FontKind::Mono, 42.0, None, Align::Left)
    }

    #[test]
    fn bundled_database_is_host_font_free_and_complete() {
        let engine = TextEngine::new();
        assert_eq!(engine.font_system.db().len(), 8);
        let families = engine
            .font_system
            .db()
            .faces()
            .flat_map(|face| face.families.iter().map(|(name, _)| name.as_str()))
            .collect::<BTreeSet<_>>();
        for required in [
            "IBM Plex Mono",
            "Noto Sans",
            "Noto Sans Math",
            "Noto Sans Symbols",
            "Noto Sans Symbols2",
            "Noto Sans Arabic",
            "Noto Sans Devanagari",
        ] {
            assert!(families.contains(required), "missing `{required}`");
        }
    }

    #[test]
    fn advanced_shaping_covers_ligatures_bidi_arabic_and_indic_clusters() {
        let mut engine = TextEngine::new();
        let latin = engine.layout(key("office affine"));
        let arabic = engine.layout(key("التعلّم يجعل الأفكار واضحة"));
        let devanagari = engine.layout(key("ज्ञान से प्रकाश मिलता है"));
        let bidi = engine.layout(key("Manic مرحبا 123"));

        // Plex Mono deliberately keeps Latin glyphs monospaced, so `ffi` is
        // not expected to collapse. Advanced shaping is instead proven by the
        // scripts where joining/reordering is semantically required.
        assert_eq!(latin.line_count, 1);
        assert!(arabic.rtl);
        assert!(
            bidi.glyphs
                .windows(2)
                .any(|pair| pair[0].start > pair[1].start),
            "mixed-direction glyphs should be visually reordered"
        );
        assert!(arabic.families.contains("Noto Sans Arabic"));
        assert!(devanagari.families.contains("Noto Sans Devanagari"));
        assert!(
            devanagari.glyphs.len() < devanagari.key.text().chars().count(),
            "Indic conjuncts should shape into fewer glyphs than input scalars"
        );
    }

    #[test]
    fn one_layout_cache_drives_full_and_revealed_rasters() {
        let mut engine = TextEngine::new();
        let layout = engine.layout(key("e\u{301} → ज्ञान"));
        let same = engine.layout(key("e\u{301} → ज्ञान"));
        assert!(Arc::ptr_eq(&layout, &same));

        let (_, first) = engine.raster(&layout, 1);
        let (_, full) = engine.raster(&layout, layout.graphemes);
        let (_, full_again) = engine.raster(&layout, layout.graphemes);
        assert!(Arc::ptr_eq(&full, &full_again));
        assert_ne!(first.alpha_hash(), full.alpha_hash());
        assert_eq!(engine.cache_sizes(), (1, 2));
    }

    #[test]
    fn say_and_rewrite_text_changes_invalidate_by_content_and_reuse_on_seek() {
        let mut engine = TextEngine::new();
        let before = engine.layout(key("predict → compare"));
        let after = engine.layout(key("predict → compare → update"));
        let before_after_seek = engine.layout(key("predict → compare"));

        assert!(!Arc::ptr_eq(&before, &after));
        assert!(Arc::ptr_eq(&before, &before_after_seek));
        assert_eq!(engine.cache_sizes(), (2, 0));
    }

    #[test]
    fn parity_fixture_has_stable_layout_and_pixel_fingerprints() {
        let mut engine = TextEngine::new();
        let layout = engine.layout(LayoutKey::new(
            "Manic → التعلّم → ज्ञान",
            FontKind::Mono,
            40.0,
            Some(420.0),
            Align::Center,
        ));
        let (_, image) = engine.raster(&layout, layout.graphemes);
        // These values intentionally describe CPU layout/raster output only;
        // native preview, backend recording, and WASM all consume these bytes.
        assert_eq!(layout.fingerprint(), 11_185_033_739_856_291_608);
        assert_eq!(
            image.alpha_hash(),
            "96fee40c0135731e11c08c194b86c7de71d68076b086710add9f62d4dfca28a0"
        );
        assert_eq!((image.width, image.height), (436, 72));
        assert_eq!(
            image.pixels.len(),
            image.width as usize * image.height as usize * 4
        );
        assert!(image
            .pixels
            .iter()
            .skip(3)
            .step_by(4)
            .any(|alpha| *alpha > 0));
    }

    #[test]
    fn wrapped_multiscript_visual_golden_is_transform_independent() {
        let mut engine = TextEngine::new();
        let key = LayoutKey::new(
            "A measured idea wraps cleanly → التعلّم يجعل الأفكار واضحة → ज्ञान",
            FontKind::MonoBold,
            36.0,
            Some(260.0),
            Align::Center,
        );
        let layout = engine.layout(key.clone());
        let same_for_rotated_zoomed_glow = engine.layout(key);
        let (_, image) = engine.raster(&layout, layout.graphemes);

        assert!(Arc::ptr_eq(&layout, &same_for_rotated_zoomed_glow));
        assert_eq!(layout.line_count, 6);
        assert_eq!(layout.fingerprint(), 11_406_229_831_200_116_231);
        assert_eq!(
            image.alpha_hash(),
            "5d0448d92637ed18a954a8001ed17a920452d3399e807a1b08c403372474a95c"
        );
        assert_eq!((image.width, image.height), (276, 319));
    }
}
