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
    dpi::PhysicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
};

const TITLE: &str = "gfx_glyph example - resize to see multi-text layout";

fn main() -> Result<(), Box<dyn Error>> {
    init_example("varied");
    Ok(EventLoop::new()?.run_app(&mut WinitApp::None)?)
}

enum WinitApp {
    None,
    Resumed(App),
}

impl winit::application::ApplicationHandler for WinitApp {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        events.set_control_flow(ControlFlow::Poll);
        *self = Self::Resumed(App::new(events).unwrap());
    }

    fn window_event(
        &mut self,
        events: &ActiveEventLoop,
        _: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Self::Resumed(app) = self {
            app.window_event(events, event);
        }
    }

    fn about_to_wait(&mut self, _events: &ActiveEventLoop) {
        if let Self::Resumed(App { window, .. }) = self {
            window.request_redraw();
        };
    }
}

struct App {
    device: gfx_device_gl::Device,
    encoder: gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    color_view: gfx::handle::RenderTargetView<gfx_device_gl::Resources, Srgba8>,
    depth_view: gfx::handle::DepthStencilView<gfx_device_gl::Resources, Depth>,
    glyph_brush: GlyphBrush<gfx_device_gl::Resources, gfx_device_gl::Factory>,
    view_size: PhysicalSize<u32>,
    interval: spin_sleep_util::Interval,
    reporter: spin_sleep_util::RateReporter,
    sans_font: FontId,
    italic_font: FontId,
    serif_font: FontId,
    mono_font: FontId,
    gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
    gl_context: glutin::context::PossiblyCurrentContext,
    window: winit::window::Window,
}

impl App {
    fn new(events: &ActiveEventLoop) -> Result<Self, Box<dyn Error>> {
        let window_attrs = winit::window::Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(winit::dpi::PhysicalSize::new(1024, 576));

        let old_school_gfx_glutin_ext::Init {
            window,
            gl_surface,
            gl_context,
            device,
            mut factory,
            color_view,
            depth_view,
            ..
        } = old_school_gfx_glutin_ext::window_builder(events, window_attrs)
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

        let glyph_brush = builder.build(factory.clone());
        let encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
        let view_size = window.inner_size();

        Ok(Self {
            window,
            gl_surface,
            gl_context,
            device,
            encoder,
            color_view,
            depth_view,
            glyph_brush,
            interval: spin_sleep_util::interval(Duration::from_secs(1) / 250),
            reporter: spin_sleep_util::RateReporter::new(Duration::from_secs(1)),
            view_size,
            sans_font,
            italic_font,
            serif_font,
            mono_font,
        })
    }

    fn window_event(&mut self, events: &ActiveEventLoop, event: WindowEvent) {
        let Self {
            window,
            gl_surface,
            gl_context,
            device,
            encoder,
            color_view,
            depth_view,
            glyph_brush,
            view_size,
            interval,
            reporter,
            sans_font,
            italic_font,
            serif_font,
            mono_font,
        } = self;

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => events.exit(),
            WindowEvent::RedrawRequested => {
                // handle resizes
                let w_size = window.inner_size();
                if *view_size != w_size {
                    window.resize_surface(gl_surface, gl_context);
                    old_school_gfx_glutin_ext::resize_views(w_size, color_view, depth_view);
                    *view_size = w_size;
                }

                encoder.clear(color_view, [0.02, 0.02, 0.02, 1.0]);

                let (width, height, ..) = color_view.get_dimensions();
                let (width, height) = (f32::from(width), f32::from(height));

                glyph_brush.queue(Section {
                    screen_position: (0.0, height / 2.0),
                    bounds: (width * 0.49, height),
                    text: vec![
                        Text {
                            text: "Lorem ipsum dolor sit amet, ferri simul omittantur eam eu, ",
                            scale: PxScale::from(45.0),
                            font_id: *sans_font,
                            extra: Extra {
                                color: [0.9, 0.3, 0.3, 1.0],
                                z: 0.0,
                            },
                        },
                        Text {
                            text: "dolorem",
                            scale: PxScale::from(150.0),
                            font_id: *serif_font,
                            extra: Extra {
                                color: [0.3, 0.9, 0.3, 1.0],
                                z: 0.0,
                            },
                        },
                        Text {
                            text: " Iriure vocibus est te, natum delicata dignissim pri ea.",
                            scale: PxScale::from(25.0),
                            font_id: *sans_font,
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
                            font_id: *mono_font,
                            extra: Extra {
                                color: [0.3, 0.3, 0.9, 1.0],
                                z: 0.0,
                            },
                        },
                        Text {
                            text: " eruditi habemus qualisque eam an. No atqui apeirian phaedrum pri ex, hinc omnes sapientem. ",
                            scale: PxScale::from(30.0),
                            font_id: *italic_font,
                            extra: Extra {
                                color: [0.9, 0.3, 0.3, 1.0],
                                z: 0.0,
                            },
                        },
                        Text {
                            text: "Eu facilisi maluisset eos.",
                            scale: PxScale::from(55.0),
                            font_id: *sans_font,
                            extra: Extra {
                                color: [0.3, 0.9, 0.3, 1.0],
                                z: 0.0,
                            },
                        },
                        Text {
                            text: " ius nullam impetus. ",
                            scale: PxScale { x: 25.0, y: 45.0 },
                            font_id: *serif_font,
                            extra: Extra {
                                color: [0.9, 0.9, 0.3, 1.0],
                                z: 0.0,
                            },
                        },
                        Text {
                            text: "Ut quo elitr viderer constituam, pro omnesque forensibus at. Timeam scaevola mediocrem ut pri, te pro congue delicatissimi. Mei wisi nostro imperdiet ea, ridens salutatus per no, ut viris partem disputationi sit. Exerci eripuit referrentur vix at, sale mediocrem repudiare per te, modus admodum an eam. No vocent indoctum vis, ne quodsi patrioque vix. Vocent labores omittam et usu.",
                            scale: PxScale::from(22.0),
                            font_id: *italic_font,
                            extra: Extra {
                                color: [0.8, 0.3, 0.5, 1.0],
                                z: 0.0,
                            },
                        },
                    ],
                    layout: Layout::default().h_align(HorizontalAlign::Right).v_align(VerticalAlign::Center),
                });

                glyph_brush.use_queue().draw(encoder, color_view).unwrap();

                encoder.flush(device);
                gl_surface.swap_buffers(gl_context).unwrap();
                device.cleanup();

                if let Some(rate) = reporter.increment_and_report() {
                    window.set_title(&format!("{TITLE} - {rate:.0} FPS"));
                }
                interval.tick();
            }
            _ => (),
        }
    }
}
