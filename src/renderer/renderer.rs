use glutin::ContextTrait;
use crate::renderer::gl;
use crate::renderer::gl::types::*;
use std::ffi::{CStr, CString};
use crate::math::vec2::*;
use crate::scene::*;
use crate::utils::pool::*;
use crate::scene::node::*;
use crate::renderer::surface::*;
use crate::resource::*;
use std::rc::*;
use std::ffi::c_void;
use std::cell::*;
use crate::engine::ResourceManager;

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

pub struct Renderer {
    pub(crate) events_loop: glutin::EventsLoop,
    pub(crate) context: glutin::WindowedContext,
    flat_shader: GpuProgram,

    /// Separate lists of handles to nodes of specified kinds
    /// Used reduce tree traversal count, it will performed once.
    /// Lists are valid while there is scene to render.
    cameras: Vec<Handle<Node>>,
    lights: Vec<Handle<Node>>,
    meshes: Vec<Handle<Node>>,

    /// Scene graph traversal stack
    traversal_stack: Vec<Handle<Node>>,
}

impl Renderer {
    pub fn new() -> Self {
        let el = glutin::EventsLoop::new();

        let primary_monitor = el.get_primary_monitor();
        let mut dim = primary_monitor.get_dimensions();
        dim.height = dim.height * 0.5;
        dim.width = dim.width * 0.5;
        let window_size = dim.to_logical(primary_monitor.get_hidpi_factor());

        let wb = glutin::WindowBuilder::new()
            .with_title("RG3D")
            .with_dimensions(window_size)
            .with_resizable(false);

        let context = glutin::ContextBuilder::new()
            .with_vsync(true)
            .build_windowed(wb, &el)
            .unwrap();

        unsafe {
            context.make_current().unwrap();
        }

        gl::load_with(|symbol| context.get_proc_address(symbol) as *const _);

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
        }

        println!("creating shaders...");

        let fragment_source = CString::new(r#"
            #version 330 core
            uniform sampler2D diffuseTexture;
            out vec4 FragColor;
            in vec2 texCoord;
            void main()
            {
                FragColor = texture(diffuseTexture, texCoord);
            }"#
        ).unwrap();

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
            }"#
        ).unwrap();

        Self {
            context,
            events_loop: el,
            flat_shader: GpuProgram::from_source(&vertex_source, &fragment_source).unwrap(),
            traversal_stack: Vec::new(),
            cameras: Vec::new(),
            lights: Vec::new(),
            meshes: Vec::new(),
        }
    }

    fn draw_surface(&mut self, surf: &Surface) {}

    pub fn upload_resources(&mut self, resource_manager: &mut ResourceManager) {
        resource_manager.for_each_texture_mut(|texture| {
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
    }

    pub fn render(&mut self, scenes: &Pool<Scene>, resource_manager: &ResourceManager) {
        let client_size = self.context.get_inner_size().unwrap();

        unsafe {
            gl::ClearColor(0.0, 0.63, 0.91, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        for scene in scenes.iter() {
            // Prepare for render - fill lists of nodes participating in rendering
            // by traversing scene graph
            self.meshes.clear();
            self.lights.clear();
            self.cameras.clear();
            self.traversal_stack.clear();
            self.traversal_stack.push(scene.root.clone());
            while let Some(node_handle) = self.traversal_stack.pop() {
                if let Some(node) = scene.borrow_node(&node_handle) {
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

            unsafe {
                gl::UseProgram(self.flat_shader.id);
            }

            let u_wvp = self.flat_shader.get_uniform_location("worldViewProjection");

            // Render scene from each camera
            for camera_handle in self.cameras.iter() {
                if let Some(camera_node) = scene.borrow_node(&camera_handle) {
                    if let NodeKind::Camera(camera) = camera_node.borrow_kind() {

                        // Setup viewport
                        unsafe {
                            let viewport = camera.get_viewport_pixels(
                                Vec2 {
                                    x: client_size.width as f32,
                                    y: client_size.height as f32,
                                });
                            gl::Viewport(viewport.x, viewport.y, viewport.w, viewport.h);
                        }

                        let view_projection = camera.get_view_projection_matrix();

                        for mesh_handle in self.meshes.iter() {
                            if let Some(node) = scene.borrow_node(&mesh_handle) {
                                let mvp = view_projection * node.global_transform;

                                unsafe {
                                    gl::UseProgram(self.flat_shader.id);
                                    gl::UniformMatrix4fv(u_wvp, 1, gl::FALSE, &mvp.f as *const GLfloat);
                                }

                                if let NodeKind::Mesh(mesh) = node.borrow_kind() {
                                    for surface in mesh.get_surfaces().iter() {
                                        surface.draw(scene.get_surface_data_storage(), resource_manager);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        self.context.swap_buffers().unwrap();
    }
}