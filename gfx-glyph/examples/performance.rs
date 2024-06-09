mod init;

use gfx::{
    format::{Depth, Srgba8},
    Device,
};
use gfx_glyph::{ab_glyph::*, *};
use glutin::surface::GlSurface;
use glutin_winit::GlWindow;
use init::init_example;
use std::{env, error::Error, time::Duration};
use winit::{
    event::{ElementState, Event, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::ControlFlow,
    keyboard::{Key, NamedKey},
};

const MAX_FONT_SIZE: f32 = 4000.0;

fn main() -> Result<(), Box<dyn Error>> {
    init_example("performance");
    if cfg!(debug_assertions) && env::var("yes_i_really_want_debug_mode").is_err() {
        eprintln!(
            "You should probably run an example called 'performance' in release mode, \
            don't you think?\n    \
            If you really want to see debug performance set env var `yes_i_really_want_debug_mode`"
        );
        return Ok(());
    }

    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let title = "gfx_glyph rendering 30,000 glyphs - scroll to size, type to modify";
    let window_attrs = winit::window::Window::default_attributes()
        .with_title(title)
        .with_inner_size(winit::dpi::PhysicalSize::new(1024, 576));

    let old_school_gfx_glutin_ext::Init {
        window,
        gl_surface,
        gl_context,
        mut device,
        mut factory,
        mut color_view,
        mut depth_view,
        ..
    } = old_school_gfx_glutin_ext::window_builder(&event_loop, window_attrs)
        .build::<Srgba8, Depth>()?;

    let dejavu = FontRef::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?;
    let mut glyph_brush = GlyphBrushBuilder::using_font(dejavu)
        .initial_cache_size((2048, 2048))
        .draw_cache_position_tolerance(1.0)
        .build(factory.clone());

    let mut text: String = include_str!("loads-of-unicode.txt").into();
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut font_size: f32 = 25.0;
    let mut interval = spin_sleep_util::interval(Duration::from_secs(1) / 250);
    let mut reporter = spin_sleep_util::RateReporter::new(Duration::from_secs(1));
    let mut view_size = window.inner_size();

    event_loop.run(move |event, elwt| {
        match event {
            Event::AboutToWait => window.request_redraw(),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            logical_key,
                            state: ElementState::Pressed,
                            ..
                        },
                    ..
                } => match logical_key {
                    Key::Named(NamedKey::Escape) => elwt.exit(),
                    Key::Named(NamedKey::Backspace) => {
                        text.pop();
                    }
                    key => {
                        if let Some(str) = key.to_text() {
                            text.push_str(str);
                        }
                    }
                },
                WindowEvent::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(_, y),
                    ..
                } => {
                    // increase/decrease font size with mouse wheel
                    if y > 0.0 {
                        font_size += (font_size / 4.0).max(2.0)
                    } else {
                        font_size *= 4.0 / 5.0
                    };
                    font_size = font_size.clamp(1.0, MAX_FONT_SIZE);
                }
                WindowEvent::RedrawRequested => {
                    // handle resizes
                    let w_size = window.inner_size();
                    if view_size != w_size {
                        window.resize_surface(&gl_surface, &gl_context);
                        old_school_gfx_glutin_ext::resize_views(
                            w_size,
                            &mut color_view,
                            &mut depth_view,
                        );
                        view_size = w_size;
                    }

                    encoder.clear(&color_view, [0.02, 0.02, 0.02, 1.0]);

                    let (width, height, ..) = color_view.get_dimensions();
                    let (width, height) = (f32::from(width), f32::from(height));
                    let scale = PxScale::from(font_size * window.scale_factor() as f32);

                    // The section is all the info needed for the glyph brush to render a 'section' of text.
                    let section = Section::default()
                        .add_text(
                            Text::new(&text)
                                .with_scale(scale)
                                .with_color([0.8, 0.8, 0.8, 1.0]),
                        )
                        .with_bounds((width, height))
                        .with_layout(
                            Layout::default().line_breaker(BuiltInLineBreaker::AnyCharLineBreaker),
                        );

                    // Adds a section & layout to the queue for the next call to `use_queue().draw(..)`,
                    // this can be called multiple times for different sections that want to use the
                    // same font and gpu cache.
                    // This step computes the glyph positions, this is cached to avoid unnecessary
                    // recalculation.
                    glyph_brush.queue(&section);

                    // Finally once per frame you want to actually draw all the sections you've
                    // submitted with `queue` calls.
                    //
                    // Note: Drawing in the case the text is unchanged from the previous frame
                    // (a common case) is essentially free as the vertices are reused &  gpu cache
                    // updating interaction can be skipped.
                    glyph_brush
                        .use_queue()
                        .draw(&mut encoder, &color_view)
                        .unwrap();

                    encoder.flush(&mut device);
                    gl_surface.swap_buffers(&gl_context).unwrap();
                    device.cleanup();

                    if let Some(rate) = reporter.increment_and_report() {
                        window.set_title(&format!("{title} - {rate:.0} FPS"));
                    }
                    interval.tick();
                }
                _ => (),
            },
            _ => (),
        }
    })?;
    Ok(())
}
