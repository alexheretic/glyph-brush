//! An example of rendering multiple fonts, sizes & colours within a single layout
//! Controls
//!
//! * Resize window to adjust layout
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
    init_example("varied");

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

    let font_0 = FontArc::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?;

    let mut builder = GlyphBrushBuilder::using_font(font_0).initial_cache_size((512, 512));
    let sans_font = FontId::default();

    let italic_font = builder.add_font(FontArc::try_from_slice(include_bytes!(
        "../../fonts/OpenSans-Italic.ttf"
    ))?);
    let serif_font = builder.add_font(FontArc::try_from_slice(include_bytes!(
        "../../fonts/GaramondNo8-Reg.ttf"
    ))?);
    let mono_font = builder.add_font(FontArc::try_from_slice(include_bytes!(
        "../../fonts/DejaVuSansMono.ttf"
    ))?);

    let mut glyph_brush = builder.build(factory.clone());

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

                    let (width, height, ..) = color_view.get_dimensions();
                    let (width, height) = (f32::from(width), f32::from(height));

                    glyph_brush.queue(Section {
                        screen_position: (0.0, height / 2.0),
                        bounds: (width * 0.49, height),
                        text: vec![
                            Text {
                                text: "Lorem ipsum dolor sit amet, ferri simul omittantur eam eu, ",
                                scale: PxScale::from(45.0),
                                font_id: sans_font,
                                extra: Extra {
                                    color: [0.9, 0.3, 0.3, 1.0],
                                    z: 0.0,
                                },
                            },
                            Text {
                                text: "dolorem",
                                scale: PxScale::from(150.0),
                                font_id: serif_font,
                                extra: Extra {
                                    color: [0.3, 0.9, 0.3, 1.0],
                                    z: 0.0,
                                },
                            },
                            Text {
                                text: " Iriure vocibus est te, natum delicata dignissim pri ea.",
                                scale: PxScale::from(25.0),
                                font_id: sans_font,
                                extra: Extra {
                                    color: [0.3, 0.3, 0.9, 1.0],
                                    z: 0.0,
                                },
                            },
                        ],
                        layout: Layout::default().v_align(VerticalAlign::Center),
                    });

                    glyph_brush.queue(Section {
                        screen_position: (width, height / 2.0),
                        bounds: (width * 0.49, height),
                        text: vec![
                            Text {
                                text: "foo += bar;",
                                scale: PxScale::from(45.0),
                                font_id: mono_font,
                                extra: Extra {
                                    color: [0.3, 0.3, 0.9, 1.0],
                                    z: 0.0,
                                },
                            },
                            Text {
                                text: " eruditi habemus qualisque eam an. No atqui apeirian phaedrum pri ex, hinc omnes sapientem. ",
                                scale: PxScale::from(30.0),
                                font_id: italic_font,
                                extra: Extra {
                                    color: [0.9, 0.3, 0.3, 1.0],
                                    z: 0.0,
                                },
                            },
                            Text {
                                text: "Eu facilisi maluisset eos.",
                                scale: PxScale::from(55.0),
                                font_id: sans_font,
                                extra: Extra {
                                    color: [0.3, 0.9, 0.3, 1.0],
                                    z: 0.0,
                                },
                            },
                            Text {
                                text: " ius nullam impetus. ",
                                scale: PxScale { x: 25.0, y: 45.0 },
                                font_id: serif_font,
                                extra: Extra {
                                    color: [0.9, 0.9, 0.3, 1.0],
                                    z: 0.0,
                                },
                            },
                            Text {
                                text: "Ut quo elitr viderer constituam, pro omnesque forensibus at. Timeam scaevola mediocrem ut pri, te pro congue delicatissimi. Mei wisi nostro imperdiet ea, ridens salutatus per no, ut viris partem disputationi sit. Exerci eripuit referrentur vix at, sale mediocrem repudiare per te, modus admodum an eam. No vocent indoctum vis, ne quodsi patrioque vix. Vocent labores omittam et usu.",
                                scale: PxScale::from(22.0),
                                font_id: italic_font,
                                extra: Extra {
                                    color: [0.8, 0.3, 0.5, 1.0],
                                    z: 0.0,
                                },
                            },
                        ],
                        layout: Layout::default().h_align(HorizontalAlign::Right).v_align(VerticalAlign::Center),
                    });

                    glyph_brush.use_queue().draw(&mut encoder, &color_view).unwrap();

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
