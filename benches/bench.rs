#![cfg_attr(feature = "bench", feature(test))]
#[cfg(feature = "bench")]
extern crate test;

extern crate gfx;
extern crate gfx_core;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate time;
extern crate pretty_env_logger;
extern crate gfx_glyph;

#[cfg(feature = "bench")]
mod gfx_noop;

#[cfg(feature = "bench")]
const TEST_FONT: &[u8] = include_bytes!("../tests/DejaVuSansMono.ttf");

#[bench]
#[cfg(feature = "bench")]
fn render_3_medium_sections_fully(b: &mut ::test::Bencher) {
    use std::f32;
    use gfx_glyph::*;

    let brush = GlyphBrushBuilder::using_font(TEST_FONT);
    let text = include_str!("lipsum.txt");

    bench(b,
        &[(
            StaticSection {
                text,
                bounds: (600.0, f32::INFINITY),
                ..StaticSection::default() },
            Layout::Wrap(StandardLineBreaker, HorizontalAlign::Left)
        ), (
            StaticSection {
                text,
                screen_position: (600.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                ..StaticSection::default() },
            Layout::Wrap(StandardLineBreaker, HorizontalAlign::Center)
        ), (
            StaticSection {
                text,
                screen_position: (1200.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                ..StaticSection::default() },
            Layout::Wrap(StandardLineBreaker, HorizontalAlign::Right)
        )],
        brush);
}

#[bench]
#[cfg(feature = "bench")]
fn no_cache_render_3_medium_sections_fully(b: &mut ::test::Bencher) {
    use std::f32;
    use gfx_glyph::*;

    let brush = GlyphBrushBuilder::using_font(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false);
    let text = include_str!("lipsum.txt");

    bench(b,
        &[(
            StaticSection {
                text,
                bounds: (600.0, f32::INFINITY),
                ..StaticSection::default() },
            Layout::Wrap(StandardLineBreaker, HorizontalAlign::Left)
        ), (
            StaticSection {
                text,
                screen_position: (600.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                ..StaticSection::default() },
            Layout::Wrap(StandardLineBreaker, HorizontalAlign::Center)
        ), (
            StaticSection {
                text,
                screen_position: (1200.0, 0.0),
                bounds: (600.0, f32::INFINITY),
                ..StaticSection::default() },
            Layout::Wrap(StandardLineBreaker, HorizontalAlign::Right)
        )],
        brush);
}

#[bench]
#[cfg(feature = "bench")]
fn render_1_large_section_partially(b: &mut ::test::Bencher) {
    use gfx_glyph::*;

    let brush = GlyphBrushBuilder::using_font(TEST_FONT);
    let text = include_str!("lots_of_lipsum.txt");

    bench(b,
        &[(StaticSection {
            text,
            bounds: (600.0, 600.0),
            ..StaticSection::default()
        }, Layout::default())],
        brush);
}

#[bench]
#[cfg(feature = "bench")]
fn no_cache_render_1_large_section_partially(b: &mut ::test::Bencher) {
    use gfx_glyph::*;

    let brush = GlyphBrushBuilder::using_font(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false);
    let text = include_str!("lots_of_lipsum.txt");

    bench(b,
        &[(StaticSection {
            text,
            bounds: (600.0, 600.0),
            ..StaticSection::default()
        }, Layout::default())],
        brush);
}

#[bench]
#[cfg(feature = "bench")]
fn render_100_small_sections_fully(b: &mut ::test::Bencher) {
    use std::f32;
    use gfx_glyph::*;

    let brush = GlyphBrushBuilder::using_font(TEST_FONT);
    let text = include_str!("small_lipsum.txt");

    let mut section_layouts = vec![];
    for i in 0..100 {
        section_layouts.push((StaticSection {
            text,
            screen_position: (i as f32, 0.0),
            bounds: (100.0, f32::INFINITY),
            ..StaticSection::default()
        }, Layout::default()));
    }

    bench(b, &section_layouts, brush);
}

#[bench]
#[cfg(feature = "bench")]
fn no_cache_render_100_small_sections_fully(b: &mut ::test::Bencher) {
    use std::f32;
    use gfx_glyph::*;

    let brush = GlyphBrushBuilder::using_font(TEST_FONT)
        .cache_glyph_positioning(false)
        .cache_glyph_drawing(false);
    let text = include_str!("small_lipsum.txt");

    let mut section_layouts = vec![];
    for i in 0..100 {
        section_layouts.push((StaticSection {
            text,
            screen_position: (i as f32, 0.0),
            bounds: (100.0, f32::INFINITY),
            ..StaticSection::default()
        }, Layout::default()));
    }

    bench(b, &section_layouts, brush);
}

#[cfg(feature = "bench")]
fn bench<L: gfx_glyph::LineBreaker>(
    b: &mut ::test::Bencher,
    sections: &[(gfx_glyph::StaticSection, gfx_glyph::Layout<L>)],
    brush: gfx_glyph::GlyphBrushBuilder)
{
    use gfx::format;
    use std::env;

    let _ = pretty_env_logger::init();

    // winit wayland is currently still wip
    if cfg!(target_os = "linux") && env::var("WINIT_UNIX_BACKEND").is_err() {
        env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

    // TODO use headless/fake
    let events_loop = glutin::EventsLoop::new();
    let (_window, _device, factory, main_color, _main_depth) =
        gfx_window_glutin::init::<format::Srgba8, format::Depth>(
            glutin::WindowBuilder::new().with_dimensions(1, 1),
            glutin::ContextBuilder::new(),
            &events_loop);
    let mut encoder: gfx::Encoder<_, _> = gfx_noop::NoopCommandBuffer.into();

    let mut glyph_brush = brush.build(factory.clone());

    b.iter(|| {
        for &(ref section, ref layout) in sections.iter() {
            glyph_brush.queue(section.clone(), layout);
        }
        glyph_brush.draw_queued(&mut encoder, &main_color).expect("draw");
    });
}
