use gfx::{format, Device};
use gfx_glyph::*;
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
             e.g. use `cargo run --example depth --release`"
        );
    }

    let mut events_loop = glutin::EventsLoop::new();
    let title = "gfx_glyph example - resize to see multi-text layout";
    let window_builder = glutin::WindowBuilder::new()
        .with_title(title)
        .with_dimensions((700, 320).into());
    let context = glutin::ContextBuilder::new();
    let (window_ctx, mut device, mut factory, mut main_color, mut main_depth) =
        gfx_window_glutin::init::<format::Srgba8, format::Depth>(
            window_builder,
            context,
            &events_loop,
        )
        .unwrap();
    let window = window_ctx.window();

    let fonts: Vec<&[u8]> = vec![
        include_bytes!("../../fonts/DejaVuSans.ttf"),
        include_bytes!("../../fonts/OpenSans-Italic.ttf"),
    ];
    let italic_font = FontId(1);

    let mut glyph_brush = GlyphBrushBuilder::using_fonts_bytes(fonts)
        .initial_cache_size((512, 512))
        .build(factory.clone());

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut running = true;
    let mut loop_helper = spin_sleep::LoopHelper::builder().build_with_target_rate(250.0);

    while running {
        loop_helper.loop_start();

        events_loop.poll_events(|event| {
            use glutin::*;
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    }
                    | WindowEvent::CloseRequested => running = false,
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
        encoder.clear_depth(&main_depth, 1.0);

        let (width, height, ..) = main_color.get_dimensions();
        let (width, height) = (f32::from(width), f32::from(height));

        // first section is queued, and therefore drawn, first with lower z
        glyph_brush.queue(Section {
            screen_position: (width / 2.0, 100.0),
            bounds: (width, height - 100.0),
            text: "On top",
            scale: Scale::uniform(95.0),
            color: [0.8, 0.8, 0.8, 1.0],
            font_id: italic_font,
            layout: Layout::default().h_align(HorizontalAlign::Center),
            z: 0.2,
        });

        // 2nd section is drawn last but with higher z,
        // draws are subject to depth testing
        glyph_brush.queue(Section {
            bounds: (width, height),
            text: &include_str!("lipsum.txt").replace("\n\n", "").repeat(10),
            scale: Scale::uniform(30.0),
            color: [0.05, 0.05, 0.1, 1.0],
            z: 1.0,
            ..Section::default()
        });

        glyph_brush
            .use_queue()
            // Enable depth testing with default less-equal drawing and update the depth buffer
            .depth_target(&main_depth)
            .draw(&mut encoder, &main_color)?;

        encoder.flush(&mut device);
        window_ctx.swap_buffers()?;
        device.cleanup();

        if let Some(rate) = loop_helper.report_rate() {
            window.set_title(&format!("{} - {:.0} FPS", title, rate));
        }

        loop_helper.loop_sleep();
    }
    Ok(())
}
