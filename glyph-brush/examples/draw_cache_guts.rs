//! Example showing the guts of glyph_brush_draw_cache
mod opengl;

use approx::relative_eq;
use gl::types::*;
use glutin::{
    context::PossiblyCurrentContext,
    display::GetGlDisplay,
    prelude::{GlConfig, GlDisplay, NotCurrentGlContext},
    surface::{GlSurface, Surface, WindowSurface},
};
use glutin_winit::GlWindow;
use glyph_brush::{ab_glyph::*, *};
use opengl::{GlGlyphTexture, GlTextPipe, Res, Vertex};
use raw_window_handle::HasWindowHandle;
use std::{env, ffi::CString, mem, time::Duration};
use winit::{
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::Window,
};

/// `[left_top * 3, right_bottom * 2]`
type ImgVertex = [GLfloat; 5];

macro_rules! gl_assert_ok {
    () => {{
        let err = gl::GetError();
        assert_eq!(err, gl::NO_ERROR, "{}", opengl::gl_err_to_str(err));
    }};
}

fn main() -> Res<()> {
    env_logger::init();

    // disables vsync maybe
    if env::var_os("vblank_mode").is_none() {
        env::set_var("vblank_mode", "0");
    }

    EventLoop::new()?.run_app(&mut WinitApp::None)?;
    Ok(())
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
    glyph_brush: GlyphBrush<Vertex, Extra, FontRef<'static>>,
    texture: GlGlyphTexture,
    text_pipe: GlTextPipe,
    draw_cache_guts_pipe: GlDrawCacheGutsPipe,
    section: OwnedSection,
    mods: winit::event::Modifiers,
    font_size: f32,
    interval: spin_sleep_util::Interval,
    reporter: spin_sleep_util::RateReporter,
    max_image_dimension: u32,
    dimensions: PhysicalSize<u32>,
    fps: f64,
    title: String,
    gl_surface: Surface<WindowSurface>,
    gl_ctx: PossiblyCurrentContext,
    window: Window,
}

impl App {
    fn new(events: &ActiveEventLoop) -> Res<Self> {
        let (window, gl_config) = glutin_winit::DisplayBuilder::new()
            .with_window_attributes(Some(
                winit::window::Window::default_attributes()
                    .with_inner_size(winit::dpi::PhysicalSize::new(1024, 576))
                    .with_title("draw cache example"),
            ))
            .build(events, <_>::default(), |mut configs| {
                configs
                    .find(|c| c.srgb_capable() && c.num_samples() == 0)
                    .unwrap()
            })?;

        let window = window.unwrap(); // set in display builder
        let window_handle = window.window_handle()?;
        let gl_display = gl_config.display();

        let context_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_profile(glutin::context::GlProfile::Core)
            .with_context_api(glutin::context::ContextApi::OpenGl(Some(
                glutin::context::Version::new(3, 2),
            )))
            .build(Some(window_handle.as_raw()));

        let dimensions = window.inner_size();

        let (gl_surface, gl_ctx) = {
            let attrs = window.build_surface_attributes(<_>::default())?;
            let surface = unsafe { gl_display.create_window_surface(&gl_config, &attrs)? };
            let context = unsafe { gl_display.create_context(&gl_config, &context_attributes)? }
                .make_current(&surface)?;
            (surface, context)
        };

        gl::load_with(|symbol| gl_display.get_proc_address(&CString::new(symbol).unwrap()) as _);

        let sans = FontRef::try_from_slice(include_bytes!("../../fonts/OpenSans-Light.ttf"))?;
        let glyph_brush = GlyphBrushBuilder::using_font(sans)
            // .draw_cache_position_tolerance(2.0) // ignore subpixel differences totally
            // .draw_cache_scale_tolerance(1000.0) // ignore scale differences
            .build();

        let max_image_dimension = {
            let mut value = 0;
            unsafe { gl::GetIntegerv(gl::MAX_TEXTURE_SIZE, &mut value) };
            value as u32
        };

        let texture = GlGlyphTexture::new(glyph_brush.texture_dimensions());
        texture.clear();

        let text_pipe = GlTextPipe::new(dimensions)?;
        let draw_cache_guts_pipe =
            GlDrawCacheGutsPipe::new(dimensions, glyph_brush.texture_dimensions())?;

        let section = Section::default()
            .add_text(
                Text::new(
                    "* Type text\n\
                 * Scroll to set typed size (see window title)\n\
                 * ctrl r  Clear & reorder draw cache\n\
                 * ctrl shift r  Reset & resize draw cache\n\
                 * ctrl backspace  Delete all text\n\
                ",
                )
                .with_scale(25.0)
                .with_color([0.5, 0.5, 0.5, 1.0]),
            )
            .with_bounds((dimensions.width as f32 / 2.0, dimensions.height as f32))
            .with_layout(
                Layout::default()
                    .v_align(VerticalAlign::Center)
                    .line_breaker(BuiltInLineBreaker::AnyCharLineBreaker),
            )
            .with_screen_position((0.0, dimensions.height as f32 * 0.5))
            .to_owned();

        Ok(Self {
            window,
            gl_surface,
            gl_ctx,
            glyph_brush,
            texture,
            text_pipe,
            draw_cache_guts_pipe,
            section,
            mods: <_>::default(),
            font_size: 28.0,
            interval: spin_sleep_util::interval(Duration::from_secs(1) / 250),
            reporter: spin_sleep_util::RateReporter::new(Duration::from_secs(1)),
            max_image_dimension,
            dimensions,
            title: String::new(),
            fps: 0.0,
        })
    }

    fn window_event(&mut self, events: &ActiveEventLoop, event: WindowEvent) {
        let Self {
            glyph_brush,
            texture,
            draw_cache_guts_pipe,
            section,
            mods,
            font_size,
            dimensions,
            ..
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
                Key::Named(NamedKey::Backspace) if mods.state().control_key() => {
                    section.text.clear()
                }
                Key::Named(NamedKey::Backspace) if !section.text.is_empty() => {
                    let mut end_text = section.text.remove(section.text.len() - 1);
                    end_text.text.pop();
                    if !end_text.text.is_empty() {
                        section.text.push(end_text);
                    }
                }
                Key::Character(r)
                    if r == "R" && mods.state().control_key() && mods.state().shift_key() =>
                {
                    // reset draw cache to 16x16 and let it resize up to the minimum required
                    eprintln!("Resetting draw cache");
                    *texture = GlGlyphTexture::new((16, 16));
                    texture.clear();
                    glyph_brush.resize_texture(16, 16);
                    draw_cache_guts_pipe.update_geometry(*dimensions, (16, 16));
                }
                Key::Character(r) if r == "r" && mods.state().control_key() => {
                    // reset draw cache
                    eprintln!("Reordering draw cache - clear texture and reprocess current glyphs");
                    texture.clear();
                    let (tw, th) = glyph_brush.texture_dimensions();
                    glyph_brush.resize_texture(tw, th);
                }
                key => {
                    if let Some(str) = key.to_text() {
                        if section.text.is_empty() {
                            section.text.push(
                                OwnedText::default()
                                    .with_scale(*font_size)
                                    .with_color([0.4, 1.0, 0.4, 1.0]),
                            );
                        }
                        if let Some(t) = section
                            .text
                            .last_mut()
                            .filter(|t| relative_eq!(t.scale.y, font_size))
                        {
                            t.text.push_str(str);
                        } else {
                            section.text.push(
                                OwnedText::new(str)
                                    .with_scale(*font_size)
                                    .with_color([0.4, 1.0, 0.4, 1.0]),
                            );
                        }
                    }
                }
            },
            WindowEvent::ModifiersChanged(newmods) => *mods = newmods,
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, y),
                ..
            } => {
                // increase/decrease font size
                let mut size = *font_size;
                if y > 0.0 {
                    size += (size / 4.0).max(2.0)
                } else {
                    size *= 4.0 / 5.0
                };
                *font_size = (size.clamp(3.0, 2000.0) * 2.0).round() / 2.0;
            }
            WindowEvent::RedrawRequested => self.draw(),
            _ => (),
        }
    }

    fn draw(&mut self) {
        let Self {
            window,
            gl_surface,
            gl_ctx,
            glyph_brush,
            texture,
            text_pipe,
            draw_cache_guts_pipe,
            section,
            font_size,
            interval,
            reporter,
            max_image_dimension,
            dimensions,
            fps,
            title,
            ..
        } = self;

        // handle window size changes
        let window_size = window.inner_size();
        if *dimensions != window_size {
            *dimensions = window_size;
            window.resize_surface(gl_surface, gl_ctx);
            unsafe {
                gl::Viewport(0, 0, dimensions.width as _, dimensions.height as _);
            }

            section.bounds = (window_size.width as f32 * 0.5, window_size.height as _);
            section.screen_position.1 = window_size.height as f32 * 0.5;

            text_pipe.update_geometry(*dimensions);
            draw_cache_guts_pipe.update_geometry(*dimensions, glyph_brush.texture_dimensions());
        }

        glyph_brush.queue(&*section);

        let mut brush_action;
        loop {
            brush_action = glyph_brush.process_queued(
                |rect, tex_data| unsafe {
                    // Update part of gpu texture with new glyph alpha values
                    gl::BindTexture(gl::TEXTURE_2D, texture.name);
                    gl::TexSubImage2D(
                        gl::TEXTURE_2D,
                        0,
                        rect.min[0] as _,
                        rect.min[1] as _,
                        rect.width() as _,
                        rect.height() as _,
                        gl::RED,
                        gl::UNSIGNED_BYTE,
                        tex_data.as_ptr() as _,
                    );
                    gl_assert_ok!();
                },
                opengl::to_vertex,
            );

            match brush_action {
                Ok(_) => break,
                Err(BrushError::TextureTooSmall { suggested, .. }) => {
                    let (new_width, new_height) = if (suggested.0 > *max_image_dimension
                        || suggested.1 > *max_image_dimension)
                        && (glyph_brush.texture_dimensions().0 < *max_image_dimension
                            || glyph_brush.texture_dimensions().1 < *max_image_dimension)
                    {
                        (*max_image_dimension, *max_image_dimension)
                    } else {
                        suggested
                    };

                    // Recreate texture as a larger size to fit more
                    *texture = GlGlyphTexture::new((new_width, new_height));
                    texture.clear();
                    glyph_brush.resize_texture(new_width, new_height);
                    draw_cache_guts_pipe.update_geometry(*dimensions, (new_width, new_height));
                    eprintln!("Resizing texture -> {new_width}x{new_height} to fit glyphs");
                }
            }
        }
        match brush_action.unwrap() {
            BrushAction::Draw(vertices) => text_pipe.upload_vertices(&vertices),
            BrushAction::ReDraw => {}
        }

        unsafe {
            gl::ClearColor(0.02, 0.02, 0.02, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        text_pipe.draw();
        draw_cache_guts_pipe.draw();

        gl_surface.swap_buffers(gl_ctx).unwrap();

        if let Some(rate) = reporter.increment_and_report() {
            *fps = rate;
        }

        let (tw, th) = glyph_brush.texture_dimensions();
        let new_title = format!(
            "draw cache example - typing size {font_size}, cache size {tw}x{th}, {fps:.0} FPS"
        );
        if new_title != *title {
            *title = new_title;
            window.set_title(title);
        }

        interval.tick();
    }
}

pub struct GlDrawCacheGutsPipe {
    shaders: [GLuint; 2],
    program: GLuint,
    vao: GLuint,
    vbo: GLuint,
    transform_uniform: GLint,
}

impl GlDrawCacheGutsPipe {
    pub fn new(window_size: winit::dpi::PhysicalSize<u32>, texture_size: (u32, u32)) -> Res<Self> {
        let (w, h) = (window_size.width as f32, window_size.height as f32);

        let vs = opengl::compile_shader(include_str!("shader/img.vs"), gl::VERTEX_SHADER)?;
        let fs = opengl::compile_shader(include_str!("shader/img.fs"), gl::FRAGMENT_SHADER)?;
        let program = opengl::link_program(vs, fs)?;

        let mut vao = 0;
        let mut vbo = 0;

        let transform_uniform = unsafe {
            // Create Vertex Array Object
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            // Create a Vertex Buffer Object
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            // Use shader program
            gl::UseProgram(program);
            gl::BindFragDataLocation(program, 0, CString::new("out_color")?.as_ptr());

            // Specify the layout of the vertex data
            let uniform = gl::GetUniformLocation(program, CString::new("transform")?.as_ptr());
            if uniform < 0 {
                return Err(format!("GetUniformLocation(\"transform\") -> {uniform}").into());
            }
            let transform = opengl::ortho(0.0, w, 0.0, h, 1.0, -1.0);
            gl::UniformMatrix4fv(uniform, 1, 0, transform.as_ptr());

            let mut offset = 0;
            for (v_field, float_count) in &[("left_top", 3), ("right_bottom", 2)] {
                let attr = gl::GetAttribLocation(program, CString::new(*v_field)?.as_ptr());
                if attr < 0 {
                    return Err(format!("{v_field} GetAttribLocation -> {attr}").into());
                }
                gl::VertexAttribPointer(
                    attr as _,
                    *float_count,
                    gl::FLOAT,
                    gl::FALSE as _,
                    mem::size_of::<Vertex>() as _,
                    offset as _,
                );
                gl::EnableVertexAttribArray(attr as _);
                gl::VertexAttribDivisor(attr as _, 1);

                offset += float_count * 4;
            }

            let (tw, th) = (texture_size.0 as f32, texture_size.1 as f32);
            let left = (0.75 * w - 0.5 * tw).ceil();
            let right = left + tw;
            let top = (h * 0.5 - (th * 0.5)).floor();
            let bottom = top + th;
            let z = 0.0;

            let vertices = [[left, bottom, z, right, top]];

            gl::BufferData(
                gl::ARRAY_BUFFER,
                (mem::size_of::<ImgVertex>()) as GLsizeiptr,
                vertices.as_ptr() as _,
                gl::DYNAMIC_DRAW,
            );
            gl_assert_ok!();

            uniform
        };

        Ok(Self {
            shaders: [vs, fs],
            program,
            vao,
            vbo,
            transform_uniform,
        })
    }

    pub fn update_geometry(
        &self,
        window_size: winit::dpi::PhysicalSize<u32>,
        texture_size: (u32, u32),
    ) {
        let (w, h) = (window_size.width as f32, window_size.height as f32);
        let (tw, th) = (texture_size.0 as f32, texture_size.1 as f32);
        let left = (0.75 * w - 0.5 * tw).ceil();
        let right = left + tw;
        let top = (h * 0.5 - (th * 0.5)).floor();
        let bottom = top + th;
        let z = 0.0;

        let transform = opengl::ortho(0.0, w, 0.0, h, 1.0, -1.0);

        let vertices = [[left, bottom, z, right, top]];
        unsafe {
            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (mem::size_of::<ImgVertex>()) as GLsizeiptr,
                vertices.as_ptr() as _,
                gl::DYNAMIC_DRAW,
            );
            // update transform
            gl::UseProgram(self.program);
            gl::UniformMatrix4fv(self.transform_uniform, 1, 0, transform.as_ptr());
            gl_assert_ok!();
        }
    }

    pub fn draw(&self) {
        unsafe {
            // Enabled alpha blending
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            // Use srgb for consistency with other examples
            gl::Enable(gl::FRAMEBUFFER_SRGB);

            gl::UseProgram(self.program);
            gl::BindVertexArray(self.vao);
            gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, 1);
            gl_assert_ok!();
        }
    }
}

impl Drop for GlDrawCacheGutsPipe {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
            self.shaders.iter().for_each(|s| gl::DeleteShader(*s));
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
