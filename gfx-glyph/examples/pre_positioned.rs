//! `queue_pre_positioned` example
mod init;

use gfx::{
    format::{Depth, Srgba8},
    Device,
};
use gfx_glyph::{ab_glyph::*, *};
use glutin::surface::GlSurface;
use init::{init_example, WindowExt};
use std::error::Error;
use winit::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
};

fn main() -> Result<(), Box<dyn Error>> {
    init_example("pre_positioned");

    let event_loop = winit::event_loop::EventLoop::new();
    let title = "gfx_glyph example";
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

    let font: &[u8] = include_bytes!("../../fonts/OpenSans-Light.ttf");
    let font = FontRef::try_from_slice(font)?;
    let mut glyph_brush = gfx_glyph::GlyphBrushBuilder::using_font(font.clone())
        .initial_cache_size((1024, 1024))
        .build(factory.clone());

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut loop_helper = spin_sleep::LoopHelper::builder().build_with_target_rate(250.0);
    let mut view_size = window.inner_size();

    let (width, height, ..) = color_view.get_dimensions();
    let (width, height) = (f32::from(width), f32::from(height));
    let color = [0.8, 0.8, 0.8, 1.0];

    let glyphs: Vec<_> = gfx_glyph::Layout::default().calculate_glyphs(
        &[font],
        &gfx_glyph::SectionGeometry {
            screen_position: (0.0, 0.0),
            bounds: (width, height),
        },
        &[gfx_glyph::SectionText {
            text: include_str!("lipsum.txt"),
            scale: PxScale::from(30.0),
            font_id: FontId(0),
        }],
    );

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

                glyph_brush.queue_pre_positioned(
                    glyphs.clone(),
                    vec![Extra { color, z: 0.0 }],
                    Rect {
                        min: point(0.0, 0.0),
                        max: point(width, height),
                    },
                );

                glyph_brush
                    .use_queue()
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
            _ => (),
        }
    });
}
