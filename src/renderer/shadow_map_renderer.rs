use std::ffi::CString;
use crate::core::math::{
    mat4::Mat4,
    vec3::Vec3,
    frustum::Frustum,
};
use crate::{
    scene::{
        node::Node,
        graph::Graph,
        base::AsBase,
    },
    renderer::{
        gpu_texture::GpuTexture,
        gl::types::GLuint,
        gpu_program::{GpuProgram, UniformLocation},
        error::RendererError,
        gl,
    },
};
use crate::renderer::RenderPassStatistics;

pub struct SpotShadowMapShader {
    program: GpuProgram,
    bone_matrices: UniformLocation,
    world_view_projection_matrix: UniformLocation,
    use_skeletal_animation: UniformLocation,
    diffuse_texture: UniformLocation,
}

impl SpotShadowMapShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = CString::new(include_str!("shaders/spot_shadow_map_fs.glsl"))?;
        let vertex_source = CString::new(include_str!("shaders/spot_shadow_map_vs.glsl"))?;
        let mut program = GpuProgram::from_source("SpotShadowMapShader", &vertex_source, &fragment_source)?;
        Ok(Self {
            bone_matrices: program.get_uniform_location("boneMatrices")?,
            world_view_projection_matrix: program.get_uniform_location("worldViewProjection")?,
            use_skeletal_animation: program.get_uniform_location("useSkeletalAnimation")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            program,
        })
    }

    fn bind(&self) {
        self.program.bind()
    }

    fn set_wvp_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.world_view_projection_matrix, mat)
    }

    fn set_use_skeletal_animation(&self, value: bool) {
        self.program.set_int(self.use_skeletal_animation, if value { 1 } else { 0 })
    }

    fn set_bone_matrices(&self, matrices: &[Mat4]) {
        self.program.set_mat4_array(self.bone_matrices, matrices);
    }

    fn set_diffuse_texture(&self, id: i32) {
        self.program.set_int(self.diffuse_texture, id)
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
    ) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        let mut old_fbo = 0;
        let mut old_viewport = [0; 4];

        unsafe {
            gl::DepthMask(gl::TRUE);
            gl::Disable(gl::BLEND);
            gl::Disable(gl::STENCIL_TEST);
            gl::Enable(gl::CULL_FACE);

            gl::GetIntegerv(gl::DRAW_FRAMEBUFFER_BINDING, &mut old_fbo);
            gl::GetIntegerv(gl::VIEWPORT, old_viewport.as_mut_ptr());

            gl::Viewport(0, 0, self.size as i32, self.size as i32);
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
            gl::Clear(gl::DEPTH_BUFFER_BIT);
        }

        self.shader.bind();

        let frustum = Frustum::from(*light_view_projection).unwrap();

        for node in graph.linear_iter() {
            if let Node::Mesh(mesh) = node {
                if !node.base().get_global_visibility() {
                    continue;
                }

                let global_transform = node.base().get_global_transform();

                if !frustum.is_intersects_aabb_transform(&mesh.get_bounding_box(), &global_transform) {
                    continue;
                }

                for surface in mesh.get_surfaces().iter() {
                    let is_skinned = !surface.bones.is_empty();

                    let world = if is_skinned {
                        Mat4::IDENTITY
                    } else {
                        global_transform
                    };
                    let mvp = *light_view_projection * world;

                    self.shader.set_wvp_matrix(&mvp);
                    self.shader.set_use_skeletal_animation(is_skinned);

                    if is_skinned {
                        self.bone_matrices.clear();
                        for bone_handle in surface.bones.iter() {
                            let bone_node = graph.get(*bone_handle);
                            self.bone_matrices.push(
                                bone_node.base().get_global_transform() *
                                    bone_node.base().get_inv_bind_pose_transform());
                        }

                        self.shader.set_bone_matrices(&self.bone_matrices);
                    }

                    // Bind diffuse texture.
                    self.shader.set_diffuse_texture(0);
                    if let Some(texture) = surface.get_diffuse_texture() {
                        if let Some(texture) = texture.lock().unwrap().gpu_tex.as_ref() {
                            texture.bind(0);
                        } else {
                            white_dummy.bind(0);
                        }
                    } else {
                        white_dummy.bind(0);
                    }

                    statistics.add_draw_call(surface.get_data().lock().unwrap().draw());
                }
            }
        }

        unsafe {
            // Set previous state
            gl::BindFramebuffer(gl::FRAMEBUFFER, old_fbo as u32);
            gl::Viewport(old_viewport[0], old_viewport[1], old_viewport[2], old_viewport[3]);
        }

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
        let fragment_source = CString::new(include_str!("shaders/point_shadow_map_fs.glsl"))?;
        let vertex_source = CString::new(include_str!("shaders/point_shadow_map_vs.glsl"))?;
        let mut program = GpuProgram::from_source("PointShadowMapShader", &vertex_source, &fragment_source)?;
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

    pub fn bind(&self) {
        self.program.bind();
    }

    fn set_wvp_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.world_view_projection_matrix, mat)
    }

    fn set_world_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.world_matrix, mat)
    }

    fn set_use_skeletal_animation(&self, value: bool) {
        self.program.set_int(self.use_skeletal_animation, if value { 1 } else { 0 })
    }

    fn set_bone_matrices(&self, matrices: &[Mat4]) {
        self.program.set_mat4_array(self.bone_matrices, matrices);
    }

    fn set_diffuse_texture(&self, id: i32) {
        self.program.set_int(self.diffuse_texture, id)
    }

    fn set_light_position(&self, pos: Vec3) {
        self.program.set_vec3(self.light_position, &pos);
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
    ) -> RenderPassStatistics {
        let mut statistics = RenderPassStatistics::default();

        unsafe {
            gl::DepthMask(gl::TRUE);
            gl::Disable(gl::BLEND);
            gl::Disable(gl::STENCIL_TEST);
            gl::Enable(gl::CULL_FACE);
            gl::CullFace(gl::BACK);

            let mut old_fbo = 0;
            gl::GetIntegerv(gl::DRAW_FRAMEBUFFER_BINDING, &mut old_fbo);

            let mut old_viewport = [0; 4];
            gl::GetIntegerv(gl::VIEWPORT, old_viewport.as_mut_ptr());

            gl::Viewport(0, 0, self.size as i32, self.size as i32);
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);

            let light_projection_matrix = Mat4::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.01, light_radius);

            self.shader.bind();
            self.shader.set_light_position(light_pos);

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
                        if !node.base().get_global_visibility() {
                            continue;
                        }

                        let global_transform = node.base().get_global_transform();

                        if !frustum.is_intersects_aabb_transform(&mesh.get_bounding_box(), &global_transform) {
                            continue;
                        }

                        for surface in mesh.get_surfaces().iter() {
                            let is_skinned = !surface.bones.is_empty();

                            let world = if is_skinned {
                                Mat4::IDENTITY
                            } else {
                                global_transform
                            };
                            let mvp = light_view_projection_matrix * world;

                            self.shader.set_world_matrix(&world);
                            self.shader.set_wvp_matrix(&mvp);
                            self.shader.set_use_skeletal_animation(is_skinned);

                            if is_skinned {
                                self.bone_matrices.clear();
                                for bone_handle in surface.bones.iter() {
                                    let bone_node = graph.get(*bone_handle);
                                    self.bone_matrices.push(
                                        bone_node.base().get_global_transform() *
                                            bone_node.base().get_inv_bind_pose_transform());
                                }

                                self.shader.set_bone_matrices(&self.bone_matrices);
                            }

                            // Bind diffuse texture.
                            self.shader.set_diffuse_texture(0);
                            if let Some(texture) = surface.get_diffuse_texture() {
                                if let Some(texture) = texture.lock().unwrap().gpu_tex.as_ref() {
                                    texture.bind(0);
                                } else {
                                    white_dummy.bind(0);
                                }
                            } else {
                                white_dummy.bind(0);
                            }

                            statistics.add_draw_call(surface.get_data().lock().unwrap().draw());
                        }
                    }
                }
            }

            gl::DepthMask(gl::FALSE);
            gl::Enable(gl::BLEND);

            // Set previous state
            gl::BindFramebuffer(gl::FRAMEBUFFER, old_fbo as u32);
            gl::Viewport(old_viewport[0], old_viewport[1], old_viewport[2], old_viewport[3]);
        }

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