use criterion::{criterion_group, criterion_main, Criterion};
use glyph_brush_draw_cache::*;
use glyph_brush_layout::ab_glyph::*;
use std::sync::LazyLock;

fn mock_gpu_upload_4us(_region: Rectangle<u32>, _bytes: &[u8]) {
    use std::time::{Duration, Instant};

    let now = Instant::now();
    while now.elapsed() < Duration::from_micros(4) {}
}

fn test_glyphs<F: Font>(font: F, string: &str) -> Vec<Glyph> {
    let mut glyphs = vec![];
    // use a bunch of different but similar scales.
    for scale in &[25_f32, 24.5, 25.01, 24.7, 24.99] {
        layout_paragraph(
            font.as_scaled(*scale),
            point(0.0, 0.0),
            500.0,
            string,
            &mut glyphs,
        );
    }
    glyphs
}

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

static FONTS: LazyLock<Vec<FontRef<'static>>> = LazyLock::new(|| {
    vec![
        include_bytes!("../../fonts/WenQuanYiMicroHei.ttf") as &[u8],
        include_bytes!("../../fonts/OpenSans-Italic.ttf") as &[u8],
        include_bytes!("../../fonts/Exo2-Light.otf") as &[u8],
    ]
    .into_iter()
    .map(|bytes| FontRef::try_from_slice(bytes).unwrap())
    .collect()
});

const TEST_STR: &str = include_str!("lipsum.txt");

// **************************************************************************
// General use benchmarks.
// **************************************************************************

/// Benchmark using a single font at "don't care" position tolerance
///
/// # Changes
/// * v2: Add 4us gpu upload wait + pre-populate
fn bench_high_position_tolerance(c: &mut Criterion) {
    c.bench_function("high_position_tolerance_v2", |b| {
        let font_id = 0;
        let glyphs = test_glyphs(&FONTS[font_id], TEST_STR);
        let mut cache = DrawCache::builder()
            .dimensions(1024, 1024)
            .scale_tolerance(0.1)
            .position_tolerance(1.0)
            .build();

        {
            // warm up / avoid benching population performance
            for glyph in &glyphs {
                cache.queue_glyph(font_id, glyph.clone());
            }
            cache
                .cache_queued(&FONTS, |_, _| {})
                .expect("cache_queued initial");
        }

        let space_id = &FONTS[font_id].glyph_id(' ');

        b.iter(|| {
            for glyph in &glyphs {
                cache.queue_glyph(font_id, glyph.clone());
            }

            cache
                .cache_queued(&FONTS, mock_gpu_upload_4us)
                .expect("cache_queued");

            for (index, glyph) in glyphs.iter().enumerate().filter(|(_, g)| g.id != *space_id) {
                let rect = cache.rect_for(font_id, glyph);
                assert!(
                    rect.is_some(),
                    "Gpu cache rect lookup failed ({:?}) for glyph index {}, id {}",
                    rect,
                    index,
                    glyph.id.0
                );
            }
        })
    });
}

/// Benchmark using a single ttf with default tolerances
///
/// # Changes
/// * v2: Add 4us gpu upload wait + pre-populate
fn bench_ttf_font(c: &mut Criterion) {
    c.bench_function("single_ttf_v2", |b| {
        let font_id = 0;
        let glyphs = test_glyphs(&FONTS[font_id], TEST_STR);
        let mut cache = DrawCache::builder().dimensions(1024, 1024).build();

        {
            // warm up / avoid benching population performance
            for glyph in &glyphs {
                cache.queue_glyph(font_id, glyph.clone());
            }
            cache
                .cache_queued(&FONTS, |_, _| {})
                .expect("cache_queued initial");
        }

        let space_id = &FONTS[font_id].glyph_id(' ');

        b.iter(|| {
            for glyph in &glyphs {
                cache.queue_glyph(font_id, glyph.clone());
            }

            cache
                .cache_queued(&FONTS, mock_gpu_upload_4us)
                .expect("cache_queued");

            for (index, glyph) in glyphs.iter().enumerate().filter(|(_, g)| g.id != *space_id) {
                let rect = cache.rect_for(font_id, glyph);
                assert!(
                    rect.is_some(),
                    "Gpu cache rect lookup failed ({:?}) for glyph index {}, id {}",
                    rect,
                    index,
                    glyph.id.0
                );
            }
        })
    });
}

/// Benchmark using a single ttf with default tolerances
///
/// # Changes
/// * v2: Add 4us gpu upload wait + pre-populate
fn bench_otf_font(c: &mut Criterion) {
    c.bench_function("single_otf_v2", |b| {
        let font_id = 2;
        let glyphs = test_glyphs(&FONTS[font_id], TEST_STR);
        let mut cache = DrawCache::builder().dimensions(1024, 1024).build();

        {
            // warm up / avoid benching population performance
            for glyph in &glyphs {
                cache.queue_glyph(font_id, glyph.clone());
            }
            cache
                .cache_queued(&FONTS, |_, _| {})
                .expect("cache_queued initial");
        }

        let space_id = &FONTS[font_id].glyph_id(' ');

        b.iter(|| {
            for glyph in &glyphs {
                cache.queue_glyph(font_id, glyph.clone());
            }

            cache
                .cache_queued(&FONTS, mock_gpu_upload_4us)
                .expect("cache_queued");

            for (index, glyph) in glyphs.iter().enumerate().filter(|(_, g)| g.id != *space_id) {
                let rect = cache.rect_for(font_id, glyph);
                assert!(
                    rect.is_some(),
                    "Gpu cache rect lookup failed ({:?}) for glyph index {}, id {}",
                    rect,
                    index,
                    glyph.id.0
                );
            }
        })
    });
}

/// Benchmark using multiple fonts with default tolerances
///
/// # Changes
/// * v2: Add 4us gpu upload wait + pre-populate
fn bench_multi_font(c: &mut Criterion) {
    c.bench_function("multi_font_v2", |b| {
        // Use a smaller amount of the test string, to offset the extra font-glyph
        // bench load
        let up_to_index = TEST_STR
            .char_indices()
            .nth(TEST_STR.chars().count() / FONTS.len())
            .unwrap()
            .0;
        let string = &TEST_STR[..up_to_index];

        let font_glyphs: Vec<_> = FONTS
            .iter()
            .enumerate()
            .map(|(id, font)| (id, test_glyphs(font, string)))
            .collect();
        let mut cache = DrawCache::builder().dimensions(1024, 1024).build();

        {
            // warm up / avoid benching population performance
            for &(font_id, ref glyphs) in &font_glyphs {
                for glyph in glyphs {
                    cache.queue_glyph(font_id, glyph.clone());
                }
            }
            cache.cache_queued(&FONTS, |_, _| {}).expect("cache_queued");
        }

        b.iter(|| {
            for &(font_id, ref glyphs) in &font_glyphs {
                for glyph in glyphs {
                    cache.queue_glyph(font_id, glyph.clone());
                }
            }

            cache
                .cache_queued(&FONTS, mock_gpu_upload_4us)
                .expect("cache_queued");

            for &(font_id, ref glyphs) in &font_glyphs {
                let space_id = &FONTS[font_id].glyph_id(' ');

                for (index, glyph) in glyphs.iter().enumerate().filter(|(_, g)| g.id != *space_id) {
                    let rect = cache.rect_for(font_id, glyph);
                    assert!(
                        rect.is_some(),
                        "Gpu cache rect lookup failed ({:?}) for font {} glyph index {}, id {}",
                        rect,
                        font_id,
                        index,
                        glyph.id.0
                    );
                }
            }
        })
    });
}

/// Benchmark using multiple fonts with default tolerances, clears the
/// cache each run to test the population "first run" performance
///
/// # Changes
/// * v2: Add 4us gpu upload wait
fn bench_multi_font_population(c: &mut Criterion) {
    c.bench_function("multi_font_population_v2", |b| {
        // Use a much smaller amount of the test string, to offset the extra font-glyph
        // bench load & much slower performance of fresh population each run
        let up_to_index = TEST_STR.char_indices().nth(70).unwrap().0;
        let string = &TEST_STR[..up_to_index];
        let font_glyphs: Vec<_> = FONTS
            .iter()
            .enumerate()
            .map(|(id, font)| (id, test_glyphs(font, string)))
            .collect();

        b.iter(|| {
            let mut cache = DrawCache::builder().dimensions(1024, 1024).build();

            for &(font_id, ref glyphs) in &font_glyphs {
                for glyph in glyphs {
                    cache.queue_glyph(font_id, glyph.clone());
                }
            }

            cache
                .cache_queued(&FONTS, mock_gpu_upload_4us)
                .expect("cache_queued");

            for &(font_id, ref glyphs) in &font_glyphs {
                let space_id = &FONTS[font_id].glyph_id(' ');
                for (index, glyph) in glyphs.iter().enumerate().filter(|(_, g)| g.id != *space_id) {
                    let rect = cache.rect_for(font_id, glyph);
                    assert!(
                        rect.is_some(),
                        "Gpu cache rect lookup failed ({:?}) for font {} glyph index {}, id {}",
                        rect,
                        font_id,
                        index,
                        glyph.id.0
                    );
                }
            }
        })
    });
}

/// Benchmark using multiple fonts and a different text group of glyphs
/// each run
///
/// # Changes
/// * v2: Add 4us gpu upload wait + pre-populate
fn bench_moving_text(c: &mut Criterion) {
    let chars: Vec<_> = TEST_STR.chars().collect();
    let subsection_len = chars.len() / FONTS.len();
    let distinct_subsection: Vec<_> = chars.windows(subsection_len).collect();

    let mut first_glyphs = vec![];
    let mut middle_glyphs = vec![];
    let mut last_glyphs = vec![];

    for (id, font) in FONTS.iter().enumerate() {
        let first_str: String = distinct_subsection[0].iter().collect();
        first_glyphs.push((id, test_glyphs(font, &first_str)));

        let middle_str: String = distinct_subsection[distinct_subsection.len() / 2]
            .iter()
            .collect();
        middle_glyphs.push((id, test_glyphs(font, &middle_str)));

        let last_str: String = distinct_subsection[distinct_subsection.len() - 1]
            .iter()
            .collect();
        last_glyphs.push((id, test_glyphs(font, &last_str)));
    }

    let test_variants = [first_glyphs, middle_glyphs, last_glyphs];
    let mut test_variants = test_variants.iter().cycle();

    let mut cache = DrawCache::builder()
        .dimensions(1500, 1500)
        .scale_tolerance(0.1)
        .position_tolerance(0.1)
        .build();

    {
        // warm up / avoid benching population performance
        let glyphs = test_variants.next().unwrap();
        for &(font_id, ref glyphs) in glyphs {
            for glyph in glyphs {
                cache.queue_glyph(font_id, glyph.clone());
            }
        }
        cache.cache_queued(&FONTS, |_, _| {}).expect("cache_queued");
    }

    c.bench_function("moving_text_v2", |b| {
        b.iter(|| {
            // switch text variant each run to force cache to deal with moving text
            // requirements
            let glyphs = test_variants.next().unwrap();
            for &(font_id, ref glyphs) in glyphs {
                for glyph in glyphs {
                    cache.queue_glyph(font_id, glyph.clone());
                }
            }

            cache
                .cache_queued(&FONTS, mock_gpu_upload_4us)
                .expect("cache_queued");

            for &(font_id, ref glyphs) in glyphs {
                let space_id = &FONTS[font_id].glyph_id(' ');
                for (index, glyph) in glyphs.iter().enumerate().filter(|(_, g)| g.id != *space_id) {
                    let rect = cache.rect_for(font_id, glyph);
                    assert!(
                        rect.is_some(),
                        "Gpu cache rect lookup failed ({:?}) for font {} glyph index {}, id {}",
                        rect,
                        font_id,
                        index,
                        glyph.id.0
                    );
                }
            }
        })
    });
}

// **************************************************************************
// Benchmarks for cases that should generally be avoided by the cache user if
// at all possible (ie by picking a better initial cache size).
// **************************************************************************

/// Cache isn't large enough for a queue so a new cache is created to hold
/// the queue.
///
/// # Changes
/// * v2: 4us gpu upload wait
fn bench_resizing(c: &mut Criterion) {
    let up_to_index = TEST_STR.char_indices().nth(120).unwrap().0;
    let string = &TEST_STR[..up_to_index];

    let font_glyphs: Vec<_> = FONTS
        .iter()
        .enumerate()
        .map(|(id, font)| (id, test_glyphs(font, string)))
        .collect();

    c.bench_function("resizing_v2", |b| {
        b.iter(|| {
            let mut cache = DrawCache::builder().dimensions(256, 256).build();

            for &(font_id, ref glyphs) in &font_glyphs {
                for glyph in glyphs {
                    cache.queue_glyph(font_id, glyph.clone());
                }
            }

            cache
                .cache_queued(&FONTS, mock_gpu_upload_4us)
                .expect_err("shouldn't fit");

            cache.to_builder().dimensions(512, 512).rebuild(&mut cache);

            cache
                .cache_queued(&FONTS, mock_gpu_upload_4us)
                .expect("should fit now");

            for &(font_id, ref glyphs) in &font_glyphs {
                let space_id = &FONTS[font_id].glyph_id(' ');
                for (index, glyph) in glyphs.iter().enumerate().filter(|(_, g)| g.id != *space_id) {
                    let rect = cache.rect_for(font_id, glyph);
                    assert!(
                        rect.is_some(),
                        "Gpu cache rect lookup failed ({:?}) for font {} glyph index {}, id {}",
                        rect,
                        font_id,
                        index,
                        glyph.id.0
                    );
                }
            }
        })
    });
}

/// Benchmark using multiple fonts and a different text group of glyphs
/// each run. The cache is only large enough to fit each run if it is
/// cleared and re-built.
///
/// # Changes
/// * v2: 4us gpu upload wait
fn bench_moving_text_thrashing(c: &mut Criterion) {
    let chars: Vec<_> = TEST_STR.chars().collect();
    let subsection_len = 60;
    let distinct_subsection: Vec<_> = chars.windows(subsection_len).collect();

    let mut first_glyphs = vec![];
    let mut middle_glyphs = vec![];
    let mut last_glyphs = vec![];

    for (id, font) in FONTS.iter().enumerate() {
        let first_str: String = distinct_subsection[0].iter().collect();
        first_glyphs.push((id, test_glyphs(font, &first_str)));

        let middle_str: String = distinct_subsection[distinct_subsection.len() / 2]
            .iter()
            .collect();
        middle_glyphs.push((id, test_glyphs(font, &middle_str)));

        let last_str: String = distinct_subsection[distinct_subsection.len() - 1]
            .iter()
            .collect();
        last_glyphs.push((id, test_glyphs(font, &last_str)));
    }

    let test_variants = [first_glyphs, middle_glyphs, last_glyphs];

    // Cache is only a little larger than each variants size meaning a lot of
    // re-ordering, re-rasterization & re-uploading has to occur.
    let mut cache = DrawCache::builder()
        .dimensions(320, 320)
        .scale_tolerance(0.1)
        .position_tolerance(0.1)
        .build();

    c.bench_function("moving_text_thrashing_v2", |b| {
        b.iter(|| {
            // switch text variant each run to force cache to deal with moving text
            // requirements
            for glyphs in &test_variants {
                for &(font_id, ref glyphs) in glyphs {
                    for glyph in glyphs {
                        cache.queue_glyph(font_id, glyph.clone());
                    }
                }

                cache
                    .cache_queued(&FONTS, mock_gpu_upload_4us)
                    .expect("cache_queued");

                for &(font_id, ref glyphs) in glyphs {
                    let space_id = &FONTS[font_id].glyph_id(' ');
                    for (index, glyph) in
                        glyphs.iter().enumerate().filter(|(_, g)| g.id != *space_id)
                    {
                        let rect = cache.rect_for(font_id, glyph);
                        assert!(
                            rect.is_some(),
                            "Gpu cache rect lookup failed ({:?}) for font {} glyph index {}, id {}",
                            rect,
                            font_id,
                            index,
                            glyph.id.0
                        );
                    }
                }
            }
        })
    });
}

criterion_group!(
    benches,
    bench_high_position_tolerance,
    bench_ttf_font,
    bench_otf_font,
    bench_multi_font,
    bench_multi_font_population,
    bench_moving_text,
    bench_resizing,
    bench_moving_text_thrashing,
);

criterion_main!(benches);
