//! An example of paragraph rendering
//! Controls
//!
//! * Scroll to modify font size
//! * Type to add/remove text
//! * Ctrl-Scroll to zoom in/out using a transform, this is cheap but notice how rusttype can't
//!   render at full quality without the correct pixel information.

extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate pretty_env_logger;
extern crate gfx_glyph;
extern crate cgmath;
extern crate spin_sleep;

use glutin::GlContext;
use gfx::{format, Device};
use std::env;
use gfx_glyph::*;

fn main() {
    pretty_env_logger::init().expect("log");

    // winit wayland is currently still wip
    if cfg!(target_os = "linux") && env::var("WINIT_UNIX_BACKEND").is_err() {
        env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

    if cfg!(debug_assertions) && env::var("yes_i_really_want_debug_mode").is_err() {
        eprintln!("Note: Release mode will improve performance greatly.\n    \
            e.g. use `cargo run --example complex_layout --release`");
    }

    let mut events_loop = glutin::EventsLoop::new();
    let title = "gfx_glyph example - resize to see multi-text layout";
    let window_builder = glutin::WindowBuilder::new()
        .with_title(title)
        .with_dimensions(1024, 576);
    let context = glutin::ContextBuilder::new();
    let (window, mut device, mut factory, mut main_color, mut main_depth) =
        gfx_window_glutin::init::<format::Srgba8, format::Depth>(window_builder, context, &events_loop);

    let mut builder = GlyphBrushBuilder::using_font(include_bytes!("Arial Unicode.ttf") as &[u8])
        .initial_cache_size((1024, 1024));
    let sans_font = FontId::default();
    let italic_font = builder.add_font(include_bytes!("OpenSans-Italic.ttf") as &[u8]);
    let serif_font = builder.add_font(include_bytes!("GaramondNo8-Reg.ttf") as &[u8]);
    let mono_font = builder.add_font(include_bytes!("../tests/DejaVuSansMono.ttf") as &[u8]);

    let mut glyph_brush = builder.build(factory.clone());

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut running = true;
    let mut loop_helper = spin_sleep::LoopHelper::builder().build_without_target_rate();

    while running {
        loop_helper.loop_start();

        events_loop.poll_events(|event| {
            use glutin::*;
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::KeyboardInput {
                        input: KeyboardInput { virtual_keycode: Some(VirtualKeyCode::Escape), .. },
                        ..
                    } | WindowEvent::Closed => running = false,
                    WindowEvent::Resized(width, height) => {
                        window.resize(width, height);
                        gfx_window_glutin::update_views(&window, &mut main_color, &mut main_depth);
                    },
                    _ => {},
                }
            }
        });

        encoder.clear(&main_color, [0.02, 0.02, 0.02, 1.0]);

        let (width, height, ..) = main_color.get_dimensions();
        let (width, height) = (width as f32, height as f32);

        glyph_brush.queue(Section2 {
            bounds: (width * 0.49, height),
            text: vec![
                SectionText {
                    text: "Lorem ipsum dolor sit amet, ferri simul omittantur eam eu, ",
                    scale: gfx_glyph::Scale::uniform(45.0),
                    color: [0.9, 0.3, 0.3, 1.0],
                    font_id: sans_font,
                },
                SectionText {
                    text: "dolorem",
                    scale: gfx_glyph::Scale::uniform(150.0),
                    color: [0.3, 0.9, 0.3, 1.0],
                    font_id: serif_font,
                },
                SectionText {
                    text: "Iriure vocibus est te, natum delicata dignissim pri ea.",
                    scale: gfx_glyph::Scale::uniform(25.0),
                    color: [0.3, 0.3, 0.9, 1.0],
                    font_id: sans_font,
                },
            ],
            ..Section2::default()
        });

        glyph_brush.queue(Section2 {
            bounds: (width * 0.49, height),
            screen_position: (width * 0.51, 0.0),
            text: vec![
                SectionText {
                    text: "foo += bar;",
                    scale: gfx_glyph::Scale::uniform(45.0),
                    color: [0.3, 0.3, 0.9, 1.0],
                    font_id: mono_font,
                },
                SectionText {
                    text: "eruditi habemus qualisque eam an. ",
                    scale: gfx_glyph::Scale::uniform(30.0),
                    color: [0.9, 0.3, 0.3, 1.0],
                    font_id: italic_font,
                },
                SectionText {
                    text: "Eu facilisi maluisset eos.",
                    scale: gfx_glyph::Scale::uniform(55.0),
                    color: [0.3, 0.9, 0.3, 1.0],
                    font_id: sans_font,
                },
            ],
            ..Section2::default()
        });

        glyph_brush.draw_queued(&mut encoder, &main_color, &main_depth).expect("draw");

        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();

        if let Some(rate) = loop_helper.report_rate() {
            window.set_title(&format!("{} - {:.0} FPS", title, rate));
        }
    }
}
