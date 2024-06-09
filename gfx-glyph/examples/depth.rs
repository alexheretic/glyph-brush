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
    init_example("depth");

    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let title = "gfx_glyph example - resize to see multi-text layout";
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

    let fonts = vec![
        FontArc::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?,
        FontArc::try_from_slice(include_bytes!("../../fonts/OpenSans-Italic.ttf"))?,
    ];
    let italic_font = FontId(1);

    let mut glyph_brush = GlyphBrushBuilder::using_fonts(fonts)
        .initial_cache_size((512, 512))
        .build(factory.clone());

    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

    let mut interval = spin_sleep_util::interval(Duration::from_secs(1) / 250);
    let mut reporter = spin_sleep_util::RateReporter::new(Duration::from_secs(1));
    let mut view_size = window.inner_size();

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
                    encoder.clear_depth(&depth_view, 1.0);

                    let (width, height) = (w_size.width as f32, w_size.height as f32);

                    // first section is queued, and therefore drawn, first with lower z
                    glyph_brush.queue(
                        Section::default()
                            .add_text(
                                Text::new("On top")
                                    .with_scale(95.0)
                                    .with_color([0.8, 0.8, 0.8, 1.0])
                                    .with_z(0.2)
                                    .with_font_id(italic_font),
                            )
                            .with_screen_position((width / 2.0, 100.0))
                            .with_bounds((width, height - 100.0))
                            .with_layout(Layout::default().h_align(HorizontalAlign::Center)),
                    );

                    // 2nd section is drawn last but with higher z,
                    // draws are subject to depth testing
                    glyph_brush.queue(
                        Section::default()
                            .add_text(
                                Text::new(
                                    &include_str!("lipsum.txt").replace("\n\n", "").repeat(10),
                                )
                                .with_scale(30.0)
                                .with_color([0.05, 0.05, 0.1, 1.0])
                                .with_z(1.0),
                            )
                            .with_bounds((width, height)),
                    );

                    glyph_brush
                        .use_queue()
                        // Enable depth testing with default less-equal drawing and update the depth buffer
                        .depth_target(&depth_view)
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
