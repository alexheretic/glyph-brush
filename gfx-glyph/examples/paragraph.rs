//! An example of paragraph rendering
//! Controls
//!
//! * Resize window to adjust layout
//! * Scroll to modify font size
//! * Type to add/remove text
//! * Ctrl-Scroll to zoom in/out using a transform, this is cheap but notice how ab_glyph can't
//!   render at full quality without the correct pixel information.
mod init;

use cgmath::{Matrix4, Rad, Transform, Vector3};
use gfx::{
    format::{Depth, Srgba8},
    Device,
};
use gfx_glyph::ab_glyph;
use glutin::surface::GlSurface;
use glutin_winit::GlWindow;
use init::init_example;
use std::{
    error::Error,
    f32::consts::PI as PI32,
    io::{self, Write},
};
use winit::{
    event::{
        ElementState, Event, KeyboardInput, ModifiersState, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::ControlFlow,
};

const MAX_FONT_SIZE: f32 = 2000.0;

fn main() -> Result<(), Box<dyn Error>> {
    init_example("paragraph");

    let event_loop = winit::event_loop::EventLoop::new();
    let title = "gfx_glyph example - scroll to size, type to modify, ctrl-scroll \
                 to gpu zoom, ctrl-shift-scroll to gpu rotate";
    let window_builder = winit::window::WindowBuilder::new()
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
    } = old_school_gfx_glutin_ext::window_builder(&event_loop, window_builder)
        .build::<Srgba8, Depth>()?;

    let font = ab_glyph::FontArc::try_from_slice(include_bytes!("../../fonts/OpenSans-Light.ttf"))?;
    let mut glyph_brush = gfx_glyph::GlyphBrushBuilder::using_font(font)
        .initial_cache_size((1024, 1024))
        .build(factory.clone());

    let mut text: String = include_str!("lipsum.txt").into();

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    // let mut running = true;
    let mut font_size: f32 = 18.0;
    let mut zoom: f32 = 1.0;
    let mut angle = 0.0;
    let mut loop_helper = spin_sleep::LoopHelper::builder().build_with_target_rate(250.0);
    let mut view_size = window.inner_size();

    let mut modifiers = ModifiersState::default();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::ModifiersChanged(new_mods) => modifiers = new_mods,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                }
                | WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Back),
                            ..
                        },
                    ..
                } => {
                    text.pop();
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
                    let ctrl = modifiers.ctrl();
                    let shift = modifiers.shift();
                    if ctrl && shift {
                        if y > 0.0 {
                            angle += 0.02 * PI32;
                        } else {
                            angle -= 0.02 * PI32;
                        }
                        if (angle % (PI32 * 2.0)).abs() < 0.01 {
                            angle = 0.0;
                        }
                        print!("\r                            \r");
                        print!("transform-angle -> {:.2} * π", angle / PI32);
                        let _ = io::stdout().flush();
                    } else if ctrl && !shift {
                        let old_zoom = zoom;
                        // increase/decrease zoom
                        if y > 0.0 {
                            zoom += 0.1;
                        } else {
                            zoom -= 0.1;
                        }
                        zoom = zoom.clamp(0.1, 1.0);
                        if (zoom - old_zoom).abs() > 1e-2 {
                            print!("\r                            \r");
                            print!("transform-zoom -> {zoom:.1}");
                            let _ = io::stdout().flush();
                        }
                    } else {
                        // increase/decrease font size
                        let old_size = font_size;
                        let mut size = font_size;
                        if y > 0.0 {
                            size += (size / 4.0).max(2.0)
                        } else {
                            size *= 4.0 / 5.0
                        };
                        font_size = size.clamp(1.0, MAX_FONT_SIZE);
                        if (font_size - old_size).abs() > 1e-2 {
                            print!("\r                            \r");
                            print!("font-size -> {font_size:.1}");
                            let _ = io::stdout().flush();
                        }
                    }
                }
                _ => (),
            },
            Event::MainEventsCleared => {
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
                let scale = font_size * window.scale_factor() as f32;

                // The section is all the info needed for the glyph brush to render a 'section' of text.
                let section = gfx_glyph::Section::default()
                    .add_text(
                        Text::new(&text)
                            .with_scale(scale)
                            .with_color([0.9, 0.3, 0.3, 1.0]),
                    )
                    .with_bounds((width / 3.15, height));

                // Adds a section & layout to the queue for the next call to `use_queue().draw(..)`,
                // this can be called multiple times for different sections that want to use the
                // same font and gpu cache.
                // This step computes the glyph positions, this is cached to avoid unnecessary
                // recalculation.
                glyph_brush.queue(&section);

                use gfx_glyph::*;
                glyph_brush.queue(
                    Section::default()
                        .add_text(
                            Text::new(&text)
                                .with_scale(scale)
                                .with_color([0.3, 0.9, 0.3, 1.0]),
                        )
                        .with_screen_position((width / 2.0, height / 2.0))
                        .with_bounds((width / 3.15, height))
                        .with_layout(
                            Layout::default()
                                .h_align(HorizontalAlign::Center)
                                .v_align(VerticalAlign::Center),
                        ),
                );

                glyph_brush.queue(
                    Section::default()
                        .add_text(
                            Text::new(&text)
                                .with_scale(scale)
                                .with_color([0.3, 0.3, 0.9, 1.0]),
                        )
                        .with_screen_position((width, height))
                        .with_bounds((width / 3.15, height))
                        .with_layout(
                            Layout::default()
                                .h_align(HorizontalAlign::Right)
                                .v_align(VerticalAlign::Bottom),
                        ),
                );

                // Rotation
                let offset =
                    Matrix4::from_translation(Vector3::new(-width / 2.0, -height / 2.0, 0.0));
                let rotation = offset.inverse_transform().unwrap()
                    * Matrix4::from_angle_z(Rad(angle))
                    * offset;

                // Default projection
                let projection: Matrix4<f32> = gfx_glyph::default_transform(&color_view).into();

                // Here an example transform is used as a cheap zoom out (controlled with ctrl-scroll)
                let zoom = Matrix4::from_scale(zoom);

                // Combined transform
                let transform = zoom * projection * rotation;

                // Finally once per frame you want to actually draw all the sections you've submitted
                // with `queue` calls.
                //
                // Note: Drawing in the case the text is unchanged from the previous frame (a common case)
                // is essentially free as the vertices are reused & gpu cache updating interaction
                // can be skipped.
                glyph_brush
                    .use_queue()
                    .transform(transform)
                    .draw(&mut encoder, &color_view)
                    .unwrap();

                encoder.flush(&mut device);
                gl_surface.swap_buffers(&gl_context).unwrap();
                device.cleanup();

                if let Some(rate) = loop_helper.report_rate() {
                    window.set_title(&format!("{title} - {rate:.0} FPS"));
                }

                loop_helper.loop_sleep();
                loop_helper.loop_start();
            }
            Event::LoopDestroyed => println!(),
            _ => (),
        }
    });
}
