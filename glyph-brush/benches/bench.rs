use criterion::{criterion_group, criterion_main, Bencher, Criterion};
use glyph_brush::{rusttype::*, *};
use std::{borrow::Cow, f32};

const TEST_FONT: &[u8] = include_bytes!("../../fonts/DejaVuSansMono.ttf");
const LIPSUM: &str = include_str!("lipsum.txt");
const LOTS_OF_LIPSUM: &str = include_str!("lots_of_lipsum.txt");
const SMALL_LIPSUM: &str = include_str!("small_lipsum.txt");

fn render_3_medium_sections_fully(c: &mut Criterion) {
    let _ = env_logger::try_init();

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();

    let sections = &[
        Section {
            text: LIPSUM,
            bounds: (600.0, f32::INFINITY),
            ..<_>::default()
        },
        Section {
            text: LIPSUM,
            screen_position: (600.0, 0.0),
            bounds: (600.0, f32::INFINITY),
            layout: Layout::default().h_align(HorizontalAlign::Center),
            ..<_>::default()
        },
        Section {
            text: LIPSUM,
            screen_position: (1200.0, 0.0),
            bounds: (600.0, f32::INFINITY),
            layout: Layout::default().h_align(HorizontalAlign::Right),
            ..<_>::default()
        },
    ];

    c.bench_function("render_3_medium_sections_fully", |b| {
        b.iter(|| {
            for section in sections {
                glyph_brush.queue(*section);
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

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false)
        .build();

    let sections = &[
        Section {
            text: LIPSUM,
            bounds: (600.0, f32::INFINITY),
            ..<_>::default()
        },
        Section {
            text: LIPSUM,
            screen_position: (600.0, 0.0),
            bounds: (600.0, f32::INFINITY),
            layout: Layout::default().h_align(HorizontalAlign::Center),
            ..<_>::default()
        },
        Section {
            text: LIPSUM,
            screen_position: (1200.0, 0.0),
            bounds: (600.0, f32::INFINITY),
            layout: Layout::default().h_align(HorizontalAlign::Right),
            ..<_>::default()
        },
    ];

    c.bench_function("no_cache_render_3_medium_sections_fully", |b| {
        b.iter(|| {
            for section in sections {
                glyph_brush.queue(*section);
            }
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

fn render_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();

    let section = Section {
        text: LOTS_OF_LIPSUM,
        bounds: (600.0, 600.0),
        ..<_>::default()
    };

    c.bench_function("render_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false)
        .build();

    let section = Section {
        text: LOTS_OF_LIPSUM,
        bounds: (600.0, 600.0),
        ..<_>::default()
    };

    c.bench_function("no_cache_render_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

fn render_v_center_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();

    let section = Section {
        text: LOTS_OF_LIPSUM,
        screen_position: (0.0, 300.0),
        bounds: (600.0, 600.0),
        layout: Layout::default().v_align(VerticalAlign::Center),
        ..<_>::default()
    };

    c.bench_function("render_v_center_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

fn no_cache_render_v_center_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false)
        .build();

    let section = Section {
        text: LOTS_OF_LIPSUM,
        screen_position: (0.0, 300.0),
        bounds: (600.0, 600.0),
        layout: Layout::default().v_align(VerticalAlign::Center),
        ..<_>::default()
    };

    c.bench_function("no_cache_render_v_center_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

fn render_v_bottom_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();

    let section = Section {
        text: LOTS_OF_LIPSUM,
        screen_position: (0.0, 600.0),
        bounds: (600.0, 600.0),
        layout: Layout::default().v_align(VerticalAlign::Bottom),
        ..<_>::default()
    };

    c.bench_function("render_v_bottom_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_v_bottom_1_large_section_partially(c: &mut Criterion) {
    let _ = env_logger::try_init();

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false)
        .build();

    let section = Section {
        text: LOTS_OF_LIPSUM,
        screen_position: (0.0, 600.0),
        bounds: (600.0, 600.0),
        layout: Layout::default().v_align(VerticalAlign::Bottom),
        ..<_>::default()
    };

    c.bench_function("no_cache_render_v_bottom_1_large_section_partially", |b| {
        b.iter(|| {
            glyph_brush.queue(section);
            glyph_brush
                .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
                .unwrap();
        })
    });
}

fn render_100_small_sections_fully(c: &mut Criterion) {
    let _ = env_logger::try_init();

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();

    let mut sections = vec![];
    for i in 0..100 {
        sections.push(Section {
            text: SMALL_LIPSUM,
            screen_position: (i as f32, 0.0),
            bounds: (100.0, f32::INFINITY),
            ..<_>::default()
        });
    }

    c.bench_function("render_100_small_sections_fully", |b| {
        b.iter(|| {
            for section in &sections {
                glyph_brush.queue(*section);
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

    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false)
        .build();

    let mut sections = vec![];
    for i in 0..100 {
        sections.push(Section {
            text: SMALL_LIPSUM,
            screen_position: (i as f32, 0.0),
            bounds: (100.0, f32::INFINITY),
            ..<_>::default()
        });
    }

    c.bench_function("no_cache_render_100_small_sections_fully", |b| {
        b.iter(|| {
            for section in &sections {
                glyph_brush.queue(*section);
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

    let mut brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();
    let text = LIPSUM;

    let string_variants = vec![
        text.to_owned(),
        text.to_owned() + "a",
        text.to_owned() + "ab",
        text.to_owned() + "a",
    ];

    let variants: Vec<_> = string_variants
        .iter()
        .map(|s| {
            vec![
                Section {
                    text: s,
                    bounds: (600.0, f32::INFINITY),
                    ..<_>::default()
                },
                Section {
                    text,
                    screen_position: (600.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Center),
                    ..<_>::default()
                },
                Section {
                    text,
                    screen_position: (1200.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Right),
                    ..<_>::default()
                },
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

    let mut brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();
    let text = LIPSUM;

    let string_variants = vec![
        text.to_owned(),
        "a".to_owned() + text,
        "ab".to_owned() + text,
        "a".to_owned() + text,
    ];

    let variants: Vec<_> = string_variants
        .iter()
        .map(|s| {
            vec![
                Section {
                    text: s,
                    bounds: (600.0, f32::INFINITY),
                    ..<_>::default()
                },
                Section {
                    text,
                    screen_position: (600.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Center),
                    ..<_>::default()
                },
                Section {
                    text,
                    screen_position: (1200.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Right),
                    ..<_>::default()
                },
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

    let mut brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();
    let text = LIPSUM;
    let middle_index = {
        let mut ci = text.char_indices();
        ci.nth(text.chars().count() / 2);
        ci.next().unwrap().0
    };

    let string_variants = vec![
        text.to_owned(),
        text[..middle_index].to_owned() + "a" + &text[middle_index..],
        text[..middle_index].to_owned() + "ab" + &text[middle_index..],
        text[..middle_index].to_owned() + "a" + &text[middle_index..],
    ];

    let variants: Vec<_> = string_variants
        .iter()
        .map(|s| {
            vec![
                Section {
                    text: s,
                    bounds: (600.0, f32::INFINITY),
                    ..<_>::default()
                },
                Section {
                    text,
                    screen_position: (600.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Center),
                    ..<_>::default()
                },
                Section {
                    text,
                    screen_position: (1200.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Right),
                    ..<_>::default()
                },
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

    let mut brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();
    let text = LIPSUM;

    let variants: Vec<_> = vec![400, 600, 855]
        .into_iter()
        .map(|width| {
            vec![
                Section {
                    text,
                    bounds: (width as f32, f32::INFINITY),
                    ..<_>::default()
                },
                Section {
                    text,
                    screen_position: (600.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Center),
                    ..<_>::default()
                },
                Section {
                    text,
                    screen_position: (1200.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Right),
                    ..<_>::default()
                },
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

    let mut brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();
    let text = LIPSUM;

    let variants: Vec<_> = vec![
        [0.1, 0.2, 0.7, 1.0],
        [1.3, 0.5, 0.0, 1.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
    .into_iter()
    .map(|color| {
        vec![
            Section {
                text,
                color,
                bounds: (600.0, f32::INFINITY),
                ..<_>::default()
            },
            Section {
                text,
                screen_position: (600.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                layout: Layout::default().h_align(HorizontalAlign::Center),
                ..<_>::default()
            },
            Section {
                text,
                screen_position: (1200.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                layout: Layout::default().h_align(HorizontalAlign::Right),
                ..<_>::default()
            },
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

    let mut brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();
    let text = LIPSUM;

    let variants: Vec<_> = vec![
        // fade out alpha
        1.0, 0.8, 0.6, 0.4, 0.2, 0.0,
    ]
    .into_iter()
    .map(|alpha| {
        vec![
            VariedSection {
                text: vec![
                    SectionText {
                        text: "Heading\n",
                        color: [1.0, 1.0, 0.0, alpha],
                        ..<_>::default()
                    },
                    SectionText {
                        text,
                        color: [1.0, 1.0, 1.0, alpha],
                        ..<_>::default()
                    },
                ],
                bounds: (600.0, f32::INFINITY),
                ..<_>::default()
            },
            VariedSection {
                text: vec![SectionText {
                    text,
                    ..<_>::default()
                }],
                screen_position: (600.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                layout: Layout::default().h_align(HorizontalAlign::Center),
                ..<_>::default()
            },
            VariedSection {
                text: vec![SectionText {
                    text,
                    ..<_>::default()
                }],
                screen_position: (1200.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                layout: Layout::default().h_align(HorizontalAlign::Right),
                ..<_>::default()
            },
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

    let mut brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT).build();
    let text = LIPSUM;

    let variants: Vec<_> = vec![(0, 0), (100, 50), (101, 300)]
        .into_iter()
        .map(|(x, y)| {
            vec![
                Section {
                    text,
                    screen_position: (x as f32, y as f32),
                    bounds: (600.0, f32::INFINITY),
                    ..<_>::default()
                },
                Section {
                    text,
                    screen_position: (600.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Center),
                    ..<_>::default()
                },
                Section {
                    text,
                    screen_position: (1200.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Right),
                    ..<_>::default()
                },
            ]
        })
        .collect();

    c.bench_function("continually_modify_position_of_1_of_3", |b| {
        bench_variants(b, &variants, &mut brush)
    });
}

/// Renders a different set of sections each run by
/// cycling through the provided `variants`
#[inline]
fn bench_variants<'a, S: 'a>(
    b: &mut Bencher,
    variants: &'a [std::vec::Vec<S>],
    glyph_brush: &mut GlyphBrush<'_, [f32; 13]>,
) where
    &'a S: Into<Cow<'a, VariedSection<'a>>>,
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
        color,
        z,
    }: glyph_brush::GlyphVertex,
) -> [f32; 13] {
    let gl_bounds = bounds;

    let mut gl_rect = Rect {
        min: point(pixel_coords.min.x as f32, pixel_coords.min.y as f32),
        max: point(pixel_coords.max.x as f32, pixel_coords.max.y as f32),
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
        z,
        gl_rect.max.x,
        gl_rect.min.y,
        tex_coords.min.x,
        tex_coords.max.y,
        tex_coords.max.x,
        tex_coords.min.y,
        color[0],
        color[1],
        color[2],
        color[3],
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
