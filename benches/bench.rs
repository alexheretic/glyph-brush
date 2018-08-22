#![feature(test)]
extern crate test;

extern crate env_logger;
extern crate gfx;
extern crate gfx_core;
extern crate gfx_device_gl;
extern crate gfx_glyph;
extern crate gfx_window_glutin;
extern crate glutin;

mod gfx_noop;

use gfx_glyph::*;
use std::f32;

const TEST_FONT: &[u8] = include_bytes!("../tests/DejaVuSansMono.ttf");

#[bench]
fn render_3_medium_sections_fully(b: &mut ::test::Bencher) {
    use gfx_glyph::*;

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
fn no_cache_render_3_medium_sections_fully(b: &mut ::test::Bencher) {
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
fn render_1_large_section_partially(b: &mut ::test::Bencher) {
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
fn no_cache_render_1_large_section_partially(b: &mut ::test::Bencher) {
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
fn render_v_center_1_large_section_partially(b: &mut ::test::Bencher) {
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
fn no_cache_render_v_center_1_large_section_partially(b: &mut ::test::Bencher) {
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
fn render_v_bottom_1_large_section_partially(b: &mut ::test::Bencher) {
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
fn no_cache_render_v_bottom_1_large_section_partially(b: &mut ::test::Bencher) {
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
fn render_100_small_sections_fully(b: &mut ::test::Bencher) {
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
fn no_cache_render_100_small_sections_fully(b: &mut ::test::Bencher) {
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
fn continually_modify_end_text_of_1_of_3(b: &mut ::test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lipsum.txt");

    let string_variants = vec![
        text.to_owned(),
        text.to_owned() + "a",
        text.to_owned() + "ab",
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
        }).collect();

    bench_variants(b, &variants, brush);
}

#[bench]
/// section is rendered with text edits each run to the beginning
fn continually_modify_start_text_of_1_of_3(b: &mut ::test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lipsum.txt");

    let string_variants = vec![
        text.to_owned(),
        "a".to_owned() + text,
        "ab".to_owned() + text,
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
        }).collect();

    bench_variants(b, &variants, brush);
}

#[bench]
/// section is rendered with text edits each run to the middle
fn continually_modify_middle_text_of_1_of_3(b: &mut ::test::Bencher) {
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
        }).collect();

    bench_variants(b, &variants, brush);
}

#[bench]
/// section is rendered with the bounds redefined each run to the middle
fn continually_modify_bounds_of_1_of_3(b: &mut ::test::Bencher) {
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
        }).collect();

    bench_variants(b, &variants, brush);
}

#[bench]
/// section is rendered with the bounds redefined each run to the middle
fn continually_modify_z_of_1_of_3(b: &mut ::test::Bencher) {
    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lipsum.txt");

    let variants: Vec<_> = vec![0.1, 0.2, 0.7]
        .into_iter()
        .map(|z| {
            vec![
                Section {
                    text,
                    z,
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
        }).collect();

    bench_variants(b, &variants, brush);
}

#[bench]
/// section is rendered with the bounds redefined each run to the middle
fn continually_modify_position_of_1_of_3(b: &mut ::test::Bencher) {
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
        }).collect();

    bench_variants(b, &variants, brush);
}

/// Renders a different set of sections each run by
/// cycling through the provided `variants`
fn bench_variants(
    b: &mut ::test::Bencher,
    variants: &[Vec<gfx_glyph::Section>],
    brush: gfx_glyph::GlyphBrushBuilder,
) {
    let _ = env_logger::try_init();

    let mut variants = variants.iter().cycle();

    let (_context, _device, factory, main_color, main_depth) = headless_gl_init();
    let mut encoder: gfx::Encoder<_, _> = gfx_noop::NoopCommandBuffer.into();

    let mut glyph_brush = brush.build(factory.clone());

    // once before, to warm up cache benches
    for s in variants.next().unwrap() {
        glyph_brush.queue(s);
    }
    glyph_brush
        .draw_queued(&mut encoder, &main_color, &main_depth)
        .expect("draw");

    b.iter(|| {
        for s in variants.next().unwrap() {
            glyph_brush.queue(s);
        }
        glyph_brush
            .draw_queued(&mut encoder, &main_color, &main_depth)
            .expect("draw");
    });
}

fn bench(
    b: &mut ::test::Bencher,
    sections: &[gfx_glyph::Section],
    brush: gfx_glyph::GlyphBrushBuilder,
) {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "gfx_glyph=warn");
    }

    let _ = env_logger::try_init();

    let (_context, _device, factory, main_color, main_depth) = headless_gl_init();
    let mut encoder: gfx::Encoder<_, _> = gfx_noop::NoopCommandBuffer.into();

    let mut glyph_brush = brush.build(factory.clone());

    // once before, to warm up cache benches
    for section in sections.iter() {
        glyph_brush.queue(*section);
    }
    glyph_brush
        .draw_queued(&mut encoder, &main_color, &main_depth)
        .expect("draw");

    b.iter(|| {
        for section in sections.iter() {
            glyph_brush.queue(*section);
        }
        glyph_brush
            .draw_queued(&mut encoder, &main_color, &main_depth)
            .expect("draw");
    });
}

fn headless_gl_init() -> (
    glutin::HeadlessContext,
    gfx_device_gl::Device,
    gfx_device_gl::Factory,
    gfx::handle::RenderTargetView<gfx_device_gl::Resources, gfx::format::Srgba8>,
    gfx::handle::DepthStencilView<gfx_device_gl::Resources, gfx::format::Depth>,
) {
    use gfx::format;
    use gfx::format::Formatted;
    use gfx_core::memory::Typed;
    use glutin::GlContext;

    let (width, height) = (400, 300);
    let context = glutin::HeadlessRendererBuilder::new(width, height)
        .with_gl_profile(glutin::GlProfile::Core)
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)))
        .build()
        .unwrap();

    unsafe { context.make_current().unwrap() };
    let (device, factory) =
        gfx_device_gl::create(|s| context.get_proc_address(s) as *const std::os::raw::c_void);

    let (color_view, ds_view) = gfx_device_gl::create_main_targets_raw(
        (width as _, height as _, 1, gfx::texture::AaMode::Single),
        format::Srgba8::get_format().0,
        format::Depth::get_format().0,
    );
    (
        context,
        device,
        factory,
        Typed::new(color_view),
        Typed::new(ds_view),
    )
}
