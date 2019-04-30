#![feature(test)]
extern crate test;

use glyph_brush::{rusttype::*, *};
use std::f32;

const TEST_FONT: &[u8] = include_bytes!("../../fonts/DejaVuSansMono.ttf");

#[bench]
fn render_3_medium_sections_fully(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lipsum.txt");

    bench(
        b,
        &[
            Section {
                text,
                bounds: (600.0, f32::INFINITY),
                ..Section::default()
            },
            Section {
                text,
                screen_position: (600.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                layout: Layout::default().h_align(HorizontalAlign::Center),
                ..Section::default()
            },
            Section {
                text,
                screen_position: (1200.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                layout: Layout::default().h_align(HorizontalAlign::Right),
                ..Section::default()
            },
        ],
        brush,
    );
}

#[bench]
// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_3_medium_sections_fully(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false);
    let text = include_str!("lipsum.txt");

    bench(
        b,
        &[
            Section {
                text,
                bounds: (600.0, f32::INFINITY),
                ..Section::default()
            },
            Section {
                text,
                screen_position: (600.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                layout: Layout::default().h_align(HorizontalAlign::Center),
                ..Section::default()
            },
            Section {
                text,
                screen_position: (1200.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                layout: Layout::default().h_align(HorizontalAlign::Right),
                ..Section::default()
            },
        ],
        brush,
    );
}

#[bench]
fn render_1_large_section_partially(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lots_of_lipsum.txt");

    bench(
        b,
        &[Section {
            text,
            bounds: (600.0, 600.0),
            ..Section::default()
        }],
        brush,
    );
}

#[bench]
// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_1_large_section_partially(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false);
    let text = include_str!("lots_of_lipsum.txt");

    bench(
        b,
        &[Section {
            text,
            bounds: (600.0, 600.0),
            ..Section::default()
        }],
        brush,
    );
}

#[bench]
// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn render_v_center_1_large_section_partially(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lots_of_lipsum.txt");

    bench(
        b,
        &[Section {
            text,
            screen_position: (0.0, 300.0),
            bounds: (600.0, 600.0),
            layout: Layout::default().v_align(VerticalAlign::Center),
            ..Section::default()
        }],
        brush,
    );
}

#[bench]
// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_v_center_1_large_section_partially(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false);
    let text = include_str!("lots_of_lipsum.txt");

    bench(
        b,
        &[Section {
            text,
            screen_position: (0.0, 300.0),
            bounds: (600.0, 600.0),
            layout: Layout::default().v_align(VerticalAlign::Center),
            ..Section::default()
        }],
        brush,
    );
}

#[bench]
fn render_v_bottom_1_large_section_partially(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lots_of_lipsum.txt");

    bench(
        b,
        &[Section {
            text,
            screen_position: (0.0, 600.0),
            bounds: (600.0, 600.0),
            layout: Layout::default().v_align(VerticalAlign::Bottom),
            ..Section::default()
        }],
        brush,
    );
}

#[bench]
// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_v_bottom_1_large_section_partially(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false);
    let text = include_str!("lots_of_lipsum.txt");

    bench(
        b,
        &[Section {
            text,
            screen_position: (0.0, 600.0),
            bounds: (600.0, 600.0),
            layout: Layout::default().v_align(VerticalAlign::Bottom),
            ..Section::default()
        }],
        brush,
    );
}

#[bench]
fn render_100_small_sections_fully(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("small_lipsum.txt");

    let mut section_layouts = vec![];
    for i in 0..100 {
        section_layouts.push(Section {
            text,
            screen_position: (i as f32, 0.0),
            bounds: (100.0, f32::INFINITY),
            ..Section::default()
        });
    }

    bench(b, &section_layouts, brush);
}

#[bench]
// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_100_small_sections_fully(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false);
    let text = include_str!("small_lipsum.txt");

    let mut section_layouts = vec![];
    for i in 0..100 {
        section_layouts.push(Section {
            text,
            screen_position: (i as f32, 0.0),
            bounds: (100.0, f32::INFINITY),
            ..Section::default()
        });
    }

    bench(b, &section_layouts, brush);
}

#[bench]
/// section is rendered with text edits each run to the end
fn continually_modify_end_text_of_1_of_3(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lipsum.txt");

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
                    ..Section::default()
                },
                Section {
                    text,
                    screen_position: (600.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Center),
                    ..Section::default()
                },
                Section {
                    text,
                    screen_position: (1200.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Right),
                    ..Section::default()
                },
            ]
        })
        .collect();

    bench_variants(b, &variants, brush);
}

#[bench]
/// section is rendered with text edits each run to the beginning
fn continually_modify_start_text_of_1_of_3(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lipsum.txt");

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
                    ..Section::default()
                },
                Section {
                    text,
                    screen_position: (600.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Center),
                    ..Section::default()
                },
                Section {
                    text,
                    screen_position: (1200.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Right),
                    ..Section::default()
                },
            ]
        })
        .collect();

    bench_variants(b, &variants, brush);
}

#[bench]
/// section is rendered with text edits each run to the middle
fn continually_modify_middle_text_of_1_of_3(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lipsum.txt");
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
                    ..Section::default()
                },
                Section {
                    text,
                    screen_position: (600.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Center),
                    ..Section::default()
                },
                Section {
                    text,
                    screen_position: (1200.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Right),
                    ..Section::default()
                },
            ]
        })
        .collect();

    bench_variants(b, &variants, brush);
}

#[bench]
/// section is rendered with the bounds redefined each run to the middle
fn continually_modify_bounds_of_1_of_3(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lipsum.txt");

    let variants: Vec<_> = vec![400, 600, 855]
        .into_iter()
        .map(|width| {
            vec![
                Section {
                    text,
                    bounds: (width as f32, f32::INFINITY),
                    ..Section::default()
                },
                Section {
                    text,
                    screen_position: (600.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Center),
                    ..Section::default()
                },
                Section {
                    text,
                    screen_position: (1200.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Right),
                    ..Section::default()
                },
            ]
        })
        .collect();

    bench_variants(b, &variants, brush);
}

#[bench]
/// 1 section of 3 is rendered with a different colour each frame
fn continually_modify_color_of_1_of_3(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lipsum.txt");

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
                ..Section::default()
            },
            Section {
                text,
                screen_position: (600.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                layout: Layout::default().h_align(HorizontalAlign::Center),
                ..Section::default()
            },
            Section {
                text,
                screen_position: (1200.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                layout: Layout::default().h_align(HorizontalAlign::Right),
                ..Section::default()
            },
        ]
    })
    .collect();

    bench_variants(b, &variants, brush);
}

#[bench]
/// 1 section of 3 is rendered with a different colour each frame
fn continually_modify_alpha_of_1_of_3(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lipsum.txt");

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

    bench_varied_variants(b, &variants, brush);
}

#[bench]
/// section is rendered with the bounds redefined each run to the middle
fn continually_modify_position_of_1_of_3(b: &mut test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lipsum.txt");

    let variants: Vec<_> = vec![(0, 0), (100, 50), (101, 300)]
        .into_iter()
        .map(|(x, y)| {
            vec![
                Section {
                    text,
                    screen_position: (x as f32, y as f32),
                    bounds: (600.0, f32::INFINITY),
                    ..Section::default()
                },
                Section {
                    text,
                    screen_position: (600.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Center),
                    ..Section::default()
                },
                Section {
                    text,
                    screen_position: (1200.0, 0.0),
                    bounds: (600.0, f32::INFINITY),
                    layout: Layout::default().h_align(HorizontalAlign::Right),
                    ..Section::default()
                },
            ]
        })
        .collect();

    bench_variants(b, &variants, brush);
}

/// Renders a different set of sections each run by
/// cycling through the provided `variants`
fn bench_variants(
    b: &mut test::Bencher,
    variants: &[Vec<Section<'_>>],
    brush: GlyphBrushBuilder<'_>,
) {
    let _ = env_logger::try_init();

    let mut variants = variants.iter().cycle();
    let mut glyph_brush = brush.build();

    // once before, to warm up cache benches
    for s in variants.next().unwrap() {
        glyph_brush.queue(s);
    }
    glyph_brush
        .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
        .unwrap();

    b.iter(|| {
        for s in variants.next().unwrap() {
            glyph_brush.queue(s);
        }
        glyph_brush
            .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
            .unwrap();
    });
}

fn bench_varied_variants(
    b: &mut test::Bencher,
    variants: &[Vec<VariedSection<'_>>],
    brush: GlyphBrushBuilder<'_>,
) {
    let _ = env_logger::try_init();

    let mut variants = variants.iter().cycle();
    let mut glyph_brush = brush.build();

    // once before, to warm up cache benches
    for s in variants.next().unwrap() {
        glyph_brush.queue(s);
    }
    glyph_brush
        .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
        .unwrap();

    b.iter(|| {
        for s in variants.next().unwrap() {
            glyph_brush.queue(s);
        }
        glyph_brush
            .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
            .unwrap();
    });
}

fn bench(b: &mut test::Bencher, sections: &[Section<'_>], brush: GlyphBrushBuilder<'_>) {
    let _ = env_logger::try_init();

    let mut glyph_brush = brush.build();

    // once before, to warm up cache benches
    for section in sections {
        glyph_brush.queue(*section);
    }

    glyph_brush
        .process_queued(|_rect, _tex_data| {}, gl_to_vertex)
        .unwrap();

    b.iter(|| {
        for section in sections {
            glyph_brush.queue(*section);
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
