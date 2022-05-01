//! Example of glyph_brush usage with raw OpenGL.
//!
//! Uses instanced rendering with 1 vertex per glyph referencing a 1 byte per pixel texture.
//!
//! Controls
//! * Scroll to size text.
//! * Type to modify text.
//! * Resize window.
//!
//! The main operating structure is as follows:
//! * Load a font
//! * Initialize the brush
//! * Set up the glyph cache texture
//! Per frame:
//! * Queue up Sections of text, containing per-glyph colors and layout information
//! * Process the text into vertices, increasing glyph cache size if necessary
//! * Upload the vertices to the GPU if they've changed, and draw to the screen

use gl::types::*;
use glutin::{
    event::{ElementState, Event, KeyboardInput, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    Api, GlProfile, GlRequest,
};
use glyph_brush::{ab_glyph::*, *};
use std::{
    env,
    ffi::CString,
    io::{self, Write},
    mem, ptr, str,
};

pub type Res<T> = Result<T, Box<dyn std::error::Error>>;
/// `[left_top * 3, right_bottom * 2, tex_left_top * 2, tex_right_bottom * 2, color * 4]`
pub type Vertex = [GLfloat; 13];

macro_rules! gl_assert_ok {
    () => {{
        let err = gl::GetError();
        assert_eq!(err, gl::NO_ERROR, "{}", gl_err_to_str(err));
    }};
}

#[allow(unused)] // it _is_ used
fn main() -> Res<()> {
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

    let events = glutin::event_loop::EventLoop::new();
    let title = "glyph_brush opengl example - scroll to size, type to modify";

    let window_ctx = glutin::ContextBuilder::new()
        .with_gl_profile(GlProfile::Core)
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 2)))
        .with_srgb(true)
        .build_windowed(
            glutin::window::WindowBuilder::new()
                .with_inner_size(glutin::dpi::PhysicalSize::new(1024, 576))
                .with_title(title),
            &events,
        )?;
    let window_ctx = unsafe { window_ctx.make_current().unwrap() };

    let dejavu = FontRef::try_from_slice(include_bytes!("../../fonts/OpenSans-Light.ttf"))?;
    let mut glyph_brush = GlyphBrushBuilder::using_font(dejavu).build();

    // Load the OpenGL function pointers
    gl::load_with(|symbol| window_ctx.get_proc_address(symbol) as _);

    let max_image_dimension = {
        let mut value = 0;
        unsafe { gl::GetIntegerv(gl::MAX_TEXTURE_SIZE, &mut value) };
        value as u32
    };

    let mut texture = GlGlyphTexture::new(glyph_brush.texture_dimensions());

    let mut dimensions = window_ctx.window().inner_size();

    let mut text_pipe = GlTextPipe::new(dimensions)?;

    let mut text: String = include_str!("text/lipsum.txt").into();
    let mut font_size: f32 = 18.0;

    let mut loop_helper = spin_sleep::LoopHelper::builder().build_with_target_rate(250.0);
    let mut vertex_count = 0;
    let mut vertex_max = vertex_count;

    events.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(keypress),
                            ..
                        },
                    ..
                } => match keypress {
                    VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
                    VirtualKeyCode::Back => {
                        text.pop();
                    }
                    _ => (),
                },
                WindowEvent::ReceivedCharacter(c) => {
                    if c != '\u{7f}' && c != '\u{8}' {
                        text.push(c);
                    }
                }
                WindowEvent::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(_, y),
                    ..
                } => {
                    // increase/decrease font size
                    let old_size = font_size;
                    let mut size = font_size;
                    if y > 0.0 {
                        size += (size / 4.0).max(2.0)
                    } else {
                        size *= 4.0 / 5.0
                    };
                    font_size = size.max(1.0).min(2000.0);
                    if (font_size - old_size).abs() > 1e-2 {
                        eprint!("\r                            \r");
                        eprint!("font-size -> {:.1}", font_size);
                        let _ = io::stderr().flush();
                    }
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                // handle window size changes
                let window_size = window_ctx.window().inner_size();
                if dimensions != window_size {
                    dimensions = window_size;
                    window_ctx.resize(window_size); // update context with new size
                    unsafe {
                        gl::Viewport(0, 0, dimensions.width as _, dimensions.height as _);
                    }
                    text_pipe.update_geometry(window_size);
                }

                let width = dimensions.width as f32;
                let height = dimensions.height as _;
                let scale = (font_size * window_ctx.window().scale_factor() as f32).round();
                let base_text = Text::new(&text).with_scale(scale);

                // Queue up all sections of text to be drawn
                glyph_brush.queue(
                    Section::default()
                        .add_text(base_text.with_color([0.9, 0.3, 0.3, 1.0]))
                        .with_bounds((width / 3.15, height)),
                );

                glyph_brush.queue(
                    Section::default()
                        .add_text(base_text.with_color([0.3, 0.9, 0.3, 1.0]))
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
                        .add_text(base_text.with_color([0.3, 0.3, 0.9, 1.0]))
                        .with_screen_position((width, height))
                        .with_bounds((width / 3.15, height))
                        .with_layout(
                            Layout::default()
                                .h_align(HorizontalAlign::Right)
                                .v_align(VerticalAlign::Bottom),
                        ),
                );

                // Tell glyph_brush to process the queued text
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
                        to_vertex,
                    );

                    // If the cache texture is too small to fit all the glyphs, resize and try again
                    match brush_action {
                        Ok(_) => break,
                        Err(BrushError::TextureTooSmall { suggested, .. }) => {
                            let (new_width, new_height) = if (suggested.0 > max_image_dimension
                                || suggested.1 > max_image_dimension)
                                && (glyph_brush.texture_dimensions().0 < max_image_dimension
                                    || glyph_brush.texture_dimensions().1 < max_image_dimension)
                            {
                                (max_image_dimension, max_image_dimension)
                            } else {
                                suggested
                            };
                            eprint!("\r                            \r");
                            eprintln!("Resizing glyph texture -> {}x{}", new_width, new_height);

                            // Recreate texture as a larger size to fit more
                            texture = GlGlyphTexture::new((new_width, new_height));

                            glyph_brush.resize_texture(new_width, new_height);
                        }
                    }
                }
                // If the text has changed from what was last drawn, upload the new vertices to GPU
                match brush_action.unwrap() {
                    BrushAction::Draw(vertices) => text_pipe.upload_vertices(&vertices),
                    BrushAction::ReDraw => {}
                }

                // Draw the text to the screen
                unsafe {
                    gl::Clear(gl::COLOR_BUFFER_BIT);
                }
                text_pipe.draw();

                window_ctx.swap_buffers().unwrap();

                if let Some(rate) = loop_helper.report_rate() {
                    window_ctx
                        .window()
                        .set_title(&format!("{} {:.0} FPS", title, rate));
                }
                loop_helper.loop_sleep();
                loop_helper.loop_start();
            }
            _ => (),
        }
    });
}

pub fn gl_err_to_str(err: u32) -> &'static str {
    match err {
        gl::INVALID_ENUM => "INVALID_ENUM",
        gl::INVALID_VALUE => "INVALID_VALUE",
        gl::INVALID_OPERATION => "INVALID_OPERATION",
        gl::INVALID_FRAMEBUFFER_OPERATION => "INVALID_FRAMEBUFFER_OPERATION",
        gl::OUT_OF_MEMORY => "OUT_OF_MEMORY",
        gl::STACK_UNDERFLOW => "STACK_UNDERFLOW",
        gl::STACK_OVERFLOW => "STACK_OVERFLOW",
        _ => "Unknown error",
    }
}

pub fn compile_shader(src: &str, ty: GLenum) -> Res<GLuint> {
    let shader;
    unsafe {
        shader = gl::CreateShader(ty);
        // Attempt to compile the shader
        let c_str = CString::new(src.as_bytes())?;
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = GLint::from(gl::FALSE);
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != GLint::from(gl::TRUE) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = vec![0; len as usize - 1]; // -1 to skip the trailing null character
            gl::GetShaderInfoLog(
                shader,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            return Err(str::from_utf8(&buf)?.into());
        }
    }
    Ok(shader)
}

pub fn link_program(vs: GLuint, fs: GLuint) -> Res<GLuint> {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);
        // Get the link status
        let mut status = GLint::from(gl::FALSE);
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != GLint::from(gl::TRUE) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = vec![0; len as usize - 1]; // -1 to skip the trailing null character
            gl::GetProgramInfoLog(
                program,
                len,
                ptr::null_mut(),
                buf.as_mut_ptr() as *mut GLchar,
            );
            return Err(str::from_utf8(&buf)?.into());
        }
        Ok(program)
    }
}

#[inline]
pub fn to_vertex(
    glyph_brush::GlyphVertex {
        mut tex_coords,
        pixel_coords,
        bounds,
        extra,
    }: glyph_brush::GlyphVertex,
) -> Vertex {
    let gl_bounds = bounds;

    let mut gl_rect = Rect {
        min: point(pixel_coords.min.x as f32, pixel_coords.min.y as f32),
        max: point(pixel_coords.max.x as f32, pixel_coords.max.y as f32),
    };

    // handle overlapping bounds, modify uv_rect to preserve texture aspect
    if gl_rect.max.x > gl_bounds.max.x {
        let old_width = gl_rect.width();
        gl_rect.max.x = gl_bounds.max.x;
        tex_coords.max.x = tex_coords.min.x + tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.min.x < gl_bounds.min.x {
        let old_width = gl_rect.width();
        gl_rect.min.x = gl_bounds.min.x;
        tex_coords.min.x = tex_coords.max.x - tex_coords.width() * gl_rect.width() / old_width;
    }
    if gl_rect.max.y > gl_bounds.max.y {
        let old_height = gl_rect.height();
        gl_rect.max.y = gl_bounds.max.y;
        tex_coords.max.y = tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
    }
    if gl_rect.min.y < gl_bounds.min.y {
        let old_height = gl_rect.height();
        gl_rect.min.y = gl_bounds.min.y;
        tex_coords.min.y = tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
    }

    [
        gl_rect.min.x,
        gl_rect.max.y,
        extra.z,
        gl_rect.max.x,
        gl_rect.min.y,
        tex_coords.min.x,
        tex_coords.max.y,
        tex_coords.max.x,
        tex_coords.min.y,
        extra.color[0],
        extra.color[1],
        extra.color[2],
        extra.color[3],
    ]
}

#[rustfmt::skip]
pub fn ortho(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> [f32; 16] {
    let tx = -(right + left) / (right - left);
    let ty = -(top + bottom) / (top - bottom);
    let tz = -(far + near) / (far - near);
    [
        2.0 / (right - left), 0.0, 0.0, 0.0,
        0.0, 2.0 / (top - bottom), 0.0, 0.0,
        0.0, 0.0, -2.0 / (far - near), 0.0,
        tx, ty, tz, 1.0,
    ]
}

/// The texture used to cache drawn glyphs
pub struct GlGlyphTexture {
    pub name: GLuint,
}

impl GlGlyphTexture {
    pub fn new((width, height): (u32, u32)) -> Self {
        let mut name = 0;
        unsafe {
            // Create a texture for the glyphs
            // The texture holds 1 byte per pixel as alpha data
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::GenTextures(1, &mut name);
            gl::BindTexture(gl::TEXTURE_2D, name);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RED as _,
                width as _,
                height as _,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                ptr::null(),
            );
            gl_assert_ok!();

            Self { name }
        }
    }

    pub fn clear(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.name);
            gl::ClearTexImage(
                self.name,
                0,
                gl::RED,
                gl::UNSIGNED_BYTE,
                [12_u8].as_ptr() as _,
            );
            gl_assert_ok!();
        }
    }
}

impl Drop for GlGlyphTexture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.name);
        }
    }
}

pub struct GlTextPipe {
    shaders: [GLuint; 2],
    program: GLuint,
    vao: GLuint,
    vbo: GLuint,
    transform_uniform: GLint,
    vertex_count: usize,
    vertex_buffer_len: usize,
}

impl GlTextPipe {
    pub fn new(window_size: glutin::dpi::PhysicalSize<u32>) -> Res<Self> {
        let (w, h) = (window_size.width as f32, window_size.height as f32);

        let vs = compile_shader(include_str!("shader/text.vs"), gl::VERTEX_SHADER)?;
        let fs = compile_shader(include_str!("shader/text.fs"), gl::FRAGMENT_SHADER)?;
        let program = link_program(vs, fs)?;

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
                return Err(format!("GetUniformLocation(\"transform\") -> {}", uniform).into());
            }
            let transform = ortho(0.0, w, 0.0, h, 1.0, -1.0);
            gl::UniformMatrix4fv(uniform, 1, 0, transform.as_ptr());

            let mut offset = 0;
            for (v_field, float_count) in &[
                ("left_top", 3),
                ("right_bottom", 2),
                ("tex_left_top", 2),
                ("tex_right_bottom", 2),
                ("color", 4),
            ] {
                let attr = gl::GetAttribLocation(program, CString::new(*v_field)?.as_ptr());
                if attr < 0 {
                    return Err(format!("{} GetAttribLocation -> {}", v_field, attr).into());
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
                gl::VertexAttribDivisor(attr as _, 1); // Important for use with DrawArraysInstanced

                offset += float_count * 4;
            }

            // Enabled alpha blending
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            // Use srgb for consistency with other examples
            gl::Enable(gl::FRAMEBUFFER_SRGB);
            gl::ClearColor(0.02, 0.02, 0.02, 1.0);
            gl_assert_ok!();

            uniform
        };

        Ok(Self {
            shaders: [vs, fs],
            program,
            vao,
            vbo,
            transform_uniform,
            vertex_count: 0,
            vertex_buffer_len: 0,
        })
    }

    pub fn upload_vertices(&mut self, vertices: &[Vertex]) {
        // Draw new vertices
        self.vertex_count = vertices.len();

        unsafe {
            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
            if self.vertex_buffer_len < self.vertex_count {
                gl::BufferData(
                    gl::ARRAY_BUFFER,
                    (self.vertex_count * mem::size_of::<Vertex>()) as GLsizeiptr,
                    vertices.as_ptr() as _,
                    gl::DYNAMIC_DRAW,
                );
                self.vertex_buffer_len = self.vertex_count;
            } else {
                gl::BufferSubData(
                    gl::ARRAY_BUFFER,
                    0,
                    (self.vertex_count * mem::size_of::<Vertex>()) as GLsizeiptr,
                    vertices.as_ptr() as _,
                );
            }
            gl_assert_ok!();
        }
    }

    pub fn update_geometry(&self, window_size: glutin::dpi::PhysicalSize<u32>) {
        let (w, h) = (window_size.width as f32, window_size.height as f32);
        let transform = ortho(0.0, w, 0.0, h, 1.0, -1.0);

        unsafe {
            gl::UseProgram(self.program);
            gl::UniformMatrix4fv(self.transform_uniform, 1, 0, transform.as_ptr());
            gl_assert_ok!();
        }
    }

    pub fn draw(&self) {
        unsafe {
            gl::UseProgram(self.program);
            gl::BindVertexArray(self.vao);
            // If implementing this yourself, make sure to set VertexAttribDivisor as well
            gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, self.vertex_count as _);
            gl_assert_ok!();
        }
    }
}

impl Drop for GlTextPipe {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.program);
            self.shaders.iter().for_each(|s| gl::DeleteShader(*s));
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}
