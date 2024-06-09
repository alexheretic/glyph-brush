use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use glyph_brush::{ab_glyph::*, *};
use std::{borrow::Cow, f32};

const TEST_FONT: &[u8] = include_bytes!("../../fonts/DejaVuSansMono.ttf");
const TEST_OTF_FONT: &[u8] = include_bytes!("../../fonts/Exo2-Light.otf");
const LIPSUM: &str = include_str!("lipsum.txt");
const LOTS_OF_LIPSUM: &str = include_str!("lots_of_lipsum.txt");
const SMALL_LIPSUM: &str = include_str!("small_lipsum.txt");

fn three_medium_sections() -> [Section<'static>; 3] {
    [
        Section::default()
            .add_text(Text::new(LIPSUM))
            .with_bounds((600.0, f32::INFINITY)),
        Section::default()
            .add_text(Text::new(LIPSUM))
            .with_bounds((600.0, f32::INFINITY))
            .with_screen_position((600.0, 0.0))
            .with_layout(Layout::default().h_align(HorizontalAlign::Center)),
        Section::default()
            .add_text(Text::new(LIPSUM))
            .with_bounds((1200.0, f32::INFINITY))
            .with_screen_position((600.0, 0.0))
            .with_layout(Layout::default().h_align(HorizontalAlign::Right)),
    ]
}

fn render_3_medium_sections_fully(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut glyph_brush = GlyphBrushBuilder::using_font(font).build();

    let sections = three_medium_sections();

    c.bench_function("render_3_medium_sections_fully", |b| {
        b.iter(|| {
            for section in &sections {
                glyph_brush.queue(section);
            }
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_3_medium_sections_fully(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut glyph_brush = GlyphBrushBuilder::using_font(font)
        .cache_glyph_positioning(false)
        .cache_redraws(false)
        .build();

    let sections = three_medium_sections();

    c.bench_function("no_cache_render_3_medium_sections_fully", |b| {
        b.iter(|| {
            for section in &sections {
                glyph_brush.queue(section);
            }
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

fn render_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut glyph_brush = GlyphBrushBuilder::using_font(font).build();

    let section = Section::default()
        .add_text(Text::new(LOTS_OF_LIPSUM))
        .with_bounds((600.0, 600.0));

    c.bench_function("render_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(&section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut glyph_brush = GlyphBrushBuilder::using_font(font)
        .cache_glyph_positioning(false)
        .cache_redraws(false)
        .build();

    let section = Section::default()
        .add_text(Text::new(LOTS_OF_LIPSUM))
        .with_bounds((600.0, 600.0));

    c.bench_function("no_cache_render_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(&section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

fn render_v_center_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut glyph_brush = GlyphBrushBuilder::using_font(font).build();

    let section = Section::default()
        .add_text(Text::new(LOTS_OF_LIPSUM))
        .with_screen_position((0.0, 300.0))
        .with_bounds((600.0, 600.0))
        .with_layout(Layout::default().v_align(VerticalAlign::Center));

    c.bench_function("render_v_center_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(&section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

fn no_cache_render_v_center_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut glyph_brush = GlyphBrushBuilder::using_font(font)
        .cache_glyph_positioning(false)
        .cache_redraws(false)
        .build();

    let section = Section::default()
        .add_text(Text::new(LOTS_OF_LIPSUM))
        .with_screen_position((0.0, 300.0))
        .with_bounds((600.0, 600.0))
        .with_layout(Layout::default().v_align(VerticalAlign::Center));

    c.bench_function("no_cache_render_v_center_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(&section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

fn render_v_bottom_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut glyph_brush = GlyphBrushBuilder::using_font(font).build();

    let section = Section::default()
        .add_text(Text::new(LOTS_OF_LIPSUM))
        .with_screen_position((0.0, 600.0))
        .with_bounds((600.0, 600.0))
        .with_layout(Layout::default().v_align(VerticalAlign::Bottom));

    c.bench_function("render_v_bottom_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(&section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_v_bottom_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut glyph_brush = GlyphBrushBuilder::using_font(font)
        .cache_glyph_positioning(false)
        .cache_redraws(false)
        .build();

    let section = Section::default()
        .add_text(Text::new(LOTS_OF_LIPSUM))
        .with_screen_position((0.0, 600.0))
        .with_bounds((600.0, 600.0))
        .with_layout(Layout::default().v_align(VerticalAlign::Bottom));

    c.bench_function("no_cache_render_v_bottom_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(&section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

fn render_100_small_sections_fully(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut glyph_brush = GlyphBrushBuilder::using_font(font).build();

    let mut sections = vec![];
    for i in 0..100 {
        sections.push(
            Section::default()
                .add_text(Text::new(SMALL_LIPSUM))
                .with_screen_position((i as f32, 0.0))
                .with_bounds((100.0, f32::INFINITY)),
        );
    }

    c.bench_function("render_100_small_sections_fully", |b| {
        b.iter(|| {
            for section in &sections {
                glyph_brush.queue(section);
            }
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_100_small_sections_fully(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut glyph_brush = GlyphBrushBuilder::using_font(font)
        .cache_glyph_positioning(false)
        .cache_redraws(false)
        .build();

    let mut sections = vec![];
    for i in 0..100 {
        sections.push(
            Section::default()
                .add_text(Text::new(SMALL_LIPSUM))
                .with_screen_position((i as f32, 0.0))
                .with_bounds((100.0, f32::INFINITY)),
        );
    }

    c.bench_function("no_cache_render_100_small_sections_fully", |b| {
        b.iter(|| {
            for section in &sections {
                glyph_brush.queue(section);
            }
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

/// section is rendered with text edits each run to the end
fn continually_modify_end_text_of_1_of_3(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut brush = GlyphBrushBuilder::using_font(font).build();
    let text = LIPSUM;

    let string_variants = [
        text.to_owned(),
        text.to_owned() + "a",
        text.to_owned() + "ab",
        text.to_owned() + "a",
    ];

    let variants: Vec<_> = string_variants
        .iter()
        .map(|s| {
            vec![
                Section::default()
                    .add_text(Text::new(s))
                    .with_bounds((600.0, f32::INFINITY)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((600.0, 0.0))
                    .with_bounds((600.0, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Center)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((1200.0, 0.0))
                    .with_bounds((600.0, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Right)),
            ]
        })
        .collect();

    c.bench_function("continually_modify_end_text_of_1_of_3", |b| {
        bench_variants(b, &variants, &mut brush)
    });
}

/// section is rendered with text edits each run to the beginning
fn continually_modify_start_text_of_1_of_3(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut brush = GlyphBrushBuilder::using_font(font).build();
    let text = LIPSUM;

    let string_variants = [
        text.to_owned(),
        "a".to_owned() + text,
        "ab".to_owned() + text,
        "a".to_owned() + text,
    ];

    let variants: Vec<_> = string_variants
        .iter()
        .map(|s| {
            vec![
                Section::default()
                    .add_text(Text::new(s))
                    .with_bounds((600.0, f32::INFINITY)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((600.0, 0.0))
                    .with_bounds((600.0, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Center)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((1200.0, 0.0))
                    .with_bounds((600.0, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Right)),
            ]
        })
        .collect();

    c.bench_function("continually_modify_start_text_of_1_of_3", |b| {
        bench_variants(b, &variants, &mut brush)
    });
}

/// section is rendered with text edits each run to the middle
fn continually_modify_middle_text_of_1_of_3(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut brush = GlyphBrushBuilder::using_font(font).build();
    let text = LIPSUM;
    let middle_index = {
        let mut ci = text.char_indices();
        ci.nth(text.chars().count() / 2);
        ci.next().unwrap().0
    };

    let string_variants = [
        text.to_owned(),
        text[..middle_index].to_owned() + "a" + &text[middle_index..],
        text[..middle_index].to_owned() + "ab" + &text[middle_index..],
        text[..middle_index].to_owned() + "a" + &text[middle_index..],
    ];

    let variants: Vec<_> = string_variants
        .iter()
        .map(|s| {
            vec![
                Section::default()
                    .add_text(Text::new(s))
                    .with_bounds((600.0, f32::INFINITY)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((600.0, 0.0))
                    .with_bounds((600.0, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Center)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((1200.0, 0.0))
                    .with_bounds((600.0, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Right)),
            ]
        })
        .collect();

    c.bench_function("continually_modify_middle_text_of_1_of_3", |b| {
        bench_variants(b, &variants, &mut brush)
    });
}

/// section is rendered with the bounds redefined each run to the middle
fn continually_modify_bounds_of_1_of_3(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut brush = GlyphBrushBuilder::using_font(font).build();
    let text = LIPSUM;

    let variants: Vec<_> = vec![400, 600, 855]
        .into_iter()
        .map(|width| {
            vec![
                Section::default()
                    .add_text(Text::new(text))
                    .with_bounds((width as f32, f32::INFINITY)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((600.0, 0.0))
                    .with_bounds((600.0, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Center)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((1200.0, 0.0))
                    .with_bounds((600.0, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Right)),
            ]
        })
        .collect();

    c.bench_function("continually_modify_bounds_of_1_of_3", |b| {
        bench_variants(b, &variants, &mut brush)
    });
}

/// 1 section of 3 is rendered with a different colour each frame
fn continually_modify_color_of_1_of_3(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut brush = GlyphBrushBuilder::using_font(font).build();
    let text = LIPSUM;

    let variants: Vec<_> = vec![
        [0.1, 0.2, 0.7, 1.0],
        [1.3, 0.5, 0.0, 1.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
    .into_iter()
    .map(|color| {
        vec![
            Section::default()
                .add_text(Text::new(text).with_color(color))
                .with_bounds((600.0, f32::INFINITY)),
            Section::default()
                .add_text(Text::new(text))
                .with_screen_position((600.0, 0.0))
                .with_bounds((600.0, f32::INFINITY))
                .with_layout(Layout::default().h_align(HorizontalAlign::Center)),
            Section::default()
                .add_text(Text::new(text))
                .with_screen_position((1200.0, 0.0))
                .with_bounds((600.0, f32::INFINITY))
                .with_layout(Layout::default().h_align(HorizontalAlign::Right)),
        ]
    })
    .collect();

    c.bench_function("continually_modify_color_of_1_of_3", |b| {
        bench_variants(b, &variants, &mut brush)
    });
}

/// 1 section of 3 is rendered with a different colour each frame
fn continually_modify_alpha_of_1_of_3(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut brush = GlyphBrushBuilder::using_font(font).build();
    let text = LIPSUM;

    let variants: Vec<_> = vec![
        // fade out alpha
        1.0, 0.8, 0.6, 0.4, 0.2, 0.0,
    ]
    .into_iter()
    .map(|alpha| {
        vec![
            Section::default()
                .add_text(Text::new("Heading\n").with_color([1.0, 1.0, 0.0, alpha]))
                .add_text(Text::new(text).with_color([1.0, 1.0, 0.0, alpha]))
                .with_bounds((600.0, f32::INFINITY)),
            Section::default()
                .add_text(Text::new(text))
                .with_screen_position((600.0, 0.0))
                .with_bounds((600.0, f32::INFINITY))
                .with_layout(Layout::default().h_align(HorizontalAlign::Center)),
            Section::default()
                .add_text(Text::new(text))
                .with_screen_position((1200.0, 0.0))
                .with_bounds((600.0, f32::INFINITY))
                .with_layout(Layout::default().h_align(HorizontalAlign::Right)),
        ]
    })
    .collect();

    c.bench_function("continually_modify_alpha_of_1_of_3", |b| {
        bench_variants(b, &variants, &mut brush)
    });
}

/// section is rendered with the bounds redefined each run to the middle
fn continually_modify_position_of_1_of_3(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_FONT).unwrap();

    let mut brush = GlyphBrushBuilder::using_font(font).build();
    let text = LIPSUM;

    let variants: Vec<_> = vec![(0, 0), (100, 50), (101, 300)]
        .into_iter()
        .map(|(x, y)| {
            vec![
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((x as f32, y as f32))
                    .with_bounds((600.0, f32::INFINITY)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((600.0, 0.0))
                    .with_bounds((600.0, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Center)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((1200.0, 0.0))
                    .with_bounds((600.0, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Right)),
            ]
        })
        .collect();

    c.bench_function("continually_modify_position_of_1_of_3", |b| {
        bench_variants(b, &variants, &mut brush)
    });
}

/// Zooming into text.
/// * Scales increase & decrease according to quadratic ease-out.
/// * Positions & bounds shift around as the scale changes.
fn continually_zoom(c: &mut Criterion) {
    let _ = env_logger::try_init();
    let font = FontRef::try_from_slice(TEST_OTF_FONT).unwrap();

    let mut brush = GlyphBrushBuilder::using_font(font)
        .initial_cache_size((768, 768))
        .build();
    let text = LIPSUM;

    let variants: Vec<_> = (0..500)
        .chain((1..=500).rev())
        .map(|v| {
            let factor = v as f32 / 500.0;
            // ease between pixel heights 12 -> 36
            let scale = quad_ease_out(factor, 12.0, 24.0, 1.0);

            vec![
                Section::default()
                    .add_text(Text::new(text).with_scale(scale))
                    .with_screen_position((-200.0 * factor, 0.0))
                    .with_bounds((600.0 + 600.0 * factor, f32::INFINITY)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((600.0, 0.0))
                    .with_bounds((600.0 + 600.0 * factor, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Center)),
                Section::default()
                    .add_text(Text::new(text))
                    .with_screen_position((1200.0 + 200.0 * factor, 0.0))
                    .with_bounds((600.0 + 600.0 * factor, f32::INFINITY))
                    .with_layout(Layout::default().h_align(HorizontalAlign::Right)),
            ]
        })
        .collect();

    c.bench_function("continually_zoom", |b| {
        bench_variants(b, &variants, &mut brush)
    });
}

fn quad_ease_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    let t = t / d;
    -c * t * (t - 2.0) + b
}

/// Renders a different set of sections each run by
/// cycling through the provided `variants`
#[inline]
fn bench_variants<'a, S: 'a>(
    b: &mut Bencher,
    variants: &'a [std::vec::Vec<S>],
    glyph_brush: &mut GlyphBrush<[f32; 13], Extra, FontRef<'static>>,
) where
    &'a S: Into<Cow<'a, Section<'a>>>,
{
    let mut variants = variants.iter().cycle();

    b.iter(|| {
        for s in variants.next().unwrap() {
            glyph_brush.queue(s);
        }
        glyph_brush
            .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
            .unwrap();
    });
}

/// opengl vertex generator
#[inline]
fn gl_to_vertex(
    glyph_brush::GlyphVertex {
        mut tex_coords,
        pixel_coords,
        bounds,
        extra,
    }: glyph_brush::GlyphVertex,
) -> [f32; 13] {
    let gl_bounds = bounds;

    let mut gl_rect = Rect {
        min: point(pixel_coords.min.x, pixel_coords.min.y),
        max: point(pixel_coords.max.x, pixel_coords.max.y),
    };

    // handle overlapping bounds, modify uv_rect to preserve texture aspect
    if gl_rect.max.x > gl_bounds.max.x {
        let old_width = gl_rect.width();
        gl_rect.max.x = gl_bounds.max.x;
        tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.min.x < gl_bounds.min.x {
        let old_width = gl_rect.width();
        gl_rect.min.x = gl_bounds.min.x;
        tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.max.y > gl_bounds.max.y {
        let old_height = gl_rect.height();
        gl_rect.max.y = gl_bounds.max.y;
        tex_coords.max.y = tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
    }
    if gl_rect.min.y < gl_bounds.min.y {
        let old_height = gl_rect.height();
        gl_rect.min.y = gl_bounds.min.y;
        tex_coords.min.y = tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
    }

    [
        gl_rect.min.x,
        gl_rect.max.y,
        extra.z,
        gl_rect.max.x,
        gl_rect.min.y,
        tex_coords.min.x,
        tex_coords.max.y,
        tex_coords.max.x,
        tex_coords.min.y,
        extra.color[0],
        extra.color[1],
        extra.color[2],
        extra.color[3],
    ]
}

criterion_group!(
    benches,
    render_3_medium_sections_fully,
    render_1_large_section_partially,
    render_v_center_1_large_section_partially,
    render_v_bottom_1_large_section_partially,
    render_100_small_sections_fully,
    continually_modify_end_text_of_1_of_3,
    continually_modify_start_text_of_1_of_3,
    continually_modify_middle_text_of_1_of_3,
    continually_modify_bounds_of_1_of_3,
    continually_modify_color_of_1_of_3,
    continually_modify_alpha_of_1_of_3,
    continually_modify_position_of_1_of_3,
    continually_zoom,
);

criterion_group!(
    name = no_cache_benches;
    config = Criterion::default().sample_size(20);
    targets = no_cache_render_3_medium_sections_fully,
        no_cache_render_1_large_section_partially,
        no_cache_render_v_center_1_large_section_partially,
        no_cache_render_v_bottom_1_large_section_partially,
        no_cache_render_100_small_sections_fully,
);

criterion_main!(benches, no_cache_benches);
