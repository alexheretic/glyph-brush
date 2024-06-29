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
    dpi::PhysicalSize,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
};

const COLOR: [f32; 4] = [0.8, 0.8, 0.8, 1.0];
const TITLE: &str = "gfx_glyph example";

fn main() -> Result<(), Box<dyn Error>> {
    init_example("pre_positioned");
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
    glyph_brush: GlyphBrush<gfx_device_gl::Resources, gfx_device_gl::Factory, FontRef<'static>>,
    view_size: PhysicalSize<u32>,
    interval: spin_sleep_util::Interval,
    reporter: spin_sleep_util::RateReporter,
    glyphs: Vec<SectionGlyph>,
    width: f32,
    height: f32,
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

        let font = FontRef::try_from_slice(include_bytes!("../../fonts/OpenSans-Light.ttf"))?;
        let glyph_brush = gfx_glyph::GlyphBrushBuilder::using_font(font.clone())
            .initial_cache_size((1024, 1024))
            .build(factory.clone());

        let encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

        let view_size = window.inner_size();
        let (width, height, ..) = color_view.get_dimensions();
        let (width, height) = (f32::from(width), f32::from(height));

        let glyphs = gfx_glyph::Layout::default().calculate_glyphs(
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
            glyphs,
            width,
            height,
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
            glyphs,
            width,
            height,
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

                glyph_brush.queue_pre_positioned(
                    glyphs.clone(),
                    vec![Extra {
                        color: COLOR,
                        z: 0.0,
                    }],
                    Rect {
                        min: point(0.0, 0.0),
                        max: point(*width, *height),
                    },
                );

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
