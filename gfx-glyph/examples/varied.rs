//! An example of rendering multiple fonts, sizes & colours within a single layout
//! Controls
//!
//! * Resize window to adjust layout
use gfx::{
    format::{Depth, Srgba8},
    Device,
};
use gfx_glyph::{ab_glyph::*, *};
use glutin::{
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
};
use old_school_gfx_glutin_ext::*;
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
    let event_loop = glutin::event_loop::EventLoop::new();
    let title = "gfx_glyph example - resize to see multi-text layout";
    let window_builder = glutin::window::WindowBuilder::new()
        .with_title(title)
        .with_inner_size(glutin::dpi::PhysicalSize::new(1024, 576));

    let (window_ctx, mut device, mut factory, mut main_color, mut main_depth) =
        glutin::ContextBuilder::new()
            .with_gfx_color_depth::<Srgba8, Depth>()
            .build_windowed(window_builder, &event_loop)?
            .init_gfx::<Srgba8, Depth>();

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

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::MainEventsCleared => window_ctx.window().request_redraw(),
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
                WindowEvent::Resized(size) => {
                    window_ctx.resize(size);
                    window_ctx.update_gfx(&mut main_color, &mut main_depth);
                }
                _ => (),
            },
            Event::RedrawRequested(_) => {
                encoder.clear(&main_color, [0.02, 0.02, 0.02, 1.0]);

                let (width, height, ..) = main_color.get_dimensions();
                let (width, height) = (f32::from(width), f32::from(height));

                glyph_brush.queue(VariedSection {
                    screen_position: (0.0, height / 2.0),
                    bounds: (width * 0.49, height),
                    text: vec![
                        VariedSectionText {
                            text: "Lorem ipsum dolor sit amet, ferri simul omittantur eam eu, ",
                            scale: PxScale::from(45.0),
                            font_id: sans_font,
                            color: [0.9, 0.3, 0.3, 1.0]
                        },
                        VariedSectionText {
                            text: "dolorem",
                            scale: PxScale::from(150.0),
                            font_id: serif_font,
                            color: [0.3, 0.9, 0.3, 1.0]
                        },
                        VariedSectionText {
                            text: " Iriure vocibus est te, natum delicata dignissim pri ea.",
                            scale: PxScale::from(25.0),
                            font_id: sans_font,
                            color: [0.3, 0.3, 0.9, 1.0]
                        },
                    ],
                    layout: Layout::default().v_align(VerticalAlign::Center),
                    ..<_>::default()
                });

                glyph_brush.queue(VariedSection {
                    screen_position: (width, height / 2.0),
                    bounds: (width * 0.49, height),
                    text: vec![
                        VariedSectionText {
                            text: "foo += bar;",
                            scale: PxScale::from(45.0),
                            font_id: mono_font,
                            color: [0.3, 0.3, 0.9, 1.0]
                        },
                        VariedSectionText {
                            text: " eruditi habemus qualisque eam an. No atqui apeirian phaedrum pri ex, hinc omnes sapientem. ",
                            scale: PxScale::from(30.0),
                            font_id: italic_font,
                            color: [0.9, 0.3, 0.3, 1.0]
                        },
                        VariedSectionText {
                            text: "Eu facilisi maluisset eos.",
                            scale: PxScale::from(55.0),
                            font_id: sans_font,
                            color: [0.3, 0.9, 0.3, 1.0]
                        },
                        VariedSectionText {
                            text: " ius nullam impetus. ",
                            scale: PxScale { x: 25.0, y: 45.0 },
                            font_id: serif_font,
                            color: [0.9, 0.9, 0.3, 1.0]
                        },
                        VariedSectionText {
                            text: "Ut quo elitr viderer constituam, pro omnesque forensibus at. Timeam scaevola mediocrem ut pri, te pro congue delicatissimi. Mei wisi nostro imperdiet ea, ridens salutatus per no, ut viris partem disputationi sit. Exerci eripuit referrentur vix at, sale mediocrem repudiare per te, modus admodum an eam. No vocent indoctum vis, ne quodsi patrioque vix. Vocent labores omittam et usu.",
                            scale: PxScale::from(22.0),
                            font_id: italic_font,
                            color: [0.8, 0.3, 0.5, 1.0]
                        },
                    ],
                    layout: Layout::default().h_align(HorizontalAlign::Right).v_align(VerticalAlign::Center),
                    ..<_>::default()
                });

                glyph_brush.use_queue().draw(&mut encoder, &main_color).unwrap();

                encoder.flush(&mut device);
                window_ctx.swap_buffers().unwrap();
                device.cleanup();

                if let Some(rate) = loop_helper.report_rate() {
                    window_ctx.window().set_title(&format!("{} - {:.0} FPS", title, rate));
                }
                loop_helper.loop_sleep();
                loop_helper.loop_start();
            }
            _ => (),
        }
    });
}
