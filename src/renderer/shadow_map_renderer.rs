use crate::{
    renderer::gpu_program::{GpuProgram, UniformLocation},
    renderer::error::RendererError,
};
use std::ffi::CString;
use crate::renderer::gl::types::GLuint;
use crate::renderer::gl;

pub struct SpotShadowMapShader {
    program: GpuProgram,
    bone_matrices: UniformLocation,
    world_view_projection_matrix: UniformLocation,
    use_skeletal_animation: UniformLocation,
    diffuse_texture: UniformLocation,
}

impl SpotShadowMapShader {
    pub fn new() -> Result<Self, RendererError> {
        let fragment_source = CString::new(r#"
            #version 330 core

            uniform sampler2D diffuseTexture;

            in vec2 texCoord;

            void main()
            {
               if(texture(diffuseTexture, texCoord).a < 0.2) discard;
            }"
        "#)?;

        let vertex_source = CString::new(r#"
            #version 330 core

            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;
            layout(location = 4) in vec4 boneWeights;
            layout(location = 5) in vec4 boneIndices;
    
            uniform mat4 worldViewProjection;
            uniform bool useSkeletalAnimation;
            uniform mat4 boneMatrices[60];
    
            out vec2 texCoord;
    
            void main()
            {
               vec4 localPosition = vec4(0);
    
               if(useSkeletalAnimation)
               {
                   vec4 vertex = vec4(vertexPosition, 1.0);
    
                   localPosition += boneMatrices[int(boneIndices.x)] * vertex * boneWeights.x;
                   localPosition += boneMatrices[int(boneIndices.y)] * vertex * boneWeights.y;
                   localPosition += boneMatrices[int(boneIndices.z)] * vertex * boneWeights.z;
                   localPosition += boneMatrices[int(boneIndices.w)] * vertex * boneWeights.w;
               }
               else
               {
                   localPosition = vec4(vertexPosition, 1.0);
               }
    
               gl_Position = worldViewProjection * localPosition;
               texCoord = vertexTexCoord;
            }
        "#)?;

        let mut program = GpuProgram::from_source("SpotShadowMapShader", &vertex_source, &fragment_source)?;

        Ok(Self {
            bone_matrices: program.get_uniform_location("boneMatrices")?,
            world_view_projection_matrix: program.get_uniform_location("worldViewProjection")?,
            use_skeletal_animation: program.get_uniform_location("useSkeletalAnimation")?,
            diffuse_texture: program.get_uniform_location("diffuseTexture")?,
            program,
        })
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
        let fragment_source = CString::new(r#"
            #version 330 core
    
            uniform sampler2D diffuseTexture;
            uniform vec3 lightPosition;
    
            in vec2 texCoord;
            in vec3 worldPosition;
    
            layout(location = 0) out float depth;
    
            void main() 
            {
               if(texture(diffuseTexture, texCoord).a < 0.2) discard;
               depth = length(lightPosition - worldPosition);
            }
        "#)?;

        let vertex_source = CString::new(r#"
            #version 330 core

            layout(location = 0) in vec3 vertexPosition;
            layout(location = 1) in vec2 vertexTexCoord;
            layout(location = 4) in vec4 boneWeights;
            layout(location = 5) in vec4 boneIndices;

            uniform mat4 worldMatrix;
            uniform mat4 worldViewProjection;
            uniform bool useSkeletalAnimation;
            uniform mat4 boneMatrices[ DE_STRINGIZE(DE_RENDERER_MAX_SKINNING_MATRICES) ];

            out vec2 texCoord;
            out vec3 worldPosition;

            void main()
            {
               vec4 localPosition = vec4(0);

               if(useSkeletalAnimation)
               {
                   vec4 vertex = vec4(vertexPosition, 1.0);

                   localPosition += boneMatrices[int(boneIndices.x)] * vertex * boneWeights.x;
                   localPosition += boneMatrices[int(boneIndices.y)] * vertex * boneWeights.y;
                   localPosition += boneMatrices[int(boneIndices.z)] * vertex * boneWeights.z;
                   localPosition += boneMatrices[int(boneIndices.w)] * vertex * boneWeights.w;
               }
               else
               {
                   localPosition = vec4(vertexPosition, 1.0);
               }

               gl_Position = worldViewProjection * localPosition;
               worldPosition = (worldMatrix * localPosition).xyz;
               texCoord = vertexTexCoord;
            }
        "#)?;

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
}

pub struct PointShadowMapRenderer {
    shader: PointShadowMapShader,
    fbo: GLuint,
    texture: GLuint,
    depth_buffer: GLuint,
}

impl PointShadowMapRenderer {
    pub fn new(size: i32) -> Result<PointShadowMapRenderer, RendererError> {
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
                           size,
                           size,
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
                               size,
                               size,
                               0,
                               gl::RED,
                               gl::FLOAT,
                               std::ptr::null());
            }

            gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::TEXTURE_2D, depth_buffer, 0);

            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                panic!("Unable to initialize shadow map.");
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

            Ok(Self {
                shader: PointShadowMapShader::new()?,
                fbo,
                texture,
                depth_buffer,
            })
        }
    }
}