//! An example of rendering multiple fonts, sizes & colours within a single layout
//! Controls
//!
//! * Resize window to adjust layout
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
             e.g. use `cargo run --example varied --release`"
        );
    }

    let mut events_loop = glutin::EventsLoop::new();
    let title = "gfx_glyph example - resize to see multi-text layout";
    let window_builder = glutin::WindowBuilder::new()
        .with_title(title)
        .with_dimensions((1024, 576).into());
    let context = glutin::ContextBuilder::new().with_vsync(false);
    let (window_ctx, mut device, mut factory, mut main_color, mut main_depth) =
        gfx_window_glutin::init::<format::Srgba8, format::Depth>(
            window_builder,
            context,
            &events_loop,
        )
        .unwrap();
    let window = window_ctx.window();

    let mut builder =
        GlyphBrushBuilder::using_font_bytes(include_bytes!("../../fonts/DejaVuSans.ttf") as &[u8])
            .initial_cache_size((512, 512));
    let sans_font = FontId::default();
    let italic_font =
        builder.add_font_bytes(include_bytes!("../../fonts/OpenSans-Italic.ttf") as &[u8]);
    let serif_font =
        builder.add_font_bytes(include_bytes!("../../fonts/GaramondNo8-Reg.ttf") as &[u8]);
    let mono_font =
        builder.add_font_bytes(include_bytes!("../../fonts/DejaVuSansMono.ttf") as &[u8]);

    let mut glyph_brush = builder.build(factory.clone());

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

        let (width, height, ..) = main_color.get_dimensions();
        let (width, height) = (f32::from(width), f32::from(height));

        glyph_brush.queue(VariedSection {
            screen_position: (0.0, height / 2.0),
            bounds: (width * 0.49, height),
            text: vec![
                SectionText {
                    text: "Lorem ipsum dolor sit amet, ferri simul omittantur eam eu, ",
                    scale: Scale::uniform(45.0),
                    color: [0.9, 0.3, 0.3, 1.0],
                    font_id: sans_font,
                },
                SectionText {
                    text: "dolorem",
                    scale: Scale::uniform(150.0),
                    color: [0.3, 0.9, 0.3, 1.0],
                    font_id: serif_font,
                },
                SectionText {
                    text: " Iriure vocibus est te, natum delicata dignissim pri ea.",
                    scale: Scale::uniform(25.0),
                    color: [0.3, 0.3, 0.9, 1.0],
                    font_id: sans_font,
                },
            ],
            layout: Layout::default().v_align(VerticalAlign::Center),
            ..VariedSection::default()
        });

        glyph_brush.queue(VariedSection {
            screen_position: (width, height / 2.0),
            bounds: (width * 0.49, height),
            text: vec![
                SectionText {
                    text: "foo += bar;",
                    scale: Scale::uniform(45.0),
                    color: [0.3, 0.3, 0.9, 1.0],
                    font_id: mono_font,
                },
                SectionText {
                    text: " eruditi habemus qualisque eam an. No atqui apeirian phaedrum pri ex, hinc omnes sapientem. ",
                    scale: Scale::uniform(30.0),
                    color: [0.9, 0.3, 0.3, 1.0],
                    font_id: italic_font,
                },
                SectionText {
                    text: "Eu facilisi maluisset eos.",
                    scale: Scale::uniform(55.0),
                    color: [0.3, 0.9, 0.3, 1.0],
                    font_id: sans_font,
                },
                SectionText {
                    text: " ius nullam impetus. ",
                    scale: Scale { x: 25.0, y: 45.0 },
                    color: [0.9, 0.9, 0.3, 1.0],
                    font_id: serif_font,
                },
                SectionText {
                    text: "Ut quo elitr viderer constituam, pro omnesque forensibus at. Timeam scaevola mediocrem ut pri, te pro congue delicatissimi. Mei wisi nostro imperdiet ea, ridens salutatus per no, ut viris partem disputationi sit. Exerci eripuit referrentur vix at, sale mediocrem repudiare per te, modus admodum an eam. No vocent indoctum vis, ne quodsi patrioque vix. Vocent labores omittam et usu.",
                    scale: Scale::uniform(22.0),
                    color: [0.8, 0.3, 0.5, 1.0],
                    font_id: italic_font,
                },
            ],
            layout: Layout::default().h_align(HorizontalAlign::Right).v_align(VerticalAlign::Center),
            ..VariedSection::default()
        });

        glyph_brush.use_queue().draw(&mut encoder, &main_color)?;

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
