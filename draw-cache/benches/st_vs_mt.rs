use criterion::{criterion_group, criterion_main, measurement::WallTime, Bencher, Criterion};
use glyph_brush_draw_cache::*;
use glyph_brush_layout::ab_glyph::*;
use std::sync::LazyLock;

/// Simple paragraph layout for glyphs into `target`.
///
/// This is for testing and examples.
pub fn layout_paragraph<F, SF>(
    font: SF,
    position: Point,
    max_width: f32,
    text: &str,
    target: &mut Vec<Glyph>,
) where
    F: Font,
    SF: ScaleFont<F>,
{
    let v_advance = font.height() + font.line_gap();
    let mut caret = position + point(0.0, font.ascent());
    let mut last_glyph: Option<Glyph> = None;
    for c in text.chars() {
        if c.is_control() {
            if c == '\n' {
                caret = point(position.x, caret.y + v_advance);
                last_glyph = None;
            }
            continue;
        }
        let mut glyph = font.scaled_glyph(c);
        if let Some(previous) = last_glyph.take() {
            caret.x += font.kern(previous.id, glyph.id);
        }
        glyph.position = caret;

        last_glyph = Some(glyph.clone());
        caret.x += font.h_advance(glyph.id);

        if !c.is_whitespace() && caret.x > position.x + max_width {
            caret = point(position.x, caret.y + v_advance);
            glyph.position = caret;
            last_glyph = None;
        }

        target.push(glyph);
    }
}

static DEJA_VU_SANS: LazyLock<FontRef<'static>> = LazyLock::new(|| {
    FontRef::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf") as &[u8]).unwrap()
});

const LOADS_OF_UNICODE: &str = include_str!("loads-of-unicode.txt");

fn unicode_chars(len: usize) -> &'static str {
    let (index, _) = LOADS_OF_UNICODE.char_indices().nth(len).unwrap();
    &LOADS_OF_UNICODE[..index]
}

#[inline]
fn do_population_bench(
    b: &mut Bencher<WallTime>,
    cache_builder: DrawCacheBuilder,
    text: &str,
    scale: f32,
) {
    let font_id = 0;

    let mut glyphs = Vec::new();
    layout_paragraph(
        DEJA_VU_SANS.as_scaled(scale),
        point(0.0, 0.0),
        500.0,
        text,
        &mut glyphs,
    );

    let mut cache = cache_builder.build();

    {
        // warm up / avoid benching population performance
        for glyph in &glyphs {
            cache.queue_glyph(font_id, glyph.clone());
        }
        cache
            .cache_queued(&[&*DEJA_VU_SANS], |_, _| {})
            .expect("cache_queued initial");
    }

    b.iter(|| {
        cache.clear();
        cache.clear_queue();

        for glyph in &glyphs {
            cache.queue_glyph(font_id, glyph.clone());
        }
        cache
            .cache_queued(&[&*DEJA_VU_SANS], |_, _| {})
            .expect("cache_queued");
    })
}

/// Run single threaded:
/// * Leave code unmodified
/// * `cargo bench --bench st_vs_mt -- --save-baseline st`
///
/// Run multithreaded:
/// * Modify to `.multithread(true)`
/// * `cargo bench --bench st_vs_mt -- --save-baseline mt`
///
/// Compare with `critcmp mt st --target-dir draw-cache/target/`
fn bench_population_st_vs_mt(c: &mut Criterion) {
    for char_len in &[1500, 300, 50, 16] {
        for scale in &[150.0, 75.0, 30.0, 12.0] {
            let title = format!("bench_{char_len}_chars_{scale}px");
            c.bench_function(&title, |b| {
                do_population_bench(
                    b,
                    DrawCache::builder()
                        .dimensions(4096, 4096)
                        .multithread(false), // use `true` and save as `mt` baseline
                    unicode_chars(*char_len),
                    *scale,
                );
            });
        }
    }
}

criterion_group!(draw_st_mt, bench_population_st_vs_mt);

criterion_main!(draw_st_mt);
