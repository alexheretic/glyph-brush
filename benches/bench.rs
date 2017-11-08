#![cfg_attr(feature = "bench", feature(test))]
#[cfg(feature = "bench")]
extern crate test;

#[cfg(feature = "bench")]
extern crate gfx;
#[cfg(feature = "bench")]
extern crate gfx_core;
#[cfg(feature = "bench")]
extern crate gfx_glyph;
#[cfg(feature = "bench")]
extern crate gfx_window_glutin;
#[cfg(feature = "bench")]
extern crate glutin;
#[cfg(feature = "bench")]
extern crate pretty_env_logger;

#[cfg(feature = "bench")]
mod gfx_noop;

#[cfg(feature = "bench")]
const TEST_FONT: &[u8] = include_bytes!("../tests/DejaVuSansMono.ttf");

#[bench]
#[cfg(feature = "bench")]
fn render_3_medium_sections_fully(b: &mut ::test::Bencher) {
    use std::f32;
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
#[cfg(feature = "bench")]
// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_3_medium_sections_fully(b: &mut ::test::Bencher) {
    use std::f32;
    use gfx_glyph::*;

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
#[cfg(feature = "bench")]
fn render_1_large_section_partially(b: &mut ::test::Bencher) {
    use gfx_glyph::*;

    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT);
    let text = include_str!("lots_of_lipsum.txt");

    bench(b, &[Section { text, bounds: (600.0, 600.0), ..Section::default() }], brush);
}

#[bench]
#[cfg(feature = "bench")]
// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_1_large_section_partially(b: &mut ::test::Bencher) {
    use gfx_glyph::*;

    let brush = GlyphBrushBuilder::using_font_bytes(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false);
    let text = include_str!("lots_of_lipsum.txt");

    bench(b, &[Section { text, bounds: (600.0, 600.0), ..Section::default() }], brush);
}

#[bench]
#[cfg(feature = "bench")]
fn render_100_small_sections_fully(b: &mut ::test::Bencher) {
    use std::f32;
    use gfx_glyph::*;

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
#[cfg(feature = "bench")]
// Note: 'no_cache' here refers to the glyph positioning/drawing caches (not the gpu cache)
fn no_cache_render_100_small_sections_fully(b: &mut ::test::Bencher) {
    use std::f32;
    use gfx_glyph::*;

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

#[cfg(feature = "bench")]
fn bench(
    b: &mut ::test::Bencher,
    sections: &[gfx_glyph::Section<'static>],
    brush: gfx_glyph::GlyphBrushBuilder,
) {
    use gfx::format;
    use std::env;

    let _ = pretty_env_logger::init();

    // winit wayland is currently still wip
    if cfg!(target_os = "linux") && env::var("WINIT_UNIX_BACKEND").is_err() {
        env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

    // TODO use headless/fake
    let events_loop = glutin::EventsLoop::new();
    let (_window, _device, factory, main_color, main_depth) =
        gfx_window_glutin::init::<format::Srgba8, format::Depth>(
            glutin::WindowBuilder::new().with_dimensions(1, 1),
            glutin::ContextBuilder::new(),
            &events_loop,
        );
    let mut encoder: gfx::Encoder<_, _> = gfx_noop::NoopCommandBuffer.into();

    let mut glyph_brush = brush.build(factory.clone());

    // once before, to warm up cache benches
    for section in sections.iter() {
        glyph_brush.queue(*section);
    }
    glyph_brush.draw_queued(&mut encoder, &main_color, &main_depth).expect("draw");

    b.iter(|| {
        for section in sections.iter() {
            glyph_brush.queue(*section);
        }
        glyph_brush.draw_queued(&mut encoder, &main_color, &main_depth).expect("draw");
    });
}
