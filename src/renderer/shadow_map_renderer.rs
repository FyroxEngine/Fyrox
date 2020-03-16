use crate::{
    renderer::{
        RenderPassStatistics,
        GlState,
        gpu_texture::GpuTexture,
        gl::types::GLuint,
        gpu_program::{GpuProgram, UniformLocation},
        error::RendererError,
        gl,
    },
    scene::{
        node::Node,
        graph::Graph,
        base::AsBase,
    },
    core::{
        math::{
            mat4::Mat4,
            vec3::Vec3,
            frustum::Frustum,
            Rect,
        }
    },
};
use crate::renderer::{TextureCache, GeometryCache};

pub struct SpotShadowMapShader {
    program: GpuProgram,
    bone_matrices: UniformLocation,
    world_view_projection_matrix: UniformLocation,
    use_skeletal_animation: UniformLocation,
    diffuse_texture: UniformLocation,
}

impl SpotShadowMapShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/spot_shadow_map_fs.glsl");
        let vertex_source = include_str!("shaders/spot_shadow_map_vs.glsl");
        let mut program = GpuProgram::from_source("SpotShadowMapShader", vertex_source, fragment_source)?;
        Ok(Self {
            bone_matrices: program.get_uniform_location("boneMatrices")?,
            world_view_projection_matrix: program.get_uniform_location("worldViewProjection")?,
            use_skeletal_animation: program.get_uniform_location("useSkeletalAnimation")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            program,
        })
    }

    fn bind(&mut self) -> &mut Self {
        self.program.bind();
        self
    }

    fn set_wvp_matrix(&mut self, mat: &Mat4) -> &mut Self {
        self.program.set_mat4(self.world_view_projection_matrix, mat);
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
}

pub struct SpotShadowMapRenderer {
    shader: SpotShadowMapShader,
    fbo: GLuint,
    pub texture: GLuint,
    bone_matrices: Vec<Mat4>,
    pub size: usize,
}

impl SpotShadowMapRenderer {
    pub fn new(size: usize) -> Result<Self, RendererError> {
        unsafe {
            let mut fbo = 0;
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);

            gl::DrawBuffer(gl::NONE);

            let mut texture = 0;
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_BORDER as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_BORDER as i32);
            let color = [1.0, 1.0, 1.0, 1.0];
            gl::TexParameterfv(gl::TEXTURE_2D, gl::TEXTURE_BORDER_COLOR, color.as_ptr());
            gl::TexImage2D(gl::TEXTURE_2D,
                           0,
                           gl::DEPTH_COMPONENT as i32,
                           size as i32,
                           size as i32,
                           0,
                           gl::DEPTH_COMPONENT,
                           gl::FLOAT, std::ptr::null());

            gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::TEXTURE_2D, texture, 0);

            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                return Err(RendererError::InvalidFrameBuffer);
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

            Ok(Self {
                size,
                fbo,
                texture,
                shader: SpotShadowMapShader::new()?,
                bone_matrices: Vec::new(),
            })
        }
    }

    pub fn render(&mut self,
                  graph: &Graph,
                  light_view_projection: &Mat4,
                  white_dummy: &GpuTexture,
                  gl_state: &mut GlState,
                  textures: &mut TextureCache,
                  geom_map: &mut GeometryCache,
    ) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        gl_state.push_viewport(Rect::new(0, 0, self.size as i32, self.size as i32));
        gl_state.push_fbo(self.fbo);

        unsafe {
            gl::DepthMask(gl::TRUE);
            gl::Disable(gl::BLEND);
            gl::Disable(gl::STENCIL_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);
            gl::Clear(gl::DEPTH_BUFFER_BIT);
        }

        self.shader.bind();

        let frustum = Frustum::from(*light_view_projection).unwrap();

        for node in graph.linear_iter() {
            if let Node::Mesh(mesh) = node {
                if !node.base().global_visibility() {
                    continue;
                }

                let global_transform = node.base().global_transform();

                if !frustum.is_intersects_aabb_transform(&mesh.bounding_box(), &global_transform) {
                    continue;
                }

                for surface in mesh.surfaces().iter() {
                    let is_skinned = !surface.bones.is_empty();

                    let world = if is_skinned {
                        Mat4::IDENTITY
                    } else {
                        global_transform
                    };
                    let mvp = *light_view_projection * world;

                    self.shader.set_wvp_matrix(&mvp)
                        .set_use_skeletal_animation(is_skinned);

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
                    self.shader.set_diffuse_texture(0);
                    if let Some(texture) = surface.get_diffuse_texture() {
                        if let Some(texture) = textures.get(texture) {
                            texture.bind(0);
                        } else {
                            white_dummy.bind(0);
                        }
                    } else {
                        white_dummy.bind(0);
                    }

                    statistics.add_draw_call(geom_map.draw(&surface.get_data().lock().unwrap()));
                }
            }
        }

        unsafe {
            gl::DepthMask(gl::FALSE);
            gl::Enable(gl::BLEND);
        }

        gl_state.pop_fbo();
        gl_state.pop_viewport();

        statistics
    }
}

impl Drop for SpotShadowMapRenderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.texture);
            gl::DeleteFramebuffers(1, &self.fbo);
        }
    }
}

pub struct PointShadowMapShader {
    program: GpuProgram,
    world_matrix: UniformLocation,
    bone_matrices: UniformLocation,
    world_view_projection_matrix: UniformLocation,
    use_skeletal_animation: UniformLocation,
    diffuse_texture: UniformLocation,
    light_position: UniformLocation,
}

impl PointShadowMapShader
{
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = include_str!("shaders/point_shadow_map_fs.glsl");
        let vertex_source = include_str!("shaders/point_shadow_map_vs.glsl");
        let mut program = GpuProgram::from_source("PointShadowMapShader", vertex_source, fragment_source)?;
        Ok(Self {
            world_matrix: program.get_uniform_location("worldMatrix")?,
            bone_matrices: program.get_uniform_location("boneMatrices")?,
            world_view_projection_matrix: program.get_uniform_location("worldViewProjection")?,
            use_skeletal_animation: program.get_uniform_location("useSkeletalAnimation")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            light_position: program.get_uniform_location("lightPosition")?,
            program,
        })
    }

    pub fn bind(&mut self) -> &mut Self {
        self.program.bind();
        self
    }

    fn set_wvp_matrix(&mut self, mat: &Mat4) -> &mut Self {
        self.program.set_mat4(self.world_view_projection_matrix, mat);
        self
    }

    fn set_world_matrix(&mut self, mat: &Mat4) -> &mut Self {
        self.program.set_mat4(self.world_matrix, mat);
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

    fn set_light_position(&mut self, pos: Vec3) -> &mut Self {
        self.program.set_vec3(self.light_position, &pos);
        self
    }
}

pub struct PointShadowMapRenderer {
    bone_matrices: Vec<Mat4>,
    shader: PointShadowMapShader,
    fbo: GLuint,
    pub texture: GLuint,
    depth_buffer: GLuint,
    pub size: usize,
}

struct CubeMapFace {
    id: GLuint,
    look: Vec3,
    up: Vec3,
}

impl PointShadowMapRenderer {
    const FACES: [CubeMapFace; 6] = [
        CubeMapFace {
            id: gl::TEXTURE_CUBE_MAP_POSITIVE_X,
            look: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
            up: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
        },
        CubeMapFace {
            id: gl::TEXTURE_CUBE_MAP_NEGATIVE_X,
            look: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
            up: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
        },
        CubeMapFace {
            id: gl::TEXTURE_CUBE_MAP_POSITIVE_Y,
            look: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
            up: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
        },
        CubeMapFace {
            id: gl::TEXTURE_CUBE_MAP_NEGATIVE_Y,
            look: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
            up: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
        },
        CubeMapFace {
            id: gl::TEXTURE_CUBE_MAP_POSITIVE_Z,
            look: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
            up: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
        },
        CubeMapFace {
            id: gl::TEXTURE_CUBE_MAP_NEGATIVE_Z,
            look: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
            up: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
        },
    ];

    pub fn new(size: usize) -> Result<PointShadowMapRenderer, RendererError> {
        unsafe {
            let mut fbo = 0;
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);

            gl::DrawBuffer(gl::NONE);

            let mut depth_buffer = 0;
            gl::GenTextures(1, &mut depth_buffer);
            gl::BindTexture(gl::TEXTURE_2D, depth_buffer);
            gl::TexImage2D(gl::TEXTURE_2D,
                           0,
                           gl::DEPTH_COMPONENT as i32,
                           size as i32,
                           size as i32,
                           0,
                           gl::DEPTH_COMPONENT,
                           gl::FLOAT,
                           std::ptr::null());
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::BindTexture(gl::TEXTURE_2D, 0);

            let mut texture = 0;
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_CUBE_MAP, texture);
            gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_BORDER as i32);
            gl::TexParameteri(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_BORDER as i32);
            let color: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
            gl::TexParameterfv(gl::TEXTURE_CUBE_MAP, gl::TEXTURE_BORDER_COLOR, color.as_ptr());

            for i in 0..6 {
                gl::TexImage2D(gl::TEXTURE_CUBE_MAP_POSITIVE_X + i,
                               0,
                               gl::R32F as i32,
                               size as i32,
                               size as i32,
                               0,
                               gl::RED,
                               gl::FLOAT,
                               std::ptr::null());
            }

            gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::TEXTURE_2D, depth_buffer, 0);

            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                return Err(RendererError::InvalidFrameBuffer);
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

            gl::BindTexture(gl::TEXTURE_CUBE_MAP, 0);

            Ok(Self {
                fbo,
                size,
                texture,
                depth_buffer,
                bone_matrices: Vec::new(),
                shader: PointShadowMapShader::new()?,
            })
        }
    }

    pub fn render(&mut self,
                  graph: &Graph,
                  white_dummy: &GpuTexture,
                  light_pos: Vec3,
                  light_radius: f32,
                  gl_state: &mut GlState,
                  texture_cache: &mut TextureCache,
                  geom_cache: &mut GeometryCache,
    ) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        gl_state.push_viewport(Rect::new(0, 0, self.size as i32, self.size as i32));

        unsafe {
            gl::DepthMask(gl::TRUE);
            gl::Disable(gl::BLEND);
            gl::Disable(gl::STENCIL_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);

            gl_state.push_fbo(self.fbo);

            let light_projection_matrix = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.01, light_radius);

            self.shader.bind()
                .set_light_position(light_pos);

            for face in Self::FACES.iter() {
                gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, face.id, self.texture, 0);
                gl::DrawBuffer(gl::COLOR_ATTACHMENT0);
                gl::ClearColor(std::f32::MAX, std::f32::MAX, std::f32::MAX, std::f32::MAX);
                gl::Clear(gl::DEPTH_BUFFER_BIT | gl::COLOR_BUFFER_BIT);

                let light_look_at = light_pos + face.look;
                let light_view_matrix = Mat4::look_at(light_pos, light_look_at, face.up).unwrap_or_default();
                let light_view_projection_matrix = light_projection_matrix * light_view_matrix;

                let frustum = Frustum::from(light_view_projection_matrix).unwrap();

                for node in graph.linear_iter() {
                    if let Node::Mesh(mesh) = node {
                        if !node.base().global_visibility() {
                            continue;
                        }

                        let global_transform = node.base().global_transform();

                        if !frustum.is_intersects_aabb_transform(&mesh.bounding_box(), &global_transform) {
                            continue;
                        }

                        for surface in mesh.surfaces().iter() {
                            let is_skinned = !surface.bones.is_empty();

                            let world = if is_skinned {
                                Mat4::IDENTITY
                            } else {
                                global_transform
                            };
                            let mvp = light_view_projection_matrix * world;

                            self.shader.set_world_matrix(&world)
                                .set_wvp_matrix(&mvp)
                                .set_use_skeletal_animation(is_skinned);

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
                            self.shader.set_diffuse_texture(0);
                            if let Some(texture) = surface.get_diffuse_texture() {
                                if let Some(texture) = texture_cache.get(texture) {
                                    texture.bind(0);
                                } else {
                                    white_dummy.bind(0);
                                }
                            } else {
                                white_dummy.bind(0);
                            }

                            statistics.add_draw_call(geom_cache.draw(&surface.get_data().lock().unwrap()));
                        }
                    }
                }
            }

            gl::DepthMask(gl::FALSE);
            gl::Enable(gl::BLEND);
        }

        gl_state.pop_fbo();
        gl_state.pop_viewport();

        statistics
    }
}

impl Drop for PointShadowMapRenderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.fbo);
            gl::DeleteTextures(1, &self.texture);
            gl::DeleteTextures(1, &self.depth_buffer);
        }
    }
}