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
    init_example("depth");
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

        let fonts = vec![
            FontArc::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?,
            FontArc::try_from_slice(include_bytes!("../../fonts/OpenSans-Italic.ttf"))?,
        ];

        let glyph_brush = GlyphBrushBuilder::using_fonts(fonts)
            .initial_cache_size((512, 512))
            .build(factory.clone());

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
        })
    }

    fn window_event(&mut self, events: &ActiveEventLoop, event: WindowEvent) {
        const ITALIC_FONT: FontId = FontId(1);

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
                encoder.clear_depth(depth_view, 1.0);

                let (width, height) = (w_size.width as f32, w_size.height as f32);

                // first section is queued, and therefore drawn, first with lower z
                glyph_brush.queue(
                    Section::default()
                        .add_text(
                            Text::new("On top")
                                .with_scale(95.0)
                                .with_color([0.8, 0.8, 0.8, 1.0])
                                .with_z(0.2)
                                .with_font_id(ITALIC_FONT),
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
                            Text::new(&include_str!("lipsum.txt").replace("\n\n", "").repeat(10))
                                .with_scale(30.0)
                                .with_color([0.05, 0.05, 0.1, 1.0])
                                .with_z(1.0),
                        )
                        .with_bounds((width, height)),
                );

                glyph_brush
                    .use_queue()
                    // Enable depth testing with default less-equal drawing and update the depth buffer
                    .depth_target(depth_view)
                    .draw(encoder, color_view)
                    .unwrap();

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
