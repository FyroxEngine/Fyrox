use std::ffi::CString;
use crate::{
    scene::{
        camera::Camera,
        Scene,
        node::NodeKind
    },
    renderer::{
        gl::types::GLuint,
        gl,
        gpu_program::{GpuProgram, UniformLocation},
        error::RendererError
    },
    resource::ResourceKind,
};
use rg3d_core::{
    math::{
        mat4::Mat4,
        vec2::Vec2,
        Rect,
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
        let fragment_source = CString::new(r#"
            #version 330 core

            layout(location = 0) out float outDepth;
            layout(location = 1) out vec4 outColor;
            layout(location = 2) out vec4 outNormal;

            uniform sampler2D diffuseTexture;
            uniform sampler2D normalTexture;
            uniform sampler2D specularTexture;

            in vec4 position;
            in vec3 normal;
            in vec2 texCoord;
            in vec3 tangent;
            in vec3 binormal;

            void main()
            {
               outDepth = position.z / position.w;
               outColor = texture2D(diffuseTexture, texCoord);
               if(outColor.a < 0.5) discard;
               outColor.a = 1;
               vec4 n = normalize(texture2D(normalTexture, texCoord) * 2.0 - 1.0);
               mat3 tangentSpace = mat3(tangent, binormal, normal);
               outNormal.xyz = normalize(tangentSpace * n.xyz) * 0.5 + 0.5;
               outNormal.w = texture2D(specularTexture, texCoord).r;
            }
        "#)?;

        let vertex_source = CString::new(r#"
            #version 330 core

            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;
            layout(location = 2) in vec3 vertexNormal;
            layout(location = 3) in vec4 vertexTangent;
            layout(location = 4) in vec4 boneWeights;
            layout(location = 5) in vec4 boneIndices;

            uniform mat4 worldMatrix;
            uniform mat4 worldViewProjection;
            uniform bool useSkeletalAnimation;
            uniform mat4 boneMatrices[60];

            out vec4 position;
            out vec3 normal;
            out vec2 texCoord;
            out vec3 tangent;
            out vec3 binormal;

            void main()
            {
               vec4 localPosition = vec4(0);
               vec3 localNormal = vec3(0);
               vec3 localTangent = vec3(0);
               if(useSkeletalAnimation)
               {
                   vec4 vertex = vec4(vertexPosition, 1.0);

                   int i0 = int(boneIndices.x);
                   int i1 = int(boneIndices.y);
                   int i2 = int(boneIndices.z);
                   int i3 = int(boneIndices.w);

                   localPosition += boneMatrices[i0] * vertex * boneWeights.x;
                   localPosition += boneMatrices[i1] * vertex * boneWeights.y;
                   localPosition += boneMatrices[i2] * vertex * boneWeights.z;
                   localPosition += boneMatrices[i3] * vertex * boneWeights.w;

                   localNormal += mat3(boneMatrices[i0]) * vertexNormal * boneWeights.x;
                   localNormal += mat3(boneMatrices[i1]) * vertexNormal * boneWeights.y;
                   localNormal += mat3(boneMatrices[i2]) * vertexNormal * boneWeights.z;
                   localNormal += mat3(boneMatrices[i3]) * vertexNormal * boneWeights.w;

                   localTangent += mat3(boneMatrices[i0]) * vertexTangent.xyz * boneWeights.x;
                   localTangent += mat3(boneMatrices[i1]) * vertexTangent.xyz * boneWeights.y;
                   localTangent += mat3(boneMatrices[i2]) * vertexTangent.xyz * boneWeights.z;
                   localTangent += mat3(boneMatrices[i3]) * vertexTangent.xyz * boneWeights.w;
               }
               else
               {
                   localPosition = vec4(vertexPosition, 1.0);
                   localNormal = vertexNormal;
                   localTangent = vertexTangent.xyz;
               }
               gl_Position = worldViewProjection * localPosition;
               normal = normalize(mat3(worldMatrix) * localNormal);
               tangent = normalize(mat3(worldMatrix) * localTangent);
               binormal = normalize(vertexTangent.w * cross(tangent, normal));
               texCoord = vertexTexCoord;
               position = gl_Position;
            }
        "#)?;

        let mut program = GpuProgram::from_source("GBufferShader", &vertex_source, &fragment_source)?;

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

    fn bind(&self) {
        self.program.bind()
    }

    fn set_world_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.world_matrix, mat)
    }

    fn set_wvp_matrix(&self, mat: &Mat4) {
        self.program.set_mat4(self.wvp_matrix, mat)
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

    fn set_normal_texture(&self, id: i32) {
        self.program.set_int(self.normal_texture, id)
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
                panic!("Unable to construct G-Buffer FBO.");
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
                panic!("Unable to initialize Stencil FBO.");
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
            })
        }
    }

    pub fn fill(&mut self, frame_width: f32, frame_height: f32, scene: &Scene, camera: &Camera, white_dummy: GLuint, normal_dummy: GLuint) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
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

            // Setup viewport
            let viewport: Rect<i32> = camera.get_viewport_pixels(Vec2 { x: frame_width, y: frame_height });
            gl::Viewport(viewport.x, viewport.y, viewport.w, viewport.h);

            let view_projection = camera.get_view_projection_matrix();

            for node in scene.get_nodes().iter() {
                if let NodeKind::Mesh(mesh) = node.borrow_kind() {
                    if !node.get_global_visibility() {
                        continue;
                    }

                    for surface in mesh.get_surfaces().iter() {
                        let is_skinned = !surface.bones.is_empty();

                        let world = if is_skinned {
                            Mat4::identity()
                        } else {
                            *node.get_global_transform()
                        };
                        let mvp = view_projection * world;

                        self.shader.set_wvp_matrix(&mvp);
                        self.shader.set_world_matrix(&world);

                        self.shader.set_use_skeletal_animation(is_skinned);

                        if is_skinned {
                            self.bone_matrices.clear();
                            for bone_handle in surface.bones.iter() {
                                if let Some(bone_node) = scene.get_node(*bone_handle) {
                                    self.bone_matrices.push(
                                        *bone_node.get_global_transform() *
                                            *bone_node.get_inv_bind_pose_transform());
                                } else {
                                    self.bone_matrices.push(Mat4::identity())
                                }
                            }

                            self.shader.set_bone_matrices(&self.bone_matrices);
                        }

                        // Bind diffuse texture.
                        gl::ActiveTexture(gl::TEXTURE0);
                        if let Some(resource) = surface.get_diffuse_texture() {
                            if let ResourceKind::Texture(texture) = resource.lock().unwrap().borrow_kind() {
                                gl::BindTexture(gl::TEXTURE_2D, texture.gpu_tex);
                            } else {
                                gl::BindTexture(gl::TEXTURE_2D, white_dummy);
                            }
                        } else {
                            gl::BindTexture(gl::TEXTURE_2D, white_dummy);
                        }

                        // Bind normal texture.
                        gl::ActiveTexture(gl::TEXTURE1);
                        if let Some(resource) = surface.get_normal_texture() {
                            if let ResourceKind::Texture(texture) = resource.lock().unwrap().borrow_kind() {
                                gl::BindTexture(gl::TEXTURE_2D, texture.gpu_tex);
                            } else {
                                gl::BindTexture(gl::TEXTURE_2D, normal_dummy);
                            }
                        } else {
                            gl::BindTexture(gl::TEXTURE_2D, normal_dummy);
                        }

                        surface.get_data().lock().unwrap().draw();
                    }
                }
            }
        }
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