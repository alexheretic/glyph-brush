// TODO finish

extern crate gl;
extern crate glutin;
extern crate glyph_brush;
extern crate spin_sleep;
extern crate env_logger;

use gl::types::*;
use glutin::GlContext;
use glyph_brush::{rusttype::*, *};
use std::{ffi::CString, env, ptr, str};

type Res<T> = Result<T, Box<std::error::Error>>;
type Vertex = [GLfloat; 13];

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

    let mut events = glutin::EventsLoop::new();

    let mut dimensions = glutin::dpi::PhysicalSize::new(1024.0, 576.0);
    let window = glutin::GlWindow::new(
        glutin::WindowBuilder::new().with_dimensions(
            dimensions.to_logical(events.get_primary_monitor().get_hidpi_factor()),
        ),
        glutin::ContextBuilder::new(),
        &events,
    )?;
    unsafe { window.make_current()? };

    let text: String = include_str!("text/lipsum.txt").into();
    let scale = Scale::uniform(18.0);

    let dejavu: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
    let mut glyph_brush = GlyphBrushBuilder::using_font_bytes(dejavu).build();

    // Load the OpenGL function pointers
    gl::load_with(|symbol| window.get_proc_address(symbol) as _);

    // Create GLSL shaders
    let vs = compile_shader(include_str!("shader/vert.glsl"), gl::VERTEX_SHADER)?;
    let fs = compile_shader(include_str!("shader/frag.glsl"), gl::FRAGMENT_SHADER)?;
    let program = link_program(vs, fs)?;

    let mut vao = 0;
    let mut vbo = 0;

    unsafe {
        // Create Vertex Array Object
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // Create a Vertex Buffer Object and copy the vertex data to it
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        // gl::BufferData(
        //     gl::GL_ELEMENT_ARRAY_BUFFER,
        //     (VERTEX_DATA.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
        //     mem::transmute(&VERTEX_DATA[0]),
        //     gl::STATIC_DRAW,
        // );

        // Use shader program
        gl::UseProgram(program);
        gl::BindFragDataLocation(program, 0, CString::new("out_color")?.as_ptr());

        // Specify the layout of the vertex data
        for (v_field, float_count) in &[
            ("left_top", 3),
            ("right_bottom", 2),
            ("tex_left_top", 2),
            ("tex_right_bottom", 2),
            ("color", 4),
        ] {
            let pos_attr = gl::GetAttribLocation(program, CString::new(*v_field)?.as_ptr());
            gl::EnableVertexAttribArray(pos_attr as GLuint);
            gl::VertexAttribPointer(
                pos_attr as _,
                *float_count,
                gl::FLOAT,
                gl::FALSE as _,
                0,
                ptr::null(),
            );
        }
    }

    let mut loop_helper = spin_sleep::LoopHelper::builder().build_with_target_rate(250.0);
    let mut running = true;
    let mut vertex_count = 0;

    while running {
        loop_helper.loop_start();

        events.poll_events(|event| {
            use glutin::*;
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    }
                    | WindowEvent::CloseRequested => running = false,
                    WindowEvent::Resized(size) => {
                        dimensions = size.to_physical(window.get_hidpi_factor());
                        window.resize(dimensions);
                    }
                    _ => {}
                }
            }
        });

        let width = dimensions.width as _;
        let height = dimensions.height as _;

        glyph_brush.queue(Section {
            text: &text,
            scale,
            screen_position: (0.0, 0.0),
            bounds: (width / 3.15, height),
            color: [0.9, 0.3, 0.3, 1.0],
            ..Section::default()
        });

        glyph_brush.queue(Section {
            text: &text,
            scale,
            screen_position: (width / 2.0, height / 2.0),
            bounds: (width / 3.15, height),
            color: [0.3, 0.9, 0.3, 1.0],
            layout: Layout::default()
                .h_align(HorizontalAlign::Center)
                .v_align(VerticalAlign::Center),
            ..Section::default()
        });

        glyph_brush.queue(Section {
            text: &text,
            scale,
            screen_position: (width, height),
            bounds: (width / 3.15, height),
            color: [0.3, 0.3, 0.9, 1.0],
            layout: Layout::default()
                .h_align(HorizontalAlign::Right)
                .v_align(VerticalAlign::Bottom),
            ..Section::default()
        });

        let mut brush_action;
        loop {
            brush_action = glyph_brush.process_queued(
                (width as _, height as _),
                |_rect, _tex_data| {},
                to_vertex,
            );

            match brush_action {
                Ok(_) => break,
                Err(BrushError::TextureTooSmall { suggested, .. }) => {
                    let (new_width, new_height) = suggested;

                    // TODO create new texture

                    glyph_brush.resize_texture(new_width, new_height);
                }
            }
        }
        match brush_action? {
            BrushAction::Draw(vertices) => {
                // Draw new vertices
                vertex_count = vertices.len();
            }
            BrushAction::ReDraw => {}
        }

        unsafe {
            // Clear the screen to black
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, vertex_count as _);
        }

        window.swap_buffers()?;

        if let Some(rate) = loop_helper.report_rate() {
            window.set_title(&format!("{:.0} FPS", rate));
        }
        loop_helper.loop_sleep();
    }

    Ok(())
}

fn compile_shader(src: &str, ty: GLenum) -> Res<GLuint> {
    let shader;
    unsafe {
        shader = gl::CreateShader(ty);
        // Attempt to compile the shader
        let c_str = CString::new(src.as_bytes())?;
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != gl::TRUE as GLint {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
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

fn link_program(vs: GLuint, fs: GLuint) -> Res<GLuint> {
    unsafe {
        let program = gl::CreateProgram();
        gl::AttachShader(program, vs);
        gl::AttachShader(program, fs);
        gl::LinkProgram(program);
        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != gl::TRUE as GLint {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize) - 1); // subtract 1 to skip the trailing null character
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
fn to_vertex(
    glyph_brush::GlyphVertex {
        mut tex_coords,
        pixel_coords,
        bounds,
        screen_dimensions: (screen_w, screen_h),
        color,
        z,
    }: glyph_brush::GlyphVertex,
) -> Vertex {
    let gl_bounds = Rect {
        min: point(
            2.0 * (bounds.min.x / screen_w - 0.5),
            2.0 * (0.5 - bounds.min.y / screen_h),
        ),
        max: point(
            2.0 * (bounds.max.x / screen_w - 0.5),
            2.0 * (0.5 - bounds.max.y / screen_h),
        ),
    };

    let mut gl_rect = Rect {
        min: point(
            2.0 * (pixel_coords.min.x as f32 / screen_w - 0.5),
            2.0 * (0.5 - pixel_coords.min.y as f32 / screen_h),
        ),
        max: point(
            2.0 * (pixel_coords.max.x as f32 / screen_w - 0.5),
            2.0 * (0.5 - pixel_coords.max.y as f32 / screen_h),
        ),
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
    // note: y access is flipped gl compared with screen,
    // texture is not flipped (ie is a headache)
    if gl_rect.max.y < gl_bounds.max.y {
        let old_height = gl_rect.height();
        gl_rect.max.y = gl_bounds.max.y;
        tex_coords.max.y = tex_coords.min.y + tex_coords.height() * gl_rect.height() / old_height;
    }
    if gl_rect.min.y > gl_bounds.min.y {
        let old_height = gl_rect.height();
        gl_rect.min.y = gl_bounds.min.y;
        tex_coords.min.y = tex_coords.max.y - tex_coords.height() * gl_rect.height() / old_height;
    }

    [
        gl_rect.min.x,
        gl_rect.max.y,
        z,
        gl_rect.max.x,
        gl_rect.min.y,
        tex_coords.min.x,
        tex_coords.max.y,
        tex_coords.max.x,
        tex_coords.min.y,
        color[0],
        color[1],
        color[2],
        color[3],
    ]
}
