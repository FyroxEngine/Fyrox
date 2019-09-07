use crate::{
    renderer::gpu_program::{GpuProgram, UniformLocation},
    renderer::error::RendererError
};
use std::ffi::CString;

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
}

impl PointShadowMapRenderer {
    pub fn new() -> Result<PointShadowMapRenderer, RendererError> {
        Ok(Self {
            shader: PointShadowMapShader::new()?
        })
    }
}