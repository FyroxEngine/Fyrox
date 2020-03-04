use std::ffi::CString;
use crate::{
    scene::{
        node::Node,
        base::AsBase,
        graph::Graph,
        camera::Camera
    },
    renderer::{
        surface::SurfaceSharedData,
        gpu_program::{GpuProgram, UniformLocation},
        error::RendererError,
        gl,
        gpu_texture::GpuTexture,
        RenderPassStatistics,
    },
    core::{
        color::Color,
        math::{
            mat4::Mat4,
            vec3::Vec3,
        },
    },
};
use crate::renderer::gbuffer::GBuffer;
use crate::renderer::GlState;
use rg3d_core::math::Rect;

pub struct SpriteShader {
    program: GpuProgram,
    view_projection_matrix: UniformLocation,
    world_matrix: UniformLocation,
    camera_side_vector: UniformLocation,
    camera_up_vector: UniformLocation,
    color: UniformLocation,
    diffuse_texture: UniformLocation,
    size: UniformLocation,
    rotation: UniformLocation,
}

impl SpriteShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = CString::new(include_str!("shaders/sprite_fs.glsl"))?;
        let vertex_source = CString::new(include_str!("shaders/sprite_vs.glsl"))?;
        let mut program = GpuProgram::from_source("FlatShader", &vertex_source, &fragment_source)?;
        Ok(Self {
            view_projection_matrix: program.get_uniform_location("viewProjectionMatrix")?,
            world_matrix: program.get_uniform_location("worldMatrix")?,
            camera_side_vector: program.get_uniform_location("cameraSideVector")?,
            camera_up_vector: program.get_uniform_location("cameraUpVector")?,
            size: program.get_uniform_location("size")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            color: program.get_uniform_location("color")?,
            rotation: program.get_uniform_location("rotation")?,
            program,
        })
    }

    pub fn bind(&self) {
        self.program.bind();
    }

    pub fn set_view_projection_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.view_projection_matrix, mat)
    }

    pub fn set_world_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.world_matrix, mat)
    }

    pub fn set_camera_side_vector(&self, vec: &Vec3) {
        self.program.set_vec3(self.camera_side_vector, vec)
    }

    pub fn set_camera_up_vector(&self, vec: &Vec3) {
        self.program.set_vec3(self.camera_up_vector, vec)
    }

    pub fn set_size(&self, s: f32) {
        self.program.set_float(self.size, s)
    }

    pub fn set_rotation(&self, r: f32) {
        self.program.set_float(self.rotation, r)
    }

    pub fn set_diffuse_texture(&self, id: i32) {
        self.program.set_int(self.diffuse_texture, id)
    }

    pub fn set_color(&self, color: Color) {
        self.program.set_vec4(self.color, &color.as_frgba());
    }
}

pub struct SpriteRenderer {
    shader: SpriteShader,
    surface: SurfaceSharedData,
}

impl SpriteRenderer {
    pub fn new() -> Result<Self, RendererError> {
        let surface = SurfaceSharedData::make_collapsed_xy_quad();

        Ok(Self {
            shader: SpriteShader::new()?,
            surface,
        })
    }

    #[must_use]
    pub fn render(&mut self,
                  graph: &Graph,
                  camera: &Camera,
                  white_dummy: &GpuTexture,
                  gbuffer: &GBuffer,
                  gl_state: &mut GlState
    ) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        gl_state.push_viewport(Rect::new(0, 0, gbuffer.width, gbuffer.height));

        unsafe {
            gl::Disable(gl::CULL_FACE);
            gl::Enable(gl::BLEND);
            gl::DepthMask(gl::FALSE);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        self.shader.bind();

        let inv_view = camera.inv_view_matrix().unwrap();

        let camera_up = inv_view.up();
        let camera_side = inv_view.side();

        for node in graph.linear_iter() {
            let sprite = if let Node::Sprite(sprite) = node {
                sprite
            } else {
                continue;
            };

            if let Some(texture) = sprite.get_texture() {
                if let Some(texture) = texture.lock().unwrap().gpu_tex.as_ref() {
                    texture.bind(0);
                } else {
                    white_dummy.bind(0)
                }
            } else {
                white_dummy.bind(0)
            }

            self.shader.set_diffuse_texture(0);
            self.shader.set_view_projection_matrix(&camera.view_projection_matrix());
            self.shader.set_world_matrix(&node.base().get_global_transform());
            self.shader.set_camera_up_vector(&camera_up);
            self.shader.set_camera_side_vector(&camera_side);
            self.shader.set_size(sprite.get_size());
            self.shader.set_color(sprite.get_color());
            self.shader.set_rotation(sprite.get_rotation());

            statistics.add_draw_call(self.surface.draw());
        }

        unsafe {
            gl::Disable(gl::BLEND);
            gl::DepthMask(gl::TRUE);
        }

        gl_state.pop_viewport();

        statistics
    }
}