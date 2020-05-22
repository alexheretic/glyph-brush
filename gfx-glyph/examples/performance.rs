use gfx::{
    format::{Depth, Srgba8},
    Device,
};
use gfx_glyph::{ab_glyph::*, *};
use glutin::{
    event::{ElementState, Event, KeyboardInput, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
};
use old_school_gfx_glutin_ext::*;
use std::{env, error::Error};

const MAX_FONT_SIZE: f32 = 4000.0;

fn main() -> Result<(), Box<dyn Error>> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "gfx_glyph=warn");
    }

    env_logger::init();

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
        eprintln!(
            "You should probably run an example called 'performance' in release mode, \
            don't you think?\n    \
            e.g. use `cargo run --example performance --release`\n\n\
            If you really want to see debug performance set env var `yes_i_really_want_debug_mode`"
        );
        return Ok(());
    }

    let event_loop = glutin::event_loop::EventLoop::new();
    let title = "gfx_glyph rendering 30,000 glyphs - scroll to size, type to modify";
    let window_builder = glutin::window::WindowBuilder::new()
        .with_title(title)
        .with_inner_size(glutin::dpi::PhysicalSize::new(1024, 576));

    let (window_ctx, mut device, mut factory, mut main_color, mut main_depth) =
        glutin::ContextBuilder::new()
            .with_gfx_color_depth::<Srgba8, Depth>()
            .build_windowed(window_builder, &event_loop)?
            .init_gfx::<Srgba8, Depth>();

    let dejavu = FontRef::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?;
    let mut glyph_brush = GlyphBrushBuilder::using_font(dejavu)
        .initial_cache_size((2048, 2048))
        .draw_cache_position_tolerance(1.0)
        .build(factory.clone());

    let mut text: String = include_str!("loads-of-unicode.txt").into();
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut font_size: f32 = 25.0;
    let mut loop_helper = spin_sleep::LoopHelper::builder().build_without_target_rate();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                }
                | WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    window_ctx.resize(size);
                    window_ctx.update_gfx(&mut main_color, &mut main_depth);
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(keypress),
                            ..
                        },
                    ..
                } => {
                    if let VirtualKeyCode::Back = keypress {
                        text.pop();
                    }
                }
                WindowEvent::ReceivedCharacter(c) => {
                    if c != '\u{7f}' && c != '\u{8}' {
                        text.push(c);
                    }
                }
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
                    font_size = font_size.max(1.0).min(MAX_FONT_SIZE);
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                encoder.clear(&main_color, [0.02, 0.02, 0.02, 1.0]);

                let (width, height, ..) = main_color.get_dimensions();
                let (width, height) = (f32::from(width), f32::from(height));
                let scale = PxScale::from(font_size * window_ctx.window().scale_factor() as f32);

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
                    .draw(&mut encoder, &main_color)
                    .unwrap();

                encoder.flush(&mut device);
                window_ctx.swap_buffers().unwrap();
                device.cleanup();

                if let Some(rate) = loop_helper.report_rate() {
                    window_ctx
                        .window()
                        .set_title(&format!("{} - {:.0} FPS", title, rate));
                }

                loop_helper.loop_start();
            }
            _ => (),
        }
    });
}
