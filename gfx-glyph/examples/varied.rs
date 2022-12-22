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
use init::{init_example, WindowExt};
use std::error::Error;
use winit::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
};

fn main() -> Result<(), Box<dyn Error>> {
    init_example("varied");

    let event_loop = winit::event_loop::EventLoop::new();
    let title = "gfx_glyph example - resize to see multi-text layout";
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

    let mut loop_helper = spin_sleep::LoopHelper::builder().build_with_target_rate(250.0);
    let mut view_size = window.inner_size();

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
