extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate time;
extern crate pretty_env_logger;
extern crate gfx_glyph;
extern crate spin_sleep;

use glutin::GlContext;
use gfx::{format, Device};
use std::env;
use gfx_glyph::*;

fn main() {
    pretty_env_logger::init().expect("log");

    if cfg!(target_os = "linux") {
        // winit wayland is currently still wip
        if env::var("WINIT_UNIX_BACKEND").is_err() {
            env::set_var("WINIT_UNIX_BACKEND", "x11");
        }
        // disables vsync sometimes on x11
        if env::var("vblank_mode").is_err() {
            env::set_var("vblank_mode", "0");
        }
    }
    if cfg!(debug_assertions) && env::var("yes_i_really_want_debug_mode").is_err() {
        eprintln!("You should probably run an example called 'performance' in release mode, \
            don't you think?\n    \
            e.g. use `cargo run --example performance --release`\n\n\
            If you really want to see debug performance set env var `yes_i_really_want_debug_mode`");
        return;
    }

    let mut events_loop = glutin::EventsLoop::new();
    let title = "gfx_glyph rendering 100,000 glyphs - scroll to size, type to modify";
    let window_builder = glutin::WindowBuilder::new()
        .with_title(title)
        .with_dimensions(1024, 576);
    let context = glutin::ContextBuilder::new()
        .with_vsync(false);
    let (window, mut device, mut factory, mut main_color, mut main_depth) =
        gfx_window_glutin::init::<format::Srgba8, format::Depth>(window_builder, context, &events_loop);

    let mut glyph_brush = GlyphBrushBuilder::using_font(include_bytes!("Arial Unicode.ttf") as &[u8])
        .initial_cache_size((2048, 2048))
        .build(factory.clone());

    let mut text: String = include_str!("100000_items.txt").into();

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut running = true;
    let mut font_size = Scale::uniform(25.0 * window.hidpi_factor());
    let mut loop_helper = spin_sleep::LoopHelper::builder().build_without_target_rate();

    while running {
        loop_helper.loop_start();

        events_loop.poll_events(|event| {
            use glutin::*;

            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::Closed => running = false,
                    WindowEvent::KeyboardInput {
                        input: KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(keypress),
                            .. },
                        ..
                    } => match keypress {
                        VirtualKeyCode::Escape => running = false,
                        VirtualKeyCode::Back => { text.pop(); },
                        _ => (),
                    },
                    WindowEvent::ReceivedCharacter(c) => if c != '\u{7f}' && c != '\u{8}' {
                        text.push(c);
                    },
                    WindowEvent::Resized(width, height) => {
                        window.resize(width, height);
                        gfx_window_glutin::update_views(&window, &mut main_color, &mut main_depth);
                    },
                    WindowEvent::MouseWheel{ delta: MouseScrollDelta::LineDelta(_, y), .. } => {
                        // increase/decrease font size with mouse wheel
                        let mut size = font_size.x / window.hidpi_factor();
                        if y > 0.0 { size += (size / 4.0).max(2.0) }
                        else { size *= 4.0 / 5.0 };
                        size = size.max(1.0);
                        font_size = Scale::uniform(size * window.hidpi_factor());
                    },
                    _ => {},
                }
            }
        });

        encoder.clear(&main_color, [0.02, 0.02, 0.02, 1.0]);

        let (width, height, ..) = main_color.get_dimensions();

        // The section is all the info needed for the glyph brush to render a 'section' of text
        // can use `..Section::default()` to skip the bits you don't care about
        // also see convenience variants StaticSection & OwnedSection
        let section = Section {
            text: &text,
            scale: font_size,
            bounds: (width as f32, height as f32),
            color: [0.8, 0.8, 0.8, 1.0],
            ..Section::default()
        };

        // the lib needs layout logic to render the glyphs, ie a gfx_glyph::GlyphPositioner
        // See the built-in ones, ie Layout::default()
        // This is an example of implementing your own, see below
        let layout = CustomContiguousParagraphLayout;

        // Adds a section & layout to the queue for the next call to `draw_queued`, this
        // can be called multiple times for different sections that want to use the same
        // font and gpu cache
        // This step computes the glyph positions, this is cached to avoid unnecessary recalculation
        glyph_brush.queue(section, &layout);

        // Finally once per frame you want to actually draw all the sections you've submitted
        // with `queue` calls.
        //
        // Note: Drawing in the case the text is unchanged from the previous frame (a common case)
        // is essentially free as the vertices are reused &  gpu cache updating interaction
        // can be skipped.
        glyph_brush.draw_queued(&mut encoder, &main_color).expect("draw");

        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();

        if let Some(rate) = loop_helper.report_rate() {
            window.set_title(&format!("{} - {:.0} FPS", title, rate));
        }
    }
    println!();
}

/// Example of a custom layout, ie different from the built-in ones see `Layout`
/// This one is like the default left aligner, but ignores new lines to fill the
/// screen with characters. Note Hash is required in order to cache the glyph positioning, as
/// such you'll notice `calculate_glyphs` is only called once per distinct section
#[derive(Debug, Clone, Copy, Hash)]
pub struct CustomContiguousParagraphLayout;

impl gfx_glyph::GlyphPositioner for CustomContiguousParagraphLayout {

    /// Calculate a sequence of positioned glyphs to render
    fn calculate_glyphs<'a, G>(&self, font: &Font, section: G)
        -> Vec<PositionedGlyph>
        where G: Into<GlyphInfo<'a>>
    {
        let mut glyph_info = section.into();
        let original_screen_x = glyph_info.screen_position.0;
        let original_bound_w = glyph_info.bounds.0;

        let v_metrics = font.v_metrics(glyph_info.scale);
        let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;

        let mut out = vec![];
        loop {
            let (glyphs, leftover) = Layout::SingleLine(AnyCharLineBreaker, HorizontalAlign::Left)
                .calculate_glyphs_and_leftover(font, &glyph_info);
            out.extend_from_slice(&glyphs);
            match leftover {
                Some(LayoutLeftover::HardBreak(point, leftover)) => {
                    // ignore newlines just keep rendering on the same line
                    glyph_info = leftover;
                    glyph_info.screen_position.0 = point.x;
                    glyph_info.bounds.0 = original_bound_w - (point.x - original_screen_x);
                }
                Some(LayoutLeftover::OutOfWidthBound(_, leftover)) => {
                    // use the next line when we run out of width
                    glyph_info = leftover;
                    glyph_info.screen_position.1 += advance_height;
                    glyph_info.screen_position.0 = original_screen_x;
                    glyph_info.bounds.1 -= advance_height;
                    glyph_info.bounds.0 = original_bound_w;
                    if glyph_info.bounds.1 < 0.0 { break; }
                }
                Some(LayoutLeftover::OutOfHeightBound(..)) | None => break,
            }
        }
        out
    }
    /// Bounds rectangle is the same as built-in left align
    fn bounds_rect<'a, G: Into<GlyphInfo<'a>>>(&self, section: G) -> Rect<f32> {
        Layout::SingleLine(AnyCharLineBreaker, HorizontalAlign::Left).bounds_rect(section)
    }
}
