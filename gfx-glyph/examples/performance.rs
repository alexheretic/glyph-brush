mod init;

use gfx::{
    format::{Depth, Srgba8},
    Device,
};
use gfx_glyph::{ab_glyph::*, *};
use glutin::surface::GlSurface;
use glutin_winit::GlWindow;
use init::init_example;
use std::{env, error::Error, time::Duration};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
};

const MAX_FONT_SIZE: f32 = 4000.0;
const TITLE: &str = "gfx_glyph rendering 30,000 glyphs - scroll to size, type to modify";

fn main() -> Result<(), Box<dyn Error>> {
    init_example("performance");
    if cfg!(debug_assertions) && env::var("yes_i_really_want_debug_mode").is_err() {
        eprintln!(
            "You should probably run an example called 'performance' in release mode, \
            don't you think?\n    \
            If you really want to see debug performance set env var `yes_i_really_want_debug_mode`"
        );
        return Ok(());
    }

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
    reporter: spin_sleep_util::RateReporter,
    text: String,
    font_size: f32,
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

        let dejavu = FontRef::try_from_slice(include_bytes!("../../fonts/DejaVuSans.ttf"))?;
        let glyph_brush = GlyphBrushBuilder::using_font(dejavu)
            .initial_cache_size((2048, 2048))
            .draw_cache_position_tolerance(1.0)
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
            reporter: spin_sleep_util::RateReporter::new(Duration::from_secs(1)),
            view_size,
            text: include_str!("loads-of-unicode.txt").into(),
            font_size: 25.0,
        })
    }

    fn window_event(&mut self, events: &ActiveEventLoop, event: WindowEvent) {
        let Self {
            text, font_size, ..
        } = self;

        match event {
            WindowEvent::CloseRequested => events.exit(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match logical_key {
                Key::Named(NamedKey::Escape) => events.exit(),
                Key::Named(NamedKey::Backspace) => {
                    text.pop();
                }
                key => {
                    if let Some(str) = key.to_text() {
                        text.push_str(str);
                    }
                }
            },
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, y),
                ..
            } => {
                // increase/decrease font size with mouse wheel
                if y > 0.0 {
                    *font_size += (*font_size / 4.0).max(2.0)
                } else {
                    *font_size *= 4.0 / 5.0
                };
                *font_size = font_size.clamp(1.0, MAX_FONT_SIZE);
            }
            WindowEvent::RedrawRequested => self.draw(),
            _ => (),
        }
    }

    fn draw(&mut self) {
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
            reporter,
            text,
            font_size,
            ..
        } = self;

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
        let scale = PxScale::from(*font_size * window.scale_factor() as f32);

        // The section is all the info needed for the glyph brush to render a 'section' of text.
        let section = Section::default()
            .add_text(
                Text::new(text)
                    .with_scale(scale)
                    .with_color([0.8, 0.8, 0.8, 1.0]),
            )
            .with_bounds((width, height))
            .with_layout(Layout::default().line_breaker(BuiltInLineBreaker::AnyCharLineBreaker));

        // Adds a section & layout to the queue for the next call to `use_queue().draw(..)`,
        // this can be called multiple times for different sections that want to use the
        // same font and gpu cache.
        // This step computes the glyph positions, this is cached to avoid unnecessary
        // recalculation.
        glyph_brush.queue(&section);

        // Finally once per frame you want to actually draw all the sections you've
        // submitted with `queue` calls.
        //
        // Note: Drawing in the case the text is unchanged from the previous frame
        // (a common case) is essentially free as the vertices are reused &  gpu cache
        // updating interaction can be skipped.
        glyph_brush.use_queue().draw(encoder, color_view).unwrap();

        encoder.flush(device);
        gl_surface.swap_buffers(gl_context).unwrap();
        device.cleanup();

        if let Some(rate) = reporter.increment_and_report() {
            window.set_title(&format!("{TITLE} - {rate:.0} FPS"));
        }
    }
}
