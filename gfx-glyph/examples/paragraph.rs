//! An example of paragraph rendering
//! Controls
//!
//! * Resize window to adjust layout
//! * Scroll to modify font size
//! * Type to add/remove text
//! * Ctrl-Scroll to zoom in/out using a transform, this is cheap but notice how ab_glyph can't
//!   render at full quality without the correct pixel information.
mod init;

use cgmath::{Matrix4, Rad, Transform, Vector3};
use gfx::{
    format::{Depth, Srgba8},
    Device,
};
use gfx_glyph::{ab_glyph, GlyphBrush};
use glutin::surface::GlSurface;
use glutin_winit::GlWindow;
use init::init_example;
use std::{
    error::Error,
    f32::consts::PI as PI32,
    io::{self, Write},
    time::Duration,
};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, Modifiers, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
};

const MAX_FONT_SIZE: f32 = 2000.0;
const TITLE: &str = "gfx_glyph example - scroll to size, type to modify, ctrl-scroll \
             to gpu zoom, ctrl-shift-scroll to gpu rotate";

fn main() -> Result<(), Box<dyn Error>> {
    init_example("paragraph");
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
    modifiers: Modifiers,
    text: String,
    font_size: f32,
    zoom: f32,
    angle: f32,
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

        let font =
            ab_glyph::FontArc::try_from_slice(include_bytes!("../../fonts/OpenSans-Light.ttf"))?;
        let glyph_brush = gfx_glyph::GlyphBrushBuilder::using_font(font)
            .initial_cache_size((1024, 1024))
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
            modifiers: <_>::default(),
            text: include_str!("lipsum.txt").into(),
            font_size: 18.0,
            zoom: 1.0,
            angle: 0.0,
        })
    }

    fn window_event(&mut self, events: &ActiveEventLoop, event: WindowEvent) {
        let Self {
            modifiers,
            text,
            font_size,
            zoom,
            angle,
            ..
        } = self;

        match event {
            WindowEvent::ModifiersChanged(new_mods) => *modifiers = new_mods,
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
                let ctrl = modifiers.state().control_key();
                let shift = modifiers.state().shift_key();
                if ctrl && shift {
                    if y > 0.0 {
                        *angle += 0.02 * PI32;
                    } else {
                        *angle -= 0.02 * PI32;
                    }
                    if (*angle % (PI32 * 2.0)).abs() < 0.01 {
                        *angle = 0.0;
                    }
                    print!("\r                            \r");
                    print!("transform-angle -> {:.2} * π", *angle / PI32);
                    let _ = io::stdout().flush();
                } else if ctrl && !shift {
                    let old_zoom = *zoom;
                    // increase/decrease zoom
                    if y > 0.0 {
                        *zoom += 0.1;
                    } else {
                        *zoom -= 0.1;
                    }
                    *zoom = zoom.clamp(0.1, 1.0);
                    if (*zoom - old_zoom).abs() > 1e-2 {
                        print!("\r                            \r");
                        print!("transform-zoom -> {zoom:.1}");
                        let _ = io::stdout().flush();
                    }
                } else {
                    // increase/decrease font size
                    let old_size = *font_size;
                    let mut size = *font_size;
                    if y > 0.0 {
                        size += (size / 4.0).max(2.0)
                    } else {
                        size *= 4.0 / 5.0
                    };
                    *font_size = size.clamp(1.0, MAX_FONT_SIZE);
                    if (*font_size - old_size).abs() > 1e-2 {
                        print!("\r                            \r");
                        print!("font-size -> {font_size:.1}");
                        let _ = io::stdout().flush();
                    }
                }
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
            interval,
            reporter,
            text,
            font_size,
            zoom,
            angle,
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
        let scale = *font_size * window.scale_factor() as f32;

        // The section is all the info needed for the glyph brush to render a 'section' of text.
        let section = gfx_glyph::Section::default()
            .add_text(
                Text::new(text)
                    .with_scale(scale)
                    .with_color([0.9, 0.3, 0.3, 1.0]),
            )
            .with_bounds((width / 3.15, height));

        // Adds a section & layout to the queue for the next call to `use_queue().draw(..)`,
        // this can be called multiple times for different sections that want to use the
        // same font and gpu cache.
        // This step computes the glyph positions, this is cached to avoid unnecessary
        // recalculation.
        glyph_brush.queue(&section);

        use gfx_glyph::*;
        glyph_brush.queue(
            Section::default()
                .add_text(
                    Text::new(text)
                        .with_scale(scale)
                        .with_color([0.3, 0.9, 0.3, 1.0]),
                )
                .with_screen_position((width / 2.0, height / 2.0))
                .with_bounds((width / 3.15, height))
                .with_layout(
                    Layout::default()
                        .h_align(HorizontalAlign::Center)
                        .v_align(VerticalAlign::Center),
                ),
        );

        glyph_brush.queue(
            Section::default()
                .add_text(
                    Text::new(text)
                        .with_scale(scale)
                        .with_color([0.3, 0.3, 0.9, 1.0]),
                )
                .with_screen_position((width, height))
                .with_bounds((width / 3.15, height))
                .with_layout(
                    Layout::default()
                        .h_align(HorizontalAlign::Right)
                        .v_align(VerticalAlign::Bottom),
                ),
        );

        // Rotation
        let offset = Matrix4::from_translation(Vector3::new(-width / 2.0, -height / 2.0, 0.0));
        let rotation =
            offset.inverse_transform().unwrap() * Matrix4::from_angle_z(Rad(*angle)) * offset;

        // Default projection
        let projection: Matrix4<f32> = gfx_glyph::default_transform(&*color_view).into();

        // Here an example transform is used as a cheap zoom out (controlled with ctrl-scroll)
        let zoom = Matrix4::from_scale(*zoom);

        // Combined transform
        let transform = zoom * projection * rotation;

        // Finally once per frame you want to actually draw all the sections you've submitted
        // with `queue` calls.
        //
        // Note: Drawing in the case the text is unchanged from the previous frame (a common case)
        // is essentially free as the vertices are reused & gpu cache updating interaction
        // can be skipped.
        glyph_brush
            .use_queue()
            .transform(transform)
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
}
