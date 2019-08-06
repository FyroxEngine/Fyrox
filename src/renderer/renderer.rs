use glutin::ContextTrait;
use crate::{
    renderer::{
        gl,
        gl::types::*,
        surface::*
    },
    math::{
        vec2::*,
        vec3::*,
        mat4::Mat4
    },
    utils::pool::*,
    scene::node::*,
    engine::{ResourceManager, State, duration_to_seconds_f32},
    resource::{
        ResourceKind,
        ttf::Font
    },
    gui::draw::{DrawingContext, CommandKind, Color},
};
use std::{
    ffi::{CStr, CString, c_void},
    mem::size_of,
    time::{Instant, Duration},
    thread
};

// Welcome to the kingdom of Unsafe Code

pub fn check_gl_error() {
    unsafe {
        match gl::GetError() {
            gl::NO_ERROR => (),
            _ => panic!("unknown opengl error!")
        }
    }
}

pub struct GpuProgram {
    id: GLuint,
    name_buf: Vec<u8>,
}

impl GpuProgram {
    pub fn create_shader(actual_type: GLuint, source: &CStr) -> Result<GLuint, String> {
        unsafe {
            let shader = gl::CreateShader(actual_type);
            gl::ShaderSource(shader, 1, &source.as_ptr(), std::ptr::null());
            gl::CompileShader(shader);

            let mut status = 1;
            gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
            if status == 0 {
                let mut log_len = 0;
                gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut log_len);
                let mut buffer: Vec<u8> = Vec::with_capacity(log_len as usize);
                gl::GetShaderInfoLog(shader, log_len, std::ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
                Err(String::from_utf8_unchecked(buffer))
            } else {
                println!("Shader compiled!");
                Ok(shader)
            }
        }
    }

    pub fn from_source(vertex_source: &CStr, fragment_source: &CStr) -> Result<GpuProgram, String> {
        unsafe {
            let vertex_shader = Self::create_shader(gl::VERTEX_SHADER, vertex_source).unwrap();
            let fragment_shader = Self::create_shader(gl::FRAGMENT_SHADER, fragment_source).unwrap();
            let program: GLuint = gl::CreateProgram();
            gl::AttachShader(program, vertex_shader);
            gl::DeleteShader(vertex_shader);
            gl::AttachShader(program, fragment_shader);
            gl::DeleteShader(fragment_shader);
            gl::LinkProgram(program);
            let mut status = 1;
            gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);
            if status == 0 {
                let mut log_len = 0;
                gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut log_len);
                let mut buffer: Vec<u8> = Vec::with_capacity(log_len as usize);
                gl::GetProgramInfoLog(program, log_len, std::ptr::null_mut(), buffer.as_mut_ptr() as *mut i8);
                Err(String::from_utf8_unchecked(buffer))
            } else {
                Ok(Self {
                    id: program,
                    name_buf: Vec::new(),
                })
            }
        }
    }

    pub fn get_uniform_location(&mut self, name: &str) -> GLint {
        // Form c string in special buffer to reduce memory allocations
        let buf = &mut self.name_buf;
        buf.clear();
        buf.extend_from_slice(name.as_bytes());
        buf.push(0);
        unsafe {
            gl::GetUniformLocation(self.id, buf.as_ptr() as *const i8)
        }
    }
}

impl Drop for GpuProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

struct UIShader {
    program: GpuProgram,
    wvp_matrix: GLint,
    diffuse_texture: GLint,
}

struct FlatShader {
    program: GpuProgram,
    wvp_matrix: GLint,
    diffuse_texture: GLint,
}

struct UIRenderBuffers {
    vbo: GLuint,
    vao: GLuint,
    ebo: GLuint,
}

pub struct Statistics {
    pub frame_time: f32,
    pub mean_fps: usize,
    pub min_fps: usize,
    pub current_fps: usize,
    frame_time_accumulator: f32,
    frame_time_measurements: usize,
    time_last_fps_measured: f32,
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            frame_time: 0.0,
            mean_fps: 0,
            min_fps: 0,
            current_fps: 0,
            frame_time_accumulator: 0.0,
            frame_time_measurements: 0,
            time_last_fps_measured: 0.0,
        }
    }
}

pub struct Renderer {
    pub(crate) events_loop: glutin::EventsLoop,
    pub(crate) context: glutin::WindowedContext,
    flat_shader: FlatShader,
    ui_shader: UIShader,
    /// Dummy white one pixel texture which will be used as stub when rendering
    /// something without texture specified.
    white_dummy: GLuint,
    /// Separate lists of handles to nodes of specified kinds. Used reduce tree traversal
    /// count, it will performed once. Lists are valid while there is scene to render.
    cameras: Vec<Handle<Node>>,
    lights: Vec<Handle<Node>>,
    meshes: Vec<Handle<Node>>,
    /// Scene graph traversal stack.
    traversal_stack: Vec<Handle<Node>>,
    frame_rate_limit: usize,
    ui_render_buffers: UIRenderBuffers,
    statistics: Statistics,
}

fn create_flat_shader() -> FlatShader {
    let fragment_source = CString::new(r#"
        #version 330 core

        uniform sampler2D diffuseTexture;

        out vec4 FragColor;

        in vec2 texCoord;

        void main()
        {
            FragColor = texture(diffuseTexture, texCoord);
        }
        "#).unwrap();

    let vertex_source = CString::new(r#"
        #version 330 core

        layout(location = 0) in vec3 vertexPosition;
        layout(location = 1) in vec2 vertexTexCoord;

        uniform mat4 worldViewProjection;

        out vec2 texCoord;

        void main()
        {
            texCoord = vertexTexCoord;
            gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
        }
        "#).unwrap();

    let mut program = GpuProgram::from_source(&vertex_source, &fragment_source).unwrap();
    FlatShader {
        wvp_matrix: program.get_uniform_location("worldViewProjection"),
        diffuse_texture: program.get_uniform_location("diffuseTexture"),
        program,
    }
}

fn create_ui_shader() -> UIShader {
    let fragment_source = CString::new(r#"
        #version 330 core

        uniform sampler2D diffuseTexture;

        out vec4 FragColor;
        in vec2 texCoord;
        in vec4 color;

        void main()
        {
            FragColor = color;
            FragColor.a *= texture(diffuseTexture, texCoord).r;
        };"#).unwrap();


    let vertex_source = CString::new(r#"
        #version 330 core

        layout(location = 0) in vec3 vertexPosition;
        layout(location = 1) in vec2 vertexTexCoord;
        layout(location = 2) in vec4 vertexColor;

        uniform mat4 worldViewProjection;

        out vec2 texCoord;
        out vec4 color;

        void main()
        {
            texCoord = vertexTexCoord;
            color = vertexColor;
            gl_Position = worldViewProjection * vec4(vertexPosition, 1.0);
        };"#).unwrap();

    let mut program = GpuProgram::from_source(&vertex_source, &fragment_source).unwrap();
    UIShader {
        wvp_matrix: program.get_uniform_location("worldViewProjection"),
        diffuse_texture: program.get_uniform_location("diffuseTexture"),
        program,
    }
}

fn create_ui_render_buffers() -> UIRenderBuffers {
    unsafe {
        let mut ui_render_buffers = UIRenderBuffers {
            vbo: 0,
            ebo: 0,
            vao: 0,
        };

        gl::GenVertexArrays(1, &mut ui_render_buffers.vao);
        gl::GenBuffers(1, &mut ui_render_buffers.vbo);
        gl::GenBuffers(1, &mut ui_render_buffers.ebo);

        ui_render_buffers
    }
}

fn create_white_dummy() -> GLuint {
    unsafe {
        let mut texture: GLuint = 0;
        let white_pixel: [Color; 1] = [Color { r: 255, g: 255, b: 255, a: 255 }; 1];
        gl::GenTextures(1, &mut texture);

        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            1,
            1,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            white_pixel.as_ptr() as *const c_void,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MAG_FILTER,
            gl::LINEAR as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR as i32,
        );
        gl::BindTexture(gl::TEXTURE_2D, 0);

        texture
    }
}

impl Renderer {
    pub fn new() -> Self {
        unsafe {
            let events_loop = glutin::EventsLoop::new();

            let primary_monitor = events_loop.get_primary_monitor();
            let mut monitor_dimensions = primary_monitor.get_dimensions();
            monitor_dimensions.height = monitor_dimensions.height * 0.6;
            monitor_dimensions.width = monitor_dimensions.width * 0.6;
            let window_size = monitor_dimensions.to_logical(primary_monitor.get_hidpi_factor());

            let window_builder = glutin::WindowBuilder::new()
                .with_title("RG3D")
                .with_dimensions(window_size)
                .with_resizable(false);

            let context = glutin::ContextBuilder::new()
                .with_vsync(true)
                .build_windowed(window_builder, &events_loop)
                .unwrap();

            context.make_current().unwrap();
            gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

            gl::Enable(gl::DEPTH_TEST);

            Self {
                context,
                events_loop,
                flat_shader: create_flat_shader(),
                ui_shader: create_ui_shader(),
                traversal_stack: Vec::new(),
                cameras: Vec::new(),
                lights: Vec::new(),
                meshes: Vec::new(),
                frame_rate_limit: 60,
                statistics: Statistics::default(),
                white_dummy: create_white_dummy(),
                ui_render_buffers: create_ui_render_buffers(),
            }
        }
    }

    pub fn get_statistics(&self) -> &Statistics {
        &self.statistics
    }

    fn draw_surface(&self, surf: &Surface, data: &SurfaceSharedData, resource_manager: &ResourceManager) {
        unsafe {
            if let Some(resource) = resource_manager.borrow_resource(surf.get_texture_resource_handle()) {
                if let ResourceKind::Texture(texture) = resource.borrow_kind() {
                    gl::BindTexture(gl::TEXTURE_2D, texture.gpu_tex);
                } else {
                    gl::BindTexture(gl::TEXTURE_2D, self.white_dummy);
                }
            } else {
                gl::BindTexture(gl::TEXTURE_2D, self.white_dummy);
            }
            gl::BindVertexArray(data.get_vertex_array_object());
            gl::DrawElements(gl::TRIANGLES,
                             data.get_indices().len() as GLint,
                             gl::UNSIGNED_INT,
                             std::ptr::null());
        }
    }

    fn upload_surface_data(&self, ssd: &mut SurfaceSharedData) {
        let total_size_bytes = ssd.get_vertices().len() * std::mem::size_of::<Vertex>();

        unsafe {
            gl::BindVertexArray(ssd.get_vertex_array_object());

            // Upload indices
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ssd.get_element_buffer_object());
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER,
                           (ssd.get_indices().len() * std::mem::size_of::<i32>()) as GLsizeiptr,
                           ssd.get_indices().as_ptr() as *const GLvoid,
                           gl::STATIC_DRAW);

            // Upload vertices
            gl::BindBuffer(gl::ARRAY_BUFFER, ssd.get_vertex_buffer_object());
            gl::BufferData(gl::ARRAY_BUFFER,
                           total_size_bytes as GLsizeiptr,
                           ssd.get_vertices().as_ptr() as *const GLvoid,
                           gl::STATIC_DRAW);

            let mut offset = 0;

            // Positions
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE,
                                    size_of::<Vertex>() as GLint, offset as *const c_void);
            gl::EnableVertexAttribArray(0);
            offset += size_of::<Vec3>();

            // Texture coordinates
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE,
                                    size_of::<Vertex>() as GLint, offset as *const c_void);
            gl::EnableVertexAttribArray(1);
            offset += size_of::<Vec2>();

            // Normals
            gl::VertexAttribPointer(2, 3, gl::FLOAT, gl::FALSE,
                                    size_of::<Vertex>() as GLint, offset as *const c_void);
            gl::EnableVertexAttribArray(2);
            offset += size_of::<Vec3>();

            // Tangents
            gl::VertexAttribPointer(3, 4, gl::FLOAT, gl::FALSE,
                                    size_of::<Vertex>() as GLint, offset as *const c_void);
            gl::EnableVertexAttribArray(3);

            gl::BindVertexArray(0);

            check_gl_error();
        }
    }

    pub fn upload_font_cache(&mut self, font_cache: &mut Pool<Font>) {
        unsafe {
            for font in font_cache.iter_mut() {
                if font.get_texture_id() == 0 {
                    let mut texture: GLuint = 0;
                    gl::GenTextures(1, &mut texture);

                    gl::BindTexture(gl::TEXTURE_2D, texture);

                    let rgba_pixels: Vec<Color> = font.get_atlas_pixels().
                        iter().map(|p| Color { r: *p, g: *p, b: *p, a: *p }).collect();

                    gl::TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        gl::RGBA as i32,
                        font.get_atlas_size(),
                        font.get_atlas_size(),
                        0,
                        gl::RGBA,
                        gl::UNSIGNED_BYTE,
                        rgba_pixels.as_ptr() as *const c_void,
                    );
                    gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MAG_FILTER,
                        gl::LINEAR as i32,
                    );
                    gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MIN_FILTER,
                        gl::LINEAR as i32,
                    );
                    gl::BindTexture(gl::TEXTURE_2D, 0);

                    println!("font cache loaded! {}", texture);

                    font.set_texture_id(texture);
                }
            }
        }

        check_gl_error();
    }

    pub fn upload_resources(&mut self, state: &mut State) {
        state.get_resource_manager_mut().for_each_texture_mut(|texture| {
            if texture.need_upload {
                unsafe {
                    if texture.gpu_tex == 0 {
                        gl::GenTextures(1, &mut texture.gpu_tex);
                    }
                    gl::BindTexture(gl::TEXTURE_2D, texture.gpu_tex);
                    gl::TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        gl::RGBA as i32,
                        texture.width as i32,
                        texture.height as i32,
                        0,
                        gl::RGBA,
                        gl::UNSIGNED_BYTE,
                        texture.pixels.as_ptr() as *const c_void,
                    );
                    gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MAG_FILTER,
                        gl::LINEAR as i32,
                    );
                    gl::TexParameteri(
                        gl::TEXTURE_2D,
                        gl::TEXTURE_MIN_FILTER,
                        gl::LINEAR_MIPMAP_LINEAR as i32,
                    );
                    gl::GenerateMipmap(gl::TEXTURE_2D);
                    texture.need_upload = false;
                }
            }
        });

        for data in state.get_surface_data_storage_mut().iter_mut() {
            if data.need_upload {
                self.upload_surface_data(data);
                data.need_upload = false;
            }
        }
    }

    pub fn render(&mut self, state: &State, drawing_context: &DrawingContext) {
        let frame_start_time = Instant::now();

        unsafe {
            let client_size = self.context.get_inner_size().unwrap();

            gl::ClearColor(0.0, 0.63, 0.91, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);

            for scene in state.get_scenes().iter() {
                // Prepare for render - fill lists of nodes participating in rendering
                // by traversing scene graph
                self.meshes.clear();
                self.lights.clear();
                self.cameras.clear();
                self.traversal_stack.clear();
                self.traversal_stack.push(scene.root.clone());
                while let Some(node_handle) = self.traversal_stack.pop() {
                    if let Some(node) = scene.get_node(&node_handle) {
                        match node.borrow_kind() {
                            NodeKind::Mesh(_) => self.meshes.push(node_handle),
                            NodeKind::Light(_) => self.lights.push(node_handle),
                            NodeKind::Camera(_) => self.cameras.push(node_handle),
                            _ => ()
                        }
                        // Queue children for render
                        for child_handle in node.children.iter() {
                            self.traversal_stack.push(child_handle.clone());
                        }
                    }
                }

                gl::UseProgram(self.flat_shader.program.id);

                // Render scene from each camera
                for camera_handle in self.cameras.iter() {
                    if let Some(camera_node) = scene.get_node(&camera_handle) {
                        if let NodeKind::Camera(camera) = camera_node.borrow_kind() {

                            // Setup viewport
                            let viewport = camera.get_viewport_pixels(
                                Vec2 {
                                    x: client_size.width as f32,
                                    y: client_size.height as f32,
                                });
                            gl::Viewport(viewport.x, viewport.y, viewport.w, viewport.h);


                            let view_projection = camera.get_view_projection_matrix();

                            for mesh_handle in self.meshes.iter() {
                                if let Some(node) = scene.get_node(&mesh_handle) {
                                    if !node.get_global_visibility() {
                                        continue;
                                    }

                                    let mvp = view_projection * node.global_transform;

                                    gl::UseProgram(self.flat_shader.program.id);
                                    gl::UniformMatrix4fv(self.flat_shader.wvp_matrix, 1, gl::FALSE, &mvp.f as *const GLfloat);

                                    if let NodeKind::Mesh(mesh) = node.borrow_kind() {
                                        for surface in mesh.get_surfaces().iter() {
                                            if let Some(data) = state.get_surface_data_storage().borrow(surface.get_data_handle()) {
                                                self.draw_surface(surface, data, state.get_resource_manager());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Render UI on top of everything
            gl::Disable(gl::DEPTH_TEST);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Disable(gl::CULL_FACE);

            gl::UseProgram(self.ui_shader.program.id);
            gl::ActiveTexture(gl::TEXTURE0);

            let index_bytes = drawing_context.get_indices_bytes();
            let vertex_bytes = drawing_context.get_vertices_bytes();

            // Upload to GPU.
            gl::BindVertexArray(self.ui_render_buffers.vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, self.ui_render_buffers.vbo);
            gl::BufferData(gl::ARRAY_BUFFER, vertex_bytes, drawing_context.get_vertices_ptr(), gl::DYNAMIC_DRAW);

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ui_render_buffers.ebo);
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER, index_bytes, drawing_context.get_indices_ptr(), gl::DYNAMIC_DRAW);

            let mut offset = 0;
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE,
                                    drawing_context.get_vertex_size(),
                                    offset as *const c_void);
            gl::EnableVertexAttribArray(0);
            offset += std::mem::size_of::<Vec2>();

            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE,
                                    drawing_context.get_vertex_size(),
                                    offset as *const c_void);
            gl::EnableVertexAttribArray(1);
            offset += std::mem::size_of::<Vec2>();

            gl::VertexAttribPointer(2, 4, gl::UNSIGNED_BYTE, gl::TRUE,
                                    drawing_context.get_vertex_size(),
                                    offset as *const c_void);
            gl::EnableVertexAttribArray(2);

            let ortho = Mat4::ortho(0.0,
                                    client_size.width as f32,
                                    client_size.height as f32,
                                    0.0,
                                    -1.0,
                                    1.0);
            gl::UniformMatrix4fv(self.ui_shader.wvp_matrix, 1, gl::FALSE, ortho.f.as_ptr() as *const f32);

            for cmd in drawing_context.get_command_buffer() {
                let index_count = cmd.get_triangle_count() * 3;
                if cmd.get_nesting() != 0 {
                    gl::Enable(gl::STENCIL_TEST);
                } else {
                    gl::Disable(gl::STENCIL_TEST);
                }
                match cmd.get_kind() {
                    CommandKind::Clip => {
                        if cmd.get_nesting() == 1 {
                            gl::Clear(gl::STENCIL_BUFFER_BIT);
                        }
                        gl::StencilOp(gl::KEEP, gl::KEEP, gl::INCR);
                        // Make sure that clipping rect will be drawn at previous nesting level only (clip to parent)
                        gl::StencilFunc(gl::EQUAL, (cmd.get_nesting() - 1) as i32, 0xFF);
                        gl::BindTexture(gl::TEXTURE_2D, self.white_dummy);
                        // Draw clipping geometry to stencil buffer
                        gl::StencilMask(0xFF);
                        gl::ColorMask(gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);
                    }
                    CommandKind::Geometry => {
                        // Make sure to draw geometry only on clipping geometry with current nesting level
                        gl::StencilFunc(gl::EQUAL, cmd.get_nesting() as i32, 0xFF);

                        if cmd.get_texture() != 0 {
                            gl::ActiveTexture(gl::TEXTURE0);
                            gl::Uniform1i(self.ui_shader.diffuse_texture, 0);
                            gl::BindTexture(gl::TEXTURE_2D, cmd.get_texture());
                        } else {
                            gl::BindTexture(gl::TEXTURE_2D, self.white_dummy);
                        }

                        gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
                        // Do not draw geometry to stencil buffer
                        gl::StencilMask(0x00);
                    }
                }

                let index_offset_bytes = cmd.get_index_offset() * std::mem::size_of::<GLuint>();
                gl::DrawElements(gl::TRIANGLES, index_count as i32, gl::UNSIGNED_INT,
                                 index_offset_bytes as *const c_void);

            }
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::BindVertexArray(0);
            gl::Disable(gl::STENCIL_TEST);
            gl::Disable(gl::BLEND);
            gl::Enable(gl::DEPTH_TEST);
        }

        check_gl_error();

        self.context.swap_buffers().unwrap();

        if self.frame_rate_limit > 0 {
            let frame_time_ms = 1000.0 * duration_to_seconds_f32(Instant::now().duration_since(frame_start_time));
            let desired_frame_time_ms = 1000.0 / self.frame_rate_limit as f32;
            if frame_time_ms < desired_frame_time_ms {
                let sleep_time_us = 1000.0 * (desired_frame_time_ms - frame_time_ms);
                thread::sleep(Duration::from_micros(sleep_time_us as u64));
            }
        }

        let total_time_s = duration_to_seconds_f32(Instant::now().duration_since(frame_start_time));
        self.statistics.frame_time = total_time_s;
        self.statistics.current_fps = (1.0 / total_time_s) as usize;
    }
}