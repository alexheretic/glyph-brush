//! An example of paragraph rendering
//! Controls
//!
//! * Resize window to adjust layout
//! * Scroll to modify font size
//! * Type to add/remove text
//! * Ctrl-Scroll to zoom in/out using a transform, this is cheap but notice how rusttype can't
//!   render at full quality without the correct pixel information.

extern crate cgmath;
extern crate env_logger;
extern crate gfx;
extern crate gfx_glyph;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate spin_sleep;

use cgmath::{Matrix4, Rad, Transform};
use gfx::{format, Device};
use glutin::GlContext;
use std::env;
use std::f32::consts::PI as PI32;
use std::io;
use std::io::Write;

fn main() {
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
             e.g. use `cargo run --example paragraph --release`"
        );
    }

    let mut events_loop = glutin::EventsLoop::new();
    let title = "gfx_glyph example - scroll to size, type to modify, ctrl-scroll to gpu zoom, ctrl-shift-scroll to gpu rotate";
    let window_builder = glutin::WindowBuilder::new()
        .with_title(title)
        .with_dimensions(1024, 576);
    let context = glutin::ContextBuilder::new();
    let (window, mut device, mut factory, mut main_color, mut main_depth) =
        gfx_window_glutin::init::<format::Srgba8, format::DepthStencil>(
            window_builder,
            context,
            &events_loop,
        );

    let mut glyph_brush =
        gfx_glyph::GlyphBrushBuilder::using_font_bytes(include_bytes!("DejaVuSans.ttf") as &[u8])
            .initial_cache_size((1024, 1024))
            .build(factory.clone());

    let mut text: String = include_str!("lipsum.txt").into();

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut running = true;
    let mut font_size = gfx_glyph::Scale::uniform(18.0 * window.hidpi_factor());
    let mut zoom: f32 = 1.0;
    let mut angle = 0.0;
    let mut ctrl = false;
    let mut loop_helper = spin_sleep::LoopHelper::builder().build_with_target_rate(250.0);

    while running {
        loop_helper.loop_start();

        events_loop.poll_events(|event| {
            use glutin::*;

            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => running = false,
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(keypress),
                                ..
                            },
                        ..
                    } => match keypress {
                        VirtualKeyCode::Escape => running = false,
                        VirtualKeyCode::Back => {
                            text.pop();
                        }
                        VirtualKeyCode::LControl | VirtualKeyCode::RControl => ctrl = true,
                        _ => (),
                    },
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Released,
                                ..
                            },
                        ..
                    } => ctrl = false,
                    WindowEvent::ReceivedCharacter(c) => if c != '\u{7f}' && c != '\u{8}' {
                        text.push(c);
                    },
                    WindowEvent::Resized(width, height) => {
                        window.resize(width, height);
                        gfx_window_glutin::update_views(&window, &mut main_color, &mut main_depth);
                    }
                    WindowEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(_, y),
                        modifiers: ModifiersState { ctrl, shift, .. },
                        ..
                    } => {
                        if ctrl && shift {
                            if y > 0.0 {
                                angle += 0.02 * PI32;
                            }
                            else {
                                angle -= 0.02 * PI32;
                            }
                            if (angle % (PI32 * 2.0)).abs() < 0.01 {
                                angle = 0.0;
                            }
                            print!("\r                            \r");
                            print!("transform-angle -> {:.2} * π", angle / PI32);
                            io::stdout().flush().ok().unwrap();
                        }
                        else if ctrl && !shift {
                            let old_zoom = zoom;
                            // increase/decrease zoom
                            if y > 0.0 {
                                zoom += 0.1;
                            }
                            else {
                                zoom -= 0.1;
                            }
                            zoom = zoom.min(1.0).max(0.1);
                            if (zoom - old_zoom).abs() > 1e-2 {
                                print!("\r                            \r");
                                print!("transform-zoom -> {:.1}", zoom);
                                io::stdout().flush().ok().unwrap();
                            }
                        }
                        else {
                            // increase/decrease font size
                            let old_size = font_size.x;
                            let mut size = font_size.x / window.hidpi_factor();
                            if y > 0.0 {
                                size += (size / 4.0).max(2.0)
                            }
                            else {
                                size *= 4.0 / 5.0
                            };
                            size = size.max(1.0);
                            font_size = gfx_glyph::Scale::uniform(size * window.hidpi_factor());
                            if (font_size.x - old_size).abs() > 1e-2 {
                                print!("\r                            \r");
                                print!("font-size -> {:.1}", font_size.x);
                                io::stdout().flush().ok().unwrap();
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        encoder.clear(&main_color, [0.02, 0.02, 0.02, 1.0]);

        let (width, height, ..) = main_color.get_dimensions();
        let (width, height) = (f32::from(width), f32::from(height));
        let scale = font_size;

        // The section is all the info needed for the glyph brush to render a 'section' of text.
        // Use `..Section::default()` to skip the bits you don't care about
        let section = gfx_glyph::Section {
            text: &text,
            scale,
            screen_position: (0.0, 0.0),
            bounds: (width / 3.15, height),
            color: [0.9, 0.3, 0.3, 1.0],
            ..Section::default()
        };

        // bounds of a section can be fetched with `pixel_bounds`
        let _bounds: Option<Rect<i32>> = glyph_brush.pixel_bounds(section);

        // Adds a section & layout to the queue for the next call to `draw_queued`, this
        // can be called multiple times for different sections that want to use the same
        // font and gpu cache
        // This step computes the glyph positions, this is cached to avoid unnecessary recalculation
        glyph_brush.queue(section);

        use gfx_glyph::*;
        glyph_brush.queue(Section {
            text: &text,
            scale,
            screen_position: (width / 2.0, 0.0),
            bounds: (width / 3.15, height),
            color: [0.3, 0.9, 0.3, 1.0],
            layout: Layout::default().h_align(HorizontalAlign::Center),
            ..Section::default()
        });

        glyph_brush.queue(Section {
            text: &text,
            scale,
            screen_position: (width, 0.0),
            bounds: (width / 3.15, height),
            color: [0.3, 0.3, 0.9, 1.0],
            layout: Layout::default().h_align(HorizontalAlign::Right),
            ..Section::default()
        });

        // Note: Can be drawn simply with the below, when transforms are not needed:
        // `glyph_brush.draw_queued(&mut encoder, &main_color, &main_depth).expect("draw");`

        // Here an example transform is used as a cheap zoom out (controlled with ctrl-scroll)
        let transform_zoom = Matrix4::from_scale(zoom);

        // Orthographic rotation transform
        let transform_rotate = {
            let aspect = width / height;
            let zoom = 1.0;
            let origin = (0.0, 0.0); // top-corner: `let origin = (1.0 * aspect, -1.0);`
            let projection = cgmath::ortho(
                origin.0 - zoom * aspect,
                origin.0 + zoom * aspect,
                origin.1 - zoom,
                origin.1 + zoom,
                1.0,
                -1.0,
            );
            projection * Matrix4::from_angle_z(Rad(angle)) * projection.inverse_transform().unwrap()
        };

        let transform = transform_rotate * transform_zoom;

        // Finally once per frame you want to actually draw all the sections you've submitted
        // with `queue` calls.
        //
        // Note: Drawing in the case the text is unchanged from the previous frame (a common case)
        // is essentially free as the vertices are reused & gpu cache updating interaction
        // can be skipped.
        glyph_brush
            .draw_queued_with_transform(transform.into(), &mut encoder, &main_color, &main_depth)
            .expect("draw");

        encoder.flush(&mut device);
        window.swap_buffers().unwrap();
        device.cleanup();

        if let Some(rate) = loop_helper.report_rate() {
            window.set_title(&format!("{} - {:.0} FPS", title, rate));
        }

        loop_helper.loop_sleep();
    }
    println!();
}
