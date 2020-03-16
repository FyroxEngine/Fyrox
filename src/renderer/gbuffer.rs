use crate::{
    scene::{
        node::Node,
        graph::Graph,
        camera::Camera,
        base::AsBase,
    },
    renderer::{
        gl::types::GLuint,
        gl,
        gpu_program::{GpuProgram, UniformLocation},
        error::RendererError,
        gpu_texture::GpuTexture,
        RenderPassStatistics,
        GlState,
        TextureCache,
        GeometryCache
    },
    core::math::{
        Rect,
        mat4::Mat4,
        frustum::Frustum,
    },
};

struct GBufferShader {
    program: GpuProgram,
    world_matrix: UniformLocation,
    wvp_matrix: UniformLocation,
    use_skeletal_animation: UniformLocation,
    bone_matrices: UniformLocation,
    diffuse_texture: UniformLocation,
    normal_texture: UniformLocation,
}

impl GBufferShader {
    fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/gbuffer_fs.glsl");
        let vertex_source = include_str!("shaders/gbuffer_vs.glsl");
        let mut program = GpuProgram::from_source("GBufferShader", vertex_source, fragment_source)?;
        Ok(Self {
            world_matrix: program.get_uniform_location("worldMatrix")?,
            wvp_matrix: program.get_uniform_location("worldViewProjection")?,
            use_skeletal_animation: program.get_uniform_location("useSkeletalAnimation")?,
            bone_matrices: program.get_uniform_location("boneMatrices")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            normal_texture: program.get_uniform_location("normalTexture")?,
            program,
        })
    }

    fn bind(&mut self) -> &mut Self {
        self.program.bind();
        self
    }

    fn set_world_matrix(&mut self, mat: &Mat4) -> &mut Self {
        self.program.set_mat4(self.world_matrix, mat);
        self
    }

    fn set_wvp_matrix(&mut self, mat: &Mat4) -> &mut Self {
        self.program.set_mat4(self.wvp_matrix, mat);
        self
    }

    fn set_use_skeletal_animation(&mut self, value: bool) -> &mut Self {
        self.program.set_int(self.use_skeletal_animation, if value { 1 } else { 0 });
        self
    }

    fn set_bone_matrices(&mut self, matrices: &[Mat4]) -> &mut Self {
        self.program.set_mat4_array(self.bone_matrices, matrices);
        self
    }

    fn set_diffuse_texture(&mut self, id: i32) -> &mut Self {
        self.program.set_int(self.diffuse_texture, id);
        self
    }

    fn set_normal_texture(&mut self, id: i32) -> &mut Self {
        self.program.set_int(self.normal_texture, id);
        self
    }
}

pub struct GBuffer {
    shader: GBufferShader,
    pub fbo: GLuint,
    pub depth_rt: GLuint,
    pub depth_buffer: GLuint,
    pub depth_texture: GLuint,
    pub color_rt: GLuint,
    pub color_texture: GLuint,
    pub normal_rt: GLuint,
    pub normal_texture: GLuint,
    pub opt_fbo: GLuint,
    pub frame_texture: GLuint,
    bone_matrices: Vec<Mat4>,
    pub width: i32,
    pub height: i32,
}

impl GBuffer {
    pub fn new(width: i32, height: i32) -> Result<Self, RendererError>
    {
        unsafe {
            let mut fbo = 0;
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);

            let buffers = [
                gl::COLOR_ATTACHMENT0,
                gl::COLOR_ATTACHMENT1,
                gl::COLOR_ATTACHMENT2
            ];
            gl::DrawBuffers(3, buffers.as_ptr());

            let mut depth_rt = 0;
            gl::GenRenderbuffers(1, &mut depth_rt);
            gl::BindRenderbuffer(gl::RENDERBUFFER, depth_rt);
            gl::RenderbufferStorage(gl::RENDERBUFFER, gl::R32F, width, height);
            gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::RENDERBUFFER, depth_rt);

            let mut color_rt = 0;
            gl::GenRenderbuffers(1, &mut color_rt);
            gl::BindRenderbuffer(gl::RENDERBUFFER, color_rt);
            gl::RenderbufferStorage(gl::RENDERBUFFER, gl::RGBA8, width, height);
            gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT1, gl::RENDERBUFFER, color_rt);

            let mut normal_rt = 0;
            gl::GenRenderbuffers(1, &mut normal_rt);
            gl::BindRenderbuffer(gl::RENDERBUFFER, normal_rt);
            gl::RenderbufferStorage(gl::RENDERBUFFER, gl::RGBA8, width, height);
            gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT2, gl::RENDERBUFFER, normal_rt);

            let mut depth_buffer = 0;
            gl::GenRenderbuffers(1, &mut depth_buffer);
            gl::BindRenderbuffer(gl::RENDERBUFFER, depth_buffer);
            gl::RenderbufferStorage(gl::RENDERBUFFER, gl::DEPTH24_STENCIL8, width, height);
            gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::DEPTH_STENCIL_ATTACHMENT, gl::RENDERBUFFER, depth_buffer);

            let mut depth_texture = 0;
            gl::GenTextures(1, &mut depth_texture);
            gl::BindTexture(gl::TEXTURE_2D, depth_texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::R32F as i32, width, height, 0, gl::BGRA, gl::FLOAT, std::ptr::null());

            gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, depth_texture, 0);

            let mut color_texture = 0;
            gl::GenTextures(1, &mut color_texture);
            gl::BindTexture(gl::TEXTURE_2D, color_texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA8 as i32, width, height, 0, gl::BGRA, gl::UNSIGNED_BYTE, std::ptr::null());

            gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT1, gl::TEXTURE_2D, color_texture, 0);

            let mut normal_texture = 0;
            gl::GenTextures(1, &mut normal_texture);
            gl::BindTexture(gl::TEXTURE_2D, normal_texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA8 as i32, width, height, 0, gl::BGRA, gl::UNSIGNED_BYTE, std::ptr::null());

            gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT2, gl::TEXTURE_2D, normal_texture, 0);

            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                return Err(RendererError::FailedToConstructFBO);
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

            // Create another framebuffer for stencil optimizations.
            let mut opt_fbo = 0;
            gl::GenFramebuffers(1, &mut opt_fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, opt_fbo);

            let light_buffers = [gl::COLOR_ATTACHMENT0];
            gl::DrawBuffers(1, light_buffers.as_ptr());

            let mut frame_texture = 0;
            gl::GenTextures(1, &mut frame_texture);
            gl::BindTexture(gl::TEXTURE_2D, frame_texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA8 as i32, width, height, 0, gl::BGRA, gl::UNSIGNED_BYTE, std::ptr::null());

            gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, frame_texture, 0);

            gl::FramebufferRenderbuffer(gl::FRAMEBUFFER, gl::DEPTH_STENCIL_ATTACHMENT, gl::RENDERBUFFER, depth_buffer);

            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                return Err(RendererError::FailedToConstructFBO);
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

            Ok(GBuffer {
                fbo,
                depth_rt,
                depth_buffer,
                depth_texture,
                color_rt,
                color_texture,
                normal_rt,
                normal_texture,
                opt_fbo,
                frame_texture,
                shader: GBufferShader::new()?,
                bone_matrices: Vec::new(),
                width,
                height,
            })
        }
    }

    #[must_use]
    pub fn fill(&mut self,
                graph: &Graph,
                camera: &Camera,
                white_dummy: &GpuTexture,
                normal_dummy: &GpuTexture,
                gl_state: &mut GlState,
                texture_cache: &mut TextureCache,
                geom_cache: &mut GeometryCache,
    ) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        let frustum = Frustum::from(camera.view_projection_matrix()).unwrap();

        gl_state.push_viewport(Rect::new(0, 0, self.width, self.height));

        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);

            self.shader.bind();
            self.shader.set_diffuse_texture(0);
            self.shader.set_normal_texture(1);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::STENCIL_TEST);
            gl::Disable(gl::BLEND);
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthMask(gl::TRUE);
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        }

        let view_projection = camera.view_projection_matrix();

        for mesh in graph.linear_iter().filter_map(|node| {
            if let Node::Mesh(mesh) = node { Some(mesh) } else { None }
        }) {
            let global_transform = mesh.base().global_transform();

            if !frustum.is_intersects_aabb_transform(&mesh.bounding_box(), &global_transform) {
                continue;
            }

            if !mesh.base().global_visibility() {
                continue;
            }

            for surface in mesh.surfaces().iter() {
                let is_skinned = !surface.bones.is_empty();

                let world = if is_skinned {
                    Mat4::IDENTITY
                } else {
                    mesh.base().global_transform()
                };
                let mvp = view_projection * world;

                self.shader.set_wvp_matrix(&mvp);
                self.shader.set_world_matrix(&world);

                self.shader.set_use_skeletal_animation(is_skinned);

                if is_skinned {
                    self.bone_matrices.clear();
                    for bone_handle in surface.bones.iter() {
                        let bone_node = graph.get(*bone_handle);
                        self.bone_matrices.push(
                            bone_node.base().global_transform() *
                                bone_node.base().inv_bind_pose_transform());
                    }

                    self.shader.set_bone_matrices(&self.bone_matrices);
                }

                // Bind diffuse texture.
                if let Some(texture) = surface.get_diffuse_texture() {
                    if let Some(texture) = texture_cache.get(texture) {
                        texture.bind(0);
                    }
                } else {
                    white_dummy.bind(0);
                }

                // Bind normal texture.
                if let Some(texture) = surface.get_normal_texture() {
                    if let Some(texture) = texture_cache.get(texture) {
                        texture.bind(1);
                    }
                } else {
                    normal_dummy.bind(1);
                }

                statistics.add_draw_call( geom_cache.draw(&surface.get_data().lock().unwrap()));
            }
        }

        gl_state.pop_viewport();

        statistics
    }
}

impl Drop for GBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.fbo);
            gl::DeleteRenderbuffers(1, &self.depth_buffer);
            gl::DeleteRenderbuffers(1, &self.depth_rt);
            gl::DeleteRenderbuffers(1, &self.normal_rt);
            gl::DeleteRenderbuffers(1, &self.color_rt);
            gl::DeleteTextures(1, &self.color_texture);
            gl::DeleteTextures(1, &self.depth_texture);
            gl::DeleteTextures(1, &self.normal_texture);
            gl::DeleteFramebuffers(1, &self.opt_fbo);
            gl::DeleteTextures(1, &self.frame_texture);
        }
    }
}