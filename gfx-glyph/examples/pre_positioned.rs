//!
use gfx::{format, Device};
use gfx_glyph::{rusttype, GlyphPositioner};
use std::{env, error::Error};
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
            "Note: Release mode will improve performance greatly.\n    \
             e.g. use `cargo run --example pre_positioned --release`"
        );
    }

    let mut events_loop = glutin::EventsLoop::new();
    let title = "gfx_glyph example";
    let window_builder = glutin::WindowBuilder::new()
        .with_title(title)
        .with_dimensions((1024, 576).into());
    let context = glutin::ContextBuilder::new();
    let (window_ctx, mut device, mut factory, mut main_color, mut main_depth) =
        gfx_window_glutin::init::<format::Srgba8, format::DepthStencil>(
            window_builder,
            context,
            &events_loop,
        )
        .unwrap();
    let window = window_ctx.window();

    let font: &[u8] = include_bytes!("../../fonts/OpenSans-Light.ttf");
    let font = gfx_glyph::Font::from_bytes(font)?;
    let mut glyph_brush = gfx_glyph::GlyphBrushBuilder::using_font(font.clone())
        .initial_cache_size((1024, 1024))
        .build(factory.clone());

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut running = true;
    let mut loop_helper = spin_sleep::LoopHelper::builder().build_with_target_rate(250.0);

    let (width, height, ..) = main_color.get_dimensions();
    let (width, height) = (f32::from(width), f32::from(height));

    let glyphs: Vec<_> = gfx_glyph::Layout::default().calculate_glyphs(
        &[font],
        &gfx_glyph::SectionGeometry {
            screen_position: (0.0, 0.0),
            bounds: (width, height),
        },
        &[gfx_glyph::SectionText {
            text: include_str!("lipsum.txt"),
            color: [0.8, 0.8, 0.8, 1.0],
            scale: rusttype::Scale::uniform(30.0),
            ..<_>::default()
        }],
    );

    while running {
        loop_helper.loop_start();

        events_loop.poll_events(|event| {
            use glutin::*;

            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => running = false,
                    WindowEvent::Resized(size) => {
                        window_ctx.resize(size.to_physical(window.get_hidpi_factor()));
                        gfx_window_glutin::update_views(
                            &window_ctx,
                            &mut main_color,
                            &mut main_depth,
                        );
                    }
                    _ => {}
                }
            }
        });

        encoder.clear(&main_color, [0.02, 0.02, 0.02, 1.0]);

        glyph_brush.queue_pre_positioned(
            glyphs.clone(),
            rusttype::Rect {
                min: rusttype::point(0.0, 0.0),
                max: rusttype::point(width, height),
            },
            0.0,
        );

        glyph_brush.use_queue().draw(&mut encoder, &main_color)?;

        encoder.flush(&mut device);
        window_ctx.swap_buffers()?;
        device.cleanup();

        if let Some(rate) = loop_helper.report_rate() {
            window.set_title(&format!("{} - {:.0} FPS", title, rate));
        }

        loop_helper.loop_sleep();
    }
    println!();
    Ok(())
}
