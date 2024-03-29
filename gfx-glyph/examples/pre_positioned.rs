//! `queue_pre_positioned` example
mod init;

use gfx::{
    format::{Depth, Srgba8},
    Device,
};
use gfx_glyph::{ab_glyph::*, *};
use glutin::surface::GlSurface;
use glutin_winit::GlWindow;
use init::init_example;
use std::{error::Error, time::Duration};
use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::ControlFlow,
    keyboard::{Key, NamedKey},
};

fn main() -> Result<(), Box<dyn Error>> {
    init_example("pre_positioned");

    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
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

    let mut interval = spin_sleep_util::interval(Duration::from_secs(1) / 250);
    let mut reporter = spin_sleep_util::RateReporter::new(Duration::from_secs(1));
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

    event_loop.run(move |event, elwt| {
        match event {
            Event::AboutToWait => window.request_redraw(),
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            logical_key: Key::Named(NamedKey::Escape),
                            ..
                        },
                    ..
                } => elwt.exit(),
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
